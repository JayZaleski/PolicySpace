# Natural resources

Natural resources are the first data in a setting that refers to other data. A **resource type** is defined in `rules/` and names the good it produces. A **deposit** is placed in `start/` on a hex and names its resource type. The domain types are in `crates/engine/src/resources.rs`, amounts are `Quantity` values (`crates/engine/src/quantity.rs`), and the reading code is in `crates/engine/src/setting/resources.rs`.

Each deposit emits its resource type's `output` of its good at the start of every turn (see [world.md](world.md)): into its owner's stockpile if a business owns it, and into its hex's stockpile of unowned goods otherwise. Ownership is written on the business, not on the deposit (see [businesses.md](businesses.md)). There is no deposit stock, depletion, or regeneration (later). The engine does not require the produced good to be unmovable or to perish after one period; the setting decides.

## Setting format

Resource types (`[[resource]]`, `rules/` only). Each has exactly three required keys:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique across the setting. Resource type names and good names are separate namespaces. |
| `produces` | The name of the one good a deposit produces. Must be a good the setting defines. |
| `output` | How much of that good one deposit produces per period. A whole number or a decimal, finite and greater than 0. |

Deposits (`[[deposit]]`, `start/` only). Keys:

| Key | Meaning |
|---|---|
| `resource` | Required. The name of a resource type. |
| `q`, `r` | Required. Axial coordinates, read and range-checked exactly as for `[[hex]]` (see [map.md](map.md)). A hex must exist there. |
| `count` | Optional, default 1. A whole number from 1 to 1,000,000. |

An entry with `count = 3` places three separate deposits. They are identical at the start, each has its own `DepositId`, and each can have a different single owner: a `[[business]]` claims deposits by resource type and count on its hex, and the loader hands them out in deposit ID order (see [businesses.md](businesses.md)). A hex may hold deposits of several resource types, but each resource type may appear in at most one entry per hex (raise `count` instead). The `count` cap of 1,000,000 turns a typo into a clear error instead of an attempt to allocate billions of deposits. `[[factory]]` uses the same `count` rule and cap (see [factories.md](factories.md)).

Example:

```toml
# rules/resources.toml
[[resource]]
name = "copper_ore"
produces = "unimproved_copper"
output = 2.5

# start/deposits.toml
[[deposit]]
resource = "copper_ore"
q = 1
r = -1
count = 2
```

## Types

`quantity.rs`:
- `Quantity` wraps an `f64` and is always finite and never negative. `Quantity::new(f64) -> Option<Quantity>` rejects negative, NaN, and infinite values and turns `-0.0` into `0.0`; `get()` returns the `f64`; `ZERO` and `Default` are zero, and `ONE` is one; `Display` prints like `f64` (`4`, `2.5`). It derives `PartialEq` and `PartialOrd` but not `Eq`, `Ord`, or `Hash`. Its arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result stays finite, `checked_sub`, which returns `None` instead of a negative result, and `checked_div`, which returns `None` when dividing by zero and clamps at `f64::MAX`. Arithmetic is added only when a system needs it. Every system should use this type for amounts of goods, so the representation can be swapped later (for example to fixed decimal for cross-platform determinism) in one place.

