# Step 08: dev-ui-pops — Manual Changes

The developer UI can now show everything Stage 02 built. Select a hex to see its currency and posted prices, and each POP on it: its type and size, how it grew this turn, the numbers behind its demand, what it held, wanted, needed, sold, bought and consumed of each good, the currency it carried over, and its own stockpile. The world panel gains a POP types table and splits the goods' "held" amount into what hexes hold and what POPs hold, and the map marks hexes that have POPs. Nothing changes for players, in the setting format, or in the checker's output.

## What users can now see or do

- **Hex panel, Market section** (after Deposits and Stockpile): the hex's currency (`Currency: silver`), then each posted price as the amount of currency one unit costs (`grain: 2 silver`), in goods order. A hex with a market but no prices says `No prices posted.`; a hex with no market says `No market.`. The currency line is always there, since every hex has a currency.
- **Hex panel, POPs section**: `No POPs.`, or one collapsible section per POP, open by default, headed by type and current size (`peasant × 61.6`). Inside, in order:
  - a **growth line**, `Started at 60; index 0.6667; rate +2.67%`: the size the turn began with, the subsistence index, and the growth rate. Hovering it shows the unrounded index and rate;
  - a **demand line**, `Income 600 silver; subsistence cost 360 silver; supernumerary 240 silver`;
  - a **goods table** with columns Good, Held, Demanded, Subsistence, Sold, Bought, Consumed. Explain the columns in plain words: what the POP brought to market, what it wanted to end up with, what it needs to subsist (its type's amount times its size), and what happened in the trade round. **A row in the error color (red) means the POP wanted less of that good than it needs to subsist**, so a struggling POP and the good it lacks stand out;
  - **Carried over**: currency left after consumption, usually 0 or a tiny rounding sliver;
  - its **stockpile**, written exactly like the hex's.
- **What "this turn" means**: between steps the world has just traded, consumed and grown, and nothing has perished yet. So the panel shows the outcome of the turn in the status bar, each POP's size after growth, and its stockpile after consumption. Each step replaces it all with the next turn's. A collapsed POP section stays collapsed when you step.
- **Numbers**: sizes and amounts are written in full, with long decimals after a few turns, as in the turn log. Only the index (4 decimals), rate (signed percent, 2 decimals) and utility shares (up to 4 decimals) are rounded. **This supersedes Step 04's manual draft**, which said the developer UI "may round" sizes for display later; it doesn't.
- **World panel, Goods**: the single "held" column is now two, **In hexes** (unowned deposit output in hex stockpiles) and **In POPs** (goods POPs own). **This supersedes the notes in Step 04's and Step 06's manual drafts** that the Goods amount counts only hex stockpiles: POP holdings are now shown, in their own column. In the sample, raw goods appear only under In hexes and silver only under In POPs.
- **World panel, POP types**: a new section after Resource types, with each type's name, endowment (or `nothing`), utility goods with their subsistence amounts and normalized shares (hover for the full value), growth parameters, number of POPs and total size. The total sizes add up to the turn log's `population`.
- **Map**: a hex with POPs shows a small **hollow circle above** its label; the deposit mark is a **filled circle below** it. A hex can have both, either or neither. Marks show only when hexes are drawn as hexagons, not as dots.
- **The hex panel starts wider** and scrolls sideways when a table doesn't fit.

## Where it fits

All under **"The developer UI"**:

- *Reading the map*: extend the sentence about the deposit mark to describe both marks (filled below for deposits, hollow above for POPs).
- *Inspecting a hex*: add the Market and POPs sections to the list of what the hex panel shows, with the "what this turn means" explanation and a walk-through of one POP section (see the example below). Add `No market.`, `No prices posted.` and `No POPs.` to the "Other messages" sentence.
- *The world panel*: it now has four sections. Replace the Goods bullet with the two columns (this is where Step 04's and Step 06's "only hex stockpiles" note would have gone; drop it) and add a POP types bullet.
- *The author's loop*: step 3 can now say "check the stockpiles, prices and POPs".
- *Not available yet*: remove everything that Step 02, 03, 04, 06 and 07's drafts said can't be inspected yet (prices, POP types, POPs, their sizes and stockpiles, what each POP sold, bought and consumed, its income, whether it is below subsistence, its index and rate). **This supersedes those drafts' "Not available yet" additions.** What remains unavailable: auto-run, stepping by another number, coloring hexes by data (population included), a POP's history over turns or charts, editing the world, and selecting several hexes.
- The manual's opening and "Writing a setting" sections can now point readers to the window as the place to watch POPs trade and grow, for example from Step 07's "How POPs grow" paragraph and Step 06's "Trade and consume" description in "Running a setting".

## Links to docs/

- [docs/dev-ui.md](docs/dev-ui.md), for the panels, the number formatting and the map marks (already linked at the end of the section).
- [docs/demand.md](docs/demand.md), for income, subsistence cost, supernumerary income and the demanded quantities in the demand line and goods table.
- [docs/trade.md](docs/trade.md), for sold, bought, consumed and carried over.
- [docs/growth.md](docs/growth.md), for the subsistence index and growth rate on the growth line.
- [docs/market.md](docs/market.md) and [docs/currency.md](docs/currency.md), for the Market section.

## Examples or diagrams

- **A worked POP section** for the sample's center peasant on hex (0, 0) at turn 0, laid out as the panel shows it, with a sentence per line explaining where the numbers come from: 60 peasants endowed 10 labor each give 600 labor worth 600 silver; subsistence needs 2 labor and 2 grain each (120 of each, costing 120 + 240 = 360 silver); the 240 supernumerary silver is split 0.2 / 0.8 between labor and grain, so it keeps 168 labor as leisure and sells 432, and buys 216 grain at 2 silver; index 240 / 360 = 0.6667 gives a rate of +2.67% and a size of 61.6. Goods table rows: unskilled_labor (held 600, demanded 168, subsistence 120, sold 432, bought 0, consumed 168) and grain (held 0, demanded 216, subsistence 120, sold 0, bought 216, consumed 216).
- **A contrasting section** for the north-east peasant on (2, -2) (`No market.`): `peasant × 10.8`, `Started at 12; index -1.0000; rate -10.00%`, income 0, its grain row (demanded 0, subsistence 24) in red, all 120 labor consumed as leisure. Together the two show what the red row and the index mean.
- **A small diagram of a hexagon with both marks** (hollow circle above the `q, r` label, filled circle below), if the manual's existing map SVG helper can draw it; otherwise a sentence is enough.
- The checker can't print the panels, so these sections should be static text rather than a code cell. Their numbers are the design's acceptance criteria, which the dev UI's tests check.
