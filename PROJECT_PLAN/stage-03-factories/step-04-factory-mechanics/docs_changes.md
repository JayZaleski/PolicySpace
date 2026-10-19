# Step 04: factory-mechanics — Docs Changes

## docs/production.md — new file
- **Location:** whole file
- **Change:** add

# Factory mechanics

Source: `crates/engine/src/production.rs`, plus `Bundle::scaled` and `Bundle::merged` in `crates/engine/src/bundle.rs`. The engine crate root re-exports `Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord` and the function `operate`. The three parts `run`, `wear` and `maintain` are public in the `production` module (`policyspace_engine::production::run`, and so on).

This module gives factories (see [factories.md](factories.md)) their physics. It covers what happens when a business runs a factory at some fraction of its capacity, how a factory wears each turn, and what maintaining one costs. Its rules implement the meaning of a technology's numbers (see [technologies.md](technologies.md#what-the-numbers-mean)). Like `demand` (see [demand.md](demand.md)), these are **pure functions**: they work on one business's factories, their technologies, that business's stockpile and its instructions. They read no world state and write none.

**Nothing in the turn calls them yet.** The fractions a business asks for will come from its decider (Steps 05 to 07 of Stage 03). Step 08 calls `operate` once per business in the turn, after the trade round, and keeps the records. Step 09 shows them in the dev UI. Until then the functions can only be observed through their tests.

`production.rs` depends on `bundle`, `goods`, `quantity`, `stockpile`, `technologies` and `turn`. It knows nothing about the `World`, `Factories`, businesses or files. The caller moves a business's stockpile out, builds one `Operation` per factory, calls the functions, and writes each factory's condition back.

## The model

### Two instructions per factory

Each turn a business gives every factory it owns two numbers, each from 0 to 1 inclusive:

- **Run fraction:** the share of the technology's capacity to use this turn. This is a fraction of capacity, not an amount of output. It is the same kind of number as the condition and the maintenance fraction, and it keeps its meaning if the technology or capacity changes.
- **Maintenance fraction:** the share of the technology's full capital bundle to replenish this turn.

The functions assume both numbers are in range and panic if they aren't (see [Checks](#checks)). The decision validation in Step 05 makes sure they arrive in range.

### Lifetime-slices and shutdown

A **lifetime-slice** is 1/t₁ of the capital bundle, where t₁ is the technology's `lifetime()`. It is what one turn of wear takes. A factory **runs only if it holds at least one whole slice**, which means `condition × lifetime ≥ 1`. A factory below one slice is **shut down**: it makes nothing, whatever it is asked. Shutdown is not a separate state. It follows from the condition and the lifetime, and maintenance can undo it.

The reason for the threshold: every turn of output then costs exactly one slice. If a factory could run with any condition above zero, a business could hold it a speck above zero, run it at full capacity, let wear take the speck (wear stops at zero, so the rest of the slice would be forgiven), and buy the speck back for almost nothing. The price is that a partial slice, such as half a slice left, produces nothing and is lost to wear.

Condition does not slow a factory down. A factory that holds a slice runs at its full capacity times the run fraction.

### Running

For each factory that holds a slice, the output asked for is `capacity × run fraction`. Its inputs are that output times each amount in the technology's `inputs()` (fixed proportions). A technology with no inputs runs on nothing.

- **No order among a business's factories.** Nothing a business sees depends on which of its factories has the lower ID. The factories run as if at the same moment.
- **A shortage is shared, not first-come-first-served.** The needs of every running factory are summed for each good. For each good the factories together need more of than the stockpile holds, the proportion is `held / needed`. Each factory is cut by the **smallest proportion over the goods its recipe names**, so its actual run fraction is `asked × min(proportion)`. This is done in one pass. When a factory is cut for lack of one good, some of another good may go unused that a different factory could have taken. The engine doesn't go back to redistribute it, and the leftover stays in the stockpile.
- **Inputs leave first, output arrives after.** Every factory's inputs are removed from the stockpile (soonest-perishing groups first, as every `Stockpile::remove` does). Then every factory's output is added, as goods of the current turn that perish by the output good's own rule (`PerishesAt::for_good`). So this turn's output can't feed another of the business's factories in the same turn. A good that perishes after one turn is useless as factory output, because it is gone before anyone can use it. That is a mistake by the setting's author, and the engine doesn't police it.

