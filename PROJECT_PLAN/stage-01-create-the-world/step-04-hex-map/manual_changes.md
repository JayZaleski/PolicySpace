# Step 04: hex-map — Manual Changes

No change for players. Setting authors can now define a map, so this extends the "Writing a setting" section drafted in Step 02 and extended in Step 03.

## What the manual should cover
- What the map is: the set of hex tiles in the setting's starting state. For now a hex has only a position; terrain and other attributes come later.
- How to define hexes: `[[hex]]` sections in `start/` (a `[[hex]]` in `rules/` is an error), in any file at any depth, any number per file; hexes from all files are merged. Each hex has exactly two required whole-number keys, `q` and `r`. Any other key (such as `terrain`) is an error. Hexes have no names; the coordinates are the identity, and two hexes at the same coordinates is an error naming both places.
- Coordinates and orientation: axial coordinates, explained plainly. The engine fixes the orientation (pointy-top, each `r` is an east-west row, `q` increases eastward, `r` increases southward); settings cannot change it. Values may be negative, within about plus or minus 2.1 billion.
- Shape: no rules. Holes, islands and negative coordinates are fine; a setting with no hexes is valid. Say that neighbors, distance and wrapping do not exist yet, and that wrapping is planned later as an optional per-setting value.
- The sample setting's map: a hexagon of radius 2 around (0, 0), 19 hexes, listed in rows.
- The setting checker's success line now ends with a hex count (`... (format version 1): 5 goods, 19 hexes`, with `1 hex` and `0 hexes` wording). Update the example output from Steps 02 and 03.
- Update the Step 03 note that `start/` holds only a README: the sample's `start/` now has `map.toml`.

## Where it fits
Inside the "Writing a setting" section, as a new subsection "The map" after "Goods". It is the first content for `start/`, so say that `start/` describes what turn 0 looks like.

## Links
- `docs/map.md` for coordinates, orientation, the data model and how hexes are loaded.
- `docs/setting-loader.md` for error reporting and the checker.

## Example
A diagram would help most: the 19-hex sample map drawn pointy-top, each hex labelled with its `(q, r)`, showing that `q` grows eastward, `r` grows southward, and each `r` is a horizontal row. Also show a short TOML snippet (two `[[hex]]` entries) and one or two sample error lines (a duplicate hex naming both places, and an unknown key `terrain`). A static diagram is enough; an interactive demo is not needed.
