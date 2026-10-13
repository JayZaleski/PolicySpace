# Step 04: hex-map — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-04-hex-map/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-03-goods-and-properties/spec.md` and `docs_changes.md`: the goods pattern this Step copies (domain module separate from the loader, a crate-private reader, a crate-private constructor, test layout).
- `PROJECT_PLAN/stage-01-create-the-world/step-02-setting-loader/docs_changes.md`: the loader framework (`SectionType`, `SettingContent`, `WorldContent`, `EntryReader`, `ErrorSink`, error order).
- Code this Step extends:
  - `crates/engine/src/lib.rs`
  - `crates/engine/src/id.rs` (doc comments only; see Constraints)
  - `crates/engine/src/goods.rs` and `crates/engine/src/setting/goods.rs` (the pattern to follow)
  - `crates/engine/src/setting/mod.rs`, `reader.rs`, `error.rs`
  - `crates/engine/tests/goods.rs`, `crates/engine/tests/setting_loader.rs`, `crates/engine/tests/settings/` (and its `README.md`)
  - `crates/dev-ui/src/main.rs`, `crates/dev-ui/tests/checker.rs`
  - `settings/sample/`
- `Cargo.toml` (workspace): `missing_docs = "warn"`, clippy `all`, and `tools/check.py` runs clippy with `-D warnings`. Every public item and public field needs a doc comment.

## Approach
- **Hexes get their own engine module, `map`, separate from the loader**, as goods did. It holds the coordinate type, the hex ID kind, and the map, and knows nothing about files.
- **A hex is identified by its coordinates, not a name, so there is no `NameRegistry`.** `HexMap` hands out `HexId`s directly from load order with `Id::from_index` (already `pub(crate)`), and keeps a coordinate → ID lookup table.
- **The loader gets a small reader, `setting/map.rs`**, that reads `[[hex]]` sections. `[[hex]]` is the first `SectionType` in `Folder::Start`.
- **Duplicate coordinates are caught by the reader's own table** (coordinates → where first defined), not by generalizing `Definitions`, which is keyed by names. One small map is simpler than making the name machinery generic for one caller; comp-review can revisit it if Step 05 needs the same thing.
- **Only the loader builds a `HexMap`.** Its constructor is crate-private, so a `HexMap` always comes from a checked setting.
- **Lookup by coordinates returns `Option<HexId>`.** "No hex there" is an ordinary answer, not an error type. Step 05 writes its own error message when a deposit names empty coordinates.
- **The orientation and the wrapping plan are documented in the `map` module docs**, since the engine (not the setting) fixes them and Step 07 draws from them.

## Changes
Create:
- `crates/engine/src/map.rs`: `HexKind`, `HexId`, `HexCoord`, `HexMap`, and unit tests.
- `crates/engine/src/setting/map.rs`: crate-private `MapReader`.
- `crates/engine/tests/map.rs`: integration tests through the public API.
- `settings/sample/start/map.toml`.
- Test settings `hexes_valid`, `hexes_one`, `hexes_errors` under `crates/engine/tests/settings/` (see Test Plan).

Modify:
- `crates/engine/src/lib.rs`: `pub mod map;`, re-exports, a crate-docs sentence.
- `crates/engine/src/id.rs`: module doc wording only (see Interfaces).
- `crates/engine/src/setting/mod.rs`: `mod map;`; `StartState` gains `map`; `WorldContent` loses its `start` field and gains `map: MapReader`; the hex section type; `load_setting` builds `StartState`; doc comments.
- `crates/engine/src/setting/reader.rs`: remove `#[allow(dead_code)]` and its comment from `EntryReader::location` (now used). Leave the others to clippy: an item still dead in non-test builds keeps its allow.
- `crates/engine/tests/setting_loader.rs`: `sample_setting_loads` also checks the hex count.
- `crates/engine/tests/settings/README.md`: describe the new folders.
- `crates/dev-ui/src/main.rs`: hex count on the success line; crate docs.
- `crates/dev-ui/tests/checker.rs`: new expected lines, and a one-hex test.
- `settings/sample/start/README.md`: one line saying the map is in `map.toml`.

