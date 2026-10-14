# Step 01: currency — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-01-currency/design.md` — the behavior this spec implements.
- `docs/setting-loader.md` — the loader's two phases, `EntryReader`, `Definitions`, error order, and "Adding a section type".
- `docs/map.md` — how `[[hex]]` entries are read today, and "Adding a hex attribute" (this spec deliberately does **not** follow it: the currency is stored next to the map, not in it).
- `docs/world.md` — how `World` holds per-hex state (`hex_stockpiles`) and exposes it.
- `docs/resources.md` — `Deposits` is the pattern for a public type that wraps a per-hex `IdVec`; `ResourcesReader` is the pattern for a reference resolved in phase 2.
- `docs/workspace.md` — the check command and test layout.
- Code: `crates/engine/src/setting/map.rs` (`MapReader`), `crates/engine/src/setting/resources.rs` (`ResourcesReader::read`/`resolve`/`finish`, `DepositsReader`), `crates/engine/src/setting/mod.rs` (`WorldContent`, `load_setting`, `StartState`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/resources.rs` (`Deposits`), `crates/engine/src/id.rs` (`IdVec`), `crates/engine/src/world.rs`, `crates/engine/src/lib.rs`, `crates/engine/tests/map.rs`, `crates/engine/tests/world.rs`, `crates/engine/tests/common/mod.rs`, `crates/dev-ui/tests/checker.rs`, `crates/dev-ui/src/session.rs` (its unit tests write temporary settings with hexes).

## Approach

- **The `currency` key is read as part of each `[[hex]]` entry** by `MapReader`, because it is a per-hex value in `start/`. The value is read with `required_string` and kept unresolved (`Located<String>`), exactly like a resource type's `produces`: no name-syntax check in phase 1, so a badly spelled value is simply an unknown good in phase 2.
- **The map stays geometry.** `HexMap` and `HexCoord` do not change. The currencies go into a new public type `HexCurrencies` (one `GoodId` per hex, by `HexId`) in a new module `crates/engine/src/currency.rs`, built only by the loader. Rationale: the designation changes during play (later Stages) while the map does not, and the World already keeps per-hex state in parallel tables.
- **`StartState` gains `currencies: HexCurrencies`**, and `World` takes it over as a field with read-only accessors, next to `hex_stockpiles`. Nothing changes it.
- **Phase 2 resolves each placed hex's currency** through `Definitions::<GoodKind>::resolve` (error text `unknown good '<name>'` at the value). A hex whose coordinates are invalid is not placed (as today) and its currency is not resolved, matching how an unplaced deposit's references are not checked. A hex whose currency is missing or not a string **is still placed**, so duplicates and deposit references keep working; the load fails anyway.
- **Every test setting with hexes gains a currency** (and a good where it had none), and the sample setting gains the good `silver`, designated on all 19 hexes.
- **No dev UI code changes.** The checker's success line and the window are unchanged. Only test fixtures in the dev UI change (temporary settings written by `session.rs` tests, and the expected line for `hexes_one`).

## Changes

### New

- `crates/engine/src/currency.rs` — `HexCurrencies` and its unit tests.
- `crates/engine/tests/currency.rs` — integration tests (see Test Plan).
- `crates/engine/tests/settings/hexes_valid/rules/goods.toml`, `crates/engine/tests/settings/hexes_one/rules/goods.toml`, `crates/engine/tests/settings/hexes_errors/rules/goods.toml` — the goods these settings' hexes designate.

### Modified

- `crates/engine/src/lib.rs` — declare `pub mod currency;` and `pub use currency::HexCurrencies;`, placed before `goods` to keep the modules in alphabetical order. Add a sentence to the crate docs: each hex designates a currency good (see the `currency` module).
- `crates/engine/src/setting/map.rs` — `MapReader` reads, stores, resolves and finishes currencies (see Interfaces & Data). Module docs: say it reads the `currency` key and builds `HexCurrencies` too.
- `crates/engine/src/setting/mod.rs` — `StartState.currencies`; `WorldContent::resolve` calls `self.map.resolve(self.goods.defs(), sink)`; `load_setting` destructures `content.map.finish()` into the map and the currencies. Module docs: phase 2 also resolves each hex's currency against the goods; the "Adding a section type" text is unchanged.
- `crates/engine/src/map.rs` — module docs only: a hex entry also names its currency, which is read by the map reader but stored in `crate::currency::HexCurrencies`, not in the map.
- `crates/engine/src/world.rs` — field `hex_currencies: HexCurrencies`, moved from `start.currencies` in `World::new`; accessors `hex_currencies()` and `hex_currency(hex)`. Module docs: the world also holds each hex's currency, unchanged for now.
- `crates/engine/tests/map.rs` — update `every_hex_error_is_reported_at_its_place` to the new expected lines (below).
- `crates/engine/tests/world.rs` — add the currency tests listed in the Test Plan.
- `crates/engine/tests/settings/README.md` — describe the new goods/currencies in `hexes_valid`, `hexes_one`, `hexes_errors`, and the hex added to `resources_broken_file`.
- Test settings (exact contents in Interfaces & Data): `hexes_valid/start/{a.toml,a/z.toml,b.toml}`, `hexes_one/start/map.toml`, `hexes_errors/start/{a.toml,b.toml}`, `deposits_errors/start/h.toml`, `resources_one/start/s.toml`, `resources_valid/start/hexes.toml`, `turns_valid/start/map.toml`, `resources_broken_file/start/s.toml`.
- `settings/sample/rules/goods.toml` — add `silver`; `settings/sample/start/map.toml` — `currency = "silver"` on every hex, and a header comment.
- `crates/engine/tests/goods.rs` — the sample now has six goods: the sample-goods test expects `silver` as the sixth (never perishes, movable) and is renamed accordingly.
- `crates/dev-ui/src/text.rs` (tests only) — the good-rows test expects `silver` as the sixth name with `0` held.
- `crates/dev-ui/tests/checker.rs` — `says_1_hex_for_one_hex` expects `1 good, 1 hex, 0 resources, 0 deposits`; `checks_the_sample_setting` and `SAMPLE_LINE` expect `6 goods, 19 hexes, 2 resources, 10 deposits`.
- `crates/dev-ui/src/session.rs` (tests only) — the temporary settings must stay valid: add a `rules/goods.toml` with one good `coin` wherever a `rules/README.md` is written next to a hex map, and add `currency = "coin"` to every hex in `ONE_HEX_MAP` and `TWO_HEX_MAP`.
- `crates/engine/src/setting/test_content.rs` — only if any test there writes a `[[hex]]`; otherwise untouched.

## Interfaces & Data

### `crates/engine/src/currency.rs`

```rust
//! Module docs: each hex designates one good as its currency, the good prices on that hex
//! are posted in. The designation is starting state (it can change during play, so it is
//! not a rule), one per hex, in hex ID order. The engine has no opinion about which good
//! may be a currency. Nothing changes a currency yet. This module knows nothing about
//! files: the loader builds `HexCurrencies` from a checked setting.

use crate::goods::GoodId;
use crate::id::IdVec;
use crate::map::{HexId, HexKind};

/// The currency good of every hex, by `HexId`.
#[derive(Debug, Default)]
pub struct HexCurrencies {
    by_hex: IdVec<HexKind, GoodId>,
}

impl HexCurrencies {
    /// One entry per hex, in hex ID order. Crate-private: only the loader builds this.
    /// Panics if there are more than u32::MAX entries (via IdVec::from_vec).
    pub(crate) fn from_vec(currencies: Vec<GoodId>) -> Self;
    /// Number of hexes.
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// The currency of `hex`. Panics (`is not from this list`, from IdVec::get) if `hex`
    /// isn't from the map these were built with.
    pub fn get(&self, hex: HexId) -> GoodId;
    /// Every hex's currency as (hex, good), in hex ID order.
    pub fn iter(&self) -> impl Iterator<Item = (HexId, GoodId)> + '_;
}
```

`HexCurrencies::default()` is empty (no hexes). Doc comments on every public item (clippy runs with `-D warnings`).

### `MapReader` (`crates/engine/src/setting/map.rs`)

Add two fields, both indexed like `coords`:

```rust
/// Each placed hex's currency as written. None if the key was missing or not a string.
currencies: Vec<Option<Located<String>>>,
/// Filled in phase 2, indexed like `coords`.
currency_ids: Vec<Option<GoodId>>,
```

`read` changes:

1. Read `q`, `r` **and** `currency` (`reader.required_string("currency")`) before any early return, so none is falsely reported as unknown. A missing key reports `missing required key 'currency' in hex` at the header; a non-string reports `key 'currency' in hex must be a string, found <type>` at the value (both come from `EntryReader`; do not add messages).
2. The existing early return on invalid coordinates and the duplicate check are unchanged. When the hex is placed, push the currency (`Some` or `None`) alongside `coords` and `locations`.

New methods:

```rust
/// Phase 2: resolves each placed hex's currency against the goods. An unknown name is
/// reported at the value (`unknown good '<name>'`) by `Definitions::resolve`.
pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink);
/// The finished map and currencies. Only call after a successful load, when phase 2 has
/// resolved every currency (expect with an "internal error: ..." message, like
/// `ResourcesReader::finish`).
pub fn finish(self) -> (HexMap, HexCurrencies);
```

`resolve` mirrors `ResourcesReader::resolve`: map each `Option<Located<String>>` through `goods.resolve` and collect into `currency_ids`. `MapReader::id` is unchanged.

### `StartState` and `load_setting` (`crates/engine/src/setting/mod.rs`)

```rust
pub struct StartState {
    pub map: HexMap,
    /// Each hex's currency good, by hex ID (one per hex in `map`).
    pub currencies: HexCurrencies,
    pub deposits: Deposits,
}
```

`WorldContent::resolve` becomes: `self.map.resolve(self.goods.defs(), sink); self.resources.resolve(...); self.deposits.resolve(...)`. Order does not affect the error list (it is sorted). `load_setting`: `let (map, currencies) = content.map.finish();` then build `StartState { map, currencies, deposits }`.

### `World` (`crates/engine/src/world.rs`)

```rust
/// One per hex, by `HexId`. Nothing changes it yet.
hex_currencies: HexCurrencies,

