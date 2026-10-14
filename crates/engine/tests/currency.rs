//! Integration tests for hex currencies, through the engine's public API only.

mod common;

use common::{load, sample_setting, test_setting};
use policyspace_engine::{GoodId, LoadedSetting};

/// The currency of every hex as a good name, in hex ID order.
fn names(setting: &LoadedSetting) -> Vec<String> {
    setting
        .start
        .currencies
        .iter()
        .map(|(_, good)| setting.rules.goods.name(good).to_string())
        .collect()
}

#[test]
fn the_sample_setting_designates_silver_on_every_hex() {
    let setting = load(&sample_setting());
    let silver: GoodId = setting.rules.goods.id("silver").unwrap();
    assert_eq!(setting.start.currencies.len(), 19);
    assert_eq!(setting.start.currencies.len(), setting.start.map.len());
    for (_, good) in setting.start.currencies.iter() {
        assert_eq!(good, silver);
    }
    for (hex, _) in setting.start.map.iter() {
        assert_eq!(setting.start.currencies.get(hex), silver);
    }
}

#[test]
fn different_hexes_can_designate_different_currencies() {
    let setting = load(&test_setting("hexes_valid"));
    assert!(!setting.start.currencies.is_empty());
    assert_eq!(names(&setting), vec!["coin", "shell", "coin", "shell"]);
}

#[test]
fn a_setting_without_hexes_has_no_currencies() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.start.currencies.is_empty());
    assert_eq!(setting.start.currencies.len(), 0);
}

#[test]
fn loading_twice_gives_the_same_currencies() {
    let first = load(&test_setting("hexes_valid"));
    let second = load(&test_setting("hexes_valid"));
    let list = |setting: &LoadedSetting| -> Vec<_> { setting.start.currencies.iter().collect() };
    assert_eq!(list(&first), list(&second));
}
