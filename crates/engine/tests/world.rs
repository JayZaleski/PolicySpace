//! Integration tests for the world and its turn clock, through the engine's public API
//! only.

mod common;

use std::path::Path;

use common::{load, sample_setting, test_setting};
use policyspace_engine::{GoodId, HexCoord, HexId, PerishesAt, Quantity, Turn, TurnReport, World};

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

/// A hex's stockpile as (good name, total), in the stockpile's order.
fn totals(world: &World, q: i32, r: i32) -> Vec<(String, f64)> {
    world
        .hex_stockpile(hex(world, q, r))
        .iter()
        .map(|(g, amount)| (world.rules().goods.name(g).to_string(), amount.get()))
        .collect()
}

/// One good's groups on a hex as (when it perishes, amount), soonest first.
fn groups(world: &World, q: i32, r: i32, name: &str) -> Vec<(PerishesAt, f64)> {
    world
        .hex_stockpile(hex(world, q, r))
        .groups(good(world, name))
        .map(|(at, amount)| (at, amount.get()))
        .collect()
}

fn total(world: &World, q: i32, r: i32, name: &str) -> f64 {
    world
        .hex_stockpile(hex(world, q, r))
        .total(good(world, name))
        .get()
}

fn end(t: u64) -> PerishesAt {
    PerishesAt::EndOfTurn(Turn::new(t))
}

/// A report's list as (good name, amount).
fn named(world: &World, list: &[(GoodId, policyspace_engine::Quantity)]) -> Vec<(String, f64)> {
    list.iter()
        .map(|(g, amount)| (world.rules().goods.name(*g).to_string(), amount.get()))
        .collect()
}

fn pair(name: &str, amount: f64) -> (String, f64) {
    (name.to_string(), amount)
}

#[test]
fn sample_setting_at_turn_0() {
    let (world, report) = world_of(&sample_setting());
    assert_eq!(world.turn(), Turn::ZERO);
    assert_eq!(
        totals(&world, 0, 0),
        vec![
            pair("unimproved_copper", 2.5),
            pair("unimproved_grain", 12.0)
        ]
    );
    assert_eq!(
        groups(&world, 0, 0, "unimproved_copper"),
        vec![(end(0), 2.5)]
    );
    assert_eq!(
        groups(&world, 0, 0, "unimproved_grain"),
        vec![(end(0), 12.0)]
    );
    assert!(world.hex_stockpile(hex(&world, 2, -2)).is_empty());
    assert_eq!(report.turn, Turn::ZERO);
    assert!(report.perished.is_empty());
    assert_eq!(
        named(&world, &report.emitted),
        vec![
            pair("unimproved_copper", 10.0),
            pair("unimproved_grain", 24.0)
        ]
    );
    assert_eq!(
        named(&world, &report.endowed),
        vec![pair("unskilled_labor", 4100.0)]
    );
}

#[test]
fn sample_setting_steps() {
    let (mut world, mut previous) = world_of(&sample_setting());
    for t in 1..=3u64 {
        let report = world.step().unwrap();
        assert_eq!(report.turn, Turn::new(t));
        assert_eq!(world.turn(), Turn::new(t));
        let both = vec![
            pair("unimproved_copper", 10.0),
            pair("unimproved_grain", 24.0),
        ];
        // The POPs are endowed by their grown sizes: 10 labor each individual.
        let endowed = named(&world, &report.endowed);
        assert_eq!(endowed.len(), 1);
        assert_eq!(endowed[0].0, "unskilled_labor");
        assert!((endowed[0].1 - 10.0 * previous.population.get()).abs() < 1e-9);
        // The POPs' labor is traded and consumed in the round, so only the hex stockpiles
        // (and slivers of rounding) perish.
        let perished = named(&world, &report.perished);
        let big: Vec<_> = perished
            .iter()
            .filter(|(_, a)| *a >= 1e-9)
            .cloned()
            .collect();
        assert_eq!(big, both);
        assert_eq!(named(&world, &report.emitted), both);
        assert_eq!(
            totals(&world, 0, 0),
            vec![
                pair("unimproved_copper", 2.5),
                pair("unimproved_grain", 12.0)
            ]
        );
        assert_eq!(
            groups(&world, 0, 0, "unimproved_copper"),
            vec![(end(t), 2.5)]
        );
        assert_eq!(
            groups(&world, 0, 0, "unimproved_grain"),
            vec![(end(t), 12.0)]
        );
        previous = report;
    }
}

