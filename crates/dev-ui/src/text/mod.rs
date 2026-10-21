//! All the text the dev UI shows: the `check` output (load errors, counts, turn lines) and
//! the window's panels (the world's goods, resource types, POP types, technologies and
//! businesses, and a hex's deposits, stockpile, market, POPs and businesses). It's built from
//! engine types without egui, so it can be tested. The wording follows the Step 02–09 designs. Amounts are written with `Quantity`'s `Display`,
//! the same way `check --turns` writes them.

use std::path::Path;

use policyspace_engine::{
    GoodId, HexId, LoadErrors, Perishes, PerishesAt, PopId, Quantity, ReportedFailure, ResourceId,
    Stockpile, Turn, TurnReport, World,
};

mod businesses;
pub use businesses::{business_rows, business_section, technology_rows};

/// The lines reported for a failed load: each error's `Display`, in order, then
/// `"{n} error(s) found in {folder.display()}"`. The `check` command prints them to stderr
/// and the window's error view shows them, so both show the same lines.
pub fn error_lines(errors: &LoadErrors, folder: &Path) -> Vec<String> {
    let mut lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    lines.push(format!(
        "{} error(s) found in {}",
        errors.len(),
        folder.display()
    ));
    lines
}

/// The lines shown when a setting loads but its deciders can't start (or a name can't be
/// resolved): `message`'s lines, then `1 error(s) found in {folder.display()}` so the status
/// bar's count and the error view read as for a broken setting.
pub fn load_failure_lines(message: &str, folder: &Path) -> Vec<String> {
    let mut lines: Vec<String> = message.lines().map(str::to_string).collect();
    lines.push(format!("1 error(s) found in {}", folder.display()));
    lines
}

/// The lines logged for one decider failure at `turn`: `Turn {turn}: decider '{name}' failed:
/// {failure}` (the failure's one-line `Display`), then each stderr tail line as
/// `    stderr: {line}`.
pub fn failure_lines(turn: Turn, failure: &ReportedFailure) -> Vec<String> {
    let mut lines = vec![format!(
        "Turn {turn}: decider '{}' failed: {}",
        failure.decider, failure.failure
    )];
    lines.extend(
        failure
            .failure
            .stderr_tail()
            .iter()
            .map(|line| format!("    stderr: {line}")),
    );
    lines
}

/// "1 hex", "19 hexes": the count with the singular or plural word.
pub fn count(n: usize, singular: &str, plural: &str) -> String {
    let word = if n == 1 { singular } else { plural };
    format!("{n} {word}")
}

