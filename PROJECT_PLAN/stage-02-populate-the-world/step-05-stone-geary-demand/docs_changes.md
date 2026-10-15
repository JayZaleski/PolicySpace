# Step 05: stone-geary-demand — Docs Changes

This Step adds one new document, `docs/demand.md`, and updates the places in earlier Steps' drafts that said demand "comes in Step 05". Steps 02 to 04's drafts have not been applied to `docs/` yet; where a proposal below replaces part of one of them, it says so in **Supersedes**.

## docs/demand.md — new file
- **Location:** whole file
- **Change:** add

# Stone-Geary demand

Source: `crates/engine/src/demand.rs`. The public items are re-exported from the engine crate root: `PriceList`, `DemandedGood`, `Demand`, and the function `demand`.

`demand` works out what a POP wants to **end up with** this turn: the quantity of each good after it has sold what it doesn't keep and bought what it wants. Its inputs are the POP type's Stone-Geary utility (see [pops.md](pops.md)), the POP's size N, what the POP holds, and a hex's prices. It is a **pure function**: it reads its inputs, returns a `Demand`, and changes no stockpile, trades nothing and touches no world state. It doesn't round anything.

Nothing calls it during a turn yet. POP trade (Stage 02 Step 06) will call it in the "actors act" part of the turn, compare each good's demanded quantity with what the POP holds, sell the surplus and buy the shortfall through the dummy market (see [market.md](market.md)). POP growth (Step 07) will build its utility index and growth rate from the income, subsistence cost and per-good subsistence status the result reports.

`demand.rs` depends only on `goods`, `pops` (for `PopType`) and `quantity`. It knows nothing about the `World`, `Stockpile`, `Pops`, markets or files. Prices come in through the `PriceList` trait and holdings as any iterator of goods and amounts, so the function can be tested with hand-written prices and lists.

## The model

A POP type's utility lists, for each good an individual wants, a **subsistence** amount (needed before anything else) and a normalized **share** of spending beyond subsistence. `demand` multiplies every subsistence amount by N up front, so every amount it works with and reports is for the whole POP. Demand is linear in N, so this gives the same result as computing per individual and scaling.

### Tradable and untradable goods

The **involved** goods are every utility good and every held good. Each is either **tradable** on this hex (`PriceList::price` returns a price; the currency is priced at 1) or **untradable** (`None`). The optimization runs over the tradable goods only:

- An untradable good the POP holds is **kept whole**: its demanded quantity is the held amount, whether or not it is in the utility. It adds nothing to income, since it can't be sold.
- An untradable utility good the POP doesn't hold gets **zero**: it can't be bought.
- The shares of the tradable utility goods are **renormalized** to sum to 1 over that set. With some quantities fixed, this is the exact conditional Stone-Geary demand, not an approximation.

### Income and the subsistence bundle

- **Income M** is the value of every tradable held good at its price. Currency held counts at price 1.
- **Subsistence cost C** is the value of the tradable part of the subsistence bundle: the sum over tradable utility goods of subsistence × N × price.
- **Supernumerary income** is M − C. It is the one signed number in the result, so it is a plain `f64`; it is negative below subsistence.

### At or above subsistence (M ≥ C)

Every tradable utility good gets its subsistence × N plus its renormalized share of the supernumerary income, divided by its price:

```
quantity = subsistence × N + (share / total tradable share) × (M − C) / price
```

- A **pure necessity** (share 0) gets exactly its subsistence × N.
- A tradable held good **not in the utility** gets zero: it is all for sale.
- **Currency fallback.** If no tradable utility good has a positive share (for example the only good with a share isn't priced on this hex), the supernumerary income has nowhere to go. It is demanded as the currency (`PriceList::currency`), which is a good at price 1, so the budget still balances. The currency then appears in the result even if the POP neither holds nor wants it. When M − C is zero, nothing is added and no currency entry is created.

### Below subsistence (M < C)

The POP spends everything buying toward subsistence. Every tradable utility good gets its subsistence × N scaled by the same fraction M / C, including leisure goods such as labor. So a peasant facing a grain price spike sells more labor than its leisure subsistence allows and still falls short of grain in the same proportion. Tradable goods with zero subsistence, and tradable held goods not in the utility, get zero. C is positive whenever this case occurs, so the division is defined. A POP that holds nothing tradable has M = 0 and, if C is positive, demands nothing tradable.

The two rules meet at M = C: both give exactly the subsistence bundle.

### The budget balances

In every case the demanded tradable goods are worth M at the posted prices: above subsistence, C + (M − C) = M (the currency fallback adds (M − C) × 1); below, C × M / C = M. This holds up to floating-point rounding only. `demand` promises nothing about exact equality; POP trade (Step 06) handles the difference when it trades.

### Below subsistence, per good

A good is **below subsistence** when its demanded quantity is less than its subsistence × N. A POP is below subsistence if any good is. This covers both ways it can happen: income too small for the tradable bundle, and an untradable utility good the POP doesn't hold enough of. A peasant on a hex with no market has M and C both zero, keeps its labor, and is below subsistence on grain. A good with zero subsistence is never below subsistence.

## Data model

### `PriceList`

```rust
pub trait PriceList {
    fn currency(&self) -> GoodId;
    fn price(&self, good: GoodId) -> Option<Quantity>;
}
```

A hex's prices as `demand` needs them: the good prices are posted in, and the price of any good in that currency (`Quantity::ONE` for the currency itself, the posted price, or `None` if the good isn't traded here). Every price must be greater than 0; `demand` relies on this.

