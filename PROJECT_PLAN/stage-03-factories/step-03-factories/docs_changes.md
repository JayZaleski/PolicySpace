# Step 03: factories — Docs Changes

## docs/factories.md — new file
- **Location:** whole file
- **Change:** add

# Factories

Source: `crates/engine/src/factories.rs` (the data model), `crates/engine/src/setting/factories.rs` (reading `[[factory]]` sections), the shared `count` helper in `crates/engine/src/setting/map.rs`, and the factory list in `crates/engine/src/world.rs`. The public types are re-exported from the engine crate root: `FactoryKind`, `FactoryId`, `Factory`, `Factories`.

A **factory** is a discrete asset on a hex that runs one **technology** (see [technologies.md](technologies.md)) and belongs to exactly one **business** (see [businesses.md](businesses.md)). It carries the state that changes during play: which technology it runs, and its **condition**, the share of its technology's full capital bundle it holds. Factories are starting state, defined in `start/` and checked by the loader, and the World takes them over as state.

Nothing runs a factory yet. Running, depreciation and maintenance are Step 04 of Stage 03, Step 05 serves each business its factories in the view, Step 08 wires the mechanics into the turn, and Step 09 shows factories in the dev UI. Until then factories are loaded, checked and held in the world, unchanged.

## Concepts

