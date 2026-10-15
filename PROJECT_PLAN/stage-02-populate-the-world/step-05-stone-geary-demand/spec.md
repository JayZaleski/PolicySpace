# Step 05: stone-geary-demand — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-05-stone-geary-demand/design.md` — the behavior this spec implements, including the worked examples and the Acceptance Criteria.
- `PROJECT_PLAN/stage-02-populate-the-world/step-02-dummy-market/docs_changes.md` — the dummy market as built: `Market`, `Market::price` (the three-way answer), `Market::currency`.
- `PROJECT_PLAN/stage-02-populate-the-world/step-03-pop-types/docs_changes.md` — POP types as built: `PopType`, `UtilityGood` (subsistence per individual, normalized shares), `PopType::utility`.
- `PROJECT_PLAN/stage-02-populate-the-world/step-04-pops/docs_changes.md` — POPs and their stockpiles as built: `Pop::size`, `World::pop_stockpile`, `Stockpile::iter`.
- `docs/world.md` (the `Quantity` and `Tests` sections), `docs/workspace.md` (lints, the check command, test layout).
- Code: `crates/engine/src/quantity.rs`, `crates/engine/src/market.rs` (`Market`, `Market::price`, `Market::currency`), `crates/engine/src/pops.rs` (`PopType`, `UtilityGood`, `PopType::new` for unit tests), `crates/engine/src/stockpile.rs` (`Stockpile::iter`), `crates/engine/src/world.rs` (`World::market`, `World::pops`, `World::pop_stockpile`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/market.rs` and `crates/engine/tests/pops.rs` (helper style for integration tests), `settings/sample/rules/pops.toml`, `settings/sample/start/markets.toml`, `settings/sample/start/pops.toml`.

## Approach

- **A new module `crates/engine/src/demand.rs`** holds the pure function `demand` and its result type `Demand`. Rationale: the Stone-Geary math is a standalone piece of economics that Steps 06 and 07 call; `pops.rs` is the data model and stays as it is. Nothing in `world.rs` or the turn changes.
- **Prices come through a two-method trait `PriceList`** (the currency good, and the price of a good: posted price, `Quantity::ONE` for the currency, or None if not traded), defined in `demand.rs` and implemented by `Market<'_>` in `market.rs`. Rationale: the demand math stays independent of the dummy market type, unit tests drive it with hand-written prices, and Stage 05's live market implements the same trait.
- **Holdings come in as any iterator of `(GoodId, Quantity)` pairs.** `Stockpile::iter()` yields exactly that; tests pass vectors. Duplicate goods are summed. Rationale: the function makes no assumption about where the holdings came from (design.md).
- **Subsistence is scaled by N up front** (`subsistence × size` per utility good) and the formulas use the scaled amounts. Rationale: the reported per-good subsistence is then the number Steps 06 and 07 compare against.
- **`Quantity` gains `checked_div`.** All arithmetic goes through `Quantity` (`saturating_mul`, `saturating_add`, `checked_sub`, `checked_div`); a share is multiplied in as `Quantity::new(share)`. Supernumerary income is the one signed number and is reported as `f64`. Rationale: keep the one-numeric-representation rule, and `Quantity` can't be negative.
- **The result is one record per involved good in good ID order** (good, held, demanded quantity, subsistence × N), plus income, subsistence cost and supernumerary income. `below_subsistence` is derived (quantity < subsistence), not stored. Rationale: Step 06 derives its trade plan from held vs quantity; Step 07 reads the totals and the per-good status.
- **The build runs as one.** It is one small module, one method on `Quantity`, one trait impl, and tests.

## Changes

### New

- `crates/engine/src/demand.rs` — `PriceList`, `DemandedGood`, `Demand`, `demand`, unit tests.
- `crates/engine/tests/demand.rs` — integration tests against the sample setting (Test Plan).

### Modified

- `crates/engine/src/quantity.rs` — add `checked_div`; update the module doc's list of operations; unit tests.
- `crates/engine/src/market.rs` — `impl PriceList for Market<'_>`; add one sentence to the module doc (a `Market` is a `PriceList`, so the `demand` module can compute a POP's demand against it).
- `crates/engine/src/lib.rs` — `pub mod demand;` (alphabetical, after `currency`) and `pub use demand::{Demand, DemandedGood, PriceList, demand};`. Crate docs: after the POP sentences, add that the [`demand`] module computes a POP's Stone-Geary demand from its type's utility, its holdings and a hex's prices, as a pure function nothing calls during the turn yet.
- `crates/engine/src/pops.rs` — module doc: replace "Step 05 computes demand from the utility." with "The [`demand`](crate::demand) module computes demand from the utility."

Nothing else changes: no setting files, no test settings, no dev UI, no `world.rs`.

## Interfaces & Data

### `crates/engine/src/quantity.rs`

```rust
/// `self / other`, or None if `other` is zero. Clamped at `f64::MAX`.
pub fn checked_div(self, other: Quantity) -> Option<Quantity>
```

Both values are finite and non-negative, so the only hazards are division by zero (None) and overflow to infinity (clamp with `.min(f64::MAX)`). Zero divided by a positive value is zero. Update the module doc: "so far, `saturating_add`, `checked_sub`, `saturating_mul` and `checked_div`."

### `crates/engine/src/demand.rs`

Module doc: the closed-form Stone-Geary demand as a pure function; tradable vs untradable goods; the at-or-above and below-subsistence rules; the budget balances up to rounding; nothing calls it during the turn yet (Step 06 does); knows nothing about the World or files. Summarize design.md's Behavior section in prose; don't copy the examples.

```rust
use std::collections::BTreeMap;

use crate::goods::GoodId;
use crate::pops::PopType;
use crate::quantity::Quantity;

/// A hex's prices as the demand function needs them: which good is the currency, and the
/// price of any good in that currency. The dummy market implements this; the live market
/// will too.
pub trait PriceList {
    /// The good prices are posted in.
    fn currency(&self) -> GoodId;
    /// The price of `good` in the currency: [`Quantity::ONE`] for the currency itself,
    /// the posted price, or None if the good isn't traded here. Every price is > 0.
    fn price(&self, good: GoodId) -> Option<Quantity>;
}

/// One good in a demand: how much the POP holds, how much it wants to end up with, and
/// how much of it the POP's subsistence bundle needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemandedGood {
    /// The good.
    pub good: GoodId,
    /// How much the POP holds (the sum of the holdings passed in); zero if it holds none.
    pub held: Quantity,
    /// How much the POP wants to end up with.
    pub quantity: Quantity,
    /// The type's subsistence amount times N; zero if the good isn't in the utility.
    pub subsistence: Quantity,
}

impl DemandedGood {
    /// True if the demanded quantity doesn't reach the subsistence amount. Never true for
    /// a good with zero subsistence.
    pub fn below_subsistence(&self) -> bool  // self.quantity < self.subsistence
}

/// What a POP wants to end up with, and the numbers behind it. See [`demand`].
#[derive(Debug, Clone, PartialEq)]
pub struct Demand {
    /// In good ID order, each good once: every utility good, every held good, and the
    /// currency if supernumerary income was demanded as currency.
    goods: Vec<DemandedGood>,
    /// M: the value of every tradable held good at its price.
    income: Quantity,
    /// C: the value of the tradable part of the subsistence bundle (times N).
    subsistence_cost: Quantity,
    /// M − C. Finite; negative below subsistence.
    supernumerary: f64,
}

impl Demand {
    /// Every involved good, in good ID order.
    pub fn goods(&self) -> &[DemandedGood]
    /// The record of `good`, or None if it isn't involved.
    pub fn get(&self, good: GoodId) -> Option<&DemandedGood>
    /// The demanded quantity of `good`; zero if it isn't involved.
    pub fn quantity_of(&self, good: GoodId) -> Quantity
    /// Income M.
    pub fn income(&self) -> Quantity
    /// The subsistence cost C.
    pub fn subsistence_cost(&self) -> Quantity
    /// Supernumerary income M − C; negative below subsistence.
    pub fn supernumerary(&self) -> f64
    /// True if any good is below subsistence.
    pub fn below_subsistence(&self) -> bool
}

/// The Stone-Geary demand of a POP of `pop_type` with `size` individuals that holds
/// `holdings` (goods and amounts, in any order; a good listed twice is summed) on a hex
/// with `prices`. Reads its inputs and changes nothing.
pub fn demand(
    pop_type: &PopType,
    size: Quantity,
    holdings: impl IntoIterator<Item = (GoodId, Quantity)>,
    prices: &impl PriceList,
) -> Demand
```

`get` uses `binary_search_by_key` on `good` (the list is sorted); `quantity_of` maps it to the quantity or `Quantity::ZERO`.

### The algorithm (`demand`)

Use a private working record per good, kept in a `BTreeMap<GoodId, _>` so the result is in good ID order:

```rust
struct Entry { held: Quantity, subsistence: Quantity, share: f64, price: Option<Quantity>, quantity: Quantity }
```

1. **Collect the involved goods.** For each `UtilityGood` of `pop_type.utility()`: insert an entry with `subsistence = u.subsistence.saturating_mul(size)`, `share = u.share`, `held = ZERO`. For each `(good, amount)` in `holdings`: `held = held.saturating_add(amount)` on the existing entry, or insert one with `subsistence = ZERO`, `share = 0.0`. A holding with amount zero still makes the good involved (its demand is zero). Then set each entry's `price = prices.price(good)`; an entry with `Some` price is *tradable*.
2. **Totals over tradable entries.** `income = Σ held × price` and `subsistence_cost = Σ subsistence × price`, each a fold with `saturating_mul` and `saturating_add` starting at `ZERO`; `total_share = Σ share` (plain `f64`; shares are in `[0, 1]`). Untradable entries contribute nothing to any of these. `supernumerary = income.get() - subsistence_cost.get()`.
3. **Quantities.** For every untradable entry, `quantity = held` (kept whole, whether or not it is in the utility). For the tradable entries, branch on `income.checked_sub(subsistence_cost)`:
   - `Some(extra)` (M ≥ C): `quantity = subsistence`, plus, if `share > 0.0` and `total_share > 0.0`, `extra.saturating_mul(Quantity::new(share / total_share).unwrap()).checked_div(price).unwrap()` (the price is > 0; `unwrap` is justified by the trait's contract, or use `unwrap_or(ZERO)` with a comment). A tradable held good not in the utility has `subsistence = ZERO` and `share = 0.0`, so it gets zero. A share-0 utility good gets exactly its subsistence.
   - `None` (M < C): `fraction = income.checked_div(subsistence_cost)` (C > M ≥ 0, so C > 0 and this is `Some`); `quantity = subsistence.saturating_mul(fraction)`. Goods with zero subsistence get zero.
4. **Currency fallback.** In the `Some(extra)` branch, if `total_share == 0.0` and `extra > Quantity::ZERO`: the currency's entry gets `quantity = quantity.saturating_add(extra)`; if the currency has no entry (not in the utility and not held), insert one with `held = ZERO`, `subsistence = ZERO`, `quantity = extra`. When `extra` is zero nothing is added and no entry is created (a POP on a hex with no market reports only the goods it holds or wants).
5. **Build the result** from the map in key order: `DemandedGood { good, held, quantity, subsistence }`.

Notes:
- `size` is any `Quantity`; the loader guarantees a POP's size is > 0, but the function doesn't rely on it.
- Shares are normalized once over the whole utility by `PopType::new`; `share / total_share` renormalizes them over the tradable utility goods. When every utility good is tradable, `total_share` is 1 up to rounding.
- The budget balances by construction: Σ tradable `quantity × price` = C + (M − C) = M above subsistence (the fallback adds `extra × 1`), and = C × M / C = M below. The function does no rounding and guarantees equality only up to floating-point error.
- Everything is `f64` arithmetic through `Quantity`; no `f64` comparisons other than `share > 0.0` and `total_share > 0.0` / `== 0.0`.

### `crates/engine/src/market.rs`

```rust
impl PriceList for Market<'_> {
    fn currency(&self) -> GoodId { self.currency() }      // the inherent method
    fn price(&self, good: GoodId) -> Option<Quantity> { Market::price(self, good) }
}
```

Keep the inherent `currency` and `price` methods as they are (other code and tests call them). Inside the impl, call the inherent methods by their qualified path (`Market::price(self, good)`) to avoid recursion.

## Tasks

1. `quantity.rs`: add `checked_div` with doc comment; update the module doc; unit tests: `checked_div_divides_normal_values` (`6 / 4 = 1.5`, `0 / 4 = 0`, `x / ONE = x`), `checked_div_by_zero_is_none`, `checked_div_clamps_at_the_largest_finite_value` (`f64::MAX / 0.5`).
2. `demand.rs`: the module doc, `PriceList`, `DemandedGood`, `Demand` and its accessors, and `demand` as specified. Register the module and the re-exports in `lib.rs`; update the crate docs and the `pops.rs` module doc.
3. `market.rs`: `impl PriceList for Market<'_>` and the module-doc sentence.
4. Unit tests in `demand.rs` (Test Plan, unit section): a test `PriceList` over a `BTreeMap<GoodId, Quantity>` plus a currency `GoodId`; POP types via `PopType::new`; goods via `GoodId::from_index`.
5. `crates/engine/tests/demand.rs`: the integration tests (Test Plan, integration section).
6. Run `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests) and fix everything it reports. Every public item needs a doc comment.

## Test Plan

Tolerance: compare quantities with `(a - b).abs() < 1e-9` (as `tests/pops.rs`'s `close`). "Budget balances" means Σ over goods with a price of `quantity × price` is within `1e-9 × max(1, M)` of `income`. Where a test lists a result, check the full `goods()` list (goods, in ID order, and the quantity of each) unless it says otherwise.

### Unit tests (`demand.rs`, hand-built prices)

Use goods `labor` = 0, `grain` = 1, `copper` = 2, `silver` = 3 (the currency); a `peasant` type like the sample's (`labor` subsistence 2 weight 0.2, `grain` subsistence 2 weight 0.8) and typical prices (labor 1, grain 2, copper 5) unless stated.

| Test | Criterion | Setup → expectation |
|---|---|---|
| `below_subsistence_scales_the_bundle` | below subsistence | grain at 10; peasant N 1 holds labor 10 → M 10, C 22, supernumerary −12; labor and grain each 10/11 ≈ 0.909; both `below_subsistence()`; `Demand::below_subsistence()` true; budget balances |
| `at_exactly_subsistence_gives_the_bundle` | continuity at M = C | grain at 4; peasant holds labor 10 → M 10, C 10, supernumerary 0; labor 2, grain 2; nothing below subsistence |
| `an_untradable_utility_good_renormalizes_the_shares` | untradable, unheld | no grain price; peasant holds labor 10 → M 10, C 2; labor 10 (share renormalized to 1), grain 0 and below subsistence; budget balances |
| `an_untradable_held_good_is_kept_whole` | untradable, held | no labor price; type with utility labor subsistence 2 weight 0.2 and grain subsistence 2 weight 0.8; holds labor 10 and silver 6 → M 6, C 4 (grain only); labor 10 (not below), grain 2 + 0.8/0.8 × 2 / 2 = 3, silver 0; budget balances |
| `no_positive_tradable_share_demands_currency` | currency fallback | type: labor subsistence 2 weight 0, grain weight 1; no grain price; holds labor 10 → M 10, C 2, total share 0; labor 2, grain 0 (not below: its subsistence is 0), silver 8; silver appears in `goods()` though not held or wanted; budget balances |
| `the_fallback_adds_nothing_when_there_is_no_surplus` | currency fallback | same type, holds labor 2 → labor 2, grain 0; no silver entry |
| `a_utility_that_names_the_currency_prices_it_at_one` | currency in utility | type: silver subsistence 1 weight 0.5, labor weight 0.5; labor at 2; holds labor 10 → M 20, C 1, supernumerary 19; silver 10.5, labor 4.75; budget balances |
| `held_currency_is_income` | currency held | peasant holds labor 10 and silver 5 → M 15; labor 3.8, grain 5.6, silver 0 |
| `a_tradable_held_good_outside_the_utility_is_all_for_sale` | not in utility | peasant holds labor 10 and copper 1 → M 15; copper quantity 0, held 1 |
| `a_pure_necessity_gets_exactly_its_subsistence` | share 0 | type: labor subsistence 3 weight 0, grain weight 1; holds labor 10 → labor exactly 3, grain 3.5 |
| `holding_nothing_demands_nothing_tradable` | empty holdings | peasant, empty holdings → M 0, C 6, supernumerary −6; labor 0, grain 0, both below |
| `duplicate_holdings_are_summed` | input shape | peasant holds `[(labor, 4), (labor, 6)]` → same as holding labor 10; `held` is 10 |
| `a_zero_size_demands_nothing` | edge | peasant N 0 holds nothing → every quantity 0, C 0, nothing below |
| `accessors_find_goods_or_give_zero` | API | `get`, `quantity_of` on an involved and an uninvolved good |

### Integration tests (`crates/engine/tests/demand.rs`, the sample setting)

Load the sample with `common::load(&sample_setting())`, build a `World`, and use `world.market(hex)` as the `PriceList`, `world.rules().pop_types.get(id)` for the types (`id("peasant")`, `id("artisan")`), goods by name. Hex (0, 0) has the typical prices (plus unimproved goods); (2, −2) has no market.

| Test | Criterion | Expectation |
|---|---|---|
| `a_peasant_at_typical_prices` | AC 1 | N 1, holds unskilled_labor 10 → labor 2.8, grain 3.6; income 10, cost 6, supernumerary 4; nothing below; budget balances |
| `an_artisan_at_typical_prices` | AC 1 | N 1, holds unskilled_labor 10 → labor 1, grain 2, copper 0; supernumerary 0; nothing below |
| `demand_is_linear_in_size` | AC 2 | N 50 holding 500 labor gives 50 × the N 1 quantities, income, and cost, for both types |
| `a_pop_on_a_hex_with_no_market_keeps_what_it_holds` | untradable | hex (2, −2), peasant N 1 holding labor 10 → `goods()` is exactly labor 10 (not below) and grain 0 (below); M 0, C 0, supernumerary 0; `below_subsistence()` true |
| `a_pops_stockpile_is_a_valid_holdings_list` | purity, AC last | the peasant POP at (0, 0) (size 60) after `World::new`: `demand(type, pop.size, world.pop_stockpile(id).iter(), &market)` gives labor 168, grain 216, income 600; the stockpile is unchanged afterwards (compare clones) and `world.turn()` is still 0 |
| `every_case_balances_the_budget` | AC 3 | a helper applied in each test above; at least one explicit assertion per test with a market |

Run: `cargo test -p policyspace-engine` for this crate, `.venv/Scripts/python tools/check.py` for the gate.

## Constraints

- Follow the crate's conventions: doc comments on every public item (clippy runs with `-D warnings` and `missing_docs = "warn"`), `BTreeMap` for anything iterated, no `f64` arithmetic outside `Quantity` except `share` sums and ratios.
- Keep `demand.rs` free of `World`, `Stockpile`, `Pops` and file handling. It imports only `goods`, `pops` (for `PopType` and `UtilityGood`) and `quantity`.
- Don't change `world.rs`, the turn, `TurnReport`, any setting file, any test setting, the checker, or the dev UI. Every existing test passes unchanged.
- Don't add rounding, a smallest unit, or any guarantee of exact budget equality (Step 06). No utility index or growth rate (Step 07).
- Don't commit.

## Done When

All tasks complete, `tools/check.py` passes (fmt, clippy, every test in the workspace), every test in the Test Plan passes, and every Acceptance Criterion in design.md is met.
