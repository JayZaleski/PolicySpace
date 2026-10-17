# Step 02: businesses — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-02-businesses/design.md` — the behavior this spec implements.
- `PROJECT_PLAN/stage-03-factories/step-01-technologies/spec.md` — the most recent section type; this Step copies its reader shape (every key read first, every entry kept for phase 2, `Bundle` for a good = amount table).
- `docs/setting-loader.md` — the two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/pops.md` (Placed POPs: `PopsReader`, `Pops` with its per-hex index, POP stockpiles in the World), `docs/resources.md` (`Deposits`, `DepositsReader::resolve`), `docs/world.md` (the turn, emission as "the seam for deposit ownership", perishing, determinism), `docs/ids.md` (`NamedList`, `IdVec`), `docs/dev-ui.md` (the checker's success line), `docs/workspace.md` (the check command, test layout, shared test helpers).
- Code: `crates/engine/src/setting/technologies.rs` (the reader pattern), `crates/engine/src/setting/pops.rs` (`PopsReader`: coordinates, `no hex at`), `crates/engine/src/setting/resources.rs` (`DepositsReader`, `deposit_count` and its unit tests), `crates/engine/src/setting/reader.rs` (`required_number_table`, `required_quantity_table`, `integer`, `number_value`, the `run` test helper), `crates/engine/src/setting/map.rs` (`coordinate`, `MapReader::id`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/setting/mod.rs`, `crates/engine/src/pops.rs` (`Pops`: the list-plus-per-hex-index pattern), `crates/engine/src/resources.rs` (`Deposit`, `Deposits`), `crates/engine/src/bundle.rs`, `crates/engine/src/technologies.rs` (`NamedList` alias), `crates/engine/src/id.rs` (`NamedList`, `IdVec`), `crates/engine/src/world.rs` (`new`, `emit`, `perish`, accessors), `crates/engine/src/stockpile.rs` (`add`, `PerishesAt::for_good`), `crates/engine/src/lib.rs`, `crates/engine/src/test_support.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/pops.rs` (`pops_of`, `the_sample_setting_loads_with_its_pops`, `every_pop_error_is_reported_at_its_place`, `the_pops_placed_world_endows_durable_and_perishable_goods`), `crates/engine/tests/world.rs` (`sample_setting_at_turn_0`, `sample_setting_steps`, `identical_runs_give_identical_results`), `crates/engine/tests/resources.rs` (`references_are_not_checked_after_a_broken_file_or_incomplete_discovery`, the `help` constant), `crates/dev-ui/src/check.rs`, `crates/dev-ui/src/text.rs` (`stockpile_lines_on_the_sample_at_turn_0`, `good_rows_list_every_good_with_properties_and_world_totals`), `crates/dev-ui/tests/checker.rs`, `settings/sample/start/pops.toml` (header comment style), `settings/sample/start/deposits.toml`, `settings/sample/rules/resources.toml`.

## Approach

- **A `[[business]]` section in `start/`**, read by a new `BusinessesReader` (`setting/businesses.rs`) that follows `TechnologiesReader`: the name goes through `Definitions<BusinessKind>`, every key is read before anything is defined, every entry is kept for phase 2 (so an entry with a bad name still reports its unknown goods and resource types), and the references (goods in `inventory`, resource types in `deposits`, the hex) are resolved in phase 2. Rationale: businesses are the first named kind in `start/`; the named-kind reader pattern already does everything but the hex.
- **Domain types in a new module `crates/engine/src/businesses.rs`**: `BusinessKind`/`BusinessId`, `Business` (hex, decider name, starting inventory as a `Bundle`), and `Businesses`, a `NamedList` plus a per-hex index like `Pops`. The module knows nothing about files. The starting inventory stays on the record after the world is created (the World fills the stockpile from it at turn 0; from then on the stockpile is the truth). Rationale: one public type, no second per-business list in `StartState`.
- **Deposit ownership lives on the deposit**: `Deposit` gains `owner: Option<BusinessId>`, set by the loader and nothing else yet, and `Deposits::owned_by(business)` lists a business's deposits. Rationale: `Deposits` is already world state for exactly this reason (see `docs/world.md`), and emission already walks the deposits.
- **Deposit assignment in phase 2**, after `DepositsReader::resolve`, through a new `DepositsReader::claim(hex, resource, count, owner)` that gives the first `count` unowned deposits of that type on that hex, in deposit ID order, to the owner, or assigns none and reports how many were available. Businesses claim in load order, so the same folder always gives the same ownership. Only an entry that got an ID and whose hex, resource and count are all valid claims anything.
- **A new `EntryReader::required_integer_table`** for the `deposits` table (whole-number counts), mirroring `required_number_table` with an `integer_value` helper that `integer` is refactored onto, so a bad count reports the same `must be an integer, found float` / `is out of range` as a deposit's `count`. The range check (at least 1, fits a `u32`) is the business reader's.
- **World**: `businesses: Businesses` and one `Stockpile` per business, filled from the starting inventory at turn 0 (each good in the group `PerishesAt::for_good(perishes, Turn::ZERO)`) before turn 0 runs. `emit` adds an owned deposit's output to the owner's stockpile and an unowned deposit's to the hex stockpile. `perish` covers hexes, then POPs, then businesses, each in ID order. The report's shape doesn't change.
- **Checker**: the success line gains `, N businesses` at the end. No other dev UI change: the goods panel's world totals don't count business stockpiles until Step 09 shows businesses (decided with the user).
- **The build runs as one.** The reader needs the domain types, the `EntryReader` addition and `DepositsReader::claim`; the World needs the domain types; every test (engine and dev UI) needs the loaded sample. No package could be built and tested alone.

## Changes

### New

- `crates/engine/src/businesses.rs` — domain types and unit tests.
- `crates/engine/src/setting/businesses.rs` — `BusinessesReader` and unit tests.
- `crates/engine/tests/businesses.rs` — integration tests (Test Plan).
- Test settings `crates/engine/tests/settings/businesses_valid/`, `businesses_one/`, `businesses_errors/` (exact contents below).
- `settings/sample/start/businesses.toml`.

