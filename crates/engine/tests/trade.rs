//! Integration tests for the POP trade round: each POP's demand, sales, purchases and
//! consumption every turn, through the engine's public API only.

mod common;

use std::path::Path;

use common::{load, sample_setting, test_setting};
use policyspace_engine::{
    GoodId, HexCoord, HexId, PopId, PopOutcome, Quantity, Turn, TurnReport, World,
};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn world_of(path: &Path) -> (World, TurnReport) {
    World::new(load(path))
}

fn hex(world: &World, q: i32, r: i32) -> HexId {
    world
        .map()
        .id(HexCoord { q, r })
        .unwrap_or_else(|| panic!("no hex at ({q}, {r})"))
}

fn good(world: &World, name: &str) -> GoodId {
    world.rules().goods.id(name).unwrap()
}

fn pop_id(index: usize, world: &World) -> PopId {
    world
        .pops()
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no POP {index}"))
}

fn outcome(world: &World, index: usize) -> &PopOutcome {
    world.pop_outcome(pop_id(index, world))
}

/// A list of (good, amount) as (good name, amount).
fn list(world: &World, entries: &[(GoodId, Quantity)]) -> Vec<(String, f64)> {
    entries
        .iter()
        .map(|(g, a)| (world.rules().goods.name(*g).to_string(), a.get()))
        .collect()
}

/// Asserts a list of (good, amount) holds exactly these goods, in this order, with amounts
/// within 1e-9.
fn assert_list(world: &World, actual: &[(GoodId, Quantity)], expected: &[(&str, f64)]) {
    let actual = list(world, actual);
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for ((an, aq), (en, eq)) in actual.iter().zip(expected) {
        assert_eq!(an, en, "{actual:?} vs {expected:?}");
        assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
    }
}

fn total(world: &World, pop: usize, name: &str) -> f64 {
    world
        .pop_stockpile(pop_id(pop, world))
        .total(good(world, name))
        .get()
}

/// Asserts the POP's stockpile holds nothing but a sliver of silver.
fn assert_only_a_silver_sliver(world: &World, pop: usize) {
    for (g, amount) in world.pop_stockpile(pop_id(pop, world)).iter() {
        assert!(
            world.rules().goods.name(g) == "silver" && amount.get() < 1e-9,
            "POP {pop} holds {} {}",
            amount,
            world.rules().goods.name(g)
        );
    }
}

// --- The sample setting ---

#[test]
fn a_fresh_sample_world_has_traded_turn_0() {
    let (world, _) = world_of(&sample_setting());
    let no_market = hex(&world, 2, -2);
    let labor = good(&world, "unskilled_labor");
    let types = &world.rules().pop_types;
    let artisan = types.id("artisan").unwrap();
    let mut peasants = 0;
    for (id, pop) in world.pops().iter() {
        let o = world.pop_outcome(id);
        // The POP has grown since the round; the round was for its starting size.
        let n = o.starting_size().get();
        let i = id.index();
        if pop.hex == no_market {
            continue;
        }
        assert!(world.pop_stockpile(id).total(labor) == Quantity::ZERO);
        assert_only_a_silver_sliver(&world, i);
        if pop.pop_type == artisan {
            assert_eq!(n, 10.0);
            assert_eq!(i, 8);
            assert_list(&world, o.sold(), &[("unskilled_labor", 85.0)]);
            assert_list(&world, o.bought(), &[("copper", 5.0), ("grain", 30.0)]);
            assert_list(
                &world,
                o.consumed(),
                &[("unskilled_labor", 15.0), ("copper", 5.0), ("grain", 30.0)],
            );
            continue;
        }
        peasants += 1;
        assert_list(&world, o.sold(), &[("unskilled_labor", 7.2 * n)]);
        assert_list(&world, o.bought(), &[("grain", 3.6 * n)]);
        assert_list(
            &world,
            o.consumed(),
            &[("unskilled_labor", 2.8 * n), ("grain", 3.6 * n)],
        );
        assert!(close(o.demand().income().get(), 10.0 * n), "POP {i}");
        assert!(
            close(o.demand().subsistence_cost().get(), 6.0 * n),
            "POP {i}"
        );
    }
    assert_eq!(peasants, 14);
}

