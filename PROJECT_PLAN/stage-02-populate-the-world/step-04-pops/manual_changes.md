# Step 04: pops — Manual Changes

POPs now exist on the map. A setting author places them in `start/`, and every turn each POP receives its type's endowment into its own holdings. A player still sees nothing new, since nothing consumes or trades yet. The visible changes are a new setting section, a new count in the checker's success line, and a third list (`endowed`) on every turn line in `check --turns` and the developer UI's turn log. The developer UI does not show POPs yet (Step 08).

## What setting authors can now do

- Place **POPs** with `[[pop]]` sections in `start/` (in `rules/` it is an error, because POPs are state that will change during play). A POP is a group of N identical individuals of one POP type living on one hex. Repeat the distinction from Step 03's draft: `[[pop_type]]` in `rules/` describes one individual, and `[[pop]]` in `start/` places a group of them.
- Four required keys:
  - `type`: the name of a POP type from `rules/`.
  - `q`, `r`: the coordinates of a hex in the map, as for deposits and markets.
  - `size`: N, the number of individuals. It is a number greater than 0, and **decimals are allowed**. Explain briefly why: a POP is an aggregate whose endowment (and later its demand) scales with N, and its size will grow by a rate, so whole numbers would be awkward. The developer UI may round it for display later.
- **At most one POP of each type per hex.** A second `[[pop]]` with the same type and coordinates is an error that names the first one. A hex may hold POPs of several types, or none.
- **POPs start with nothing.** There is no starting cash or goods. At the start of every turn, including turn 0, each POP receives its type's endowment times its size. The endowment perishes like any other good: the sample's labor (`perishes_after = 1`) is gone at the end of the turn it arrives, while an endowed durable good would pile up. Say plainly that nothing uses what POPs hold yet: demand, trade, and growth come in the next Steps of this Stage.
- Deposits are still unowned. Their output still goes to the hex's unowned stockpile, not to the POPs there.
- New errors, with file, line and column: a missing `type`, `q`, `r` or `size`; a `type` that isn't text; bad or out-of-range coordinates; coordinates with no hex (`no hex at (9, 9)`); a `size` that isn't a number, or is 0, negative or `inf`; a duplicate POP (`duplicate pop of 'worker' at (0, 0) (first defined at ...)`); an unknown POP type (`unknown pop type 'ghost'`); an unknown key; and `[[pop]]` in `rules/`. As with other references, the unknown-type and no-hex errors are skipped when part of the setting couldn't be read.

## Where it fits

- **New subsection "POPs"** under "Writing a setting", directly after the "POP types" subsection that Step 03's draft adds (and before "Running a setting"). Add it to the Contents list. Cover the keys, continuous size, one POP per type per hex, that POPs start empty and are endowed every turn, a short TOML example, and the sample's POPs. The "POP types" subsection from Step 03 should end with a pointer to this one. Step 03's draft said placed POPs were "coming in the next Step" and that "nothing uses POP types yet". Both are now out of date: POP types are used to place POPs and to endow them.
- **"The folder layout"**: the `start/` line in the layout block should mention POPs (`the map, the deposits, the markets, the POPs`), and the bullet listing which section types go in which folder should add `[[pop]]` to `start/`.
- **"Running a setting"**:
  - Part 1 of the three-part turn becomes "**Emit and endow**": deposits emit, then every POP receives its endowment times its size into its own stockpile. Part 3 (Perish) now covers POP stockpiles too. Part 2 (Actors act) is still empty; POPs will buy and sell here from Step 06.
  - The sentence "Deposits are unowned, so everything lands in an 'unowned goods' stockpile on the hex" still holds for deposit output. Add that each POP has its own stockpile, which is where its endowment goes.
  - **Turn reports** gain a third list, `endowed`: the total each good POPs received at the start of the turn, kept separate from `emitted` so deposit output and POP income can be told apart. `perished` now includes what POPs lost. Replace the code cell that prints the sample's turns 0 to 2 with:
    ```
    Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor
    Turn 1: perished 4100 unskilled_labor, 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor
    Turn 2: perished 4100 unskilled_labor, 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor
    ```
    Explain that 4100 is 10 labor per individual times the 410 individuals in the sample, and that `unskilled_labor` comes first in `perished` because lists are in goods order. A setting with no POPs prints `endowed nothing`.
- **"Checking a setting"**: the sample success line gains a count. **This supersedes Step 03's manual draft**, which gave the line ending in `2 pop types`. It is now `Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops`. The singular-wording note should add `1 pop`. If the section shows `check --turns` output, use the new lines above. Optionally add a POP error (for example the duplicate-POP error) to the sample error listing, and keep the error count line consistent.
- **"The developer UI"**:
  - *Turn log*: it already says the log uses the same wording as `check --turns`, so it now shows the `endowed` list too. No other wording change is needed.
  - *Goods*: the "amount held in the whole world" counts **only the hex stockpiles**, not what POPs hold. The sample's 4100 labor doesn't appear there. Add a short note so authors aren't confused by it.
  - *Not available yet*: add that POPs, their sizes and their stockpiles can't be inspected in the window yet (Step 08).

## The sample setting

`settings/sample/start/pops.toml` places 16 POPs, 410 individuals in all:

- a `peasant` POP on 15 of the 19 hexes, with sizes from 12 to 60 (60 at the center);
- an `artisan` POP of size 10 on the center hex (0, 0), which is the only hex that prices every good, so the center holds two POPs;
- a `peasant` POP of size 12 on the north-east corner (2, -2), which has no market, so its labor will never be sellable (a deliberate test case for Step 06);
- no POPs on (2, -1), (-2, 0), (1, 1) and (-2, 2).

The file's comments explain every key, that size is continuous, the one-per-type-per-hex rule, and that POPs start empty.

## Links to docs/

- [docs/pops.md](docs/pops.md) (new in Step 03's draft, now also covering placed POPs). Link it from the end of the new "POPs" subsection for the data model, the loading rules and the full error list.
- [docs/world.md](docs/world.md), from "Running a setting", for how endowment and perishing fit into the turn and for the turn report.

## Examples or diagrams

- A short TOML example with two POPs of different types on the same hex, one with a decimal size.
- Worth a diagram: the sample map with each hex labelled by its POPs and sizes, marking the center's two POPs, the corner hex without a market, and the four empty hexes. If the manual already draws the sample map (in "The map" or "Natural resources"), the same drawing code with a POP overlay would fit. A small table works too if a diagram is too much.
- A small worked example of one POP across two turns would make the timing clear. For example, the center peasant (size 60) holds 600 labor during turn 0, it perishes at the end of turn 0, and 600 arrives again at the start of turn 1. If a type endowed a durable good, the amount would grow each turn instead. No interactive demo is needed until demand (Step 05) gives POPs something to decide.
