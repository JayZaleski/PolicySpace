# Step 03: factories — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-03-factories/design.md` — the behavior this spec implements.
- `PROJECT_PLAN/stage-03-factories/step-02-businesses/spec.md` — the most recent section type; this Step copies its reader shape (every key read first, every entry kept for phase 2, references resolved in phase 2, the reader exposing what later readers need).
- `docs/setting-loader.md` — the two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/resources.md` (`Deposits`: the unnamed placed kind with a count and a per-hex index; `DepositsReader::resolve` expanding `count`), `docs/businesses.md` (`BusinessesReader`, `Businesses`), `docs/technologies.md`, `docs/world.md`, `docs/ids.md` (`IdVec`), `docs/dev-ui.md` (the checker's success line), `docs/workspace.md` (the check command, test layout).
- Code: `crates/engine/src/resources.rs` (`Deposit`, `Deposits`: the domain pattern to copy), `crates/engine/src/setting/resources.rs` (`DepositsReader`: `read`, `resolve` with the count expansion and total guard, `deposit_count` and its unit tests), `crates/engine/src/setting/businesses.rs` (`BusinessesReader`, `BusinessEntry`), `crates/engine/src/setting/technologies.rs` (`TechnologiesReader`), `crates/engine/src/setting/pops.rs` (`PopsReader`: `no hex at`), `crates/engine/src/setting/map.rs` (`coordinate`, `MapReader::id`), `crates/engine/src/setting/reader.rs` (`optional_number`, `optional_integer`, `has`, `required_string`, the `run` test helper), `crates/engine/src/setting/defs.rs`, `crates/engine/src/setting/mod.rs`, `crates/engine/src/businesses.rs`, `crates/engine/src/technologies.rs`, `crates/engine/src/id.rs` (`IdVec`, `Id::from_index`), `crates/engine/src/world.rs`, `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/businesses.rs` (helpers `businesses_of`, `owners_of`, `owned`, `business_id`; the determinism test), `crates/engine/tests/resources.rs` (`references_are_not_checked_after_a_broken_file_or_incomplete_discovery`), `crates/dev-ui/src/check.rs`, `crates/dev-ui/tests/checker.rs`, `settings/sample/start/businesses.toml` (header comment style), `settings/sample/start/deposits.toml`, `settings/sample/rules/technologies.toml`.

## Approach

- **A `[[factory]]` section in `start/`**, read by a new `FactoriesReader` (`setting/factories.rs`). Factories are unnamed (like deposits and POPs) but have no duplicate check (any number of identical factories is allowed), so the reader is `DepositsReader` without `seen`, keeping every entry for phase 2 as `BusinessesReader` does: an entry with a bad `condition` or `count` still has its technology, owner and hex checked. `technology` and `owner` are read as plain strings (not name-checked; a bad value is simply an unknown name in phase 2). Rationale: the design's "every key is read before anything is decided" and "a bad condition or count doesn't stop the reference checks".
- **Domain types in a new module `crates/engine/src/factories.rs`**: `FactoryKind`/`FactoryId`, `Factory` with public fields (a state record, like `Deposit` and `Pop`; `Technology`'s private fields are for a shared rules record with invariants), and `Factories`, an `IdVec` plus a per-hex index exactly like `Deposits`, with `owned_by(business)` as a filter like `Deposits::owned_by`. Rationale: the design names deposits as the model; lists are short, and consolidation can add a per-owner index to both if Step 05 wants one.
- **`condition` is a plain `f64`**: the share of the technology's capital bundle the factory holds. The loader guarantees `0 < c <= 1`; `Factories::from_factories` asserts `0 <= c <= 1` (finite), so Step 04 can store 0 when a factory is exhausted. Rationale: a share is not a quantity of a good, and the engine derives the goods from the technology's bundle when needed.
- **References resolved in phase 2** through `Definitions`: a new `TechnologiesReader::defs()` and a new `BusinessesReader::defs()`. The owner's-hex check uses a new `BusinessesReader::hex_of(id)`, so `FactoriesReader::resolve` runs last, after `BusinessesReader::resolve`. The `count` copies are pushed in phase 2 like deposits, with the same total guard.
- **The deposit `count` helper becomes shared**: `deposit_count` moves from `setting/resources.rs` to `setting/map.rs` as `pub(crate) fn count(reader)`, next to `coordinate`, with its messages built from `reader.label()`; `MAX_DEPOSIT_COUNT` becomes `MAX_COUNT` there. Deposit messages stay byte-for-byte the same. Rationale: the design says factories "reuse the count's rules and range".
- **World**: `factories: Factories` and `World::factories()`. Nothing in the turn touches factories; the report is unchanged.
- **Checker**: the success line gains `, N factories` at the end; a new `factories_one` test setting gives `1 factory`.
- **The build runs as one.** The reader needs the domain types and the two readers' new accessors; the World needs the domain types; every test (engine and dev UI) needs the loaded sample. No package could be built and tested alone.

## Changes

### New

- `crates/engine/src/factories.rs` — domain types and unit tests.
- `crates/engine/src/setting/factories.rs` — `FactoriesReader` and unit tests.
- `crates/engine/tests/factories.rs` — integration tests (Test Plan).
- Test settings `crates/engine/tests/settings/factories_valid/`, `factories_one/`, `factories_errors/` (exact contents below).
- `settings/sample/start/factories.toml`.

### Modified

- `crates/engine/src/lib.rs` — `pub mod factories; pub use factories::{Factories, Factory, FactoryId, FactoryKind};` after `demand`, before `goods`. Crate docs: a sentence that factories (the [`factories`] module) are placed in the starting state, each on its owner's hex, running one technology and holding a share (its condition) of that technology's capital bundle; nothing runs them yet.
- `crates/engine/src/setting/map.rs` — `count` and `MAX_COUNT` (moved from `setting/resources.rs`, below); module docs.
- `crates/engine/src/setting/resources.rs` — remove `deposit_count` and `MAX_DEPOSIT_COUNT`; `DepositsReader::read` calls `super::map::count`; the `deposit_count` unit tests stay here, calling `count` with the label `deposit` (every expected message unchanged); module docs.
- `crates/engine/src/setting/technologies.rs` — `pub fn defs(&self) -> &Definitions<TechnologyKind>` (like `PopTypesReader::defs`).
- `crates/engine/src/setting/businesses.rs` — `defs()` and `hex_of()` (below); module docs (factories refer to businesses by name and must sit on their owner's hex, which the factories reader checks through `hex_of`).
- `crates/engine/src/setting/mod.rs` — `mod factories;`, `StartState.factories`, `WorldContent.factories`, the `factory` section type, `resolve`, `load_setting`; module docs (factories are the sixth `start/` kind, unnamed, with no duplicate check; they name a technology and a business, and a hex by coordinates, all resolved in phase 2, where the loader also checks the hex is the owner's; the "Adding a section type" list names `crate::factories` / `setting/factories.rs` as the placed, counted kind that keeps every entry for phase 2 and reads another reader's results (`BusinessesReader::hex_of`)).
- `crates/engine/src/world.rs` — the `factories` field, `new`, `factories()`, module docs (the world holds the factories as state because their technology and condition change in later Steps; nothing runs, wears or maintains them yet).
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 factories` at the end (`SAMPLE_LINE` and the sample test: `, 3 factories`); a new test for `factories_one` (`1 factory`). The turn lines don't change.
- `crates/engine/tests/resources.rs` — the comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a factory of an unknown technology, owned by an unknown business, on a hex that doesn't exist).
- Test setting `resources_broken_file/start/s.toml` (append a `[[factory]]`, below).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- `settings/sample/start/README.md` — mention `factories.toml`.
- `settings/sample/rules/technologies.toml` — the paragraph "Nothing runs factories yet: technologies are only loaded and checked. Later Steps build and run factories from them." becomes: factories are placed in `start/factories.toml`, each holding a share of its technology's capital bundle, but nothing runs them yet; later Steps do.

