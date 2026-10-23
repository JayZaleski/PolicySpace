# Step 01: order-lines — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-04-market/step-01-order-lines/design.md` — the behavior this spec implements. Every rule there is required.
- `docs/workspace.md` — crate layout, lints (`missing_docs`, clippy `-D warnings`), unit-test helpers in `crates/engine/src/test_support.rs`, integration-test helpers in `crates/engine/tests/common/mod.rs`, and the check command.
- `crates/engine/src/quantity.rs` — `Quantity`: finite, non-negative `f64`, with `new`, `get`, `ZERO`, `ONE`, `saturating_add`, `checked_sub`, `saturating_mul`, `checked_div`. It has `PartialOrd` but not `Ord`.
- `crates/engine/src/error_list.rs` — `ErrorList<E>`, the never-empty "every problem found" list; `ErrorList::new` is crate-private and `lines()` exists for tests.
- `crates/engine/src/decider/decision.rs` lines 55–135 — the style of validation messages (`amount must be a finite number of at least 0 (got -2)`), and `decider::is_amount` / `decider::is_fraction` in `crates/engine/src/decider/mod.rs`.
- `crates/engine/src/production.rs` and `crates/engine/src/demand.rs` — the shape of a pure-function module: a module doc comment stating the rules, public record types with documented fields, pure functions, unit tests at the bottom using `test_support`.
- `crates/engine/src/lib.rs` — module list and re-exports.
- `docs/market.md` — the dummy market this Step does not touch; read "The exchange" and "Not yet" to see the seam.

## Approach
- **One new module, `crates/engine/src/orders.rs`**, holding the order line, the summed curve and the crossing as data types and pure functions. Nothing else in the engine changes except `lib.rs` (module declaration and re-exports). Nothing is wired into the turn, the loader or the dev UI.
- **Lines are built from raw `f64`s and report every problem at once** (`Line::new(side, &LineParams) -> Result<Line, LineErrors>`), in the decider's validation style, so Step 02 can report a malformed line's problems beside its other checks. Inside, a `Line` holds `Quantity`s and a checked slope, so every later function trusts its inputs.
- **A curve is its side plus a sorted breakpoint list** of `(price, quantity)` `Quantity` pairs. A jump is two points at one price. Evaluation at a jump price is inclusive by side: a demand curve takes the value before the jump (buyers still buy at the give-up price), a supply curve the value after it (sellers already sell at theirs). So a demand curve is left-continuous and a supply curve right-continuous, and excess demand is non-increasing everywhere.
- **The crossing works on two curves and the current price only**, and returns one `Crossing` record: the price, the traded quantity, and what each side wanted at that price. Shares and unfilled quantities are derived. The no-trade case returns the current price with zero traded and each side's quantity at that price as "wanted".
- **No tolerance.** Comparisons are exact `f64` comparisons on `Quantity` values; the curves are built from exact sums and merged breakpoints, not sampled. Tests compare with `close` where arithmetic can round.
- **Build as one.** One module and one small test file; nothing to split.

## Changes
- `crates/engine/src/orders.rs` — new. Module doc comment stating the rules (line shape, inclusive give-up, curve representation and the inclusive-by-side evaluation, the four crossing cases). Contains `Side`, `LineParams`, `LineError`, `LineErrors`, `Line`, `Curve`, `Crossing`, `cross`, and unit tests.
- `crates/engine/src/lib.rs` — add `pub mod orders;` (alphabetically, after `market`) and `pub use orders::{Crossing, Curve, Line, LineError, LineErrors, LineParams, Side, cross};`. Add one sentence to the crate doc comment: the `orders` module holds the order line and the live market's arithmetic, not yet wired into the turn.
- `crates/engine/tests/orders.rs` — new integration test through the public interface only (see Test Plan).
- No change to `docs/` (docs are updated at the end of the Stage), the setting format, the sample setting, the dev UI, or the Python package.

## Interfaces & Data
All public items need doc comments (the lint is `-D warnings`).

