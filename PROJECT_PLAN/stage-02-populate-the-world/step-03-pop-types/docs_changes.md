# Step 03: pop-types — Docs Changes

## docs/pops.md — new file
- **Location:** whole file
- **Change:** add

# POPs

Source: `crates/engine/src/pops.rs` (the data model) and `crates/engine/src/setting/pops.rs` (reading `[[pop_type]]` sections). The public types are re-exported from the engine crate root: `PopKind`, `PopId`, `PopType`, `PopTypes`, `Endowment`, `UtilityGood`.

A **POP** is a group of identical individuals. This document covers **POP types**, which describe one individual: the goods it receives at the start of every turn (its **endowment**) and the Stone-Geary utility function it consumes by, as a **subsistence** amount and a **share** for each good it wants. POP types are persistent rules, defined in `rules/` and checked by the loader like resource types (see [resources.md](resources.md)).

Nothing uses POP types yet, and the turn is unchanged. Step 04 places POPs of these types and emits each type's endowment (times the POP's size) into the POP's stockpile; Step 05 computes demand from the subsistence amounts and shares; Step 06 sells what a POP doesn't keep through the dummy market (see [market.md](market.md)).

## Concepts

- **Endowment.** The amount of each good one individual receives every turn. The intended design endows only unskilled labor. Any good may be endowed so that the POP system can be tested before production and the live market exist; the sample setting's comments say so. An endowment may be empty: such a type has no income and is always below subsistence unless every subsistence amount is zero (useful for testing the shrinking path in Step 07).
- **Utility.** One entry per good the individual consumes. `subsistence` is the amount needed before anything else; `share` is the good's weight in spending beyond subsistence. A good with `share` 0 and a positive subsistence is a **pure necessity**: bought at its subsistence amount and never more.
- **Shares are weights.** The engine normalizes a type's shares so they sum to 1, and stores only the normalized shares. Setting authors never rebalance a file, and `0.1 + 0.2 + 0.7` can't fail on floating point.
- **Leisure** is not a special case: it is an endowed good that also appears in the utility, so it is consumed rather than sold. An endowed good not in the utility is sold entirely (from Step 06); a utility good not endowed is simply bought.
- **No cross-checks.** The loader doesn't compare a good's endowment with its subsistence, or endowed goods with utility goods, and puts no restriction on which goods may be endowed or consumed (the currency included; its price is 1). Because a POP may buy more of an endowed good than it receives, a subsistence above the endowment is not necessarily unreachable. But for a good only the POP itself produces, such as labor, a subsistence at or above the endowment means the type can never reach subsistence.

## Data model

```rust
pub enum PopKind {}                       // IdKind, label "pop type"
pub type PopId = Id<PopKind>;
pub struct Endowment { pub good: GoodId, pub amount: Quantity }
pub struct UtilityGood { pub good: GoodId, pub subsistence: Quantity, pub share: f64 }
pub struct PopType { endowment: Vec<Endowment>, utility: Vec<UtilityGood> }
pub type PopTypes = NamedList<PopKind, PopType>;
```

| `PopType` method | Result |
|---|---|
| `endowment() -> &[Endowment]` | The endowment in good ID order, no good twice. May be empty. Each `amount` is finite and greater than 0 |
| `utility() -> &[UtilityGood]` | The utility goods in good ID order, no good twice. Never empty; the shares sum to 1 and at least one is positive. `subsistence` may be zero |
| `endowment_of(good) -> Quantity` | One individual's endowment of `good`, or `Quantity::ZERO` if it isn't endowed |
| `utility_of(good) -> Option<&UtilityGood>` | The utility parameters of `good`, or `None` if the type doesn't want it |

Both lookups are binary searches on the sorted lists. `share` is a plain `f64` (a proportion, not an amount of a good); it is always finite and in `[0, 1]`.

`PopTypes` is a `NamedList` (see [ids.md](ids.md)): `len`, `is_empty`, `id(name)`, `name(id)`, `get(id)` and `iter()` as `(PopId, &str, &PopType)` in load order, which is also ID order. An unknown name gives `IdError::UnknownName { kind: "pop type", .. }`. A setting with no POP types is valid and gives an empty list. `pops.rs` depends on `goods`, `id` and `quantity` and knows nothing about files.

### Construction and normalization