Running doesn't change any condition. Wear does.

### Wear

Every factory loses one lifetime-slice of condition per turn, whether or not it ran, stopping at zero. A factory with less than a slice left loses what it has.

**Rounding never changes a factory's life.** A full factory that is never maintained runs on exactly t₁ turns, then makes nothing. That holds even for lifetimes whose slice isn't exact in floating point, such as 3 or 24. Wear does its arithmetic in whole slices. It converts the condition to `condition × lifetime` and snaps that to the nearest whole number when it is within `TOLERANCE` (1e-9). It subtracts at most one, then converts back to a condition, snapping to exactly 0 or 1 within `TOLERANCE`. An unmaintained condition therefore stays an exact whole number of slices (from 1: 2/3, then 1/3, then exactly 0), and no speck of leftover condition can buy an extra turn of output. Running uses the same snapped slice count to decide whether a factory holds a slice.

### Maintenance

Maintenance is construction run for a fraction of its time. Replenishing a fraction f costs **f of the capital bundle plus f of the construction bundle** (one per-unit bundle, `capital().merged(construction())`, scaled by f). The capital goods become the factory's capital, the construction goods are consumed, and the condition rises by f. The fraction asked for is reduced in this order:

1. to at most `1 / construction_turns`, the most one turn of construction can do;
2. to at most what brings the factory to full condition (`1 − condition`, snapped);
3. by the goods on hand. This uses the same proportional sharing as running, across the business's factories, over the merged per-unit bundle: each factory's fraction is multiplied by the smallest `held / needed` proportion over the goods that bundle names.

The goods leave the stockpile soonest-perishing first, after every factory's fraction has been decided. The new condition is snapped to 0 or 1 within `TOLERANCE`.

A shut-down factory can be maintained like any other. An exhausted factory comes back the way construction builds one, slice by slice, and runs again once it holds a whole slice. **If a technology's construction takes longer than its lifetime (t > t₁), a factory of it can't be revived from zero.** Maintenance adds at most 1/t per turn and wear takes back up to 1/t₁, so the factory never reaches a whole slice. It stays exhausted until scrapping or rebuilding exists (Stage 07). A business may leave a factory at zero indefinitely as a free option on reviving it. That is accepted.

### Order within a turn

