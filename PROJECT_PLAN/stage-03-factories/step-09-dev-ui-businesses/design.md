# Step 09: dev-ui-businesses

## Summary
This Step finishes Stage 03's developer UI. The hex panel gains the hex's businesses: for each, its decider and what it answered this turn, its owned deposits, its cash through the round, a table of what it offered, sold, bid and bought, a table of its factories with their condition, remaining life and what they did this turn, the trace its decider sent, and its stockpile. The world panel gains a Technologies table and a Businesses table, and the Goods table gains a column for what businesses hold. Hexes with a business get a map mark beside the deposit and POP marks. With this, everything Steps 01 to 08 built can be watched working from the window. The turn log already shows the business totals, since Step 08 added them to the turn line the window shares with `check --turns`. The engine already makes everything this needs public (each business's stored answer, its outcome with the per-factory records, its stockpile, its factories and deposits), except the rule that turns a condition into a count of lifetime-slices, which the engine makes public for the panel; otherwise only the dev UI changes, the way Stage 02's dev-ui Step changed only the dev UI.

## Behavior

### What "this turn" means in the panels
Between steps the world is after part 2 of the current turn: every business has been served its view, answered, traded and operated its factories, and nothing has perished yet. So for the turn in the status bar the business sections show the answer its decider gave at that turn's decision point, the trade and operation that answer led to, the stockpile after operation (holding this turn's factory output, to be sold next turn, and whatever labor the factories left), and each factory's condition after this turn's wear and maintenance. The roadmap's "last turn's decision" is this: the most recent decision point, which is the current turn's. Stepping replaces every answer and outcome with the next turn's. This matches what the POP sections show.

### Hex panel (right)
The heading, Deposits, Stockpile, Market and POPs sections are unchanged. One section follows them.

**Businesses.** One collapsible section per business on the hex, in business ID order, open by default, headed by the business's name and its current cash in the hex's currency. A hex with no businesses shows `No businesses.` Inside each section, in this order:

1. **Decider and answer:** the decider's name, and the state of this turn's answer: the business decided nothing (no decision was sent, or its decider failed), sent a valid decision, or sent a decision that was refused. A refused answer lists every validation error, each at its place as the engine reports it, in the theme's error color, so a broken decider's mistake is readable where it matters.
2. **Deposits:** the deposits the business owns, one line per resource type, written like the hex's deposit lines (count, output per deposit, total), or a line saying it owns none.
3. **Cash:** one line with the currency held at the start of the round, after the hex's sales and after the purchases, and the proportion the cash cap applied to the bids. The proportion is 1 when the bids were within the cash; a proportion below 1 is drawn in the error color, because it means the decider asked for more than it could pay and its factories were cut through the shortage.
4. **Trade table:** one row per good that was offered, sold, bid or bought, in good ID order, with columns Good, Offered, Sold, Bid, Bought, 0 where nothing happened. The decision and the outcome are merged into one table because they are two halves of one story: a cut shows as asked above done in the same row, the way the factory records show a cut order. A refused decision's rows show what was asked with everything done at 0, so the developer sees what the decider wanted; a business that decided nothing has an empty table, shown as a single line saying nothing was traded.
5. **Factories table:** one row per factory the business owns, in factory ID order, with the factory's ID number (the number the decider addresses it by), its technology, its condition before and after this turn, its remaining life, the run fraction asked and done, the output made, the slice lost to wear, and the maintenance fraction asked and done. The inputs the run used, the goods maintenance cost, and the capital bundle the factory holds are detail text on the row (hover text or indented lines, for the spec to choose), so the table stays readable at the sample's numbers.
   - **Remaining life** is derived from the condition: the number of whole turns the factory can still run with no maintenance, which is the number of lifetime-slices its condition holds. It is computed by calling the engine's own rule for the number of slices a condition holds (the rule its mechanics use to decide whether a factory runs, made public for this), so the panel never says a factory can run when the engine won't run it, or the reverse; a factory holding less than one slice reads `shut down`, in the error color. The condition itself stays in the table because it is the factory's real state and shows partial slices the life count hides.
   - **The capital bundle** is derived the same way: the condition times the technology's capital bundle, since capital only ever changes in the bundle's own proportions (Step 03).
   - The inputs, maintenance cost and capital bundle are hover text on the row's Output, Maintenance done and Condition after cells.
   - A factory the decision didn't mention, and every factory of a business that decided nothing or was refused, shows run and maintenance asked and done at 0, with its wear.
