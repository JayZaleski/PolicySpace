# The trade round

Source: `crates/engine/src/trade.rs` (the round itself), with the World's side in `crates/engine/src/world.rs` (`World::trade`, `World::pop_outcome`). The public type `PopOutcome` is re-exported from the engine crate root; `Submission`, `RefusedTrade` and `hex_round` are crate-private.

The trade round is part 2 of the turn ("actors act", see [world.md](world.md#the-turn)) for POPs. On every hex, each POP hands the hex's market a **submission**: its type's Stone-Geary parameters, its size and its whole stockpile. The market posts its prices, computes every submission's demand with the `demand` function (see [demand.md](demand.md)), settles **every POP's sales before any POP's purchases**, and then each POP **consumes** the utility goods it ended up with. The World keeps each POP's **outcome** (its demand, what it sold, bought and consumed, and its subsistence index and growth rate) until the next turn, and the turn report gains the totals. After every hex has traded, the World applies each outcome's growth rate to the POP's size (see [growth.md](growth.md)).

The structure is shaped for the live market of Stage 05. In this Stage the market is the dummy market (see [market.md](market.md)): fixed prices and any amount traded. Stage 05 replaces the inside of the round (adjusted prices, rationing, aggregate curves built from the submissions); the submission, the sales-before-purchases order, consumption and the recorded outcome stay, so POPs and the World don't change when the market does.

`trade.rs` depends only on `demand`, `goods`, `growth`, `market`, `pops` (for `PopType`), `quantity` and `stockpile`. It knows nothing about the `World`, `Pops`, `IdVec` or files: the World builds the submissions and puts the results back.

## Where it runs in the turn

`World::new` and `World::step` both run a turn through the private `World::run_turn`, which runs the turn's start (emission, endowment), then the trade round, then growth, and builds the `TurnReport`. So between calls the world is **after part 2**: every POP has traded, consumed and grown, and its stockpile holds only what is left (leftover currency, goods it couldn't sell, untradable goods). One `TurnReport` covers one whole turn's trade. Because growth has already run, a POP's current size is not the size the turn's trade was for; that size is the outcome's `starting_size()`.

The private `World::trade` does the World's side:

1. For each hex in hex ID order (`map.iter()`), it takes the POPs on the hex (`pops.on_hex`, POP ID order) and skips the hex if there are none.
2. It builds the hex's `Market` directly from the World's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`), not through `World::market`, which would borrow the whole World.
3. It builds one `Submission` per POP. Each submission **owns** the POP's stockpile, moved out with `std::mem::take` (leaving an empty one behind), so the round can hold every submission on the hex at once without borrowing several elements of `pop_stockpiles` mutably.
4. It calls `hex_round`, moves each stockpile back, adds each outcome's `sold`, `bought` and `consumed` to the turn's accumulators, and stores the outcome by `PopId`.
5. After the last hex it rebuilds `pop_outcomes: IdVec<PopKind, PopOutcome>` (every POP is on exactly one hex, so every POP has an outcome) and returns the three totals through `totals_by_good` as a private `TradeTotals { sold, bought, consumed }`, which `run_turn` destructures into the report.

Every POP submits, including a POP on a hex with no market: its `Market` posts no prices, so `demand` keeps everything it holds and nothing is traded.

## Data model

### `Submission`

```rust
pub(crate) struct Submission<'a> {
    pub pop_type: &'a PopType,   // the utility parameters
    pub size: Quantity,          // N
    pub stockpile: Stockpile,    // moved out of the World for the round, moved back after
}
```

The POP computes nothing itself; its submission is its parameters and holdings, not an answer. The round trades from and into `stockpile`, and consumption removes from it. Industries (Stage 03) are expected to submit through the same round in their own form.

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

### `hex_round`

```rust
pub(crate) fn hex_round(market: &Market<'_>, submissions: &mut [Submission<'_>])
    -> Result<Vec<PopOutcome>, RefusedTrade>
