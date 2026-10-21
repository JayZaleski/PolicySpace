# Step 09: dev-ui-businesses — Manual Changes

Nothing changes for players or setting authors' files. The developer UI can now show everything Stage 03 built: each business on a hex, with its decision and what came of it, its factories, and its stockpile; the world's technologies and businesses; how much of each good businesses hold; and which hexes have businesses on the map. This finishes Stage 03's developer UI and replaces the "not visible yet" and "Not available yet" notes in the earlier drafts (see "Changes to earlier drafts").

## What users can now see or do

- **A Businesses section in the hex panel**, after POPs. One collapsible section per business on the hex, open at first, headed by its name and current cash (`center_farm: 85 silver`), or `No businesses.` Inside it, in order:
  - **Decider and answer:** the decider's name and whether this turn's decision was valid, refused, or never came (nothing sent, or the decider failed). A refused decision lists every error, in red, with the place it was found (for example a factory number the business doesn't own).
  - **Deposits** the business owns, written like the hex's deposit lines, or `Owns no deposits.`
  - **Cash** through the round: before trading, after its sales, after its purchases, and the **proportion** the engine cut its purchases by. 1 means it could afford everything; a value below 1 is in red and means it asked to buy more than it could pay for, so it got a smaller share of each good and its factories ran less.
  - **A trade table:** one row per good, with what it offered beside what it sold, and what it bid beside what it bought. A cut shows as "asked" above "done" in the same row. A refused decision shows what was asked with everything done at 0; a business that decided nothing shows `Nothing was traded.`
  - **A factories table:** each factory's number (the one the decider uses), technology, condition before and after this turn, **remaining life** (how many more turns it can run with no maintenance, or `shut down` in red), the run asked and done, the output, the wear, and the maintenance asked and done. Hovering shows the inputs the run used (on Output), what maintenance cost (on Maintenance done), and the capital the factory holds (on Condition after).
  - **Trace:** the decider's written explanation of its arithmetic, collapsed at first.
  - **Stockpile**, written exactly like the hex's and the POPs', including this turn's output waiting to be sold next turn and the business's cash.
- **"This turn" means the same as for POPs:** the decision made at the current turn's decision point and what came of it. Stepping replaces everything with the next turn's figures; a collapsed business section stays collapsed.
- **The world panel gains two sections and a column:** the Goods table's **In businesses** column (what businesses own; in the sample the owned deposits' raw output, the factories' output, and the businesses' silver now appear here); a **Technologies** table (recipe, output, capacity, capital, construction goods and turns, lifetime, and how many factories run it); and a **Businesses** table (hex, decider, cash, factory count, and this turn's answer as `valid`, `refused` or `nothing`). A refused business's row is in red, so a broken decider shows without clicking around.
- **A third map mark:** a small filled square at the lower left of the label marks a hex with at least one business. The sample's center hex shows all three marks. As before, marks only appear when hexes are drawn as hexagons.
- Nothing is rounded in the business sections, so a decider's floating-point leftovers show as they are (the sample's toolmaker buys `0.12499999999999989` tools rather than 0.125).

## Changes to earlier drafts

- **Step 02 (businesses):** its note for "The world panel" that business holdings aren't counted in either column is out of date: the new In businesses column counts them. The sentence "the raw goods appear only under In hexes and silver only under In POPs" also needs updating: silver and the owned raw goods now also appear under In businesses.
- **Steps 02 and 03:** "Not available yet" was to gain "inspecting businesses and their stockpiles" and "inspecting factories". Drop both; they now exist.
- **Step 06 (business-script):** "the trace, which Step 09 will show in the developer UI": it now does, in each business section.
- **Step 08 (business-trade):** its "Not visible yet" bullet (a business's stockpile, cash, decision and per-factory results can't be inspected) and its "Not available yet" note for the developer UI are out of date. Its example table of the sample at turn 0 and turn 1 can now be pointed to in the window: selecting hex (0, 0) shows the same numbers.

## Where it goes

- **"The developer UI" intro:** add businesses and factories to what the window lets you inspect.
- **"Reading the map":** add the business mark (filled square, lower left) to the description of the marks; the sample's center now has all three.
- **"Inspecting a hex":** add a **businesses** bullet to the list of what the hex panel shows, then a short subsection-style block, after the POP section example, describing a business section. Reuse the "What 'this turn' means" paragraph by extending it to businesses rather than writing a second one. Add `No businesses.`, `Owns no deposits.`, `Nothing was traded.` and `No factories.` to the "Other messages" list. Extend the sentence about what's rounded to say business numbers are never rounded.
- **"The world panel":** "four sections" becomes six; the Goods bullet gains In businesses; add Technologies and Businesses bullets.
- **"The author's loop":** step 3 can add "and the businesses' trades and factories". Optionally a tip: to see why a business did something, open its Trace; to test a decider change, edit the Python decider and Reload (Step 07's draft already says Reload restarts deciders).
- **"Not available yet":** keep the existing list, add "a business's history over turns", and note that taking over a business and sending its decisions by hand from the window is planned for a later Stage. Remove anything about inspecting businesses or factories.
- **Links:** [docs/dev-ui.md](docs/dev-ui.md#businesses-factories-and-technologies) for the business sections and tables; [docs/trade.md](docs/trade.md) for the round, the cash figures and the proportion; [docs/production.md](docs/production.md) for run, wear, maintenance, shutdown and why remaining life is a whole number of turns; [docs/decider.md](docs/decider.md) for the validation errors a refused decision can show; [docs/python.md](docs/python.md) for how to read the arithmetic rule's trace.

## Example

A text block like the existing POP section example, for the sample's `center_farm` at turn 0 under the `arithmetic` decider, would make the section concrete. Its numbers (all checked by the tests):

- heading `center_farm: 85 silver`; `Decider: arithmetic`; `Answer: valid decision`;
- deposits `arable_land × 2: 4 unimproved_grain each, 8 total`;
- `Cash: 100 silver before; 100 after sales; 85 after purchases;` `proportion 1`;
- trade rows (Offered / Sold / Bid / Bought): unskilled_labor 0 / 0 / 7 / 7, unimproved_grain 0 / 0 / 4 / 4, tools 0 / 0 / 0.5 / 0.5;
- one factory: `0`, `farm`, condition 1 → 1, `8 turns`, run 1 / 1, output `12 grain`, wear 0.125, maintenance 0.125 / 0.125 (hover: inputs 6 unskilled_labor and 12 unimproved_grain; maintenance cost 1 unskilled_labor and 0.5 tools; capital 4 tools);
- stockpile `12 grain` (this turn's output, to be sold next turn, with its perish countdown) and `85 silver` (`never perishes`).

Follow it with a short reading guide, as the POP example has: the farm bought what its fields didn't cover, ran at full capacity, lost one eighth of its condition to wear and paid it back with maintenance, so it is back at 1 with 8 turns of life. After one Step it sells the 12 grain and ends with 94 silver.

A second, shorter contrast would help, like the north-east peasant does for POPs: what a business that decided nothing looks like (the same farm with no decider answer: `Answer: nothing ...`, `Nothing was traded.`, run 0, condition 1 → 0.875, `7 turns`), so readers see that an idle factory still wears down. The red cases (a refused decision, a proportion below 1, `shut down`) can be described in a sentence each rather than shown; the sample doesn't produce them.

No executed cell or diagram is needed: the window can't be shown from the notebook, and the numbers above come from the tests.
