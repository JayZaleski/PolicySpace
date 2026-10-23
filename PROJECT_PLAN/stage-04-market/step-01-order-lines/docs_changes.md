# Step 01: order-lines — Docs Changes

## docs/orders.md — new file
- **Location:** whole file
- **Change:** add
- **Supersedes:** none

# Order lines and the market's arithmetic

Source: `crates/engine/src/orders.rs`, with unit tests at the bottom of the file, and `crates/engine/tests/orders.rs`. The public items are re-exported from the engine crate root: `Side`, `LineParams`, `LineError`, `LineErrors`, `Line`, `Curve`, `Crossing`, `cross`.

This module holds the **order line**, the one form an actor's market orders take, and the arithmetic the live market runs on lines: evaluating a line at a price, summing the lines on one side of a good into one **curve**, and finding the price where the summed demand and supply curves **cross**. It is a pure-function module in the style of `production.rs` and `demand.rs`: it depends only on `quantity` and `error_list`, and knows nothing about the `World`, stockpiles, goods, hexes, the `Market` or files. Prices and quantities are `Quantity` values (finite, never negative, see [resources.md](resources.md)); the slope is a signed `f64`.

**Nothing calls it yet.** The turn still trades at the dummy market's posted prices (see [market.md](market.md) and [trade.md](trade.md)). Later work carries lines across the decider protocol, draws them for POPs, settles each good's trades on the crossing, and publishes the summed curves in the views.

## Lines

### `Side`

```rust
pub enum Side { Demand, Supply }
```

Which side of a good's market an order is on. `Display` writes `demand` or `supply`; the panic and error messages use it.

### `LineParams` and `Line::new`

```rust
pub struct LineParams {
    pub anchor_price: f64, // the price the line was drawn at: finite, > 0
    pub quantity: f64,     // the quantity at the anchor price: finite, >= 0
    pub slope: f64,        // quantity change per unit of price: finite; <= 0 for demand, >= 0 for supply
    pub maximum: f64,      // the most the actor trades at any price: finite, >= 0
    pub give_up: f64,      // the last price the actor still trades at: finite, >= 0
}

impl Line {
    pub fn new(side: Side, params: &LineParams) -> Result<Line, LineErrors>;
}
```

`LineParams` is the raw form, as a sender gives it. `Line::new` checks every field and builds a `Line`, or refuses with a `LineErrors` (an `ErrorList<LineError>`, never empty, see [setting-loader.md](setting-loader.md)) listing **every** problem, in field order. A malformed line is refused, never repaired, in the spirit of decision validation (see [decider.md](decider.md)), so the protocol can report a line's problems beside its other checks. Inside, a `Line` holds `Quantity` values and a checked slope, so every later function trusts its input.

| Variant | When | Message |
|---|---|---|
| `AnchorPrice { got }` | not finite, or not above 0 | `anchor price must be a finite number greater than 0 (got 0)` |
| `Quantity { got }` | not finite, or negative | `quantity must be a finite number of at least 0 (got -2)` |
| `Slope { got }` | not finite | `slope must be a finite number (got NaN)` |
| `SlopeWrongWay { side, got }` | demand slope above 0, or supply slope below 0 (checked only when the slope is finite) | `slope of a demand line must be at most 0 (got 1.5)` / `slope of a supply line must be at least 0 (got -1.5)` |
| `Maximum { got }` | not finite, or negative | `maximum must be a finite number of at least 0 (got inf)` |
| `GiveUp { got }` | not finite, or negative | `give-up price must be a finite number of at least 0 (got -1)` |
| `QuantityAboveMaximum { quantity, maximum }` | the quantity is above the maximum (checked only when both are valid numbers) | `quantity at the anchor price (5) is above the maximum (4)` |

Numbers are written with `f64`'s `Display`, like the decider's messages. `LineError` implements `std::error::Error`; `LineErrors` displays one problem per line. A slope of zero is allowed on both sides, and a line that trades nothing (a maximum of zero, or a quantity of zero with a slope of zero) is well-formed, not an error.

