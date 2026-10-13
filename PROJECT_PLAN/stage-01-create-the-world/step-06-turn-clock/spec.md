# Step 06: turn-clock — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-06-turn-clock/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-05-natural-resources/spec.md` and `docs_changes.md`: `Quantity`, `ResourceTypes`, `Deposits`, and the conventions for domain modules (module docs, crate-private constructors, panics on foreign IDs, test layout).
- `PROJECT_PLAN/stage-01-create-the-world/step-03-goods-and-properties/docs_changes.md`: `Goods`, `Good`, `Perishes` and the meaning of `perishes_after`.
- Code this Step uses or extends:
  - `crates/engine/src/lib.rs`, `crates/engine/src/quantity.rs`, `crates/engine/src/resources.rs` (module docs only)
  - read only: `crates/engine/src/goods.rs`, `crates/engine/src/map.rs`, `crates/engine/src/id.rs`, `crates/engine/src/setting/mod.rs` (`LoadedSetting`, `Manifest`, `Rules`, `StartState`)
  - `crates/engine/tests/resources.rs` (helper style for integration tests), `crates/engine/tests/settings/README.md`
  - `crates/dev-ui/src/main.rs`, `crates/dev-ui/tests/checker.rs`
  - `settings/sample/` (read only: its goods, resource types and deposits give the expected numbers)
- `Cargo.toml` (workspace): `missing_docs = "warn"`, clippy `all`, and `tools/check.py` runs clippy with `-D warnings`. Every public item and public field needs a doc comment.
- `docs/` holds nothing yet for this Stage (it is written at the end of the Stage); the Step folders above are the reference.

## Approach
- **Three new domain modules**, none of which knows about files:
  - `turn`: the `Turn` number. A newtype over `u64`, so the clock can't silently wrap and turn numbers can't be mixed up with other integers.
  - `stockpile`: a general `Stockpile` of goods. Each good's amount is grouped by when it perishes (`PerishesAt::EndOfTurn(t)` or `PerishesAt::Never`). It's a general type: hexes use it now, and POPs and industries reuse it later.
  - `world`: `World` (the running world), `TurnReport`, and `StepError`.
- **The world owns the setting.** `World::new` consumes a `LoadedSetting`, keeping the manifest and rules, and taking the map and deposits as world state, which later Stages will change. Owning the setting means callers (the dev UI) have no lifetimes to manage.
- **One stockpile of unowned goods per hex**, in a `Vec` indexed by `HexId::index`. Hex IDs are dense.
- **The stockpile is sparse and ordered**: a `BTreeMap<GoodId, BTreeMap<PerishesAt, Quantity>>`. It holds only goods with a positive amount, iterates in good ID order and then in perish order, and has no hash maps, so it's deterministic.
- **Changes are engine-only.** Adding to a stockpile and removing what perishes are `pub(crate)`. Callers can only read. Stage 02 adds taking goods out.
- **The turn:**
  - `World::new` puts the world at turn 0 and runs turn 0's emission.
  - `World::step` checks that the clock can advance, removes what perishes at the end of the current turn, advances the clock, and emits for the new turn.
  - Both return a `TurnReport`.
  - If the clock is at its maximum, the step is refused before anything changes.
- **Emission is per deposit, in deposit ID order.** Each deposit adds its type's `output` once. There's no multiplication by a count, so the order of floating-point additions is fixed and the result deterministic.
- **`Quantity` gains one method, `saturating_add`.** Two finite, non-negative values can sum to infinity in `f64`; this clamps at `f64::MAX`, so a `Quantity` stays finite without panicking on setting-controlled data. In practice the limit is unreachable.
- **The dev UI's setting checker gains `--turns N`**, parsed by hand with no new crates.
- **One build, no packages.** `world` depends on `stockpile`, which depends on `turn` and `quantity`, and the dev UI depends on `world`. The work is a chain and too small to split.

## Changes
Create:
- `crates/engine/src/turn.rs`: `Turn`, unit tests.
- `crates/engine/src/stockpile.rs`: `PerishesAt`, `Stockpile`, unit tests.
- `crates/engine/src/world.rs`: `World`, `TurnReport`, `StepError`, unit tests (including the clock maximum).
- `crates/engine/tests/world.rs`: integration tests through the public API.
- Test setting `crates/engine/tests/settings/turns_valid/` (see Test Plan).

