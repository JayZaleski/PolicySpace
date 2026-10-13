# Step 04: hex-map

## Summary
This Step adds hex tiles to the setting format: the first kind of content in the starting state. Settings define their map in `start/` as a set of hexes, each placed on the grid by axial coordinates and carrying nothing else. Terrain and other attributes come later, when movement and trade need them. A hex is identified by its coordinates, not by a name, so Step 05 can place deposits on hexes by position. The engine fixes the grid's orientation (pointy-top, with east–west rows) so that wrapping, needed for world maps, can be added later without changing how hexes are written. This Step also adds a 19-hex map to the sample setting, and adds a hex count to the setting checker.

## Behavior

### Defining hexes
- Hexes are defined in `[[hex]]` sections. They belong in `start/`, because the map is part of what turn 0 looks like. A `[[hex]]` in `rules/` is an error saying it belongs in `start/`.
- Any file in `start/`, at any depth, may hold any number of hexes. Hexes from every file are merged into one map.
- Each hex has exactly two keys, both required whole numbers: `q` and `r`, its axial coordinates. Any other key (for example `terrain`) is an unknown-key error.
- A hex has no name. Its coordinates are its identity.
- Two hexes at the same coordinates is an error, even across files, and the error names both places.
- Hexes get their IDs in load order: the order files are read, then the order of hexes within each file. The same folder always gives the same IDs.

Example:
```toml
[[hex]]
q = 0
r = 0

[[hex]]
q = 1
r = -1
```

### Coordinates and orientation
- Coordinates are axial, the standard system for hex grids: two numbers per hex.
- The engine fixes the orientation; settings don't choose it. Hexes are **pointy-top**. Each value of `r` is a straight east–west row, `q` increases eastward, and `r` increases southward. The dev UI (Step 07) draws the map this way.
- `q` and `r` may be any whole number in the 32-bit signed range (about ±2.1 billion), including negative numbers.

### Map shape
- There are no shape rules. Any set of distinct coordinates is a valid map: holes, separate islands, and negative coordinates are all fine. Settings such as galaxies or archipelagos can take whatever shape they need.
- A setting with no hexes is valid.

### Errors
Each is reported at the right file, line, and column:
- a missing `q` or `r`;
- a `q` or `r` that isn't an integer;
- a `q` or `r` outside the 32-bit range;
- an unknown key in a `[[hex]]`;
- two hexes at the same coordinates (naming both places);
- a `[[hex]]` in `rules/`.

As with every section, all of a hex's problems are reported in one load. A hex whose coordinates are missing or invalid isn't placed on the map, so it can't produce a misleading duplicate error. The load fails anyway.

Hexes don't refer to anything, so they add nothing to the reference check.

### What a loaded setting provides
- The loaded setting's starting state holds the map. Through the engine's public interface, a caller (the dev UI, and the world state in Step 06) can:
  - list every hex in load order, which is also ID order, with its coordinates;
  - look up a hex's ID by its coordinates, with a clear "no such hex" result when there's none;
  - get a hex's coordinates from its ID;
  - get the number of hexes.

### The sample setting
- `settings/sample/start/map.toml` defines a hexagon of radius 2 centred on (0, 0): 19 hexes. These are every hex whose `q`, `r`, and `q + r` are all between −2 and 2.
- Hexes are listed row by row from north to south, and west to east within each row:

| `r` | `q` values | Hexes |
|---|---|---|
| −2 | 0, 1, 2 | 3 |
| −1 | −1, 0, 1, 2 | 4 |
| 0 | −2, −1, 0, 1, 2 | 5 |
| 1 | −2, −1, 0, 1 | 4 |
| 2 | −2, −1, 0 | 3 |

- A comment at the top of the file explains axial coordinates and the orientation.
- The sample setting still loads without errors.

### The setting checker
- On success, the dev UI's setting checker adds a hex count after the goods count: `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes`. With exactly one hex it says `1 hex`; with none, `0 hexes`.
- Running with no argument, and the failure output, are unchanged.

### Test settings
- Small test settings cover hexes: a valid map across several files (including a nested folder, and negative coordinates), a one-hex map, and a map that triggers each error above.

## Player-facing Changes
None for players. Setting authors can define a map, and the sample setting shows how. The setting checker reports how many hexes a setting defines.

## Interactions
- **Step 01 (engine-workspace):** hexes are a new ID kind.
- **Step 02 (setting-loader):** `[[hex]]` is the first real section type in `start/`. The loader's duplicate check so far works on names; hexes need the same check on coordinates.
- **Step 03 (goods-and-properties):** follows its pattern (a map module that knows nothing about files, plus a small reader in the loader), and extends the setting checker's success line it introduced.
- **Step 05 (natural-resources):** deposits are placed on hexes by coordinates, and a deposit on coordinates with no hex is an error there.
- **Step 06 (turn-clock):** builds world state from the loaded map.
- **Step 07 (dev-ui-map):** draws the map pointy-top using the coordinates.
- **Step 08 (dev-ui-inspect-and-step):** lets the developer inspect hexes.

## Out of Scope
- **Wrapping** (the east edge bordering the west edge). It only changes which hexes are neighbors, and nothing uses neighbors until movement and trade arrive after Stage 04. When it's added, it will be an optional per-setting map value (a wrap width along `q`), defaulting to no wrap, so settings like galaxies are unaffected and existing settings keep working. The orientation fixed in this Step is what makes that possible.
  - North–south wrapping can be added the same way, as extra neighbor rules on top of the flat grid: either a torus (the north edge borders the south edge) or pole crossing (leaving the top row comes back onto the top row half the map's width away). Pole crossing keeps the polar distortion of a flat map, as on a Mercator projection.
  - A true globe with roughly equal-area tiles (a geodesic grid, which needs 12 pentagons among the hexes) is not supported. Axial coordinates can't describe it, and adopting one would mean changing how hexes are identified.
- Neighbors, distance, and any other grid geometry.
- Terrain, names, or any other attribute of a hex.
- Shortcuts for writing large maps (filled rectangles, hexagons of radius N).
- Flat-top orientation, or letting a setting choose its orientation.
- Drawing the map (Step 07).

## Acceptance Criteria
- The sample setting loads with the 19 hexes in the table, in that order.
- A valid test setting with hexes across several files, including a nested folder and negative coordinates, loads. The hexes get IDs in load order, and loading it twice gives the same IDs.
- For each hex, the loaded setting gives its coordinates, and looking up those coordinates gives its ID. Looking up coordinates with no hex gives a clear "no such hex" result.
- A setting with no hexes loads.
- Each of these is an error at the right file, line, and column:
  - a `[[hex]]` missing `q` or `r`;
  - a `q` or `r` that isn't an integer;
  - a `q` or `r` outside the 32-bit range;
  - an unknown key in a `[[hex]]`;
  - two hexes at the same coordinates in two files (the error names both places);
  - a `[[hex]]` in `rules/` (the error says it belongs in `start/`).
- One hex with several problems reports all of them in one load, and a hex with a missing or invalid coordinate causes no duplicate error.
- The dev UI with the sample setting's path prints `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes` and exits successfully. A setting with one hex says `1 hex`; one with none says `0 hexes`.
- The dev UI with no argument, and with a broken setting, behaves as before.
- The single check command passes.
