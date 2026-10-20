# Step 08: business-trade — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-08-business-trade/design.md` — the behavior this spec implements: where the business's turn sits in part 2, the settlement passes, resolving a decision (offers trimmed to holdings, the cash cap with one proportion, fractions handed to the mechanics untouched), operating the factories, idle businesses, the per-business outcome, the four report lists, determinism.
- `PROJECT_PLAN/stage-03-factories/step-04-factory-mechanics/docs_changes.md` — the factory mechanics as built (`production::operate`, `Operation`, `FactoryRecord`, the shared-shortage cuts, the tolerance).
- `PROJECT_PLAN/stage-03-factories/step-05-business-view-and-decisions/docs_changes.md` and `PROJECT_PLAN/stage-03-factories/step-07-decider-host/docs_changes.md` — the decision (`Decision`, `FactoryOrder`, good names and factory ID numbers, what validation guarantees), `BusinessAnswer`, the decision point, the recordings and how they are regenerated.
- `docs/trade.md`, `docs/world.md`, `docs/market.md`, `docs/dev-ui.md` — the POP trade round, the three-part turn, the dummy market's exchange (`sell`, `sell_down_to`, `buy`, `buy_for`, `TradeError`), the checker's turn line.
- Code: `crates/engine/src/trade.rs` (all of it: `Submission`, `PopOutcome`, `RefusedTrade`, `hex_round`, `settle_sales`, `settle_purchases`, `consume`, the unit tests), `crates/engine/src/world.rs` (all of it), `crates/engine/src/production.rs` (`Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord`, `operate`; the module docs), `crates/engine/src/market.rs` (`Market`), `crates/engine/src/factories.rs`, `crates/engine/src/decider/host.rs` (`BusinessAnswer`), `crates/engine/src/decider/decision.rs` (`Decision`, `FactoryOrder`), `crates/engine/src/decider/view.rs` (`build`: how factory ID numbers are derived), `crates/engine/src/id.rs` (`Id::from_index`, `IdVec`), `crates/engine/src/stockpile.rs`, `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/production.rs` (the `operations` helper and `assert_record`), `crates/engine/tests/decider_host.rs` (closure deciders, `decision_for`, `answers_of`), `crates/engine/tests/recordings.rs`, `crates/engine/tests/decider.rs`, `crates/engine/tests/factories.rs`, `crates/engine/tests/world.rs` (the determinism test), `crates/engine/tests/settings/README.md`, `crates/engine/tests/settings/production_valid/`, `crates/dev-ui/src/text.rs` (`turn_line` and its tests), `crates/dev-ui/src/session.rs` (tests with sample turn lines), `crates/dev-ui/tests/checker.rs`.

## Approach

