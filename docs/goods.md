# Goods

Source: `crates/engine/src/goods.rs` (the data model) and `crates/engine/src/setting/goods.rs` (reading `[[good]]` sections). The public types are re-exported from the engine crate root: `Good`, `GoodId`, `GoodKind`, `Goods`, `Perishes`.

A good is anything an actor can own: labor, resource output, intermediate goods, consumption goods. The engine has no built-in goods and no categories such as "labor" or "capital"; every good comes from the setting's `rules/`. The engine understands exactly two properties of a good, and both are optional in the data.

## Data model

| Type | Role |
|---|---|
| `GoodKind` | Marker kind (`IdKind`, label `"good"`) for `Id<GoodKind>` |
| `GoodId` | `Id<GoodKind>`. Valid only for the `Goods` list that produced it |
| `Perishes` | `Never` or `After(NonZeroU32)` |
| `Good` | The properties of one good with defaults applied: `perishes: Perishes`, `movable: bool` |
| `Goods` | `NamedList<GoodKind, Good>`: the list of all goods plus their name registry. Immutable once built |

`Goods` is a type alias of the generic `NamedList` (see [ids.md](ids.md)), so it offers `len`, `is_empty`, `id(name) -> Result<GoodId, IdError>`, `name(id) -> &str`, `get(id) -> &Good` and `iter() -> (GoodId, &str, &Good)` in load order, which is also ID order. `Goods::default()` is the empty list. It is built only by the setting loader through the crate-private `Goods::from_parts(names, goods)`, which requires the registry and the vector to have the same length. `goods.rs` knows nothing about files.

Callers see "never perishes" or "movable", never "key missing": the engine does not record whether a value was written out or defaulted.

## Properties

- **`perishes_after`** (integer, 1 or more, optional). A unit survives N turn-ends, counting the turn it appears in. `perishes_after = 1`: usable in the turn it appears, gone when that turn ends. `perishes_after = 3` appearing in turn 5: gone at the end of turn 7. Absent means `Perishes::Never` (a durable good); there is no sentinel value. `0`, negative numbers and values above `u32::MAX` are errors at the value; non-integers are wrong-type errors. The value takes effect in the turn clock: a good with `perishes_after = N` that enters a stockpile in turn T perishes at the end of turn T+N-1, and a good without it never perishes. See `PerishesAt::for_good` and [world.md](world.md).
- **`movable`** (boolean, optional, default `true`). No effect until movement and trade between places exist, after Stage 04.

Any other key in a `[[good]]` is an unknown-key error.

## How goods are loaded

`[[good]]` is a `SectionType` in the loader (see [setting-loader.md](setting-loader.md)): header `good`, folder `Rules`. A `[[good]]` in `start/` gets the standard "belongs in rules/" error.

`setting/goods.rs` holds `GoodsReader`, the loader-side accumulator. `WorldContent` has a `goods: GoodsReader` field, and the section's `read` function calls `GoodsReader::read(reader)` for each entry. `read`:

1. reads `name` with `required_name`;
2. reads `perishes_after` with `optional_integer`, converts it to `NonZeroU32` and reports `key 'perishes_after' in good must be at least 1` (value below 1) or `... is out of range` (above `u32::MAX`), at the value;
3. reads `movable` with `optional_bool`;
4. only if the name is valid, calls `Definitions<GoodKind>::define`, which reports duplicates (the message names the first definition's location, so the error names both places), and pushes the `Good`.

Every key is read even if another one is bad, so one good with several problems reports all of them in one load. A good whose name is missing or a duplicate is not added, so the vector and the registry stay the same length. `GoodsReader::finish` turns the registry and vector into `Goods`, which `load_setting` stores in `Rules::goods`.

IDs follow load order: file read order (`rules/` files in byte order of relative path), then the order of goods within each file. The same folder always gives the same IDs. A setting with no goods is valid and gives an empty `Goods`.

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Sections that name goods (resource types, and Stage 02 onward) resolve them with `Definitions::resolve` against the goods registry, through `GoodsReader::defs()`.

## Using goods

A loaded setting exposes goods as `setting.rules.goods`:

```rust
let setting = load_setting(path)?;
let goods = &setting.rules.goods;
let grain = goods.id("grain")?;
assert_eq!(goods.name(grain), "grain");
let props = goods.get(grain);   // props.perishes, props.movable
```

## Adding a property

1. Add the field to `Good` in `goods.rs`, with its default documented there.
2. Read it in `GoodsReader::read` with the matching `EntryReader` accessor (reading it also marks the key as known), and report range problems at the value.
3. Add it to the sample setting and the test settings, and update the tables here and in the manual.

## The sample setting and tests

`settings/sample/rules/goods.toml` defines five goods in this order: `unskilled_labor` (perishes after 1, not movable), `unimproved_copper` (1, not movable), `unimproved_grain` (1, not movable), `copper` (never, movable), `grain` (3, movable). They cover each combination of the properties and give the resource types something to produce.

Tests:
- The list behavior (accessors, order, foreign-ID panics) is tested once for all named lists in `id.rs` (see [ids.md](ids.md)).
- `crates/engine/tests/goods.rs` loads the sample and the test settings through the public API and checks names, IDs, properties and defaults.
- `crates/engine/tests/setting_loader.rs` asserts exact error strings for the goods error cases.
- Test settings: `goods_valid` (goods across several files, including nested folders), `goods_one`, and `goods_errors` (missing name, bad name, duplicates across files, bad property values, unknown key, one good with several problems). `unknown_section` uses made-up section names (`widgets`, `gadgets`) so it stays valid as real sections are added.