Modify:
- `crates/engine/src/lib.rs`: `pub mod stockpile; pub mod turn; pub mod world;`, re-exports, crate-docs sentences.
- `crates/engine/src/quantity.rs`: `saturating_add`, its unit tests; module docs.
- `crates/engine/src/resources.rs`: module docs only. Replace "Nothing is produced yet (Step 06)" with a sentence saying each deposit emits its type's `output` every turn into its hex's stockpile (see the `world` module).
- `crates/engine/tests/settings/README.md`: describe `turns_valid`.
- `crates/dev-ui/src/main.rs`: `--turns N`, the new usage message, crate docs.
- `crates/dev-ui/tests/checker.rs`: new tests and the updated usage expectation.

## Interfaces & Data

### `crates/engine/src/turn.rs` (public)
Module docs: the clock counts turns as whole numbers from 0. A world starts at turn 0 and a step advances it by one. `Turn` is a `u64` behind a type, so turn numbers can't be confused with other integers and the clock can't wrap.

```rust
/// A turn number. Turn 0 is the first turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Turn(u64);

impl Turn {
    /// Turn 0.
    pub const ZERO: Turn = Turn(0);
    /// The largest turn number. A world at this turn can't step.
    pub const MAX: Turn = Turn(u64::MAX);
    /// Turn `n`.
    pub const fn new(n: u64) -> Turn;
    /// The turn number.
    pub const fn get(self) -> u64;
    /// The next turn, or None at `MAX`.
    pub fn next(self) -> Option<Turn>;
    /// `self + n`, or `MAX` if that would overflow.
    pub fn saturating_add(self, n: u64) -> Turn;
}

/// Displays the number: `0`, `7`.
impl fmt::Display for Turn { ... }
```

### `crates/engine/src/quantity.rs`
```rust
impl Quantity {
    /// `self + other`, clamped to the largest finite `f64` so the result stays finite.
    pub fn saturating_add(self, other: Quantity) -> Quantity;
}
```
- Both values are finite and non-negative, so the sum is non-negative; only overflow to infinity needs the clamp (`(a + b).min(f64::MAX)`).
- Module docs: replace "Arithmetic is added when a system needs it" with: "Arithmetic is added when a system needs it; so far, only `saturating_add`."
- Add nothing else to `Quantity`.

### `crates/engine/src/stockpile.rs` (public)
Module docs must say:
- A stockpile holds amounts of goods. It is a general type: each hex has a stockpile of unowned goods (see `world`), and later Stages give POPs and industries their own.
- Each good's amount is split into groups by when it perishes, so `perishes_after` works without tracking individual units. A good with `perishes_after = N` added in turn T perishes at the end of turn T+N−1. A durable good's amount is one group that never perishes. Amounts of a good that perish at the same time are merged.
- A stockpile lists only goods it holds a positive amount of.
- Only the engine changes stockpiles. Goods enter by emission and leave by perishing; taking goods out to use them comes with Stage 02.

```rust
/// When a group of a good's units perishes.
/// Ordered soonest first: every EndOfTurn (by turn) before Never.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PerishesAt {
    /// Removed when this turn ends.
    EndOfTurn(Turn),
    /// Never perishes (a durable good).
    Never,
}

impl PerishesAt {
    /// When units of a good with `perishes` that are added in turn `added_in` perish:
    /// `Never` for `Perishes::Never`; `EndOfTurn(added_in + n - 1)` for `Perishes::After(n)`,
    /// saturating at `Turn::MAX` (a world at `Turn::MAX` can't step, so it never comes).
    pub fn for_good(perishes: Perishes, added_in: Turn) -> PerishesAt;
}

/// Amounts of goods, each grouped by when it perishes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stockpile {
    /// Only goods with at least one group; only groups with an amount > 0.
    goods: BTreeMap<GoodId, BTreeMap<PerishesAt, Quantity>>,
}

impl Stockpile {
    /// An empty stockpile.
    pub fn new() -> Stockpile;
    /// True if it holds nothing.
    pub fn is_empty(&self) -> bool;
    /// Every good held, with its total, in good ID order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_;
    /// The total of `good`; zero if it holds none.
    pub fn total(&self, good: GoodId) -> Quantity;
    /// `good`'s groups as (when it perishes, amount), soonest first; empty if it holds none.
    pub fn groups(&self, good: GoodId) -> impl Iterator<Item = (PerishesAt, Quantity)> + '_;

    /// Adds `amount` of `good` to the group that perishes at `at` (merging with it).
    /// A zero amount changes nothing. Crate-private.
    pub(crate) fn add(&mut self, good: GoodId, amount: Quantity, at: PerishesAt);
    /// Removes every group that perishes at the end of `turn` or earlier, drops goods left
    /// with no groups, and returns the total removed per good, in good ID order, listing
    /// only goods with an amount > 0. Crate-private.
    pub(crate) fn remove_perishing(&mut self, turn: Turn) -> Vec<(GoodId, Quantity)>;
}
```
- Totals are summed with `saturating_add` in group order (soonest first), so they're deterministic.
- `remove_perishing` removes `EndOfTurn(t)` for every `t <= turn`, not only `t == turn`. In normal play only `t == turn` exists. Removing earlier ones too means no group can ever be stranded.
- `GoodId` doesn't say which `Goods` list it came from. A stockpile simply holds whatever IDs it is given; the world only ever gives it IDs from its own rules.
- `Stockpile` derives `PartialEq` (`Quantity` is `PartialEq`). It's used to compare two worlds in tests.

