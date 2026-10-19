//! Integration tests for POP growth: each POP's subsistence index, growth rate and size,
//! through the engine's public API only.

mod common;

use common::{close, load, named, outcome, pop_id, sample_setting, test_setting, world_of};
use policyspace_engine::{Quantity, TurnReport, World};

/// The current size of POP `index`.
fn size(world: &World, index: usize) -> f64 {
    world.pops().get(pop_id(index, world)).size.get()
}

/// The sum of every POP's current size.
fn total_size(world: &World) -> f64 {
    world.pops().iter().map(|(_, p)| p.size.get()).sum()
}

/// The rate of the sample's peasants on a market hex: 0.06 u − 0.03 u² at u = 2/3.
const PEASANT_RATE: f64 = 0.04 - 0.04 / 3.0;

// --- The sample setting ---

#[test]
fn a_fresh_sample_world_has_grown_at_turn_0() {
    let setting = load(&sample_setting());
    let start_sizes: Vec<f64> = setting
        .start
        .pops
        .iter()
        .map(|(_, p)| p.size.get())
        .collect();
    let (world, report) = World::new(setting, &mut common::idle()).unwrap();
    assert_eq!(world.pops().len(), 16);
    for (i, &start) in start_sizes.iter().enumerate() {
        let o = outcome(&world, i);
        assert_eq!(o.starting_size().get(), start, "POP {i}");
        let (index, rate) = match i {
            2 => (-1.0, -0.1),
            8 => (1.0, 0.03),
            _ => (2.0 / 3.0, PEASANT_RATE),
        };
        assert!(close(o.subsistence_index(), index), "POP {i}");
        assert!(close(o.growth_rate(), rate), "POP {i}");
        assert!(close(size(&world, i), start * (1.0 + rate)), "POP {i}");
    }
    assert_eq!(o_start(&world, 2), 12.0);
    assert!(close(size(&world, 2), 10.8));
    assert_eq!(o_start(&world, 8), 10.0);
    assert!(close(size(&world, 8), 10.3));
    assert!(close(report.population.get(), total_size(&world)));
    assert!(close(
        report.population.get(),
        388.0 * (1.0 + PEASANT_RATE) + 10.3 + 10.8
    ));
    assert!((report.population.get() - 419.44667).abs() < 1e-4);
}

fn o_start(world: &World, index: usize) -> f64 {
    outcome(world, index).starting_size().get()
}

/// Every list of one outcome per unit of its starting size.
fn per_individual(world: &World, index: usize) -> Vec<Vec<(String, f64)>> {
    let o = outcome(world, index);
    let n = o.starting_size().get();
    [o.sold(), o.bought(), o.consumed()]
        .iter()
        .map(|l| {
            named(world, l)
                .into_iter()
                .map(|(name, amount)| (name, amount / n))
                .collect()
        })
        .collect()
}

#[test]
fn stepping_the_sample_grows_again() {
    let (mut world, first) = world_of(&sample_setting());
    let per_unit: Vec<_> = (0..16).map(|i| per_individual(&world, i)).collect();
    let start_0 = o_start(&world, 0);
    let report = world.step(&mut common::idle()).unwrap();
    assert!(close(size(&world, 2), 9.72));
    assert!(close(
        size(&world, 0),
        start_0 * (1.0 + PEASANT_RATE) * (1.0 + PEASANT_RATE)
    ));
    assert!(close(size(&world, 8), 10.609));

    let endowed = named(&world, &report.endowed);
    assert_eq!(endowed.len(), 1);
    assert_eq!(endowed[0].0, "unskilled_labor");
    assert!(close(endowed[0].1, 10.0 * first.population.get()));

    for (i, before) in per_unit.iter().enumerate() {
        let now = per_individual(&world, i);
        assert_eq!(now.len(), before.len());
        for (a, b) in now.iter().zip(before) {
            assert_eq!(a.len(), b.len(), "POP {i}");
            for ((an, aq), (bn, bq)) in a.iter().zip(b) {
                assert_eq!(an, bn, "POP {i}");
                assert!(close(*aq, *bq), "POP {i}: {aq} vs {bq}");
            }
        }
    }
    world.step(&mut common::idle()).unwrap();
    assert!(close(size(&world, 2), 8.748));
}

#[test]
fn two_sample_worlds_grow_identically() {
    let (mut world, _) = world_of(&sample_setting());
    let (mut other, _) = world_of(&sample_setting());
    for _ in 0..3 {
        assert_eq!(
            world.step(&mut common::idle()).unwrap(),
            other.step(&mut common::idle()).unwrap()
        );
        for (id, pop) in world.pops().iter() {
            assert_eq!(pop.size, other.pops().get(id).size);
            assert_eq!(world.pop_outcome(id), other.pop_outcome(id));
            assert_eq!(world.pop_stockpile(id), other.pop_stockpile(id));
        }
    }
}

// --- The pops_growth setting ---

fn growth_world() -> (World, TurnReport) {
    world_of(&test_setting("pops_growth"))
}

/// (index, rate, starting size, size after growth) of the ten POPs at turn 0.
fn table() -> [(f64, f64, f64, f64); 10] {
    [
        (2.0 / 3.0, PEASANT_RATE, 20.0, 20.0 * (1.0 + PEASANT_RATE)),
        (9.0, 0.05, 10.0, 10.5),
        (3.0, -0.09, 10.0, 9.1),
        (9.0, -0.1, 10.0, 9.0),
        (0.0, 0.0, 10.0, 10.0),
        (-1.0, -0.1, 10.0, 9.0),
        (2.0 / 3.0, 0.0, 10.0, 10.0),
        (0.0, 0.0, 10.0, 10.0),
        (-6.0 / 11.0, -0.6 / 11.0, 11.0, 10.4),
        (-6.0 / 11.0, 0.0, 10.0, 10.0),
    ]
}

