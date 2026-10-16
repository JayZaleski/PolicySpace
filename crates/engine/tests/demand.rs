//! Integration tests for Stone-Geary demand against the sample setting, through the
//! engine's public API only.

mod common;

use common::{close, good, hex, load, q, sample_setting};
use policyspace_engine::{Demand, Quantity, World, demand};

fn sample_world() -> World {
    World::new(load(&sample_setting())).0
}

/// The demand of a POP of the named type with `size` individuals holding `labor` unskilled
/// labor, on the hex at (`q`, `r`).
fn demand_of(world: &World, type_name: &str, size: f64, labor: f64, at: (i32, i32)) -> Demand {
    let pop_types = &world.rules().pop_types;
    let pop_type = pop_types.get(pop_types.id(type_name).unwrap());
    let market = world.market(hex(world, at.0, at.1));
    let holdings = vec![(good(world, "unskilled_labor"), q(labor))];
    demand(pop_type, q(size), holdings, &market)
}

/// The demanded goods as (good name, quantity), in the order of the result.
fn quantities(world: &World, d: &Demand) -> Vec<(String, f64)> {
    d.goods()
        .iter()
        .map(|g| {
            (
                world.rules().goods.name(g.good).to_string(),
                g.quantity.get(),
            )
        })
        .collect()
}

/// Asserts the value of the demanded goods that have a price equals the income.
fn assert_balances(world: &World, d: &Demand, at: (i32, i32)) {
    let market = world.market(hex(world, at.0, at.1));
    let spent: f64 = d
        .goods()
        .iter()
        .filter_map(|g| {
            market
                .price(g.good)
                .map(|price| g.quantity.get() * price.get())
        })
        .sum();
    let m = d.income().get();
    assert!(
        (spent - m).abs() < 1e-9 * m.max(1.0),
        "spends {spent} of income {m}"
    );
}

fn assert_quantities(world: &World, d: &Demand, expected: &[(&str, f64)]) {
    let actual = quantities(world, d);
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for ((an, aq), (en, eq)) in actual.iter().zip(expected) {
        assert_eq!(an, en, "{actual:?} vs {expected:?}");
        assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
    }
}

const CENTER: (i32, i32) = (0, 0);
const NO_MARKET: (i32, i32) = (2, -2);

#[test]
fn a_peasant_at_typical_prices() {
    let world = sample_world();
    let d = demand_of(&world, "peasant", 1.0, 10.0, CENTER);
    assert_quantities(&world, &d, &[("unskilled_labor", 2.8), ("grain", 3.6)]);
    assert!(close(d.income().get(), 10.0));
    assert!(close(d.subsistence_cost().get(), 6.0));
    assert!(close(d.supernumerary(), 4.0));
    assert!(!d.below_subsistence());
    assert_balances(&world, &d, CENTER);
}

#[test]
fn an_artisan_at_typical_prices() {
    // The sample artisan's subsistence bundle (1 labor, 2 grain) costs 1 + 4 = 5 at these
    // prices, so 10 labor leaves 5 to spend: labor 1 + 0.1 * 5 / 1, grain 2 + 0.4 * 5 / 2,
    // copper 0.5 * 5 / 5.
    let world = sample_world();
    let d = demand_of(&world, "artisan", 1.0, 10.0, CENTER);
    assert_quantities(
        &world,
        &d,
        &[("unskilled_labor", 1.5), ("copper", 0.5), ("grain", 3.0)],
    );
    assert!(close(d.income().get(), 10.0));
    assert!(close(d.subsistence_cost().get(), 5.0));
    assert!(close(d.supernumerary(), 5.0));
    assert!(!d.below_subsistence());
    assert_balances(&world, &d, CENTER);
}

#[test]
fn demand_is_linear_in_size() {
    let world = sample_world();
    for type_name in ["peasant", "artisan"] {
        let one = demand_of(&world, type_name, 1.0, 10.0, CENTER);
        let fifty = demand_of(&world, type_name, 50.0, 500.0, CENTER);
        assert_eq!(one.goods().len(), fifty.goods().len());
        for (a, b) in one.goods().iter().zip(fifty.goods()) {
            assert_eq!(a.good, b.good);
            assert!(
                close(a.quantity.get() * 50.0, b.quantity.get()),
                "{type_name}: {a:?} vs {b:?}"
            );
        }
        assert!(close(one.income().get() * 50.0, fifty.income().get()));
        assert!(close(
            one.subsistence_cost().get() * 50.0,
            fifty.subsistence_cost().get()
        ));
        assert_balances(&world, &fifty, CENTER);
    }
}

#[test]
fn a_pop_on_a_hex_with_no_market_keeps_what_it_holds() {
    let world = sample_world();
    let d = demand_of(&world, "peasant", 1.0, 10.0, NO_MARKET);
    assert_quantities(&world, &d, &[("unskilled_labor", 10.0), ("grain", 0.0)]);
    assert_eq!(d.income(), Quantity::ZERO);
    assert_eq!(d.subsistence_cost(), Quantity::ZERO);
    assert_eq!(d.supernumerary(), 0.0);
    let labor = d.get(good(&world, "unskilled_labor")).unwrap();
    let grain = d.get(good(&world, "grain")).unwrap();
    assert!(!labor.below_subsistence());
    assert!(grain.below_subsistence());
    assert!(d.below_subsistence());
    assert_balances(&world, &d, NO_MARKET);
}

#[test]
fn the_recorded_demand_matches_the_pure_function() {
    let world = sample_world();
    let center = hex(&world, 0, 0);
    let peasant_id = world.rules().pop_types.id("peasant").unwrap();
    let pop_id = world.pops().of_type_on_hex(center, peasant_id).unwrap();
    // The POP has grown since the round, so the demand was for its starting size.
    let size = world.pop_outcome(pop_id).starting_size();
    assert_eq!(size, q(60.0));
    let pop_type = world.rules().pop_types.get(peasant_id);
    let market = world.market(center);

    // The stockpile is empty after turn 0's trade round, so the demand to compare is the
    // one the round recorded.
    let labor = good(&world, "unskilled_labor");
    let recorded = world.pop_outcome(pop_id).demand();
    assert_eq!(recorded.get(labor).unwrap().held, q(600.0));
    assert_quantities(
        &world,
        recorded,
        &[("unskilled_labor", 168.0), ("grain", 216.0)],
    );
    assert!(close(recorded.income().get(), 600.0));
    assert_balances(&world, recorded, CENTER);
    let pure = demand(pop_type, size, [(labor, q(600.0))], &market);
    assert_eq!(recorded, &pure);
    assert_eq!(world.turn().get(), 0);
}

#[test]
fn every_case_balances_the_budget() {
    let world = sample_world();
    for type_name in ["peasant", "artisan"] {
        for size in [0.5, 1.0, 7.0, 50.0] {
            for labor in [0.0, 1.0, 3.0, 10.0, 500.0] {
                for at in [CENTER, (1, 0), NO_MARKET] {
                    let d = demand_of(&world, type_name, size, labor, at);
                    assert_balances(&world, &d, at);
                }
            }
        }
    }
}