- **The hex round takes both kinds of trader.** `trade::hex_round` gains a second slice, the hex's businesses, and runs: POP demand; POP sales then business sales; POP purchases then business purchases; POP consumption and the records. The sales-before-purchases rule across everyone on the hex stays in the one function that Stage 04 replaces. Factory operation is not in the round: the World runs it right after, per business on the hex. Rationale: the design's order, kept in one place (the user's choice).
- **Business submissions are resolved by the World, not the round.** The World turns a `BusinessAnswer` into a `BusinessSubmission` of engine IDs and quantities (good names → `GoodId`, amounts → `Quantity`, both guaranteed by validation; empty for `Nothing` and `Refused`), so `trade.rs` keeps knowing nothing about deciders or names. An idle business takes part with empty offers and bids: the same observable result as skipping it, one code path, and every business gets an outcome with its cash figures.
- **The cash cap is one proportion applied in the purchase pass.** Bids are valued at posted prices after the hex's sales; if the total exceeds the currency held, every bid is scaled by `cash / cost`; each purchase is still capped at the currency actually left (`buy_for`), as POP purchases are. The fractions in the decision reach `production::operate` untouched; the input shortage is what cuts the run.
- **One `BusinessOutcome` per business, kept by the World.** It holds a `BusinessTrade` (what the round did: offered/sold, bid/bought, the proportion, the three cash figures) built by `hex_round`, plus the `FactoryRecord` of each factory from `operate`. The decision, its trace and its errors stay in `business_answer`.
- **Factory conditions are written back** through a new crate-private `Factories::set_condition`, the only mutation of factories in play.
- **The report gains four business lists** (`business_sold`, `business_bought`, `produced`, `used`), separate from the POP lists, and the turn line prints them after `population` as `businesses sold`, `businesses bought`, `produced`, `used`.
- **Tests that read the world after `World::new` now see a world past operation.** Under the idle host factories have worn one slice at turn 0 (nothing else changes). The recordings test must capture turn 0's request at the decision point (through a closure decider) rather than build it from the world afterwards. These are the only existing-test changes besides the longer turn lines.
- **The build runs as one.** The round, the world, the factory write-back and the tests are one piece of work; the dev UI change is one line. No package could be built and tested alone.

## Changes

### New
- `crates/engine/tests/settings/business_trade/` — a test setting for this Step (contents in Interfaces & Data). Add its entry to `crates/engine/tests/settings/README.md`.
- `crates/engine/tests/business_trade.rs` — integration tests (Test Plan).

### Modified
- `crates/engine/src/trade.rs` — `BusinessSubmission`, `BusinessTrade`, `HexRound`, `RefusedTrader`; `hex_round` takes both slices and returns `HexRound`; `settle_offers`, `settle_bids`; module docs (the round is for POPs and businesses); unit tests.
- `crates/engine/src/world.rs` — `BusinessOutcome`; `business_outcomes` field and `business_outcome` accessor; `TurnReport` gains the four lists; `trade` builds business submissions, calls the new `hex_round`, operates each business after its hex's round, records outcomes and totals; `operate_business` and `business_submission` helpers; module docs (part 2 now: decision point, then per hex the round, consumption and operation, then growth; between calls every business has traded and operated); the unit test at the bottom compares `business_outcomes` and conditions too.
- `crates/engine/src/factories.rs` — `set_condition`; module docs (conditions change in play).
- `crates/engine/src/production.rs`, `crates/engine/src/decider/mod.rs`, `crates/engine/src/decider/host.rs`, `crates/engine/src/businesses.rs`, `crates/engine/src/lib.rs` — module docs only: the turn now calls `operate` and resolves the valid decisions (remove "nothing ... yet (Step 08)" wording); `lib.rs` re-exports `BusinessOutcome` and `BusinessTrade`.
- `crates/engine/tests/recordings.rs` — `sample_request` captures the request at turn 0's decision point.
- `crates/engine/tests/decider.rs`, `crates/engine/tests/factories.rs` — expected conditions after `World::new` reflect one slice of wear.
- `crates/engine/tests/production.rs` — the `operations` helper takes conditions from the loaded setting, not the world (task 4).
- `crates/engine/tests/world.rs` — the determinism test also compares every `business_outcome` and every factory's condition.
- `crates/dev-ui/src/text.rs` — `turn_line` prints the four lists; its tests and docs.
- `crates/dev-ui/src/session.rs`, `crates/dev-ui/tests/checker.rs` — expected turn lines gain the four parts; the sample's `--turns 2` lines change (the arithmetic decider now trades and operates).

## Interfaces & Data

### `crates/engine/src/trade.rs`

```rust
/// One business's submission to its hex's market: its offers and bids as the World resolved
/// them from its decision (good ID order, only amounts > 0; both empty for a business that
/// decided nothing or was refused), and its stockpile, moved out for the round.
pub(crate) struct BusinessSubmission {
    pub offers: Vec<(GoodId, Quantity)>,
    pub bids: Vec<(GoodId, Quantity)>,
    pub stockpile: Stockpile,
}

/// What one turn's round did for one business.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessTrade {
    offered: Vec<(GoodId, Quantity)>,   // as submitted
    sold: Vec<(GoodId, Quantity)>,      // amounts > 0 only
    bid: Vec<(GoodId, Quantity)>,       // as submitted
    bought: Vec<(GoodId, Quantity)>,    // amounts > 0 only
    proportion: f64,                    // the cash cap's scaling, 1.0 when within cash
    cash_before: Quantity,              // currency held at the start of the round
    cash_after_sales: Quantity,
    cash_after_purchases: Quantity,
}
impl BusinessTrade {
    pub fn offered(&self) -> &[(GoodId, Quantity)];
    pub fn sold(&self) -> &[(GoodId, Quantity)];
    pub fn bid(&self) -> &[(GoodId, Quantity)];
    pub fn bought(&self) -> &[(GoodId, Quantity)];
    // zero if the good is absent, like PopOutcome's *_of:
    pub fn offered_of(&self, good: GoodId) -> Quantity;
    pub fn sold_of(&self, good: GoodId) -> Quantity;
    pub fn bid_of(&self, good: GoodId) -> Quantity;
    pub fn bought_of(&self, good: GoodId) -> Quantity;
    pub fn proportion(&self) -> f64;
    pub fn cash_before(&self) -> Quantity;
    pub fn cash_after_sales(&self) -> Quantity;
    pub fn cash_after_purchases(&self) -> Quantity;
}

/// The results of one hex's round, each in submission order.
pub(crate) struct HexRound {
    pub pops: Vec<PopOutcome>,
    pub businesses: Vec<BusinessTrade>,
}

/// Which submission the exchange refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusedTrader {
    Pop(usize),
    Business(usize),
}

/// Replaces the `submission: usize` field.
pub(crate) struct RefusedTrade {
    pub trader: RefusedTrader,
    pub error: TradeError,
}

pub(crate) fn hex_round(
    market: &Market<'_>,
    pops: &mut [Submission<'_>],
    businesses: &mut [BusinessSubmission],
) -> Result<HexRound, RefusedTrade>;
```

`hex_round` order:
1. Every POP's demand (as now).
2. Sales: every POP (`settle_sales`, as now), then every business (`settle_offers`), in slice order.
3. Purchases: every POP (`settle_purchases`), then every business (`settle_bids`).
4. POP consumption and `PopOutcome`s (as now); then one `BusinessTrade` per business.

`cash_before` is read before step 2, `cash_after_sales` after the whole sales pass (so that in Stage 04 it sees every sale on the hex; with the dummy market it equals the figure right after the business's own sales), `cash_after_purchases` after step 3, each as `stockpile.total(market.currency())`.

```rust
/// Sells each offered amount, or everything held if that is less. Returns (good, sold) in
/// the offers' order, only amounts > 0.
fn settle_offers(
    market: &Market<'_>,
    offers: &[(GoodId, Quantity)],
    stockpile: &mut Stockpile,
) -> Result<Vec<(GoodId, Quantity)>, TradeError>;
```
Per offer: `held = stockpile.total(good)`; `amount = min(offer, held)`; if `amount > 0`: `market.sell(stockpile, good, amount)?` (it can't refuse `NotEnoughGoods` after the `min`).

```rust
/// Values the bids at posted prices; scales every bid by `cash / cost` when the cost exceeds
/// the currency held (1 otherwise); buys each scaled amount, capped at the currency left.
/// Returns ((good, bought) in the bids' order, only amounts > 0; the proportion).
fn settle_bids(
    market: &Market<'_>,
    bids: &[(GoodId, Quantity)],
    stockpile: &mut Stockpile,
) -> Result<(Vec<(GoodId, Quantity)>, f64), TradeError>;
```
Steps: `cost = Σ bid.saturating_mul(price)` with `price = market.price(good).ok_or(TradeError::NotTraded { good })?`; `cash = stockpile.total(currency)`; `proportion = if cost > cash { cash.get() / cost.get() } else { 1.0 }` (cost > cash implies cost > 0, so no division by zero; cash 0 gives 0). Then per bid: `wanted = bid.saturating_mul(Quantity::new(proportion).expect("a proportion is a valid quantity"))`; skip if zero; `needed = wanted.saturating_mul(price)`; `held = stockpile.total(currency)`; if `needed <= held` → `market.buy(stockpile, good, wanted)?`, bought = wanted; else `bought = market.buy_for(stockpile, good, held)?` (the rounding-sliver case; a business with no currency buys nothing). Push if bought > 0. Empty bids → `(vec![], 1.0)`.

### `crates/engine/src/world.rs`

```rust
/// What the current turn did for one business, after its hex's round and its operation.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessOutcome {
    trade: BusinessTrade,
    /// One record per factory the business owns, in factory ID order.
    factories: Vec<(FactoryId, FactoryRecord)>,
}
impl BusinessOutcome {
    pub fn trade(&self) -> &BusinessTrade;
    pub fn factories(&self) -> &[(FactoryId, FactoryRecord)];
    pub fn factory(&self, id: FactoryId) -> Option<&FactoryRecord>;
}

pub struct TurnReport {
    // ... existing fields through `population` ...
    /// Total sold by businesses per good in the round of `turn`, in good ID order, only
    /// amounts > 0.
    pub business_sold: Vec<(GoodId, Quantity)>,
    /// Total bought by businesses, likewise.
    pub business_bought: Vec<(GoodId, Quantity)>,
    /// Total output made by factories, likewise.
    pub produced: Vec<(GoodId, Quantity)>,
    /// Total inputs and maintenance goods consumed by factories, likewise.
    pub used: Vec<(GoodId, Quantity)>,
    pub decisions: DecisionCounts,
    pub decider_failures: Vec<ReportedFailure>,
}

impl World {
    /// The outcome of the current turn for `business`: its trade and its factories' records.
    ///
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_outcome(&self, business: BusinessId) -> &BusinessOutcome;
}
```

New field `business_outcomes: IdVec<BusinessKind, BusinessOutcome>`, rebuilt every turn (initially empty like `pop_outcomes`; filled by `trade` before `run_turn` returns, so it is never read empty through the public API). `TradeTotals` gains `business_sold`, `business_bought`, `produced`, `used`.

`World::trade` (private) becomes, for every hex in hex ID order, skipping a hex with no POPs and no businesses:
1. `market = Market::new(...)` as now.
2. POP submissions as now; business submissions: `self.businesses.on_hex(hex)` in ID order → `business_submission(id)` for each, with the stockpile `mem::take`n.
3. `hex_round(&market, &mut pops, &mut businesses)`; on `Err(RefusedTrade { trader, error })` panic with `"engine bug: the market on hex #{} refused a trade for POP #{}: {error}"` (as now) or `"engine bug: the market on hex #{} refused a trade for business '{name}': {error}"`.
4. Write every stockpile back; store POP outcomes and extend the POP totals as now; extend `business_sold`/`business_bought` from each `BusinessTrade`.
5. For each business on the hex in ID order: `records = self.operate_business(id)`; extend `produced` with `(technology.output(), record.run.output)` for each record with output > 0, and `used` with every entry of `record.run.inputs` and of `record.maintenance.cost`; store `BusinessOutcome { trade, factories: records }`.

```rust
/// Resolves `business`'s answer into offers and bids by good ID, in good ID order, only
/// amounts > 0; both empty unless the answer is `Valid`. Moves the stockpile out.
fn business_submission(&mut self, business: BusinessId) -> BusinessSubmission;
```
Names resolve with `self.rules.goods.id(name).expect("a valid decision names known goods")`, amounts with `Quantity::new(amount).expect("a valid decision has valid amounts")`; collect, then sort by `GoodId` (the decision's `BTreeMap<String, _>` is name order, not ID order); drop zero amounts.

```rust
/// Operates `business`'s factories with the fractions its answer gives (0 and 0 for a factory
/// it doesn't mention, and for every factory when it decided nothing or was refused),
/// writes the conditions back, and returns the records in factory ID order.
fn operate_business(&mut self, business: BusinessId) -> Vec<(FactoryId, FactoryRecord)>;
```
Factory IDs: `self.factories.owned_by(business)` collected in ID order. The order for factory `id` is `decision.factories.get(&u32::try_from(id.index()).expect("factory IDs fit in a u32"))` (the same number `view::build` uses). Build `Operation { technology: rules.technologies.get(f.technology), condition: f.condition, run_fraction, maintenance_fraction }` per factory, `mem::take` the stockpile, call `production::operate(&mut ops, &mut stockpile, &rules.goods, self.turn)`, put the stockpile back, then `self.factories.set_condition(id, op.condition)` for each. Borrowing: take `let rules = &self.rules;` and index the other fields directly (as `emit` does), so the technology borrows don't conflict with the mutable stockpile and factories. A business with no factories returns an empty `Vec` and touches nothing.

Also: `World::new`'s doc and the module docs describe the new part 2; the unit test `a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged` also snapshots `business_outcomes` and every factory's condition.

### `crates/engine/src/factories.rs`

```rust
impl Factories {
    /// Sets `id`'s condition. Crate-private: only the World's operation changes it.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list, or if `condition` isn't a finite number from 0
    /// to 1 (`"a factory's condition must be a finite number from 0 to 1"`).
    pub(crate) fn set_condition(&mut self, id: FactoryId, condition: f64);
}
```

### `crates/engine/src/lib.rs`

`pub use trade::{BusinessTrade, PopOutcome};` and `pub use world::{BusinessOutcome, StepError, TurnReport, World};`.

### `crates/dev-ui/src/text.rs`

`turn_line` becomes

```
Turn {n}: perished {}; emitted {}; endowed {}; sold {}; bought {}; consumed {}; population {}; businesses sold {}; businesses bought {}; produced {}; used {}
```

with the four new lists from `report.business_sold`, `report.business_bought`, `report.produced`, `report.used`, each through the existing `list` helper (`nothing` when empty).

### The `business_trade` test setting (`crates/engine/tests/settings/business_trade/`)

`setting.toml`: name `Business Trade Test`, format version 1.

`rules/goods.toml` (IDs 0-5): `coin`; `labor` (perishes_after 1); `ore` (perishes_after 3); `iron`; `tools`; `bread` (perishes_after 2).

`rules/technologies.toml` (IDs 0-1): `smelter` (inputs ore 2, labor 1; output iron; capacity 10; capital tools 4; construction labor 8; construction_turns 2; lifetime 4) and `bakery` (inputs labor 1; output bread; capacity 6; capital tools 3; construction {}; construction_turns 1; lifetime 3). The same numbers as `production_valid`'s.

`rules/pops.toml`: one POP type `worker`: endowment labor 10; utility labor (subsistence 2, share 0.2) and bread (subsistence 2, share 0.8); growth `[0]`, decline 0, max_growth 0.

`start/map.toml`: hexes (0, 0) and (1, 0), currency `coin`.

`start/markets.toml`: (0, 0) prices labor 1, ore 2, iron 6, tools 10, bread 3. (1, 0) has no market.

`start/pops.toml`: one `worker` at (0, 0), size 2.

`start/businesses.toml` (IDs 0-2, every decider `rule`, no deposits):
- `works` at (0, 0): inventory coin 100, tools 3.
- `poor` at (0, 0): inventory coin 10, ore 30.
- `remote` at (1, 0): inventory labor 6.

`start/factories.toml` (IDs 0-3): 0 `smelter` `works` condition 1; 1 `bakery` `works` condition 1; 2 `smelter` `poor` condition 1; 3 `bakery` `remote` condition 1.

README entry: list the goods, technologies, the POP type and POP, hexes and prices, businesses and factories as above, and the purpose (the business round and operation, `crates/engine/tests/business_trade.rs`).

Why these numbers: at the posted prices `works`'s steady plan (below) costs 78 a turn (ore 40, labor 18, tools 20) and earns 78 (iron 60, bread 18), all whole numbers, so its cash cycles 100 → 22 → 100 exactly and no rounding sliver appears. `poor`'s bid costs 50 against cash 10, so the proportion is exactly 0.2 and only labor is short, so the run is cut to exactly 0.2. `remote` has labor for exactly one full run.

## Tasks

1. `trade.rs`: `BusinessSubmission`, `BusinessTrade` and accessors, `HexRound`, `RefusedTrader`, the new `hex_round`, `settle_offers`, `settle_bids`; update the module docs; fix the existing unit tests for the new signature (`&mut []` for businesses, `.pops` of the result). Add the unit tests in the Test Plan.
2. `factories.rs`: `set_condition` with a unit test (sets one factory, others unchanged; panics on a bad value).
3. `world.rs`: `BusinessOutcome`, the field and accessor, the report fields, `TradeTotals`, the new `trade`, `business_submission`, `operate_business`, docs, the unit test. `lib.rs` re-exports and docs; docs in `production.rs`, `decider/mod.rs`, `decider/host.rs`. `cargo build -p policyspace-engine`.
4. Fix the existing engine tests: `recordings.rs` (capture the decision-point request: build the sample world under `DeciderHost::new().with("arithmetic", ClosureDecider::new(..))` whose closure clones the request into an `Rc<RefCell<Option<Request>>>` and answers an empty reply; use that request), `decider.rs` and `factories.rs` (conditions after `World::new` under the idle host are one slice lower: sample farm 0.875, mine 0.625, toolmaker 23/24; `production_valid` smelters 0.75 and 0.25, bakery 2/3, springs 2/3 and 1/6; `factories_valid` by its technologies' lifetimes; compare with `close`, not `==`, where the arithmetic isn't exact), `production.rs` (its `operations` helper builds each `Operation` with the condition the setting places the factory at, read from a separately loaded `LoadedSetting`'s `start.factories`, not from the world, so every expected record keeps its numbers; the world still supplies the technologies and the stockpile, which the idle host doesn't change; the check at the end of `every_factory_of_the_sample_can_be_operated_without_labor_nothing_runs_or_is_maintained` that the world's factory is untouched compares against the world's own condition before the call), `world.rs` (determinism compares outcomes and conditions). `cargo test -p policyspace-engine` with every other expectation unchanged.
5. The `business_trade` test setting and its README entry.
6. `crates/engine/tests/business_trade.rs` (Test Plan).
7. Dev UI: `text.rs` `turn_line`, its docs and tests; `session.rs` and `tests/checker.rs` expectations. For the sample's `--turns 2` lines run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2`, check the turn-0 business parts against the figures in the Test Plan (Dev UI), and paste the printed lines.
8. Run `.venv/Scripts/python tools/check.py` until it passes (run `.venv/Scripts/python -m pip install -r requirements.txt` first if `import policyspace` fails). The committed recordings must not need regenerating: if `recordings.rs` says the request is stale, the capture in task 4 is wrong (the decision-point request at turn 0 is the loaded state plus emission, which this Step doesn't change).

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (fmt, clippy, every test, pytest). Engine tests alone: `cargo test -p policyspace-engine`; one file: `cargo test -p policyspace-engine --test business_trade`.

### Unit tests in `trade.rs`

Use the `pops_trade` setting's market at hex (0, 0) (labor 1, grain 2, copper 5, currency silver) and build stockpiles by hand as the existing tests do; a `BusinessSubmission` needs no business in the setting.

- `a_business_sells_its_offers_and_buys_its_bids`: offers copper 2 (held 2), bids grain 3; stockpile silver 10 → sold copper 2, bought grain 3, proportion 1, cash 10 / 20 / 14; stockpile copper 0, grain 3, silver 14; grain's group perishes at the end of turn 2 (bought in turn 0, perishes after 3).
- `an_offer_above_holdings_sells_what_is_held`: offers copper 5, held 2 → sold copper 2, `offered_of(copper) == 5`; an offer of labor 1 holding none → not in `sold()`.
- `bids_beyond_cash_are_scaled_by_one_proportion`: silver 10, bids grain 10 (cost 20) and copper 6 (cost 30): cost 50 → proportion 0.2; bought grain 2, copper 1.2; silver 0 (within 1e-9); cash figures 10 / 10 / ~0.
- `a_business_with_no_currency_buys_nothing`: no silver, bids grain 1 → bought empty, proportion 0, stockpile unchanged.
- `a_rounding_sliver_caps_the_last_purchase`: silver `6 - 1e-9`, bids grain 3 (cost 6) → proportion `(6 - 1e-9) / 6`; no refusal, silver 0 afterwards, grain within 1e-9 of 3.
- `empty_submissions_trade_nothing`: offers and bids empty → all lists empty, proportion 1, three equal cash figures.
- `pops_and_businesses_together_equal_each_alone`: a peasant (labor 10) and a business (silver 10, copper 2; offers copper 2, bids grain 3) in one `hex_round` → the POP's outcome and stockpile equal those of the POP alone, and the business's trade and stockpile equal those of the business alone (the dummy market makes them independent; the test pins that the interleaving is harmless).
- `a_refused_business_trade_names_the_business`: the bare market of hex (2, 0) (no market) with a business bidding grain → `Err(RefusedTrade { trader: RefusedTrader::Business(0), error: TradeError::NotTraded { good: grain } })`. The existing `settle_sales_reports_a_refusal` keeps working; the POP refusal path reports `RefusedTrader::Pop(i)`.

### `crates/engine/tests/business_trade.rs` (`mod common;`; closure deciders as in `decider_host.rs`; the `business_trade` setting unless said otherwise)

Helpers: `rule(f)` builds `DeciderHost::new().with("rule", ClosureDecider::new(f))`; `decision(name, factories: &[(u32, f64, f64)], offers: &[(&str, f64)], bids: &[(&str, f64)]) -> Decision`; `outcome(world, name) -> &BusinessOutcome`; `condition(world, index) -> f64`.

The **steady closure** answers every request, for every business in it, with: `works` → factories {0: (1, 0.25), 1: (1, 1/3)}, offers iron 15 and bread 10, bids ore 20, labor 18 and tools 2; `poor` → factory {2: (1, 0)}, bids labor 10 and tools 4; `remote` → factory {3: (1, 0)}, no offers or bids.

- **AC 2, AC 6 (`works`, turn by turn)** `a_business_trades_operates_and_sells_its_output_next_turn`: under the steady closure. Turn 0: `works`'s trade: offered iron 15 and bread 10, `sold()` empty; bid as sent; bought ore 20, labor 18, tools 2; proportion 1; cash 100 / 100 / 22. Factory 0: run asked 1 done 1, inputs ore 20 and labor 10, output 10; wear 0.25; maintenance asked 0.25 done 0.25, cost tools 1 and labor 2; condition 1 → 1. Factory 1: run 1/1, inputs labor 6, output 6; wear 1/3; maintenance 1/3 done 1/3, cost tools 1; condition 1 → 1. Stockpile after the step: coin 22, iron 10, tools 3, bread 6 (one group, perishes at the end of turn 1), no labor, no ore. `world.factories()` conditions 1 and 1. Turn 1 (`step`): sold iron 10 and bread 6 (offered 15 and 10); cash 22 / 100 / 22; the same factory records; stockpile coin 22, iron 10, tools 3, bread 6 (end of turn 2). The turn-1 report's `perished` has no `labor` and no `bread` (and no `iron`); its `business_sold` = iron 10, bread 6. Step once more: the turn-2 report's `perished` still has no labor; the figures repeat.
- **AC 3** `a_cash_short_business_is_scaled_and_its_run_is_cut`: same world, `poor` at turn 0: bid labor 10 and tools 4 (cost 50) against cash 10 → proportion 0.2; bought labor 2, tools 0.8; cash 10 / 10 / 0 (within 1e-9); factory 2: run asked 1, done 0.2 (within 1e-9), inputs ore 4 and labor 2, output 2; wear 0.25; maintenance 0/0; condition 1 → 0.75. At turn 1 `poor` has no cash: proportion 0, bought empty, run done 0, output 0, condition 0.5.
- **AC 5 (no market)** `a_business_on_a_hex_with_no_market_operates_from_its_stockpile`: `remote` at turn 0: trade empty, proportion 1, cash 0 / 0 / 0; factory 3 run 1 done 1, inputs labor 6, output 6; condition 1 → 2/3. Stockpile bread 6 perishing at the end of turn 1. Turn 1: run asked 1 done 0 (no labor), output 0, condition 1/3. The turn-2 report's `perished` includes bread 6.
- **AC 5 (idle)** `an_idle_business_trades_nothing_and_its_factories_wear_down`: `world_of(business_trade)` (idle host). Every business's trade is empty with proportion 1; every factory record shows run 0/0, output 0, maintenance 0/0, wear one slice; conditions after turn 0: 0.75, 2/3, 0.75, 2/3; stockpiles unchanged (`remote`'s labor still there, it perishes at the end of turn 0). Three more steps under the idle host → every condition 0 (exactly, by the tolerance snap). Then `step` under the steady closure → `works`'s factory records show run asked 1, done 0, output 0, inputs empty (shut down), although it bought its inputs; `produced` in that report is empty. Maintenance still happens, since the decision asks for it and the business can pay: factory 0 condition 0 → 0.25 (maintenance 0.25 done 0.25, cost labor 2 and tools 1), factory 1 condition 0 → 1/3 (maintenance 1/3 done 1/3, cost tools 1), wear 0 for both.
- **AC 5 (refused)** `a_refused_decision_idles_its_business`: a closure that answers `poor` with factory #99 (refused) and the others as the steady closure → `poor`'s answer is `Refused`, its trade empty, its factory wears to 0.75 with run 0/0; `works` trades as in the first test. `decisions == { valid: 2, refused: 1, nothing: 0 }`.
- **AC 4** `pops_on_the_hex_are_unaffected_by_the_businesses`: the `worker`'s `pop_outcome` and stockpile after turn 0 under the steady closure equal those under the idle host (with the dummy market the interleaving changes nothing for the POP; the unit test covers the round itself).
- **AC 7** `the_report_totals_every_business`: the turn-0 report under the steady closure: `business_sold` empty; `business_bought` = labor 20, ore 20, tools 2.8; `produced` = iron 12, bread 12; `used` = labor 26, ore 24, tools 2 (good ID order; `assert_list`). Under the idle host all four are empty at turn 0 and after a step.
- **Determinism** `two_worlds_are_identical`: two `business_trade` worlds under two steady closures stepped 3 times: every report equal (`TurnReport: PartialEq`), every `business_outcome` equal, every business stockpile equal, every condition equal.
- **AC 1 (the sample under the real decider; needs the venv, like `external_decider.rs`; say so in the file's doc)** `the_sample_chain_runs_under_the_arithmetic_decider`: `python::host_for(&load(sample))`; `World::new`; turn 0 (figures from the committed recording's reply): `center_farm` bought tools 0.5, unimproved_grain 4, unskilled_labor 7, sold nothing, proportion 1, cash 100 / 100 / 85; factory 0 run 1/1, output 12, maintenance 0.125/0.125, condition 1 → 1; stockpile after the step holds grain 12 and silver 85 and no tools or labor. `center_mine`: bought tools 0.75, labor 4; cash 50 / 50 / 37; factory 1 output 2.5, wear 0.125, maintenance 0.375 done 0.375, condition 0.75 → 1. `center_toolmaker`: bought copper 8, tools 0.125 (within 1e-9), labor 6.25; cash 200 / 200 / 152.25; factory 2 output 4, condition 1 → 1; stockpile tools 4. No decider failure, `decisions == { valid: 3, .. }`. Then `step` (turn 1): each business's `sold()` is non-empty (the farm sells grain 12, the mine copper 2.5, the toolmaker tools above 3.8), and each one's `cash_after_purchases` is greater than at turn 0 (farm 94, mine 43.5, toolmaker 152.5; the test asserts `>` only, as built; the exact figures were confirmed at Code Review). Step 3 more times: no failures, every condition stays 1 (within 1e-9), every `sold()` non-empty. Two sample worlds under two fresh hosts stepped 3 times have equal reports and outcomes.

### Dev UI
- `text.rs` unit tests: the existing turn-line strings gain `; businesses sold nothing; businesses bought nothing; produced nothing; used nothing` (idle host); add one that formats a hand-built `TurnReport` with non-empty business lists and checks the four parts and their order.
- `session.rs` tests: the sample lines (idle host) gain the same suffix.
- `tests/checker.rs`: `turns_prints_a_line_for_each_turn` (the sample, Python): the three lines as the checker now prints them. Turn 0 must end with `businesses sold nothing; businesses bought 17.25 unskilled_labor, 4 unimproved_grain, 8 copper, 1.375 tools; produced 2.5 copper, 12 grain, 4 tools; used 17.25 unskilled_labor, 2.5 unimproved_copper, 12 unimproved_grain, 8 copper, 1.375 tools` (the toolmaker's bid tails like `0.12499999999999989` don't show in the totals), and turn 1's `perished` must hold `7.5 unimproved_copper, 16 unimproved_grain` (the owned deposits' output is now used), preceded by a deterministic floating-point sliver of `unskilled_labor` (`0.0000000000000002220446049250313`) that the businesses' labor purchases leave; the lines are pasted as printed, slivers included. The other turn-line expectations (`pops_placed`, `valid_minimal`, `turns_valid`) gain the `nothing` suffix.

## Constraints

- Follow the existing conventions: module docs that say what the module is and isn't responsible for, `pub(crate)` for engine-internal types, `Quantity` arithmetic through its saturating/checked methods, `assert_list`/`close` in tests, no `unwrap` on user data.
- Don't change: the decider protocol, `Decision` or its validation, the view, the Python package or the arithmetic rule, the committed recordings, the sample setting, `production.rs`'s functions (docs only), `market.rs`.
- Don't scale fractions in the engine or derive bids from fractions; don't skip idle businesses in a way that leaves them without an outcome.
- Don't touch `docs/` or `manual.ipynb`; `step-docs-draft` handles documentation.
- Out of scope: showing outcomes in the dev UI beyond the turn line (Step 09); rationing or price changes (Stage 04); budgets, dividends, construction (later Stages).
- `build` never commits.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan exists and passes, and every Acceptance Criterion in `design.md` is met.

## Notes from the build

- Two Test Plan expectations were corrected during the build (a shut-down factory is still maintained when its decision asks and the business can pay; the sample's turn-1 and turn-2 `perished` carry a labor sliver). The code didn't change.
- Known, by design: a decider whose bids don't agree exactly with its fractions leaves a floating-point speck of labor that perishes and prints on the turn line. The sample's arithmetic rule does this for the toolmaker (bug 01 in `PROJECT_PLAN/bugs.md`); the engine rounds nothing.
- `crates/engine/src/technologies.rs` still says Step 08 "wires it into the turn" as future work; left for comp-review.
