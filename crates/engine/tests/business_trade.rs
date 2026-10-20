//! Integration tests for the business round and factory operation in the turn, through the
//! engine's public API only. The deciders are closures, except the sample test at the end,
//! which runs the project's Python arithmetic decider and so needs the project's virtual
//! environment (see `decider::python`), like `external_decider.rs`: without it that test
//! fails with the interpreter's message rather than skipping.

mod common;

use common::{assert_list, close, end, good, groups, load, named, sample_setting, test_setting};
use policyspace_engine::decider::python;
use policyspace_engine::decider::{ClosureDecider, DecisionCounts, FactoryOrder};
use policyspace_engine::{
    BusinessAnswer, BusinessId, BusinessOutcome, DeciderHost, Decision, FactoryId, FactoryRecord,
    Reply, Request, Stockpile, TurnReport, World,
};

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

/// The steady decisions: `works` runs both its factories, sells everything it makes and
/// buys the inputs and the maintenance goods; `poor` bids for far more than it can pay;
/// `remote` runs its factory and trades nothing.
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

/// The steady closure: answers every request, for every business in it, with its steady
/// decision.
fn steady(request: &Request) -> Reply {
    Reply {
        turn: request.turn,
        decisions: steady_decisions()
            .into_iter()
            .filter(|d| request.views.iter().any(|v| v.business == d.business))
            .collect(),
    }
}

fn setting_world(name: &str, host: &mut DeciderHost) -> (World, TurnReport) {
    World::new(load(&test_setting(name)), host).expect("the host holds `rule`")
}

fn business(world: &World, name: &str) -> BusinessId {
    world.businesses().id(name).unwrap()
}

fn outcome<'a>(world: &'a World, name: &str) -> &'a BusinessOutcome {
    world.business_outcome(business(world, name))
}

fn factory_id(world: &World, index: usize) -> FactoryId {
    world
        .factories()
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no factory {index}"))
}

fn condition(world: &World, index: usize) -> f64 {
    world.factories().get(factory_id(world, index)).condition
}

fn record<'a>(world: &'a World, name: &str, factory: usize) -> &'a FactoryRecord {
    outcome(world, name)
        .factory(factory_id(world, factory))
        .unwrap_or_else(|| panic!("{name} doesn't own factory {factory}"))
}

