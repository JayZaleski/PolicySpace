# Step 05: natural-resources — Docs Changes

## docs/resources.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# Natural resources

Natural resources are the first data in a setting that refers to other data. A **resource type** is defined in `rules/` and names the good it produces. A **deposit** is placed in `start/` on a hex and names its resource type. The domain types are in `crates/engine/src/resources.rs`, amounts are `Quantity` values (`crates/engine/src/quantity.rs`), and the reading code is in `crates/engine/src/setting/resources.rs`.

Nothing is produced yet (the turn clock is Step 06), there is no ownership (Stage 02), and there is no stock, depletion, or regeneration (later). The engine does not require the produced good to be unmovable or to perish after one period; the setting decides.

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

An entry with `count = 3` places three separate deposits. They are identical at the start, each has its own `DepositId`, and in Stage 02 each can have a different single owner. A hex may hold deposits of several resource types, but each resource type may appear in at most one entry per hex (raise `count` instead). The `count` cap of 1,000,000 turns a typo into a clear error instead of an attempt to allocate billions of deposits.

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
- `Quantity` wraps an `f64` and is always finite and never negative. `Quantity::new(f64) -> Option<Quantity>` rejects negative, NaN, and infinite values and turns `-0.0` into `0.0`; `get()` returns the `f64`; `ZERO` and `Default` are zero; `Display` prints like `f64` (`4`, `2.5`). It derives `PartialEq` and `PartialOrd` but not `Eq`, `Ord`, or `Hash`. It has no arithmetic yet; Step 06 adds what it needs. Every system should use this type for amounts of goods, so the representation can be swapped later (for example to fixed decimal for cross-platform determinism) in one place.

`resources.rs` (depends on `goods`, `map`, `quantity`, `id`; knows nothing about files):
- `ResourceKind` / `ResourceId`: a named ID kind (label `"resource"`).
- `ResourceType { produces: GoodId, output: Quantity }`.
- `ResourceTypes`: a `NameRegistry<ResourceKind>` plus a `Vec<ResourceType>` in ID order. `id(name)`, `name(id)`, `get(id)`, `len`, `is_empty`, and `iter()` (ID, name, data, in load order). `get` panics with `is not from this list`; `name` panics with `is not from this registry`. `from_parts` is `pub(crate)` and panics if the lengths differ.
- `DepositKind` / `DepositId`: an unnamed ID kind (label `"deposit"`). `DepositId`s are valid only within the `Deposits` that produced them.
- `Deposit { resource: ResourceId, hex: HexId }`.
- `Deposits`: a `Vec<Deposit>` plus `by_hex`, a `Vec<Vec<DepositId>>` indexed by `HexId::index` (hex IDs are dense, so no hash map is needed). `get`, `len`, `is_empty`, `iter()`, `on_hex(hex) -> &[DepositId]`, and `of_type_on_hex(hex, resource)`; lists are in ID order. `on_hex` panics with `is not from this map` for a hex beyond the map the deposits were built with, so `Deposits::default()` (which has no hexes) panics for any hex. `from_deposits(deposits, hex_count)` is `pub(crate)`; it numbers deposits 0, 1, 2, ... in the order given and panics on a hex index `>= hex_count`.

The loaded setting exposes them as `Rules::resources: ResourceTypes` and `StartState::deposits: Deposits`; all are re-exported from the crate root.

## How resources are loaded

`setting/resources.rs` holds two crate-private readers, both registered as section types in `WorldContent` (`resource` in `rules/`, `deposit` in `start/`).

`ResourcesReader` (resource types are a named kind, so it uses `Definitions<ResourceKind>`, like goods):
- `read` reads all three keys first, then defines the name if it is valid. `output` is read with `EntryReader::required_number` and must be finite and greater than 0, otherwise `key 'output' in resource must be a finite number greater than 0` is reported at the value. `produces` is only read as a string; a bad value is simply an unknown good in phase 2. The name is defined even when another key is bad, so a later duplicate is still caught and a deposit that refers to it does not get a misleading unknown-resource error. Parallel vectors `produces` and `outputs` are pushed exactly when `define` returns an ID (a placeholder `Quantity::ZERO` stands in for an invalid output).
- `resolve` (phase 2) resolves each `produces` against the goods `Definitions` (`unknown good 'x'` at the `produces` value).
- `finish` builds `ResourceTypes::from_parts`. It `expect`s each `produces` to be resolved, which is an internal invariant: `finish` is only called after a successful load, when phase 2 has run.