`Market<'_>` implements it by forwarding to its own inherent `currency` and `price` methods (see [market.md](market.md)), so `world.market(hex)` can be passed straight in. The live market of Stage 05 should implement the same trait, so `demand` and its callers don't change when the market does. Tests implement it over a `BTreeMap<GoodId, Quantity>`.

### `DemandedGood`

```rust
pub struct DemandedGood {
    pub good: GoodId,
    pub held: Quantity,         // sum of the holdings passed in; zero if none
    pub quantity: Quantity,     // what the POP wants to end up with
    pub subsistence: Quantity,  // the type's subsistence × N; zero if not in the utility
}
```

One record per involved good. `below_subsistence()` is derived, not stored: `quantity < subsistence`. A caller derives its trade plan from `held` and `quantity`: sell where `held > quantity`, buy where `quantity > held`.

### `Demand`

| Method | Result |
|---|---|
| `goods() -> &[DemandedGood]` | Every involved good once, in good ID order: every utility good, every held good, and the currency if the fallback demanded it. Zeros are included |
| `get(good) -> Option<&DemandedGood>` | The record of `good` (a binary search), or `None` if it isn't involved |
| `quantity_of(good) -> Quantity` | The demanded quantity of `good`, or `Quantity::ZERO` if it isn't involved |
| `income() -> Quantity` | M |
| `subsistence_cost() -> Quantity` | C |
| `supernumerary() -> f64` | M − C; negative below subsistence |
| `below_subsistence() -> bool` | True if any good is below subsistence |

The fields are private; a `Demand` only comes from `demand`. Both types derive `Debug`, `Clone` and `PartialEq` (`DemandedGood` is also `Copy`).

### `demand`

```rust
pub fn demand(
    pop_type: &PopType,
    size: Quantity,
    holdings: impl IntoIterator<Item = (GoodId, Quantity)>,
    prices: &impl PriceList,
) -> Demand
```

`holdings` may be in any order, and a good listed twice is summed. The function treats everything in it as the POP's endowment for this turn and makes no assumption about where it came from: `Stockpile::iter()` yields exactly this shape, so a POP's stockpile can be passed directly. Durable goods and leftover currency in the list are income like anything else. `size` may be any `Quantity`, including zero (which demands nothing); the loader guarantees a placed POP's size is positive, but `demand` doesn't rely on it.

## How it is computed

`demand` keeps a private working record per good (`held`, `subsistence`, `share`, `price`, `quantity`) in a `BTreeMap<GoodId, _>`, so the result comes out in good ID order without sorting.

1. **Collect the involved goods.** One entry per utility good, with `subsistence.saturating_mul(size)` and the type's normalized share. Each holding adds to its good's `held`, creating an entry (zero subsistence, share 0) if the good isn't in the utility. A holding of zero still makes the good involved. Then every entry's price is looked up once.
2. **Totals over tradable entries.** `income` and `subsistence_cost` are folds of `held × price` and `subsistence × price` with `saturating_mul` and `saturating_add`; `total_share` is the plain `f64` sum of the tradable entries' shares. `supernumerary = income.get() − subsistence_cost.get()`.
3. **Quantities.** Untradable entries get `held`. For tradable entries, `income.checked_sub(subsistence_cost)` chooses the branch: `Some(extra)` is the at-or-above rule (the share part is `extra × (share / total_share) / price`, added only when both `share` and `total_share` are positive), `None` is the below rule (`subsistence × income.checked_div(subsistence_cost)`).
4. **Currency fallback.** In the `Some(extra)` branch, if `total_share` is 0 and `extra` is positive, `extra` is added to the currency's entry, which is created if needed.
5. **Result.** The map becomes the `goods` list, with `income`, `subsistence_cost` and `supernumerary`.