## Interfaces & Data

### `crates/engine/src/factories.rs`

```rust
//! Module docs: Factories. A factory (`[[factory]]` in `start/`) is a discrete asset on a
//! hex that runs one technology (see `technologies`) and belongs to exactly one business
//! (see `businesses`). It carries the state that changes during play: which technology it
//! runs, and its condition, the share of its technology's full capital bundle it holds
//! (0 to 1; the goods in the factory are the technology's capital bundle times the
//! condition, derived whenever needed, because depreciation and maintenance only ever
//! change the bundle in its own proportions). Factories have no names: deciders, the dev
//! UI and later decisions address them by ID. A factory sits on its owner's hex until
//! trade between hexes exists (the loader checks this). A hex and a business may hold any
//! number of factories. Nothing runs, wears, maintains or shuts down a factory yet (Steps
//! 04 and 08), and nothing changes a factory during play yet. Knows nothing about files.

use crate::businesses::BusinessId;
use crate::id::{Id, IdKind, IdVec};
use crate::map::{HexId, HexKind};
use crate::technologies::TechnologyId;

/// Marker kind for factories.
pub enum FactoryKind {}
impl IdKind for FactoryKind {
    const LABEL: &'static str = "factory";
}
/// The ID of a factory. Valid only within the [`Factories`] that produced it.
pub type FactoryId = Id<FactoryKind>;

/// One factory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Factory {
    /// The hex it sits on: its owner's hex, until trade between hexes exists.
    pub hex: HexId,
    /// The technology it runs. State: a later Stage can retool it.
    pub technology: TechnologyId,
    /// The business that owns it. Exactly one; every factory has an owner.
    pub owner: BusinessId,
    /// The share of its technology's full capital bundle it holds: finite, from 0 to 1.
    /// A setting places a factory with a condition greater than 0 (1 if none is written);
    /// once depreciation runs (Step 04) the engine may store 0.
    pub condition: f64,
}

/// Every factory in a setting, in load order (which is also ID order), with a per-hex
/// index. `Factories::default()` is empty and was built with no map, so `on_hex` panics on
/// it for any hex.
#[derive(Debug, Default)]
pub struct Factories {
    /// One entry per factory, by `FactoryId`.
    factories: IdVec<FactoryKind, Factory>,
    /// One entry per hex, by `HexId`: the factories on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<FactoryId>>,
}

impl Factories {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map. Crate-private: only the loader builds factories.
    ///
    /// # Panics
    /// Panics if a factory's hex index is `>= hex_count` (`"{hex:?} is not from a map of
    /// {hex_count} hexes"`), if a condition isn't a finite number from 0 to 1 (`"a
    /// factory's condition must be a finite number from 0 to 1"`), or if there are more
    /// than `u32::MAX` factories.
    pub(crate) fn from_factories(factories: Vec<Factory>, hex_count: usize) -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// Panics (`is not from this list`) for a foreign ID.
    pub fn get(&self, id: FactoryId) -> &Factory;
    /// (ID, factory) in load order.
    pub fn iter(&self) -> impl Iterator<Item = (FactoryId, &Factory)> + '_;
    /// The factories on `hex`, in ID order; empty if none. Panics for a hex not from the
    /// map the list was built with.
    pub fn on_hex(&self, hex: HexId) -> &[FactoryId];
    /// The factories owned by `business`, in ID order (a filter over every factory).
    pub fn owned_by(&self, business: BusinessId) -> impl Iterator<Item = FactoryId> + '_;
}
```

`from_factories`: exactly `Deposits::from_deposits`, plus the condition assertion (`condition.is_finite() && (0.0..=1.0).contains(&condition)`) in the same loop. `factories.rs` imports `businesses` and `technologies`; neither imports `factories`, so there is no cycle.

### `setting/map.rs`: the shared `count`

```rust
/// The most copies one `[[deposit]]` or `[[factory]]` entry may place.
pub(crate) const MAX_COUNT: u32 = 1_000_000;

