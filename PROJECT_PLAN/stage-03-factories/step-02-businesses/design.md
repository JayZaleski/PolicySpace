# Step 02: businesses

## Summary
This Step puts minimal businesses in the starting state. A business is a named actor on one hex with a starting inventory (its cash is the hex's currency in that inventory), a set of deposits it owns, and the name of the decider that controls it. Each business gets an ID and one stockpile for its goods and cash. Emission is reworked so an owned deposit emits into its owner's stockpile while unowned deposits keep emitting into the hex stockpile, which replaces the ownership seam Stage 01 left in emission. Nothing decides or trades yet: factories are Step 03, the decider is Steps 05 to 07, and business trade is Step 08. The sample setting gains the chain's three businesses on the center hex.

## Behavior

### What a business is
A business is state, defined by `[[business]]` entries in `start/` (one in `rules/` gets the usual "belongs in start/" error). It is the first named kind in the starting state: it has a name, unique among businesses, because factories (Step 03) refer to their owner by name. Business names are their own namespace, separate from goods, resource types, POP types and technologies.

A business has:

- **A hex**, given by axial coordinates read exactly as a deposit's or POP's. A business sits on one hex and may own only deposits on that hex. Once trade between hexes exists, businesses are expected to operate across several hexes with this one as their headquarters; until then one hex is all a business can use, and the view it is served (Step 05) is scoped to that hex's prices.
- **A starting inventory**: a table of good = amount, the goods in its stockpile when the world is created. There is no separate cash key: a business's starting cash is the amount of its hex's currency it lists here, zero if it lists none. A business that starts with no cash is a legitimate test case for Step 08's cash cap; the engine doesn't police economics. Each amount must be finite and greater than 0. The table may be empty, but must be written, so an empty one says the author meant it.
- **The deposits it owns**: a table of resource type = count, the number of deposits of each type the business owns on its hex. Deposits of one type on one hex are identical, so which units a business gets doesn't matter; the loader assigns that many not-yet-owned deposits of that type on the hex, in deposit ID order, with businesses taken in load order, so the same folder always gives the same ownership. A count is a whole number of at least 1. If the hex holds fewer unowned deposits of that type than a business claims (counting what earlier businesses took), the loader reports an error at that key, naming how many were available. The table may be empty, but must be written: a business may own no deposits at all (the sample's toolmaker buys its copper), and a business that owns nothing is harmless. `[[deposit]]` entries are unchanged; ownership is written only on the business.
- **A decider name**: required, and a valid name in the same format as every other name, so that Step 07 can match it against a registry of deciders without a second format. This Step only stores it: there is no check that the name means anything, and several businesses may name the same decider.

A hex may hold any number of businesses, and a business may sit on a hex with no market; POPs already do, and Step 08 decides what trading there means.

### Validation
Every key is read before anything is decided, so one entry with several problems reports all of them in one load, and a business with a bad name still takes part in the duplicate check. Errors, each at its place: a missing or non-table key; a bad or duplicate name; a bad, missing or out-of-range coordinate; coordinates that match no hex; an inventory amount that isn't a number, is zero, negative, infinite or NaN; a deposit count that isn't a whole number or is below 1; an unknown good in the inventory; an unknown resource type in the deposits table; a claim of more deposits than the hex has unowned; a bad decider name; an unknown key. Goods, resource types and hexes are references, resolved after every file has been read, so they may be defined in any file; like every reference check, they and the deposit assignment are skipped when the loader couldn't see every file. An invalid amount or count doesn't stop the reference checks. Business IDs follow load order. A setting with no businesses is valid.

### In the engine
Businesses are a list in the starting state, like POPs: each has an ID, a hex, a decider name, and the list can be asked for the businesses on a hex in ID order. The World takes the businesses over as state and gives every business its own stockpile, the same type hexes and POPs use, filled with the starting inventory when the world is created: each good goes in as a group that perishes by the good's own rule at turn 0, so labor listed there is gone at the end of turn 0 and a durable currency stays. Deposit ownership is world state too: each deposit has at most one owner, readable from the World, and nothing changes it yet. Through the World a caller can read the business list, each business's hex and decider name, its deposits, and its stockpile.

