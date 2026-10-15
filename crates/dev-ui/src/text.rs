//! All the text the dev UI shows: the `check` output (load errors, counts, turn lines) and
//! the window's panels. It's built from engine types without egui, so it can be tested. The
//! wording follows the Step 02–08 designs. Amounts are written with `Quantity`'s `Display`,
//! the same way `check --turns` writes them.

use std::path::Path;

use policyspace_engine::{
    GoodId, HexId, LoadErrors, Perishes, PerishesAt, Quantity, Turn, TurnReport, World,
};

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
/// deposits emitted, what POPs were endowed with, and what POPs sold, bought and consumed,
/// each a list or `nothing`, and the total population after growth.
pub fn turn_line(world: &World, report: &TurnReport) -> String {
    format!(
        "Turn {}: perished {}; emitted {}; endowed {}; sold {}; bought {}; consumed {}; population {}",
        report.turn,
        list(world, &report.perished),
        list(world, &report.emitted),
        list(world, &report.endowed),
        list(world, &report.sold),
        list(world, &report.bought),
        list(world, &report.consumed),
        report.population
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
    let rules = world.rules();
    let mut lines = Vec::new();
    for (id, name, resource) in rules.resources.iter() {
        let count = world.deposits().of_type_on_hex(hex, id).count();
        if count == 0 {
            continue;
        }
        // Quantity has no multiply, so the total is the output added `count` times, which
        // is how emission sums it.
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
    let stockpile = world.hex_stockpile(hex);
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
    pub held: String,
}

/// Every good, in good ID order.
pub fn good_rows(world: &World) -> Vec<GoodRow> {
    let goods = &world.rules().goods;
    // Walk the hexes once, adding each stockpile's totals into the good's slot.
    let mut held = vec![Quantity::ZERO; goods.len()];
    for (hex, _) in world.map().iter() {
        for (good, total) in world.hex_stockpile(hex).iter() {
            held[good.index()] = held[good.index()].saturating_add(total);
        }
    }
    goods
        .iter()
        .map(|(id, name, good)| GoodRow {
            name: name.to_string(),
            lasts: lasts(good.perishes),
            movable: if good.movable { "movable" } else { "fixed" },
            held: held[id.index()].to_string(),
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use policyspace_engine::{HexCoord, load_setting};

    use super::*;
    use crate::test_support::world_at;

    /// The sample world at turn 0.
    fn sample() -> World {
        world_at("settings/sample")
    }

    /// The `turns_valid` world stepped to turn 2.
    fn turns_valid_at_2() -> World {
        let mut world = world_at("crates/engine/tests/settings/turns_valid");
        world.step().unwrap();
        world.step().unwrap();
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
            vec![
                StockLine {
                    good: "2.5 unimproved_copper".to_string(),
                    parts: strings(&["2.5: perishes this turn"]),
                },
                StockLine {
                    good: "12 unimproved_grain".to_string(),
                    parts: strings(&["12: perishes this turn"]),
                },
            ]
        );
    }

    #[test]
    fn good_rows_list_every_good_with_properties_and_world_totals() {
        let mut world = sample();
        for steps in 0..2 {
            if steps == 1 {
                for _ in 0..3 {
                    world.step().unwrap();
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
                    "silver"
                ]
            );
            let held: Vec<&str> = rows.iter().map(|row| row.held.as_str()).collect();
            assert_eq!(held, ["0", "10", "24", "0", "0", "0"]);
            assert_eq!(rows[4].lasts, "perishes after 3 turns");
            assert_eq!(rows[4].movable, "movable");
            assert_eq!(rows[0].movable, "fixed");
            assert_eq!(rows[3].lasts, "never perishes");
        }
    }

    #[test]
    fn good_rows_held_matches_the_per_hex_totals_on_turns_valid() {
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
        let held: Vec<String> = good_rows(&world).into_iter().map(|row| row.held).collect();
        assert_eq!(held, expected);
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
}
