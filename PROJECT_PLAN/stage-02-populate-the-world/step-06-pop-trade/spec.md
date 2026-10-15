# Step 06: pop-trade — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-06-pop-trade/design.md` — the behavior this spec implements, the worked examples, and the Acceptance Criteria.
- `PROJECT_PLAN/stage-02-populate-the-world/step-05-stone-geary-demand/spec.md` — the demand function as built (`Demand`, `DemandedGood`, `PriceList`, `demand`), which this Step calls.
- `PROJECT_PLAN/stage-02-populate-the-world/step-04-pops/spec.md` — POPs, their stockpiles, the `endowed` report list, the `pops_placed` test setting, and the tests this Step changes.
- `PROJECT_PLAN/stage-02-populate-the-world/step-02-dummy-market/docs_changes.md` — the dummy market as built: `Market`, `Market::price`, `sell`, `sell_all`, `buy`, `TradeError`.
- `docs/world.md` (the turn, `Stockpile`, `TurnReport`, the checker's turn line), `docs/workspace.md` (lints, the check command, test layout), `docs/dev-ui.md` (the turn log).
- Code: `crates/engine/src/world.rs` (the turn: `new`, `step`, `emit`, `endow`, `perish`, `totals_by_good`), `crates/engine/src/market.rs`, `crates/engine/src/demand.rs`, `crates/engine/src/stockpile.rs` (`add`, `remove`, `remove_all`, `total`, `groups`), `crates/engine/src/pops.rs` (`Pops::on_hex`, `PopType::utility`, `PopType::new`), `crates/engine/src/id.rs` (`IdVec`: `from_vec`, `get`, `get_mut`, `iter`), `crates/engine/src/map.rs` (`HexMap::iter`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/pops.rs`, `crates/engine/tests/world.rs`, `crates/engine/tests/demand.rs`, `crates/engine/tests/market.rs`, `crates/engine/tests/settings/README.md`, `crates/engine/tests/settings/pops_placed/`, `crates/dev-ui/src/text.rs` (`turn_line`, `list`), `crates/dev-ui/src/session.rs` (tests assert turn lines), `crates/dev-ui/tests/checker.rs`, `settings/sample/rules/pops.toml`, `settings/sample/start/markets.toml`, `settings/sample/start/pops.toml`.

## Approach

- **A new module `crates/engine/src/trade.rs` holds the round.** It defines the submission (`Submission`), the per-POP outcome record (`PopOutcome`), and `hex_round`, which runs one hex's trade round and consumption: every submission's demand, every POP's sales, every POP's purchases, then every POP's consumption. `market.rs` stays prices plus the exchange; `world.rs` runs the round hex by hex, keeps the outcomes, and fills the report. Rationale: each module keeps one job, and Stage 05 replaces the inside of `hex_round` (prices, settlement) without touching the POPs, the World, or the outcome record.
- **The World takes each POP's stockpile out for the round and puts it back.** A `Submission` owns its `Stockpile` (moved out of `pop_stockpiles` with `std::mem::take`, moved back after). Rationale: the round can hold every submission on the hex at once without borrowing several elements of an `IdVec` mutably.
- **The currency is never traded.** Both settlement loops skip the hex's currency: proceeds add it, purchases spend it, and because the budget balances, what is left after purchases is the demanded quantity of currency up to rounding (the parking case, or a utility that names the currency). Rationale: the exchange refuses to trade the currency against itself, and the algebra makes trading it unnecessary.
- **Sales leave exactly the demanded quantity behind.** A new `Stockpile::retain(good, keep)` removes everything above `keep` (soonest-perishing first) and sets the last kept group so that exactly `keep` remains when the kept amount lies in one group; a new `Market::sell_down_to` wraps it. Rationale: `remove(held − demanded)` can leave `held − (held − demanded) ≠ demanded` by one floating-point ulp, and that sliver would then show in the report as `2.8e-14 unskilled_labor`. `retain` also gives the design's "sell everything when the demand is zero" for free (`keep = 0`).
- **Purchases are capped with an exact-spend method.** A new `Market::buy_for(stockpile, good, spend)` spends exactly `spend` currency and adds `spend / price` of the good. The round calls `buy(diff)` when `diff × price ≤ currency held`, otherwise `buy_for(good, currency held)`. Rationale: capping as `buy(held / price)` can still be refused when `(held / price) × price` rounds above `held`.
- **A refused trade is an engine bug and panics.** `hex_round` returns `Err(RefusedTrade)`; `World` turns it into `panic!("engine bug: ...")` naming the hex, the POP and the refusal. Rationale: the design proves it can't happen; the codebase already panics on caller bugs (foreign IDs), `World::new` stays infallible, and no half-stepped world is ever handed back.
- **Outcomes live in `World` as `IdVec<PopKind, PopOutcome>`,** rebuilt every turn by the round. Every POP is on exactly one hex, so after `World::new` every POP has one. `World::pop_outcome(pop)` returns it. Rationale: Step 07 reads the demand numbers, Step 08 displays the record.
- **`TurnReport` gains `sold`, `bought`, `consumed`** (totals over every POP, good ID order, > 0 only), built with the existing `totals_by_good`. The dev UI's `turn_line` appends `; sold ...; bought ...; consumed ...`.
- **The build runs as one.** The engine round, the World wiring, the report, the UI line, and the test updates are tightly connected and small enough for one builder.

## Changes

### New

- `crates/engine/src/trade.rs` — `Submission`, `PopOutcome`, `RefusedTrade`, `hex_round`, private `settle_sales`, `settle_purchases`, `consume`; unit tests.
- `crates/engine/tests/trade.rs` — integration tests (Test Plan).
- `crates/engine/tests/settings/pops_trade/` — a test setting with markets and POPs for the trade round (described under Interfaces & Data).

### Modified

- `crates/engine/src/stockpile.rs` — add `pub(crate) fn retain`; module doc: goods also leave by consumption; unit tests.
- `crates/engine/src/market.rs` — add `sell_down_to` and `buy_for`; module doc: the trade round in `trade` uses the exchange; unit tests for `TradeError` unchanged.
- `crates/engine/src/world.rs` — field `pop_outcomes`; `TurnReport` gains three lists; `new` and `step` run `trade` after `endow`; new private `trade`; public `pop_outcome`; module doc (part 2 of the turn, the outcomes, determinism); unit tests.
- `crates/engine/src/demand.rs` — module doc: replace "Nothing calls it during the turn yet (the POP trade step does)." with "The trade round (the [`trade`](crate::trade) module) calls it for every POP each turn."
- `crates/engine/src/lib.rs` — `pub mod trade;` (alphabetical, after `stockpile`; before `turn`) and `pub use trade::PopOutcome;`. Crate docs: the `demand` sentence no longer says "nothing calls during the turn yet"; add that the [`trade`] module runs each hex's trade round (every POP submits its parameters and holdings, the market computes demand and settles sales then purchases, the POP consumes) and records each POP's outcome.
- `crates/engine/tests/settings/pops_placed/rules/types.toml` — the `hoarder` type's utility becomes `{ grain = { share = 1 } }`.
- `crates/engine/tests/settings/README.md` — update `pops_placed` (hoarder's utility is grain, so its gold is parked and accumulates; workers consume their labor), add `pops_trade`.
- `crates/engine/tests/pops.rs`, `crates/engine/tests/demand.rs`, `crates/engine/tests/world.rs` — update the tests that assert POP stockpiles or turn reports (listed in the Test Plan).
- `crates/dev-ui/src/text.rs` — `turn_line` appends the three lists; doc comment.
- `crates/dev-ui/src/session.rs` — the two hardcoded turn lines in its tests.
- `crates/dev-ui/tests/checker.rs` — every hardcoded turn line.

Nothing else changes: no setting file under `settings/`, no `docs/`, no `manual.ipynb`, no loader code, no `app.rs`.

## Interfaces & Data

### `crates/engine/src/stockpile.rs`

```rust
/// Removes everything of `good` above `keep`, taking from the group that perishes soonest
/// first, and returns the amount removed (zero, changing nothing, if it holds `keep` or
/// less). When the kept amount lies in a single group, exactly `keep` remains. Crate-private.
pub(crate) fn retain(&mut self, good: GoodId, keep: Quantity) -> Quantity
```

Algorithm: `total = self.total(good)`; if `total <= keep` return `ZERO`. Walk the good's groups from the **latest**-perishing to the soonest (reverse key order), with `kept = ZERO`: if `kept.saturating_add(group) <= keep`, keep the whole group and add it to `kept`; otherwise this group is the boundary: its new amount is `keep.checked_sub(kept)` (remove the group if that is zero), add `group − new amount` to `removed`, and remove every sooner group whole, adding each to `removed`. Drop the good's entry if no group is left. Return `removed`. With one group this leaves the group at exactly `keep` (or removes it when `keep` is zero) and returns `group − keep`.

### `crates/engine/src/market.rs`

```rust
/// Sells everything of `good` above `keep` from `stockpile` (soonest-perishing first), so
/// that `keep` remains, and adds the proceeds in currency. Returns (amount sold, currency
/// received); (0, 0) if the stockpile holds `keep` or less. Same refusals as
/// [`Market::sell`] except `NotEnoughGoods`.
pub fn sell_down_to(&self, stockpile: &mut Stockpile, good: GoodId, keep: Quantity)
    -> Result<(Quantity, Quantity), TradeError>

/// Spends exactly `spend` of the currency from `stockpile` on `good`, adding
/// `spend / price` of it, perishing as a good added this turn does. Returns the amount
/// bought (zero for a zero spend). Refuses `NotEnoughCurrency` if the stockpile holds less
/// than `spend`; same other refusals as [`Market::buy`].
pub fn buy_for(&self, stockpile: &mut Stockpile, good: GoodId, spend: Quantity)
    -> Result<Quantity, TradeError>
```

`sell_down_to`: `trade_price(good)?`, then `let sold = stockpile.retain(good, keep)`, `received = sold.saturating_mul(price)`, `stockpile.add(currency, received, perishes_at(currency))`. `buy_for`: `trade_price(good)?`; `held = stockpile.total(currency)`; if `held < spend` → `NotEnoughCurrency { currency, held, needed: spend }`; `stockpile.remove(currency, spend)` (an `Err` here can't happen after the check: refuse with the same error, as `buy` does); `amount = spend.checked_div(price).unwrap_or(ZERO)` (the price is > 0); `stockpile.add(good, amount, perishes_at(good))`; `Ok(amount)`. `sell`, `sell_all` and `buy` stay as they are.

### `crates/engine/src/trade.rs`

Module doc: the "actors act" part of the turn for POPs. Each POP submits its type's utility parameters, its size and its holdings to its hex's market; the market posts its prices (fixed, this Stage), computes every submission's demand with [`demand`](crate::demand), and settles every POP's sales before any POP's purchases (so that in the live market every purchase sees the income the sales actually produced); then each POP consumes its utility goods up to what it demanded. The currency is never traded: proceeds add it, purchases spend it, what is left is its demanded quantity up to rounding. Hexes are handled by the World in hex ID order; within a hex POPs go in POP ID order and goods in good ID order. Leftover currency stays in the stockpile. A refused trade is an engine bug. Stage 05 replaces the inside of the round (adjusted prices, rationing); the submission, the order, consumption and the outcome stay. Knows nothing about the World or files.

```rust
use crate::demand::{Demand, demand};
use crate::goods::GoodId;
use crate::market::{Market, TradeError};
use crate::pops::PopType;
use crate::quantity::Quantity;
use crate::stockpile::Stockpile;

/// One POP's submission to its hex's market: its type (the utility parameters), its size,
/// and its stockpile, moved out of the world for the round and moved back afterwards. The
/// round trades from and into the stockpile and consumption removes from it.
pub(crate) struct Submission<'a> {
    pub pop_type: &'a PopType,
    pub size: Quantity,
    pub stockpile: Stockpile,
}

/// What one turn's trade round did for one POP, kept by the World until the next turn.
#[derive(Debug, Clone, PartialEq)]
pub struct PopOutcome {
    /// The demand the market computed from the submission.
    demand: Demand,
    /// Per good sold, in good ID order, only amounts > 0.
    sold: Vec<(GoodId, Quantity)>,
    /// Per good bought, in good ID order, only amounts > 0.
    bought: Vec<(GoodId, Quantity)>,
    /// Per good consumed, in good ID order, only amounts > 0.
    consumed: Vec<(GoodId, Quantity)>,
    /// The hex's currency left in the stockpile after consumption.
    carried_over: Quantity,
}

impl PopOutcome {
    pub fn demand(&self) -> &Demand
    pub fn sold(&self) -> &[(GoodId, Quantity)]
    pub fn bought(&self) -> &[(GoodId, Quantity)]
    pub fn consumed(&self) -> &[(GoodId, Quantity)]
    /// The amount of `good` sold; zero if none.
    pub fn sold_of(&self, good: GoodId) -> Quantity
    pub fn bought_of(&self, good: GoodId) -> Quantity
    pub fn consumed_of(&self, good: GoodId) -> Quantity
    pub fn carried_over(&self) -> Quantity
}

/// The exchange refused a trade during the round: an engine bug, since the demand and the
/// purchase cap make every trade acceptable. The stockpiles are left as they were at the
/// refusal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RefusedTrade {
    /// Index of the submission in the slice passed to `hex_round`.
    pub submission: usize,
    pub error: TradeError,
}

