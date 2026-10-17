# Step 02: businesses — Manual Changes

Nothing new appears in the developer UI window: businesses can't be inspected there until Step 09. The changes are for setting authors, plus corrections where the manual says deposits have no owners and where it describes what the sample's center hex holds. This builds on Step 01's draft (technologies). Step 01's success line for the checker is extended here (see below).

## What users can now see or do

- Setting authors can place **businesses** with a new `[[business]]` section in `start/`. A business is a named actor on one hex. It holds a starting inventory, owns some of the deposits on its hex, and names the decider that will control it. Six required keys:
  - `name`: unique among businesses (its own namespace, separate from goods, resource types, POP types and technologies);
  - `q`, `r`: the hex it sits on;
  - `inventory`: goods = amounts it starts with, each greater than 0. Its **cash** is just the hex's currency listed here (no separate cash key; none listed means it starts with no cash);
  - `deposits`: resource type = how many deposits of that type it owns on its hex, each a whole number of at least 1;
  - `decider`: a name. For now it is only stored; a later Step connects it to an actual decision-maker.

  `inventory` and `deposits` may be empty (`{}`) but must be written. Both TOML table forms work, as for a POP type's endowment.
- Rules the manual should state plainly:
  - A business sits on one hex and can own only deposits on that hex. A hex can hold several businesses, and a business can sit on a hex with no market.
  - Ownership is written on the business, not on the `[[deposit]]` entry, which doesn't change. The loader hands out deposits of each type in order, with businesses taken in file order, so the same folder always gives the same owners. Claiming more deposits than the hex has left unowned is an error, and the message says how many were available.
  - A `[[business]]` in `rules/` is an error.
- **What changes in a running world.** A deposit that a business owns now produces into that business's stockpile instead of the hex's stockpile of unowned goods. Unowned deposits work as before. A business's stockpile starts with its inventory, and its goods perish like everyone else's: labor listed in an inventory is gone at the end of turn 0, while silver stays. Nothing else happens to businesses yet. They don't trade, produce or decide anything, so the turn reports are unchanged: `emitted` still counts every deposit, and the owned output perishes from the business stockpiles at the same time it would have perished from the hex's.
- The checker's success line now ends with a business count. The sample's line becomes `Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses`, with `1 business` as the singular form. This extends the line given in Step 01's manual draft.
- **The sample setting** gains `start/businesses.toml`: three businesses on the center hex `(0, 0)`, one for each technology in the chain, all with decider `arithmetic`:
  - `center_farm`: 100 silver, owns 2 of the center's 3 arable land deposits;
  - `center_mine`: 50 silver, owns the center's copper ore deposit;
  - `center_toolmaker`: 200 silver, owns no deposit (it will buy its copper).

  The center's third arable land deposit and every deposit on other hexes stay unowned.
- **What users see differently in the developer UI** (no new controls, but numbers change):
  - The center hex's stockpile now shows only `4 unimproved_grain` (from its one unowned deposit), not 12 grain and 2.5 copper. Its deposit lines still list all four deposits with their full totals, so the deposit total and the stockpile no longer match on that hex.
  - In the world panel's Goods table, **In hexes** drops for the raw goods (sample: unimproved copper 7.5, unimproved grain 16), because owned output no longer sits in hex stockpiles. Business holdings appear in neither column yet, so the farm's grain, the mine's copper and the businesses' silver aren't shown anywhere.

## Where it fits

- **New subsection "Businesses"** under "Writing a setting", after "Technologies" (Step 01's new subsection) and before "Running a setting", with an entry in the Contents list. It should cover the keys, a short TOML example (the sample's `center_farm` inline, and a small one in sub-table form with an empty `[business.deposits]`), the rules above, the error list in prose as the other sections do (missing keys, bad or duplicate names, bad coordinates or no hex there, amounts not above 0, counts not whole or below 1, unknown goods and resource types, claiming more deposits than are left, a bad decider name, unknown keys, the wrong folder; the reference checks and the deposit assignment are skipped while part of the setting couldn't be read), and the sample's three businesses, perhaps as a table.
- **Natural resources:** replace "There is no ownership yet (that is a later stage)" and "Each will be ownable by a single owner later" with: a deposit can be owned by one business, set in the business's entry (link to the Businesses subsection), and an owned deposit produces into its owner's stockpile.
- **POPs:** the bullet "Deposits are still unowned: their output goes to the hex's unowned stockpile, not to the POPs there" needs rewording: POPs still never receive deposit output; it goes to the owning business or, if unowned, to the hex.
- **Running a setting:** the opening paragraph (the world holds a stockpile of unowned goods per hex and one per POP) gains one stockpile per business, filled from its starting inventory. Step 1 of the turn ("into the unowned stockpile of its hex (deposits have no owners yet)") becomes: into the owner's stockpile if a business owns the deposit, otherwise into the hex's. Step 3 (perishing) covers business stockpiles too. Add a sentence that businesses don't act in the middle part of the turn yet.
- **Turn reports:** a short note that `emitted` and `perished` count business stockpiles too, so the sample's lines are unchanged.
- **Checking a setting:** update the sample's success line, add `1 business` to the singular forms, and optionally add one business error to the sample error block (for example the "claims more deposits than the hex has unowned (3 claimed, 1 available)" message).
- **The developer UI:** under "Inspecting a hex", note that a hex's stockpile holds only its unowned deposits' output, so the center hex shows less than its deposits produce. Under "The world panel", update the "In hexes" description (unowned deposit output only) and say that business holdings aren't counted yet. "Not available yet" gains "inspecting businesses and their stockpiles".
- The intro to "Writing a setting", which lists what a setting describes, can add "and the businesses that own deposits".

## Links to docs/

- [docs/businesses.md](docs/businesses.md) for the keys, every error message, how deposits are handed out, and business stockpiles.
- [docs/resources.md](docs/resources.md) for deposits and their owners.
- [docs/world.md](docs/world.md) for emission to owners and the order of perishing.
- [docs/currency.md](docs/currency.md) for why cash is just the hex's currency.
- [docs/setting-loader.md](docs/setting-loader.md) for the checker's success line.

## Examples, diagrams, demos

- Reuse the existing `hex_svg` helper for a small map of the center hex showing who owns what: "center_farm: arable_land x2", "center_mine: copper_ore x1", "unowned: arable_land x1", with "center_toolmaker" listed as owning nothing. It could sit next to the deposit map in Natural resources or in the Businesses subsection.
- A small table or diagram of where the center hex's output goes each turn: 8 unimproved grain to `center_farm`, 2.5 unimproved copper to `center_mine`, 4 unimproved grain to the hex stockpile. Together that is the 14.5 the deposit lines show, and the 4 is what the dev UI's hex stockpile shows.
- A worked example of deposit assignment with two businesses on one hex sharing three deposits of one type (the first claims 2, the second 1; a third business claiming 1 more gets the error with "0 available"), so authors see that file order decides who gets which unit, and that it doesn't matter because the units are identical.
