# Stage 02: populate-the-world — Consolidation

Consolidation must not change behavior: every setting loads with the same IDs, the same errors (same text, same order), the same turn reports, the same POP outcomes and sizes, and the dev UI shows the same text and numbers. Every existing test keeps its assertions, except where a recommendation below says exactly which tests change and how.

## Context
Files `build` must read before starting:
- `CLAUDE.md` (Development Loop, Safe points)
- `PROJECT_PLAN/stage-01-create-the-world/consolidate.md` (C04 and C05 set the pattern for shared test helpers that C01 and C02 extend)
- Engine: `crates/engine/src/lib.rs`, `world.rs`, `trade.rs`, `market.rs`, `stockpile.rs`, `demand.rs`, `growth.rs`, `pops.rs`, `currency.rs`, `quantity.rs`, `id.rs`, `setting/reader.rs`, `setting/resources.rs`, `setting/market.rs`, `setting/pops.rs`, `setting/map.rs`, `setting/error.rs`
- Engine tests: `crates/engine/tests/common/mod.rs` and every `crates/engine/tests/*.rs`
- Dev UI: `crates/dev-ui/src/text.rs`, `crates/dev-ui/src/test_support.rs`
- Designs, for the behavior that must not change: `PROJECT_PLAN/stage-02-populate-the-world/step-*/design.md`
- The check command: `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests)

## Recommendations

### C01: Shared helpers for the engine's integration tests
- **Status:** approved
- **Summary:** Six integration test files (`demand.rs`, `growth.rs`, `market.rs`, `pops.rs`, `trade.rs`, `world.rs`) each define their own copies of the same helpers: build a world from a path, find a hex by coordinates, find a good by name, make a `Quantity`, compare floats, find a POP by index, read a POP's outcome, turn a `(GoodId, Quantity)` list into `(name, f64)` pairs. Stage 01's C04 created `tests/common/mod.rs` for exactly this; the Stage 02 Steps added local copies instead of extending it. The copies move into `common`.
- **Changes:**
  - `crates/engine/tests/common/mod.rs`: add these `pub fn`s, each with a one-line doc comment, keeping the existing ones and the `#![allow(dead_code)]`:
    - `world_of(path: &Path) -> (World, TurnReport)`: `World::new(load(path))`.
    - `hex(world: &World, q: i32, r: i32) -> HexId`: `world.map().id(HexCoord { q, r })`, panicking with `"no hex at ({q}, {r})"`.
    - `good(world: &World, name: &str) -> GoodId`: `world.rules().goods.id(name).unwrap()`.
    - `q(value: f64) -> Quantity`: `Quantity::new(value).unwrap()`.
    - `close(a: f64, b: f64) -> bool`: `(a - b).abs() < 1e-9`.
    - `end(t: u64) -> PerishesAt`: `PerishesAt::EndOfTurn(Turn::new(t))`.
    - `pop_id(index: usize, world: &World) -> PopId`: the POP whose `index()` is `index`, panicking with `"no POP {index}"`.
    - `outcome(world: &World, index: usize) -> &PopOutcome`: `world.pop_outcome(pop_id(index, world))`.
    - `named(world: &World, list: &[(GoodId, Quantity)]) -> Vec<(String, f64)>`: each entry as (good name, amount as f64). This replaces `named` in `pops.rs` and `world.rs` and `list` in `trade.rs` and `growth.rs` (identical bodies).
    - `pair(name: &str, amount: f64) -> (String, f64)`.
    - `assert_list(world: &World, actual: &[(GoodId, Quantity)], expected: &[(&str, f64)])`: as in `trade.rs` today (same names in the same order, amounts within `close`), with its doc comment.
    - `stockpile_totals(world: &World, stockpile: &Stockpile) -> Vec<(String, f64)>`: the stockpile's `iter()` as (good name, total). Replaces the bodies of `world.rs::totals`, `pops.rs::pop_totals` and `pops.rs::hex_totals`.
    - `groups(stockpile: &Stockpile, good: GoodId) -> Vec<(PerishesAt, f64)>`: as in `market.rs` today (soonest first). Replaces the bodies of `world.rs::groups` and `pops.rs::pop_groups`.
  - In each of the six test files: delete the local copies of the helpers above and import them from `common`. File-specific helpers stay local, calling the shared ones where they can: `demand.rs` (`sample_world`, `demand_of`, `quantities`, `assert_balances`, `assert_quantities`), `growth.rs` (`size`, `total_size`, `o_start`, `per_individual`, `growth_world`, `table`, `PEASANT_RATE`), `market.rs` (`posted`, `turns_world`, `stock`), `pops.rs` (`s`, `types_of`, `assert_types`, `assert_shares_sum_to_one`, `assert_growth`, `row`, `pops_of`, `pop_row`, `SAMPLE_POPS`, and thin wrappers `pop_groups`, `pop_totals`, `hex_totals` if keeping their signatures reads better than inlining), `trade.rs` (`total`, `assert_only_a_silver_sliver`, `trade_world`, `check_table`), `world.rs` (`totals`, `groups`, `total` as thin wrappers or inlined).
  - Remove imports that become unused (clippy runs with `-D warnings`). `world.rs` has a fully qualified `policyspace_engine::Quantity` in `named`'s signature that goes away with it.