## Interfaces & Data

### `crates/engine/src/map.rs` (public)
Module docs must say:
- The map is the set of hex tiles in a setting's starting state. A hex carries only its position for now; terrain and other attributes come later.
- Coordinates are **axial** (`q`, `r`). The engine fixes the orientation; settings don't choose it: hexes are **pointy-top**, each value of `r` is a straight east–west row, `q` increases eastward, `r` increases southward. Step 07's dev UI draws the map this way.
- A hex is identified by its coordinates. There are no hex names. Anything saved long-term stores coordinates, never `HexId`s, which are valid only within the `HexMap` that produced them.
- There are no shape rules: any set of distinct coordinates is a map (holes, islands, negative coordinates), and an empty map is valid.
- Neighbors, distance and wrapping don't exist yet. Wrapping is planned as an optional per-setting map value (a wrap width along `q`, off by default), possibly with a north–south rule (torus or pole crossing); the fixed orientation is what makes that possible. A true geodesic globe is not supported by axial coordinates.

```rust
/// Marker kind for hexes. LABEL = "hex".
pub enum HexKind {}
impl IdKind for HexKind { const LABEL: &'static str = "hex"; }

/// The ID of a hex.
pub type HexId = Id<HexKind>;

/// Axial coordinates of a hex (see the module docs for the orientation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HexCoord {
    /// Column: increases eastward.
    pub q: i32,
    /// Row: increases southward.
    pub r: i32,
}
impl HexCoord {
    /// Coordinates (q, r).
    pub const fn new(q: i32, r: i32) -> Self;
}
/// Displays as `(q, r)`, e.g. `(1, -2)`. Used in error messages.
impl fmt::Display for HexCoord { ... }

/// Every hex in a setting, in load order (which is also ID order).
#[derive(Debug, Default)]
pub struct HexMap {
    /// Indexed by `HexId::index`.
    coords: Vec<HexCoord>,
    /// Lookup only; never iterated (iteration order must not depend on hashing).
    by_coord: HashMap<HexCoord, HexId>,
}

impl HexMap {
    /// Builds the map, giving IDs 0, 1, 2, ... in the order given.
    /// Crate-private: only the setting loader builds a map.
    ///
    /// # Panics
    /// Panics if two entries have the same coordinates (the loader rejects duplicates
    /// first, so this is a programming error), or if there are more than u32::MAX hexes.
    pub(crate) fn from_coords(coords: Vec<HexCoord>) -> Self;
    /// Number of hexes.
    pub fn len(&self) -> usize;
    /// True if the setting defines no hexes.
    pub fn is_empty(&self) -> bool;
    /// The hex at `coord`, or None if there is none.
    pub fn id(&self, coord: HexCoord) -> Option<HexId>;
    /// The coordinates of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this map.
    pub fn coord(&self, id: HexId) -> HexCoord;
    /// Every hex as (ID, coordinates), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (HexId, HexCoord)> + '_;
}
```
- `coord` panics with a message containing `is not from this map` (mirror `Goods::get`).
- `from_coords` panics with a message containing `duplicate hex` on a duplicate. Use `u32::try_from(index).expect(...)` for the ID, as `NameRegistry` does.
- `iter` walks `coords` with `enumerate`, never `by_coord`.
- `HexMap` derives only `Debug` and `Default`, like `Goods`.

### `crates/engine/src/lib.rs`
```rust
pub mod map;
pub use map::{HexCoord, HexId, HexKind, HexMap};
```
Add a crate-docs sentence: the map is a set of hexes in the setting's starting state (see the `map` module).

### `crates/engine/src/id.rs` (doc comments only)
The module docs currently say that setting files refer to every kind of thing by text name, and that anything saved stores names. Reword them so they're true for hexes too: most kinds are named and get IDs from a `NameRegistry`; some kinds, such as hexes, are identified by other data (coordinates) and their list hands out IDs directly; anything saved long-term stores names or those identifying data, never IDs. Don't change any code, signatures, or tests in this file.

