//! The text of the business, factory and technology panels: a hex's business sections
//! (decider and answer, deposits, cash, the trade table, the factories table, the trace
//! and the stockpile) and the world panel's Technologies and Businesses tables. Pure
//! functions over engine types, like the rest of `text`, so every line is tested without a
//! window. Nothing is rounded: fractions, conditions, wear and the proportion are f64
//! `Display`, amounts are `Quantity` `Display`.

use std::collections::{BTreeMap, BTreeSet};

use policyspace_engine::{BusinessAnswer, BusinessId, GoodId, Quantity, World, slices_held};

use super::{StockLine, count, deposit_lines_from, list, stock_lines};

/// Remaining life of a factory at `condition` for a technology with `lifetime`:
/// `Some(n)` with `n = slices_held(condition, lifetime).floor() as u32` when at least one
/// slice is held, else `None` (shut down: the engine won't run it).
pub fn remaining_life(condition: f64, lifetime: u32) -> Option<u32> {
    let held = slices_held(condition, lifetime);
    if held >= 1.0 {
        Some(held.floor() as u32)
    } else {
        None
    }
}

/// "shut down" for None, "1 turn" for Some(1), "{n} turns" otherwise.
pub fn format_life(life: Option<u32>) -> String {
    match life {
        None => "shut down".to_string(),
        Some(n) => count(n as usize, "turn", "turns"),
    }
}

/// The owned deposits of `business`, one line per resource type in resource ID order,
/// worded exactly like `deposit_lines` ("{type} × {count}: {output} {good} each, {total}
/// total"); empty if it owns none.
pub fn business_deposit_lines(world: &World, business: BusinessId) -> Vec<String> {
    let mut counts = vec![0usize; world.rules().resources.len()];
    for id in world.deposits().owned_by(business) {
        counts[world.deposits().get(id).resource.index()] += 1;
    }
    deposit_lines_from(world, |id| counts[id.index()])
}

/// `stock_lines(world, world.business_stockpile(business))`: written exactly like a hex's.
pub fn business_stockpile_lines(world: &World, business: BusinessId) -> Vec<StockLine> {
    stock_lines(world, world.business_stockpile(business))
}

/// One row of a business's trade table.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRow {
    /// The good's name (or, in a refused decision, the name as sent).
    pub good: String,
    /// What the decision offered ("0" when nothing).
    pub offered: String,
    /// What was sold.
    pub sold: String,
    /// What the decision bid.
    pub bid: String,
    /// What was bought.
    pub bought: String,
}

/// One row of a business's factories table.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryRow {
    /// The factory's ID number: `id.index() as u32`, as the decider addresses it.
    pub number: u32,
    /// The technology's name.
    pub technology: String,
    /// The condition before this turn's run, wear and maintenance.
    pub condition_before: String,
    /// The condition after them.
    pub condition_after: String,
    /// `format_life(remaining_life(condition_after, lifetime))`.
    pub remaining_life: String,
    /// True if the factory holds less than one lifetime-slice.
    pub shut_down: bool,
    /// The run fraction asked.
    pub run_asked: String,
    /// The run fraction done.
    pub run_done: String,
    /// "{output} {output good}".
    pub output: String,
    /// The condition lost to wear.
    pub wear: String,
    /// The maintenance fraction asked.
    pub maintenance_asked: String,
    /// The maintenance fraction done.
    pub maintenance_done: String,
    /// Hover text on Output: "Inputs: {list}" ("Inputs: nothing" when empty).
    pub inputs: String,
    /// Hover text on Maintenance done: "Maintenance cost: {list}" ("... nothing" when empty).
    pub maintenance_cost: String,
    /// Hover text on Condition after: "Capital: {list}", the technology's capital bundle
    /// scaled by the condition after (every good of the bundle, even at amount 0).
    pub capital: String,
}

/// Everything the hex panel shows for one business.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessSection {
    /// "{name}: {cash} {currency}", e.g. "center_farm: 85 silver".
    pub heading: String,
    /// "Decider: {business.decider}".
    pub decider: String,
    /// "Answer: valid decision", "Answer: refused, {n} error(s)" or "Answer: nothing (no
    /// decision was sent, or its decider failed)".
    pub answer: String,
    /// True if the decision was refused.
    pub refused: bool,
    /// Each validation error's `Display`, in order; empty unless refused.
    pub errors: Vec<String>,
    /// `business_deposit_lines`.
    pub deposits: Vec<String>,
    /// "Cash: {before} {currency} before; {after sales} after sales; {after purchases}
    /// after purchases;".
    pub cash: String,
    /// "proportion {p}" (f64 `Display`: "1", "0.2", "0").
    pub proportion: String,
    /// True if the cash cap cut the bids (`proportion < 1`).
    pub cash_short: bool,
    /// The trade table.
    pub trade: Vec<TradeRow>,
    /// The factories table.
    pub factories: Vec<FactoryRow>,
    /// The decision's trace as sent (valid or refused); None when absent or nothing.
    pub trace: Option<String>,
    /// `business_stockpile_lines`.
    pub stockpile: Vec<StockLine>,
}