`DepositsReader` (deposits have no names, so, like hexes, it keeps its own duplicate table):
- `read` reads `resource`, `q`, `r` (the `q`/`r` through `map::coordinate`, shared with hexes, whose message uses `EntryReader::label()`, so `key 'q' in deposit is out of range`), and `count` (`deposit_count`: absent gives 1; below 1 gives `must be at least 1`; above `MAX_DEPOSIT_COUNT` gives `is out of range (the most is 1000000)`). If `resource`, `q`, or `r` is missing or invalid, nothing is stored and no duplicate check runs. Otherwise a second entry for the same (resource name as written, coordinates) is reported at its header as `duplicate deposit of 'x' at (q, r) (first defined at ...)`. The `seen` table is lookup only and is never iterated. An invalid `count` stores 1 so phase 2 still checks the references.
- `resolve` (phase 2) resolves each entry's resource through the resource types' `Definitions`, and its hex through `MapReader::id` (`no hex at (9, 9)` at the `[[deposit]]` header, because the coordinates are two keys). If both resolve, it pushes `count` copies of `Deposit { resource, hex }`, so deposits are numbered in file order and, within an entry, consecutively. A running total guards against more than `u32::MAX` deposits (`too many deposits in total ...` at the header), so `from_deposits` can never panic on setting data.
- `finish(hex_count)` builds `Deposits::from_deposits`. `load_setting` builds the map first and passes `map.len()`.

References are checked in `WorldContent::resolve`, so they may point into any file: resources first against goods, then deposits against resource types and the map. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete.

`EntryReader` gained two methods for this Step: `label()` (the entry label used in messages) and `required_number(key)`, which accepts an integer or a float and returns an `f64`. It returns `inf`, `nan`, and negative values as written (range checks belong to the caller), reports an integer beyond `i64` as out of range, and reports other types as `must be a number, found <type>`. `MapReader` gained `id(coord) -> Option<HexId>`.

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

Output is per deposit; a hex's total is the sum over its deposits (the number of deposits times `output` for a single type).

## The sample setting and tests

- `settings/sample/rules/resources.toml`: `copper_ore` (produces `unimproved_copper`, output 2.5) and `arable_land` (produces `unimproved_grain`, output 4).
- `settings/sample/start/deposits.toml`: six entries placing 10 deposits on the 19-hex map, with IDs 0-2 `arable_land` at (0, 0), 3 `copper_ore` at (0, 0), 4-5 `arable_land` at (-1, 0), 6 `arable_land` at (1, 0), 7-8 `copper_ore` at (1, -1), 9 `copper_ore` at (-2, 1).
- Test settings under `crates/engine/tests/settings/`: `resources_valid` (several files and nested folders, references to goods and hexes defined in later files, two types producing one good, a type with no deposits), `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file` and `resources_missing_start` (reference errors must not appear after a broken file or an incomplete discovery).
- Integration tests: `crates/engine/tests/resources.rs`. Unit tests: `quantity.rs`, `resources.rs`, `setting/resources.rs` (`deposit_count`), and `setting/reader.rs` (`required_number`).

## Planned extensions

A resource type may later get an optional `stock` (how much a fresh deposit holds; leaving it out means it never depletes) and regeneration, both as optional data on the resource type. Per-deposit remaining stock will be world state. Step 06 will make each deposit emit its resource type's `output` of its good every turn; Stage 02 gives each deposit one owner.

