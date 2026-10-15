# Step 07: pop-growth

## Summary
Each turn, after it has traded and consumed, a POP's size N grows or shrinks by a rate that depends on how far above or below subsistence it is. The measure is a dimensionless **subsistence index** u built from the numbers Step 05's demand already reports: zero at exactly subsistence, positive above, negative (down to −1) below. The rate is a function of u whose parameters live on the POP type: a polynomial above subsistence, so growth can rise, peak and fall off as prosperity rises, and a linear decline below. Next turn's endowment follows the new N automatically. Each POP's outcome records its index, rate and starting size, and the turn report gains the total population. The sample's two types gain growth parameters.

## Behavior

**The subsistence index.** Every POP's index u is computed from the demand the market recorded for it this turn (Step 05's income M, subsistence cost C, and per good the demanded quantity and the subsistence need, which is the type's subsistence amount times N).

- **Above subsistence** (no utility good is below subsistence, as Step 05 reports it): u = (M − C) / C, the number of extra subsistence bundles the POP's income could buy. At the sample's prices a peasant has u = 2/3 and an artisan u = 1. If C is zero, there is no prosperity to measure, and u is 0: a type whose subsistence goods all cost nothing here (or has none) neither grows nor shrinks.
- **Below subsistence** (at least one utility good is below subsistence): for each utility good with a positive subsistence, its **attainment** is the demanded quantity divided by the need. u is the smallest attainment minus 1, so u lies in [−1, 0). Step 05 scales every tradable good by M/C when income falls short, so for a POP that is simply poor this is M/C − 1; for a POP that can't get a subsistence good at all, such as the sample's north-east peasant with no market for grain, the attainment of that good is 0 and u is −1, however much it holds of anything else.

The two sides meet at 0: a POP whose income exactly covers its subsistence bundle has u = 0 from either formula. u is per individual (M, C and the needs all scale with N), so the index doesn't depend on the POP's size.

In this Stage the demanded quantities and the consumed quantities are the same, so the index is a measure of what the POP actually consumed. Stage 05, where rationing can leave a POP short of what it demanded, revisits which numbers the index reads.

**The growth rate.** The POP type carries a `growth` sub-table with three required keys:

- `growth`: a list of one or more polynomial coefficients c₁, c₂, …, each a finite number. Above subsistence the rate is c₁u + c₂u² + …; there is no constant term, so the rate is zero at subsistence. The coefficients may give any shape the setting author wants, including a rate that falls as u rises or turns negative for very rich POPs: `[0.06, -0.03]` peaks at u = 1 with 3% a turn and crosses zero at u = 2. A single coefficient `[0.03]` is plain linear growth.
- `decline`: a finite number from 0 up to but not including 1. Below subsistence the rate is `decline` × u, so a POP with nothing (u = −1) shrinks by exactly `decline` per turn, and a POP at half its subsistence shrinks by half that. Keeping this side linear means the polynomial never has to behave on the negative side, where a general polynomial could flip sign.
- `max_growth`: a finite number of at least 0, the cap on growth.

On both sides the rate is clamped to the range [−`decline`, `max_growth`]. So a polynomial that goes negative can never shrink a POP faster than `decline` does, and the cap holds whatever the coefficients are. Because `decline` is below 1, the rate is always above −1 and N stays positive forever: a starving POP shrinks geometrically but never reaches zero, and the engine never removes a POP. A `decline` of 0 and `growth` of `[0]` give a type that never changes size, which the test settings use where growth would disturb what they already check.

**Applying the rate.** N becomes N × (1 + rate). N stays continuous; nothing is rounded. The new size is what the next turn's endowment uses, and it is what Step 05's demand sees next turn, so a growing POP earns and consumes more each turn.

**Where in the turn.** Growth runs in part 2 ("actors act"), right after the trade round and consumption on every hex, for every POP in POP ID order. Creating a world runs turn 0's start, turn 0's trade round and then turn 0's growth, so a fresh world's POPs already have their grown size; stepping ends the turn, advances the clock, and runs the next turn's start, trade round and growth. Between calls the world is still "after part 2": every POP has traded, consumed and grown. Part 1 (deposits emit, POPs are endowed) and part 3 (perishing) are unchanged. Growth doesn't depend on the market, so running it after every hex's trade round rather than hex by hex changes nothing; it is done after all trading so the outcomes a caller reads between calls are complete.

**What the World records.** Each POP's turn outcome (Step 06) gains its subsistence index, its growth rate, and the size the turn started with. The POP's current size, readable as before, is the size after growth. The outcome is replaced every turn, as before.

**The turn report** gains a `population` part after `consumed`: the total size of every POP after growth. The checker's turn line and the dev UI's turn log print it at the end: `...; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.45` (the number is rounded here; the line prints the full floating-point value, `419.4466666666666`, since nothing is rounded). A world with no POPs reports a population of 0.

**In the setting folder.** Every `[[pop_type]]` must now have a `growth` sub-table (inline or as a `[pop_type.growth]` table, like `utility`) with the three keys above. **Errors**, each reported at the offending place: a missing `growth`; a `growth` that is not a table; a missing `growth`, `decline` or `max_growth` key inside it; a `growth` list that is not an array, is empty, or contains something that is not a finite number; a `decline` that is not a number, is negative, is 1 or more, or is not finite; a `max_growth` that is not a number, is negative, or is not finite; an unknown key in the sub-table. Every key is read even when another is bad, so one entry with several problems reports all of them in one load, and a type with a bad `growth` is still defined by name for the duplicate check.

