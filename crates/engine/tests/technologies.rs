//! Integration tests for technologies, through the engine's public API only.

mod common;

use common::{failures_of, load, sample_setting, test_setting, world_of};
use policyspace_engine::{Bundle, IdError, LoadedSetting, Quantity};

/// A list of (good name, amount).
type Amounts = Vec<(String, f64)>;

/// One technology as (index, name, inputs, output name, capacity, capital, construction,
/// construction turns, lifetime).
type TechRow = (
    usize,
    String,
    Amounts,
    String,
    f64,
    Amounts,
    Amounts,
    u32,
    u32,
);

fn s(text: &str) -> String {
    text.to_string()
}

fn amounts(setting: &LoadedSetting, bundle: &Bundle) -> Amounts {
    bundle
        .iter()
        .map(|(g, a)| (setting.rules.goods.name(g).to_string(), a.get()))
        .collect()
}

fn techs_of(setting: &LoadedSetting) -> Vec<TechRow> {
    setting
        .rules
        .technologies
        .iter()
        .map(|(id, name, t)| {
            (
                id.index(),
                name.to_string(),
                amounts(setting, t.inputs()),
                setting.rules.goods.name(t.output()).to_string(),
                t.capacity().get(),
                amounts(setting, t.capital()),
                amounts(setting, t.construction()),
                t.construction_turns(),
                t.lifetime(),
            )
        })
        .collect()
}

fn list(items: &[(&str, f64)]) -> Amounts {
    items.iter().map(|(g, a)| (s(g), *a)).collect()
}

#[allow(clippy::too_many_arguments)]
fn row(
    index: usize,
    name: &str,
    inputs: &[(&str, f64)],
    output: &str,
    capacity: f64,
    capital: &[(&str, f64)],
    construction: &[(&str, f64)],
    construction_turns: u32,
    lifetime: u32,
) -> TechRow {
    (
        index,
        s(name),
        list(inputs),
        s(output),
        capacity,
        list(capital),
        list(construction),
        construction_turns,
        lifetime,
    )
}

#[test]
fn the_sample_setting_loads_with_tools_its_price_and_the_three_technologies() {
    let setting = load(&sample_setting());
    assert_eq!(setting.rules.goods.id("tools").unwrap().index(), 6);
    assert_eq!(setting.rules.technologies.len(), 3);
    assert_eq!(
        techs_of(&setting),
        vec![
            row(
                0,
                "farm",
                &[("unskilled_labor", 0.5), ("unimproved_grain", 1.0)],
                "grain",
                12.0,
                &[("tools", 4.0)],
                &[("unskilled_labor", 8.0)],
                2,
                8,
            ),
            row(
                1,
                "mine",
                &[("unskilled_labor", 1.0), ("unimproved_copper", 1.0)],
                "copper",
                2.5,
                &[("tools", 2.0)],
                &[("unskilled_labor", 4.0)],
                2,
                8,
            ),
            row(
                2,
                "toolmaker",
                &[("unskilled_labor", 1.5), ("copper", 2.0)],
                "tools",
                4.0,
                &[("tools", 3.0)],
                &[("unskilled_labor", 6.0)],
                3,
                24,
            ),
        ]
    );
    assert_eq!(setting.rules.technologies.id("farm").unwrap().index(), 0);

    let (world, _) = world_of(&sample_setting());
    let tools = world.rules().goods.id("tools").unwrap();
    let center = world.market(common::hex(&world, 0, 0));
    assert_eq!(center.price(tools), Some(Quantity::new(12.0).unwrap()));
}

#[test]
fn technologies_across_files_in_both_forms_with_forward_references_and_empty_tables() {
    let setting = load(&test_setting("technologies_valid"));
    let expected = vec![
        row(
            0,
            "smelter",
            &[("labor", 1.0), ("ore", 2.0)],
            "iron",
            10.0,
            &[("tools", 5.0)],
            &[("labor", 20.0)],
            2,
            10,
        ),
        row(
            1,
            "well",
            &[],
            "bread",
            0.5,
            &[("labor", 3.0), ("tools", 1.5)],
            &[],
            1,
            1,
        ),
        row(
            2,
            "toolmaker",
            &[("iron", 1.0)],
            "tools",
            3.0,
            &[("iron", 4.0), ("tools", 2.0)],
            &[],
            3,
            3,
        ),
    ];
    assert_eq!(techs_of(&setting), expected);

    let technologies = &setting.rules.technologies;
    let goods = &setting.rules.goods;
    let well = technologies.get(technologies.id("well").unwrap());
    assert!(well.inputs().is_empty());
    assert!(well.construction().is_empty());
    let smelter = technologies.get(technologies.id("smelter").unwrap());
    assert_eq!(
        smelter.inputs().amount_of(goods.id("ore").unwrap()),
        Quantity::new(2.0).unwrap()
    );
    assert_eq!(
        smelter.inputs().amount_of(goods.id("bread").unwrap()),
        Quantity::ZERO
    );
    assert!(smelter.capital().contains(goods.id("tools").unwrap()));

    let again = load(&test_setting("technologies_valid"));
    assert_eq!(techs_of(&again), expected);
}

