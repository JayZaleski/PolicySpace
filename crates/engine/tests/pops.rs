//! Integration tests for POP types and placed POPs, through the engine's public API only.

mod common;

use std::path::Path;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{
    GoodId, HexCoord, HexId, IdError, LoadedSetting, PerishesAt, PopId, Quantity, Turn, TurnReport,
    World,
};

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

// --- Placed POPs ---

/// One POP as (index, type name, coordinates, size).
type PopRow = (usize, String, HexCoord, f64);

fn pops_of(setting: &LoadedSetting) -> Vec<PopRow> {
    setting
        .start
        .pops
        .iter()
        .map(|(id, pop)| {
            let coord = setting
                .start
                .map
                .iter()
                .find(|(hex, _)| *hex == pop.hex)
                .map(|(_, coord)| coord)
                .unwrap();
            (
                id.index(),
                setting.rules.pop_types.name(pop.pop_type).to_string(),
                coord,
                pop.size.get(),
            )
        })
        .collect()
}

fn pop_row(index: usize, name: &str, q: i32, r: i32, size: f64) -> PopRow {
    (index, s(name), HexCoord { q, r }, size)
}

fn world_of(path: &Path) -> (World, TurnReport) {
    World::new(load(path))
}

fn hex(world: &World, q: i32, r: i32) -> HexId {
    world
        .map()
        .id(HexCoord { q, r })
        .unwrap_or_else(|| panic!("no hex at ({q}, {r})"))
}

fn good(world: &World, name: &str) -> GoodId {
    world.rules().goods.id(name).unwrap()
}

fn end(t: u64) -> PerishesAt {
    PerishesAt::EndOfTurn(Turn::new(t))
}

fn pop_id(index: usize, world: &World) -> PopId {
    world
        .pops()
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no POP {index}"))
}

/// One good's groups in a POP's stockpile as (when it perishes, amount), soonest first.
fn pop_groups(world: &World, pop: PopId, name: &str) -> Vec<(PerishesAt, f64)> {
    world
        .pop_stockpile(pop)
        .groups(good(world, name))
        .map(|(at, amount)| (at, amount.get()))
        .collect()
}

/// A POP's whole stockpile as (good name, total).
fn pop_totals(world: &World, pop: PopId) -> Vec<(String, f64)> {
    world
        .pop_stockpile(pop)
        .iter()
        .map(|(g, amount)| (world.rules().goods.name(g).to_string(), amount.get()))
        .collect()
}

/// A hex's whole stockpile as (good name, total).
fn hex_totals(world: &World, q: i32, r: i32) -> Vec<(String, f64)> {
    world
        .hex_stockpile(hex(world, q, r))
        .iter()
        .map(|(g, amount)| (world.rules().goods.name(g).to_string(), amount.get()))
        .collect()
}

fn named(world: &World, list: &[(GoodId, Quantity)]) -> Vec<(String, f64)> {
    list.iter()
        .map(|(g, amount)| (world.rules().goods.name(*g).to_string(), amount.get()))
        .collect()
}

fn pair(name: &str, amount: f64) -> (String, f64) {
    (s(name), amount)
}

const SAMPLE_POPS: [(&str, i32, i32, f64); 16] = [
    ("peasant", 0, -2, 20.0),
    ("peasant", 1, -2, 15.0),
    ("peasant", 2, -2, 12.0),
    ("peasant", -1, -1, 25.0),
    ("peasant", 0, -1, 30.0),
    ("peasant", 1, -1, 35.0),
    ("peasant", -1, 0, 40.0),
    ("peasant", 0, 0, 60.0),
    ("artisan", 0, 0, 10.0),
    ("peasant", 1, 0, 35.0),
    ("peasant", 2, 0, 18.0),
    ("peasant", -2, 1, 22.0),
    ("peasant", -1, 1, 28.0),
    ("peasant", 0, 1, 24.0),
    ("peasant", -1, 2, 16.0),
    ("peasant", 0, 2, 20.0),
];

