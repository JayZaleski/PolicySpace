# Step 01: currency — Manual Changes

Nothing new is visible when running the world: there are no prices or trade yet, and the developer UI does not show a hex's currency (that comes in Step 08). The change is to the setting format, which the manual documents for setting authors.

## What setting authors can now do, and must now do

- Every `[[hex]]` now **requires** a third key, `currency`: the name of a good defined in `rules/`. It names the good that prices on that hex will be posted in (by the markets added in later Steps of this Stage).
- Any good may be the currency. The engine has no special "money" property, so a perishable or non-movable good is allowed.
- Different hexes may name different currencies. There is no map-wide default: every hex writes its own.
- New errors, each with file, line and column: a hex with no `currency` (`missing required key 'currency' in hex`), a value that isn't text (`key 'currency' in hex must be a string, found integer`), and a name that matches no good, including wrong case (`unknown good 'Coin'`). Like other reference errors, the unknown-good error is not shown when part of the setting could not be read.
- **Breaking change for existing settings:** a setting written before this Step won't load until every hex has a `currency` key. This deserves a short note.

## Where it fits

- **"The map" section** (under "Writing a setting"): it currently says each `[[hex]]` has "exactly two required whole-number keys, `q` and `r`". Change that to three required keys, and add a short paragraph or subsection ("Currency") explaining what the currency is, that any good may be one, and that hexes may differ. Add `currency = "..."` to each hex in the TOML example, and mention that the sample map uses `silver` on all 19 hexes.
- **"Goods" section**: the sample table says "five goods", but there are now six. Add a `silver` row: `perishes_after` never, `movable` true, role "the currency every sample hex designates".
- **"The folder layout"**: optionally note on the `start/` line that hexes also name their currency.
- **"Checking a setting"**: the sample success line changes from `5 goods, 19 hexes, ...` to `6 goods, 19 hexes, 2 resources, 10 deposits`. Optionally add a currency error (a missing `currency` key, or `unknown good`) to the sample error listing; if so, keep the closing error count consistent.
- **"The world panel" (developer UI)**: the sample's Goods list now includes `silver` (0 held). No wording change is needed unless the manual lists the sample's goods there.
- **Intro cell**: can stay as it is; optionally mention that each hex names a currency good.

## Links to docs/

- [docs/currency.md](docs/currency.md) (new), for how currencies are stored and checked. Link it from the new currency paragraph in "The map".
- [docs/map.md](docs/map.md) is already linked from "The map".

## Examples or diagrams

- A short TOML example of two hexes with different currencies (for example `coin` and `shell`) would show that hexes may differ, next to the sample's uniform `silver`.
- No diagram or interactive demo is needed. The existing map diagram doesn't change, and labelling every hex `silver` would add nothing.
