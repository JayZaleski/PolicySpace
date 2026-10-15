# Step 07: pop-growth — Manual Changes

POPs now grow and shrink. Every turn, after trading and consuming, each POP's size changes by a rate that depends on how far above or below subsistence it was. Setting authors must give every POP type a new `growth` sub-table, and `check --turns` and the developer UI's turn log show the total population at the end of every turn line. Because the sample's POPs grow, its turn lines are no longer the same from turn to turn. A POP's own index, rate and size can't be inspected in the window yet (Step 08).

## What users can now see or do

- **A new required key on every POP type: `growth`**, a sub-table with three required keys (inline or as a `[pop_type.growth]` table, like `utility`):
  - `growth`: a list of one or more numbers, the coefficients of the growth rate above subsistence. Explain in plain terms: the rate is c₁u + c₂u² + …, where u is how many extra "subsistence bundles" the POP's income could buy (0 at exactly subsistence, 1 when its income is twice its subsistence cost). One number gives a straight line; a negative second number makes growth peak and then fall off as POPs get richer.
  - `decline`: a number from 0 up to but not including 1, how fast a POP with nothing shrinks per turn. Below subsistence the shrink is in proportion: a POP that gets half of what it needs shrinks by half of `decline`.
  - `max_growth`: a number of at least 0, the cap on growth per turn.
  - The rate always stays between −`decline` and `max_growth`, and a POP is never removed: a starving POP shrinks by the same fraction every turn but never reaches zero.
  - `growth = [0]` with `decline = 0` freezes a type's size, useful for a setting that should stay put.
- **"Below subsistence" in plain words**, as it matters for growth: a POP that can't get a good it needs at all (no market sells it and it doesn't hold any) counts as having nothing and shrinks at the full `decline`, however rich it is otherwise. A type whose subsistence goods cost nothing where it lives (or that has none) neither grows nor shrinks.
- **New load errors**, with file, line and column: a missing `growth`; a `growth` that isn't a table; a missing `growth`, `decline` or `max_growth` key inside it; a `growth` list that isn't a list, is empty, or holds something that isn't a finite number; a `decline` that is negative, 1 or more, or not a finite number; a `max_growth` that is negative or not a finite number; an unknown key in the sub-table. Every existing setting with POP types must add the sub-table or it no longer loads; say this plainly.
- **`population` on every turn line**, after `consumed`: the total size of all POPs after this turn's growth (`population 0` for a setting without POPs). The number is printed in full, with nothing rounded.
- **Sizes change every turn**, so endowments, sales, purchases and consumption grow or shrink with them, and sizes become long decimals after the first turn. A fresh world has already grown once, because creating the world runs turn 0 in full.

## Where it fits

- **"Writing a setting" → "POP types"** (Step 03's subsection): add `growth` as the fourth required key and a short explanation of the three sub-keys, as above. **This supersedes Step 03's manual draft**, which listed "three required keys" (`name`, `endowment`, `utility`); there are now four. Add `growth` to the TOML examples Step 03 suggested (one inline, one as a `[pop_type.growth]` table), extend the error list with the new errors, and extend the sample's table (or add a line under it) saying both sample types use `growth = [0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05`: growth peaks at 3% a turn for a POP whose income is twice its subsistence cost, falls to zero at three times, and a POP with nothing shrinks 10% a turn. This fits as a short "How POPs grow" paragraph or sub-subsection at the end of "POP types".
- **"Writing a setting" → "POPs"** (Step 04's subsection): Step 04's draft said a POP's size "will grow by a rate". It now does: say that `size` is the starting size, that it changes every turn, and that this is why decimals are allowed. Step 04's sample description ("endowed 4100 unskilled_labor every turn") is now true of turn 0 only; from then on the sizes grow or shrink.
- **"Running a setting"**:
  - Part 2 of the turn ("Trade and consume", from Step 06's draft) gains a last step: **then every POP grows or shrinks**. **This supersedes Step 06's manual draft**'s description of part 2, which ended with consumption. Say that a new world has already run turn 0's growth, and that what the checker and window show between steps is the world after growth.
  - **Turn reports** gain `population`. Replace the code cell that prints the sample's turns 0 to 2 (**this supersedes the lines in Step 06's manual draft**, whose turn 2 was "the same as turn 1", no longer true). The cell should run the checker and print real output rather than hardcode it; for reference, the real lines are:
    ```
    Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666
    Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4194.466666666667 unskilled_labor; sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain; population 429.2982444444444
    Turn 2: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4292.982444444444 unskilled_labor; sold 3034.7550599999995 unskilled_labor; bought 5.304500000000001 copper, 1504.1162799999997 grain; consumed 1258.2273844444444 unskilled_labor, 5.304500000000001 copper, 1504.1162799999997 grain; population 439.55036096296294
    ```
    Explain: the setting has 410 individuals, but turn 0 already reports about 419.4 because turn 0's growth has run; each turn's `endowed` is 10 labor times the previous turn's population (4194.47 = 10 × 419.447); and the amounts now print with long decimals because nothing is rounded. Step 06's explanation of where turn 0's trade numbers come from still holds for turn 0.
- **"Checking a setting"**: if it shows `check --turns` output, use the lines above. The success line is unchanged. Optionally add a growth error (for example `key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1`) to the sample error listing, keeping the error count line consistent.
- **"The developer UI"**:
  - *Turn log*: it already says the log uses the same wording as `check --turns`, so it now shows `population`. No other wording change is needed.
  - *Not available yet*: add that each POP's size, subsistence index and growth rate can't be inspected in the window yet (Step 08); only the total population is visible, in the turn log.

## Links to docs/

- [docs/growth.md](docs/growth.md) (new), from the "How POPs grow" paragraph in "POP types" and from part 2 in "Running a setting": the subsistence index, the rate, where growth runs in the turn, and worked examples.
- [docs/pops.md](docs/pops.md), for the `growth` key's exact rules and error messages.
- [docs/world.md](docs/world.md), for the turn and the turn report's `population`.

## Examples or diagrams

- **A chart of the growth rate against the subsistence index** for the sample's parameters (`[0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05`), u from −1 to about 4: the straight decline from −10% at u = −1 to 0 at u = 0, the curve peaking at 3% at u = 1, crossing zero at u = 2, and flattening at the −10% floor beyond about u = 3.08. Mark where the sample's POPs sit: the no-market peasant at u = −1 (−10%), a peasant on a market hex at u = 2/3 (about +2.7%), the center artisan at u = 1 (+3%). Computed in the notebook from the formula; a slider for the coefficients, `decline` and `max_growth` would help authors try shapes if the manual already uses widgets, otherwise a static chart is enough.
- **A chart of sizes over turns** from running the sample for, say, 20 turns: a growing market-hex peasant, the center artisan, and the shrinking north-east peasant (12 → 10.8 → 9.72 → …), and/or the total population from the turn lines. It shows compounding growth and that a starving POP shrinks but never vanishes.
- If Step 06's suggested "what a POP sells, buys and consumes as the grain price varies" demo is built, it could add the growth rate as one more line, crossing zero where the peasant falls below subsistence (grain price 4 with labor at 1).
- The checked numbers in docs/growth.md's worked examples are the values these examples should match.