#[test]
fn the_sample_setting_loads_with_its_pops() {
    let setting = load(&sample_setting());
    assert_eq!(setting.start.pops.len(), 16);
    let expected: Vec<PopRow> = SAMPLE_POPS
        .iter()
        .enumerate()
        .map(|(i, (name, q, r, size))| pop_row(i, name, *q, *r, *size))
        .collect();
    assert_eq!(pops_of(&setting), expected);

    let hex_at = |q, r| setting.start.map.id(HexCoord { q, r }).unwrap();
    let types = &setting.rules.pop_types;
    let center: Vec<(usize, String)> = setting
        .start
        .pops
        .on_hex(hex_at(0, 0))
        .iter()
        .map(|id| {
            let pop = setting.start.pops.get(*id);
            (id.index(), types.name(pop.pop_type).to_string())
        })
        .collect();
    assert_eq!(center, vec![(7, s("peasant")), (8, s("artisan"))]);

    let corner = setting.start.pops.on_hex(hex_at(2, -2));
    assert_eq!(corner.iter().map(|id| id.index()).collect::<Vec<_>>(), [2]);
    let peasant = types.id("peasant").unwrap();
    assert_eq!(setting.start.pops.get(corner[0]).pop_type, peasant);
    assert!(setting.start.pops.on_hex(hex_at(2, -1)).is_empty());
    let artisan = types.id("artisan").unwrap();
    assert_eq!(
        setting
            .start
            .pops
            .of_type_on_hex(hex_at(0, 0), artisan)
            .map(|id| id.index()),
        Some(8)
    );
}

#[test]
fn a_fresh_sample_world_endows_every_pop() {
    let (world, report) = world_of(&sample_setting());
    assert_eq!(world.turn(), Turn::ZERO);
    assert_eq!(world.pops().len(), 16);
    for (id, pop) in world.pops().iter() {
        let labor = 10.0 * pop.size.get();
        assert_eq!(
            pop_totals(&world, id),
            vec![pair("unskilled_labor", labor)],
            "POP {}",
            id.index()
        );
        assert_eq!(
            pop_groups(&world, id, "unskilled_labor"),
            vec![(end(0), labor)],
            "POP {}",
            id.index()
        );
    }
    assert_eq!(
        hex_totals(&world, 0, 0),
        vec![
            pair("unimproved_copper", 2.5),
            pair("unimproved_grain", 12.0)
        ]
    );
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
}

#[test]
fn stepping_the_sample_re_endows_and_perishes_the_labor() {
    let (mut world, _) = world_of(&sample_setting());
    let (mut other, _) = world_of(&sample_setting());
    for t in 1..=3u64 {
        let report = world.step().unwrap();
        assert_eq!(report, other.step().unwrap());
        assert_eq!(
            named(&world, &report.perished),
            vec![
                pair("unskilled_labor", 4100.0),
                pair("unimproved_copper", 10.0),
                pair("unimproved_grain", 24.0)
            ]
        );
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
        let center = pop_id(7, &world);
        assert_eq!(
            pop_totals(&world, center),
            vec![pair("unskilled_labor", 600.0)]
        );
        assert_eq!(
            pop_groups(&world, center, "unskilled_labor"),
            vec![(end(t), 600.0)]
        );
        for (id, _) in world.pops().iter() {
            assert_eq!(world.pop_stockpile(id), other.pop_stockpile(id));
        }
    }
}

#[test]
fn pops_across_files_with_two_types_on_a_hex_are_placed_in_load_order() {
    let setting = load(&test_setting("pops_placed"));
    let expected = vec![
        pop_row(0, "worker", 0, 0, 3.0),
        pop_row(1, "hoarder", 0, 0, 2.0),
        pop_row(2, "worker", 1, 0, 0.5),
        pop_row(3, "idle", 1, 0, 7.0),
    ];
    assert_eq!(pops_of(&setting), expected);
    let hex_at = |q, r| setting.start.map.id(HexCoord { q, r }).unwrap();
    let on = |q, r| -> Vec<usize> {
        setting
            .start
            .pops
            .on_hex(hex_at(q, r))
            .iter()
            .map(|id| id.index())
            .collect()
    };
    assert_eq!(on(0, 0), vec![0, 1]);
    assert_eq!(on(1, 0), vec![2, 3]);
    assert!(on(2, 0).is_empty());
    let hoarder = setting.rules.pop_types.id("hoarder").unwrap();
    assert_eq!(
        setting
            .start
            .pops
            .of_type_on_hex(hex_at(0, 0), hoarder)
            .map(|id| id.index()),
        Some(1)
    );
    assert!(
        setting
            .start
            .pops
            .of_type_on_hex(hex_at(1, 0), hoarder)
            .is_none()
    );

    let again = load(&test_setting("pops_placed"));
    assert_eq!(pops_of(&setting), pops_of(&again));
}