/// Every hex's currency.
pub fn hex_currencies(&self) -> &HexCurrencies;
/// The currency good of `hex`.
/// # Panics
/// Panics if `hex` isn't from this world's map.
pub fn hex_currency(&self, hex: HexId) -> GoodId;
```

`World::new` moves `start.currencies` into the field. `emit`, `perish`, `step` and `TurnReport` are unchanged.

### Setting file format

A `[[hex]]` in `start/` has three required keys: `q`, `r`, `currency`. `currency` is a string naming a good defined in `rules/`. Any other key is unknown.

### Test setting contents

Write these files exactly (line numbers matter for the expected error lines). Each setting's `setting.toml` is unchanged unless listed.

**`hexes_valid/rules/goods.toml`** (new):
```toml
[[good]]
name = "coin"

[[good]]
name = "shell"
```
**`hexes_valid/start/a.toml`**: hex (0, 0) `currency = "coin"`, then hex (1, -1) `currency = "shell"`.
**`hexes_valid/start/a/z.toml`**: hex (-3, 2) `currency = "coin"`.
**`hexes_valid/start/b.toml`**: hex (2147483647, -2147483648) `currency = "shell"`.
(Load order of goods: `coin` = ID 0, `shell` = ID 1. Hex IDs 0..3 have currencies coin, shell, coin, shell.)

**`hexes_one/rules/goods.toml`** (new): one good `coin`. **`hexes_one/start/map.toml`**: hex (0, 0) `currency = "coin"`.

**`hexes_errors/rules/goods.toml`** (new): one good `coin`. (`rules/h.toml`, the `[[hex]]` in `rules/`, is unchanged: its contents are not read, so it needs no currency.)

**`hexes_errors/start/a.toml`** (replace the whole file with exactly this, 63 lines):
```toml
[[hex]]
q = 0
r = 0
currency = "coin"

