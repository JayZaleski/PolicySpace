# Step 07: pop-growth — Docs Changes

This Step adds one new document, `docs/growth.md`, for the subsistence index and the growth rate, and updates the places in earlier Steps' drafts that said a POP's size "will" change in Step 07, that every turn of the sample looks the same, or that list the `[[pop_type]]` keys, `PopOutcome`'s methods, `TurnReport`'s fields or the `EntryReader` accessors. Steps 01 to 06's drafts have not been applied to `docs/` yet; where a proposal below replaces part of one of them, it says so in **Supersedes**. The sample's turn lines changed again: every line ends with `; population N`, and the sample's numbers now change from turn to turn because its POPs grow, so every quoted turn line is replaced below with the real printed form.

## docs/growth.md — new file
- **Location:** whole file
- **Change:** add

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

`World::new` runs turn 0's start, trade round and growth, so a fresh world's POPs already have their grown size; `World::step` runs the next turn's start, trade round and growth. Between calls every POP has traded, consumed and grown. Growth doesn't read the market, so applying it after all hexes rather than hex by hex changes no result; it is done last so the outcomes a caller reads between calls are complete.

What a caller sees for one POP between calls:

| Read | Value |
|---|---|
| `world.pop_outcome(pop).starting_size()` | The size this turn's endowment, demand and trade were for |
| `world.pop_outcome(pop).subsistence_index()` | u this turn |
| `world.pop_outcome(pop).growth_rate()` | The rate applied after the round |
| `world.pops().get(pop).size` | The size after growth: `starting_size × (1 + rate)`, which next turn's endowment will use |
| `report.population` | The sum of every POP's size after growth; 0 for a world without POPs |

Because the size after growth differs from the size the turn's numbers were for, code that compares this turn's demand or trade with a size must use `starting_size()`, not `Pop.size`.

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

Removing a POP, a minimum size, or any event when a POP becomes very small (N only stays positive); rounding N; growth parameters anywhere but the POP type (no global table, no per-POP override) or changing during play; age structure, migration, or any dependence on other POPs; rationing's effect on the index (Stage 05); and showing each POP's index, rate and size in the dev UI (Step 08).

## Tests

- `growth.rs` unit tests build demands with `demand` and a hand-written `PriceList`: the index above subsistence (2/3 for a peasant, 1 for an artisan-like type), exactly 0 at subsistence, 0 when subsistence costs nothing, the shortfall below subsistence (10/22 − 1), −1 for an unobtainable need whatever the income, the worst attainment winning, staying finite when the ratio overflows, and not depending on size; the rate following the polynomial, exactly 0 at u = 0, capped and floored (including an overflowing polynomial), linear below subsistence, and 0 for zero parameters; `grown_size` multiplying and saturating.
- `pops.rs` unit tests: `GrowthParams` stored and returned by `PopType::growth()`, each `GrowthParams::new` panic, and `Pops::set_size`.
- `crates/engine/tests/growth.rs`: the sample at turn 0 (every POP's index, rate, starting size and grown size; the population as the sum), the sample after one and two steps (the north-east peasant at 9.72 and 8.748, the endowment equal to 10 × the previous population, trade per individual unchanged), two sample worlds growing identically; the `pops_growth` table at turn 0 and after a step; a POP at subsistence keeping its size; a poor POP shrinking by `decline` times its shortfall; the cap and the floor; zero-cost and unobtainable needs; zero parameters never changing size; worlds without POPs reporting a population of 0; and the zero-growth settings `pops_trade` and `pops_placed` keeping their sizes and totals.
- `pops_growth` (`crates/engine/tests/settings/pops_growth/`): goods `labor`, `grain`, `copper`, `silver` (every hex's currency); hexes (0, 0), (1, 0), (2, 0) with grain at 2, 4 and 10 and copper unpriced; types `peasant`, `boomer` (`[1]`, so the cap binds), `rich` (u = 3, past the zero crossing), `richer` (u = 9, floored at −0.1), `carefree` (no subsistence, u = 0), `wanting` (an unobtainable copper need, u = −1) and `static` (zero parameters); ten POPs covering every case. Its `README.md` entry lists every POP with its expected index, rate and size.

## docs/README.md — modify
- **Location:** the document table, new row after the `trade.md` row (added by Step 06)
- **Change:** add

| [growth.md](growth.md) | POP growth in part 2 of the turn: the subsistence index built from a POP's demand, the growth rate from its type's `growth` parameters (polynomial above subsistence, linear decline below, clamped), and how the World applies it to POP sizes |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same row

| [world.md](world.md) | The running `World`, the three-part turn (emission and POP endowment, the trade round and POP growth, perishing), hex and POP stockpiles with perish groups and removal, per-POP trade outcomes, turn reports with the total population, and the setting checker's `--turns` option |

## docs/README.md — modify
- **Location:** the document table, the `pops.md` row (as replaced by Step 04)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same row

| [pops.md](pops.md) | POP types (the endowment, the Stone-Geary utility parameters and how shares are normalized, and the growth parameters) and placed POPs (`Pops`, continuous size, one POP per type per hex), how `[[pop_type]]` and `[[pop]]` sections are loaded and checked, and the per-turn endowment |

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Steps 04, 05 and 06)
- **Location:** the opening, the paragraph starting "The names follow one rule"
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

The names follow one rule: "POP" means the placed group, so `PopId` is a placed POP's ID and `PopTypeId` is a POP type's. Every turn, after the endowment, each POP trades and consumes in the trade round (see [trade.md](trade.md)): the hex's market computes its demand (see [demand.md](demand.md)) from its type's subsistence amounts and shares, sells what it doesn't keep and buys what it wants through the dummy market (see [market.md](market.md)), and the POP consumes its utility goods. Then each POP's size grows or shrinks by a rate from its type's growth parameters and how far above or below subsistence it was (see [growth.md](growth.md)).

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Concepts", new bullet after the "Leisure" bullet
- **Change:** add

- **Growth.** A type also says how its POPs' size responds to prosperity: a `growth` sub-table with polynomial coefficients for the rate above subsistence, a `decline` for the rate below it, and a `max_growth` cap. The rate depends only on the POP's subsistence index, so every POP of a type at the same prices grows at the same rate whatever its size (see [growth.md](growth.md)).

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Step 04)
- **Location:** "Data model", the code block
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same code block