/// Asserts a stockpile's totals are exactly these goods, in good ID order, within 1e-9.
fn assert_totals(world: &World, stockpile: &Stockpile, expected: &[(&str, f64)]) {
    let actual = common::stockpile_totals(world, stockpile);
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

/// Asserts one group of `good` in `stockpile`: (when it perishes, amount).
fn assert_group(
    world: &World,
    stockpile: &Stockpile,
    name: &str,
    expected: &[(policyspace_engine::PerishesAt, f64)],
) {
    let actual = groups(stockpile, good(world, name));
    assert_eq!(actual.len(), expected.len(), "{name}: {actual:?}");
    for ((at, amount), (eat, eamount)) in actual.iter().zip(expected) {
        assert_eq!(at, eat, "{name}: {actual:?}");
        assert!(close(*amount, *eamount), "{name}: {actual:?}");
    }
}

fn perished_names(world: &World, report: &TurnReport) -> Vec<String> {
    named(world, &report.perished)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The two factories of `works` make iron 10 and bread 6 from the steady plan.
fn assert_works_steady_records(world: &World) {
    assert_record(
        world,
        record(world, "works", 0),
        1.0,
        (1.0, 1.0, &[("labor", 10.0), ("ore", 20.0)][..], 10.0),
        0.25,
        (0.25, 0.25, &[("labor", 2.0), ("tools", 1.0)][..]),
        1.0,
    );
    assert_record(
        world,
        record(world, "works", 1),
        1.0,
        (1.0, 1.0, &[("labor", 6.0)][..], 6.0),
        1.0 / 3.0,
        (1.0 / 3.0, 1.0 / 3.0, &[("tools", 1.0)][..]),
        1.0,
    );
}

#[test]
fn a_business_trades_operates_and_sells_its_output_next_turn() {
    let mut host = rule(steady);
    let (mut world, report) = setting_world("business_trade", &mut host);
    let works = business(&world, "works");
    let (iron, bread) = (good(&world, "iron"), good(&world, "bread"));
    let (ore, labor, tools) = (
        good(&world, "ore"),
        good(&world, "labor"),
        good(&world, "tools"),
    );

    // Turn 0: it holds no iron or bread yet, so it sells nothing; it buys its plan.
    let t = outcome(&world, "works").trade();
    assert_list(&world, t.offered(), &[("iron", 15.0), ("bread", 10.0)]);
    assert!(t.sold().is_empty());
    assert_list(
        &world,
        t.bid(),
        &[("labor", 18.0), ("ore", 20.0), ("tools", 2.0)],
    );
    assert!(close(t.bought_of(ore).get(), 20.0));
    assert!(close(t.bought_of(labor).get(), 18.0));
    assert!(close(t.bought_of(tools).get(), 2.0));
    assert_eq!(t.proportion(), 1.0);
    assert!(close(t.cash_before().get(), 100.0));
    assert!(close(t.cash_after_sales().get(), 100.0));
    assert!(close(t.cash_after_purchases().get(), 22.0));
    assert_works_steady_records(&world);
    assert_eq!(outcome(&world, "works").factories().len(), 2);
    assert_totals(
        &world,
        world.business_stockpile(works),
        &[
            ("coin", 22.0),
            ("iron", 10.0),
            ("tools", 3.0),
            ("bread", 6.0),
        ],
    );
    assert_group(
        &world,
        world.business_stockpile(works),
        "bread",
        &[(end(1), 6.0)],
    );
    assert!(close(condition(&world, 0), 1.0));
    assert!(close(condition(&world, 1), 1.0));
    assert!(report.business_sold.is_empty());

    // Turn 1: it sells last turn's output, and its cash is back where it started.
    let report = world.step(&mut host).unwrap();
    let t = outcome(&world, "works").trade();
    assert!(close(t.sold_of(iron).get(), 10.0));
    assert!(close(t.sold_of(bread).get(), 6.0));
    assert!(close(t.offered_of(iron).get(), 15.0));
    assert!(close(t.offered_of(bread).get(), 10.0));
    assert!(close(t.cash_before().get(), 22.0));
    assert!(close(t.cash_after_sales().get(), 100.0));
    assert!(close(t.cash_after_purchases().get(), 22.0));
    assert_works_steady_records(&world);
    assert_totals(
        &world,
        world.business_stockpile(works),
        &[
            ("coin", 22.0),
            ("iron", 10.0),
            ("tools", 3.0),
            ("bread", 6.0),
        ],
    );
    assert_group(
        &world,
        world.business_stockpile(works),
        "bread",
        &[(end(2), 6.0)],
    );
    let perished = perished_names(&world, &report);
    for name in ["labor", "bread", "iron"] {
        assert!(!perished.iter().any(|n| n == name), "{perished:?}");
    }
    assert_list(
        &world,
        &report.business_sold,
        &[("iron", 10.0), ("bread", 6.0)],
    );

    // Turn 2: the figures repeat, and no labor is left over to perish.
    let report = world.step(&mut host).unwrap();
    assert!(!perished_names(&world, &report).iter().any(|n| n == "labor"));
    let t = outcome(&world, "works").trade();
    assert!(close(t.cash_before().get(), 22.0));
    assert!(close(t.cash_after_sales().get(), 100.0));
    assert!(close(t.cash_after_purchases().get(), 22.0));
    assert_works_steady_records(&world);
}

#[test]
fn a_cash_short_business_is_scaled_and_its_run_is_cut() {
    let mut host = rule(steady);
    let (mut world, _) = setting_world("business_trade", &mut host);
    let (labor, tools) = (good(&world, "labor"), good(&world, "tools"));

    let t = outcome(&world, "poor").trade();
    assert_list(&world, t.bid(), &[("labor", 10.0), ("tools", 4.0)]);
    assert!(close(t.proportion(), 0.2));
    assert!(close(t.bought_of(labor).get(), 2.0));
    assert!(close(t.bought_of(tools).get(), 0.8));
    assert!(close(t.cash_before().get(), 10.0));
    assert!(close(t.cash_after_sales().get(), 10.0));
    assert!(close(t.cash_after_purchases().get(), 0.0));
    assert_record(
        &world,
        record(&world, "poor", 2),
        1.0,
        (1.0, 0.2, &[("labor", 2.0), ("ore", 4.0)][..], 2.0),
        0.25,
        (0.0, 0.0, &[][..]),
        0.75,
    );
    assert!(close(condition(&world, 2), 0.75));

    // Turn 1: no cash, so nothing is bought, no labor is held, and the factory only wears.
    world.step(&mut host).unwrap();
    let t = outcome(&world, "poor").trade();
    assert_eq!(t.proportion(), 0.0);
    assert!(t.bought().is_empty());
    assert_record(
        &world,
        record(&world, "poor", 2),
        0.75,
        (1.0, 0.0, &[][..], 0.0),
        0.25,
        (0.0, 0.0, &[][..]),
        0.5,
    );
    assert!(close(condition(&world, 2), 0.5));
}

#[test]
fn a_business_on_a_hex_with_no_market_operates_from_its_stockpile() {
    let mut host = rule(steady);
    let (mut world, _) = setting_world("business_trade", &mut host);
    let remote = business(&world, "remote");

    let t = outcome(&world, "remote").trade();
    assert!(t.offered().is_empty() && t.sold().is_empty());
    assert!(t.bid().is_empty() && t.bought().is_empty());
    assert_eq!(t.proportion(), 1.0);
    assert_eq!(t.cash_before().get(), 0.0);
    assert_eq!(t.cash_after_sales().get(), 0.0);
    assert_eq!(t.cash_after_purchases().get(), 0.0);
    assert_record(
        &world,
        record(&world, "remote", 3),
        1.0,
        (1.0, 1.0, &[("labor", 6.0)][..], 6.0),
        1.0 / 3.0,
        (0.0, 0.0, &[][..]),
        2.0 / 3.0,
    );
    assert!(close(condition(&world, 3), 2.0 / 3.0));
    assert_totals(&world, world.business_stockpile(remote), &[("bread", 6.0)]);
    assert_group(
        &world,
        world.business_stockpile(remote),
        "bread",
        &[(end(1), 6.0)],
    );

    // Turn 1: no labor left, so nothing runs.
    world.step(&mut host).unwrap();
    assert_record(
        &world,
        record(&world, "remote", 3),
        2.0 / 3.0,
        (1.0, 0.0, &[][..], 0.0),
        1.0 / 3.0,
        (0.0, 0.0, &[][..]),
        1.0 / 3.0,
    );
    assert!(close(condition(&world, 3), 1.0 / 3.0));

    // Turn 2's report shows what perished at the end of turn 1: remote's bread.
    let report = world.step(&mut host).unwrap();
    assert_list(&world, &report.perished, &[("bread", 6.0)]);
}

#[test]
fn an_idle_business_trades_nothing_and_its_factories_wear_down() {
    let (mut world, report) = setting_world("business_trade", &mut DeciderHost::idle());
    let slice = [0.75, 2.0 / 3.0, 0.75, 2.0 / 3.0];
    let owners = ["works", "works", "poor", "remote"];
    for name in ["works", "poor", "remote"] {
        let t = outcome(&world, name).trade();
        assert!(t.offered().is_empty() && t.sold().is_empty(), "{name}");
        assert!(t.bid().is_empty() && t.bought().is_empty(), "{name}");
        assert_eq!(t.proportion(), 1.0, "{name}");
    }
    for (index, (owner, after)) in owners.iter().zip(slice).enumerate() {
        let lost = 1.0 - after;
        assert_record(
            &world,
            record(&world, owner, index),
            1.0,
            (0.0, 0.0, &[][..], 0.0),
            lost,
            (0.0, 0.0, &[][..]),
            after,
        );
        assert!(close(condition(&world, index), after));
    }
    assert!(report.produced.is_empty() && report.used.is_empty());
    assert!(report.business_sold.is_empty() && report.business_bought.is_empty());
    assert_totals(
        &world,
        world.business_stockpile(business(&world, "remote")),
        &[("labor", 6.0)],
    );
    assert_totals(
        &world,
        world.business_stockpile(business(&world, "works")),
        &[("coin", 100.0), ("tools", 3.0)],
    );

    // Three more turns of wear take every factory to zero exactly.
    for _ in 0..3 {
        let report = world.step(&mut DeciderHost::idle()).unwrap();
        assert!(report.produced.is_empty() && report.used.is_empty());
        assert!(report.business_sold.is_empty() && report.business_bought.is_empty());
    }
    for index in 0..4 {
        assert_eq!(condition(&world, index), 0.0, "factory {index}");
    }

    // A worn-out factory is shut down however well supplied: `works` buys its inputs, and
    // nothing runs. The steady decision still asks for maintenance, which the business can
    // pay for, so each factory starts to be restored.
    let report = world.step(&mut rule(steady)).unwrap();
    let t = outcome(&world, "works").trade();
    assert!(close(t.cash_after_purchases().get(), 22.0));
    assert_record(
        &world,
        record(&world, "works", 0),
        0.0,
        (1.0, 0.0, &[][..], 0.0),
        0.0,
        (0.25, 0.25, &[("labor", 2.0), ("tools", 1.0)][..]),
        0.25,
    );
    assert_record(
        &world,
        record(&world, "works", 1),
        0.0,
        (1.0, 0.0, &[][..], 0.0),
        0.0,
        (1.0 / 3.0, 1.0 / 3.0, &[("tools", 1.0)][..]),
        1.0 / 3.0,
    );
    assert!(report.produced.is_empty());
}

#[test]
fn a_refused_decision_idles_its_business() {
    let mut host = rule(|request| {
        let mut decisions = steady(request).decisions;
        for decision in &mut decisions {
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
        Reply {
            turn: request.turn,
            decisions,
        }
    });
    let (world, report) = setting_world("business_trade", &mut host);
    assert!(matches!(
        world.business_answer(business(&world, "poor")),
        BusinessAnswer::Refused { .. }
    ));
    let t = outcome(&world, "poor").trade();
    assert!(t.offered().is_empty() && t.sold().is_empty());
    assert!(t.bid().is_empty() && t.bought().is_empty());
    assert_record(
        &world,
        record(&world, "poor", 2),
        1.0,
        (0.0, 0.0, &[][..], 0.0),
        0.25,
        (0.0, 0.0, &[][..]),
        0.75,
    );
    assert!(close(condition(&world, 2), 0.75));
    assert_works_steady_records(&world);
    assert!(close(
        outcome(&world, "works")
            .trade()
            .cash_after_purchases()
            .get(),
        22.0
    ));
    assert_eq!(
        report.decisions,
        DecisionCounts {
            valid: 2,
            refused: 1,
            nothing: 0
        }
    );
}

#[test]
fn pops_on_the_hex_are_unaffected_by_the_businesses() {
    let (steady_world, _) = setting_world("business_trade", &mut rule(steady));
    let (idle_world, _) = setting_world("business_trade", &mut DeciderHost::idle());
    let pop = common::pop_id(0, &steady_world);
    assert_eq!(steady_world.pop_outcome(pop), idle_world.pop_outcome(pop));
    assert_eq!(
        steady_world.pop_stockpile(pop),
        idle_world.pop_stockpile(common::pop_id(0, &idle_world))
    );
}

#[test]
fn the_report_totals_every_business() {
    let (world, report) = setting_world("business_trade", &mut rule(steady));
    assert!(report.business_sold.is_empty());
    assert_list(
        &world,
        &report.business_bought,
        &[("labor", 20.0), ("ore", 20.0), ("tools", 2.8)],
    );
    assert_list(&world, &report.produced, &[("iron", 12.0), ("bread", 12.0)]);
    assert_list(
        &world,
        &report.used,
        &[("labor", 26.0), ("ore", 24.0), ("tools", 2.0)],
    );

    let (mut world, report) = setting_world("business_trade", &mut DeciderHost::idle());
    for list in [
        &report.business_sold,
        &report.business_bought,
        &report.produced,
        &report.used,
    ] {
        assert!(list.is_empty());
    }
    let report = world.step(&mut DeciderHost::idle()).unwrap();
    for list in [
        &report.business_sold,
        &report.business_bought,
        &report.produced,
        &report.used,
    ] {
        assert!(list.is_empty());
    }
}

/// Every business's outcome and stockpile, and every factory's condition.
fn snapshot(world: &World) -> (Vec<BusinessOutcome>, Vec<Stockpile>, Vec<f64>) {
    let ids: Vec<BusinessId> = world.businesses().iter().map(|(id, _, _)| id).collect();
    (
        ids.iter()
            .map(|id| world.business_outcome(*id).clone())
            .collect(),
        ids.iter()
            .map(|id| world.business_stockpile(*id).clone())
            .collect(),
        world.factories().iter().map(|(_, f)| f.condition).collect(),
    )
}

#[test]
fn two_worlds_are_identical() {
    let mut host_a = rule(steady);
    let mut host_b = rule(steady);
    let (mut a, report_a) = setting_world("business_trade", &mut host_a);
    let (mut b, report_b) = setting_world("business_trade", &mut host_b);
    assert_eq!(report_a, report_b);
    assert_eq!(snapshot(&a), snapshot(&b));
    for _ in 0..3 {
        assert_eq!(a.step(&mut host_a).unwrap(), b.step(&mut host_b).unwrap());
        assert_eq!(snapshot(&a), snapshot(&b));
    }
}

#[test]
fn the_sample_chain_runs_under_the_arithmetic_decider() {
    let new_world = || {
        let setting = load(&sample_setting());
        let mut host = python::host_for(&setting).unwrap_or_else(|e| panic!("{e}"));
        let (world, report) = World::new(setting, &mut host).expect("the host holds `arithmetic`");
        (world, host, report)
    };
    let (mut world, mut host, report) = new_world();
    assert!(
        report.decider_failures.is_empty(),
        "{:?}",
        report.decider_failures
    );
    assert_eq!(report.decisions.valid, 3);
    assert_eq!(report.decisions.refused, 0);
    assert_eq!(report.decisions.nothing, 0);
    let start_cash: Vec<f64> = ["center_farm", "center_mine", "center_toolmaker"]
        .iter()
        .map(|name| outcome(&world, name).trade().cash_after_purchases().get())
        .collect();

    let farm = outcome(&world, "center_farm").trade();
    assert!(farm.sold().is_empty());
    assert!(close(farm.bought_of(good(&world, "tools")).get(), 0.5));
    assert!(close(
        farm.bought_of(good(&world, "unimproved_grain")).get(),
        4.0
    ));
    assert!(close(
        farm.bought_of(good(&world, "unskilled_labor")).get(),
        7.0
    ));
    assert_eq!(farm.proportion(), 1.0);
    assert!(close(farm.cash_before().get(), 100.0));
    assert!(close(farm.cash_after_sales().get(), 100.0));
    assert!(close(farm.cash_after_purchases().get(), 85.0));
    let r = record(&world, "center_farm", 0);
    assert!(
        close(r.run.done, 1.0) && close(r.run.output.get(), 12.0),
        "{r:?}"
    );
    assert!(
        close(r.maintenance.asked, 0.125) && close(r.maintenance.done, 0.125),
        "{r:?}"
    );
    assert!(
        close(r.condition_before, 1.0) && close(r.condition_after, 1.0),
        "{r:?}"
    );
    assert_totals(
        &world,
        world.business_stockpile(business(&world, "center_farm")),
        &[("grain", 12.0), ("silver", 85.0)],
    );

    let mine = outcome(&world, "center_mine").trade();
    assert!(close(mine.bought_of(good(&world, "tools")).get(), 0.75));
    assert!(close(
        mine.bought_of(good(&world, "unskilled_labor")).get(),
        4.0
    ));
    assert!(close(mine.cash_before().get(), 50.0));
    assert!(close(mine.cash_after_purchases().get(), 37.0));
    let r = record(&world, "center_mine", 1);
    assert!(close(r.run.output.get(), 2.5), "{r:?}");
    assert!(close(r.wear, 0.125), "{r:?}");
    assert!(
        close(r.maintenance.asked, 0.375) && close(r.maintenance.done, 0.375),
        "{r:?}"
    );
    assert!(
        close(r.condition_before, 0.75) && close(r.condition_after, 1.0),
        "{r:?}"
    );

    let toolmaker = outcome(&world, "center_toolmaker").trade();
    assert!(close(
        toolmaker.bought_of(good(&world, "copper")).get(),
        8.0
    ));
    assert!(close(
        toolmaker.bought_of(good(&world, "tools")).get(),
        0.125
    ));
    assert!(close(
        toolmaker.bought_of(good(&world, "unskilled_labor")).get(),
        6.25
    ));
    assert!(close(toolmaker.cash_before().get(), 200.0));
    assert!(close(toolmaker.cash_after_purchases().get(), 152.25));
    let r = record(&world, "center_toolmaker", 2);
    assert!(close(r.run.output.get(), 4.0), "{r:?}");
    assert!(
        close(r.condition_before, 1.0) && close(r.condition_after, 1.0),
        "{r:?}"
    );
    let stock = world.business_stockpile(business(&world, "center_toolmaker"));
    assert!(close(stock.total(good(&world, "tools")).get(), 4.0));

    // Turn 1: each business sells what it made.
    let report = world.step(&mut host).unwrap();
    assert!(
        report.decider_failures.is_empty(),
        "{:?}",
        report.decider_failures
    );
    let sold =
        |name: &str| -> Vec<(String, f64)> { named(&world, outcome(&world, name).trade().sold()) };
    let grain = sold("center_farm");
    assert!(
        grain.iter().any(|(n, a)| n == "grain" && close(*a, 12.0)),
        "{grain:?}"
    );
    let copper = sold("center_mine");
    assert!(
        copper.iter().any(|(n, a)| n == "copper" && close(*a, 2.5)),
        "{copper:?}"
    );
    let tools = sold("center_toolmaker");
    assert!(
        tools.iter().any(|(n, a)| n == "tools" && *a > 3.8),
        "{tools:?}"
    );
    for (name, start) in ["center_farm", "center_mine", "center_toolmaker"]
        .iter()
        .zip(&start_cash)
    {
        let cash = outcome(&world, name).trade().cash_after_purchases().get();
        assert!(cash > *start, "{name}: {cash} vs {start}");
    }

    // Three more turns: no failures, every factory stays in full condition, and every
    // business keeps selling.
    for _ in 0..3 {
        let report = world.step(&mut host).unwrap();
        assert!(
            report.decider_failures.is_empty(),
            "{:?}",
            report.decider_failures
        );
        for index in 0..3 {
            assert!(close(condition(&world, index), 1.0), "factory {index}");
        }
        for name in ["center_farm", "center_mine", "center_toolmaker"] {
            assert!(!outcome(&world, name).trade().sold().is_empty(), "{name}");
        }
    }

    // Two worlds under two fresh hosts agree.
    let (mut a, mut host_a, report_a) = new_world();
    let (mut b, mut host_b, report_b) = new_world();
    assert_eq!(report_a, report_b);
    for _ in 0..3 {
        assert_eq!(a.step(&mut host_a).unwrap(), b.step(&mut host_b).unwrap());
        assert_eq!(snapshot(&a), snapshot(&b));
    }
}