The accessors `side()`, `anchor_price()`, `quantity()`, `slope()`, `maximum()` and `give_up()` return what was given, and `params()` gives the numbers back as a `LineParams`, for the protocol and the UI. The line keeps an explicit anchor price rather than meaning "the current price", so a line kept on file while the price moves still means what it meant when it was drawn.

### The line's shape: `Line::at`

For a line with anchor price p₀, quantity q₀, slope s, maximum m and give-up price g, the raw quantity is r(p) = q₀ + s × (p − p₀) and the clipped quantity is c(p) = min(max(r(p), 0), m). Then:

- **demand:** `at(p)` = c(p) for p ≤ g, and 0 for p > g;
- **supply:** `at(p)` = c(p) for p ≥ g, and 0 for p < g.

The **give-up price is inclusive**: at exactly g the actor trades what the line says. A give-up of 0 means a seller sells at any price, and a buyer buys only at price 0. A demand line whose give-up is below its anchor price is zero at the anchor.

Because of the clip and the give-up price, a line can be a **step**. "Sell the whole holding H at or above break-even B" is the supply line (anchor any positive price, quantity H, slope 0, maximum H, give-up B): zero below B, H from B up. No separate order kind is needed.

Worked example: the demand line (p₀ 2, q₀ 10, slope −2, maximum 12, give-up 6) gives 12 at prices 0 and 1 (clipped to the maximum), 10 at 2, 2 at 6 (the give-up price itself), and 0 above 6.