```rust
/// Which side of a good's market an order is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Demand, Supply }
// Display: "demand" / "supply".

/// The numbers a line is built from, as the sender gives them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineParams {
    pub anchor_price: f64,  // the price the line was drawn at; must be finite and > 0
    pub quantity: f64,      // the quantity at the anchor price; finite, >= 0
    pub slope: f64,         // quantity change per unit of price; finite; <= 0 for demand, >= 0 for supply
    pub maximum: f64,       // the most the actor trades at any price; finite, >= 0
    pub give_up: f64,       // the last price the actor still trades at; finite, >= 0
}

/// One problem with a line's numbers.
#[derive(Debug, Clone, PartialEq)]
pub enum LineError {
    AnchorPrice { got: f64 },        // "anchor price must be a finite number greater than 0 (got 0)"
    Quantity { got: f64 },           // "quantity must be a finite number of at least 0 (got -2)"
    Slope { got: f64 },              // "slope must be a finite number (got NaN)"
    SlopeWrongWay { side: Side, got: f64 },
        // demand: "slope of a demand line must be at most 0 (got 1.5)"
        // supply: "slope of a supply line must be at least 0 (got -1.5)"
    Maximum { got: f64 },            // "maximum must be a finite number of at least 0 (got inf)"
    GiveUp { got: f64 },             // "give-up price must be a finite number of at least 0 (got -1)"
    QuantityAboveMaximum { quantity: f64, maximum: f64 },
        // "quantity at the anchor price (5) is above the maximum (4)"
}
// Display as above (numbers with f64's Display, like the decider's messages); implements std::error::Error.

/// Every problem with one line, in field order. Never empty.
pub type LineErrors = ErrorList<LineError>;

/// A well-formed order line: a straight line through the anchor, clipped to 0 and the
/// maximum, and zero beyond the give-up price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line { /* private: side, anchor_price, quantity, slope: f64, maximum, give_up (Quantity except slope) */ }

impl Line {
    /// Checks every field and builds the line, or reports every problem in field order:
    /// anchor price, quantity, slope (not finite, then wrong way), maximum, give-up price,
    /// then quantity above maximum (only when both are valid numbers).
    pub fn new(side: Side, params: &LineParams) -> Result<Line, LineErrors>;
    pub fn side(&self) -> Side;
    pub fn anchor_price(&self) -> Quantity;
    pub fn quantity(&self) -> Quantity;   // at the anchor price, as given (already <= maximum)
    pub fn slope(&self) -> f64;
    pub fn maximum(&self) -> Quantity;
    pub fn give_up(&self) -> Quantity;
    pub fn params(&self) -> LineParams;   // the numbers back, for the protocol and the UI
    /// The quantity at `price`: 0 beyond the give-up price (above it for demand, below it for
    /// supply); otherwise quantity + slope × (price − anchor_price), clipped to [0, maximum].
    pub fn at(&self, price: Quantity) -> Quantity;
}

/// The lines on one side of a good, summed: piecewise linear, as a breakpoint list.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve { /* private: side, points: Vec<(Quantity, Quantity)> */ }

impl Curve {
    /// A curve of zero everywhere (no lines).
    pub fn empty(side: Side) -> Curve;
    /// The sum of `lines`, all of which must be on `side` (panics otherwise: that is a
    /// programming error, with the message `a <side> line can't be summed into a <side> curve`).
    pub fn sum<'a>(side: Side, lines: impl IntoIterator<Item = &'a Line>) -> Curve;
    pub fn side(&self) -> Side;
    /// The breakpoints in rising price order: (price, quantity). Linear between neighbours,
    /// constant before the first and after the last, a jump as two points at one price (the
    /// quantity before the jump, then after). Empty for the empty curve. Never two consecutive
    /// points with the same price and quantity, and never more than two at one price.
    pub fn points(&self) -> &[(Quantity, Quantity)];
    pub fn is_empty(&self) -> bool;       // no points: zero everywhere
    /// The quantity at `price`, with the side's convention at a jump: the value before the
    /// jump for demand, after it for supply.
    pub fn at(&self, price: Quantity) -> Quantity;
    /// The quantity just below `price` (the left limit); equals `at` except at a supply jump.
    pub fn before(&self, price: Quantity) -> Quantity;
    /// The quantity just above `price` (the right limit); equals `at` except at a demand jump.
    pub fn after(&self, price: Quantity) -> Quantity;
}