#[test]
fn a_three_turn_good_builds_up_and_levels_off() {
    let (mut world, first) = world_of(&test_setting("turns_valid"));
    let mut reports = vec![first];
    for t in 0..=5u64 {
        if t > 0 {
            reports.push(world.step().unwrap());
        }
        let expected: Vec<(PerishesAt, f64)> = match t {
            0 => vec![(end(2), 5.0)],
            1 => vec![(end(2), 5.0), (end(3), 5.0)],
            _ => vec![(end(t), 5.0), (end(t + 1), 5.0), (end(t + 2), 5.0)],
        };
        assert_eq!(groups(&world, 0, 0, "stew"), expected, "turn {t}");
        let expected_total = match t {
            0 => 5.0,
            1 => 10.0,
            _ => 15.0,
        };
        assert_eq!(total(&world, 0, 0, "stew"), expected_total, "turn {t}");
    }
    for (t, report) in reports.iter().enumerate() {
        assert_eq!(report.turn, Turn::new(t as u64));
        let stew = named(&world, &report.perished)
            .into_iter()
            .find(|(name, _)| name == "stew");
        if t < 3 {
            assert_eq!(stew, None, "turn {t}");
        } else {
            assert_eq!(stew, Some(pair("stew", 5.0)), "turn {t}");
        }
    }
}

#[test]
fn reports_match_the_expected_numbers() {
    let (mut world, report) = world_of(&test_setting("turns_valid"));
    let emitted = vec![pair("daily", 6.5), pair("stew", 5.0), pair("gold", 1.5)];
    assert!(report.perished.is_empty());
    assert!(report.endowed.is_empty());
    assert!(report.sold.is_empty() && report.bought.is_empty() && report.consumed.is_empty());
    assert_eq!(report.population, Quantity::ZERO);
    assert_eq!(named(&world, &report.emitted), emitted);
    for t in 1..=4u64 {
        let report = world.step().unwrap();
        let perished = if t < 3 {
            vec![pair("daily", 6.5)]
        } else {
            vec![pair("daily", 6.5), pair("stew", 5.0)]
        };
        assert_eq!(named(&world, &report.perished), perished, "turn {t}");
        assert_eq!(named(&world, &report.emitted), emitted, "turn {t}");
        assert!(report.endowed.is_empty(), "turn {t}");
        assert!(
            report.sold.is_empty() && report.bought.is_empty() && report.consumed.is_empty(),
            "turn {t}"
        );
        assert_eq!(report.population, Quantity::ZERO, "turn {t}");
    }
    assert!(world.pops().is_empty());
}

#[test]
fn a_durable_good_grows_and_never_perishes() {
    let (mut world, first) = world_of(&test_setting("turns_valid"));
    let mut reports = vec![first];
    for t in 0..=5u64 {
        if t > 0 {
            reports.push(world.step().unwrap());
        }
        assert_eq!(
            groups(&world, 1, 0, "gold"),
            vec![(PerishesAt::Never, 1.5 * (t + 1) as f64)],
            "turn {t}"
        );
    }
    for report in &reports {
        assert!(
            named(&world, &report.perished)
                .iter()
                .all(|(name, _)| name != "gold")
        );
    }
}

#[test]
fn several_deposits_merge_and_do_not_leak_into_other_hexes() {
    let (mut world, _) = world_of(&test_setting("turns_valid"));
    for t in 0..=5u64 {
        if t > 0 {
            world.step().unwrap();
        }
        assert_eq!(
            groups(&world, 0, 0, "daily"),
            vec![(end(t), 4.5)],
            "turn {t}"
        );
        assert_eq!(
            groups(&world, 1, 0, "daily"),
            vec![(end(t), 2.0)],
            "turn {t}"
        );
        assert_eq!(total(&world, 1, 0, "stew"), 0.0);
        assert_eq!(groups(&world, 1, 0, "stew"), Vec::new());
        assert_eq!(total(&world, 0, 0, "gold"), 0.0);
        assert!(
            world.hex_stockpile(hex(&world, 2, 0)).is_empty(),
            "turn {t}"
        );
    }
}