#[test]
fn the_pops_placed_world_endows_durable_and_perishable_goods() {
    let (mut world, report) = world_of(&test_setting("pops_placed"));
    let (p0, p1, p2, p3) = (
        pop_id(0, &world),
        pop_id(1, &world),
        pop_id(2, &world),
        pop_id(3, &world),
    );
    let check = |world: &World, t: u64| {
        assert_eq!(
            pop_groups(world, p0, "labor"),
            vec![(end(t), 30.0)],
            "turn {t}"
        );
        assert!(pop_groups(world, p0, "gold").is_empty(), "turn {t}");
        assert_eq!(
            pop_groups(world, p1, "labor"),
            vec![(end(t), 8.0)],
            "turn {t}"
        );
        assert_eq!(
            pop_groups(world, p1, "gold"),
            vec![(PerishesAt::Never, 3.0 * (t + 1) as f64)],
            "turn {t}"
        );
        assert_eq!(
            pop_groups(world, p2, "labor"),
            vec![(end(t), 5.0)],
            "turn {t}"
        );
        assert!(world.pop_stockpile(p3).is_empty(), "turn {t}");
        assert_eq!(
            world
                .hex_stockpile(hex(world, 2, 0))
                .groups(good(world, "grain"))
                .map(|(at, a)| (at, a.get()))
                .collect::<Vec<_>>(),
            vec![(end(t), 2.0)],
            "turn {t}"
        );
        assert!(world.hex_stockpile(hex(world, 0, 0)).is_empty(), "turn {t}");
        assert!(world.hex_stockpile(hex(world, 1, 0)).is_empty(), "turn {t}");
    };
    check(&world, 0);
    assert!(report.perished.is_empty());
    assert_eq!(named(&world, &report.emitted), vec![pair("grain", 2.0)]);
    assert_eq!(
        named(&world, &report.endowed),
        vec![pair("labor", 43.0), pair("gold", 3.0)]
    );
    for t in 1..=3u64 {
        let report = world.step().unwrap();
        check(&world, t);
        assert_eq!(
            named(&world, &report.perished),
            vec![pair("labor", 43.0), pair("grain", 2.0)],
            "turn {t}"
        );
        assert_eq!(named(&world, &report.emitted), vec![pair("grain", 2.0)]);
        assert_eq!(
            named(&world, &report.endowed),
            vec![pair("labor", 43.0), pair("gold", 3.0)]
        );
    }
}

#[test]
fn every_pop_error_is_reported_at_its_place() {
    let expected = vec![
        s("rules/p.toml:1:3: section 'pop' belongs in start/, not rules/"),
        s("start/a.toml:7:1: missing required key 'type' in pop"),
        s("start/a.toml:12:1: missing required key 'q' in pop"),
        s("start/a.toml:17:1: missing required key 'r' in pop"),
        s("start/a.toml:22:1: missing required key 'size' in pop"),
        s("start/a.toml:28:8: key 'type' in pop must be a string, found integer"),
        s("start/a.toml:35:5: key 'q' in pop must be an integer, found float"),
        s("start/a.toml:41:5: key 'q' in pop is out of range"),
        s("start/a.toml:45:1: no hex at (9, 9)"),
        s("start/a.toml:55:8: key 'size' in pop must be a number, found string"),
        s("start/a.toml:61:8: key 'size' in pop must be a finite number greater than 0"),
        s("start/a.toml:67:8: key 'size' in pop must be a finite number greater than 0"),
        s("start/a.toml:73:8: key 'size' in pop must be a finite number greater than 0"),
        s("start/a.toml:76:8: unknown pop type 'ghost'"),
        s("start/a.toml:86:1: unknown key 'colour' in pop"),
        s("start/a.toml:88:1: no hex at (10, 10)"),
        s("start/a.toml:89:8: unknown pop type 'ghost'"),
        s("start/a.toml:92:8: key 'size' in pop must be a finite number greater than 0"),
        s("start/a.toml:93:1: unknown key 'colour' in pop"),
        s(
            "start/b.toml:1:1: duplicate pop of 'worker' at (0, 0) (first defined at start/a.toml:1:1)",
        ),
    ];
    assert_eq!(failures_of("pops_placed_errors"), expected);
}

#[test]
fn a_world_with_no_pops_has_none_and_endows_nothing() {
    for name in ["valid_minimal", "turns_valid"] {
        let (mut world, report) = world_of(&test_setting(name));
        assert!(world.pops().is_empty(), "{name}");
        assert!(report.endowed.is_empty(), "{name}");
        for _ in 0..3 {
            assert!(world.step().unwrap().endowed.is_empty(), "{name}");
        }
        assert!(world.pops().is_empty(), "{name}");
    }
}

#[test]
#[should_panic(expected = "is not from this list")]
fn pop_stockpile_panics_on_a_pop_from_another_world() {
    let (sample, _) = world_of(&sample_setting());
    let foreign = pop_id(4, &sample);
    let (world, _) = world_of(&test_setting("pops_placed"));
    world.pop_stockpile(foreign);
}
