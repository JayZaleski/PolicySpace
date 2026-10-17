//! Integration tests for businesses, through the engine's public API only.

mod common;

use common::{
    end, failures_of, good, groups, hex, load, named, pair, sample_setting, stockpile_totals,
    test_setting, world_of,
};
use policyspace_engine::{
    BusinessId, HexCoord, IdError, LoadedSetting, PerishesAt, Quantity, TurnReport, World,
};

/// One business as (index, name, coordinates, decider, starting inventory as (good name,
/// amount)).
type BusinessRow = (usize, String, HexCoord, String, Vec<(String, f64)>);

fn s(text: &str) -> String {
    text.to_string()
}

fn businesses_of(setting: &LoadedSetting) -> Vec<BusinessRow> {
    let goods = &setting.rules.goods;
    setting
        .start
        .businesses
        .iter()
        .map(|(id, name, business)| {
            (
                id.index(),
                name.to_string(),
                setting.start.map.coord(business.hex),
                business.decider.clone(),
                business
                    .starting_inventory
                    .iter()
                    .map(|(g, amount)| (goods.name(g).to_string(), amount.get()))
                    .collect(),
            )
        })
        .collect()
}

fn row(
    index: usize,
    name: &str,
    q: i32,
    r: i32,
    decider: &str,
    inventory: &[(&str, f64)],
) -> BusinessRow {
    (
        index,
        s(name),
        HexCoord { q, r },
        s(decider),
        inventory.iter().map(|(g, a)| (s(g), *a)).collect(),
    )
}

/// Each deposit's owner (a business index) in deposit ID order.
fn owners_of(setting: &LoadedSetting) -> Vec<Option<usize>> {
    setting
        .start
        .deposits
        .iter()
        .map(|(_, deposit)| deposit.owner.map(|b| b.index()))
        .collect()
}

