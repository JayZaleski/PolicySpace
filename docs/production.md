# Factory mechanics

Source: `crates/engine/src/production.rs`, plus `Bundle::scaled` and `Bundle::merged` in `crates/engine/src/bundle.rs`. The engine crate root re-exports `Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord`, the function `operate` and the slice rule `slices_held`. The three parts `run`, `wear` and `maintain` are public in the `production` module (`policyspace_engine::production::run`, and so on).

This module gives factories (see [factories.md](factories.md)) their physics. It covers what happens when a business runs a factory at some fraction of its capacity, how a factory wears each turn, and what maintaining one costs. Its rules implement the meaning of a technology's numbers (see [technologies.md](technologies.md#what-the-numbers-mean)). Like `demand` (see [demand.md](demand.md)), these are **pure functions**: they work on one business's factories, their technologies, that business's stockpile and its instructions. They read no world state and write none.

**The turn calls `operate` once per business**, right after its hex's trade round, with the fractions its valid decision gives: 0 and 0 for a factory the decision doesn't mention, and for every factory of a business that decided nothing or was refused, whose factories then only wear. The World writes each condition back and keeps the records in the business's outcome (see [trade.md](trade.md#operating-the-factories)).

`production.rs` depends on `bundle`, `goods`, `quantity`, `stockpile`, `technologies` and `turn`. It knows nothing about the `World`, `Factories`, businesses or files. The caller moves a business's stockpile out, builds one `Operation` per factory, calls the functions, and writes each factory's condition back.

## The model

### Two instructions per factory

Each turn a business gives every factory it owns two numbers, each from 0 to 1 inclusive:

- **Run fraction:** the share of the technology's capacity to use this turn. This is a fraction of capacity, not an amount of output. It is the same kind of number as the condition and the maintenance fraction, and it keeps its meaning if the technology or capacity changes.
- **Maintenance fraction:** the share of the technology's full capital bundle to replenish this turn.

