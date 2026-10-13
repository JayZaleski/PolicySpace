# Step 06: turn-clock

## Summary
This Step makes the world tick. Until now a loaded setting has only described the world: its rules (goods, resource types) and what turn 0 looks like (hexes, deposits). This Step builds a running **world** from a loaded setting and adds the **turn step**. Each turn, goods that reach their `perishes_after` limit perish, the clock advances, and every deposit emits its resource type's output of its good. Emitted goods go into a **stockpile of unowned goods on the deposit's hex**. The stockpile is a general type that later Stages reuse for POPs and industries, and in Stage 02 an owned deposit's output goes to its owner instead. Each step returns a short report of what perished and what was emitted, and the dev UI's setting checker can run a number of turns and print those reports, so the turn can be seen working before the dev UI arrives in Steps 07–08.

## Behavior

### The world
- A world is built from a successfully loaded setting. The setting's rules (goods, resource types) stay fixed for the whole game. The world holds what changes: the clock and the stockpiles.
- Hexes and deposits come from the starting state as loaded. They don't change in this Step, but they belong to the world because later Stages will change them (for example, deposit ownership in Stage 02).
- The setting's `start/` folder describes the world before anything happens. There are no starting stockpiles in `start/` yet. Initial holdings arrive with owners in Stage 02.

### Stockpiles
- A stockpile holds amounts of goods. Amounts are `Quantity` values: continuous, finite, and never negative.
- A stockpile records not only how much of each good it holds but **when that good perishes**. A good's amount is split into groups by the turn at whose end it perishes:
  - A good with `perishes_after = N` that is added in turn T perishes at the end of turn T+N−1. With `perishes_after = 1`, output emitted in turn 5 can be used during turn 5 and is gone when turn 5 ends. With `perishes_after = 3`, output emitted in turn 5 is gone at the end of turn 7. This is the meaning fixed in Step 03.
  - Amounts of the same good that perish at the end of the same turn are merged into one group. Several deposits on a hex emitting the same good in the same turn add to one group.
  - A durable good (no `perishes_after`) is kept as one amount that never perishes.
- A stockpile can report, for each good it holds, the total amount and the breakdown by the turn at whose end each part perishes.
- A stockpile lists only goods it holds a positive amount of. An empty stockpile lists nothing.
- The stockpile is a general type, not something specific to hexes. Stage 02 will reuse it for what POPs own, and Stage 03 for industries.
- In this Step goods only enter a stockpile (by emission) and leave it (by perishing). Taking goods out to use them comes with Stage 02, which will also decide which units are taken first.

### Where emitted goods go
- Each hex has one stockpile of **unowned goods**. Every deposit emits into the stockpile of its own hex.
- This is the seam for Stage 02: once deposits have owners, an owned deposit's output goes to its owner instead. The per-hex stockpile then either remains, as the place for goods nobody owns, or is removed. Stage 02 decides.

### The turn
- The clock counts turns as whole numbers, starting at 0.
- A turn has three parts:
  1. **Start of turn:** every deposit emits its resource type's `output` of its good into its hex's stockpile. A deposit always emits its full output. There is no depletion yet.
  2. **Actors act:** nothing yet. Later Stages add POPs, industries, and markets here.
  3. **End of turn:** every amount whose group perishes at the end of this turn is removed from every stockpile.
- **Creating a world** puts it at turn 0 and runs turn 0's start (emission) immediately.
- **A step** ends the current turn (perishing), advances the clock by one, and starts the next turn (emission).
- So between steps the world always sits at the point where actors will act: this turn's output is in the stockpiles, and nothing has perished yet this turn. This is the state that tests, the setting checker, and the dev UI observe. Without this, every good that perishes after one turn would always look empty from outside.
- Deposits emit in deposit ID order, and perishing is applied in a fixed order, so the same setting stepped the same number of times always gives an identical world and identical reports.
- The clock never wraps around. If a step would advance the clock past the largest turn number the engine can count, the step is refused with a clear error and the world is left unchanged. In practice this is unreachable.

### Turn report
- Creating a world and every step each return a **report** with two lists:
  - **perished:** the total of each good removed by perishing at the end of the previous turn (always empty for turn 0, since no turn came before it);
  - **emitted:** the total of each good emitted at the start of the new turn.
- Each list gives one total per good, summed over every hex, in good ID order. It lists only goods with an amount greater than zero.
- The report doesn't break totals down by hex. What each hex holds is visible in its stockpile.

Sample setting, where all resource output perishes after one turn: the 4 `copper_ore` deposits (output 2.5) emit 10 `unimproved_copper`, and the 6 `arable_land` deposits (output 4) emit 24 `unimproved_grain`. Turn 0's report is "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`". Every later step's report is "perished 10 `unimproved_copper` and 24 `unimproved_grain`; emitted the same".

A good with `perishes_after = 3` emitted at 5 per turn builds up: 5 in turn 0, 10 in turn 1, 15 in turn 2. From turn 3 on it stays at 15, with 5 perishing and 5 emitted every step.