```rust
pub enum PopTypeKind {}                   // IdKind, label "pop type"
pub type PopTypeId = Id<PopTypeKind>;
pub struct Endowment { pub good: GoodId, pub amount: Quantity }
pub struct UtilityGood { pub good: GoodId, pub subsistence: Quantity, pub share: f64 }
pub struct GrowthParams { coefficients: Vec<f64>, decline: f64, max_growth: f64 }
pub struct PopType { endowment: Vec<Endowment>, utility: Vec<UtilityGood>, growth: GrowthParams }
pub type PopTypes = NamedList<PopTypeKind, PopType>;
```

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Data model", the `PopType` method table, new row after the `utility_of(good)` row
- **Change:** add

| `growth() -> &GrowthParams` | The growth parameters: `coefficients()` (never empty, every one finite), `decline()` (in [0, 1)) and `max_growth()` (at least 0). See [growth.md](growth.md) |

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Data model", "### Construction and normalization", the first sentence ("`PopType::new(endowment, utility)` is crate-private: only the loader builds POP types. It takes ... and normalizes the weights:") and the paragraph starting "`new` panics if a good appears twice"
- **Change:** replace both

`PopType::new(endowment, utility, growth)` is crate-private: only the loader builds POP types. It takes `(GoodId, Quantity)` endowment pairs and `(GoodId, Quantity, f64)` utility triples (good, subsistence, weight as written) in any order, plus a `GrowthParams`, sorts both lists by good ID, and normalizes the weights:

`new` panics if a good appears twice in either list, if a weight is negative or not finite, or if no weight is positive. `GrowthParams::new` (also crate-private) panics if the coefficients are empty or not finite, if `decline` is not in [0, 1), or if `max_growth` is negative or not finite. The loader guarantees all of these, so a panic is an internal bug. Unit tests that need a type whose size never changes build it with `GrowthParams::new(vec![0.0], 0.0, 0.0)`.

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Step 04)
- **Location:** "Setting format", the first paragraph (starting "A `[[pop_type]]` belongs in `rules/`") and the key table after it
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same paragraph; Step 03 (pop-types), the key table