/// Reads the optional `count` key of a placed entry. Absent gives 1. A count outside
/// `1..=MAX_COUNT` is reported at the value and gives None: `key 'count' in <label> must be
/// at least 1` or `key 'count' in <label> is out of range (the most is 1000000)`. A count
/// that is not an integer is reported by the reader and also gives None.
pub(crate) fn count(reader: &mut EntryReader<'_, '_>) -> Option<u32>;
```

This is `deposit_count` with `reader.label()` in the messages (so for a `deposit` entry every message is unchanged) and `MAX_COUNT` in place of `MAX_DEPOSIT_COUNT`. Note `deposit_count`'s doc said a non-integer "also gives 1"; it gives None (the existing code and tests), and the new doc says so. Update the module docs of `setting/map.rs` (`coordinate` and `count` are shared by every section placed by coordinates, and `count` by every placed section with copies: deposits and factories) and `setting/resources.rs` (the count helper now lives in `setting/map.rs`).

### `TechnologiesReader::defs` and `BusinessesReader` additions

```rust
// setting/technologies.rs
/// The definitions, for resolving references to technologies in phase 2.
pub fn defs(&self) -> &Definitions<TechnologyKind>;

// setting/businesses.rs
/// The definitions, for resolving references to businesses in phase 2.
pub fn defs(&self) -> &Definitions<BusinessKind>;