### The turn
1. **Start of turn:** every deposit emits, in deposit ID order as now, but an owned deposit's output goes into its owner's stockpile and an unowned deposit's into the hex stockpile. The hex stockpile stays the home of goods nobody owns. Then POPs are endowed as before.
2. **Actors act:** businesses do nothing yet.
3. **End of turn:** perishing covers hex stockpiles, then POP stockpiles, then business stockpiles, each in ID order.

The report's `emitted` stays one total over every deposit, owned or not, and `perished` covers business stockpiles too, so the sample's turn lines are unchanged apart from goods that a business's starting inventory lets perish. What each business holds is read from its stockpile, like a POP's. This Step adds no report list; it is observable through the stockpiles, and Step 09 shows businesses in the dev UI.

### Sample setting
Three businesses on the center hex, one per technology in the chain, each starting with some silver (the center's currency): a farm business owning two of the center's three arable land deposits, a mine business owning the copper deposit, and a toolmaker business owning no deposit. Leaving one arable land deposit unowned puts owned and unowned deposits of one type on one hex, which exercises the ID-order assignment. The deposits on other hexes stay unowned. Their decider names are what Step 06's arithmetic rule will be registered as. The exact numbers are for the spec. The file's header comment explains every key in the style of the other sample files.

## Player-facing Changes
None in the UI. Setting authors get a new section, `[[business]]`, in `start/`, with clear errors. The sample setting gains `start/businesses.toml`. The checker's success line counts businesses. Inspecting businesses in the dev UI is Step 09.

## Interactions
- **Deposits** (Stage 01 Step 05): a business's deposits table refers to resource types, and the loader assigns deposits on the hex; deposits gain an owner in world state.
- **World and turn** (Stage 01 Step 06): emission goes to the owner's stockpile when there is one; the World gains the business list, one stockpile per business, and deposit ownership; perishing reaches business stockpiles. The ownership seam noted in emission is used up.
- **POPs** (Stage 02 Step 04): the model for a placed thing in `start/` with coordinates, a per-hex index and a stockpile in the World.
- **Setting loader** (Stage 01 Step 02) and **named kinds** (goods, resource types, POP types, technologies): businesses are a named kind, the first in `start/`.
- **Currency** (Stage 02 Step 01): starting cash is the hex's currency in the inventory.
- **Later Steps:** Step 03 (factories) names a business as each factory's owner. Step 05 serves each business a view of its hex's prices, stockpile and deposits. Step 07 resolves the decider name. Step 08 trades from and into the business stockpile and uses the cash in it. Step 09 shows businesses in the dev UI.

## Out of Scope
- Factories and factory ownership (Step 03), decisions and the decider (Steps 05 to 07), business trade (Step 08), showing businesses in the dev UI (Step 09).
- Any change of ownership during play, businesses on more than one hex, businesses being founded or failing.
- Resolving or validating decider names beyond their format.
- POP ownership of businesses, budgets, dividends (Stage 06).
- Starting inventories for hexes or POPs.

## Acceptance Criteria
- The sample setting loads with three businesses on the center hex with the expected names, decider names, deposits and starting inventories; two of the center's arable land deposits and its copper deposit are owned, one arable land deposit and every deposit elsewhere are unowned.
- A fresh sample world has each business's stockpile holding its starting inventory plus its deposits' first emission, the center hex stockpile holding only the unowned deposit's output, every other hex stockpile unchanged, and turn 0's report unchanged. Stepping keeps this up every turn, and two worlds from the same setting stepped the same number of times are identical.
- A test setting with businesses across several files, two businesses on one hex sharing the unowned deposits of one type, a business with an empty inventory and no deposits, a business on a hex with no market, an inventory with a perishable good (gone after turn 0) and a durable one (kept), and references to goods, resource types and hexes defined in later files loads and steps as expected.
- Each error case produces one error at the expected place with the expected text: every missing key, a bad or duplicate name, bad coordinates and coordinates with no hex, bad inventory amounts, bad deposit counts, unknown goods and resource types, more deposits claimed than the hex has unowned (including by a second business), a bad decider name, an unknown key, and a `[[business]]` in `rules/`. One entry with several problems reports all of them in one load.
- Reference checks and deposit assignment are skipped when another file is broken or discovery was incomplete.
- The checker's success line counts businesses, zero for settings without them. Every existing test passes, with expectations changed only by the added business count.