Privately, a line also knows its **left and right limits** at a price (`left`, `right`): they equal `at` everywhere except at the give-up price, where a demand line's right limit is 0 (it has just ended) and a supply line's left limit is 0 (it hasn't started). And it knows its **corners**, the prices above 0 where its quantity can change direction while it is live: the give-up price g, the price where r crosses 0 (p₀ − q₀/s) and the price where r reaches m (p₀ + (m − q₀)/s), the last two only when s ≠ 0. Corners outside the live region (above g for demand, below g for supply) lie strictly inside the line's zero region and are left out; corners at or below 0 are left out because 0 is always a breakpoint.

## Curves

### `Curve`

```rust
impl Curve {
    pub fn empty(side: Side) -> Curve;
    pub fn sum<'a>(side: Side, lines: impl IntoIterator<Item = &'a Line>) -> Curve;
    pub fn side(&self) -> Side;
    pub fn points(&self) -> &[(Quantity, Quantity)];
    pub fn is_empty(&self) -> bool;
    pub fn at(&self, price: Quantity) -> Quantity;
    pub fn before(&self, price: Quantity) -> Quantity;
    pub fn after(&self, price: Quantity) -> Quantity;
}
```

A curve is the lines on one side of a good, summed: at every price its quantity is the sum of the lines' quantities. It is stored as its side and a **list of breakpoints** `(price, quantity)` in rising price order:

- the quantity is linear between neighbouring points, constant before the first and after the last;
- a **jump** is two points at one price: the quantity before the jump, then after;
- there are never two consecutive points with the same price and quantity, and never more than two points at one price.

The breakpoint list is the curve's **only** form. There is no sampled or simplified version: whatever an actor or the UI wants (the slope near the current price, the area between the curves) is computed from the list, so there is nothing to keep in sync.

`Curve::empty(side)`, and `Curve::sum` over no lines, give the curve with no points: zero everywhere, and `is_empty()` is true. A sum over one or more lines that all trade nothing is a single `(0, 0)` point, not empty. `Curve::sum` panics with `a supply line can't be summed into a demand curve` (or the mirror) if a line is on the other side; that is a programming error, since callers sort lines by side first.

### How summing works

`Curve::sum` builds the list exactly, from the lines' corners, not by sampling:

1. Collect the candidate prices: 0, and every corner of every line. Sort and dedupe.
2. At each candidate c, compute the sum of the lines' left limits (`before`), values (`at`) and right limits (`after`) at c. Each line's limits come from its shape, never from nudging the price. At c = 0 there is no left side, so `before` is `at`.
3. If `before` ≠ `after`, push `(c, before)` then `(c, after)`; otherwise push `(c, at)`. Skip a point that would repeat the previous one.

Between consecutive candidates every line is linear, so the sum is linear and the points reproduce it exactly; after the last candidate every line is constant (zero or at its maximum). A summed demand curve never rises with the price and a summed supply curve never falls. The list can contain a point where nothing visibly changes (the give-up price of a line already clipped to zero, for example); that is harmless.

### Evaluating a curve and the side's convention

`at`, `before` and `after` find the points at a price with a binary search (`partition_point`) and interpolate linearly between neighbours. At a price with no points they all agree. At a jump price `before` is the first point's quantity and `after` the second's, and **`at` takes the side's inclusive value**:

- a demand curve's `at` is `before` (buyers still buy at their give-up price), so a demand curve is left-continuous;
- a supply curve's `at` is `after` (sellers already sell at theirs), so a supply curve is right-continuous.

This matches `Line::at` at every give-up price, and it makes excess demand (demand minus supply) non-increasing everywhere, which the crossing relies on.

Worked example: the demand line above alone sums to the points (0, 12), (1, 12), (6, 2), (6, 0). A supply step of 8 at give-up 3 (slope 0, maximum 8) sums to (0, 0), (3, 0), (3, 8): `before(3)` is 0, `at(3)` and `after(3)` are 8.

## The crossing

### `Crossing` and `cross`

```rust
pub struct Crossing {
    pub price: Quantity,  // the price set: the crossing price, or the current price when nothing trades
    pub traded: Quantity, // how much changes hands at that price
    pub demand: Quantity, // what the demand curve wants at `price`
    pub supply: Quantity, // what the supply curve wants at `price`
}

impl Crossing {
    pub fn trades(&self) -> bool;             // traded > 0
    pub fn demand_share(&self) -> f64;        // traded / demand, or 1 when demand is 0
    pub fn supply_share(&self) -> f64;        // traded / supply, or 1 when supply is 0
    pub fn unfilled_demand(&self) -> Quantity; // demand - traded
    pub fn unfilled_supply(&self) -> Quantity; // supply - traded
}

pub fn cross(demand: &Curve, supply: &Curve, current: Quantity) -> Crossing;
```

`cross` takes a good's summed demand curve, its summed supply curve and its current price, and returns what the market sets. It reads nothing else: it works on the summed curves, never on individual lines, so the rule is the same whatever the lines came from. It panics if the first curve isn't a demand curve or the second isn't a supply curve. The shares and unfilled quantities are derived; `traded` is always the smaller of `demand` and `supply`, so at most one share is below 1, and the unfilled quantities are never negative.

**How a caller uses it.** Every actor trades along its own line at `price`: a buyer trades `demand_share()` of its line's quantity there, a seller `supply_share()`. That is the project's usual one-proportion trim of a request that is too big. Trimming to what an actor actually holds, the cash cap on purchases, and what a crossing at price zero or a no-trade result means for the world are the caller's business, not this module's.

### The cases

Excess demand E(p) = demand.at(p) − supply.at(p) never rises with the price, so the prices where it is positive come first, then the prices where it is zero (possibly none), then the prices where it is negative. The result follows from where E changes sign:

| Case | Price | Quantities |
|---|---|---|
| **Clean crossing**: E crosses zero at one price, inside a linear piece | that price | `traded` = `demand` = `supply`: both shares exactly 1 |
| **Crossing at a jump**: E changes sign at a price where a curve jumps (a step, or a line reaching its give-up price) | the jump price | the short side's full quantity is traded; the long side's share is below 1 |
| **Range of equal prices**: E is zero and both quantities positive across a range | the price in the range closest to `current` | strictly inside the range the two are equal. At an edge, a jump can leave one side wanting more (a buyer whose give-up is the range's top still buys there); that side's share is below 1, the jump rule applied at the edge |
| **Nothing would trade**: one or both sides empty, or no price at which both quantities are positive | `current`, unchanged | `traded` 0; `demand` and `supply` are what each curve wants at `current` (at most one positive), so the unfilled quantities carry the signal |
| **Crossing at zero**: sellers supply more than buyers want even at price 0 | 0 | as for a jump or clean crossing; `cross` sets no price floor |

A range that reaches to infinity (both curves flat and equal at every price above some point) keeps `current` if it lies in the range. The no-trade rule covers the one-sided case and the never-overlapping case alike on purpose: a price move with no trade behind it would be a signal the orders don't support.

Worked examples:

- Demand (p₀ 4, q 10, slope −1, maximum 100, give-up 100) sums to (0, 14), (14, 0), (100, 0); supply (p₀ 4, q 2, slope 1, maximum 100, give-up 0) sums to (0, 0), (2, 0), (102, 100). They cross cleanly at price 8 with 6 traded, whatever the current price.
- A demand step of 10 up to give-up 5 against a sloped supply that offers 4 at 5: price 5, `traded` 4, `demand` 10, `supply` 4, demand share 0.4, unfilled demand 6.
- Flat demand of 10 up to 8 and flat supply of 10 from 5: the range is [5, 8], so current 3 gives price 5, current 6 gives 6, current 9 gives 8, with 10 traded each time.
- Demand that gives up at 5 and supply that starts at 8, current 6: price 6, nothing traded, both quantities 0. With current 3, `demand` is what buyers want at 3 and `supply` 0.

### How the crossing is computed

`cross` merges the breakpoint prices of both curves (with 0), sorts and dedupes them, and records E's left limit, value and right limit at each, from the curves' `before`, `at` and `after`. Between two neighbouring prices E is linear from the right limit at the lower to the left limit at the upper; after the last it is constant. One left-to-right scan finds:

- **p_c**, the supremum of the prices where E > 0 (0 if there are none), and
- **p_n**, the infimum of the prices where E < 0 (none if there are none).

A linear piece that starts positive and ends negative holds a root, found by linear interpolation and clamped into the piece. Then:

- If p_n is none or p_c < p_n, there is a **gap** where E = 0: the price is `current` clamped into [p_c, p_n] (or [p_c, ∞)), the wanted quantities are both curves' `at` there, and `traded` is the smaller.
- Otherwise there is **a single sign change** at p_c = p_n. At a breakpoint price the wanted quantities are both curves' `at`, and they may differ (a jump). At a root strictly inside a linear piece the curves are equal by construction, but interpolating them separately can leave them a floating-point step apart, which would surface downstream as an unfilled sliver. So at an interior root `cross` reports one value, the smaller of the two, for `demand`, `supply` and `traded`, and both shares are exactly 1.
- If `traded` comes out 0 in either case, the result is the no-trade result at `current`.
- Demand positive above supply after the last breakpoint can't happen (every demand line is zero above its give-up price, which is a breakpoint, so demand is zero after the last one); if it ever did, `cross` returns the no-trade result rather than inventing a price.

There is **no floating-point tolerance**: comparisons are exact on `Quantity` values, and the curves are exact sums of merged breakpoints. Iteration is always in sorted price order, so the result is deterministic.

**Known edge: a root next to a breakpoint.** A root whose true position lies within half a floating-point step of a breakpoint can round onto the breakpoint itself. It is then treated as a breakpoint crossing, not an interior root, so `demand` and `supply` are each curve's own `at` and can differ by one floating-point step: a speck-sized unfilled quantity on one side and a share a hair below 1. This is accepted as very rare. If settlement ever shows such specks in practice, the fix belongs here: treat a root that rounds onto a breakpoint the same way as an interior root.

## Using lines and curves

```rust
use policyspace_engine::{cross, Curve, Line, LineParams, Side};

let buyer = Line::new(Side::Demand, &LineParams {
    anchor_price: 4.0, quantity: 10.0, slope: -1.0, maximum: 100.0, give_up: 100.0,
})?;                                         // Err(LineErrors) lists every problem
let seller = Line::new(Side::Supply, &LineParams {
    anchor_price: 4.0, quantity: 2.0, slope: 1.0, maximum: 100.0, give_up: 0.0,
})?;
let demand = Curve::sum(Side::Demand, [&buyer]);
let supply = Curve::sum(Side::Supply, [&seller]);
let crossing = cross(&demand, &supply, current_price);
if crossing.trades() {
    let bought = buyer.at(crossing.price).get() * crossing.demand_share();
    let sold = seller.at(crossing.price).get() * crossing.supply_share();
}
```

To extend the line shape (another corner, say), change `Line::at` and its `left`/`right` limits together and add the new corner to `push_corners`, so the summed curve stays exact; the curve and the crossing need no change as long as each line stays piecewise linear, demand never rising and supply never falling. Taxes or subsidies would fit as a wedge between the two summed curves before `cross`; that isn't built.

## Not yet

Lines in the decider protocol and the Python client, POP lines, settlement on the crossing, a price floor, trimming to holdings and the cash cap, curves in the views, orders kept on file between turns, and taxes or subsidies as a wedge between the curves.

## Tests

- `orders.rs` unit tests (using `q` and `close` from `crates/engine/src/test_support.rs`, with local builders `lp`, `dl`, `sl`, `dcurve`, `scurve`, `pts`, a price `sweep` and the checks `assert_sums` and `assert_well_formed`): lines following their slope and clipped on both sides; the inclusive give-up price; step lines; a give-up of 0; every refusal message, and several problems reported in field order; lines that trade nothing accepted; `params()` and the accessors. Curves: the empty sum; one line's corners; a supply step's jump; a demand give-up at 0 jumping at 0; coinciding give-ups making one jump and different ones two; mixed supply and demand lines summing exactly over a sweep of prices; monotonicity; well-formed points; lines that trade nothing summing to one `(0, 0)` point; the wrong-side panic. Crossings: a clean crossing independent of the current price; demand and supply steps cutting the long side; steps at the same price; a line reaching its maximum making a corner, not a jump; a range choosing the price closest to the current one; awkward numbers leaving no sliver (exact equality, shares exactly 1); range edges with a demand or a supply jump; a range open at infinity; sides that never overlap; one empty side and both empty; touching at zero quantity; a crossing at zero; a sloped crossing between corners; the swapped-sides panic; the `Crossing` methods and `Side`'s `Display`.
- `crates/engine/tests/orders.rs` works through the public re-exports only (`mod common;` for `q` and `close`): two demand lines (one a step) and a supply line summed and crossed at price 8 with 6 traded and a sorted breakpoint list that includes the step's price; a malformed line's `LineErrors` displaying one problem per line; and two empty curves trading nothing at the current price.

## docs/README.md — modify
- **Location:** the documents table, new row after the `market.md` row
- **Change:** add
- **Supersedes:** none

| [orders.md](orders.md) | Order lines and the live market's arithmetic, not yet wired into the turn: `Line` and its checked construction from `LineParams` (every `LineError` at once), the line's shape (clip to the maximum, inclusive give-up price, steps), `Curve` as an exact breakpoint list summed from lines with its side's convention at a jump, and `cross`: the clean, jump, range, no-trade and zero-price cases, the `Crossing` record with its shares and unfilled quantities, and how it is computed without a tolerance |

## docs/workspace.md — modify
- **Location:** "Layout", the `crates/engine/` source list, after the `src/market.rs` line
- **Change:** add
- **Supersedes:** none

```
    src/orders.rs       order lines, summed curves and the crossing (see orders.md)
```

## docs/workspace.md — modify
- **Location:** "Layout", the `tests/*.rs` list under `crates/engine/`
- **Change:** replace
- **Supersedes:** none

```
    tests/business_trade.rs, businesses.rs, currency.rs, decider.rs, decider_host.rs,
          demand.rs, external_decider.rs, factories.rs, goods.rs, growth.rs, map.rs,
          market.rs, orders.rs, pops.rs, production.rs, recordings.rs, resources.rs,
          setting_loader.rs, technologies.rs, trade.rs, world.rs
```

## docs/market.md — modify
- **Location:** "Not yet", appended to the end of the paragraph
- **Change:** add
- **Supersedes:** none

The live market's arithmetic already exists, unwired: order lines, the summed curves and the crossing are in the `orders` module (see [orders.md](orders.md)). Nothing in the turn calls it yet, and the dummy market described here is still what every trade goes through.
