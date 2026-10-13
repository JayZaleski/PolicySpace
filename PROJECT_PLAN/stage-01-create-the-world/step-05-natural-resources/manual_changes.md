# Step 05: natural-resources — Manual Changes

No change for players. Setting authors can now define natural resources, so this extends the "Writing a setting" section drafted in Step 02 and extended in Steps 03 and 04.

## What the manual should cover
- What natural resources are: resource types say what kinds of resource exist and what one deposit produces; deposits put them on the map. Nothing is produced yet (that comes with the turn clock) and there is no ownership yet (a later Stage).
- Resource types: `[[resource]]` sections in `rules/` (in `start/` it is an error), in any file at any depth, merged across files. Three required keys: `name` (lowercase snake_case, unique among resource types; it may match a good's name), `produces` (the name of a good the setting defines), and `output` (how much of that good one deposit produces per period; a whole number or decimal, finite and greater than 0). Each type produces exactly one good, and several types may produce the same one. By convention the produced good is unmovable and perishes after one period (like the sample's `unimproved_*` goods), but the engine does not require it.
- Deposits: `[[deposit]]` sections in `start/` (in `rules/` it is an error). Keys: `resource`, `q`, `r` (same coordinates as hexes, and a hex must exist there), and optional `count` (default 1, from 1 to 1,000,000). Explain that `count = 3` makes three separate deposits, each of which a single owner will be able to own later, so the total on a hex is `count` times `output`. A hex can hold several resource types, but each type may appear once per hex: raise `count` instead.
- References: a resource's `produces`, a deposit's `resource`, and a deposit's hex are checked after all files are read, so the target may be in any file. Mention that these errors are not shown when part of the setting could not be read (consistent with the Step 03 note).
- The sample setting: the two resource types (`copper_ore` producing `unimproved_copper`, output 2.5; `arable_land` producing `unimproved_grain`, output 4) and the six deposit entries placing 10 deposits, as tables.
- The setting checker's success line now ends with resource and deposit counts (`... (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits`, with `1 resource` and `1 deposit` wording). Update the example output from Steps 02, 03, and 04.
- Update earlier text: the Step 04 note that `start/` holds only the map becomes "the map and the deposits", and the sample's `rules/` now holds `resources.toml` as well as `goods.toml`.
- Briefly, what is planned later: stock, depletion, and regeneration as optional data on resource types, so settings written now stay valid.

## Where it fits
Inside the "Writing a setting" section, as a new subsection "Natural resources" after "The map" (deposits sit on the map and refer to goods).

## Links
- `docs/resources.md` for the data model, loading, and reference checking.
- `docs/map.md` for coordinates, and `docs/goods.md` for goods.
- `docs/setting-loader.md` for error reporting and the checker.

## Example
Show the TOML example from the design (`copper_ore` with `output = 2.5`, a deposit with `count = 2`, and one without `count`). A diagram would help: the sample's 19-hex map from Step 04 with the deposits marked on their hexes (for example, a small label of type and count on (0, 0), (-1, 0), (1, 0), (1, -1), and (-2, 1)), showing two types on one hex. Also show one or two sample error lines (for example `unknown good 'gold'` at the `produces` value, and a duplicate deposit naming both places). An interactive demo is not needed.