/// "10 copper, 24 grain", or "nothing" for an empty list.
fn list(world: &World, entries: &[(GoodId, Quantity)]) -> String {
    if entries.is_empty() {
        return "nothing".to_string();
    }
    entries
        .iter()
        .map(|(good, amount)| format!("{amount} {}", world.rules().goods.name(*good)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The line printed for a turn, by `check` and in the window's turn log: what perished, what
/// deposits emitted, what POPs were endowed with, what POPs sold, bought and consumed, the
/// total population after growth, and what businesses sold and bought and what their
/// factories produced and used, each list a list or `nothing`.
pub fn turn_line(world: &World, report: &TurnReport) -> String {
    format!(
        "Turn {}: perished {}; emitted {}; endowed {}; sold {}; bought {}; consumed {}; population {}; businesses sold {}; businesses bought {}; produced {}; used {}",
        report.turn,
        list(world, &report.perished),
        list(world, &report.emitted),
        list(world, &report.endowed),
        list(world, &report.sold),
        list(world, &report.bought),
        list(world, &report.consumed),
        report.population,
        list(world, &report.business_sold),
        list(world, &report.business_bought),
        list(world, &report.produced),
        list(world, &report.used)
    )
}

/// When a group perishes, as a countdown from turn `now`: "perishes this turn",
/// "perishes in 1 turn", "perishes in {n} turns", or "never perishes".
pub fn countdown(at: PerishesAt, now: Turn) -> String {
    match at {
        PerishesAt::Never => "never perishes".to_string(),
        PerishesAt::EndOfTurn(end) => match end.get().saturating_sub(now.get()) {
            0 => "perishes this turn".to_string(),
            1 => "perishes in 1 turn".to_string(),
            n => format!("perishes in {n} turns"),
        },
    }
}

/// How long a good lasts: "perishes after 1 turn", "perishes after {n} turns", or
/// "never perishes".
pub fn lasts(perishes: Perishes) -> String {
    match perishes {
        Perishes::Never => "never perishes".to_string(),
        Perishes::After(n) if n.get() == 1 => "perishes after 1 turn".to_string(),
        Perishes::After(n) => format!("perishes after {n} turns"),
    }
}

/// One line per resource type present on `hex`, in resource ID order:
/// "{type} × {count}: {output} {good} each, {total} total". Empty if it has no deposits.
pub fn deposit_lines(world: &World, hex: HexId) -> Vec<String> {
    deposit_lines_from(world, |id| world.deposits().of_type_on_hex(hex, id).count())
}

/// One line per resource type with `count_of(type)` above 0, in resource ID order, worded as
/// `deposit_lines` words them.
fn deposit_lines_from(world: &World, count_of: impl Fn(ResourceId) -> usize) -> Vec<String> {
    let rules = world.rules();
    let mut lines = Vec::new();
    for (id, name, resource) in rules.resources.iter() {
        let count = count_of(id);
        if count == 0 {
            continue;
        }
        // The total is the output added `count` times rather than multiplied, so it matches
        // the hex stockpile's total, which emission builds by adding each deposit's output
        // in turn.
        let total = (0..count).fold(Quantity::ZERO, |sum, _| sum.saturating_add(resource.output));
        lines.push(format!(
            "{name} × {count}: {} {} each, {total} total",
            resource.output,
            rules.goods.name(resource.produces)
        ));
    }
    lines
}

/// One good in a hex's stockpile.
#[derive(Debug, Clone, PartialEq)]
pub struct StockLine {
    /// "{total} {good}", e.g. "15 stew".
    pub good: String,
    /// One per group, soonest first: "{amount}: {countdown}", e.g. "5: perishes in 1 turn".
    pub parts: Vec<String>,
}

/// The stockpile of `hex`, in good ID order. Empty if the stockpile is empty.
pub fn stockpile_lines(world: &World, hex: HexId) -> Vec<StockLine> {
    stock_lines(world, world.hex_stockpile(hex))
}

/// The stockpile of `pop`, written exactly like a hex's (`stockpile_lines`).
pub fn pop_stockpile_lines(world: &World, pop: PopId) -> Vec<StockLine> {
    stock_lines(world, world.pop_stockpile(pop))
}

/// The lines for any stockpile, in good ID order. Empty if the stockpile is empty.
fn stock_lines(world: &World, stockpile: &Stockpile) -> Vec<StockLine> {
    let goods = &world.rules().goods;
    stockpile
        .iter()
        .map(|(good, total)| StockLine {
            good: format!("{total} {}", goods.name(good)),
            parts: stockpile
                .groups(good)
                .map(|(at, amount)| format!("{amount}: {}", countdown(at, world.turn())))
                .collect(),
        })
        .collect()
}

/// The Market section of `hex`: `"Currency: {currency}"`, then one line per posted price in
/// good ID order, `"{good}: {price} {currency}"` (the currency itself is never posted, so it
/// never appears); or, after the currency line, `"No prices posted."` for a market with no
/// prices, or `"No market."` for a hex with no market. Never empty.
pub fn market_lines(world: &World, hex: HexId) -> Vec<String> {
    let market = world.market(hex);
    let goods = &world.rules().goods;
    let currency = goods.name(market.currency());
    let mut lines = vec![format!("Currency: {currency}")];
    if !market.has_market() {
        lines.push("No market.".to_string());
        return lines;
    }
    for (good, price) in market.prices() {
        lines.push(format!("{}: {price} {currency}", goods.name(good)));
    }
    if lines.len() == 1 {
        lines.push("No prices posted.".to_string());
    }
    lines
}

/// The subsistence index to 4 fixed decimals: "0.6667", "1.0000", "0.0000", "-1.0000".
pub fn format_index(index: f64) -> String {
    // Adding 0.0 turns -0.0 into 0.0, which would otherwise print as "-0.0000".
    format!("{:.4}", index + 0.0)
}

/// The growth rate as a signed percentage to 2 fixed decimals: "+2.67%", "-10.00%", "+0.00%".
pub fn format_rate(rate: f64) -> String {
    // Adding 0.0 turns -0.0 into 0.0, which would otherwise print as "-0.00%".
    format!("{:+.2}%", rate * 100.0 + 0.0)
}

/// A normalized share rounded to 4 decimals with trailing zeros dropped: "0.8", "0.1",
/// "0.3333", "1", "0".
pub fn format_share(share: f64) -> String {
    ((share * 10_000.0).round() / 10_000.0 + 0.0).to_string()
}

/// One row of a POP's goods table. Every field is formatted; `below_subsistence` flags a row
/// whose demanded quantity falls short of its subsistence (drawn in the error color).
#[derive(Debug, Clone, PartialEq)]
pub struct PopGoodRow {
    /// The good's name.
    pub good: String,
    /// What the POP held (`DemandedGood::held`).
    pub held: String,
    /// What the POP wanted to end up with (`DemandedGood::quantity`).
    pub demanded: String,
    /// The subsistence amount, already the type's amount times the starting size.
    pub subsistence: String,
    /// What the POP sold ("0" when nothing).
    pub sold: String,
    /// What the POP bought ("0" when nothing).
    pub bought: String,
    /// What the POP consumed ("0" when nothing).
    pub consumed: String,
    /// `DemandedGood::below_subsistence()`.
    pub below_subsistence: bool,
}

/// Everything the hex panel shows for one POP.
#[derive(Debug, Clone, PartialEq)]
pub struct PopSection {
    /// "{type} × {current size}", e.g. "peasant × 61.6".
    pub heading: String,
    /// "Started at {starting size}; index {format_index}; rate {format_rate}".
    pub growth: String,
    /// The hover text of the growth line: "index {index}; rate {rate}" with both as full
    /// f64 `Display` (e.g. "index 0.6666666666666666; rate 0.02666666666666667").
    pub growth_hover: String,
    /// "Income {M} {currency}; subsistence cost {C} {currency}; supernumerary {M-C} {currency}".
    /// M and C are `Quantity`; the supernumerary is an f64 written with f64 `Display`
    /// (so 240.0 writes "240", and a negative value keeps its "-").
    pub demand: String,
    /// One row per good in the recorded demand, in good ID order.
    pub goods: Vec<PopGoodRow>,
    /// "Carried over: {amount} {currency}".
    pub carried_over: String,
    /// `pop_stockpile_lines(world, pop)`.
    pub stockpile: Vec<StockLine>,
}

/// The section for `pop` from the current turn's outcome and the POP's current size.
pub fn pop_section(world: &World, pop: PopId) -> PopSection {
    let rules = world.rules();
    let record = world.pops().get(pop);
    let outcome = world.pop_outcome(pop);
    let demand = outcome.demand();
    let currency = rules.goods.name(world.hex_currency(record.hex));
    PopSection {
        heading: format!(
            "{} × {}",
            rules.pop_types.name(record.pop_type),
            record.size
        ),
        growth: format!(
            "Started at {}; index {}; rate {}",
            outcome.starting_size(),
            format_index(outcome.subsistence_index()),
            format_rate(outcome.growth_rate())
        ),
        growth_hover: format!(
            "index {}; rate {}",
            outcome.subsistence_index(),
            outcome.growth_rate()
        ),
        demand: format!(
            "Income {} {currency}; subsistence cost {} {currency}; supernumerary {} {currency}",
            demand.income(),
            demand.subsistence_cost(),
            // Adding 0.0 turns -0.0 into 0.0, which would otherwise print as "-0".
            demand.supernumerary() + 0.0
        ),
        goods: demand
            .goods()
            .iter()
            .map(|g| PopGoodRow {
                good: rules.goods.name(g.good).to_string(),
                held: g.held.to_string(),
                demanded: g.quantity.to_string(),
                subsistence: g.subsistence.to_string(),
                sold: outcome.sold_of(g.good).to_string(),
                bought: outcome.bought_of(g.good).to_string(),
                consumed: outcome.consumed_of(g.good).to_string(),
                below_subsistence: g.below_subsistence(),
            })
            .collect(),
        carried_over: format!("Carried over: {} {currency}", outcome.carried_over()),
        stockpile: pop_stockpile_lines(world, pop),
    }
}

/// One row of the Goods table.
#[derive(Debug, Clone, PartialEq)]
pub struct GoodRow {
    /// The good's name.
    pub name: String,
    /// `lasts(..)`.
    pub lasts: String,
    /// "movable" or "fixed".
    pub movable: &'static str,
    /// The world total over every hex stockpile, formatted (`Quantity` Display; "0" if none).
    pub in_hexes: String,
    /// The world total over every POP stockpile, formatted ("0" if none).
    pub in_pops: String,
    /// The world total over every business stockpile, formatted ("0" if none).
    pub in_businesses: String,
}

/// Every good, in good ID order.
pub fn good_rows(world: &World) -> Vec<GoodRow> {
    let goods = &world.rules().goods;
    // Walk the hexes once, then the POPs once, then the businesses once, adding each stockpile's totals into the
    // good's slot.
    let mut in_hexes = vec![Quantity::ZERO; goods.len()];
    for (hex, _) in world.map().iter() {
        for (good, total) in world.hex_stockpile(hex).iter() {
            in_hexes[good.index()] = in_hexes[good.index()].saturating_add(total);
        }
    }
    let mut in_pops = vec![Quantity::ZERO; goods.len()];
    for (pop, _) in world.pops().iter() {
        for (good, total) in world.pop_stockpile(pop).iter() {
            in_pops[good.index()] = in_pops[good.index()].saturating_add(total);
        }
    }
    let mut in_businesses = vec![Quantity::ZERO; goods.len()];
    for (business, _, _) in world.businesses().iter() {
        for (good, total) in world.business_stockpile(business).iter() {
            in_businesses[good.index()] = in_businesses[good.index()].saturating_add(total);
        }
    }
    goods
        .iter()
        .map(|(id, name, good)| GoodRow {
            name: name.to_string(),
            lasts: lasts(good.perishes),
            movable: if good.movable { "movable" } else { "fixed" },
            in_hexes: in_hexes[id.index()].to_string(),
            in_pops: in_pops[id.index()].to_string(),
            in_businesses: in_businesses[id.index()].to_string(),
        })
        .collect()
}

/// One row of the Resource types table.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRow {
    /// The resource type's name.
    pub name: String,
    /// The produced good's name.
    pub produces: String,
    /// Output per deposit, formatted.
    pub output: String,
    /// The number of deposits of this type in the world.
    pub deposits: usize,
}

/// Every resource type, in resource ID order.
pub fn resource_rows(world: &World) -> Vec<ResourceRow> {
    let rules = world.rules();
    let mut counts = vec![0usize; rules.resources.len()];
    for (_, deposit) in world.deposits().iter() {
        counts[deposit.resource.index()] += 1;
    }
    rules
        .resources
        .iter()
        .map(|(id, name, resource)| ResourceRow {
            name: name.to_string(),
            produces: rules.goods.name(resource.produces).to_string(),
            output: resource.output.to_string(),
            deposits: counts[id.index()],
        })
        .collect()
}

/// One utility good of a POP type.
#[derive(Debug, Clone, PartialEq)]
pub struct UtilityLine {
    /// "{good}: subsistence {amount}, share {format_share}", e.g. "grain: subsistence 2,
    /// share 0.8".
    pub line: String,
    /// The hover text: the full normalized share with f64 `Display`, e.g. "0.8".
    pub share: String,
}

/// One row of the POP types table.
#[derive(Debug, Clone, PartialEq)]
pub struct PopTypeRow {
    /// The POP type's name.
    pub name: String,
    /// One per endowed good, in good ID order: "{amount} {good}"; or exactly ["nothing"] for
    /// an empty endowment.
    pub endowment: Vec<String>,
    /// One per utility good, in good ID order.
    pub utility: Vec<UtilityLine>,
    /// "growth [{c1}, {c2}, …]; decline {d}; max {m}", every number with f64 `Display`,
    /// coefficients joined by ", ": "growth [0.06, -0.03]; decline 0.1; max 0.05".
    pub growth: String,
    /// The number of POPs of this type in the world.
    pub pops: usize,
    /// The sum of their current sizes, formatted; "0" with no POPs.
    pub total_size: String,
}

/// Every POP type, in POP type ID order.
pub fn pop_type_rows(world: &World) -> Vec<PopTypeRow> {
    let rules = world.rules();
    let mut counts = vec![0usize; rules.pop_types.len()];
    let mut sizes = vec![Quantity::ZERO; rules.pop_types.len()];
    for (_, pop) in world.pops().iter() {
        let slot = pop.pop_type.index();
        counts[slot] += 1;
        sizes[slot] = sizes[slot].saturating_add(pop.size);
    }
    rules
        .pop_types
        .iter()
        .map(|(id, name, pop_type)| {
            let endowment = if pop_type.endowment().is_empty() {
                vec!["nothing".to_string()]
            } else {
                pop_type
                    .endowment()
                    .iter()
                    .map(|e| format!("{} {}", e.amount, rules.goods.name(e.good)))
                    .collect()
            };
            let growth = pop_type.growth();
            let coefficients: Vec<String> = growth
                .coefficients()
                .iter()
                .map(ToString::to_string)
                .collect();
            PopTypeRow {
                name: name.to_string(),
                endowment,
                utility: pop_type
                    .utility()
                    .iter()
                    .map(|u| UtilityLine {
                        line: format!(
                            "{}: subsistence {}, share {}",
                            rules.goods.name(u.good),
                            u.subsistence,
                            format_share(u.share)
                        ),
                        share: u.share.to_string(),
                    })
                    .collect(),
                growth: format!(
                    "growth [{}]; decline {}; max {}",
                    coefficients.join(", "),
                    growth.decline(),
                    growth.max_growth()
                ),
                pops: counts[id.index()],
                total_size: sizes[id.index()].to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use std::time::Duration;

    use policyspace_engine::{DeciderFailure, DeciderHost, HexCoord, load_setting};

    use super::*;
    use crate::test_support::{python_world, world_at};

    #[test]
    fn failure_lines_and_load_failure_lines() {
        let failure = ReportedFailure {
            decider: "x".to_string(),
            failure: DeciderFailure::TimedOut {
                limit: Duration::from_millis(2500),
                stderr: vec!["a".to_string(), "b".to_string()],
            },
        };
        assert_eq!(
            failure_lines(Turn::new(7), &failure),
            [
                "Turn 7: decider 'x' failed: no reply within the time limit of 2.5 s",
                "    stderr: a",
                "    stderr: b"
            ]
        );
        let quiet = ReportedFailure {
            decider: "y".to_string(),
            failure: DeciderFailure::NoRecording { turn: Turn::new(3) },
        };
        assert_eq!(
            failure_lines(Turn::new(3), &quiet),
            ["Turn 3: decider 'y' failed: no recording for turn 3"]
        );
        let folder = Path::new("some/folder");
        assert_eq!(
            load_failure_lines(
                "one
two", folder
            ),
            [
                "one".to_string(),
                "two".to_string(),
                format!("1 error(s) found in {}", folder.display())
            ]
        );
    }

    /// The sample world at turn 0.
    fn sample() -> World {
        world_at("settings/sample")
    }

    /// The `turns_valid` world stepped to turn 2.
    fn turns_valid_at_2() -> World {
        let mut world = world_at("crates/engine/tests/settings/turns_valid");
        world.step(&mut DeciderHost::idle()).unwrap();
        world.step(&mut DeciderHost::idle()).unwrap();
        world
    }

    /// The hex of `world` at (q, r).
    fn hex(world: &World, q: i32, r: i32) -> HexId {
        world.map().id(HexCoord::new(q, r)).expect("hex exists")
    }

    /// `EndOfTurn(t)`.
    fn end(t: u64) -> PerishesAt {
        PerishesAt::EndOfTurn(Turn::new(t))
    }

    /// `Perishes::After(n)`.
    fn after(n: u32) -> Perishes {
        Perishes::After(NonZeroU32::new(n).unwrap())
    }

    /// Strings from literals.
    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_turn_line_ends_with_the_business_lists_in_order() {
        let world = sample();
        let good = |name: &str| world.rules().goods.id(name).unwrap();
        let q = |value: f64| Quantity::new(value).unwrap();
        let report = TurnReport {
            turn: Turn::new(4),
            perished: vec![],
            emitted: vec![],
            endowed: vec![],
            sold: vec![],
            bought: vec![],
            consumed: vec![],
            population: q(12.5),
            business_sold: vec![(good("copper"), q(2.5)), (good("grain"), q(12.0))],
            business_bought: vec![(good("tools"), q(0.75))],
            produced: vec![(good("tools"), q(4.0))],
            used: vec![
                (good("unskilled_labor"), q(17.25)),
                (good("copper"), q(8.0)),
            ],
            decisions: Default::default(),
            decider_failures: vec![],
        };
        assert_eq!(
            turn_line(&world, &report),
            "Turn 4: perished nothing; emitted nothing; endowed nothing; sold nothing; bought nothing; consumed nothing; population 12.5; businesses sold 2.5 copper, 12 grain; businesses bought 0.75 tools; produced 4 tools; used 17.25 unskilled_labor, 8 copper"
        );
    }

    #[test]
    fn error_lines_lists_each_error_then_a_summary() {
        let folder =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/settings/many_errors");
        let errors = load_setting(&folder).expect_err("many_errors has errors");
        let lines = error_lines(&errors, &folder);
        assert_eq!(
            lines,
            vec![
                "setting.toml:2:18: unsupported format_version 3 (this engine supports format version 1)".to_string(),
                "rules/a.toml:1:3: unknown section 'widget'".to_string(),
                "rules/b.txt: unexpected file (only .toml files and README.md are allowed here)".to_string(),
                "start/: missing required folder".to_string(),
                format!("4 error(s) found in {}", folder.display()),
            ]
        );
    }

    #[test]
    fn countdown_counts_from_the_current_turn() {
        assert_eq!(countdown(end(5), Turn::new(5)), "perishes this turn");
        assert_eq!(countdown(end(5), Turn::new(4)), "perishes in 1 turn");
        assert_eq!(countdown(end(5), Turn::new(3)), "perishes in 2 turns");
        assert_eq!(countdown(PerishesAt::Never, Turn::new(3)), "never perishes");
    }

    #[test]
    fn countdown_saturates_for_a_group_already_due() {
        assert_eq!(countdown(end(2), Turn::new(5)), "perishes this turn");
    }

    #[test]
    fn lasts_words_one_turn_many_turns_and_never() {
        assert_eq!(lasts(after(1)), "perishes after 1 turn");
        assert_eq!(lasts(after(3)), "perishes after 3 turns");
        assert_eq!(lasts(Perishes::Never), "never perishes");
    }

    #[test]
    fn deposit_lines_group_by_type_in_resource_order() {
        let world = sample();
        assert_eq!(
            deposit_lines(&world, hex(&world, 0, 0)),
            strings(&[
                "copper_ore × 1: 2.5 unimproved_copper each, 2.5 total",
                "arable_land × 3: 4 unimproved_grain each, 12 total",
            ])
        );
        assert!(deposit_lines(&world, hex(&world, 2, 0)).is_empty());
    }

    #[test]
    fn deposit_lines_on_turns_valid() {
        let world = world_at("crates/engine/tests/settings/turns_valid");
        assert_eq!(
            deposit_lines(&world, hex(&world, 0, 0)),
            strings(&[
                "field × 2: 2 daily each, 4 total",
                "meadow × 1: 0.5 daily each, 0.5 total",
                "pot × 1: 5 stew each, 5 total",
            ])
        );
        assert!(deposit_lines(&world, hex(&world, 2, 0)).is_empty());
    }

    #[test]
    fn stockpile_lines_split_a_good_by_when_it_perishes() {
        let world = turns_valid_at_2();
        assert_eq!(
            stockpile_lines(&world, hex(&world, 0, 0)),
            vec![
                StockLine {
                    good: "4.5 daily".to_string(),
                    parts: strings(&["4.5: perishes this turn"]),
                },
                StockLine {
                    good: "15 stew".to_string(),
                    parts: strings(&[
                        "5: perishes this turn",
                        "5: perishes in 1 turn",
                        "5: perishes in 2 turns",
                    ]),
                },
            ]
        );
        assert_eq!(
            stockpile_lines(&world, hex(&world, 1, 0)),
            vec![
                StockLine {
                    good: "2 daily".to_string(),
                    parts: strings(&["2: perishes this turn"]),
                },
                StockLine {
                    good: "4.5 gold".to_string(),
                    parts: strings(&["4.5: never perishes"]),
                },
            ]
        );
        assert!(stockpile_lines(&world, hex(&world, 2, 0)).is_empty());
    }

    #[test]
    fn stockpile_lines_on_the_sample_at_turn_0() {
        let world = sample();
        assert_eq!(
            stockpile_lines(&world, hex(&world, 0, 0)),
            vec![StockLine {
                good: "4 unimproved_grain".to_string(),
                parts: strings(&["4: perishes this turn"]),
            }]
        );
    }

    #[test]
    fn good_rows_list_every_good_with_properties_and_world_totals() {
        let mut world = sample();
        for steps in 0..2 {
            if steps == 1 {
                for _ in 0..3 {
                    world.step(&mut DeciderHost::idle()).unwrap();
                }
            }
            let rows = good_rows(&world);
            let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
            assert_eq!(
                names,
                [
                    "unskilled_labor",
                    "unimproved_copper",
                    "unimproved_grain",
                    "copper",
                    "grain",
                    "silver",
                    "tools"
                ]
            );
            let in_hexes: Vec<&str> = rows.iter().map(|row| row.in_hexes.as_str()).collect();
            assert_eq!(in_hexes, ["0", "7.5", "16", "0", "0", "0", "0"]);
            // Only silver is held by POPs: it is what they sell their labor for.
            let silver = world.rules().goods.id("silver").unwrap();
            let pop_silver = world
                .pops()
                .iter()
                .fold(Quantity::ZERO, |sum, (id, _)| {
                    sum.saturating_add(world.pop_stockpile(id).total(silver))
                })
                .to_string();
            let in_pops: Vec<&str> = rows.iter().map(|row| row.in_pops.as_str()).collect();
            assert_eq!(&in_pops[..5], ["0", "0", "0", "0", "0"]);
            assert_eq!(in_pops[5], pop_silver);
            assert_eq!(in_pops[6], "0");
            // Businesses hold their owned deposits' output and their silver.
            let in_businesses: Vec<&str> =
                rows.iter().map(|row| row.in_businesses.as_str()).collect();
            if steps == 0 {
                assert_eq!(in_businesses, ["0", "2.5", "8", "0", "0", "350", "0"]);
            } else {
                assert_eq!(in_businesses, business_sums(&world));
            }
            assert_eq!(rows[4].lasts, "perishes after 3 turns");
            assert_eq!(rows[4].movable, "movable");
            assert_eq!(rows[0].movable, "fixed");
            assert_eq!(rows[3].lasts, "never perishes");
        }
    }

    /// The per-good sums over every business stockpile, formatted, in good ID order.
    fn business_sums(world: &World) -> Vec<String> {
        world
            .rules()
            .goods
            .iter()
            .map(|(good, _, _)| {
                world
                    .businesses()
                    .iter()
                    .fold(Quantity::ZERO, |sum, (id, _, _)| {
                        sum.saturating_add(world.business_stockpile(id).total(good))
                    })
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn good_rows_in_businesses_under_the_arithmetic_decider() {
        let (world, _host) = python_world("settings/sample");
        let rows = good_rows(&world);
        let cell = |name: &str| {
            rows.iter()
                .find(|row| row.name == name)
                .unwrap()
                .in_businesses
                .as_str()
        };
        assert_eq!(cell("grain"), "12");
        assert_eq!(cell("copper"), "2.5");
        let in_businesses: Vec<String> = rows.iter().map(|row| row.in_businesses.clone()).collect();
        let sums = business_sums(&world);
        assert_eq!(in_businesses, sums);
        for name in ["silver", "tools"] {
            let index = rows.iter().position(|row| row.name == name).unwrap();
            assert_eq!(in_businesses[index], sums[index]);
        }
    }

    #[test]
    fn good_rows_in_businesses_is_zero_without_businesses() {
        for world in [
            world_at("crates/engine/tests/settings/valid_minimal"),
            turns_valid_at_2(),
        ] {
            assert!(good_rows(&world).iter().all(|row| row.in_businesses == "0"));
        }
    }

    #[test]
    fn good_rows_in_hexes_matches_the_per_hex_totals_on_turns_valid() {
        let world = turns_valid_at_2();
        let expected: Vec<String> = world
            .rules()
            .goods
            .iter()
            .map(|(id, _, _)| {
                world
                    .map()
                    .iter()
                    .fold(Quantity::ZERO, |sum, (hex, _)| {
                        sum.saturating_add(world.hex_stockpile(hex).total(id))
                    })
                    .to_string()
            })
            .collect();
        let rows = good_rows(&world);
        let in_hexes: Vec<String> = rows.iter().map(|row| row.in_hexes.clone()).collect();
        assert_eq!(in_hexes, expected);
        assert!(rows.iter().all(|row| row.in_pops == "0"));
    }

    #[test]
    fn resource_rows_count_deposits_by_type() {
        let world = sample();
        assert_eq!(
            resource_rows(&world),
            vec![
                ResourceRow {
                    name: "copper_ore".to_string(),
                    produces: "unimproved_copper".to_string(),
                    output: "2.5".to_string(),
                    deposits: 4,
                },
                ResourceRow {
                    name: "arable_land".to_string(),
                    produces: "unimproved_grain".to_string(),
                    output: "4".to_string(),
                    deposits: 6,
                },
            ]
        );
        let world = world_at("crates/engine/tests/settings/turns_valid");
        let counts: Vec<(String, usize)> = resource_rows(&world)
            .into_iter()
            .map(|row| (row.name, row.deposits))
            .collect();
        assert_eq!(
            counts,
            vec![
                ("field".to_string(), 3),
                ("meadow".to_string(), 1),
                ("pot".to_string(), 1),
                ("mine".to_string(), 1),
            ]
        );
    }

    /// The POP of the type named `name` on the hex (q, r).
    fn pop_on(world: &World, q: i32, r: i32, name: &str) -> PopId {
        let hex = hex(world, q, r);
        let pop_type = world.rules().pop_types.id(name).expect("pop type exists");
        world
            .pops()
            .of_type_on_hex(hex, pop_type)
            .expect("the POP exists")
    }

    /// The goods table row of the good called `name`.
    fn row_of<'a>(rows: &'a [PopGoodRow], name: &str) -> &'a PopGoodRow {
        rows.iter()
            .find(|row| row.good == name)
            .unwrap_or_else(|| panic!("no row for {name}"))
    }

    /// A goods table row's cells: held, demanded, subsistence, sold, bought, consumed.
    fn cells(row: &PopGoodRow) -> [&str; 6] {
        [
            &row.held,
            &row.demanded,
            &row.subsistence,
            &row.sold,
            &row.bought,
            &row.consumed,
        ]
    }

    #[test]
    fn market_lines_on_the_sample() {
        let world = sample();
        assert_eq!(
            market_lines(&world, hex(&world, 0, 0)),
            strings(&[
                "Currency: silver",
                "unskilled_labor: 1 silver",
                "unimproved_copper: 1 silver",
                "unimproved_grain: 0.5 silver",
                "copper: 5 silver",
                "grain: 2 silver",
                "tools: 12 silver",
            ])
        );
        assert_eq!(
            market_lines(&world, hex(&world, 2, -2)),
            strings(&["Currency: silver", "No market."])
        );
    }

    #[test]
    fn market_lines_on_markets_valid() {
        let world = world_at("crates/engine/tests/settings/markets_valid");
        assert_eq!(
            market_lines(&world, hex(&world, 0, 0)),
            strings(&["Currency: coin", "grain: 2 coin", "fish: 1.5 coin"])
        );
        assert_eq!(
            market_lines(&world, hex(&world, 2, 0)),
            strings(&["Currency: coin", "No prices posted."])
        );
        assert_eq!(
            market_lines(&world, hex(&world, 3, 0)),
            strings(&["Currency: coin", "No market."])
        );
    }

    #[test]
    fn format_index_uses_four_fixed_decimals() {
        assert_eq!(format_index(2.0 / 3.0), "0.6667");
        assert_eq!(format_index(1.0), "1.0000");
        assert_eq!(format_index(0.0), "0.0000");
        assert_eq!(format_index(-0.0), "0.0000");
        assert_eq!(format_index(-1.0), "-1.0000");
    }

    #[test]
    fn format_rate_is_a_signed_percentage() {
        assert_eq!(format_rate(0.026_666_666_666_666_67), "+2.67%");
        assert_eq!(format_rate(0.03), "+3.00%");
        assert_eq!(format_rate(-0.1), "-10.00%");
        assert_eq!(format_rate(0.0), "+0.00%");
        assert_eq!(format_rate(-0.0), "+0.00%");
    }

    #[test]
    fn format_share_rounds_to_four_decimals_and_drops_trailing_zeros() {
        assert_eq!(format_share(0.8), "0.8");
        assert_eq!(format_share(0.1), "0.1");
        assert_eq!(format_share(1.0 / 3.0), "0.3333");
        assert_eq!(format_share(1.0), "1");
        assert_eq!(format_share(0.0), "0");
    }

    #[test]
    fn pop_section_of_the_center_peasant() {
        let world = sample();
        let id = pop_on(&world, 0, 0, "peasant");
        let section = pop_section(&world, id);
        let outcome = world.pop_outcome(id);
        assert_eq!(
            section.heading,
            format!("peasant × {}", world.pops().get(id).size)
        );
        assert_eq!(section.growth, "Started at 60; index 0.6667; rate +2.67%");
        assert_eq!(
            section.demand,
            "Income 600 silver; subsistence cost 360 silver; supernumerary 240 silver"
        );
        // The rows are exactly the demand's goods.
        let goods = &world.rules().goods;
        let names: Vec<&str> = section.goods.iter().map(|row| row.good.as_str()).collect();
        let demanded: Vec<&str> = outcome
            .demand()
            .goods()
            .iter()
            .map(|g| goods.name(g.good))
            .collect();
        assert_eq!(names, demanded);
        let labor = row_of(&section.goods, "unskilled_labor");
        assert_eq!(cells(labor), ["600", "168", "120", "432", "0", "168"]);
        assert!(!labor.below_subsistence);
        let grain = row_of(&section.goods, "grain");
        assert_eq!(cells(grain), ["0", "216", "120", "0", "216", "216"]);
        assert!(!grain.below_subsistence);
        assert!(section.carried_over.starts_with("Carried over: "));
        assert!(section.carried_over.ends_with(" silver"));
        assert!(outcome.carried_over().get() < 1e-6);
        match section.stockpile.as_slice() {
            [] => {}
            [line] => {
                let amount = line.good.strip_suffix(" silver").expect("it is silver");
                assert_eq!(line.parts, vec![format!("{amount}: never perishes")]);
            }
            other => panic!("unexpected stockpile {other:?}"),
        }
    }

    #[test]
    fn pop_section_of_the_center_artisan() {
        let world = sample();
        let id = pop_on(&world, 0, 0, "artisan");
        let section = pop_section(&world, id);
        assert_eq!(
            section.heading,
            format!("artisan × {}", world.pops().get(id).size)
        );
        assert_eq!(section.growth, "Started at 10; index 1.0000; rate +3.00%");
        assert_eq!(
            section.demand,
            "Income 100 silver; subsistence cost 50 silver; supernumerary 50 silver"
        );
        let labor = row_of(&section.goods, "unskilled_labor");
        assert_eq!((labor.sold.as_str(), labor.consumed.as_str()), ("85", "15"));
        let grain = row_of(&section.goods, "grain");
        assert_eq!(
            (grain.bought.as_str(), grain.consumed.as_str()),
            ("30", "30")
        );
        let copper = row_of(&section.goods, "copper");
        assert_eq!(
            (
                copper.bought.as_str(),
                copper.consumed.as_str(),
                copper.subsistence.as_str()
            ),
            ("5", "5", "0")
        );
        assert!(section.goods.iter().all(|row| !row.below_subsistence));
    }

    #[test]
    fn pop_section_of_the_north_east_peasant_flags_the_row_below_subsistence() {
        let world = sample();
        let id = pop_on(&world, 2, -2, "peasant");
        let size = world.pops().get(id).size;
        assert!((size.get() - 12.0 * 0.9).abs() < 1e-9);
        let section = pop_section(&world, id);
        assert_eq!(section.heading, format!("peasant × {size}"));
        assert_eq!(section.growth, "Started at 12; index -1.0000; rate -10.00%");
        assert_eq!(
            section.demand,
            "Income 0 silver; subsistence cost 0 silver; supernumerary 0 silver"
        );
        let labor = row_of(&section.goods, "unskilled_labor");
        assert_eq!(cells(labor), ["120", "120", "24", "0", "0", "120"]);
        assert!(!labor.below_subsistence);
        let grain = row_of(&section.goods, "grain");
        assert_eq!(grain.demanded, "0");
        assert_eq!(grain.subsistence, "24");
        assert!(grain.below_subsistence);
        assert_eq!(section.carried_over, "Carried over: 0 silver");
        assert!(section.stockpile.is_empty());
    }

    #[test]
    fn pop_section_after_a_step_shows_the_size_the_turn_started_with() {
        let mut world = sample();
        let id = pop_on(&world, 2, -2, "peasant");
        let before = world.pops().get(id).size;
        world.step(&mut DeciderHost::idle()).unwrap();
        let section = pop_section(&world, id);
        assert_eq!(section.growth, {
            format!("Started at {before}; index -1.0000; rate -10.00%")
        });
        let after = world.pops().get(id).size;
        assert!((after.get() - 12.0 * 0.9 * 0.9).abs() < 1e-9);
        assert_eq!(section.heading, format!("peasant × {after}"));
    }

    #[test]
    fn pop_section_at_exact_subsistence() {
        let world = world_at("crates/engine/tests/settings/pops_growth");
        let id = world.pops().iter().nth(7).expect("POP 7 exists").0;
        let section = pop_section(&world, id);
        assert_eq!(section.heading, "peasant × 10");
        assert_eq!(section.growth, "Started at 10; index 0.0000; rate +0.00%");
    }

    #[test]
    fn the_growth_hover_has_the_full_values() {
        let world = sample();
        let id = pop_on(&world, 0, 0, "peasant");
        let section = pop_section(&world, id);
        let outcome = world.pop_outcome(id);
        assert_eq!(
            section.growth_hover,
            format!(
                "index {}; rate {}",
                outcome.subsistence_index(),
                outcome.growth_rate()
            )
        );
        // More digits than the growth line's "0.6667" and "+2.67%".
        assert!(outcome.subsistence_index().to_string().len() > "0.6667".len());
        assert!(outcome.growth_rate().to_string().len() > "0.0267".len());
    }

    #[test]
    fn pop_type_rows_on_the_sample() {
        let (world, report) = {
            let setting = load_setting(crate::test_support::repo_path("settings/sample"))
                .expect("the sample loads");
            World::new(setting, &mut DeciderHost::idle()).unwrap()
        };
        let rows = pop_type_rows(&world);
        assert_eq!(rows.len(), 2);

        let size_of = |name: &str| {
            let pop_type = world.rules().pop_types.id(name).unwrap();
            world
                .pops()
                .iter()
                .filter(|(_, pop)| pop.pop_type == pop_type)
                .fold(Quantity::ZERO, |sum, (_, pop)| sum.saturating_add(pop.size))
        };
        let shares = |name: &str| -> Vec<String> {
            let pop_type = world.rules().pop_types.id(name).unwrap();
            world
                .rules()
                .pop_types
                .get(pop_type)
                .utility()
                .iter()
                .map(|u| u.share.to_string())
                .collect()
        };

        let peasant = &rows[0];
        assert_eq!(peasant.name, "peasant");
        assert_eq!(peasant.endowment, strings(&["10 unskilled_labor"]));
        let lines: Vec<&str> = peasant.utility.iter().map(|u| u.line.as_str()).collect();
        assert_eq!(
            lines,
            [
                "unskilled_labor: subsistence 2, share 0.2",
                "grain: subsistence 2, share 0.8"
            ]
        );
        let hovers: Vec<String> = peasant.utility.iter().map(|u| u.share.clone()).collect();
        assert_eq!(hovers, shares("peasant"));
        assert_eq!(
            peasant.growth,
            "growth [0.06, -0.03]; decline 0.1; max 0.05"
        );
        assert_eq!(peasant.pops, 15);
        assert_eq!(peasant.total_size, size_of("peasant").to_string());

        let artisan = &rows[1];
        assert_eq!(artisan.name, "artisan");
        // In good ID order (labor, copper, grain), not the order written in the setting.
        let lines: Vec<&str> = artisan.utility.iter().map(|u| u.line.as_str()).collect();
        assert_eq!(
            lines,
            [
                "unskilled_labor: subsistence 1, share 0.1",
                "copper: subsistence 0, share 0.5",
                "grain: subsistence 2, share 0.4"
            ]
        );
        let hovers: Vec<String> = artisan.utility.iter().map(|u| u.share.clone()).collect();
        assert_eq!(hovers, shares("artisan"));
        assert_eq!(artisan.pops, 1);
        assert_eq!(artisan.total_size, size_of("artisan").to_string());

        let total = size_of("peasant").get() + size_of("artisan").get();
        assert!((total - report.population.get()).abs() < 1e-9);
    }

    #[test]
    fn pop_type_rows_write_an_empty_endowment_as_nothing() {
        let world = world_at("crates/engine/tests/settings/pops_valid");
        let idle = pop_type_rows(&world)
            .into_iter()
            .find(|row| row.name == "idle")
            .expect("idle exists");
        assert_eq!(idle.endowment, strings(&["nothing"]));
        assert_eq!(idle.pops, 0);
        assert_eq!(idle.total_size, "0");
    }

    #[test]
    fn pop_stockpile_lines_are_written_like_a_hexs() {
        let mut world = world_at("crates/engine/tests/settings/pops_placed");
        world.step(&mut DeciderHost::idle()).unwrap();
        let hoarder = world.pops().iter().nth(1).expect("POP 1 exists").0;
        assert_eq!(
            pop_stockpile_lines(&world, hoarder),
            vec![
                StockLine {
                    good: "8 labor".to_string(),
                    parts: strings(&["8: perishes this turn"]),
                },
                StockLine {
                    good: "6 gold".to_string(),
                    parts: strings(&["6: never perishes"]),
                },
            ]
        );
    }
}