[[hex]]
r = 1
currency = "coin"

[[hex]]
q = 0
currency = "coin"

[[hex]]
q = 1.5
r = 0
currency = "coin"

[[hex]]
q = "2"
r = 0
currency = "coin"

[[hex]]
q = 2147483648
r = 0
currency = "coin"

[[hex]]
q = 0
r = -2147483649
currency = "coin"

[[hex]]
q = 3
r = 0
currency = "coin"
terrain = "hills"

[[hex]]
q = 4
r = 0

[[hex]]
q = 5
r = 0
currency = 7

[[hex]]
q = 6
r = 0
currency = "Coin"

[[hex]]
q = 7
r = 0
currency = "tin"

[[hex]]
q = 1.5
r = 9999999999
currency = "tin"
terrain = "x"
```
**`hexes_errors/start/b.toml`**: hex (0, 0) with `currency = "coin"` as its 4th line (the duplicate; its error stays at `1:1`).

Expected error lines for `hexes_errors` (replace the list in `crates/engine/tests/map.rs`):
```
rules/h.toml:1:3: section 'hex' belongs in start/, not rules/
start/a.toml:6:1: missing required key 'q' in hex
start/a.toml:10:1: missing required key 'r' in hex
start/a.toml:15:5: key 'q' in hex must be an integer, found float
start/a.toml:20:5: key 'q' in hex must be an integer, found string
start/a.toml:25:5: key 'q' in hex is out of range
start/a.toml:31:5: key 'r' in hex is out of range
start/a.toml:38:1: unknown key 'terrain' in hex
start/a.toml:40:1: missing required key 'currency' in hex
start/a.toml:47:12: key 'currency' in hex must be a string, found integer
start/a.toml:52:12: unknown good 'Coin'
start/a.toml:57:12: unknown good 'tin'
start/a.toml:60:5: key 'q' in hex must be an integer, found float
start/a.toml:61:5: key 'r' in hex is out of range
start/a.toml:63:1: unknown key 'terrain' in hex
start/b.toml:1:1: duplicate hex at (0, 0) (first defined at start/a.toml:1:1)
```
Note the last `[[hex]]` (line 59) has invalid coordinates, so it is not placed and its `currency = "tin"` is **not** reported: that is the intended behavior. If the actual output differs from this list, check the implementation against the Approach before changing the list; the `EntryReader` message texts are fixed.

**Other settings** — add `currency = "<good>"` to every `[[hex]]`:
- `deposits_errors/start/h.toml`: all ten hexes `"ore"`.
- `resources_one/start/s.toml`: `"ore"`.
- `resources_valid/start/hexes.toml`: all three `"grain"`.
- `turns_valid/start/map.toml`: all three `"gold"`.
- `resources_broken_file/start/s.toml`: append a `[[hex]]` at (0, 0) with `currency = "nothing"`. The expected errors for this setting are unchanged (only the syntax error), which shows the currency check is skipped when a file is broken.

**Sample setting**:
- `settings/sample/rules/goods.toml`: append a sixth good after `grain`, with a comment in the file's style: `silver`, durable and movable (only `name = "silver"`), noting it is the currency every hex designates (see `start/map.toml`).
- `settings/sample/start/map.toml`: extend the header comment: every hex also needs `currency`, the name of a good from `rules/`, the good that prices on that hex are posted in; different hexes may name different goods, and this map uses `silver` everywhere. Add `currency = "silver"` to all 19 hexes. The dev-ui checker line becomes `Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits`.

## Tasks

1. Create `crates/engine/src/currency.rs` with `HexCurrencies` and unit tests; export it from `lib.rs`.
2. Extend `MapReader` in `setting/map.rs`: read `currency`, store it, add `resolve`, change `finish` to return `(HexMap, HexCurrencies)`. Update the module docs.
3. Update `setting/mod.rs`: `StartState.currencies`, `WorldContent::resolve`, `load_setting`, module docs. Fix anything else that calls `MapReader::finish`.
4. Update `world.rs`: the field, `World::new`, the two accessors, module docs. Update `map.rs` module docs.
5. Update the test settings and `crates/engine/tests/settings/README.md` as listed. Update the sample setting.
6. Update the expected error lines in `crates/engine/tests/map.rs`; update `crates/dev-ui/tests/checker.rs` expectations; fix the `session.rs` unit-test fixtures.
7. Write `crates/engine/tests/currency.rs` and the additions to `crates/engine/tests/world.rs` (Test Plan).
8. Run `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests) until it passes. Also run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the success line and that the turn lines are unchanged.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`).

Unit tests in `currency.rs`: `from_vec` gives IDs 0, 1, 2… in order; `get` and `iter` return them; `default` is empty; `get` with a foreign hex panics with `is not from this list` (`#[should_panic(expected = "is not from this list")]`).