### Modified

- `crates/engine/src/lib.rs` — `pub mod businesses; pub use businesses::{Business, BusinessId, BusinessKind, Businesses};` after `bundle`, before `currency`. Crate docs: a sentence that businesses (the [`businesses`] module) are named actors in the starting state, each on one hex with a decider name, a starting inventory and the deposits it owns, each with its own stockpile in the world; an owned deposit emits into its owner's stockpile; nothing decides or trades for them yet.
- `crates/engine/src/resources.rs` — `Deposit.owner`, `Deposits::owned_by`; module docs (ownership exists: at most one owner per deposit, written on the business in the setting, unchanged during play yet; an owned deposit emits into its owner's stockpile); `owner: None` in every test fixture; a unit test for `owned_by`.
- `crates/engine/src/setting/resources.rs` — `Deposit { resource, hex, owner: None }` in `resolve`; `DepositsReader::claim`; module docs; a unit test for `claim`.
- `crates/engine/src/setting/reader.rs` — `integer_value`, `integer` refactored onto it, `required_integer_table`; a unit test.
- `crates/engine/src/setting/mod.rs` — `mod businesses;`, `StartState.businesses`, `WorldContent.businesses`, the `business` section type, `resolve`, `load_setting`; module docs (businesses are the fifth `start/` kind and a named kind; they name goods in `inventory`, resource types in `deposits` and a hex by coordinates, all resolved in phase 2, where they also claim their deposits; the "Adding a section type" list names `crate::businesses` / `setting/businesses.rs` as the named kind in `start/` with a per-hex index).
- `crates/engine/src/world.rs` — fields, `new`, `emit`, `perish`, accessors, module docs and `TurnReport` docs (`perished` covers business stockpiles too; the ownership seam is used); the clock-maximum unit test also runs `businesses_valid` and checks business stockpiles are unchanged.
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 businesses` at the end, except the sample (`, 3 businesses`); a new test for `businesses_one` (`1 business`). The turn lines don't change.
- `crates/dev-ui/src/text.rs` — tests `stockpile_lines_on_the_sample_at_turn_0` (the center holds only `4 unimproved_grain`, `4: perishes this turn`) and `good_rows_list_every_good_with_properties_and_world_totals` (`in_hexes` becomes `["0", "7.5", "16", "0", "0", "0", "0"]`; `in_pops` unchanged). No code change.
- `crates/engine/tests/world.rs` — `sample_setting_at_turn_0` and `sample_setting_steps`: the center hex's totals become `[pair("unimproved_grain", 4.0)]`, its `unimproved_copper` groups are empty and its `unimproved_grain` groups `[(end(t), 4.0)]`; the reports are unchanged.
- `crates/engine/tests/pops.rs` — `a_fresh_sample_world_endows_every_pop`: its assertion on the center hex's stockpile becomes `[("unimproved_grain", 4.0)]` (the same change as in `tests/world.rs`).
- `crates/engine/tests/resources.rs` — the comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a business whose good doesn't exist, on a hex that doesn't exist).
- Test setting `resources_broken_file/start/s.toml` (append a `[[business]]`, below).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- `settings/sample/start/deposits.toml` and `settings/sample/rules/resources.toml` — the comments that say deposits have no owners yet now say ownership is written on the business (`start/businesses.toml`) and that an owned deposit emits into its owner's stockpile.
- `settings/sample/start/README.md` — mention `businesses.toml`.

## Interfaces & Data

### `crates/engine/src/businesses.rs`

```rust
//! Module docs: Businesses. A business (`[[business]]` in `start/`) is a named actor on one
//! hex: it has a decider name (the name of the decider that will control it; Step 07
//! resolves it, this Step only stores it), a starting inventory (the goods, cash included,
//! in its stockpile when the world is created; after that the stockpile in the `world`
//! module is the truth), and owns deposits on its hex (written on the deposit, see
//! `resources`). A business may own only deposits on its hex, and may sit on a hex with
//! no market. Business names are their own namespace. Nothing decides or trades for a
//! business yet. Knows nothing about files.

use crate::bundle::Bundle;
use crate::id::{Id, IdError, IdKind, IdVec, NameRegistry, NamedList};
use crate::map::{HexId, HexKind};

/// Marker kind for businesses.
pub enum BusinessKind {}
impl IdKind for BusinessKind {
    const LABEL: &'static str = "business";
}
/// The ID of a business. Valid only within the [`Businesses`] that produced it.
pub type BusinessId = Id<BusinessKind>;

/// One business. Its stockpile is world state, not part of this record.
#[derive(Debug, Clone, PartialEq)]
pub struct Business {
    /// The hex it sits on (its only hex until trade between hexes exists).
    pub hex: HexId,
    /// The name of the decider that controls it. A valid name; not resolved yet.
    pub decider: String,
    /// What its stockpile holds when the world is created, cash included. May be empty.
    /// Every amount is greater than 0. The World fills the stockpile from this at turn 0.
    pub starting_inventory: Bundle,
}

/// Every business in a setting, in load order (which is also ID order), with a per-hex
/// index. `Businesses::default()` is empty and was built with no map, so `on_hex` panics
/// on it for any hex.
#[derive(Debug, Default)]
pub struct Businesses {
    list: NamedList<BusinessKind, Business>,
    /// One entry per hex, by `HexId`: the businesses on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<BusinessId>>,
}

impl Businesses {
    /// Crate-private: only the loader builds businesses. `hex_count` is the number of
    /// hexes in the map.
    ///
    /// # Panics
    /// Panics if `names` and `businesses` differ in length (through `NamedList::from_parts`,
    /// `"every business name needs exactly one entry"`), if a business's hex index is
    /// `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), or if there are
    /// more than `u32::MAX` businesses.
    pub(crate) fn from_parts(
        names: NameRegistry<BusinessKind>,
        businesses: Vec<Business>,
        hex_count: usize,
    ) -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// The ID of the business named `name`, or `IdError::UnknownName { kind: "business", .. }`.
    pub fn id(&self, name: &str) -> Result<BusinessId, IdError>;
    /// Panics (`not from this registry`) for a foreign ID.
    pub fn name(&self, id: BusinessId) -> &str;
    /// Panics (`is not from this list`) for a foreign ID.
    pub fn get(&self, id: BusinessId) -> &Business;
    /// (ID, name, business) in load order.
    pub fn iter(&self) -> impl Iterator<Item = (BusinessId, &str, &Business)> + '_;
    /// The businesses on `hex`, in ID order; empty if none. Panics for a hex not from the
    /// map the list was built with.
    pub fn on_hex(&self, hex: HexId) -> &[BusinessId];
}
```

`from_parts`: build the `NamedList` first, then `by_hex` exactly as `Pops::from_pops` does (`IdVec::from_vec(vec![Vec::new(); hex_count])`, assert the hex index, push). Everything else delegates to `list`.

### `crates/engine/src/resources.rs`

```rust
pub struct Deposit {
    pub resource: ResourceId,
    pub hex: HexId,
    /// The business that owns it, if any. Set by the loader from the owner's `[[business]]`
    /// entry; nothing changes it during play yet. An owned deposit emits into its owner's
    /// stockpile, an unowned one into its hex's.
    pub owner: Option<BusinessId>,
}

impl Deposits {
    /// The deposits owned by `business`, in ID order (a filter over every deposit).
    pub fn owned_by(&self, business: BusinessId) -> impl Iterator<Item = DepositId> + '_;
}
```

`Deposit` keeps `Debug, Clone, Copy, PartialEq, Eq, Hash`. `resources.rs` now imports `crate::businesses::BusinessId`; `businesses.rs` doesn't import `resources`, so there is no cycle.

### `EntryReader` additions (`crates/engine/src/setting/reader.rs`)

```rust
/// Parses `value` as an integer (`i64`). `prefix` starts the message of any problem, as
/// for `number_value`: a non-integer is `<prefix> must be an integer, found <type>`, an
/// integer beyond `i64` is `<prefix> is out of range`.
fn integer_value(&mut self, prefix: &str, value: &Spanned<DeValue<'i>>) -> Option<Located<i64>>;

/// Reads `key` as a table whose values are integers: like `required_number_table`, but
/// each value is parsed as `required_integer` parses one. Returns every key with its
/// location, in the table's order, and `None` where the value isn't an integer or is out
/// of range (reported as `key '<k>' in <key> of <label> must be an integer, found <type>`
/// or `... is out of range`). A missing key gives the usual missing-key error and a
/// non-table value `key '<key>' in <label> must be a table, found <type>`; both return None.
/// The table's keys are data: not name-checked, not tracked as read or unknown.
#[allow(clippy::type_complexity)]
pub fn required_integer_table(&mut self, key: &str)
    -> Option<Vec<(Located<String>, Option<Located<i64>>)>>;
```

Refactor the private `integer(key, required)` to call `integer_value(&format!("key '{key}' in {}", self.label), value)` so the two share one parse; its messages stay byte-for-byte the same (`key 'q' in pop must be an integer, found float`, `key 'q' in pop is out of range`). `required_integer_table` is `required_number_table` with `integer_value` in place of `number_value`.

### `DepositsReader::claim` (`crates/engine/src/setting/resources.rs`)

```rust
/// Phase 2, after `resolve`: gives the first `count` unowned deposits of `resource` on
/// `hex`, in ID order, to `owner`. If fewer than `count` are unowned, assigns none and
/// returns `Err(available)`.
pub fn claim(&mut self, hex: HexId, resource: ResourceId, count: u32, owner: BusinessId) -> Result<(), u32>;
```

Count the matching unowned deposits first (`self.deposits.iter().filter(..)`; the count fits a `u32` because the total does); if short, return `Err`; otherwise walk `iter_mut()` and set `owner = Some(owner)` on the first `count` matches.

### `BusinessesReader` (`crates/engine/src/setting/businesses.rs`)

```rust
//! Reading `[[business]]` sections. Domain types in `crate::businesses`. Businesses are a
//! named kind in `start/`, so they go through `Definitions` like technologies; the goods in
//! `inventory`, the resource types in `deposits` and the hex are resolved in phase 2, where
//! each business also claims its deposits from `DepositsReader`. Every entry is kept for
//! phase 2, even one whose name was bad, so its unknown goods and resource types are still
//! reported; only entries with an ID reach `finish`, and only they claim deposits.

/// Every `inventory` key with its amount if it was valid (from `required_quantity_table`).
type Pairs = Vec<(Located<String>, Option<Quantity>)>;
/// Every `deposits` key with its count if it was valid.
type Counts = Vec<(Located<String>, Option<u32>)>;

/// One `[[business]]` entry, waiting for phase 2.
struct BusinessEntry {
    /// The `[[business]]` header.
    at: Location,
    /// None if the name was missing, invalid or a duplicate.
    id: Option<BusinessId>,
    /// None if `q` or `r` was missing or invalid (already reported).
    coord: Option<HexCoord>,
    inventory: Pairs,
    deposits: Counts,
    /// None if missing or invalid (already reported).
    decider: Option<Located<String>>,
    /// Filled in phase 2.
    hex: Option<HexId>,
    /// Filled in phase 2, parallel to `inventory`.
    inventory_ids: Vec<Option<GoodId>>,
    /// Filled in phase 2, parallel to `deposits`.
    deposit_ids: Vec<Option<ResourceId>>,
}

#[derive(Default)]
pub(crate) struct BusinessesReader {
    defs: Definitions<BusinessKind>,
    entries: Vec<BusinessEntry>,
}

impl BusinessesReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(
        &mut self,
        goods: &Definitions<GoodKind>,
        resources: &Definitions<ResourceKind>,
        map: &MapReader,
        deposits: &mut DepositsReader,
        sink: &mut ErrorSink,
    );
    /// Only after a successful load.
    pub fn finish(self, hex_count: usize) -> Businesses;
}
```

A private helper `fn read_deposits(reader: &mut EntryReader<'_, '_>) -> Counts`: `reader.required_integer_table("deposits").unwrap_or_default()`, then for each `(key, value)`: `None` stays `None` (already reported); a value `< 1` is reported at the value as `key '<k>' in deposits of <label> must be at least 1` and becomes `None`; a value that doesn't fit a `u32` is reported at the value as `key '<k>' in deposits of <label> is out of range` and becomes `None`; otherwise `Some(count)`. (`label` is `reader.label()`.)

`read`, in this order, all before defining anything:

1. `at = reader.location().clone()`.
2. `name = reader.required_name("name")`.
3. `q = coordinate(reader, "q")`, `r = coordinate(reader, "r")` (the shared helper from `setting/map.rs`).
4. `inventory = reader.required_quantity_table("inventory").unwrap_or_default()` (a missing or non-table key is reported there and gives an empty list; the load fails anyway).
5. `deposits = read_deposits(reader)`.
6. `decider = reader.required_name("decider")` (a non-string gives `key 'decider' in business must be a string, found <type>`; a bad name the usual `invalid name '...'`).
7. `id = name.and_then(|n| self.defs.define(&n, reader.errors()))`. Push the entry whatever `id` is, with `coord: q.zip(r).map(|(q, r)| HexCoord::new(q, r))`.

`resolve`, for every entry in order (load order, which is also claim order):

1. `inventory_ids`: every key through `goods.resolve` (`unknown good 'tin'` at the key). Always.
2. `deposit_ids`: every key through `resources.resolve` (`unknown resource 'meadow'` at the key). Always.
3. `hex`: if `coord` is `Some`, `map.id(coord)`; if that is `None`, report `no hex at (q, r)` at the header (`format!("no hex at {coord}")`, as POPs do).
4. Claims: only if `id` and `hex` are both `Some`. For each `(key, count)` zipped with `deposit_ids`, only if both `count` and the resource are `Some`: `deposits.claim(hex, resource, count, id)`; on `Err(available)` report at `key.at`: `key '<k>' in deposits of business claims more deposits than the hex has unowned (<count> claimed, <available> available)`. (The label is hard-coded `business` here, like `no hex at`.)

`finish(hex_count)`: for each entry with `Some(id)`, in order, `Business { hex: hex.expect(UNRESOLVED), decider: decider.expect(UNRESOLVED).value, starting_inventory: Bundle::new(inventory zipped with inventory_ids, each good and amount expected) }` with `UNRESOLVED = "internal error: a business was not fully resolved in a successful load"`. Then `Businesses::from_parts(self.defs.into_registry(), list, hex_count)`. `Bundle::new` can't see a duplicate good because TOML forbids repeated keys.

### `setting/mod.rs`

- `StartState` gains `pub businesses: Businesses` (doc: every business, in load order; see `crate::businesses`).
- `WorldContent` gains `pub businesses: BusinessesReader`; `section_types` gains `SectionType { header: "business", folder: Folder::Start, read: |c, r| c.businesses.read(r) }` after `technology`.
- `resolve`: add last `self.businesses.resolve(self.goods.defs(), self.resources.defs(), &self.map, &mut self.deposits, sink);` (disjoint fields, so the borrows don't conflict; it must come after `self.deposits.resolve`, which builds the deposit list the claims walk). Extend the comment.
- `load_setting`: `let businesses = content.businesses.finish(map.len());` after `pops`, and `StartState { map, currencies, deposits, prices, pops, businesses }`.

### `world.rs`

- Fields: `businesses: Businesses` (doc: nothing changes them yet) and `business_stockpiles: IdVec<BusinessKind, Stockpile>` (one per business, by `BusinessId`; holds the starting inventory when the world is created).
- `new`: before building the `World`, build the business stockpiles: for each `(_, _, business)` in `start.businesses.iter()`, a `Stockpile::new()` with, for each `(good, amount)` in `business.starting_inventory.iter()`, `add(good, amount, PerishesAt::for_good(rules.goods.get(good).perishes, Turn::ZERO))`. Then `run_turn` as now, so turn 0's emission lands on top.
- `emit`: the target stockpile is `match deposit.owner { Some(b) => self.business_stockpiles.get_mut(b), None => self.hex_stockpiles.get_mut(deposit.hex) }`. Deposit ID order is unchanged, so `emitted` is unchanged.
- `perish`: chain `self.business_stockpiles.values_mut()` after the POP stockpiles.
- Accessors: `pub fn businesses(&self) -> &Businesses` and `pub fn business_stockpile(&self, business: BusinessId) -> &Stockpile` (panics `is not from this list` for a foreign ID, like `pop_stockpile`).
- Docs: the module docs (the world holds the businesses and one stockpile each; owned deposits emit into their owner's stockpile, the hex stockpile stays the home of unowned goods; perishing covers hexes, POPs, businesses; the "Stage 04" ownership remark becomes the present tense; business ID order joins the determinism list), `TurnReport::perished` (every hex, POP and business stockpile), `TurnReport::emitted` (into hex or business stockpiles).

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.start.businesses.len(), "business", "businesses")`. The sample line becomes
`Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses`.
Every other expected line in `checker.rs` ends in `, 0 businesses`; the new `businesses_one` test expects `Loaded setting 'Businesses One' (format version 1): 1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 0 technologies, 1 business`.

### Setting file format

A `[[business]]` in `start/` has six required keys: `name` (a valid name, unique among businesses; a separate namespace from goods, resource types, POP types and technologies), `q` and `r` (integer axial coordinates of a hex in the map, read as a deposit's), `inventory` (a table of good name = amount, each a finite number greater than 0; may be empty but must be written; the hex's currency listed here is the business's starting cash), `deposits` (a table of resource type name = count, each a whole number of at least 1; may be empty but must be written; the business gets that many not-yet-owned deposits of that type on its hex, in deposit ID order, businesses taken in load order), `decider` (a valid name; stored, not resolved). Both inline tables and sub-tables (`[business.inventory]`, `[business.deposits]`) are accepted. Any other key is unknown. Goods, resource types and the hex are resolved in phase 2, so they may be defined in any file; deposit assignment happens there too, and both are skipped when a file was rejected or discovery was incomplete.

All `[[business]]` errors and their exact texts (`{help}` is the usual invalid-name suffix, `{positive}` is `must be a finite number greater than 0`):

| Problem | Reported at | Message |
|---|---|---|
| `name`, `q`, `r`, `inventory`, `deposits` or `decider` missing | header | `missing required key 'inventory' in business` |
| a bad `name` or `decider` | value | `invalid name 'Bad Name' {help}` |
| a duplicate name | value | `duplicate business name 'ok' (first defined at <location>)` |
| `q` or `r` not an integer | value | `key 'q' in business must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in business is out of range` |
| `inventory` or `deposits` not a table | value | `key 'inventory' in business must be a table, found integer` |
| `decider` not a string | value | `key 'decider' in business must be a string, found integer` |
| an inventory amount that isn't a number | value | `key 'coin' in inventory of business must be a number, found string` |
| an inventory amount that is zero, negative, `inf` or `nan` | value | `key 'grain' in inventory of business {positive}` |
| a deposit count that isn't an integer | value | `key 'field' in deposits of business must be an integer, found float` |
| a deposit count below 1 | value | `key 'field' in deposits of business must be at least 1` |
| a deposit count beyond `u32` (or `i64`) | value | `key 'bog' in deposits of business is out of range` |
| an unknown good in `inventory` | key | `unknown good 'tin'` |
| an unknown resource type in `deposits` | key | `unknown resource 'meadow'` |
| more deposits claimed than the hex has unowned | key | `key 'field' in deposits of business claims more deposits than the hex has unowned (3 claimed, 1 available)` |
| coordinates that match no hex | header | `no hex at (9, 9)` |
| any other key | key | `unknown key 'colour' in business` |
| `[[business]]` in `rules/` | header | `section 'business' belongs in start/, not rules/` |

### Test setting contents

Write these files exactly; line numbers and columns matter for the expected errors. Every test setting has a `setting.toml` with `format_version = 1`.

**`businesses_valid/`** — `setting.toml`: name `Businesses Test`. `rules/goods.toml`: `coin` (no properties), `labor` (`perishes_after = 1`, `movable = false`), `grain` (`perishes_after = 3`), `ore` (no properties), in that order (IDs 0-3). `rules/resources.toml`: `field` produces `grain`, output 2; `vein` produces `ore`, output 1.5 (IDs 0, 1). `start/a.toml` (an inventory written out of ID order; the second business in the sub-table forms, its deposits out of resource ID order):
```toml
[[business]]
name = "farm_a"
q = 0
r = 0
inventory = { labor = 4, coin = 10 }
deposits = { field = 2 }
decider = "rule"

[[business]]
name = "farm_b"
q = 0
r = 0
decider = "rule"

[business.inventory]
coin = 5

[business.deposits]
vein = 1
field = 1
```
`start/b/c.toml` (an empty inventory and no deposits, on a hex with no market):
```toml
[[business]]
name = "idle"
q = 1
r = 0
inventory = {}
deposits = {}
decider = "other_rule"
```
`start/z.toml` (read after every business file, so the hexes and deposits are forward references):
```toml
[[hex]]
q = 0
r = 0
currency = "coin"

[[hex]]
q = 1
r = 0
currency = "coin"

[[deposit]]
resource = "field"
q = 0
r = 0
count = 3

[[deposit]]
resource = "vein"
q = 0
r = 0

[[deposit]]
resource = "field"
q = 1
r = 0

[[market]]
q = 0
r = 0
prices = { grain = 2, ore = 3 }
```
Expected: businesses `farm_a` = ID 0 at (0, 0), decider `rule`, inventory `[(coin, 10), (labor, 4)]`; `farm_b` = ID 1 at (0, 0), `rule`, `[(coin, 5)]`; `idle` = ID 2 at (1, 0), `other_rule`, `[]`. `on_hex((0, 0))` is `[0, 1]`, `on_hex((1, 0))` is `[2]`. Deposit IDs 0-2 are `field` at (0, 0), 3 `vein` at (0, 0), 4 `field` at (1, 0); owners by deposit ID: `[Some(0), Some(0), Some(1), Some(1), None]`; `owned_by(0)` is `[0, 1]`, `owned_by(1)` is `[2, 3]`, `owned_by(2)` is empty. `hex_prices.has_market` is true for (0, 0) and false for (1, 0).

World (good ID order in every list; `end(t)` is `PerishesAt::EndOfTurn(Turn::new(t))`):
- Turn 0: `farm_a` holds `[(coin, 10), (labor, 4), (grain, 4)]` with groups coin `[(Never, 10)]`, labor `[(end(0), 4)]`, grain `[(end(2), 4)]`; `farm_b` `[(coin, 5), (grain, 2), (ore, 1.5)]`; `idle` empty; hex (0, 0) empty; hex (1, 0) `[(grain, 2)]`. Report: `perished` empty, `emitted` `[(grain, 8), (ore, 1.5)]`, `endowed`, `sold`, `bought`, `consumed` empty, `population` 0.
- Turn 1: `perished` `[(labor, 4)]`; `farm_a` `[(coin, 10), (grain, 8)]` with grain groups `[(end(2), 4), (end(3), 4)]`; `farm_b` `[(coin, 5), (grain, 4), (ore, 3)]`; hex (1, 0) `[(grain, 4)]`; `emitted` as before.
- Turn 2: `perished` empty; `farm_a` grain 12; `farm_b` grain 6, ore 4.5; hex (1, 0) grain 6.
- Turn 3: `perished` `[(grain, 8)]` (4 from `farm_a`, 2 from `farm_b`, 2 from hex (1, 0)); `farm_a` grain 12 with groups at `end(3)`, `end(4)`, `end(5)`; `farm_b` grain 6, ore 6; hex (1, 0) grain 6.

**`businesses_one/`** — `setting.toml`: name `Businesses One`. `rules/goods.toml`: good `coin`. `start/map.toml`: one hex (0, 0), currency `coin`. `start/businesses.toml`: one `[[business]]` `solo` at (0, 0), `inventory = {}`, `deposits = {}`, `decider = "rule"`.

**`businesses_errors/`** — `setting.toml`: name `Businesses Errors`. `rules/goods.toml`: goods `coin`, `grain`. `rules/r.toml`: resource `field` produces `grain`, output 2. `rules/x.toml`:
```toml
[[business]]
name = "misplaced"
q = 0
r = 0
inventory = {}
deposits = {}
decider = "rule"
```
`start/d.toml`: one `[[deposit]]`, `resource = "field"`, `q = 0`, `r = 0`, `count = 2`. `start/h.toml`: hexes (0, 0) and (1, 0), currency `coin`. `start/b.toml`:
```toml
[[business]]
name = "ok"
q = 1
r = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "second"
q = 0
r = 0
inventory = {}
deposits = { field = 2 }
decider = "rule"
```
`start/a.toml` (164 lines):
```toml
[[business]]
name = "ok"
q = 0
r = 0
inventory = { coin = 10 }
deposits = { field = 1 }
decider = "rule"

[[business]]
q = 0
r = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "no_q"
r = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "no_r"
q = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "no_inventory"
q = 0
r = 0
deposits = {}
decider = "rule"

[[business]]
name = "no_deposits"
q = 0
r = 0
inventory = {}
decider = "rule"

[[business]]
name = "no_decider"
q = 0
r = 0
inventory = {}
deposits = {}

[[business]]
name = "Bad Name"
q = 0
r = 0
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "ok"
q = 0
r = 0
inventory = {}
deposits = { field = 0 }
decider = "rule"

[[business]]
name = "wrong_types"
q = 1.5
r = "0"
inventory = 5
deposits = "field"
decider = 7

[[business]]
name = "out_of_range"
q = 2147483648
r = -2147483649
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "no_hex"
q = 9
r = 9
inventory = {}
deposits = {}
decider = "rule"

[[business]]
name = "bad_amounts"
q = 0
r = 0
inventory = { coin = "2", grain = 0, tin = -1 }
deposits = {}
decider = "rule"

[[business]]
name = "bad_amounts_more"
q = 0
r = 0
inventory = { coin = inf, grain = nan }
deposits = {}
decider = "rule"

[[business]]
name = "bad_counts"
q = 0
r = 0
inventory = {}
deposits = { field = 0, meadow = -1 }
decider = "rule"

[[business]]
name = "bad_counts_more"
q = 0
r = 0
inventory = {}
deposits = { field = 1.5, meadow = "2", bog = 4294967296 }
decider = "rule"

[[business]]
name = "unknown_refs"
q = 0
r = 0
inventory = { tin = 1 }
deposits = { gold_vein = 1 }
decider = "rule"

[[business]]
name = "too_many"
q = 0
r = 0
inventory = {}
deposits = { field = 3 }
decider = "rule"

[[business]]
name = "bad_decider"
q = 0
r = 0
inventory = {}
deposits = {}
decider = "Rule One"

[[business]]
name = "unknown_keys"
q = 0
r = 0
inventory = {}
deposits = {}
decider = "rule"
colour = "red"
cash = 5

[[business]]
name = "Several"
q = 0
r = 10
inventory = { tin = 0 }
deposits = { gold_vein = 0, field = 1 }
decider = "Bad"
colour = "red"
```
Expected error lines, in this order:
```
rules/x.toml:1:3: section 'business' belongs in start/, not rules/
start/a.toml:9:1: missing required key 'name' in business
start/a.toml:16:1: missing required key 'q' in business
start/a.toml:23:1: missing required key 'r' in business
start/a.toml:30:1: missing required key 'inventory' in business
start/a.toml:37:1: missing required key 'deposits' in business
start/a.toml:44:1: missing required key 'decider' in business
start/a.toml:52:8: invalid name 'Bad Name' {help}
start/a.toml:60:8: duplicate business name 'ok' (first defined at start/a.toml:2:8)
start/a.toml:64:22: key 'field' in deposits of business must be at least 1
start/a.toml:69:5: key 'q' in business must be an integer, found float
start/a.toml:70:5: key 'r' in business must be an integer, found string
start/a.toml:71:13: key 'inventory' in business must be a table, found integer
start/a.toml:72:12: key 'deposits' in business must be a table, found string
start/a.toml:73:11: key 'decider' in business must be a string, found integer
start/a.toml:77:5: key 'q' in business is out of range
start/a.toml:78:5: key 'r' in business is out of range
start/a.toml:83:1: no hex at (9, 9)
start/a.toml:95:22: key 'coin' in inventory of business must be a number, found string
start/a.toml:95:35: key 'grain' in inventory of business {positive}
start/a.toml:95:38: unknown good 'tin'
start/a.toml:95:44: key 'tin' in inventory of business {positive}
start/a.toml:103:22: key 'coin' in inventory of business {positive}
start/a.toml:103:35: key 'grain' in inventory of business {positive}
start/a.toml:112:22: key 'field' in deposits of business must be at least 1
start/a.toml:112:25: unknown resource 'meadow'
start/a.toml:112:34: key 'meadow' in deposits of business must be at least 1
start/a.toml:120:22: key 'field' in deposits of business must be an integer, found float
start/a.toml:120:27: unknown resource 'meadow'
start/a.toml:120:36: key 'meadow' in deposits of business must be an integer, found string
start/a.toml:120:41: unknown resource 'bog'
start/a.toml:120:47: key 'bog' in deposits of business is out of range
start/a.toml:127:15: unknown good 'tin'
start/a.toml:128:14: unknown resource 'gold_vein'
start/a.toml:136:14: key 'field' in deposits of business claims more deposits than the hex has unowned (3 claimed, 1 available)
start/a.toml:145:11: invalid name 'Rule One' {help}
start/a.toml:154:1: unknown key 'colour' in business
start/a.toml:155:1: unknown key 'cash' in business
start/a.toml:157:1: no hex at (0, 10)
start/a.toml:158:8: invalid name 'Several' {help}
start/a.toml:161:15: unknown good 'tin'
start/a.toml:161:21: key 'tin' in inventory of business {positive}
start/a.toml:162:14: unknown resource 'gold_vein'
start/a.toml:162:26: key 'gold_vein' in deposits of business must be at least 1
start/a.toml:163:11: invalid name 'Bad' {help}
start/a.toml:164:1: unknown key 'colour' in business
start/b.toml:2:8: duplicate business name 'ok' (first defined at start/a.toml:2:8)
start/b.toml:14:14: key 'field' in deposits of business claims more deposits than the hex has unowned (2 claimed, 1 available)
```
Notes: the hex (0, 0) has two `field` deposits; `ok` (line 1) claims one, so one is left for everybody after it. The duplicate `ok` (line 59) has no ID and claims nothing, but its bad count is still reported. `too_many` (line 131) asks for 3 with 1 available and gets none. `Several` (line 157) has no ID, so its valid `field = 1` claims nothing; its unknown resource and bad count are still reported, and `no hex` is reported at its header. `second` in `start/b.toml` shows a second business running out. If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader` and `Definitions` message forms are fixed. The columns come from the TOML spans (column of the key's or value's first character); if `toml` reports a span differently than assumed, adjust the columns only.

**`resources_broken_file/start/s.toml`**: append
```toml

[[business]]
name = "nobody"
q = 9
r = 9
inventory = { tin = 1 }
deposits = { gold_ore = 5 }
decider = "ghost"
```
The expected errors stay exactly the one syntax error.

### Sample setting

**`settings/sample/start/businesses.toml`**, with a header comment in the style of `pops.toml` that explains every key (the six keys and their rules, as in "Setting file format" above), says that a business sits on one hex and may own only deposits there, that its cash is the hex's currency in its inventory, that owned deposits emit into the business's stockpile while unowned ones keep emitting into the hex stockpile, that deposits are handed out in ID order with businesses taken in load order, that the decider name is only stored for now (Step 07 resolves it; `arithmetic` is the name Step 06's rule will be registered as), and that nothing decides or trades for a business yet. Explain the numbers: one business per technology in the chain; the farm owns two of the center's three arable land deposits (the third stays unowned, so the center hex keeps 4 unimproved grain a turn), the mine owns the copper deposit, the toolmaker owns none and will buy its copper; the starting silver (100, 50, 200) is a few turns of inputs at the center's prices (about 18, 5 and 46 silver a turn at capacity), so each can run until its first sales come in once Step 08 lets them trade. Then:
```toml
# The farm owns two of the center's three arable land deposits.
[[business]]
name = "center_farm"
q = 0
r = 0
inventory = { silver = 100 }
deposits = { arable_land = 2 }
decider = "arithmetic"

# The mine owns the center's copper deposit.
[[business]]
name = "center_mine"
q = 0
r = 0
inventory = { silver = 50 }
deposits = { copper_ore = 1 }
decider = "arithmetic"

# The toolmaker owns no deposit: it buys its copper.
[[business]]
name = "center_toolmaker"
q = 0
r = 0
inventory = { silver = 200 }
deposits = {}
decider = "arithmetic"
```
(Comment wording is free; the values are exact.) Expected: businesses `center_farm` = ID 0, `center_mine` = ID 1, `center_toolmaker` = ID 2, all at (0, 0), every decider `arithmetic`, inventories `[(silver, 100)]`, `[(silver, 50)]`, `[(silver, 200)]`. Deposit owners by deposit ID (0-2 `arable_land` at the center, 3 `copper_ore` at the center, 4-9 elsewhere): `[Some(0), Some(0), None, Some(1), None, None, None, None, None, None]`.

Sample world at turn 0: `center_farm` holds `[(unimproved_grain, 8), (silver, 100)]` with groups `[(end(0), 8)]` and `[(Never, 100)]`; `center_mine` `[(unimproved_copper, 2.5), (silver, 50)]`; `center_toolmaker` `[(silver, 200)]`; the center hex `[(unimproved_grain, 4)]` with `[(end(0), 4)]`; (-1, 0) `[(unimproved_grain, 8)]`, (1, -1) `[(unimproved_copper, 5)]`, (2, -2) empty, as before. The report is unchanged (`emitted` 10 `unimproved_copper` and 24 `unimproved_grain`, `endowed` 4100 `unskilled_labor`, `perished` empty). After each step the businesses hold the same (the owned output at `end(t)`, the silver unchanged), and `perished` still holds 10 `unimproved_copper` and 24 `unimproved_grain` (owned output perishes from the business stockpiles), so the checker's turn lines don't change.

## Tasks

1. `businesses.rs`: domain types, `Businesses::from_parts`, accessors, unit tests. Export from `lib.rs`; crate docs.
2. `resources.rs`: `Deposit.owner`, `Deposits::owned_by`, docs, fixtures, unit test. `setting/resources.rs`: `owner: None`, `claim`, docs, unit test.
3. `reader.rs`: `integer_value`, `integer` refactored, `required_integer_table`; unit test.
4. `setting/businesses.rs`: `BusinessesReader`, `read_deposits` with its unit tests. `setting/mod.rs`: wiring and docs.
5. `world.rs`: fields, `new`, `emit`, `perish`, accessors, docs; the clock-maximum unit test extended.
6. `check.rs` success line; `checker.rs` expectations and the `businesses_one` test; `text.rs` test expectations.
7. Test settings (`businesses_valid`, `businesses_one`, `businesses_errors`, `resources_broken_file`), `tests/settings/README.md`; the sample's `start/businesses.toml`, `start/README.md`, and the ownership comments in `start/deposits.toml` and `rules/resources.toml`.
8. `crates/engine/tests/businesses.rs` (Test Plan); `tests/world.rs` and `tests/resources.rs` updates.
9. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the new success line and unchanged turn lines.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). Every amount in these tests is exactly representable, so compare with `assert_eq!`.

**Unit tests.**
- `businesses.rs`: `from_parts` gives sequential IDs in the order given, `id`/`name` round-trip, `get`, `iter` in order, `on_hex` in ID order and empty for a hex with none, the default list is empty; `#[should_panic]`: `get` on a foreign ID (`is not from this list`), `on_hex` on a hex beyond the map and on the default list (`is not from this list`), `from_parts` with a hex beyond `hex_count` (`is not from a map of 2 hexes`) and with mismatched lengths (`every business name needs exactly one entry`). Build IDs with throwaway `NameRegistry`s as `resources.rs`'s tests do.
- `resources.rs`: `owned_by` lists only the deposits with that owner, in ID order, and nothing for a business that owns none; a fresh `Deposit` fixture has `owner: None`.
- `setting/resources.rs`: `claim` on a reader holding three deposits (r0 on h0, r1 on h0, r0 on h0): claiming 1 r0 on h0 for b0 marks deposit 0 only; claiming 2 r0 on h0 for b1 returns `Err(1)` and changes nothing; claiming 1 r0 on h0 for b1 marks deposit 2; claiming 1 r0 on h1 returns `Err(0)`. (Build the reader with `DepositsReader { deposits, ..Default::default() }` inside the module.)
- `reader.rs` (the `run` helper): `required_integer_table` on `p = { a = 1, b = -2, c = 2.5, d = "x", e = 99999999999999999999 }`, `q = 7` and a missing key returns `a` and `b` as integers and the rest as `None`, and reports `key 'c' in p of thing must be an integer, found float`, `key 'd' in p of thing must be an integer, found string`, `key 'e' in p of thing is out of range`, `key 'q' in thing must be a table, found integer` and the missing-key error. The existing `required_integer` tests keep passing after the refactor.
- `setting/businesses.rs` (a `run` helper like `setting/resources.rs`'s, label `business`): `read_deposits` on `deposits = { a = 1, b = 0, c = -3, d = 4294967296, e = 2.5 }` returns `a` as 1 and the rest `None`, reporting `key 'b' in deposits of business must be at least 1`, `key 'c' ... must be at least 1`, `key 'd' ... is out of range`, `key 'e' ... must be an integer, found float`; a missing `deposits` gives an empty list and the missing-key error; the largest `u32` count is accepted.
- `world.rs`: the clock-maximum test also runs `businesses_valid` and asserts `business_stockpiles` unchanged.

**`crates/engine/tests/businesses.rs`** (`mod common;`). Helpers: `businesses_of(&LoadedSetting)` returning each business as (index, name, `HexCoord`, decider, inventory as `Vec<(good name, f64)>`); `owners_of(&LoadedSetting)` returning each deposit's owner index as `Vec<Option<usize>>` in deposit ID order; `owned(&LoadedSetting, index)` returning the deposit indexes of `owned_by`; `business_id(index, &World)` like `pop_id`; `business_totals(world, id)` via `stockpile_totals`.
- **AC "the sample loads with three businesses; owned and unowned deposits"**: load the sample; `businesses.len() == 3`; `businesses_of` equals the list above; `businesses.id("center_farm")` is ID 0 and `id("farm")` is `Err(IdError::UnknownName { kind: "business", name })`; `on_hex(center)` is `[0, 1, 2]` and `on_hex((2, -2))` is empty; `owners_of` equals the list above; `owned(0)` is `[0, 1]`, `owned(1)` is `[3]`, `owned(2)` is empty.
- **AC "a fresh sample world fills business stockpiles and emits to owners; stepping keeps it up; determinism"**: `world_of(sample)`; the three business stockpiles, the center hex and the other hexes as listed under "Sample setting"; the turn 0 report as before. Step 3 times: each turn `center_farm` holds `[(unimproved_grain, 8), (silver, 100)]` with grain at `end(t)`, `center_mine` `[(unimproved_copper, 2.5), (silver, 50)]`, the toolmaker `[(silver, 200)]`, the center hex `[(unimproved_grain, 4)]`, and `perished` (filtered of slivers below 1e-9, as `sample_setting_steps` does) is `[(unimproved_copper, 10), (unimproved_grain, 24)]`. Two sample worlds stepped 3 times give equal reports and equal business, hex and POP stockpiles after every step.
- **AC "businesses across files; two on one hex sharing a type; empty inventory and no deposits; no market; perishable and durable inventory; forward references"**: load `businesses_valid`; `businesses_of`, `on_hex`, `owners_of`, `owned` and `has_market` as listed; loading twice gives equal `businesses_of` and `owners_of`. Build the world and check turns 0 to 3 exactly as listed (totals and groups for `farm_a`, totals for `farm_b` and `idle`, both hex stockpiles, every report field).
- **AC "each error case, one entry with several problems"**: `failures_of("businesses_errors")` equals the expected list (the `Several` entry covers "several problems").
- **AC "references and assignment skipped after a broken file"**: the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing (add the comment).
- **AC "a setting with no businesses is valid"**: `valid_minimal` loads with `businesses.is_empty()`; the `turns_valid` world has `businesses().len() == 0`, every deposit `owner == None`, and its stockpiles and reports are as the `tests/world.rs` tests already assert.
- `business_stockpile` panics (`is not from this list`) on a business from another world (pattern of `pop_stockpile_panics_on_a_pop_from_another_world`).
- **AC "every existing test passes, with expectations changed only by the business count and the center hex's stockpile"**: `tests/world.rs` and the dev UI tests updated as listed in Changes; everything else unchanged.

**Dev UI**: `checker.rs` passes with the updated lines, including `1 business` for `businesses_one` and `0 businesses` elsewhere, and the sample's turn lines unchanged; `text.rs`'s two updated tests pass.

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`.
- `Businesses::from_parts`, `DepositsReader::claim`, `Bundle::new` and `NamedList::from_parts` stay `pub(crate)`. `Businesses`' fields stay private. `IdVec::push` stays `#[cfg(test)]`.
- Nothing changes a deposit's owner or a business's stockpile outside the loader, `World::new`, `emit` and `perish`.
- Do not change `TurnReport`'s fields, the turn lines, `Market`, `HexPrices`, the trade round, growth, POP code, the dev UI window or panels (beyond the two test expectations), or the checker's output beyond the business count. Do not change existing `EntryReader` message texts.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: factories and factory ownership, decisions and deciders (beyond storing the name), business trade, showing businesses in the dev UI, any change of ownership during play, businesses on several hexes, founding or failing businesses, POP ownership of businesses, starting inventories for hexes or POPs, validating decider names beyond their format.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line and unchanged turn lines, and every Acceptance Criterion in `design.md` is met.