#[test]
fn the_no_market_peasant_consumes_its_labor_and_goes_without_grain() {
    let (world, _) = world_of(&sample_setting());
    let pop = pop_id(2, &world);
    let o = world.pop_outcome(pop);
    assert_eq!(o.starting_size().get(), 12.0);
    assert!(o.sold().is_empty());
    assert!(o.bought().is_empty());
    assert_list(&world, o.consumed(), &[("unskilled_labor", 120.0)]);
    let grain = o.demand().get(good(&world, "grain")).unwrap();
    assert_eq!(grain.quantity, Quantity::ZERO);
    assert!(grain.below_subsistence());
    let labor = o.demand().get(good(&world, "unskilled_labor")).unwrap();
    assert_eq!(labor.quantity.get(), 120.0);
    assert!(world.pop_stockpile(pop).is_empty());
}

#[test]
fn the_sample_report_totals_pops_in_good_id_order() {
    let (mut world, first) = world_of(&sample_setting());
    let sold = [("unskilled_labor", 2878.6)];
    let bought = [("copper", 5.0), ("grain", 1426.8)];
    let consumed = [
        ("unskilled_labor", 1221.4),
        ("copper", 5.0),
        ("grain", 1426.8),
    ];
    assert_list(&world, &first.sold, &sold);
    assert_list(&world, &first.bought, &bought);
    assert_list(&world, &first.consumed, &consumed);

    // Turn 1 follows the sizes after turn 0's growth: s is the sum of the market-hex
    // peasants' sizes, a the artisan's, n the no-market peasant's.
    let size_of = |index: usize| world.pops().get(pop_id(index, &world)).size.get();
    let (a, n) = (size_of(8), size_of(2));
    let s: f64 = (0..16).filter(|i| *i != 2 && *i != 8).map(size_of).sum();
    let second = world.step().unwrap();
    assert_list(
        &world,
        &second.sold,
        &[("unskilled_labor", 7.2 * s + 8.5 * a)],
    );
    assert_list(
        &world,
        &second.bought,
        &[("copper", 0.5 * a), ("grain", 3.6 * s + 3.0 * a)],
    );
    assert_list(
        &world,
        &second.consumed,
        &[
            ("unskilled_labor", 2.8 * s + 1.5 * a + 10.0 * n),
            ("copper", 0.5 * a),
            ("grain", 3.6 * s + 3.0 * a),
        ],
    );
    let perished = list(&world, &second.perished);
    for (name, amount) in &perished {
        assert!(
            *amount < 1e-9 || name == "unimproved_copper" || name == "unimproved_grain",
            "{perished:?}"
        );
    }
    let big: Vec<_> = perished.iter().filter(|(_, a)| *a >= 1e-9).collect();
    assert_eq!(big.len(), 2, "{perished:?}");
    assert_eq!(big[0], &("unimproved_copper".to_string(), 10.0));
    assert_eq!(big[1], &("unimproved_grain".to_string(), 24.0));
}

