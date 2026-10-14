# Step 01: currency — Docs Changes

## docs/currency.md — new file
- **Location:** whole file
- **Change:** add

# Currencies

Source: `crates/engine/src/currency.rs` (the data model) and `crates/engine/src/setting/map.rs` (reading the `currency` key of each `[[hex]]`). The public type is re-exported from the engine crate root: `HexCurrencies`.

Every hex designates one good as its **currency**: the good that prices on that hex are posted in. The designation is required, so every hex has exactly one currency and no code ever handles a hex without one. Different hexes may designate different goods. Nothing uses the currency yet; the dummy market (Stage 02 Step 02) posts prices in it, and POP trade (Step 06) pays and is paid in it.

The engine has no opinion about money. The only requirement is that the good exists in the setting's `rules/`: a perishable or non-movable good (even labor) is a valid currency, and there is no currency-specific good property.

## Why it is state, not a rule, and not part of the map

The designation is starting state (it is written in `start/`) because it can change during play, for example when a central bank issues a synthetic currency in a later Stage. For the same reason it is not stored in `HexMap`: the map is geometry and never changes, while currencies are per-hex state kept in a parallel table, like the hex stockpiles in `World`. Nothing changes a currency yet, and there is no API to change one.

## Data model

```rust
pub struct HexCurrencies { by_hex: IdVec<HexKind, GoodId> }
```

