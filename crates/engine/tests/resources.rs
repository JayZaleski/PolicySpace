//! Integration tests for natural resources (resource types and deposits), through the
//! engine's public API only.

mod common;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{DepositId, HexCoord, IdError, LoadedSetting, Quantity};

/// Resource types as (index, name, produced good's name, output).
fn types_of(setting: &LoadedSetting) -> Vec<(usize, String, String, f64)> {
    setting
        .rules
        .resources
        .iter()
        .map(|(id, name, t)| {
            (
                id.index(),
                name.to_string(),
                setting.rules.goods.name(t.produces).to_string(),
                t.output.get(),
            )
        })
        .collect()
}

/// Deposits as (index, resource name, hex coordinates).
fn deposits_of(setting: &LoadedSetting) -> Vec<(usize, String, HexCoord)> {
    setting
        .start
        .deposits
        .iter()
        .map(|(id, d)| {
            (
                id.index(),
                setting.rules.resources.name(d.resource).to_string(),
                setting.start.map.coord(d.hex),
            )
        })
        .collect()
}

fn s(text: &str) -> String {
    text.to_string()
}

fn indexes(ids: impl IntoIterator<Item = DepositId>) -> Vec<usize> {
    ids.into_iter().map(|id| id.index()).collect()
}

#[test]
fn the_sample_setting_has_its_resource_types_and_ten_deposits() {
    let path = sample_setting();
    let setting = load(&path);
    assert_eq!(
        types_of(&setting),
        vec![
            (0, s("copper_ore"), s("unimproved_copper"), 2.5),
            (1, s("arable_land"), s("unimproved_grain"), 4.0),
        ]
    );
    let copper = "copper_ore";
    let land = "arable_land";
    let expected = vec![
        (0, s(land), HexCoord::new(0, 0)),
        (1, s(land), HexCoord::new(0, 0)),
        (2, s(land), HexCoord::new(0, 0)),
        (3, s(copper), HexCoord::new(0, 0)),
        (4, s(land), HexCoord::new(-1, 0)),
        (5, s(land), HexCoord::new(-1, 0)),
        (6, s(land), HexCoord::new(1, 0)),
        (7, s(copper), HexCoord::new(1, -1)),
        (8, s(copper), HexCoord::new(1, -1)),
        (9, s(copper), HexCoord::new(-2, 1)),
    ];
    assert_eq!(setting.start.deposits.len(), 10);
    assert_eq!(deposits_of(&setting), expected);
}

#[test]
fn resources_and_deposits_across_files_and_folders_get_ids_in_load_order() {
    let setting = load(&test_setting("resources_valid"));
    let resources = &setting.rules.resources;
    assert_eq!(resources.len(), 4);
    assert!(!resources.is_empty());
    assert_eq!(
        types_of(&setting),
        vec![
            (0, s("copper_ore"), s("ore"), 2.5),
            (1, s("iron_ore"), s("ore"), 3.0),
            (2, s("farmland"), s("grain"), 4.0),
            (3, s("unused"), s("grain"), 0.25),
        ]
    );
    for (id, name, _) in resources.iter() {
        assert_eq!(resources.id(name), Ok(id));
        assert_eq!(resources.name(id), name);
    }
    let copper = resources.id("copper_ore").unwrap();
    let iron = resources.id("iron_ore").unwrap();
    assert_eq!(resources.get(copper).produces, resources.get(iron).produces);
    assert_eq!(resources.get(copper).output, Quantity::new(2.5).unwrap());

    let at_origin = HexCoord::new(0, 0);
    let at_east = HexCoord::new(1, 0);
    assert_eq!(
        deposits_of(&setting),
        vec![
            (0, s("copper_ore"), at_origin),
            (1, s("copper_ore"), at_origin),
            (2, s("farmland"), at_origin),
            (3, s("iron_ore"), at_east),
            (4, s("iron_ore"), at_east),
            (5, s("iron_ore"), at_east),
        ]
    );
    assert_eq!(setting.start.deposits.len(), 6);
    assert!(!setting.start.deposits.is_empty());
    for (id, deposit) in setting.start.deposits.iter() {
        assert_eq!(setting.start.deposits.get(id), deposit);
    }
}

#[test]
fn deposits_can_be_listed_by_hex_and_by_type() {
    let setting = load(&test_setting("resources_valid"));
    let deposits = &setting.start.deposits;
    let map = &setting.start.map;
    let origin = map.id(HexCoord::new(0, 0)).unwrap();
    let east = map.id(HexCoord::new(1, 0)).unwrap();
    let south = map.id(HexCoord::new(0, 1)).unwrap();
    assert_eq!(
        indexes(deposits.on_hex(origin).iter().copied()),
        vec![0, 1, 2]
    );
    assert_eq!(
        indexes(deposits.on_hex(east).iter().copied()),
        vec![3, 4, 5]
    );
    assert!(deposits.on_hex(south).is_empty());
    let copper = setting.rules.resources.id("copper_ore").unwrap();
    let iron = setting.rules.resources.id("iron_ore").unwrap();
    assert_eq!(indexes(deposits.of_type_on_hex(origin, copper)), vec![0, 1]);
    assert_eq!(
        indexes(deposits.of_type_on_hex(origin, iron)),
        Vec::<usize>::new()
    );
}

