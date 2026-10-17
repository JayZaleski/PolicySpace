# Step 03: factories — Manual Changes

Nothing new appears in the developer UI window: factories can't be inspected there until Step 09. The changes are for setting authors, plus the checker's success line. This builds on the drafts of Step 01 (technologies) and Step 02 (businesses); it extends Step 02's success line (see below) and turns several "factories come later" remarks in those drafts into present tense.

## What users can now see or do

- Setting authors can place **factories** with a new `[[factory]]` section in `start/`. A factory is an asset on a hex that runs one technology and belongs to one business. Keys:
  - `q`, `r` (required): the hex it sits on. For now this must be its owner's hex.
  - `technology` (required): the name of a technology from `rules/`.
  - `owner` (required): the name of a business. Every factory has exactly one owner.
  - `condition` (optional, default 1): how much of its technology's capital the factory still holds, as a share from just above 0 up to 1. 1 is a factory in full condition; 0.75 is one with three quarters of its capital left. A factory can't be placed at 0.
  - `count` (optional, default 1): place that many identical factories at once, from 1 to 1,000,000, the same rule as a deposit's `count`.
- Rules the manual should state plainly:
  - Factories have no names. Each gets a number (its ID) in load order, so the same folder always numbers them the same way; later features and the dev UI refer to factories by that number.
  - Why one number for the capital: wear and maintenance always change a factory's capital as a whole, in the technology's own proportions, so the share is all that needs storing. The actual goods in a factory are the technology's capital bundle times the condition (the sample's mine at 0.75 holds 1.5 of its 2 tools).
  - A hex or a business may have any number of factories, of any technologies. Nothing checks that a factory makes economic sense.
  - The factory must be on its owner's hex; this restriction will be lifted once trade between hexes exists.
  - A `[[factory]]` in `rules/` is an error.
- **Nothing runs yet.** Factories are loaded, checked and held in the world, but they don't produce, wear out or appear in the turn reports. The sample's turn reports are unchanged.
- The checker's success line now ends with a factory count. The sample's line becomes `Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories`, with `1 factory` as the singular form. This replaces the line given in Step 02's manual draft.
- **The sample setting** gains `start/factories.toml`: one factory per business on the center hex, each running the technology the business is named for. The farm and the toolmaker are in full condition; the mine's factory starts worn at 0.75 (six turns of its eight-turn lifetime left), so the next Step has a worn factory to work on.

## Where it fits

- **New subsection "Factories"** under "Writing a setting", after "Businesses" (Step 02's new subsection) and before "Running a setting", with an entry in the Contents list. It should cover the keys, a short TOML example (the sample's worn mine, and perhaps an entry with `count = 2`), the rules above, the condition explained in plain terms, and the errors in prose as the other sections do (missing keys, bad coordinates or no hex there, an unknown technology or business, a factory not on its owner's hex, a condition that isn't a number or is 0 or below, above 1, infinite or NaN, a count that isn't whole or is out of range, unknown keys, the wrong folder; the reference checks, including the owner's-hex check, are skipped while part of the setting couldn't be read). It can list the sample's three factories, perhaps as a table.
- **Technologies** (Step 01's subsection): where it says factories will run technologies later, say that factories now exist and hold a share of their technology's capital bundle (link to the Factories subsection), but don't run yet.
- **Businesses** (Step 02's subsection): note that a business can own factories on its hex, and that factories are why businesses have names.
- **Running a setting:** one sentence that the world holds the factories but nothing in the turn uses them yet.
- **Checking a setting:** update the sample's success line, add `1 factory` to the singular forms, and optionally add a factory error to the sample error block (for example `factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)`).
- **The developer UI:** "Not available yet" gains "inspecting factories" (Step 09).
- The intro to "Writing a setting", which lists what a setting describes, can add "and the factories they own".

## Links to docs/

- [docs/factories.md](docs/factories.md) for the keys, every error message, how condition and count work, and how factories are held in the world.
- [docs/technologies.md](docs/technologies.md) for the capital bundle the condition is a share of, and what depreciation and maintenance will mean.
- [docs/businesses.md](docs/businesses.md) for owners and their hexes.
- [docs/setting-loader.md](docs/setting-loader.md) for the checker's success line.

## Examples, diagrams, demos

- Extend the center-hex diagram suggested in Step 02's draft (who owns which deposits, using `hex_svg`) with each business's factory: `center_farm`: farm, `center_mine`: mine (condition 0.75), `center_toolmaker`: toolmaker.
- A small worked table of what the condition means in goods for the sample: the farm holds 4 tools (4 x 1), the mine 1.5 tools (2 x 0.75), the toolmaker 3 tools (3 x 1). It shows readers that the condition is a share of the technology's capital bundle, not an amount.