`resources.rs` (depends on `businesses` (for `BusinessId` and `BusinessKind`), `goods`, `map`, `quantity`, `id`; knows nothing about files):
- `ResourceKind` / `ResourceId`: a named ID kind (label `"resource"`).
- `ResourceType { produces: GoodId, output: Quantity }`.
- `ResourceTypes`: `NamedList<ResourceKind, ResourceType>` (see [ids.md](ids.md)), with `id(name)`, `name(id)`, `get(id)`, `len`, `is_empty`, and `iter()` (ID, name, data, in load order). `get` panics with `is not from this list`; `name` panics with `is not from this registry`. `from_parts` is `pub(crate)` and panics if the lengths differ.
- `DepositKind` / `DepositId`: an unnamed ID kind (label `"deposit"`). `DepositId`s are valid only within the `Deposits` that produced them.
- `Deposit { resource: ResourceId, hex: HexId, owner: Option<BusinessId> }`. `owner` is the business that owns the deposit, if any. Only the loader sets it, and nothing changes it during play yet.
- `Deposits`: an `IdVec<DepositKind, Deposit>` plus two indexes built once at load: `by_hex`, an `IdVec<HexKind, Vec<DepositId>>` indexed by `HexId` (hex IDs are dense, so no hash map is needed), and `by_owner`, an `IdVec<BusinessKind, Vec<DepositId>>` indexed by `BusinessId` (a deposit with no owner is in no owner's list). `get`, `len`, `is_empty`, `iter()`, `on_hex(hex) -> &[DepositId]`, `of_type_on_hex(hex, resource)`, and `owned_by(business)`, an iterator over the deposits that business owns; lists are in ID order. `get` panics with `is not from this list`; `on_hex` panics the same way for a hex beyond the map the deposits were built with, and `owned_by` for a business beyond the businesses they were built with, so `Deposits::default()` (which has no hexes and no businesses) panics for any hex or business. `from_deposits(deposits, hex_count, business_count)` is `pub(crate)`; it numbers deposits 0, 1, 2, ... in the order given and panics (`is not from a map of N hexes`, `is not from a list of N businesses`) on a hex index `>= hex_count` or an owner index `>= business_count`. `Pops`, `Businesses` and `Factories` (see [pops.md](pops.md), [businesses.md](businesses.md) and [factories.md](factories.md)) mirror this shape: a list in load order plus a per-hex index, and for factories a per-owner index too.

The loaded setting exposes them as `Rules::resources: ResourceTypes` and `StartState::deposits: Deposits`; all are re-exported from the crate root.

## How resources are loaded

`setting/resources.rs` holds two crate-private readers, both registered as section types in `WorldContent` (`resource` in `rules/`, `deposit` in `start/`).

`ResourcesReader` (resource types are a named kind, so it uses `Definitions<ResourceKind>`, like goods):
- `read` reads all three keys first, then defines the name if it is valid. `output` is read with `EntryReader::required_number` and checked with `EntryReader::positive_quantity("key 'output' in resource", ..)`, which reports `key 'output' in resource must be a finite number greater than 0` at the value otherwise (see [setting-loader.md](setting-loader.md#entryreader)). `produces` is only read as a string; a bad value is simply an unknown good in phase 2. The name is defined even when another key is bad, so a later duplicate is still caught and a deposit that refers to it does not get a misleading unknown-resource error. Parallel vectors `produces` and `outputs` are pushed exactly when `define` returns an ID (a placeholder `Quantity::ZERO` stands in for an invalid output).
- `resolve` (phase 2) resolves each `produces` against the goods `Definitions` (`unknown good 'x'` at the `produces` value).
- `finish` builds `ResourceTypes::from_parts`. It `expect`s each `produces` to be resolved, which is an internal invariant: `finish` is only called after a successful load, when phase 2 has run.

`DepositsReader` (deposits have no names, so, like hexes, it keeps its own duplicate table):
- `read` reads `resource`, `q`, `r` (the `q`/`r` through `map::coordinate`, shared with hexes, markets, POPs, businesses and factories, whose message uses `EntryReader::label()`, so `key 'q' in deposit is out of range`), and `count` through `map::count`, shared with factories (absent gives 1; not an integer gives the reader's `must be an integer` error; below 1 gives `key 'count' in deposit must be at least 1`; above `MAX_COUNT` gives `key 'count' in deposit is out of range (the most is 1000000)`; each error gives None; see [factories.md](factories.md#the-shared-count-helper)). If `resource`, `q`, or `r` is missing or invalid, nothing is stored and no duplicate check runs. Otherwise a second entry for the same (resource name as written, coordinates) is reported at its header as `duplicate deposit of 'x' at (q, r) (first defined at ...)`. The `seen` table is lookup only and is never iterated. When `count` gives None the reader stores 1, so phase 2 still checks the references (the load fails anyway).
- `resolve` (phase 2) resolves each entry's resource through the resource types' `Definitions`, and its hex through `MapReader::id` (`no hex at (9, 9)` at the `[[deposit]]` header, because the coordinates are two keys). If both resolve, it pushes `count` copies of `Deposit { resource, hex, owner: None }`, so deposits are numbered in file order and, within an entry, consecutively. A running total guards against more than `u32::MAX` deposits (`too many deposits in total ...` at the header), so `from_deposits` can never panic on setting data.
- `claim(hex, resource, count, owner) -> Result<(), u32>` (phase 2, after `resolve`) gives the first `count` unowned deposits of `resource` on `hex`, in ID order, to `owner`, or, if fewer than `count` are unowned, assigns none and returns how many were available. `BusinessesReader::resolve` calls it for each business in load order and reports a shortfall (see [businesses.md](businesses.md#how-businesses-are-loaded)).
- `finish(hex_count, business_count)` builds `Deposits::from_deposits`. `load_setting` builds the map and the businesses first and passes `map.len()` and `businesses.len()`.

References are checked in `WorldContent::resolve`, so they may point into any file: hex currencies and resource types against goods, then deposits against resource types and the map, and later the businesses, which claim their deposits. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)). `resources_broken_file` also holds a hex with an unknown currency, a market that prices goods that don't exist, a POP type whose endowment and utility name goods that don't exist, a technology whose goods don't exist, a POP of an unknown type at coordinates with no hex, a business whose good and resource type don't exist on a hex that doesn't exist, and a factory of an unknown technology owned by an unknown business on a hex that doesn't exist, which are likewise not reported (and no deposit is claimed). `[[market]]` follows `[[deposit]]` as the pattern for an entry identified by coordinates (see [market.md](market.md)); `[[pop_type]]` follows `[[resource]]` as a named `rules/` kind with good references, and `[[pop]]` follows `[[deposit]]` as a placed thing with a type reference, coordinates and a per-hex index (see [pops.md](pops.md)).

## Using resources

```rust
let setting = load_setting(path)?;
let types = &setting.rules.resources;
let copper = types.id("copper_ore")?;
let good = types.get(copper).produces;      // GoodId
let output = types.get(copper).output.get(); // f64, per deposit per period
for (id, deposit) in setting.start.deposits.iter() { /* load order */ }
let here = setting.start.deposits.on_hex(hex);
```

Output is per deposit; a hex's total is the sum over its deposits (the number of deposits times `output` for a single type). Output is emitted per deposit per turn, in deposit ID order, so a hex's stockpile holds the sum over its unowned deposits, and a business's stockpile the sum over the deposits it owns (`deposits.owned_by(business)`).

## The sample setting and tests

- `settings/sample/rules/resources.toml`: `copper_ore` (produces `unimproved_copper`, output 2.5) and `arable_land` (produces `unimproved_grain`, output 4).
- `settings/sample/start/deposits.toml`: six entries placing 10 deposits on the 19-hex map, with IDs 0-2 `arable_land` at (0, 0), 3 `copper_ore` at (0, 0), 4-5 `arable_land` at (-1, 0), 6 `arable_land` at (1, 0), 7-8 `copper_ore` at (1, -1), 9 `copper_ore` at (-2, 1). Deposits 0 and 1 are owned by `center_farm` and deposit 3 by `center_mine` (see [businesses.md](businesses.md)); deposit 2 and every deposit off the center are unowned, so the center hex's stockpile receives 4 `unimproved_grain` a turn.
- Test settings under `crates/engine/tests/settings/`: `resources_valid` (several files and nested folders, references to goods and hexes defined in later files, two types producing one good, a type with no deposits), `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file` and `resources_missing_start` (reference errors must not appear after a broken file or an incomplete discovery).
- Integration tests: `crates/engine/tests/resources.rs`; deposit ownership in `crates/engine/tests/businesses.rs`. Unit tests: `quantity.rs`, `resources.rs` (the `Deposits` behavior, including `owned_by` and the `from_deposits` owner panic), `setting/resources.rs` (the shared `count` helper from `setting/map.rs`, run on a `deposit` entry, and `claim`), and `setting/reader.rs` (`required_number`, `positive_quantity`, `at_least_one`). The `ResourceTypes` list behavior is covered by the `NamedList` tests in `id.rs`.

## Planned extensions

A resource type may later get an optional `stock` (how much a fresh deposit holds; leaving it out means it never depletes) and regeneration, both as optional data on the resource type. Per-deposit remaining stock will be world state. Each deposit already emits its resource type's `output` every turn (see [world.md](world.md)), with no depletion. Ownership exists (see [businesses.md](businesses.md)); changing a deposit's owner during play is later.