- **Tasks:**
  1. Add the helpers to `common/mod.rs`.
  2. Switch the six files, one at a time, running `cargo test -p policyspace-engine --test <name>` after each.
- **Tests:** No test is added or removed. The same number of engine integration tests pass with the same assertions.

### C02: Shared helpers for the engine's unit tests
- **Status:** approved
- **Summary:** Inside the engine crate, the `demand` and `growth` test modules carry an identical hand-written `Prices` price list (about 20 lines) and the same sample POP type builders, and seven modules repeat the same `q`, `good`, `close`, `end`, `after` and `no_growth` helpers. A test-only support module, like the dev UI's `test_support.rs`, holds each once.
- **Changes:**
  - New `crates/engine/src/test_support.rs`, declared in `lib.rs` as `#[cfg(test)] pub(crate) mod test_support;` (after the existing `pub mod` lines). Module doc: "Helpers shared by the engine's unit tests." Everything in it is `pub(crate)`:
    - `fn q(value: f64) -> Quantity`.
    - `fn close(a: f64, b: f64) -> bool` (`< 1e-9`).
    - `fn good(index: u32) -> GoodId` (`GoodId::from_index`).
    - `fn hex(index: u32) -> HexId` (`HexId::from_index`).
    - `fn end(t: u64) -> PerishesAt` and `fn after(n: u32) -> Perishes` (`Perishes::After(NonZeroU32::new(n).unwrap())`).
    - `fn no_growth() -> GrowthParams` (`GrowthParams::new(vec![0.0], 0.0, 0.0)`).
    - `const LABOR: u32 = 0; GRAIN: u32 = 1; COPPER: u32 = 2; SILVER: u32 = 3;` (the good indexes the demand and growth tests use).
    - `struct Prices { currency: GoodId, table: BTreeMap<GoodId, Quantity> }` with the `PriceList` impl, as in `demand.rs` today.
    - `fn prices(list: &[(u32, f64)]) -> Prices` (currency `good(SILVER)`), `fn typical() -> Prices` (labor 1, grain 2, copper 5), `fn peasant() -> PopType` (the `demand.rs` version, with the 10-labor endowment; the growth tests never read the endowment, so they get the same results), `fn type_of(utility: &[(u32, f64, f64)]) -> PopType`, `fn held(list: &[(u32, f64)]) -> Vec<(GoodId, Quantity)>`.
  - Switch these test modules to `use crate::test_support::*;` (or named imports) and delete their local copies: `currency.rs` (`good`, `hex`; they take `usize` today, integer literals infer `u32`), `market.rs` (`good`, `hex`, `q`), `pops.rs` (`good`, `q`, `close`, `no_growth`), `stockpile.rs` (`q`, `good`, `after`, `end`), `demand.rs` (`good`, `no_growth`, `q`, `close`, `Prices`, `prices`, `typical`, `peasant`, `type_of`, `held`, the four constants), `growth.rs` (the same set plus its own `g()` stays local), `trade.rs` (`no_growth`, `q`, `close`; its `hex(world, x)` and `good(world, name)` take a world and stay local).
  - Keep `demand.rs`'s `assert_quantities` and `assert_balances` local; nothing else uses them.
- **Tasks:**
  1. Add `test_support.rs` and declare it.
  2. Switch the seven modules, running `cargo test -p policyspace-engine --lib` after each.
- **Tests:** No test is added or removed. The same number of engine unit tests pass with the same assertions.

