//! Integration tests for factories, through the engine's public API only.

mod common;

use common::{close, failures_of, load, sample_setting, test_setting, world_of};
use policyspace_engine::{FactoryId, HexCoord, LoadedSetting, World};

/// One factory as (index, coordinates, technology name, owner name, condition).
type FactoryRow = (usize, HexCoord, String, String, f64);

fn row(index: usize, q: i32, r: i32, technology: &str, owner: &str, condition: f64) -> FactoryRow {
    (
        index,
        HexCoord { q, r },
        technology.to_string(),
        owner.to_string(),
        condition,
    )
}

fn factories_of(setting: &LoadedSetting) -> Vec<FactoryRow> {
    setting
        .start
        .factories
        .iter()
        .map(|(id, factory)| {
            (
                id.index(),
                setting.start.map.coord(factory.hex),
                setting
                    .rules
                    .technologies
                    .name(factory.technology)
                    .to_string(),
                setting.start.businesses.name(factory.owner).to_string(),
                factory.condition,
            )
        })
        .collect()
}

fn factory_rows(world: &World) -> Vec<FactoryRow> {
    world
        .factories()
        .iter()
        .map(|(id, factory)| {
            (
                id.index(),
                world.map().coord(factory.hex),
                world
                    .rules()
                    .technologies
                    .name(factory.technology)
                    .to_string(),
                world.businesses().name(factory.owner).to_string(),
                factory.condition,
            )
        })
        .collect()
}

/// The factory indexes `on_hex` lists for the hex at (`q`, `r`).
fn on_hex(setting: &LoadedSetting, q: i32, r: i32) -> Vec<usize> {
    let hex = setting.start.map.id(HexCoord { q, r }).unwrap();
    setting
        .start
        .factories
        .on_hex(hex)
        .iter()
        .map(|id| id.index())
        .collect()
}

/// The factory indexes `owned_by` lists for the business whose index is `index`.
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
        .factories
        .owned_by(id)
        .map(|f| f.index())
        .collect()
}

/// Asserts `actual` equals `expected` row by row, the conditions to within 1e-9.
fn assert_rows(actual: &[FactoryRow], expected: &[FactoryRow]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!(
            (&a.0, &a.1, &a.2, &a.3),
            (&e.0, &e.1, &e.2, &e.3),
            "{a:?} vs {e:?}"
        );
        assert!(close(a.4, e.4), "{a:?} vs {e:?}");
    }
}

fn sample_rows() -> Vec<FactoryRow> {
    vec![
        row(0, 0, 0, "farm", "center_farm", 1.0),
        row(1, 0, 0, "mine", "center_mine", 0.75),
        row(2, 0, 0, "toolmaker", "center_toolmaker", 1.0),
    ]
}

#[test]
fn the_sample_setting_loads_with_three_factories_that_a_world_wears_one_slice_a_turn() {
    let setting = load(&sample_setting());
    assert_eq!(setting.start.factories.len(), 3);
    assert_eq!(factories_of(&setting), sample_rows());
    assert_eq!(on_hex(&setting, 0, 0), vec![0, 1, 2]);
    assert!(on_hex(&setting, 2, -2).is_empty());
    assert_eq!(owned(&setting, 0), vec![0]);
    assert_eq!(owned(&setting, 1), vec![1]);
    assert_eq!(owned(&setting, 2), vec![2]);

    // Under the idle host nothing runs or is maintained, so each factory wears one
    // lifetime-slice a turn, turn 0's included (a world is past turn 0's operation when
    // `World::new` returns). The lifetimes are 8, 8 and 24 turns.
    let worn = |slices: f64| {
        let mut rows = sample_rows();
        for (row, lifetime) in rows.iter_mut().zip([8.0, 8.0, 24.0]) {
            row.4 -= slices / lifetime;
        }
        rows
    };
    let (mut world, _) = world_of(&sample_setting());
    assert_rows(&factory_rows(&world), &worn(1.0));
    for turn in 1..=3 {
        world.step(&mut common::idle()).unwrap();
        assert_rows(&factory_rows(&world), &worn(f64::from(turn) + 1.0));
    }
}

#[test]
fn identical_sample_runs_give_identical_reports_and_factories() {
    let (mut a, report_a) = world_of(&sample_setting());
    let (mut b, report_b) = world_of(&sample_setting());
    assert_eq!(report_a, report_b);
    assert_eq!(factory_rows(&a), factory_rows(&b));
    for _ in 0..3 {
        assert_eq!(
            a.step(&mut common::idle()).unwrap(),
            b.step(&mut common::idle()).unwrap()
        );
        assert_eq!(factory_rows(&a), factory_rows(&b));
    }
}

