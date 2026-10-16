# Step 01: technologies — Manual Changes

This is the first Step of Stage 03, so no earlier draft in this Stage is affected. Nothing changes in the developer UI window. The changes are for setting authors, plus small updates where the manual quotes the sample setting.

## What users can now see or do

- Setting authors can define **technologies** with a new `[[technology]]` section in `rules/`. A technology is a named recipe that factories will run, shared by every factory that uses it (the manual can compare this with how a POP type is shared by POPs). It has eight required keys: `name`, `inputs` (a table of good = amount per unit of output, may be empty), `output` (one good), `capacity` (the most output per factory per turn), `capital` (the goods a finished factory is made of, at least one), `construction` (the goods used up to build one, may be empty), `construction_turns` and `lifetime` (whole numbers of at least 1). Both TOML table forms work, as for a POP type's endowment. Empty tables have to be written out (`{}`).
- Rules the manual should state plainly: amounts and capacity are numbers greater than 0. Inputs are fixed proportions with no substitution. The output good can't also be an input, but it can be part of the capital (a toolmaker made of tools is fine). A `[[technology]]` in `start/` is an error.
- **What the numbers will mean.** The manual should say clearly that nothing runs yet: technologies are only loaded and checked, and factories arrive in later Steps. It should still explain the rules in plain words, because authors are choosing these numbers now:
  - a factory wears out 1/lifetime of its capital each turn, whether or not it runs, and shuts down when its capital is used up;
  - building one takes `construction_turns` turns and pays an equal slice of the capital and construction goods each turn;
  - maintenance works like a partial rebuild: topping up a share of the capital costs that share of both bundles, and a factory can regain at most 1/`construction_turns` per turn;
  - so if `construction_turns` is greater than `lifetime`, the factory wears out faster than it can be maintained. That is allowed, not an error.
- The checker's success line now ends with a technology count. The sample's line becomes `Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies`, with `1 technology` as the singular form.
- **The sample setting** gains:
  - a seventh good, `tools` (durable, movable), the capital good its factories are made of;
  - a price of 12 `silver` for `tools` on the center hex `(0, 0)`, which is still the only hex that prices every good;
  - three technologies: `farm` (unimproved grain and labor into grain, capacity 12), `mine` (unimproved copper and labor into copper, capacity 2.5) and `toolmaker` (copper and labor into tools, capacity 4). Each is made of tools and built with labor, with lifetimes of 8, 8 and 24 turns. The capacities match what the center's deposits produce each turn. At the center's prices the farm and mine would make a clear profit and the toolmaker only a thin one, so later Steps have both kinds of case to show.
  - The sample's turn reports don't change: nobody wants tools and nothing produces them yet.

## Where it fits

- **New subsection "Technologies"** under "Writing a setting", after "POPs" and before "Running a setting", with an entry in the Contents list. It should cover the keys table, a short TOML example in both table forms (for example a sample-style `farm` inline and a small one using sub-tables with an empty `[technology.inputs]`), the rules, "what the numbers mean", the error list in prose (as the other sections do: missing keys, bad or duplicate names, wrong types, amounts not above 0, turns or lifetime below 1 or not whole, an empty capital, the output among the inputs, unknown goods, unknown keys, the wrong folder; unknown goods are skipped while part of the setting could not be read), and the sample's three technologies, perhaps as a table.
- **Goods:** the sample goods table gains a `tools` row (never perishes, movable, "the capital good the sample's factories are made of"), and "six goods" becomes "seven".
- **Markets:** the sample prices table gains a `tools` column (12 at the center, blank elsewhere). Remove or reword the line saying the prices are placeholders "since nothing is produced for sale yet": the center's prices are now what the sample's technologies are balanced against, though nothing is produced yet.
- **Checking a setting:** update the sample's success line, add `1 technology` to the list of singular forms, and optionally add one technology error to the sample error block (for example `key 'grain' in inputs of technology must not be the output good`).
- **The developer UI, world panel:** the Goods table now lists `tools` with 0 in hexes and 0 in POPs. The sentence "the raw goods appear only under In hexes and silver only under In POPs" stays true; at most add that `tools` is held by nobody yet.
- The intro to "Writing a setting", which lists what a setting describes, can add "and the technologies factories will run".

## Links to docs/

- [docs/technologies.md](docs/technologies.md) for the keys, every error message, and what the numbers mean.
- [docs/goods.md](docs/goods.md) for the goods a technology names.
- [docs/setting-loader.md](docs/setting-loader.md) for the checker's success line.

## Examples, diagrams, demos

- A small worked example would help authors understand "what the numbers mean": take the sample `farm` (capital 4 tools, construction 8 labor, t = 2, t₁ = 8). It loses 0.5 tools per turn. Building it takes 2 turns at 2 tools and 4 labor each. Topping it up from 75% to 100% costs 1 tool and 2 labor, and can be done in one turn because that is within the 1/t = 50% cap.
- An optional code cell could chart a factory's capital over time: unmaintained, falling in a straight line to zero at t₁ and shutting down; and maintained at the most allowed per turn, with one curve for t ≤ t₁ (holds steady) and one for t > t₁ (still runs down). This shows the t ≤ t₁ rule at a glance. It should be labelled as how factories *will* behave, since nothing runs yet.
- Optionally, a small diagram of the sample's production chain on the center hex: deposits → unimproved grain/copper → farm/mine → grain/copper → toolmaker → tools, with tools feeding back as every factory's capital. The existing `hex_svg` helper could instead shade the center hex with "farm, mine, toolmaker", but a chain diagram says more.