/// Runs one hex's trade round and consumption for `submissions` (the hex's POPs in POP ID
/// order). Returns one outcome per submission, in the same order.
pub(crate) fn hex_round(market: &Market<'_>, submissions: &mut [Submission<'_>])
    -> Result<Vec<PopOutcome>, RefusedTrade>
```

`hex_round`, in four passes over the submissions (each in slice order):

1. **Demand:** `demands[i] = demand(s.pop_type, s.size, s.stockpile.iter(), market)`.
2. **Sales:** `sold[i] = settle_sales(market, &demands[i], &mut s.stockpile)?` (map a `TradeError` to `RefusedTrade { submission: i, error }`).
3. **Purchases:** `bought[i] = settle_purchases(market, &demands[i], &mut s.stockpile)?`. This pass starts only after pass 2 has finished for every submission.
4. **Consumption and the record:** `consumed = consume(s.pop_type, &demands[i], &mut s.stockpile)`, `carried_over = s.stockpile.total(market.currency())`, then `PopOutcome { demand, sold, bought, consumed, carried_over }`.

An empty slice returns an empty vector. Private helpers:

```rust
/// Sells, for every good in the demand except the currency, whatever the stockpile holds
/// above the demanded quantity. Returns (good, amount sold) in good ID order, amounts > 0.
fn settle_sales(market: &Market<'_>, demand: &Demand, stockpile: &mut Stockpile)
    -> Result<Vec<(GoodId, Quantity)>, TradeError>
```
For each `g` in `demand.goods()`: skip if `g.good == market.currency()`; `held = stockpile.total(g.good)`; if `held > g.quantity`, `let (sold, _) = market.sell_down_to(stockpile, g.good, g.quantity)?`; push `(g.good, sold)` if `sold > ZERO`. An untradable good has `quantity == held` and is skipped by the comparison; a `NotTraded` error here is therefore a bug and is returned.

```rust
/// Buys, for every good in the demand except the currency, the difference between the
/// demanded quantity and what the stockpile holds, capped at what its currency affords.
/// Returns (good, amount bought) in good ID order, amounts > 0.
fn settle_purchases(market: &Market<'_>, demand: &Demand, stockpile: &mut Stockpile)
    -> Result<Vec<(GoodId, Quantity)>, TradeError>
```
For each `g` in `demand.goods()`: skip the currency; `held = stockpile.total(g.good)`; if `g.quantity > held`: `diff = g.quantity.checked_sub(held)` (Some, since quantity > held); `price = market.price(g.good).ok_or(TradeError::NotTraded { good })?`; `cost = diff.saturating_mul(price)`; `currency_held = stockpile.total(market.currency())`; if `cost <= currency_held` → `market.buy(stockpile, g.good, diff)?` and `bought = diff`; else → `bought = market.buy_for(stockpile, g.good, currency_held)?` (spends everything, possibly zero). Push `(g.good, bought)` if `bought > ZERO`. The cap means a rounding shortfall lands on the last good in ID order and a POP with no currency buys nothing.

```rust
/// Consumes, for every good in the type's utility, the smaller of what the stockpile holds
/// and what was demanded, soonest-perishing first. Returns (good, amount) in good ID
/// order, amounts > 0.
fn consume(pop_type: &PopType, demand: &Demand, stockpile: &mut Stockpile)
    -> Vec<(GoodId, Quantity)>
```
For each `u` in `pop_type.utility()` (sorted by good): `held = stockpile.total(u.good)`, `wanted = demand.quantity_of(u.good)`, `amount = min(held, wanted)` (compare the `Quantity` values); if `amount > ZERO`, `stockpile.remove(u.good, amount)` (it can't fail since `amount <= held`; on `Err` panic with an `engine bug` message) and push `(u.good, amount)`. Goods outside the utility are never consumed, including currency held as the parking place and untradable goods.

Pitfalls:
- Use `stockpile.total(good)` for the current holding in every pass, not `DemandedGood::held` (the holding changes between passes). In pass 2 the two are equal anyway.
- Never call `sell`, `sell_all` or `buy` on the currency good.
- `min` on `Quantity`: `Quantity` is `PartialOrd`, not `Ord`; write `if held < wanted { held } else { wanted }`.

### `crates/engine/src/world.rs`

```rust
pub struct TurnReport {
    pub turn: Turn,
    pub perished: Vec<(GoodId, Quantity)>,
    pub emitted: Vec<(GoodId, Quantity)>,
    pub endowed: Vec<(GoodId, Quantity)>,
    /// Total sold by POPs per good in the trade round of `turn`, on every hex, in good ID
    /// order, only amounts > 0.
    pub sold: Vec<(GoodId, Quantity)>,
    /// Total bought by POPs per good in the trade round of `turn`, likewise.
    pub bought: Vec<(GoodId, Quantity)>,
    /// Total consumed by POPs per good after the trade round of `turn`, likewise.
    pub consumed: Vec<(GoodId, Quantity)>,
}

pub struct World {
    ...existing fields...,
    /// One per POP, by `PopId`: the outcome of the current turn's trade round. Rebuilt
    /// every turn.
    pop_outcomes: IdVec<PopKind, PopOutcome>,
}

impl World {
    /// The outcome of the current turn's trade round for `pop`: its demand, what it sold,
    /// bought and consumed, and the currency it carried over.
    ///
    /// # Panics
    /// Panics if `pop` isn't from this world's POPs.
    pub fn pop_outcome(&self, pop: PopId) -> &PopOutcome

    /// Part 2 of the turn, after `endow`: runs the trade round and consumption on every
    /// hex in hex ID order, records every POP's outcome, and returns the totals
    /// (sold, bought, consumed) per good, each in good ID order.
    fn trade(&mut self) -> (Vec<(GoodId, Quantity)>, Vec<(GoodId, Quantity)>, Vec<(GoodId, Quantity)>)
}
```

- `World::new`: construct with `pop_outcomes: IdVec::from_vec(Vec::new())` (a placeholder for the moment before the round), run `emit`, `endow`, then `trade`, and put the three lists in the report. Doc: "runs turn 0's start and turn 0's trade round."
- `World::step`: after `endow`, `let (sold, bought, consumed) = self.trade();` into the report.
- `trade`: `let mut outcomes: Vec<Option<PopOutcome>> = (0..self.pops.len()).map(|_| None).collect();` and three `Vec<(GoodId, Quantity)>` accumulators. For each `(hex, _)` in `self.map.iter()` (hex ID order): `let ids = self.pops.on_hex(hex);` skip if empty; build the `Market` directly from fields, as `World::market` does (`Market::new(hex, &self.rules.goods, self.hex_currencies.get(hex), self.hex_prices.list(hex), self.turn)`), not via `self.market(hex)` (that borrows all of `self`); build `submissions: Vec<Submission>` from `ids` with `pop_type: self.rules.pop_types.get(pop.pop_type)`, `size: pop.size`, `stockpile: std::mem::take(self.pop_stockpiles.get_mut(id))`; call `hex_round`; on `Err(RefusedTrade { submission, error })` panic with `"engine bug: the market on hex #{} refused a trade for POP #{}: {error}"` using `hex.index()` and `ids[submission].index()`; on `Ok(results)`, for each `(id, submission, outcome)` zipped: move the stockpile back (`*self.pop_stockpiles.get_mut(id) = submission.stockpile`), extend the accumulators with `outcome.sold()`, `bought()`, `consumed()`, and set `outcomes[id.index()] = Some(outcome)`. After the loop: `self.pop_outcomes = IdVec::from_vec(outcomes.into_iter().map(|o| o.expect("every POP is on exactly one hex")).collect())`; return the three `totals_by_good(...)`.
- Field borrows: the loop reads `self.map`, `self.pops`, `self.rules`, `self.hex_currencies`, `self.hex_prices` and writes `self.pop_stockpiles`; access them as fields (disjoint borrows), not through `&self` methods, inside the loop body.
- Module doc: part 2 of the turn is now the trade round and consumption (summarize `trade.rs`'s doc in two sentences and link the module); the World keeps each POP's outcome; `new` runs turn 0's trade round too; between calls the world is after part 2; determinism now also lists hexes in ID order, POPs in ID order within a hex, goods in ID order within a POP, sales before purchases, then consumption. Update the `World` doc list of what the world holds (each POP's latest outcome).

### `crates/dev-ui/src/text.rs`

```rust
/// The line printed for a turn, by `check` and in the window's turn log: what perished,
/// what deposits emitted, what POPs were endowed with, and what POPs sold, bought and
/// consumed, each a list or `nothing`.
pub fn turn_line(world: &World, report: &TurnReport) -> String
// "Turn {}: perished {}; emitted {}; endowed {}; sold {}; bought {}; consumed {}"
```

### The `pops_trade` test setting (`crates/engine/tests/settings/pops_trade/`)

`setting.toml`: `name = "Pops Trade"`, `format_version = 1`.

`rules/goods.toml`, in this order (IDs 0–4): `labor` (`perishes_after = 1`), `grain` (`perishes_after = 3`), `copper` (durable), `silver` (durable), `gem` (durable).

`rules/pops.toml`:

| Type | Endowment | Utility |
|---|---|---|
| `peasant` | labor 10 | labor { subsistence 2, share 0.2 }, grain { subsistence 2, share 0.8 } |
| `artisan` | labor 10 | labor { subsistence 1, share 1 }, grain { subsistence 2, share 4 }, copper { share 5 } |
| `saver` | labor 10, silver 5 | same as peasant |
| `collector` | labor 10, gem 1 | same as peasant |
| `miser` | labor 10 | labor { subsistence 2, share 0 }, copper { share 1 } |
| `idle` | (empty) | grain { share 1 } |

`start/map.toml`: hexes (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), every one with `currency = "silver"`.

`start/markets.toml`: (0, 0) `labor = 1, grain = 2, copper = 5`; (1, 0) `labor = 1, grain = 10`; (2, 0) no market; (3, 0) `labor = 1, grain = 2`; (4, 0) `labor = 1, grain = 2, copper = 5`.

`start/pops.toml`, in this order (IDs 0–7): 0 `peasant` at (0, 0) size 2; 1 `artisan` at (0, 0) size 1; 2 `peasant` at (1, 0) size 1; 3 `idle` at (1, 0) size 3; 4 `peasant` at (2, 0) size 1; 5 `saver` at (3, 0) size 1; 6 `miser` at (3, 0) size 1; 7 `collector` at (3, 0) size 1. (4, 0) has a market and no POPs.

Expected per turn (every turn is the same unless noted; `t` is the turn number):

| POP | Demand (income, cost) | Sold | Bought | Consumed | Stockpile after |
|---|---|---|---|---|---|
| 0 peasant ×2 | labor 5.6, grain 7.2 (20, 12) | labor 14.4 | grain 7.2 | labor 5.6, grain 7.2 | silver < 1e-9, nothing else |
| 1 artisan | labor 1.5, grain 3, copper 0.5 (10, 5) | labor 8.5 | grain 3, copper 0.5 | labor 1.5, grain 3, copper 0.5 | silver < 1e-9 |
| 2 peasant, grain at 10 | labor 10/11, grain 10/11 (10, 22), below subsistence | labor 100/11 | grain 10/11 | labor 10/11, grain 10/11 | silver < 1e-9 |
| 3 idle | income 0, cost 0; grain 0 | — | — | — | empty |
| 4 peasant, no market | labor 10 (kept), grain 0 below subsistence (0, 0) | — | — | labor 10 | empty |
| 5 saver | labor 3.8, grain 5.6 (15, 6) | labor 6.2 | grain 5.6 | labor 3.8, grain 5.6 | silver < 1e-9 |
| 6 miser | labor 2, copper 0, silver 8(t+1) (10 + 8t, 2) | labor 8 | — | labor 2 | silver 8(t+1) exactly |
| 7 collector | labor 2.8, grain 3.6, gem 1 kept (10, 6) | labor 7.2 | grain 3.6 | labor 2.8, grain 3.6 | gem t+1, silver < 1e-9 |

Report totals at every turn: `sold` = labor 44.3 + 100/11 (≈ 53.390909); `bought` = grain 19.4 + 10/11 (≈ 20.309091), copper 0.5; `consumed` = labor 25.7 + 10/11 (≈ 26.609091), grain 19.4 + 10/11, copper 0.5. `perished` at turn ≥ 1: nothing but rounding (labor is sold or consumed whole; the no-market peasant consumes all of it) — assert it is empty or every amount is < 1e-9.

### Sample setting expectations

Peasant sizes other than the no-market hex (2, −2) total 388; the artisan has size 10. At every turn: `sold` = unskilled_labor 388 × 7.2 + 85 = 2878.6; `bought` = copper 5, grain 388 × 3.6 + 30 = 1426.8 (copper has the lower ID); `consumed` = unskilled_labor 388 × 2.8 + 120 + 15 = 1221.4, copper 5, grain 1426.8. Every typical-hex peasant ends with no labor and no grain and silver < 1e-9; the center artisan likewise and no copper; the (2, −2) peasant (POP 2, size 12) ends empty with outcome demand labor 120 kept, grain 0 below subsistence, consumed labor 120. `perished` at turn ≥ 1 is unimproved_copper 10 and unimproved_grain 24 (hex stockpiles) and no labor beyond rounding.

## Tasks

1. `stockpile.rs`: `retain` with doc; module doc sentence (goods also leave by consumption and by sale down to a kept amount); unit tests: `retain_removes_everything_above_keep_from_the_soonest_group_first` (groups end(1) 4 and end(2) 6, keep 7 → removes 3 from end(1), leaves end(1) 1 and end(2) 6, returns 3), `retain_leaves_exactly_keep_in_a_single_group` (one group 10, keep 2.8 → group is exactly `q(2.8)`, returns `10 − 2.8`), `retain_with_zero_keep_removes_the_good`, `retain_at_or_below_keep_changes_nothing` (returns zero, groups unchanged), `retain_drops_whole_groups_when_keep_falls_on_a_boundary` (groups 4 and 6, keep 6 → only end(2) 6 remains).
2. `market.rs`: `sell_down_to` and `buy_for` with docs; module doc sentence. Tests in `crates/engine/tests/market.rs` (Test Plan).
3. `trade.rs`: module doc, `Submission`, `PopOutcome` and accessors, `RefusedTrade`, `hex_round`, `settle_sales`, `settle_purchases`, `consume`. Register in `lib.rs` with the re-export and the crate-doc sentences; update the `demand.rs` module doc.
4. `world.rs`: `TurnReport` fields, `pop_outcomes` field, `pop_outcome`, `trade`, wiring in `new` and `step`, module and struct docs. Update the unit test `a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged` to also compare `pop_outcomes` before and after (derive nothing new: `IdVec` is `PartialEq` when `T` is).
5. `text.rs`: `turn_line` and its doc. Run `cargo test -p policyspace-dev-ui` to see every failing hardcoded line, then update `session.rs` tests and `checker.rs` tests to the new lines (Test Plan).
6. Test setting `pops_trade` as specified; `pops_placed` hoarder utility → grain; `README.md` entries.
7. Unit tests in `trade.rs` (Test Plan). Load `pops_trade` via `load_setting` on `tests/settings/pops_trade` as `world.rs`'s unit tests do, and get a `Market` with `world.market(hex)`.
8. `crates/engine/tests/trade.rs` integration tests, and the updates to `pops.rs`, `demand.rs`, `world.rs` tests (Test Plan).
9. Run `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests) and fix everything it reports. Every public item needs a doc comment.

## Test Plan

Tolerance: `close(a, b)` is `(a − b).abs() < 1e-9`. "Sliver" means a total < 1e-9. Where a list is checked, check goods (by name, in order) and amounts with `close`. Reuse the helper style of `tests/pops.rs` (`world_of`, `hex`, `good`, `pop_id`, `pop_totals`, `named`, `pair`); `tests/trade.rs` gets its own copies plus `outcome(world, index) -> &PopOutcome` and `list(world, &[(GoodId, Quantity)]) -> Vec<(String, f64)>`.

### Unit tests (`trade.rs`, with the `pops_trade` world's markets, hand-built stockpiles via `Stockpile::add`, types via `PopType::new` or `world.rules().pop_types`)

| Test | Criterion | Setup → expectation |
|---|---|---|
| `a_single_peasant_sells_buys_and_consumes` | AC 1 | market of (0, 0), peasant N 1 holding labor 10 at end(0) → outcome sold labor 7.2, bought grain 3.6, consumed labor 2.8 and grain 3.6, demand income 10; stockpile holds no labor, no grain, silver sliver; `carried_over` is that sliver |
| `sales_on_the_hex_settle_before_purchases` | AC 7 | two submissions (peasant N 2, artisan N 1) → outcomes equal those of running each alone on a fresh stockpile; the returned vector is in submission order |
| `a_hex_without_a_market_trades_nothing` | AC 3 | market of (2, 0), peasant holding labor 10 → sold and bought empty, consumed labor 10, demand grain below subsistence, stockpile empty |
| `an_empty_slice_gives_no_outcomes` | edge | `hex_round(&market, &mut [])` → `Ok(vec![])` |
| `a_zero_demand_sells_everything` | design (no sliver) | market of (0, 0), peasant holding labor 10 and copper 1 → copper sold 1 exactly, `stockpile.total(copper) == ZERO` |
| `the_purchase_cap_reduces_rather_than_refuses` | AC 5 | `demand()` for a type with utility grain { share 1 } holding silver 6 at (0, 0)'s market → grain 3; call `settle_purchases` with a stockpile holding silver `6 − 1e-9` → `Ok`, bought grain ≈ 2.9999999995, silver total exactly 0 |
| `a_rounding_surplus_stays_as_currency` | AC 5 | same demand, stockpile holding silver `6 + 1e-9` → bought grain 3, silver total ≈ 1e-9 |
| `a_pop_with_no_currency_buys_nothing` | edge | demand for a peasant holding labor 10, but settle purchases on an empty stockpile → `Ok(empty)`, stockpile still empty |
| `consumption_takes_only_utility_goods_up_to_the_demand` | AC 9 | peasant demand from labor 10 at (0, 0); stockpile holding labor 2.8, grain 5 (more than demanded 3.6), copper 1 → consumed labor 2.8, grain 3.6; left: grain 1.4, copper 1 |
| `consumption_takes_what_is_held_when_short` | design ("smaller of") | same demand; stockpile holding grain 1 → consumed grain 1, labor none; grain gone |
| `the_currency_is_never_traded` | design | market of (3, 0), type with utility labor { share 0.5 }, silver { subsistence 1, share 0.5 } holding labor 10 → no `IsCurrency` error; sold labor, bought empty, consumed includes silver, silver left is a sliver |
| `settle_sales_reports_a_refusal` | design (bug path) | a refusal can't be produced through `hex_round`, so test the helper: compute a `Demand` at (0, 0)'s market for a peasant holding labor 10 and copper 2 (copper priced there, quantity 0), then call `settle_sales` against (3, 0)'s market (copper unpriced) with a stockpile holding the same → `Err(TradeError::NotTraded { good: copper })`; the labor sale before it went through (income was 20 with the copper counted, so the stockpile holds labor 4.8 and silver 5.2) |

### Market tests (`crates/engine/tests/market.rs`, `turns_valid` world as the existing trade tests; `Stockpile::add` is crate-private, so start from the world's hex stockpiles)

| Test | Expectation |
|---|---|
| `sell_down_to_leaves_the_kept_amount_and_pays_the_proceeds` | hex (0, 0)'s stockpile at turn 0 (`daily` 4.5 in one group; daily 2): `sell_down_to(daily, 2.5)` → `Ok((2, 4))`, `daily` total is exactly `q(2.5)` in its original group, gold 4 added `Never` |
| `sell_down_to_at_or_below_keep_sells_nothing` | keep 4.5 and keep 10 → `Ok((0, 0))`, stockpile unchanged |
| `sell_down_to_zero_sells_everything` | → same result as `sell_all` |
| `sell_down_to_refuses_untraded_goods_and_the_currency` | `daily` on hex (2, 0) (no market) → `NotTraded`; `gold` on (0, 0) → `IsCurrency`; the stockpile is unchanged in both cases |
| `buy_for_spends_exactly_and_adds_the_good` | on (0, 0), `sell_all(stew)` first (5 stew → gold 20), then `buy_for(stew, 6)` (stew 4) → `Ok(1.5)`, gold exactly 14, stew 1.5 at end(2) |
| `buy_for_refuses_when_short_and_zero_spend_buys_nothing` | step the world once so (1, 0) holds gold 3: `buy_for(stew, 6)` → `NotEnoughCurrency { held 3, needed 6 }`, unchanged; `buy_for(stew, 0)` → `Ok(0)`, unchanged |

### Integration tests (`crates/engine/tests/trade.rs`)

| Test | Criterion | Expectation |
|---|---|---|
| `a_fresh_sample_world_has_traded_turn_0` | AC 1, 2 | sample: every peasant POP not at (2, −2): stockpile has no `unskilled_labor`, no `grain`, silver sliver; outcome sold labor 7.2 N, bought grain 3.6 N, consumed labor 2.8 N and grain 3.6 N, demand income 10 N, cost 6 N; the artisan (POP 8, size 10): sold labor 85, bought copper 5 and grain 30, consumed labor 15, copper 5, grain 30 |
| `the_no_market_peasant_consumes_its_labor_and_goes_without_grain` | AC 3 | sample POP 2 (size 12): sold, bought empty; consumed labor 120; demand grain quantity 0, `below_subsistence()`; stockpile empty |
| `the_sample_report_totals_pops_in_good_id_order` | AC 10 | turn 0 report: sold `[unskilled_labor 2878.6]`, bought `[copper 5, grain 1426.8]`, consumed `[unskilled_labor 1221.4, copper 5, grain 1426.8]`; turn 1's report equal (lists); turn 1 perished is `[unimproved_copper 10, unimproved_grain 24]` plus nothing else above a sliver |
| `stepping_repeats_the_outcomes_and_two_worlds_agree` | AC 4 | two sample worlds stepped 3 times: equal reports each step; after each step every POP's `pop_outcome` and `pop_stockpile` equal between worlds, and each POP's outcome matches its turn-0 outcome within 1e-9 (compare `sold()`, `bought()`, `consumed()` lists and `demand().income()`; the leftover silver sliver feeds the next turn's income, so the match is not exact) |
| `the_pops_trade_setting_matches_the_table` | AC 1, 3, 6, 7, 8, 9 | the `pops_trade` world at turn 0 and after one step: every row of the expected table (outcomes, stockpiles), and the report totals |
| `currency_carried_over_is_income_next_turn` | AC 6 | `pops_trade` miser (POP 6): turn 0 carried_over 8, demand income 10; after step, demand income 18, silver demanded 16, carried_over 16, sold labor 8 again; saver (POP 5): income 15 at every turn, bought grain 5.6 |
| `a_below_subsistence_pop_scales_the_bundle` | AC 8 | `pops_trade` POP 2: sold labor 100/11, bought grain 10/11, consumed both 10/11, `demand().below_subsistence()` |
| `held_goods_outside_the_utility_stay_or_sell` | AC 9 | `pops_trade` collector (POP 7): gem 1 at turn 0, 2 after a step, never sold or consumed; miser's silver never consumed |
| `a_hex_with_a_market_and_no_pops_is_untouched` | edge | `pops_trade` hex (4, 0): hex stockpile empty, nothing in the report attributable (covered by the totals) |
| `worlds_without_pops_report_nothing_traded` | AC 11 | `turns_valid` and `valid_minimal`: `sold`, `bought`, `consumed` empty at turn 0 and after 2 steps; stockpiles as before |
| `pop_outcome_panics_on_a_pop_from_another_world` | API | like `pop_stockpile_panics_on_a_pop_from_another_world` in `pops.rs`: `#[should_panic(expected = "is not from this list")]` |

### Updates to existing tests

- `tests/pops.rs`:
  - `a_fresh_sample_world_endows_every_pop`: the endowment is now observed through `report.endowed` (unchanged) and each POP's `pop_outcome(id).demand().get(labor).held == 10 N`; replace the stockpile assertions by: each POP's stockpile has no `unskilled_labor` and at most a sliver of silver. Keep the hex stockpile and report assertions.
  - `stepping_the_sample_re_endows_and_perishes_the_labor`: `perished` is now `[unimproved_copper 10, unimproved_grain 24]` (assert no good other than those two has an amount ≥ 1e-9); the center peasant's stockpile has no labor; keep the `endowed` and the two-worlds equality assertions. Rename to `stepping_the_sample_re_endows_and_the_labor_is_used_up`.
  - `the_pops_placed_world_endows_durable_and_perishable_goods`: with the hoarder's new utility: p0 and p2 hold no labor after the round (consumed); p1 keeps labor 8 at end(t) and gold 3(t+1) `Never` as before; p3 empty; hex stockpiles as before. Reports: turn 0 `consumed = [labor 35]`, `sold`/`bought` empty; turn ≥ 1 `perished = [labor 8, grain 2]`, `emitted`, `endowed` unchanged, `consumed = [labor 35]`.
  - `a_world_with_no_pops_has_none_and_endows_nothing`: also assert `sold`, `bought`, `consumed` empty.
- `tests/demand.rs` `a_pops_stockpile_is_a_valid_holdings_list`: the stockpile is empty after turn 0's round, so instead: `world.pop_outcome(id).demand()` for the center peasant (POP 7, size 60) has labor held 600, quantity 168, grain 216, income 600; and `demand(type, size, [(labor, 600)], &market)` equals it (`assert_eq!`). Rename to `the_recorded_demand_matches_the_pure_function`.
- `tests/world.rs`: `sample_setting_steps` now asserts that only the two unimproved goods (10 copper, 24 grain) perish at or above 1e-9, since POPs use up their labor. `reports_match_the_expected_numbers` and `worlds_without_deposits_stay_empty` assert the three new lists are empty for `turns_valid` / `valid_minimal` (wherever they already assert `endowed.is_empty()`). `identical_runs_give_identical_results` is unchanged (reports now include the new lists).
- `crates/dev-ui/src/session.rs` tests: the two sample lines gain `; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain`, and turn 1's `perished` becomes `10 unimproved_copper, 24 unimproved_grain`. The amounts are sums of floating-point products: run the checker on the sample, confirm each printed amount is within 1e-9 of the expected value, and paste the printed line (which may read e.g. `2878.6000000000004`) into the tests.
- `crates/dev-ui/tests/checker.rs`: `pops_placed` lines become `Turn 0: perished nothing; emitted 2 grain; endowed 43 labor, 3 gold; sold nothing; bought nothing; consumed 35 labor` and `Turn 1: perished 8 labor, 2 grain; emitted 2 grain; endowed 43 labor, 3 gold; sold nothing; bought nothing; consumed 35 labor`; the sample lines as in `session.rs`; `valid_minimal` and `turns_valid` lines gain `; sold nothing; bought nothing; consumed nothing`.

Run: `cargo test -p policyspace-engine`, `cargo test -p policyspace-dev-ui`, then `.venv/Scripts/python tools/check.py` for the gate.

## Constraints

- Follow the crate's conventions: doc comments on every public item (clippy runs with `-D warnings` and `missing_docs = "warn"`), `BTreeMap` for anything iterated, every amount a `Quantity` with its checked and saturating arithmetic, no rounding anywhere.
- `trade.rs` imports only `demand`, `goods`, `market`, `pops`, `quantity`, `stockpile`; it knows nothing about `World`, `Pops`, `IdVec` or files. `market.rs` and `stockpile.rs` keep their existing methods and behavior unchanged.
- Don't change the loader, any setting file under `settings/`, `docs/`, `manual.ipynb`, or `crates/dev-ui/src/app.rs`. The checker changes only through `text::turn_line`.
- Don't aggregate submissions, ration, adjust prices, change N, show outcomes in the dev UI, or add saving behavior (later Steps and Stage 05).
- Don't commit.

## Done When

All tasks complete, `tools/check.py` passes (fmt, clippy, every test in the workspace), every test in the Test Plan passes, and every Acceptance Criterion in design.md is met.