#[test]
fn stepping_repeats_the_outcomes_per_individual_and_two_worlds_agree() {
    let (mut world, _) = world_of(&sample_setting());
    let (mut other, _) = world_of(&sample_setting());
    let turn_0: Vec<PopOutcome> = world
        .pops()
        .iter()
        .map(|(id, _)| world.pop_outcome(id).clone())
        .collect();
    for _ in 0..3 {
        assert_eq!(world.step().unwrap(), other.step().unwrap());
        for (id, _) in world.pops().iter() {
            assert_eq!(world.pop_outcome(id), other.pop_outcome(id));
            assert_eq!(world.pop_stockpile(id), other.pop_stockpile(id));
            // Per individual: divide by the size each round was for.
            let (now, then) = (world.pop_outcome(id), &turn_0[id.index()]);
            let (now_n, then_n) = (now.starting_size().get(), then.starting_size().get());
            for (a, b) in [
                (now.sold(), then.sold()),
                (now.bought(), then.bought()),
                (now.consumed(), then.consumed()),
            ] {
                assert_eq!(a.len(), b.len());
                for ((ag, aq), (bg, bq)) in a.iter().zip(b) {
                    assert_eq!(ag, bg);
                    assert!(close(aq.get() / now_n, bq.get() / then_n));
                }
            }
            assert!(close(
                now.demand().income().get() / now_n,
                then.demand().income().get() / then_n
            ));
        }
    }
    assert_eq!(world.turn(), Turn::new(3));
}

// --- The pops_trade setting ---

fn trade_world() -> (World, TurnReport) {
    world_of(&test_setting("pops_trade"))
}

