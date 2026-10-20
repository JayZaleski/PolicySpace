//! Integration tests for the factory mechanics, through the engine's public API only.

mod common;

use common::{
    assert_list, close, end, good, groups, load, sample_setting, stockpile_totals, test_setting,
    world_of,
};
use policyspace_engine::{
    BusinessId, FactoryRecord, LoadedSetting, Operation, PerishesAt, Stockpile, Turn, World,
    operate,
};

fn business(world: &World, name: &str) -> BusinessId {
    world.businesses().id(name).unwrap()
}

/// One operation per factory the business owns, in ID order, with the given
/// (run, maintenance) fractions. The conditions are the ones `placed` (the same setting the
/// world was built from, loaded separately) puts the factories at: a world is past turn 0's
/// operation when `World::new` returns, so under the idle host its factories have already
/// worn one slice. The technologies come from the world.
fn operations<'a>(
    world: &'a World,
    placed: &LoadedSetting,
    owner: BusinessId,
    orders: &[(f64, f64)],
) -> Vec<Operation<'a>> {
    let factories: Vec<_> = world.factories().owned_by(owner).collect();
    assert_eq!(factories.len(), orders.len(), "one order per factory");
    factories
        .into_iter()
        .zip(orders)
        .map(|(id, &(run_fraction, maintenance_fraction))| {
            let factory = world.factories().get(id);
            Operation {
                technology: world.rules().technologies.get(factory.technology),
                condition: placed.start.factories.get(id).condition,
                run_fraction,
                maintenance_fraction,
            }
        })
        .collect()
}

fn stock(world: &World, owner: BusinessId) -> Stockpile {
    world.business_stockpile(owner).clone()
}

/// Asserts a stockpile's totals are exactly these goods, in good ID order, within 1e-9.
fn assert_totals(world: &World, stockpile: &Stockpile, expected: &[(&str, f64)]) {
    let actual = stockpile_totals(world, stockpile);
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for ((an, aq), (en, eq)) in actual.iter().zip(expected) {
        assert_eq!(an, en, "{actual:?} vs {expected:?}");
        assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
    }
}

/// Asserts one factory's whole record: condition before, then (asked, done, inputs, output)
/// of the run, the wear, (asked, done, cost) of maintenance, and the condition after.
#[allow(clippy::too_many_arguments)]
fn assert_record(
    world: &World,
    record: &FactoryRecord,
    before: f64,
    run: (f64, f64, &[(&str, f64)], f64),
    wear: f64,
    maintenance: (f64, f64, &[(&str, f64)]),
    after: f64,
) {
    assert!(close(record.condition_before, before), "{record:?}");
    assert!(close(record.run.asked, run.0), "{record:?}");
    assert!(close(record.run.done, run.1), "{record:?}");
    assert_list(world, record.run.inputs.as_slice(), run.2);
    assert!(close(record.run.output.get(), run.3), "{record:?}");
    assert!(close(record.wear, wear), "{record:?}");
    assert!(close(record.maintenance.asked, maintenance.0), "{record:?}");
    assert!(close(record.maintenance.done, maintenance.1), "{record:?}");
    assert_list(world, record.maintenance.cost.as_slice(), maintenance.2);
    assert!(close(record.condition_after, after), "{record:?}");
}

