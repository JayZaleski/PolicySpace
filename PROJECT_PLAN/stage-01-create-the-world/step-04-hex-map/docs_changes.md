# Step 04: hex-map — Docs Changes

## docs/map.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# The map

Source: `crates/engine/src/map.rs` (the data model) and `crates/engine/src/setting/map.rs` (reading `[[hex]]` sections). The public types are re-exported from the engine crate root: `HexCoord`, `HexId`, `HexKind`, `HexMap`.

The map is the set of hex tiles in a setting's starting state. A hex carries only its position for now; terrain and other attributes come in later Steps. `map.rs` knows nothing about files: the dependency runs from `setting` to `map` only.

## Coordinates and orientation

Coordinates are **axial** (`q`, `r`), both `i32`. The engine fixes the orientation; settings do not choose it:

- Hexes are **pointy-top**.
- Each value of `r` is a straight east-west row.
- `q` increases eastward; `r` increases southward.

The dev UI (Step 07) draws the map this way. Fixing the orientation is what lets wrapping be added later without changing how hexes are written.

There are no shape rules. Any set of distinct coordinates is a valid map: holes, separate islands and negative coordinates are all fine, and an empty map is valid. Neighbors, distance and wrapping do not exist yet; nothing needs them until movement and trade.

## Types

| Type | Meaning |
|---|---|
| `HexKind` | Marker enum implementing `IdKind` (`LABEL = "hex"`) |
| `HexId` | `Id<HexKind>`. Valid only for the `HexMap` that produced it |
| `HexCoord` | `{ q: i32, r: i32 }` with `HexCoord::new(q, r)`. Derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`. `Display` prints `(q, r)`, for example `(1, -2)` |
| `HexMap` | Every hex in load order. Immutable once built |

A hex is identified by its coordinates, not by a name, so there is no `NameRegistry` for hexes. `HexMap` hands out `HexId`s directly with the crate-private `Id::from_index`, numbering them 0, 1, 2, ... in load order. Anything saved long-term must store coordinates, never `HexId`s.

`HexMap` stores `coords: Vec<HexCoord>` (indexed by `HexId::index`) and a `by_coord: HashMap<HexCoord, HexId>` used for lookup only. Iteration walks the vector, never the hash map, so order is deterministic.

| Method | Result |
|---|---|
| `len()`, `is_empty()` | Number of hexes |
| `id(coord) -> Option<HexId>` | The hex at `coord`, or `None`. "No hex there" is an ordinary answer, not an error type; callers such as Step 05 write their own message |
| `coord(id) -> HexCoord` | The coordinates of `id`. Panics (`is not from this map`) if the ID is foreign |
| `iter() -> (HexId, HexCoord)` | Every hex in load order, which is also ID order |

`HexMap::default()` is the empty map. `HexMap::from_coords(Vec<HexCoord>)` is crate-private, so a `HexMap` always comes from a checked setting; it panics (`duplicate hex`) on duplicate coordinates, which the loader rejects first.

## How hexes are loaded

`setting/map.rs` holds `MapReader`, the loader-side accumulator. `WorldContent` has a `map: MapReader` field, and the `hex` section type (folder `Start`) calls `MapReader::read(reader)` for each `[[hex]]` entry. `read`:

1. Records the `[[hex]]` header's location (`EntryReader::location`).
2. Reads `q`, then `r`, with `required_integer` and converts each to `i32`. A value outside `i32` is reported at the value as `key 'q' in hex is out of range`. Both keys are always read, so neither is falsely reported as unknown.
3. If both are valid, checks `by_coord` (coordinates to index). A repeat is reported at the second header as `duplicate hex at (0, 0) (first defined at start/a.toml:1:1)` and not placed. Otherwise the coordinates and location are appended.

A hex with a missing or invalid coordinate is never placed, so it cannot cause a misleading duplicate error; the load fails anyway. The loader calls `EntryReader::finish` afterwards, which reports unknown keys such as `terrain`.

Duplicates are detected by the reader's own coordinate table, not by `Definitions`, which is keyed by names. Hexes contain no references, so `resolve` has nothing to do for them.

`MapReader::finish` calls `HexMap::from_coords`, and `load_setting` stores the result in `StartState::map`. IDs follow load order: file read order (`start/` files in byte order of relative path), then the order of hexes within each file. A `[[hex]]` in `rules/` gets the usual "belongs in start/" error.

## Using the map

```rust
let map = &setting.start.map;
for (id, coord) in map.iter() { /* load order */ }
let id = map.id(HexCoord::new(0, 0));   // Option<HexId>
```

## Planned extension: wrapping

Wrapping is planned as an optional per-setting map value (a wrap width along `q`, off by default), so existing settings are unaffected. North-south wrapping could be added as extra neighbor rules (torus, or pole crossing). A true geodesic globe is not supported: axial coordinates cannot describe its 12 pentagons.

## The sample setting and tests

- `settings/sample/start/map.toml` defines a hexagon of radius 2 around (0, 0): 19 hexes, listed row by row from north to south and west to east. Its header comment explains the coordinates.
- `map.rs` unit tests cover sequential IDs, the accessors, the default map, `Display`, and both panics.
- `crates/engine/tests/map.rs` loads the sample and test settings through the public API.
- `crates/engine/tests/setting_loader.rs` also checks the sample's hex count.
- Test settings: `hexes_valid` (hexes across several files including a nested folder, negative coordinates and the `i32` extremes), `hexes_one`, and `hexes_errors` (missing, wrongly typed and out-of-range coordinates, an unknown key, one hex with several problems, a duplicate across files, a `[[hex]]` in `rules/`).

## Adding a hex attribute

1. Add the field to the hex data in `map.rs`. `HexMap` currently stores a bare `HexCoord` per hex, so this means storing a small struct instead. Document the default.
2. Read it in `MapReader::read` with the matching `EntryReader` accessor (reading it also marks the key as known).
3. Update the `hexes_*` test settings and the sample map.

## docs/setting-loader.md — modify (hex section type)
- **Location:** intro paragraph; "Section types and content" (`WorldContent` paragraph); "Adding a section type"; module table; "The sample setting and test settings"; "Setting checker (dev UI)"
- **Change:** replace
- **Supersedes:** step-03-goods-and-properties/docs_changes.md (the intro's "one real section type so far", the `WorldContent` paragraph's "one section type" and "`StartState` still has no fields", the sample setting's "`start/` still holds only a README", and the checker's success output)

- Intro: replace "The loader defines one real section type so far, `[[good]]` (see [goods.md](goods.md)); later Steps add more." with: The loader defines two real section types so far, `[[good]]` in `rules/` (see [goods.md](goods.md)) and `[[hex]]` in `start/` (see [map.md](map.md)); later Steps add more.
- `WorldContent` paragraph: `WorldContent` is the real content type. It holds a `GoodsReader` and a `MapReader` (it no longer holds a `StartState`), and returns two section types: `good` in `rules/` and `hex` in `start/`. `load_setting` builds `Rules { goods }` and `StartState { map }` from it. Because the loader is generic over the content type, tests supply their own (`TestContent`) as before.
- Module table: add row `map.rs` | `MapReader`: reads `[[hex]]` entries and builds `HexMap`.
- "Adding a section type", step 1: add that `[[hex]]` is the worked example for a starting-state section type and for a kind identified by something other than a name. Its reader keeps its own table of what makes an entry unique (coordinates) and checks duplicates itself, instead of using `Definitions`. Domain types are in `crate::map`, the reader in `setting/map.rs`, and the finished map is `StartState::map`.
- `WorldContent::resolve`: goods and hexes refer to nothing, so there is nothing to check yet.
- `EntryReader::location` is now used (by `MapReader`), so it no longer carries `#[allow(dead_code)]`.
- Sample setting: `settings/sample/start/map.toml` now defines 19 hexes (see [map.md](map.md)); `start/` also holds its README. Add `hexes_valid`, `hexes_one` and `hexes_errors` to the list of test setting folders.
- Setting checker table, one-path success row: prints `Loaded setting '<name>' (format version <n>): <goods>, <hexes>`, for example `5 goods, 19 hexes`, `1 good, 0 hexes`, `0 goods, 1 hex`. Hexes use `count(n, "hex", "hexes")`. Failure output, exit codes and the no-argument form are unchanged.