`PopType::new(endowment, utility)` is crate-private: only the loader builds POP types. It takes `(GoodId, Quantity)` endowment pairs and `(GoodId, Quantity, f64)` utility triples (good, subsistence, weight as written) in any order, sorts both by good ID, and normalizes the weights:

1. Find the largest weight.
2. Divide every weight by it, so each scaled weight is in `[0, 1]`, and sum the scaled weights. Scaling first keeps the sum finite even for weights near `f64::MAX`.
3. Each share is `(weight / largest) / total`.

`new` panics if a good appears twice in either list, if a weight is negative or not finite, or if no weight is positive. The loader guarantees all of these, so a panic is an internal bug.

## Setting format

A `[[pop_type]]` belongs in `rules/` (in `start/` it gets the usual `section 'pop_type' belongs in rules/, not start/`). The header is `[[pop_type]]`; `[[pop]]` is reserved for placed POPs (Step 04). Three keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among POP types. POP type names and good names are separate namespaces |
| `endowment` | A table of `good name = amount`. Each amount is a number (whole or decimal), finite and greater than 0. May be empty (`{}`) |
| `utility` | A table of `good name = sub-table`. Each sub-table has two optional keys, `subsistence` and `share`, each a finite number of at least 0 that defaults to 0. A good needs at least one of them positive, and at least one good needs a positive `share` |

Both TOML table forms are accepted for `endowment`, for `utility` and for each utility good:

```toml
[[pop_type]]
name = "farmer"
endowment = { labor = 10 }
utility = { grain = { subsistence = 2, share = 3 }, labor = { subsistence = 1, share = 1 } }

[[pop_type]]
name = "fisher"

[pop_type.endowment]
fish = 2
labor = 8

[pop_type.utility.fish]
subsistence = 1        # share 0: a pure necessity

[pop_type.utility.coin]
share = 0.5
```

TOML forbids repeated keys in one table, so a good can't appear twice in an endowment or a utility.

## How POP types are loaded

`setting/pops.rs` holds `PopTypesReader`. `WorldContent` has a `pop_types: PopTypesReader` field, and the `pop_type` section type (folder `Rules`, listed after `market`) calls `PopTypesReader::read` for each entry. POP types are a named kind, so the reader uses `Definitions<PopKind>` like `ResourcesReader`, and resolves good names in phase 2.

1. **Phase 1, `read`.** Every key is read before anything is defined, so no key is falsely reported as unknown and one entry with several problems reports all of them.
   - `name` with `required_name`.
   - `endowment` with `EntryReader::required_number_table` (see [setting-loader.md](setting-loader.md)). Each value that parsed as a number must pass `Quantity::new(v)` filtered to `> 0`, otherwise `key 'grain' in endowment of pop_type must be a finite number greater than 0` is reported at the value. Every key is kept, with `None` for a bad amount, so its good is still resolved.
   - `utility` with `EntryReader::required_table_of_tables`. A value that isn't a sub-table is reported there and kept as `None`. Each sub-table is read with `EntryReader::nested("utility", name, table)`, whose label is `utility.<good> of pop_type`. The private `amount` helper reads `subsistence` and `share`: it uses `has` to tell an absent key (default 0) from a present one, then `optional_number`, and checks the value is finite and at least 0 (`key 'share' in utility.grain of pop_type must be a finite number of at least 0`, at the value). The nested reader's `finish` reports unknown keys in the sub-table. A good whose subsistence and share are both 0 (or both absent) is reported at its key as `key 'coin' in utility of pop_type must have a positive subsistence or share`.
   - If `utility` was a table and no good has a positive share, `utility of pop_type needs at least one good with a positive share` is reported at the `[[pop_type]]` header. A good that couldn't be read (not a table, or a bad `subsistence` or `share`) counts as possibly positive, so an entry isn't given this error on top of the one already reported. A missing or non-table `utility` doesn't get it either.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)`). The name is defined even when other keys are bad, so later duplicates are still caught.
   - **Every entry is kept** for phase 2, as a `PopEntry` with an optional `PopId` (`None` if the name was missing, invalid or a duplicate). Unlike `ResourcesReader`, which stores data only when the name defines, this lets an entry with a bad name still have its unknown goods reported.
2. **Phase 2, `resolve(goods, sink)`.** For every entry, resolves each endowment key and each utility key through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key). The good may be defined in any `rules/` file. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `PopTypesReader::finish` skips entries without an ID, builds each `PopType::new` from the resolved goods and the valid values, and returns `PopTypes::from_parts`. It is only called after a successful load, when every good resolved and every value was valid; it panics with `internal error: a POP type's goods were not resolved in a successful load` otherwise. `load_setting` stores the result in `Rules::pop_types`.

