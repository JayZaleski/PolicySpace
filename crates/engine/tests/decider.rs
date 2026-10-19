//! Integration tests for the decider protocol (views, decisions, batches), through the
//! engine's public API only.

mod common;

use std::collections::BTreeMap;

use common::{q, sample_setting, test_setting, world_of};
use policyspace_engine::decider::{FactoryOrder, PerishGroup, ReplyError};
use policyspace_engine::{
    Answer, BusinessId, BusinessView, Decision, Quantity, Recording, Reply, Request, Turn, World,
};

fn business(world: &World, name: &str) -> BusinessId {
    world.businesses().id(name).unwrap()
}

fn view(world: &World, name: &str) -> BusinessView {
    world.business_view(business(world, name))
}

fn groups_of(view: &BusinessView, good: &str) -> Vec<(f64, Option<u64>)> {
    view.stockpile[good]
        .iter()
        .map(|g: &PerishGroup| (g.amount.get(), g.perishes.map(Turn::get)))
        .collect()
}

fn qmap(items: &[(&str, f64)]) -> BTreeMap<String, Quantity> {
    items.iter().map(|(n, a)| (n.to_string(), q(*a))).collect()
}

fn table(items: &[(&str, f64)]) -> BTreeMap<String, f64> {
    items.iter().map(|(n, a)| (n.to_string(), *a)).collect()
}

fn order(run: f64, maintenance: f64) -> FactoryOrder {
    FactoryOrder { run, maintenance }
}

/// (technology, condition) per factory.
fn factory_rows(view: &BusinessView) -> Vec<(u32, String, f64)> {
    view.factories
        .iter()
        .map(|(id, f)| (*id, f.technology.clone(), f.condition))
        .collect()
}

fn deposit_rows(view: &BusinessView) -> Vec<(String, u32, String, f64)> {
    view.deposits
        .iter()
        .map(|(name, d)| (name.clone(), d.count, d.produces.clone(), d.output.get()))
        .collect()
}

/// A full decision for `view`'s business: the first factory at 1 and 0.125, an offer of
/// the first good it holds that isn't the currency, a bid for labor.
fn full_decision(view: &BusinessView) -> Decision {
    let mut d = Decision::empty(view.business.clone());
    for id in view.factories.keys() {
        d.factories.insert(*id, order(1.0, 0.125));
    }
    d.offers = table(&[("unimproved_grain", 8.0)]);
    d.bids = table(&[("unskilled_labor", 6.0)]);
    d.trace = Some("first line\nsecond line".to_string());
    d
}