## docs/setting-loader.md — modify (resource and deposit section types)
- **Location:** intro paragraph; "Section types and content" (`WorldContent` paragraph); "How loading works" step 5 (Phase 2); "Adding a section type"; module table; "The sample setting and test settings"; "Setting checker (dev UI)"; "EntryReader"
- **Change:** replace
- **Supersedes:** step-04-hex-map/docs_changes.md (the intro's "two real section types", the `WorldContent` paragraph, `WorldContent::resolve` doing nothing, the sample setting text, and the checker's success output); step-02-setting-loader/docs_changes.md (the "Remove the `#[allow(dead_code)]` markers" sentence for `Definitions::resolve`)

- Intro: replace "two real section types so far" with: The loader defines four real section types so far: `[[good]]` and `[[resource]]` in `rules/` (see [goods.md](goods.md) and [resources.md](resources.md)) and `[[hex]]` and `[[deposit]]` in `start/` (see [map.md](map.md) and [resources.md](resources.md)); later Steps add more.
- `WorldContent` paragraph: `WorldContent` holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, and a `DepositsReader`, and returns four section types: `good` and `resource` in `rules/`, `hex` and `deposit` in `start/`. `load_setting` builds `Rules { goods, resources }` and `StartState { map, deposits }` from it (the map first, because `DepositsReader::finish` needs the hex count).
- "How loading works" step 5 (Phase 2), add after the skip rules: `WorldContent::resolve` is where real references are checked: it resolves each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`). A reader's `finish` that depends on phase 2 (`ResourcesReader::finish`) is only called after a successful load.
- "Adding a section type", step 3: resources and deposits are the worked example for checking references: a name through `Definitions::resolve`, a hex through `MapReader::id`. Step 1: resource types (`Rules::resources`) and deposits (`StartState::deposits`) follow the goods pattern and the hexes pattern respectively; see [resources.md](resources.md).
- Remove the sentence about removing `#[allow(dead_code)]` from `Definitions::resolve`: it is now used and carries no allow. `Definitions::registry` and `Definitions::location` still do, as do some `EntryReader` accessors that only tests use.
- "EntryReader": add `label()` (the label used in messages) and `required_number(key)` (an integer or a float as `f64`; `inf` and `nan` are returned as written, so the caller does the range check).
- Module table: add row `resources.rs` | `ResourcesReader` and `DepositsReader`: read `[[resource]]` and `[[deposit]]` entries and build `ResourceTypes` and `Deposits`. Add to the `map.rs` row that its `coordinate` helper is shared with deposits and that `MapReader::id` resolves coordinates to a `HexId`.
- Sample setting: `settings/sample/rules/resources.toml` and `settings/sample/start/deposits.toml` define two resource types and 10 deposits (see [resources.md](resources.md)). Add `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file` and `resources_missing_start` to the list of test setting folders.
- Setting checker table, one-path success row: prints `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>`, for example `5 goods, 19 hexes, 2 resources, 10 deposits` or `1 good, 1 hex, 1 resource, 1 deposit`. The `count(n, "resource", "resources")` and `count(n, "deposit", "deposits")` helpers do the wording. Failure output, exit codes and the no-argument form are unchanged.

## docs/ids.md — modify
- **Location:** "Adding a new ID kind"
- **Change:** add
- **Supersedes:** none

Add: `ResourceKind` / `ResourceId` is a second named kind, and `DepositKind` / `DepositId` is a second unnamed kind. A deposit has no name or identifying data of its own (an entry's `count` makes several identical deposits), so `Deposits::from_deposits` simply numbers them in load order. Their IDs are valid only within the `Deposits` that produced them. See [resources.md](resources.md). The known follow-up for the `compile_fail` example (spec F1 of Step 04) is unchanged; `DepositKind` could replace the hex in that example.

## docs/workspace.md — modify
- **Location:** Layout block
- **Change:** add
- **Supersedes:** none

Under `crates/engine/` add `src/quantity.rs` (the `Quantity` type), `src/resources.rs` (resource types and deposits, see resources.md), and `tests/resources.rs`; under `src/setting/` add `resources.rs`.

## docs/README.md — modify
- **Location:** the table of documents, after the `map.md` row
- **Change:** add
- **Supersedes:** none

| [resources.md](resources.md) | Resource types and deposits, the `Quantity` type, how `[[resource]]` and `[[deposit]]` sections are loaded and their references checked |
