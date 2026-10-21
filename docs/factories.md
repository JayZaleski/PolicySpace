# Factories

Source: `crates/engine/src/factories.rs` (the data model), `crates/engine/src/setting/factories.rs` (reading `[[factory]]` sections), the shared `count` helper in `crates/engine/src/setting/map.rs`, and the factory list in `crates/engine/src/world.rs`. The public types are re-exported from the engine crate root: `FactoryKind`, `FactoryId`, `Factory`, `Factories`.

A **factory** is a discrete asset on a hex that runs one **technology** (see [technologies.md](technologies.md)) and belongs to exactly one **business** (see [businesses.md](businesses.md)). It carries the state that changes during play: which technology it runs, and its **condition**, the share of its technology's full capital bundle it holds. Factories are starting state, defined in `start/` and checked by the loader, and the World takes them over as state.

Every turn, right after its hex's trade round, each business's factories are run, worn and maintained by the factory mechanics (see [production.md](production.md)) at the fractions its decision gives, and their conditions are written back (see [trade.md](trade.md#operating-the-factories)). Each business's view lists its factories by ID number with their condition and technology, and a decision addresses them by the same number (see [decider.md](decider.md)). The dev UI shows each business's factories with their condition, remaining life and this turn's record (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## Concepts

- **Unnamed.** Factories have no names, like deposits and POPs: nothing refers to a factory by name. Deciders, the dev UI and later decisions address a factory by its `FactoryId`, or its ID number in the protocol. IDs follow load order (file read order, then entry order, with an entry's `count` copies consecutive), so the same folder always gives the same IDs.
- **Its own hex.** A factory's hex is stored on the factory, not derived from its owner, because businesses will operate across several hexes once trade between hexes exists. Until then the loader requires the factory's hex to be its owner's hex (see [How factories are loaded](#how-factories-are-loaded)). That check is to be relaxed when trade arrives; the data doesn't change.
- **One owner, always.** Every factory has exactly one owning business. There are no unowned factories.
- **Condition, not a goods table.** Depreciation takes a slice of the whole capital bundle and maintenance replenishes a share of the whole bundle (see [technologies.md](technologies.md#what-the-numbers-mean)), so a factory's capital is always its technology's capital bundle times some share. That share is the only real state, stored as `condition: f64`. The goods in a factory are derived from the technology's capital bundle and the condition whenever they are needed. A setting places a factory with a condition greater than 0 and at most 1 (1 when none is written). The condition changes every turn in play: wear takes one lifetime-slice and maintenance adds back what the business pays for, so the engine type accepts 0 too. A factory runs only while it holds at least one lifetime-slice (a condition of at least 1/t₁). Below that it is shut down until maintenance brings it back (see [production.md](production.md#lifetime-slices-and-shutdown)).
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
    factories: IdVec<FactoryKind, Factory>,        // by FactoryId, in load order
    by_hex: IdVec<HexKind, Vec<FactoryId>>,        // by HexId: the factories on each hex, in ID order
    by_owner: IdVec<BusinessKind, Vec<FactoryId>>, // by BusinessId: the factories each business owns, in ID order
}
```

`Factories` mirrors `Deposits` (see [resources.md](resources.md)): an unnamed list in load order plus a per-hex index and a per-owner index, both built once at load.

| `Factories` method | Result |
|---|---|
| `len()`, `is_empty()` | The number of factories; a setting with none is valid |
| `get(id) -> &Factory` | The factory. Panics with `is not from this list` for a foreign ID |
| `iter()` | `(FactoryId, &Factory)` in load order, which is ID order |
| `on_hex(hex) -> &[FactoryId]` | The factories on `hex`, in ID order; empty if none. Panics with `is not from this list` for a hex not from the map the list was built with |
| `owned_by(business)` | An iterator over the `FactoryId`s that business owns, in ID order, from the per-owner index. Panics for a business not from the businesses the list was built with, like `Deposits::owned_by` |
| `set_condition(id, condition)` (crate-private) | Writes a factory's condition back after its owner's operation, the only change to a factory during play. Panics with `a factory's condition must be a finite number from 0 to 1` for a value out of range, and `is not from this list` for a foreign ID |

`Factories::from_factories(factories, hex_count, business_count)` is crate-private: only the loader builds factories. It numbers the factories 0, 1, 2, ... in the order given and fills `by_hex` and `by_owner`, panicking if a factory's hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), if its owner index is `>= business_count` (`"{owner:?} is not from a list of {business_count} businesses"`), if a condition isn't a finite number from 0 to 1 (`"a factory's condition must be a finite number from 0 to 1"`), or if there are more than `u32::MAX` factories. `Factories::default()` is empty and was built with no map and no businesses, so `on_hex` and `owned_by` panic on it for any hex or business. The fields are private. `factories.rs` depends on `businesses`, `id`, `map` and `technologies` (for the ID types only) and knows nothing about files.

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
3. **Finish.** `FactoriesReader::finish(hex_count, business_count)` returns `Factories::from_factories` over the expanded list. It is only called after a successful load. `load_setting` calls it after `BusinessesReader::finish`, passing `map.len()` and `businesses.len()`, and stores the result in `StartState::factories`.

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

`setting/map.rs` holds `count(reader) -> Option<u32>` and `MAX_COUNT` (1,000,000), shared by `[[deposit]]` and `[[factory]]`. An absent `count` gives `Some(1)`. A present value goes through `EntryReader::at_least_one` with the prefix `key 'count' in <label>` and `Some(MAX_COUNT)` as the bound (see [setting-loader.md](setting-loader.md#entryreader)): a value below 1 is reported at the value as `key 'count' in <label> must be at least 1` and one above `MAX_COUNT` (or beyond a `u32`) as `key 'count' in <label> is out of range (the most is 1000000)`, both giving None; `<label>` is the entry's `EntryReader::label()`, so the messages name `deposit` or `factory`. A value that is not an integer is reported by `optional_integer` (`key 'count' in <label> must be an integer, found float`) and also gives None. Each caller decides what None means: the factories reader produces no factory for the entry, and the deposits reader stores 1 so its references are still checked (see [resources.md](resources.md#how-resources-are-loaded)).

## In the world

The `World` takes the `Factories` over as state (`World::factories() -> &Factories`). A caller reads every factory with `world.factories().iter()`, the factories on a hex with `on_hex(hex)` and the factories a business owns with `owned_by(business)`, all in ID order. The condition is the only thing that changes during play: after each business's operation the World writes each factory's new condition back with the crate-private `Factories::set_condition(id, condition)`. Technology and owner stay as placed until later Stages. Because a world is observed after part 2, a fresh world's conditions are already after turn 0's operation: under the idle host every factory has worn one slice (the sample's farm reads 0.875, its mine 0.625 and its toolmaker 23/24). What each factory did this turn is in its owner's `World::business_outcome` (see [trade.md](trade.md#businesstrade)). See [world.md](world.md).

## Using factories

```rust
let mut host = DeciderHost::idle();
let (world, _report) = World::new(load_setting(path)?, &mut host)?;
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
for &id in world.factories().on_hex(center) {                 // ID order
    let factory = world.factories().get(id);
    let technology = world.rules().technologies.get(factory.technology);
    let owner = world.businesses().name(factory.owner);
    let condition = factory.condition;                         // share of the capital bundle, after this turn's operation
}
let farm = world.businesses().id("center_farm")?;
let owned: Vec<FactoryId> = world.factories().owned_by(farm).collect();
```

The goods in a factory are its technology's capital bundle with every amount multiplied by `condition`: `technology.capital().scaled(Quantity::new(factory.condition)?)`. To run, wear or maintain factories outside the turn, build one `production::Operation` per factory a business owns and call `operate` (see [production.md](production.md#using-the-mechanics)).

## Not yet

Factories under construction, building, scrapping, retooling, and buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes.

## The sample setting and tests

- `settings/sample/start/factories.toml` places one factory per business on the center hex (0, 0), each running the technology its business is named for:

  | Factory (ID) | Technology | Owner | Condition |
  |---|---|---|---|
  | 0 | `farm` | `center_farm` | 1 (not written) |
  | 1 | `mine` | `center_mine` | 0.75 |
  | 2 | `toolmaker` | `center_toolmaker` | 1 (not written) |

  The mine starts worn: 0.75 of its 2 `tools` of capital, six turns of its eight-turn lifetime, so the sample has a worn factory from turn 0 (under the `arithmetic` decider it is maintained back to full in turn 0; see [trade.md](trade.md#worked-examples)). The deposits on other hexes have no factory. The file's header comment explains every key, what the condition means and these numbers; `rules/technologies.toml` and `start/README.md` point to it.
- `factories.rs` unit tests: sequential IDs in the order given, `get`, `iter` in order, `on_hex` and `owned_by` in ID order (and empty for a hex or business with none), the empty default, conditions of exactly 0 and 1 accepted, `set_condition` writing one factory's condition and leaving the rest, and the panics (a foreign ID, a hex beyond the map, the default list for `on_hex` and `owned_by`, `from_factories` with a hex beyond `hex_count`, an owner beyond `business_count`, and a condition above 1, negative or NaN; `set_condition` with a value above 1, NaN or a foreign ID).
- `setting/factories.rs` unit tests: the `condition` helper (absent gives 1; 0.0001, 0.5, `1` and `1.0` accepted; 0, a negative, above 1, `inf` and `nan` reported with the range message; a string reported as not a number). The `count` helper is tested in `setting/resources.rs`, against a `deposit` entry.
- `crates/engine/tests/factories.rs`: the sample's three factories, which under the idle host wear one lifetime-slice a turn from turn 0 (lifetimes 8, 8 and 24; compared to within 1e-9), and two runs with identical reports and factories; `factories_valid` (factories across `start/a.toml`, `start/b/c.toml` and a later `start/z.toml` holding the hexes and businesses they refer to, a `count = 2` entry giving two equal factories with consecutive IDs, a condition written and not written, two businesses on one hex owning factories of different technologies), with `on_hex` and `owned_by` checked and the same result on a second load; every `factories_errors` error at its place, including one entry with several problems (a bad condition and count do not stop the technology and owner's-hex checks) and a factory whose owner's own hex doesn't exist (no owner's-hex error); `valid_minimal` with no factories; `factories_one`; and `get` panicking on a factory from another world.
- Test settings: `factories_valid`, `factories_one` (one good, technology, hex, business and factory, for the checker's `1 factory`), `factories_errors`, and a factory of an unknown technology owned by an unknown business on a hex that doesn't exist in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.
