# The trade round

Source: `crates/engine/src/trade.rs` (the round itself), with the World's side in `crates/engine/src/world.rs` (`World::trade`, `World::business_submission`, `World::operate_business`, `World::pop_outcome`, `World::business_outcome`). The public types `PopOutcome`, `BusinessTrade` and `BusinessOutcome` are re-exported from the engine crate root; `Submission`, `BusinessSubmission`, `HexRound`, `RefusedTrade`, `RefusedTrader` and `hex_round` are crate-private.

The trade round is part 2 of the turn ("actors act", see [world.md](world.md#the-turn)) for POPs and businesses. On every hex, each POP hands the hex's market a **submission**: its type's Stone-Geary parameters, its size and its whole stockpile. Each business on the hex hands it a **business submission**: the offers and bids its decision resolves to, and its whole stockpile. The market posts its prices, computes every POP submission's demand with the `demand` function (see [demand.md](demand.md)), settles **every sale on the hex before any purchase** (POPs, then businesses, in each pass), and then each POP **consumes** the utility goods it ended up with. Right after the hex's round, the World **operates** each business's factories on that hex with the factory mechanics (see [production.md](production.md)). The World keeps each POP's **outcome** (its demand, what it sold, bought and consumed, and its subsistence index and growth rate) and each business's **outcome** (what it offered, sold, bid and bought, the cash cap's proportion, its cash at three points, and one record per factory) until the next turn, and the turn report gains the totals. After every hex has traded, the World applies each POP outcome's growth rate to the POP's size (see [growth.md](growth.md)).

The structure is shaped for the live market of Stage 04. In this Stage the market is the dummy market (see [market.md](market.md)): fixed prices and any amount traded. Stage 04 replaces the inside of the round (adjusted prices, rationing, aggregate curves built from the submissions); the submissions, the sales-before-purchases order, consumption and the recorded outcomes stay, so POPs, businesses and the World don't change when the market does. The business rules below (offers trimmed to holdings, bids capped to cash, shortages shared in the factory mechanics) already cope with being sold or bought less than asked.

`trade.rs` depends only on `demand`, `goods`, `growth`, `market`, `pops` (for `PopType`), `quantity` and `stockpile`. It knows nothing about the `World`, `Pops`, `Businesses`, `IdVec`, deciders or files: the World builds the submissions (resolving each business's decision from names to engine IDs first) and puts the results back. Factory operation is not in `trade.rs` either; the World calls the factory mechanics after the round.

## Where it runs in the turn

`World::new` and `World::step` both run a turn through the private `World::run_turn`, which runs the turn's start (emission, endowment), the decision point, then the trade round with factory operation, then growth, and builds the `TurnReport`. So between calls the world is **after part 2**: every business has traded and operated its factories, every POP has traded, consumed and grown, and each stockpile holds what is left (leftover currency, goods a POP couldn't sell, untradable goods, a business's factory output). One `TurnReport` covers one whole turn's trade. Because growth has already run, a POP's current size is not the size the turn's trade was for; that size is the outcome's `starting_size()`. Likewise a factory's current condition is its condition after this turn's wear and maintenance; the condition it ran at is its record's `condition_before`.

The private `World::trade` does the World's side:

1. For each hex in hex ID order (`map.iter()`), it takes the POPs on the hex (`pops.on_hex`, POP ID order) and the businesses on it (`businesses.on_hex`, business ID order), and skips the hex if there are neither.
2. It builds one `BusinessSubmission` per business with `World::business_submission` (below), which moves the business's stockpile out with `std::mem::take`. This happens before the market is built, because it needs the World mutably.
3. It builds the hex's `Market` directly from the World's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`), not through `World::market`, which would borrow the whole World.
4. It builds one `Submission` per POP. Each submission **owns** the POP's stockpile, moved out with `std::mem::take` (leaving an empty one behind), so the round can hold every submission on the hex at once without borrowing several elements of `pop_stockpiles` mutably.
5. It calls `hex_round` with both slices. It moves each POP's stockpile back, adds each POP outcome's `sold`, `bought` and `consumed` to the turn's accumulators, and stores the outcome by `PopId`.
6. For each business on the hex in ID order, it moves the stockpile back, adds the `BusinessTrade`'s `sold` and `bought` to the business accumulators, and calls `World::operate_business` (below). Each factory record adds `(technology output, output made)` to `produced` when the output is above zero, and its run inputs and maintenance cost to `used`. The `BusinessTrade` and the records become the business's `BusinessOutcome`.
7. After the last hex it rebuilds `pop_outcomes` and `business_outcomes` (every POP and every business is on exactly one hex, so each has an outcome) and returns the seven totals through `totals_by_good` as a private `TradeTotals { sold, bought, consumed, business_sold, business_bought, produced, used }`, which `run_turn` destructures into the report.

Every POP and every business submits, including those on a hex with no market: its `Market` posts no prices, so `demand` keeps everything a POP holds, and a business has no offers or bids there (validation refuses an offer or bid of an unpriced good), so nothing is traded. A business there still operates its factories from what it holds.

## Data model

### `Submission`

```rust
pub(crate) struct Submission<'a> {
    pub pop_type: &'a PopType,   // the utility parameters
    pub size: Quantity,          // N
    pub stockpile: Stockpile,    // moved out of the World for the round, moved back after
}
```

The POP computes nothing itself; its submission is its parameters and holdings, not an answer. The round trades from and into `stockpile`, and consumption removes from it. Businesses submit through the same round in their own form, the `BusinessSubmission`.

### `PopOutcome`

What one turn's round did for one POP. `#[derive(Debug, Clone, PartialEq)]`; the fields are private and an outcome only comes from the round.

| Method | Result |
|---|---|
| `demand() -> &Demand` | The demand the market computed from the submission: income, subsistence cost, supernumerary income, and per good the held, demanded and subsistence quantities (see [demand.md](demand.md)) |
| `sold()`, `bought()`, `consumed() -> &[(GoodId, Quantity)]` | Per good, in good ID order, only amounts greater than zero |
| `sold_of(good)`, `bought_of(good)`, `consumed_of(good) -> Quantity` | The amount for one good; zero if none |
| `carried_over() -> Quantity` | The hex's currency left in the POP's stockpile after consumption |
| `subsistence_index() -> f64` | The POP's subsistence index this turn: 0 at subsistence, positive above, down to −1 below (see [growth.md](growth.md)) |
| `growth_rate() -> f64` | The growth rate the World applies to the size after the round, within `[−decline, max_growth]` of the POP's type |
| `starting_size() -> Quantity` | The size the turn started with: the size the endowment, the demand and the trade were for. The POP's current size is the size after growth |

`World::pop_outcome(pop) -> &PopOutcome` returns the outcome of the current turn's round. It panics with `is not from this list` for a POP from another world. The outcome is replaced every turn; the World's growth step reads its growth rate, and the dev UI's hex panel displays it (see [dev-ui.md](dev-ui.md)).

### `BusinessSubmission`

```rust
pub(crate) struct BusinessSubmission {
    pub offers: Vec<(GoodId, Quantity)>,   // good ID order, only amounts > 0
    pub bids: Vec<(GoodId, Quantity)>,     // good ID order, only amounts > 0
    pub stockpile: Stockpile,              // moved out of the World for the round, moved back after
}
```

The World resolves it from the business's stored answer (see [decider-host.md](decider-host.md#answers-and-the-report)) in the private `World::business_submission`. For `BusinessAnswer::Valid(decision)`, the decision's `offers` and `bids` (tables keyed by good name, see [decider.md](decider.md#the-decision)) become lists of `(GoodId, Quantity)`: each name is looked up in `rules.goods` and each amount wrapped with `Quantity::new`, both guaranteed to succeed by validation (an `expect` names the guarantee); zero amounts are dropped; and the list is sorted by `GoodId`, because the decision's `BTreeMap<String, _>` is in name order, not good ID order. For `Nothing` and `Refused { .. }` both lists are empty. An idle business therefore takes part in the round with nothing to trade: the result is the same as leaving it out, there is one code path, and every business gets an outcome with its cash figures.

### `BusinessTrade`

What one turn's round did for one business. `#[derive(Debug, Clone, PartialEq)]`; the fields are private and a `BusinessTrade` only comes from the round.

| Method | Result |
|---|---|
| `offered()`, `bid() -> &[(GoodId, Quantity)]` | The offers and bids as submitted (good ID order, amounts > 0) |
| `sold()`, `bought() -> &[(GoodId, Quantity)]` | What was actually sold and bought, in the same order, only amounts greater than zero |
| `offered_of(good)`, `sold_of(good)`, `bid_of(good)`, `bought_of(good) -> Quantity` | The amount for one good; zero if none |
| `proportion() -> f64` | The cash cap's scaling of every bid: 1 when the bids were within the cash, below 1 when they were cut, 0 for a business with no currency and some bids |
| `cash_before() -> Quantity` | The hex's currency held at the start of the round |
| `cash_after_sales() -> Quantity` | The currency held after the whole hex's sales pass |
| `cash_after_purchases() -> Quantity` | The currency held after the whole hex's purchase pass |

A cut shows as asked above done: `sold_of(g) < offered_of(g)` when the business held less than it offered, and `bought_of(g) < bid_of(g)` when the cash cap scaled the bids, the way the factory records show a cut order (see [production.md](production.md#data-model)). `cash_after_sales` is read after every sale on the hex, so in Stage 04 it will see the income the whole sales pass produced; with the dummy market it equals the figure right after the business's own sales.

`World::business_outcome(business) -> &BusinessOutcome` (in `world.rs`) holds the `BusinessTrade` beside the factory records: `trade()` gives the `BusinessTrade`, `factories()` the `(FactoryId, FactoryRecord)` of each factory the business owns in factory ID order, and `factory(id)` one factory's record (None if the business doesn't own it). It panics for a business from another world (see [world.md](world.md#what-a-world-holds)).

### `hex_round`

```rust
pub(crate) fn hex_round(
    market: &Market<'_>,
    pops: &mut [Submission<'_>],          // the hex's POPs, in POP ID order
    businesses: &mut [BusinessSubmission], // the hex's businesses, in business ID order
) -> Result<HexRound, RefusedTrade>

pub(crate) struct HexRound { pub pops: Vec<PopOutcome>, pub businesses: Vec<BusinessTrade> }
pub(crate) struct RefusedTrade { pub trader: RefusedTrader, pub error: TradeError }
pub(crate) enum RefusedTrader { Pop(usize), Business(usize) }
```

Runs one hex's round for its POPs and businesses and returns one result per submission, each list in its slice's order. Empty slices give empty lists; either slice may be empty. `RefusedTrade` names the refused submission by kind and index in its slice, and carries the exchange's refusal.

## The passes

`hex_round` makes four passes, each finishing for every submission before the next starts. Within the sales and purchase passes the POPs go first, in slice order, then the businesses, in slice order. Before the sales pass it reads each business's `cash_before`, and after it each business's `cash_after_sales`.

1. **Demand.** `demand(s.pop_type, s.size, s.stockpile.iter(), market)` for every POP submission. The whole stockpile is the holdings, so durable goods and currency carried over from earlier turns count as income, exactly as the endowment does. `Market` is a `PriceList`, so the posted prices go straight in.
2. **Sales.** Every POP (`settle_sales`), then every business (`settle_offers`).
   - `settle_sales`: for each good in the POP's demand, in good ID order, skipping the hex's currency: if the stockpile holds more than the demanded quantity, `Market::sell_down_to(stockpile, good, demanded)` sells everything above it. When the demanded quantity is zero this sells everything, so a good the POP doesn't want never leaves a rounding sliver. Untradable goods are demanded at their held amount, so they are never sold.
   - `settle_offers`: for each offer, the business sells the smaller of the offered amount and what it holds (`Stockpile::total`) with `Market::sell`, so an offer above holdings sells everything held, soonest-perishing groups first, and an offer of a good it doesn't hold sells nothing. The proceeds enter its stockpile as currency of this turn. It returns what was sold, only amounts greater than zero.
3. **Purchases.** Every POP (`settle_purchases`), then every business (`settle_bids`).
   - `settle_purchases`: for each good in the POP's demand, in good ID order, skipping the currency: if the demanded quantity exceeds the holding, the POP buys the difference, **capped at what its currency affords**. If `difference × price` is at most the currency held, `Market::buy` buys the difference; otherwise `Market::buy_for` spends exactly all the currency held. The budget balances only up to floating-point rounding, so without the cap the last purchase could be refused for a few units in the last decimal place. With the dummy market the cap only ever trims a rounding shortfall, which lands on the last good bought; a POP with no currency buys nothing.
   - `settle_bids`: the **cash cap** (see [The cash cap](#the-cash-cap)).
4. **Consumption** (`consume`). For each good in the POP type's utility, the POP consumes the smaller of what it now holds and what it demanded, with `Stockpile::remove` (soonest-perishing group first). Leisure (the endowed good kept) is consumed this way with no special case. Nothing outside the utility is consumed: currency demanded as a parking place for income, a durable good the POP couldn't sell, and untradable goods stay in the stockpile. With the dummy market a POP always holds what it demanded when it consumes; the "smaller of" rule is for Stage 04, where rationing can leave a POP short. Then `carried_over` is read from the stockpile, the POP's subsistence index and growth rate are computed from its demand and its type's growth parameters (`growth::subsistence_index` and `growth::growth_rate`, see [growth.md](growth.md)), and the outcome is built with the submission's size as `starting_size`. The round only records the rate; the World applies it after every hex has traded.

After the four passes, `hex_round` builds one `BusinessTrade` per business, reading `cash_after_purchases` from its stockpile.

Sales settle before purchases **across the whole hex**, POPs and businesses alike, not per trader, because in the live market a seller rationed when selling has less income than it planned, and every purchase must see the income the sales actually produced. With the dummy market the order makes no visible difference, and putting POPs before businesses within a pass only fixes determinism. The order among businesses is invisible too: a business trades only against the market and operates only on its own stockpile, so nothing one business does can reach another in the same turn.

### The cash cap

`settle_bids` values the business's bids at the posted prices **after the hex's sales**: `cost = Σ bid × price` (saturating). If `cost` exceeds the currency the business holds, every bid is scaled by one proportion, `cash / cost`; otherwise the proportion is 1. A business with no currency and some bids gets proportion 0 and buys nothing (`cost > cash` implies `cost > 0`, so there is no division by zero). Then, for each bid in order, it buys `bid × proportion`: with `Market::buy` if that costs at most the currency left, otherwise with `Market::buy_for`, spending exactly what is left. Scaled purchases balance only up to floating-point rounding, so that second branch only ever trims a rounding sliver, as in `settle_purchases`. It returns what was bought (only amounts greater than zero) and the proportion.

One proportion for every bid mirrors the factory mechanics' proportional sharing of a shortage (see [production.md](production.md#running)) and the arithmetic rule's own budgeting (see [python.md](python.md#the-cash-budget)): a cash-short business buys a smaller bundle in the proportions it asked for, rather than all of one good and none of another in good ID order, which would spend cash on inputs its factories then can't use.

### The engine never touches the fractions

The run and maintenance fractions in the decision go to the factory mechanics as they are. When the cash cap has cut the bids, the factories find fewer inputs and maintenance goods than they need, and the mechanics' shortage cuts reduce the run and maintenance fractions done, in the same proportion across the business's factories (see [production.md](production.md#a-safety-net-not-the-normal-path)). So "when cash is short the run level is scaled down" happens through the one safety net that already exists, and the engine never has to know what a bid is for or derive bids from fractions, which the protocol leaves to the decider. The factory record shows the fraction asked (the decision's) above the fraction done.

Offering and bidding the same good needs no special case: the offer is sold in the sales pass and the bid bought in the purchase pass.

### Operating the factories

Operation is not part of `hex_round`. Right after the hex's round, `World::trade` calls the private `World::operate_business` for each business on the hex, in business ID order:

1. It collects the business's factories (`factories.owned_by`, factory ID order). A business with no factories gets an empty list and nothing is touched.
2. For each factory it builds a `production::Operation` with the technology from `rules.technologies`, the factory's current condition, and the fractions the business's answer gives: for a `Valid` decision, the `FactoryOrder` under the factory's ID number (`u32::try_from(id.index())`, the same number the view uses; see [decider.md](decider.md#concepts)); `0` and `0` for a factory the decision doesn't mention, and for every factory of a business whose answer is `Nothing` or `Refused`.
3. It moves the business's stockpile out, calls `production::operate(&mut operations, &mut stockpile, &rules.goods, turn)` (run, then wear, then maintain; see [production.md](production.md#order-within-a-turn)), and moves the stockpile back.
4. It writes each factory's new condition back with the crate-private `Factories::set_condition`, the only change to a factory during play, and returns `(FactoryId, FactoryRecord)` per factory in factory ID order.

Inputs and maintenance goods leave the business's stockpile; the output enters it as goods added this turn, perishing by the output good's own rule. So **this turn's output is sold next turn**: it is in the stockpile at the next decision point, where the view shows it and the decider can offer it. Labor bought in the purchase pass is in the stockpile when the factories run, and whatever is left of it perishes at the end of the turn like any labor.

An idle business (no answer, or a refused one) trades nothing and operates at run 0 and maintenance 0, so its factories only **wear**, one lifetime-slice a turn, and eventually shut down. This happens under every host, the idle host included: a world that has just been created has already worn every factory one slice.

### Slivers

The engine rounds nothing. A business's purchases can leave a rounding sliver of currency, which stays in its stockpile. A decider whose bids don't agree exactly with its fractions can leave a speck of a bought good unused; a speck of labor then perishes at the end of the turn and appears in `perished`. The sample's arithmetic rule does this for the toolmaker's labor (about 2.2e-16 a turn), so the sample's checker lines from turn 1 on start with a tiny `unskilled_labor` amount in `perished` (see [world.md](world.md#setting-checker---turns-n)). The fix belongs in the decider, not in the engine.

### The currency is never traded

Both POP settlement passes skip the hex's currency, validation keeps it out of every decision's offers and bids, and the exchange would refuse it anyway (`IsCurrency`). Proceeds add currency and purchases spend it; because a POP's budget balances, what is left after purchases is the demanded quantity of currency, up to rounding. That covers both cases where the currency is demanded: the parking place when no good with a share is priced (see [demand.md](demand.md)), and a utility that names the currency, in which case it is consumed like any utility good.

### Leftover currency

Whatever currency a POP holds after consumption stays in its stockpile and is reported as `carried_over`. A durable currency (the sample's `silver`) survives the end of the turn and is income in the next turn's demand; a perishable one perishes by its own rule. The engine does no rounding: a typical POP carries over a sliver below `1e-9`, or nothing. A business's currency likewise stays in its stockpile; its cash at each point of the round is in its `BusinessTrade`.

### A refused trade is an engine bug

After the demand, the offers' trim and the caps, no trade can be refused: sales never exceed holdings, purchases are capped, untradable goods and the currency are never traded, and validation has kept unpriced goods and the currency out of every decision. If the exchange refuses anyway, `hex_round` stops at that trade and returns `RefusedTrade`, and `World::trade` panics with `engine bug: the market on hex #<hex> refused a trade for POP #<pop>: <error>` or `engine bug: the market on hex #<hex> refused a trade for business '<name>': <error>`. The World is never handed back half-stepped. `consume` likewise panics with an `engine bug` message if a removal it has already checked fails.

## Determinism

Hexes in hex ID order; within a hex, POPs in POP ID order then businesses in business ID order within each pass; goods in good ID order within each trader; every sale on a hex before any purchase; then POP consumption; then each business on the hex operates its factories, in business ID order and factory ID order: the same setting stepped the same number of times under deciders that answer the same way gives the same stockpiles, conditions, outcomes and reports.

## Worked examples

POPs, from the sample setting, per individual (every number scales with N), at a typical hex's prices (labor 1, grain 2, copper 5, in `silver`):

| POP | Demand | Sold | Bought | Consumed | Left |
|---|---|---|---|---|---|
| `peasant`, holding 10 labor | labor 2.8, grain 3.6 | 7.2 labor for 7.2 silver | 3.6 grain for 7.2 | 2.8 labor, 3.6 grain | a silver sliver at most |
| `artisan` at the center, holding 10 labor | labor 1.5, grain 3, copper 0.5 | 8.5 labor for 8.5 silver | 3 grain for 6, 0.5 copper for 2.5 | 1.5 labor, 3 grain, 0.5 copper | a silver sliver at most |
| `peasant` on (2, -2), no market | labor 10 kept, grain 0 | nothing | nothing | 10 labor | nothing; below subsistence on grain |
| `peasant`, grain at 10 | labor and grain 10/11 each | 100/11 labor | 10/11 grain | 10/11 labor and grain | below subsistence |

Per individual, every turn looks the same, because labor is endowed again each turn and the silver carried over is only a sliver. The totals change from turn to turn because the POPs grow (see [growth.md](growth.md)). At turn 0, with the setting's sizes, the sample reports `sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain` (the 388 peasants with a market at 7.2, 3.6 and 2.8 labor or grain each, plus the size-10 artisan, plus the 120 labor the no-market peasant consumes). At turn 1 the same per-individual amounts times the grown sizes give `sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain`. The POPs' labor never appears in `perished`.

Businesses, from the sample under its `arithmetic` decider (see [python.md](python.md)), at the center's prices (`unimproved_grain` 0.5, `unskilled_labor` 1, `grain` 2, `copper` 5, `tools` 12):

| Business, turn 0 | Holds at the decision point | Sold | Bought | Factory | Cash before → after sales → after purchases |
|---|---|---|---|---|---|
| `center_farm` | 8 `unimproved_grain`, 100 `silver` | nothing | 4 `unimproved_grain`, 7 `unskilled_labor`, 0.5 `tools` | run 1: 12 `grain` from 12 `unimproved_grain` and 6 labor; wear 1/8; maintenance 0.125 (0.5 tools, 1 labor); condition 1 → 1 | 100 → 100 → 85 |
| `center_mine` | 2.5 `unimproved_copper`, 50 `silver` | nothing | 4 `unskilled_labor`, 0.75 `tools` | output 2.5 `copper`; wear 1/8; maintenance 0.375; condition 0.75 → 1 | 50 → 50 → 37 |
| `center_toolmaker` | 200 `silver` | nothing | 8 `copper`, 6.25 `unskilled_labor`, 0.125 `tools` | output 4 `tools`; condition 1 → 1 | 200 → 200 → 152.25 |

At turn 0 nothing has been made yet, so nothing is sold. At turn 1 each business offers last turn's output: the farm sells 12 `grain`, the mine 2.5 `copper` and the toolmaker 3.875 `tools` (keeping what its own maintenance needs), and each ends the round with more cash than at turn 0 (farm 94, mine 43.5, toolmaker 152.5). From then on each turn repeats, and every factory stays at full condition.

## Using outcomes

```rust
let mut host = DeciderHost::idle();
let (world, report) = World::new(load_setting(path)?, &mut host)?;   // turn 0's round, operation and growth have run
let outcome = world.pop_outcome(pop_id);                  // &PopOutcome
let grain = world.rules().goods.id("grain")?;
let bought = outcome.bought_of(grain);
let income = outcome.demand().income();
let poor = outcome.demand().below_subsistence();
let left = outcome.carried_over();                        // in the hex's currency
let traded_for = outcome.starting_size();                 // the size this trade was for
let (u, rate) = (outcome.subsistence_index(), outcome.growth_rate());
let traded = (&report.sold, &report.bought, &report.consumed);

let farm = world.businesses().id("center_farm")?;
let outcome = world.business_outcome(farm);               // &BusinessOutcome
let trade = outcome.trade();                              // &BusinessTrade
let cut = trade.proportion() < 1.0;                       // the cash cap scaled the bids
let (asked, sold) = (trade.offered_of(grain), trade.sold_of(grain));
let cash = (trade.cash_before(), trade.cash_after_sales(), trade.cash_after_purchases());
for (factory, record) in outcome.factories() {            // factory ID order
    let (asked, done) = (record.run.asked, record.run.done);
}
let business_totals = (&report.business_sold, &report.business_bought, &report.produced, &report.used);
```

To change how trade works, change the inside of `hex_round` (the passes, or the `Market` methods they call). To change how a business's decision becomes trades, change `World::business_submission` (resolution) or `settle_offers` and `settle_bids` (trimming and the cash cap); to change what operation is given, change `World::operate_business`. To add a new kind of trader, give it a submission and an outcome of its own, add a slice to `hex_round`, and keep sales before purchases across everyone on the hex. A new decision verb that trades (a budget, say) belongs in the cash cap: apply it where `settle_bids` computes the proportion, so the bids keep their shape.

## Not yet

Price adjustment, rationing, market inventory or any limit on what the market trades, and aggregating submissions into one demand per type (Stage 04; with rationing, the subsistence index will need to reconsider whether it reads what was demanded or what was consumed); saving beyond rounding leftovers; rounding rules or a smallest unit of currency; operating budgets and dividends, which will plug into the cash cap (Stage 06); construction and expansion, which will plug into maintenance (Stage 07); businesses trading with each other directly or across hexes; and per-hex, per-POP or per-business breakdowns in the turn report (those numbers are in `World::pop_outcome` and `World::business_outcome`).

## Tests

- `trade.rs` unit tests run `hex_round` and the private passes against markets from the `pops_trade` world and hand-built stockpiles. For POPs: a single peasant selling, buying and consuming; two POPs on a hex giving the same outcomes as each alone (sales before purchases); a hex without a market; empty slices; a zero demand selling everything with no sliver; the purchase cap reducing a purchase that rounding would make unaffordable, and a rounding surplus left as currency; a POP with no currency buying nothing; consumption taking only utility goods up to the demand, and taking what is held when short; the currency never traded when a utility names it; and `settle_sales` returning a refusal when called against a market that doesn't price a good. For businesses (a `BusinessSubmission` needs no business in the setting): selling offers and buying bids with the three cash figures and the bought group's perish turn; an offer above holdings selling what is held and an offer of a good not held selling nothing; bids beyond cash scaled by one proportion (0.2); no currency buying nothing with proportion 0; a rounding sliver capping the last purchase; empty submissions; a POP and a business on one hex each giving the same result as alone; and a refused business trade reported as `RefusedTrader::Business(0)`. They take `q`, `close` and `no_growth` from `crates/engine/src/test_support.rs`; their `hex(world, x)` and `good(world, name)` helpers, which need a world, stay local.
- `crates/engine/tests/trade.rs`: the sample's turn 0 (every peasant with a market and the artisan, with their outcomes and stockpiles, sizes taken from `starting_size()`), the no-market peasant, the report totals and the perished list at turn 0, and turn 1's totals computed from the grown sizes; three steps repeating the outcomes per individual (each amount and the income divided by `starting_size()`) with two worlds agreeing; the `pops_trade` table at turn 0 and after a step (zero growth, so sizes don't change); currency carried over as income (the miser, the saver); a below-subsistence POP; held goods outside the utility (the collector's gem, the miser's silver); a hex with a market and no POPs; worlds without POPs reporting nothing traded and a population of 0; and `pop_outcome`'s foreign-POP panic. These run under the idle host, so no business trades.
- `crates/engine/tests/business_trade.rs`, with closure deciders on the `business_trade` setting: `works` trading, operating and selling its output the next turn, its cash cycling 100 → 22 → 100, and no labor or bread in `perished`; `poor` bidding beyond its cash, scaled to exactly 0.2 with its run cut to 0.2 through the labor shortage while the fraction asked stays 1, then buying nothing with no cash; `remote` on a hex with no market running from its own labor and its bread perishing; idle businesses trading nothing and their factories wearing to 0, then shut-down factories making nothing although inputs were bought, while maintenance still restores them; a refused decision idling only its business; a POP on the hex unaffected by the businesses; the report's four business totals, and all four empty under the idle host; and two worlds stepped identically. The last test runs the sample under the real `arithmetic` decider (it needs the project's Python, like `external_decider.rs`): the turn-0 figures in [Worked examples](#worked-examples), each business selling and gaining cash at turn 1, every condition staying 1 for three more steps with no decider failure, and two sample worlds agreeing.
- `pops_trade` (`crates/engine/tests/settings/pops_trade/`): goods `labor` (perishes after 1), `grain` (after 3), and durable `copper`, `silver` (every hex's currency) and `gem`; hexes (0, 0) to (4, 0) with markets at different prices, (2, 0) without one, and (4, 0) with a market and no POPs; POP types `peasant`, `artisan`, `saver` (also endowed silver), `collector` (also endowed a gem), `miser` (labor with share 0 and copper, which (3, 0) doesn't price, so its income is parked as silver and accumulates), and `idle` (empty endowment). Every type has zero growth parameters. Its `README.md` entry lists every hex, price and POP.
- `business_trade` (`crates/engine/tests/settings/business_trade/`): goods `coin`, `labor` (perishes after 1), `ore` (after 3), `iron`, `tools`, `bread` (after 2); technologies `smelter` and `bakery` with `production_valid`'s numbers; one `worker` POP at (0, 0); hexes (0, 0), with a market (labor 1, ore 2, iron 6, tools 10, bread 3), and (1, 0) without one; businesses `works` (100 coin, 3 tools; a smelter and a bakery) and `poor` (10 coin, 30 ore; a smelter) at (0, 0), and `remote` (6 labor; a bakery) at (1, 0), every one with decider `rule`. The numbers make `works`'s steady plan cost and earn exactly 78 a turn, `poor`'s bids cost 50 against 10 cash, and `remote` hold labor for exactly one full run. Its `README.md` entry lists the exact values.