### What the engine provides
Through the engine's public interface, a caller (tests, the setting checker, and the dev UI in Steps 07–08) can:
- create a world from a loaded setting, getting the world and turn 0's report;
- step the world, getting the step's report (or the error if the clock can't advance);
- read the current turn number;
- reach the rules, the hexes, and the deposits;
- read any hex's stockpile: the goods it holds, each total, and each total's breakdown by the turn it perishes.

### The setting checker
- The dev UI's setting checker takes an optional `--turns N` after the setting folder: `dev-ui <SETTING_FOLDER> --turns N`, where N is a whole number from 0 to 10,000.
- With `--turns`, after the usual success line it creates a world, prints turn 0's line, then steps N times and prints one line per step:
  ```
  Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits
  Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain
  Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain
  ```
  - Each line names the turn just started. Perished comes before emitted, matching the order within a step.
  - Goods are listed in good ID order, each as its amount and then its name, separated by commas. An empty list prints `nothing`.
  - `--turns 0` prints only turn 0's line.
- `--turns` without a number, a number outside 0–10,000, a non-number, or any other extra argument is a usage error: the usage message is printed and the exit code is 2. The usage message shows the new option.
- A setting with errors behaves as before (errors printed, exit code 1). No turns are run.
- Without `--turns`, the output is unchanged.
- This option is deliberately minimal: Step 07 replaces the checker with a real developer UI.

### Test settings
Small test settings cover the turn:
- a resource that produces a good with `perishes_after = 3`, so its stock builds up over three turns and then levels off;
- a durable good produced by a resource, so its stock grows every turn and never perishes;
- several deposits of one or more types on one hex, emitting the same good into one group;
- deposits on several hexes, each emitting only into its own hex's stockpile;
- a setting with no deposits, where every stockpile stays empty and every report lists nothing.

## Player-facing Changes
None for players. A developer can create a world from a setting and step it through the engine's interface, and can run `dev-ui <setting> --turns N` to watch the clock advance, deposits emit, and goods perish.

## Interactions
- **Step 03 (goods-and-properties):** `perishes_after` now takes effect, with the meaning fixed there. `movable` still has no effect.
- **Step 04 (hex-map):** each hex gets a stockpile of unowned goods.
- **Step 05 (natural-resources):** each deposit emits its resource type's `output` of its `produces` good every turn. `Quantity` gains the arithmetic this needs.
- **Step 02 (setting-loader):** the setting checker gains `--turns N`. Loading itself is unchanged.
- **Steps 07–08 (dev UI):** build on the world. Step 08 shows the hex stockpiles (with their perish breakdown) and the turn reports, and steps turns.
- **Stage 02 (populate-the-world):** deposits get owners, and an owned deposit's output goes to its owner instead of the hex stockpile. POPs reuse the stockpile type, and taking goods out of a stockpile (consumption, using labor) is added there. Generating labor and acting both fit into the turn's parts.

## Out of Scope
- Taking goods out of a stockpile to use them, and the rule for which units go first (Stage 02).
- Ownership of deposits or goods (Stage 02).
- Starting stockpiles in `start/`.
- Stock, depletion, and regeneration of deposits.
- Any effect of `movable`, and moving goods between hexes.
- Per-hex breakdowns in the turn report.
- Saving and loading a world.
- Any actor behavior in the "actors act" part of the turn.

## Acceptance Criteria
- Creating a world from the sample setting gives turn 0. Each hex's stockpile holds exactly the output of its own deposits: for example, (0, 0) holds 12 `unimproved_grain` (3 × 4) and 2.5 `unimproved_copper`, and a hex with no deposits holds nothing. Turn 0's report is: perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`.
- Stepping the sample world gives turn 1 with the same stockpiles. The report shows 10 `unimproved_copper` and 24 `unimproved_grain` both perished and emitted. This repeats every step.
- A good with `perishes_after = 3` emitted at a fixed amount per turn holds 1×, 2×, and 3× that amount in turns 0, 1, and 2, and stays at 3× from then on. Its breakdown shows one group perishing at the end of each of the next three turns. From turn 3 on, the report shows that amount perished each step.
- A durable good produced by a resource grows by its output every turn, never appears under perished, and is shown as never perishing.
- Several deposits on one hex that emit the same good add to a single group. Deposits on different hexes never emit into each other's stockpiles.
- A setting with no deposits creates a world whose stockpiles are all empty, and every report lists nothing.
- Two worlds created from the same setting and stepped the same number of times are identical, and so are their reports.
- A step at the clock's maximum is refused with an error, and the world is unchanged.
- `dev-ui settings/sample --turns 2` prints the success line, then the lines for turns 0, 1, and 2 in the format above, and exits successfully. `--turns 0` prints only turn 0's line. A setting with no deposits prints `perished nothing; emitted nothing`.
- `--turns` with no number, a number outside 0–10,000, or a non-number, and any unexpected extra argument, print the usage message and exit with code 2. A broken setting with `--turns` prints its errors and exits with code 1. Without `--turns`, the output is as before.
- The single check command passes.