### `StartState` and `LoadedSetting` (`setting/mod.rs`)
```rust
/// Starting state (turn 0). Step 05 onward add more fields.
#[derive(Debug, Default)]
pub struct StartState {
    /// Every hex in the setting's map, in load order.
    pub map: HexMap,
}
```
- `LoadedSetting::start` doc: drop "Empty until Step 04".
- `load_setting` builds `start: StartState { map: content.map.finish() }`.

### Reading hexes (`setting/map.rs`, crate-private)
```rust
/// Collects hexes during phase 1.
#[derive(Default)]
pub(crate) struct MapReader {
    /// Indexed by the future `HexId::index`.
    coords: Vec<HexCoord>,
    /// Where each placed hex's `[[hex]]` header is; indexed like `coords`.
    locations: Vec<Location>,
    /// Coordinates → index into `coords`. Lookup only; never iterated.
    by_coord: HashMap<HexCoord, usize>,
}
impl MapReader {
    /// Reads one [[hex]] entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    /// The finished map.
    pub fn finish(self) -> HexMap;   // HexMap::from_coords(self.coords)
}
```
`read`:
1. `let at = reader.location().clone();` (the `[[hex]]` header's position).
2. Read `q`, then `r`, each with `required_integer`, then convert to `i32` with `i32::try_from`. If it doesn't fit, report at the value: `key 'q' in hex is out of range` (or `'r'`), the same text the reader uses beyond `i64`. Read both keys even if the first failed, so neither is falsely reported as unknown. A small helper `fn coordinate(reader, key) -> Option<i32>` keeps this tidy.
3. Only if both are valid: if `by_coord` already holds the coordinates, report at `at`: `duplicate hex at (q, r) (first defined at {first location})`, and don't place it. Otherwise push the coordinates and location, and record the index.
4. A hex with a missing or invalid coordinate is not placed, so it can't produce a duplicate error. The load fails anyway.

`EntryReader::finish` is called by the loader, never here; it reports unknown keys such as `terrain`.

Messages added to the catalogue (exact; tests assert them):

| Situation | Message (after the location) |
|---|---|
| `q`/`r` outside `i32` (but within `i64`) | `key 'q' in hex is out of range` / `key 'r' in hex is out of range` |
| Two hexes at the same coordinates | `duplicate hex at (0, 0) (first defined at start/a.toml:1:1)` (at the second `[[hex]]` header) |

Everything else uses existing messages: `missing required key 'q' in hex`, `key 'q' in hex must be an integer, found float`, `key 'q' in hex is out of range` (beyond `i64`, from the reader), `unknown key 'terrain' in hex`, `section 'hex' belongs in start/, not rules/`.

### `WorldContent` (`setting/mod.rs`)
```rust
#[derive(Default)]
pub(crate) struct WorldContent {
    pub goods: GoodsReader,
    pub map: MapReader,
}
// section_types(): good (Folder::Rules) as now, plus
//   SectionType { header: "hex", folder: Folder::Start, read: |c, r| c.map.read(r) }
// resolve(): still a no-op. Update its comment: goods and hexes refer to nothing.
```
Update the module docs' "Adding a section type" list: hexes are the worked example of a starting-state section type (domain types in `crate::map`, reader in `setting/map.rs`, data in `StartState::map`), and an example of a kind identified by something other than a name, which checks its own duplicates.

### Dev UI (`crates/dev-ui/src/main.rs`)
- The success line becomes `Loaded setting '{name}' (format version {v}): {goods}, {hexes}`, using the existing `count` helper: `count(n, "hex", "hexes")`. Examples: `5 goods, 19 hexes`; `1 good, 0 hexes`; `0 goods, 1 hex`.
- Crate docs: the counts are now goods and hexes.
- Everything else is unchanged.

### Sample setting (`settings/sample/start/map.toml`)
A comment header explaining: axial coordinates; pointy-top orientation, `r` rows east–west, `q` eastward, `r` southward; no shape rules; this map is a hexagon of radius 2 around (0, 0). Then 19 `[[hex]]` entries, row by row from north to south, west to east within each row:

| `r` | `q` values |
|---|---|
| −2 | 0, 1, 2 |
| −1 | −1, 0, 1, 2 |
| 0 | −2, −1, 0, 1, 2 |
| 1 | −2, −1, 0, 1 |
| 2 | −2, −1, 0 |

A short comment line before each row (for example `# Row r = -2`) is welcome. Add one line to `settings/sample/start/README.md` saying the map is defined in `map.toml`.

## Tasks
1. Create `crates/engine/src/map.rs` with module docs, the types, and unit tests. Add it to `lib.rs` with the re-exports and the crate-docs sentence.
2. Reword the `id.rs` module docs (doc comments only).
3. Create `setting/map.rs` with `MapReader`.
4. Wire it into `setting/mod.rs`: `StartState.map`, `WorldContent`, the hex section type, `load_setting`, the doc comments.
5. Remove the `#[allow(dead_code)]` on `EntryReader::location`, and any other allow that clippy shows is no longer needed.
6. Create the hex test settings and update `crates/engine/tests/settings/README.md`.
7. Write `crates/engine/tests/map.rs`, and extend `sample_setting_loads` in `setting_loader.rs`.
8. Create `settings/sample/start/map.toml` and update the sample's `start/README.md`.
9. Dev UI: the hex count, and the updated and new checker tests.
10. Run `python tools/check.py` and fix everything until it passes. Run `cargo run -p policyspace-dev-ui -- settings/sample` and `… -- crates/engine/tests/settings/hexes_errors`, and check the output by eye.

## Test Plan

### New test settings (`crates/engine/tests/settings/`)
| Folder | Contents |
|---|---|
| `hexes_valid` | Valid manifest. `rules/README.md`. `start/a.toml`: (0, 0), then (1, −1). `start/a/z.toml`: (−3, 2). `start/b.toml`: (2147483647, −2147483648). Expected load order: (0, 0), (1, −1), (−3, 2), (i32::MAX, i32::MIN). |
| `hexes_one` | Manifest `name = "Hexes Test"`, format version 1; `rules/README.md`; `start/map.toml` with one hex at (0, 0). |
| `hexes_errors` | Valid manifest. `start/a.toml` has these `[[hex]]` entries, in order: (1) `q = 0`, `r = 0`; (2) `r = 1` only; (3) `q = 0` only; (4) `q = 1.5`, `r = 0`; (5) `q = "2"`, `r = 0`; (6) `q = 2147483648`, `r = 0`; (7) `q = 0`, `r = -2147483649`; (8) `q = 3`, `r = 0`, `terrain = "hills"`; (9) `q = 1.5`, `r = 9999999999`, `terrain = "x"`. `start/b.toml`: `q = 0`, `r = 0` (duplicate of entry 1). `rules/h.toml`: a `[[hex]]` with `q = 5`, `r = 5`. Both `start/a.toml` and `start/b.toml` start with their first `[[hex]]` header on line 1 (no leading comments), so the duplicate message reads exactly as in test 6. |

### Tests
Engine integration tests go in `crates/engine/tests/map.rs` (public API only). Assert exact `Display` strings, with exact line and column, wherever practical.

| # | Test | Acceptance Criterion |
|---|---|---|
| 1 | Sample setting: `start.map` has 19 hexes, and `iter()` gives exactly the coordinates in the table's order, with IDs 0–18. Also extend `sample_setting_loads` in `setting_loader.rs` to check `start.map.len() == 19`. | sample loads with the 19 hexes in order |
| 2 | `hexes_valid` loads. `iter()` gives `(0, (0,0)), (1, (1,-1)), (2, (-3,2)), (3, (i32::MAX, i32::MIN))`. For each, `id(coord) == Some(id)` and `coord(id) == coord`. | valid map across files, nested folder, negative coordinates; IDs in load order; coordinates and lookup |
| 3 | Loading `hexes_valid` twice gives the same `(index, coord)` list. | loading twice gives the same IDs |
| 4 | `id(HexCoord::new(9, 9))` on `hexes_valid` is `None`. | "no such hex" result |
| 5 | `valid_minimal` loads, and `start.map.is_empty()`. | a setting with no hexes loads |
| 6 | `hexes_errors` gives exactly this list, in sorted order, with exact positions (fill in lines and columns from the files): `rules/h.toml:…: section 'hex' belongs in start/, not rules/`; entry 2: `missing required key 'q' in hex` at its header; entry 3: `missing required key 'r' in hex` at its header (and **no** duplicate error, though its `q` matches entry 1); entry 4: `key 'q' in hex must be an integer, found float`; entry 5: `… must be an integer, found string`; entry 6: `key 'q' in hex is out of range`; entry 7: `key 'r' in hex is out of range`; entry 8: `unknown key 'terrain' in hex`; entry 9: all three of `key 'q' in hex must be an integer, found float`, `key 'r' in hex is out of range`, `unknown key 'terrain' in hex`; `start/b.toml:1:1: duplicate hex at (0, 0) (first defined at start/a.toml:1:1)`. No other errors. | every listed hex error at file/line/column; several problems in one hex all reported; no duplicate error from an invalid hex; duplicate across files names both places |
| 7 | `map.rs` unit tests: `from_coords` gives sequential IDs and working `id`/`coord`/`iter`/`len`/`is_empty`; `HexMap::default()` is empty; `HexCoord` displays as `(1, -2)`; `coord` panics on a foreign ID (`#[should_panic(expected = "is not from this map")]`); `from_coords` panics on duplicate coordinates (`#[should_panic(expected = "duplicate hex")]`). | (robustness) |
| 8 | `checker.rs`: the sample prints `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes`. `goods_one` prints `…: 1 good, 0 hexes`. `valid_minimal` prints `…: 0 goods, 0 hexes`. New: `hexes_one` prints `Loaded setting 'Hexes Test' (format version 1): 0 goods, 1 hex`. `many_errors` and the two-argument test are unchanged, and so is `placeholder.rs`. | dev UI counts; no-argument and broken-setting behavior unchanged |
| 9 | All existing tests still pass, with expectations changed only as listed in test 8. | (regression) |
| 10 | `python tools/check.py` exits 0. | single check command passes |

Run everything with `python tools/check.py` (or `.venv/Scripts/python tools/check.py`).

## Constraints
- No new external crates.
- `id.rs`: doc comments only. No code, signature, or test changes. If it seems to need one, stop and report.
- `map.rs` must not depend on anything in `setting/`. The dependency runs from `setting` to `map` only.
- `MapReader`, `SectionType`, `SettingContent`, `WorldContent`, and `load_with` stay `pub(crate)`. `HexMap::from_coords` stays `pub(crate)`.
- Determinism: hexes are listed and numbered in load order. `HashMap`s are for lookup only; never iterate one to produce output, IDs, or errors.
- No `unwrap`/`expect` on anything a setting file controls.
- No neighbors, distance, wrapping, terrain, or any hex attribute beyond `q` and `r`. No shape checks. Don't add deposits or any other section type.
- Don't change `Definitions` or the goods code.
- Don't touch `PROJECT_PLAN/`, `CLAUDE.md`, `.claude/`, `docs/`, `manual.ipynb`, `manual.html`, or `tools/`.
- Don't commit.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` exits 0, and every Acceptance Criterion in design.md is met.

## Follow-ups for comp-review
- **F1 (build-review, minor; deferred by the user):** `crates/engine/src/id.rs` item docs still describe IDs as coming only from name registries, while `HexMap` now hands out IDs too. The `compile_fail` example on `Id` registers a hex by name (hexes have no names; switch it to another made-up kind such as `DepositKind`), and the `from_index` and `index` docs say "registries" (make it "registries or lists"). This spec limited the build to the module docs.