**Examples from the sample setting**, with `growth = [0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05` on both types.
- A peasant on a market hex: u = 2/3, rate = 0.06 × 2/3 − 0.03 × 4/9 ≈ 0.0267, so 20 individuals become about 20.53 at turn 0 and about 21.08 after one step.
- The artisan at the center: u = 1, rate = 0.03, so 10 become 10.3 at turn 0.
- The north-east peasant with no market: grain attainment 0, u = −1, rate = −0.1, so 12 become 10.8 at turn 0 and 9.72 after one step. Its endowment, and what it consumes as leisure, shrinks with it.
- A peasant facing grain at 10 (a test): M/C = 10/22, u ≈ −0.545, rate ≈ −0.0545.

**Determinism.** POPs grow in POP ID order after every hex's trade round; the same setting stepped the same number of times gives the same sizes, outcomes and reports.

## Player-facing Changes
A setting author adds a `growth` sub-table to every `[[pop_type]]`. The sample's `rules/pops.toml` gains one on each type, with comments explaining the index, the polynomial, the decline rule and the caps. `check --turns` and the dev UI's turn log show the population at the end of every turn line, so the sample's peasants and artisans can be watched growing a little each turn and the north-east peasant shrinking. Inspecting a POP's index, rate and size in the dev UI is Step 08.

## Interactions
- **POP types (Step 03):** the `[[pop_type]]` entry gains the required `growth` sub-table, read and validated with the same reader pattern as `utility`. The normalized shares and subsistence amounts are unchanged.
- **POPs (Step 04):** the size is what growth changes; the endowment at the start of the next turn reads the new size with no change of its own.
- **Stone-Geary demand (Step 05):** the source of M, C and the per-good demanded and subsistence quantities the index is built from. Unchanged.
- **POP trade (Step 06):** growth runs after the trade round and consumption in part 2; the outcome gains the index, the rate and the starting size; the report gains `population`.
- **World and turn (Stage 01 Step 06):** part 2 gains growth; the report gains one scalar.
- **Dev UI (Step 08):** shows each POP's index, rate and size, and the population in the turn log. **Checker and dev UI turn line:** gain `population`.
- **Stage 05 (market):** revisits which numbers the index reads once rationing can leave a POP short of its demand.
- **Test settings:** every test setting with POP types (`pops_valid`, `pops_errors`, `pops_one`, `pops_placed`, `pops_placed_errors`, `pops_trade`, `resources_broken_file`) gains `growth` sub-tables. Where a test checks sizes or stockpiles over several turns and growth would disturb it, the types get `growth = [0]` and `decline = 0`, so their existing expectations hold apart from the added `; population N` on turn lines. Test settings with no POP types are unchanged except for `; population 0` on their turn lines.

## Out of Scope
- Removing a POP, a minimum size, or any event when a POP becomes very small: N only ever stays positive.
- Rounding N or a whole-number size.
- Growth parameters anywhere but the POP type (no global table, no per-POP override), and parameters that change during play.
- A growth rule that depends on anything but the subsistence index: no age structure, migration, or dependence on other POPs.
- Rationing's effect on the index (Stage 05), and showing growth in the dev UI (Step 08).
- Changes to the checker beyond the turn line, or to the dev UI.

## Acceptance Criteria
- The sample setting loads without errors; both types carry `growth = [0.06, -0.03]`, `decline = 0.1` and `max_growth = 0.05`.
- A fresh sample world has already grown at turn 0: every market-hex peasant POP's outcome has index 2/3 and rate about 0.0267 and its size is its starting size times about 1.0267; the center artisan has index 1, rate 0.03 and size 10.3; the north-east peasant has index −1, rate −0.1 and size 10.8. Each outcome's starting size is the size from the setting. The report's population is the sum of the grown sizes.
- Stepping the sample world endows every POP by its new size, trades and consumes in proportion, and grows again: after one step the north-east peasant's size is 9.72 and a market-hex peasant's is its starting size times about 1.0267 squared. Two worlds from the same setting stepped the same number of times are identical, sizes included.
- In a test, a POP exactly at subsistence (income equal to subsistence cost) has index 0 and rate 0, and its size doesn't change; a POP with grain at 10 has index 10/22 − 1 and rate `decline` times that.
- A POP whose polynomial exceeds `max_growth` grows by exactly `max_growth`; a POP whose polynomial is negative (rich enough to pass the zero crossing) shrinks by the polynomial's value, and never by more than `decline`.
- A type with a zero subsistence cost on its hex and every subsistence good met has index 0 and doesn't change size; a type with a positive subsistence it can't obtain (an untradable, unheld good) has index −1 whatever its income.
- A type with `growth = [0]` and `decline = 0` never changes size, above or below subsistence.
- Each error case produces one error at the expected file, line and column with the expected text: a missing `growth` sub-table, a non-table `growth`, each missing key inside it, a `growth` list that is a number, empty, or holds a string or `inf`, a `decline` that is negative, 1, or `inf`, a `max_growth` that is negative or `inf`, and an unknown key in the sub-table. One sub-table with several problems reports all of them in one load, and a type with a bad `growth` still takes part in the duplicate-name check.
- The report's `population` is the total POP size after growth; `check --turns` prints it after `consumed` on every turn line, 0 for a world with no POPs, and the dev UI's turn log shows the same lines.
- Every existing test passes, with turn-line expectations extended by the added `population` part, the sample's turn 1 and later expectations (endowment, sales, purchases, consumption and the turn lines) following its growth, and test settings' POP types given `growth` sub-tables as Interactions describes.
