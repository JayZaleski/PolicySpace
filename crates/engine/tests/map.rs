//! Integration tests for the map, through the engine's public API only.

mod common;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{HexCoord, LoadedSetting};

fn list(setting: &LoadedSetting) -> Vec<(usize, HexCoord)> {
    setting
        .start
        .map
        .iter()
        .map(|(id, coord)| (id.index(), coord))
        .collect()
}

#[test]
fn the_sample_setting_has_its_nineteen_hexes_in_order() {
    let setting = load(&sample_setting());
    assert_eq!(setting.start.map.len(), 19);
    let rows: [(i32, &[i32]); 5] = [
        (-2, &[0, 1, 2]),
        (-1, &[-1, 0, 1, 2]),
        (0, &[-2, -1, 0, 1, 2]),
        (1, &[-2, -1, 0, 1]),
        (2, &[-2, -1, 0]),
    ];
    let expected: Vec<(usize, HexCoord)> = rows
        .iter()
        .flat_map(|(r, qs)| qs.iter().map(move |q| HexCoord::new(*q, *r)))
        .enumerate()
        .collect();
    assert_eq!(expected.len(), 19);
    assert_eq!(list(&setting), expected);
}

#[test]
fn hexes_across_files_and_folders_get_ids_in_load_order() {
    let setting = load(&test_setting("hexes_valid"));
    let map = &setting.start.map;
    assert_eq!(map.len(), 4);
    assert!(!map.is_empty());
    let expected = vec![
        (0, HexCoord::new(0, 0)),
        (1, HexCoord::new(1, -1)),
        (2, HexCoord::new(-3, 2)),
        (3, HexCoord::new(i32::MAX, i32::MIN)),
    ];
    assert_eq!(list(&setting), expected);
    for (id, coord) in map.iter() {
        assert_eq!(map.id(coord), Some(id));
        assert_eq!(map.coord(id), coord);
    }
}

#[test]
fn loading_twice_gives_the_same_ids() {
    let first = list(&load(&test_setting("hexes_valid")));
    let second = list(&load(&test_setting("hexes_valid")));
    assert_eq!(first, second);
}

#[test]
fn looking_up_coordinates_with_no_hex_gives_none() {
    let setting = load(&test_setting("hexes_valid"));
    assert_eq!(setting.start.map.id(HexCoord::new(9, 9)), None);
}

#[test]
fn a_setting_with_no_hexes_loads() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.start.map.is_empty());
    assert_eq!(setting.start.map.len(), 0);
}

#[test]
fn every_hex_error_is_reported_at_its_place() {
    let shown = failures_of("hexes_errors");
    let expected = vec![
        "rules/h.toml:1:3: section 'hex' belongs in start/, not rules/",
        "start/a.toml:6:1: missing required key 'q' in hex",
        "start/a.toml:10:1: missing required key 'r' in hex",
        "start/a.toml:15:5: key 'q' in hex must be an integer, found float",
        "start/a.toml:20:5: key 'q' in hex must be an integer, found string",
        "start/a.toml:25:5: key 'q' in hex is out of range",
        "start/a.toml:31:5: key 'r' in hex is out of range",
        "start/a.toml:38:1: unknown key 'terrain' in hex",
        "start/a.toml:40:1: missing required key 'currency' in hex",
        "start/a.toml:47:12: key 'currency' in hex must be a string, found integer",
        "start/a.toml:52:12: unknown good 'Coin'",
        "start/a.toml:57:12: unknown good 'tin'",
        "start/a.toml:60:5: key 'q' in hex must be an integer, found float",
        "start/a.toml:61:5: key 'r' in hex is out of range",
        "start/a.toml:63:1: unknown key 'terrain' in hex",
        "start/b.toml:1:1: duplicate hex at (0, 0) (first defined at start/a.toml:1:1)",
    ];
    assert_eq!(shown, expected);
}