#[test]
fn every_factory_of_the_sample_can_be_operated_without_labor_nothing_runs_or_is_maintained() {
    let (world, _) = world_of(&sample_setting());
    let placed = load(&sample_setting());
    let turn = world.turn();
    for (name, before, after, wear_lost) in [
        ("center_farm", 1.0, 0.875, 0.125),
        ("center_mine", 0.75, 0.625, 0.125),
        ("center_toolmaker", 1.0, 23.0 / 24.0, 1.0 / 24.0),
    ] {
        let owner = business(&world, name);
        let mut ops = operations(&world, &placed, owner, &[(1.0, 1.0)]);
        let mut stockpile = stock(&world, owner);
        let factory_id = world.factories().owned_by(owner).next().unwrap();
        let world_condition = world.factories().get(factory_id).condition;
        let records = operate(&mut ops, &mut stockpile, &world.rules().goods, turn);
        assert_eq!(records.len(), 1, "{name}");
        let record = &records[0];
        assert!(close(record.condition_before, before), "{name}: {record:?}");
        assert!(close(record.run.asked, 1.0), "{name}: {record:?}");
        assert!(close(record.run.done, 0.0), "{name}: {record:?}");
        assert!(record.run.inputs.is_empty(), "{name}: {record:?}");
        assert!(close(record.run.output.get(), 0.0), "{name}: {record:?}");
        assert!(close(record.wear, wear_lost), "{name}: {record:?}");
        assert!(close(record.maintenance.asked, 1.0), "{name}: {record:?}");
        assert!(close(record.maintenance.done, 0.0), "{name}: {record:?}");
        assert!(record.maintenance.cost.is_empty(), "{name}: {record:?}");
        assert!(close(record.condition_after, after), "{name}: {record:?}");
        assert!(close(ops[0].condition, after), "{name}");
        assert_eq!(&stockpile, world.business_stockpile(owner), "{name}");
        assert_eq!(
            world.factories().get(factory_id).condition,
            world_condition,
            "{name}"
        );
    }
}

#[test]
fn running_shares_a_shortage_cuts_by_the_worst_good_and_keeps_the_leftover() {
    let (world, _) = world_of(&test_setting("production_valid"));
    let placed = load(&test_setting("production_valid"));
    let works = business(&world, "works");
    let mut ops = operations(
        &world,
        &placed,
        works,
        &[(1.0, 0.0), (1.0, 0.0), (1.0, 0.0)],
    );
    let mut stockpile = stock(&world, works);
    let records = operate(&mut ops, &mut stockpile, &world.rules().goods, Turn::ZERO);

    let smelting = (1.0, 0.5, &[("labor", 5.0), ("ore", 10.0)][..], 5.0);
    let none = (0.0, 0.0, &[][..]);
    assert_record(&world, &records[0], 1.0, smelting, 0.25, none, 0.75);
    assert_record(&world, &records[1], 0.5, smelting, 0.25, none, 0.25);
    assert_record(
        &world,
        &records[2],
        1.0,
        (1.0, 0.5, &[("labor", 3.0)][..], 3.0),
        1.0 / 3.0,
        none,
        2.0 / 3.0,
    );
    assert_totals(
        &world,
        &stockpile,
        &[
            ("coin", 50.0),
            ("ore", 10.0),
            ("iron", 10.0),
            ("tools", 3.0),
            ("bread", 3.0),
        ],
    );
    let goods = |name| good(&world, name);
    assert_eq!(groups(&stockpile, goods("ore")), vec![(end(2), 10.0)]);
    assert_eq!(
        groups(&stockpile, goods("iron")),
        vec![(PerishesAt::Never, 10.0)]
    );
    assert_eq!(groups(&stockpile, goods("bread")), vec![(end(1), 3.0)]);
}

#[test]
fn the_order_of_a_businesss_factories_does_not_matter() {
    let (world, _) = world_of(&test_setting("production_valid"));
    let placed = load(&test_setting("production_valid"));
    let works = business(&world, "works");
    let goods = &world.rules().goods;

    let mut ops = operations(
        &world,
        &placed,
        works,
        &[(1.0, 0.0), (1.0, 0.0), (1.0, 0.0)],
    );
    let mut reversed_ops = ops.clone();
    reversed_ops.reverse();

    let mut forward = stock(&world, works);
    let records = operate(&mut ops, &mut forward, goods, Turn::ZERO);

    let mut backward = stock(&world, works);
    let mut reversed_records = operate(&mut reversed_ops, &mut backward, goods, Turn::ZERO);
    reversed_records.reverse();

    assert_eq!(records, reversed_records);
    assert_eq!(forward, backward);
}