#[test]
fn factories_load_across_files_with_counts_defaults_and_forward_references() {
    let setting = load(&test_setting("factories_valid"));
    let expected = vec![
        row(0, 0, 0, "mill", "farm_a", 1.0),
        row(1, 0, 0, "forge", "farm_a", 0.5),
        row(2, 0, 0, "forge", "farm_a", 0.5),
        row(3, 0, 0, "mill", "farm_b", 1.0),
        row(4, 1, 0, "forge", "idle", 0.25),
    ];
    assert_eq!(factories_of(&setting), expected);
    assert_eq!(on_hex(&setting, 0, 0), vec![0, 1, 2, 3]);
    assert_eq!(on_hex(&setting, 1, 0), vec![4]);
    assert_eq!(owned(&setting, 0), vec![0, 1, 2]);
    assert_eq!(owned(&setting, 1), vec![3]);
    assert_eq!(owned(&setting, 2), vec![4]);

    // The two copies of the count-2 entry are equal records with consecutive IDs.
    let factories = &setting.start.factories;
    let (first, second): (Vec<_>, Vec<_>) = factories
        .iter()
        .filter(|(id, _)| id.index() == 1 || id.index() == 2)
        .partition(|(id, _)| id.index() == 1);
    assert_eq!(first[0].1, second[0].1);

    // Loading twice gives the same factories.
    let again = load(&test_setting("factories_valid"));
    assert_eq!(factories_of(&again), expected);
}

#[test]
fn each_factory_error_is_reported_and_a_bad_entry_still_has_its_references_checked() {
    let condition =
        "key 'condition' in factory must be a finite number greater than 0 and at most 1";
    let at = |position: &str, message: &str| format!("start/a.toml:{position}: {message}");
    let expected = vec![
        "rules/x.toml:1:3: section 'factory' belongs in start/, not rules/".to_string(),
        at("7:1", "missing required key 'q' in factory"),
        at("12:1", "missing required key 'r' in factory"),
        at("17:1", "missing required key 'technology' in factory"),
        at("22:1", "missing required key 'owner' in factory"),
        at("28:5", "key 'q' in factory must be an integer, found float"),
        at(
            "29:5",
            "key 'r' in factory must be an integer, found string",
        ),
        at(
            "30:14",
            "key 'technology' in factory must be a string, found integer",
        ),
        at(
            "31:9",
            "key 'owner' in factory must be a string, found boolean",
        ),
        at(
            "32:13",
            "key 'condition' in factory must be a number, found string",
        ),
        at(
            "33:9",
            "key 'count' in factory must be an integer, found float",
        ),
        at("36:5", "key 'q' in factory is out of range"),
        at("37:5", "key 'r' in factory is out of range"),
        at("41:1", "no hex at (9, 9)"),
        at("50:14", "unknown technology 'loom'"),
        at("51:9", "unknown business 'nobody'"),
        at(
            "53:1",
            "factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)",
        ),
        at("64:13", condition),
        at("71:13", condition),
        at("78:13", condition),
        at("85:13", condition),
        at("92:13", condition),
        at("99:9", "key 'count' in factory must be at least 1"),
        at("106:9", "key 'count' in factory must be at least 1"),
        at(
            "113:9",
            "key 'count' in factory is out of range (the most is 1000000)",
        ),
        at("120:1", "unknown key 'colour' in factory"),
        at("121:1", "unknown key 'size' in factory"),
        at(
            "129:1",
            "factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)",
        ),
        at("132:14", "unknown technology 'loom'"),
        at("134:13", condition),
        at("135:9", "key 'count' in factory must be at least 1"),
        at("136:1", "unknown key 'colour' in factory"),
        "start/b.toml:9:1: no hex at (9, 9)".to_string(),
    ];
    assert_eq!(failures_of("factories_errors"), expected);
}

#[test]
fn a_setting_with_no_factories_is_valid() {
    let setting = load(&test_setting("valid_minimal"));
    assert!(setting.start.factories.is_empty());

    let setting = load(&test_setting("businesses_valid"));
    assert!(setting.start.factories.is_empty());
    let (world, _) = world_of(&test_setting("businesses_valid"));
    assert_eq!(world.factories().len(), 0);
}

#[test]
fn the_one_factory_setting_loads() {
    let setting = load(&test_setting("factories_one"));
    assert_eq!(
        factories_of(&setting),
        vec![row(0, 0, 0, "oven", "solo", 1.0)]
    );
}

/// The factory whose index is `index`, panicking if there is none.
fn factory_id(index: usize, world: &World) -> FactoryId {
    world
        .factories()
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no factory {index}"))
}

#[test]
#[should_panic(expected = "is not from this list")]
fn get_panics_on_a_factory_from_another_world() {
    let (valid, _) = world_of(&test_setting("factories_valid"));
    let foreign = factory_id(4, &valid);
    let (world, _) = world_of(&test_setting("factories_one"));
    world.factories().get(foreign);
}