#[test]
fn the_sample_businesses_see_their_own_circumstances() {
    let (mut world, _) = world_of(&sample_setting());
    let prices = qmap(&[
        ("copper", 5.0),
        ("grain", 2.0),
        ("tools", 12.0),
        ("unimproved_copper", 1.0),
        ("unimproved_grain", 0.5),
        ("unskilled_labor", 1.0),
    ]);

    let farm = view(&world, "center_farm");
    let mine = view(&world, "center_mine");
    let toolmaker = view(&world, "center_toolmaker");
    for v in [&farm, &mine, &toolmaker] {
        assert_eq!(v.turn, Turn::ZERO);
        assert_eq!(v.currency, "silver");
        assert_eq!(v.prices, prices);
        assert_eq!(v.factories.len(), 1);
        assert_eq!(v.technologies.len(), 1);
    }

    assert_eq!(farm.business, "center_farm");
    assert_eq!(farm.cash.get(), 100.0);
    assert_eq!(
        farm.stockpile.keys().collect::<Vec<_>>(),
        ["silver", "unimproved_grain"]
    );
    assert_eq!(groups_of(&farm, "silver"), [(100.0, None)]);
    assert_eq!(groups_of(&farm, "unimproved_grain"), [(8.0, Some(0))]);
    assert_eq!(factory_rows(&farm), [(0, "farm".to_string(), 1.0)]);
    let t = &farm.technologies["farm"];
    assert_eq!(
        t.inputs,
        qmap(&[("unimproved_grain", 1.0), ("unskilled_labor", 0.5)])
    );
    assert_eq!(t.output, "grain");
    assert_eq!(t.capacity.get(), 12.0);
    assert_eq!(t.capital, qmap(&[("tools", 4.0)]));
    assert_eq!(t.construction, qmap(&[("unskilled_labor", 8.0)]));
    assert_eq!((t.construction_turns, t.lifetime), (2, 8));
    assert_eq!(
        deposit_rows(&farm),
        [(
            "arable_land".to_string(),
            2,
            "unimproved_grain".to_string(),
            4.0
        )]
    );

    assert_eq!(mine.cash.get(), 50.0);
    assert_eq!(
        mine.stockpile.keys().collect::<Vec<_>>(),
        ["silver", "unimproved_copper"]
    );
    assert_eq!(groups_of(&mine, "silver"), [(50.0, None)]);
    assert_eq!(groups_of(&mine, "unimproved_copper"), [(2.5, Some(0))]);
    assert_eq!(factory_rows(&mine), [(1, "mine".to_string(), 0.75)]);
    let t = &mine.technologies["mine"];
    assert_eq!(
        t.inputs,
        qmap(&[("unimproved_copper", 1.0), ("unskilled_labor", 1.0)])
    );
    assert_eq!(t.output, "copper");
    assert_eq!(t.capacity.get(), 2.5);
    assert_eq!(t.capital, qmap(&[("tools", 2.0)]));
    assert_eq!(t.construction, qmap(&[("unskilled_labor", 4.0)]));
    assert_eq!((t.construction_turns, t.lifetime), (2, 8));
    assert_eq!(
        deposit_rows(&mine),
        [(
            "copper_ore".to_string(),
            1,
            "unimproved_copper".to_string(),
            2.5
        )]
    );

    assert_eq!(toolmaker.cash.get(), 200.0);
    assert_eq!(toolmaker.stockpile.keys().collect::<Vec<_>>(), ["silver"]);
    assert_eq!(groups_of(&toolmaker, "silver"), [(200.0, None)]);
    assert_eq!(
        factory_rows(&toolmaker),
        [(2, "toolmaker".to_string(), 1.0)]
    );
    let t = &toolmaker.technologies["toolmaker"];
    assert_eq!(t.inputs, qmap(&[("copper", 2.0), ("unskilled_labor", 1.5)]));
    assert_eq!(t.output, "tools");
    assert_eq!(t.capacity.get(), 4.0);
    assert_eq!(t.capital, qmap(&[("tools", 3.0)]));
    assert_eq!(t.construction, qmap(&[("unskilled_labor", 6.0)]));
    assert_eq!((t.construction_turns, t.lifetime), (3, 24));
    assert!(toolmaker.deposits.is_empty());

    world.step().unwrap();
    let farm = view(&world, "center_farm");
    assert_eq!(farm.turn, Turn::new(1));
    assert_eq!(groups_of(&farm, "unimproved_grain"), [(8.0, Some(1))]);
}

#[test]
fn a_business_on_a_hex_with_no_market_sees_no_prices() {
    let (world, _) = world_of(&test_setting("businesses_valid"));

    let idle = view(&world, "idle");
    assert!(idle.prices.is_empty());
    assert!(idle.stockpile.is_empty());
    assert_eq!(idle.cash, Quantity::ZERO);
    assert!(idle.factories.is_empty());
    assert!(idle.technologies.is_empty());
    assert!(idle.deposits.is_empty());
    assert_eq!(idle.currency, "coin");

    let a = view(&world, "farm_a");
    assert_eq!(a.prices, qmap(&[("grain", 2.0), ("ore", 3.0)]));
    assert_eq!(
        a.stockpile.keys().collect::<Vec<_>>(),
        ["coin", "grain", "labor"]
    );
    assert_eq!(groups_of(&a, "coin"), [(10.0, None)]);
    assert_eq!(groups_of(&a, "labor"), [(4.0, Some(0))]);
    assert_eq!(
        deposit_rows(&a),
        [("field".to_string(), 2, "grain".to_string(), 2.0)]
    );
    assert_eq!(groups_of(&a, "grain"), [(4.0, Some(2))]);

    let b = view(&world, "farm_b");
    assert_eq!(groups_of(&b, "coin"), [(5.0, None)]);
    assert_eq!(groups_of(&b, "ore"), [(1.5, None)]);
    assert_eq!(groups_of(&b, "grain"), [(2.0, Some(2))]);
    assert_eq!(
        deposit_rows(&b),
        [
            ("field".to_string(), 1, "grain".to_string(), 2.0),
            ("vein".to_string(), 1, "ore".to_string(), 1.5),
        ]
    );
}

