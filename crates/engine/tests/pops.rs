//! Integration tests for POP types, through the engine's public API only.

mod common;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{IdError, LoadedSetting, Quantity};

/// One POP type as (index, name, endowment as (good name, amount), utility as (good name,
/// subsistence, share)).
type TypeRow = (usize, String, Vec<(String, f64)>, Vec<(String, f64, f64)>);

fn s(text: &str) -> String {
    text.to_string()
}

fn types_of(setting: &LoadedSetting) -> Vec<TypeRow> {
    let goods = &setting.rules.goods;
    setting
        .rules
        .pop_types
        .iter()
        .map(|(id, name, t)| {
            (
                id.index(),
                name.to_string(),
                t.endowment()
                    .iter()
                    .map(|e| (goods.name(e.good).to_string(), e.amount.get()))
                    .collect(),
                t.utility()
                    .iter()
                    .map(|u| (goods.name(u.good).to_string(), u.subsistence.get(), u.share))
                    .collect(),
            )
        })
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// Compares types, with a tolerance for the shares.
fn assert_types(actual: &[TypeRow], expected: &[TypeRow]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!((a.0, &a.1, &a.2), (e.0, &e.1, &e.2), "{a:?}");
        assert_eq!(a.3.len(), e.3.len(), "{a:?}");
        for (au, eu) in a.3.iter().zip(&e.3) {
            assert_eq!(au.0, eu.0, "{a:?}");
            assert!(close(au.1, eu.1), "{a:?} vs {e:?}");
            assert!(close(au.2, eu.2), "{a:?} vs {e:?}");
        }
    }
}

fn assert_shares_sum_to_one(setting: &LoadedSetting) {
    for (_, name, t) in setting.rules.pop_types.iter() {
        let total: f64 = t.utility().iter().map(|u| u.share).sum();
        assert!(close(total, 1.0), "{name}: shares sum to {total}");
    }
}

fn row(
    index: usize,
    name: &str,
    endowment: &[(&str, f64)],
    utility: &[(&str, f64, f64)],
) -> TypeRow {
    (
        index,
        s(name),
        endowment.iter().map(|(g, a)| (s(g), *a)).collect(),
        utility.iter().map(|(g, a, w)| (s(g), *a, *w)).collect(),
    )
}

#[test]
fn the_sample_setting_has_two_pop_types() {
    let setting = load(&sample_setting());
    assert_eq!(setting.rules.pop_types.len(), 2);
    assert_types(
        &types_of(&setting),
        &[
            row(
                0,
                "peasant",
                &[("unskilled_labor", 10.0)],
                &[("unskilled_labor", 2.0, 0.2), ("grain", 2.0, 0.8)],
            ),
            row(
                1,
                "artisan",
                &[("unskilled_labor", 10.0)],
                &[
                    ("unskilled_labor", 1.0, 0.1),
                    ("copper", 0.0, 0.5),
                    ("grain", 2.0, 0.4),
                ],
            ),
        ],
    );
    assert_shares_sum_to_one(&setting);
    assert_eq!(setting.rules.pop_types.id("peasant").unwrap().index(), 0);
}

#[test]
fn pop_types_across_files_and_both_forms_get_ids_in_load_order() {
    let setting = load(&test_setting("pops_valid"));
    let expected = [
        row(
            0,
            "farmer",
            &[("labor", 10.0)],
            &[("labor", 1.0, 0.25), ("grain", 2.0, 0.75)],
        ),
        row(
            1,
            "fisher",
            &[("labor", 8.0), ("fish", 2.0)],
            &[
                ("coin", 0.0, 0.25),
                ("grain", 0.0, 0.75),
                ("fish", 1.0, 0.0),
            ],
        ),
        row(2, "idle", &[], &[("grain", 0.0, 1.0)]),
    ];
    assert_types(&types_of(&setting), &expected);
    assert_shares_sum_to_one(&setting);

    let pops = &setting.rules.pop_types;
    let goods = &setting.rules.goods;
    let labor = goods.id("labor").unwrap();
    let fish = goods.id("fish").unwrap();
    let coin = goods.id("coin").unwrap();
    let fisher = pops.get(pops.id("fisher").unwrap());
    let idle = pops.get(pops.id("idle").unwrap());
    let farmer = pops.get(pops.id("farmer").unwrap());
    assert_eq!(fisher.endowment_of(labor), Quantity::new(8.0).unwrap());
    assert_eq!(idle.endowment_of(labor), Quantity::ZERO);
    let fish_utility = fisher.utility_of(fish).unwrap();
    assert_eq!(fish_utility.subsistence, Quantity::new(1.0).unwrap());
    assert_eq!(fish_utility.share, 0.0);
    assert!(farmer.utility_of(coin).is_none());

    let again = load(&test_setting("pops_valid"));
    assert_eq!(types_of(&setting), types_of(&again));
}