### `crates/engine/src/world.rs` (public)
Module docs must say:
- A `World` is a running world, built from a `LoadedSetting`. The rules (goods, resource types) never change. The world holds what does: the clock, the hexes, the deposits, and one stockpile of unowned goods per hex. Hexes and deposits don't change yet, but they are world state because later Stages change them (deposit ownership in Stage 02).
- A turn has three parts:
  1. start of turn: every deposit emits its type's `output` of its good into its hex's stockpile;
  2. actors act: nothing yet, and later Stages add POPs, industries, and markets here;
  3. end of turn: whatever perishes at the end of this turn is removed.
- `World::new` puts the world at turn 0 and runs turn 0's start. `World::step` ends the current turn, advances the clock, and starts the next one. Between calls the world is always at part 2, where this turn's output is in the stockpiles and nothing has perished yet this turn.
- Deposits emit in deposit ID order, and stockpiles iterate in a fixed order, so the same setting stepped the same number of times always gives the same world and reports.
- Stage 02 will send an owned deposit's output to its owner instead of the hex stockpile.

```rust
/// What happened in one step (or in creating the world).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnReport {
    /// The turn that started.
    pub turn: Turn,
    /// Total removed per good at the end of the previous turn, in good ID order, only
    /// amounts > 0. Always empty for turn 0.
    pub perished: Vec<(GoodId, Quantity)>,
    /// Total emitted per good at the start of `turn`, in good ID order, only amounts > 0.
    pub emitted: Vec<(GoodId, Quantity)>,
}

/// Why a step was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepError {
    /// The clock is at `Turn::MAX` and can't advance. The world is unchanged.
    ClockAtMaximum,
}
// Display: "the clock is at its maximum turn (18446744073709551615) and can't advance"
// impl std::error::Error for StepError {}

/// A running world.
#[derive(Debug)]
pub struct World {
    manifest: Manifest,
    rules: Rules,
    map: HexMap,
    deposits: Deposits,
    turn: Turn,
    /// Indexed by HexId::index; one per hex.
    hex_stockpiles: Vec<Stockpile>,
}

impl World {
    /// Builds a world from `setting` at turn 0 and runs turn 0's emission.
    /// Returns the world and turn 0's report.
    pub fn new(setting: LoadedSetting) -> (World, TurnReport);
    /// Ends the current turn, advances the clock, starts the next turn.
    /// At `Turn::MAX` returns `Err(StepError::ClockAtMaximum)` and changes nothing.
    pub fn step(&mut self) -> Result<TurnReport, StepError>;
    /// The current turn.
    pub fn turn(&self) -> Turn;
    /// The setting's manifest.
    pub fn manifest(&self) -> &Manifest;
    /// The rules (never change).
    pub fn rules(&self) -> &Rules;
    /// The hexes.
    pub fn map(&self) -> &HexMap;
    /// The deposits.
    pub fn deposits(&self) -> &Deposits;
    /// The stockpile of unowned goods on `hex`.
    /// Panics (message containing "is not from this map") if `hex` isn't from this world's map.
    pub fn hex_stockpile(&self, hex: HexId) -> &Stockpile;
}
```
Logic:
- `new`: destructure the `LoadedSetting`, create `map.len()` empty stockpiles, set the turn to `Turn::ZERO`, run `emit()`, and return the report with `perished` empty.
- `step`: `let next = self.turn.next().ok_or(StepError::ClockAtMaximum)?;` before any change. Then `perish()` (each hex's `remove_perishing(self.turn)` in hex ID order, totals per good merged with `saturating_add`), set `self.turn = next`, `emit()`, and build the report.
- `emit()` (private): for each deposit in `deposits.iter()` order, look up its resource type, compute `PerishesAt::for_good(goods.get(produces).perishes, self.turn)`, `add` the type's `output` to `hex_stockpiles[deposit.hex.index()]`, and add the same amount to a per-good total. Return the totals in good ID order, only amounts > 0. Use a `BTreeMap<GoodId, Quantity>` or a `Vec` indexed by good, never a `HashMap`.
- `hex_stockpile` uses `get(hex.index())` and panics with `"hex {hex:?} is not from this map"` (match the wording of `Deposits::on_hex`).

### `crates/engine/src/lib.rs`
```rust
pub mod stockpile;
pub use stockpile::{PerishesAt, Stockpile};
pub mod turn;
pub use turn::Turn;
pub mod world;
pub use world::{StepError, TurnReport, World};
```
Keep the modules in alphabetical order with the existing ones. Crate docs, add: a running world is built from a loaded setting and stepped turn by turn (see the `world` module). Each hex holds a stockpile of goods (see the `stockpile` module), and turn numbers are `Turn` values (see the `turn` module).

### Dev UI (`crates/dev-ui/src/main.rs`)
Arguments (parse them all before loading anything):
- none → name and version, exit 0 (unchanged);
- `[path]` → setting checker as now (unchanged output);
- `[path, "--turns", n]` → setting checker, then turns;
- anything else → `usage: dev-ui [SETTING_FOLDER [--turns N]]` on stderr, exit 2.

`n` is valid only if it is valid UTF-8, non-empty, all ASCII digits (so `+5`, `-1`, ` 5`, `5.0`, and `abc` are rejected, even though `"+5".parse::<u32>()` would accept `+5`), and its value is at most 10,000 (`MAX_TURNS`). Leading zeros are fine (`007` is 7). A digit string too long for `u32` is invalid too. An invalid `n` is the usage error.

With `--turns N`, after the success line:
1. `let (mut world, report) = World::new(setting);` and print `report`'s line.
2. Repeat N times: `world.step()`. On `Ok`, print the report's line. On `Err(e)` (unreachable for N ≤ 10,000, but handle it), print `e` to stderr and exit 1.

Line format (exact; tests assert it):
```
Turn {turn}: perished {list}; emitted {list}
```
- `{list}` is `{amount} {good name}` per entry, joined by `, `, in report order, with the amount via `Quantity`'s `Display`. An empty list is `nothing`.
- Example: `Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain`.

A setting with errors prints its errors and exits 1 as now. No turns are run. Crate docs: describe `--turns N` and the new usage line.

## Tasks
1. Create `turn.rs` with module docs, `Turn`, and unit tests. Add it to `lib.rs`.
2. Add `Quantity::saturating_add` with unit tests, and update the module docs.
3. Create `stockpile.rs` with module docs, `PerishesAt`, `Stockpile`, and unit tests. Add it to `lib.rs`.
4. Create `world.rs` with module docs, `TurnReport`, `StepError`, `World`, and unit tests. Add it to `lib.rs` with the re-exports and crate docs.
5. Update the `resources.rs` module docs.
6. Create the `turns_valid` test setting and update `crates/engine/tests/settings/README.md`.
7. Write `crates/engine/tests/world.rs`.
8. Dev UI: argument parsing, `--turns`, the line format, the usage message, crate docs; update and extend `checker.rs`.
9. Run `python tools/check.py` and fix everything until it passes. Run `cargo run -p policyspace-dev-ui -- settings/sample --turns 3` and `… -- crates/engine/tests/settings/turns_valid --turns 4`, and check the output by eye against the Test Plan.

## Test Plan

### New test setting (`crates/engine/tests/settings/turns_valid/`)
`setting.toml`: `name = "Turns Test"`, `format_version = 1`.

`rules/goods.toml`, in order:
| `name` | `perishes_after` | `movable` |
|---|---|---|
| `daily` | 1 | false |
| `stew` | 3 | (omitted) |
| `gold` | (omitted) | (omitted) |

`rules/resources.toml`, in order:
| `name` | `produces` | `output` |
|---|---|---|
| `field` | `daily` | 2 |
| `meadow` | `daily` | 0.5 |
| `pot` | `stew` | 5 |
| `mine` | `gold` | 1.5 |

`start/map.toml`: hexes (0, 0), (1, 0), (2, 0).

`start/deposits.toml`, in order: `field` (0, 0) `count = 2`; `meadow` (0, 0); `pot` (0, 0); `field` (1, 0); `mine` (1, 0). Deposit IDs: 0–1 `field`@(0,0), 2 `meadow`@(0,0), 3 `pot`@(0,0), 4 `field`@(1,0), 5 `mine`@(1,0).

Expected (T = turn):
- (0, 0): `daily` 4.5 in one group `EndOfTurn(T)`. `stew` is 5 in turn 0 (`EndOfTurn(2)`), 10 in turn 1 (`EndOfTurn(2)`: 5, `EndOfTurn(3)`: 5), and 15 from turn 2 on (three groups of 5, at `EndOfTurn(T)`, `EndOfTurn(T+1)`, `EndOfTurn(T+2)`).
- (1, 0): `daily` 2 (`EndOfTurn(T)`); `gold` 1.5 × (T + 1) in one `Never` group.
- (2, 0): always empty.
- Report for turn 0: perished nothing; emitted `daily` 6.5, `stew` 5, `gold` 1.5. Turns 1 and 2: perished `daily` 6.5; emitted the same as turn 0. Turn 3 onward: perished `daily` 6.5, `stew` 5; emitted the same.

### Tests
Engine integration tests go in `crates/engine/tests/world.rs` (public API only). Compare amounts with `Quantity::get()` and `assert_eq!` on `f64`. Every expected value here is exact in binary floating point.

| # | Test | Acceptance Criterion |
|---|---|---|
| 1 | Sample setting: `World::new` gives turn 0. (0, 0)'s stockpile holds exactly `unimproved_copper` 2.5 and `unimproved_grain` 12, each in one `EndOfTurn(0)` group, listed in good ID order (copper before grain). A hex with no deposits, for example (2, -2) or any hex not in the deposit table, is empty. Turn 0's report is `perished` empty, `emitted` [(`unimproved_copper`, 10), (`unimproved_grain`, 24)]. | sample at turn 0 |
| 2 | Sample: step 3 times. Each report has `turn` 1, 2, 3, with `perished` and `emitted` both [(`unimproved_copper`, 10), (`unimproved_grain`, 24)]. After each step, (0, 0) holds the same totals as in turn 0, in one `EndOfTurn(current turn)` group. | sample steps |
| 3 | `turns_valid`: step to turn 5 and check `stew` at (0, 0) every turn (totals 5, 10, 15, 15, 15, 15 and the groups listed above), and each report's `perished` (`stew` absent in turns 1 and 2, `stew` 5 from turn 3 on). | 3-turn good builds up and levels off |
| 4 | `turns_valid`: `gold` at (1, 0) is 1.5 × (T + 1) for T = 0..=5, always one `Never` group. `gold` never appears in any report's `perished`. | durable good grows, never perishes |
| 5 | `turns_valid`: at (0, 0), `daily` is 4.5 in exactly one group (two `field` deposits plus one `meadow`, merged). (1, 0) holds no `stew`, and (0, 0) holds no `gold`. (2, 0) is empty in every turn. | several deposits into one group; deposits don't emit into other hexes |
| 6 | `valid_minimal` (no hexes) and `hexes_one` (one hex, no deposits): `World::new` and 3 steps. Every report has empty `perished` and `emitted`, and `hexes_one`'s one stockpile stays empty. | no deposits |
| 7 | Determinism: two worlds from `turns_valid` stepped 6 times give equal reports at every step (`TurnReport: PartialEq`) and equal stockpiles on every hex (`Stockpile: PartialEq`). | identical runs |
| 8 | `World` accessors: `manifest().name == "Turns Test"`, `rules().goods.len() == 3`, `map().len() == 3`, `deposits().len() == 6`, `turn()` after 2 steps is `Turn::new(2)`. | engine interface |
| 9 | `hex_stockpile` with a `HexId` from a bigger map (load the sample, take a hex with index ≥ 3, and ask the `turns_valid` world) panics with `is not from this map` (`#[should_panic(expected = ...)]`). | (robustness) |
| 10 | `world.rs` unit test: set a world's `turn` field to `Turn::MAX` directly, then `step()` returns `Err(StepError::ClockAtMaximum)`, and the turn and every stockpile are unchanged. Check `StepError`'s `Display` text. | clock maximum refused, world unchanged |
| 11 | `turn.rs` unit tests: `new`/`get`; `next` of 0 is 1, of `MAX` is None; `saturating_add` normal and at overflow; `Display`; ordering; `Default` is `ZERO`. | (robustness) |
| 12 | `quantity.rs` unit tests: `saturating_add` of 2.5 and 4 is 6.5; with `ZERO` is unchanged; `f64::MAX + f64::MAX` is `f64::MAX` (finite). | (robustness) |
| 13 | `stockpile.rs` unit tests (build `GoodId`s with `Id::from_index`, which is `pub(crate)`): `PerishesAt::for_good` (`Never`; `After(1)` in turn 5 → `EndOfTurn(5)`; `After(3)` in turn 5 → `EndOfTurn(7)`; `After(3)` in turn `MAX - 1` → `EndOfTurn(MAX)`); `PerishesAt` order (`EndOfTurn(1) < EndOfTurn(2) < Never`); `add` merges into one group; adding zero leaves the stockpile empty; `iter` in good ID order; `total` and `groups` (soonest first, `Never` last; empty for a good not held); `remove_perishing(t)` removes groups at `t` and earlier, keeps later and `Never` ones, drops emptied goods, and returns totals in good ID order without zeros; `is_empty`. | (robustness) |
| 14 | `checker.rs`: `settings/sample --turns 2` prints, on stdout, exactly the success line then `Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain`, `Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain`, and the same line for `Turn 2`, and exits 0. | checker `--turns 2` |
| 15 | `checker.rs`: `settings/sample --turns 0` prints the success line and only the `Turn 0` line. `valid_minimal --turns 1` prints `Turn 0: perished nothing; emitted nothing` and `Turn 1: perished nothing; emitted nothing`. `turns_valid --turns 3` prints `Turn 3: perished 6.5 daily, 5 stew; emitted 6.5 daily, 5 stew, 1.5 gold` as its last line. | `--turns 0`; empty lists; decimal amounts |
| 16 | `checker.rs`: each of these prints exactly `usage: dev-ui [SETTING_FOLDER [--turns N]]` on stderr, nothing on stdout, and exits 2: `a b` (the existing test, with its expectation updated); `settings/sample --turns`; and `settings/sample` followed by `--turns 10001`, `--turns -1`, `--turns +5`, `--turns abc`, `--turns 99999999999`, `--turns 1 extra`, or `--steps 1`. `settings/sample --turns 10000` is **not** run (a large output for a test); `settings/sample --turns 007` exits 0 with 8 turn lines (turns 0–7). | usage errors |
| 17 | `checker.rs`: `many_errors --turns 2` prints the same errors as the existing broken-setting test, no turn lines, and exits 1. Running without `--turns` is covered by the existing tests, unchanged. | broken setting with `--turns`; output unchanged without it |
| 18 | All existing tests still pass, with only the usage expectation in `two_arguments_print_usage_and_exit_2` changed. | (regression) |
| 19 | `python tools/check.py` exits 0. | single check command passes |

`checker.rs`'s `run` takes `&[&Path]`. Either pass flags as `Path::new("--turns")` or change it to take `&[&OsStr]` or `&[impl AsRef<OsStr>]`, whichever is tidier.

Run everything with `python tools/check.py` (or `.venv/Scripts/python tools/check.py`).

## Constraints
- No new external crates (no argument-parsing crate).
- Don't change the loader (`crates/engine/src/setting/`), `id.rs`, `goods.rs`, or `map.rs`. In `resources.rs`, change only the module docs. If any of these seems to need a change, stop and report.
- `turn`, `stockpile`, and `world` must not depend on anything in `setting/` except the public types `LoadedSetting`, `Manifest`, and `Rules` (re-exported from the crate root), which `world` takes apart.
- `Stockpile::add` and `Stockpile::remove_perishing` stay `pub(crate)`. There's no public way to change a stockpile, and no way to take goods out yet.
- Determinism: no `HashMap` or `HashSet` anywhere in this Step's new code. Deposits emit in ID order. Every list is in good ID order (or hex ID order).
- No `unwrap`/`expect`/panic on anything a setting controls. The only panic is `hex_stockpile` given a foreign hex (a caller bug).
- `Quantity` gets only `saturating_add`.
- No ownership, consumption, depletion, starting stockpiles, effect of `movable`, per-hex report breakdowns, or saving and loading.
- Don't change existing dev UI output: the no-argument line, the success line, the error output, and exit codes stay as they are. Only the usage message gains `[--turns N]`.
- Don't touch `PROJECT_PLAN/`, `CLAUDE.md`, `.claude/`, `docs/`, `manual.ipynb`, `manual.html`, `tools/`, or `settings/sample/`.
- Don't commit.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` exits 0, and every Acceptance Criterion in design.md is met.