A `[[pop_type]]` belongs in `rules/` (in `start/` it gets the usual `section 'pop_type' belongs in rules/, not start/`). The header is `[[pop_type]]`; `[[pop]]` places POPs of these types in `start/` (see [Placed POPs](#placed-pops)). Four keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among POP types. POP type names and good names are separate namespaces |
| `endowment` | A table of `good name = amount`. Each amount is a number (whole or decimal), finite and greater than 0. May be empty (`{}`) |
| `utility` | A table of `good name = sub-table`. Each sub-table has two optional keys, `subsistence` and `share`, each a finite number of at least 0 that defaults to 0. A good needs at least one of them positive, and at least one good needs a positive `share` |
| `growth` | A sub-table with three required keys: `growth`, a non-empty array of finite numbers (the polynomial coefficients c₁, c₂, … of the rate above subsistence); `decline`, a finite number of at least 0 and less than 1 (the rate at u = −1, applied linearly below subsistence); and `max_growth`, a finite number of at least 0 (the cap on the rate). See [growth.md](growth.md) |

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Setting format", the sentence before the TOML example ("Both TOML table forms are accepted for `endowment`, for `utility` and for each utility good:"), the TOML example, and the sentence after it
- **Change:** replace

Both TOML table forms are accepted for `endowment`, for `utility`, for each utility good and for `growth`:

```toml
[[pop_type]]
name = "farmer"
endowment = { labor = 10 }
utility = { grain = { subsistence = 2, share = 3 }, labor = { subsistence = 1, share = 1 } }
growth = { growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }

[[pop_type]]
name = "fisher"

[pop_type.endowment]
fish = 2
labor = 8

[pop_type.utility.fish]
subsistence = 1        # share 0: a pure necessity

[pop_type.utility.coin]
share = 0.5

[pop_type.growth]
growth = [0.03]        # plain linear growth
decline = 0            # never shrinks
max_growth = 1
```

TOML forbids repeated keys in one table, so a good can't appear twice in an endowment or a utility.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "How POP types are loaded", the numbered list, item 1: new sub-bullet after the bullet about "no good has a positive share" and before the bullet starting "Finally the name is defined"
- **Change:** add

   - `growth` with the private `read_growth`, which calls `EntryReader::required_sub_table("growth")` (see [setting-loader.md](setting-loader.md#entryreader)) and reads, in this order and all before deciding anything: `growth` with `required_number_array` (an empty array is reported at the array as `key 'growth' in growth of pop_type must have at least one coefficient`; an element that is a number but not finite at the element as `element 3 of key 'growth' in growth of pop_type must be a finite number`), `decline` and `max_growth` with `required_number` and their range checks, and then the sub-reader's `finish` for unknown keys. Its keys are fields (name-checked, unknown ones reported), unlike the data keys of `endowment` and `utility`. Missing keys inside are reported at the `growth` key (or at its name in a `[pop_type.growth]` header). The entry's `growth: Option<GrowthParams>` is `None` if anything in the sub-table was bad. The name is still defined, so a type with a bad `growth` takes part in the duplicate check.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "How POP types are loaded", item 3 ("**Finish.**"), its second sentence ("`PopTypesReader::finish` skips entries without an ID, builds each `PopType::new` from the resolved goods and the valid values, and returns `PopTypes::from_parts`.")
- **Change:** replace

`PopTypesReader::finish` skips entries without an ID, builds each `PopType::new` from the resolved goods, the valid values and the entry's `GrowthParams`, and returns `PopTypes::from_parts`. A `None` growth in a successful load panics with `internal error: a POP type's growth was invalid in a successful load`.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "How POP types are loaded", the `[[pop_type]]` error table, new rows after the `an unknown key in a utility good` row
- **Change:** add

| `growth` missing | header | `missing required key 'growth' in pop_type` |
| `growth` not a table | value | `key 'growth' in pop_type must be a table, found integer` |
| `growth`, `decline` or `max_growth` missing inside `growth` | the `growth` key (or its name in a `[pop_type.growth]` header) | `missing required key 'decline' in growth of pop_type` |
| the `growth` list not an array | value | `key 'growth' in growth of pop_type must be an array, found float` |
| the `growth` list empty | value | `key 'growth' in growth of pop_type must have at least one coefficient` |
| a coefficient that isn't a number, or an integer beyond `i64` | element | `element 2 of key 'growth' in growth of pop_type must be a number, found string` (or `... is out of range`) |
| a coefficient that is `inf` or `nan` | element | `element 3 of key 'growth' in growth of pop_type must be a finite number` |
| `decline` or `max_growth` not a number | value | `key 'decline' in growth of pop_type must be a number, found string` |
| `decline` negative, 1 or more, or not finite | value | `key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1` |
| `max_growth` negative or not finite | value | `key 'max_growth' in growth of pop_type must be a finite number of at least 0` |
| an unknown key in `growth` | key | `unknown key 'cap' in growth of pop_type` |

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Using POP types", after the code block (after the sentence Step 05 added about `demand`)
- **Change:** add

`peasant.growth()` gives the type's `GrowthParams`, and `growth_rate(peasant.growth(), u)` gives the rate at any index (see [growth.md](growth.md)).

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", the first paragraph, the sentence starting "**Size is continuous**" ("... Step 07's growth multiplies N by a rate, which with whole numbers would freeze small POPs or need a rounding rule.")
- **Change:** replace

**Size is continuous**, a `Quantity` rather than an integer: growth multiplies N by a rate every turn (see [growth.md](growth.md)), which with whole numbers would freeze small POPs or need a rounding rule.

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", "### Data model", the code block's `Pop` struct, the `size` line
- **Change:** replace

```rust
    pub size: Quantity,                   // N; finite, greater than 0, changed each turn by growth
```

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", "### Data model", the paragraph starting "`Pops::from_pops(pops, hex_count)` is crate-private", new sentences at its end
- **Change:** add

`Pops::set_size(id, size)` is crate-private too: only the World's growth step changes a size. A size is greater than 0 in the setting and is multiplied by a factor above 0 each turn, so it stays positive (only floating-point underflow after thousands of turns of decline could reach 0).

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", "### Endowment in the turn", the first sentence ("The `World` takes the `Pops` over as state (`World::pops()`), because Step 07 will change sizes, ...")
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same sentence

The `World` takes the `Pops` over as state (`World::pops()`), because growth changes their sizes every turn, and gives every POP its own `Stockpile` in `pop_stockpiles: IdVec<PopKind, Stockpile>`, read with `World::pop_stockpile(pop)`.

## docs/pops.md — modify (Step 04's added "Placed POPs" section)
- **Location:** "Placed POPs", new subsection after "### Trade and consumption in the turn" (added by Step 06) and before "### Using POPs"
- **Change:** add

### Growth in the turn

After every hex has traded, each POP's size becomes `size × (1 + rate)`, where the rate comes from its subsistence index this turn and its type's growth parameters (see [growth.md](growth.md)). The next turn's endowment and demand use the new size. So between calls `world.pops().get(id).size` is the size after growth, and the size this turn's endowment and trade were for is `world.pop_outcome(id).starting_size()`. In the sample, a peasant on a market hex grows about 2.67% a turn, the center artisan 3%, and the north-east peasant, which can never get grain, shrinks 10% a turn (12 → 10.8 → 9.72).

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

Showing POP types, POPs, their trade outcomes and their growth in the dev UI (Step 08). Removing POPs that shrink, a minimum size, starting stockpiles or cash for POPs, saving, changing a POP's hex or type during play, migration, and per-POP parameters that differ from the type (growth included) are not planned for this Stage.

## docs/pops.md — modify (Step 03's new-file draft, as replaced by Step 04)
- **Location:** "The sample setting and tests", the first two bullets (`settings/sample/rules/pops.toml` and `settings/sample/start/pops.toml`)
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same two bullets

- `settings/sample/rules/pops.toml` defines two types, both endowed with 10 `unskilled_labor` per individual per turn and both with `growth = [0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05` in a `[pop_type.growth]` table. `peasant` (ID 0): `unskilled_labor` subsistence 2, share 0.2; `grain` subsistence 2, share 0.8. `artisan` (ID 1): written with weights 1, 4 and 5, normalized to `unskilled_labor` subsistence 1, share 0.1; `copper` share 0.5 (no subsistence); `grain` subsistence 2, share 0.4. Its header comment explains the keys (including the subsistence index, the polynomial, the decline, the clamp, and that `growth = [0]` with `decline = 0` freezes a type), that shares are weights, leisure, and that unskilled labor is the intended endowment, and points to `start/pops.toml`.
- `settings/sample/start/pops.toml` places 16 POPs, listed in the map's order (row by row, north to south, west to east): a `peasant` on 15 of the 19 hexes, with sizes from 12 to 60, and an `artisan` of size 10 on the center hex (0, 0), the only hex that prices every good. The center therefore holds two POPs (IDs 7 and 8). The north-east corner (2, -2), which has no market, holds a `peasant` (ID 2) whose labor can never be sold and which never gets grain, so it shrinks every turn. Hexes (2, -1), (-2, 0), (1, 1) and (-2, 2) have no POPs. The sizes total 410 in the setting, so POPs are endowed `4100 unskilled_labor` at turn 0; from then on the sizes grow or shrink. The file's header comment explains the keys, continuous size, the one-POP-per-type-per-hex rule, that POPs start with nothing, and that sizes change each turn.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "The sample setting and tests", the `crates/engine/tests/pops.rs` bullet and the test-settings bullet
- **Change:** replace both
- **Supersedes:** Step 06 (pop-trade), the same two bullets

- `crates/engine/tests/pops.rs`: the sample's two types with their normalized shares (summing to 1) and their growth parameters; `pops_valid` with IDs in load order, both table forms (for `growth` too), a multi-good endowment written out of ID order, an empty endowment, a pure necessity, the currency in a utility, each type's growth parameters, and the same result on a second load; every `pops_errors` error at its place, including every `growth` error; `valid_minimal` with no POP types. For placed POPs: the sample's 16 POPs with their hexes, types and sizes and its center, corner and empty hexes; a fresh sample world in which every POP's recorded demand shows `10 × starting_size` labor held, every POP stockpile holds no labor and at most a sliver of silver, and the report's `endowed` is 4100 labor; steps that re-endow by the grown sizes (each turn's `endowed` is 10 × the previous report's `population`), with the labor used up rather than perishing; `pops_placed` loaded and stepped (the workers consume their labor, the hoarder keeps its labor and its growing gold, every size unchanged and the population 12.5 every turn); every `pops_placed_errors` error at its place; worlds with no POPs endowing and trading nothing, with a population of 0; and `pop_stockpile` panicking on a POP from another world. Growth itself is tested in `crates/engine/tests/growth.rs` (see [growth.md](growth.md)).
- Test settings: `pops_valid` (goods `coin`, `labor`, `grain`, `fish`; types `farmer`, `fisher`, `idle` across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, with the goods in a file read after them; real growth parameters, `fisher`'s in the `[pop_type.growth]` form); `pops_one` (one good, one type, one hex and one POP, for the checker's `1 pop type`, `1 hex` and `1 pop`); `pops_errors` (every entry in `rules/a.toml`, `rules/b.toml` and `start/p.toml` ends with a valid `growth` line, and `rules/c.toml` holds every `growth` error, including one entry, a duplicate of `ok`, with four growth problems); `pops_placed` (goods `labor` and `grain`, both perishing after 1 turn, and durable `gold`; types `worker` endowing 10 `labor` with a `labor` utility, `hoarder` endowing 4 `labor` and 1.5 `gold` with a `grain` utility, and `idle` with an empty endowment; hexes (0, 0), (1, 0), (2, 0), each with currency `gold` and none with a market, and a `field` deposit on (2, 0); POPs `worker` 3 and `hoarder` 2 on (0, 0) in `start/a.toml`, `worker` 0.5 and `idle` 7 on (1, 0) in `start/b/c.toml`. With no market, the workers consume their labor and the hoarder, whose utility good it doesn't hold, keeps its labor and its gold, which accumulates); `pops_placed_errors` (every `[[pop]]` error in the table above, one entry with four problems, a duplicate across files, and a `[[pop]]` in `rules/`); `pops_trade`, for the trade round (see [trade.md](trade.md)); and `pops_growth`, for growth (see [growth.md](growth.md)). `pops_one`, `pops_placed`, `pops_placed_errors` and `pops_trade` give every type zero growth parameters (`growth = [0]`, `decline = 0`, `max_growth = 0`), so their sizes never change and their earlier expectations hold.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** the intro, the paragraph starting "The trade round is part 2 of the turn"
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

The trade round is part 2 of the turn ("actors act", see [world.md](world.md#the-turn)) for POPs. On every hex, each POP hands the hex's market a **submission**: its type's Stone-Geary parameters, its size and its whole stockpile. The market posts its prices, computes every submission's demand with the `demand` function (see [demand.md](demand.md)), settles **every POP's sales before any POP's purchases**, and then each POP **consumes** the utility goods it ended up with. The World keeps each POP's **outcome** (its demand, what it sold, bought and consumed, and its subsistence index and growth rate) until the next turn, and the turn report gains the totals. After every hex has traded, the World applies each outcome's growth rate to the POP's size (see [growth.md](growth.md)).

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** the intro, the paragraph starting "`trade.rs` depends only on"
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

`trade.rs` depends only on `demand`, `goods`, `growth`, `market`, `pops` (for `PopType`), `quantity` and `stockpile`. It knows nothing about the `World`, `Pops`, `IdVec` or files: the World builds the submissions and puts the results back.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Where it runs in the turn", the first paragraph (starting "`World::new` runs turn 0's start")
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

`World::new` runs turn 0's start (emission, endowment), then turn 0's trade round, then turn 0's growth; `World::step` ends the current turn, advances the clock, and runs the new turn's start, trade round and growth. So between calls the world is **after part 2**: every POP has traded, consumed and grown, and its stockpile holds only what is left (leftover currency, goods it couldn't sell, untradable goods). One `TurnReport` covers one whole turn's trade. Because growth has already run, a POP's current size is not the size the turn's trade was for; that size is the outcome's `starting_size()`.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Data model", "### `PopOutcome`", the method table, new rows after the `carried_over()` row
- **Change:** add

| `subsistence_index() -> f64` | The POP's subsistence index this turn: 0 at subsistence, positive above, down to −1 below (see [growth.md](growth.md)) |
| `growth_rate() -> f64` | The growth rate the World applies to the size after the round, within `[−decline, max_growth]` of the POP's type |
| `starting_size() -> Quantity` | The size the turn started with: the size the endowment, the demand and the trade were for. The POP's current size is the size after growth |

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Data model", "### `PopOutcome`", the paragraph after the table (starting "`World::pop_outcome(pop) -> &PopOutcome`")
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

`World::pop_outcome(pop) -> &PopOutcome` returns the outcome of the current turn's round. It panics with `is not from this list` for a POP from another world. The outcome is replaced every turn; the World's growth step reads its growth rate, and Step 08 displays it.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "The four passes", item 4 (**Consumption**), its last sentence ("Then `carried_over` is read from the stockpile and the outcome is built.")
- **Change:** replace

Then `carried_over` is read from the stockpile, the POP's subsistence index and growth rate are computed from its demand and its type's growth parameters (`growth::subsistence_index` and `growth::growth_rate`, see [growth.md](growth.md)), and the outcome is built with the submission's size as `starting_size`. The round only records the rate; the World applies it after every hex has traded.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Worked examples", the paragraph after the table (starting "Every turn looks the same")
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

Per individual, every turn looks the same, because labor is endowed again each turn and the silver carried over is only a sliver. The totals change from turn to turn because the POPs grow (see [growth.md](growth.md)). At turn 0, with the setting's sizes, the sample reports `sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain` (the 388 peasants with a market at 7.2, 3.6 and 2.8 labor or grain each, plus the size-10 artisan, plus the 120 labor the no-market peasant consumes). At turn 1 the same per-individual amounts times the grown sizes give `sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain`. The POPs' labor never appears in `perished`.

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Using outcomes", the code block
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same code block

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

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

Price adjustment, rationing, market inventory or any limit on what the market trades, and aggregating submissions into one demand per type (Stage 05; with rationing, the subsistence index will need to reconsider whether it reads what was demanded or what was consumed); saving beyond rounding leftovers; rounding rules or a smallest unit of currency; any actor other than POPs trading; per-hex or per-POP breakdowns in the turn report (the per-POP numbers are in `World::pop_outcome`); and showing outcomes in the dev UI (Step 08).

## docs/trade.md — modify (Step 06's new-file draft)
- **Location:** "Tests", the `crates/engine/tests/trade.rs` bullet
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `crates/engine/tests/trade.rs`: the sample's turn 0 (every peasant with a market and the artisan, with their outcomes and stockpiles, sizes taken from `starting_size()`), the no-market peasant, the report totals and the perished list at turn 0, and turn 1's totals computed from the grown sizes; three steps repeating the outcomes per individual (each amount and the income divided by `starting_size()`) with two worlds agreeing; the `pops_trade` table at turn 0 and after a step (zero growth, so sizes don't change); currency carried over as income (the miser, the saver); a below-subsistence POP; held goods outside the utility (the collector's gem, the miser's silver); a hex with a market and no POPs; worlds without POPs reporting nothing traded and a population of 0; and `pop_outcome`'s foreign-POP panic.

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** the intro, the paragraph starting "Nothing calls it during a turn yet." (as replaced by Step 06, starting "The trade round (see [trade.md](trade.md)) calls it")
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

The trade round (see [trade.md](trade.md)) calls it for every POP in the "actors act" part of every turn: the market computes each POP's demand from its whole stockpile, sells the surplus and buys the shortfall, and the POP consumes its utility goods up to the demanded quantities. The World keeps each POP's `Demand` in its turn outcome (`World::pop_outcome(pop).demand()`). POP growth builds the POP's subsistence index from the income, subsistence cost and per-good demanded and subsistence quantities it reports (`growth::subsistence_index`, see [growth.md](growth.md)), so how far a POP is above or below subsistence decides how fast it grows or shrinks.

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "Not yet", the whole paragraph (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

Rounding or a smallest unit, and demand rules other than the closed form, such as priorities among subsistence goods or saving.

## docs/demand.md — modify (Step 05's new-file draft)
- **Location:** "Tests", the `crates/engine/tests/demand.rs` bullet (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `crates/engine/tests/demand.rs` uses the sample setting and `world.market(hex)` as the `PriceList`: the peasant and artisan at typical prices, demand linear in size (N 50 versus N 1), a POP on the hex with no market, the demand recorded in a POP's turn-0 outcome equal to the pure function called on its endowment with the outcome's `starting_size()` (the center peasant, size 60, whose current size after turn 0's growth is 61.6: 600 labor held, 168 labor and 216 grain demanded), and a sweep over types, sizes, holdings and hexes checking that the budget balances. Quantities are compared within `1e-9`; the budget within `1e-9 × max(1, M)`.

## docs/world.md — modify
- **Location:** "What a World holds", the field table: the `pops` row (added by Step 04) and the `pop_outcomes` row (added by Step 06)
- **Change:** replace both rows
- **Supersedes:** Step 04 (pops), the `pops` row; Step 06 (pop-trade), the `pop_outcomes` row

| `pops: Pops` | the setting's `StartState::pops` | every turn: each POP's size is set by growth after the trade round (see [growth.md](growth.md)); hexes and types never change |
| `pop_outcomes: IdVec<PopKind, PopOutcome>` | built by turn 0's trade round | rebuilt every turn by the trade round: each POP's demand, what it sold, bought, consumed and carried over, its subsistence index and growth rate, and its starting size (see [trade.md](trade.md)) |

## docs/world.md — modify
- **Location:** "The turn", from the numbered list through the paragraph starting "So between calls the world is always **after part 2**" (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same span

A turn has three parts:

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good into its hex's stockpile. Then every POP receives its type's endowment times its size into its own stockpile (see [Endowment](#endowment)).
2. **Actors act:** the trade round and consumption, then growth (see [Trade round](#trade-round), [Growth](#growth), [trade.md](trade.md) and [growth.md](growth.md)). On every hex, each POP submits its parameters and holdings, the hex's market computes their demand and settles every sale before any purchase, and each POP consumes its utility goods. Then, once every hex has traded, every POP grows or shrinks by the rate its outcome records. Later Stages add industries and the live market here.
3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile and then from every POP stockpile.

Two calls drive the clock:

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 and runs turn 0's start, turn 0's trade round and turn 0's growth, so a fresh world's deposits have emitted and its POPs have already been endowed, traded, consumed and grown. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing (sizes included). Then it runs the end of the current turn (perishing: hexes in ID order, then POPs in ID order), advances the clock, and runs the start of the new turn (emission, then endowment by the grown sizes), its trade round and its growth.

So between calls the world is always **after part 2**: this turn's output is in the hex stockpiles, this turn's trade is done, each POP's stockpile holds what the round left, each POP's size is its size after growth, and nothing has perished yet this turn. This is the state that tests, the checker, and the dev UI observe. If observation happened after the end of the turn, goods with `perishes_after = 1` would always look empty.

## docs/world.md — modify
- **Location:** "The turn", new subsection after "### Trade round" (added by Step 06) and before "### Perishing"
- **Change:** add

### Growth

`World::grow` (private) runs after `trade`, in both `new` and `step`. For each POP in ID order it reads the growth rate from the POP's `PopOutcome`, sets the size to `growth::grown_size(size, rate)` (`size × (1 + rate)`, saturating) with the crate-private `Pops::set_size`, and adds the new size to a running total, which becomes the report's `population`. It reads `pop_outcomes` and writes `pops`, which are separate fields, so the borrows don't conflict. The rate was computed by the trade round from the POP's demand and its type's growth parameters; see [growth.md](growth.md) for the index and the rate.

## docs/world.md — modify
- **Location:** "The turn", "### Determinism", the first sentence (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same sentence

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and goods in ID order within a POP, with every sale on a hex before any purchase and then consumption, POPs grow in ID order, stockpiles perish hexes first and then POPs, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds (POP sizes included), outcomes and reports.

## docs/world.md — modify
- **Location:** "TurnReport", the whole section body (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same section

```rust
pub struct TurnReport {
    pub turn: Turn,                         // the turn that started
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn, hexes and POPs
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted by deposits at the start of `turn`
    pub endowed: Vec<(GoodId, Quantity)>,   // received by POPs as endowment at the start of `turn`
    pub sold: Vec<(GoodId, Quantity)>,      // sold by POPs in the trade round of `turn`
    pub bought: Vec<(GoodId, Quantity)>,    // bought by POPs in the trade round of `turn`
    pub consumed: Vec<(GoodId, Quantity)>,  // consumed by POPs after the trade round of `turn`
    pub population: Quantity,               // total size of every POP after growth in `turn`
}
```

Each list has one total per good, summed over every hex (and, for `perished`, `endowed`, `sold`, `bought` and `consumed`, every POP), in good ID order, and lists only amounts greater than zero. `emitted` and `endowed` are kept apart so deposit output and POP income can be told apart. `population` is a single total, `Quantity::ZERO` for a world with no POPs. There is no per-hex or per-POP breakdown in the report: what each holds is in its stockpile, what each POP traded and how it grew is in its `PopOutcome` (`World::pop_outcome`), and its size is in `World::pops()`. For the sample setting turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`; endowed 4100 `unskilled_labor`; sold 2878.6 `unskilled_labor`; bought 5 `copper` and 1426.8 `grain`; consumed 1221.4 `unskilled_labor`, 5 `copper` and 1426.8 `grain`; population 419.4466666666666". Because the POPs grow, later turns endow, trade and consume more (each turn's `endowed` is 10 labor times the previous turn's `population`), and `perished` is the deposit output only: the POPs sell or consume all their labor in the round, so none is left to perish (apart from floating-point slivers, which the checker would print).

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`", the example block and the sentence after it (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same block and sentence

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666
Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4194.466666666667 unskilled_labor; sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain; population 429.2982444444444
```

The line lists `perished`, `emitted`, `endowed`, `sold`, `bought` and `consumed` in that order, then `population`. Entries are `<amount> <good name>` joined by `, ` in report order (good ID order: `copper` comes before `grain` in the sample's goods); an empty list prints `nothing`, so a setting without POPs ends every line with `; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0`. The amounts are floating-point sums printed with `Quantity`'s `Display`, with nothing rounded, so they can print with a long tail of digits, as the sample's do from turn 1 on once its POPs have grown.

## docs/world.md — modify
- **Location:** "Tests", the unit-tests bullet (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub`, `saturating_mul` and `checked_div`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, zero, and `remove_all`; and `retain`: from the soonest group first, exactly `keep` left in a single group, `keep = 0`, at or below `keep`, and `keep` on a group boundary), `world.rs` (a world forced to `Turn::MAX`, for both `turns_valid` and `pops_placed`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles, POP outcomes and POP sizes unchanged), `trade.rs` (see [trade.md](trade.md)) and `growth.rs` (see [growth.md](growth.md)). Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)), demand in `crates/engine/tests/demand.rs` (see [demand.md](demand.md)), the trade round in `crates/engine/tests/trade.rs`, and growth in `crates/engine/tests/growth.rs`.

## docs/world.md — modify
- **Location:** "Tests", the `crates/engine/tests/world.rs` bullet (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports (the POPs' labor in `endowed`, 10 × the previous report's `population` after turn 0, and only the deposit output in `perished`, since POPs use up their labor), a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits (which also endow and trade nothing, with a population of 0), determinism (reports, hex stockpiles and POP stockpiles of two runs), the accessors, hex currencies being readable and unchanged by stepping, and the foreign-hex panics of `hex_stockpile` and `hex_currency`. POP endowment in the world is tested in `crates/engine/tests/pops.rs` (see [pops.md](pops.md)), the trade round in `crates/engine/tests/trade.rs` (see [trade.md](trade.md)), and growth in `crates/engine/tests/growth.rs` (see [growth.md](growth.md)).

## docs/world.md — modify
- **Location:** "Not yet", the whole paragraph (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same paragraph

Ownership of deposits, starting stockpiles in `start/` (for hexes or POPs), deposit stock, depletion and regeneration, any effect of `movable`, per-hex and per-POP report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, and actors other than POPs in the "actors act" part of the turn.

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first bullet (the list of accessors, as replaced by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same bullet

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `optional_number`, `required_number_table`, `required_number_array`, `required_table_of_tables`, `required_sub_table`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>` (or, for the two table readers, an `Option` of a list, and for `required_sub_table` an `Option<EntryReader>`), where `Located` carries the value and its `Location`. `required_number` and `optional_number` accept an integer or a float and return an `f64`; `inf` and `nan` are returned as written (the caller does the range check), an integer beyond `i64` is reported as out of range, and other types are reported as `must be a number, found <type>`. `optional_number` returns `None` both when the key is absent and when it has the wrong type (the latter reported); use `has` to tell them apart.

## docs/setting-loader.md — modify
- **Location:** "EntryReader", new bullets after the `nested(key, name, table)` bullet (added by Step 03)
- **Change:** add

- `required_sub_table(key)` reads a table-valued key whose keys are **fields** (a POP type's `growth`) and returns a nested `EntryReader` for it. It shares the file and the error sink, its label is `<key> of <label>` (for example `growth of pop_type`, giving `key 'decline' in growth of pop_type must be a number, found string`), and its missing required keys are reported at `key`'s own location: the key in an inline entry, or its name in a `[section.key]` header. A missing `key` gives the usual missing-key error, and a value that isn't a table `key '<key>' in <label> must be a table, found <type>`; both return `None`. The caller calls `finish` on the returned reader to report unknown keys (`unknown key 'cap' in growth of pop_type`). Unlike `required_number_table` and `required_table_of_tables`, whose keys are data, the sub-table's keys are name-checked like the entry's own. Internally it uses the private `find_entry`, a version of `find` that also returns the key (so its span is known); `find` calls it.
- `required_number_array(key)` reads an array of numbers and returns `Option<Located<Vec<Option<Located<f64>>>>>`: the array's location and every element in order, `Some` for a number (parsed as `required_number` parses one, with `inf` and `nan` returned as written) and `None` for an element that isn't one. A bad element is reported at its own position as `element <n> of key '<key>' in <label> must be a number, found <type>` or `... is out of range`, with `n` counted from 1. An empty array returns an empty list (the caller decides whether that's allowed). A missing `key` gives the usual missing-key error, and a value that isn't an array `key '<key>' in <label> must be an array, found <type>`; both return `None`.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the first bullet (as replaced by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same bullet

   - `[[good]]` (`Rules::goods`) is the worked example for a named kind in `rules/`; `[[resource]]` (`Rules::resources`) follows it. `[[pop_type]]` (`Rules::pop_types`) is the example for an entry with nested tables of good references (its `endowment` and `utility`, read with `required_number_table`, `required_table_of_tables` and `nested`), for a sub-table of fixed fields (its `growth`, read with `required_sub_table` and `required_number_array`), and for a reader that keeps every entry for phase 2, even one whose name failed, so its references are still checked (see [pops.md](pops.md)).

## docs/setting-loader.md — modify
- **Location:** "Tests", the first bullet (as replaced by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same bullet

- Unit tests sit in each `setting` module. Framework tests that use `TestContent` are in `test_content.rs`. Tests that need the errors as text lines use the `#[cfg(test)]` helpers `ErrorSink::into_lines` and `LoadErrors::lines`. `reader.rs` tests cover each accessor, including `optional_number` (present, absent, wrong type), `has` not marking a key as read, `required_table_of_tables` in both TOML forms with non-table values, `nested` readers reporting with the nested label and location, `required_sub_table` in both TOML forms (its label, missing keys at the key's location, unknown keys, a non-table and a missing key), and `required_number_array` (every element returned in order, a string and an out-of-range element reported at their positions, `inf` returned as written, an empty array, a non-array and a missing key).

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", the second bullet (the list of test settings, as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `markets_valid`, `markets_errors`, `pops_valid`, `pops_one`, `pops_errors`, `pops_placed`, `pops_placed_errors`, `pops_trade`, `pops_growth`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `turn_line` bullet (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}; sold {list}; bought {list}; consumed {list}; population {n}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn, `sold`, `bought` and `consumed` are the POPs' totals from the turn's trade round, and `population` is the total POP size after growth, printed with `Quantity`'s `Display` (`0` for a world without POPs; see [world.md](world.md), [trade.md](trade.md) and [growth.md](growth.md)).

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet (as replaced by Step 06)
- **Change:** replace
- **Supersedes:** Step 06 (pop-trade), the same bullet

- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets, POP types and POPs counts (`18 markets, 2 pop types, 16 pops` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `3 pop types, 4 pops` for `pops_placed`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists and its `population` (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0` for settings without POPs). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines (`consumed 35 labor; population 12.5`, with nothing sold or bought, since it has no markets, and the same population every turn, since its types don't grow). The sample's turn 0, 1 and 2 lines all differ, because its POPs grow, and are compared exactly as printed. It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules. The `session.rs` unit tests compare the sample's turn 0 and turn 1 lines in the log with the same text.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/currency.rs, ...`
- **Supersedes:** Step 06 (pop-trade), the same block

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/demand.rs       Stone-Geary demand and the PriceList trait (see demand.md)
    src/goods.rs        goods data model (see goods.md)
    src/growth.rs       POP growth: subsistence index, growth rate, grown size (see growth.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types, growth parameters and placed POPs (see pops.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/trade.rs        the trade round and PopOutcome (see trade.md)
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, demand.rs, goods.rs, growth.rs, map.rs, market.rs, pops.rs, resources.rs, setting_loader.rs, trade.rs, world.rs
```