#[test]
fn a_technology_is_listed_once_however_many_factories_run_it() {
    let (world, _) = world_of(&test_setting("production_valid"));
    let works = view(&world, "works");
    assert_eq!(
        factory_rows(&works),
        [
            (0, "smelter".to_string(), 1.0),
            (1, "smelter".to_string(), 0.5),
            (2, "bakery".to_string(), 1.0),
        ]
    );
    assert_eq!(
        works.technologies.keys().collect::<Vec<_>>(),
        ["bakery", "smelter"]
    );
    assert_eq!(groups_of(&works, "coin"), [(50.0, None)]);
    assert_eq!(groups_of(&works, "labor"), [(13.0, Some(0))]);
    assert_eq!(groups_of(&works, "ore"), [(30.0, Some(2))]);
    assert_eq!(groups_of(&works, "tools"), [(3.0, None)]);
    assert_eq!(works.cash.get(), 50.0);
}

#[test]
fn building_views_changes_nothing() {
    let (world, _) = world_of(&sample_setting());
    let ids: Vec<BusinessId> = world.businesses().iter().map(|(id, _, _)| id).collect();
    let stockpiles: Vec<_> = ids
        .iter()
        .map(|id| world.business_stockpile(*id).clone())
        .collect();
    let conditions: Vec<f64> = world.factories().iter().map(|(_, f)| f.condition).collect();

    let first: Vec<BusinessView> = ids.iter().map(|id| world.business_view(*id)).collect();
    let second: Vec<BusinessView> = ids.iter().map(|id| world.business_view(*id)).collect();

    for (id, before) in ids.iter().zip(&stockpiles) {
        assert_eq!(world.business_stockpile(*id), before);
    }
    let after: Vec<f64> = world.factories().iter().map(|(_, f)| f.condition).collect();
    assert_eq!(after, conditions);
    assert_eq!(first, second);
}

#[test]
fn a_decision_validates_against_the_sample_view() {
    let (world, _) = world_of(&sample_setting());
    let goods = &world.rules().goods;
    let farm = view(&world, "center_farm");

    let mut full = Decision::empty("center_farm");
    full.factories.insert(0, order(1.0, 0.125));
    full.offers = table(&[("unimproved_grain", 8.0)]);
    full.bids = table(&[("unskilled_labor", 6.0)]);
    full.trace = Some("sell the grain".to_string());
    assert_eq!(full.validate(&farm, goods), Ok(()));

    assert_eq!(
        Decision::empty("center_farm").validate(&farm, goods),
        Ok(())
    );

    let mut greedy = Decision::empty("center_farm");
    greedy.offers = table(&[("unimproved_grain", 1000.0)]);
    greedy.bids = table(&[("tools", 1000.0)]);
    assert_eq!(greedy.validate(&farm, goods), Ok(()));

    let mut other = Decision::empty("center_farm");
    other.factories.insert(1, order(1.0, 0.0));
    let errors = other.validate(&farm, goods).unwrap_err();
    assert_eq!(
        errors.to_string(),
        "factory #1: not in this business's view"
    );

    let mut currency = Decision::empty("center_farm");
    currency.offers = table(&[("silver", 1.0)]);
    let errors = currency.validate(&farm, goods).unwrap_err();
    assert_eq!(
        errors.to_string(),
        "offer 'silver': it is this hex's currency, which can't be traded against itself"
    );

    let (market_world, _) = world_of(&test_setting("businesses_valid"));
    let idle = view(&market_world, "idle");
    let mut bid = Decision::empty("idle");
    bid.bids = table(&[("grain", 1.0)]);
    let errors = bid
        .validate(&idle, &market_world.rules().goods)
        .unwrap_err();
    assert_eq!(
        errors.to_string(),
        "bid 'grain': no price is posted for it on this hex"
    );
}

