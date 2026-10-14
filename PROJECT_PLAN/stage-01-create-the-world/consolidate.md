# Stage 01: create-the-world — Consolidation

Consolidation must not change behavior: every setting loads with the same IDs, the same errors (same text, same order), the same turn reports, and the dev UI shows the same text and numbers. The only text that changes is the message of internal "foreign ID" panics (programming errors that users never see), as described in C01.

## Context
Files `build` must read before starting:
- `CLAUDE.md` (Development Loop, Parallel builds)
- Engine: `crates/engine/src/lib.rs`, `id.rs`, `goods.rs`, `map.rs`, `resources.rs`, `world.rs`, `stockpile.rs`, `quantity.rs`, `setting/mod.rs`, `setting/defs.rs`, `setting/reader.rs`, `setting/error.rs`, `setting/discover.rs`, `setting/goods.rs`, `setting/map.rs`, `setting/resources.rs`, `setting/test_content.rs`, and every file in `crates/engine/tests/*.rs`
- Dev UI: `crates/dev-ui/src/main.rs`, `app.rs`, `check.rs`, `inspect.rs`, `session.rs`, `geometry.rs`
- Designs, for the behavior that must not change: `PROJECT_PLAN/stage-01-create-the-world/step-*/design.md`

## Recommendations

### C01: Typed ID-indexed list (`IdVec`)
- **Status:** approved
- **Summary:** Six places keep "a `Vec` indexed by a typed ID" and hand-write the same lookup that panics on a foreign ID, and five places repeat the `u32::try_from(index).expect(..)` conversion (one, `HexMap::iter`, uses an unchecked `as u32`). One crate-private `IdVec<K, T>` and one `Id::from_usize` replace them, and Stage 02's per-POP and per-deposit data can use it.
- **Changes:**
  - `crates/engine/src/id.rs`:
    - Add `pub(crate) fn Id::from_usize(index: usize) -> Id<K>`. It panics with `"{K::LABEL} index {index} does not fit in a u32 ID"` if `index > u32::MAX`.
    - `NameRegistry::register` and `NameRegistry::iter` use `Id::from_usize` instead of their own `u32::try_from(..).expect(..)`. `NameRegistry` otherwise keeps its own `Vec<String>` storage and its existing "not from this registry" panic.
    - Add `pub(crate) struct IdVec<K: IdKind, T>`: a `Vec<T>` where element `i` belongs to the ID with index `i`. The impls are hand-written (no `#[derive]`, for the same reason as `Id`): `Default`; `Clone` where `T: Clone`; `PartialEq` where `T: PartialEq`; `Debug` where `T: Debug`. Methods (add only those that the call sites below use):
      - `from_vec(items: Vec<T>) -> Self`. Panics (through `Id::from_usize`) if the last index doesn't fit in a `u32`.
      - `push(&mut self, item: T) -> Id<K>`, returning the new item's ID.
      - `len`, `is_empty`.
      - `get(&self, id: Id<K>) -> &T` and `get_mut(&mut self, id: Id<K>) -> &mut T`. Both panic with `"{id:?} is not from this list (it has {len} {K::LABEL} entries)"` when `id` is out of range.
      - `iter(&self) -> impl Iterator<Item = (Id<K>, &T)>` in index order.
      - `values_mut(&mut self) -> impl Iterator<Item = &mut T>` in index order.
  - `crates/engine/src/map.rs`: `HexMap.coords` becomes `IdVec<HexKind, HexCoord>`. `from_coords` builds it with `IdVec::from_vec` and fills `by_coord` from its `iter()`. The duplicate-coordinate assert and its message stay. `coord` uses `get`, and `iter` uses `IdVec::iter`. The `as u32` cast goes away.
  - `crates/engine/src/resources.rs`:
    - `Deposits.deposits` becomes `IdVec<DepositKind, Deposit>`, and `Deposits.by_hex` becomes `IdVec<HexKind, Vec<DepositId>>`.
    - `from_deposits` keeps its explicit `hex < hex_count` assert and that assert's message (`"{:?} is not from a map of {hex_count} hexes"`). It then pushes each deposit's ID with `by_hex.get_mut(deposit.hex)`.
    - `get`, `iter`, `on_hex` (via `get`, returning `&[DepositId]`), and `of_type_on_hex` (via `self.deposits.get(id)`) use `IdVec`. The hand-written panics and index conversions go away.
  - `crates/engine/src/world.rs`: `World.hex_stockpiles` becomes `IdVec<HexKind, Stockpile>`. `hex_stockpile` uses `get`, `emit` uses `get_mut(deposit.hex)`, and `perish` uses `values_mut()`. In the unit test `a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged`, `before` changes type to match.
  - `crates/engine/src/setting/map.rs`: `MapReader::id` returns `Some(HexId::from_usize(index))` for a found coordinate (it currently uses `u32::try_from(index).ok()`).