| Method | Result |
|---|---|
| `len()`, `is_empty()` | Number of hexes (equal to the map's hex count) |
| `get(hex) -> GoodId` | The currency of `hex`. Panics (`is not from this list`) if `hex` is not from the map these were built with |
| `iter() -> (HexId, GoodId)` | Every hex's currency, in hex ID order |

`HexCurrencies::default()` is empty (no hexes). The constructor `from_vec(Vec<GoodId>)` is crate-private, so a `HexCurrencies` always comes from a checked setting; entries are in hex ID order. `currency.rs` depends on `goods`, `map` and `id` and knows nothing about files.

A `GoodId` is only meaningful with the `Goods` list of the same setting; look the name up with `rules.goods.name(id)`.

## How currencies are loaded

The `currency` key is part of each `[[hex]]` entry, so `MapReader` (see [map.md](map.md)) reads it alongside `q` and `r`:

1. **Phase 1.** `MapReader::read` reads `currency` with `required_string`, before any early return, so it is never falsely reported as unknown. A missing key is reported at the header (`missing required key 'currency' in hex`); a non-string at the value (`key 'currency' in hex must be a string, found integer`). The value is not name-checked: like a resource type's `produces`, a badly spelled value is simply an unknown good in phase 2. The currency is kept unresolved (`Option<Located<String>>`) in the `currencies` vector, indexed like the hex coordinates. Only placed hexes push a currency: a hex with invalid coordinates is dropped entirely, so its currency is never checked. A hex whose currency is missing or not a string *is* still placed (with `None`), so duplicate-hex and deposit-reference checks keep working; the load fails anyway.
2. **Phase 2.** `WorldContent::resolve` calls `MapReader::resolve(goods.defs(), sink)`, which resolves each placed hex's currency through `Definitions::<GoodKind>::resolve`. An unknown name is reported at the value as `unknown good 'tin'`. The good may be defined in any `rules/` file. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)), so no misleading "unknown good" is shown.
3. **Finish.** `MapReader::finish` returns `(HexMap, HexCurrencies)`. It is only called after a successful load, when every currency has resolved (it panics with an `internal error: ...` message otherwise). `load_setting` stores the currencies in `StartState::currencies`.

The currency for `HexId` n is the one written on the n-th hex loaded, so IDs and order follow the map and the same folder always gives the same result.

## Using currencies

```rust
let setting = load_setting(path)?;
let silver = setting.rules.goods.id("silver")?;
for (hex, good) in setting.start.currencies.iter() { /* hex ID order */ }

let (world, _) = World::new(setting);
let good = world.hex_currency(hex);        // GoodId
let all = world.hex_currencies();          // &HexCurrencies
```

`World::new` moves `StartState::currencies` into the world (see [world.md](world.md)). Stepping the world does not change it.

## Not yet

Changing a hex's currency during play (and what then happens to prices and holdings), any restriction on which goods may be currencies, a map-wide default currency, and showing the currency in the dev UI (Stage 02 Step 08).

## The sample setting and tests

- The sample defines `silver` (durable, movable) in `settings/sample/rules/goods.toml`, and all 19 hexes in `settings/sample/start/map.toml` designate it. The file's header comment notes that hexes may differ.
- `currency.rs` unit tests: IDs in order, `get`, `iter`, the empty default, and the foreign-hex panic.
- `crates/engine/tests/currency.rs`: the sample's hexes are all `silver`; `hexes_valid` gives `coin, shell, coin, shell` by hex ID; `valid_minimal` has no currencies; loading twice gives the same list.
- `crates/engine/tests/world.rs`: `hex_currency` on `turns_valid` (all `gold`) before and after three steps, the sample still `silver` after a step, and the foreign-hex panic.
- `crates/engine/tests/map.rs` (`every_hex_error_is_reported_at_its_place`): the missing key, the integer value, the wrongly cased `'Coin'`, the unknown `'tin'`, and a hex with invalid coordinates whose unknown currency is not reported.
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a hex with an unknown currency, and only the syntax error is reported.
- Test settings that designate currencies: `hexes_valid` (`coin`, `shell`), `hexes_one` and `hexes_errors` (`coin`), and every other setting with hexes (`deposits_errors`, `resources_one`, `resources_valid`, `turns_valid`) names one of its existing goods. The dev UI's `session.rs` tests write temporary settings whose hexes use a good `coin`.

## docs/README.md — modify
- **Location:** the document table, new row after `map.md`
- **Change:** add

| [currency.md](currency.md) | Each hex's currency good: `HexCurrencies`, why it is per-hex state and not part of the map, and how the `currency` key of `[[hex]]` is read and resolved |

## docs/map.md — modify
- **Location:** intro paragraph, the sentence "A hex carries only its position for now; terrain and other attributes come in later Steps."
- **Change:** replace

A hex carries only its position for now; terrain and other attributes come in later Steps. Each `[[hex]]` entry also names the hex's currency, which the map reader reads but which is stored separately in `HexCurrencies`, not in the map (see [currency.md](currency.md)).

## docs/map.md — modify
- **Location:** "How hexes are loaded", the numbered list and the two paragraphs after it (from "`setting/map.rs` holds `MapReader`" through "...the deposit reader uses it.")
- **Change:** replace

`setting/map.rs` holds `MapReader`, the loader-side accumulator. `WorldContent` has a `map: MapReader` field, and the `hex` section type (folder `Start`) calls `MapReader::read(reader)` for each `[[hex]]` entry. `read`:

1. Records the `[[hex]]` header's location (`EntryReader::location`).
2. Reads `q`, then `r`, with `coordinate` (a shared helper: `required_integer` converted to `i32`). A value outside `i32` is reported at the value as `key 'q' in hex is out of range`. Then reads `currency` with `required_string` and keeps it unresolved (see [currency.md](currency.md)). All three keys are always read, so none is falsely reported as unknown.
3. If both coordinates are valid, checks `by_coord` (coordinates to index). A repeat is reported at the second header as `duplicate hex at (0, 0) (first defined at start/a.toml:1:1)` and not placed. Otherwise the coordinates, the location and the unresolved currency (`None` if it was missing or not a string) are appended.

A hex with a missing or invalid coordinate is never placed, so it cannot cause a misleading duplicate error, and its currency is not checked; the load fails anyway. A hex whose only problem is its currency is still placed. The loader calls `EntryReader::finish` afterwards, which reports unknown keys such as `terrain`.

Duplicates are detected by the reader's own coordinate table, not by `Definitions`, which is keyed by names. The only reference in a hex is its currency: `MapReader::resolve` (phase 2) resolves each placed hex's currency against the goods. `MapReader::id(coord)` gives the `HexId` a coordinate will have in the finished map; the deposit reader uses it.

## docs/map.md — modify
- **Location:** "How hexes are loaded", the paragraph starting "`MapReader::finish` calls `HexMap::from_coords`"
- **Change:** replace

`MapReader::finish` returns `(HexMap, HexCurrencies)`: it calls `HexMap::from_coords` and builds the currencies from the phase-2 results, so it is only called after a successful load. `load_setting` stores them in `StartState::map` and `StartState::currencies`. IDs follow load order: file read order (`start/` files in byte order of relative path), then the order of hexes within each file. A `[[hex]]` in `rules/` gets the usual "belongs in start/" error.

## docs/map.md — modify
- **Location:** "The sample setting and tests", first and last bullets
- **Change:** replace

- `settings/sample/start/map.toml` defines a hexagon of radius 2 around (0, 0): 19 hexes, listed row by row from north to south and west to east, each with `currency = "silver"`. Its header comment explains the coordinates and the currency.

- Test settings: `hexes_valid` (hexes across several files including a nested folder, negative coordinates and the `i32` extremes, with currencies `coin` and `shell`), `hexes_one`, and `hexes_errors` (missing, wrongly typed and out-of-range coordinates, a missing, wrongly typed and unknown currency, an unknown key, one hex with several problems, a duplicate across files, a `[[hex]]` in `rules/`). Each has a `rules/goods.toml` defining the goods its hexes designate.

## docs/map.md — modify
- **Location:** "Adding a hex attribute", new paragraph after the numbered list
- **Change:** add

This is for attributes that never change during play. A per-hex value that can change (like the currency) is world state: keep it out of `HexMap`, store it in its own type indexed by `HexId`, return it from `MapReader::finish` alongside the map, and add it to `StartState` and `World`. `HexCurrencies` is the worked example (see [currency.md](currency.md)).

## docs/setting-loader.md — modify
- **Location:** "Module structure", the `map.rs` row of the table
- **Change:** replace

| `map.rs` | `MapReader`: reads `[[hex]]` entries, including each hex's `currency`, and builds `HexMap` and `HexCurrencies`. Its `coordinate` helper is shared with deposits, and `MapReader::id` resolves coordinates to a `HexId` |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph starting "`WorldContent::resolve` is where real references are checked"
- **Change:** replace

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`). The order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the sentence starting "`load_setting` builds `Rules { goods, resources }`"
- **Change:** replace

`load_setting` builds `Rules { goods, resources }` and `StartState { map, currencies, deposits }` from it (`MapReader::finish` returns the map and the currencies together, and comes first because `DepositsReader::finish` needs the hex count).

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the `[[hex]]` bullet
- **Change:** replace

   - `[[hex]]` (`StartState::map`) is the worked example for a starting-state section type and for a kind identified by something other than a name. Its reader keeps its own table of what makes an entry unique (coordinates) and checks duplicates itself, instead of using `Definitions`. `[[deposit]]` (`StartState::deposits`) follows it. A `[[hex]]` also carries a reference, its `currency`, which is stored separately in `StartState::currencies` (see [currency.md](currency.md)).

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 3, the sentence "Resources and deposits are the worked example."
- **Change:** replace

Resources, deposits and hex currencies are the worked examples.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", first bullet
- **Change:** replace

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (six goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types) and `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), plus `README.md` files. It must always load without errors.

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph starting "The success line is"
- **Change:** replace (the example counts change, and one sentence is added)

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>`, for example `6 goods, 19 hexes, 2 resources, 10 deposits` or `1 good, 1 hex, 1 resource, 1 deposit`. Currencies are not counted: there is one per hex, not a new kind of thing. The `count(n, singular, plural)` helper in `text.rs` does the wording; later Steps append their own counts. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

## docs/goods.md — modify
- **Location:** "How goods are loaded", last paragraph ("Goods contain no references, ...")
- **Change:** replace

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Sections that name goods (a resource type's `produces`, a hex's `currency`, and more from Stage 02 onward) resolve them with `Definitions::resolve` against the goods registry, through `GoodsReader::defs()`.

## docs/goods.md — modify
- **Location:** "The sample setting and tests", first paragraph
- **Change:** replace

`settings/sample/rules/goods.toml` defines six goods in this order: `unskilled_labor` (perishes after 1, not movable), `unimproved_copper` (1, not movable), `unimproved_grain` (1, not movable), `copper` (never, movable), `grain` (3, movable), `silver` (never, movable). They cover each combination of the properties and give the resource types something to produce; `silver` is the currency every sample hex designates (see [currency.md](currency.md)).

## docs/world.md — modify
- **Location:** "What a World holds", the field table
- **Change:** add a row after the `map: HexMap`, `deposits: Deposits` row

| `hex_currencies: HexCurrencies` | the setting's `StartState::currencies` | not yet; world state because a later Stage changes a hex's currency (see [currency.md](currency.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, from "Public accessors are read-only" to the end of the paragraph
- **Change:** replace

Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)` and `hex_stockpile(hex)`. `hex_currency` and `hex_stockpile` panic with `is not from this list` for a hex from another map (a caller bug). The turn neither reads nor changes currencies.

## docs/world.md — modify
- **Location:** "Tests", the `crates/engine/tests/world.rs` bullet
- **Change:** replace

- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports, a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits, determinism, the accessors, hex currencies being readable and unchanged by stepping, and the foreign-hex panics of `hex_stockpile` and `hex_currency`.

## docs/resources.md — modify
- **Location:** the paragraph starting "References are checked in `WorldContent::resolve`" (after the `DepositsReader` list)
- **Change:** replace

References are checked in `WorldContent::resolve`, so they may point into any file: hex currencies and resource types against goods, then deposits against resource types and the map. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)). `resources_broken_file` also holds a hex with an unknown currency, which is likewise not reported.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/goods.rs, ...`

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/goods.rs        goods data model (see goods.md)
    src/map.rs          hex map data model (see map.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, goods.rs, map.rs, resources.rs, setting_loader.rs, world.rs
```