/// What the market sets for one good from its two summed curves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// The price set: the crossing price, or the current price when nothing trades.
    pub price: Quantity,
    /// How much changes hands at that price (zero when nothing trades).
    pub traded: Quantity,
    /// What the demand curve wants at `price` (its unfilled part is `demand - traded`).
    pub demand: Quantity,
    /// What the supply curve wants at `price`.
    pub supply: Quantity,
}

impl Crossing {
    pub fn trades(&self) -> bool;                  // traded > 0
    /// The share of its line every buyer trades: traded / demand, or 1 when demand is 0.
    pub fn demand_share(&self) -> f64;
    pub fn supply_share(&self) -> f64;             // likewise for sellers
    pub fn unfilled_demand(&self) -> Quantity;     // demand - traded
    pub fn unfilled_supply(&self) -> Quantity;     // supply - traded
}

/// Finds the price where `demand` and `supply` cross, given the good's `current` price.
/// Panics if `demand` is not a demand curve or `supply` not a supply curve.
pub fn cross(demand: &Curve, supply: &Curve, current: Quantity) -> Crossing;
```

### The line's shape
For a line with anchor price p₀, quantity q₀, slope s, maximum m and give-up price g, define the raw quantity r(p) = q₀ + s × (p − p₀) and the clipped quantity c(p) = min(max(r(p), 0), m). Then:
- demand: `at(p)` = c(p) for p ≤ g, 0 for p > g;
- supply: `at(p)` = c(p) for p ≥ g, 0 for p < g.

Compute `at` in `f64` and convert with `Quantity::new`; the clip guarantees the result is in [0, m], so `new` can't fail (use `unwrap_or(Quantity::ZERO)` defensively, never a panic path). Clip in `f64` before converting.

**Corner prices.** A line's quantity is linear between its corners. The corners are:
- the give-up price g;
- where r crosses 0: p₀ − q₀/s, when s ≠ 0 and this is > 0;
- where r crosses m: p₀ + (m − q₀)/s, when s ≠ 0 and this is > 0.
`sum` uses only the corners on the side of g where the line is live (at or below g for demand, at or above g for supply): a dead corner lies strictly inside the line's zero region, and keeping it would add a redundant point to the published breakpoint list. Corners at or below 0 are dropped: price 0 is always a breakpoint of a non-empty curve (see below).

### Summing
`Curve::sum` builds the breakpoint list from the lines' corners, exactly:
1. Collect the candidate prices: 0, and every corner of every line that is > 0. Sort (via `partial_cmp`; every price is finite) and dedupe.
2. For each candidate price c, in order, compute `before` = Σ line.at(just below c), `at` = Σ line.at(c) and `after` = Σ line.at(just above c). "Just below" and "just above" are not computed by nudging c: compute each line's left and right limits exactly from its shape (for a demand line, the right limit at its give-up price is 0 and the left limit is c(g); for a supply line the left limit at g is 0 and the right limit is c(g); everywhere else the limits equal `at`). At c = 0 there is no left side; use `at`.
3. Push points: if before ≠ after, push (c, before) then (c, after); otherwise push (c, at) only. The side's inclusive convention makes `at` equal to `before` for demand and `after` for supply at every jump, so a two-point jump always records the inclusive value in the correct position.
4. Drop a point that would repeat the previous point's price and quantity. A curve with no lines, or whose lines are all zero everywhere, is still represented by its points (a single (0, 0) point when there is at least one line, since 0 is always a candidate); `is_empty` is true only for a curve with no points, which only `Curve::empty` and a sum over no lines produce. (`sum` over an empty iterator returns `Curve::empty(side)`.)

Between consecutive candidate prices every line is linear, so the sum is linear there and the points reproduce the sum exactly. After the last point every line is constant (at 0 or its maximum), so the curve is constant.

### Evaluating a curve
`at(p)`, `before(p)`, `after(p)` on the points list:
- No points: 0.
- p below the first point's price: the first point's quantity (this can only happen for the empty case, since 0 is always first, but handle it).
- p at or above the last point's price: the last point's quantity.
- p strictly between two consecutive points with different prices: linear interpolation.
- p equal to a breakpoint price with one point there: that quantity.
- p equal to a jump price (two points): `before` is the first point's quantity, `after` the second's, `at` is `before` for demand and `after` for supply.
Use a binary search over prices (`partition_point`). Interpolation in `f64`, converted with `Quantity::new` (the value lies between two non-negative quantities, so it is valid; clamp defensively).

### The crossing
Let E(p) = demand.at(p) − supply.at(p) (as `f64`). With the side conventions E is non-increasing in p. Let P = {p : E(p) > 0}, Z = {p : E(p) = 0}, N = {p : E(p) < 0}; they come in that order along the price axis, each possibly empty. Define:
- p_c = the largest price in P's closure (the supremum of P), or 0 if P is empty;
- p_n = the smallest price in N's closure (the infimum of N), or "none" if N is empty.

`cross` computes these by scanning the merged breakpoint prices of both curves (0 and every breakpoint price of either curve, sorted, deduped). On each candidate c it knows E's left limit, value and right limit (from `before`, `at`, `after` of both curves); on each open interval between candidates E is linear from the right limit at the lower candidate to the left limit at the upper one, and E is constant after the last candidate. Within one linear piece the price where E = 0 is found by linear interpolation. Then:

1. **A gap where E = 0** (p_n is "none", or p_c < p_n): the price is `current` clamped into [p_c, p_n] (into [p_c, ∞) when p_n is none). Wanted quantities are `demand.at(price)` and `supply.at(price)` at the chosen price, and `traded` is the smaller of the two. Strictly inside the gap the two are equal. At an edge they can differ: a buyer whose give-up price is exactly p_c still buys there (the give-up is inclusive), so demand can exceed supply at p_c, and a seller whose give-up price is exactly p_n sells there, so supply can exceed demand at p_n. Because demand curves are left-continuous and supply curves right-continuous, the smaller of the two at an edge is always the gap's common quantity, and the long side's extra want shows up as a share below 1, which is the jump rule applied at the edge. If `traded` is 0, this is the no-trade case: return `Crossing { price: current, traded: ZERO, demand: demand.at(current), supply: supply.at(current) }`.
2. **A single sign change** (p_c == p_n == p\*): `price` = p\*. When p\* is a breakpoint price, `demand` = demand.at(p\*), `supply` = supply.at(p\*) and `traded` = min of the two; the result is a jump when they differ (the long side's share below 1). When p\* is a root found *inside* a linear piece, the two curves are equal there by construction, but interpolating them separately can leave them one floating-point step apart, which would show up downstream as an unfilled sliver (the kind of speck bug 01 is about). So at an interior root report one value for all three: `traded` = `demand` = `supply` = the smaller of the two interpolated values, so both shares are exactly 1 and nothing is unfilled. If `traded` is 0 it is again the no-trade case (return as above with `current`).
3. A crossing at p\* = 0 is reported as is (price 0). `cross` never floors the price.

Both curves empty: E ≡ 0, Z is everything, the gap rule gives the no-trade result at `current`. One curve empty: E is one-signed-or-zero, and the gap rule gives no trade at `current` with that side's `at(current)` as its wanted quantity.

Implementation notes: scan candidates left to right keeping the last price where E was > 0 (p_c) and stop at the first price where E < 0 (p_n), checking at each candidate the left limit, the value and the right limit in that order, and checking each open interval between candidates for a root (E positive at its start and negative at its end) or for E ≡ 0 across it. If E is positive across the whole open interval and then ≤ 0 at the upper candidate's left limit... that can't happen (limits match the interior), so the interior root test covers it. When the root of a linear piece is found, p_c = p_n = root. Keep `price` as a `Quantity` built from the root; it lies within the piece, so it is finite and ≥ 0.

Shares: `demand_share` = 1.0 when `demand` is zero, else `traded.get() / demand.get()`; likewise supply. `unfilled_*` use `checked_sub` with `unwrap_or(ZERO)` (never negative by construction).

## Tasks
1. Create `crates/engine/src/orders.rs` with the module doc comment, `Side` (with `Display`), `LineParams`, `LineError` (with `Display` and `Error`), `LineErrors`, and `Line` with `new` and the accessors. `new` reports every problem in field order.
2. Add `Line::at` and the private corner/limit helpers (left limit, right limit, corner prices).
3. Add `Curve`: `empty`, `sum`, `side`, `points`, `is_empty`, `at`, `before`, `after`, following "Summing" and "Evaluating a curve".
4. Add `Crossing` with its derived methods, and `cross`, following "The crossing".
5. Declare and re-export the module in `lib.rs`; add the one-sentence crate doc note.
6. Write the unit tests in `orders.rs` (Test Plan).
7. Write `crates/engine/tests/orders.rs` (Test Plan).
8. Run `.venv/Scripts/python tools/check.py` and fix formatting, lints and failures until it passes.

## Test Plan
Run everything with `.venv/Scripts/python tools/check.py` (fmt, clippy `-D warnings`, `cargo test --workspace`, pytest). During development, `cargo test -p policyspace-engine orders` runs just these tests.

Unit tests in `orders.rs` (use `q` and `close` from `test_support`; add local helpers `demand(params)`/`supply(params)` that build a `Line` or panic, and `pts(curve) -> Vec<(f64, f64)>`):

**Lines (AC 1 and 2):**
- `at` on a demand line (p₀ 2, q₀ 10, slope −2, max 12, g 6): 12 at 0 and at 1 (clipped to the maximum), 10 at 2, 2 at 6, 0 at 6.5 and at 7 (beyond the give-up price); a supply line (p₀ 2, q₀ 10, slope 4, max 20, g 1): 0 at 0.5, 6 at 1 (inclusive give-up), 10 at 2, 20 at 4.5 and 20 at 100 (clipped), 0 below 1.
- The give-up price itself trades the line's quantity, on both sides; a demand line with g below p₀ is zero at p₀.
- A step: supply with slope 0, q₀ = max = 8, g 3: 0 at 2.999, 8 at 3 and at 1000.
- A line with g 0: supply sells at any price (positive at 0); demand with g 0 is zero at every price above 0 and `at(0)` is its clipped quantity.
- `new` refuses, with the exact messages above: anchor price 0 and −1 and NaN; quantity −2; slope NaN and inf; demand slope 1.5 and supply slope −1.5; maximum inf; give-up −1; quantity 5 above maximum 4. One test passes a params with several problems and checks the messages come out in field order and all of them appear. A line with maximum 0, and one with quantity 0 and slope 0, are accepted and give 0 everywhere.
- `params()` round-trips the numbers; the accessors return what was given.

**Curves (AC 3):**
- Sum of no lines is `empty`, with no points and 0 everywhere; `Curve::empty` likewise.
- One demand line's curve: points are (0, 12), (1, 12), (6, 2), (6, 0) for the first line above; `at` reproduces `Line::at` at every point price, between them, and beyond the last.
- One supply step's curve: (0, 0), (3, 0), (3, 8); `before(3)` 0, `at(3)` 8, `after(3)` 8.
- Two demand lines whose corners coincide (same give-up price) produce one jump of the summed size, and two with different give-ups produce two jumps; in both, `at(p)` equals the sum of `Line::at` at a sweep of prices (every breakpoint, midpoints between them, 0 and a large price), compared with `close`.
- Mixed lines (sloped, clipped at max, step) on the supply side: the same sweep check; and monotonicity: on a sweep of rising prices a demand curve's `at` never rises and a supply curve's never falls.
- No two consecutive points share both price and quantity; never more than two points at one price.
- `sum` with a line of the wrong side panics with the stated message (`#[should_panic(expected = ...)]`).