- **Tasks:**
  1. Add `Id::from_usize` and `IdVec` to `id.rs`, with unit tests.
  2. Switch `NameRegistry` to `Id::from_usize`.
  3. Convert `HexMap`, `Deposits`, `World`, and `MapReader::id` as above.
  4. Update the `should_panic(expected = ..)` strings that change: `map.rs` `coord_panics_on_a_foreign_id`; `resources.rs` `on_hex_panics_on_a_hex_beyond_the_map` and `on_hex_panics_on_the_default_list`; `crates/engine/tests/world.rs` `hex_stockpile_panics_on_a_foreign_hex`. Each becomes `"is not from this list"`. Leave every other expected string alone.
- **Tests:**
  - New `id.rs` unit tests:
    - `push` hands out IDs 0, 1, 2 and `get` returns each item;
    - `get_mut` changes the right item;
    - `iter` is in index order;
    - `from_vec` gives the same IDs as pushing;
    - `get` with a foreign ID panics with `"is not from this list"`;
    - `Clone` and `PartialEq` work;
    - on 64-bit targets only (`#[cfg(target_pointer_width = "64")]`), `Id::from_usize(u32::MAX as usize + 1)` panics with `"does not fit in a u32 ID"`.
  - All existing tests pass, with only the expected strings in Task 4 changed.