IDs follow load order (file read order, then the order of entries in each file), so the same folder always gives the same result.

All `[[pop_type]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `name`, `endowment` or `utility` missing | header | `missing required key 'endowment' in pop_type` |
| a bad name | value | `invalid name 'Bad Name' (names must start with a lowercase letter ...)` |
| a duplicate name | value | `duplicate pop type name 'ok' (first defined at <location>)` |
| `endowment` or `utility` not a table | value | `key 'utility' in pop_type must be a table, found string` |
| an endowment amount that isn't a number | value | `key 'grain' in endowment of pop_type must be a number, found string` |
| an endowment amount that is zero, negative, `inf` or `nan` | value | `key 'grain' in endowment of pop_type must be a finite number greater than 0` |
| a utility entry that isn't a table | value | `key 'grain' in utility of pop_type must be a table, found integer` |
| a `subsistence` or `share` that isn't a number | value | `key 'subsistence' in utility.coin of pop_type must be a number, found string` |
| a `subsistence` or `share` that is negative or not finite | value | `key 'share' in utility.grain of pop_type must be a finite number of at least 0` |
| a utility good with nothing positive | key | `key 'coin' in utility of pop_type must have a positive subsistence or share` |
| no utility good with a positive share | header | `utility of pop_type needs at least one good with a positive share` |
| a good name that matches no good | key | `unknown good 'tin'` |
| an unknown key in a utility good | key | `unknown key 'weight' in utility.grain of pop_type` |
| any other key in the entry | key | `unknown key 'colour' in pop_type` |
| `[[pop_type]]` in `start/` | header | `section 'pop_type' belongs in rules/, not start/` |

## Using POP types

```rust
let setting = load_setting(path)?;
let types = &setting.rules.pop_types;
let peasant = types.get(types.id("peasant")?);
let labor = setting.rules.goods.id("unskilled_labor")?;
let per_turn = peasant.endowment_of(labor);      // Quantity, per individual
for u in peasant.utility() {                      // good ID order
    let (good, subsistence, share) = (u.good, u.subsistence, u.share);
}
```

## Not yet

Placing POPs and emitting endowments (Step 04), computing demand (Step 05), trading (Step 06), growth and its parameters (Step 07), and showing POP types in the dev UI (Step 08). Per-POP parameters that differ from the type, or drawing parameters from a set of possibilities, are not planned for this Stage.

## The sample setting and tests

- `settings/sample/rules/pops.toml` defines two types, both endowed with 10 `unskilled_labor` per individual per turn. `peasant` (ID 0): `unskilled_labor` subsistence 2, share 0.2; `grain` subsistence 2, share 0.8. `artisan` (ID 1): written with weights 1, 4 and 5, normalized to `unskilled_labor` subsistence 1, share 0.1; `copper` share 0.5 (no subsistence); `grain` subsistence 2, share 0.4. Its header comment explains the keys, that shares are weights, leisure, and that unskilled labor is the intended endowment.
- `pops.rs` unit tests: normalization and sorting, huge weights that don't overflow, a single weight giving share 1, a zero-weight good kept with share 0, an empty endowment, `endowment_of` and `utility_of`, and the three panics.
- `crates/engine/tests/pops.rs`: the sample's two types with their normalized shares (summing to 1); `pops_valid` with IDs in load order, both table forms, a multi-good endowment written out of ID order, an empty endowment, a pure necessity, the currency in a utility, and the same result on a second load; every `pops_errors` error at its place, including one entry with several problems and a duplicate across files; and `valid_minimal` with no POP types.
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a POP type whose endowment and utility name goods that don't exist, and only the syntax error is reported.
- Test settings: `pops_valid` (goods `coin`, `labor`, `grain`, `fish`; types `farmer`, `fisher`, `idle` across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, with the goods in a file read after them), `pops_one` (one good and one type, for the checker's `1 pop type`), and `pops_errors`.

## docs/README.md — modify
- **Location:** the document table, new row after the `market.md` row (added by Step 02)
- **Change:** add

| [pops.md](pops.md) | POP types: the endowment and the Stone-Geary utility parameters (`PopType`, `PopTypes`), how shares are normalized, and how `[[pop_type]]` sections are loaded and checked |

## docs/setting-loader.md — modify
- **Location:** intro, the sentence starting "The loader defines four real section types so far" (as replaced by Step 02: "The loader defines five real section types so far")
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same sentence

The loader defines six real section types so far: `[[good]]`, `[[resource]]` and `[[pop_type]]` in `rules/` (see [goods.md](goods.md), [resources.md](resources.md) and [pops.md](pops.md)) and `[[hex]]`, `[[deposit]]` and `[[market]]` in `start/` (see [map.md](map.md), [resources.md](resources.md) and [market.md](market.md)); later Steps add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure", new row after the `market.rs` row (added by Step 02)
- **Change:** add

| `pops.rs` | `PopTypesReader`: reads `[[pop_type]]` entries, resolves the goods in their endowment and utility in phase 2, and builds `PopTypes` (see [pops.md](pops.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph starting "`WorldContent::resolve` is where real references are checked"
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), then each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency, and finally the goods in each POP type's `endowment` and `utility` (`PopTypesReader::resolve`). The map must resolve before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`); otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`, `PopTypesReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph starting "`WorldContent` is the real content type"
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader`, a `MarketsReader` and a `PopTypesReader`, and returns six section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`, and `pop_type` in `rules/`. `load_setting` builds `Rules { goods, resources, pop_types }` and `StartState { map, currencies, deposits, prices }` from it. `MarketsReader::finish` runs first, because `MapReader::finish` consumes the map reader; `MapReader::finish` returns the map and the currencies together and comes before `DepositsReader::finish`, which needs the hex count. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the first bullet ("`[[good]]` (`Rules::goods`) is the worked example for a named kind in `rules/`; ...")
- **Change:** replace

   - `[[good]]` (`Rules::goods`) is the worked example for a named kind in `rules/`; `[[resource]]` (`Rules::resources`) follows it. `[[pop_type]]` (`Rules::pop_types`) is the example for an entry with nested tables of good references (its `endowment` and `utility`, read with `required_number_table`, `required_table_of_tables` and `nested`), and for a reader that keeps every entry for phase 2, even one whose name failed, so its references are still checked (see [pops.md](pops.md)).

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first bullet (the list of accessors), as replaced by Step 02, and the `required_number_table` bullet Step 02 added after it
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same two bullets

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `optional_number`, `required_number_table`, `required_table_of_tables`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>` (or, for the two table readers, an `Option` of a list), where `Located` carries the value and its `Location`. `required_number` and `optional_number` accept an integer or a float and return an `f64`; `inf` and `nan` are returned as written (the caller does the range check), an integer beyond `i64` is reported as out of range, and other types are reported as `must be a number, found <type>`. `optional_number` returns `None` both when the key is absent and when it has the wrong type (the latter reported); use `has` to tell them apart.
- `has(key)` is true if the table has `key`. It does not mark the key as read, so a key only checked with `has` is still reported as unknown by `finish`.
- `required_number_table(key)` reads a table whose values are numbers (a market's `prices`, a POP type's `endowment`) and returns `Option<Vec<(Located<String>, Option<Located<f64>>)>>`: every key with its location, in the table's order, and its value parsed exactly as `required_number` parses one (both share the private `number_value` helper, which takes the message prefix). A bad value is reported as `key '<k>' in <key> of <label> must be a number, found <type>` or `... is out of range` and returned as `None`, so the caller still sees the key. A missing key gives the usual missing-key error and a non-table value `key '<key>' in <label> must be a table, found <type>`; both return `None`. The nested table's keys are data, not fields: they are not name-checked and not tracked as read or unknown. Both `key = { ... }` and a `[section.key]` sub-table are accepted.
- `required_table_of_tables(key)` reads a table whose values are tables (a POP type's `utility`) and returns `Option<Vec<(Located<String>, Option<&DeTable>)>>`: every key with its location, in the table's order. A value that isn't a table is reported as `key '<k>' in <key> of <label> must be a table, found <type>` and returned as `None`, so the caller still sees the key. Missing and non-table `key` are handled as for `required_number_table`, and the outer table's keys are data in the same way. Both inline tables and `[section.key.name]` sub-tables are accepted.
- `nested(key, name, table)` returns an `EntryReader` for one sub-table from `required_table_of_tables`. It shares the file and the error sink, its label is `<key>.<name> of <label>` (for example `utility.grain of pop_type`, giving `key 'share' in utility.grain of pop_type must be a number, found string`), and its missing required keys are reported at `name`'s location. The caller calls `finish` on it to report the sub-table's unknown keys (`unknown key 'weight' in utility.grain of pop_type`). Because nested labels are built at run time, the label is a `String` and `label()` returns `&str`.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", both bullets
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same two bullets

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (six goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `rules/pops.toml` (two POP types, see [pops.md](pops.md)), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), plus `README.md` files. It must always load without errors.
- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `markets_valid`, `markets_errors`, `pops_valid`, `pops_one`, `pops_errors`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph starting "The success line is"
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>, <pop types>`, for example `6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types` or `1 good, 0 hexes, 0 resources, 0 deposits, 0 markets, 1 pop type`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. POP types are counted with `Rules::pop_types.len()`. Currencies are not counted: there is one per hex, not a new kind of thing. The `count(n, singular, plural)` helper in `text.rs` does the wording; later Steps append their own counts. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