/// The amount `map` holds for `name`, f64 `Display`, "0" when it lacks the good.
fn sent(map: &BTreeMap<String, f64>, name: &str) -> String {
    map.get(name).copied().unwrap_or(0.0).to_string()
}

/// The trade table of `business`, merging its decision with what happened.
fn trade_rows(world: &World, business: BusinessId) -> Vec<TradeRow> {
    let goods = &world.rules().goods;
    match world.business_answer(business) {
        BusinessAnswer::Nothing => Vec::new(),
        BusinessAnswer::Valid(_) => {
            let trade = world.business_outcome(business).trade();
            let ids: BTreeSet<GoodId> = trade
                .offered()
                .iter()
                .chain(trade.bid())
                .map(|(good, _)| *good)
                .collect();
            ids.into_iter()
                .map(|good| TradeRow {
                    good: goods.name(good).to_string(),
                    offered: trade.offered_of(good).to_string(),
                    sold: trade.sold_of(good).to_string(),
                    bid: trade.bid_of(good).to_string(),
                    bought: trade.bought_of(good).to_string(),
                })
                .collect()
        }
        BusinessAnswer::Refused { decision, .. } => {
            let names: BTreeSet<&String> =
                decision.offers.keys().chain(decision.bids.keys()).collect();
            // The names the goods list knows come first, in good ID order; any other name
            // (a misspelt good) follows in name order, so it is still shown.
            let mut known: Vec<(GoodId, &String)> = Vec::new();
            let mut unknown: Vec<&String> = Vec::new();
            for name in names {
                match goods.id(name) {
                    Ok(id) => known.push((id, name)),
                    Err(_) => unknown.push(name),
                }
            }
            known.sort_by_key(|(id, _)| *id);
            known
                .into_iter()
                .map(|(_, name)| name)
                .chain(unknown)
                .map(|name| TradeRow {
                    good: name.clone(),
                    offered: sent(&decision.offers, name),
                    sold: "0".to_string(),
                    bid: sent(&decision.bids, name),
                    bought: "0".to_string(),
                })
                .collect()
        }
    }
}

/// The factories table of `business`, one row per factory it owns.
fn factory_rows(world: &World, business: BusinessId) -> Vec<FactoryRow> {
    let rules = world.rules();
    world
        .business_outcome(business)
        .factories()
        .iter()
        .map(|(id, record)| {
            let technology_id = world.factories().get(*id).technology;
            let technology = rules.technologies.get(technology_id);
            let life = remaining_life(record.condition_after, technology.lifetime());
            let capital = technology.capital().scaled(
                Quantity::new(record.condition_after)
                    .expect("a condition is a finite number from 0 to 1"),
            );
            FactoryRow {
                number: id.index() as u32,
                technology: rules.technologies.name(technology_id).to_string(),
                condition_before: record.condition_before.to_string(),
                condition_after: record.condition_after.to_string(),
                remaining_life: format_life(life),
                shut_down: life.is_none(),
                run_asked: record.run.asked.to_string(),
                run_done: record.run.done.to_string(),
                output: format!(
                    "{} {}",
                    record.run.output,
                    rules.goods.name(technology.output())
                ),
                wear: record.wear.to_string(),
                maintenance_asked: record.maintenance.asked.to_string(),
                maintenance_done: record.maintenance.done.to_string(),
                inputs: format!("Inputs: {}", list(world, record.run.inputs.as_slice())),
                maintenance_cost: format!(
                    "Maintenance cost: {}",
                    list(world, record.maintenance.cost.as_slice())
                ),
                capital: format!("Capital: {}", list(world, capital.as_slice())),
            }
        })
        .collect()
}