#[test]
fn the_pops_growth_table_at_turn_0() {
    let (world, report) = growth_world();
    assert_eq!(world.pops().len(), 10);
    for (i, (index, rate, start, after)) in table().into_iter().enumerate() {
        let o = outcome(&world, i);
        assert!(close(o.subsistence_index(), index), "POP {i}");
        assert!(close(o.growth_rate(), rate), "POP {i}");
        assert_eq!(o.starting_size().get(), start, "POP {i}");
        assert!(
            close(size(&world, i), after),
            "POP {i}: {}",
            size(&world, i)
        );
    }
    assert!(close(report.population.get(), 108.0 + 8.0 / 15.0));
    assert!(close(report.population.get(), total_size(&world)));
}

#[test]
fn the_pops_growth_table_after_a_step() {
    let (mut world, first) = growth_world();
    let report = world.step(&mut common::idle()).unwrap();
    let expected = [
        20.0 * (1.0 + PEASANT_RATE) * (1.0 + PEASANT_RATE),
        11.025,
        8.281,
        8.1,
        10.0,
        8.1,
        10.0,
        10.0,
        10.4 * 10.4 / 11.0,
        10.0,
    ];
    for (i, e) in expected.into_iter().enumerate() {
        assert!(close(size(&world, i), e), "POP {i}: {}", size(&world, i));
    }
    assert!((report.population.get() - 106.41962).abs() < 1e-4);
    assert!(close(report.population.get(), total_size(&world)));
    let endowed = named(&world, &report.endowed);
    assert_eq!(endowed.len(), 1);
    assert_eq!(endowed[0].0, "labor");
    assert!(close(endowed[0].1, 10.0 * first.population.get()));
    assert!(close(endowed[0].1, 1085.0 + 1.0 / 3.0));
}

#[test]
fn a_pop_at_subsistence_keeps_its_size() {
    let (mut world, _) = growth_world();
    for turn in 0..3 {
        let o = outcome(&world, 7);
        assert_eq!(o.subsistence_index(), 0.0, "turn {turn}");
        assert_eq!(o.growth_rate(), 0.0, "turn {turn}");
        assert_eq!(size(&world, 7), 10.0, "turn {turn}");
        world.step(&mut common::idle()).unwrap();
    }
}

#[test]
fn a_poor_pop_shrinks_by_decline_times_its_shortfall() {
    let (world, _) = growth_world();
    let o = outcome(&world, 8);
    assert!(close(o.subsistence_index(), 10.0 / 22.0 - 1.0));
    assert!(close(o.growth_rate(), 0.1 * (10.0 / 22.0 - 1.0)));
    assert!(close(size(&world, 8), 10.4));
}

#[test]
fn the_cap_and_the_floor_bound_the_polynomial() {
    let (world, _) = growth_world();
    assert_eq!(outcome(&world, 1).growth_rate(), 0.05);
    assert!(close(size(&world, 1), 10.5));
    assert!(close(outcome(&world, 2).growth_rate(), -0.09));
    assert!(close(size(&world, 2), 9.1));
    assert_eq!(outcome(&world, 3).growth_rate(), -0.1);
    assert!(close(size(&world, 3), 9.0));
}

#[test]
fn zero_cost_and_unobtainable_needs() {
    let (world, _) = growth_world();
    let carefree = outcome(&world, 4);
    assert_eq!(carefree.subsistence_index(), 0.0);
    assert_eq!(carefree.growth_rate(), 0.0);
    assert!(close(size(&world, 4), 10.0));

    let wanting = outcome(&world, 5);
    assert_eq!(wanting.subsistence_index(), -1.0);
    assert_eq!(wanting.growth_rate(), -0.1);
    assert!(close(size(&world, 5), 9.0));
    assert!(close(wanting.demand().income().get(), 100.0));
}

#[test]
fn zero_parameters_never_change_size() {
    let (mut world, _) = growth_world();
    for turn in 0..4 {
        for (i, index) in [(6, 2.0 / 3.0), (9, -6.0 / 11.0)] {
            let o = outcome(&world, i);
            assert!(close(o.subsistence_index(), index), "POP {i} turn {turn}");
            assert_eq!(o.growth_rate(), 0.0, "POP {i} turn {turn}");
            assert_eq!(size(&world, i), 10.0, "POP {i} turn {turn}");
        }
        world.step(&mut common::idle()).unwrap();
    }
}

// --- Other settings ---

#[test]
fn worlds_without_pops_report_zero_population() {
    for name in ["turns_valid", "valid_minimal"] {
        let (mut world, report) = world_of(&test_setting(name));
        assert_eq!(report.population, Quantity::ZERO, "{name}");
        for _ in 0..2 {
            assert_eq!(
                world.step(&mut common::idle()).unwrap().population,
                Quantity::ZERO,
                "{name}"
            );
        }
    }
}

#[test]
fn zero_growth_settings_keep_their_sizes_and_totals() {
    for (name, population) in [("pops_trade", 11.0), ("pops_placed", 12.5)] {
        let (mut world, report) = world_of(&test_setting(name));
        let sizes: Vec<Quantity> = world.pops().iter().map(|(_, p)| p.size).collect();
        assert_eq!(report.population.get(), population, "{name}");
        for _ in 0..3 {
            let report = world.step(&mut common::idle()).unwrap();
            assert_eq!(report.population.get(), population, "{name}");
            let now: Vec<Quantity> = world.pops().iter().map(|(_, p)| p.size).collect();
            assert_eq!(now, sizes, "{name}");
        }
    }
}
