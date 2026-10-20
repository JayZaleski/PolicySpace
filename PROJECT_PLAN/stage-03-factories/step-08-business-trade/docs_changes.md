# Step 08: business-trade — Docs Changes

This Step builds on earlier drafts in this Stage that are not in `docs/` yet: Step 02's `docs/businesses.md`, Step 03's `docs/factories.md`, Step 04's `docs/production.md`, Step 05's `docs/decider.md`, Step 06's `docs/python.md`, Step 07's `docs/decider-host.md`, and their edits to `world.md`, `dev-ui.md`, `technologies.md`, `workspace.md`, `setting-loader.md` and `README.md`. Changes to those files below are relative to those drafts and name the sections they define. `docs/trade.md` exists in `docs/` and is changed directly; it becomes the home of the business round, since the business trades in the same hex round as the POPs.

## docs/trade.md — modify
- **Location:** the "Source:" paragraph and the two introduction paragraphs after it (up to, not including, "`trade.rs` depends only on ...")
- **Change:** replace

Source: `crates/engine/src/trade.rs` (the round itself), with the World's side in `crates/engine/src/world.rs` (`World::trade`, `World::business_submission`, `World::operate_business`, `World::pop_outcome`, `World::business_outcome`). The public types `PopOutcome` and `BusinessTrade` are re-exported from the engine crate root (and `BusinessOutcome` from `world`); `Submission`, `BusinessSubmission`, `HexRound`, `RefusedTrade`, `RefusedTrader` and `hex_round` are crate-private.

