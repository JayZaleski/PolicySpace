# Step 05: natural-resources — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-05-natural-resources/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-03-goods-and-properties/spec.md` and `docs_changes.md`: the goods pattern (a domain module separate from the loader, a crate-private reader using `Definitions`, a crate-private constructor, test layout).
- `PROJECT_PLAN/stage-01-create-the-world/step-04-hex-map/spec.md` and `docs_changes.md`: the hex pattern (a kind identified by something other than a name, which checks its own duplicates; `MapReader`'s `coordinate` helper).
- `PROJECT_PLAN/stage-01-create-the-world/step-02-setting-loader/docs_changes.md`: the loader framework (`SectionType`, `SettingContent`, `WorldContent`, `EntryReader`, `Definitions`, `ErrorSink`, phase 1 and phase 2, error order).
- Code this Step extends:
  - `crates/engine/src/lib.rs`, `crates/engine/src/id.rs` (read only), `crates/engine/src/goods.rs`, `crates/engine/src/map.rs` (read only)
  - `crates/engine/src/setting/mod.rs`, `reader.rs`, `defs.rs`, `goods.rs`, `map.rs`, `error.rs`, `test_content.rs`
  - `crates/engine/tests/goods.rs`, `crates/engine/tests/map.rs`, `crates/engine/tests/setting_loader.rs`, `crates/engine/tests/settings/` (and its `README.md`)
  - `crates/dev-ui/src/main.rs`, `crates/dev-ui/tests/checker.rs`
  - `settings/sample/`
- `Cargo.toml` (workspace): `missing_docs = "warn"`, clippy `all`, and `tools/check.py` runs clippy with `-D warnings`. Every public item and public field needs a doc comment.

## Approach
- **A new `quantity` module with a `Quantity` type.** It's the engine's first continuous quantity. It wraps an `f64` and guarantees a finite, non-negative value. Every system uses this one type for amounts of goods (Step 06's stockpiles next), so the representation can be swapped later (for example, to fixed decimal) in one place. It has no arithmetic yet; Step 06 adds what it needs.
- **One domain module, `resources`, for resource types and deposits**, separate from the loader as `goods` and `map` are. It depends on `goods` (`GoodId`), `map` (`HexId`), `quantity`, and `id`, and knows nothing about files.
- **Resource types are a named kind** (like goods): a `NameRegistry` plus per-ID data, built through `Definitions` in the loader, so duplicates and unknown-name references use the shared machinery.
- **Deposits are stored one by one.** An entry with `count = 3` becomes three `Deposit`s with consecutive IDs, each holding its resource type and hex. This gives each deposit its own ID now, and its own owner in Stage 02. `count` is capped at 1,000,000 per entry, so a typo can't try to allocate billions of deposits.
- **Deposits on a hex: a per-hex list**, indexed by `HexId::index`, built when the `Deposits` is created. Hex IDs are dense, so a `Vec` works and there's no hash-map iteration.
- **The loader gets `setting/resources.rs`** with two readers, `ResourcesReader` (`[[resource]]`, in `rules/`) and `DepositsReader` (`[[deposit]]`, in `start/`).
- **References are resolved in phase 2** (`WorldContent::resolve`): `produces` against the goods' `Definitions`, a deposit's `resource` against the resource types' `Definitions`, and a deposit's coordinates against the `MapReader`. This is the first real use of phase 2.
- **Deposit duplicates are checked in phase 1** by (resource name as written, coordinates), only when `resource`, `q`, and `r` are all valid. Like hexes, deposits have no name, so their reader keeps its own table.
- **`EntryReader` gains `required_number`** (an integer or a float, returned as `f64`) and a `label()` accessor. The hex `coordinate` helper is shared with deposits, using the reader's label in its message.
- **One build, no packages.** The domain module, the readers, and the wiring all depend on each other.

## Changes
Create:
- `crates/engine/src/quantity.rs`: `Quantity`, unit tests.
- `crates/engine/src/resources.rs`: `ResourceKind`, `ResourceId`, `ResourceType`, `ResourceTypes`, `DepositKind`, `DepositId`, `Deposit`, `Deposits`, unit tests.
- `crates/engine/src/setting/resources.rs`: crate-private `ResourcesReader`, `DepositsReader`, unit tests for the count rule.
- `crates/engine/tests/resources.rs`: integration tests through the public API.
- `settings/sample/rules/resources.toml`, `settings/sample/start/deposits.toml`.
- Test settings `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start` under `crates/engine/tests/settings/` (see Test Plan).

Modify:
- `crates/engine/src/lib.rs`: `pub mod quantity; pub mod resources;`, re-exports, crate-docs sentences.
- `crates/engine/src/setting/mod.rs`: `mod resources;`; `Rules` gains `resources`; `StartState` gains `deposits`; `WorldContent` gains both readers, two section types, and a real `resolve`; `load_setting` builds them; module docs.
- `crates/engine/src/setting/reader.rs`: `required_number`, `label`, unit tests.
- `crates/engine/src/setting/map.rs`: `coordinate` becomes `pub(crate)` and uses `reader.label()`; `MapReader::id`.
- `crates/engine/src/setting/goods.rs`, `defs.rs`: remove `#[allow(dead_code)]` (and its comment) from `GoodsReader::defs` and `Definitions::resolve`, now used. Leave the rest to clippy: an item still dead in non-test builds keeps its allow with an updated comment.
- `crates/engine/tests/setting_loader.rs`: `sample_setting_loads` also checks the resource and deposit counts.
- `crates/engine/tests/settings/README.md`: describe the new folders.
- `crates/dev-ui/src/main.rs`: resource and deposit counts on the success line; crate docs.
- `crates/dev-ui/tests/checker.rs`: updated expected lines, and a one-resource test.
- `settings/sample/rules/README.md`, `settings/sample/start/README.md`: one line each.

## Interfaces & Data

### `crates/engine/src/quantity.rs` (public)
Module docs: a quantity is an amount of a good (or, here, a deposit's output per period). Quantities are continuous. They're stored as `f64` for now, behind this type, so every system uses one representation that can be swapped later (for example, to fixed decimal for cross-platform determinism) in one place. A `Quantity` is always finite and never negative. Arithmetic is added when a system needs it.

```rust
/// An amount of a good: a finite, non-negative number.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Quantity(f64);

impl Quantity {
    /// Zero.
    pub const ZERO: Quantity = Quantity(0.0);
    /// The quantity `value`, or None if it is negative, NaN, or infinite.
    /// `-0.0` becomes `0.0`.
    pub fn new(value: f64) -> Option<Quantity>;
    /// The value as an `f64`.
    pub fn get(self) -> f64;
}

/// Displays the number as `f64` does: `4`, `2.5`, `0`.
impl fmt::Display for Quantity { ... }
```
- `Default` is `ZERO`. Don't derive `Eq`, `Ord`, or `Hash`.
- Normalize `-0.0` so that `Quantity::new(-0.0).unwrap().to_string() == "0"`.

### `crates/engine/src/resources.rs` (public)
Module docs must say:
- A **resource type** (`[[resource]]` in `rules/`) is a kind of natural resource: the one good it produces, and how much one deposit produces per period. Several resource types may produce the same good. The engine doesn't require that good to be unmovable or to perish, though settings usually make it so.
- A **deposit** (`[[deposit]]` in `start/`) is one unit of a resource type on a hex. A setting entry can place several identical deposits on a hex (`count`); each is its own deposit with its own ID, and later its own single owner. A hex may hold deposits of several types.
- Deposits have no names. They are identified in setting files by hex and resource type; `DepositId`s are valid only within the `Deposits` that produced them.
- Nothing is produced yet (Step 06), there is no ownership (Stage 02), and there is no stock, depletion, or regeneration (later).

```rust
/// Marker kind for resource types. LABEL = "resource".
pub enum ResourceKind {}
/// The ID of a resource type.
pub type ResourceId = Id<ResourceKind>;

/// One resource type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceType {
    /// The good one deposit produces.
    pub produces: GoodId,
    /// How much of it one deposit produces per period. Always greater than 0.
    pub output: Quantity,
}

/// Every resource type in a setting, in load order (which is also ID order).
#[derive(Debug, Default)]
pub struct ResourceTypes { names: NameRegistry<ResourceKind>, types: Vec<ResourceType> }

impl ResourceTypes {
    /// Crate-private: only the setting loader builds this. Panics if the lengths differ.
    pub(crate) fn from_parts(names: NameRegistry<ResourceKind>, types: Vec<ResourceType>) -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// The ID of the resource type named `name`, or IdError::UnknownName.
    pub fn id(&self, name: &str) -> Result<ResourceId, IdError>;
    /// The name of `id`. Panics if `id` isn't from this list.
    pub fn name(&self, id: ResourceId) -> &str;
    /// The resource type `id`. Panics if `id` isn't from this list.
    pub fn get(&self, id: ResourceId) -> &ResourceType;
    /// Every resource type as (ID, name, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (ResourceId, &str, &ResourceType)> + '_;
}

/// Marker kind for deposits. LABEL = "deposit".
pub enum DepositKind {}
/// The ID of a deposit.
pub type DepositId = Id<DepositKind>;

/// One deposit: one unit of a resource type on a hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Deposit {
    /// Its resource type.
    pub resource: ResourceId,
    /// The hex it sits on.
    pub hex: HexId,
}

/// Every deposit in a setting, in load order (which is also ID order).
#[derive(Debug, Default)]
pub struct Deposits {
    /// Indexed by `DepositId::index`.
    deposits: Vec<Deposit>,
    /// Indexed by `HexId::index`: the deposits on each hex, in ID order.
    by_hex: Vec<Vec<DepositId>>,
}

impl Deposits {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map the deposits sit on. Crate-private.
    /// # Panics
    /// If a deposit's hex index is `>= hex_count`, or there are more than u32::MAX deposits.
    pub(crate) fn from_deposits(deposits: Vec<Deposit>, hex_count: usize) -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// The deposit `id`. Panics if `id` isn't from this list.
    pub fn get(&self, id: DepositId) -> &Deposit;
    /// Every deposit as (ID, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (DepositId, &Deposit)> + '_;
    /// The deposits on `hex`, in ID order; empty if it has none.
    /// Panics if `hex` isn't from the map these deposits were built with.
    pub fn on_hex(&self, hex: HexId) -> &[DepositId];
    /// The deposits of `resource` on `hex`, in ID order. Panics as `on_hex` does.
    pub fn of_type_on_hex(&self, hex: HexId, resource: ResourceId) -> impl Iterator<Item = DepositId> + '_;
}
```
- `ResourceTypes::get`/`name` and `Deposits::get` panic with a message containing `is not from this list` (mirror `Goods::get`). `on_hex` panics with `is not from this map`.
- `Deposits::default()` has an empty `by_hex`, so `on_hex` on it panics for any hex. That's correct: no map was given. Document it.
- `iter` walks the vectors, never a hash map.

### `crates/engine/src/lib.rs`
```rust
pub mod quantity;
pub use quantity::Quantity;
pub mod resources;
pub use resources::{Deposit, DepositId, DepositKind, Deposits, ResourceId, ResourceKind, ResourceType, ResourceTypes};
```
Crate docs: add that natural resources (resource types in the rules, deposits on hexes in the starting state) are in the `resources` module, and amounts are `Quantity` values (`quantity` module). Keep the modules in alphabetical order with the existing ones.

### `Rules`, `StartState`, `load_setting` (`setting/mod.rs`)
```rust
pub struct Rules {
    pub goods: Goods,
    /// Every resource type the setting defines, in load order.
    pub resources: ResourceTypes,
}
pub struct StartState {
    pub map: HexMap,
    /// Every deposit, in load order.
    pub deposits: Deposits,
}
```
Change "Step 05 onward add more fields" to "Later Steps add more fields" on both. `load_setting` builds the map first, then `deposits: content.deposits.finish(map.len())`.

### `EntryReader` additions (`setting/reader.rs`)
```rust
/// The label used in messages ("good", "hex", "manifest", ...).
pub fn label(&self) -> &'static str;
/// A number that must be present: an integer or a float, as f64.
pub fn required_number(&mut self, key: &str) -> Option<Located<f64>>;
```
`required_number`:
- integer → parse as `integer` does (`i64::from_str_radix`), then `as f64`. Beyond `i64`, it's the existing `key 'k' in <label> is out of range`.
- float → `float.as_str().parse::<f64>()`. The string has no underscores (the TOML decoder removes them; add a unit test with `1_000.5` to confirm). Rust's parser accepts `inf`, `+inf`, `-inf`, `nan` (with a sign too); return those values as they are. Range checks belong to the caller. If parsing fails anyway, report `key 'k' in <label> is out of range`.
- other types → `wrong_type(key, "a number", value)`, giving `key 'k' in <label> must be a number, found string`.
- missing → the usual `missing required key`.

Add only `required_number`; no optional variant until something needs one.

### Shared coordinate helper (`setting/map.rs`)
- `coordinate` becomes `pub(crate) fn coordinate(reader, key) -> Option<i32>`, and its message uses `reader.label()`: `key 'q' in hex is out of range` stays the same for hexes, and deposits get `key 'q' in deposit is out of range`.
- `MapReader` gains `pub fn id(&self, coord: HexCoord) -> Option<HexId>`. It returns `HexId::from_index` of the stored index, which `finish` gives the same ID.

### Reading resource types (`setting/resources.rs`, crate-private)
```rust
#[derive(Default)]
pub(crate) struct ResourcesReader {
    defs: Definitions<ResourceKind>,
    /// Indexed by ResourceId::index; pushed exactly when defs.define returns an ID.
    produces: Vec<Option<Located<String>>>,   // None if missing or not a string
    outputs: Vec<Quantity>,                   // Quantity::ZERO placeholder if invalid
    /// Filled in phase 2, indexed like `produces`.
    produces_ids: Vec<Option<GoodId>>,
}
impl ResourcesReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    /// Phase 2: resolves each `produces` against the goods.
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink);
    pub fn defs(&self) -> &Definitions<ResourceKind>;
    /// Only after a successful load: every produces is resolved.
    pub fn finish(self) -> ResourceTypes;
}
```
`read`, reading every key before defining anything:
1. `required_name("name")`.
2. `required_string("produces")`. Don't name-check it: a value like `"Gold"` is simply an unknown good in phase 2.
3. `required_number("output")`. If the value isn't finite or isn't greater than 0, report at the value: `key 'output' in resource must be a finite number greater than 0`. Otherwise `Quantity::new(v)`.
4. If the name is valid and `defs.define` returns an ID, push `produces` and the output (or the placeholder). As with goods, the name is defined even when another key is bad, so a later duplicate is still caught and a deposit referring to it doesn't get a misleading "unknown resource" error.

`resolve`: for each entry, in order, `Some(p)` → `goods.resolve(p, sink)` (message `unknown good 'gold'`, at the `produces` value). `None` → `None`.

`finish`: builds `ResourceTypes::from_parts(defs.into_registry(), types)`, where each `produces_ids` entry is `expect`ed. This is an internal invariant (phase 2 ran, and the load succeeded), not setting-controlled. Use a message that says so.

### Reading deposits (`setting/resources.rs`, crate-private)
```rust
/// The most deposits one [[deposit]] entry may place.
pub(crate) const MAX_DEPOSIT_COUNT: u32 = 1_000_000;

struct DepositEntry {
    at: Location,                 // the [[deposit]] header
    resource: Located<String>,
    coord: HexCoord,
    count: u32,
}
#[derive(Default)]
pub(crate) struct DepositsReader {
    entries: Vec<DepositEntry>,
    /// (resource name as written, coordinates) → header of the first entry. Lookup only.
    seen: HashMap<(String, HexCoord), Location>,
    /// Filled in phase 2.
    deposits: Vec<Deposit>,
}
impl DepositsReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    /// Phase 2: resolves each entry's resource type and hex, and expands `count`.
    pub fn resolve(&mut self, resources: &Definitions<ResourceKind>, map: &MapReader, sink: &mut ErrorSink);
    /// Only after a successful load.
    pub fn finish(self, hex_count: usize) -> Deposits;
}
```
`read`, reading every key before anything else:
1. `let at = reader.location().clone();`
2. `required_string("resource")`.
3. `coordinate(reader, "q")`, `coordinate(reader, "r")`.
4. `optional_integer("count")`: absent → 1. `< 1` → at the value, `key 'count' in deposit must be at least 1`. `> MAX_DEPOSIT_COUNT` → at the value, `key 'count' in deposit is out of range (the most is 1000000)`. Keep the count check in a small function (`fn deposit_count(reader) -> Option<u32>`, `None` meaning invalid and already reported) so it can be unit tested.
5. If `resource`, `q`, or `r` is missing or invalid, stop: no duplicate check, nothing stored.
6. If `seen` has `(resource name, coord)`, report at `at`: `duplicate deposit of 'copper_ore' at (1, -1) (first defined at start/a.toml:1:1)`, and stop.
7. Otherwise record it in `seen`, and push the entry. If the count was invalid, store `count = 1`, so phase 2 still checks its references (the load fails anyway).

`resolve`, entries in order:
- `resources.resolve(&entry.resource, sink)` (message `unknown resource 'gold_deposit'`, at the `resource` value);
- `map.id(entry.coord)`; if `None`, report at `entry.at`: `no hex at (9, 9)`;
- if both resolved, push `count` copies of `Deposit { resource, hex }`;
- keep a running total. If adding an entry would take it past `u32::MAX`, report at `entry.at`: `too many deposits in total (the most is 4294967295)`, and stop expanding. In practice the per-entry cap makes this unreachable short of thousands of huge entries, but it keeps `from_deposits` from panicking on setting data.

`finish`: `Deposits::from_deposits(self.deposits, hex_count)`.

### Messages added to the catalogue (exact; tests assert them)
| Situation | Message (after the location) |
|---|---|
| a number key of the wrong type | `key 'output' in resource must be a number, found string` |
| `output` ≤ 0, NaN, or infinite | `key 'output' in resource must be a finite number greater than 0` |
| `count` < 1 | `key 'count' in deposit must be at least 1` |
| `count` > 1,000,000 | `key 'count' in deposit is out of range (the most is 1000000)` |
| deposit coordinate outside `i32` | `key 'q' in deposit is out of range` (or `'r'`) |
| second entry, same type and hex | `duplicate deposit of 'copper_ore' at (1, -1) (first defined at start/a.toml:1:1)` (at the second header) |
| no hex at the coordinates | `no hex at (9, 9)` (at the header) |
| total deposits beyond `u32::MAX` | `too many deposits in total (the most is 4294967295)` (at the header) |

Existing messages cover the rest: `missing required key 'k' in resource|deposit`, `invalid name '…' (…)`, `duplicate resource name 'x' (first defined at …)`, `unknown good 'x'`, `unknown resource 'x'`, `key 'produces' in resource must be a string, found integer`, `key 'q' in deposit must be an integer, found float`, `key 'count' in deposit must be an integer, found float`, `unknown key 'k' in resource|deposit`, `section 'resource' belongs in rules/, not start/`, and `section 'deposit' belongs in start/, not rules/`.

### `WorldContent` (`setting/mod.rs`)
```rust
pub(crate) struct WorldContent {
    pub goods: GoodsReader,
    pub map: MapReader,
    pub resources: ResourcesReader,
    pub deposits: DepositsReader,
}
// section_types(): good (Rules), hex (Start), resource (Rules), deposit (Start)
fn resolve(&mut self, sink: &mut ErrorSink) {
    self.resources.resolve(self.goods.defs(), sink);
    self.deposits.resolve(self.resources.defs(), &self.map, sink);
}
```
(Disjoint field borrows; no cloning needed.)

Module docs: update "Adding a section type" step 3 to point at the real example (resources and deposits resolve references in `WorldContent::resolve`: a name through `Definitions::resolve`, a hex through `MapReader::id`). Mention that a `finish` that depends on phase 2 is only called after a successful load.

### Dev UI (`crates/dev-ui/src/main.rs`)
- The success line becomes `Loaded setting '{name}' (format version {v}): {goods}, {hexes}, {resources}, {deposits}`, with `count(n, "resource", "resources")` and `count(n, "deposit", "deposits")`.
- Crate docs: the counts are now goods, hexes, resources, and deposits.

### Sample setting
`settings/sample/rules/resources.toml`: a comment header explaining resource types (`name`, `produces`, `output` per deposit per period; the produced good is usually unmovable and perishes after one period, by convention, not by rule). Then, in order:

| `name` | `produces` | `output` |
|---|---|---|
| `copper_ore` | `unimproved_copper` | `2.5` |
| `arable_land` | `unimproved_grain` | `4` |

`settings/sample/start/deposits.toml`: a comment header explaining deposits (`resource`, `q`, `r`, optional `count`, default 1; each deposit is a separate unit for a future single owner; one entry per type per hex). Then, in order:

| `resource` | `q` | `r` | `count` |
|---|---|---|---|
| `arable_land` | 0 | 0 | 3 |
| `copper_ore` | 0 | 0 | (omitted) |
| `arable_land` | -1 | 0 | 2 |
| `arable_land` | 1 | 0 | (omitted) |
| `copper_ore` | 1 | -1 | 2 |
| `copper_ore` | -2 | 1 | (omitted) |

That's 10 deposits, with expected IDs 0–2 `arable_land`@(0,0), 3 `copper_ore`@(0,0), 4–5 `arable_land`@(−1,0), 6 `arable_land`@(1,0), 7–8 `copper_ore`@(1,−1), and 9 `copper_ore`@(−2,1). Add one line to each sample README saying where resource types and deposits are defined.

## Tasks
1. Create `quantity.rs` with module docs, `Quantity`, and unit tests. Add it to `lib.rs`.
2. Create `resources.rs` with module docs, the types, and unit tests. Add it to `lib.rs` with the re-exports and crate docs.
3. `reader.rs`: add `label` and `required_number`, with unit tests.
4. `setting/map.rs`: make `coordinate` shared and label-aware; add `MapReader::id`.
5. Create `setting/resources.rs` with `ResourcesReader`, `DepositsReader`, `MAX_DEPOSIT_COUNT`, and unit tests for `deposit_count`.
6. Wire into `setting/mod.rs`: `Rules.resources`, `StartState.deposits`, `WorldContent` (fields, section types, `resolve`), `load_setting`, docs.
7. Remove the `#[allow(dead_code)]` markers that are no longer needed (`GoodsReader::defs`, `Definitions::resolve`, and any others clippy shows).
8. Create the test settings and update `crates/engine/tests/settings/README.md`.
9. Write `crates/engine/tests/resources.rs`, and extend `sample_setting_loads`.
10. Create the sample files and update the sample READMEs.
11. Dev UI: the counts, and the updated and new checker tests.
12. Run `python tools/check.py` and fix everything until it passes. Run `cargo run -p policyspace-dev-ui -- settings/sample`, `… -- crates/engine/tests/settings/resources_errors`, and `… -- crates/engine/tests/settings/deposits_errors`, and check the output by eye.

## Test Plan

### New test settings (`crates/engine/tests/settings/`)
Every folder has a valid `setting.toml` (format version 1) unless stated. Put each `[[section]]` header on the line stated, or fill in the exact positions from the files when writing assertions.

| Folder | Contents |
|---|---|
| `resources_valid` | `rules/a.toml`: resource `copper_ore` (produces `ore`, output `2.5`). `rules/a/z.toml`: `iron_ore` (produces `ore`, output `3`). `rules/b.toml`: `farmland` (produces `grain`, output `4`), then `unused` (produces `grain`, output `0.25`). `rules/z.toml`: goods `ore` (`perishes_after = 1`, `movable = false`) and `grain`, defined **after** the resource types that name them. `start/a.toml`: deposit `copper_ore` at (0, 0) `count = 2`, then `farmland` at (0, 0) without `count`. `start/a/n.toml`: `iron_ore` at (1, 0) `count = 3`. `start/hexes.toml`: hexes (0, 0), (1, 0), (0, 1), defined **after** the deposits on them. Expected resource types: 0 `copper_ore`, 1 `iron_ore`, 2 `farmland`, 3 `unused`. Expected deposits: 0–1 `copper_ore`@(0,0), 2 `farmland`@(0,0), 3–5 `iron_ore`@(1,0). |
| `resources_one` | Manifest `name = "Resources Test"`. `rules/r.toml`: good `ore`; resource `ore_deposit` (produces `ore`, output `1`). `start/s.toml`: hex (0, 0); deposit `ore_deposit` at (0, 0). |
| `resources_errors` | `rules/a.toml`: good `ore`, then these `[[resource]]` entries in order: (1) `copper_ore`, `ore`, `1` (valid); (2) no `name`, `ore`, `1`; (3) `name = "Copper"`, `ore`, `1`; (4) `zero_out`, `ore`, `output = 0`; (5) `neg_out`, `ore`, `output = -1.5`; (6) `inf_out`, `ore`, `output = inf`; (7) `nan_out`, `ore`, `output = nan`; (8) `huge_out`, `ore`, `output = 1e400`; (9) `string_out`, `ore`, `output = "5"`; (10) `no_produces`, `output = 1`; (11) `no_output`, `produces = "ore"`; (12) `num_produces`, `produces = 3`, `output = 1`; (13) `bad_good`, `produces = "gold"`, `output = 1`; (14) `typo`, `ore`, `1`, `amount = 2`; (15) `name = "Bad-Name"`, `produces = 7`, `output = 0`, `colour = "red"`. `rules/b.toml`: `copper_ore` again (`ore`, `1`). `start/s.toml`: a `[[resource]]` (`name = "tin_ore"`, `produces = "ore"`, `output = 1`). If the TOML parser itself rejects `1e400` as a syntax error, drop entry (8) and say so in the report. |
| `deposits_errors` | `rules/r.toml`: good `ore`; resource `ore_deposit` (`ore`, `1`). `start/h.toml`: hexes (0, 0) through (9, 0) (q = 0..9, r = 0). `start/a.toml`, `[[deposit]]` entries in order: (1) `ore_deposit` (0, 0) (valid); (2) no `resource`, (0, 0); (3) `ore_deposit`, `r = 0` only; (4) `ore_deposit`, `q = 0` only; (5) `ore_deposit`, `q = 1.5`, `r = 0`; (6) `ore_deposit`, `q = 2147483648`, `r = 0`; (7) `ore_deposit` (1, 0) `count = 0`; (8) `ore_deposit` (2, 0) `count = -3`; (9) `ore_deposit` (3, 0) `count = 1000001`; (10) `ore_deposit` (4, 0) `count = 2.5`; (11) `gold_deposit` (0, 0); (12) `ore_deposit` (9, 9); (13) `ore_deposit` (6, 0) `output = 3`; (14) `resource = 5`, (7, 0); (15) `gold_deposit` (8, 0) `count = 0`, `colour = "x"`. `start/b.toml`: `ore_deposit` (0, 0) on line 1 (duplicate of entry 1). `rules/d.toml`: a `[[deposit]]` (`ore_deposit`, (0, 0)). Make `start/a.toml` start with entry 1's header on line 1, so the duplicate message reads `(first defined at start/a.toml:1:1)`. |
| `resources_broken_file` | `rules/broken.toml` with a TOML syntax error. `rules/r.toml`: resource `gold_ore` (produces `gold`, output `1`); no goods anywhere. `start/s.toml`: deposit `gold_ore` at (9, 9); no hexes. |
| `resources_missing_start` | `rules/r.toml`: resource `gold_ore` (produces `gold`, output `1`). No `start/` folder. |

### Tests
Engine integration tests go in `crates/engine/tests/resources.rs` (public API only). Assert exact `Display` strings, with exact line and column, wherever practical.

| # | Test | Acceptance Criterion |
|---|---|---|
| 1 | Sample: `rules.resources` is `copper_ore` (produces `unimproved_copper`, output 2.5), `arable_land` (`unimproved_grain`, 4), in that order. `start.deposits` has 10 deposits with the IDs, types, and hexes listed under Sample setting. Extend `sample_setting_loads` to check 2 resource types and 10 deposits. | sample loads with its resource types and 10 deposits |
| 2 | `resources_valid` loads. Resource types in the expected order, with name ↔ ID round trips, `produces` (by good name), and `output` (2.5, 3, 4, 0.25 via `Quantity::get`). `copper_ore` and `iron_ore` both produce `ore`. Deposits in the expected order with their types and hexes. | valid across files and nested folders; IDs in load order; name, ID, good, output; type and hex per deposit; whole and decimal outputs |
| 3 | `resources_valid`: `on_hex((0,0))` is deposits [0, 1, 2]; `on_hex((1,0))` is [3, 4, 5]; `on_hex((0,1))` is empty; `of_type_on_hex((0,0), copper_ore)` is [0, 1]; `of_type_on_hex((0,0), iron_ore)` is empty. | listing a hex's deposits, and of one type |
| 4 | `resources_valid`: the entry with `count = 3` gives three deposits and the entry without `count` gives one (covered by 2; assert the per-entry sizes explicitly). | count, and its default of 1 |
| 5 | Loading `resources_valid` twice gives the same resource-type and deposit lists. | loading twice gives the same IDs |
| 6 | `resources.id("missing")` returns `IdError::UnknownName { kind: "resource", .. }`. | lookup by name |
| 7 | `valid_minimal` loads with empty `rules.resources` and `start.deposits`. | no resource types and no deposits |
| 8 | `resources_errors` gives exactly this list, in sorted order, with exact positions: (2) `missing required key 'name' in resource` at its header; (3) `invalid name 'Copper' …`; (4)–(8) `key 'output' in resource must be a finite number greater than 0` at each value; (9) `key 'output' in resource must be a number, found string`; (10) `missing required key 'produces' in resource`; (11) `missing required key 'output' in resource`; (12) `key 'produces' in resource must be a string, found integer`; (13) `unknown good 'gold'` at the `produces` value; (14) `unknown key 'amount' in resource`; (15) all four of `invalid name 'Bad-Name' …`, `key 'produces' … found integer`, `… must be a finite number greater than 0`, `unknown key 'colour' in resource`; `rules/b.toml:…: duplicate resource name 'copper_ore' (first defined at rules/a.toml:…)`; `start/s.toml:…: section 'resource' belongs in rules/, not start/`. No other errors. | every resource error at file/line/column; several problems in one entry; unknown good reported at the reference |
| 9 | `deposits_errors` gives exactly this list, in sorted order, with exact positions: (2) `missing required key 'resource' in deposit` at its header, and no duplicate error; (3) `missing required key 'q' in deposit`; (4) `missing required key 'r' in deposit`, and no duplicate error; (5) `key 'q' in deposit must be an integer, found float`; (6) `key 'q' in deposit is out of range`; (7), (8) `key 'count' in deposit must be at least 1`; (9) `key 'count' in deposit is out of range (the most is 1000000)`; (10) `key 'count' in deposit must be an integer, found float`; (11) `unknown resource 'gold_deposit'` at the `resource` value; (12) `no hex at (9, 9)` at its header; (13) `unknown key 'output' in deposit`; (14) `key 'resource' in deposit must be a string, found integer`; (15) `unknown resource 'gold_deposit'`, `key 'count' in deposit must be at least 1`, and `unknown key 'colour' in deposit`; `start/b.toml:1:1: duplicate deposit of 'ore_deposit' at (0, 0) (first defined at start/a.toml:1:1)`; `rules/d.toml:…: section 'deposit' belongs in start/, not rules/`. No other errors. | every deposit error at file/line/column; several problems in one entry; no duplicate error from an invalid entry; duplicate names both places; unknown resource and missing hex at the reference |
| 10 | `resources_broken_file` gives only the TOML syntax error(s) for `rules/broken.toml`: no `unknown good`, `unknown resource`, or `no hex`. `resources_missing_start` gives only `start/: missing required folder` (exact text as the loader reports it): no `unknown good`. | reference errors not reported after a broken file or incomplete discovery |
| 11 | Test 2's setting defines goods and hexes in files read after the references to them. It loads, so references into later files resolve. | references resolve even when the target is in a later file |
| 12 | `quantity.rs` unit tests: `new` accepts `0.0`, `2.5`, `1e300`; rejects `-1.0`, `NaN`, `±inf`; `new(-0.0)` displays `0`; `Display` gives `4` and `2.5`; `Default` is `ZERO`. | (robustness) |
| 13 | `resources.rs` unit tests: `ResourceTypes` accessors and `iter` order; `get`/`name` panic on a foreign ID; `from_parts` panics on mismatched lengths. `Deposits::from_deposits` gives sequential IDs, `on_hex` and `of_type_on_hex` in ID order, an empty slice for a hex with none; `get` panics on a foreign ID (`is not from this list`); `on_hex` panics on a hex beyond `hex_count` (`is not from this map`); `from_deposits` panics on a hex index `>= hex_count`; the default is empty. | (robustness) |
| 14 | `reader.rs` unit tests for `required_number`: `5` → 5.0; `2.5`; `1_000.5` → 1000.5; `-3` → -3.0; `inf` → infinite; `nan` → NaN; `"5"` → `must be a number, found string`; missing → `missing required key`; `99999999999999999999` → `out of range`. | (robustness) |
| 15 | `setting/resources.rs` unit tests for `deposit_count` (with the `run`-style helper from `reader.rs`'s tests, or an equivalent): absent → 1; `1` → 1; `1000000` → 1000000; `1000001` → out of range; `0` and `-1` → at least 1. | count bounds (1,000,000 accepted, 1,000,001 rejected) |
| 16 | `checker.rs`: the sample prints `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits`. `goods_one` prints `…: 1 good, 0 hexes, 0 resources, 0 deposits`. `valid_minimal` prints `…: 0 goods, 0 hexes, 0 resources, 0 deposits`. `hexes_one` prints `Loaded setting 'Hexes Test' (format version 1): 0 goods, 1 hex, 0 resources, 0 deposits`. New: `resources_one` prints `Loaded setting 'Resources Test' (format version 1): 1 good, 1 hex, 1 resource, 1 deposit`. `many_errors`, the two-argument test, and `placeholder.rs` are unchanged. | dev UI counts; no-argument and broken-setting behavior unchanged |
| 17 | All existing tests still pass, with expectations changed only as listed in test 16 and `sample_setting_loads`. | (regression) |
| 18 | `python tools/check.py` exits 0. | single check command passes |

Run everything with `python tools/check.py` (or `.venv/Scripts/python tools/check.py`).

## Constraints
- No new external crates.
- Don't change `crates/engine/src/id.rs` or `crates/engine/src/map.rs`. If one seems to need a change, stop and report.
- `quantity.rs` and `resources.rs` must not depend on anything in `setting/`. The dependency runs from `setting` to the domain modules only.
- `ResourcesReader`, `DepositsReader`, `MapReader`, `Definitions`, `EntryReader`, `SectionType`, `SettingContent`, `WorldContent`, and `load_with` stay `pub(crate)`. `ResourceTypes::from_parts` and `Deposits::from_deposits` stay `pub(crate)`.
- Determinism: resource types and deposits are listed and numbered in load order. `HashMap`s are for lookup only; never iterate one to produce output, IDs, or errors.
- No `unwrap`/`expect` on anything a setting file controls. The `expect` in `ResourcesReader::finish` guards an internal invariant and is allowed.
- `Quantity` gets no arithmetic and no other methods beyond those listed.
- No emitting, ownership, stock, depletion, regeneration, or checks on the produced good's properties. No other section types.
- Don't change the goods or hex behavior or their messages. The only change to the hex code is sharing `coordinate` (same messages for hexes) and adding `MapReader::id`.
- Don't touch `PROJECT_PLAN/`, `CLAUDE.md`, `.claude/`, `docs/`, `manual.ipynb`, `manual.html`, or `tools/`.
- Don't commit.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` exits 0, and every Acceptance Criterion in design.md is met.

## Follow-ups for comp-review
- Step 04's F1 (`id.rs` item docs and the `compile_fail` example that registers a hex by name) is still open. `DepositKind` now exists and could replace the hex in that example. Left for comp-review, as before.
