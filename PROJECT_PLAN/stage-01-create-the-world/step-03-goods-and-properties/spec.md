# Step 03: goods-and-properties — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-03-goods-and-properties/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-02-setting-loader/spec.md` and `docs_changes.md`: how the loader framework works (`SectionType`, `SettingContent`, `WorldContent`, `EntryReader`, `Definitions`, `ErrorSink`, the message catalogue, the error order).
- `PROJECT_PLAN/stage-01-create-the-world/step-01-engine-workspace/docs_changes.md` (the `ids.md` part): the ID mechanism.
- Code this Step extends:
  - `crates/engine/src/lib.rs`
  - `crates/engine/src/id.rs` (read only; don't change it)
  - `crates/engine/src/setting/mod.rs`, `discover.rs`, `reader.rs`, `defs.rs`, `test_content.rs`
  - `crates/engine/tests/setting_loader.rs`, `crates/engine/tests/settings/` (and its `README.md`)
  - `crates/dev-ui/src/main.rs`, `crates/dev-ui/tests/checker.rs`
  - `settings/sample/`
- `Cargo.toml` (workspace): `missing_docs = "warn"`, clippy `all`, and `tools/check.py` runs clippy with `-D warnings`. Every public item and public field needs a doc comment.

## Approach
- **Goods get their own engine module, `goods`, separate from the loader.** It holds the good kind, the properties, and the goods list, and knows nothing about files. The world state (Step 06) and the dev UI use it without touching the loader. Hexes and deposits will follow the same split in Steps 04–05.
- **The loader gets a small reader, `setting/goods.rs`**, that reads `[[good]]` sections and builds the `Goods` list. `[[good]]` is the first real `SectionType` in `WorldContent`.
- **Only the loader builds a `Goods`.** Its constructor is crate-private. Callers can only read it, so a `Goods` always comes from a checked setting.
- **Property defaults are applied by the reader.** `Good` stores final values (`Perishes::Never`, `movable: true`), not `Option`s. The design says callers never see "key missing".
- **"Never perishes" is an enum variant, not a magic number.** `Perishes::Never` / `Perishes::After(NonZeroU32)` makes the "≥ 1" rule part of the type.
- **Reference checking gets a second skip condition.** Discovery reports whether it saw every data file. Phase 2 runs only if no file was rejected *and* discovery was complete.
- **Test settings that used `[[good]]` (and `[[hex]]`) as examples of unknown sections switch to made-up headers**, since `good` is now real and `hex` will be in Step 04.

## Changes
Create:
- `crates/engine/src/goods.rs`: `GoodKind`, `GoodId`, `Perishes`, `Good`, `Goods`, and unit tests.
- `crates/engine/src/setting/goods.rs`: crate-private `GoodsReader`, `read_good`, and unit tests if useful (most tests are integration tests).
- `crates/engine/tests/goods.rs`: integration tests for goods through the public API.
- `settings/sample/rules/goods.toml`.
- Test settings `goods_valid`, `goods_one`, `goods_errors` under `crates/engine/tests/settings/` (see Test Plan).

Modify:
- `crates/engine/src/lib.rs`: `pub mod goods;`, re-exports, crate docs mention goods.
- `crates/engine/src/setting/mod.rs`: `mod goods;`; `Rules` gains `goods`; `WorldContent` gains the good section type; `load_setting` builds `Rules` from the reader; phase 2 skip condition; module docs and stale doc comments ("Empty until Step 03", "no fields yet").
- `crates/engine/src/setting/discover.rs`: the `complete` flag on `Discovered`.
- `crates/engine/src/setting/reader.rs`, `defs.rs`, `mod.rs`: remove `#[allow(dead_code)]` (and its "Used by section types from Step 03 on" comment) from every item that real code now uses. Keep it, with an updated comment, only on items still used only by tests. Clippy decides: an item still dead in non-test builds keeps the allow.
- `crates/engine/src/setting/test_content.rs`: new framework tests for the phase 2 skip.
- `crates/engine/tests/setting_loader.rs`: updated expectations for the renamed unknown-section examples; the sample-setting test checks goods.
- `crates/engine/tests/settings/unknown_section/`, `crates/engine/tests/settings/many_errors/`: headers renamed (see Test Plan).
- `crates/engine/tests/settings/README.md`: describe the new and changed folders.
- `crates/dev-ui/src/main.rs`: goods count on the success line; crate docs.
- `crates/dev-ui/tests/checker.rs`: new expected lines.
- `settings/sample/rules/README.md`: mention that goods are defined here.

## Interfaces & Data

### `crates/engine/src/goods.rs` (public)
Module docs: a good is anything an actor can own (labor, resource output, intermediate and consumption goods alike). There are no built-in goods and no categories; every good comes from the setting's `[[good]]` sections. The engine understands exactly two properties, and explains what each means (see `Perishes` below). Goods are identified by `GoodId`, valid only within the `Goods` (setting) that produced them.

```rust
/// Marker kind for goods. LABEL = "good".
pub enum GoodKind {}
impl IdKind for GoodKind { const LABEL: &'static str = "good"; }

/// The ID of a good.
pub type GoodId = Id<GoodKind>;

/// When units of a good perish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Perishes {
    /// Never perishes (a durable good). The default when `perishes_after` is absent.
    Never,
    /// A unit survives this many turn-ends, counting the turn it appears in:
    /// `After(1)` is usable in the turn it appears and gone when that turn ends;
    /// `After(3)` appearing in turn 5 is gone at the end of turn 7.
    After(NonZeroU32),
}

/// The properties of one good, with defaults already applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Good {
    /// When units of this good perish.
    pub perishes: Perishes,
    /// Whether the good can be moved between places. No effect yet (movement and trade
    /// come after the Closed Market Stage). Defaults to true.
    pub movable: bool,
}

/// Every good in a setting, in load order (which is also ID order).
#[derive(Debug, Default)]
pub struct Goods { names: NameRegistry<GoodKind>, goods: Vec<Good> }   // goods[id.index()]

impl Goods {
    /// Builds the list. Crate-private: only the setting loader builds goods.
    /// Panics if the lengths differ (a programming error).
    pub(crate) fn from_parts(names: NameRegistry<GoodKind>, goods: Vec<Good>) -> Self;
    /// Number of goods.
    pub fn len(&self) -> usize;
    /// True if the setting defines no goods.
    pub fn is_empty(&self) -> bool;
    /// The ID of the good named `name`, or IdError::UnknownName.
    pub fn id(&self, name: &str) -> Result<GoodId, IdError>;
    /// The name of `id`. Panics if `id` isn't from this list.
    pub fn name(&self, id: GoodId) -> &str;
    /// The properties of `id`. Panics if `id` isn't from this list.
    pub fn get(&self, id: GoodId) -> &Good;
    /// Every good as (ID, name, properties), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, &str, &Good)> + '_;
}
```
- `#[derive(Debug, Default)]` on `Goods` works because `NameRegistry<K>` implements `Debug` and `Default` for every `K`. Don't derive `Clone` or `PartialEq` (the registry implements neither).
- `get` panics with a clear message on an out-of-range ID, matching `NameRegistry::name`.
- Unit tests here build a `Goods` with `from_parts` (crate-visible in tests).

### `crates/engine/src/lib.rs`
```rust
pub mod goods;
pub use goods::{Good, GoodId, GoodKind, Goods, Perishes};
```
Add a sentence to the crate docs: goods are defined by the setting (see the `goods` module).

### `Rules` (`setting/mod.rs`)
```rust
/// Persistent rules. Steps 05 onward add more fields.
#[derive(Debug, Default)]
pub struct Rules {
    /// Every good the setting defines, in load order.
    pub goods: Goods,
}
```
Update `LoadedSetting`'s field docs: `rules` is no longer empty; `start` is "Empty until Step 04" (unchanged).

### Reading goods (`setting/goods.rs`, crate-private)
```rust
/// Collects goods during phase 1.
#[derive(Default)]
pub(crate) struct GoodsReader {
    defs: Definitions<GoodKind>,
    goods: Vec<Good>,          // goods[id.index()], pushed exactly when defs.define returns Some
}
impl GoodsReader {
    /// Reads one [[good]] entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    /// The definitions, for resolving references to goods in phase 2 (Step 05 uses this).
    pub fn defs(&self) -> &Definitions<GoodKind>;   // add #[allow(dead_code)] + comment if unused until Step 05
    /// The finished list.
    pub fn finish(self) -> Goods;                    // Goods::from_parts(defs.into_registry(), goods)
}
```
`read`, in this order:
1. `required_name("name")`.
2. `optional_integer("perishes_after")`, then validate:
   - absent → `Perishes::Never`;
   - `n < 1` → error at the value: `key 'perishes_after' in good must be at least 1`;
   - `n > u32::MAX` → error at the value: `key 'perishes_after' in good is out of range` (the same text the reader uses for values beyond `i64`);
   - otherwise `Perishes::After(NonZeroU32::new(n as u32))`. Use `u32::try_from` and `NonZeroU32::new`, not unchecked casts.
3. `optional_bool("movable")` → absent means `true`.
4. If the name was valid, `defs.define(&name, reader.errors())`. If it returns `Some(id)`, push the `Good`. If a property was invalid, push the `Good` with that property's default. The load fails anyway, but the name must still be defined, so a duplicate later is still caught and a reference from Step 05 doesn't produce a misleading "unknown good" error.

Read all three keys before defining, and always read every key even when an earlier one failed, so no key is falsely reported as unknown. `finish()` on the `EntryReader` is called by the loader, never here.

Messages added to the catalogue (exact; tests assert them):

| Situation | Message (after the location) |
|---|---|
| `perishes_after` below 1 | `key 'perishes_after' in good must be at least 1` |
| `perishes_after` above `u32::MAX` | `key 'perishes_after' in good is out of range` |

Everything else uses existing messages: `missing required key 'name' in good`, `invalid name '…' (…)`, `duplicate good name 'x' (first defined at …)`, `unknown key 'k' in good`, `key 'perishes_after' in good must be an integer, found {type_str}`, `key 'movable' in good must be a boolean, found {type_str}`, `section 'good' belongs in rules/, not start/`.

### `WorldContent` (`setting/mod.rs`)
```rust
#[derive(Default)]
pub(crate) struct WorldContent {
    pub goods: GoodsReader,
    pub start: StartState,
}
// section_types(): vec![SectionType { header: "good", folder: Folder::Rules, read: |c, r| c.goods.read(r) }]
//   (a closure without captures coerces to fn; or a named fn read_good)
// resolve(): still a no-op (goods have no references).
```
`load_setting` builds `LoadedSetting { manifest, rules: Rules { goods: content.goods.finish() }, start: content.start }`.

Update the module docs' "Adding a section type" list: it now has a real example (goods: domain types in `crate::goods`, the reader in `setting/goods.rs`, data in `Rules`). Mention the phase 2 skip conditions in "Strictness and error reporting".

### Discovery completeness (`setting/discover.rs`)
```rust
pub(crate) struct Discovered {
    pub manifest: Option<PathBuf>,
    pub data_files: Vec<DataFile>,
    /// False if discovery may have missed data files in rules/ or start/.
    pub complete: bool,
}
```
`complete` starts `true` and becomes `false` when any of these happen:
- `rules/` or `start/` is missing, or exists as a file instead of a folder;
- the root folder can't be listed;
- an entry at the root, or anywhere in `rules/` or `start/`, can't be identified: a name that isn't valid UTF-8, a `read_dir` item error, or a `fs::metadata` failure. At the root, the unidentified entry could be `rules/` or `start/`, so it counts too;
- a folder inside `rules/` or `start/` (including `rules/`/`start/` themselves) can't be listed.

Implementation hint: have `list_dir` report, alongside its entries, whether it had to leave anything out or failed outright, so every error path in it sets the flag in one place. `walk` propagates the flag.

These don't clear the flag: a missing manifest, an unexpected root file or folder, a non-`.toml` file or `README.md` handled as today, ignored files.

### Phase 2 (`load_with`)
```rust
if !rejected && discovered.complete { content.resolve(&mut sink); }
```
Update the comment above it.

### Dev UI (`crates/dev-ui/src/main.rs`)
- The success line becomes `Loaded setting '{name}' (format version {v}): {n} good{s}`, where the suffix is empty for `n == 1` and `s` otherwise: `0 goods`, `1 good`, `5 goods`. Use a small helper (`fn count(n, singular, plural) -> String` or similar), because later Steps add `, 7 hexes` and so on.
- Everything else is unchanged. Update the crate docs to mention the counts.

### Sample setting (`settings/sample/rules/goods.toml`)
A comment header explaining the two properties and their defaults, then, in this order:

| `name` | `perishes_after` | `movable` |
|---|---|---|
| `unskilled_labor` | 1 | false |
| `unimproved_copper` | 1 | false |
| `unimproved_grain` | 1 | false |
| `copper` | (omitted) | (omitted) |
| `grain` | 3 | (omitted) |

One short comment per good saying its role (labor from Stage 02; resource output for Step 05; an intermediate good; a perishable movable good). Add one line to `settings/sample/rules/README.md` saying goods are defined in `goods.toml`.

## Tasks
1. Create `crates/engine/src/goods.rs` with module docs, the types, and unit tests. Add it to `lib.rs` with the re-exports.
2. Add the `complete` flag to `discover.rs`, and extend its unit tests (see Test Plan).
3. Create `setting/goods.rs` with `GoodsReader`.
4. Wire it into `setting/mod.rs`: `Rules.goods`, `WorldContent`, `load_setting`, the phase 2 condition, the module docs.
5. Remove the `#[allow(dead_code)]` markers that are no longer needed.
6. Rename the unknown-section examples in the `unknown_section` and `many_errors` test settings, and update the expectations in `setting_loader.rs` and `checker.rs`.
7. Create the goods test settings, and update `crates/engine/tests/settings/README.md`.
8. Write `crates/engine/tests/goods.rs`.
9. Add the phase 2 skip tests to `test_content.rs`.
10. Create `settings/sample/rules/goods.toml`, and update the sample's `rules/README.md`.
11. Dev UI: the goods count, and the updated and new checker tests.
12. Run `python tools/check.py` and fix everything until it passes. Run `cargo run -p policyspace-dev-ui -- settings/sample` and `… -- crates/engine/tests/settings/goods_errors`, and check the output by eye.

## Test Plan

### Changed test settings
- `unknown_section`: `rules/goods.toml` becomes `rules/widgets.toml`, containing `[[widget]]` (with `name = "copper"`). `start/map.toml` becomes `start/gadgets.toml`, keeping the comment line and blank line and using `[[gadget]]`. The expected errors become `rules/widgets.toml:1:3: unknown section 'widget'` and `start/gadgets.toml:3:3: unknown section 'gadget'`.
- `many_errors`: `rules/a.toml`'s `[[good]]` becomes `[[widget]]`. The expected line becomes `rules/a.toml:1:3: unknown section 'widget'`, in both `setting_loader.rs` and `checker.rs`.

### New test settings (`crates/engine/tests/settings/`)
| Folder | Contents |
|---|---|
| `goods_valid` | Valid manifest. `rules/a.toml`: `grain` (`perishes_after = 3`), then `copper` (name only). `rules/a/z.toml`: `labor` (`perishes_after = 1`, `movable = false`). `rules/b.toml`: `tools` (`movable = true` written out). `start/README.md`. Expected load order: `grain`, `copper`, `labor`, `tools`. |
| `goods_one` | Valid manifest; `rules/goods.toml` with one good; `start/README.md`. |
| `goods_errors` | Valid manifest. `rules/a.toml` has these `[[good]]` entries, in order: (1) `name = "copper"`; (2) no `name`, `movable = false`; (3) `name = "Copper"`; (4) `name = "zero"`, `perishes_after = 0`; (5) `name = "negative"`, `perishes_after = -2`; (6) `name = "fraction"`, `perishes_after = 1.5`; (7) `name = "huge"`, `perishes_after = 4294967296`; (8) `name = "wordy"`, `movable = "yes"`; (9) `name = "typo"`, `perishes = 1`; (10) `name = "Bad-Name"`, `perishes_after = 0`, `movable = 1`, `colour = "red"`. `rules/b.toml`: `name = "copper"` again. `start/s.toml`: a `[[good]]` with `name = "iron"`. |

### Tests
Engine integration tests are in `crates/engine/tests/goods.rs` (public API only). Framework tests that need `TestContent` are in `test_content.rs`. Discovery unit tests are in `discover.rs`. Assert exact `Display` strings, with exact line and column, wherever practical.

| # | Test | Acceptance Criterion |
|---|---|---|
| 1 | Sample setting: `rules.goods` has 5 goods, in the table's order, with the properties from the table (`Perishes::After(1)`, `After(3)`, `Never`; `movable` false/false/false/true/true). Update `sample_setting_loads` in `setting_loader.rs` or add a test in `goods.rs`. | sample setting loads with the five goods |
| 2 | `goods_valid` loads. `iter()` yields `(0, grain), (1, copper), (2, labor), (3, tools)`. `id("labor")` is index 2, `name` of each ID round-trips, and `get` returns the expected properties. `copper` (no keys) is `Perishes::Never` and movable. | valid setting across files and nested folders; IDs in load order; name/ID/properties; defaults |
| 3 | Loading `goods_valid` twice gives the same `(index, name, Good)` list. | loading twice gives the same IDs |
| 4 | `id("missing")` on the goods list returns `IdError::UnknownName { kind: "good", .. }`. | look-up by name |
| 5 | `goods_errors` gives exactly this list, in sorted order, with exact positions (fill in lines and columns from the file): `missing required key 'name' in good` at entry 2's header; `invalid name 'Copper' …`; `key 'perishes_after' in good must be at least 1` (entry 4); the same for entry 5; `key 'perishes_after' in good must be an integer, found float` (entry 6, using whatever `type_str` returns for a float; check it); `key 'perishes_after' in good is out of range` (entry 7); `key 'movable' in good must be a boolean, found string` (entry 8); `unknown key 'perishes' in good` (entry 9); for entry 10, all four of `invalid name 'Bad-Name' …`, `… must be at least 1`, `key 'movable' in good must be a boolean, found integer`, `unknown key 'colour' in good`; `rules/b.toml:…: duplicate good name 'copper' (first defined at rules/a.toml:…)`; `start/s.toml:…: section 'good' belongs in rules/, not start/`. | every listed good error, at file/line/column; several problems in one good all reported |
| 6 | `valid_minimal` loads, and `rules.goods.is_empty()`. | a setting with no goods loads |
| 7 | `goods.rs` unit tests: `from_parts` with matching lengths; `get`/`name` panic on a foreign ID (`#[should_panic]`); `from_parts` panics on mismatched lengths (`#[should_panic]`); `iter` order. | (robustness) |
| 8 | `discover.rs` unit tests: `complete` is true for `valid_nested`, for a setting with a missing manifest, for one with an unexpected root file and folder, and for one with a non-`.toml` file in `rules/`. It is false for a missing `rules/`, a missing `start/`, and `rules` as a file. Calling `walk` (or the function that lists a data folder) on a path that doesn't exist reports `could not read folder: …` and sets the flag to false. That is the unreadable-subfolder path. | skip conditions (discovery side) |
| 9 | Non-UTF-8 file name: a test that creates a file in `rules/` whose name isn't valid UTF-8 and checks that the flag is false and the `file name is not valid UTF-8` error is reported. On Unix, use `OsStr::from_bytes(b"bad\xFF.toml")`. On Windows, use `OsString::from_wide` with an unpaired surrogate (`0xD800`) in the name. Gate each with `#[cfg(unix)]` / `#[cfg(windows)]`. If the platform refuses to create the file, the test must fail with a clear message, not silently pass. Report it if this happens on Windows. | skip conditions (bad file name) |
| 10 | `test_content.rs`: a temp setting with no `rules/` folder, and `start/places.toml` placing `copper`, gives only `rules/: missing required folder`, with no `unknown thing`. The same with `rules` as a file gives only `rules: expected a folder, found a file`. A setting with `rules/a.toml` defining `iron`, an unknown key in that entry, `rules/notes.txt`, and a placed thing referring to `bronze` still reports `unknown thing 'bronze'`, alongside the other two errors. | reference check skipped after discovery problems; not skipped for other problems |
| 11 | All existing tests still pass, with the expectations changed only as listed under "Changed test settings". The existing `phase_two_is_skipped_after_a_broken_file` stays unchanged. | (regression) |
| 12 | `checker.rs`: the sample prints `Loaded setting 'Sample Setting' (format version 1): 5 goods`. `goods_one` prints `…: 1 good` (its manifest name and version). `valid_minimal` prints `…: 0 goods`. `many_errors` gives the updated expected lines and exit code 1. The two-argument test is unchanged. `placeholder.rs` is unchanged. | dev UI counts; no-argument and broken-setting behavior unchanged |
| 13 | `python tools/check.py` exits 0. | single check command passes |

Run everything with `python tools/check.py` (or `.venv/Scripts/python tools/check.py`).

## Constraints
- No new external crates.
- Don't change `crates/engine/src/id.rs`. If it seems to need a change, stop and report.
- `goods.rs` must not depend on anything in `setting/`. The dependency runs from `setting` to `goods` only.
- `GoodsReader`, `Definitions`, `EntryReader`, `SectionType`, `SettingContent`, `WorldContent`, and `load_with` stay `pub(crate)`. `Goods::from_parts` stays `pub(crate)`.
- Determinism: goods are listed and numbered in load order. Never iterate a `HashMap` to produce output or IDs.
- No `unwrap`/`expect` on anything a setting file controls.
- `movable` has no behavior beyond being stored. Don't add perishing logic, ages, or a turn. Don't add resource types or any other section type.
- Don't touch `PROJECT_PLAN/`, `CLAUDE.md`, `.claude/`, `docs/`, `manual.ipynb`, `manual.html`, or `tools/`.
- Don't commit.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` exits 0, and every Acceptance Criterion in design.md is met.