## docs/ids.md — modify
- **Location:** "Rules" and "Adding a new ID kind"
- **Change:** replace
- **Supersedes:** step-01-engine-workspace/docs_changes.md (the rule that setting files always refer to things by name and that every ID comes from a registry)

- Rules: most kinds are named, and a `NameRegistry` turns each name into an ID. Some kinds are identified by other data, such as hexes by their coordinates; their list hands out IDs directly in load order with the crate-private `Id::from_index`. Anything saved long-term stores names or that identifying data, never IDs.
- Adding a new ID kind: add a second path to the steps. If the kind has no name, the list type (like `HexMap`) assigns sequential IDs itself and keeps its own lookup from the identifying data to the ID; see [map.md](map.md). `HexKind` / `HexId` is the example.
- Note for the applier: the `compile_fail` example on `Id` still registers a hex by name (hexes have no names), and the item docs for `from_index` and `index` still say "registries". This is a known follow-up (spec F1) for comp-review. The Step 01 "Tests" paragraph describing that example should be revisited if comp-review changes it.

## docs/workspace.md — modify
- **Location:** Layout block
- **Change:** add
- **Supersedes:** none

Under `crates/engine/` add `src/map.rs` (hex map data model, see map.md) and `tests/map.rs`; under `src/setting/` add `map.rs`.

## docs/README.md — modify
- **Location:** the table of documents, after the `goods.md` row
- **Change:** add
- **Supersedes:** none

| [map.md](map.md) | Hex coordinates and orientation, `HexMap`, how `[[hex]]` sections are loaded, and how to add a hex attribute |