For each business the order is **run, then wear, then maintain**. `operate` fixes this order, and it is what makes the numbers mean what [technologies.md](technologies.md#what-the-numbers-mean) says:

- Running before wear lets a full, never-maintained factory run exactly t₁ turns. With wear first it would get one turn fewer.
- Maintaining after wear lets a business that replenishes one slice each turn hold a factory at full condition. Maintaining first would be capped at full and then lose a slice.
- Running takes its inputs from the stockpile before maintenance takes its goods. When a good is both an input and a maintenance good, running gets it first.

### A safety net, not the normal path

The cuts for missing goods keep the world consistent whatever a decider sends. A business isn't supposed to send orders its goods can't cover: the arithmetic rule in Step 06 asks only for what it can supply. That is why the sharing rules stay simple, and why every cut shows up in the record as asked above done instead of being absorbed silently.

## Data model

```rust
pub struct Operation<'a> {
    pub technology: &'a Technology,
    pub condition: f64,             // in, and out after wear / maintain / operate
    pub run_fraction: f64,          // 0 to 1
    pub maintenance_fraction: f64,  // 0 to 1
}
pub struct RunRecord { pub asked: f64, pub done: f64, pub inputs: Bundle, pub output: Quantity }
pub struct MaintenanceRecord { pub asked: f64, pub done: f64, pub cost: Bundle }
pub struct FactoryRecord {
    pub condition_before: f64,
    pub run: RunRecord,
    pub wear: f64,
    pub maintenance: MaintenanceRecord,
    pub condition_after: f64,
}
```

- `Operation` is one factory as the mechanics see it. It is `Copy`, and it borrows the technology from the rules. The functions take a slice of operations, one business's factories. Each record list comes back in the same order as the slice.
- `RunRecord`: `asked` is the run fraction asked for. `done` is the fraction actually run: `asked`, or less after a shared cut, or 0 for a factory below one slice. `inputs` is the goods taken from the stockpile, with zero amounts dropped (empty when nothing ran). `output` is `capacity × done`.
- `MaintenanceRecord`: `asked` is the maintenance fraction asked for. `done` is what remains after the construction-slice cap, the full-condition cap and the goods on hand. `cost` is `done` times the capital and construction bundles together, with zero amounts dropped.
- `FactoryRecord` is one turn for one factory, in the order things happened. `wear` is the condition lost, which is one slice, or what was left if that was less. `condition_after` is what the caller writes back to the factory.
- **A cut is visible as `done < asked`** in either record. Step 08 keeps these records as the business's per-turn record, and Step 09 shows them.

## Functions

| Function | Does | Returns |
|---|---|---|
| `run(operations, &mut stockpile, &goods, turn)` | Runs the factories together as described in [Running](#running). Conditions don't change | `Vec<RunRecord>` |
| `wear(&mut operations)` | Takes one slice from each condition, stopping at zero | `Vec<f64>`, the condition each lost |
| `maintain(&mut operations, &mut stockpile)` | Maintains the factories together as described in [Maintenance](#maintenance) and raises each condition | `Vec<MaintenanceRecord>` |
| `operate(&mut operations, &mut stockpile, &goods, turn)` | `run`, then `wear`, then `maintain` | `Vec<FactoryRecord>` |

`goods` is needed only for the output's perishing rule, and `turn` is the turn the output is added in. The records don't depend on the order of the slice. Running the same operations reversed gives the same records, reversed, and the same stockpile.

### Checks

Each function first asserts that every condition is a finite number from 0 to 1. `run` also checks every run fraction, and `maintain` every maintenance fraction. A failure panics with `a factory's condition must be a finite number from 0 to 1`, `a run fraction must be a finite number from 0 to 1` or `a maintenance fraction must be a finite number from 0 to 1`. The loader guarantees valid conditions, and Step 05's validation guarantees valid fractions, so a panic is a bug in the caller.

### Floating point and removal

Each shared proportion is computed from stockpile totals, and the goods are removed afterwards. A factory's share of a scarce good can therefore come out a rounding speck larger than what is left. The private `take` helper removes what the stockpile holds in that case. A debug assertion checks that the difference really is rounding.

## Worked example

The sample's `farm` has capacity 12, inputs of 1 `unimproved_grain` and 0.5 `unskilled_labor` per unit, a capital of 4 `tools`, a construction of 8 `unskilled_labor`, t = 2 and t₁ = 8. It starts at condition 1 and is asked for run 1 and maintenance 0.125, with enough goods on hand:

1. **Run:** it holds 8 slices, so it runs. Output is 12 `grain`, using 12 `unimproved_grain` and 6 `unskilled_labor`.
2. **Wear:** it loses 1/8, so the condition becomes 0.875.
3. **Maintain:** the 0.125 asked is within the cap of 1/t = 0.5 and the room of 0.125. It costs 0.125 × (4 `tools` + 8 `unskilled_labor`) = 0.5 `tools` and 1 `unskilled_labor`, and the condition is back to exactly 1.

If it were asked for maintenance 1 instead, the full-condition cap would still give `done` 0.125, and the record would show `asked` 1 and `done` 0.125. As a shared-shortage example, take two factories of one technology that each need 10 labor at run 1, with only 5 labor held. Both get `done` 0.25, whatever their order.

## Using the mechanics

```rust
let goods = &world.rules().goods;
let owner = world.businesses().id("center_farm")?;
let mut ops: Vec<Operation> = world.factories().owned_by(owner).map(|id| {
    let factory = world.factories().get(id);
    Operation {
        technology: world.rules().technologies.get(factory.technology),
        condition: factory.condition,
        run_fraction: 1.0,
        maintenance_fraction: 0.125,
    }
}).collect();
let mut stockpile = world.business_stockpile(owner).clone();
let records = operate(&mut ops, &mut stockpile, goods, world.turn());
// records[i].condition_after (= ops[i].condition) is factory i's new condition
```

The World doesn't yet expose a way to write the stockpile or the conditions back. Step 08 does that inside the turn.

## Extending

- **A new cap on maintenance or running**, such as a cash limit: apply it where the wanted fraction is computed, before the shared proportions, so the sharing still sees the real need.
- **A new field in a record**: add it to the record struct and fill it in the function that knows it. `operate` assembles `FactoryRecord` from the three parts.
- **Condition affecting capacity**, or any other effect of condition beyond running or not, is out of scope. It would change the running rule above, the life-length guarantee, and the tests that pin both.

## Not yet

Calling the mechanics in the turn, keeping the per-business records in the World, and anything in the turn report are Step 08. Choosing the fractions is Steps 05 to 07 (Step 06's rule never asks for more than it can cover). Showing the records in the dev UI is Step 09. Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## Tests

- `production.rs` unit tests, on hand-built goods and technologies (a smelter-like recipe and no-input technologies):
  - running at capacity × fraction;
  - output perishing by its good's rule from the current turn;
  - inputs taken soonest-perishing first;
  - output delivered only after every factory has run;
  - a technology with no inputs;
  - a shortage shared proportionally whatever the order;
  - a factory short of two goods cut by the worse shortage, with the leftover staying;
  - wear of one slice, run or not, stopping at zero;
  - a factory below one slice making nothing, and 1.5 slices running once;
  - a full factory running exactly its lifetime for lifetimes 1, 2, 3, 7, 24 and 100;
  - an unmaintained condition staying a whole number of slices;
  - maintenance costing both bundles and raising the condition;
  - the construction-slice and full-condition caps;
  - a maintenance shortage shared by the worst good;
  - a shut-down factory coming back slice by slice (and a t > t₁ factory never coming back);
  - steady one-slice maintenance holding a factory at exactly 1;
  - running taking its inputs before maintenance takes its goods;
  - the record showing asked above done on a cut;
  - the panics for out-of-range fractions and conditions.
- `bundle.rs` unit tests for `scaled` and `merged` (see [technologies.md](technologies.md#bundle)).
- `crates/engine/tests/production.rs`, through the public API only:
  - the sample's three factories, operated at turn 0, where no business holds labor, so nothing runs or is maintained and only wear applies;
  - `production_valid`'s `works` business, where three factories share a scarce input, are cut by the worse good and keep the leftover;
  - the same run with the factories reversed, giving identical results;
  - maintenance costing both bundles, capped, and sharing a shortage;
  - `springs`, where a full factory runs its lifetime, one and a half slices run once, and output perish groups follow the turn.
- Test setting `production_valid`: goods `coin`, `labor`, `ore`, `iron`, `tools` and `bread`. Technologies `smelter`, `bakery` and `spring`, with different lifetimes and construction times. Two businesses, `works` and `springs`, with five factories at conditions 1 and 0.5, and stockpiles chosen so that every shortage and cap happens. `crates/engine/tests/settings/README.md` lists its exact values.

## docs/README.md — modify
- **Location:** the document table, new row after `factories.md` (the row Step 03 adds)
- **Change:** add

| [production.md](production.md) | Factory mechanics as pure functions over one business's factories: the run and maintenance fractions, running at a fraction of capacity with shortages shared proportionally, shutdown below one lifetime-slice, wear with exact factory lifetimes, maintenance and its caps, the run-wear-maintain order, and the per-factory records (`Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord`, `operate`) |

## docs/technologies.md — modify
- **Location:** introduction, the paragraph beginning "Nothing runs technologies yet."
- **Change:** replace

Factories (see [factories.md](factories.md)) run technologies. The factory mechanics in [production.md](production.md) implement the rules in [What the numbers mean](#what-the-numbers-mean) as pure functions over one business's factories. Step 08 of Stage 03 wires them into the turn, so nothing in a running world uses a technology yet.

## docs/technologies.md — modify
- **Location:** "What the numbers mean", the "Depreciation." bullet
- **Change:** replace

- **Depreciation.** Every turn, whether or not it ran, a factory consumes 1/t₁ of its technology's capital bundle (one **lifetime-slice**), in the bundle's proportions, stopping at zero. A factory that starts full and is never maintained runs exactly t₁ turns. A factory runs only while it holds at least one whole slice, so every turn of output costs exactly one slice. Below that it is shut down and makes nothing, and a partial slice is lost to wear. Condition doesn't slow a factory: it runs at full capacity while it holds a slice. See [production.md](production.md).

## docs/technologies.md — modify
- **Location:** "What the numbers mean", the "t > t₁ is allowed." bullet
- **Change:** replace

- **t > t₁ is allowed.** A factory loses 1/t₁ per turn and can regain at most 1/t per turn, so steady maintenance keeps up with depreciation only when t ≤ t₁. A technology with t > t₁ wears out faster than it can be maintained, and once a factory of it is exhausted it can't be revived from zero, because maintenance never brings it up to a whole slice. That is a legitimate thing for a setting to say, so the loader neither rejects nor warns about it.

## docs/technologies.md — modify
- **Location:** "What the numbers mean", new bullet after the "t > t₁ is allowed." bullet
- **Change:** add

- **Order.** Each turn a business's factories run, then wear, then are maintained. Running before wear gives a full factory exactly t₁ turns. Maintaining after wear lets one slice of maintenance per turn hold a factory at full condition. See [production.md](production.md#order-within-a-turn).

## docs/technologies.md — modify
- **Location:** "Data model", "Bundle", the paragraph after the code block, its second sentence ("A technology's inputs, capital and construction are bundles, and later Steps reuse the type for a factory's current capital and for depreciation slices.")
- **Change:** replace

A technology's inputs, capital and construction are bundles. The engine also uses the type for its working lists of goods with amounts, such as the inputs a factory run uses and the goods maintenance costs (see [production.md](production.md)).

## docs/technologies.md — modify
- **Location:** "Data model", "Bundle", the method table, two new rows after the `amount_of` row
- **Change:** add

| `scaled(factor: Quantity) -> Bundle` | Every amount multiplied by `factor` (saturating). The goods stay named even when `factor` is 0 |
| `merged(&other) -> Bundle` | The goods of both bundles in good ID order, with the amounts of a good named in both added (saturating) |

## docs/technologies.md — modify
- **Location:** "Data model", "Bundle", the paragraph after the table ("`Bundle::default()` is the empty bundle. Only construction, lookup and iteration exist; ...")
- **Change:** replace

`Bundle::default()` is the empty bundle. Beyond construction, lookup and iteration, the only arithmetic is `scaled` and `merged`, which the factory mechanics need: the inputs of a run are `inputs().scaled(output)`, and the cost of maintenance is `capital().merged(construction()).scaled(f)`. Further arithmetic is added when a system needs it, following the rule for `Quantity`.

## docs/technologies.md — modify
- **Location:** "Using technologies", the code block, its last line (`let per_turn_wear = ...`)
- **Change:** replace

```rust
let per_turn_wear = farm.capital().scaled(Quantity::new(1.0 / f64::from(farm.lifetime()))?); // one lifetime-slice
let maintenance_per_unit = farm.capital().merged(farm.construction());              // cost of f = this × f
```

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 03 (factories), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the factory mechanics of running, depreciation and maintenance exist as pure functions (see [production.md](production.md)). The view a business is served and its decisions (Step 05), the first decider (Step 06), the decider host (Step 07), business trade with factories running in the turn (Step 08), and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/technologies.md — modify
- **Location:** "The sample setting and tests", the `bundle.rs` unit tests bullet
- **Change:** add, at the end

`scaled` multiplies every amount and keeps zero-scaled goods named; `merged` adds shared goods, keeps the rest sorted and gives the same result either way round; merging with an empty bundle gives a copy.

## docs/factories.md — modify
- **Location:** introduction, the second paragraph ("Nothing runs a factory yet. ...")
- **Change:** replace

The mechanics of running, wear and maintenance exist as pure functions over one business's factories (see [production.md](production.md)), but nothing in the turn calls them yet. Step 05 serves each business its factories in the view, Step 08 wires the mechanics into the turn, and Step 09 shows factories in the dev UI. Until then, factories are loaded, checked and held in the world, unchanged.

## docs/factories.md — modify
- **Location:** "Concepts", the "Condition, not a goods table." bullet, its last sentence ("A setting places a factory with a condition greater than 0 and at most 1 ...")
- **Change:** replace

A setting places a factory with a condition greater than 0 and at most 1 (1 when none is written). The engine type accepts 0 too, because wear can bring a factory down to nothing. A factory runs only while it holds at least one lifetime-slice (a condition of at least 1/t₁). Below that it is shut down until maintenance brings it back (see [production.md](production.md#lifetime-slices-and-shutdown)).

## docs/factories.md — modify
- **Location:** "Using factories", the paragraph after the code block ("The goods in a factory are its technology's capital bundle with every amount multiplied by `condition`; no helper computes them yet ...")
- **Change:** replace

The goods in a factory are its technology's capital bundle with every amount multiplied by `condition`: `technology.capital().scaled(Quantity::new(factory.condition)?)`. To run, wear or maintain factories, build one `production::Operation` per factory a business owns and call `operate` (see [production.md](production.md#using-the-mechanics)).

## docs/factories.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 03 (factories), the section as written in its new file

Running a factory, depreciation, maintenance and shutting down exist as pure functions (see [production.md](production.md)) but aren't called in the turn. The following are later Steps of Stage 03: serving each business its factories in the view (Step 05), the mechanics in the turn (Step 08), and showing factories in the dev UI (Step 09). Factories under construction, building, scrapping, retooling, and buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes. Nothing changes a factory during play yet.

## docs/businesses.md — modify
- **Location:** "Not yet"
- **Change:** replace its first sentence
- **Supersedes:** Step 03 (factories), the same sentence

The factory mechanics a business's factories will follow exist as pure functions (see [production.md](production.md)). The following are later Steps of Stage 03: the view a business is served and its decisions (Step 05), the decider and resolving decider names (Steps 06 and 07), business trade, running its factories in the turn and the use of its cash (Step 08), and showing businesses and their factories in the dev UI (Step 09).

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 03 (factories), the same section

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, businesses acting in the "actors act" part of the turn (they exist and hold goods, but decide and trade nothing until later Steps of Stage 03), and factories running, wearing or being maintained in the turn (the mechanics exist as pure functions, see [production.md](production.md), and Step 08 of Stage 03 calls them; until then factories are held as state, unchanged).

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries
- **Change:** add one line after `src/pops.rs`, before `src/quantity.rs`

```
    src/production.rs   factory mechanics: run, wear, maintain, operate (see production.md)
```

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry
- **Change:** replace
- **Supersedes:** Step 03 (factories), the same entry

```
    tests/businesses.rs, currency.rs, demand.rs, factories.rs, goods.rs, growth.rs, map.rs,
          market.rs, pops.rs, production.rs, resources.rs, setting_loader.rs,
          technologies.rs, trade.rs, world.rs
```

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", second bullet, the list of test settings
- **Change:** add `production_valid` to the list, after `factories_errors` (which Step 03 adds)
