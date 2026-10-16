# POP growth

Source: `crates/engine/src/growth.rs` (the math), `GrowthParams` in `crates/engine/src/pops.rs` (the parameters), the trade round in `crates/engine/src/trade.rs` (where the index and rate are computed), and `World::grow` in `crates/engine/src/world.rs` (where they are applied). The public items `subsistence_index`, `growth_rate`, `grown_size` and `GrowthParams` are re-exported from the engine crate root.

Every turn, after it has traded and consumed, a POP's size N grows or shrinks by a **rate** that depends on how far above or below subsistence it is. The measure is the **subsistence index** u, a dimensionless number built from the numbers its demand already reports (see [demand.md](demand.md)): 0 at exactly subsistence, positive above, negative (down to −1) below. The rate is a function of u with parameters on the POP type: a polynomial above subsistence, a linear decline below, both clamped. N becomes N × (1 + rate), and the next turn's endowment and demand use the new N.

`growth.rs` is a module of pure functions. It depends only on `demand` (for `Demand`), `pops` (for `GrowthParams`) and `quantity`, and knows nothing about the `World`, stockpiles, markets or files, so the formulas can be tested on their own and revisited without touching the turn.

## The subsistence index

`subsistence_index(demand: &Demand) -> f64` reads the `Demand` the market computed for the POP this turn: income M, subsistence cost C, and per good the demanded quantity and the subsistence need (the type's subsistence amount times N).

- **Above subsistence** (`demand.below_subsistence()` is false): u = (M − C) / C, the number of extra subsistence bundles the POP's income could buy. If C is 0 there is no prosperity to measure and u is 0, so a type whose subsistence goods cost nothing on its hex (or that has none) neither grows nor shrinks. The ratio is clamped to `[0, f64::MAX]`: the lower bound only guards rounding (not below means M ≥ C), and the upper bound keeps u finite when C is tiny and M huge, because an infinite u would make the polynomial evaluate `0 × ∞ = NaN`.
- **Below subsistence** (at least one utility good is below its need): for each good with a positive need, its **attainment** is the demanded quantity divided by the need; u is the smallest attainment minus 1, so u lies in [−1, 0). For a POP that is simply poor, demand scales every tradable good by M/C, so u = M/C − 1. For a POP that can't get a subsistence good at all (unpriced and not held), that good's attainment is 0 and u is −1, however much it holds of anything else.

The two sides meet at 0: a POP whose income exactly covers its subsistence bundle has u = 0 from either formula. Every input scales with N, so u is per individual and doesn't depend on the POP's size.

In this Stage the demanded and the consumed quantities are the same, so the index measures what the POP actually consumed. Stage 05, where rationing can leave a POP short of its demand, revisits which numbers the index reads; that change belongs inside `subsistence_index`.

## The growth rate

```rust
pub struct GrowthParams {
    coefficients: Vec<f64>,   // c₁, c₂, …; never empty, every one finite
    decline: f64,             // finite, in [0, 1)
    max_growth: f64,          // finite, at least 0
}
```

`GrowthParams` lives on the `PopType` (`PopType::growth() -> &GrowthParams`, see [pops.md](pops.md)) and has the read-only accessors `coefficients() -> &[f64]`, `decline() -> f64` and `max_growth() -> f64`. It is `Debug`, `Clone` and `PartialEq`. `GrowthParams::new(coefficients, decline, max_growth)` is crate-private: only the loader (and unit tests) build parameters. It asserts what the loader guarantees and panics with `a POP type's growth coefficients must be a non-empty list of finite numbers`, `a POP type's decline must be a finite number of at least 0 and less than 1`, or `a POP type's max_growth must be a finite number of at least 0`.

`growth_rate(params: &GrowthParams, index: f64) -> f64`:

- for `index >= 0`, the polynomial c₁u + c₂u² + …, evaluated by Horner's method. It has no constant term, so the rate is exactly 0 at u = 0;
- for `index < 0`, `decline × index`, so a POP with nothing (u = −1) shrinks by exactly `decline`, and a POP at half its subsistence by half that. Keeping this side linear means the polynomial never has to behave for negative u, where a general polynomial could flip sign;
- either way, clamped to `[−decline, max_growth]`. Since `−decline ≤ 0 ≤ max_growth`, the clamp can't panic. A polynomial that goes negative for rich POPs can never shrink one faster than `decline`, and the cap holds whatever the coefficients are. The index is always finite, so the polynomial can overflow to ±infinity but never yields NaN, and the clamp turns it back into a finite rate.

Because `decline < 1`, the rate is always above −1, so N stays positive: a starving POP shrinks geometrically but never reaches zero, and the engine never removes a POP. A type with `growth = [0]` and `decline = 0` never changes size, which several test settings use.

`grown_size(size: Quantity, rate: f64) -> Quantity` returns `size × (1 + rate)`, saturating at the largest finite quantity. Nothing is rounded: N stays continuous.

## Where it runs in the turn

Growth is the last thing in part 2 of the turn ("actors act", see [world.md](world.md#the-turn)):

1. **In the trade round.** `hex_round`'s fourth pass (consumption, see [trade.md](trade.md)) computes, for each POP, `subsistence_index(&demand)` and `growth_rate(pop_type.growth(), index)`, and stores them in the POP's `PopOutcome` together with `starting_size` (the size the demand and the trade were for). So an outcome is complete when it is created.
2. **After every hex has traded.** The private `World::grow` walks `pop_outcomes` in POP ID order, sets each POP's size to `grown_size(size, outcome.growth_rate())` with the crate-private `Pops::set_size`, and sums the new sizes into `TurnReport::population`. It reads `pop_outcomes` and writes `pops`, which are separate fields, so the borrows don't conflict.

`World::new` and `World::step` both run the turn through the private `World::run_turn`, which calls `emit`, `endow`, `trade` and `grow` in that order, so a fresh world's POPs already have their grown size and so do a stepped world's. Between calls every POP has traded, consumed and grown. Growth doesn't read the market, so applying it after all hexes rather than hex by hex changes no result; it is done last so the outcomes a caller reads between calls are complete.

What a caller sees for one POP between calls:

| Read | Value |
|---|---|
| `world.pop_outcome(pop).starting_size()` | The size this turn's endowment, demand and trade were for |
| `world.pop_outcome(pop).subsistence_index()` | u this turn |
| `world.pop_outcome(pop).growth_rate()` | The rate applied after the round |
| `world.pops().get(pop).size` | The size after growth: `starting_size × (1 + rate)`, which next turn's endowment will use |
| `report.population` | The sum of every POP's size after growth; 0 for a world without POPs |

Because the size after growth differs from the size the turn's numbers were for, code that compares this turn's demand or trade with a size must use `starting_size()`, not `Pop.size`. The dev UI's hex panel shows both, with the index and the rate (see [dev-ui.md](dev-ui.md)).

## Setting format

Every `[[pop_type]]` needs a `growth` sub-table with three required keys, written inline or as a `[pop_type.growth]` table like `utility` (the keys, errors and loading are in [pops.md](pops.md#setting-format)):

```toml
[[pop_type]]
name = "peasant"
endowment = { unskilled_labor = 10 }
utility = { unskilled_labor = { subsistence = 2, share = 0.2 }, grain = { subsistence = 2, share = 0.8 } }
growth = { growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }
```

`growth = [0.06, -0.03]` is the rate 0.06u − 0.03u²: it peaks at u = 1 with 3% a turn and crosses zero at u = 2, so growth falls off as prosperity rises and turns negative for very rich POPs (floored at −`decline`). A single coefficient such as `[0.03]` is plain linear growth.

## Worked examples

From the sample setting, whose two types both carry `growth = [0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05`, at a typical hex's prices (labor 1, grain 2, copper 5, in `silver`):

| POP | M, C per individual | u | rate | Size |
|---|---|---|---|---|
| `peasant` on a market hex | 10, 6 | 2/3 | 0.06 × 2/3 − 0.03 × 4/9 ≈ 0.026667 | × 1.026667 each turn: the center peasant 60 → 61.6 at turn 0 |
| `artisan` at the center | 10, 5 | 1 | 0.03 | 10 → 10.3 at turn 0 → 10.609 after one step |
| `peasant` on (2, −2), no market | grain attainment 0 | −1 | −0.1 | 12 → 10.8 at turn 0 → 9.72 → 8.748 |
| `peasant`, grain at 10 | 10, 22 | 10/22 − 1 ≈ −0.5455 | ≈ −0.05455 | shrinks about 5.5% a turn |

The rates repeat every turn (u depends only on prices and the type, not on N), so the sample's sizes compound: the population is 419.4466666666666 at turn 0, 429.2982444444444 at turn 1 and 439.55036096296294 at turn 2, from 410 in the setting. Each turn's endowment is 10 labor times the previous turn's population.

## Extending growth

- **A different rule** (another shape below subsistence, a dependence on something other than u): change `growth_rate`, and `GrowthParams` with its loader if it needs new parameters.
- **A different index** (Stage 05's rationing, where demanded and consumed quantities differ): change `subsistence_index`; its callers don't change.
- **New parameters**: add a field and accessor to `GrowthParams`, extend its `new` checks, read the key in `read_growth` in `setting/pops.rs`, and give every test setting's `growth` sub-table the key (it is a fixed-field sub-table, so an unlisted key is reported as unknown).

## Not yet

Removing a POP, a minimum size, or any event when a POP becomes very small (N only stays positive); rounding N; growth parameters anywhere but the POP type (no global table, no per-POP override) or changing during play; age structure, migration, or any dependence on other POPs; and rationing's effect on the index (Stage 05).

## Tests

- `growth.rs` unit tests build demands with `demand` and the shared `Prices` price list, POP type builders and goods constants from `crates/engine/src/test_support.rs` (its own `g()` parameters stay local): the index above subsistence (2/3 for a peasant, 1 for an artisan-like type), exactly 0 at subsistence, 0 when subsistence costs nothing, the shortfall below subsistence (10/22 − 1), −1 for an unobtainable need whatever the income, the worst attainment winning, staying finite when the ratio overflows, and not depending on size; the rate following the polynomial, exactly 0 at u = 0, capped and floored (including an overflowing polynomial), linear below subsistence, and 0 for zero parameters; `grown_size` multiplying and saturating.
- `pops.rs` unit tests: `GrowthParams` stored and returned by `PopType::growth()`, each `GrowthParams::new` panic, and `Pops::set_size`.
- `crates/engine/tests/growth.rs`: the sample at turn 0 (every POP's index, rate, starting size and grown size; the population as the sum), the sample after one and two steps (the north-east peasant at 9.72 and 8.748, the endowment equal to 10 × the previous population, trade per individual unchanged), two sample worlds growing identically; the `pops_growth` table at turn 0 and after a step; a POP at subsistence keeping its size; a poor POP shrinking by `decline` times its shortfall; the cap and the floor; zero-cost and unobtainable needs; zero parameters never changing size; worlds without POPs reporting a population of 0; and the zero-growth settings `pops_trade` and `pops_placed` keeping their sizes and totals.
- `pops_growth` (`crates/engine/tests/settings/pops_growth/`): goods `labor`, `grain`, `copper`, `silver` (every hex's currency); hexes (0, 0), (1, 0), (2, 0) with grain at 2, 4 and 10 and copper unpriced; types `peasant`, `boomer` (`[1]`, so the cap binds), `rich` (u = 3, past the zero crossing), `richer` (u = 9, floored at −0.1), `carefree` (no subsistence, u = 0), `wanting` (an unobtainable copper need, u = −1) and `static` (zero parameters); ten POPs covering every case. Its `README.md` entry lists every POP with its expected index, rate and size.