`crates/engine/tests/currency.rs` (use `mod common;` and its helpers):
- **AC "sample loads, every hex silver":** load the sample; `start.currencies.len() == 19`; every `(hex, good)` from `iter()` has `good == goods.id("silver")`; `get(hex)` agrees with `iter()` for every hex of `start.map`.
- **AC "different currencies per hex":** load `hexes_valid`; currencies by hex ID 0..3 are `coin, shell, coin, shell`; `is_empty()` is false.
- **AC "no hexes":** `valid_minimal` loads with `start.currencies.is_empty()` and `len() == 0`.
- Loading `hexes_valid` twice gives the same currency list (determinism).
- **AC "each error at its place":** covered by the updated `every_hex_error_is_reported_at_its_place` in `tests/map.rs` (the four new cases are the missing key, the integer value, `'Coin'`, and `'tin'`).
- **AC "check skipped when a file is broken":** `failures_of("resources_broken_file")` is still exactly the one syntax error (this assertion already exists in `tests/resources.rs`; keep it passing, and add a comment there that the setting now also holds a hex with an unknown currency).

`crates/engine/tests/world.rs` additions:
- **AC "readable from a World, unchanged by stepping":** build the world from `turns_valid`; `hex_currency` of each of the three hexes is `gold`; `hex_currencies().len() == 3`; step 3 times; the currencies are identical. Also the sample: all 19 are `silver` after a step.
- `hex_currency` with a hex from another world's map panics with `is not from this list` (follow the existing foreign-hex panic test for `hex_stockpile`).

Dev UI: the existing `checker.rs` and `session.rs` tests pass with the updated expectations and fixtures. No new dev UI tests.

## Constraints

- Follow the existing conventions: hand-written readers with every key read before any early return; message texts come from `EntryReader` and `Definitions`, do not invent new ones; domain modules know nothing about files; no `HashMap` iteration; doc comments on every public item; `#[cfg(test)]` rather than `#[allow(dead_code)]`.
- `IdVec` stays `pub(crate)`; `HexCurrencies` is the public face.
- Do not change `HexMap`, `HexCoord`, `Stockpile`, the turn, `TurnReport`, the dev UI's code (only its tests' expectations and fixtures), or the checker's output format.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Keep the message texts in the Interfaces section exact; tests assert on them.
- Out of scope: changing a currency during play, starting stockpiles, prices, any restriction on which goods may be currencies, a default currency, showing the currency in the dev UI.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line, and every Acceptance Criterion in `design.md` is met.