```

Runs one hex's round for its POPs (passed in POP ID order) and returns one outcome per submission, in the same order. An empty slice returns an empty vector. `RefusedTrade { submission: usize, error: TradeError }` names the submission (its index in the slice) and the exchange's refusal.

## The four passes

`hex_round` makes four passes over the submissions, each in slice order, and each pass finishes for every POP before the next starts.

1. **Demand.** `demand(s.pop_type, s.size, s.stockpile.iter(), market)` for every submission. The whole stockpile is the holdings, so durable goods and currency carried over from earlier turns count as income, exactly as the endowment does. `Market` is a `PriceList`, so the posted prices go straight in.
2. **Sales** (`settle_sales`). For each good in the demand, in good ID order, skipping the hex's currency: if the stockpile holds more than the demanded quantity, `Market::sell_down_to(stockpile, good, demanded)` sells everything above it. When the demanded quantity is zero this sells everything, so a good the POP doesn't want never leaves a rounding sliver. Untradable goods are demanded at their held amount, so they are never sold.
3. **Purchases** (`settle_purchases`). For each good in the demand, in good ID order, skipping the currency: if the demanded quantity exceeds the holding, the POP buys the difference, **capped at what its currency affords**. If `difference × price` is at most the currency held, `Market::buy` buys the difference; otherwise `Market::buy_for` spends exactly all the currency held. The budget balances only up to floating-point rounding, so without the cap the last purchase could be refused for a few units in the last decimal place. With the dummy market the cap only ever trims a rounding shortfall, which lands on the last good bought; a POP with no currency buys nothing.
4. **Consumption** (`consume`). For each good in the POP type's utility, the POP consumes the smaller of what it now holds and what it demanded, with `Stockpile::remove` (soonest-perishing group first). Leisure (the endowed good kept) is consumed this way with no special case. Nothing outside the utility is consumed: currency demanded as a parking place for income, a durable good the POP couldn't sell, and untradable goods stay in the stockpile. With the dummy market a POP always holds what it demanded when it consumes; the "smaller of" rule is for Stage 05, where rationing can leave a POP short. Then `carried_over` is read from the stockpile, the POP's subsistence index and growth rate are computed from its demand and its type's growth parameters (`growth::subsistence_index` and `growth::growth_rate`, see [growth.md](growth.md)), and the outcome is built with the submission's size as `starting_size`. The round only records the rate; the World applies it after every hex has traded.

Sales settle before purchases **across the whole hex**, not per POP, because in the live market a POP rationed when selling has less income than it planned, and every purchase must see the income the sales actually produced. With the dummy market the order makes no visible difference.

### The currency is never traded

Both settlement passes skip the hex's currency, and the exchange would refuse it anyway (`IsCurrency`). Proceeds add currency and purchases spend it; because the budget balances, what is left after purchases is the demanded quantity of currency, up to rounding. That covers both cases where the currency is demanded: the parking place when no good with a share is priced (see [demand.md](demand.md)), and a utility that names the currency, in which case it is consumed like any utility good.

### Leftover currency

Whatever currency a POP holds after consumption stays in its stockpile and is reported as `carried_over`. A durable currency (the sample's `silver`) survives the end of the turn and is income in the next turn's demand; a perishable one perishes by its own rule. The engine does no rounding: a typical POP carries over a sliver below `1e-9`, or nothing.

### A refused trade is an engine bug

After the demand and the cap, no trade can be refused: sales never exceed holdings, purchases are capped, untradable goods and the currency are never traded. If the exchange refuses anyway, `hex_round` stops at that trade and returns `RefusedTrade`, and `World::trade` panics with `engine bug: the market on hex #<hex> refused a trade for POP #<pop>: <error>`. The World is never handed back half-stepped. `consume` likewise panics with an `engine bug` message if a removal it has already checked fails.

## Determinism

Hexes in hex ID order, POPs in POP ID order within a hex, goods in good ID order within a POP, every sale on a hex before any purchase, then consumption: the same setting stepped the same number of times gives the same stockpiles, outcomes and reports.

## Worked examples

From the sample setting, per individual (every number scales with N), at a typical hex's prices (labor 1, grain 2, copper 5, in `silver`):

| POP | Demand | Sold | Bought | Consumed | Left |
|---|---|---|---|---|---|
| `peasant`, holding 10 labor | labor 2.8, grain 3.6 | 7.2 labor for 7.2 silver | 3.6 grain for 7.2 | 2.8 labor, 3.6 grain | a silver sliver at most |
| `artisan` at the center, holding 10 labor | labor 1.5, grain 3, copper 0.5 | 8.5 labor for 8.5 silver | 3 grain for 6, 0.5 copper for 2.5 | 1.5 labor, 3 grain, 0.5 copper | a silver sliver at most |
| `peasant` on (2, -2), no market | labor 10 kept, grain 0 | nothing | nothing | 10 labor | nothing; below subsistence on grain |
| `peasant`, grain at 10 | labor and grain 10/11 each | 100/11 labor | 10/11 grain | 10/11 labor and grain | below subsistence |