#[test]
fn every_pop_type_error_is_reported_at_its_place() {
    let help = "(names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)";
    let expected = vec![
        s("rules/a.toml:6:1: missing required key 'name' in pop_type"),
        s("rules/a.toml:10:1: missing required key 'endowment' in pop_type"),
        s("rules/a.toml:14:1: missing required key 'utility' in pop_type"),
        format!("rules/a.toml:19:8: invalid name 'Bad Name' {help}"),
        s("rules/a.toml:24:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)"),
        s("rules/a.toml:30:13: key 'endowment' in pop_type must be a table, found integer"),
        s("rules/a.toml:36:11: key 'utility' in pop_type must be a table, found string"),
        s(
            "rules/a.toml:40:23: key 'grain' in endowment of pop_type must be a number, found string",
        ),
        s(
            "rules/a.toml:40:35: key 'coin' in endowment of pop_type must be a finite number greater than 0",
        ),
        s(
            "rules/a.toml:45:23: key 'grain' in endowment of pop_type must be a finite number greater than 0",
        ),
        s(
            "rules/a.toml:45:34: key 'coin' in endowment of pop_type must be a finite number greater than 0",
        ),
        s("rules/a.toml:51:21: key 'grain' in utility of pop_type must be a table, found integer"),
        s(
            "rules/a.toml:56:37: key 'subsistence' in utility.grain of pop_type must be a finite number of at least 0",
        ),
        s(
            "rules/a.toml:56:77: key 'subsistence' in utility.coin of pop_type must be a number, found string",
        ),
        s(
            "rules/a.toml:61:31: key 'share' in utility.grain of pop_type must be a finite number of at least 0",
        ),
        s(
            "rules/a.toml:66:36: key 'coin' in utility of pop_type must have a positive subsistence or share",
        ),
        s(
            "rules/a.toml:71:36: key 'coin' in utility of pop_type must have a positive subsistence or share",
        ),
        s("rules/a.toml:73:1: utility of pop_type needs at least one good with a positive share"),
        s("rules/a.toml:78:1: utility of pop_type needs at least one good with a positive share"),
        s("rules/a.toml:85:15: unknown good 'tin'"),
        s("rules/a.toml:86:13: unknown good 'tin'"),
        s("rules/a.toml:91:34: unknown key 'weight' in utility.grain of pop_type"),
        s("rules/a.toml:92:1: unknown key 'colour' in pop_type"),
        format!("rules/a.toml:95:8: invalid name 'Several' {help}"),
        s("rules/a.toml:96:15: unknown good 'tin'"),
        s("rules/a.toml:97:13: unknown good 'tin'"),
        s(
            "rules/a.toml:97:29: key 'share' in utility.tin of pop_type must be a finite number of at least 0",
        ),
        s("rules/a.toml:97:33: unknown key 'weight' in utility.tin of pop_type"),
        s(
            "rules/a.toml:97:47: key 'coin' in utility of pop_type must have a positive subsistence or share",
        ),
        s("rules/a.toml:98:1: unknown key 'colour' in pop_type"),
        s("rules/b.toml:2:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)"),
        s("start/p.toml:1:3: section 'pop_type' belongs in rules/, not start/"),
    ];
    assert_eq!(failures_of("pops_errors"), expected);
}

#[test]
fn a_setting_with_no_pop_types_loads() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.rules.pop_types.is_empty());
    assert_eq!(setting.rules.pop_types.len(), 0);
    assert_eq!(
        setting.rules.pop_types.id("missing"),
        Err(IdError::UnknownName {
            kind: "pop type",
            name: s("missing")
        })
    );
}