The functions assume both numbers are in range and panic if they aren't (see [Checks](#checks)). A decision's fractions are checked to be finite and from 0 to 1 when it is read from JSON and again when it is validated against its view (see [decider.md](decider.md#validation)), so they arrive in range.

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

**Rounding never changes a factory's life.** A full factory that is never maintained runs on exactly t₁ turns, then makes nothing. That holds even for lifetimes whose slice isn't exact in floating point, such as 3 or 24. Wear does its arithmetic in whole slices. It converts the condition to `condition × lifetime` and snaps that to the nearest whole number when it is within `TOLERANCE` (1e-9). It subtracts at most one, then converts back to a condition, snapping to exactly 0 or 1 within `TOLERANCE`. An unmaintained condition therefore stays an exact whole number of slices (from 1: 2/3, then 1/3, then exactly 0), and no speck of leftover condition can buy an extra turn of output. Running uses the same snapped slice count to decide whether a factory holds a slice. The rule is public as `slices_held(condition, lifetime) -> f64`: `condition × lifetime`, snapped to the nearest whole number within `TOLERANCE`. A factory runs only when it is at least 1, and its `floor` is the number of further turns the factory can run with no maintenance. Callers outside the engine (the dev UI's remaining-life column) use it instead of copying the arithmetic, so they always agree with the engine about whether a factory runs.

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

The cuts for missing goods keep the world consistent whatever a decider sends. A business isn't supposed to send orders its goods can't cover: the arithmetic rule bids for the goods its plan needs and scales the plan down to the cash it will have (see [python.md](python.md#the-cash-budget)). That is why the sharing rules stay simple, and why every cut shows up in the record as asked above done instead of being absorbed silently.

The engine's cash cap relies on these cuts. When a business's bids cost more than its cash, the trade round scales every bid by one proportion and leaves the fractions alone; the factories then find too few goods, and the shared cuts here reduce the run and maintenance done (see [trade.md](trade.md#the-engine-never-touches-the-fractions)). For a business whose only short good is the one the cap cut, its run is cut by the cap's proportion.

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
- **A cut is visible as `done < asked`** in either record. The World keeps each business's records, in factory ID order, in its `BusinessOutcome` (`World::business_outcome(business).factories()`), and the turn report totals their output as `produced` and their inputs and maintenance cost as `used` (see [world.md](world.md#turnreport)). The dev UI shows them per business, one row per factory, with asked beside done (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## Functions

| Function | Does | Returns |
|---|---|---|
| `run(operations, &mut stockpile, &goods, turn)` | Runs the factories together as described in [Running](#running). Conditions don't change | `Vec<RunRecord>` |
| `wear(&mut operations)` | Takes one slice from each condition, stopping at zero | `Vec<f64>`, the condition each lost |
| `maintain(&mut operations, &mut stockpile)` | Maintains the factories together as described in [Maintenance](#maintenance) and raises each condition | `Vec<MaintenanceRecord>` |
| `operate(&mut operations, &mut stockpile, &goods, turn)` | `run`, then `wear`, then `maintain` | `Vec<FactoryRecord>` |
| `slices_held(condition, lifetime)` | Counts the lifetime-slices a condition holds, snapped within `TOLERANCE` (see [Wear](#wear)). `run` uses it to decide eligibility and `wear` to take a slice | `f64` |

`goods` is needed only for the output's perishing rule, and `turn` is the turn the output is added in. The records don't depend on the order of the slice. Running the same operations reversed gives the same records, reversed, and the same stockpile.

### Checks

Each function first asserts that every condition is a finite number from 0 to 1. `run` also checks every run fraction, and `maintain` every maintenance fraction. A failure panics with `a factory's condition must be a finite number from 0 to 1`, `a run fraction must be a finite number from 0 to 1` or `a maintenance fraction must be a finite number from 0 to 1`. The loader guarantees valid conditions, and the decision's validation guarantees valid fractions, so a panic is a bug in the caller.

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

The World runs the same call inside the turn (`World::operate_business`) and writes the stockpile and conditions back; from outside, the example works on a clone and changes nothing. Note that `factory.condition` read between steps is already after this turn's operation, so this example operates the factories a second time from that state. To read what the turn itself did, use `world.business_outcome(owner).factories()`.

## Extending

- **A new cap on maintenance or running**, such as a cash limit: apply it where the wanted fraction is computed, before the shared proportions, so the sharing still sees the real need.
- **A new field in a record**: add it to the record struct and fill it in the function that knows it. `operate` assembles `FactoryRecord` from the three parts.
- **Condition affecting capacity**, or any other effect of condition beyond running or not, is out of scope. It would change the running rule above, the life-length guarantee, and the tests that pin both.

## Not yet

The fractions come from a business's `Decision` (see [decider.md](decider.md)), which every business's decider sends at the decision point (see [decider-host.md](decider-host.md)); the first decider, the arithmetic rule, chooses them in Python with the same slice arithmetic as this module (see [python.md](python.md#orders-per-factory)). Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

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
  - `slices_held` snapping within the tolerance: 8 for (1, 8), exactly 1 for (0.125, 8), below 1 for a hair under 0.125, 0 for (0, 8), exactly 2 for (2/3, 3) and exactly 24 for (1 − 1e-12, 24);
  - maintenance costing both bundles and raising the condition;
  - the construction-slice and full-condition caps;
  - a maintenance shortage shared by the worst good;
  - a shut-down factory coming back slice by slice (and a t > t₁ factory never coming back);
  - steady one-slice maintenance holding a factory at exactly 1;
  - running taking its inputs before maintenance takes its goods;
  - the record showing asked above done on a cut;
  - the panics for out-of-range fractions and conditions.
- `bundle.rs` unit tests for `scaled` and `merged` (see [technologies.md](technologies.md#bundle)).
- `crates/engine/tests/production.rs`, through the public API only. Each `Operation` takes its condition from a separately loaded `LoadedSetting` (the condition the setting places the factory at), not from the world, because a world is already past turn 0's operation when `World::new` returns; the technologies and stockpiles come from the world:
  - the sample's three factories, operated at turn 0, where no business holds labor, so nothing runs or is maintained and only wear applies; the world's own conditions are untouched by the call;
  - `production_valid`'s `works` business, where three factories share a scarce input, are cut by the worse good and keep the leftover;
  - the same run with the factories reversed, giving identical results;
  - maintenance costing both bundles, capped, and sharing a shortage;
  - `springs`, where a full factory runs its lifetime, one and a half slices run once, and output perish groups follow the turn.

  The mechanics in the turn are tested in `crates/engine/tests/business_trade.rs` (see [trade.md](trade.md#tests)).
- Test setting `production_valid`: goods `coin`, `labor`, `ore`, `iron`, `tools` and `bread`. Technologies `smelter`, `bakery` and `spring`, with different lifetimes and construction times. Two businesses, `works` and `springs` (both naming the decider `rule`), with five factories at conditions 1 and 0.5, and stockpiles chosen so that every shortage and cap happens. `crates/engine/tests/settings/README.md` lists its exact values.