Per individual, every turn looks the same, because labor is endowed again each turn and the silver carried over is only a sliver. The totals change from turn to turn because the POPs grow (see [growth.md](growth.md)). At turn 0, with the setting's sizes, the sample reports `sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain` (the 388 peasants with a market at 7.2, 3.6 and 2.8 labor or grain each, plus the size-10 artisan, plus the 120 labor the no-market peasant consumes). At turn 1 the same per-individual amounts times the grown sizes give `sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain`. The POPs' labor never appears in `perished`.

## Using outcomes

```rust
let (world, report) = World::new(load_setting(path)?);   // turn 0's round and growth have run
let outcome = world.pop_outcome(pop_id);                  // &PopOutcome
let grain = world.rules().goods.id("grain")?;
let bought = outcome.bought_of(grain);
let income = outcome.demand().income();
let poor = outcome.demand().below_subsistence();
let left = outcome.carried_over();                        // in the hex's currency
let traded_for = outcome.starting_size();                 // the size this trade was for
let (u, rate) = (outcome.subsistence_index(), outcome.growth_rate());
let traded = (&report.sold, &report.bought, &report.consumed);
```

To change how trade works, change the inside of `hex_round` (the passes, or the `Market` methods they call). To add a new kind of trader, give it a submission and an outcome of its own and run it through the same hex loop in `World::trade`, keeping sales before purchases across everyone on the hex.

## Not yet

Price adjustment, rationing, market inventory or any limit on what the market trades, and aggregating submissions into one demand per type (Stage 05; with rationing, the subsistence index will need to reconsider whether it reads what was demanded or what was consumed); saving beyond rounding leftovers; rounding rules or a smallest unit of currency; any actor other than POPs trading; and per-hex or per-POP breakdowns in the turn report (the per-POP numbers are in `World::pop_outcome`).

## Tests

- `trade.rs` unit tests run `hex_round` and the private passes against markets from the `pops_trade` world and hand-built stockpiles: a single peasant selling, buying and consuming; two POPs on a hex giving the same outcomes as each alone (sales before purchases); a hex without a market; an empty slice; a zero demand selling everything with no sliver; the purchase cap reducing a purchase that rounding would make unaffordable, and a rounding surplus left as currency; a POP with no currency buying nothing; consumption taking only utility goods up to the demand, and taking what is held when short; the currency never traded when a utility names it; and `settle_sales` returning a refusal when called against a market that doesn't price a good. They take `q`, `close` and `no_growth` from `crates/engine/src/test_support.rs`; their `hex(world, x)` and `good(world, name)` helpers, which need a world, stay local.
- `crates/engine/tests/trade.rs`: the sample's turn 0 (every peasant with a market and the artisan, with their outcomes and stockpiles, sizes taken from `starting_size()`), the no-market peasant, the report totals and the perished list at turn 0, and turn 1's totals computed from the grown sizes; three steps repeating the outcomes per individual (each amount and the income divided by `starting_size()`) with two worlds agreeing; the `pops_trade` table at turn 0 and after a step (zero growth, so sizes don't change); currency carried over as income (the miser, the saver); a below-subsistence POP; held goods outside the utility (the collector's gem, the miser's silver); a hex with a market and no POPs; worlds without POPs reporting nothing traded and a population of 0; and `pop_outcome`'s foreign-POP panic.
- `pops_trade` (`crates/engine/tests/settings/pops_trade/`): goods `labor` (perishes after 1), `grain` (after 3), and durable `copper`, `silver` (every hex's currency) and `gem`; hexes (0, 0) to (4, 0) with markets at different prices, (2, 0) without one, and (4, 0) with a market and no POPs; POP types `peasant`, `artisan`, `saver` (also endowed silver), `collector` (also endowed a gem), `miser` (labor with share 0 and copper, which (3, 0) doesn't price, so its income is parked as silver and accumulates), and `idle` (empty endowment). Every type has zero growth parameters. Its `README.md` entry lists every hex, price and POP.