/// Checks every row of the expected table and the report totals at turn `t`.
fn check_table(world: &World, report: &TurnReport, t: u64) {
    let tf = t as f64;
    let c = |i: usize| outcome(world, i);

    // 0: peasant x2 on (0, 0).
    assert_list(world, c(0).sold(), &[("labor", 14.4)]);
    assert_list(world, c(0).bought(), &[("grain", 7.2)]);
    assert_list(world, c(0).consumed(), &[("labor", 5.6), ("grain", 7.2)]);
    assert!(close(
        c(0).demand().quantity_of(good(world, "labor")).get(),
        5.6
    ));
    assert!(close(
        c(0).demand().quantity_of(good(world, "grain")).get(),
        7.2
    ));
    assert!(close(c(0).demand().income().get(), 20.0));
    assert!(close(c(0).demand().subsistence_cost().get(), 12.0));
    assert_only_a_silver_sliver(world, 0);

    // 1: artisan on (0, 0).
    assert_list(world, c(1).sold(), &[("labor", 8.5)]);
    assert_list(world, c(1).bought(), &[("grain", 3.0), ("copper", 0.5)]);
    assert_list(
        world,
        c(1).consumed(),
        &[("labor", 1.5), ("grain", 3.0), ("copper", 0.5)],
    );
    assert!(close(c(1).demand().income().get(), 10.0));
    assert!(close(c(1).demand().subsistence_cost().get(), 5.0));
    assert_only_a_silver_sliver(world, 1);

    // 2: peasant where grain costs 10, below subsistence.
    assert_list(world, c(2).sold(), &[("labor", 100.0 / 11.0)]);
    assert_list(world, c(2).bought(), &[("grain", 10.0 / 11.0)]);
    assert_list(
        world,
        c(2).consumed(),
        &[("labor", 10.0 / 11.0), ("grain", 10.0 / 11.0)],
    );
    assert!(close(c(2).demand().income().get(), 10.0));
    assert!(close(c(2).demand().subsistence_cost().get(), 22.0));
    assert!(c(2).demand().below_subsistence());
    assert_only_a_silver_sliver(world, 2);

    // 3: idle, holds nothing.
    assert!(c(3).sold().is_empty() && c(3).bought().is_empty() && c(3).consumed().is_empty());
    assert_eq!(c(3).demand().income(), Quantity::ZERO);
    assert_eq!(c(3).demand().subsistence_cost(), Quantity::ZERO);
    assert_eq!(
        c(3).demand().quantity_of(good(world, "grain")),
        Quantity::ZERO
    );
    assert!(world.pop_stockpile(pop_id(3, world)).is_empty());

    // 4: peasant with no market keeps its labor and consumes it.
    assert!(c(4).sold().is_empty() && c(4).bought().is_empty());
    assert_list(world, c(4).consumed(), &[("labor", 10.0)]);
    assert!(close(
        c(4).demand().quantity_of(good(world, "labor")).get(),
        10.0
    ));
    assert_eq!(
        c(4).demand().quantity_of(good(world, "grain")),
        Quantity::ZERO
    );
    assert!(c(4).demand().below_subsistence());
    assert_eq!(c(4).demand().income(), Quantity::ZERO);
    assert_eq!(c(4).demand().subsistence_cost(), Quantity::ZERO);
    assert!(world.pop_stockpile(pop_id(4, world)).is_empty());

    // 5: saver, whose silver endowment adds to its income.
    assert_list(world, c(5).sold(), &[("labor", 6.2)]);
    assert_list(world, c(5).bought(), &[("grain", 5.6)]);
    assert_list(world, c(5).consumed(), &[("labor", 3.8), ("grain", 5.6)]);
    assert!(close(c(5).demand().income().get(), 15.0));
    assert!(close(c(5).demand().subsistence_cost().get(), 6.0));
    assert_only_a_silver_sliver(world, 5);

    // 6: miser, which parks the surplus as silver.
    assert_list(world, c(6).sold(), &[("labor", 8.0)]);
    assert!(c(6).bought().is_empty());
    assert_list(world, c(6).consumed(), &[("labor", 2.0)]);
    assert!(close(c(6).demand().income().get(), 10.0 + 8.0 * tf));
    assert!(close(c(6).demand().subsistence_cost().get(), 2.0));
    assert!(close(
        c(6).demand().quantity_of(good(world, "silver")).get(),
        8.0 * (tf + 1.0)
    ));
    assert_eq!(
        c(6).demand().quantity_of(good(world, "copper")),
        Quantity::ZERO
    );
    assert_eq!(total(world, 6, "silver"), 8.0 * (tf + 1.0));
    assert_eq!(c(6).carried_over().get(), 8.0 * (tf + 1.0));
    assert_eq!(world.pop_stockpile(pop_id(6, world)).iter().count(), 1);

    // 7: collector, whose gem is not traded and piles up.
    assert_list(world, c(7).sold(), &[("labor", 7.2)]);
    assert_list(world, c(7).bought(), &[("grain", 3.6)]);
    assert_list(world, c(7).consumed(), &[("labor", 2.8), ("grain", 3.6)]);
    assert!(close(c(7).demand().income().get(), 10.0));
    assert!(close(c(7).demand().subsistence_cost().get(), 6.0));
    assert_eq!(
        c(7).demand().quantity_of(good(world, "gem")).get(),
        tf + 1.0
    );
    assert_eq!(total(world, 7, "gem"), tf + 1.0);
    for (g, amount) in world.pop_stockpile(pop_id(7, world)).iter() {
        let name = world.rules().goods.name(g);
        assert!(
            name == "gem" || (name == "silver" && amount.get() < 1e-9),
            "collector holds {amount} {name}"
        );
    }

    // The report totals.
    assert_list(world, &report.sold, &[("labor", 44.3 + 100.0 / 11.0)]);
    assert_list(
        world,
        &report.bought,
        &[("grain", 19.4 + 10.0 / 11.0), ("copper", 0.5)],
    );
    assert_list(
        world,
        &report.consumed,
        &[
            ("labor", 25.7 + 10.0 / 11.0),
            ("grain", 19.4 + 10.0 / 11.0),
            ("copper", 0.5),
        ],
    );
    assert_eq!(report.turn, Turn::new(t));
    if t > 0 {
        assert!(
            report.perished.iter().all(|(_, a)| a.get() < 1e-9),
            "{:?}",
            list(world, &report.perished)
        );
    }
}

#[test]
fn the_pops_trade_setting_matches_the_table() {
    let (mut world, report) = trade_world();
    assert_eq!(world.pops().len(), 8);
    check_table(&world, &report, 0);
    let report = world.step().unwrap();
    check_table(&world, &report, 1);
    let report = world.step().unwrap();
    check_table(&world, &report, 2);
}