All amounts go through `Quantity` (see [world.md](world.md#quantity)), which can't be negative, so the signed supernumerary income is computed as `f64` only for reporting. The only other `f64` arithmetic is on shares, which are proportions rather than amounts. The `unwrap_or(ZERO)` fallbacks in step 3 can't be reached given `PriceList`'s positive-price contract and shares in `[0, 1]`.

## Worked examples

From the sample setting, with N = 1, 10 `unskilled_labor` held, and a typical hex's prices (labor 1, grain 2, copper 5, in `silver`):

| POP | M | C | M − C | Demand | Trade it implies |
|---|---|---|---|---|---|
| `peasant` (labor subsistence 2 share 0.2; grain subsistence 2 share 0.8) | 10 | 6 | 4 | labor 2.8, grain 3.6 | sell 7.2 labor for 7.2 silver, buy 3.6 grain |
| `artisan` (labor subsistence 1 share 0.1; grain subsistence 2 share 0.4; copper share 0.5) | 10 | 5 | 5 | labor 1.5, copper 0.5, grain 3 | sell 8.5 labor for 8.5 silver, spend 6 on grain and 2.5 on copper |
| `peasant`, grain at 10 | 10 | 22 | −12 | labor and grain each 10/11 ≈ 0.909 | below subsistence on both |
| `peasant` on (2, −2), no market | 0 | 0 | 0 | labor 10 (kept whole), grain 0 | none; below subsistence on grain |

The artisan's C counts labor (1 × 1) and grain (2 × 2); copper has no subsistence. In the sample, the center peasant POP (size 60) holds 600 labor at turn 0 and demands 168 labor and 216 grain, with M = 600.

## Using demand

```rust
let world = /* a World */;
let market = world.market(hex);                               // a PriceList
let pop = world.pops().get(pop_id);
let pop_type = world.rules().pop_types.get(pop.pop_type);
let d = demand(pop_type, pop.size, world.pop_stockpile(pop_id).iter(), &market);
for g in d.goods() {
    // sell g.held - g.quantity if positive, buy g.quantity - g.held if positive
}
let poor = d.below_subsistence();
```

`Market` borrows the world immutably and so does `pop_stockpile`, so both can be held at once. Code inside the engine that later trades with the result (Step 06) needs a `&mut` to the POP's stockpile, so it computes the `Demand` first (it owns its data and borrows nothing) and then trades.

## Not yet

Calling `demand` during the turn, trading and consuming (Step 06); a utility index, growth rate, or any effect of being below subsistence (Step 07); rounding, a smallest unit, or exact affordability of the trade plan (Step 06); and demand rules other than the closed form, such as priorities among subsistence goods or saving.

## Tests

- `demand.rs` unit tests drive `demand` with a test `PriceList` over a `BTreeMap` and POP types built with `PopType::new`: below subsistence scaling the bundle; exactly at subsistence giving the bundle; an untradable utility good renormalizing the shares; an untradable held good kept whole; the currency fallback, and no fallback without surplus; a utility that names the currency; held currency as income; a tradable held good outside the utility all for sale; a pure necessity; empty holdings; duplicate holdings summed; zero size; and the accessors.
- `crates/engine/tests/demand.rs` uses the sample setting and `world.market(hex)` as the `PriceList`: the peasant and artisan at typical prices, demand linear in size (N 50 versus N 1), a POP on the hex with no market, a POP's stockpile passed directly (and left unchanged, with the turn still 0), and a sweep over types, sizes, holdings and hexes checking that the budget balances. Quantities are compared within `1e-9`; the budget within `1e-9 × max(1, M)`.

## docs/README.md — modify
- **Location:** the document table, new row after the `pops.md` row (added by Step 03, replaced by Step 04)
- **Change:** add

| [demand.md](demand.md) | Stone-Geary demand: the pure `demand` function, the `PriceList` trait the market implements, tradable and untradable goods, the at-or-above and below-subsistence rules, and the `Demand` result Steps 06 and 07 build on |

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** the second paragraph of the intro, starting "The dummy market is the **seam**"
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph of its `docs/market.md` draft

The dummy market is the **seam** that Stage 05 replaces with the live market. Callers ask a `Market` for prices and trade through it; when the live market arrives, what stands behind `Market` changes, not the callers. `Market` implements the `PriceList` trait (see [Market as a price list](#market-as-a-price-list)), so Stone-Geary demand (see [demand.md](demand.md)) can be computed against it. Nothing trades yet: no actor calls the exchange during a turn, and the turn report has no market activity. POP trade (Stage 02 Step 06) is the first user of the exchange. `market.rs` depends on `demand` (for the trait), `goods`, `id`, `map`, `quantity`, `stockpile` and `turn`, and knows nothing about files.

## docs/market.md — modify (Step 02's new-file draft)
- **Location:** "Data model", new subsection after "### `Market<'a>`" (after the paragraph starting "`Market::new` is crate-private")
- **Change:** add

### Market as a price list

`Market<'_>` implements `PriceList` from the `demand` module by forwarding to its inherent methods: `PriceList::currency` is `Market::currency`, and `PriceList::price` is `Market::price`, with the same three-way answer. The inherent methods stay, so existing callers are unaffected; inside the impl they are called by their qualified path (`Market::price(self, good)`) so the trait methods don't call themselves. A replacement market should implement `PriceList` the same way, keeping every price greater than 0.

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Step 04)
- **Location:** the opening, the paragraph starting "The names follow one rule"
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same paragraph

The names follow one rule: "POP" means the placed group, so `PopId` is a placed POP's ID and `PopTypeId` is a POP type's. The `demand` module (see [demand.md](demand.md)) computes what a POP wants to end up with from its type's subsistence amounts and shares, but nothing calls it during the turn yet. Nothing consumes, trades or grows yet: Step 06 sells what a POP doesn't keep through the dummy market (see [market.md](market.md)), and Step 07 changes a POP's size.

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", the first paragraph, its first sentence ("A POP is N identical individuals of one type on one hex. Its endowment and (from Step 05) its demand are per-individual amounts scaled by N.")
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same sentences

A POP is N identical individuals of one type on one hex. Its endowment and its demand (see [demand.md](demand.md)) are per-individual amounts scaled by N.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Using POP types", after the code block
- **Change:** add

To compute what a POP of a type wants from its holdings and a hex's prices, pass the type, a size, the holdings and a `PriceList` to `demand` (see [demand.md](demand.md)).

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same paragraph

Calling demand during the turn, trading and consuming (Step 06), growth and its parameters, and what happens to a POP that shrinks toward zero (Step 07), and showing POP types and POPs in the dev UI (Step 08). Starting stockpiles or cash for POPs, changing a POP's hex or type during play, migration, and per-POP parameters that differ from the type are not planned for this Stage.

## docs/world.md — modify
- **Location:** "The turn", item 2 of the numbered list (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same item

2. **Actors act:** nothing yet. POP demand can already be computed (see [demand.md](demand.md)), but nothing calls it during a step. POP trade (Stage 02 Step 06) will compute each POP's demand here and trade through the dummy market (see [market.md](market.md)); later Stages add industries and the live market. Nothing calls the exchange during a step yet.

## docs/world.md — modify
- **Location:** "Quantity", the whole section body
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same section

`Quantity`'s arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result of two finite values never becomes infinite; `checked_sub`, which returns `None` if the result would be negative and never returns `-0.0`; and `checked_div`, which returns `None` when dividing by zero and otherwise clamps at `f64::MAX` (zero divided by a positive value is zero). `Quantity::ONE` is one unit (the price of a hex's currency in its own market). Arithmetic is added only when a system needs it (see [resources.md](resources.md)); `checked_div` was added for Stone-Geary demand (see [demand.md](demand.md)).

## docs/world.md — modify
- **Location:** "Tests", the unit-tests bullet (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub`, `saturating_mul` and `checked_div`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, zero, and `remove_all`), and `world.rs` (a world forced to `Turn::MAX`, for both `turns_valid` and `pops_placed`, refuses to step and leaves its turn, hex stockpiles and POP stockpiles unchanged). Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)), and demand in `crates/engine/tests/demand.rs` (see [demand.md](demand.md)).

## docs/world.md — modify
- **Location:** "Not yet", the whole paragraph (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same paragraph

Ownership of deposits, starting stockpiles in `start/` (for hexes or POPs), deposit stock, depletion and regeneration, any effect of `movable`, per-hex and per-POP report breakdowns, saving and loading a world, prices that change during play, changes to POPs during play (size from Step 07), and any behavior in the "actors act" part of the turn (demand is computable but not called; trading arrives in Step 06).

## docs/resources.md — modify
- **Location:** "Types", the `Quantity` bullet under `quantity.rs:` (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same bullet

- `Quantity` wraps an `f64` and is always finite and never negative. `Quantity::new(f64) -> Option<Quantity>` rejects negative, NaN, and infinite values and turns `-0.0` into `0.0`; `get()` returns the `f64`; `ZERO` and `Default` are zero, and `ONE` is one; `Display` prints like `f64` (`4`, `2.5`). It derives `PartialEq` and `PartialOrd` but not `Eq`, `Ord`, or `Hash`. Its arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result stays finite, `checked_sub`, which returns `None` instead of a negative result, and `checked_div`, which returns `None` when dividing by zero and clamps at `f64::MAX`. Arithmetic is added only when a system needs it. Every system should use this type for amounts of goods, so the representation can be swapped later (for example to fixed decimal for cross-platform determinism) in one place.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/currency.rs, ...`
- **Supersedes:** Step 04 (pops), the same block

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
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, demand.rs, goods.rs, map.rs, market.rs, pops.rs, resources.rs, setting_loader.rs, world.rs
```