#[test]
fn count_places_that_many_deposits_and_defaults_to_one() {
    let setting = load(&test_setting("resources_valid"));
    let deposits = &setting.start.deposits;
    let map = &setting.start.map;
    let origin = map.id(HexCoord::new(0, 0)).unwrap();
    let east = map.id(HexCoord::new(1, 0)).unwrap();
    let copper = setting.rules.resources.id("copper_ore").unwrap();
    let farmland = setting.rules.resources.id("farmland").unwrap();
    let iron = setting.rules.resources.id("iron_ore").unwrap();
    assert_eq!(deposits.of_type_on_hex(origin, copper).count(), 2);
    assert_eq!(deposits.of_type_on_hex(origin, farmland).count(), 1);
    assert_eq!(deposits.of_type_on_hex(east, iron).count(), 3);
}

#[test]
fn loading_twice_gives_the_same_ids() {
    let first = load(&test_setting("resources_valid"));
    let second = load(&test_setting("resources_valid"));
    assert_eq!(types_of(&first), types_of(&second));
    assert_eq!(deposits_of(&first), deposits_of(&second));
}

#[test]
fn looking_up_an_unknown_resource_name_is_an_error() {
    let setting = load(&test_setting("resources_valid"));
    assert_eq!(
        setting.rules.resources.id("missing"),
        Err(IdError::UnknownName {
            kind: "resource",
            name: s("missing")
        })
    );
}

#[test]
fn a_setting_with_no_resources_or_deposits_loads() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.rules.resources.is_empty());
    assert_eq!(setting.rules.resources.len(), 0);
    assert!(setting.start.deposits.is_empty());
    assert_eq!(setting.start.deposits.len(), 0);
}

#[test]
fn references_resolve_even_when_the_target_is_in_a_later_file() {
    // In `resources_valid` the resource types are in rules/a.toml and rules/b.toml, but
    // the goods they produce are in rules/z.toml, and the deposits in start/a.toml are
    // read before the hexes in start/hexes.toml.
    let setting = load(&test_setting("resources_valid"));
    let ore = setting.rules.goods.id("ore").unwrap();
    let copper = setting.rules.resources.id("copper_ore").unwrap();
    assert_eq!(setting.rules.resources.get(copper).produces, ore);
    assert_eq!(setting.start.deposits.len(), 6);
}

#[test]
fn every_resource_error_is_reported_at_its_place() {
    let help = "(names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)";
    let output_error = "key 'output' in resource must be a finite number greater than 0";
    let produces_error = "key 'produces' in resource must be a string, found integer";
    let expected = vec![
        s("rules/a.toml:9:1: missing required key 'name' in resource"),
        format!("rules/a.toml:14:8: invalid name 'Copper' {help}"),
        format!("rules/a.toml:21:10: {output_error}"),
        format!("rules/a.toml:26:10: {output_error}"),
        format!("rules/a.toml:31:10: {output_error}"),
        format!("rules/a.toml:36:10: {output_error}"),
        format!("rules/a.toml:41:10: {output_error}"),
        s("rules/a.toml:46:10: key 'output' in resource must be a number, found string"),
        s("rules/a.toml:48:1: missing required key 'produces' in resource"),
        s("rules/a.toml:52:1: missing required key 'output' in resource"),
        format!("rules/a.toml:58:12: {produces_error}"),
        s("rules/a.toml:63:12: unknown good 'gold'"),
        s("rules/a.toml:70:1: unknown key 'amount' in resource"),
        format!("rules/a.toml:73:8: invalid name 'Bad-Name' {help}"),
        format!("rules/a.toml:74:12: {produces_error}"),
        format!("rules/a.toml:75:10: {output_error}"),
        s("rules/a.toml:76:1: unknown key 'colour' in resource"),
        s(
            "rules/b.toml:2:8: duplicate resource name 'copper_ore' (first defined at rules/a.toml:5:8)",
        ),
        s("start/s.toml:1:3: section 'resource' belongs in rules/, not start/"),
    ];
    assert_eq!(failures_of("resources_errors"), expected);
}

#[test]
fn every_deposit_error_is_reported_at_its_place() {
    let expected = vec![
        "rules/d.toml:1:3: section 'deposit' belongs in start/, not rules/",
        "start/a.toml:6:1: missing required key 'resource' in deposit",
        "start/a.toml:10:1: missing required key 'q' in deposit",
        "start/a.toml:14:1: missing required key 'r' in deposit",
        "start/a.toml:20:5: key 'q' in deposit must be an integer, found float",
        "start/a.toml:25:5: key 'q' in deposit is out of range",
        "start/a.toml:32:9: key 'count' in deposit must be at least 1",
        "start/a.toml:38:9: key 'count' in deposit must be at least 1",
        "start/a.toml:44:9: key 'count' in deposit is out of range (the most is 1000000)",
        "start/a.toml:50:9: key 'count' in deposit must be an integer, found float",
        "start/a.toml:53:12: unknown resource 'gold_deposit'",
        "start/a.toml:57:1: no hex at (9, 9)",
        "start/a.toml:66:1: unknown key 'output' in deposit",
        "start/a.toml:69:12: key 'resource' in deposit must be a string, found integer",
        "start/a.toml:74:12: unknown resource 'gold_deposit'",
        "start/a.toml:77:9: key 'count' in deposit must be at least 1",
        "start/a.toml:78:1: unknown key 'colour' in deposit",
        "start/b.toml:1:1: duplicate deposit of 'ore_deposit' at (0, 0) (first defined at start/a.toml:1:1)",
    ];
    assert_eq!(failures_of("deposits_errors"), expected);
}

#[test]
fn references_are_not_checked_after_a_broken_file_or_incomplete_discovery() {
    assert_eq!(
        failures_of("resources_broken_file"),
        vec!["rules/broken.toml:1:11: invalid TOML: unclosed array table, expected `]]`"]
    );
    assert_eq!(
        failures_of("resources_missing_start"),
        vec!["start/: missing required folder"]
    );
}
