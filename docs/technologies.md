# Technologies

Source: `crates/engine/src/technologies.rs` and `crates/engine/src/bundle.rs` (the data model) and `crates/engine/src/setting/technologies.rs` (reading `[[technology]]` sections). The public types are re-exported from the engine crate root: `TechnologyKind`, `TechnologyId`, `Technology`, `Technologies` and `Bundle`.

A **technology** is a named recipe that factories run. It is shared by every factory that uses it, the way a POP type is shared by POPs (see [pops.md](pops.md)): it says what a factory of that kind makes and from what, how much it can make per turn, what a finished factory is made of (its **capital bundle**), what it takes to build one (its **construction bundle** and **construction turns**), and how long its capital lasts (its **lifetime**). Technologies are persistent rules, defined in `rules/` and checked by the loader like POP types. The recipe never changes during play.

Factories (see [factories.md](factories.md)) run technologies. The factory mechanics in [production.md](production.md) implement the rules in [What the numbers mean](#what-the-numbers-mean), and the turn runs them for every business's factories right after its hex's trade round (see [trade.md](trade.md#operating-the-factories)).

## Concepts

- **Inputs** are per unit of output. The recipe is fixed-proportion: making X units of output takes X times each input amount, with no substitution between inputs. The table may be empty: a technology that makes its output from nothing is a legitimate test case, and the engine doesn't police economics.
- **Output** is exactly one good, and it may not also be an input (a recipe that consumes its own product is almost certainly a mistake). It may appear in the capital bundle: a toolmaker made of tools is fine.
- **Capacity** is the most output one factory makes in one turn. It belongs to the technology, so more output means more factories.
- **Capital bundle**: the goods a finished factory holds, and what wears out. It must name at least one good, because a factory with nothing to wear out would never shut down.
- **Construction bundle**: the goods spent to build a factory. They are consumed and don't become part of it. The intended use is labor, but it is a general table of goods, because the engine has no notion of labor. It may be empty.
- **Construction turns** (t) and **lifetime** (t₁): whole numbers of at least 1.
- **No economic checks.** The loader doesn't check that a technology is profitable, that it can be maintained, that its inputs can be had anywhere, or that its capital goods are durable.

## What the numbers mean

These rules are the meaning of a technology's numbers; the factory mechanics implement them (see [production.md](production.md)).

- **Depreciation.** Every turn, whether or not it ran, a factory consumes 1/t₁ of its technology's capital bundle (one **lifetime-slice**), in the bundle's proportions, stopping at zero. A factory that starts full and is never maintained runs exactly t₁ turns. A factory runs only while it holds at least one whole slice, so every turn of output costs exactly one slice. Below that it is shut down and makes nothing, and a partial slice is lost to wear. Condition doesn't slow a factory: it runs at full capacity while it holds a slice.
- **Construction.** Building a factory pays 1/t of the capital bundle and 1/t of the construction bundle each turn, for t turns. The capital slices accumulate into the factory; the construction slices are spent. The factory can't run until construction is complete. (Construction itself is a Stage 07 business decision; the parameters exist now because maintenance is defined by them.)
- **Maintenance** is the construction project run for a fraction of its time, while the factory keeps running. Replenishing a share f of the capital bundle costs f of the capital bundle plus f of the construction bundle. It is continuous: any share may be replenished, and exactly that share is paid. The per-turn construction slice is a cap, not a minimum: a factory regains at most 1/t of its bundle per turn, and bringing one from 95% to 100% costs 5% of each bundle.
- **t > t₁ is allowed.** A factory loses 1/t₁ per turn and can regain at most 1/t per turn, so steady maintenance keeps up with depreciation only when t ≤ t₁. A technology with t > t₁ wears out faster than it can be maintained, and once a factory of it is exhausted it can't be revived from zero, because maintenance never brings it up to a whole slice. That is a legitimate thing for a setting to say, so the loader neither rejects nor warns about it.
- **Order.** Each turn a business's factories run, then wear, then are maintained. Running before wear gives a full factory exactly t₁ turns. Maintaining after wear lets one slice of maintenance per turn hold a factory at full condition. See [production.md](production.md#order-within-a-turn).

## Data model

### Bundle

```rust
pub struct Bundle { items: Vec<(GoodId, Quantity)> }   // sorted by good ID, no good twice
```

A `Bundle` is a list of goods with amounts. A technology's inputs, capital and construction are bundles. The engine also uses the type for its working lists of goods with amounts, such as the inputs a factory run uses and the goods maintenance costs (see [production.md](production.md)), and for a business's starting inventory (see [businesses.md](businesses.md)). It is kept sorted by good ID with no good twice, so two bundles with the same goods and amounts compare equal (`PartialEq`) and lookups are binary searches. Amounts are `Quantity` (finite, at least 0; see [resources.md](resources.md)); whether 0 is allowed is up to the owner. A technology's bundles never hold a 0. The module knows nothing about files.

| `Bundle` method | Result |
|---|---|
| `new(items)` (crate-private) | Builds a bundle from (good, amount) pairs in any order, sorting them by good ID. Panics with `a good appears twice in a bundle` on a duplicate good |
| `as_slice() -> &[(GoodId, Quantity)]` | The pairs in good ID order |
| `iter()` | Every `(GoodId, Quantity)` in good ID order, by value |
| `amount_of(good) -> Quantity` | The amount of `good`, or `Quantity::ZERO` if the bundle doesn't name it |
| `scaled(factor: Quantity) -> Bundle` | Every amount multiplied by `factor` (saturating). The goods stay named even when `factor` is 0 |
| `merged(&other) -> Bundle` | The goods of both bundles in good ID order, with the amounts of a good named in both added (saturating) |
| `contains(good) -> bool` | True if the bundle names `good`, even with amount 0 |
| `len()`, `is_empty()` | The number of goods named |

`Bundle::default()` is the empty bundle. Beyond construction, lookup and iteration, the only arithmetic is `scaled` and `merged`, which the factory mechanics need: the inputs of a run are `inputs().scaled(output)`, and the cost of maintenance is `capital().merged(construction()).scaled(f)`. Further arithmetic is added when a system needs it, following the rule for `Quantity`.

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
| `name` | Lowercase `snake_case`, unique among technologies. Technology names, good names, resource type names, POP type names and business names are separate namespaces |
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
   - `construction_turns` and `lifetime` with the private `turns` helper: `required_integer`, then `EntryReader::at_least_one` with the prefix `key '<key>' in technology` and no upper bound, the shared check for a whole number of at least 1 that fits a `u32` (see [setting-loader.md](setting-loader.md#entryreader)). Below 1 is `key '<key>' in technology must be at least 1`, above `u32::MAX` is `key '<key>' in technology is out of range` (the same two shapes as a good's `perishes_after`, see [goods.md](goods.md)); an integer beyond `i64` is already reported as out of range by `required_integer`.
   - **Output among inputs.** If `output` was read, every input key whose name equals it is reported at the key as `key '<good>' in inputs of technology must not be the output good`. Names are compared as written (good names are unique, so equal names are the same good), so this is reported in phase 1, whether or not the goods exist and whether or not the amount was valid.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate technology name 'ok' (first defined at <location>)`).
   - **Every entry is kept** for phase 2 as a `TechnologyEntry` with an optional `TechnologyId` (`None` if the name was missing, invalid or a duplicate), so an entry with a bad name still has its unknown goods reported.
2. **Phase 2, `resolve(goods, sink)`.** For every entry, resolves each key of `inputs`, the `output`, and each key of `capital` and `construction` through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key or value). A good may be defined in any `rules/` file. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `TechnologiesReader::finish` skips entries without an ID, builds each bundle with `Bundle::new` (which sorts by good ID) and each `Technology::new`, and returns `Technologies::from_parts`. It is only called after a successful load, and panics with `internal error: a technology was not fully resolved in a successful load` (or, for a bundle, `internal error: a technology's goods were not resolved in a successful load`) otherwise.

`TechnologiesReader::defs()` gives the technology `Definitions`, which the factories reader uses to resolve the technology a factory names (see [factories.md](factories.md)).

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
let per_turn_wear = farm.capital().scaled(Quantity::new(1.0 / f64::from(farm.lifetime()))?); // one lifetime-slice
let maintenance_per_unit = farm.capital().merged(farm.construction());              // cost of f = this × f
```

A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)), and the arithmetic rule tests each technology's margin at posted prices from those numbers (see [python.md](python.md#the-margin-test-per-technology)). The dev UI's world panel lists every technology with its numbers and the number of factories running it (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## Adding a key

1. Add the field to `Technology` with an accessor, and to `Technology::new` with any invariant it needs as an assertion.
2. Read it in `TechnologiesReader::read` before the name is defined, store it on `TechnologyEntry`, and pass it through `finish`. A good reference is kept as written and resolved in `resolve`; a good = amount table uses `required_quantity_table` and a `BundleEntry`.
3. Add it to the sample's `technologies.toml` header comment, the test settings and their expectations, the `TechnologyView` of the decider protocol and its Python dataclass (see [decider.md](decider.md#extending)), and the tables here and in the manual.

## Not yet

Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## The sample setting and tests

- `settings/sample/rules/technologies.toml` defines three technologies on the center hex's production chain, each with a capital bundle of `tools`, a construction bundle of `unskilled_labor`, and t ≤ t₁:

  | Technology (ID) | Inputs per unit | Output | Capacity | Capital | Construction | t | t₁ |
  |---|---|---|---|---|---|---|---|
  | `farm` (0) | 1 `unimproved_grain`, 0.5 `unskilled_labor` | `grain` | 12 | 4 `tools` | 8 `unskilled_labor` | 2 | 8 |
  | `mine` (1) | 1 `unimproved_copper`, 1 `unskilled_labor` | `copper` | 2.5 | 2 `tools` | 4 `unskilled_labor` | 2 | 8 |
  | `toolmaker` (2) | 2 `copper`, 1.5 `unskilled_labor` | `tools` | 4 | 3 `tools` | 6 `unskilled_labor` | 3 | 24 |

  The capacities match what the center's deposits emit each turn (12 `unimproved_grain`, 2.5 `unimproved_copper`). At the center's prices, revenue at capacity minus inputs minus one lifetime's share of capital plus construction leaves the farm 5 on revenue 24, the mine 4 on 12.5 and the toolmaker 0.25 on 48, so the sample has both clearly profitable and marginal cases. The file's header comment explains every key, the rules in [What the numbers mean](#what-the-numbers-mean), and these numbers. The sample's `tools` good (see [goods.md](goods.md)) has a price of 12 `silver` on the center hex (see [market.md](market.md)). The sample places one factory of each technology on the center hex, the mine's at condition 0.75 (see [factories.md](factories.md)).
- `bundle.rs` unit tests: pairs given out of order come back sorted; `amount_of` gives the amount or zero; `contains` names a good even with amount 0; `len` and `is_empty`; the empty default; the duplicate-good panic; `scaled` multiplies every amount and keeps zero-scaled goods named; `merged` adds shared goods, keeps the rest sorted and gives the same result either way round; merging with an empty bundle gives a copy.
- `technologies.rs` unit tests: every field stored and returned; an empty input table, an empty construction bundle and the output in the capital bundle are allowed; each `Technology::new` panic with its exact message.
- `setting/reader.rs`: `required_quantity_table` and `at_least_one` (see [setting-loader.md](setting-loader.md)).
- `crates/engine/tests/technologies.rs`: the sample's three technologies, `tools` as good 6 and its price on the center; `technologies_valid` (across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, both table forms, the goods in a file read after them, empty tables, amounts written out of ID order, the same result on a second load); every `technologies_errors` error at its place, including one entry (`Several`) with many problems, which still reports its unknown goods although its name is invalid; and `valid_minimal` with no technologies and an unknown name giving `IdError::UnknownName`.
- Test settings: `technologies_valid`, `technologies_one` (one good and one technology, for the checker's `1 technology`), `technologies_errors`, and a technology with unknown goods in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.
