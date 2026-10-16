# Step 01: technologies — Docs Changes

## docs/technologies.md — new file
- **Location:** whole file
- **Change:** add

# Technologies

Source: `crates/engine/src/technologies.rs` and `crates/engine/src/bundle.rs` (the data model) and `crates/engine/src/setting/technologies.rs` (reading `[[technology]]` sections). The public types are re-exported from the engine crate root: `TechnologyKind`, `TechnologyId`, `Technology`, `Technologies` and `Bundle`.

A **technology** is a named recipe that factories run. It is shared by every factory that uses it, the way a POP type is shared by POPs (see [pops.md](pops.md)): it says what a factory of that kind makes and from what, how much it can make per turn, what a finished factory is made of (its **capital bundle**), what it takes to build one (its **construction bundle** and **construction turns**), and how long its capital lasts (its **lifetime**). Technologies are persistent rules, defined in `rules/` and checked by the loader like POP types. The recipe never changes during play.

Nothing runs technologies yet. Factories, production, depreciation and maintenance, and business trade are later Steps of Stage 03; they will read the numbers defined here by the rules in [What the numbers mean](#what-the-numbers-mean).

## Concepts

- **Inputs** are per unit of output. The recipe is fixed-proportion: making X units of output takes X times each input amount, with no substitution between inputs. The table may be empty: a technology that makes its output from nothing is a legitimate test case, and the engine doesn't police economics.
- **Output** is exactly one good, and it may not also be an input (a recipe that consumes its own product is almost certainly a mistake). It may appear in the capital bundle: a toolmaker made of tools is fine.
- **Capacity** is the most output one factory makes in one turn. It belongs to the technology, so more output means more factories.
- **Capital bundle**: the goods a finished factory holds, and what wears out. It must name at least one good, because a factory with nothing to wear out would never shut down.
- **Construction bundle**: the goods spent to build a factory. They are consumed and don't become part of it. The intended use is labor, but it is a general table of goods, because the engine has no notion of labor. It may be empty.
- **Construction turns** (t) and **lifetime** (t₁): whole numbers of at least 1.
- **No economic checks.** The loader doesn't check that a technology is profitable, that it can be maintained, that its inputs can be had anywhere, or that its capital goods are durable.

## What the numbers mean

These rules are the meaning of a technology's numbers. The later Steps of this Stage implement them; the data shape is chosen to fit them.

- **Depreciation.** Every turn, whether or not it ran, a factory consumes 1/t₁ of its technology's capital bundle, in the bundle's proportions. A factory that starts full is empty after t₁ turns, and it shuts down when its capital is exhausted.
- **Construction.** Building a factory pays 1/t of the capital bundle and 1/t of the construction bundle each turn, for t turns. The capital slices accumulate into the factory; the construction slices are spent. The factory can't run until construction is complete. (Construction itself is a Stage 07 business decision; the parameters exist now because maintenance is defined by them.)
- **Maintenance** is the construction project run for a fraction of its time, while the factory keeps running. Replenishing a share f of the capital bundle costs f of the capital bundle plus f of the construction bundle. It is continuous: any share may be replenished, and exactly that share is paid. The per-turn construction slice is a cap, not a minimum: a factory regains at most 1/t of its bundle per turn, and bringing one from 95% to 100% costs 5% of each bundle.
- **t > t₁ is allowed.** A factory loses 1/t₁ per turn and can regain at most 1/t per turn, so steady maintenance keeps up with depreciation only when t ≤ t₁. A technology with t > t₁ wears out faster than it can be maintained. That is a legitimate thing for a setting to say, so the loader neither rejects nor warns about it.

## Data model

### Bundle

```rust
pub struct Bundle { items: Vec<(GoodId, Quantity)> }   // sorted by good ID, no good twice
```

A `Bundle` is a list of goods with amounts. A technology's inputs, capital and construction are bundles, and later Steps reuse the type for a factory's current capital and for depreciation slices. It is kept sorted by good ID with no good twice, so two bundles with the same goods and amounts compare equal (`PartialEq`) and lookups are binary searches. Amounts are `Quantity` (finite, at least 0; see [resources.md](resources.md)); whether 0 is allowed is up to the owner. A technology's bundles never hold a 0; a factory's capital may later wear down to 0. The module knows nothing about files.

| `Bundle` method | Result |
|---|---|
| `new(items)` (crate-private) | Builds a bundle from (good, amount) pairs in any order, sorting them by good ID. Panics with `a good appears twice in a bundle` on a duplicate good |
| `as_slice() -> &[(GoodId, Quantity)]` | The pairs in good ID order |
| `iter()` | Every `(GoodId, Quantity)` in good ID order, by value |
| `amount_of(good) -> Quantity` | The amount of `good`, or `Quantity::ZERO` if the bundle doesn't name it |
| `contains(good) -> bool` | True if the bundle names `good`, even with amount 0 |
| `len()`, `is_empty()` | The number of goods named |

`Bundle::default()` is the empty bundle. Only construction, lookup and iteration exist; scaling and other arithmetic are added when a system needs them (depreciation and maintenance), following the rule for `Quantity`.

### Technology

```rust
pub enum TechnologyKind {}                    // IdKind, label "technology"
pub type TechnologyId = Id<TechnologyKind>;
pub struct Technology {
    inputs: Bundle, output: GoodId, capacity: Quantity,
    capital: Bundle, construction: Bundle,
    construction_turns: u32, lifetime: u32,
}
pub type Technologies = NamedList<TechnologyKind, Technology>;
```

| `Technology` method | Result |
|---|---|
| `inputs() -> &Bundle` | The inputs one unit of output takes. May be empty; never names `output`; every amount greater than 0 |
| `output() -> GoodId` | The good a factory of this technology makes |
| `capacity() -> Quantity` | The most output a factory makes in one turn. Greater than 0 |
| `capital() -> &Bundle` | The goods a finished factory holds. Never empty; every amount greater than 0 |
| `construction() -> &Bundle` | The goods spent to build a factory. May be empty; every amount greater than 0 |
| `construction_turns() -> u32` | t, the turns building a factory takes. At least 1 |
| `lifetime() -> u32` | t₁, the turns a factory's capital lasts. At least 1 |

The fields are private. `Technology::new(inputs, output, capacity, capital, construction, construction_turns, lifetime)` is crate-private: only the loader builds technologies. It asserts the invariants and panics with an exact message if one fails: `a technology's output good may not be among its inputs`, `a technology's capital bundle must name at least one good`, `a technology's amounts must be greater than 0` (any amount in the three bundles, or the capacity), and `a technology's construction turns and lifetime must be at least 1`. The loader guarantees all of these, so a panic is an internal bug.

`Technologies` is a `NamedList` (see [ids.md](ids.md)): `len`, `is_empty`, `id(name)`, `name(id)`, `get(id)` and `iter()` as `(TechnologyId, &str, &Technology)` in load order, which is also ID order. An unknown name gives `IdError::UnknownName { kind: "technology", .. }`. A setting with no technologies is valid and gives an empty list. `load_setting` stores it in `Rules::technologies`; `World` holds `Rules` whole, so a running world reads it with `world.rules().technologies`. `technologies.rs` depends on `bundle`, `goods`, `id` and `quantity` and knows nothing about files.

## Setting format

A `[[technology]]` belongs in `rules/` (in `start/` it gets the usual `section 'technology' belongs in rules/, not start/`). Eight keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among technologies. Technology names, good names, resource type names and POP type names are separate namespaces |
| `inputs` | A table of `good name = amount`, each amount a finite number greater than 0. May be empty (`{}`). The output good may not appear |
| `output` | The name of one good |
| `capacity` | A finite number greater than 0 |
| `capital` | A table like `inputs`, with at least one good |
| `construction` | A table like `inputs`. May be empty (`{}`) |
| `construction_turns` | A whole number from 1 to 4294967295 (a `u32`) |
| `lifetime` | A whole number from 1 to 4294967295 |

The empty tables must still be written: a missing `inputs` or `construction` is an error, so an empty one says the author meant it. Both TOML table forms are accepted for `inputs`, `capital` and `construction`:

```toml
[[technology]]
name = "smelter"
inputs = { ore = 2, labor = 1 }
output = "iron"
capacity = 10
capital = { tools = 5 }
construction = { labor = 20 }
construction_turns = 2
lifetime = 10

[[technology]]
name = "well"
output = "bread"
capacity = 0.5
construction_turns = 1
lifetime = 1

[technology.inputs]            # empty: bread from nothing

[technology.capital]
tools = 1.5
labor = 3

[technology.construction]      # empty
```

## How technologies are loaded

`setting/technologies.rs` holds `TechnologiesReader`, which follows `PopTypesReader` (see [pops.md](pops.md#how-pop-types-are-loaded)). `WorldContent` has a `technologies: TechnologiesReader` field, and the `technology` section type (folder `Rules`, listed after `pop`) calls `TechnologiesReader::read` for each entry. Technologies are a named kind, so the reader uses `Definitions<TechnologyKind>` and resolves good names in phase 2.

1. **Phase 1, `read`.** Every key is read before anything is defined, so no key is falsely reported as unknown and one entry with several problems reports all of them. In order:
   - `name` with `required_name`.
   - `inputs`, `capital` and `construction` with `EntryReader::required_quantity_table` (see [setting-loader.md](setting-loader.md#entryreader)), which applies `positive_quantity` to every value with the prefix `key '<good>' in <key> of technology`. Every key is kept, with `None` for a bad amount, so its good is still resolved. Each table is held in a private `BundleEntry` (the keys and amounts as written, plus the good IDs filled in phase 2). If `capital` is a table with no keys, `capital of technology needs at least one good` is reported at the entry header.
   - `output` with `required_string`. It is not name-checked: a bad value is simply an unknown good in phase 2.
   - `capacity` with `required_number` and `positive_quantity` (`key 'capacity' in technology must be a finite number greater than 0`).
   - `construction_turns` and `lifetime` with the private `turns` helper: `required_integer`, then a check that the value fits a `u32` and is at least 1. Below 1 is `key '<key>' in technology must be at least 1`, above `u32::MAX` is `key '<key>' in technology is out of range` (the same two shapes as a good's `perishes_after`, see [goods.md](goods.md)); an integer beyond `i64` is already reported as out of range by `required_integer`.
   - **Output among inputs.** If `output` was read, every input key whose name equals it is reported at the key as `key '<good>' in inputs of technology must not be the output good`. Names are compared as written (good names are unique, so equal names are the same good), so this is reported in phase 1, whether or not the goods exist and whether or not the amount was valid.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate technology name 'ok' (first defined at <location>)`).
   - **Every entry is kept** for phase 2 as a `TechnologyEntry` with an optional `TechnologyId` (`None` if the name was missing, invalid or a duplicate), so an entry with a bad name still has its unknown goods reported.
2. **Phase 2, `resolve(goods, sink)`.** For every entry, resolves each key of `inputs`, the `output`, and each key of `capital` and `construction` through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key or value). A good may be defined in any `rules/` file. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `TechnologiesReader::finish` skips entries without an ID, builds each bundle with `Bundle::new` (which sorts by good ID) and each `Technology::new`, and returns `Technologies::from_parts`. It is only called after a successful load, and panics with `internal error: a technology was not fully resolved in a successful load` (or, for a bundle, `internal error: a technology's goods were not resolved in a successful load`) otherwise.

IDs follow load order (file read order, then entry order in each file), so the same folder always gives the same result.

All `[[technology]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| any of the eight keys missing | header | `missing required key 'capital' in technology` |
| a bad name | value | `invalid name 'Bad Name' (names must start with a lowercase letter ...)` |
| a duplicate name | value | `duplicate technology name 'ok' (first defined at <location>)` |
| `inputs`, `capital` or `construction` not a table | value | `key 'inputs' in technology must be a table, found integer` |
| `output` not a string | value | `key 'output' in technology must be a string, found integer` |
| `capacity` not a number | value | `key 'capacity' in technology must be a number, found string` |
| `capacity` zero, negative, `inf` or `nan` | value | `key 'capacity' in technology must be a finite number greater than 0` |
| a bundle amount that isn't a number | value | `key 'grain' in inputs of technology must be a number, found string` |
| a bundle amount that is zero, negative, `inf` or `nan` | value | `key 'labor' in capital of technology must be a finite number greater than 0` |
| `construction_turns` or `lifetime` not an integer | value | `key 'lifetime' in technology must be an integer, found float` |
| `construction_turns` or `lifetime` below 1 | value | `key 'construction_turns' in technology must be at least 1` |
| `construction_turns` or `lifetime` above 4294967295 | value | `key 'lifetime' in technology is out of range` |
| an empty `capital` | header | `capital of technology needs at least one good` |
| the output good among the inputs | key | `key 'grain' in inputs of technology must not be the output good` |
| a good name that matches no good | key (or the `output` value) | `unknown good 'tin'` |
| any other key | key | `unknown key 'colour' in technology` |
| `[[technology]]` in `start/` | header | `section 'technology' belongs in rules/, not start/` |

## Using technologies

```rust
let setting = load_setting(path)?;
let techs = &setting.rules.technologies;
let farm = techs.get(techs.id("farm")?);
let grain = setting.rules.goods.id("grain")?;
assert_eq!(farm.output(), grain);
for (good, per_unit) in farm.inputs().iter() {   // good ID order
    // per_unit × output = what making `output` units takes
}
let tools = setting.rules.goods.id("tools")?;
let per_turn_wear = farm.capital().amount_of(tools).get() / f64::from(farm.lifetime());
```

## Adding a key

1. Add the field to `Technology` with an accessor, and to `Technology::new` with any invariant it needs as an assertion.
2. Read it in `TechnologiesReader::read` before the name is defined, store it on `TechnologyEntry`, and pass it through `finish`. A good reference is kept as written and resolved in `resolve`; a good = amount table uses `required_quantity_table` and a `BundleEntry`.
3. Add it to the sample's `technologies.toml` header comment, the test settings and their expectations, and the tables here and in the manual.

## Not yet

Factories (Step 03), production (Step 04), depreciation and maintenance (Step 05), business trade (Step 06) and showing technologies in the dev UI (Step 07) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## The sample setting and tests

- `settings/sample/rules/technologies.toml` defines three technologies on the center hex's production chain, each with a capital bundle of `tools`, a construction bundle of `unskilled_labor`, and t ≤ t₁:

  | Technology (ID) | Inputs per unit | Output | Capacity | Capital | Construction | t | t₁ |
  |---|---|---|---|---|---|---|---|
  | `farm` (0) | 1 `unimproved_grain`, 0.5 `unskilled_labor` | `grain` | 12 | 4 `tools` | 8 `unskilled_labor` | 2 | 8 |
  | `mine` (1) | 1 `unimproved_copper`, 1 `unskilled_labor` | `copper` | 2.5 | 2 `tools` | 4 `unskilled_labor` | 2 | 8 |
  | `toolmaker` (2) | 2 `copper`, 1.5 `unskilled_labor` | `tools` | 4 | 3 `tools` | 6 `unskilled_labor` | 3 | 24 |

  The capacities match what the center's deposits emit each turn (12 `unimproved_grain`, 2.5 `unimproved_copper`). At the center's prices, revenue at capacity minus inputs minus one lifetime's share of capital plus construction leaves the farm 5 on revenue 24, the mine 4 on 12.5 and the toolmaker 0.25 on 48, so later Steps have both clearly profitable and marginal cases. The file's header comment explains every key, the rules in [What the numbers mean](#what-the-numbers-mean), and these numbers. The sample also gains the `tools` good (see [goods.md](goods.md)) and a price of 12 `silver` for it on the center hex (see [market.md](market.md)). No POP wants `tools` and nothing produces it yet, so the sample's turn reports are unchanged.
- `bundle.rs` unit tests: pairs given out of order come back sorted; `amount_of` gives the amount or zero; `contains` names a good even with amount 0; `len` and `is_empty`; the empty default; the duplicate-good panic.
- `technologies.rs` unit tests: every field stored and returned; an empty input table, an empty construction bundle and the output in the capital bundle are allowed; each `Technology::new` panic with its exact message.
- `setting/reader.rs`: `required_quantity_table` (see [setting-loader.md](setting-loader.md)).
- `crates/engine/tests/technologies.rs`: the sample's three technologies, `tools` as good 6 and its price on the center; `technologies_valid` (across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, both table forms, the goods in a file read after them, empty tables, amounts written out of ID order, the same result on a second load); every `technologies_errors` error at its place, including one entry (`Several`) with many problems, which still reports its unknown goods although its name is invalid; and `valid_minimal` with no technologies and an unknown name giving `IdError::UnknownName`.
- Test settings: `technologies_valid`, `technologies_one` (one good and one technology, for the checker's `1 technology`), `technologies_errors`, and a technology with unknown goods in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.

## docs/README.md — modify
- **Location:** the document table, new row after `pops.md` (before `demand.md`)
- **Change:** add

| [technologies.md](technologies.md) | Technologies, the recipes factories run: inputs, output and capacity, the capital and construction bundles, construction turns and lifetime, what those numbers mean for depreciation, construction and maintenance, the shared `Bundle` type, and how `[[technology]]` sections are loaded and checked |

## docs/setting-loader.md — modify
- **Location:** introduction, the sentence "The loader defines seven real section types so far: ..."
- **Change:** replace

The loader defines eight real section types so far: `[[good]]`, `[[resource]]`, `[[pop_type]]` and `[[technology]]` in `rules/` (see [goods.md](goods.md), [resources.md](resources.md), [pops.md](pops.md) and [technologies.md](technologies.md)) and `[[hex]]`, `[[deposit]]`, `[[market]]` and `[[pop]]` in `start/` (see [map.md](map.md), [resources.md](resources.md), [market.md](market.md) and [pops.md](pops.md)); later Stages add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure" table, new row after `pops.rs`
- **Change:** add

| `technologies.rs` | `TechnologiesReader`: reads `[[technology]]` entries, checks that the output good isn't among the inputs, resolves the goods in their `inputs`, `output`, `capital` and `construction` in phase 2, and builds `Technologies` (see [technologies.md](technologies.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph beginning "`WorldContent::resolve` is where real references are checked"
- **Change:** replace

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), then each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency, then the goods in each POP type's `endowment` and `utility` (`PopTypesReader::resolve`), then each POP's `type` against the POP types (`PopTypesReader::defs`) and its coordinates against the map (`PopsReader::resolve`), and finally the goods in each technology's `inputs`, `output`, `capital` and `construction` (`TechnologiesReader::resolve`). The map must resolve before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`); otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`, `PopTypesReader::finish`, `PopsReader::finish`, `TechnologiesReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph beginning "`WorldContent` is the real content type."
- **Change:** replace (the first two sentences and the `load_setting` sentence; the rest of the paragraph is unchanged)

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader`, a `MarketsReader`, a `PopTypesReader`, a `PopsReader` and a `TechnologiesReader`, and returns eight section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`, `pop_type` in `rules/`, `pop` in `start/`, and `technology` in `rules/`. `load_setting` builds `Rules { goods, resources, pop_types, technologies }` and `StartState { map, currencies, deposits, prices, pops }` from it.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the first sub-bullet (the one about `[[good]]`, `[[resource]]` and `[[pop_type]]`)
- **Change:** add, at the end of that sub-bullet

`[[technology]]` (`Rules::technologies`) is another named kind that keeps every entry for phase 2, and the example for tables of good = positive amount read with `required_quantity_table` and stored as a `Bundle`, and for a check between two keys of one entry made in phase 1 by comparing names as written (its output good may not be among its inputs; see [technologies.md](technologies.md)).

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first bullet (the accessor list)
- **Change:** replace the list of accessor names at the start of the bullet

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `optional_number`, `required_number_table`, `required_quantity_table`, `required_number_array`, `required_table_of_tables`, `required_sub_table`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>` (or, for the three table readers, an `Option` of a list, and for `required_sub_table` an `Option<EntryReader>`), where `Located` carries the value and its `Location`.

(The rest of the bullet is unchanged.)

## docs/setting-loader.md — modify
- **Location:** "EntryReader", new bullet after the `required_number_table(key)` bullet
- **Change:** add

- `required_quantity_table(key)` is `required_number_table` with `positive_quantity` applied to every value, using the same prefix (`key '<k>' in <key> of <label>`). It returns `Option<Vec<(Located<String>, Option<Quantity>)>>`: every key in the table's order, with `None` for a value that isn't a number, is out of range, or isn't finite and greater than 0 (each reported at the value), so the caller still sees and resolves the key. Missing and non-table `key` are handled as for `required_number_table`. A technology's `inputs`, `capital` and `construction` use it; a market's `prices` and a POP type's `endowment` still call `required_number_table` and `positive_quantity` themselves.

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the "Range checks on numbers" bullet, its last sentence
- **Change:** replace

A resource type's `output`, a POP's `size`, a market's prices, a POP type's endowment amounts, and a technology's `capacity` and bundle amounts use the first; a utility good's `subsistence` and `share` and a growth table's `max_growth` use the second (`decline` keeps its own message, because its range is different).

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", first bullet
- **Change:** replace

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (seven goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `rules/pops.toml` (two POP types, see [pops.md](pops.md)), `rules/technologies.toml` (three technologies, see [technologies.md](technologies.md)), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), `start/pops.toml` (16 POPs, see [pops.md](pops.md)), plus `README.md` files. It must always load without errors.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", second bullet, the list of test settings
- **Change:** add `technologies_valid`, `technologies_one` and `technologies_errors` to the list, after `pops_growth`

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph beginning "The success line is"
- **Change:** replace (its first three sentences; the rest of the paragraph is unchanged)

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>, <pop types>, <pops>, <technologies>`, for example `7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies` or `1 good, 0 hexes, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 1 technology`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. POP types are counted with `Rules::pop_types.len()`, POPs with `StartState::pops.len()`, and technologies with `Rules::technologies.len()`.

## docs/setting-loader.md — modify
- **Location:** "Tests", first bullet, the list of what the `reader.rs` tests cover
- **Change:** add, after the `required_number_array` item

, and `required_quantity_table` (valid amounts returned as quantities; a string, a zero and an `inf` value each reported with the `key '<k>' in <key> of <label>` prefix at its position and returned as `None`; a non-table and a missing key)

## docs/goods.md — modify
- **Location:** "The sample setting and tests", first paragraph
- **Change:** replace

`settings/sample/rules/goods.toml` defines seven goods in this order: `unskilled_labor` (perishes after 1, not movable), `unimproved_copper` (1, not movable), `unimproved_grain` (1, not movable), `copper` (never, movable), `grain` (3, movable), `silver` (never, movable), `tools` (never, movable). They cover each combination of the properties and give the resource types something to produce; `unskilled_labor` is what the sample's POP types are endowed with (see [pops.md](pops.md)), `silver` is the currency every sample hex designates (see [currency.md](currency.md)), and `tools` is the capital good the sample's technologies are made of (see [technologies.md](technologies.md)).

## docs/market.md — modify
- **Location:** "The sample setting and tests", first bullet
- **Change:** replace

- `settings/sample/start/markets.toml` holds 18 markets, one per hex of the map in the map's order, skipping (2, -2), which has none. Every market prices `unskilled_labor` 1, `grain` 2 and `copper` 5 in `silver`; the center (0, 0) also prices `unimproved_grain` 0.5, `unimproved_copper` 1 and `tools` 12 (the only hex that trades resource output, and the only hex that trades tools). So the sample has a hex that trades every non-currency good, hexes that trade some goods, and a hex that trades nothing. Nothing is produced for sale yet; the center's prices are the ones the sample's technologies are balanced against (see [technologies.md](technologies.md)). The file's header comment explains the keys.

## docs/market.md — modify
- **Location:** "The sample setting and tests", second bullet, its opening clause
- **Change:** replace "the sample's prices on the center, a typical hex and the edge hex without a market (and `silver` at 1 everywhere, 18 markets)" with

the sample's prices on the center (six goods, `tools` among them), a typical hex (which doesn't price `tools`) and the edge hex without a market (and `silver` at 1 everywhere, 18 markets)

## docs/ids.md — modify
- **Location:** the `NamedList<K, T>` bullet, its last sentence
- **Change:** replace

`Goods`, `ResourceTypes`, `PopTypes` and `Technologies` are aliases of it.

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries
- **Change:** add two lines, keeping alphabetical order

```
    src/bundle.rs       Bundle: goods with amounts (see technologies.md)
```
before `src/currency.rs`, and
```
    src/technologies.rs technologies, the recipes factories run (see technologies.md)
```
after `src/stockpile.rs`.

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/currency.rs, ...` entry
- **Change:** replace

```
    tests/currency.rs, demand.rs, goods.rs, growth.rs, map.rs, market.rs, pops.rs,
          resources.rs, setting_loader.rs, technologies.rs, trade.rs, world.rs
```

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text.rs` sub-bullets about goods and `market_lines`
- **Change:** replace those two sub-bullets

  - goods' `in_hexes` totals on the sample (at turn 0 and after steps; `tools` reads `0`, since nothing holds it) and `turns_valid`, and `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`;
  - `market_lines` on the sample (hex (0, 0)'s prices, ending with `tools: 12 silver`, and `No market.` on (2, -2)) and on `markets_valid` (prices, `No prices posted.`, `No market.`);

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, its second sentence
- **Change:** replace the parenthetical list of counts

Every expected success line includes the markets, POP types, POPs and technologies counts (`18 markets, 2 pop types, 16 pops, 3 technologies` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `1 technology` for `technologies_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists and its `population` (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0` for settings without POPs).
