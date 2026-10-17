# Step 02: businesses — Docs Changes

## docs/businesses.md — new file
- **Location:** whole file
- **Change:** add

# Businesses

Source: `crates/engine/src/businesses.rs` (the data model), `crates/engine/src/setting/businesses.rs` (reading `[[business]]` sections), deposit ownership in `crates/engine/src/resources.rs` and `crates/engine/src/setting/resources.rs`, and the business stockpiles in `crates/engine/src/world.rs`. The public types are re-exported from the engine crate root: `BusinessKind`, `BusinessId`, `Business`, `Businesses`.

A **business** is a named actor on one hex. It has a **starting inventory** (the goods, cash included, in its stockpile when the world is created), owns some of the **deposits** on its hex, and names the **decider** that controls it. Businesses are starting state, defined in `start/` and checked by the loader. In the running world each business has its own stockpile, and an owned deposit emits into its owner's stockpile instead of the hex's.

Nothing decides or trades for a business yet. Factories (Step 03 of Stage 03) will name a business as their owner, the decider (Steps 05 to 07) will choose what each business does, and business trade (Step 08) will trade from and into the business stockpile. Until then a business only holds its inventory and receives the output of its deposits.

## Concepts

- **One hex.** A business sits on one hex and may own only deposits on that hex. Once trade between hexes exists, businesses are expected to operate across several hexes with this one as their headquarters. A hex may hold any number of businesses, and a business may sit on a hex with no market (Step 08 decides what trading there means).
- **Named.** Businesses are the first named kind in the starting state: each has a name, unique among businesses, because factories refer to their owner by name. Business names are their own namespace, separate from goods, resource types, POP types and technologies.
- **Cash is a good.** There is no separate cash field. A business's cash is the amount of its hex's currency in its stockpile, so its starting cash is the currency it lists in its inventory, zero if it lists none (see [currency.md](currency.md)). A business that starts with no cash is valid; the engine doesn't police economics.
- **Deposit ownership** is written on the business, not on the deposit: the business says how many deposits of each resource type it owns, and the loader hands out that many not-yet-owned deposits of that type on its hex. Deposits of one type on one hex are identical, so which units a business gets doesn't matter, but the assignment is deterministic: deposit ID order, businesses taken in load order. Each deposit has at most one owner. Nothing changes ownership during play yet.
- **Decider name.** Required and a valid name, so it can later be matched against a registry of deciders without a second format. It is only stored: nothing checks that it means anything, and several businesses may name the same decider. Step 07 resolves it.

## Data model

```rust
pub enum BusinessKind {}                    // IdKind, label "business"
pub type BusinessId = Id<BusinessKind>;
pub struct Business {
    pub hex: HexId,                         // its only hex until trade between hexes exists
    pub decider: String,                    // a valid name; not resolved yet
    pub starting_inventory: Bundle,         // may be empty; every amount greater than 0
}
pub struct Businesses {
    list: NamedList<BusinessKind, Business>,  // by BusinessId, in load order
    by_hex: IdVec<HexKind, Vec<BusinessId>>,  // by HexId: the businesses on each hex, in ID order
}
```

A business's stockpile is world state, not part of the `Business` record (see [In the world](#in-the-world)). `starting_inventory` is a `Bundle` (sorted by good ID, no good twice; see [technologies.md](technologies.md#bundle)); it stays on the record after the world is created, but from then on the stockpile is the truth.

