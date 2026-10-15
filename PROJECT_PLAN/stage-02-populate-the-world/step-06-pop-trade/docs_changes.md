# Step 06: pop-trade — Docs Changes

This Step adds one new document, `docs/trade.md`, for the trade round, and updates the places in earlier Steps' drafts that said trading, consumption or calling demand during the turn "comes in Step 06". Steps 01 to 05's drafts have not been applied to `docs/` yet; where a proposal below replaces part of one of them, it says so in **Supersedes**. The sample's turn lines changed again (`; sold ...; bought ...; consumed ...`, and the POPs' labor no longer perishes), so every quoted turn line is replaced below.

## docs/trade.md — new file
- **Location:** whole file
- **Change:** add

# The trade round

Source: `crates/engine/src/trade.rs` (the round itself), with the World's side in `crates/engine/src/world.rs` (`World::trade`, `World::pop_outcome`). The public type `PopOutcome` is re-exported from the engine crate root; `Submission`, `RefusedTrade` and `hex_round` are crate-private.

The trade round is part 2 of the turn ("actors act", see [world.md](world.md#the-turn)) for POPs. On every hex, each POP hands the hex's market a **submission**: its type's Stone-Geary parameters, its size and its whole stockpile. The market posts its prices, computes every submission's demand with the `demand` function (see [demand.md](demand.md)), settles **every POP's sales before any POP's purchases**, and then each POP **consumes** the utility goods it ended up with. The World keeps each POP's **outcome** (its demand and what it sold, bought and consumed) until the next turn, and the turn report gains the totals.

The structure is shaped for the live market of Stage 05. In this Stage the market is the dummy market (see [market.md](market.md)): fixed prices and any amount traded. Stage 05 replaces the inside of the round (adjusted prices, rationing, aggregate curves built from the submissions); the submission, the sales-before-purchases order, consumption and the recorded outcome stay, so POPs and the World don't change when the market does.

`trade.rs` depends only on `demand`, `goods`, `market`, `pops` (for `PopType`), `quantity` and `stockpile`. It knows nothing about the `World`, `Pops`, `IdVec` or files: the World builds the submissions and puts the results back.

## Where it runs in the turn

`World::new` runs turn 0's start (emission, endowment) and then turn 0's trade round; `World::step` ends the current turn, advances the clock, runs the new turn's start and then its trade round. So between calls the world is **after part 2**: every POP has traded and consumed, and its stockpile holds only what is left (leftover currency, goods it couldn't sell, untradable goods). One `TurnReport` covers one whole turn's trade.

The private `World::trade` does the World's side:

1. For each hex in hex ID order (`map.iter()`), it takes the POPs on the hex (`pops.on_hex`, POP ID order) and skips the hex if there are none.
2. It builds the hex's `Market` directly from the World's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`), not through `World::market`, which would borrow the whole World.
3. It builds one `Submission` per POP. Each submission **owns** the POP's stockpile, moved out with `std::mem::take` (leaving an empty one behind), so the round can hold every submission on the hex at once without borrowing several elements of `pop_stockpiles` mutably.
4. It calls `hex_round`, moves each stockpile back, adds each outcome's `sold`, `bought` and `consumed` to the turn's accumulators, and stores the outcome by `PopId`.
5. After the last hex it rebuilds `pop_outcomes: IdVec<PopKind, PopOutcome>` (every POP is on exactly one hex, so every POP has an outcome) and returns the three lists through `totals_by_good`.

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

`World::pop_outcome(pop) -> &PopOutcome` returns the outcome of the current turn's round. It panics with `is not from this list` for a POP from another world. The outcome is replaced every turn; Step 07 reads its demand numbers for growth, and Step 08 displays it.

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
4. **Consumption** (`consume`). For each good in the POP type's utility, the POP consumes the smaller of what it now holds and what it demanded, with `Stockpile::remove` (soonest-perishing group first). Leisure (the endowed good kept) is consumed this way with no special case. Nothing outside the utility is consumed: currency demanded as a parking place for income, a durable good the POP couldn't sell, and untradable goods stay in the stockpile. With the dummy market a POP always holds what it demanded when it consumes; the "smaller of" rule is for Stage 05, where rationing can leave a POP short. Then `carried_over` is read from the stockpile and the outcome is built.

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

Every turn looks the same, because labor is endowed again each turn and the silver carried over is only a sliver. For the whole sample, every turn reports `sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain` (the 388 peasants with a market at 7.2, 3.6 and 2.8 labor or grain each, plus the size-10 artisan, plus the 120 labor the no-market peasant consumes), and the POPs' labor no longer appears in `perished`.

## Using outcomes

```rust
let (world, report) = World::new(load_setting(path)?);   // turn 0's round has run
let outcome = world.pop_outcome(pop_id);                  // &PopOutcome
let grain = world.rules().goods.id("grain")?;
let bought = outcome.bought_of(grain);
let income = outcome.demand().income();
let poor = outcome.demand().below_subsistence();
let left = outcome.carried_over();                        // in the hex's currency
let traded = (&report.sold, &report.bought, &report.consumed);
```

To change how trade works, change the inside of `hex_round` (the passes, or the `Market` methods they call). To add a new kind of trader, give it a submission and an outcome of its own and run it through the same hex loop in `World::trade`, keeping sales before purchases across everyone on the hex.

## Not yet

Price adjustment, rationing, market inventory or any limit on what the market trades, and aggregating submissions into one demand per type (Stage 05); saving beyond rounding leftovers; rounding rules or a smallest unit of currency; any actor other than POPs trading; per-hex or per-POP breakdowns in the turn report (the per-POP numbers are in `World::pop_outcome`); growth from the outcome (Step 07) and showing outcomes in the dev UI (Step 08).

## Tests

- `trade.rs` unit tests run `hex_round` and the private passes against markets from the `pops_trade` world and hand-built stockpiles: a single peasant selling, buying and consuming; two POPs on a hex giving the same outcomes as each alone (sales before purchases); a hex without a market; an empty slice; a zero demand selling everything with no sliver; the purchase cap reducing a purchase that rounding would make unaffordable, and a rounding surplus left as currency; a POP with no currency buying nothing; consumption taking only utility goods up to the demand, and taking what is held when short; the currency never traded when a utility names it; and `settle_sales` returning a refusal when called against a market that doesn't price a good.
- `crates/engine/tests/trade.rs`: the sample's turn 0 (every peasant with a market and the artisan, with their outcomes and stockpiles), the no-market peasant, the report totals and the perished list, three steps repeating the outcomes with two worlds agreeing; the `pops_trade` table at turn 0 and after a step; currency carried over as income (the miser, the saver); a below-subsistence POP; held goods outside the utility (the collector's gem, the miser's silver); a hex with a market and no POPs; worlds without POPs reporting nothing traded; and `pop_outcome`'s foreign-POP panic.
- `pops_trade` (`crates/engine/tests/settings/pops_trade/`): goods `labor` (perishes after 1), `grain` (after 3), and durable `copper`, `silver` (every hex's currency) and `gem`; hexes (0, 0) to (4, 0) with markets at different prices, (2, 0) without one, and (4, 0) with a market and no POPs; POP types `peasant`, `artisan`, `saver` (also endowed silver), `collector` (also endowed a gem), `miser` (labor with share 0 and copper, which (3, 0) doesn't price, so its income is parked as silver and accumulates), and `idle` (empty endowment). Its `README.md` entry lists every hex, price and POP.

## docs/README.md — modify
- **Location:** the document table, new row after the `demand.md` row (added by Step 05)
- **Change:** add

| [trade.md](trade.md) | The trade round in part 2 of the turn: POP submissions, demand computed by the market, sales before purchases across a hex, the purchase cap, consumption, leftover currency, and the per-POP `PopOutcome` the World records |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same row

| [world.md](world.md) | The running `World`, the three-part turn (emission and POP endowment, the trade round, perishing), hex and POP stockpiles with perish groups and removal, per-POP trade outcomes, turn reports, and the setting checker's `--turns` option |

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** the intro, the paragraph starting "Nothing calls it during a turn yet."
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

The trade round (see [trade.md](trade.md)) calls it for every POP in the "actors act" part of every turn: the market computes each POP's demand from its whole stockpile, sells the surplus and buys the shortfall, and the POP consumes its utility goods up to the demanded quantities. The World keeps each POP's `Demand` in its turn outcome (`World::pop_outcome(pop).demand()`), and POP growth (Step 07) will build its utility index and growth rate from the income, subsistence cost and per-good subsistence status it reports.

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "The model", "### The budget balances", the last two sentences ("This holds up to floating-point rounding only. `demand` promises nothing about exact equality; POP trade (Step 06) handles the difference when it trades.")
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same sentences

This holds up to floating-point rounding only. `demand` promises nothing about exact equality; the trade round caps each purchase at the currency held, so a rounding shortfall trims the last purchase instead of being refused, and a rounding surplus stays as currency (see [trade.md](trade.md)).

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "Using demand", the paragraph after the code block ("`Market` borrows the world immutably ...")
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

`Market` borrows the world immutably and so does `pop_stockpile`, so both can be held at once. After turn 0 a POP's stockpile holds only what the trade round left, so for the demand the round actually used, read `world.pop_outcome(pop_id).demand()`. The round itself computes every `Demand` first (it owns its data and borrows nothing) and then trades with a `&mut` to the POP's stockpile (see [trade.md](trade.md)).

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

A utility index, growth rate, or any effect of being below subsistence (Step 07); rounding or a smallest unit; and demand rules other than the closed form, such as priorities among subsistence goods or saving.

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "Tests", the `crates/engine/tests/demand.rs` bullet
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same bullet

- `crates/engine/tests/demand.rs` uses the sample setting and `world.market(hex)` as the `PriceList`: the peasant and artisan at typical prices, demand linear in size (N 50 versus N 1), a POP on the hex with no market, the demand recorded in a POP's turn-0 outcome equal to the pure function called on its endowment (the center peasant, size 60: 600 labor held, 168 labor and 216 grain demanded), and a sweep over types, sizes, holdings and hexes checking that the budget balances. Quantities are compared within `1e-9`; the budget within `1e-9 × max(1, M)`.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** the second paragraph of the intro, starting "The dummy market is the **seam**"
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph (which itself superseded Step 02's)

The dummy market is the **seam** that Stage 05 replaces with the live market. Callers ask a `Market` for prices and trade through it; when the live market arrives, what stands behind `Market` changes, not the callers. `Market` implements the `PriceList` trait (see [Market as a price list](#market-as-a-price-list)), so Stone-Geary demand (see [demand.md](demand.md)) can be computed against it. The trade round (see [trade.md](trade.md)) is the exchange's user: every turn, on every hex, it computes each POP's demand against the market and settles the POP's sales with `sell_down_to` and its purchases with `buy` and `buy_for`. `market.rs` depends on `demand` (for the trait), `goods`, `id`, `map`, `quantity`, `stockpile` and `turn`, and knows nothing about files.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "Data model", "### `Market<'a>`", the method table: new rows after the `sell_all` row and after the `buy` row
- **Change:** add

After `sell_all`:

| `sell_down_to(&mut stockpile, good, keep) -> Result<(Quantity, Quantity), TradeError>` | Sells everything of `good` above `keep`, so that `keep` remains; returns (amount sold, currency received), `(0, 0)` if the stockpile holds `keep` or less |

After `buy`:

| `buy_for(&mut stockpile, good, spend) -> Result<Quantity, TradeError>` | Spends exactly `spend` of the currency on `good`; returns the amount bought (`spend / price`) |

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "Data model", "### `Market<'a>`", the paragraph starting "`Market::new` is crate-private"
- **Change:** replace

`Market::new` is crate-private. `World::market` builds one from shared borrows of the world, so the world can't be stepped while a `Market` is alive, and a `Market` always trades at the world's current turn. `World::trade` (the trade round, see [trade.md](trade.md)) builds a `Market` from split borrows of the world's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`) while it moves POP stockpiles out of another field and back.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "The exchange", the bullet list (from "**Selling** removes" through "**Buying** removes ...")
- **Change:** replace

- **Selling** removes `amount` of the good from the stockpile (soonest-perishing group first, see [world.md](world.md)) and adds `amount × price` of the currency.
- **Selling everything** (`sell_all`) removes every unit of the good (`Stockpile::remove_all`) and adds `sold × price` of the currency. A good not held sells `(0, 0)`.
- **Selling down to an amount** (`sell_down_to`) removes everything above `keep` (`Stockpile::retain`, soonest-perishing group first) and adds `sold × price` of the currency. When the kept amount lies in a single perish group, exactly `keep` remains; selling `held − keep` with `sell` could leave a remainder one floating-point unit off, which would then show up in reports as a sliver such as `2.8e-14`. `keep = 0` sells everything, like `sell_all`. The trade round uses it for every sale.
- **Buying** removes `amount × price` of the currency (soonest-perishing group first) and adds `amount` of the good.
- **Buying for an amount of currency** (`buy_for`) removes exactly `spend` of the currency and adds `spend / price` of the good. The trade round uses it to cap a purchase at the currency held: buying `held / price` with `buy` could still be refused when `(held / price) × price` rounds above `held`. A zero spend buys nothing.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "The exchange", "### Refusals", the paragraph starting "`sell_all` can only fail with the first two."
- **Change:** replace

`sell_all` and `sell_down_to` can only fail with the first two. `buy_for` fails with `NotEnoughCurrency { currency, held, needed: spend }` when the stockpile holds less than `spend`. A zero amount (or zero spend) of a traded good passes every check, changes nothing, and returns zero (a zero amount of an untraded good or of the currency is still refused). `TradeError` implements `Display` and `std::error::Error`; IDs are written as `#index` because the error has no access to names:

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "Using markets", the paragraph after the code block ("`Stockpile` is public and `Clone`, ...")
- **Change:** replace

`Stockpile` is public and `Clone`, but its changing methods are crate-private, so code outside the engine can only trade with a stockpile it obtained from the world (as the tests do with clones of hex stockpiles). Inside the engine, POPs trade in the "actors act" part of every turn (see [trade.md](trade.md)); hex stockpiles are never traded during the turn.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace

Changing prices during play (and what happens to them when a hex's currency changes), a market stockpile, rationing or any limit on volume, prices in another hex's currency or exchange between currencies, rounding or a smallest unit of currency, actors other than POPs trading, and showing markets in the dev UI (Stage 02 Step 08). Stage 05 replaces the posted prices and the exchange with live market outcomes inside the trade round; the `[[market]]` section may survive as starting prices or be dropped then.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "The sample setting and tests", the `crates/engine/tests/market.rs` bullet
- **Change:** replace

- `crates/engine/tests/market.rs`: the sample's prices on the center, a typical hex and the edge hex without a market (and `silver` at 1 everywhere, 18 markets); `markets_valid` (prices across files and both table forms, an empty `prices`, a hex without a market, the same result on a second load); every `markets_errors` error at its place; selling, `sell_all` and buying on `turns_valid` (amounts, perish groups of what was added, refusals leaving the stockpile unchanged); `sell_down_to` (leaving exactly the kept amount in its original group, selling nothing at or below it, keep 0 matching `sell_all`, refusing an untraded good and the currency); `buy_for` (spending exactly, the perish group of the good bought, refusing a shortfall, a zero spend); the soonest-perishing group sold first; zero amounts; prices unchanged by stepping; and the foreign-hex panic of `World::market`. The trade round is tested in `crates/engine/tests/trade.rs` (see [trade.md](trade.md)).

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Steps 04 and 05)
- **Location:** the opening, the paragraph starting "The names follow one rule"
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

The names follow one rule: "POP" means the placed group, so `PopId` is a placed POP's ID and `PopTypeId` is a POP type's. Every turn, after the endowment, each POP trades and consumes in the trade round (see [trade.md](trade.md)): the hex's market computes its demand (see [demand.md](demand.md)) from its type's subsistence amounts and shares, sells what it doesn't keep and buys what it wants through the dummy market (see [market.md](market.md)), and the POP consumes its utility goods. Nothing grows yet: Step 07 changes a POP's size.

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", new subsection after "### Endowment in the turn" and before "### Using POPs"
- **Change:** add

### Trade and consumption in the turn

Right after the endowment, the trade round runs on every hex (see [trade.md](trade.md)). Each POP submits its type, its size and its whole stockpile; the market computes its demand, settles its sales (after every POP's sales on the hex, its purchases), and the POP consumes, of each utility good, the smaller of what it holds and what it demanded. Goods outside the utility are never consumed. Leftover currency stays in the stockpile and is income next turn. So between calls a POP's stockpile holds only what the round left: in the sample, a typical POP holds at most a sliver of `silver`, and the no-market peasant on (2, -2) holds nothing, having consumed its whole labor endowment. The World keeps each POP's outcome of the current turn, read with `World::pop_outcome(pop)`.

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", "### Using POPs", the code block
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same code block

```rust
let (world, report) = World::new(load_setting(path)?);
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
let labor = world.rules().goods.id("unskilled_labor")?;
for &id in world.pops().on_hex(center) {                // ID order
    let pop = world.pops().get(id);
    let type_name = world.rules().pop_types.name(pop.pop_type);
    let left = world.pop_stockpile(id);                 // what the trade round left
    let outcome = world.pop_outcome(id);                // this turn's trade
    let labor_endowed = outcome.demand().get(labor).map(|g| g.held);
    let labor_sold = outcome.sold_of(labor);
}
let artisan = world.rules().pop_types.id("artisan")?;
let artisans_here = world.pops().of_type_on_hex(center, artisan);   // Option<PopId>
```

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

Growth and its parameters, and what happens to a POP that shrinks toward zero (Step 07), and showing POP types, POPs and their trade outcomes in the dev UI (Step 08). Starting stockpiles or cash for POPs, saving, changing a POP's hex or type during play, migration, and per-POP parameters that differ from the type are not planned for this Stage.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "The sample setting and tests", the `crates/engine/tests/pops.rs` bullet and the test-settings bullet
- **Change:** replace both
- **Supersedes:** Step 04 (pops), the same two bullets

- `crates/engine/tests/pops.rs`: the sample's two types with their normalized shares (summing to 1); `pops_valid` with IDs in load order, both table forms, a multi-good endowment written out of ID order, an empty endowment, a pure necessity, the currency in a utility, and the same result on a second load; every `pops_errors` error at its place; `valid_minimal` with no POP types. For placed POPs: the sample's 16 POPs with their hexes, types and sizes and its center, corner and empty hexes; a fresh sample world in which every POP's recorded demand shows `10 × size` labor held, every POP stockpile holds no labor and at most a sliver of silver, and the report's `endowed` is 4100 labor; three steps that re-endow the same amounts, with the labor used up rather than perishing; `pops_placed` loaded and stepped (the workers consume their labor, the hoarder keeps its labor and its growing gold); every `pops_placed_errors` error at its place; worlds with no POPs endowing and trading nothing; and `pop_stockpile` panicking on a POP from another world.
- Test settings: `pops_valid` (goods `coin`, `labor`, `grain`, `fish`; types `farmer`, `fisher`, `idle` across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, with the goods in a file read after them); `pops_one` (one good, one type, one hex and one POP, for the checker's `1 pop type`, `1 hex` and `1 pop`); `pops_errors`; `pops_placed` (goods `labor` and `grain`, both perishing after 1 turn, and durable `gold`; types `worker` endowing 10 `labor` with a `labor` utility, `hoarder` endowing 4 `labor` and 1.5 `gold` with a `grain` utility, and `idle` with an empty endowment; hexes (0, 0), (1, 0), (2, 0), each with currency `gold` and none with a market, and a `field` deposit on (2, 0); POPs `worker` 3 and `hoarder` 2 on (0, 0) in `start/a.toml`, `worker` 0.5 and `idle` 7 on (1, 0) in `start/b/c.toml`. With no market, the workers consume their labor and the hoarder, whose utility good it doesn't hold, keeps its labor and its gold, which accumulates); `pops_placed_errors` (every `[[pop]]` error in the table above, one entry with four problems, a duplicate across files, and a `[[pop]]` in `rules/`); and `pops_trade`, for the trade round (see [trade.md](trade.md)).

## docs/world.md — modify
- **Location:** "What a World holds", the field table: the `hex_stockpiles` row (as replaced by Step 02) and the `pop_stockpiles` row (added by Step 04), plus a new row after `pop_stockpiles`
- **Change:** replace the two rows and add one
- **Supersedes:** Step 02 (dummy-market), the `hex_stockpiles` row; Step 04 (pops), the `pop_stockpiles` row

| `hex_stockpiles: IdVec<HexKind, Stockpile>` | empty at creation | goods enter by emission and leave by perishing; nothing in the turn trades them |
| `pop_stockpiles: IdVec<PopKind, Stockpile>` | one empty stockpile per POP at creation | goods enter by endowment and purchase, and leave by sale, consumption and perishing (see [trade.md](trade.md)) |
| `pop_outcomes: IdVec<PopKind, PopOutcome>` | built by turn 0's trade round | rebuilt every turn by the trade round: each POP's demand and what it sold, bought, consumed and carried over (see [trade.md](trade.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same paragraph

The world owns the setting, so callers (the dev UI) have no lifetimes to manage. There is one stockpile of **unowned goods** per hex, in an `IdVec` indexed by `HexId`, and one stockpile per POP, the goods that POP owns, in an `IdVec` indexed by `PopId` (see [ids.md](ids.md)). Both use the same `Stockpile` type. Deposits have no owners in this Stage, so their output still goes to the hex stockpiles. Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)`, `hex_stockpile(hex)`, `pops()`, `pop_stockpile(pop)` and `pop_outcome(pop)`. `market(hex)` returns the hex's `Market` view (its prices in the hex's currency and the exchange, at the current turn; see [market.md](market.md)); because it borrows the world, the world can't be stepped while a `Market` is alive. `pop_outcome(pop)` returns the POP's `PopOutcome` from the current turn's trade round (see [trade.md](trade.md)). `hex_currency`, `market` and `hex_stockpile` panic with `is not from this list` for a hex from another map, and `pop_stockpile` and `pop_outcome` for a POP from another world's `Pops` (a caller bug). The turn reads prices through the trade round but changes neither currencies nor prices.

## docs/world.md — modify
- **Location:** "The turn", from the numbered list through the paragraph starting "So between calls the world is always at part 2"
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same span, and Step 05 (stone-geary-demand), item 2 of the numbered list

A turn has three parts:

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good into its hex's stockpile. Then every POP receives its type's endowment times its size into its own stockpile (see [Endowment](#endowment)).
2. **Actors act:** the trade round and consumption (see [Trade round](#trade-round) and [trade.md](trade.md)). On every hex, each POP submits its parameters and holdings, the hex's market computes their demand and settles every sale before any purchase, and each POP consumes its utility goods. Later Stages add industries and the live market here.
3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile and then from every POP stockpile.

Two calls drive the clock:

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 and runs turn 0's start and turn 0's trade round, so a fresh world's deposits have emitted and its POPs have already been endowed, traded and consumed. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing. Then it runs the end of the current turn (perishing: hexes in ID order, then POPs in ID order), advances the clock, and runs the start of the new turn (emission, then endowment) and its trade round.

So between calls the world is always **after part 2**: this turn's output is in the hex stockpiles, this turn's trade is done, each POP's stockpile holds what the round left, and nothing has perished yet this turn. This is the state that tests, the checker, and the dev UI observe. If observation happened after the end of the turn, goods with `perishes_after = 1` would always look empty.

## docs/world.md — modify
- **Location:** "The turn", new subsection after "### Endowment" (added by Step 04) and before "### Perishing"
- **Change:** add

### Trade round

`World::trade` (private) runs after `endow`. For each hex in hex ID order that has POPs, it builds the hex's `Market` from the world's fields, moves each POP's stockpile out into a `Submission` (`std::mem::take`), calls `trade::hex_round`, moves the stockpiles back, and stores each `PopOutcome` by `PopId`. Then it replaces `pop_outcomes` and returns the totals sold, bought and consumed through `totals_by_good`, which become the report's `sold`, `bought` and `consumed`. A refused trade can't happen after the demand and the purchase cap; if one does, it panics with `engine bug: the market on hex #<hex> refused a trade for POP #<pop>: <error>`. The loop reads `map`, `pops`, `rules`, `hex_currencies` and `hex_prices` and writes `pop_stockpiles`, as separate fields, so the borrows don't conflict. See [trade.md](trade.md) for the round itself.

## docs/world.md — modify
- **Location:** "The turn", "### Endowment" (added by Step 04), its last paragraph's second sentence ("`endow` borrows `self.rules` and `self.pops` while mutating `self.pop_stockpiles`; they are separate fields, so Step 06 can likewise borrow a POP's stockpile mutably while a `Market` borrows the rules and prices.")
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same sentence

`endow` borrows `self.rules` and `self.pops` while mutating `self.pop_stockpiles`; they are separate fields, and the trade round relies on the same split (see [Trade round](#trade-round)).

## docs/world.md — modify
- **Location:** "The turn", "### Determinism", the first sentence (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same sentence

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and goods in ID order within a POP, with every sale on a hex before any purchase and then consumption, stockpiles perish hexes first and then POPs, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds, outcomes and reports.

## docs/world.md — modify
- **Location:** "The turn", "### Determinism", the second sentence ("`World` itself is not `PartialEq`, but `Stockpile` and `TurnReport` are, which is how tests compare two runs.")
- **Change:** replace

`World` itself is not `PartialEq`, but `Stockpile`, `PopOutcome` and `TurnReport` are, which is how tests compare two runs.

## docs/world.md — modify
- **Location:** "TurnReport", the whole section body
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same section

```rust
pub struct TurnReport {
    pub turn: Turn,                         // the turn that started
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn, hexes and POPs
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted by deposits at the start of `turn`
    pub endowed: Vec<(GoodId, Quantity)>,   // received by POPs as endowment at the start of `turn`
    pub sold: Vec<(GoodId, Quantity)>,      // sold by POPs in the trade round of `turn`
    pub bought: Vec<(GoodId, Quantity)>,    // bought by POPs in the trade round of `turn`
    pub consumed: Vec<(GoodId, Quantity)>,  // consumed by POPs after the trade round of `turn`
}
```

Each list has one total per good, summed over every hex (and, for `perished`, `endowed`, `sold`, `bought` and `consumed`, every POP), in good ID order, and lists only amounts greater than zero. `emitted` and `endowed` are kept apart so deposit output and POP income can be told apart. There is no per-hex or per-POP breakdown in the report: what each holds is in its stockpile, and what each POP traded is in its `PopOutcome` (`World::pop_outcome`). For the sample setting turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`; endowed 4100 `unskilled_labor`; sold 2878.6 `unskilled_labor`; bought 5 `copper` and 1426.8 `grain`; consumed 1221.4 `unskilled_labor`, 5 `copper` and 1426.8 `grain`". Every later step reports the same, except that `perished` is the deposit output only: the POPs sell or consume all their labor in the round, so none is left to perish (apart from floating-point slivers, which the checker would print).

## docs/world.md — modify
- **Location:** "Stockpile", the "Changing (`pub(crate)`, engine only):" list and the paragraph after it (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same span

Changing (`pub(crate)`, engine only):

- `add(good, amount, at)` merges `amount` into the group `at`; a zero amount changes nothing.
- `remove(good, amount) -> Result<(), Quantity>` takes `amount` out, **soonest-perishing group first**, then the next, so what would spoil first is used first and durable units are kept longest. It is all-or-nothing: if the stockpile holds less than `amount`, it returns `Err(held)` (the total held) and changes nothing; there is no partial removal. A zero amount is `Ok` and changes nothing. A group that reaches zero is dropped, and so is a good left with no groups, so the stockpile stays sparse. Because the check compares against the floating-point total of the groups, a tiny remainder from rounding can be left unremoved when the groups run out; that is accepted.
- `retain(good, keep) -> Quantity` removes everything of `good` above `keep`, soonest-perishing group first, and returns the amount removed (zero, changing nothing, if it holds `keep` or less). It walks the groups from the latest-perishing back, keeping whole groups while they fit in `keep`; the group where `keep` runs out is set to exactly the remainder (or dropped if that is zero), and every sooner group is dropped. So when the kept amount lies in one group, exactly `keep` remains, with no rounding sliver; `keep = 0` removes the good.
- `remove_all(good) -> Quantity` removes every unit of `good` and returns the total removed (zero if none was held).
- `remove_perishing(turn)` removes every group at `EndOfTurn(t)` with `t <= turn` (not only `t == turn`, so no group can be stranded), drops goods left with no groups, and returns the totals removed per good.

There is no public way to change a stockpile. Removal is used by the dummy market (see [market.md](market.md)), which is the only way code outside the engine can change one: a caller passes a stockpile it owns (in tests, a clone of a hex stockpile) to `Market::sell`, `sell_all`, `sell_down_to`, `buy` or `buy_for`. Inside the engine, the trade round sells POP goods with `sell_down_to` (which uses `retain`) and removes consumed goods with `remove` (see [trade.md](trade.md)). A `GoodId` does not say which `Goods` list it came from, so a stockpile simply holds the IDs it is given; the world only gives it IDs from its own rules.

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`", the example block and the sentence after it
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same block and sentence

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain
Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain
```

The line lists `perished`, `emitted`, `endowed`, `sold`, `bought` and `consumed` in that order. Entries are `<amount> <good name>` joined by `, ` in report order (good ID order: `copper` comes before `grain` in the sample's goods); an empty list prints `nothing`, so a setting without POPs ends every line with `; endowed nothing; sold nothing; bought nothing; consumed nothing`. The amounts are floating-point sums printed with `Quantity`'s `Display`, so a total can in principle print with a long tail of digits; the sample's print exactly.

## docs/world.md — modify
- **Location:** "Tests", the unit-tests bullet (as replaced by Step 05)
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same bullet

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub`, `saturating_mul` and `checked_div`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, zero, and `remove_all`; and `retain`: from the soonest group first, exactly `keep` left in a single group, `keep = 0`, at or below `keep`, and `keep` on a group boundary), `world.rs` (a world forced to `Turn::MAX`, for both `turns_valid` and `pops_placed`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles and POP outcomes unchanged), and `trade.rs` (see [trade.md](trade.md)). Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)), demand in `crates/engine/tests/demand.rs` (see [demand.md](demand.md)), and the trade round in `crates/engine/tests/trade.rs`.

## docs/world.md — modify
- **Location:** "Tests", the `crates/engine/tests/world.rs` bullet (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports (the POPs' labor in `endowed`, and only the deposit output in `perished`, since POPs use up their labor), a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits (which also endow and trade nothing), determinism (reports, hex stockpiles and POP stockpiles of two runs), the accessors, hex currencies being readable and unchanged by stepping, and the foreign-hex panics of `hex_stockpile` and `hex_currency`. POP endowment in the world is tested in `crates/engine/tests/pops.rs` (see [pops.md](pops.md)), and the trade round in `crates/engine/tests/trade.rs` (see [trade.md](trade.md)).

## docs/world.md — modify
- **Location:** "Not yet", the whole paragraph (as replaced by Step 05)
- **Change:** replace
- **Supersedes:** Step 05 (stone-geary-demand), the same paragraph

Ownership of deposits, starting stockpiles in `start/` (for hexes or POPs), deposit stock, depletion and regeneration, any effect of `movable`, per-hex and per-POP report breakdowns (per-POP trade is in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), changes to POPs during play (size from Step 07), and actors other than POPs in the "actors act" part of the turn.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `turn_line` bullet (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}; sold {list}; bought {list}; consumed {list}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn, and `sold`, `bought` and `consumed` are the POPs' totals from the turn's trade round (see [world.md](world.md) and [trade.md](trade.md)).

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets, POP types and POPs counts (`18 markets, 2 pop types, 16 pops` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `3 pop types, 4 pops` for `pops_placed`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists (`; endowed nothing; sold nothing; bought nothing; consumed nothing` for settings without POPs). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines (`consumed 35 labor`, with nothing sold or bought, since it has no markets). It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules. The `session.rs` unit tests compare the sample's turn 0 and turn 1 lines in the log with the same text.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", the second bullet (the list of test settings)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `markets_valid`, `markets_errors`, `pops_valid`, `pops_one`, `pops_errors`, `pops_placed`, `pops_placed_errors`, `pops_trade`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/currency.rs, ...`
- **Supersedes:** Step 05 (stone-geary-demand), the same block

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/demand.rs       Stone-Geary demand and the PriceList trait (see demand.md)
    src/goods.rs        goods data model (see goods.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types and placed POPs (see pops.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/trade.rs        the trade round and PopOutcome (see trade.md)
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, demand.rs, goods.rs, map.rs, market.rs, pops.rs, resources.rs, setting_loader.rs, trade.rs, world.rs
```
