# Step 08: dev-ui-pops

## Summary
This Step finishes Stage 02's developer UI. The hex panel gains the hex's market (its currency and posted prices) and its POPs: for each POP, its type and current size, its growth this turn, the numbers behind its demand, a per-good table of what it held, demanded, needed, sold, bought and consumed, the currency it carried over, and its own stockpile. The world panel gains a POP types table and splits the Goods table's "Held" column into what hexes hold and what POPs hold. Hexes with POPs get a map mark next to the deposit mark. With this, everything Steps 01 to 07 built can be watched working from the window. The turn log already shows the POP totals (`endowed`, `sold`, `bought`, `consumed` and `population`), since those Steps added them to the turn line the window shares with `check --turns`. The engine already makes everything this needs public, so only the dev UI changes.

## Behavior

### What "this turn" means in the panels
Between steps the world is after part 2 of the current turn: every POP has been endowed, traded, consumed and grown, and nothing has perished yet. So the hex panel shows, for the turn in the status bar, the outcome of that turn's trade round, each POP's size after that turn's growth, and each POP's stockpile after consumption and before perishing. The roadmap's "last turn's submission and trade outcome" is this: the most recent round, which is the current turn's. Stepping replaces every outcome with the next turn's.

### Hex panel (right)
The heading, Deposits and Stockpile sections are unchanged. Two sections follow them.

**Market.** The first line names the hex's currency: `Currency: silver`. Then one line per posted price, in good ID order, each written as the amount of currency one unit costs: `grain: 2 silver`. The currency itself is never listed, since it is never posted. A hex with a market that posts no prices shows `No prices posted.` under the currency line, and a hex with no market shows `No market.` under it. Every hex has a currency, so the currency line is always there.

**POPs.** One collapsible section per POP on the hex, in POP ID order, open by default, headed by the POP's type and current size: `peasant × 61.6`. A hex with no POPs shows `No POPs.` Inside each section, in this order:

1. **Growth:** one line, `Started at 60; index 0.6667; rate +2.67%`. The starting size is the size the turn began with, as the outcome records it; the current size is in the heading. The index is shown to 4 decimal places and the rate as a signed percentage to 2 decimal places (`−10.00%` for the north-east peasant, `+0.00%` at exactly subsistence). Hovering the line shows the full floating-point index and rate.
2. **Demand:** one line, `Income 600 silver; subsistence cost 360 silver; supernumerary 240 silver`, all in the hex's currency and written in full like every other amount.
3. **Goods table:** one row per good in the recorded demand (every utility good, every held good, and the currency when income was parked in it), in good ID order, with columns Good, Held, Demanded, Subsistence, Sold, Bought, Consumed. Held and Demanded are what the demand recorded (the holdings submitted and the quantity the POP wanted to end up with); Subsistence is the type's subsistence amount times the starting size, 0 for a good outside the utility; Sold, Bought and Consumed are the round's amounts, 0 when nothing happened. A row whose demanded quantity falls short of its subsistence is drawn in the theme's error color, so a POP below subsistence is spotted at a glance and the good it lacks is obvious.
4. **Carried over:** one line, `Carried over: 0 silver`, the hex's currency left in the stockpile after consumption. For the sample this is at most a rounding sliver.
5. **Stockpile:** the POP's stockpile, written exactly as the hex stockpile is: each good held with its total, and under it each perish group with its countdown, soonest first, or `Empty.` In the sample a market-hex peasant's stockpile after the round holds nothing but any silver sliver (labor and grain were consumed), and the north-east peasant's holds nothing, since it consumed all its labor as leisure.

Sizes and every amount are written in full, as everywhere in the dev UI; only the index and rate are rounded, because they are ratios rather than quantities.

### World panel (left)
Two changes.

**Goods table.** The "Held" column becomes two columns, **In hexes** and **In POPs**: the good's total over every hex stockpile, and over every POP stockpile. The two are kept apart because they are different things: hex stockpiles are the unowned output of deposits, POP stockpiles are owned goods. In the sample, unimproved goods appear only under In hexes, and silver only under In POPs.

**POP types table.** A new collapsible section after Resource types, with one row per POP type in ID order and columns Name, Endowment, Utility, Growth, POPs, Total size:
- Endowment: one line per endowed good, `10 unskilled_labor`, or `nothing` for an empty endowment.
- Utility: one line per utility good, `grain: subsistence 2, share 0.8`. Shares are the engine's normalized shares, shown to 4 decimal places (so the artisan's weights 1, 4, 5 read `0.1`, `0.4`, `0.5`), with the full value in hover text.
- Growth: `growth [0.06, -0.03]; decline 0.1; max 0.05`, the parameters as the setting gave them.
- POPs: the number of POPs of the type in the world. Total size: the sum of their current sizes, in full.

Both panels keep reading the world directly each frame, so they update straight after every step and reload.

### Map marks
A hex with at least one POP gets a second mark: a hollow circle of the same size as the deposit mark, placed in the top half of the hexagon mirroring the deposit mark's position below the label, so a hex can show both, either, or neither. Like the deposit mark it is drawn only when hexes are drawn as hexagons, never as dots, and the map shows nothing else new: no coloring by population or any other data.

### Determinism
Prices, POPs, goods in a POP's table, POP types and goods rows all come out in ID order, so the panels are always the same for the same world.