#[test]
fn worlds_without_deposits_stay_empty() {
    for name in ["valid_minimal", "hexes_one"] {
        let (mut world, report) = world_of(&test_setting(name));
        assert!(
            report.perished.is_empty()
                && report.emitted.is_empty()
                && report.endowed.is_empty()
                && report.sold.is_empty()
                && report.bought.is_empty()
                && report.consumed.is_empty(),
            "{name}"
        );
        assert_eq!(report.population, Quantity::ZERO, "{name}");
        for _ in 0..3 {
            let report = world.step().unwrap();
            assert!(
                report.perished.is_empty()
                    && report.emitted.is_empty()
                    && report.endowed.is_empty()
                    && report.sold.is_empty()
                    && report.bought.is_empty()
                    && report.consumed.is_empty(),
                "{name}"
            );
            assert_eq!(report.population, Quantity::ZERO, "{name}");
        }
        assert!(world.pops().is_empty(), "{name}");
        assert_eq!(world.map().len(), if name == "hexes_one" { 1 } else { 0 });
        for (id, _) in world.map().iter() {
            assert!(world.hex_stockpile(id).is_empty(), "{name}");
        }
    }
}

#[test]
fn identical_runs_give_identical_results() {
    let (mut a, report_a) = world_of(&test_setting("turns_valid"));
    let (mut b, report_b) = world_of(&test_setting("turns_valid"));
    assert_eq!(report_a, report_b);
    for _ in 0..6 {
        assert_eq!(a.step().unwrap(), b.step().unwrap());
        for (id, _) in a.map().iter() {
            assert_eq!(a.hex_stockpile(id), b.hex_stockpile(id));
        }
        for (id, _) in a.pops().iter() {
            assert_eq!(a.pop_stockpile(id), b.pop_stockpile(id));
        }
    }
}

#[test]
fn world_accessors() {
    let (mut world, _) = world_of(&test_setting("turns_valid"));
    assert_eq!(world.manifest().name, "Turns Test");
    assert_eq!(world.rules().goods.len(), 3);
    assert_eq!(world.map().len(), 3);
    assert_eq!(world.deposits().len(), 6);
    world.step().unwrap();
    world.step().unwrap();
    assert_eq!(world.turn(), Turn::new(2));
}

#[test]
#[should_panic(expected = "is not from this list")]
fn hex_stockpile_panics_on_a_foreign_hex() {
    let sample = load(&sample_setting());
    let foreign = sample
        .start
        .map
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() >= 3)
        .unwrap();
    let (world, _) = world_of(&test_setting("turns_valid"));
    world.hex_stockpile(foreign);
}

#[test]
fn hex_currencies_are_readable_and_unchanged_by_stepping() {
    let (mut world, _) = world_of(&test_setting("turns_valid"));
    let gold = good(&world, "gold");
    let hexes: Vec<HexId> = (0..3).map(|q| hex(&world, q, 0)).collect();
    let check = |world: &World| {
        assert_eq!(world.hex_currencies().len(), 3);
        for &h in &hexes {
            assert_eq!(world.hex_currency(h), gold);
        }
    };
    check(&world);
    for _ in 0..3 {
        world.step().unwrap();
        check(&world);
    }
}

#[test]
fn every_sample_hex_still_has_silver_after_a_step() {
    let (mut world, _) = world_of(&sample_setting());
    let silver = good(&world, "silver");
    world.step().unwrap();
    assert_eq!(world.hex_currencies().len(), 19);
    for (h, _) in world.map().iter() {
        assert_eq!(world.hex_currency(h), silver);
    }
}

#[test]
#[should_panic(expected = "is not from this list")]
fn hex_currency_panics_on_a_foreign_hex() {
    let sample = load(&sample_setting());
    let foreign = sample
        .start
        .map
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() >= 3)
        .unwrap();
    let (world, _) = world_of(&test_setting("turns_valid"));
    world.hex_currency(foreign);
}