/// The deposit indexes `owned_by` lists for the business whose index is `index`.
fn owned(setting: &LoadedSetting, index: usize) -> Vec<usize> {
    let id = setting
        .start
        .businesses
        .iter()
        .map(|(id, _, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no business {index}"));
    setting
        .start
        .deposits
        .owned_by(id)
        .map(|d| d.index())
        .collect()
}

/// The business whose index is `index`, panicking if there is none.
fn business_id(index: usize, world: &World) -> BusinessId {
    world
        .businesses()
        .iter()
        .map(|(id, _, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no business {index}"))
}

/// A business's stockpile as (good name, total).
fn business_totals(world: &World, id: BusinessId) -> Vec<(String, f64)> {
    stockpile_totals(world, world.business_stockpile(id))
}

fn hex_totals(world: &World, q: i32, r: i32) -> Vec<(String, f64)> {
    stockpile_totals(world, world.hex_stockpile(hex(world, q, r)))
}

fn indexes(ids: &[BusinessId]) -> Vec<usize> {
    ids.iter().map(|id| id.index()).collect()
}

fn hex_of(setting: &LoadedSetting, q: i32, r: i32) -> policyspace_engine::HexId {
    setting.start.map.id(HexCoord { q, r }).unwrap()
}

#[test]
fn the_sample_setting_loads_with_three_businesses() {
    let setting = load(&sample_setting());
    assert_eq!(setting.start.businesses.len(), 3);
    assert_eq!(
        businesses_of(&setting),
        vec![
            row(0, "center_farm", 0, 0, "arithmetic", &[("silver", 100.0)]),
            row(1, "center_mine", 0, 0, "arithmetic", &[("silver", 50.0)]),
            row(
                2,
                "center_toolmaker",
                0,
                0,
                "arithmetic",
                &[("silver", 200.0)]
            ),
        ]
    );
    let businesses = &setting.start.businesses;
    assert_eq!(businesses.id("center_farm").unwrap().index(), 0);
    assert_eq!(
        businesses.id("farm"),
        Err(IdError::UnknownName {
            kind: "business",
            name: s("farm")
        })
    );
    assert_eq!(
        indexes(businesses.on_hex(hex_of(&setting, 0, 0))),
        vec![0, 1, 2]
    );
    assert!(businesses.on_hex(hex_of(&setting, 2, -2)).is_empty());
    assert_eq!(
        owners_of(&setting),
        vec![
            Some(0),
            Some(0),
            None,
            Some(1),
            None,
            None,
            None,
            None,
            None,
            None
        ]
    );
    assert_eq!(owned(&setting, 0), vec![0, 1]);
    assert_eq!(owned(&setting, 1), vec![3]);
    assert!(owned(&setting, 2).is_empty());
}

/// Asserts the sample world's business and center stockpiles as they are at the end of
/// turn-`t` part 2 (output at `end(t)`).
fn assert_sample_stockpiles(world: &World, t: u64) {
    let farm = business_id(0, world);
    let mine = business_id(1, world);
    let toolmaker = business_id(2, world);
    assert_eq!(
        business_totals(world, farm),
        vec![pair("unimproved_grain", 8.0), pair("silver", 100.0)],
        "turn {t}"
    );
    assert_eq!(
        groups(
            world.business_stockpile(farm),
            good(world, "unimproved_grain")
        ),
        vec![(end(t), 8.0)],
        "turn {t}"
    );
    assert_eq!(
        groups(world.business_stockpile(farm), good(world, "silver")),
        vec![(PerishesAt::Never, 100.0)],
        "turn {t}"
    );
    assert_eq!(
        business_totals(world, mine),
        vec![pair("unimproved_copper", 2.5), pair("silver", 50.0)],
        "turn {t}"
    );
    assert_eq!(
        groups(
            world.business_stockpile(mine),
            good(world, "unimproved_copper")
        ),
        vec![(end(t), 2.5)],
        "turn {t}"
    );
    assert_eq!(
        business_totals(world, toolmaker),
        vec![pair("silver", 200.0)],
        "turn {t}"
    );
    assert_eq!(
        hex_totals(world, 0, 0),
        vec![pair("unimproved_grain", 4.0)],
        "turn {t}"
    );
    assert_eq!(
        groups(
            world.hex_stockpile(hex(world, 0, 0)),
            good(world, "unimproved_grain")
        ),
        vec![(end(t), 4.0)],
        "turn {t}"
    );
}

/// The report's `perished`, without slivers of rounding.
fn big_perished(world: &World, report: &TurnReport) -> Vec<(String, f64)> {
    named(world, &report.perished)
        .into_iter()
        .filter(|(_, a)| *a >= 1e-9)
        .collect()
}

#[test]
fn a_fresh_sample_world_fills_business_stockpiles_and_emits_to_owners() {
    let (mut world, report) = world_of(&sample_setting());
    assert_eq!(world.businesses().len(), 3);
    assert_sample_stockpiles(&world, 0);
    assert_eq!(
        hex_totals(&world, -1, 0),
        vec![pair("unimproved_grain", 8.0)]
    );
    assert_eq!(
        hex_totals(&world, 1, -1),
        vec![pair("unimproved_copper", 5.0)]
    );
    assert!(world.hex_stockpile(hex(&world, 2, -2)).is_empty());
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
    for t in 1..=3u64 {
        let report = world.step().unwrap();
        assert_sample_stockpiles(&world, t);
        assert_eq!(
            big_perished(&world, &report),
            vec![
                pair("unimproved_copper", 10.0),
                pair("unimproved_grain", 24.0)
            ],
            "turn {t}"
        );
        assert_eq!(
            named(&world, &report.emitted),
            vec![
                pair("unimproved_copper", 10.0),
                pair("unimproved_grain", 24.0)
            ],
            "turn {t}"
        );
    }
}

#[test]
fn identical_sample_runs_give_identical_reports_and_stockpiles() {
    let (mut a, report_a) = world_of(&sample_setting());
    let (mut b, report_b) = world_of(&sample_setting());
    assert_eq!(report_a, report_b);
    for _ in 0..3 {
        assert_eq!(a.step().unwrap(), b.step().unwrap());
        for (id, _, _) in a.businesses().iter() {
            assert_eq!(a.business_stockpile(id), b.business_stockpile(id));
        }
        for (id, _) in a.map().iter() {
            assert_eq!(a.hex_stockpile(id), b.hex_stockpile(id));
        }
        for (id, _) in a.pops().iter() {
            assert_eq!(a.pop_stockpile(id), b.pop_stockpile(id));
        }
    }
}

fn valid_rows() -> Vec<BusinessRow> {
    vec![
        row(0, "farm_a", 0, 0, "rule", &[("coin", 10.0), ("labor", 4.0)]),
        row(1, "farm_b", 0, 0, "rule", &[("coin", 5.0)]),
        row(2, "idle", 1, 0, "other_rule", &[]),
    ]
}

#[test]
fn businesses_load_across_files_with_forward_references() {
    let setting = load(&test_setting("businesses_valid"));
    assert_eq!(businesses_of(&setting), valid_rows());
    let businesses = &setting.start.businesses;
    assert_eq!(
        indexes(businesses.on_hex(hex_of(&setting, 0, 0))),
        vec![0, 1]
    );
    assert_eq!(indexes(businesses.on_hex(hex_of(&setting, 1, 0))), vec![2]);
    assert_eq!(
        owners_of(&setting),
        vec![Some(0), Some(0), Some(1), Some(1), None]
    );
    assert_eq!(owned(&setting, 0), vec![0, 1]);
    assert_eq!(owned(&setting, 1), vec![2, 3]);
    assert!(owned(&setting, 2).is_empty());
    assert!(setting.start.prices.has_market(hex_of(&setting, 0, 0)));
    assert!(!setting.start.prices.has_market(hex_of(&setting, 1, 0)));

    let again = load(&test_setting("businesses_valid"));
    assert_eq!(businesses_of(&again), businesses_of(&setting));
    assert_eq!(owners_of(&again), owners_of(&setting));
}

#[test]
fn businesses_valid_world_runs_turns_0_to_3() {
    let (mut world, report) = world_of(&test_setting("businesses_valid"));
    let farm_a = business_id(0, &world);
    let farm_b = business_id(1, &world);
    let idle = business_id(2, &world);
    let grain = good(&world, "grain");
    let labor = good(&world, "labor");
    let coin = good(&world, "coin");

    // Turn 0.
    assert_eq!(
        business_totals(&world, farm_a),
        vec![pair("coin", 10.0), pair("labor", 4.0), pair("grain", 4.0)]
    );
    let stock = world.business_stockpile(farm_a);
    assert_eq!(groups(stock, coin), vec![(PerishesAt::Never, 10.0)]);
    assert_eq!(groups(stock, labor), vec![(end(0), 4.0)]);
    assert_eq!(groups(stock, grain), vec![(end(2), 4.0)]);
    assert_eq!(
        business_totals(&world, farm_b),
        vec![pair("coin", 5.0), pair("grain", 2.0), pair("ore", 1.5)]
    );
    assert!(world.business_stockpile(idle).is_empty());
    assert!(world.hex_stockpile(hex(&world, 0, 0)).is_empty());
    assert_eq!(hex_totals(&world, 1, 0), vec![pair("grain", 2.0)]);
    check_report(&world, &report, 0, &[], 8.0);

    // Turn 1.
    let report = world.step().unwrap();
    assert_eq!(
        business_totals(&world, farm_a),
        vec![pair("coin", 10.0), pair("grain", 8.0)]
    );
    assert_eq!(
        groups(world.business_stockpile(farm_a), grain),
        vec![(end(2), 4.0), (end(3), 4.0)]
    );
    assert_eq!(
        business_totals(&world, farm_b),
        vec![pair("coin", 5.0), pair("grain", 4.0), pair("ore", 3.0)]
    );
    assert!(world.business_stockpile(idle).is_empty());
    assert!(world.hex_stockpile(hex(&world, 0, 0)).is_empty());
    assert_eq!(hex_totals(&world, 1, 0), vec![pair("grain", 4.0)]);
    check_report(&world, &report, 1, &[pair("labor", 4.0)], 8.0);

    // Turn 2.
    let report = world.step().unwrap();
    assert_eq!(
        business_totals(&world, farm_a),
        vec![pair("coin", 10.0), pair("grain", 12.0)]
    );
    assert_eq!(
        business_totals(&world, farm_b),
        vec![pair("coin", 5.0), pair("grain", 6.0), pair("ore", 4.5)]
    );
    assert_eq!(hex_totals(&world, 1, 0), vec![pair("grain", 6.0)]);
    check_report(&world, &report, 2, &[], 8.0);

    // Turn 3: the first grain perishes, from the businesses and from the hex.
    let report = world.step().unwrap();
    assert_eq!(
        business_totals(&world, farm_a),
        vec![pair("coin", 10.0), pair("grain", 12.0)]
    );
    assert_eq!(
        groups(world.business_stockpile(farm_a), grain),
        vec![(end(3), 4.0), (end(4), 4.0), (end(5), 4.0)]
    );
    assert_eq!(
        business_totals(&world, farm_b),
        vec![pair("coin", 5.0), pair("grain", 6.0), pair("ore", 6.0)]
    );
    assert!(world.business_stockpile(idle).is_empty());
    assert_eq!(hex_totals(&world, 1, 0), vec![pair("grain", 6.0)]);
    check_report(&world, &report, 3, &[pair("grain", 8.0)], 8.0);
}

/// Checks a `businesses_valid` report: its turn, what perished, what was emitted (the
/// grain total and 1.5 ore) and that nothing else happened.
fn check_report(
    world: &World,
    report: &TurnReport,
    turn: u64,
    perished: &[(String, f64)],
    grain: f64,
) {
    assert_eq!(report.turn.get(), turn);
    assert_eq!(named(world, &report.perished), perished, "turn {turn}");
    assert_eq!(
        named(world, &report.emitted),
        vec![pair("grain", grain), pair("ore", 1.5)],
        "turn {turn}"
    );
    assert!(report.endowed.is_empty(), "turn {turn}");
    assert!(report.sold.is_empty(), "turn {turn}");
    assert!(report.bought.is_empty(), "turn {turn}");
    assert!(report.consumed.is_empty(), "turn {turn}");
    assert_eq!(report.population, Quantity::ZERO, "turn {turn}");
}

#[test]
fn every_business_error_is_reported_at_its_place() {
    let help = "(names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)";
    let positive = "must be a finite number greater than 0";
    let expected: Vec<String> = vec![
        s("rules/x.toml:1:3: section 'business' belongs in start/, not rules/"),
        s("start/a.toml:9:1: missing required key 'name' in business"),
        s("start/a.toml:16:1: missing required key 'q' in business"),
        s("start/a.toml:23:1: missing required key 'r' in business"),
        s("start/a.toml:30:1: missing required key 'inventory' in business"),
        s("start/a.toml:37:1: missing required key 'deposits' in business"),
        s("start/a.toml:44:1: missing required key 'decider' in business"),
        format!("start/a.toml:52:8: invalid name 'Bad Name' {help}"),
        s("start/a.toml:60:8: duplicate business name 'ok' (first defined at start/a.toml:2:8)"),
        s("start/a.toml:64:22: key 'field' in deposits of business must be at least 1"),
        s("start/a.toml:69:5: key 'q' in business must be an integer, found float"),
        s("start/a.toml:70:5: key 'r' in business must be an integer, found string"),
        s("start/a.toml:71:13: key 'inventory' in business must be a table, found integer"),
        s("start/a.toml:72:12: key 'deposits' in business must be a table, found string"),
        s("start/a.toml:73:11: key 'decider' in business must be a string, found integer"),
        s("start/a.toml:77:5: key 'q' in business is out of range"),
        s("start/a.toml:78:5: key 'r' in business is out of range"),
        s("start/a.toml:83:1: no hex at (9, 9)"),
        s("start/a.toml:95:22: key 'coin' in inventory of business must be a number, found string"),
        format!("start/a.toml:95:35: key 'grain' in inventory of business {positive}"),
        s("start/a.toml:95:38: unknown good 'tin'"),
        format!("start/a.toml:95:44: key 'tin' in inventory of business {positive}"),
        format!("start/a.toml:103:22: key 'coin' in inventory of business {positive}"),
        format!("start/a.toml:103:35: key 'grain' in inventory of business {positive}"),
        s("start/a.toml:112:22: key 'field' in deposits of business must be at least 1"),
        s("start/a.toml:112:25: unknown resource 'meadow'"),
        s("start/a.toml:112:34: key 'meadow' in deposits of business must be at least 1"),
        s(
            "start/a.toml:120:22: key 'field' in deposits of business must be an integer, found float",
        ),
        s("start/a.toml:120:27: unknown resource 'meadow'"),
        s(
            "start/a.toml:120:36: key 'meadow' in deposits of business must be an integer, found string",
        ),
        s("start/a.toml:120:41: unknown resource 'bog'"),
        s("start/a.toml:120:47: key 'bog' in deposits of business is out of range"),
        s("start/a.toml:127:15: unknown good 'tin'"),
        s("start/a.toml:128:14: unknown resource 'gold_vein'"),
        s(
            "start/a.toml:136:14: key 'field' in deposits of business claims more deposits than the hex has unowned (3 claimed, 1 available)",
        ),
        format!("start/a.toml:145:11: invalid name 'Rule One' {help}"),
        s("start/a.toml:154:1: unknown key 'colour' in business"),
        s("start/a.toml:155:1: unknown key 'cash' in business"),
        s("start/a.toml:157:1: no hex at (0, 10)"),
        format!("start/a.toml:158:8: invalid name 'Several' {help}"),
        s("start/a.toml:161:15: unknown good 'tin'"),
        format!("start/a.toml:161:21: key 'tin' in inventory of business {positive}"),
        s("start/a.toml:162:14: unknown resource 'gold_vein'"),
        s("start/a.toml:162:26: key 'gold_vein' in deposits of business must be at least 1"),
        format!("start/a.toml:163:11: invalid name 'Bad' {help}"),
        s("start/a.toml:164:1: unknown key 'colour' in business"),
        s("start/b.toml:2:8: duplicate business name 'ok' (first defined at start/a.toml:2:8)"),
        s(
            "start/b.toml:14:14: key 'field' in deposits of business claims more deposits than the hex has unowned (2 claimed, 1 available)",
        ),
    ];
    assert_eq!(failures_of("businesses_errors"), expected);
}

#[test]
fn a_setting_with_no_businesses_is_valid() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.start.businesses.is_empty());

    let setting = load(&test_setting("turns_valid"));
    assert!(setting.start.businesses.is_empty());
    assert!(
        setting
            .start
            .deposits
            .iter()
            .all(|(_, d)| d.owner.is_none())
    );
    let (world, _) = world_of(&test_setting("turns_valid"));
    assert_eq!(world.businesses().len(), 0);
    assert!(world.deposits().iter().all(|(_, d)| d.owner.is_none()));
}

#[test]
fn the_one_business_setting_loads() {
    let setting = load(&test_setting("businesses_one"));
    assert_eq!(
        businesses_of(&setting),
        vec![row(0, "solo", 0, 0, "rule", &[])]
    );
}

#[test]
#[should_panic(expected = "is not from this list")]
fn business_stockpile_panics_on_a_business_from_another_world() {
    let (valid, _) = world_of(&test_setting("businesses_valid"));
    let foreign = business_id(2, &valid);
    let (world, _) = world_of(&test_setting("businesses_one"));
    world.business_stockpile(foreign);
}