### C03: One turn runner in `World`
- **Status:** approved
- **Summary:** `World::new` and `World::step` both run `emit`, `endow`, `trade`, `grow` in that order and assemble the same seven-field `TurnReport` by hand, and `trade` returns a three-tuple that needs `#[allow(clippy::type_complexity)]`. One private function runs a turn's start and part 2 and builds the report; `trade` returns a small named struct.
- **Changes:** `crates/engine/src/world.rs`:
  - Add a private `struct TradeTotals { sold: Vec<(GoodId, Quantity)>, bought: Vec<(GoodId, Quantity)>, consumed: Vec<(GoodId, Quantity)> }` with a one-line doc ("The trade round's totals per good, each in good ID order").
  - `fn trade(&mut self) -> TradeTotals`; drop the `#[allow(clippy::type_complexity)]`. The body is unchanged apart from the return.
  - Add `fn run_turn(&mut self, perished: Vec<(GoodId, Quantity)>) -> TurnReport` with a doc comment: "Runs the current turn's start (emission, endowment) and part 2 (the trade round, consumption, growth) and returns its report; `perished` is what the previous turn's end removed." It calls `emit`, `endow`, `trade`, `grow` in that order and builds the `TurnReport` (`turn: self.turn`).
  - `new` ends with `let report = world.run_turn(Vec::new()); (world, report)`. `step` becomes: `let next = ...?; let perished = self.perish(); self.turn = next; Ok(self.run_turn(perished))`.
  - Module docs are unchanged (apart from C06).
- **Tasks:**
  1. Add `TradeTotals` and change `trade`'s return.
  2. Add `run_turn` and use it from `new` and `step`.
- **Tests:** No new tests. `crates/engine/tests/world.rs`, `trade.rs`, `growth.rs`, `pops.rs` (exact reports, determinism, sizes) and the dev UI's exact turn-line tests pass unchanged.

### C04: One validation helper for positive and non-negative numbers in the loader
- **Status:** approved
- **Summary:** Four readers hand-write the same check and message for "a finite number greater than 0" (resource `output`, POP `size`, market `prices` values, POP type `endowment` amounts), and two write "a finite number of at least 0" (utility `subsistence`/`share`, growth `max_growth`). Two helpers on `EntryReader` produce byte-identical messages.
- **Changes:**
  - `crates/engine/src/setting/reader.rs`: add to `EntryReader`:
    - `pub fn positive_quantity(&mut self, prefix: &str, value: Located<f64>) -> Option<Quantity>`: returns `Quantity::new(value.value)` when it is finite and greater than 0; otherwise reports `"{prefix} must be a finite number greater than 0"` at `value.at` and returns None. Doc: "`prefix` is the same text `number_value` uses: `key 'output' in resource`, `key 'grain' in prices of market`."
    - `pub fn non_negative_number(&mut self, prefix: &str, value: Located<f64>) -> Option<f64>`: returns the value when it is finite and at least 0; otherwise reports `"{prefix} must be a finite number of at least 0"` at `value.at` and returns None.
    - Unit tests in the reader's `tests` module: each helper accepts a valid value and reports exactly the message above (at the value's position) for a negative, zero (positive only), `inf` and `nan` value.
  - Call sites, keeping every message exactly as today:
    - `setting/resources.rs` `ResourcesReader::read`: `output` becomes `reader.required_number("output").and_then(|v| reader.positive_quantity("key 'output' in resource", v))`.
    - `setting/pops.rs` `PopsReader::read`: `size` likewise with prefix `"key 'size' in pop"`.
    - `setting/market.rs` `MarketsReader::read`: each price with prefix `format!("key '{}' in prices of market", name.value)`.
    - `setting/pops.rs` `PopTypesReader::read`: each endowment amount with prefix `format!("key '{}' in endowment of {}", key.value, reader.label())`.
    - `setting/pops.rs` `amount`: after `optional_number`, `sub.non_negative_number(&format!("key '{key}' in {}", sub.label()), value)` maps Some to `Amount::Valid`, None to `Amount::Invalid`.
    - `setting/pops.rs` `read_growth`: `max_growth` uses `sub.non_negative_number(&format!("key 'max_growth' in {label}"), v)`. (`decline` keeps its own message, which has a different range.)
  - The error sink sorts errors by position, so the order of reports is unchanged.
- **Tasks:**
  1. Add the two helpers and their unit tests.
  2. Switch the six call sites.
- **Tests:** The new reader unit tests. Every `every_*_error_is_reported_at_its_place` integration test (`resources.rs`, `market.rs`, `pops.rs`) passes unchanged: they pin each message and position.