- **Unnamed.** Factories have no names, like deposits and POPs: nothing refers to a factory by name. Deciders, the dev UI and later decisions address a factory by its `FactoryId`. IDs follow load order (file read order, then entry order, with an entry's `count` copies consecutive), so the same folder always gives the same IDs.
- **Its own hex.** A factory's hex is stored on the factory, not derived from its owner, because businesses will operate across several hexes once trade between hexes exists. Until then the loader requires the factory's hex to be its owner's hex (see [How factories are loaded](#how-factories-are-loaded)). That check is to be relaxed when trade arrives; the data doesn't change.
- **One owner, always.** Every factory has exactly one owning business. There are no unowned factories.
- **Condition, not a goods table.** Depreciation takes a slice of the whole capital bundle and maintenance replenishes a share of the whole bundle (see [technologies.md](technologies.md#what-the-numbers-mean)), so a factory's capital is always its technology's capital bundle times some share. That share is the only real state, stored as `condition: f64`. The goods in a factory are derived from the technology's capital bundle and the condition whenever they are needed. A setting places a factory with a condition greater than 0 and at most 1 (1 when none is written); the engine type accepts 0 too, because once depreciation runs (Step 04) a factory may wear down to nothing.
- **Any number, no economics.** A hex and a business may hold any number of factories, of any technologies. Nothing checks that a factory makes economic sense (that its inputs can be had on the hex, that its owner owns matching deposits, or that its technology is maintainable).

## Data model

```rust
pub enum FactoryKind {}                    // IdKind, label "factory"
pub type FactoryId = Id<FactoryKind>;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Factory {
    pub hex: HexId,                        // its owner's hex, until trade between hexes exists
    pub technology: TechnologyId,          // state: a later Stage can retool it
    pub owner: BusinessId,                 // exactly one
    pub condition: f64,                    // finite, 0 to 1: the share of the capital bundle held
}
pub struct Factories {
    factories: IdVec<FactoryKind, Factory>,  // by FactoryId, in load order
    by_hex: IdVec<HexKind, Vec<FactoryId>>,  // by HexId: the factories on each hex, in ID order
}
```

`Factories` mirrors `Deposits` (see [resources.md](resources.md)): an unnamed list in load order plus a per-hex index.

| `Factories` method | Result |
|---|---|
| `len()`, `is_empty()` | The number of factories; a setting with none is valid |
| `get(id) -> &Factory` | The factory. Panics with `is not from this list` for a foreign ID |
| `iter()` | `(FactoryId, &Factory)` in load order, which is ID order |
| `on_hex(hex) -> &[FactoryId]` | The factories on `hex`, in ID order; empty if none. Panics with `is not from this list` for a hex not from the map the list was built with |
| `owned_by(business)` | An iterator over the `FactoryId`s that business owns, in ID order. A filter over every factory (there is no per-owner index; the lists are short), like `Deposits::owned_by` |

`Factories::from_factories(factories, hex_count)` is crate-private: only the loader builds factories. It numbers the factories 0, 1, 2, ... in the order given and fills `by_hex`, panicking if a factory's hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), if a condition isn't a finite number from 0 to 1 (`"a factory's condition must be a finite number from 0 to 1"`), or if there are more than `u32::MAX` factories. `Factories::default()` is empty and was built with no map, so `on_hex` panics on it for any hex. The fields are private. `factories.rs` depends on `businesses`, `id`, `map` and `technologies` (for the ID types only) and knows nothing about files.

## Setting format

A `[[factory]]` belongs in `start/` (in `rules/` it gets `section 'factory' belongs in start/, not rules/`). Four keys are required and two are optional:

| Key | Meaning |
|---|---|
| `q`, `r` | Integer axial coordinates of a hex in the map, read as a deposit's, POP's or business's are. Must be the owner's hex |
| `technology` | The name of a technology (from `rules/`) |
| `owner` | The name of a business (from `start/`) |
| `condition` | Optional, default 1. The share of the technology's capital bundle the factory holds: a finite number greater than 0 and at most 1. An integer such as `1` is accepted |
| `count` | Optional, default 1. The entry places that many identical factories, each with its own ID: a whole number from 1 to 1,000,000, the same rule and range as a deposit's `count` |

```toml
[[factory]]
q = 0
r = 0
technology = "mine"
owner = "center_mine"
condition = 0.75

[[factory]]                # two identical full-condition farms
q = 0
r = 0
technology = "farm"
owner = "center_farm"
count = 2
```

Any other key is an error. A setting with no `[[factory]]` is valid.

## How factories are loaded

`setting/factories.rs` holds `FactoriesReader`. `WorldContent` has a `factories: FactoriesReader` field, and the `factory` section type (folder `Start`, listed after `business`) calls `FactoriesReader::read` for each entry. Factories are unnamed and have no duplicate check: any number of identical factories may sit on one hex, so the reader neither uses `Definitions` nor keeps a table of what it has seen.

1. **Phase 1, `read`.** Records the header's location, then reads every key before deciding anything, so no key is falsely reported as unknown and one entry with several problems reports all of them. In order:
   - `q` and `r` with the shared `coordinate` helper from `setting/map.rs`.
   - `technology` and `owner` with `required_string`. They are not name-checked: a bad value is just an unknown name in phase 2.
   - `condition` with the private `condition` helper: absent gives 1; otherwise `optional_number` reads it (a non-number is reported by the reader, `key 'condition' in factory must be a number, found string`, and gives None), and a value that is not finite, not greater than 0 or above 1 is reported at the value and gives None.
   - `count` with the shared `count` helper from `setting/map.rs` (see [The shared count helper](#the-shared-count-helper)).
   - **Every entry is kept** for phase 2 as a `FactoryEntry` (header location, optional coordinates, technology, owner, condition and count, each `None` if missing or invalid), so an entry with a bad condition or count, or bad coordinates, still has its references checked.
2. **Phase 2, `resolve(technologies, businesses, map, sink)`.** For every entry, in load order:
   - resolves `technology` through the technologies' `Definitions` (`TechnologiesReader::defs`; `unknown technology 'loom'` at the value) and `owner` through the businesses' `Definitions` (`BusinessesReader::defs`; `unknown business 'nobody'` at the value);
   - resolves the coordinates through `MapReader::id` (`no hex at (9, 9)` at the header);
   - if the owner and the hex both resolved, checks the hex against the owner's through `BusinessesReader::hex_of(owner)`, which returns the owner's resolved hex and the coordinates it was written with, or None if the owner's own coordinates were bad or matched no hex (already reported at the business, so there is nothing to check). A mismatch is reported at the header as `factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)`;
   - only if the technology, owner, hex, condition and count are all valid and the hex is the owner's, pushes `count` copies of `Factory { hex, technology, owner, condition }`. A running total guards against more than `u32::MAX` factories (`too many factories in total (the most is 4294967295)` at the header, reported once), so `from_factories` can never panic on setting data.

   `WorldContent::resolve` calls this last, after `BusinessesReader::resolve`, because `hex_of` needs the businesses' resolved hexes. Technologies are rules, so they are always read before any factory; businesses and hexes may be defined in any `start/` file, before or after the factories. As for every reference, phase 2 (and with it the owner's-hex check) is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `FactoriesReader::finish(hex_count)` returns `Factories::from_factories` over the expanded list. It is only called after a successful load. `load_setting` calls it after `BusinessesReader::finish` and stores the result in `StartState::factories`.

All `[[factory]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `q`, `r`, `technology` or `owner` missing | header | `missing required key 'technology' in factory` |
| `q` or `r` not an integer | value | `key 'q' in factory must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in factory is out of range` |
| `technology` or `owner` not a string | value | `key 'owner' in factory must be a string, found boolean` |
| `condition` not a number | value | `key 'condition' in factory must be a number, found string` |
| `condition` zero, negative, above 1, `inf` or `nan` | value | `key 'condition' in factory must be a finite number greater than 0 and at most 1` |
| `count` not an integer | value | `key 'count' in factory must be an integer, found float` |
| `count` below 1 | value | `key 'count' in factory must be at least 1` |
| `count` above 1,000,000 | value | `key 'count' in factory is out of range (the most is 1000000)` |
| an unknown technology | value | `unknown technology 'loom'` |
| an unknown business | value | `unknown business 'nobody'` |
| coordinates that match no hex | header | `no hex at (9, 9)` |
| a hex that isn't the owner's | header | `factory at (1, 0) is not on the hex of its owner 'farm' at (0, 0)` |
| more than `u32::MAX` factories in total | header | `too many factories in total (the most is 4294967295)` |
| any other key | key | `unknown key 'colour' in factory` |
| `[[factory]]` in `rules/` | header | `section 'factory' belongs in start/, not rules/` |

### The shared count helper

`setting/map.rs` holds `count(reader) -> Option<u32>` and `MAX_COUNT` (1,000,000), shared by `[[deposit]]` and `[[factory]]`. An absent `count` gives `Some(1)`. A value below 1 is reported at the value as `key 'count' in <label> must be at least 1` and one above `MAX_COUNT` (or beyond a `u32`) as `key 'count' in <label> is out of range (the most is 1000000)`, both giving None; `<label>` is the entry's `EntryReader::label()`, so the messages name `deposit` or `factory`. A value that is not an integer is reported by `optional_integer` (`key 'count' in <label> must be an integer, found float`) and also gives None. Each caller decides what None means: the factories reader produces no factory for the entry, and the deposits reader stores 1 so its references are still checked (see [resources.md](resources.md#how-resources-are-loaded)).

## In the world

The `World` takes the `Factories` over as state (`World::factories() -> &Factories`), because a factory's technology and condition change in later Steps. A caller reads every factory with `world.factories().iter()`, the factories on a hex with `on_hex(hex)` and the factories a business owns with `owned_by(business)`, all in ID order. Nothing in the turn touches factories yet: they don't run, wear or appear in the `TurnReport`, so two worlds from the same setting stepped the same number of times hold identical factories, equal to the setting's. See [world.md](world.md).

## Using factories

```rust
let (world, _report) = World::new(load_setting(path)?);
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
for &id in world.factories().on_hex(center) {                 // ID order
    let factory = world.factories().get(id);
    let technology = world.rules().technologies.get(factory.technology);
    let owner = world.businesses().name(factory.owner);
    let condition = factory.condition;                         // share of the capital bundle
}
let farm = world.businesses().id("center_farm")?;
let owned: Vec<FactoryId> = world.factories().owned_by(farm).collect();
```

The goods in a factory are its technology's capital bundle with every amount multiplied by `condition`; no helper computes them yet (Step 04 adds the arithmetic that needs them).

## Not yet

Running a factory, depreciation, maintenance, shutting down and what an exhausted factory is (Step 04); serving each business its factories in the view (Step 05); the mechanics in the turn (Step 08); and showing factories in the dev UI (Step 09) are later Steps of Stage 03. Factories under construction, building, scrapping, retooling, buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes. Nothing changes a factory during play yet.

## The sample setting and tests

- `settings/sample/start/factories.toml` places one factory per business on the center hex (0, 0), each running the technology its business is named for:

  | Factory (ID) | Technology | Owner | Condition |
  |---|---|---|---|
  | 0 | `farm` | `center_farm` | 1 (not written) |
  | 1 | `mine` | `center_mine` | 0.75 |
  | 2 | `toolmaker` | `center_toolmaker` | 1 (not written) |

  The mine starts worn: 0.75 of its 2 `tools` of capital, six turns of its eight-turn lifetime, so Step 04 has a worn factory from turn 0. The deposits on other hexes have no factory. The file's header comment explains every key, what the condition means and these numbers; `rules/technologies.toml` and `start/README.md` point to it. The sample's turn reports are unchanged.
- `factories.rs` unit tests: sequential IDs in the order given, `get`, `iter` in order, `on_hex` and `owned_by` in ID order (and empty for a hex or business with none), the empty default, conditions of exactly 0 and 1 accepted, and the panics (a foreign ID, a hex beyond the map, the default list, `from_factories` with a hex beyond `hex_count`, and a condition above 1, negative or NaN).
- `setting/factories.rs` unit tests: the `condition` helper (absent gives 1; 0.0001, 0.5, `1` and `1.0` accepted; 0, a negative, above 1, `inf` and `nan` reported with the range message; a string reported as not a number). The `count` helper is tested in `setting/resources.rs`, against a `deposit` entry.
- `crates/engine/tests/factories.rs`: the sample's three factories, held unchanged by a fresh world through several steps, and two runs with identical reports and factories; `factories_valid` (factories across `start/a.toml`, `start/b/c.toml` and a later `start/z.toml` holding the hexes and businesses they refer to, a `count = 2` entry giving two equal factories with consecutive IDs, a condition written and not written, two businesses on one hex owning factories of different technologies), with `on_hex` and `owned_by` checked and the same result on a second load; every `factories_errors` error at its place, including one entry with several problems (a bad condition and count do not stop the technology and owner's-hex checks) and a factory whose owner's own hex doesn't exist (no owner's-hex error); `valid_minimal` with no factories; `factories_one`; and `get` panicking on a factory from another world.
- Test settings: `factories_valid`, `factories_one` (one good, technology, hex, business and factory, for the checker's `1 factory`), `factories_errors`, and a factory of an unknown technology owned by an unknown business on a hex that doesn't exist in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.

## docs/README.md — modify
- **Location:** the document table, new row after `businesses.md` (the row Step 02 adds)
- **Change:** add

| [factories.md](factories.md) | Factories, the assets that run a technology: their hex, technology, owning business and condition (the share of the capital bundle held), `Factories` with its per-hex and per-owner lookups, how `[[factory]]` sections are loaded and checked (including the owner's-hex check and the `count` helper shared with deposits), and the factory list in the World |

## docs/README.md — modify
- **Location:** the `world.md` row
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same row

| [world.md](world.md) | The running `World`, the three-part turn (emission to hex or owner stockpiles and POP endowment, the trade round and POP growth, perishing), hex, POP and business stockpiles with perish groups and removal, the factories held as state, per-POP trade outcomes, turn reports with the total population, and the setting checker's `--turns` option |

## docs/setting-loader.md — modify
- **Location:** introduction, the sentence "The loader defines seven real section types so far: ..."
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

The loader defines ten real section types so far: `[[good]]`, `[[resource]]`, `[[pop_type]]` and `[[technology]]` in `rules/` (see [goods.md](goods.md), [resources.md](resources.md), [pops.md](pops.md) and [technologies.md](technologies.md)) and `[[hex]]`, `[[deposit]]`, `[[market]]`, `[[pop]]`, `[[business]]` and `[[factory]]` in `start/` (see [map.md](map.md), [resources.md](resources.md), [market.md](market.md), [pops.md](pops.md), [businesses.md](businesses.md) and [factories.md](factories.md)); later Stages add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, the `map.rs` row
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same row

| `map.rs` | `MapReader`: reads `[[hex]]` entries, including each hex's `currency`, and builds `HexMap` and `HexCurrencies`. It also holds the helpers shared by placed sections: `coordinate` (deposits, markets, POPs, businesses and factories) and `count` with `MAX_COUNT` (deposits and factories; see [factories.md](factories.md#the-shared-count-helper)). `MapReader::id` resolves coordinates to a `HexId`, `MapReader::len` gives the hex count, and `MapReader::currency` gives a hex's resolved currency after phase 2 |

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, the `technologies.rs` row (the row Step 01 adds) and the `businesses.rs` row (the row Step 02 adds)
- **Change:** add a sentence to the end of each row

`technologies.rs`: `TechnologiesReader::defs` gives the definitions, for resolving the technology a factory names.

`businesses.rs`: `BusinessesReader::defs` gives the definitions, for resolving a factory's owner, and `BusinessesReader::hex_of(id)` gives a business's resolved hex and its written coordinates after phase 2, for the factories' owner's-hex check.

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, new row after `businesses.rs` (the row Step 02 adds)
- **Change:** add

| `factories.rs` | `FactoriesReader`: reads `[[factory]]` entries, resolves each factory's technology, owner and hex in phase 2, checks that the hex is the owner's, expands `count`, and builds `Factories` (see [factories.md](factories.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph beginning "`WorldContent::resolve` is where real references are checked"
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same paragraph

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), then each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency, then the goods in each POP type's `endowment` and `utility` (`PopTypesReader::resolve`), then each POP's `type` against the POP types (`PopTypesReader::defs`) and its coordinates against the map (`PopsReader::resolve`), then the goods in each technology's `inputs`, `output`, `capital` and `construction` (`TechnologiesReader::resolve`), then the goods in each business's `inventory`, the resource types in its `deposits` and its coordinates, after which each business claims its deposits (`BusinessesReader::resolve`, which takes the `DepositsReader` mutably), and finally each factory's `technology` (`TechnologiesReader::defs`), `owner` (`BusinessesReader::defs`) and coordinates, checking that the hex is the owner's (`FactoriesReader::resolve`, through `BusinessesReader::hex_of`). Three calls must keep their order: the map resolves before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`); the deposits resolve before the businesses, because the claims walk the deposit list that `DepositsReader::resolve` builds, and businesses claim in load order; and the businesses resolve before the factories, because the owner's-hex check needs the businesses' resolved hexes. Otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`, `PopTypesReader::finish`, `PopsReader::finish`, `TechnologiesReader::finish`, `BusinessesReader::finish`, `FactoriesReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph beginning "`WorldContent` is the real content type."
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same paragraph

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader`, a `MarketsReader`, a `PopTypesReader`, a `PopsReader`, a `TechnologiesReader`, a `BusinessesReader` and a `FactoriesReader`, and returns ten section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`, `pop_type` in `rules/`, `pop` in `start/`, `technology` in `rules/`, and `business` and `factory` in `start/`. `load_setting` builds `Rules { goods, resources, pop_types, technologies }` and `StartState { map, currencies, deposits, prices, pops, businesses, factories }` from it. `MarketsReader::finish` runs first, because `MapReader::finish` consumes the map reader; `MapReader::finish` returns the map and the currencies together and comes before `DepositsReader::finish`, `PopsReader::finish`, `BusinessesReader::finish` and `FactoriesReader::finish`, which need the hex count. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the second sub-bullet (the one about `[[hex]]`, `[[deposit]]`, `[[market]]` and `[[pop]]`, which Step 02 extends with `[[business]]`)
- **Change:** add, at the end of that sub-bullet

`[[factory]]` (`StartState::factories`) is the worked example of a placed, counted kind that keeps every entry for phase 2 (so a bad condition or count doesn't stop its reference checks) and reads another reader's phase-2 results (`BusinessesReader::hex_of`, so it must resolve after the businesses). A new placed kind with several copies per entry should use the shared `count` helper in `setting/map.rs` (see [factories.md](factories.md)).

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", first bullet
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same bullet

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (seven goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `rules/pops.toml` (two POP types, see [pops.md](pops.md)), `rules/technologies.toml` (three technologies, see [technologies.md](technologies.md)), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), `start/pops.toml` (16 POPs, see [pops.md](pops.md)), `start/businesses.toml` (three businesses on the center hex, owning three of its deposits, see [businesses.md](businesses.md)), `start/factories.toml` (three factories on the center hex, one per business, see [factories.md](factories.md)), plus `README.md` files. It must always load without errors.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", second bullet, the list of test settings
- **Change:** add `factories_valid`, `factories_one` and `factories_errors` to the list, after `businesses_errors` (which Step 02 adds)

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph beginning "The success line is"
- **Change:** replace (its first three sentences; the rest of the paragraph is unchanged)
- **Supersedes:** Step 02 (businesses), the same sentences

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>, <pop types>, <pops>, <technologies>, <businesses>, <factories>`, for example `7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories` or `1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 1 technology, 1 business, 1 factory`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. POP types are counted with `Rules::pop_types.len()`, POPs with `StartState::pops.len()`, technologies with `Rules::technologies.len()`, businesses with `StartState::businesses.len()`, and factories with `StartState::factories.len()` (after `count` expansion).

## docs/resources.md — modify
- **Location:** "Setting format", the paragraph beginning "An entry with `count = 3` places three separate deposits.", its last sentence ("The `count` cap of 1,000,000 ...")
- **Change:** replace

The `count` cap of 1,000,000 turns a typo into a clear error instead of an attempt to allocate billions of deposits. `[[factory]]` uses the same `count` rule and cap (see [factories.md](factories.md)).

## docs/resources.md — modify
- **Location:** "How resources are loaded", the `DepositsReader` list: the `read` bullet
- **Change:** replace

- `read` reads `resource`, `q`, `r` (the `q`/`r` through `map::coordinate`, shared with hexes, markets, POPs, businesses and factories, whose message uses `EntryReader::label()`, so `key 'q' in deposit is out of range`), and `count` through `map::count`, shared with factories (absent gives 1; not an integer gives the reader's `must be an integer` error; below 1 gives `key 'count' in deposit must be at least 1`; above `MAX_COUNT` gives `key 'count' in deposit is out of range (the most is 1000000)`; each error gives None; see [factories.md](factories.md#the-shared-count-helper)). If `resource`, `q`, or `r` is missing or invalid, nothing is stored and no duplicate check runs. Otherwise a second entry for the same (resource name as written, coordinates) is reported at its header as `duplicate deposit of 'x' at (q, r) (first defined at ...)`. The `seen` table is lookup only and is never iterated. When `count` gives None the reader stores 1, so phase 2 still checks the references (the load fails anyway).

## docs/resources.md — modify
- **Location:** "How resources are loaded", the last paragraph ("References are checked in `WorldContent::resolve` ..."), its third sentence (the one about `resources_broken_file`)
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

`resources_broken_file` also holds a hex with an unknown currency, a market that prices goods that don't exist, a POP type whose endowment and utility name goods that don't exist, a technology whose goods don't exist, a POP of an unknown type at coordinates with no hex, a business whose good and resource type don't exist on a hex that doesn't exist, and a factory of an unknown technology owned by an unknown business on a hex that doesn't exist, which are likewise not reported (and no deposit is claimed).

## docs/resources.md — modify
- **Location:** "The sample setting and tests", the last bullet (unit tests)
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same bullet

- Integration tests: `crates/engine/tests/resources.rs`; deposit ownership in `crates/engine/tests/businesses.rs`. Unit tests: `quantity.rs`, `resources.rs` (the `Deposits` behavior, including `owned_by`), `setting/resources.rs` (the shared `count` helper from `setting/map.rs`, run on a `deposit` entry, and `claim`), and `setting/reader.rs` (`required_number`, `positive_quantity`). The `ResourceTypes` list behavior is covered by the `NamedList` tests in `id.rs`.

## docs/businesses.md — modify
- **Location:** introduction, the second paragraph ("Nothing decides or trades for a business yet. ...")
- **Change:** replace

Nothing decides or trades for a business yet. Factories (see [factories.md](factories.md)) name a business as their owner, and must sit on its hex; the decider (Steps 05 to 07) will choose what each business does, and business trade (Step 08) will trade from and into the business stockpile. Until then a business only holds its inventory, receives the output of its deposits, and owns factories that don't run yet.

## docs/businesses.md — modify
- **Location:** "Concepts", the "Named." bullet
- **Change:** replace

- **Named.** Businesses are the first named kind in the starting state: each has a name, unique among businesses, because factories refer to their owner by name (see [factories.md](factories.md)). Business names are their own namespace, separate from goods, resource types, POP types and technologies.

## docs/businesses.md — modify
- **Location:** "How businesses are loaded", new paragraph after the `DepositsReader::claim` paragraph
- **Change:** add

Two accessors serve the factories reader, which resolves after the businesses: `BusinessesReader::defs()` gives the business `Definitions`, for resolving a factory's `owner`, and `BusinessesReader::hex_of(id) -> Option<(HexId, HexCoord)>` gives the resolved hex of business `id` and the coordinates it was written with, or None if its coordinates were missing, invalid or matched no hex (already reported). Entries without an ID are skipped, so `hex_of` searches the entries for the ID rather than indexing them. See [factories.md](factories.md#how-factories-are-loaded).

## docs/businesses.md — modify
- **Location:** "Using businesses", the code block
- **Change:** add, inside the `for` loop after the `owned` line

```rust
    let factories: Vec<FactoryId> = world.factories().owned_by(id).collect();
```

## docs/businesses.md — modify
- **Location:** "Not yet"
- **Change:** replace its first sentence
- **Supersedes:** Step 02 (businesses), the same sentence

Running the factories a business owns (Steps 04 and 08), the view a business is served and its decisions (Step 05), the decider and resolving decider names (Steps 06 and 07), business trade and the use of its cash (Step 08), and showing businesses and their factories in the dev UI (Step 09) are later Steps of Stage 03.

## docs/businesses.md — modify
- **Location:** "The sample setting and tests", first bullet (`settings/sample/start/businesses.toml`), at the end
- **Change:** add

Each business owns one factory on the center hex, running the technology it is named for (see [factories.md](factories.md)).

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)): each names its technology and holds a share of its capital bundle. The factory mechanics of running, depreciation and maintenance (Step 04), the view a business is served and its decisions (Step 05), the first decider (Step 06), the decider host (Step 07), business trade with factories running in the turn (Step 08), and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/technologies.md — modify
- **Location:** "The sample setting and tests", first bullet, its last sentence ("No POP wants `tools` and nothing produces it yet, ...")
- **Change:** add, after it

The sample places one factory of each technology on the center hex, the mine's at condition 0.75 (see [factories.md](factories.md)); they don't run yet.

## docs/world.md — modify
- **Location:** "What a World holds", the table, new row after the `business_stockpiles` row (which Step 02 adds)
- **Change:** add

| `factories: Factories` | the setting's `StartState::factories` | not yet; world state because their technology and condition change in later Steps (see [factories.md](factories.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table ("The world owns the setting, ..."), its list of public accessors
- **Change:** replace the accessor sentence
- **Supersedes:** Step 02 (businesses), the same sentence in its replacement of this paragraph

Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)`, `hex_stockpile(hex)`, `pops()`, `pop_stockpile(pop)`, `pop_outcome(pop)`, `businesses()`, `business_stockpile(business)` and `factories()`. `factories()` returns the `Factories`, whose `on_hex` and `owned_by` give the factories on a hex and those a business owns, in ID order.

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same section

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, businesses acting in the "actors act" part of the turn (they exist and hold goods, but decide and trade nothing until later Steps of Stage 03), and factories running, wearing or being maintained (they are held as state, unchanged, until Steps 04 and 08 of Stage 03).

## docs/ids.md — modify
- **Location:** "Types", the `IdVec<K, T>` bullet, its last sentence
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

`HexMap`, `Deposits`, `HexCurrencies`, `HexPrices`, `Pops`, `Businesses`, `Factories` and `World` use it.

## docs/ids.md — modify
- **Location:** "Adding a new ID kind", step 3, after the `PopKind` / `PopId` sentence
- **Change:** add

`FactoryKind` / `FactoryId` (label `"factory"`) is another unnamed kind with no identifying data: like deposits, an entry's `count` makes several identical factories, and `Factories::from_factories` numbers them in load order and keeps a per-hex index (see [factories.md](factories.md)).

## docs/map.md — modify
- **Location:** "How hexes are loaded" (the paragraph beginning "Duplicates are detected by the reader's own coordinate table"), the two lists of users
- **Change:** replace "(deposits, markets, POPs and businesses use it)" with "(deposits, markets, POPs, businesses and factories use it)" and "(hexes, deposits, markets, POPs and businesses)" with "(hexes, deposits, markets, POPs, businesses and factories)"
- **Supersedes:** Step 02 (businesses), the same replacement

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries
- **Change:** add one line after `src/demand.rs`, before `src/goods.rs`

```
    src/factories.rs    factories: hex, technology, owner, condition (see factories.md)
```

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same entry

```
    tests/businesses.rs, currency.rs, demand.rs, factories.rs, goods.rs, growth.rs, map.rs,
          market.rs, pops.rs, resources.rs, setting_loader.rs, technologies.rs, trade.rs,
          world.rs
```

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, its second sentence
- **Change:** replace the parenthetical list of counts
- **Supersedes:** Step 02 (businesses), the same sentence

Every expected success line includes the markets, POP types, POPs, technologies, businesses and factories counts (`18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `1 technology` for `technologies_one`, `1 business` for `businesses_one`, `1 technology, 1 business, 1 factory` for `factories_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists and its `population` (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0` for settings without POPs). The sample's turn lines are unchanged by its businesses and factories.