/// The section for `business` from the current turn's answer and outcome.
pub fn business_section(world: &World, business: BusinessId) -> BusinessSection {
    let record = world.businesses().get(business);
    let goods = &world.rules().goods;
    let currency_id = world.hex_currency(record.hex);
    let currency = goods.name(currency_id);
    let cash = world.business_stockpile(business).total(currency_id);
    let trade = world.business_outcome(business).trade();
    let answer = world.business_answer(business);
    let (answer_line, errors, trace) = match answer {
        BusinessAnswer::Nothing => (
            "Answer: nothing (no decision was sent, or its decider failed)".to_string(),
            Vec::new(),
            None,
        ),
        BusinessAnswer::Valid(decision) => (
            "Answer: valid decision".to_string(),
            Vec::new(),
            decision.trace.clone(),
        ),
        BusinessAnswer::Refused { decision, errors } => (
            format!(
                "Answer: refused, {}",
                count(errors.len(), "error", "errors")
            ),
            errors.iter().map(ToString::to_string).collect(),
            decision.trace.clone(),
        ),
    };
    BusinessSection {
        heading: format!("{}: {cash} {currency}", world.businesses().name(business)),
        decider: format!("Decider: {}", record.decider),
        answer: answer_line,
        refused: matches!(answer, BusinessAnswer::Refused { .. }),
        errors,
        deposits: business_deposit_lines(world, business),
        cash: format!(
            "Cash: {} {currency} before; {} after sales; {} after purchases;",
            trade.cash_before(),
            trade.cash_after_sales(),
            trade.cash_after_purchases()
        ),
        proportion: format!("proportion {}", trade.proportion()),
        cash_short: trade.proportion() < 1.0,
        trade: trade_rows(world, business),
        factories: factory_rows(world, business),
        trace,
        stockpile: business_stockpile_lines(world, business),
    }
}

/// One row of the Technologies table.
#[derive(Debug, Clone, PartialEq)]
pub struct TechnologyRow {
    /// The technology's name.
    pub name: String,
    /// Per input in good ID order: "{amount} {good} per unit"; exactly ["nothing"] when the
    /// inputs are empty.
    pub recipe: Vec<String>,
    /// The output good's name.
    pub output: String,
    /// Capacity per turn.
    pub capacity: String,
    /// Per good: "{amount} {good}" (never empty).
    pub capital: Vec<String>,
    /// Per good: "{amount} {good}"; exactly ["nothing"] when empty.
    pub construction: Vec<String>,
    /// The number of construction turns.
    pub construction_turns: String,
    /// The lifetime in turns.
    pub lifetime: String,
    /// The number of factories running it.
    pub factories: usize,
}

/// Every technology, in technology ID order.
pub fn technology_rows(world: &World) -> Vec<TechnologyRow> {
    let rules = world.rules();
    let mut counts = vec![0usize; rules.technologies.len()];
    for (_, factory) in world.factories().iter() {
        counts[factory.technology.index()] += 1;
    }
    let amounts = |entries: &[(GoodId, Quantity)], suffix: &str| -> Vec<String> {
        if entries.is_empty() {
            return vec!["nothing".to_string()];
        }
        entries
            .iter()
            .map(|(good, amount)| format!("{amount} {}{suffix}", rules.goods.name(*good)))
            .collect()
    };
    rules
        .technologies
        .iter()
        .map(|(id, name, technology)| TechnologyRow {
            name: name.to_string(),
            recipe: amounts(technology.inputs().as_slice(), " per unit"),
            output: rules.goods.name(technology.output()).to_string(),
            capacity: technology.capacity().to_string(),
            capital: amounts(technology.capital().as_slice(), ""),
            construction: amounts(technology.construction().as_slice(), ""),
            construction_turns: technology.construction_turns().to_string(),
            lifetime: technology.lifetime().to_string(),
            factories: counts[id.index()],
        })
        .collect()
}

/// One row of the Businesses table.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessRow {
    /// The business's name.
    pub name: String,
    /// Its hex's coordinates: "(0, 0)".
    pub hex: String,
    /// Its decider's name.
    pub decider: String,
    /// "{total of the currency in its stockpile} {currency}".
    pub cash: String,
    /// The number of factories it owns.
    pub factories: usize,
    /// The state of this turn's answer: "nothing", "valid" or "refused".
    pub answer: &'static str,
    /// True if the answer was refused.
    pub refused: bool,
}