### C05: Remove the unused "sell everything" operation
- **Status:** approved
- **Summary:** Step 02 added `Market::sell_all` and `Stockpile::remove_all`. Step 06 added `Market::sell_down_to(keep)` and `Stockpile::retain(keep)`, which with `keep` of zero do the same thing, and the trade round uses only those. The older pair is now called by tests only. Removing it narrows the public API that the Step 02 design named ("a way to sell everything"); the user approved this knowing the capability remains as `sell_down_to(stockpile, good, Quantity::ZERO)`. `Market::sell` (an exact amount) stays, as the mirror of `buy`.
- **Changes:**
  - `crates/engine/src/market.rs`: delete `sell_all`. In the module doc and `sell_down_to`'s doc nothing refers to it; leave them.
  - `crates/engine/src/stockpile.rs`: delete `remove_all`. In the module doc's last bullet, "which the market uses to sell goods and pay currency, by sale down to a kept amount, and by consumption" is still right; leave it. Delete the unit tests `remove_all_returns_the_total_and_drops_the_good` and `remove_all_of_a_good_not_held_returns_zero` (`retain_with_zero_keep_removes_the_good` and `retain_at_or_below_keep_changes_nothing` cover the same cases).
  - `crates/engine/tests/market.rs`:
    - `selling_removes_the_good_and_adds_currency`: the "Selling everything" block calls `market.sell_down_to(&mut pile, daily, Quantity::ZERO)` instead of `sell_all`, with the same expected results (`Ok((q(2.0), q(1.0)))`, then `Ok((Quantity::ZERO, Quantity::ZERO))` for the good no longer held).
    - `untraded_goods_and_the_currency_are_refused`: delete the two `sell_all` assertions (`sell_down_to_refuses_untraded_goods_and_the_currency` already checks the same refusals for `sell_down_to`).
    - `sell_down_to_zero_sells_everything`: no longer compares with `sell_all`. It records `let held = all.total(daily)` and `let price = market.price(daily).unwrap()` before the call, asserts the call returns `Ok((held, held.saturating_mul(price)))`, that `down.total(daily)` is zero afterwards and `down.groups(daily).count()` is 0, and that the currency total rose by `held × price`. Drop the second stockpile.
    - `buy_for_spends_exactly_and_adds_the_good`: `sell_all(&mut pile, stew)` becomes `sell_down_to(&mut pile, stew, Quantity::ZERO)` with the same expected `Ok((q(5.0), q(20.0)))`.
  - Documentation: the Step 02 and Step 06 `docs_changes.md` describe `sell_all` and `remove_all`. `update-docs` must leave them out of `docs/` and describe "sell everything" as `sell_down_to` with a zero `keep`.
- **Tasks:**
  1. Remove the two functions and the two stockpile unit tests.
  2. Rewrite the four `tests/market.rs` tests as above.
- **Tests:** Two engine unit tests are removed; no integration test is removed. `cargo test -p policyspace-engine` passes, and `crates/engine/tests/trade.rs` (which exercises the trade round's real sales path) passes unchanged.

### C06: Stale comments
- **Status:** approved
- **Summary:** Four comments say something that stopped being true during the Stage. Only comment text changes.
- **Changes:**
  - `crates/engine/src/world.rs` module doc: "later Steps change them (deposit ownership in Stage 02)" becomes "later Stages change them (deposit ownership in Stage 04)"; the last bullet "Stage 02 will send an owned deposit's output to its owner instead of the hex stockpile." becomes "Stage 04 (businesses) will send an owned deposit's output to its owner instead of the hex stockpile."
  - `crates/engine/src/stockpile.rs` module doc, first bullet: "It is a general type: each hex has a stockpile of unowned goods (see the [`world`](crate::world) module), and later Stages give POPs and industries their own." becomes "It is a general type: each hex has a stockpile of unowned goods and each POP has its own (see the [`world`](crate::world) module); later Stages give industries theirs."
  - `crates/engine/src/setting/map.rs`, `coordinate`'s doc: "Shared by hexes and deposits; the message names the entry's label." becomes "Shared by every section placed by coordinates (hexes, deposits, markets, POPs); the message names the entry's label."
  - `crates/dev-ui/src/text.rs`, `deposit_lines`: the comment "Quantity has no multiply, so the total is the output added `count` times, which is how emission sums it." becomes "The total is the output added `count` times rather than multiplied, so it matches the hex stockpile's total, which emission builds by adding each deposit's output in turn." The code stays as it is.
- **Tasks:**
  1. Edit the four comments.
- **Tests:** None; `cargo doc`-style links are unchanged. The full suite passes unchanged.

### C07: Shared reader for sections placed by coordinates
- **Status:** rejected
- **Reason rejected:** Deposits, markets and POPs each check duplicates and report "no hex at" the same way, but their duplicate messages differ per kind and the shared part is small. Stage 01's C09 rejected this for two readers; three doesn't change the trade-off.

### C08: Shared by-hex index builder for `Pops` and `Deposits`
- **Status:** rejected
- **Reason rejected:** About ten identical lines in `Pops::from_pops` and `Deposits::from_deposits`; a shared builder would be cosmetic.

### C09: Dev UI table helper for the three grids
- **Status:** rejected
- **Reason rejected:** The Goods, Resource types and POP types grids share only the "headings, then rows" shape; their cells differ (hover text, vertical lists). Cosmetic.

## Done When
Every approved recommendation (C01–C06) is applied, `.venv/Scripts/python tools/check.py` passes (fmt, clippy with `-D warnings`, all tests), and no test's assertions changed except as C05 lists.