/// After `resolve`: the hex of the business `id` and the coordinates it was written with,
/// or None if its coordinates were missing or invalid or matched no hex (already
/// reported). Factories use it to check that they sit on their owner's hex.
pub fn hex_of(&self, id: BusinessId) -> Option<(HexId, HexCoord)>;
```

`hex_of` scans `self.entries` for the entry with `id == Some(id)` (entries without an ID are skipped, so IDs can't index the list directly) and returns `entry.hex.zip(entry.coord)`.

### `FactoriesReader` (`crates/engine/src/setting/factories.rs`)

```rust
//! Reading `[[factory]]` sections. Domain types in `crate::factories`. Factories have no
//! names and no duplicate check: any number of identical factories may sit on one hex. A
//! factory refers to a technology and a business by name and to its hex by coordinates;
//! all three are resolved in phase 2, where the loader also checks that the hex is the
//! owner's (through `BusinessesReader::hex_of`) and expands `count` into that many
//! factories, like deposits. Every entry is kept for phase 2, even one with a bad
//! condition or count, so its references are still checked; only entries whose every key
//! is valid and whose references all resolve produce factories.

/// One `[[factory]]` entry, waiting for phase 2.
struct FactoryEntry {
    /// The `[[factory]]` header.
    at: Location,
    /// None if `q` or `r` was missing or invalid (already reported).
    coord: Option<HexCoord>,
    /// None if missing or not a string (already reported).
    technology: Option<Located<String>>,
    /// None if missing or not a string (already reported).
    owner: Option<Located<String>>,
    /// 1 if absent; None if invalid (already reported).
    condition: Option<f64>,
    /// 1 if absent; None if invalid (already reported).
    count: Option<u32>,
}

/// Collects factories during phase 1, and resolves and expands them in phase 2.
#[derive(Default)]
pub(crate) struct FactoriesReader {
    entries: Vec<FactoryEntry>,
    /// Filled in phase 2.
    factories: Vec<Factory>,
}

impl FactoriesReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(
        &mut self,
        technologies: &Definitions<TechnologyKind>,
        businesses: &BusinessesReader,
        map: &MapReader,
        sink: &mut ErrorSink,
    );
    /// Only after a successful load.
    pub fn finish(self, hex_count: usize) -> Factories;
}
```

A private helper `fn condition(reader: &mut EntryReader<'_, '_>) -> Option<f64>`: if `!reader.has("condition")`, `Some(1.0)`. Otherwise `reader.optional_number("condition")?` (a non-number is reported there as `key 'condition' in factory must be a number, found <type>`); then, if the value is finite, greater than 0 and at most 1, `Some(value)`, else report at the value `key 'condition' in factory must be a finite number greater than 0 and at most 1` (label from `reader.label()`) and `None`. This one message covers zero, negative, above 1, infinite and NaN.

`read`, in this order, all before anything is decided:

1. `at = reader.location().clone()`.
2. `q = coordinate(reader, "q")`, `r = coordinate(reader, "r")`.
3. `technology = reader.required_string("technology")`, `owner = reader.required_string("owner")` (not name-checked).
4. `condition = condition(reader)`.
5. `count = count(reader)` (the shared helper from `setting/map.rs`).
6. Push the entry whatever failed, with `coord: q.zip(r).map(|(q, r)| HexCoord::new(q, r))`.

`resolve`, for every entry in order, with a running total and `too_many_reported` flag exactly as `DepositsReader::resolve`:

1. `technology_id = entry.technology.as_ref().and_then(|t| technologies.resolve(t, sink))` (`unknown technology 'loom'` at the value). Always.
2. `owner_id = entry.owner.as_ref().and_then(|o| businesses.defs().resolve(o, sink))` (`unknown business 'nobody'` at the value). Always.
3. `hex = entry.coord.and_then(|coord| map.id(coord))`; if `coord` is `Some` and `hex` is `None`, report `no hex at {coord}` at the header.
4. The owner's hex: only if `owner_id` and `hex` are both `Some`, and `businesses.hex_of(owner_id)` is `Some((owner_hex, owner_coord))`: if `owner_hex != hex`, report at the header `factory at {coord} is not on the hex of its owner '{owner}' at {owner_coord}` (`owner` is the name as written in the entry, `coord` the factory's own) and treat the entry as failed. If `hex_of` is `None` (the owner's own hex didn't resolve; its entry already reported it), there is no check.
5. If `technology_id`, `owner_id`, `hex`, `condition` and `count` are all `Some` and step 4 didn't fail: guard the total (`too many factories in total (the most is 4294967295)` at the header, once), then push `count` copies of `Factory { hex, technology, owner, condition }`.

`finish(hex_count)`: `Factories::from_factories(self.factories, hex_count)`.

### `setting/mod.rs`

- `StartState` gains `pub factories: Factories` (doc: every factory, in load order; see `crate::factories`).
- `WorldContent` gains `pub factories: FactoriesReader`; `section_types` gains `SectionType { header: "factory", folder: Folder::Start, read: |c, r| c.factories.read(r) }` after `business`.
- `resolve`: add last `self.factories.resolve(self.technologies.defs(), &self.businesses, &self.map, sink);` (after `self.businesses.resolve`, which fills the hexes `hex_of` reads). Extend the comment: factories refer to technologies, businesses and hexes, and check the hex against the owner's, so they resolve after the businesses.
- `load_setting`: `let factories = content.factories.finish(map.len());` after `businesses`, and `StartState { map, currencies, deposits, prices, pops, businesses, factories }`.

### `world.rs`

- Field `factories: Factories` (doc: nothing changes them yet; their technology and condition are state because later Steps change them), set from `start.factories` in `new`.
- `pub fn factories(&self) -> &Factories`.
- Module docs: the world holds the factories; nothing runs, wears or maintains them yet; the turn and the report are unchanged. No change to `emit`, `perish`, the trade round, growth or `TurnReport`.

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.start.factories.len(), "factory", "factories")`. The sample line becomes
`Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories`.
Every other expected line in `checker.rs` ends in `, 0 factories`; the new `factories_one` test expects `Loaded setting 'Factories One' (format version 1): 1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 1 technology, 1 business, 1 factory`.