#[test]
fn every_technology_error_is_reported_at_its_place() {
    let help = "(names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)";
    let positive = "must be a finite number greater than 0";
    let expected = vec![
        s("rules/a.toml:11:1: missing required key 'name' in technology"),
        s("rules/a.toml:20:1: missing required key 'inputs' in technology"),
        s("rules/a.toml:29:1: missing required key 'output' in technology"),
        s("rules/a.toml:38:1: missing required key 'capacity' in technology"),
        s("rules/a.toml:47:1: missing required key 'capital' in technology"),
        s("rules/a.toml:56:1: missing required key 'construction' in technology"),
        s("rules/a.toml:65:1: missing required key 'construction_turns' in technology"),
        s("rules/a.toml:74:1: missing required key 'lifetime' in technology"),
        format!("rules/a.toml:84:8: invalid name 'Bad Name' {help}"),
        s("rules/a.toml:94:8: duplicate technology name 'ok' (first defined at rules/a.toml:2:8)"),
        s("rules/a.toml:105:10: key 'inputs' in technology must be a table, found integer"),
        s("rules/a.toml:106:10: key 'output' in technology must be a string, found integer"),
        s("rules/a.toml:107:12: key 'capacity' in technology must be a number, found string"),
        s("rules/a.toml:108:11: key 'capital' in technology must be a table, found string"),
        s("rules/a.toml:109:16: key 'construction' in technology must be a table, found integer"),
        s(
            "rules/a.toml:110:22: key 'construction_turns' in technology must be an integer, found float",
        ),
        s("rules/a.toml:111:12: key 'lifetime' in technology must be an integer, found string"),
        s(
            "rules/a.toml:115:20: key 'grain' in inputs of technology must be a number, found string",
        ),
        format!("rules/a.toml:115:33: key 'labor' in inputs of technology {positive}"),
        format!("rules/a.toml:117:12: key 'capacity' in technology {positive}"),
        format!("rules/a.toml:118:21: key 'labor' in capital of technology {positive}"),
        format!("rules/a.toml:118:32: key 'coin' in capital of technology {positive}"),
        format!("rules/a.toml:119:26: key 'grain' in construction of technology {positive}"),
        format!("rules/a.toml:127:12: key 'capacity' in technology {positive}"),
        s("rules/a.toml:130:22: key 'construction_turns' in technology must be at least 1"),
        s("rules/a.toml:131:12: key 'lifetime' in technology must be at least 1"),
        s("rules/a.toml:140:22: key 'construction_turns' in technology is out of range"),
        s("rules/a.toml:141:12: key 'lifetime' in technology is out of range"),
        s("rules/a.toml:143:1: capital of technology needs at least one good"),
        s("rules/a.toml:155:12: key 'grain' in inputs of technology must not be the output good"),
        s("rules/a.toml:165:12: unknown good 'tin'"),
        s("rules/a.toml:166:10: unknown good 'bronze'"),
        s("rules/a.toml:168:13: unknown good 'tin'"),
        s("rules/a.toml:169:18: unknown good 'tin'"),
        s("rules/a.toml:182:1: unknown key 'colour' in technology"),
        s("rules/a.toml:183:1: unknown key 'speed' in technology"),
        s("rules/a.toml:185:1: capital of technology needs at least one good"),
        format!("rules/a.toml:186:8: invalid name 'Several' {help}"),
        s("rules/a.toml:187:12: unknown good 'tin'"),
        s("rules/a.toml:187:21: key 'grain' in inputs of technology must not be the output good"),
        format!("rules/a.toml:187:29: key 'grain' in inputs of technology {positive}"),
        format!("rules/a.toml:189:12: key 'capacity' in technology {positive}"),
        s("rules/a.toml:191:18: unknown good 'tin'"),
        format!("rules/a.toml:191:24: key 'tin' in construction of technology {positive}"),
        s("rules/a.toml:192:22: key 'construction_turns' in technology must be at least 1"),
        s("rules/a.toml:193:12: key 'lifetime' in technology must be an integer, found float"),
        s("rules/a.toml:194:1: unknown key 'colour' in technology"),
        s("rules/b.toml:2:8: duplicate technology name 'ok' (first defined at rules/a.toml:2:8)"),
        s("start/t.toml:1:3: section 'technology' belongs in rules/, not start/"),
    ];
    assert_eq!(failures_of("technologies_errors"), expected);
}

#[test]
fn no_technologies_is_valid_and_an_unknown_technology_is_an_error() {
    let setting = load(&test_setting("valid_minimal"));
    let technologies = &setting.rules.technologies;
    assert!(technologies.is_empty());
    assert_eq!(technologies.len(), 0);
    assert_eq!(
        technologies.id("missing"),
        Err(IdError::UnknownName {
            kind: "technology",
            name: s("missing"),
        })
    );
}
