# Stone-Geary demand

Source: `crates/engine/src/demand.rs`. The public items are re-exported from the engine crate root: `PriceList`, `DemandedGood`, `Demand`, and the function `demand`.

`demand` works out what a POP wants to **end up with** this turn: the quantity of each good after it has sold what it doesn't keep and bought what it wants. Its inputs are the POP type's Stone-Geary utility (see [pops.md](pops.md)), the POP's size N, what the POP holds, and a hex's prices. It is a **pure function**: it reads its inputs, returns a `Demand`, and changes no stockpile, trades nothing and touches no world state. It doesn't round anything.

The trade round (see [trade.md](trade.md)) calls it for every POP in the "actors act" part of every turn: the market computes each POP's demand from its whole stockpile, sells the surplus and buys the shortfall, and the POP consumes its utility goods up to the demanded quantities. The World keeps each POP's `Demand` in its turn outcome (`World::pop_outcome(pop).demand()`). POP growth builds the POP's subsistence index from the income, subsistence cost and per-good demanded and subsistence quantities it reports (`growth::subsistence_index`, see [growth.md](growth.md)), so how far a POP is above or below subsistence decides how fast it grows or shrinks.

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

In every case the demanded tradable goods are worth M at the posted prices: above subsistence, C + (M − C) = M (the currency fallback adds (M − C) × 1); below, C × M / C = M. This holds up to floating-point rounding only. `demand` promises nothing about exact equality; the trade round caps each purchase at the currency held, so a rounding shortfall trims the last purchase instead of being refused, and a rounding surplus stays as currency (see [trade.md](trade.md)).

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

`Market<'_>` implements it by forwarding to its own inherent `currency` and `price` methods (see [market.md](market.md)), so `world.market(hex)` can be passed straight in. The live market of Stage 05 should implement the same trait, so `demand` and its callers don't change when the market does. The engine's unit tests implement it over a `BTreeMap<GoodId, Quantity>` (`Prices` in `crates/engine/src/test_support.rs`, see [workspace.md](workspace.md)).

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

The artisan's C counts labor (1 × 1) and grain (2 × 2); copper has no subsistence. In the sample, the center peasant POP (size 60 in the setting) holds 600 labor at turn 0 and demands 168 labor and 216 grain, with M = 600.

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

`Market` borrows the world immutably and so does `pop_stockpile`, so both can be held at once. After turn 0 a POP's stockpile holds only what the trade round left and its size has grown, so for the demand the round actually used, read `world.pop_outcome(pop_id).demand()`. The round itself computes every `Demand` first (it owns its data and borrows nothing) and then trades with a `&mut` to the POP's stockpile (see [trade.md](trade.md)).

## Not yet

Rounding or a smallest unit, and demand rules other than the closed form, such as priorities among subsistence goods or saving.

## Tests

- `demand.rs` unit tests drive `demand` with the shared `Prices` price list and the POP type builders (`peasant`, `type_of`), goods constants (`LABOR`, `GRAIN`, `COPPER`, `SILVER`) and `held` from `crates/engine/src/test_support.rs`: below subsistence scaling the bundle; exactly at subsistence giving the bundle; an untradable utility good renormalizing the shares; an untradable held good kept whole; the currency fallback, and no fallback without surplus; a utility that names the currency; held currency as income; a tradable held good outside the utility all for sale; a pure necessity; empty holdings; duplicate holdings summed; zero size; and the accessors. The `assert_quantities` and `assert_balances` helpers stay local to the module.
- `crates/engine/tests/demand.rs` uses the sample setting and `world.market(hex)` as the `PriceList`: the peasant and artisan at typical prices, demand linear in size (N 50 versus N 1), a POP on the hex with no market, the demand recorded in a POP's turn-0 outcome equal to the pure function called on its endowment with the outcome's `starting_size()` (the center peasant, size 60, whose current size after turn 0's growth is 61.6: 600 labor held, 168 labor and 216 grain demanded), and a sweep over types, sizes, holdings and hexes checking that the budget balances. Quantities are compared within `1e-9`; the budget within `1e-9 × max(1, M)`.