6. **Trace:** the free text the decider sent with its decision, in a collapsible block of monospace text, collapsed by default, because the arithmetic rule writes several lines per business and the tables carry the numbers. Absent when the decision carried no trace or the business decided nothing. The engine stores the trace and never acts on it; the panel shows it as sent.
7. **Stockpile:** the business's stockpile, written exactly as the hex and POP stockpiles are: each good held with its total, and under it each perish group with its countdown, soonest first, or `Empty.` Its currency is its cash.

As everywhere in the dev UI, sizes and amounts are written in full; fractions, conditions, wear and the proportion are the engine's numbers and are not rounded either, except that where the spec rounds a ratio for the table it keeps the full value in hover text, as the POP sections do for the index and rate.

### World panel (left)
Three changes.

**Goods table.** A new **In businesses** column after In POPs: the good's total over every business stockpile. The three columns are kept apart because they are different things: hex stockpiles are unowned deposit output, POP stockpiles are what POPs own, business stockpiles are what businesses own, including their cash and their factories' output. In the sample, owned deposits' output and all the silver businesses hold appear only under this column, which closes the gap Step 02's docs noted.

**Technologies table.** A new collapsible section after POP types, one row per technology in ID order: name, the recipe (one line per input, amount per unit of output), the output good, capacity per turn, the capital bundle, the construction bundle, construction turns, lifetime, and the number of factories running it. Bundles are one line per good, like the POP types table's endowment column.

**Businesses table.** A new collapsible section after Technologies, one row per business in ID order: name, hex coordinates, decider, current cash in its hex's currency, the number of factories it owns, and the state of this turn's answer (nothing, valid, or refused). A refused row is drawn in the error color, so a refusal anywhere in the world is visible without clicking hexes. The turn line stays as Step 08 left it: the report's decision counts aren't printed, because this table shows the same thing per business.

Both panels keep reading the world directly each frame, so they update straight after every step and reload.

### Map marks
A hex with at least one business gets a third mark, of a different shape from the filled circle (deposits) and the hollow circle (POPs), placed off the label and off the other two marks, so a hex can show any combination of the three. Like the other marks it is drawn only when hexes are drawn as hexagons, never as dots, and the map shows nothing else new.

### Determinism
Businesses, factories, deposits per type, goods in the trade table, technologies and goods rows all come out in ID order, so the panels are always the same for the same world.