The trade round is part 2 of the turn ("actors act", see [world.md](world.md#the-turn)) for POPs and businesses. On every hex, each POP hands the hex's market a **submission**: its type's Stone-Geary parameters, its size and its whole stockpile. Each business on the hex hands it a **business submission**: the offers and bids its decision resolves to, and its whole stockpile. The market posts its prices, computes every POP submission's demand with the `demand` function (see [demand.md](demand.md)), settles **every sale on the hex before any purchase** (POPs, then businesses, in each pass), and then each POP **consumes** the utility goods it ended up with. Right after the hex's round, the World **operates** each business's factories on that hex with the factory mechanics (see [production.md](production.md)). The World keeps each POP's **outcome** (its demand, what it sold, bought and consumed, and its subsistence index and growth rate) and each business's **outcome** (what it offered, sold, bid and bought, the cash cap's proportion, its cash at three points, and one record per factory) until the next turn, and the turn report gains the totals. After every hex has traded, the World applies each POP outcome's growth rate to the POP's size (see [growth.md](growth.md)).

The structure is shaped for the live market of Stage 05. In this Stage the market is the dummy market (see [market.md](market.md)): fixed prices and any amount traded. Stage 05 replaces the inside of the round (adjusted prices, rationing, aggregate curves built from the submissions); the submissions, the sales-before-purchases order, consumption and the recorded outcomes stay, so POPs, businesses and the World don't change when the market does. The business rules below (offers trimmed to holdings, bids capped to cash, shortages shared in the factory mechanics) already cope with being sold or bought less than asked.

## docs/trade.md — modify
- **Location:** the paragraph "`trade.rs` depends only on ..."
- **Change:** replace

`trade.rs` depends only on `demand`, `goods`, `growth`, `market`, `pops` (for `PopType`), `quantity` and `stockpile`. It knows nothing about the `World`, `Pops`, `Businesses`, `IdVec`, deciders or files: the World builds the submissions (resolving each business's decision from names to engine IDs first) and puts the results back. Factory operation is not in `trade.rs` either; the World calls the factory mechanics after the round.

## docs/trade.md — modify
- **Location:** "Where it runs in the turn", whole section body
- **Change:** replace

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

## docs/trade.md — modify
- **Location:** "Data model", "`Submission`", the last sentence of the paragraph after the code block ("Industries (Stage 03) are expected to submit through the same round in their own form.")
- **Change:** replace

Businesses submit through the same round in their own form, the `BusinessSubmission`.

## docs/trade.md — modify
- **Location:** "Data model", new subsections after "`PopOutcome`" (after the `World::pop_outcome` paragraph), before "`hex_round`"
- **Change:** add

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

A cut shows as asked above done: `sold_of(g) < offered_of(g)` when the business held less than it offered, and `bought_of(g) < bid_of(g)` when the cash cap scaled the bids, the way the factory records show a cut order (see [production.md](production.md#data-model)). `cash_after_sales` is read after every sale on the hex, so in Stage 05 it will see the income the whole sales pass produced; with the dummy market it equals the figure right after the business's own sales.

`World::business_outcome(business) -> &BusinessOutcome` (in `world.rs`) holds the `BusinessTrade` beside the factory records (see [world.md](world.md#what-a-world-holds)).

## docs/trade.md — modify
- **Location:** "Data model", "`hex_round`", whole subsection body
- **Change:** replace

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

## docs/trade.md — modify
- **Location:** "The four passes", the heading and the first paragraph ("`hex_round` makes four passes ...") and the list items 2 and 3
- **Change:** replace the heading with "## The passes", the first paragraph with the text below, and items 2 and 3 with the items below (items 1 and 4 stay)

`hex_round` makes four passes, each finishing for every submission before the next starts. Within the sales and purchase passes the POPs go first, in slice order, then the businesses, in slice order. Before the sales pass it reads each business's `cash_before`, and after it each business's `cash_after_sales`.

2. **Sales.** Every POP (`settle_sales`), then every business (`settle_offers`).
   - `settle_sales`: for each good in the POP's demand, in good ID order, skipping the hex's currency: if the stockpile holds more than the demanded quantity, `Market::sell_down_to(stockpile, good, demanded)` sells everything above it. When the demanded quantity is zero this sells everything, so a good the POP doesn't want never leaves a rounding sliver. Untradable goods are demanded at their held amount, so they are never sold.
   - `settle_offers`: for each offer, the business sells the smaller of the offered amount and what it holds (`Stockpile::total`) with `Market::sell`, so an offer above holdings sells everything held, soonest-perishing groups first, and an offer of a good it doesn't hold sells nothing. The proceeds enter its stockpile as currency of this turn. It returns what was sold, only amounts greater than zero.
3. **Purchases.** Every POP (`settle_purchases`), then every business (`settle_bids`).
   - `settle_purchases`: for each good in the POP's demand, in good ID order, skipping the currency: if the demanded quantity exceeds the holding, the POP buys the difference, **capped at what its currency affords**. If `difference × price` is at most the currency held, `Market::buy` buys the difference; otherwise `Market::buy_for` spends exactly all the currency held. The budget balances only up to floating-point rounding, so without the cap the last purchase could be refused for a few units in the last decimal place. With the dummy market the cap only ever trims a rounding shortfall, which lands on the last good bought; a POP with no currency buys nothing.
   - `settle_bids`: the **cash cap** (see [The cash cap](#the-cash-cap)).

After the four passes, `hex_round` builds one `BusinessTrade` per business, reading `cash_after_purchases` from its stockpile.

## docs/trade.md — modify
- **Location:** "The four passes" (now "The passes"), the paragraph after the list ("Sales settle before purchases **across the whole hex**, not per POP, ...")
- **Change:** replace

Sales settle before purchases **across the whole hex**, POPs and businesses alike, not per trader, because in the live market a seller rationed when selling has less income than it planned, and every purchase must see the income the sales actually produced. With the dummy market the order makes no visible difference, and putting POPs before businesses within a pass only fixes determinism. The order among businesses is invisible too: a business trades only against the market and operates only on its own stockpile, so nothing one business does can reach another in the same turn.

## docs/trade.md — modify
- **Location:** new subsections after the paragraph above, before "The currency is never traded"
- **Change:** add

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

## docs/trade.md — modify
- **Location:** "A refused trade is an engine bug", whole subsection body
- **Change:** replace

After the demand, the offers' trim and the caps, no trade can be refused: sales never exceed holdings, purchases are capped, untradable goods and the currency are never traded, and validation has kept unpriced goods and the currency out of every decision. If the exchange refuses anyway, `hex_round` stops at that trade and returns `RefusedTrade`, and `World::trade` panics with `engine bug: the market on hex #<hex> refused a trade for POP #<pop>: <error>` or `engine bug: the market on hex #<hex> refused a trade for business '<name>': <error>`. The World is never handed back half-stepped. `consume` likewise panics with an `engine bug` message if a removal it has already checked fails.

## docs/trade.md — modify
- **Location:** "Determinism", whole section body
- **Change:** replace

Hexes in hex ID order; within a hex, POPs in POP ID order then businesses in business ID order within each pass; goods in good ID order within each trader; every sale on a hex before any purchase; then POP consumption; then each business on the hex operates its factories, in business ID order and factory ID order: the same setting stepped the same number of times under deciders that answer the same way gives the same stockpiles, conditions, outcomes and reports.

## docs/trade.md — modify
- **Location:** "Worked examples", at the end of the section
- **Change:** add

Businesses, from the sample under its `arithmetic` decider (see [python.md](python.md)), at the center's prices (`unimproved_grain` 0.5, `unskilled_labor` 1, `grain` 2, `copper` 5, `tools` 12):

| Business, turn 0 | Holds at the decision point | Sold | Bought | Factory | Cash before → after sales → after purchases |
|---|---|---|---|---|---|
| `center_farm` | 8 `unimproved_grain`, 100 `silver` | nothing | 4 `unimproved_grain`, 7 `unskilled_labor`, 0.5 `tools` | run 1: 12 `grain` from 12 `unimproved_grain` and 6 labor; wear 1/8; maintenance 0.125 (0.5 tools, 1 labor); condition 1 → 1 | 100 → 100 → 85 |
| `center_mine` | 2.5 `unimproved_copper`, 50 `silver` | nothing | 4 `unskilled_labor`, 0.75 `tools` | output 2.5 `copper`; wear 1/8; maintenance 0.375; condition 0.75 → 1 | 50 → 50 → 37 |
| `center_toolmaker` | 200 `silver` | nothing | 8 `copper`, 6.25 `unskilled_labor`, 0.125 `tools` | output 4 `tools`; condition 1 → 1 | 200 → 200 → 152.25 |

At turn 0 nothing has been made yet, so nothing is sold. At turn 1 each business offers last turn's output: the farm sells 12 `grain`, the mine 2.5 `copper` and the toolmaker 3.875 `tools` (keeping what its own maintenance needs), and each ends the round with more cash than at turn 0 (farm 94, mine 43.5, toolmaker 152.5). From then on each turn repeats, and every factory stays at full condition.

## docs/trade.md — modify
- **Location:** "Using outcomes", the code block, at the end; and the paragraph after it
- **Change:** add to the code block, and replace the paragraph

```rust
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

## docs/trade.md — modify
- **Location:** "Not yet", whole section body
- **Change:** replace

Price adjustment, rationing, market inventory or any limit on what the market trades, and aggregating submissions into one demand per type (Stage 05; with rationing, the subsistence index will need to reconsider whether it reads what was demanded or what was consumed); saving beyond rounding leftovers; rounding rules or a smallest unit of currency; operating budgets and dividends, which will plug into the cash cap (Stage 06); construction and expansion, which will plug into maintenance (Stage 07); businesses trading with each other directly or across hexes; and per-hex, per-POP or per-business breakdowns in the turn report (those numbers are in `World::pop_outcome` and `World::business_outcome`).

## docs/trade.md — modify
- **Location:** "Tests", whole section body
- **Change:** replace

- `trade.rs` unit tests run `hex_round` and the private passes against markets from the `pops_trade` world and hand-built stockpiles. For POPs: a single peasant selling, buying and consuming; two POPs on a hex giving the same outcomes as each alone (sales before purchases); a hex without a market; empty slices; a zero demand selling everything with no sliver; the purchase cap reducing a purchase that rounding would make unaffordable, and a rounding surplus left as currency; a POP with no currency buying nothing; consumption taking only utility goods up to the demand, and taking what is held when short; the currency never traded when a utility names it; and `settle_sales` returning a refusal when called against a market that doesn't price a good. For businesses (a `BusinessSubmission` needs no business in the setting): selling offers and buying bids with the three cash figures and the bought group's perish turn; an offer above holdings selling what is held and an offer of a good not held selling nothing; bids beyond cash scaled by one proportion (0.2); no currency buying nothing with proportion 0; a rounding sliver capping the last purchase; empty submissions; a POP and a business on one hex each giving the same result as alone; and a refused business trade reported as `RefusedTrader::Business(0)`. They take `q`, `close` and `no_growth` from `crates/engine/src/test_support.rs`; their `hex(world, x)` and `good(world, name)` helpers, which need a world, stay local.
- `crates/engine/tests/trade.rs`: the sample's turn 0 (every peasant with a market and the artisan, with their outcomes and stockpiles, sizes taken from `starting_size()`), the no-market peasant, the report totals and the perished list at turn 0, and turn 1's totals computed from the grown sizes; three steps repeating the outcomes per individual (each amount and the income divided by `starting_size()`) with two worlds agreeing; the `pops_trade` table at turn 0 and after a step (zero growth, so sizes don't change); currency carried over as income (the miser, the saver); a below-subsistence POP; held goods outside the utility (the collector's gem, the miser's silver); a hex with a market and no POPs; worlds without POPs reporting nothing traded and a population of 0; and `pop_outcome`'s foreign-POP panic. These run under the idle host, so no business trades.
- `crates/engine/tests/business_trade.rs`, with closure deciders on the `business_trade` setting: `works` trading, operating and selling its output the next turn, its cash cycling 100 → 22 → 100, and no labor or bread in `perished`; `poor` bidding beyond its cash, scaled to exactly 0.2 with its run cut to 0.2 through the labor shortage while the fraction asked stays 1, then buying nothing with no cash; `remote` on a hex with no market running from its own labor and its bread perishing; idle businesses trading nothing and their factories wearing to 0, then shut-down factories making nothing although inputs were bought, while maintenance still restores them; a refused decision idling only its business; a POP on the hex unaffected by the businesses; the report's four business totals, and all four empty under the idle host; and two worlds stepped identically. The last test runs the sample under the real `arithmetic` decider (it needs the project's Python, like `external_decider.rs`): the turn-0 figures in [Worked examples](#worked-examples), each business selling and gaining cash at turn 1, every condition staying 1 for three more steps with no decider failure, and two sample worlds agreeing.
- `pops_trade` (`crates/engine/tests/settings/pops_trade/`): goods `labor` (perishes after 1), `grain` (after 3), and durable `copper`, `silver` (every hex's currency) and `gem`; hexes (0, 0) to (4, 0) with markets at different prices, (2, 0) without one, and (4, 0) with a market and no POPs; POP types `peasant`, `artisan`, `saver` (also endowed silver), `collector` (also endowed a gem), `miser` (labor with share 0 and copper, which (3, 0) doesn't price, so its income is parked as silver and accumulates), and `idle` (empty endowment). Every type has zero growth parameters. Its `README.md` entry lists every hex, price and POP.
- `business_trade` (`crates/engine/tests/settings/business_trade/`): goods `coin`, `labor` (perishes after 1), `ore` (after 3), `iron`, `tools`, `bread` (after 2); technologies `smelter` and `bakery` with `production_valid`'s numbers; one `worker` POP at (0, 0); hexes (0, 0), with a market (labor 1, ore 2, iron 6, tools 10, bread 3), and (1, 0) without one; businesses `works` (100 coin, 3 tools; a smelter and a bakery) and `poor` (10 coin, 30 ore; a smelter) at (0, 0), and `remote` (6 labor; a bakery) at (1, 0), every one with decider `rule`. The numbers make `works`'s steady plan cost and earn exactly 78 a turn, `poor`'s bids cost 50 against 10 cash, and `remote` hold labor for exactly one full run. Its `README.md` entry lists the exact values.

## docs/world.md — modify
- **Location:** "What a World holds", the table, the `business_stockpiles` row (which Step 02 adds)
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same row

| `business_stockpiles: IdVec<BusinessKind, Stockpile>` | one per business at creation, filled from its starting inventory | goods enter by emission from the deposits it owns, by purchase in the trade round and as factory output, and leave by sale, as factory inputs and maintenance goods, and by perishing (see [trade.md](trade.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the table, the `factories` row (which Step 03 adds)
- **Change:** replace
- **Supersedes:** Step 03 (factories), the same row

| `factories: Factories` | the setting's `StartState::factories` | every turn: each factory's condition is written back after its owner's operation (see [trade.md](trade.md#operating-the-factories)); technology and owner are state because later Stages change them (see [factories.md](factories.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the table, new row after the `business_answers` row (which Step 07 adds)
- **Change:** add

| `business_outcomes: IdVec<BusinessKind, BusinessOutcome>` | built by turn 0's trade round | rebuilt every turn: each business's `BusinessTrade` (what it offered, sold, bid and bought, the cash cap's proportion, its cash at three points) and one `FactoryRecord` per factory it owns (see [trade.md](trade.md#businesstrade)) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, after the `business_answer` sentence (the one Step 07 adds)
- **Change:** add

`business_outcome(business)` returns the business's `BusinessOutcome` from the current turn: `trade()` gives its `BusinessTrade`, `factories()` the `(FactoryId, FactoryRecord)` of each factory it owns in factory ID order, and `factory(id)` one factory's record (None if the business doesn't own it). It panics for a business from another world. The outcome doesn't copy the decision, its trace or its errors; those are in `business_answer`. Because the world is observed after operation, `business_view(business)` called between steps shows the business after this turn's trade and operation (this turn's output held, conditions after wear and maintenance), not what its decider was served at this turn's decision point; that is next turn's view, before next turn's emission.

## docs/world.md — modify
- **Location:** "The turn", item 2 of the numbered list
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same item

2. **Actors act:** the decision point, then on every hex the trade round, consumption and factory operation, then growth. At the decision point every business's view goes to the decider its setting entry names, through the host passed to `new` or `step`, and its answer is stored (see [decider-host.md](decider-host.md#the-decision-point)). Then on every hex each POP submits its parameters and holdings and each business the offers and bids its valid decision resolves to (none for a business that decided nothing or was refused); the hex's market computes the POPs' demand and settles every sale before any purchase, POPs before businesses in each pass, with a business's bids scaled by one proportion when they cost more than its cash; and each POP consumes its utility goods. Right after the hex's round, each business on it runs, wears and maintains its factories at the fractions its decision gives (0 for an idle business, whose factories only wear), and the conditions are written back. Then, once every hex is done, every POP grows or shrinks by the rate its outcome records (see [Trade round and operation](#trade-round-and-operation), [Growth](#growth), [trade.md](trade.md) and [growth.md](growth.md)). Later Stages add the live market.

## docs/world.md — modify
- **Location:** "The turn", the three bullets after "Two calls drive the clock, and both run a turn through one private function"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same bullets

- `World::new(setting, &mut host) -> Result<(World, TurnReport), UnknownDecider>` first checks that the host holds every business's decider name (`host.check`), and fails, building nothing, if one is missing. Then it creates the world at turn 0 with empty hex and POP stockpiles, each business's stockpile filled from its starting inventory (each good in the group it would get if added in turn 0, `PerishesAt::for_good(perishes, Turn::ZERO)`), and every business's answer `Nothing`, and runs `run_turn` for turn 0. So a fresh world's deposits have emitted on top of that, its businesses have answered, traded and operated their factories, and its POPs have already been endowed, traded, consumed and grown. In particular, a fresh world's factories are not at the conditions the setting places them at: under the idle host each has worn one lifetime-slice. The report's `perished` is empty.
- `World::step(&mut self, &mut host) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`) and that the host holds every business's decider name, so a refused step changes nothing (sizes, conditions, outcomes, business stockpiles and answers included). Then it runs the end of the current turn (`perish`: hexes in ID order, then POPs in ID order, then businesses in ID order), advances the clock, and runs `run_turn` for the new turn with what perished. The host may be a different one from the last call (a rebuilt host, say), as long as it holds every name.
- `World::run_turn(&mut self, perished, &mut host) -> TurnReport` (private) calls `emit`, `endow`, `decide`, `trade` and `grow` in that order and builds the report from their results: `emitted` and `endowed` from the first two, `decisions` and `decider_failures` from `decide`, `sold`, `bought`, `consumed`, `business_sold`, `business_bought`, `produced` and `used` from the private `TradeTotals` that `trade` returns (`trade` also operates the factories), `population` from `grow`, and `turn` from the clock.

## docs/world.md — modify
- **Location:** "The turn", the paragraph beginning "So between calls the world is always **after part 2**", its first sentence
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same sentence

So between calls the world is always **after part 2**: this turn's deposit output is in the hex stockpiles (unowned deposits) and the business stockpiles (owned deposits, unless a factory has used it), every business's answer for this turn is stored, every business has traded and operated its factories (its stockpile holds this turn's factory output, to be sold next turn, and its factories' conditions are after this turn's wear and maintenance), this turn's POP trade is done, each POP's stockpile holds what the round left, each POP's size is its size after growth, and nothing has perished yet this turn.

## docs/world.md — modify
- **Location:** "The turn", "Decision point" (the subsection Step 07 adds), its last sentence ("The decision point only reads the world, so the views see the stockpiles after emission and endowment; nothing in the turn acts on the answers yet.")
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same sentence

The decision point only reads the world, so the views see the stockpiles after emission and endowment, with last turn's factory output and the conditions left by last turn's operation. The trade round and operation then act on the stored answers (see [Trade round and operation](#trade-round-and-operation)).

## docs/world.md — modify
- **Location:** "The turn", the "Trade round" subsection, heading and body
- **Change:** replace

### Trade round and operation

`World::trade` (private) runs after `decide`. For each hex in hex ID order that has POPs or businesses, it resolves each business's answer into a `BusinessSubmission` with `business_submission` (moving its stockpile out), builds the hex's `Market` from the world's fields, moves each POP's stockpile out into a `Submission` (`std::mem::take`), and calls `trade::hex_round` with both. It moves the stockpiles back and stores each `PopOutcome` by `PopId`. Then, for each business on the hex in ID order, `operate_business` runs `production::operate` on the business's factories with its decision's fractions and writes the conditions back with `Factories::set_condition`, and the business's `BusinessTrade` and factory records are stored as its `BusinessOutcome`. After the last hex it replaces `pop_outcomes` and `business_outcomes` and returns the totals through `totals_by_good` as a `TradeTotals` (a private struct of seven lists, each in good ID order: POP `sold`, `bought` and `consumed`; `business_sold` and `business_bought`; `produced`, the output of every factory record with output; and `used`, every record's run inputs and maintenance cost), which `run_turn` puts into the report. A refused trade can't happen; if one does, it panics with an `engine bug` message naming the hex and the POP or business. The loop reads `map`, `pops`, `businesses`, `rules`, `hex_currencies`, `hex_prices` and `business_answers` and writes `pop_stockpiles`, `business_stockpiles` and `factories`, as separate fields, so the borrows don't conflict. See [trade.md](trade.md) for the round, the cash cap and operation.

## docs/world.md — modify
- **Location:** "Determinism", its first sentence
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same sentence

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, deciders are taken in name order and the businesses within a decider in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and then businesses in ID order within each pass, goods in ID order within each trader, with every sale on a hex before any purchase, then POP consumption, then each business on the hex operating its factories in factory ID order, POPs grow in ID order, stockpiles perish hexes first, then POPs, then businesses, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times with deciders that answer the same way gives identical worlds (POP sizes, business stockpiles, factory conditions and answers included), outcomes and reports.

## docs/world.md — modify
- **Location:** "Determinism", the last sentence ("`World` itself is not `PartialEq`, but `Stockpile`, `PopOutcome` and `TurnReport` are, which is how tests compare two runs.")
- **Change:** replace

`World` itself is not `PartialEq`, but `Stockpile`, `PopOutcome`, `BusinessOutcome` and `TurnReport` are, which is how tests compare two runs.

## docs/world.md — modify
- **Location:** "TurnReport", the code block, after the `population` line and before the `decisions` line (which Step 07 adds)
- **Change:** add

```rust
    pub business_sold: Vec<(GoodId, Quantity)>,   // sold by businesses in the trade round of `turn`
    pub business_bought: Vec<(GoodId, Quantity)>, // bought by businesses in the trade round of `turn`
    pub produced: Vec<(GoodId, Quantity)>,        // output made by factories in `turn`
    pub used: Vec<(GoodId, Quantity)>,            // inputs and maintenance goods consumed by factories in `turn`
```

## docs/world.md — modify
- **Location:** "TurnReport", the paragraph after the code block, whole paragraph
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its replacement of the first sentence and its added sentence about the sample's businesses; Step 07 (decider-host), its addition at the end

Each list has one total per good, summed over every hex (and, for `perished`, every POP and business stockpile; for `emitted`, every deposit, whichever stockpile it emits into; for `endowed`, `sold`, `bought` and `consumed`, every POP; for `business_sold` and `business_bought`, every business; for `produced` and `used`, every factory), in good ID order, and lists only amounts greater than zero. `emitted` and `endowed` are kept apart so deposit output and POP income can be told apart, and the business lists are kept apart from the POP lists so the POP numbers keep their meaning. `population` is a single total, `Quantity::ZERO` for a world with no POPs. There is no per-hex, per-POP or per-business breakdown in the report: what each holds is in its stockpile, what each POP traded and how it grew is in its `PopOutcome` (`World::pop_outcome`), what each business traded and its factories did is in its `BusinessOutcome` (`World::business_outcome`), and a POP's size is in `World::pops()`.

For the sample setting under the idle host (every engine test that isn't about deciders), turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`; endowed 4100 `unskilled_labor`; sold 2878.6 `unskilled_labor`; bought 5 `copper` and 1426.8 `grain`; consumed 1221.4 `unskilled_labor`, 5 `copper` and 1426.8 `grain`; population 419.4466666666666", and every business list empty. Because the POPs grow, later turns endow, trade and consume more (each turn's `endowed` is 10 labor times the previous turn's `population`), and `perished` is the deposit output only, since nothing uses it: the POPs sell or consume all their labor in the round. Under the `arithmetic` decider (the checker and the dev UI) the POP lists are the same, the businesses trade and their factories run (see [Setting checker](#setting-checker---turns-n)), and `perished` from turn 1 is the unowned deposits' output only (7.5 `unimproved_copper`, 16 `unimproved_grain`, since the factories use what the owned deposits emit), plus a floating-point sliver of `unskilled_labor` that the toolmaker's purchases leave (see [trade.md](trade.md#slivers)).

`decisions` adds up to the number of businesses; `decider_failures` is empty when every decider answered (see [decider-host.md](decider-host.md#answers-and-the-report)). The turn line doesn't print `decisions`; failures are printed as lines of their own after it.

## docs/world.md — modify
- **Location:** "Stockpile", the first sentence of the section
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

`Stockpile` is a general type, not specific to hexes. Hexes use it for unowned goods, POPs for what they own (see [pops.md](pops.md)), and businesses for their goods and cash (see [businesses.md](businesses.md)); a business's factories take their inputs from and put their output into its stockpile (see [production.md](production.md)).

## docs/world.md — modify
- **Location:** "Stockpile", the paragraph beginning "There is no public way to change a stockpile.", its last two sentences ("Inside the engine, the trade round sells POP goods with `sell_down_to` ... (see [trade.md](trade.md)). A `GoodId` does not say ...")
- **Change:** replace the first of the two

Inside the engine, the trade round sells POP goods with `sell_down_to` (which uses `retain`), sells business offers with `sell`, buys with `buy` and `buy_for`, and removes consumed goods with `remove`; the factory mechanics remove inputs and maintenance goods with `remove` and add output with `add` (see [trade.md](trade.md) and [production.md](production.md)).

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`", the example block and the paragraph after it
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the last sentence of its addition at the end of this section ("The sample's deciders answer every turn, so its output is unchanged by them.")

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666; businesses sold nothing; businesses bought 17.25 unskilled_labor, 4 unimproved_grain, 8 copper, 1.375 tools; produced 2.5 copper, 12 grain, 4 tools; used 17.25 unskilled_labor, 2.5 unimproved_copper, 12 unimproved_grain, 8 copper, 1.375 tools
Turn 1: perished 0.0000000000000002220446049250313 unskilled_labor, 7.5 unimproved_copper, 16 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4194.466666666667 unskilled_labor; sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain; population 429.2982444444444; businesses sold 2.5 copper, 12 grain, 3.875 tools; businesses bought 16.25 unskilled_labor, 4 unimproved_grain, 8 copper, 0.75 tools; produced 2.5 copper, 12 grain, 4 tools; used 16.25 unskilled_labor, 2.5 unimproved_copper, 12 unimproved_grain, 8 copper, 0.8749999999999999 tools
```

The line lists `perished`, `emitted`, `endowed`, `sold`, `bought` and `consumed` in that order, then `population`, then the business lists `businesses sold`, `businesses bought`, `produced` and `used`. Entries are `<amount> <good name>` joined by `, ` in report order (good ID order: `unskilled_labor` comes first, `copper` before `grain`, and `tools` last in the sample's goods); an empty list prints `nothing`, so a setting without POPs or businesses ends every line with `; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0; businesses sold nothing; businesses bought nothing; produced nothing; used nothing`. The amounts are floating-point sums printed with `Quantity`'s `Display`, with nothing rounded, so they can print with a long tail of digits, as the sample's do from turn 1 on once its POPs have grown, and slivers print in full (the `unskilled_labor` in turn 1's `perished`, and `0.8749999999999999 tools`). In the sample, nothing is sold by businesses at turn 0 because nothing has been made yet; from turn 1 each business sells last turn's output, and every turn after repeats turn 1's business lists. The line is built by `text::turn_line`. The checker is the headless `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)). The dev UI window builds a `World` from a loaded setting, steps it with its Step buttons, and shows the same turn lines in its turn log as `check --turns` prints.

## docs/world.md — modify
- **Location:** "Tests", first bullet, the `world.rs` item
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same item

`world.rs` (a world forced to `Turn::MAX`, for `turns_valid`, `pops_placed` and `businesses_valid`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles, business stockpiles, business answers, business outcomes, factory conditions, POP outcomes and POP sizes unchanged)

## docs/world.md — modify
- **Location:** "Tests", second bullet (`crates/engine/tests/world.rs`), the sentence Step 07 adds at the end
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same sentence

The determinism test also compares every business outcome, business stockpile and factory condition of the two runs. These tests, and every engine test not about deciders, build and step worlds with the idle host (`common::idle()`), so no business trades or runs a factory, but every factory still wears one slice a turn. The decision point is tested in `crates/engine/tests/decider_host.rs` and `external_decider.rs` (see [decider-host.md](decider-host.md#tests)), and the business round and operation in `crates/engine/tests/business_trade.rs` (see [trade.md](trade.md#tests)).

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same section

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`, each business's answer in `World::business_answer` and its trade and factory records in `World::business_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, operating budgets and dividends (Stage 06), and building, scrapping or retooling factories (Stage 07).

## docs/businesses.md — modify
- **Location:** introduction, the second paragraph
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), its replacement of the same paragraph

Factories (see [factories.md](factories.md)) name a business as their owner, and must sit on its hex. Every turn, at the decision point, each business is served its view and its decider's answer is validated and stored in the world (see [decider.md](decider.md) for the view and the decision, [decider-host.md](decider-host.md) for the exchange, and [python.md](python.md) for the first decider, the arithmetic rule). Then, in its hex's trade round, a business with a valid decision sells its offers and buys its bids from and into its stockpile, and right after the round it runs, wears and maintains its factories at the fractions the decision gives (see [trade.md](trade.md)). A business that decided nothing, or whose decision was refused, trades nothing and its factories only wear.

## docs/businesses.md — modify
- **Location:** "In the world" (the section Step 02's new file defines), the "**Report.**" and "**Nothing else.**" bullets
- **Change:** replace both bullets with the three below

- **Trade and operation.** In part 2 of the turn the business takes part in its hex's round behind the POPs: its offers are sold (trimmed to what it holds) before any purchase on the hex, and its bids are bought under the **cash cap**, one proportion scaling every bid when they cost more than its currency. Its cash is the amount of its hex's currency in its stockpile. Right after the round its factories run, wear and are maintained, taking inputs and maintenance goods from its stockpile and adding their output to it, to be sold next turn (see [trade.md](trade.md)).
- **Outcome.** `World::business_outcome(business)` returns what the current turn did for it: a `BusinessTrade` (offered beside sold, bid beside bought, the proportion, its cash before the round, after the sales and after the purchases) and one `FactoryRecord` per factory it owns (see [trade.md](trade.md#businesstrade) and [production.md](production.md#data-model)).
- **Report.** `emitted` is one total over every deposit, owned or not, and `perished` includes what perished from business stockpiles. The business lists `business_sold`, `business_bought`, `produced` and `used` total every business and factory (see [world.md](world.md#turnreport)). Nothing changes a business or a deposit's owner during play.

## docs/businesses.md — modify
- **Location:** "Using businesses", the code block, inside the `for` loop after the `answer` line (which Step 07 adds)
- **Change:** add

```rust
    let outcome = world.business_outcome(id);            // its trade and its factories' records this turn
    let cash_after = outcome.trade().cash_after_purchases();
```

## docs/businesses.md — modify
- **Location:** "Not yet", its first sentence
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same sentence

Showing businesses, their stockpiles, their decisions and their outcomes in the dev UI is Step 09 of Stage 03; until then only the turn line's business totals are visible there.

## docs/businesses.md — modify
- **Location:** "The sample setting and tests", first bullet (`settings/sample/start/businesses.toml`), the sentences "In a fresh sample world `center_farm` holds 8 `unimproved_grain` and 100 `silver`, `center_mine` 2.5 `unimproved_copper` and 50 `silver`, `center_toolmaker` 200 `silver`, and the center hex stockpile only the unowned deposit's 4 `unimproved_grain`; every turn after looks the same. The turn reports are unchanged."
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentences

Under the idle host, a fresh sample world's `center_farm` holds 8 `unimproved_grain` and 100 `silver`, `center_mine` 2.5 `unimproved_copper` and 50 `silver`, `center_toolmaker` 200 `silver`, and the center hex stockpile only the unowned deposit's 4 `unimproved_grain`; every turn after looks the same, and the POP numbers in the turn reports are unchanged by the businesses. Under the `arithmetic` decider the three businesses form a chain: the farm and the mine buy what their deposits don't cover and sell their output, the toolmaker buys copper and sells tools, and each maintains its factory (see [trade.md](trade.md#worked-examples) for the numbers).

## docs/factories.md — modify
- **Location:** introduction, the second paragraph
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), its replacement of the same paragraph

Every turn, right after its hex's trade round, each business's factories are run, worn and maintained by the factory mechanics (see [production.md](production.md)) at the fractions its decision gives, and their conditions are written back (see [trade.md](trade.md#operating-the-factories)). Each business's view lists its factories by ID number with their condition and technology, and a decision addresses them by the same number (see [decider.md](decider.md)). Showing factories in the dev UI is Step 09.

## docs/factories.md — modify
- **Location:** "Concepts", the "Condition, not a goods table." bullet, its last sentence(s) (as Step 04 replaced them, starting "A setting places a factory with a condition greater than 0 and at most 1")
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same sentences

A setting places a factory with a condition greater than 0 and at most 1 (1 when none is written). The condition changes every turn in play: wear takes one lifetime-slice and maintenance adds back what the business pays for, so the engine type accepts 0 too. A factory runs only while it holds at least one lifetime-slice (a condition of at least 1/t₁). Below that it is shut down until maintenance brings it back (see [production.md](production.md#lifetime-slices-and-shutdown)).

## docs/factories.md — modify
- **Location:** "In the world", whole section body
- **Change:** replace

The `World` takes the `Factories` over as state (`World::factories() -> &Factories`). A caller reads every factory with `world.factories().iter()`, the factories on a hex with `on_hex(hex)` and the factories a business owns with `owned_by(business)`, all in ID order. The condition is the only thing that changes during play: after each business's operation the World writes each factory's new condition back with the crate-private `Factories::set_condition(id, condition)`, which panics with `a factory's condition must be a finite number from 0 to 1` for a value out of range (and `is not from this list` for a foreign ID). Technology and owner stay as placed until later Stages. Because a world is observed after part 2, a fresh world's conditions are already after turn 0's operation: under the idle host every factory has worn one slice (the sample's farm reads 0.875, its mine 0.625 and its toolmaker 23/24). What each factory did this turn is in its owner's `World::business_outcome` (see [trade.md](trade.md#businesstrade)). See [world.md](world.md).

## docs/factories.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same section

Showing factories and their records in the dev UI is Step 09 of Stage 03. Factories under construction, building, scrapping, retooling, and buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes.

## docs/factories.md — modify
- **Location:** "The sample setting and tests", the `crates/engine/tests/factories.rs` bullet, its first clause ("the sample's three factories, held unchanged by a fresh world through several steps, and two runs with identical reports and factories;")
- **Change:** replace

the sample's three factories, which under the idle host wear one lifetime-slice a turn from turn 0 (lifetimes 8, 8 and 24; compared to within 1e-9), and two runs with identical reports and factories;

## docs/production.md — modify
- **Location:** introduction, the paragraph beginning "**Nothing in the turn calls them yet.**"
- **Change:** replace

**The turn calls `operate` once per business**, right after its hex's trade round, with the fractions its valid decision gives: 0 and 0 for a factory the decision doesn't mention, and for every factory of a business that decided nothing or was refused, whose factories then only wear. The World writes each condition back and keeps the records in the business's outcome (see [trade.md](trade.md#operating-the-factories)).

## docs/production.md — modify
- **Location:** "The model", "A safety net, not the normal path", at the end of the section
- **Change:** add

The engine's cash cap relies on these cuts. When a business's bids cost more than its cash, the trade round scales every bid by one proportion and leaves the fractions alone; the factories then find too few goods, and the shared cuts here reduce the run and maintenance done (see [trade.md](trade.md#the-engine-never-touches-the-fractions)). For a business whose only short good is the one the cap cut, its run is cut by the cap's proportion.

## docs/production.md — modify
- **Location:** "Data model", the last bullet ("**A cut is visible as `done < asked`** in either record. Step 08 keeps these records as the business's per-turn record, and Step 09 shows them.")
- **Change:** replace

- **A cut is visible as `done < asked`** in either record. The World keeps each business's records, in factory ID order, in its `BusinessOutcome` (`World::business_outcome(business).factories()`), and the turn report totals their output as `produced` and their inputs and maintenance cost as `used` (see [world.md](world.md#turnreport)). Step 09 shows them in the dev UI.

## docs/production.md — modify
- **Location:** "Using the mechanics", the code block's comment line and the paragraph after it ("The World doesn't yet expose a way to write the stockpile or the conditions back. Step 08 does that inside the turn.")
- **Change:** replace the paragraph

The World runs the same call inside the turn (`World::operate_business`) and writes the stockpile and conditions back; from outside, the example works on a clone and changes nothing. Note that `factory.condition` read between steps is already after this turn's operation, so this example operates the factories a second time from that state. To read what the turn itself did, use `world.business_outcome(owner).factories()`.

## docs/production.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), its replacement of the same section

The fractions come from a business's `Decision` (see [decider.md](decider.md)), which every business's decider sends at the decision point (see [decider-host.md](decider-host.md)); the first decider, the arithmetic rule, chooses them in Python with the same slice arithmetic as this module (see [python.md](python.md#orders-per-factory)). Showing the records in the dev UI is Step 09. Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## docs/production.md — modify
- **Location:** "Tests", the `crates/engine/tests/production.rs` bullet, its first sub-bullet ("the sample's three factories, operated at turn 0, where no business holds labor, so nothing runs or is maintained and only wear applies;")
- **Change:** replace, and add a sentence at the end of the `crates/engine/tests/production.rs` bullet's lead line

  - the sample's three factories, operated at turn 0, where no business holds labor, so nothing runs or is maintained and only wear applies; the world's own conditions are untouched by the call;

and, after "through the public API only:", add: "Each `Operation` takes its condition from a separately loaded `LoadedSetting` (the condition the setting places the factory at), not from the world, because a world is already past turn 0's operation when `World::new` returns; the technologies and stockpiles come from the world." The mechanics in the turn are tested in `crates/engine/tests/business_trade.rs` (see [trade.md](trade.md#tests)).

## docs/decider.md — modify
- **Location:** introduction, the paragraph beginning "**The engine runs the exchange once per turn.**"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same paragraph

**The engine runs the exchange once per turn** and acts on it. At the decision point, through a host of named deciders the caller builds (a Python process, a replayed recording, or a Rust closure), every business's view is served and its decision validated and stored in the World (see [decider-host.md](decider-host.md)). The World then resolves each valid decision: the offers and bids go through the hex's trade round, and the fractions go to the factory mechanics (see [trade.md](trade.md)). The other side of the protocol exists in Python: the `policyspace` package reads and writes these messages byte for byte and holds the first decider, the arithmetic rule (see [python.md](python.md)). The two sides are tied together by committed recordings that both test suites read (see [Recordings](#recordings)). Step 09 shows decisions and traces in the dev UI.

## docs/decider.md — modify
- **Location:** "Concepts", the "**Asking for too much is not an error.**" and "**Doing nothing is legitimate.**" bullets
- **Change:** replace

- **Asking for too much is not an error.** Offering more of a good than the stockpile holds, bidding for more than cash can pay, or offering and bidding the same good is a valid request. The trade round trims it when it resolves it: an offer sells at most what is held, and bids that cost more than the cash held after the hex's sales are all scaled by one proportion; the factory mechanics then cut runs short of goods. The business's outcome records what was asked beside what was done (see [trade.md](trade.md#the-cash-cap) and [production.md](production.md#a-safety-net-not-the-normal-path)). Validation refuses only nonsense.
- **Doing nothing is legitimate.** A factory a decision doesn't mention is run at 0 and maintained at 0, so it only wears; a fraction left out is 0; a business with no decision in a reply trades nothing that turn and its factories only wear. `Decision::empty(name)` is the explicit "do nothing".

## docs/decider.md — modify
- **Location:** "JSON", "Format", the sentence "The farm's view in a fresh sample world, as `to_json_pretty` writes it:"
- **Change:** replace

The farm's view at the sample's turn-0 decision point (the request in `sample_turn0.recording.json`), as `to_json_pretty` writes it:

## docs/decider.md — modify
- **Location:** "Extending", the "**A new verb in the decision**" bullet, its last sentences ("... and resolve it in Step 08's resolution code. Asking for more than the business has stays valid.")
- **Change:** replace

and resolve it in the World: `World::business_submission` for a verb that trades, `World::operate_business` for one that reaches the factories, or a new step of the turn (see [trade.md](trade.md)). Asking for more than the business has stays valid, and the resolution trims it and records asked beside done.

## docs/decider.md — modify
- **Location:** "Recordings", the `the_sample_recording_matches_the_engine` bullet
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same bullet; Step 07 (decider-host), its two edits to it

- `the_sample_recording_matches_the_engine` builds the sample world under a closure decider registered as `arithmetic` that keeps a copy of the request it is served and answers nothing, so the request is exactly the one turn 0's decision point builds (the loaded state plus emission, one `business_view` per business in ID order). It can't build the request from the world afterwards, because a world is already past turn 0's trade and operation when `World::new` returns. It checks the file's request equals it, the file is in the engine's canonical pretty form (which also proves Python's pretty writer matches), and the file's reply validates against the request with one `Answer::Valid` per view, in the request's order. If the sample setting, the view or the rule changes, this test fails, so the recording can't drift from the engine. `crates/engine/tests/decider_host.rs` replays the same file through the World (three valid answers at turn 0), `crates/engine/tests/external_decider.rs` checks that the real `arithmetic` process answers the sample's turn 0 with the recorded reply, and `crates/engine/tests/business_trade.rs` checks the trades and factory runs that reply leads to.

## docs/decider.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same section

Showing decisions, traces and outcomes in the dev UI is Step 09 of Stage 03; the trace is kept in the business's stored answer, not in its outcome. Verbs beyond the two fractions, offers and bids (budgets, construction, retooling, trading assets) are later Stages. Per-deposit IDs and structured traces are not planned yet. Running the exchange is described in [decider-host.md](decider-host.md), and resolving decisions in [trade.md](trade.md).

## docs/decider.md — modify
- **Location:** "Tests", the `crates/engine/tests/decider.rs` bullet, its first sub-bullet ("the sample's three center businesses see their own prices, perish groups, cash, factories, technologies and deposits at turn 0, ...")
- **Change:** add, at the end of the sub-bullet

; their views are built from the world after `World::new` (idle host), so each factory's condition is one slice below the setting's (farm 0.875, mine 0.625, toolmaker 23/24; `production_valid`'s `works` 0.75, 0.25 and 2/3), compared to within 1e-9

## docs/decider-host.md — modify
- **Location:** introduction, the second paragraph, its last two sentences ("**Nothing resolves the answers yet:** businesses still trade nothing and their factories don't run, so the turn's other numbers are unchanged; Step 08 of Stage 03 resolves the valid decisions.")
- **Change:** replace

The World then resolves each valid decision in the trade round and in factory operation (see [trade.md](trade.md)); a business whose answer is nothing or refused trades nothing and its factories only wear.

## docs/decider-host.md — modify
- **Location:** "Concepts", the "**The idle host**" bullet
- **Change:** replace

- **The idle host** holds every name and answers nothing: every business decides nothing every turn, so no business trades or runs a factory, but every factory still wears one slice a turn. It is for callers that don't care about business behavior; every engine test that isn't about deciders uses it. Their POP numbers don't depend on it; factory conditions do (a fresh world's factories have already worn one slice).

## docs/decider-host.md — modify
- **Location:** "Not yet", its first sentence ("Resolving the stored decisions (trading, running and maintaining factories, trimming over-asking, the cash cap) is Step 08 of Stage 03, and showing each business's answer, its trace and its refusals in the dev UI is Step 09; until then only the failure lines in the turn log are visible.")
- **Change:** replace

Showing each business's answer, its trace, its refusals and its outcome in the dev UI is Step 09 of Stage 03; until then the dev UI shows only the failure lines and the turn line's business totals.

## docs/decider-host.md — modify
- **Location:** "Tests", the last bullet ("Every other engine test passes the idle host, with its expectations unchanged.")
- **Change:** replace

- Every other engine test passes the idle host. Their expectations don't depend on the host, except factory conditions after `World::new` or a step, which show the idle businesses' wear. `crates/engine/tests/business_trade.rs` resolves closure deciders' answers and, for the sample, the real `arithmetic` decider's (see [trade.md](trade.md#tests)).

## docs/python.md — modify
- **Location:** introduction, the paragraph beginning "**The engine runs it.**"
- **Change:** replace its last sentence ("The engine validates and stores the decisions; Step 08 of Stage 03 resolves them.")
- **Supersedes:** Step 07 (decider-host), the same sentence

The engine validates and stores the decisions, then acts on the valid ones: the offers and bids go through the hex's trade round and the fractions run and maintain the factories (see [trade.md](trade.md)).

## docs/python.md — modify
- **Location:** "The arithmetic rule", "The cash budget", the last sentence of the paragraph beginning "If the bids cost more than that" ("The engine's cash cap (Step 08) is the safety net, not the normal path.")
- **Change:** replace

The engine's cash cap is the safety net, not the normal path: if the bids still cost more than the cash after the sales, the engine scales every bid by one proportion and the factories are cut by the resulting shortage (see [trade.md](trade.md#the-cash-cap)).

## docs/python.md — modify
- **Location:** "The arithmetic rule", "Offers and bids", at the end of the section
- **Change:** add

The rule's bids should match exactly what its fractions use, since anything bought and not used stays in the stockpile, and labor left over perishes at the end of the turn. At present the toolmaker's maintenance fraction comes out a hair under 1/24 while its labor bid is for the full slice, so about 2.2e-16 `unskilled_labor` perishes each turn and prints on the sample's turn lines (logged as bug 01 in `PROJECT_PLAN/bugs.md`; the fix belongs in the rule).

## docs/python.md — modify
- **Location:** "Not yet"
- **Change:** replace its first sentence ("Resolving the decisions and the cash cap are Step 08 of Stage 03, and showing traces in the dev UI is Step 09.")
- **Supersedes:** Step 07 (decider-host), the same sentence

Showing traces in the dev UI is Step 09 of Stage 03.

## docs/technologies.md — modify
- **Location:** introduction, the paragraph beginning "Factories (see [factories.md](factories.md)) run technologies."
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same paragraph

Factories (see [factories.md](factories.md)) run technologies. The factory mechanics in [production.md](production.md) implement the rules in [What the numbers mean](#what-the-numbers-mean), and the turn runs them for every business's factories right after its hex's trade round (see [trade.md](trade.md#operating-the-factories)).

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the turn runs, wears and maintains them (see [production.md](production.md) and [trade.md](trade.md)). A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)), and the arithmetic rule tests each technology's margin at posted prices from those numbers (see [python.md](python.md#the-margin-test-per-technology)). Showing businesses and factories in the dev UI is Step 09 of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/dev-ui.md — modify
- **Location:** "Loading and stepping: `Session`", "**Loading.**", the first bullet as Step 07 replaced it ("Loading first drops the previous host, ..."), the parenthetical "(at turn 0, with turn 0's decision point, trade round and growth already run)"
- **Change:** replace the parenthetical
- **Supersedes:** Step 07 (decider-host), the same parenthetical

(at turn 0, with turn 0's decision point, trade round, factory operation and growth already run)

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "Shared with `check`", the `turn_line(world, report)` bullet
- **Change:** replace

- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}; sold {list}; bought {list}; consumed {list}; population {n}; businesses sold {list}; businesses bought {list}; produced {list}; used {list}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn, `sold`, `bought` and `consumed` are the POPs' totals from the turn's trade round, and `population` is the total POP size after growth, printed with `Quantity`'s `Display` (`0` for a world without POPs). `businesses sold` and `businesses bought` are the businesses' totals from the same round, and `produced` and `used` the factories' output and the inputs and maintenance goods they consumed (see [world.md](world.md#turnreport), [trade.md](trade.md) and [growth.md](growth.md)). The report's `decisions` are not printed.

## docs/dev-ui.md — modify
- **Location:** "Tests", first bullet, the parenthetical list of `session.rs` tests
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), its replacement of the same list

`session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, the sample's exact turn 0 and turn 1 lines under the idle host (ending `; businesses sold nothing; businesses bought nothing; produced nothing; used nothing`), selection kept or cleared across reload and open, all with `Hosting::Idle`; a closure decider's failure logged at turn 0 and again at every step; a `Custom` host error shown as a load error with an empty log and no world; and, with the project's Python, the sample loading and stepping under its `arithmetic` decider with no failure lines and its turn 0 line showing the businesses' trade and production, and `production_valid`, whose businesses name `rule`, failing to load with the script's `unknown decider 'rule'` message)

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text.rs` sub-bullets, at the end (after the sub-bullet Step 07 adds)
- **Change:** add

  - `turn_line` with a hand-built `TurnReport` whose business lists are not empty, checking the four business parts and their order after `population`.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, its second sentence (as Step 03 replaced it) and the sentences after it up to "It has its own `repo_path`, ..."; and the sentence Step 07 adds at the end
- **Change:** replace the second sentence and the sentences after it as below, keeping "It has its own `repo_path`, ..." and Step 07's addition
- **Supersedes:** Step 03 (factories), its replacement of the second sentence

Every expected success line includes the markets, POP types, POPs, technologies, businesses and factories counts (`18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `1 technology` for `technologies_one`, `1 business` for `businesses_one`, `1 technology, 1 business, 1 factory` for `factories_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists, its `population` and the four business lists (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0; businesses sold nothing; businesses bought nothing; produced nothing; used nothing` for settings without POPs or businesses). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines (`consumed 35 labor; population 12.5`, with nothing sold or bought, since it has no markets, the same population every turn, since its types don't grow, and every business list `nothing`). The sample's turn 0, 1 and 2 lines, under the `arithmetic` decider, are compared exactly as printed, slivers included: turn 0 shows the businesses buying and producing with nothing sold yet, turns 1 and 2 show them selling last turn's output, and from turn 1 `perished` starts with a `0.0000000000000002220446049250313 unskilled_labor` sliver followed by only the unowned deposits' output (see [world.md](world.md#setting-checker---turns-n)).

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry
- **Change:** replace (keep the `tests/recordings/` and `tests/settings/` lines after it)
- **Supersedes:** Step 07 (decider-host), the same entry

```
    tests/business_trade.rs, businesses.rs, currency.rs, decider.rs, decider_host.rs,
          demand.rs, external_decider.rs, factories.rs, goods.rs, growth.rs, map.rs,
          market.rs, pops.rs, production.rs, recordings.rs, resources.rs,
          setting_loader.rs, technologies.rs, trade.rs, world.rs
```

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", second bullet, the list of test settings
- **Change:** add `business_trade` to the list, after `production_valid` (which Step 04 adds)

## docs/README.md — modify
- **Location:** the document table, the `trade.md` row
- **Change:** replace

| [trade.md](trade.md) | The trade round in part 2 of the turn, for POPs and businesses: POP submissions and demand computed by the market, business submissions resolved from decisions, sales before purchases across a hex, the POP purchase cap and the business cash cap (one proportion for every bid), consumption, operating each business's factories right after its hex's round, leftover currency and slivers, and the per-POP `PopOutcome` and per-business `BusinessTrade` the World records |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row
- **Change:** replace
- **Supersedes:** Step 07 (decider-host), the same row

| [world.md](world.md) | The running `World`, the three-part turn (emission and POP endowment; the decision point, the trade round with factory operation, and POP growth; perishing), hex, POP and business stockpiles with perish groups and removal, factory conditions written back each turn, per-POP trade outcomes, per-business answers and outcomes, turn reports with the total population, the business and factory totals and decider failures, and the setting checker's `--turns` option |

## docs/README.md — modify
- **Location:** the document table, the `production.md` row (which Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same row

| [production.md](production.md) | Factory mechanics as pure functions over one business's factories, called once per business in the turn: the run and maintenance fractions, running at a fraction of capacity with shortages shared proportionally (which is how the cash cap reaches the run level), shutdown below one lifetime-slice, wear with exact factory lifetimes, maintenance and its caps, the run-wear-maintain order, and the per-factory records (`Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord`, `operate`) |