#[test]
fn currency_carried_over_is_income_next_turn() {
    let (mut world, _) = trade_world();
    let silver = good(&world, "silver");
    let grain = good(&world, "grain");
    let miser = outcome(&world, 6);
    assert_eq!(miser.carried_over().get(), 8.0);
    assert!(close(miser.demand().income().get(), 10.0));
    assert!(close(outcome(&world, 5).demand().income().get(), 15.0));

    world.step().unwrap();
    let miser = outcome(&world, 6);
    assert!(close(miser.demand().income().get(), 18.0));
    assert!(close(miser.demand().quantity_of(silver).get(), 16.0));
    assert_eq!(miser.carried_over().get(), 16.0);
    assert_list(&world, miser.sold(), &[("labor", 8.0)]);
    let saver = outcome(&world, 5);
    assert!(close(saver.demand().income().get(), 15.0));
    assert!(close(saver.bought_of(grain).get(), 5.6));
}

#[test]
fn a_below_subsistence_pop_scales_the_bundle() {
    let (world, _) = trade_world();
    let (labor, grain) = (good(&world, "labor"), good(&world, "grain"));
    let o = outcome(&world, 2);
    assert!(close(o.sold_of(labor).get(), 100.0 / 11.0));
    assert!(close(o.bought_of(grain).get(), 10.0 / 11.0));
    assert!(close(o.consumed_of(labor).get(), 10.0 / 11.0));
    assert!(close(o.consumed_of(grain).get(), 10.0 / 11.0));
    assert!(o.demand().below_subsistence());
}

#[test]
fn held_goods_outside_the_utility_stay_or_sell() {
    let (mut world, _) = trade_world();
    let (gem, silver) = (good(&world, "gem"), good(&world, "silver"));
    for t in 0..2u64 {
        let collector = outcome(&world, 7);
        assert_eq!(collector.sold_of(gem), Quantity::ZERO);
        assert_eq!(collector.consumed_of(gem), Quantity::ZERO);
        assert_eq!(total(&world, 7, "gem"), (t + 1) as f64);
        let miser = outcome(&world, 6);
        assert_eq!(miser.consumed_of(silver), Quantity::ZERO);
        assert_eq!(miser.sold_of(silver), Quantity::ZERO);
        world.step().unwrap();
    }
}

#[test]
fn a_hex_with_a_market_and_no_pops_is_untouched() {
    let (mut world, _) = trade_world();
    let empty = hex(&world, 4, 0);
    assert!(world.market(empty).has_market());
    assert!(world.pops().on_hex(empty).is_empty());
    for _ in 0..2 {
        assert!(world.hex_stockpile(empty).is_empty());
        world.step().unwrap();
    }
    assert!(world.hex_stockpile(empty).is_empty());
}

#[test]
fn worlds_without_pops_report_nothing_traded() {
    for name in ["turns_valid", "valid_minimal"] {
        let (mut world, report) = world_of(&test_setting(name));
        assert!(world.pops().is_empty(), "{name}");
        let mut reports = vec![report];
        for _ in 0..2 {
            reports.push(world.step().unwrap());
        }
        for report in reports {
            assert_eq!(report.population, Quantity::ZERO, "{name}");
            assert!(report.sold.is_empty(), "{name}");
            assert!(report.bought.is_empty(), "{name}");
            assert!(report.consumed.is_empty(), "{name}");
        }
    }
    // The hex stockpiles are the deposits' output alone.
    let (mut world, _) = world_of(&test_setting("turns_valid"));
    world.step().unwrap();
    world.step().unwrap();
    let center = hex(&world, 0, 0);
    let daily = good(&world, "daily");
    let stew = good(&world, "stew");
    assert_eq!(world.hex_stockpile(center).total(daily).get(), 4.5);
    assert_eq!(world.hex_stockpile(center).total(stew).get(), 15.0);
}

#[test]
#[should_panic(expected = "is not from this list")]
fn pop_outcome_panics_on_a_pop_from_another_world() {
    let (sample, _) = world_of(&sample_setting());
    let foreign = pop_id(10, &sample);
    let (world, _) = trade_world();
    world.pop_outcome(foreign);
}