`Businesses` combines a `NamedList` (see [ids.md](ids.md)) with a per-hex index, the way `Pops` combines a list with one (see [pops.md](pops.md#placed-pops)):

| `Businesses` method | Result |
|---|---|
| `len()`, `is_empty()` | The number of businesses; a setting with none is valid |
| `id(name) -> Result<BusinessId, IdError>` | The business named `name`, or `IdError::UnknownName { kind: "business", .. }` |
| `name(id) -> &str` | Its name. Panics with `not from this registry` for a foreign ID |
| `get(id) -> &Business` | The business. Panics with `is not from this list` for a foreign ID |
| `iter()` | `(BusinessId, &str, &Business)` in load order, which is ID order |
| `on_hex(hex) -> &[BusinessId]` | The businesses on `hex`, in ID order; empty if none. Panics for a hex not from the map the list was built with |

`Businesses::from_parts(names, businesses, hex_count)` is crate-private: only the loader builds businesses. It builds the `NamedList` (panicking with `every business name needs exactly one entry` if the lengths differ), then fills `by_hex`, panicking if a business's hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`) or if there are more than `u32::MAX` businesses. `Businesses::default()` is empty and was built with no map, so `on_hex` panics on it for any hex. The fields are private. `businesses.rs` depends on `bundle`, `id` and `map` and knows nothing about files; it does not depend on `resources`, which depends on it for `BusinessId`.

### Deposit ownership

```rust
pub struct Deposit {
    pub resource: ResourceId,
    pub hex: HexId,
    pub owner: Option<BusinessId>,   // set by the loader; nothing changes it during play yet
}
```

`Deposits::owned_by(business)` returns an iterator over the `DepositId`s that business owns, in ID order. It is a filter over every deposit (there is no per-owner index; the lists are short). `Deposits` lives in the World as state, so a running world reads ownership with `world.deposits().get(id).owner` or `world.deposits().owned_by(business)`. See [resources.md](resources.md).

## Setting format

A `[[business]]` belongs in `start/` (in `rules/` it gets `section 'business' belongs in start/, not rules/`). Six keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among businesses. A separate namespace from goods, resource types, POP types and technologies |
| `q`, `r` | Integer axial coordinates of a hex in the map, read as a deposit's or POP's are |
| `inventory` | A table of `good name = amount`, each amount a finite number greater than 0. May be empty (`{}`) but must be written. The hex's currency listed here is the business's starting cash |
| `deposits` | A table of `resource type name = count`, each count a whole number of at least 1 (and at most 4294967295). May be empty (`{}`) but must be written. The business gets that many not-yet-owned deposits of that type on its hex |
| `decider` | A valid name. Stored, not resolved |

The empty tables must still be written, so an empty one says the author meant it. Both TOML table forms are accepted for `inventory` and `deposits`:

```toml
[[business]]
name = "center_farm"
q = 0
r = 0
inventory = { silver = 100 }
deposits = { arable_land = 2 }
decider = "arithmetic"

[[business]]
name = "smithy"
q = 1
r = 0
decider = "arithmetic"

[business.inventory]
silver = 40
copper = 5

[business.deposits]        # empty: owns no deposit
```

Any other key is an error. `[[deposit]]` entries are unchanged: ownership is written only on the business.

## How businesses are loaded

`setting/businesses.rs` holds `BusinessesReader`, which follows `TechnologiesReader` (see [technologies.md](technologies.md#how-technologies-are-loaded)) for its name and its good references and `PopsReader` (see [pops.md](pops.md#how-pops-are-loaded)) for its coordinates. `WorldContent` has a `businesses: BusinessesReader` field, and the `business` section type (folder `Start`, listed after `technology`) calls `BusinessesReader::read` for each entry. Businesses are a named kind, so the reader uses `Definitions<BusinessKind>`.

1. **Phase 1, `read`.** Records the header's location, then reads every key before anything is defined, so no key is falsely reported as unknown and one entry with several problems reports all of them. In order:
   - `name` with `required_name`.
   - `q` and `r` with the shared `coordinate` helper from `setting/map.rs`.
   - `inventory` with `EntryReader::required_quantity_table` (prefix `key '<good>' in inventory of business`). Every key is kept, with `None` for a bad amount, so its good is still resolved. A missing or non-table `inventory` is reported there and gives an empty list (the load fails anyway).
   - `deposits` with the private `read_deposits`, which calls `EntryReader::required_integer_table("deposits")` (see [setting-loader.md](setting-loader.md#entryreader)) and then range-checks each count: below 1 is `key '<k>' in deposits of business must be at least 1` and beyond a `u32` is `key '<k>' in deposits of business is out of range`, both at the value. Every key is kept, with `None` for a bad count, so its resource type is still resolved.
   - `decider` with `required_name`.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate business name 'ok' (first defined at <location>)`).
   - **Every entry is kept** for phase 2 as a `BusinessEntry` with an optional `BusinessId` (`None` if the name was missing, invalid or a duplicate), so an entry with a bad name still has its unknown goods and resource types reported.
2. **Phase 2, `resolve(goods, resources, map, deposits, sink)`.** For every entry, in load order:
   - resolves each `inventory` key through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key) and each `deposits` key through `Definitions::<ResourceKind>::resolve` (`unknown resource 'meadow'` at the key), always;
   - resolves the coordinates through `MapReader::id` (`no hex at (9, 9)` at the header);
   - then, only if the entry has an ID and a hex, **claims its deposits**: for each `deposits` key whose count and resource type are both valid, `DepositsReader::claim(hex, resource, count, id)`. If the hex has fewer unowned deposits of that type than claimed (counting what earlier businesses took), nothing is assigned for that key and the error `key '<k>' in deposits of business claims more deposits than the hex has unowned (<count> claimed, <available> available)` is reported at the key.

   `WorldContent::resolve` calls this last, after `DepositsReader::resolve` has expanded the deposit list the claims walk. Goods, resource types and hexes may be defined in any file. As for every reference, phase 2 (and with it deposit assignment) is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `BusinessesReader::finish(hex_count)` skips entries without an ID, builds each `Business` (the starting inventory with `Bundle::new`, which sorts it by good ID), and returns `Businesses::from_parts`. It is only called after a successful load and panics with `internal error: a business was not fully resolved in a successful load` otherwise. `load_setting` calls it after `PopsReader::finish` and stores the result in `StartState::businesses`.

`DepositsReader::claim(hex, resource, count, owner) -> Result<(), u32>` (crate-private) first counts the deposits of `resource` on `hex` with no owner. If there are fewer than `count`, it returns `Err(available)` and changes nothing; otherwise it sets `owner` on the first `count` of them in deposit ID order. Because claims run in load order, the same folder always gives the same owners.

Business IDs follow load order (file read order, then entry order in each file).

All `[[business]]` errors and their exact texts (`{help}` is the usual invalid-name suffix):

| Problem | Reported at | Message |
|---|---|---|
| `name`, `q`, `r`, `inventory`, `deposits` or `decider` missing | header | `missing required key 'inventory' in business` |
| a bad `name` or `decider` | value | `invalid name 'Bad Name' {help}` |
| a duplicate name | value | `duplicate business name 'ok' (first defined at <location>)` |
| `q` or `r` not an integer | value | `key 'q' in business must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in business is out of range` |
| `inventory` or `deposits` not a table | value | `key 'inventory' in business must be a table, found integer` |
| `decider` not a string | value | `key 'decider' in business must be a string, found integer` |
| an inventory amount that isn't a number | value | `key 'coin' in inventory of business must be a number, found string` |
| an inventory amount that is zero, negative, `inf` or `nan` | value | `key 'grain' in inventory of business must be a finite number greater than 0` |
| a deposit count that isn't an integer | value | `key 'field' in deposits of business must be an integer, found float` |
| a deposit count below 1 | value | `key 'field' in deposits of business must be at least 1` |
| a deposit count beyond `u32` (or `i64`) | value | `key 'bog' in deposits of business is out of range` |
| an unknown good in `inventory` | key | `unknown good 'tin'` |
| an unknown resource type in `deposits` | key | `unknown resource 'meadow'` |
| more deposits claimed than the hex has unowned | key | `key 'field' in deposits of business claims more deposits than the hex has unowned (3 claimed, 1 available)` |
| coordinates that match no hex | header | `no hex at (9, 9)` |
| any other key | key | `unknown key 'colour' in business` |
| `[[business]]` in `rules/` | header | `section 'business' belongs in start/, not rules/` |

## In the world

The `World` takes the `Businesses` over as state (`World::businesses()`) and gives every business its own `Stockpile` in `business_stockpiles: IdVec<BusinessKind, Stockpile>`, read with `World::business_stockpile(business)` (which panics with `is not from this list` for a business from another world). See [world.md](world.md) for the turn as a whole.

- **Creation.** `World::new` fills each business's stockpile from its starting inventory before turn 0 runs. Each good goes in as one group that perishes by the good's own rule as if added in turn 0 (`PerishesAt::for_good(perishes, Turn::ZERO)`), so a perishable good listed there (labor, say) is gone at the end of turn 0, and a durable currency stays. Turn 0's emission then lands on top.
- **Emission.** At the start of every turn, every deposit emits in deposit ID order, as before, but into its owner's stockpile when it has one and into its hex's stockpile when it doesn't. The hex stockpile stays the home of goods nobody owns.
- **Perishing.** At the end of every turn, perishing covers hex stockpiles, then POP stockpiles, then business stockpiles, each in ID order.
- **Report.** `TurnReport` has no business fields. `emitted` stays one total over every deposit, owned or not, and `perished` includes what perished from business stockpiles, so splitting the output between hexes and businesses doesn't change the report. What each business holds is read from its stockpile.
- **Nothing else.** Businesses take no part in the trade round, nothing changes a business or a deposit's owner during play, and only the World's creation, emission and perishing change a business stockpile.

## Using businesses

```rust
let (world, _report) = World::new(load_setting(path)?);
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
for &id in world.businesses().on_hex(center) {          // ID order
    let name = world.businesses().name(id);
    let business = world.businesses().get(id);
    let decider = business.decider.as_str();             // not resolved yet
    let stock = world.business_stockpile(id);
    let cash = stock.total(world.hex_currency(business.hex));
    let owned: Vec<DepositId> = world.deposits().owned_by(id).collect();
}
let farm = world.businesses().id("center_farm")?;
```

## Not yet

Factories and factory ownership (Step 03), the view a business is served and its decisions (Step 05), the decider and resolving decider names (Steps 06 and 07), business trade and the use of its cash (Step 08), and showing businesses in the dev UI (Step 09) are later Steps of Stage 03. Changing ownership during play, businesses on several hexes, founding or failing businesses, POP ownership of businesses, budgets and dividends are later Stages. The dev UI doesn't show business stockpiles yet: owned deposit output and business cash appear in neither the hex panel nor the goods table's totals.

## The sample setting and tests

- `settings/sample/start/businesses.toml` places three businesses on the center hex (0, 0), one per technology in the chain (see [technologies.md](technologies.md)), every one with decider `arithmetic` (the name Step 06's rule will be registered as):

  | Business (ID) | Starting inventory | Deposits | Owns deposit IDs |
  |---|---|---|---|
  | `center_farm` (0) | 100 `silver` | 2 `arable_land` | 0, 1 |
  | `center_mine` (1) | 50 `silver` | 1 `copper_ore` | 3 |
  | `center_toolmaker` (2) | 200 `silver` | none | none |

  The center's third `arable_land` deposit (ID 2) and every deposit on other hexes stay unowned, so the center has owned and unowned deposits of one type, which exercises the ID-order assignment. The starting silver is a few turns of inputs at the center's prices. In a fresh sample world `center_farm` holds 8 `unimproved_grain` and 100 `silver`, `center_mine` 2.5 `unimproved_copper` and 50 `silver`, `center_toolmaker` 200 `silver`, and the center hex stockpile only the unowned deposit's 4 `unimproved_grain`; every turn after looks the same. The turn reports are unchanged. The file's header comment explains every key, the ownership rules and the numbers.
- `businesses.rs` unit tests: sequential IDs in the order given, `id`/`name` round trips, `get`, `iter` in order, `on_hex` in ID order and empty for a hex with none, the empty default, and the panics (foreign ID, a hex beyond the map, the default list, `from_parts` with a hex beyond `hex_count` or mismatched lengths).
- `resources.rs` unit tests: `owned_by` lists only that owner's deposits, in ID order, and nothing for a business that owns none. `setting/resources.rs`: `claim` hands out the first unowned deposits in ID order, changes nothing when short (returning how many were available), and returns `Err(0)` on a hex with none of that type. `setting/reader.rs`: `required_integer_table` (see [setting-loader.md](setting-loader.md)). `setting/businesses.rs`: `read_deposits` (a count of 1 and the largest `u32` accepted; 0, a negative, a count beyond `u32` and a float reported; a missing table). `world.rs`: the clock-maximum test also runs `businesses_valid` and checks the business stockpiles are unchanged.
- `crates/engine/tests/businesses.rs`: the sample's three businesses, their deposits and owners; a fresh sample world's business and hex stockpiles, and three steps keeping them, with two runs identical; `businesses_valid` (businesses across `start/a.toml`, `start/b/c.toml` and a later `start/z.toml` holding the hexes and deposits they refer to, both table forms, two businesses sharing the unowned deposits of one type on one hex, an empty inventory and no deposits on a hex with no market, a perishable `labor` gone after turn 0 and a durable `coin` kept), loaded twice with the same result and stepped through turn 3; every `businesses_errors` error at its place, including one entry (`Several`) with many problems and a second business running out of deposits in a later file; `valid_minimal` and `turns_valid` with no businesses and every deposit unowned; `businesses_one`; and `business_stockpile` panicking on a business from another world.
- Test settings: `businesses_valid`, `businesses_one` (one good, one hex and one business, for the checker's `1 business`), `businesses_errors`, and a business with an unknown good and resource type on a hex that doesn't exist in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.

## docs/README.md — modify
- **Location:** the document table, new row after `technologies.md` (the row Step 01 adds after `pops.md`)
- **Change:** add

| [businesses.md](businesses.md) | Businesses, the named actors in the starting state: their hex, starting inventory and cash, the decider name, deposit ownership and how deposits are claimed, how `[[business]]` sections are loaded and checked, and the business stockpiles and owned-deposit emission in the World |

## docs/README.md — modify
- **Location:** the `world.md` row
- **Change:** replace

| [world.md](world.md) | The running `World`, the three-part turn (emission to hex or owner stockpiles and POP endowment, the trade round and POP growth, perishing), hex, POP and business stockpiles with perish groups and removal, per-POP trade outcomes, turn reports with the total population, and the setting checker's `--turns` option |

## docs/setting-loader.md — modify
- **Location:** introduction, the sentence "The loader defines seven real section types so far: ..."
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same sentence

The loader defines nine real section types so far: `[[good]]`, `[[resource]]`, `[[pop_type]]` and `[[technology]]` in `rules/` (see [goods.md](goods.md), [resources.md](resources.md), [pops.md](pops.md) and [technologies.md](technologies.md)) and `[[hex]]`, `[[deposit]]`, `[[market]]`, `[[pop]]` and `[[business]]` in `start/` (see [map.md](map.md), [resources.md](resources.md), [market.md](market.md), [pops.md](pops.md) and [businesses.md](businesses.md)); later Stages add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, the `map.rs` and `resources.rs` rows
- **Change:** replace

| `map.rs` | `MapReader`: reads `[[hex]]` entries, including each hex's `currency`, and builds `HexMap` and `HexCurrencies`. Its `coordinate` helper is shared with deposits, markets, POPs and businesses. `MapReader::id` resolves coordinates to a `HexId`, `MapReader::len` gives the hex count, and `MapReader::currency` gives a hex's resolved currency after phase 2 |
| `resources.rs` | `ResourcesReader` and `DepositsReader`: read `[[resource]]` and `[[deposit]]` entries and build `ResourceTypes` and `Deposits`. `DepositsReader::claim` hands unowned deposits to a business in phase 2 |

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, new row after `technologies.rs` (the row Step 01 adds)
- **Change:** add

| `businesses.rs` | `BusinessesReader`: reads `[[business]]` entries, resolves the goods in their `inventory`, the resource types in their `deposits` and their hex in phase 2, claims their deposits from `DepositsReader`, and builds `Businesses` (see [businesses.md](businesses.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph beginning "`WorldContent::resolve` is where real references are checked"
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same paragraph

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), then each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency, then the goods in each POP type's `endowment` and `utility` (`PopTypesReader::resolve`), then each POP's `type` against the POP types (`PopTypesReader::defs`) and its coordinates against the map (`PopsReader::resolve`), then the goods in each technology's `inputs`, `output`, `capital` and `construction` (`TechnologiesReader::resolve`), and finally the goods in each business's `inventory`, the resource types in its `deposits` and its coordinates, after which each business claims its deposits (`BusinessesReader::resolve`, which takes the `DepositsReader` mutably). Two calls must keep their order: the map resolves before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`), and the deposits resolve before the businesses, because the claims walk the deposit list that `DepositsReader::resolve` builds, and businesses claim in load order. Otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`, `PopTypesReader::finish`, `PopsReader::finish`, `TechnologiesReader::finish`, `BusinessesReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph beginning "`WorldContent` is the real content type."
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same paragraph

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader`, a `MarketsReader`, a `PopTypesReader`, a `PopsReader`, a `TechnologiesReader` and a `BusinessesReader`, and returns nine section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`, `pop_type` in `rules/`, `pop` in `start/`, `technology` in `rules/`, and `business` in `start/`. `load_setting` builds `Rules { goods, resources, pop_types, technologies }` and `StartState { map, currencies, deposits, prices, pops, businesses }` from it. `MarketsReader::finish` runs first, because `MapReader::finish` consumes the map reader; `MapReader::finish` returns the map and the currencies together and comes before `DepositsReader::finish`, `PopsReader::finish` and `BusinessesReader::finish`, which need the hex count. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the second sub-bullet (the one about `[[hex]]`, `[[deposit]]`, `[[market]]` and `[[pop]]`)
- **Change:** add, at the end of that sub-bullet

`[[business]]` (`StartState::businesses`) is the worked example of a named kind in `start/`: it defines its names through `Definitions` like a `rules/` kind, keeps every entry for phase 2 like `[[technology]]`, and is placed by coordinates with a per-hex index like `[[pop]]`. It is also the example for a table of whole-number counts (read with `required_integer_table`) and for a reader whose phase 2 changes another reader's results (it claims deposits through `DepositsReader::claim`, so it must resolve after the deposits; see [businesses.md](businesses.md)).

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first bullet (the accessor list)
- **Change:** replace the list of accessor names at the start of the bullet
- **Supersedes:** Step 01 (technologies), the same list

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `optional_number`, `required_number_table`, `required_quantity_table`, `required_integer_table`, `required_number_array`, `required_table_of_tables`, `required_sub_table`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>` (or, for the four table readers, an `Option` of a list, and for `required_sub_table` an `Option<EntryReader>`), where `Located` carries the value and its `Location`.

(The rest of the bullet is unchanged.)

## docs/setting-loader.md — modify
- **Location:** "EntryReader", new bullet after the `required_quantity_table(key)` bullet (the bullet Step 01 adds)
- **Change:** add

- `required_integer_table(key)` reads a table whose values are whole numbers (a business's `deposits`) and returns `Option<Vec<(Located<String>, Option<Located<i64>>)>>`: every key with its location, in the table's order, and its value parsed exactly as `required_integer` parses one. Both share the private `integer_value` helper, which takes the message prefix, as `number_value` does for numbers. A bad value is reported at the value as `key '<k>' in <key> of <label> must be an integer, found <type>` or `... is out of range` (beyond `i64`) and returned as `None`, so the caller still sees and resolves the key. Missing and non-table `key` are handled as for `required_number_table`, and the table's keys are data in the same way. Any narrower range (a business's count must be at least 1 and fit a `u32`) is the caller's check.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", first bullet
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same bullet

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (seven goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `rules/pops.toml` (two POP types, see [pops.md](pops.md)), `rules/technologies.toml` (three technologies, see [technologies.md](technologies.md)), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), `start/pops.toml` (16 POPs, see [pops.md](pops.md)), `start/businesses.toml` (three businesses on the center hex, owning three of its deposits, see [businesses.md](businesses.md)), plus `README.md` files. It must always load without errors.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", second bullet, the list of test settings
- **Change:** add `businesses_valid`, `businesses_one` and `businesses_errors` to the list, after `technologies_errors` (which Step 01 adds)

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph beginning "The success line is"
- **Change:** replace (its first three sentences; the rest of the paragraph is unchanged)
- **Supersedes:** Step 01 (technologies), the same sentences

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>, <pop types>, <pops>, <technologies>, <businesses>`, for example `7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses` or `1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 0 technologies, 1 business`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. POP types are counted with `Rules::pop_types.len()`, POPs with `StartState::pops.len()`, technologies with `Rules::technologies.len()`, and businesses with `StartState::businesses.len()`.

## docs/setting-loader.md — modify
- **Location:** "Tests", first bullet, the list of what the `reader.rs` tests cover
- **Change:** add, after the `required_quantity_table` item (which Step 01 adds)

, and `required_integer_table` (integers, including a negative, returned with their locations; a float, a string and an integer beyond `i64` each reported with the `key '<k>' in <key> of <label>` prefix at its position and returned as `None`; a non-table and a missing key)

## docs/resources.md — modify
- **Location:** introduction, second paragraph ("Each deposit emits its resource type's `output` ...")
- **Change:** replace

Each deposit emits its resource type's `output` of its good at the start of every turn (see [world.md](world.md)): into its owner's stockpile if a business owns it, and into its hex's stockpile of unowned goods otherwise. Ownership is written on the business, not on the deposit (see [businesses.md](businesses.md)). There is no deposit stock, depletion, or regeneration (later). The engine does not require the produced good to be unmovable or to perish after one period; the setting decides.

## docs/resources.md — modify
- **Location:** "Setting format", the paragraph beginning "An entry with `count = 3` places three separate deposits."
- **Change:** replace its first sentence

An entry with `count = 3` places three separate deposits. They are identical at the start, each has its own `DepositId`, and each can have a different single owner: a `[[business]]` claims deposits by resource type and count on its hex, and the loader hands them out in deposit ID order (see [businesses.md](businesses.md)).

## docs/resources.md — modify
- **Location:** "Types", the `resources.rs` heading line and the `Deposit` and `Deposits` bullets
- **Change:** replace

`resources.rs` (depends on `businesses` (for `BusinessId`), `goods`, `map`, `quantity`, `id`; knows nothing about files):

- `Deposit { resource: ResourceId, hex: HexId, owner: Option<BusinessId> }`. `owner` is the business that owns the deposit, if any. Only the loader sets it, and nothing changes it during play yet.
- `Deposits`: an `IdVec<DepositKind, Deposit>` plus `by_hex`, an `IdVec<HexKind, Vec<DepositId>>` indexed by `HexId` (hex IDs are dense, so no hash map is needed). `get`, `len`, `is_empty`, `iter()`, `on_hex(hex) -> &[DepositId]`, `of_type_on_hex(hex, resource)`, and `owned_by(business)`, an iterator over the deposits that business owns (a filter over every deposit); lists are in ID order. `get` panics with `is not from this list`, and `on_hex` panics the same way for a hex beyond the map the deposits were built with, so `Deposits::default()` (which has no hexes) panics for any hex. `from_deposits(deposits, hex_count)` is `pub(crate)`; it numbers deposits 0, 1, 2, ... in the order given and panics (`is not from a map of N hexes`) on a hex index `>= hex_count`. `Pops` and `Businesses` (see [pops.md](pops.md) and [businesses.md](businesses.md)) mirror this shape: a list in load order plus a per-hex index.

(The other bullets are unchanged.)

## docs/resources.md — modify
- **Location:** "How resources are loaded", the `DepositsReader` list: the `resolve` bullet, and a new bullet after it
- **Change:** replace the `resolve` bullet and add the `claim` bullet

- `resolve` (phase 2) resolves each entry's resource through the resource types' `Definitions`, and its hex through `MapReader::id` (`no hex at (9, 9)` at the `[[deposit]]` header, because the coordinates are two keys). If both resolve, it pushes `count` copies of `Deposit { resource, hex, owner: None }`, so deposits are numbered in file order and, within an entry, consecutively. A running total guards against more than `u32::MAX` deposits (`too many deposits in total ...` at the header), so `from_deposits` can never panic on setting data.
- `claim(hex, resource, count, owner) -> Result<(), u32>` (phase 2, after `resolve`) gives the first `count` unowned deposits of `resource` on `hex`, in ID order, to `owner`, or, if fewer than `count` are unowned, assigns none and returns how many were available. `BusinessesReader::resolve` calls it for each business in load order and reports a shortfall (see [businesses.md](businesses.md#how-businesses-are-loaded)).

## docs/resources.md — modify
- **Location:** "How resources are loaded", the last paragraph ("References are checked in `WorldContent::resolve` ...")
- **Change:** replace its third sentence (the one about `resources_broken_file`)

`resources_broken_file` also holds a hex with an unknown currency, a market that prices goods that don't exist, a POP type whose endowment and utility name goods that don't exist, a technology whose goods don't exist, a POP of an unknown type at coordinates with no hex, and a business whose good and resource type don't exist on a hex that doesn't exist, which are likewise not reported (and no deposit is claimed).

## docs/resources.md — modify
- **Location:** "Using resources", the paragraph after the code block
- **Change:** replace

Output is per deposit; a hex's total is the sum over its deposits (the number of deposits times `output` for a single type). Output is emitted per deposit per turn, in deposit ID order, so a hex's stockpile holds the sum over its unowned deposits, and a business's stockpile the sum over the deposits it owns (`deposits.owned_by(business)`).

## docs/resources.md — modify
- **Location:** "The sample setting and tests", second bullet (`settings/sample/start/deposits.toml`)
- **Change:** add, at the end of the bullet

Deposits 0 and 1 are owned by `center_farm` and deposit 3 by `center_mine` (see [businesses.md](businesses.md)); deposit 2 and every deposit off the center are unowned, so the center hex's stockpile receives 4 `unimproved_grain` a turn.

## docs/resources.md — modify
- **Location:** "The sample setting and tests", the last bullet (unit tests)
- **Change:** replace

- Integration tests: `crates/engine/tests/resources.rs`; deposit ownership in `crates/engine/tests/businesses.rs`. Unit tests: `quantity.rs`, `resources.rs` (the `Deposits` behavior, including `owned_by`), `setting/resources.rs` (`deposit_count` and `claim`), and `setting/reader.rs` (`required_number`, `positive_quantity`). The `ResourceTypes` list behavior is covered by the `NamedList` tests in `id.rs`.

## docs/resources.md — modify
- **Location:** "Planned extensions", the last sentence ("Stage 04 (businesses) gives each deposit one owner, ...")
- **Change:** replace

Ownership exists (see [businesses.md](businesses.md)); changing a deposit's owner during play is later.

## docs/world.md — modify
- **Location:** "What a World holds", the table: the `map`/`deposits` row and the `hex_stockpiles` row, and two new rows after `pop_outcomes`
- **Change:** replace the two rows and add two

| `map: HexMap`, `deposits: Deposits` | the setting's `StartState` | not yet; they are world state because later Stages change them. Each deposit's `owner` is set by the loader and nothing changes it yet (see [businesses.md](businesses.md)) |
| `hex_stockpiles: IdVec<HexKind, Stockpile>` | empty at creation | goods enter by emission from unowned deposits and leave by perishing; nothing in the turn trades them |

and, after the `pop_outcomes` row:

| `businesses: Businesses` | the setting's `StartState::businesses` | not yet (see [businesses.md](businesses.md)) |
| `business_stockpiles: IdVec<BusinessKind, Stockpile>` | one per business at creation, filled from its starting inventory | goods enter by emission from the deposits it owns and leave by perishing; nothing trades them yet |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table ("The world owns the setting, ...")
- **Change:** replace

The world owns the setting, so callers (the dev UI) have no lifetimes to manage. There is one stockpile of **unowned goods** per hex, in an `IdVec` indexed by `HexId`, one stockpile per POP, the goods that POP owns, in an `IdVec` indexed by `PopId`, and one stockpile per business, the goods and cash that business owns, in an `IdVec` indexed by `BusinessId` (see [ids.md](ids.md)). All use the same `Stockpile` type. An owned deposit's output goes to its owner's stockpile, an unowned deposit's to its hex's. Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)`, `hex_stockpile(hex)`, `pops()`, `pop_stockpile(pop)`, `pop_outcome(pop)`, `businesses()` and `business_stockpile(business)`. `market(hex)` returns the hex's `Market` view (its prices in the hex's currency and the exchange, at the current turn; see [market.md](market.md)); because it borrows the world, the world can't be stepped while a `Market` is alive. `pop_outcome(pop)` returns the POP's `PopOutcome` from the current turn's trade round (see [trade.md](trade.md)). `hex_currency`, `market` and `hex_stockpile` panic with `is not from this list` for a hex from another map, `pop_stockpile` and `pop_outcome` for a POP from another world's `Pops`, and `business_stockpile` for a business from another world's `Businesses` (a caller bug). The turn reads prices through the trade round but changes neither currencies nor prices.

## docs/world.md — modify
- **Location:** "The turn", items 1 and 3 of the numbered list
- **Change:** replace

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good, into its owner's stockpile if a business owns it and into its hex's stockpile otherwise (see [Emission](#emission)). Then every POP receives its type's endowment times its size into its own stockpile (see [Endowment](#endowment)).

3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile, then from every POP stockpile, then from every business stockpile.

## docs/world.md — modify
- **Location:** "The turn", the first two bullets after "Two calls drive the clock"
- **Change:** replace

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 with empty hex and POP stockpiles and each business's stockpile filled from its starting inventory (each good in the group it would get if added in turn 0, `PerishesAt::for_good(perishes, Turn::ZERO)`), then runs `run_turn` for turn 0, so a fresh world's deposits have emitted on top of that and its POPs have already been endowed, traded, consumed and grown. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing (sizes, outcomes and business stockpiles included). Then it runs the end of the current turn (`perish`: hexes in ID order, then POPs in ID order, then businesses in ID order), advances the clock, and runs `run_turn` for the new turn with what perished.

## docs/world.md — modify
- **Location:** "The turn", the paragraph beginning "So between calls the world is always **after part 2**"
- **Change:** replace its first sentence

So between calls the world is always **after part 2**: this turn's output is in the hex stockpiles (unowned deposits) and the business stockpiles (owned deposits), this turn's trade is done, each POP's stockpile holds what the round left, each POP's size is its size after growth, and nothing has perished yet this turn.

## docs/world.md — modify
- **Location:** "Emission", both paragraphs
- **Change:** replace

`World::emit` (private) walks `deposits.iter()` in deposit ID order. For each deposit it looks up the resource type, computes `PerishesAt::for_good(good.perishes, self.turn)`, picks the target stockpile (the owner's in `business_stockpiles` if `deposit.owner` is `Some`, otherwise the deposit's hex's in `hex_stockpiles`), adds the type's `output` to it, and records `(good, output)`. Each deposit adds its output once; there is no multiplication by a count, so the order of floating-point additions is fixed. Several deposits of one good emitting into one stockpile merge into one group. The private `totals_by_good` helper then sums the recorded amounts per good (with `Quantity::saturating_add`) and returns them in good ID order, leaving out zero totals; these become `TurnReport::emitted`, one total over every deposit whether owned or not.

The hex stockpile stays the home of goods nobody owns. Ownership is set by the loader and doesn't change during play yet (see [businesses.md](businesses.md)).

## docs/world.md — modify
- **Location:** "Perishing"
- **Change:** replace

`World::perish` (private) calls `Stockpile::remove_perishing(self.turn)` on every hex stockpile in hex ID order, then on every POP stockpile in POP ID order, then on every business stockpile in business ID order, and passes all the removed amounts through `totals_by_good`. The totals, hexes, POPs and businesses together, become `TurnReport::perished`.

## docs/world.md — modify
- **Location:** "Determinism"
- **Change:** replace its first sentence

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and goods in ID order within a POP, with every sale on a hex before any purchase and then consumption, POPs grow in ID order, stockpiles perish hexes first, then POPs, then businesses, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds (POP sizes and business stockpiles included), outcomes and reports.

## docs/world.md — modify
- **Location:** "TurnReport", the code block's `perished` and `emitted` lines
- **Change:** replace

```rust
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn, hexes, POPs and businesses
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted by deposits at the start of `turn`, owned or not
```

## docs/world.md — modify
- **Location:** "TurnReport", the paragraph after the code block, its first sentence
- **Change:** replace

Each list has one total per good, summed over every hex (and, for `perished`, every POP and business stockpile; for `emitted`, every deposit, whichever stockpile it emits into; for `endowed`, `sold`, `bought` and `consumed`, every POP), in good ID order, and lists only amounts greater than zero.

## docs/world.md — modify
- **Location:** "TurnReport", the same paragraph, the last sentence ("Because the POPs grow, ...")
- **Change:** add, after it

The sample's businesses don't change these lines: they own three of the center's deposits, so part of the deposit output goes to their stockpiles instead of the center hex's, but `emitted` counts every deposit and that output perishes from the business stockpiles at the same time it would have perished from the hex's.

## docs/world.md — modify
- **Location:** "Stockpile", the first sentence of the section
- **Change:** replace

`Stockpile` is a general type, not specific to hexes. Hexes use it for unowned goods, POPs for what they own (see [pops.md](pops.md)), and businesses for their goods and cash (see [businesses.md](businesses.md)); Stage 03's factories will take their inputs from and put their output into their owner's stockpile.

## docs/world.md — modify
- **Location:** "Tests", first bullet, the `world.rs` item
- **Change:** replace that item

`world.rs` (a world forced to `Turn::MAX`, for `turns_valid`, `pops_placed` and `businesses_valid`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles, business stockpiles, POP outcomes and POP sizes unchanged)

## docs/world.md — modify
- **Location:** "Tests", second bullet (`crates/engine/tests/world.rs`)
- **Change:** replace its first clause, and add a sentence at the end

`crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles (the center hex holds only its unowned deposit's 4 `unimproved_grain`, since the rest of the center's output goes to its businesses) and reports (the POPs' labor in `endowed`, 10 × the previous report's `population` after turn 0, and only the deposit output in `perished`, since POPs use up their labor),

Business stockpiles, emission to owners and perishing from business stockpiles are tested in `crates/engine/tests/businesses.rs` (see [businesses.md](businesses.md)).

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, and businesses acting in the "actors act" part of the turn (they exist and hold goods, but decide and trade nothing until later Steps of Stage 03).

## docs/pops.md — modify
- **Location:** "Placed POPs", "Data model", the sentence "`Pops` mirrors `Deposits` (see [resources.md](resources.md)): a list in load order plus a per-hex index."
- **Change:** replace

`Pops` mirrors `Deposits` (see [resources.md](resources.md)): a list in load order plus a per-hex index. `Businesses` follows the same shape with a named list (see [businesses.md](businesses.md)).

## docs/pops.md — modify
- **Location:** "Endowment in the turn", the last sentences ("At the end of the turn, POP stockpiles perish after the hex stockpiles. See [world.md](world.md) for the turn as a whole.")
- **Change:** replace

At the end of the turn, POP stockpiles perish after the hex stockpiles and before the business stockpiles. See [world.md](world.md) for the turn as a whole.

## docs/ids.md — modify
- **Location:** "Types", the `IdVec<K, T>` bullet, its last sentence
- **Change:** replace

`HexMap`, `Deposits`, `HexCurrencies`, `HexPrices`, `Pops`, `Businesses` and `World` use it.

## docs/ids.md — modify
- **Location:** "Types", the `NamedList<K, T>` bullet, its last sentence
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same sentence

`Goods`, `ResourceTypes`, `PopTypes` and `Technologies` are aliases of it, and `Businesses` wraps one and adds a per-hex index (see [businesses.md](businesses.md)).

## docs/ids.md — modify
- **Location:** "Adding a new ID kind", step 3, the last sentence ("`ResourceKind` / `ResourceId` and `PopTypeKind` / `PopTypeId` ...")
- **Change:** replace

`ResourceKind` / `ResourceId`, `PopTypeKind` / `PopTypeId` (label `"pop type"`) and `TechnologyKind` / `TechnologyId` are further named kinds. `BusinessKind` / `BusinessId` (label `"business"`) is the named kind in the starting state: its `Businesses` list wraps a `NamedList` and adds a per-hex index like `Pops` (see [businesses.md](businesses.md)).

## docs/map.md — modify
- **Location:** "How hexes are loaded" (the paragraph beginning "Duplicates are detected by the reader's own coordinate table"), the two lists of users
- **Change:** replace "(deposits, markets and POPs use it)" with "(deposits, markets, POPs and businesses use it)" and "(hexes, deposits, markets and POPs)" with "(hexes, deposits, markets, POPs and businesses)"

## docs/currency.md — modify
- **Location:** introduction, end of the first paragraph ("Every hex designates one good as its **currency** ...")
- **Change:** add

A business's cash is the amount of its hex's currency in its stockpile; its starting cash is that currency listed in its `[[business]]` `inventory` (see [businesses.md](businesses.md)).

## docs/technologies.md — modify
- **Location:** "Not yet" (the file Step 01 adds)
- **Change:** replace
- **Supersedes:** Step 01 (technologies), its "Not yet" section, whose Step numbers predate the Stage 03 re-plan

Factories (Step 03), the factory mechanics of running, depreciation and maintenance (Step 04), the view a business is served and its decisions (Step 05), the first decider (Step 06), the decider host (Step 07), business trade with factories running in the turn (Step 08), and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Businesses, which will own the factories, exist already (see [businesses.md](businesses.md)). Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries
- **Change:** add one line after `src/bundle.rs` (which Step 01 adds), before `src/currency.rs`

```
    src/businesses.rs   businesses and their per-hex index (see businesses.md)
```

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/currency.rs, ...` entry
- **Change:** replace
- **Supersedes:** Step 01 (technologies), the same entry

```
    tests/businesses.rs, currency.rs, demand.rs, goods.rs, growth.rs, map.rs, market.rs,
          pops.rs, resources.rs, setting_loader.rs, technologies.rs, trade.rs, world.rs
```

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "For the panels", the `deposit_lines(world, hex)` bullet, its last sentence
- **Change:** replace

The total is the output added to itself `count` times with `Quantity::saturating_add` rather than multiplied, the same way emission builds a stockpile by adding each deposit's output in turn. The lines list every deposit on the hex, owned or not; when a business owns some of them, the hex stockpile holds only the unowned deposits' output, so it is less than the line's total.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "For the panels", the `good_rows(world)` bullet
- **Change:** add, at the end of the bullet

Business stockpiles are not counted in either column yet, so the output of owned deposits and the businesses' cash appear nowhere in the table (in the sample, the center farm's and mine's output and the businesses' silver); showing businesses is Step 09 of Stage 03.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text.rs` sub-bullets: the first one (about `error_lines` ... "deposit and stockpile lines on the sample and `turns_valid`") and the one about goods' `in_hexes`
- **Change:** replace those two sub-bullets
- **Supersedes:** Step 01 (technologies), its replacement of the goods sub-bullet

  - `error_lines` for `many_errors`, countdown and `lasts` wording, and deposit and stockpile lines on the sample (the center hex's stockpile at turn 0 is only `4 unimproved_grain`, `4: perishes this turn`, its unowned deposit's output) and `turns_valid`;
  - goods' `in_hexes` totals on the sample (at turn 0 `0, 7.5, 16, 0, 0, 0, 0` in good ID order, only unowned deposit output; the same after steps; `tools` reads `0`, since nothing holds it) and `turns_valid`, and `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`;

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, its second sentence
- **Change:** replace the parenthetical list of counts
- **Supersedes:** Step 01 (technologies), the same sentence

Every expected success line includes the markets, POP types, POPs, technologies and businesses counts (`18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `1 technology` for `technologies_one`, `1 business` for `businesses_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists and its `population` (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0` for settings without POPs). The sample's turn lines are unchanged by its businesses.
