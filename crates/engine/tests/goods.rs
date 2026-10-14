//! Integration tests for goods, through the engine's public API only.

mod common;

use std::num::NonZeroU32;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{Good, IdError, Perishes};

fn after(n: u32) -> Perishes {
    Perishes::After(NonZeroU32::new(n).unwrap())
}

fn good(perishes: Perishes, movable: bool) -> Good {
    Good { perishes, movable }
}

#[test]
fn the_sample_setting_has_its_six_goods_in_order() {
    let goods = load(&sample_setting()).rules.goods;
    let listed: Vec<(usize, &str, Good)> = goods
        .iter()
        .map(|(id, name, good)| (id.index(), name, *good))
        .collect();
    assert_eq!(
        listed,
        vec![
            (0, "unskilled_labor", good(after(1), false)),
            (1, "unimproved_copper", good(after(1), false)),
            (2, "unimproved_grain", good(after(1), false)),
            (3, "copper", good(Perishes::Never, true)),
            (4, "grain", good(after(3), true)),
            (5, "silver", good(Perishes::Never, true)),
        ]
    );
}

#[test]
fn goods_across_files_and_folders_get_ids_in_load_order() {
    let goods = load(&test_setting("goods_valid")).rules.goods;
    assert_eq!(goods.len(), 4);
    assert!(!goods.is_empty());
    let listed: Vec<(usize, &str)> = goods
        .iter()
        .map(|(id, name, _)| (id.index(), name))
        .collect();
    assert_eq!(
        listed,
        vec![(0, "grain"), (1, "copper"), (2, "labor"), (3, "tools")]
    );

    let labor = goods.id("labor").unwrap();
    assert_eq!(labor.index(), 2);
    for (id, name, properties) in goods.iter() {
        assert_eq!(goods.name(id), name);
        assert_eq!(goods.id(name).unwrap(), id);
        assert_eq!(goods.get(id), properties);
    }
    assert_eq!(*goods.get(goods.id("grain").unwrap()), good(after(3), true));
    assert_eq!(*goods.get(labor), good(after(1), false));
    assert_eq!(
        *goods.get(goods.id("tools").unwrap()),
        good(Perishes::Never, true)
    );
}

#[test]
fn a_good_with_no_property_keys_never_perishes_and_is_movable() {
    let goods = load(&test_setting("goods_valid")).rules.goods;
    let copper = goods.get(goods.id("copper").unwrap());
    assert_eq!(copper.perishes, Perishes::Never);
    assert!(copper.movable);
}

#[test]
fn loading_twice_gives_the_same_goods_and_ids() {
    let list = || -> Vec<(usize, String, Good)> {
        load(&test_setting("goods_valid"))
            .rules
            .goods
            .iter()
            .map(|(id, name, good)| (id.index(), name.to_string(), *good))
            .collect()
    };
    assert_eq!(list(), list());
}

#[test]
fn looking_up_a_missing_name_is_an_error() {
    let goods = load(&test_setting("goods_valid")).rules.goods;
    assert_eq!(
        goods.id("missing"),
        Err(IdError::UnknownName {
            kind: "good",
            name: "missing".to_string()
        })
    );
}

#[test]
fn every_good_error_is_reported_at_its_place() {
    let shown = failures_of("goods_errors");
    let invalid_name = |name: &str| {
        format!(
            "invalid name '{name}' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)"
        )
    };
    let expected = vec![
        "rules/a.toml:4:1: missing required key 'name' in good".to_string(),
        format!("rules/a.toml:8:8: {}", invalid_name("Copper")),
        "rules/a.toml:12:18: key 'perishes_after' in good must be at least 1".to_string(),
        "rules/a.toml:16:18: key 'perishes_after' in good must be at least 1".to_string(),
        "rules/a.toml:20:18: key 'perishes_after' in good must be an integer, found float"
            .to_string(),
        "rules/a.toml:24:18: key 'perishes_after' in good is out of range".to_string(),
        "rules/a.toml:28:11: key 'movable' in good must be a boolean, found string".to_string(),
        "rules/a.toml:32:1: unknown key 'perishes' in good".to_string(),
        format!("rules/a.toml:35:8: {}", invalid_name("Bad-Name")),
        "rules/a.toml:36:18: key 'perishes_after' in good must be at least 1".to_string(),
        "rules/a.toml:37:11: key 'movable' in good must be a boolean, found integer".to_string(),
        "rules/a.toml:38:1: unknown key 'colour' in good".to_string(),
        "rules/b.toml:2:8: duplicate good name 'copper' (first defined at rules/a.toml:2:8)"
            .to_string(),
        "start/s.toml:1:3: section 'good' belongs in rules/, not start/".to_string(),
    ];
    assert_eq!(shown, expected);
}

#[test]
fn a_setting_with_no_goods_loads() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.rules.goods.is_empty());
    assert_eq!(setting.rules.goods.len(), 0);
}