### Testing
As in the earlier dev-ui Steps, the panel text is built apart from the window code so it can be tested without opening one:
- each business section's heading, decider and answer lines, refused errors, deposit lines, cash line, trade rows, factory rows (including derived remaining life, `shut down`, and the capital bundle), trace and stockpile lines, checked on the sample's three center businesses under the real `arithmetic` decider at turn 0 and after a step, and under the idle host (every answer nothing, every factory worn one slice);
- a refused decision and a cash-short decision, through a closure decider on a test setting, so the error lines, the proportion below 1 and the cut run fraction are covered;
- the remaining-life rule at the boundaries (exactly one slice, a hair under one slice, a lifetime whose slice isn't exact in decimal arithmetic), agreeing with whether the engine ran the factory;
- the Goods table's third column, the technology rows and the business rows.
The window and the map mark are checked by hand at Code Review, against the acceptance criteria below.

## Player-facing Changes
None for players. In the dev UI, the developer can:
- select a hex and see each business on it: its decider, whether its decision was accepted, what it asked for beside what it got, its cash through the round, its factories' condition, remaining life and this turn's run, wear and maintenance, the decider's trace, and its stockpile;
- see the world's technologies with their numbers, every business with its answer state at a glance, and how much of each good businesses hold;
- see on the map which hexes have businesses.

## Interactions
- **Stage 01 Step 08 and Stage 02 Step 08 (dev-ui Steps):** builds on their panels, map marks, selection and text module. Deposits, the hex stockpile, Market, POPs, Resource types, POP types and the turn log are unchanged.
- **Businesses (Step 02):** the businesses on a hex, each business's name, hex, decider name, stockpile and owned deposits.
- **Factories (Step 03) and technologies (Step 01):** the factories a business owns with their condition and technology; the technology's numbers for the Technologies table and for deriving the capital bundle and remaining life.
- **Factory mechanics (Step 04):** the rule for whether a factory runs, which remaining life follows; the per-factory record's fields.
- **Business view and decisions (Step 05), decider host (Step 07):** the stored answer (nothing, valid, refused with errors), the decision's offers, bids, per-factory orders and trace.
- **Business trade (Step 08):** the outcome: offered beside sold, bid beside bought, the proportion, the three cash figures, and the factory records.
- **Stage 04 (market):** will change what the trade table and Market section show (rationing). **Stage 05 (dev UI):** will decide what the dev UI is for from now on, and will add taking over a business and submitting its decisions by hand.
- **The engine's only change is making its slices rule public; the `check` mode, the turn line and the sample setting are unchanged.**

## Out of Scope
- Any change to the engine beyond making its slices rule public; any change to the `check` mode, the turn line, the protocol, the Python package or any setting.
- Taking over a business or submitting decisions from the window (Stage 05).
- Showing the view the decider was served: between steps the engine's view of a business is next turn's, not the one this turn's decision answered, and recording views is the decider host's job, not the window's.
- Showing a business's history over turns, charts, or saving or exporting what the panels show.
- Printing the report's decision counts or anything new in the turn log.
- Coloring hexes by any data, and any map mark beyond the business mark.
- Automated tests that drive the window itself.

## Acceptance Criteria
- With `settings/sample` open at turn 0 (the real `arithmetic` decider), selecting hex (0, 0) shows a Businesses section with `center_farm`, `center_mine` and `center_toolmaker` in that order, each with decider `arithmetic` and a valid decision, and a trace.
- The farm's section shows two arable land deposits; cash 100 before, 100 after sales, 85 after purchases, proportion 1; a trade table with nothing offered or sold and 4 unimproved_grain, 7 unskilled_labor and 0.5 tools bid and bought; one factory running `farm` with condition 1 before and after, remaining life 8 turns, run 1 asked and done, 12 grain output, wear 0.125, maintenance 0.125 asked and done (detail: inputs 12 unimproved_grain and 6 unskilled_labor, maintenance cost 0.5 tools and 1 unskilled_labor, capital 4 tools); and a stockpile holding 12 grain and 85 silver.
- The mine's section shows one copper ore deposit, cash 50 to 50 to 37, 4 unskilled_labor and 0.75 tools bought, and a factory with condition 0.75 before and 1 after, 2.5 copper output, maintenance 0.375. The toolmaker's shows no deposits, cash 200 to 200 to 152.25, 8 copper, 6.25 unskilled_labor and about 0.125 tools bought (the decider's unrounded value, 0.12499999999999989, since the dev UI rounds nothing), 4 tools output, condition 1 to 1, remaining life 24.
- After one Step, the farm's table shows 12 grain offered and sold, the mine's 2.5 copper, the toolmaker's 3.875 tools, and the cash lines end at 94, 43.5 and 152.5. After Reload the sections return to turn 0's values.
- A refused decision (a test setting with a closure decider, or a deliberately broken script) shows its errors in the error color, its table with the asked amounts and 0 done, and its factories at run 0 with their wear. A cash-short decision shows a proportion below 1 in the error color and the factory's run done below the run asked. A business that decided nothing says so and shows only wear in its factories table.
- A factory holding less than one lifetime-slice reads `shut down` in the error color; the remaining-life count agrees with the number of further turns the factory produces output when stepped without maintenance.
- Hexes without businesses, such as (2, −2), show `No businesses.`
- The Goods table has an In businesses column: in the sample at turn 0 it shows the businesses' grain, copper, tools and silver, and the other two columns are as before.
- The Technologies table lists `farm`, `mine` and `toolmaker` with the rules' numbers and one factory each. The Businesses table lists the three center businesses with hex (0, 0), decider `arithmetic`, their current cash, one factory each, and a valid answer; under a refused decision that row is in the error color.
- On the map, (0, 0) shows all three marks, hexes with only deposits or POPs show only those, and dots show no marks.
- `valid_minimal` and the earlier test settings, which have no businesses, show `No businesses.` on every hex, empty Technologies and Businesses tables, an In businesses column of zeros, and otherwise look as before.
- Automated tests cover the business section text under the arithmetic decider and the idle host, the refused and cash-short cases, the remaining-life rule at its boundaries, the Goods column, and the technology and business rows.
- The engine crate's only change is a public slices-rule helper (its behavior and every existing engine test unchanged); the `check` mode, the turn line and the sample setting are unchanged, and the single check command passes.