**Crossing (AC 4):**
- Clean crossing: demand (p₀ 4, q 10, slope −1, max 100, g 100) and supply (p₀ 4, q 2, slope 1, max 100, g 0) cross at price 8, traded 6, both wanted 6, both shares 1, `trades()` true. The current price is irrelevant: the same with current 1 and 100.
- Clean crossing with awkward numbers (slopes and anchors that don't divide evenly, such as demand through (1, 7) with slope −1/3 and supply through (1, 1) with slope 2/7): `demand == supply == traded` exactly (`assert_eq`, not `close`), both shares exactly 1.0, and both `unfilled_*` exactly zero.
- Range edge with a demand jump at p_c: demand lines (1, 10, 0, 10, 8) and (1, 3, 0, 3, 5) summed, supply (1, 10, 0, 10, 5), current 3: price 5, traded 10, demand 13, supply 10, demand share 10/13, unfilled demand 3. Mirror: supply lines (1, 10, 0, 10, 5) and (1, 3, 0, 3, 8) summed, demand (1, 10, 0, 10, 8), current 9: price 8, traded 10, supply 13, supply share 10/13.
- Jump, demand step: demand (q 10, slope 0, max 10, g 5) against a sloped supply through (5, 4): price 5, traded 4, demand 10, supply 4, demand share 0.4, supply share 1, unfilled demand 6.
- Jump, supply step: supply (q 10, slope 0, max 10, g 5) against a sloped demand with 4 at 5: price 5, traded 4, supply share 0.4.
- Both steps at the same price with different sizes: price 5, traded the smaller, the larger side's share.
- A line reaching its maximum makes a corner, not a jump: a clean crossing on the flat part gives shares of 1.
- Range: flat demand 10 (slope 0, max 10, g 8) and flat supply 10 (slope 0, max 10, g 5): with current 3 the price is 5, with 6 it is 6, with 9 it is 8; traded 10 in every case.
- Range open at infinity: both curves flat 10 and equal everywhere (demand g very large, supply g 0): price = current, traded 10.
- No overlap: demand gives up at 5, supply starts at 8, current 3: price 3, traded 0, demand = demand.at(3) > 0, supply 0; current 6: price 6, both 0; current 9: price 9, demand 0, supply > 0.
- One side empty: demand lines only, current 4: price 4, traded 0, demand = demand.at(4), supply 0; supply only likewise. Both empty: price = current, everything 0.
- Touching at zero quantity: demand gives up at 5 (positive there) and a supply line whose quantity is 0 at its give-up price 5 and rises after: no trade at current.
- Crossing at zero: supply positive at price 0 (g 0, q at 0 larger than demand at 0) against demand positive at 0: price 0, traded = demand.at(0), supply share < 1.
- `cross` with the curves' sides swapped panics.
- `Crossing` derived methods: shares 1 when wanted is 0; `unfilled_*` never negative.

Integration test `crates/engine/tests/orders.rs` (public interface only, `mod common;` for `q`/`close`): build the sample-like worked example from the clean-crossing case through `policyspace_engine::{Line, LineParams, Side, Curve, cross}`, sum two demand lines and one supply line, check the breakpoint list is sorted and the crossing's price and traded quantity, and check that a malformed line's `LineErrors` display one problem per line. This confirms the re-exports and that everything needed is reachable from outside the crate.

Mapping: AC 1 → the `at` and give-up tests; AC 2 → the `new` refusal and acceptance tests; AC 3 → the curve tests; AC 4 → the crossing tests; AC 5 → the step, maximum, coinciding-corner and same-price-steps tests above.

## Constraints
- Follow the pure-function module style of `production.rs` and `demand.rs`: the module knows nothing about `World`, `Stockpile`, `Market`, goods or files. It depends only on `quantity` and `error_list`.
- Prices and quantities are `Quantity`; the slope is a signed `f64`. No new dependencies.
- No floating-point tolerance in the module; use exact arithmetic on the merged breakpoints. Tests use `close` where sums can round.
- Deterministic: every iteration is in sorted price order; no hash maps.
- Don't touch `market.rs`, `trade.rs`, `world.rs`, the decider module, the setting loader, the dev UI, the Python package, or any recording. Don't change `docs/` or the manual.
- Out of scope: wiring into the turn, the protocol, POP lines, trimming to holdings, the cash cap, a price floor, publishing curves in views.
- Every public item has a doc comment; code used only by tests is `#[cfg(test)]`.

## Done When
All tasks complete, `.venv/Scripts/python tools/check.py` passes (formatting, clippy with `-D warnings`, every Rust test including the new ones, the Python tests), and every Acceptance Criterion in `design.md` is met by a test in the Test Plan.
