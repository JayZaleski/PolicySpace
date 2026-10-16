# The map

Source: `crates/engine/src/map.rs` (the data model) and `crates/engine/src/setting/map.rs` (reading `[[hex]]` sections). The public types are re-exported from the engine crate root: `HexCoord`, `HexId`, `HexKind`, `HexMap`.

The map is the set of hex tiles in a setting's starting state. A hex carries only its position for now; terrain and other attributes come later. Each `[[hex]]` entry also names the hex's currency, which the map reader reads but which is stored separately in `HexCurrencies`, not in the map (see [currency.md](currency.md)). `map.rs` knows nothing about files: the dependency runs from `setting` to `map` only.

## Coordinates and orientation

Coordinates are **axial** (`q`, `r`), both `i32`. The engine fixes the orientation; settings do not choose it:

- Hexes are **pointy-top**.
- Each value of `r` is a straight east-west row.
- `q` increases eastward; `r` increases southward.

The dev UI draws the map this way (see [dev-ui.md](dev-ui.md)). Fixing the orientation is what lets wrapping be added later without changing how hexes are written.

There are no shape rules. Any set of distinct coordinates is a valid map: holes, separate islands and negative coordinates are all fine, and an empty map is valid. Neighbors, distance and wrapping do not exist yet; nothing needs them until movement and trade between hexes.

## Types

| Type | Meaning |
|---|---|
| `HexKind` | Marker enum implementing `IdKind` (`LABEL = "hex"`) |
| `HexId` | `Id<HexKind>`. Valid only for the `HexMap` that produced it |
| `HexCoord` | `{ q: i32, r: i32 }` with `HexCoord::new(q, r)`. Derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`. `Display` prints `(q, r)`, for example `(1, -2)` |
| `HexMap` | Every hex in load order. Immutable once built |

A hex is identified by its coordinates, not by a name, so there is no `NameRegistry` for hexes. `HexMap` hands out `HexId`s directly, numbering them 0, 1, 2, ... in load order (see [ids.md](ids.md)). Anything saved long-term must store coordinates, never `HexId`s.

`HexMap` stores `coords: IdVec<HexKind, HexCoord>` (indexed by `HexId`) and a `by_coord: HashMap<HexCoord, HexId>` used for lookup only. Iteration walks the `IdVec`, never the hash map, so order is deterministic.

| Method | Result |
|---|---|
| `len()`, `is_empty()` | Number of hexes |
| `id(coord) -> Option<HexId>` | The hex at `coord`, or `None`. "No hex there" is an ordinary answer, not an error type; callers such as the deposit reader write their own message |
| `coord(id) -> HexCoord` | The coordinates of `id`. Panics (`is not from this list`) if the ID is foreign |
| `iter() -> (HexId, HexCoord)` | Every hex in load order, which is also ID order |

`HexMap::default()` is the empty map. `HexMap::from_coords(Vec<HexCoord>)` is crate-private, so a `HexMap` always comes from a checked setting; it panics (`duplicate hex`) on duplicate coordinates, which the loader rejects first.

## How hexes are loaded

`setting/map.rs` holds `MapReader`, the loader-side accumulator. `WorldContent` has a `map: MapReader` field, and the `hex` section type (folder `Start`) calls `MapReader::read(reader)` for each `[[hex]]` entry. `read`:

1. Records the `[[hex]]` header's location (`EntryReader::location`).
2. Reads `q`, then `r`, with `coordinate` (a shared helper: `required_integer` converted to `i32`). A value outside `i32` is reported at the value as `key 'q' in hex is out of range`. Then reads `currency` with `required_string` and keeps it unresolved (see [currency.md](currency.md)). All three keys are always read, so none is falsely reported as unknown.
3. If both coordinates are valid, checks `by_coord` (coordinates to index). A repeat is reported at the second header as `duplicate hex at (0, 0) (first defined at start/a.toml:1:1)` and not placed. Otherwise the coordinates, the location and the unresolved currency (`None` if it was missing or not a string) are appended.

A hex with a missing or invalid coordinate is never placed, so it cannot cause a misleading duplicate error, and its currency is not checked; the load fails anyway. A hex whose only problem is its currency is still placed. The loader calls `EntryReader::finish` afterwards, which reports unknown keys such as `terrain`.

Duplicates are detected by the reader's own coordinate table, not by `Definitions`, which is keyed by names. The only reference in a hex is its currency: `MapReader::resolve` (phase 2) resolves each placed hex's currency against the goods. Other readers query the map reader while resolving their own references: `MapReader::id(coord)` gives the `HexId` a coordinate will have in the finished map (deposits, markets and POPs use it), `MapReader::len()` gives the number of placed hexes (markets size their per-hex lists with it), and `MapReader::currency(hex)` gives a placed hex's resolved currency, or `None` before phase 2 or if it didn't resolve (markets use it to reject a priced currency, so `MapReader::resolve` must run first). The `coordinate` helper that reads `q` and `r` is shared by every section placed by coordinates (hexes, deposits, markets and POPs); its message names the entry's label.

`MapReader::finish` returns `(HexMap, HexCurrencies)`: it calls `HexMap::from_coords` and builds the currencies from the phase-2 results, so it is only called after a successful load. `load_setting` stores them in `StartState::map` and `StartState::currencies`. IDs follow load order: file read order (`start/` files in byte order of relative path), then the order of hexes within each file. A `[[hex]]` in `rules/` gets the usual "belongs in start/" error.

## Using the map

```rust
let map = &setting.start.map;
for (id, coord) in map.iter() { /* load order */ }
let id = map.id(HexCoord::new(0, 0));   // Option<HexId>
```

## Planned extension: wrapping

Wrapping is planned as an optional per-setting map value (a wrap width along `q`, off by default), so existing settings are unaffected. North-south wrapping could be added as extra neighbor rules (torus, or pole crossing). A true geodesic globe is not supported: axial coordinates cannot describe its 12 pentagons.

## The sample setting and tests

- `settings/sample/start/map.toml` defines a hexagon of radius 2 around (0, 0): 19 hexes, listed row by row from north to south and west to east, each with `currency = "silver"`. Its header comment explains the coordinates and the currency.
- `map.rs` unit tests cover sequential IDs, the accessors, the default map, `Display`, and both panics.
- `crates/engine/tests/map.rs` loads the sample and test settings through the public API.
- `crates/engine/tests/setting_loader.rs` also checks the sample's hex count.
- Test settings: `hexes_valid` (hexes across several files including a nested folder, negative coordinates and the `i32` extremes, with currencies `coin` and `shell`), `hexes_one`, and `hexes_errors` (missing, wrongly typed and out-of-range coordinates, a missing, wrongly typed and unknown currency, an unknown key, one hex with several problems, a duplicate across files, a `[[hex]]` in `rules/`). Each has a `rules/goods.toml` defining the goods its hexes designate.

## Adding a hex attribute

1. Add the field to the hex data in `map.rs`. `HexMap` currently stores a bare `HexCoord` per hex, so this means storing a small struct instead. Document the default.
2. Read it in `MapReader::read` with the matching `EntryReader` accessor (reading it also marks the key as known).
3. Update the `hexes_*` test settings and the sample map.

This is for attributes that never change during play. A per-hex value that can change (like the currency, or a market's prices) is world state: keep it out of `HexMap`, store it in its own type indexed by `HexId`, build it from the loader alongside the map, and add it to `StartState` and `World`. `HexCurrencies` (see [currency.md](currency.md)) and `HexPrices` (see [market.md](market.md)) are the worked examples.