## docs/setting-loader.md — modify
- **Location:** "Tests", the first bullet
- **Change:** replace

- Unit tests sit in each `setting` module. Framework tests that use `TestContent` are in `test_content.rs`. Tests that need the errors as text lines use the `#[cfg(test)]` helpers `ErrorSink::into_lines` and `LoadErrors::lines`. `reader.rs` tests cover each accessor, including `optional_number` (present, absent, wrong type), `has` not marking a key as read, `required_table_of_tables` in both TOML forms with non-table values, and `nested` readers reporting with the nested label and location.

## docs/resources.md — modify
- **Location:** the paragraph starting "References are checked in `WorldContent::resolve`" (after the `DepositsReader` list), as replaced by Step 02
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

References are checked in `WorldContent::resolve`, so they may point into any file: hex currencies and resource types against goods, then deposits against resource types and the map. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)). `resources_broken_file` also holds a hex with an unknown currency, a market that prices goods that don't exist, and a POP type whose endowment and utility name goods that don't exist, which are likewise not reported. `[[market]]` follows `[[deposit]]` as the pattern for an entry identified by coordinates (see [market.md](market.md)), and `[[pop_type]]` follows `[[resource]]` as a named `rules/` kind with good references (see [pops.md](pops.md)).

## docs/goods.md — modify
- **Location:** "How goods are loaded", last paragraph ("Goods contain no references, ..."), as replaced by Step 02
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Sections that name goods (a resource type's `produces`, a hex's `currency`, the keys of a market's `prices`, the keys of a POP type's `endowment` and `utility`, and more from Stage 02 onward) resolve them with `Definitions::resolve` against the goods registry, through `GoodsReader::defs()`. A good's `perishes_after` also decides when goods bought from a market, and currency received from one, perish (see [market.md](market.md)).

