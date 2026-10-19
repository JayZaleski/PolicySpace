# Step 04: factory-mechanics — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-04-factory-mechanics/design.md` — the behavior this spec implements: the run, wear and maintenance rules, the sharing of shortages, the order within a turn, the full-slice rule for running, and the record.
- `docs/demand.md` and `crates/engine/src/demand.rs` — the pattern this Step copies: a pure-function module with its own unit tests, wired into the turn by a later Step, knowing nothing about the World or files.
- `crates/engine/src/trade.rs` — `Submission`: a caller-owned struct that carries a reference into the rules plus the state the round changes; `hex_round` over a slice of them; `consume` (removal soonest-perishing first with an `assert!` that it can't fail).
- `crates/engine/src/technologies.rs` — `Technology` and the meaning of its numbers (module docs). `crates/engine/src/bundle.rs` — `Bundle`. `crates/engine/src/factories.rs` — `Factory.condition` (an `f64` from 0 to 1). `crates/engine/src/stockpile.rs` — `Stockpile::add`, `remove`, `total`, `groups`, `PerishesAt::for_good`. `crates/engine/src/goods.rs` — `Goods`, `Good.perishes`. `crates/engine/src/quantity.rs`. `crates/engine/src/turn.rs`. `crates/engine/src/world.rs` (`World::new`: how business stockpiles are filled; `business_stockpile`, `factories`, `rules`, `turn` accessors). `crates/engine/src/test_support.rs` (unit-test helpers: `q`, `close`, `good`, `end`, `after`, the good index constants). `crates/engine/src/lib.rs`.
- Tests: `crates/engine/tests/common/mod.rs`, `crates/engine/tests/demand.rs` and `crates/engine/tests/factories.rs` (integration tests through the public API), `crates/engine/tests/settings/README.md`, `crates/engine/tests/settings/factories_valid/` (the shape of a test setting with technologies, businesses and factories).
- `docs/workspace.md` — the check command and test layout. `docs/world.md` — the turn and the stockpiles.
- `settings/sample/rules/technologies.toml`, `settings/sample/start/businesses.toml`, `settings/sample/start/factories.toml` — the sample's chain, used by one integration test.

## Approach

- **A new module `crates/engine/src/production.rs`** holds the mechanics as pure functions, like `demand.rs`: `run`, `wear` and `maintain`, each over one business's factories, plus `operate`, which chains them in the design's order (run, wear, maintain) and assembles one `FactoryRecord` per factory. Nothing in the turn calls them (Step 08). Rationale: the design asks for three functions with their own tests and a fixed order; one place owns the order.
- **The functions take a caller-owned `Operation` per factory** (a reference to its `Technology`, its condition going in and coming out, and the two fractions the business asked for), a `&mut Stockpile` (the business's, moved out of the World by the caller as `trade::Submission` does), the `Goods` (for the output's perish rule) and the current `Turn`. The module never touches `Factories`, `FactoryId`, `World` or files; Step 08 builds the operations from the business's factories and writes the conditions back. Rationale: testable without a world, and no new mutable access to `Factories` until the Step that needs it.
- **Condition arithmetic runs in lifetime-slices with snapping.** `slices = condition × lifetime`, snapped to the nearest whole number when within `1e-9` of it; wear subtracts one slice (or what is left) and recomputes the condition as `slices / lifetime`, so an unmaintained factory's condition is always exactly k/lifetime and its life is exact for any lifetime (3, 24, ...). Maintenance snaps a condition within `1e-9` of 1 to exactly 1 and within `1e-9` of 0 to exactly 0. Rationale: the design's "rounding never changes a factory's life"; recomputing from a whole number stops errors accumulating.
- **A factory runs only when it holds at least one slice** (snapped `slices >= 1`): design decision made this session, closing the hole where a factory a hair above zero ran at full capacity for a hair of maintenance.
- **Shortages are shared in one pass, per good**: the proportion held/asked per good (at most 1), and each factory is cut by the smallest proportion over the goods it needs. The same code serves running (inputs) and maintenance (capital plus construction goods). Removals use the computed amounts; a removal that exceeds the stockpile by a floating-point speck takes what is held instead (the record keeps the computed amount, so records don't depend on removal order).
- **`Bundle` gains `scaled` and `merged`**, the arithmetic Step 01 deferred to this Step.
- **Fractions and conditions are checked with `assert!`**: an out-of-range run or maintenance fraction, or condition, is an engine bug (Step 05 validates decisions before they get here), reported like the other invariant panics in the crate.
- **The build runs as one.** The module, the `Bundle` methods, the test setting and the tests are one connected piece of work; nothing could be built and tested alone.

## Changes

### New

- `crates/engine/src/production.rs` — the module, its types and functions, unit tests.
- `crates/engine/tests/production.rs` — integration tests (Test Plan).
- Test setting `crates/engine/tests/settings/production_valid/` (exact contents below).

### Modified

- `crates/engine/src/bundle.rs` — `scaled`, `merged`; unit tests; the module docs' "and later a factory's current capital" becomes "the goods a run uses, the goods maintenance costs" (bundles are also the engine's working lists of goods with amounts).
- `crates/engine/src/lib.rs` — `pub mod production; pub use production::{FactoryRecord, MaintenanceRecord, Operation, RunRecord, operate};` after `pops`, before `quantity`. Crate docs: the sentence "Factories ... nothing runs them yet." becomes: factories are placed in the starting state, each on its owner's hex, running one technology and holding a share (its condition) of that technology's capital bundle; the [`production`] module gives them their mechanics as pure functions over one business's factories and stockpile (running at a fraction of capacity with inputs shared when short, wear of one lifetime-slice per turn with shutdown below one slice, and maintenance that replenishes a fraction of the capital bundle at the cost of that fraction of the capital and construction bundles); nothing in the turn calls them yet.
- `crates/engine/src/factories.rs` and `crates/engine/src/technologies.rs` — module docs only: the sentences saying nothing runs, wears or maintains factories yet now say the `production` module does, as pure functions, and Step 08 wires it into the turn.
- `crates/engine/tests/settings/README.md` — the new setting.

Nothing else changes: not `World`, the turn, `TurnReport`, `Factory`, `Factories`, `Stockpile`, the loader, the sample setting, the dev UI or any existing test.

## Interfaces & Data

### `crates/engine/src/bundle.rs`

```rust
impl Bundle {
    /// Every amount multiplied by `factor` (saturating). The goods stay named even when
    /// `factor` is zero.
    pub fn scaled(&self, factor: Quantity) -> Bundle;
    /// The goods of both bundles, with the amounts of a good named in both added
    /// (saturating), in good ID order.
    pub fn merged(&self, other: &Bundle) -> Bundle;
}
```

`merged` walks the two sorted lists together (a merge), so it needs no `Bundle::new` sort; `scaled` maps the items. Both keep the sorted, no-duplicates invariant.

### `crates/engine/src/production.rs`

```rust
//! Module docs: Factory mechanics: what happens when a business runs a factory at a fraction
//! of its capacity, how a factory wears each turn, and what maintaining it costs. Pure
//! functions over one business's factories, each with its own tests; nothing in the turn
//! calls them yet (Step 08 does, after the trade round, once per business, in the order
//! `operate` fixes). State the rules from design.md: the two fractions (run and maintenance,
//! each 0 to 1, chosen by the business's decider); running at capacity times the run
//! fraction when the factory holds at least one lifetime-slice of capital (below one slice
//! it is shut down and makes nothing, so every turn of output costs exactly one slice),
//! inputs leaving the stockpile soonest-perishing first and output entering it only after
//! every factory has run, as goods of the current turn; a business's factories run as if at
//! the same moment, so a shortage is shared (each factory cut by the smallest held/asked
//! proportion over its goods, one pass, leftovers stay); wear of one slice per turn whether
//! or not the factory ran, stopping at zero, in slice arithmetic that keeps a never-maintained
//! factory's life exact; maintenance of a fraction f costing f of the capital bundle and f
//! of the construction bundle, capped at one construction-turn's slice, then at full
//! condition, then by the goods on hand shared like inputs; a factory whose construction
//! takes longer than its lifetime can't be revived from zero. Order: run, wear, maintain.
//! The cuts are a safety net, not the normal path. Knows nothing about the World,
//! `Factories` or files: the caller moves each business's stockpile out, builds one
//! `Operation` per factory and writes the conditions back.

use std::collections::BTreeMap;

use crate::bundle::Bundle;
use crate::goods::{GoodId, Goods};
use crate::quantity::Quantity;
use crate::stockpile::{PerishesAt, Stockpile};
use crate::technologies::Technology;
use crate::turn::Turn;

/// How close to a whole number of slices, or to 0 or 1, a condition must be to count as
/// exactly that. Conditions are decimal numbers and a lifetime-slice need not be exact, so
/// without this a factory could keep a speck of condition after its last turn.
const TOLERANCE: f64 = 1e-9;

/// One factory as the mechanics see it: its technology, its condition going in (and, after
/// `wear`, `maintain` or `operate`, coming out) and the fractions its business asked for.
/// The caller builds one per factory of a business and writes `condition` back afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operation<'a> {
    /// The technology the factory runs.
    pub technology: &'a Technology,
    /// The share of the technology's capital bundle the factory holds: finite, 0 to 1.
    /// Read by every function; changed by `wear` and `maintain`.
    pub condition: f64,
    /// How much of the capacity to use this turn: finite, 0 to 1.
    pub run_fraction: f64,
    /// How much of the full capital bundle to replenish this turn: finite, 0 to 1.
    pub maintenance_fraction: f64,
}

/// What running did for one factory.
#[derive(Debug, Clone, PartialEq)]
pub struct RunRecord {
    /// The run fraction the business asked for.
    pub asked: f64,
    /// The fraction actually run: `asked`, or less after a cut for a missing input, or 0
    /// for a factory below one slice. Below `asked` shows a cut order.
    pub done: f64,
    /// The inputs taken from the stockpile, only amounts > 0 (empty when nothing ran).
    pub inputs: Bundle,
    /// The output made: capacity times `done`.
    pub output: Quantity,
}

/// What maintenance did for one factory.
#[derive(Debug, Clone, PartialEq)]
pub struct MaintenanceRecord {
    /// The maintenance fraction the business asked for.
    pub asked: f64,
    /// The fraction actually replenished, after the construction-slice cap, the
    /// full-condition cap and the goods on hand. Below `asked` shows a cut order.
    pub done: f64,
    /// The goods taken from the stockpile: `done` of the capital bundle plus `done` of the
    /// construction bundle, only amounts > 0 (empty when nothing was done).
    pub cost: Bundle,
}

/// What one turn's mechanics did for one factory, in the order they happened.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryRecord {
    /// The condition before anything happened.
    pub condition_before: f64,
    /// Running.
    pub run: RunRecord,
    /// The condition lost to wear: one lifetime-slice, or what was left if less.
    pub wear: f64,
    /// Maintenance.
    pub maintenance: MaintenanceRecord,
    /// The condition after maintenance: what the caller writes back to the factory.
    pub condition_after: f64,
}

/// Runs one business's factories together. Each factory holding at least one lifetime-slice
/// runs at capacity times its run fraction, cut by the smallest held/asked proportion over
/// its inputs when the factories together need more of a good than `stockpile` holds (one
/// pass; the leftover of a good freed by a cut stays). Inputs leave `stockpile` soonest-
/// perishing first; every output enters it after every factory has run, as goods added in
/// `turn` (perishing by the good's rule in `goods`). Conditions don't change. Returns one
/// record per operation, in order; the records don't depend on the order of the slice.
///
/// # Panics
/// Panics if a condition or run fraction isn't a finite number from 0 to 1.
pub fn run(
    operations: &[Operation<'_>],
    stockpile: &mut Stockpile,
    goods: &Goods,
    turn: Turn,
) -> Vec<RunRecord>;

/// Wears one business's factories: each loses one lifetime-slice of condition, or what it
/// has left if less, whether or not it ran. Returns the condition each lost, in order.
///
/// # Panics
/// Panics if a condition isn't a finite number from 0 to 1.
pub fn wear(operations: &mut [Operation<'_>]) -> Vec<f64>;

/// Maintains one business's factories together. Each factory's fraction is reduced to at
/// most one construction-turn's slice, then to at most what fills it, then by the smallest
/// held/asked proportion over the goods it needs (its capital and construction bundles times
/// the fraction) when the factories together need more than `stockpile` holds. The goods
/// leave `stockpile` soonest-perishing first and each condition rises by the fraction done.
/// Returns one record per operation, in order.
///
/// # Panics
/// Panics if a condition or maintenance fraction isn't a finite number from 0 to 1.
pub fn maintain(operations: &mut [Operation<'_>], stockpile: &mut Stockpile) -> Vec<MaintenanceRecord>;

/// One turn of mechanics for one business: `run`, then `wear`, then `maintain`, in that
/// order (so a full, never-maintained factory runs exactly its lifetime, and replenishing
/// one lifetime-slice each turn holds a factory at full condition). Returns one record per
/// operation, in order, with the conditions before and after.
///
/// # Panics
/// As the three functions.
pub fn operate(
    operations: &mut [Operation<'_>],
    stockpile: &mut Stockpile,
    goods: &Goods,
    turn: Turn,
) -> Vec<FactoryRecord>;
```

Private helpers (names are suggestions; the behavior is not):

- `fn check(operations)`: asserts every `condition` is finite and in `0..=1` (`"a factory's condition must be a finite number from 0 to 1"`, the same text as `Factories::from_factories`), and, as the caller asks, every `run_fraction` (`"a run fraction must be a finite number from 0 to 1"`) or `maintenance_fraction` (`"a maintenance fraction must be a finite number from 0 to 1"`). `run` checks conditions and run fractions, `wear` conditions, `maintain` conditions and maintenance fractions.
- `fn slices(condition: f64, lifetime: u32) -> f64`: `condition × lifetime`, snapped to `round()` when within `TOLERANCE` of it.
- `fn condition_of(slices: f64, lifetime: u32) -> f64`: `slices / lifetime`, then `snap`.
- `fn snap(condition: f64) -> f64`: a value below `TOLERANCE` becomes 0, above `1 − TOLERANCE` becomes 1, otherwise unchanged.
- `fn proportions(needs: &BTreeMap<GoodId, Quantity>, stockpile: &Stockpile) -> BTreeMap<GoodId, f64>`: for every good with a need > 0, `min(1, held / need)`; a good with need 0 isn't listed (its proportion is 1).
- `fn cut(bundle: &Bundle, proportions) -> f64`: the smallest proportion over the goods the bundle names (1 if it names none, or none are listed).
- `fn take(stockpile, good, amount)`: `stockpile.remove(good, amount)`; on `Err(held)`, `debug_assert!` that `amount − held` is below `1e-9 × max(amount, 1)` (only rounding can get here: the proportions guarantee the total asked fits) and remove `held` instead. A zero amount is skipped.
- `fn positive(bundle: Bundle) -> Bundle`: the same bundle without zero amounts (`Bundle::new` over the filtered items).

`run`, step by step:

1. `check`. For each operation, `eligible = slices(condition, lifetime) >= 1.0`; `wanted = if eligible { capacity × run_fraction } else { 0 }` (a `Quantity`; `run_fraction` through `Quantity::new`, which can't fail for a checked fraction).
2. Needs per good: for each operation, for each `(good, per_unit)` in `technology.inputs()`, add `per_unit × wanted` into a `BTreeMap<GoodId, Quantity>`.
3. `proportions` against `stockpile`. For each operation: `done = if eligible { run_fraction × cut(inputs, proportions) } else { 0.0 }`; `output = capacity × done`; `inputs_used = technology.inputs().scaled(output)`. A technology with no inputs has `cut` 1 and runs on nothing.
4. Remove every operation's `inputs_used` from `stockpile` with `take`, operations in slice order, goods in bundle order. (Removal order matters only to the rounding fallback; the records carry the computed amounts.)
5. Only now add every operation's `output` (if > 0) with `stockpile.add(technology.output(), output, PerishesAt::for_good(goods.get(technology.output()).perishes, turn))`.
6. Records: `RunRecord { asked: run_fraction, done, inputs: positive(inputs_used), output }`.

`wear`: `check`; for each operation, `s = slices(condition, lifetime)`; `lost = s.min(1.0)`; `condition = condition_of(s − lost, lifetime)`; record `lost / lifetime` (as `f64`). After this an unmaintained factory's condition is `k / lifetime` for a whole `k`, exactly.

`maintain`, step by step:

1. `check`. For each operation: `room = snap((1.0 − condition).max(0.0))`; `want = maintenance_fraction.min(1.0 / construction_turns).min(room)`; `goods_per_unit = technology.capital().merged(technology.construction())` (the goods one full unit of maintenance costs).
2. Needs per good: `goods_per_unit.scaled(want)` summed into a `BTreeMap`.
3. `proportions`; `done = want × cut(goods_per_unit, proportions)`; `cost = goods_per_unit.scaled(done)`.
4. Remove every `cost` with `take` (slice order, bundle order).
5. `condition = snap(condition + done)`. Record `MaintenanceRecord { asked: maintenance_fraction, done, cost: positive(cost) }`.

`operate`: `check` happens in the three calls; take `condition_before` from each operation, call `run`, `wear`, `maintain`, and zip into `FactoryRecord`s with `condition_after` from the operations.

Pitfalls:

- `Quantity` has no `From<f64>`: build fractions with `Quantity::new(f).expect("a checked fraction is a valid quantity")`.
- Don't compare conditions to 0 or 1 directly anywhere; go through `slices` and `snap`.
- `BTreeMap` iteration is in good ID order, so every sum is in a fixed order and the results are deterministic.
- Two operations can share a `Technology` reference (two factories of one technology); `Operation` is `Copy`, so the caller can build them from a slice of factories without lifetimes trouble.

### Test setting `production_valid/`

Every file exactly as given; `setting.toml` has `format_version = 1` and name `Production Test`. Businesses have `deposits = {}` and `decider = "rule"`.

`rules/goods.toml` (IDs 0-5): `coin` (no properties), `labor` (`perishes_after = 1`), `ore` (`perishes_after = 3`), `iron`, `tools`, `bread` (`perishes_after = 2`).

`rules/technologies.toml` (IDs 0-2):
```toml
[[technology]]
name = "smelter"
inputs = { ore = 2, labor = 1 }
output = "iron"
capacity = 10
capital = { tools = 4 }
construction = { labor = 8 }
construction_turns = 2
lifetime = 4

[[technology]]
name = "bakery"
inputs = { labor = 1 }
output = "bread"
capacity = 6
capital = { tools = 3 }
construction = {}
construction_turns = 1
lifetime = 3

[[technology]]
name = "spring"
inputs = {}
output = "ore"
capacity = 5
capital = { tools = 1 }
construction = { labor = 2 }
construction_turns = 4
lifetime = 3
```
`start/map.toml`: hexes (0, 0) and (1, 0), currency `coin`. `start/businesses.toml`: `works` at (0, 0), `inventory = { coin = 50, labor = 13, ore = 30, tools = 3 }`; `springs` at (1, 0), `inventory = { tools = 1 }` (IDs 0, 1). `start/factories.toml` (IDs 0-4): 0 `smelter` owned by `works` (condition 1); 1 `smelter` `works` `condition = 0.5`; 2 `bakery` `works`; 3 `spring` `springs`; 4 `spring` `springs` `condition = 0.5`. All at their owner's hex.

The world at turn 0 (no deposits, no POPs): `works` holds coin 50 (`Never`), labor 13 (`end(0)`), ore 30 (`end(2)`), tools 3 (`Never`); `springs` holds tools 1. The `README.md` entry says what it is for and lists the goods, technologies, businesses, factories and the turn-0 stockpiles.

## Tasks

1. `bundle.rs`: `scaled`, `merged`, docs, unit tests.
2. `production.rs`: types, helpers, `run`, `wear`, `maintain`, `operate`, module docs. Export from `lib.rs`; crate docs; the doc sentences in `factories.rs` and `technologies.rs`.
3. `production.rs` unit tests (Test Plan).
4. Test setting `production_valid`; `tests/settings/README.md`.
5. `crates/engine/tests/production.rs` (Test Plan).
6. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the success line and turn lines are unchanged.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). Compare `f64` values and stockpile amounts with `close` (1e-9) by default: a value that looks exact (0.1875, say) may have come through an inexact intermediate such as a room of 1 − 2/3. Use `assert_eq!` only for what the spec guarantees exactly: a condition of `0.0` or `1.0` after snapping, k/lifetime after wear of an unmaintained factory, and amounts computed from halves, quarters and whole numbers alone (the run tests, where no third is involved). The expected values below are written as exact fractions for readability.

### Unit tests, `bundle.rs`

`scaled` multiplies every amount (by 0.5 and by 0, which keeps the goods named); `merged` adds the amounts of a shared good, keeps the goods only one bundle names, stays sorted, and merging with an empty bundle gives a copy.

### Unit tests, `production.rs`

Local helpers: `goods()` builds a `Goods` (`NamedList::from_parts` with a `NameRegistry`) of `labor` (perishes after 1), `grain` (after 3), `copper`, `silver` and `tools` (never), matching `test_support`'s `LABOR`, `GRAIN`, `COPPER`, `SILVER` indexes plus a local `TOOLS = 4`; `tech(inputs, output, capacity, capital, construction, construction_turns, lifetime)` builds a `Technology` from `(index, amount)` slices; `stock(items)` builds a `Stockpile` with every item at `PerishesAt::Never` unless a test adds groups itself; `op(&tech, condition, run, maintain)` builds an `Operation`. Test names are suggestions.

- **AC 1 (running).** `a_factory_runs_at_capacity_times_the_fraction`: smelter-like tech (inputs grain 2, labor 1; output copper; capacity 10; capital tools 4; construction labor 8; t 2; L 4), stock grain 30, labor 13, condition 1, run 0.5: record `done` 0.5, `inputs` `[(labor, 5), (grain, 10)]`, `output` 5; the stockpile holds grain 20, labor 8, copper 5 in a `Never` group (durable), and its condition is unchanged by `run`. `output_perishes_by_its_goods_rule_from_the_current_turn`: a tech with output `grain` run at `Turn::new(4)` leaves a group at `end(6)`. `inputs_leave_soonest_perishing_groups_first`: labor 4 at `end(0)` and 6 at `end(5)`; a run needing 5 leaves `[(end(5), 5)]`. `output_is_delivered_after_every_factory_has_run`: factory A makes grain from nothing (no inputs), factory B needs grain, the stock has no grain: B's `done` is 0 and its inputs empty, and afterwards the stock holds A's output; reversing the two operations gives the same. `a_technology_with_no_inputs_runs_on_nothing`: empty stock, `done` 1, `inputs` empty, output capacity.
- **AC 2 (sharing).** `a_shortage_is_shared_proportionally_whatever_the_order`: two factories of one tech needing labor 10 each at run 1, stock labor 5: both `done` 0.25, output 2.5, inputs labor 2.5; the stock has no labor left; the reversed slice gives the reversed records. `a_factory_short_of_two_goods_is_cut_by_the_worse_shortage_and_the_leftover_stays`: factory A needs grain 20 and labor 10, factory B needs labor 10 only (both at run 1), stock grain 10 and labor 10: proportions grain 0.5, labor 0.5; A `done` 0.5 (inputs grain 10, labor 5), B `done` 0.5 (labor 5); nothing left. Then a variant with stock grain 5 and labor 20: A cut to 0.25 by grain, B uncut at 1 (labor 10), and the stock keeps labor 7.5 (A's cut freed 7.5 labor that B didn't need: the leftover stays).
- **AC 3 (wear and shutdown).** `wear_takes_one_slice_run_or_not_and_stops_at_zero`: L 4, conditions 1 → 0.75 whether run 1 or run 0 (`wear` returns 0.25); 0.25 → 0 (returns 0.25); 0 → 0 (returns 0); 0.1 → 0 (returns 0.1 within `close`). `a_factory_below_one_slice_makes_nothing`: a no-input tech, L 4: condition 0 and condition 0.2 (0.8 slices) give `done` 0 and output 0 at run 1; condition 0.375 (1.5 slices) runs once (`done` 1), wears to 0.125, and the next `operate` makes nothing and wears it to exactly 0. `a_full_factory_runs_exactly_its_lifetime`: for each lifetime in `[1, 2, 3, 7, 24, 100]`, a no-input tech at condition 1 operated lifetime times at run 1 makes capacity every time, and after the last the condition is exactly `0.0`; the next `operate` makes nothing; before the last run the condition is `close` to `1 / lifetime`. `an_unmaintained_condition_stays_a_whole_number_of_slices`: L 3 from 1: exactly `2.0 / 3.0`, then `1.0 / 3.0`, then `0.0` (the recomputation from a whole number).
- **AC 4 (maintenance).** `maintenance_costs_the_fraction_of_both_bundles_and_raises_the_condition`: the smelter-like tech at condition 0.5, maintain 0.25 (t 2 allows 0.5), stock tools 10, labor 10: `done` 0.25, `cost` `[(labor, 2), (tools, 1)]`, condition 0.75, stock tools 9, labor 8. `maintenance_is_capped_by_the_construction_slice_then_by_full_condition`: t 2 at condition 0.25, maintain 1 → `done` 0.5 (the slice), condition 0.75; t 2 at 0.75, maintain 1 → `done` 0.25 (what fills it), condition exactly `1.0`; t 1 at 0.75 (room 0.25), maintain 0.1 → `done` 0.1. `maintenance_shortage_is_shared_proportionally_and_by_the_worst_good`: two factories of the smelter-like tech at condition 0.5 each, maintain 0.5 each (needs tools 2 + labor 4 each), stock tools 2 and labor 8: proportions tools 0.5, labor 1 → each `done` 0.25, cost tools 1, labor 2, condition 0.75; the stock has no tools and labor 4 left. `a_shut_down_factory_comes_back_slice_by_slice`: t 2, L 4 at condition 0: maintain 0.5 gives condition 0.5 (2 slices) and the next `operate` runs it (`done` 1); with t 4, L 4 at 0: maintain 1 → `done` 0.25 (one slice), the next `operate` runs it, wears it to 0 and maintains it to 0.25 again, so steady 1/4 maintenance keeps a t = L factory running; with t 8, L 4 at 0: `operate` with maintain 1 for 5 turns never runs it (condition 0.125 after each turn: wear takes the half slice back each time), the case design.md calls unrevivable. `steady_maintenance_of_one_slice_holds_a_factory_full`: a no-input tech with L 3, t 1, capital tools 3, construction empty, condition 1, stock tools 1000: `operate` 30 times with run 1 and maintain `1.0 / 3.0`; every turn `done` 1, output capacity, `maintenance.done` close to 1/3, `condition_after` exactly `1.0` (the snap). `run_takes_its_inputs_before_maintenance_takes_its_goods`: a tech whose input and construction good are both labor (inputs labor 1, construction labor 8, t 1, L 4, capacity 10, capital tools 4), condition 0.75, stock labor 10 and tools 10, run 1 and maintain 0.25: running uses all 10 labor (`done` 1); maintenance then finds no labor and `done` is 0 (asked 0.25; tools untouched).
- **AC 5 (the record).** `the_record_shows_asked_above_done_on_a_cut`: one `operate` on the two-goods case above: `run.asked` 1 and `run.done` 0.25, `maintenance.asked` 0.5 and `maintenance.done` 0 (no tools in stock), `condition_before`, `wear`, `condition_after` as expected; the records of a factory that was not cut show `asked == done`.
- **Checks.** `#[should_panic]` with the exact messages: a run fraction of 1.5, −0.1 and NaN (`run`), a maintenance fraction of 2 (`maintain`), a condition of 1.5 (`wear`).

### `crates/engine/tests/production.rs` (`mod common;`)

Helpers: `business(&World, name) -> BusinessId`; `operations(&World, BusinessId, orders: &[(f64, f64)]) -> Vec<Operation<'_>>` builds one `Operation` per factory the business owns, in ID order (`world.factories().owned_by`, the technology from `world.rules().technologies.get(factory.technology)`, the condition from the factory), zipped with `(run, maintenance)` fractions; `stock(&World, BusinessId) -> Stockpile` clones `world.business_stockpile`; `rows(records) -> Vec<(f64, f64, Vec<(String, f64)>, f64, f64, f64, f64, Vec<(String, f64)>, f64, f64)>` or any flattening with named goods that makes whole-record assertions readable.

- **AC "every factory of the sample can be operated; without labor nothing runs and nothing is maintained"**: load the sample, build the world (turn 0). For each of the three businesses, `operate` its one factory with run 1 and maintain 1 on a clone of its stockpile at `world.turn()`: `center_farm`'s factory has `run.done` 0, empty inputs, output 0, `wear` 0.125, `maintenance.done` 0 with empty cost, `condition_after` 0.875 (the farm holds 8 unimproved grain but no labor, so the labor proportion is 0); `center_mine`'s: 0.75 → 0.625; `center_toolmaker`'s: 1 → close to 23/24 with `wear` close to 1/24. Every clone is unchanged from the world's stockpile, and the world's own factories still have their loaded conditions (the functions touch only what they are given).
- **AC "running shares a shortage, cuts by the worst good, keeps the leftover, delivers output after every run, perishing by the good's rule"**: `production_valid` world, `works`, orders `[(1, 0), (1, 0), (1, 0)]`, turn 0: records `done` 0.5, 0.5, 0.5; inputs `[(labor, 5), (ore, 10)]`, the same, `[(labor, 3)]`; outputs 5, 5, 3; `wear` 0.25, 0.25, close to 1/3; conditions after 0.75, 0.25, close to 2/3. The stock holds coin 50, ore 10 (`[(end(2), 10)]`), iron 10 (`Never`), tools 3, bread 3 (`[(end(1), 3)]`) and no labor (ore 40 asked against 30 gives 0.75, labor 26 against 13 gives 0.5; every factory is cut by labor, and 10 ore is left).
- **AC "the order of a business's factories doesn't matter"**: the same with the operations reversed gives the reversed records and the same stock.
- **AC "maintenance costs both bundles, is capped, and shares a shortage proportionally"**: `works`, orders `[(0, 1), (0, 0.5), (0, 1)]`, fresh clone, turn 0: nothing runs (`done` 0, output 0, empty inputs); after wear the factories want 0.25 (capped by the room after wear), 0.5 (the construction slice) and 1/3 (the room), needing tools 1 + labor 2, tools 2 + labor 4, tools 1; tools 4 against 3 held gives 0.75, labor 6 against 13 gives 1; `maintenance.done` 0.1875, 0.375, 0.25; costs `[(labor, 1.5), (tools, 0.75)]`, `[(labor, 3), (tools, 1.5)]`, `[(tools, 0.75)]`; conditions after 0.9375, 0.625, close to 11/12; the stock holds labor 8.5, no tools, coin 50, ore 30.
- **AC "a full factory runs exactly its lifetime; one and a half slices run once; a missing maintenance good is a visible cut; the perish group follows the turn"**: `springs` (two no-input springs, L 3, t 4, at conditions 1 and 0.5; the business holds tools 1 and no labor). `operate` with orders `[(1, 1), (1, 1)]` at turn 0: both run (`done` 1, output 5 each), the stock holds ore 10 at `end(2)`; `wear` 1/3 each; both ask maintenance 1 and get 0 (capped to 0.25 by t 4, needing labor 0.5 each; no labor), tools untouched; conditions close to 2/3 and 1/6. Keep the operations (their conditions carry over) and `operate` again with `[(1, 0), (1, 0)]` at turn 1: the first runs (ore 5 more at `end(3)`), the second doesn't (`asked` 1, `done` 0: half a slice); conditions close to 1/3 and exactly 0. Turn 2: the first runs (ore 5 at `end(4)`), then exactly 0. Turn 3: nothing runs; the stock holds ore 20 in groups `end(2)` 10, `end(3)` 5, `end(4)` 5.
- **AC "every existing test passes unchanged"**: no other test changes; the dev UI checker's lines for the sample are unchanged (confirmed by the existing `checker.rs` tests and the manual run in Task 6).

## Constraints

- Follow the existing conventions: doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`; deterministic iteration (`BTreeMap`, slices in order; no `HashMap`).
- `Stockpile::add` and `remove`, `Bundle::new`, `Technology::new` and `NamedList::from_parts` stay `pub(crate)`. `Operation`'s and the records' fields are public. Only `operate` and the types are re-exported at the crate root; `run`, `wear` and `maintain` are reached through `policyspace_engine::production`.
- Do not change `World`, the turn, `TurnReport`, `Factory`, `Factories`, `Technology`, `Stockpile`'s behavior, the setting loader, the sample setting, the dev UI, or any existing test or test setting.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: wiring into the turn and the per-business record in the World (Step 08); choosing the fractions, any cash limit, validating decisions (Steps 05 to 08); redistributing goods left unused after a shared cut; construction, scrapping, retooling (Stage 07); any effect of condition on capacity; any check that a setting's technologies make economic sense; the dev UI (Step 09).

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample's checker success line and turn lines are unchanged, and every Acceptance Criterion in `design.md` is met.