/// Every business, in business ID order.
pub fn business_rows(world: &World) -> Vec<BusinessRow> {
    let goods = &world.rules().goods;
    let mut counts = vec![0usize; world.businesses().len()];
    for (_, factory) in world.factories().iter() {
        counts[factory.owner.index()] += 1;
    }
    world
        .businesses()
        .iter()
        .map(|(id, name, business)| {
            let currency = world.hex_currency(business.hex);
            let answer = world.business_answer(id);
            BusinessRow {
                name: name.to_string(),
                hex: world.map().coord(business.hex).to_string(),
                decider: business.decider.clone(),
                cash: format!(
                    "{} {}",
                    world.business_stockpile(id).total(currency),
                    goods.name(currency)
                ),
                factories: counts[id.index()],
                answer: match answer {
                    BusinessAnswer::Nothing => "nothing",
                    BusinessAnswer::Valid(_) => "valid",
                    BusinessAnswer::Refused { .. } => "refused",
                },
                refused: matches!(answer, BusinessAnswer::Refused { .. }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use policyspace_engine::decider::{ClosureDecider, FactoryOrder};
    use policyspace_engine::{DeciderHost, Decision, FactoryRecord, HexCoord, Reply, Request};

    use super::*;
    use crate::test_support::{python_world, world_at, world_with};

    /// A host whose `rule` decider is `f`.
    fn rule(f: impl FnMut(&Request) -> Reply + 'static) -> DeciderHost {
        DeciderHost::new().with("rule", ClosureDecider::new(f))
    }

    /// A decision for `name` with the given (factory number, run, maintenance) orders, offers
    /// and bids.
    fn decision(
        name: &str,
        factories: &[(u32, f64, f64)],
        offers: &[(&str, f64)],
        bids: &[(&str, f64)],
    ) -> Decision {
        let mut decision = Decision::empty(name);
        for &(id, run, maintenance) in factories {
            decision
                .factories
                .insert(id, FactoryOrder { run, maintenance });
        }
        for &(good, amount) in offers {
            decision.offers.insert(good.to_string(), amount);
        }
        for &(good, amount) in bids {
            decision.bids.insert(good.to_string(), amount);
        }
        decision
    }

    /// The decisions of the engine's `business_trade` tests: `works` runs both its
    /// factories, sells everything it makes and buys the inputs and maintenance goods;
    /// `poor` bids for far more than it can pay; `remote` runs its factory.
    fn steady_decisions() -> Vec<Decision> {
        vec![
            decision(
                "works",
                &[(0, 1.0, 0.25), (1, 1.0, 1.0 / 3.0)],
                &[("iron", 15.0), ("bread", 10.0)],
                &[("ore", 20.0), ("labor", 18.0), ("tools", 2.0)],
            ),
            decision(
                "poor",
                &[(2, 1.0, 0.0)],
                &[],
                &[("labor", 10.0), ("tools", 4.0)],
            ),
            decision("remote", &[(3, 1.0, 0.0)], &[], &[]),
        ]
    }

    /// Answers every request with the steady decision of each business in it.
    fn steady(request: &Request) -> Reply {
        Reply {
            turn: request.turn,
            decisions: steady_decisions()
                .into_iter()
                .filter(|d| request.views.iter().any(|v| v.business == d.business))
                .collect(),
        }
    }

    /// The `business_trade` setting under `host`.
    fn trade_world(host: &mut DeciderHost) -> World {
        world_with("crates/engine/tests/settings/business_trade", host)
    }

    /// The sample at turn 0 under the idle host.
    fn sample_idle() -> World {
        world_at("settings/sample")
    }

    fn business(world: &World, name: &str) -> BusinessId {
        world.businesses().id(name).unwrap()
    }

    fn section(world: &World, name: &str) -> BusinessSection {
        business_section(world, business(world, name))
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    /// The trade table as (good, [offered, sold, bid, bought]).
    fn trade_table(section: &BusinessSection) -> Vec<(&str, [&str; 4])> {
        section
            .trade
            .iter()
            .map(|row| {
                (
                    row.good.as_str(),
                    [&row.offered, &row.sold, &row.bid, &row.bought].map(String::as_str),
                )
            })
            .collect()
    }

    /// The record of the `index`th factory of `name` in the current outcome.
    fn factory_record(world: &World, name: &str, index: usize) -> FactoryRecord {
        world.business_outcome(business(world, name)).factories()[index]
            .1
            .clone()
    }

    #[test]
    fn remaining_life_at_the_boundaries() {
        assert_eq!(remaining_life(1.0, 8), Some(8));
        assert_eq!(remaining_life(0.125, 8), Some(1));
        assert_eq!(remaining_life(0.125 - 1e-6, 8), None);
        assert_eq!(remaining_life(0.0, 8), None);
        assert_eq!(remaining_life(2.0 / 3.0, 3), Some(2));
        assert_eq!(remaining_life(1.0 - 1e-12, 24), Some(24));
        assert_eq!(remaining_life(0.5, 3), Some(1));
        assert_eq!(format_life(None), "shut down");
        assert_eq!(format_life(Some(1)), "1 turn");
        assert_eq!(format_life(Some(8)), "8 turns");
    }

    #[test]
    fn remaining_life_counts_the_further_turns_a_factory_produces() {
        let mut host = rule(|request| Reply {
            turn: request.turn,
            decisions: vec![decision(
                "works",
                &[(0, 1.0, 0.0), (1, 1.0, 0.0)],
                &[("iron", 15.0), ("bread", 10.0)],
                &[("ore", 20.0), ("labor", 18.0)],
            )],
        });
        let mut world = trade_world(&mut host);
        // Each factory's remaining life, whether it is shut down, and the output it made this
        // turn.
        let read = |world: &World| {
            let section = section(world, "works");
            [0, 1].map(|i| {
                let row = &section.factories[i];
                (
                    row.remaining_life.clone(),
                    row.shut_down,
                    factory_record(world, "works", i).run.output.get(),
                )
            })
        };
        // The smelter's lifetime is 4 and the bakery's 3. The life is counted after the
        // turn's wear: the number of further producing turns.
        let smelter_lives = [
            "3 turns",
            "2 turns",
            "1 turn",
            "shut down",
            "shut down",
            "shut down",
        ];
        let bakery_lives = [
            "2 turns",
            "1 turn",
            "shut down",
            "shut down",
            "shut down",
            "shut down",
        ];
        for turn in 0..=5usize {
            if turn > 0 {
                world.step(&mut host).unwrap();
            }
            let [smelter, bakery] = read(&world);
            assert_eq!(smelter.0, smelter_lives[turn], "smelter, turn {turn}");
            assert_eq!(smelter.1, turn >= 3, "smelter, turn {turn}");
            assert_eq!(bakery.0, bakery_lives[turn], "bakery, turn {turn}");
            assert_eq!(bakery.1, turn >= 2, "bakery, turn {turn}");
            if turn > 0 {
                // Turn n produces only if turn n - 1 said at least one further turn.
                assert_eq!(smelter.2 > 0.0, turn <= 3, "smelter output, turn {turn}");
                assert_eq!(bakery.2 > 0.0, turn <= 2, "bakery output, turn {turn}");
            }
        }
    }

    #[test]
    fn the_sample_sections_at_turn_0_under_the_arithmetic_decider() {
        let (world, _host) = python_world("settings/sample");
        let hex = world.map().id(HexCoord::new(0, 0)).unwrap();
        let on_hex = world.businesses().on_hex(hex);
        assert_eq!(on_hex.len(), 3);
        let sections: Vec<BusinessSection> = on_hex
            .iter()
            .map(|&b| business_section(&world, b))
            .collect();
        let headings: Vec<&str> = sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(
            headings,
            [
                "center_farm: 85 silver",
                "center_mine: 37 silver",
                "center_toolmaker: 152.25 silver"
            ]
        );
        for section in &sections {
            assert_eq!(section.decider, "Decider: arithmetic");
            assert_eq!(section.answer, "Answer: valid decision");
            assert!(!section.refused);
            assert!(section.errors.is_empty());
            assert!(section.trace.as_ref().is_some_and(|t| !t.is_empty()));
            assert!(!section.cash_short);
            assert_eq!(section.proportion, "proportion 1");
        }

        let farm = &sections[0];
        assert_eq!(
            farm.deposits,
            strings(&["arable_land × 2: 4 unimproved_grain each, 8 total"])
        );
        assert_eq!(
            farm.cash,
            "Cash: 100 silver before; 100 after sales; 85 after purchases;"
        );
        assert_eq!(
            trade_table(farm),
            [
                ("unskilled_labor", ["0", "0", "7", "7"]),
                ("unimproved_grain", ["0", "0", "4", "4"]),
                ("tools", ["0", "0", "0.5", "0.5"]),
            ]
        );
        assert_eq!(
            farm.factories,
            [FactoryRow {
                number: 0,
                technology: "farm".to_string(),
                condition_before: "1".to_string(),
                condition_after: "1".to_string(),
                remaining_life: "8 turns".to_string(),
                shut_down: false,
                run_asked: "1".to_string(),
                run_done: "1".to_string(),
                output: "12 grain".to_string(),
                wear: "0.125".to_string(),
                maintenance_asked: "0.125".to_string(),
                maintenance_done: "0.125".to_string(),
                inputs: "Inputs: 6 unskilled_labor, 12 unimproved_grain".to_string(),
                maintenance_cost: "Maintenance cost: 1 unskilled_labor, 0.5 tools".to_string(),
                capital: "Capital: 4 tools".to_string(),
            }]
        );
        let goods: Vec<&str> = farm.stockpile.iter().map(|s| s.good.as_str()).collect();
        assert_eq!(goods, ["12 grain", "85 silver"]);
        assert_eq!(farm.stockpile[1].parts, strings(&["85: never perishes"]));

        let mine = &sections[1];
        assert_eq!(
            mine.deposits,
            strings(&["copper_ore × 1: 2.5 unimproved_copper each, 2.5 total"])
        );
        assert_eq!(
            mine.cash,
            "Cash: 50 silver before; 50 after sales; 37 after purchases;"
        );
        let table = trade_table(mine);
        assert!(
            table.contains(&("unskilled_labor", ["0", "0", "4", "4"])),
            "{table:?}"
        );
        assert!(
            table.contains(&("tools", ["0", "0", "0.75", "0.75"])),
            "{table:?}"
        );
        let factory = &mine.factories[0];
        assert_eq!(factory.number, 1);
        assert_eq!(factory.technology, "mine");
        assert_eq!(factory.condition_before, "0.75");
        assert_eq!(factory.condition_after, "1");
        assert_eq!(factory.remaining_life, "8 turns");
        assert_eq!(factory.output, "2.5 copper");
        assert_eq!(factory.wear, "0.125");
        assert_eq!(factory.maintenance_asked, "0.375");
        assert_eq!(factory.maintenance_done, "0.375");

        let toolmaker = &sections[2];
        assert!(toolmaker.deposits.is_empty());
        assert_eq!(
            toolmaker.cash,
            "Cash: 200 silver before; 200 after sales; 152.25 after purchases;"
        );
        let table = trade_table(toolmaker);
        assert!(
            table.contains(&("copper", ["0", "0", "8", "8"])),
            "{table:?}"
        );
        assert!(
            table.contains(&("unskilled_labor", ["0", "0", "6.25", "6.25"])),
            "{table:?}"
        );
        // The tools amount is the arithmetic decider's float arithmetic (about 0.125, not
        // exactly), so it is compared with the engine's own value.
        let outcome = world.business_outcome(business(&world, "center_toolmaker"));
        let tools = world.rules().goods.id("tools").unwrap();
        let bid = outcome.trade().bid_of(tools).to_string();
        let bought = outcome.trade().bought_of(tools).to_string();
        assert!(bid.starts_with("0.12"), "{bid}");
        assert!(
            table.contains(&("tools", ["0", "0", bid.as_str(), bought.as_str()])),
            "{table:?}"
        );
        let record = factory_record(&world, "center_toolmaker", 0);
        let factory = &toolmaker.factories[0];
        assert_eq!(factory.number, 2);
        assert_eq!(factory.technology, "toolmaker");
        assert_eq!(factory.condition_before, "1");
        assert_eq!(factory.condition_after, "1");
        assert_eq!(factory.remaining_life, "24 turns");
        assert_eq!(factory.output, "4 tools");
        assert_eq!(factory.wear, record.wear.to_string());
        assert_eq!(
            factory.maintenance_asked,
            record.maintenance.asked.to_string()
        );
        assert_eq!(
            factory.maintenance_done,
            record.maintenance.done.to_string()
        );
    }

    #[test]
    fn the_sample_sections_after_one_step() {
        let (mut world, mut host) = python_world("settings/sample");
        world.step(&mut host).unwrap();
        let farm = section(&world, "center_farm");
        let mine = section(&world, "center_mine");
        let toolmaker = section(&world, "center_toolmaker");
        let offered_and_sold = |s: &BusinessSection, good: &str| {
            s.trade
                .iter()
                .find(|row| row.good == good)
                .map(|row| (row.offered.clone(), row.sold.clone()))
                .unwrap_or_else(|| panic!("no {good} row in {:?}", s.trade))
        };
        let pair = |amount: &str| (amount.to_string(), amount.to_string());
        assert_eq!(offered_and_sold(&farm, "grain"), pair("12"));
        assert_eq!(offered_and_sold(&mine, "copper"), pair("2.5"));
        assert_eq!(offered_and_sold(&toolmaker, "tools"), pair("3.875"));
        assert!(farm.cash.ends_with("94 after purchases;"), "{}", farm.cash);
        assert!(
            mine.cash.ends_with("43.5 after purchases;"),
            "{}",
            mine.cash
        );
        assert!(
            toolmaker.cash.ends_with("152.5 after purchases;"),
            "{}",
            toolmaker.cash
        );
        assert_eq!(farm.heading, "center_farm: 94 silver");
        assert_eq!(mine.heading, "center_mine: 43.5 silver");
        assert_eq!(toolmaker.heading, "center_toolmaker: 152.5 silver");
    }

    #[test]
    fn the_sample_sections_under_the_idle_host() {
        let world = sample_idle();
        let farm = section(&world, "center_farm");
        let mine = section(&world, "center_mine");
        let toolmaker = section(&world, "center_toolmaker");
        assert_eq!(farm.heading, "center_farm: 100 silver");
        assert_eq!(mine.heading, "center_mine: 50 silver");
        assert_eq!(toolmaker.heading, "center_toolmaker: 200 silver");
        for section in [&farm, &mine, &toolmaker] {
            assert_eq!(
                section.answer,
                "Answer: nothing (no decision was sent, or its decider failed)"
            );
            assert!(!section.refused);
            assert!(section.errors.is_empty());
            assert_eq!(section.trace, None);
            assert!(section.trade.is_empty());
            assert!(!section.cash_short);
            assert_eq!(section.proportion, "proportion 1");
            let factory = &section.factories[0];
            assert_eq!(factory.run_asked, "0");
            assert_eq!(factory.run_done, "0");
            assert_eq!(factory.maintenance_asked, "0");
            assert_eq!(factory.maintenance_done, "0");
            assert_eq!(factory.inputs, "Inputs: nothing");
            assert_eq!(factory.maintenance_cost, "Maintenance cost: nothing");
        }
        assert_eq!(
            farm.cash,
            "Cash: 100 silver before; 100 after sales; 100 after purchases;"
        );
        assert_eq!(
            mine.cash,
            "Cash: 50 silver before; 50 after sales; 50 after purchases;"
        );
        assert_eq!(
            toolmaker.cash,
            "Cash: 200 silver before; 200 after sales; 200 after purchases;"
        );
        assert_eq!(farm.factories[0].output, "0 grain");
        assert_eq!(mine.factories[0].output, "0 copper");
        assert_eq!(toolmaker.factories[0].output, "0 tools");
        for (name, section, life) in [
            ("center_farm", &farm, "7 turns"),
            ("center_mine", &mine, "5 turns"),
            ("center_toolmaker", &toolmaker, "23 turns"),
        ] {
            let record = factory_record(&world, name, 0);
            let factory = &section.factories[0];
            assert_eq!(factory.wear, record.wear.to_string());
            assert_eq!(
                factory.condition_before,
                record.condition_before.to_string()
            );
            assert_eq!(factory.condition_after, record.condition_after.to_string());
            assert_eq!(factory.remaining_life, life, "{name}");
        }
        assert_eq!(farm.factories[0].wear, "0.125");
        assert_eq!(farm.factories[0].condition_after, "0.875");
        let goods: Vec<&str> = farm.stockpile.iter().map(|s| s.good.as_str()).collect();
        assert_eq!(goods, ["8 unimproved_grain", "100 silver"]);
    }

    #[test]
    fn a_refused_decision_shows_its_errors_and_what_was_asked() {
        let mut host = rule(|request| {
            let mut reply = steady(request);
            for decision in &mut reply.decisions {
                if decision.business == "poor" {
                    decision.factories.insert(
                        99,
                        FactoryOrder {
                            run: 1.0,
                            maintenance: 0.0,
                        },
                    );
                }
            }
            reply
        });
        let world = trade_world(&mut host);
        let poor = section(&world, "poor");
        assert!(poor.refused);
        assert_eq!(poor.answer, "Answer: refused, 1 error");
        let BusinessAnswer::Refused { errors, .. } =
            world.business_answer(business(&world, "poor"))
        else {
            panic!("poor should be refused");
        };
        let expected: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(poor.errors, expected);
        assert_eq!(poor.errors.len(), 1);
        assert!(
            poor.errors[0].starts_with("factory #99: "),
            "{:?}",
            poor.errors
        );
        assert_eq!(
            trade_table(&poor),
            [
                ("labor", ["0", "0", "10", "0"]),
                ("tools", ["0", "0", "4", "0"])
            ]
        );
        let factory = &poor.factories[0];
        assert_eq!(factory.number, 2);
        assert_eq!(factory.run_asked, "0");
        assert_eq!(factory.run_done, "0");
        assert_eq!(factory.maintenance_asked, "0");
        assert_eq!(factory.maintenance_done, "0");
        assert_eq!(factory.wear, "0.25");
        assert_eq!(factory.condition_before, "1");
        assert_eq!(factory.condition_after, "0.75");
        assert_eq!(factory.remaining_life, "3 turns");
        assert_eq!(poor.trace, None);

        let rows = business_rows(&world);
        let find = |name: &str| rows.iter().find(|row| row.name == name).unwrap();
        assert_eq!(find("poor").answer, "refused");
        assert!(find("poor").refused);
        assert_eq!(find("works").answer, "valid");
        assert!(!find("works").refused);
    }

    #[test]
    fn a_refused_decision_shows_a_misspelt_good_and_its_trace() {
        let mut host = rule(|request| {
            let mut reply = steady(request);
            for decision in &mut reply.decisions {
                if decision.business == "poor" {
                    decision.bids.insert("lbor".to_string(), 3.5);
                    decision.offers.insert("iron".to_string(), 1.0);
                    decision.trace = Some("line one\nline two".to_string());
                }
            }
            reply
        });
        let world = trade_world(&mut host);
        let poor = section(&world, "poor");
        assert!(poor.refused);
        assert_eq!(
            trade_table(&poor),
            [
                ("labor", ["0", "0", "10", "0"]),
                ("iron", ["1", "0", "0", "0"]),
                ("tools", ["0", "0", "4", "0"]),
                ("lbor", ["0", "0", "3.5", "0"]),
            ]
        );
        assert_eq!(poor.trace.as_deref(), Some("line one\nline two"));
    }

    #[test]
    fn a_cash_short_decision_shows_the_proportion_and_the_cut_run() {
        let world = trade_world(&mut rule(steady));
        let poor = section(&world, "poor");
        assert_eq!(
            poor.cash,
            "Cash: 10 coin before; 10 after sales; 0 after purchases;"
        );
        assert_eq!(poor.proportion, "proportion 0.2");
        assert!(poor.cash_short);
        assert_eq!(
            trade_table(&poor),
            [
                ("labor", ["0", "0", "10", "2"]),
                ("tools", ["0", "0", "4", "0.8"])
            ]
        );
        let factory = &poor.factories[0];
        assert_eq!(factory.run_asked, "1");
        assert_eq!(factory.run_done, "0.2");
        assert_eq!(factory.output, "2 iron");
        assert_eq!(factory.inputs, "Inputs: 2 labor, 4 ore");
        let works = section(&world, "works");
        assert!(!works.cash_short);
        assert_eq!(works.proportion, "proportion 1");
    }

    #[test]
    fn business_deposit_lines_and_stockpile_lines() {
        let world = sample_idle();
        let id = |name: &str| business(&world, name);
        assert_eq!(
            business_deposit_lines(&world, id("center_farm")),
            strings(&["arable_land × 2: 4 unimproved_grain each, 8 total"])
        );
        assert_eq!(
            business_deposit_lines(&world, id("center_mine")),
            strings(&["copper_ore × 1: 2.5 unimproved_copper each, 2.5 total"])
        );
        assert!(business_deposit_lines(&world, id("center_toolmaker")).is_empty());
        assert_eq!(
            business_stockpile_lines(&world, id("center_mine")),
            [
                StockLine {
                    good: "2.5 unimproved_copper".to_string(),
                    parts: strings(&["2.5: perishes this turn"]),
                },
                StockLine {
                    good: "50 silver".to_string(),
                    parts: strings(&["50: never perishes"]),
                },
            ]
        );
    }

    #[test]
    fn technology_rows_on_the_sample() {
        let world = sample_idle();
        assert_eq!(
            technology_rows(&world),
            [
                TechnologyRow {
                    name: "farm".to_string(),
                    recipe: strings(&[
                        "0.5 unskilled_labor per unit",
                        "1 unimproved_grain per unit"
                    ]),
                    output: "grain".to_string(),
                    capacity: "12".to_string(),
                    capital: strings(&["4 tools"]),
                    construction: strings(&["8 unskilled_labor"]),
                    construction_turns: "2".to_string(),
                    lifetime: "8".to_string(),
                    factories: 1,
                },
                TechnologyRow {
                    name: "mine".to_string(),
                    recipe: strings(&[
                        "1 unskilled_labor per unit",
                        "1 unimproved_copper per unit"
                    ]),
                    output: "copper".to_string(),
                    capacity: "2.5".to_string(),
                    capital: strings(&["2 tools"]),
                    construction: strings(&["4 unskilled_labor"]),
                    construction_turns: "2".to_string(),
                    lifetime: "8".to_string(),
                    factories: 1,
                },
                TechnologyRow {
                    name: "toolmaker".to_string(),
                    recipe: strings(&["1.5 unskilled_labor per unit", "2 copper per unit"]),
                    output: "tools".to_string(),
                    capacity: "4".to_string(),
                    capital: strings(&["3 tools"]),
                    construction: strings(&["6 unskilled_labor"]),
                    construction_turns: "3".to_string(),
                    lifetime: "24".to_string(),
                    factories: 1,
                },
            ]
        );
        let world = trade_world(&mut DeciderHost::idle());
        let rows = technology_rows(&world);
        let bakery = rows.iter().find(|row| row.name == "bakery").unwrap();
        assert_eq!(bakery.construction, strings(&["nothing"]));
        assert_eq!(bakery.recipe, strings(&["1 labor per unit"]));
        let smelter = rows.iter().find(|row| row.name == "smelter").unwrap();
        assert_eq!(smelter.factories, 2);
        assert!(
            technology_rows(&world_at("crates/engine/tests/settings/valid_minimal")).is_empty()
        );
    }

    #[test]
    fn business_rows_on_the_sample() {
        let (world, _host) = python_world("settings/sample");
        let rows = business_rows(&world);
        let expected = [
            ("center_farm", "85 silver"),
            ("center_mine", "37 silver"),
            ("center_toolmaker", "152.25 silver"),
        ];
        assert_eq!(rows.len(), 3);
        for (row, (name, cash)) in rows.iter().zip(expected) {
            assert_eq!(row.name, name);
            assert_eq!(row.hex, "(0, 0)");
            assert_eq!(row.decider, "arithmetic");
            assert_eq!(row.cash, cash);
            assert_eq!(row.factories, 1);
            assert_eq!(row.answer, "valid");
            assert!(!row.refused);
        }

        let idle = business_rows(&sample_idle());
        let cash: Vec<&str> = idle.iter().map(|row| row.cash.as_str()).collect();
        assert_eq!(cash, ["100 silver", "50 silver", "200 silver"]);
        assert!(
            idle.iter()
                .all(|row| row.answer == "nothing" && !row.refused)
        );

        assert!(business_rows(&world_at("crates/engine/tests/settings/valid_minimal")).is_empty());
    }
}