## docs/ids.md — modify
- **Location:** "Types", the `NamedList<K, T>` bullet, its last sentence ("`Goods` and `ResourceTypes` are aliases of it.")
- **Change:** replace

`Goods`, `ResourceTypes` and `PopTypes` are aliases of it.

## docs/ids.md — modify
- **Location:** "Adding a new ID kind", item 3, its last sentence ("`ResourceKind` / `ResourceId` is a second named kind.")
- **Change:** replace

`ResourceKind` / `ResourceId` and `PopKind` / `PopId` (label `"pop type"`, see [pops.md](pops.md)) are further named kinds.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, as replaced by Step 02
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same bullet

- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets and POP types counts (`18 markets, 2 pop types` for the sample, `1 market` for `resources_one`, `1 pop type` for `pops_one`, zero elsewhere). It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/currency.rs, ...`
- **Supersedes:** Step 02 (dummy-market), the same block

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/goods.rs        goods data model (see goods.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types: endowment and utility parameters (see pops.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, goods.rs, map.rs, market.rs, pops.rs, resources.rs, setting_loader.rs, world.rs
```

## docs/world.md — modify
- **Location:** "The turn", item 2 of the numbered list, as replaced by Step 02
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same item

2. **Actors act:** nothing yet. Later Stages add POPs, industries, and markets here. POP types are loaded as rules (see [pops.md](pops.md)) but nothing reads them during a step. Trades through the dummy market (see [market.md](market.md)) will happen in this part once POP trade arrives (Stage 02 Step 06); nothing calls the exchange during a step yet, so the turn report is unchanged.
