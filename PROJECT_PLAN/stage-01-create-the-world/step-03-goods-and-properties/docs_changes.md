# Step 03: goods-and-properties — Docs Changes

## docs/goods.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

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
| `Goods` | The list of all goods plus their name registry. Immutable once built |

`Goods` offers `len`, `is_empty`, `id(name) -> Result<GoodId, IdError>`, `name(id) -> &str`, `get(id) -> &Good` and `iter() -> (GoodId, &str, &Good)` in load order, which is also ID order. `Goods::default()` is the empty list. It is built only by the setting loader through the crate-private `Goods::from_parts(names, goods)`, which requires the registry and the vector to have the same length. `goods.rs` knows nothing about files.

Callers see "never perishes" or "movable", never "key missing": the engine does not record whether a value was written out or defaulted.

## Properties

- **`perishes_after`** (integer, 1 or more, optional). A unit survives N turn-ends, counting the turn it appears in. `perishes_after = 1`: usable in the turn it appears, gone when that turn ends. `perishes_after = 3` appearing in turn 5: gone at the end of turn 7. Absent means `Perishes::Never` (a durable good); there is no sentinel value. `0`, negative numbers and values above `u32::MAX` are errors at the value; non-integers are wrong-type errors. This Step fixes only the meaning. Nothing makes goods perish yet: the turn clock (Step 06) does that and decides how unit age is tracked.
- **`movable`** (boolean, optional, default `true`). No effect until movement and trade between places exist, after Stage 04.

Any other key in a `[[good]]` is an unknown-key error.

## How goods are loaded

`[[good]]` is the first real `SectionType` in the loader (see [setting-loader.md](setting-loader.md)): header `good`, folder `Rules`. A `[[good]]` in `start/` gets the standard "belongs in rules/" error.

`setting/goods.rs` holds `GoodsReader`, the loader-side accumulator. `WorldContent` has a `goods: GoodsReader` field, and the section's `read` function calls `GoodsReader::read(reader)` for each entry. `read`:

1. reads `name` with `required_name`;
2. reads `perishes_after` with `optional_integer`, converts it to `NonZeroU32` and reports `key 'perishes_after' in good must be at least 1` (value below 1) or `... is out of range` (above `u32::MAX`), at the value;
3. reads `movable` with `optional_bool`;
4. only if the name is valid, calls `Definitions<GoodKind>::define`, which reports duplicates (the message names the first definition's location, so the error names both places), and pushes the `Good`.

Every key is read even if another one is bad, so one good with several problems reports all of them in one load. A good whose name is missing or a duplicate is not added, so the vector and the registry stay the same length. `GoodsReader::finish` turns the registry and vector into `Goods`, which `load_setting` stores in `Rules::goods`.

IDs follow load order: file read order (`rules/` files in byte order of relative path), then the order of goods within each file. The same folder always gives the same IDs. A setting with no goods is valid and gives an empty `Goods`.

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Later sections that name goods (resource types in Step 05, and Stage 02 onward) should resolve them with `Definitions::resolve` against the goods registry; `GoodsReader::defs()` exists for that and is currently marked `#[allow(dead_code)]`.

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

`settings/sample/rules/goods.toml` defines five goods in this order: `unskilled_labor` (perishes after 1, not movable), `unimproved_copper` (1, not movable), `unimproved_grain` (1, not movable), `copper` (never, movable), `grain` (3, movable). They cover each combination of the properties and give Step 05 resource outputs to point at.

Tests:
- `goods.rs` unit tests cover the `Goods` accessors.
- `crates/engine/tests/goods.rs` loads the sample and the test settings through the public API and checks names, IDs, properties and defaults.
- `crates/engine/tests/setting_loader.rs` asserts exact error strings for the goods error cases.
- Test settings: `goods_valid` (goods across several files, including nested folders), `goods_one`, and `goods_errors` (missing name, bad name, duplicates across files, bad property values, unknown key, one good with several problems). `unknown_section` now uses made-up section names (`widgets`, `gadgets`) because `good` is now a real section.

## docs/setting-loader.md — modify (phase 2 skipping)
- **Location:** "How loading works", step 5 (Phase 2) and the "Known gap" paragraph that follows the list
- **Change:** replace
- **Supersedes:** step-02-setting-loader/docs_changes.md (step 5 text and the "Known gap (planned for Step 03)" paragraph, which is removed)

Replace step 5 with:

5. **Phase 2.** `SettingContent::resolve` checks references now that every file has been read, so a reference can point into any file. It is skipped when the loader may not have seen every definition:
   - a file was rejected (could not be read, not UTF-8, or invalid TOML); or
   - discovery was incomplete (`Discovered::complete` is false): `rules/` or `start/` is missing or is not a folder, a subfolder could not be listed, or an entry's name is not valid UTF-8.

   In those cases an "unknown" error would be misleading, because the target may be in a file the loader never saw. Other phase 1 errors (unknown keys, bad names, duplicates, a non-`.toml` file the loader could see) do not skip phase 2.

Delete the "Known gap" paragraph. In its place, add: `discover` computes `complete` by threading a boolean through `walk` and `list_dir`, which return false whenever a folder cannot be listed or an entry is left out; a missing data folder also clears it. `load_with` runs phase 2 only when no file was rejected and `discovered.complete` is true. The framework tests cover this with `TestContent`, whose "placed thing" refers to a "thing".

## docs/setting-loader.md — modify (real section types)
- **Location:** intro paragraph; "Section types and content" (the `WorldContent` paragraph); "Adding a section type"; module table; "The sample setting and test settings"; "Setting checker (dev UI)"
- **Change:** replace
- **Supersedes:** step-02-setting-loader/docs_changes.md (statements that no section types exist, that the sample setting holds only README files, and the checker's success output)

- Intro: replace "This Step builds the framework only: it defines no section types (goods, hexes and so on), which later Steps add." with: The loader defines one real section type so far, `[[good]]` (see [goods.md](goods.md)); later Steps add more.
- `WorldContent` paragraph: `WorldContent` is the real content type. It holds a `GoodsReader` and returns one section type, `good` in `rules/`. `load_setting` builds `Rules { goods }` from it. `StartState` still has no fields. Because the loader is generic over the content type, tests supply their own (`TestContent`) as before.
- Module table: add row `goods.rs` | `GoodsReader`: reads `[[good]]` entries and builds `Goods`.
- "Adding a section type", step 1: `Rules` now has a `goods` field, so follow `GoodsReader` as the pattern: one reader struct per section kind, stored on `WorldContent`, with a `finish` that produces the public type for `Rules` or `StartState`.
- Sample setting: `settings/sample/rules/goods.toml` now defines five goods (see [goods.md](goods.md)); `start/` still holds only a README. Add `goods_valid`, `goods_one` and `goods_errors` to the list of test setting folders.
- Setting checker table, one-path success row: prints `Loaded setting '<name>' (format version <n>): <count>` where the count is `<n> good` or `<n> goods` (`1 good`, `0 goods`, `5 goods`). The dev UI's `count(n, singular, plural)` helper does the wording; later Steps append their own counts (hexes, deposits) to this line. Failure output, exit codes and the no-argument form are unchanged.

## docs/ids.md — modify
- **Location:** "Adding a new ID kind"
- **Change:** add
- **Supersedes:** none

Add: `GoodKind` / `GoodId` (in `goods.rs`) is the first concrete ID kind and the pattern to follow: a marker enum implementing `IdKind`, a type alias for `Id<Kind>`, and a list type that owns the `NameRegistry` and the per-ID data. See [goods.md](goods.md).

## docs/workspace.md — modify
- **Location:** Layout block
- **Change:** add
- **Supersedes:** none

Under `crates/engine/` add `src/goods.rs` (goods data model, see goods.md) and `tests/goods.rs`; under `src/setting/` add `goods.rs`.

## docs/README.md — modify
- **Location:** the table of documents, after the `setting-loader.md` row
- **Change:** add
- **Supersedes:** none

| [goods.md](goods.md) | What a good is, its two properties, how `[[good]]` sections are loaded, and how to add a property |