### Setting file format

A `[[factory]]` in `start/` has four required keys and two optional ones: `q` and `r` (integer axial coordinates of a hex in the map, read as a deposit's), `technology` (the name of a technology), `owner` (the name of a business; the factory must sit on that business's hex), `condition` (optional, default 1: the share of the technology's capital bundle the factory holds, a finite number greater than 0 and at most 1), `count` (optional, default 1: the entry places that many identical factories, each with its own ID, a whole number from 1 to 1000000). Any other key is unknown. The technology, the owner and the hex are resolved in phase 2, so the business and the hex may be defined in any file (technologies are rules, so they are always read first); phase 2 is skipped when a file was rejected or discovery was incomplete.

All `[[factory]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `q`, `r`, `technology` or `owner` missing | header | `missing required key 'owner' in factory` |
| `q`, `r` or `count` not an integer | value | `key 'q' in factory must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in factory is out of range` |
| `technology` or `owner` not a string | value | `key 'owner' in factory must be a string, found boolean` |
| `condition` not a number | value | `key 'condition' in factory must be a number, found string` |
| `condition` zero, negative, above 1, `inf` or `nan` | value | `key 'condition' in factory must be a finite number greater than 0 and at most 1` |
| `count` below 1 | value | `key 'count' in factory must be at least 1` |
| `count` above 1000000 (or beyond `i64`) | value | `key 'count' in factory is out of range (the most is 1000000)` |
| an unknown technology | value | `unknown technology 'loom'` |
| an unknown business | value | `unknown business 'nobody'` |
| coordinates that match no hex | header | `no hex at (9, 9)` |
| a hex that isn't the owner's | header | `factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)` |
| more than `u32::MAX` factories in total | header | `too many factories in total (the most is 4294967295)` |
| any other key | key | `unknown key 'colour' in factory` |
| `[[factory]]` in `rules/` | header | `section 'factory' belongs in start/, not rules/` |

### Test setting contents

Write these files exactly; line numbers and columns matter for the expected errors. Every test setting has a `setting.toml` with `format_version = 1`. Businesses in these settings have `inventory = {}`, `deposits = {}` and `decider = "rule"` unless said otherwise.

**`factories_valid/`** — `setting.toml`: name `Factories Test`. `rules/goods.toml`: goods `coin`, `tools`, `grain` (IDs 0-2, names only). `rules/technologies.toml`: `mill` (`inputs = {}`, `output = "grain"`, `capacity = 1`, `capital = { tools = 1 }`, `construction = {}`, `construction_turns = 1`, `lifetime = 1`) and `forge` (`inputs = { grain = 1 }`, `output = "tools"`, `capacity = 2`, `capital = { tools = 2 }`, `construction = {}`, 1, 1); IDs 0, 1. `start/a.toml`:
```toml
[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm_a"

[[factory]]
q = 0
r = 0
technology = "forge"
owner = "farm_a"
condition = 0.5
count = 2

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm_b"
condition = 1
```
`start/b/c.toml`:
```toml
[[factory]]
q = 1
r = 0
technology = "forge"
owner = "idle"
condition = 0.25
```
`start/z.toml` (read after every factory file, so the hexes and businesses are forward references): hexes (0, 0) and (1, 0), currency `coin`; businesses `farm_a` at (0, 0), `farm_b` at (0, 0), `idle` at (1, 0) (IDs 0-2).

Expected factories as (index, coordinates, technology, owner, condition): 0 (0, 0) `mill` `farm_a` 1; 1 (0, 0) `forge` `farm_a` 0.5; 2 (0, 0) `forge` `farm_a` 0.5; 3 (0, 0) `mill` `farm_b` 1; 4 (1, 0) `forge` `idle` 0.25. `on_hex((0, 0))` is `[0, 1, 2, 3]`, `on_hex((1, 0))` is `[4]`. `owned_by`: `farm_a` `[0, 1, 2]`, `farm_b` `[3]`, `idle` `[4]`.

**`factories_one/`** — `setting.toml`: name `Factories One`. `rules/goods.toml`: good `coin`. `rules/technologies.toml`: `oven` (`inputs = {}`, `output = "coin"`, `capacity = 1`, `capital = { coin = 1 }`, `construction = {}`, 1, 1). `start/map.toml`: one hex (0, 0), currency `coin`. `start/businesses.toml`: business `solo` at (0, 0). `start/factories.toml`: one `[[factory]]` at (0, 0), `technology = "oven"`, `owner = "solo"`.

**`factories_errors/`** — `setting.toml`: name `Factories Errors`. `rules/goods.toml`: goods `coin`, `grain`, `tools`. `rules/t.toml`: technology `mill` (`inputs = {}`, `output = "grain"`, `capacity = 1`, `capital = { tools = 1 }`, `construction = {}`, 1, 1). `rules/x.toml`:
```toml
[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
```
`start/h.toml`: hexes (0, 0) and (1, 0), currency `coin`. `start/b.toml` (15 lines; `lost` has no hex, so a factory it owns gets no owner's-hex check):
```toml
[[business]]
name = "farm"
q = 0
r = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "lost"
q = 9
r = 9
inventory = {}
deposits = {}
decider = "rule"
```
`start/a.toml` (136 lines):
```toml
[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"

[[factory]]
r = 0
technology = "mill"
owner = "farm"

[[factory]]
q = 0
technology = "mill"
owner = "farm"

[[factory]]
q = 0
r = 0
owner = "farm"

[[factory]]
q = 0
r = 0
technology = "mill"

[[factory]]
q = 1.5
r = "0"
technology = 3
owner = true
condition = "full"
count = 2.5

[[factory]]
q = 2147483648
r = -2147483649
technology = "mill"
owner = "farm"

[[factory]]
q = 9
r = 9
technology = "mill"
owner = "farm"

[[factory]]
q = 0
r = 0
technology = "loom"
owner = "nobody"

[[factory]]
q = 1
r = 0
technology = "mill"
owner = "farm"

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
condition = 0

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
condition = -0.5

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
condition = 1.5

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
condition = inf

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
condition = nan

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
count = 0

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
count = -1

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
count = 1000001

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "farm"
colour = "red"
size = 3

[[factory]]
q = 0
r = 0
technology = "mill"
owner = "lost"

[[factory]]
q = 1
r = 0
technology = "loom"
owner = "farm"
condition = 2
count = 0
colour = "red"
```
Expected error lines, in this order (`{condition}` is `key 'condition' in factory must be a finite number greater than 0 and at most 1`):
```
rules/x.toml:1:3: section 'factory' belongs in start/, not rules/
start/a.toml:7:1: missing required key 'q' in factory
start/a.toml:12:1: missing required key 'r' in factory
start/a.toml:17:1: missing required key 'technology' in factory
start/a.toml:22:1: missing required key 'owner' in factory
start/a.toml:28:5: key 'q' in factory must be an integer, found float
start/a.toml:29:5: key 'r' in factory must be an integer, found string
start/a.toml:30:14: key 'technology' in factory must be a string, found integer
start/a.toml:31:9: key 'owner' in factory must be a string, found boolean
start/a.toml:32:13: key 'condition' in factory must be a number, found string
start/a.toml:33:9: key 'count' in factory must be an integer, found float
start/a.toml:36:5: key 'q' in factory is out of range
start/a.toml:37:5: key 'r' in factory is out of range
start/a.toml:41:1: no hex at (9, 9)
start/a.toml:50:14: unknown technology 'loom'
start/a.toml:51:9: unknown business 'nobody'
start/a.toml:53:1: factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)
start/a.toml:64:13: {condition}
start/a.toml:71:13: {condition}
start/a.toml:78:13: {condition}
start/a.toml:85:13: {condition}
start/a.toml:92:13: {condition}
start/a.toml:99:9: key 'count' in factory must be at least 1
start/a.toml:106:9: key 'count' in factory must be at least 1
start/a.toml:113:9: key 'count' in factory is out of range (the most is 1000000)
start/a.toml:120:1: unknown key 'colour' in factory
start/a.toml:121:1: unknown key 'size' in factory
start/a.toml:129:1: factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)
start/a.toml:132:14: unknown technology 'loom'
start/a.toml:134:13: {condition}
start/a.toml:135:9: key 'count' in factory must be at least 1
start/a.toml:136:1: unknown key 'colour' in factory
start/b.toml:9:1: no hex at (9, 9)
```
Notes: the entry at line 41 has an owner but no hex, and the entry at line 123 has a hex but an owner (`lost`) whose own hex didn't resolve, so neither gets an owner's-hex error. The last entry (line 129) covers "one entry with several problems": its bad condition and count don't stop the technology and owner's-hex checks. If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader` and `Definitions` message forms are fixed. The columns come from the TOML spans (column of the key's or value's first character); if `toml` reports a span differently than assumed, adjust the columns only.

**`resources_broken_file/start/s.toml`**: append
```toml

[[factory]]
q = 9
r = 9
technology = "nothing"
owner = "nobody"
```
The expected errors stay exactly the one syntax error.

### Sample setting

**`settings/sample/start/factories.toml`**, with a header comment in the style of `businesses.toml` that explains every key (the six keys and their rules, as in "Setting file format" above), says what the condition means (the share of the technology's capital bundle the factory holds; capital only ever changes in the bundle's proportions, so one number is the whole state; 1, the default, is a factory in full condition; a factory can't be placed at 0), that a factory must sit on its owner's hex until trade between hexes exists, that factories have no names and are addressed by ID in load order, that a hex and a business may hold any number of factories, that nothing checks a factory makes economic sense, and that nothing runs, wears or maintains factories yet (Steps 04 and 08). Explain the numbers: one factory per business, each running the technology its business is named for; the mine's factory starts worn (condition 0.75, so three quarters of its two tools are left, six turns of its eight-turn lifetime), the others full; the deposits on other hexes have no factory. Then:
```toml
# The farm business runs a farm on the center hex.
[[factory]]
q = 0
r = 0
technology = "farm"
owner = "center_farm"

# The mine business runs a worn mine: three quarters of its capital is left.
[[factory]]
q = 0
r = 0
technology = "mine"
owner = "center_mine"
condition = 0.75

# The toolmaker business runs a toolmaker.
[[factory]]
q = 0
r = 0
technology = "toolmaker"
owner = "center_toolmaker"
```
(Comment wording is free; the values are exact.) Expected factories as (index, coordinates, technology, owner, condition): 0 (0, 0) `farm` `center_farm` 1; 1 (0, 0) `mine` `center_mine` 0.75; 2 (0, 0) `toolmaker` `center_toolmaker` 1. `on_hex(center)` is `[0, 1, 2]`; `owned_by` gives `[0]`, `[1]`, `[2]` for the three businesses in order. The sample's turn lines, stockpiles and reports don't change.

## Tasks

1. `setting/map.rs`: `count` and `MAX_COUNT`; `setting/resources.rs`: use them, drop `deposit_count` and `MAX_DEPOSIT_COUNT`, keep the tests (calling `count` with label `deposit`); docs. Run the loader tests to confirm nothing changed.
2. `factories.rs`: domain types, `Factories::from_factories`, accessors, unit tests. Export from `lib.rs`; crate docs.
3. `setting/technologies.rs`: `defs()`. `setting/businesses.rs`: `defs()`, `hex_of()`, docs.
4. `setting/factories.rs`: `FactoriesReader`, `condition` with its unit tests. `setting/mod.rs`: wiring and docs.
5. `world.rs`: field, `new`, `factories()`, docs.
6. `check.rs` success line; `checker.rs` expectations and the `factories_one` test.
7. Test settings (`factories_valid`, `factories_one`, `factories_errors`, `resources_broken_file`), `tests/settings/README.md`; the sample's `start/factories.toml`, `start/README.md`, and the paragraph in `rules/technologies.toml`.
8. `crates/engine/tests/factories.rs` (Test Plan); the comment in `tests/resources.rs`.
9. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the new success line and unchanged turn lines.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). Every condition in these tests is exactly representable, so compare with `assert_eq!`.

**Unit tests.**
- `factories.rs`: `from_factories` gives sequential IDs in the order given; `get`; `iter` in order; `on_hex` in ID order and empty for a hex with none; `owned_by` lists only that owner's factories in ID order and nothing for a business that owns none; the default list is empty; `#[should_panic]`: `get` on a foreign ID (`is not from this list`), `on_hex` on a hex beyond the map and on the default list (`is not from this list`), `from_factories` with a hex beyond `hex_count` (`is not from a map of 2 hexes`), with a condition of 1.5, -0.1 and NaN (`a factory's condition must be a finite number from 0 to 1`); a condition of exactly 0 and exactly 1 is accepted. Build IDs with throwaway `NameRegistry`s as `resources.rs`'s tests do.
- `setting/resources.rs`: the existing `deposit_count` tests pass unchanged against `count` with the label `deposit`.
- `setting/factories.rs` (a `run` helper like `setting/resources.rs`'s, label `factory`): `condition` on an empty entry gives `Some(1.0)` and no error; `condition = 0.5` gives 0.5, `condition = 1` gives 1.0, `condition = 0.0001` gives it; `condition = 0`, `-0.5`, `1.5`, `inf`, `nan` each give None with `start/t.toml:1:13: key 'condition' in factory must be a finite number greater than 0 and at most 1`; `condition = "full"` gives None with `start/t.toml:1:13: key 'condition' in factory must be a number, found string`.

**`crates/engine/tests/factories.rs`** (`mod common;`). Helpers: `factories_of(&LoadedSetting)` returning each factory as (index, `HexCoord`, technology name, owner name, condition); `factory_rows(&World)` the same from `world.factories()` (with `world.rules().technologies.name(..)` and `world.businesses().name(..)`); `on_hex(&LoadedSetting, q, r) -> Vec<usize>`; `owned(&LoadedSetting, business index) -> Vec<usize>` via `owned_by` (find the business ID as `tests/businesses.rs`'s `owned` does).
- **AC "the sample loads with three factories; a fresh world holds them unchanged; determinism"**: load the sample; `factories.len() == 3`; `factories_of` equals the list above; `on_hex(0, 0)` is `[0, 1, 2]`, `on_hex(2, -2)` is empty; `owned(0)` is `[0]`, `owned(1)` is `[1]`, `owned(2)` is `[2]`. Build the world: `factory_rows` equals the same list; step 3 times and check it after every step. Two sample worlds stepped 3 times give equal reports and equal `factory_rows` after every step.
- **AC "factories across files, count, default and written condition, two businesses on one hex, forward references"**: load `factories_valid`; `factories_of`, `on_hex` and `owned` as listed; loading twice gives equal `factories_of`. `factories[1]` and `factories[2]` (the count-2 entry) are equal records with consecutive IDs.
- **AC "each error case, one entry with several problems"**: `failures_of("factories_errors")` equals the expected list.
- **AC "reference checks skipped after a broken file"**: the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing (update the comment).
- **AC "a setting with no factories is valid"**: `valid_minimal` loads with `factories.is_empty()`; `businesses_valid` loads with no factories and its world has `factories().len() == 0`.
- `the_one_factory_setting_loads`: `factories_one` gives one factory at (0, 0), `oven`, `solo`, condition 1.
- `Factories::get` panics (`is not from this list`) on a factory from another world's list (pattern of `business_stockpile_panics_on_a_business_from_another_world`, using `world.factories().get(foreign)`).
- **AC "every existing test passes, with expectations changed only by the factory count"**: `checker.rs` updated as listed; everything else unchanged.

**Dev UI**: `checker.rs` passes with the updated lines, including `1 factory` for `factories_one`, `3 factories` for the sample and `0 factories` elsewhere; the sample's turn lines are unchanged.

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`.
- `Factories::from_factories`, `count` and `MAX_COUNT` stay `pub(crate)`. `Factories`' fields stay private. `Factory`'s fields are public. `IdVec::push` stays `#[cfg(test)]`.
- No name key, no duplicate check and no economic check for factories. Nothing changes a factory outside the loader.
- Do not change `TurnReport`, the turn, the turn lines, `Market`, `HexPrices`, the trade round, growth, POP or business code (beyond the two `BusinessesReader` accessors), `Deposits`, the dev UI window or panels, or the checker's output beyond the factory count. Do not change existing `EntryReader` message texts, and keep every `[[deposit]]` message byte-for-byte the same.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: running, depreciating, maintaining or shutting down a factory and what an exhausted factory is (Step 04); the business view and decisions (Step 05); factories in the turn or the report (Step 08); showing factories in the dev UI (Step 09); construction, scrapping, retooling, buying and selling factories (Stage 07); factories without an owner or on another hex than their owner's; bundle arithmetic (the goods a factory holds are derived later).

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line and unchanged turn lines, and every Acceptance Criterion in `design.md` is met.