### Testing
As in Stage 01, the panel text is built apart from the window code so it can be tested without opening one:
- the market lines (currency, prices, no prices, no market);
- each POP's heading, growth line, demand line, goods-table rows with their below-subsistence flag, carried-over line and stockpile lines, checked on the sample's center peasant and artisan and the north-east peasant;
- the index and rate formatting (sign, rounding, the exact-subsistence case);
- the goods rows' two columns;
- the POP type rows, including an empty endowment and normalized shares.
The window and the map marks are checked by hand at Code Review, against the acceptance criteria below.

## Player-facing Changes
None for players. In the dev UI, the developer can:
- select a hex and see its currency and posted prices;
- see each POP on the hex: type, size, growth this turn, the numbers behind its demand, what it held, wanted, needed, sold, bought and consumed per good, the currency it carried over, and its stockpile;
- see the world's POP types with their parameters, counts and total sizes, and how much of each good is held in hexes and in POPs;
- see on the map which hexes have POPs.

## Interactions
- **Stage 01 Step 08 (dev-ui-inspect-and-step):** builds on its panels, map marks, selection and text module. Deposits, the hex stockpile, Resource types and the turn log are unchanged.
- **Currency (Step 01), dummy market (Step 02):** the Market section reads the hex's currency and posted prices through the engine's public interface.
- **POP types (Step 03):** the POP types table reads each type's endowment, utility goods with normalized shares, and growth parameters.
- **POPs (Step 04):** the POPs on a hex, each POP's current size and stockpile; the map mark uses the per-hex POP list; the Goods table's In POPs column sums every POP stockpile.
- **Stone-Geary demand (Step 05), POP trade (Step 06), POP growth (Step 07):** the POP sections read the recorded outcome: the demand numbers and per-good records, sold, bought, consumed, carried over, index, rate and starting size.
- **Stage 03 (industries), Stage 04 (businesses):** will add their own hex-panel sections the same way. **Stage 05 (market):** will change what the Market section shows (adjusted prices, rationing) and may add to the POP goods table.
- **The engine, the `check` mode and the sample setting are unchanged.**

## Out of Scope
- Any change to the engine, the `check` mode, the turn line, or any setting.
- Coloring hexes by population or any other data, and any map mark beyond the POP mark.
- Showing a POP's history over turns, charts, or saving or exporting what the panels show.
- Editing POPs, prices or the world from the window.
- Rounding sizes or amounts; only the index, rate and normalized shares are rounded for display.
- Automated tests that drive the window itself.

## Acceptance Criteria
- With `settings/sample` open at turn 0, selecting hex (0, 0) shows, after the Deposits and Stockpile sections: `Currency: silver` and the prices `unskilled_labor: 1 silver`, `unimproved_copper: 1 silver`, `unimproved_grain: 0.5 silver`, `copper: 5 silver`, `grain: 2 silver` in that order; then two POP sections.
- The center peasant's section is headed `peasant × 61.6` and shows `Started at 60; index 0.6667; rate +2.67%`, `Income 600 silver; subsistence cost 360 silver; supernumerary 240 silver`, a goods table with unskilled_labor (held 600, demanded 168, subsistence 120, sold 432, consumed 168) and grain (demanded 216, subsistence 120, bought 216, consumed 216), no row in the error color, `Carried over` of at most a rounding sliver of silver, and a stockpile that is `Empty.` or holds only that sliver, marked `never perishes`.
- The center artisan's section is headed `artisan × 10.3` and shows index `1.0000`, rate `+3.00%`, income 100, subsistence cost 50, supernumerary 50, 85 unskilled_labor sold, 30 grain and 5 copper bought and consumed, and 15 unskilled_labor consumed.
- Selecting hex (2, −2) shows `Currency: silver` and `No market.`, and a `peasant × 10.8` section with `Started at 12; index −1.0000; rate −10.00%`, income 0, a grain row in the error color (demanded 0, subsistence 24), 120 unskilled_labor held, demanded and consumed, nothing sold or bought, and an empty stockpile.
- A hex with no POPs, such as (2, −1), shows `No POPs.` A test hex with a market that posts no prices shows `No prices posted.`
- Hovering a growth line shows the full index and rate. A POP exactly at subsistence (a test setting) shows `index 0.0000; rate +0.00%`.
- After Step, every POP section shows the new turn's outcome and size (the north-east peasant reads `peasant × 9.72`), and the stockpiles and Goods columns update at once. After Reload the panels return to turn 0's values.
- The world panel's Goods table has In hexes and In POPs columns: at every turn the sample shows 10 unimproved_copper and 24 unimproved_grain in hexes and 0 in POPs, and silver only in POPs.
- The POP types table lists `peasant` (endowment `10 unskilled_labor`; utility `unskilled_labor: subsistence 2, share 0.2` and `grain: subsistence 2, share 0.8`; `growth [0.06, -0.03]; decline 0.1; max 0.05`; 15 POPs) and `artisan` (shares 0.1, 0.4, 0.5, copper with subsistence 0; 1 POP; total size 10.3 at turn 0). Total sizes add up to the turn line's population.
- On the map, hexes with POPs show a hollow circle above the label, and hexes with deposits keep their filled circle below it; (0, 0) shows both, (2, −1) shows only the deposit mark if it has deposits, and dots show no marks.
- `valid_minimal` and the Stage 01 test settings, which have no POPs, show `No POPs.` on every hex, empty POP types and In POPs columns, and otherwise look as before.
- Automated tests cover the market lines, the POP section text including the below-subsistence flag and the index and rate formatting, the two Goods columns, and the POP type rows.
- The engine crate, the `check` mode, and the sample setting are unchanged, and the single check command passes.