#[test]
fn maintenance_costs_both_bundles_is_capped_and_shares_a_shortage_proportionally() {
    let (world, _) = world_of(&test_setting("production_valid"));
    let placed = load(&test_setting("production_valid"));
    let works = business(&world, "works");
    let mut ops = operations(
        &world,
        &placed,
        works,
        &[(0.0, 1.0), (0.0, 0.5), (0.0, 1.0)],
    );
    let mut stockpile = stock(&world, works);
    let records = operate(&mut ops, &mut stockpile, &world.rules().goods, Turn::ZERO);

    let nothing = (0.0, 0.0, &[][..], 0.0);
    assert_record(
        &world,
        &records[0],
        1.0,
        nothing,
        0.25,
        (1.0, 0.1875, &[("labor", 1.5), ("tools", 0.75)][..]),
        0.9375,
    );
    assert_record(
        &world,
        &records[1],
        0.5,
        nothing,
        0.25,
        (0.5, 0.375, &[("labor", 3.0), ("tools", 1.5)][..]),
        0.625,
    );
    assert_record(
        &world,
        &records[2],
        1.0,
        nothing,
        1.0 / 3.0,
        (1.0, 0.25, &[("tools", 0.75)][..]),
        11.0 / 12.0,
    );
    let totals = stockpile_totals(&world, &stockpile);
    let total = |name: &str| {
        totals
            .iter()
            .find(|(n, _)| n == name)
            .map_or(0.0, |(_, amount)| *amount)
    };
    assert!(close(total("labor"), 8.5), "{totals:?}");
    assert!(close(total("tools"), 0.0), "{totals:?}");
    assert!(close(total("coin"), 50.0), "{totals:?}");
    assert!(close(total("ore"), 30.0), "{totals:?}");
}

#[test]
fn a_full_factory_runs_its_lifetime_one_and_a_half_slices_run_once_and_the_perish_group_follows_the_turn()
 {
    let (world, _) = world_of(&test_setting("production_valid"));
    let placed = load(&test_setting("production_valid"));
    let springs = business(&world, "springs");
    let goods = &world.rules().goods;
    let ore = good(&world, "ore");
    let mut stockpile = stock(&world, springs);
    let none = (0.0, 0.0, &[][..]);

    let mut ops = operations(&world, &placed, springs, &[(1.0, 1.0), (1.0, 1.0)]);
    let records = operate(&mut ops, &mut stockpile, goods, Turn::ZERO);
    let spring_run = (1.0, 1.0, &[][..], 5.0);
    // Maintenance is capped to 1/4 by the four construction turns and needs labor, which the
    // business doesn't hold, so it asks for 1 and gets 0.
    let blocked = (1.0, 0.0, &[][..]);
    assert_record(
        &world,
        &records[0],
        1.0,
        spring_run,
        1.0 / 3.0,
        blocked,
        2.0 / 3.0,
    );
    assert_record(
        &world,
        &records[1],
        0.5,
        spring_run,
        1.0 / 3.0,
        blocked,
        1.0 / 6.0,
    );
    assert_eq!(groups(&stockpile, ore), vec![(end(2), 10.0)]);
    assert_totals(&world, &stockpile, &[("ore", 10.0), ("tools", 1.0)]);

    for order in &mut ops {
        order.maintenance_fraction = 0.0;
    }
    let records = operate(&mut ops, &mut stockpile, goods, Turn::new(1));
    assert_record(
        &world,
        &records[0],
        2.0 / 3.0,
        spring_run,
        1.0 / 3.0,
        none,
        1.0 / 3.0,
    );
    assert_record(
        &world,
        &records[1],
        1.0 / 6.0,
        (1.0, 0.0, &[][..], 0.0),
        1.0 / 6.0,
        none,
        0.0,
    );
    assert_eq!(ops[1].condition, 0.0);
    assert!(close(records[1].run.asked, 1.0) && close(records[1].run.done, 0.0));
    assert_eq!(groups(&stockpile, ore), vec![(end(2), 10.0), (end(3), 5.0)]);

    let records = operate(&mut ops, &mut stockpile, goods, Turn::new(2));
    assert_record(
        &world,
        &records[0],
        1.0 / 3.0,
        spring_run,
        1.0 / 3.0,
        none,
        0.0,
    );
    assert_eq!(ops[0].condition, 0.0);
    assert_eq!(
        groups(&stockpile, ore),
        vec![(end(2), 10.0), (end(3), 5.0), (end(4), 5.0)]
    );

    let records = operate(&mut ops, &mut stockpile, goods, Turn::new(3));
    for record in &records {
        assert!(close(record.run.done, 0.0), "{record:?}");
        assert!(close(record.run.output.get(), 0.0), "{record:?}");
    }
    assert_eq!(
        groups(&stockpile, ore),
        vec![(end(2), 10.0), (end(3), 5.0), (end(4), 5.0)]
    );
}