### C02: One named-list type for goods and resource types (`NamedList`)
- **Status:** approved
- **Summary:** `Goods` and `ResourceTypes` are the same code twice: a `NameRegistry` plus a list of data, with identical `from_parts`, `len`, `is_empty`, `id`, `name`, `get`, and `iter`. One generic `NamedList<K, T>` replaces both. The names `Goods` and `ResourceTypes` stay as public type aliases, so nothing outside these modules changes. Stage 02 and Stage 03 named kinds (POP types, industries) reuse it.
- **Changes:**
  - `crates/engine/src/id.rs`: add `pub struct NamedList<K: IdKind, T>` with private fields `names: NameRegistry<K>` and `items: IdVec<K, T>` (from C01). Methods:
    - `pub(crate) fn from_parts(names: NameRegistry<K>, items: Vec<T>) -> Self`. It panics with `"every {K::LABEL} name needs exactly one entry"` if the lengths differ.
    - `pub fn len`, `pub fn is_empty`.
    - `pub fn id(&self, name: &str) -> Result<Id<K>, IdError>`.
    - `pub fn name(&self, id: Id<K>) -> &str`, through `NameRegistry::name`, so its panic still says "not from this registry".
    - `pub fn get(&self, id: Id<K>) -> &T`, through `IdVec::get`.
    - `pub fn iter(&self) -> impl Iterator<Item = (Id<K>, &str, &T)> + '_` in load order.
    - Hand-written `Default` and `Debug` (when `T: Debug`, like `NameRegistry`'s).
    - Doc comments in the style of the module. Load order is ID order, and IDs are valid only within the list that produced them.
  - `crates/engine/src/goods.rs`:
    - Replace the `Goods` struct and its impl with `pub type Goods = NamedList<GoodKind, Good>;`. Keep the doc comment: "Every good in a setting, in load order (which is also ID order)."
    - Keep `GoodKind`, `GoodId`, `Perishes`, `Good`, and the module docs.
    - Remove the unit tests that only exercise list behavior (they move to `id.rs`, below). Keep any test that checks something specific to goods. If none remains, remove the `tests` module.
  - `crates/engine/src/resources.rs`:
    - Replace the `ResourceTypes` struct and its impl with `pub type ResourceTypes = NamedList<ResourceKind, ResourceType>;`, with its existing doc comment.
    - Remove the resource-type tests that only exercise list behavior: `resource_type_accessors_and_iter_order`, `the_default_resource_list_is_empty`, `resource_get_panics_on_a_foreign_id`, `resource_name_panics_on_a_foreign_id`, `from_parts_panics_on_mismatched_lengths`.
    - Keep the `Deposits` tests, and keep any helper that they still use.
  - `crates/engine/src/lib.rs`: add `NamedList` to `pub use id::{...}`.
  - `crates/engine/src/setting/goods.rs` and `setting/resources.rs`: `Goods::from_parts` / `ResourceTypes::from_parts` keep working through the aliases. Change them only if the compiler requires it.
- **Tasks:**
  1. Add `NamedList` to `id.rs`, with unit tests.
  2. Replace `Goods` and `ResourceTypes` with aliases, and move the generic tests.
  3. Re-export `NamedList` from `lib.rs`.
- **Tests:** New `id.rs` unit tests for `NamedList`, using a test kind. They cover what the removed `goods.rs`/`resources.rs` tests covered:
  - accessors and IDs in order;
  - the default list is empty;
  - an unknown name gives `IdError::UnknownName`;
  - `iter` is in load order;
  - `get` with a foreign ID panics with `"is not from this list"`;
  - `name` with a foreign ID panics with `"is not from this registry"`;
  - `from_parts` with mismatched lengths panics with `"exactly one entry"`.

  Every integration test in `crates/engine/tests/` and every dev UI test passes unchanged.

### C03: Goods panel world totals in one pass
- **Status:** approved
- **Summary:** `inspect::good_rows` runs every frame. For each good it walks every hex and calls `Stockpile::total`, so the cost is goods × hexes stockpile lookups per frame. Walking the hexes once and adding each stockpile's totals into a per-good array gives the same numbers, added in the same order: for each good, over hexes in hex ID order.
- **Changes:** `crates/dev-ui/src/inspect.rs` `good_rows`:
  - Before building rows, make `held: Vec<Quantity>` of length `goods.len()`, all `Quantity::ZERO`.
  - For each `(hex, _)` in `world.map().iter()`, and each `(good, total)` in `world.hex_stockpile(hex).iter()`, set `held[good.index()] = held[good.index()].saturating_add(total)`.
  - Rows read `held[id.index()]`. Everything else (row fields, formatting, order) is unchanged.
- **Tasks:**
  1. Rewrite the total computation as above.
- **Tests:** The existing `good_rows_list_every_good_with_properties_and_world_totals` test passes unchanged. Add one test on `turns_valid` stepped to turn 2 that checks every good's `held` string. Compute the expected values by summing `Stockpile::total` per good over every hex in the test, the way the old code did.

### C04: Shared helpers for the engine's tests
- **Status:** approved
- **Summary:** The five engine integration test files each define the same `test_setting`, `load`, and sample-path helpers, and several define `failures_of` / `lines`. The engine unit tests repeat "turn an `ErrorSink` or `LoadErrors` into `Vec<String>` lines" in several modules. Each pattern moves to one place.
- **Changes:**
  - New `crates/engine/tests/common/mod.rs`:
    - Module doc: "Helpers shared by the engine's integration tests".
    - `#![allow(dead_code)]` with a comment that each test file uses only some of the helpers.
    - `pub fn test_setting(name: &str) -> PathBuf` (`tests/settings/<name>`).
    - `pub fn sample_setting() -> PathBuf` (`../../settings/sample`).
    - `pub fn load(path: &Path) -> LoadedSetting` (panics with the errors, as now).
    - `pub fn lines(errors: &LoadErrors) -> Vec<String>`.
    - `pub fn failures_of(name: &str) -> Vec<String>`: loads the test setting, panics with `"{name} must not load"` if it loads, and returns `lines` otherwise.
  - `crates/engine/tests/goods.rs`, `map.rs`, `resources.rs`, `setting_loader.rs`, `world.rs`:
    - Add `mod common;` and use its helpers.
    - Delete the local copies.
    - Replace every inline `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../settings/sample")` with `sample_setting()`.
    - File-specific helpers stay local (`setting_loader.rs`'s `failure_of` and `temp_setting`; `world.rs`'s `hex`, `totals`, etc.).
  - `crates/engine/src/setting/error.rs`:
    - Add `#[cfg(test)] impl ErrorSink { pub fn into_lines(self) -> Vec<String> }`, which returns the sorted error lines (empty if none).
    - Add `#[cfg(test)] impl LoadErrors { pub(crate) fn lines(&self) -> Vec<String> }`.
  - Engine unit tests that build `Vec<String>` from a sink or `LoadErrors` by hand use these instead: `setting/discover.rs` (`errors_of`, the folder-listing test), `setting/reader.rs` (`run`, `manifest_reader_reports_missing_keys_on_the_file`), `setting/resources.rs` (`run`), and `setting/test_content.rs` (`errors_of`, `error_lines`). Tests that check a single `Display` string (`errors.to_string()`) stay as they are.
- **Tasks:**
  1. Add `tests/common/mod.rs` and switch the five integration test files to it.
  2. Add the two `#[cfg(test)]` helpers and switch the unit tests to them.
- **Tests:** No test is added or removed. The same number of engine tests pass with the same assertions.

### C05: Shared helpers for the dev UI's tests
- **Status:** approved
- **Summary:** The `geometry`, `inspect`, and `session` test modules each define "a path from the repository root" and/or "load a setting as a world". The `session` tests also write the same manifest and map text in several tests, some as string literals that run across several source lines. One test-support module and a few named constants replace them.
- **Changes:**
  - New `crates/dev-ui/src/test_support.rs`, declared in `main.rs` as `#[cfg(test)] mod test_support;`. It has:
    - `pub fn repo_path(relative: &str) -> PathBuf`;
    - `pub fn world_at(relative: &str) -> World`, which loads the setting (panicking with its errors) and returns `World::new(..).0`.
  - `crates/dev-ui/src/geometry.rs` tests: replace the local `world` helper with `test_support::world_at`.
  - `crates/dev-ui/src/inspect.rs` tests: replace the local `world_at` with `test_support::world_at`.
  - `crates/dev-ui/src/session.rs` tests:
    - Replace the local `repo_path` with `test_support::repo_path`.
    - Add constants written with `\n` escapes: `MANIFEST` (`name = "Edited"`, `format_version = 1`), `BAD_MANIFEST` (`format_version = 99`), `ONE_HEX_MAP` (hex (0, 0)), and `TWO_HEX_MAP` (hexes (0, 0) and (1, 0)).
    - Use them in `two_hex_setting`, `reload_sees_edits_on_disk`, `reload_clears_the_selection_when_the_hex_is_gone_for_good`, and `a_failed_reload_keeps_the_selection_for_the_next_good_one`, in place of the inline literals. The file contents each test writes must stay byte-for-byte the same.
  - `crates/dev-ui/tests/checker.rs` keeps its own `repo_path`: integration tests can't see `#[cfg(test)]` modules of the binary.
- **Tasks:**
  1. Add `test_support.rs` and switch the three test modules to it.
  2. Add the session test constants and use them.
- **Tests:** No test is added or removed. All dev UI tests pass with the same assertions.

### C06: Engine leftovers
- **Status:** approved
- **Summary:**
  - Three helpers are marked `#[allow(dead_code)]` "until a real section type uses them", and only tests use them.
  - Discovery has an `unwrap_or(Folder::Rules)` fallback that can never run.
  - `World::emit` and `World::perish` each hand-write the same "total per good, in good ID order, only amounts > 0" accumulation.
- **Changes:**
  - `crates/engine/src/setting/defs.rs`: on `Definitions::registry` and `Definitions::location`, replace `#[allow(dead_code)]` with `#[cfg(test)]`. Change the comment to "Only tests use this so far; remove `#[cfg(test)]` when a section type needs it."
  - `crates/engine/src/setting/reader.rs`: do the same for `EntryReader::required_bool`. `boolean` stays as is, since `optional_bool` uses it.
  - `crates/engine/src/setting/discover.rs`, in `discover`: rewrite the `(name, is_dir) if Folder::from_dir_name(name).is_some()` arm so the `Folder` comes from a single `Folder::from_dir_name` call, with no `unwrap_or` fallback. For example, handle that case in an `if let Some(folder) = Folder::from_dir_name(..)` before the `match`. The other arms, the error messages, and the order of errors are unchanged.
  - `crates/engine/src/world.rs`:
    - Add a private `fn totals_by_good(amounts: impl IntoIterator<Item = (GoodId, Quantity)>) -> Vec<(GoodId, Quantity)>`. It sums with `saturating_add` into a `BTreeMap` and returns the totals in good ID order, dropping zero totals.
    - `emit` adds to the stockpiles as now, records each `(good, output)` in deposit ID order, and returns `totals_by_good` of them.
    - `perish` returns `totals_by_good` over every stockpile's `remove_perishing(self.turn)` results, in hex ID order (with C01, `self.hex_stockpiles.values_mut().flat_map(..)`).
    - The order of additions per good is unchanged, so the reports are bit-for-bit the same.
- **Tasks:**
  1. The `#[cfg(test)]` changes.
  2. The discovery rewrite.
  3. `totals_by_good` and its two uses.
- **Tests:** No new tests. The existing discovery, reader, defs, `test_content`, world unit tests, and `crates/engine/tests/world.rs` (exact report amounts, determinism) all pass unchanged. The clippy run has no warnings, which shows the `#[cfg(test)]` items aren't dead code.

### C07: Dev UI leftovers and `Session::world()`
- **Status:** approved
- **Summary:**
  - `app.rs` checks "is a world loaded?" six times with `matches!` / `let Some(Loaded::World(world))`. A `Session::world()` accessor makes each check one call, which matters as Stage 02 adds panels.
  - `Loaded::World`'s doc still says the world is at turn 0, which stopped being true in Step 08.
  - `paint_hex` takes an egui `Pos2` (`f32`) and converts it back to `f64` to compute the hexagon's corners. The selected-hex outline uses the `f64` point directly, so the two are computed differently.
- **Changes:**
  - `crates/dev-ui/src/session.rs`:
    - Add `pub fn world(&self) -> Option<&World>`: the loaded world, or None when nothing is loaded or the setting has errors.
    - Use it inside `Session` where only the world case matters (`reload`, `select`, `selected`).
    - Change the `Loaded::World` doc to "The setting loaded; this is its world (at turn 0 after a load, later after steps)."
    - The test helper `world(session)` may delegate to `session.world()`, but keeps its panic messages.
  - `crates/dev-ui/src/app.rs`:
    - Use `self.session.world()` in `shortcuts`, `side_panels`, `title`, and both checks in `top_panel` (`can_fit`, `can_step`).
    - `bottom_panel` and `central_panel` keep matching on `loaded()`, because they also handle the errors and nothing-loaded cases.
    - `paint_hex` takes `center: Point` (geometry's `f64` point) in place of `Pos2`. It computes the hexagon corners from that point, and uses `to_pos(center)` for the dots, the label, and the deposit mark.
    - `draw_map` passes the `Point` it already computed.
    - What is drawn, and where, is unchanged apart from sub-pixel rounding.
- **Tasks:**
  1. Add `Session::world()` and use it as above.
  2. Fix the doc.
  3. Change `paint_hex`'s `center` parameter.
- **Tests:**
  - Add a `session.rs` test: `world()` is None for a new session and for `many_errors`, and is the world for `settings/sample`.
  - The window is checked by hand at Code Review: hover, select, the outline, labels, deposit marks, and the Step buttons and Space/Shift+Space enabling work as before.

### C08: All dev UI wording in one module
- **Status:** approved
- **Summary:** `check.rs` holds the wording that the `check` command and the window share (`error_lines`, `count`, `list`, `turn_line`), and `inspect.rs` holds the panels' wording. The designs require `check` and the window to use the same wording, so all of it moves into one module named for what it is. `check.rs` keeps only the headless checker.
- **Changes:**
  - Rename `crates/dev-ui/src/inspect.rs` to `crates/dev-ui/src/text.rs`. Its module doc becomes: "All the text the dev UI shows: the `check` output (load errors, counts, turn lines) and the window's panels. It's built from engine types without egui, so it can be tested. The wording follows the Step 02–08 designs."
  - Move `error_lines`, `count`, `list` (still private to the module), and `turn_line` from `check.rs` into `text.rs`, with their doc comments. The `error_lines_lists_each_error_then_a_summary` test moves with them. Drop the "`pub(crate)` because the window uses it too" remarks; visibility is just `pub`, like the other functions in the module.
  - `crates/dev-ui/src/check.rs`:
    - Module doc: "The headless setting checker (`dev-ui check`)."
    - It imports `count`, `error_lines`, and `turn_line` from `crate::text`.
    - `run` and `run_turns` are unchanged.
  - `crates/dev-ui/src/main.rs`: `mod inspect;` becomes `mod text;`.
  - `crates/dev-ui/src/app.rs` and `session.rs`: import from `crate::text` instead of `crate::check` / `crate::inspect`. Update the `app.rs` module doc's mention of `inspect`.
  - Do C03 and C05 first: they edit `inspect.rs`, which this recommendation renames.
- **Tasks:**
  1. Rename the module and move the functions and the test.
  2. Fix the imports and docs.
- **Tests:** No test is added or removed. `crates/dev-ui/tests/checker.rs` (exact `check` output) and every unit test pass unchanged.

### C09: Shared "first defined at" duplicate detector for hexes and deposits
- **Status:** rejected
- **Reason rejected:** It saves only a few lines. `MapReader` still needs its coordinate-to-index lookup, so only the deposit reader would really shrink.

### C10: Shared bounded-integer reader for `perishes_after`, `count`, `q`/`r`
- **Status:** rejected
- **Reason rejected:** The three report out-of-range values with different wording ("must be at least 1", "is out of range", "is out of range (the most is 1000000)"). Sharing one reader would change the error text that the designs and tests fix.

## Packages
### P1: engine
- Owns: `crates/engine/`
- Tasks: C01, C02, C04, C06 (in that order)
- Isolated tests: `cargo fmt --all -- --check`, `cargo clippy -p policyspace-engine --all-targets -- -D warnings`, `cargo test -p policyspace-engine`
### P2: dev-ui
- Owns: `crates/dev-ui/`
- Tasks: C03, C05, C07, C08 (in that order)
- Isolated tests: `cargo fmt --all -- --check`, `cargo clippy -p policyspace-dev-ui --all-targets -- -D warnings`, `cargo test -p policyspace-dev-ui`
### Integration
- Owns: nothing (no shared files change; the engine's public interface stays the same, so P2 builds against the safe-point engine)
- Tasks: none beyond checking the merged tree
- Tests: the full check, `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests)

## Done When
Every approved recommendation (C01–C08) is applied, `.venv/Scripts/python tools/check.py` passes, and no test's assertions changed except the `should_panic` strings listed in C01 Task 4 and the tests moved by C02, C04, C05, and C08.