#[test]
fn every_message_round_trips_on_the_sample() {
    let (world, _) = world_of(&sample_setting());
    let names = ["center_farm", "center_mine", "center_toolmaker"];
    let views: Vec<BusinessView> = names.iter().map(|n| view(&world, n)).collect();
    let decisions: Vec<Decision> = views.iter().map(full_decision).collect();

    for v in &views {
        assert_eq!(&BusinessView::from_json(&v.to_json()).unwrap(), v);
        assert_eq!(&BusinessView::from_json(&v.to_json_pretty()).unwrap(), v);
    }
    for d in &decisions {
        assert_eq!(&Decision::from_json(&d.to_json()).unwrap(), d);
        assert_eq!(&Decision::from_json(&d.to_json_pretty()).unwrap(), d);
    }
    let request = Request {
        turn: Turn::ZERO,
        views,
    };
    let reply = Reply {
        turn: Turn::ZERO,
        decisions,
    };
    assert_eq!(Request::from_json(&request.to_json()).unwrap(), request);
    assert_eq!(
        Request::from_json(&request.to_json_pretty()).unwrap(),
        request
    );
    assert_eq!(Reply::from_json(&reply.to_json()).unwrap(), reply);
    assert_eq!(Reply::from_json(&reply.to_json_pretty()).unwrap(), reply);

    let recording = Recording { request, reply };
    assert_eq!(
        Recording::from_json(&recording.to_json()).unwrap(),
        recording
    );
    let pretty = recording.to_json_pretty();
    assert_eq!(Recording::from_json(&pretty).unwrap(), recording);
    assert!(
        pretty
            .lines()
            .any(|l| l.trim() == r#""business": "center_farm","#),
        "{pretty}"
    );
}

#[test]
fn a_hand_written_decision_reads_and_validates() {
    let (world, _) = world_of(&sample_setting());
    let text = r#"{"business": "center_farm", "factories": {"0": {"run": 1}}, "offers": {"unimproved_grain": 8}, "bids": {"unskilled_labor": 6}, "trace": "sell the grain"}"#;
    let decision = Decision::from_json(text).unwrap();
    assert_eq!(decision.factories[&0], order(1.0, 0.0));
    assert_eq!(
        decision.validate(&view(&world, "center_farm"), &world.rules().goods),
        Ok(())
    );
}

#[test]
fn a_reply_is_checked_against_its_request() {
    let (world, _) = world_of(&sample_setting());
    let goods = &world.rules().goods;
    let names = ["center_farm", "center_mine", "center_toolmaker"];
    let request = Request {
        turn: Turn::ZERO,
        views: names.iter().map(|n| view(&world, n)).collect(),
    };
    let farm = full_decision(&request.views[0]);
    let toolmaker = full_decision(&request.views[2]);

    let reply = Reply {
        turn: Turn::ZERO,
        decisions: vec![toolmaker.clone(), farm.clone()],
    };
    let answers = reply.validate(&request, goods).unwrap();
    assert_eq!(
        answers,
        [
            Answer::Valid(&reply.decisions[1]),
            Answer::Nothing,
            Answer::Valid(&reply.decisions[0]),
        ]
    );

    let late = Reply {
        turn: Turn::new(1),
        decisions: vec![farm.clone()],
    };
    let errors = late.validate(&request, goods).unwrap_err();
    assert_eq!(
        errors.iter().cloned().collect::<Vec<_>>(),
        [ReplyError::TurnMismatch {
            request: Turn::ZERO,
            reply: Turn::new(1)
        }]
    );

    let unknown = Reply {
        turn: Turn::ZERO,
        decisions: vec![Decision::empty("nobody")],
    };
    let errors = unknown.validate(&request, goods).unwrap_err();
    assert_eq!(
        errors.iter().cloned().collect::<Vec<_>>(),
        [ReplyError::UnknownBusiness {
            name: "nobody".to_string()
        }]
    );

    let twice = Reply {
        turn: Turn::ZERO,
        decisions: vec![
            Decision::empty("center_mine"),
            Decision::empty("center_mine"),
        ],
    };
    let errors = twice.validate(&request, goods).unwrap_err();
    assert_eq!(
        errors.iter().cloned().collect::<Vec<_>>(),
        [ReplyError::DuplicateBusiness {
            name: "center_mine".to_string()
        }]
    );
}
