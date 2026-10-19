//! Helpers shared by the engine's integration tests.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use policyspace_engine::{
    DeciderHost, GoodId, HexCoord, HexId, LoadErrors, LoadedSetting, PerishesAt, PopId, PopOutcome,
    Quantity, Stockpile, Turn, TurnReport, World, load_setting,
};

/// The folder of the test setting `name` (`tests/settings/<name>`).
pub fn test_setting(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/settings")
        .join(name)
}

/// The folder of the sample setting.
pub fn sample_setting() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../settings/sample")
}

/// Loads the setting at `path`, panicking with its errors if it doesn't load.
pub fn load(path: &Path) -> LoadedSetting {
    load_setting(path).unwrap_or_else(|errors| panic!("{} should load:\n{errors}", path.display()))
}

/// Each error as one line of text, in the loader's order.
pub fn lines(errors: &LoadErrors) -> Vec<String> {
    errors.iter().map(|e| e.to_string()).collect()
}

/// Loads the test setting `name`, which must fail, and returns its error lines.
pub fn failures_of(name: &str) -> Vec<String> {
    match load_setting(test_setting(name)) {
        Ok(_) => panic!("{name} must not load"),
        Err(errors) => lines(&errors),
    }
}

/// The idle host: every business does nothing.
pub fn idle() -> DeciderHost {
    DeciderHost::idle()
}

/// Builds a world from the setting at `path` under the idle host, with its turn 0 report.
pub fn world_of(path: &Path) -> (World, TurnReport) {
    World::new(load(path), &mut idle()).expect("the idle host holds every name")
}

/// The hex at (`q`, `r`), panicking if the map has none.
pub fn hex(world: &World, q: i32, r: i32) -> HexId {
    world
        .map()
        .id(HexCoord { q, r })
        .unwrap_or_else(|| panic!("no hex at ({q}, {r})"))
}

/// The good named `name`.
pub fn good(world: &World, name: &str) -> GoodId {
    world.rules().goods.id(name).unwrap()
}

/// A quantity of `value`, which must be valid.
pub fn q(value: f64) -> Quantity {
    Quantity::new(value).unwrap()
}

/// Whether two numbers are within 1e-9 of each other.
pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The end of turn `t`, when goods perish.
pub fn end(t: u64) -> PerishesAt {
    PerishesAt::EndOfTurn(Turn::new(t))
}

/// The POP whose index is `index`, panicking if there is none.
pub fn pop_id(index: usize, world: &World) -> PopId {
    world
        .pops()
        .iter()
        .map(|(id, _)| id)
        .find(|id| id.index() == index)
        .unwrap_or_else(|| panic!("no POP {index}"))
}

/// The recorded outcome of the POP whose index is `index`.
pub fn outcome(world: &World, index: usize) -> &PopOutcome {
    world.pop_outcome(pop_id(index, world))
}

/// A list of (good, amount) as (good name, amount).
pub fn named(world: &World, list: &[(GoodId, Quantity)]) -> Vec<(String, f64)> {
    list.iter()
        .map(|(g, amount)| (world.rules().goods.name(*g).to_string(), amount.get()))
        .collect()
}

/// A (good name, amount) pair.
pub fn pair(name: &str, amount: f64) -> (String, f64) {
    (name.to_string(), amount)
}

/// Asserts a list of (good, amount) holds exactly these goods, in this order, with amounts
/// within 1e-9.
pub fn assert_list(world: &World, actual: &[(GoodId, Quantity)], expected: &[(&str, f64)]) {
    let actual = named(world, actual);
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for ((an, aq), (en, eq)) in actual.iter().zip(expected) {
        assert_eq!(an, en, "{actual:?} vs {expected:?}");
        assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
    }
}

/// A stockpile as (good name, total), in the stockpile's order.
pub fn stockpile_totals(world: &World, stockpile: &Stockpile) -> Vec<(String, f64)> {
    stockpile
        .iter()
        .map(|(g, amount)| (world.rules().goods.name(g).to_string(), amount.get()))
        .collect()
}

/// One good's groups in a stockpile as (when it perishes, amount), soonest first.
pub fn groups(stockpile: &Stockpile, good: GoodId) -> Vec<(PerishesAt, f64)> {
    stockpile
        .groups(good)
        .map(|(at, amount)| (at, amount.get()))
        .collect()
}
