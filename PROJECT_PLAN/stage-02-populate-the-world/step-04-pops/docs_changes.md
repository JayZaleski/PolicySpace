# Step 04: pops — Docs Changes

This Step renamed Step 03's POP type ID: `PopKind` / `PopId` are now `PopTypeKind` / `PopTypeId` (label still `"pop type"`), and the names `PopKind` / `PopId` (label `"pop"`) now mean a placed POP. The private `PopEntry` in `setting/pops.rs` (Step 03's pop-type entry) is now `PopTypeEntry`, and `PopEntry` is the `[[pop]]` entry. Step 03's `docs_changes.md` has not been applied yet and uses the old names throughout `docs/pops.md`; the proposals below replace every affected part of it. Wherever Step 03's draft is applied, use the names given here.

The sample's turn line and the checker's success line also changed (`; endowed ...` and `, N pops`), so every quoted line is replaced below.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** the opening, from the `Source:` line through the paragraph starting "Nothing uses POP types yet, and the turn is unchanged."
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same three paragraphs of its `docs/pops.md` draft

Source: `crates/engine/src/pops.rs` (the data model) and `crates/engine/src/setting/pops.rs` (reading `[[pop_type]]` and `[[pop]]` sections). The public types are re-exported from the engine crate root: `PopTypeKind`, `PopTypeId`, `PopType`, `PopTypes`, `Endowment`, `UtilityGood`, `PopKind`, `PopId`, `Pop`, `Pops`.

A **POP** is a group of N identical individuals of one **POP type**, living on one hex. This document covers both:

- **POP types** describe one individual: the goods it receives at the start of every turn (its **endowment**) and the Stone-Geary utility function it consumes by, as a **subsistence** amount and a **share** for each good it wants. POP types are persistent rules, defined in `rules/` and checked by the loader like resource types (see [resources.md](resources.md)).
- **POPs** are starting state, placed in `start/` like deposits: each names a type, a hex and a size. Each POP owns a stockpile in the running world, and at the start of every turn it receives its type's endowment times its size (see [Endowment in the turn](#endowment-in-the-turn) and [world.md](world.md)).

The names follow one rule: "POP" means the placed group, so `PopId` is a placed POP's ID and `PopTypeId` is a POP type's. Nothing consumes, trades or grows yet: Step 05 computes demand from the subsistence amounts and shares, Step 06 sells what a POP doesn't keep through the dummy market (see [market.md](market.md)), and Step 07 changes a POP's size.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Data model", the code block
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same code block

```rust
pub enum PopTypeKind {}                   // IdKind, label "pop type"
pub type PopTypeId = Id<PopTypeKind>;
pub struct Endowment { pub good: GoodId, pub amount: Quantity }
pub struct UtilityGood { pub good: GoodId, pub subsistence: Quantity, pub share: f64 }
pub struct PopType { endowment: Vec<Endowment>, utility: Vec<UtilityGood> }
pub type PopTypes = NamedList<PopTypeKind, PopType>;
```

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Data model", the paragraph starting "`PopTypes` is a `NamedList`"
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

`PopTypes` is a `NamedList` (see [ids.md](ids.md)): `len`, `is_empty`, `id(name)`, `name(id)`, `get(id)` and `iter()` as `(PopTypeId, &str, &PopType)` in load order, which is also ID order. An unknown name gives `IdError::UnknownName { kind: "pop type", .. }`. A setting with no POP types is valid and gives an empty list. `pops.rs` depends on `goods`, `id`, `map` and `quantity` and knows nothing about files.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Setting format", the first paragraph (starting "A `[[pop_type]]` belongs in `rules/`")
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

A `[[pop_type]]` belongs in `rules/` (in `start/` it gets the usual `section 'pop_type' belongs in rules/, not start/`). The header is `[[pop_type]]`; `[[pop]]` places POPs of these types in `start/` (see [Placed POPs](#placed-pops)). Three keys, all required:

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "How POP types are loaded", the first paragraph (starting "`setting/pops.rs` holds `PopTypesReader`") and, in the numbered list, the bullet starting "**Every entry is kept** for phase 2"
- **Change:** replace both
- **Supersedes:** Step 03 (pop-types), the same paragraph and bullet

`setting/pops.rs` holds `PopTypesReader` (and `PopsReader`, see [How POPs are loaded](#how-pops-are-loaded)). `WorldContent` has a `pop_types: PopTypesReader` field, and the `pop_type` section type (folder `Rules`, listed after `market`) calls `PopTypesReader::read` for each entry. POP types are a named kind, so the reader uses `Definitions<PopTypeKind>` like `ResourcesReader`, and resolves good names in phase 2. `PopTypesReader::defs()` exposes those definitions so `PopsReader` can resolve a POP's `type` against them.

   - **Every entry is kept** for phase 2, as a `PopTypeEntry` with an optional `PopTypeId` (`None` if the name was missing, invalid or a duplicate). Unlike `ResourcesReader`, which stores data only when the name defines, this lets an entry with a bad name still have its unknown goods reported.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** new sections after "Using POP types" and before "Not yet"
- **Change:** add

## Placed POPs

A POP is N identical individuals of one type on one hex. Its endowment and (from Step 05) its demand are per-individual amounts scaled by N. **Size is continuous**, a `Quantity` rather than an integer: Step 07's growth multiplies N by a rate, which with whole numbers would freeze small POPs or need a rounding rule. Fractional individuals are no stranger than a deposit emitting 2.5 of a good, and a UI may round N for display.

**At most one POP of each type per hex.** Two POPs of one type on one hex would see the same prices and get the same per-individual endowment, so they would behave as one POP split in two. A hex may hold POPs of several types, or none.

POPs have no names. They are identified in setting files by (type, hex), and a `PopId` is valid only within the `Pops` list that produced it. A POP's **stockpile** is not part of the `Pop` record: it is world state, held by the `World` (see [Endowment in the turn](#endowment-in-the-turn)).

### Data model

```rust
pub enum PopKind {}                       // IdKind, label "pop"
pub type PopId = Id<PopKind>;
pub struct Pop {
    pub hex: HexId,
    pub pop_type: PopTypeId,
    pub size: Quantity,                   // N; always finite and greater than 0
}
pub struct Pops {
    pops: IdVec<PopKind, Pop>,            // by PopId, in load order
    by_hex: IdVec<HexKind, Vec<PopId>>,   // by HexId: the POPs on each hex, in ID order
}
```

`Pops` mirrors `Deposits` (see [resources.md](resources.md)): a list in load order plus a per-hex index.

| `Pops` method | Result |
|---|---|
| `len()`, `is_empty()` | The number of POPs; a setting with none is valid |
| `get(id) -> &Pop` | The POP. Panics with `is not from this list` for a foreign ID |
| `iter()` | `(PopId, &Pop)` in load order, which is ID order |
| `on_hex(hex) -> &[PopId]` | The POPs on `hex`, in ID order; empty if none. Panics for a hex not from the map the list was built with |
| `of_type_on_hex(hex, pop_type) -> Option<PopId>` | The POP of that type on that hex, if any (a linear search of `on_hex`, whose lists are short). Panics like `on_hex` |

`Pops::from_pops(pops, hex_count)` is crate-private: only the loader builds POPs. It numbers the POPs 0, 1, 2... in the order given, fills `by_hex`, and panics if a hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), if two POPs share a hex and a type (`"two POPs of {pop_type:?} on {hex:?}"`; the loader rejects duplicates first, so this is an internal bug), or if there are more than `u32::MAX` POPs. `Pops::default()` is empty and was built with no map, so `on_hex` and `of_type_on_hex` panic on it for any hex.

### Setting format

A `[[pop]]` belongs in `start/` (in `rules/` it gets `section 'pop' belongs in start/, not rules/`). Four keys, all required:

| Key | Meaning |
|---|---|
| `type` | The name of a POP type. Resolved in phase 2, so the type may be defined in any `rules/` file |
| `q`, `r` | Integer axial coordinates of a hex in the map, read as a deposit's are |
| `size` | N, a number (whole or decimal), finite and greater than 0 |

```toml
[[pop]]
type = "peasant"
q = 0
r = 0
size = 60

[[pop]]
type = "artisan"     # a second type on the same hex is fine
q = 0
r = 0
size = 2.5           # sizes are continuous
```

### How POPs are loaded

`setting/pops.rs` also holds `PopsReader`, which follows `DepositsReader`. `WorldContent` has a `pops: PopsReader` field, and the `pop` section type (folder `Start`, listed after `pop_type`) calls `PopsReader::read` for each entry.

1. **Phase 1, `read`.** Records the header's location, then reads every key before any early return, so no key is falsely reported as unknown and one entry with several problems reports all of them: `type` with `required_string` (not name-checked: a bad value is simply an unknown POP type in phase 2), `q` and `r` with the shared `coordinate` helper from `setting/map.rs`, and `size` with `required_number`, which must pass `Quantity::new(v)` filtered to `> 0` (otherwise `key 'size' in pop must be a finite number greater than 0` at the value).
   - An entry whose `type`, `q` or `r` is missing or invalid is not stored, so it takes no part in the duplicate check and gets no reference errors (the load fails anyway).
   - Duplicates are keyed by (type name as written, coordinates) in a `seen: HashMap<(String, HexCoord), Location>` used for lookup only. A repeat is reported at the second header as `duplicate pop of 'worker' at (0, 0) (first defined at start/a.toml:1:1)` and not stored.
   - An entry with a missing or invalid `size` is stored with the placeholder `Quantity::ONE`, so its type and hex are still checked in phase 2 and it still counts for duplicates.
2. **Phase 2, `resolve(pop_types, map, sink)`.** For every stored entry, resolves `type` through `Definitions::<PopTypeKind>::resolve` (`unknown pop type 'ghost'` at the value) and the coordinates through `MapReader::id` (`no hex at (9, 9)` at the header). Both checks always run, so an entry with an unknown type still has its hex checked. An entry with both resolved becomes a `Pop`. `WorldContent::resolve` calls this last, after `PopTypesReader::resolve`. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `PopsReader::finish(hex_count)` calls `Pops::from_pops`. It is only called after a successful load; `load_setting` calls it after `MapReader::finish` (for the hex count) and stores the result in `StartState::pops`.

POP IDs follow load order (file read order, then the order of entries in each file), so the same folder always gives the same result.

All `[[pop]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `type`, `q`, `r` or `size` missing | header | `missing required key 'size' in pop` |
| `type` not a string | value | `key 'type' in pop must be a string, found integer` |
| `q` or `r` not an integer | value | `key 'q' in pop must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in pop is out of range` |
| `size` not a number | value | `key 'size' in pop must be a number, found string` |
| `size` zero, negative, `inf` or `nan` | value | `key 'size' in pop must be a finite number greater than 0` |
| same type name and coordinates as an earlier POP | header | `duplicate pop of 'worker' at (0, 0) (first defined at <location>)` |
| a `type` that matches no POP type | value | `unknown pop type 'ghost'` |
| coordinates that match no hex | header | `no hex at (9, 9)` |
| any other key | key | `unknown key 'colour' in pop` |
| `[[pop]]` in `rules/` | header | `section 'pop' belongs in start/, not rules/` |

### Endowment in the turn

The `World` takes the `Pops` over as state (`World::pops()`), because Step 07 will change sizes, and gives every POP its own `Stockpile` in `pop_stockpiles: IdVec<PopKind, Stockpile>`, read with `World::pop_stockpile(pop)`. Every POP stockpile is empty when the world is created: POPs have no starting cash or goods.

At the start of every turn, including turn 0, after deposits emit, the private `World::endow` walks the POPs in ID order. For each `Endowment { good, amount }` of the POP's type it adds `amount.saturating_mul(size)` to the POP's stockpile, in the group `PerishesAt::for_good(good.perishes, turn)`. So an endowed good with `perishes_after = 1` (the sample's `unskilled_labor`) is gone at the end of the turn it arrives, and a durable endowed good accumulates. A type with an empty endowment adds nothing. The totals per good become `TurnReport::endowed`. At the end of the turn, POP stockpiles perish after the hex stockpiles. See [world.md](world.md) for the turn as a whole.

### Using POPs

```rust
let (world, report) = World::new(load_setting(path)?);
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
let labor = world.rules().goods.id("unskilled_labor")?;
for &id in world.pops().on_hex(center) {                // ID order
    let pop = world.pops().get(id);
    let type_name = world.rules().pop_types.name(pop.pop_type);
    let labor_held = world.pop_stockpile(id).total(labor);
}
let artisan = world.rules().pop_types.id("artisan")?;
let artisans_here = world.pops().of_type_on_hex(center, artisan);   // Option<PopId>
```

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "Not yet", the whole paragraph
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

Computing demand (Step 05), trading and consuming (Step 06), growth and its parameters, and what happens to a POP that shrinks toward zero (Step 07), and showing POP types and POPs in the dev UI (Step 08). Starting stockpiles or cash for POPs, changing a POP's hex or type during play, migration, and per-POP parameters that differ from the type are not planned for this Stage.

## docs/pops.md — modify (Step 03's new-file draft)
- **Location:** "The sample setting and tests", the whole section body
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same section

- `settings/sample/rules/pops.toml` defines two types, both endowed with 10 `unskilled_labor` per individual per turn. `peasant` (ID 0): `unskilled_labor` subsistence 2, share 0.2; `grain` subsistence 2, share 0.8. `artisan` (ID 1): written with weights 1, 4 and 5, normalized to `unskilled_labor` subsistence 1, share 0.1; `copper` share 0.5 (no subsistence); `grain` subsistence 2, share 0.4. Its header comment explains the keys, that shares are weights, leisure, and that unskilled labor is the intended endowment, and points to `start/pops.toml`.
- `settings/sample/start/pops.toml` places 16 POPs, listed in the map's order (row by row, north to south, west to east): a `peasant` on 15 of the 19 hexes, with sizes from 12 to 60, and an `artisan` of size 10 on the center hex (0, 0), the only hex that prices every good. The center therefore holds two POPs (IDs 7 and 8). The north-east corner (2, -2), which has no market, holds a `peasant` (ID 2) whose labor can never be sold, Step 06's "not traded here" case. Hexes (2, -1), (-2, 0), (1, 1) and (-2, 2) have no POPs. The sizes total 410, so POPs are endowed `4100 unskilled_labor` every turn. The file's header comment explains the keys, continuous size, the one-POP-per-type-per-hex rule, and that POPs start with nothing.
- `pops.rs` unit tests: normalization and sorting, huge weights that don't overflow, a single weight giving share 1, a zero-weight good kept with share 0, an empty endowment, `endowment_of` and `utility_of`, and the three `PopType::new` panics; for `Pops`, sequential IDs, `on_hex` in ID order and empty for a hex with none, `of_type_on_hex`, the empty default, and the panics (foreign ID, hex beyond the map, the default list, `from_pops` with a hex beyond `hex_count` or two POPs of one type on one hex).
- `crates/engine/tests/pops.rs`: the sample's two types with their normalized shares (summing to 1); `pops_valid` with IDs in load order, both table forms, a multi-good endowment written out of ID order, an empty endowment, a pure necessity, the currency in a utility, and the same result on a second load; every `pops_errors` error at its place; `valid_minimal` with no POP types. For placed POPs: the sample's 16 POPs with their hexes, types and sizes and its center, corner and empty hexes; a fresh sample world in which every POP holds `10 × size` labor perishing at the end of turn 0 and the report's `endowed` is 4100 labor; three steps that perish and re-endow the same amounts; `pops_placed` loaded and stepped; every `pops_placed_errors` error at its place; worlds with no POPs endowing nothing; and `pop_stockpile` panicking on a POP from another world.
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a POP type whose endowment and utility name goods that don't exist, and a `[[pop]]` of an unknown type at coordinates with no hex; only the syntax error is reported.
- Test settings: `pops_valid` (goods `coin`, `labor`, `grain`, `fish`; types `farmer`, `fisher`, `idle` across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, with the goods in a file read after them); `pops_one` (one good, one type, one hex and one POP, for the checker's `1 pop type`, `1 hex` and `1 pop`); `pops_errors`; `pops_placed` (goods `labor` and `grain`, both perishing after 1 turn, and durable `gold`; types `worker` endowing 10 `labor`, `hoarder` endowing 4 `labor` and 1.5 `gold`, and `idle` with an empty endowment; hexes (0, 0), (1, 0), (2, 0) with a `field` deposit on (2, 0); POPs `worker` 3 and `hoarder` 2 on (0, 0) in `start/a.toml`, `worker` 0.5 and `idle` 7 on (1, 0) in `start/b/c.toml`); and `pops_placed_errors` (every `[[pop]]` error in the table above, one entry with four problems, a duplicate across files, and a `[[pop]]` in `rules/`).

## docs/README.md — modify
- **Location:** the document table, the `pops.md` row (added by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same row

| [pops.md](pops.md) | POP types (the endowment and Stone-Geary utility parameters, how shares are normalized) and placed POPs (`Pops`, continuous size, one POP per type per hex), how `[[pop_type]]` and `[[pop]]` sections are loaded and checked, and the per-turn endowment |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same row

| [world.md](world.md) | The running `World`, the three-part turn (emission, POP endowment, perishing), hex and POP stockpiles with perish groups and removal, turn reports, and the setting checker's `--turns` option |

## docs/ids.md — modify
- **Location:** "Adding a new ID kind", item 3, its last sentence ("`ResourceKind` / `ResourceId` is a second named kind.")
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same sentence (its version named `PopKind` / `PopId` as the POP type kind)

`PopKind` / `PopId` (label `"pop"`) is an unnamed kind identified by its type and hex: `Pops::from_pops` numbers POPs in load order and keeps a per-hex index, and the loader rejects a second POP of one type on one hex (see [pops.md](pops.md)). `ResourceKind` / `ResourceId` and `PopTypeKind` / `PopTypeId` (label `"pop type"`) are further named kinds.

## docs/setting-loader.md — modify
- **Location:** intro, the sentence starting "The loader defines four real section types so far" (as replaced by Step 03: "The loader defines six real section types so far")
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same sentence

The loader defines seven real section types so far: `[[good]]`, `[[resource]]` and `[[pop_type]]` in `rules/` (see [goods.md](goods.md), [resources.md](resources.md) and [pops.md](pops.md)) and `[[hex]]`, `[[deposit]]`, `[[market]]` and `[[pop]]` in `start/` (see [map.md](map.md), [resources.md](resources.md), [market.md](market.md) and [pops.md](pops.md)); later Steps add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure", the `map.rs` row of the table
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same row

| `map.rs` | `MapReader`: reads `[[hex]]` entries, including each hex's `currency`, and builds `HexMap` and `HexCurrencies`. Its `coordinate` helper is shared with deposits, markets and POPs. `MapReader::id` resolves coordinates to a `HexId`, `MapReader::len` gives the hex count, and `MapReader::currency` gives a hex's resolved currency after phase 2 |

## docs/setting-loader.md — modify
- **Location:** "Module structure", the `pops.rs` row (added by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same row

| `pops.rs` | `PopTypesReader`: reads `[[pop_type]]` entries, resolves the goods in their endowment and utility in phase 2, and builds `PopTypes`. `PopsReader`: reads `[[pop]]` entries, checks duplicates by type and coordinates, resolves each POP's type and hex in phase 2, and builds `Pops` (see [pops.md](pops.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph starting "`WorldContent::resolve` is where real references are checked"
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), then each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency, then the goods in each POP type's `endowment` and `utility` (`PopTypesReader::resolve`), and finally each POP's `type` against the POP types (`PopTypesReader::defs`) and its coordinates against the map (`PopsReader::resolve`). The map must resolve before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`); otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`, `PopTypesReader::finish`, `PopsReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph starting "`WorldContent` is the real content type"
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader`, a `MarketsReader`, a `PopTypesReader` and a `PopsReader`, and returns seven section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`, `pop_type` in `rules/`, and `pop` in `start/`. `load_setting` builds `Rules { goods, resources, pop_types }` and `StartState { map, currencies, deposits, prices, pops }` from it. `MarketsReader::finish` runs first, because `MapReader::finish` consumes the map reader; `MapReader::finish` returns the map and the currencies together and comes before `DepositsReader::finish` and `PopsReader::finish`, which need the hex count. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the `[[hex]]` bullet (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same bullet

   - `[[hex]]` (`StartState::map`) is the worked example for a starting-state section type and for a kind identified by something other than a name. Its reader keeps its own table of what makes an entry unique (coordinates) and checks duplicates itself, instead of using `Definitions`. `[[deposit]]` (`StartState::deposits`) and `[[market]]` (`StartState::prices`) follow it. A `[[hex]]` also carries a reference, its `currency`, which is stored separately in `StartState::currencies` (see [currency.md](currency.md)). `[[market]]` is the example for per-hex state stored one entry per hex by `HexId`, with `None` for hexes that have no entry (see [market.md](market.md)). `[[pop]]` (`StartState::pops`) follows `[[deposit]]` as a placed thing that refers to a named kind and a hex, and is the example for an entry made unique by a combination of a name as written and coordinates (see [pops.md](pops.md)).

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", both bullets
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same two bullets

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (six goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `rules/pops.toml` (two POP types, see [pops.md](pops.md)), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), `start/pops.toml` (16 POPs, see [pops.md](pops.md)), plus `README.md` files. It must always load without errors.
- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `markets_valid`, `markets_errors`, `pops_valid`, `pops_one`, `pops_errors`, `pops_placed`, `pops_placed_errors`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph starting "The success line is"
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>, <pop types>, <pops>`, for example `6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops` or `1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 1 pop type, 1 pop`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. POP types are counted with `Rules::pop_types.len()` and POPs with `StartState::pops.len()`. Currencies are not counted: there is one per hex, not a new kind of thing. The `count(n, singular, plural)` helper in `text.rs` does the wording; later Steps append their own counts. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

## docs/map.md — modify
- **Location:** "How hexes are loaded", the paragraph starting "Duplicates are detected by the reader's own coordinate table" (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

Duplicates are detected by the reader's own coordinate table, not by `Definitions`, which is keyed by names. The only reference in a hex is its currency: `MapReader::resolve` (phase 2) resolves each placed hex's currency against the goods. Other readers query the map reader while resolving their own references: `MapReader::id(coord)` gives the `HexId` a coordinate will have in the finished map (deposits, markets and POPs use it), `MapReader::len()` gives the number of placed hexes (markets size their per-hex lists with it), and `MapReader::currency(hex)` gives a placed hex's resolved currency, or `None` before phase 2 or if it didn't resolve (markets use it to reject a priced currency, so `MapReader::resolve` must run first). The `coordinate` helper that reads `q` and `r` is shared by hexes, deposits, markets and POPs; its message names the entry's label.

## docs/resources.md — modify
- **Location:** the paragraph starting "References are checked in `WorldContent::resolve`" (after the `DepositsReader` list)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

References are checked in `WorldContent::resolve`, so they may point into any file: hex currencies and resource types against goods, then deposits against resource types and the map. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)). `resources_broken_file` also holds a hex with an unknown currency, a market that prices goods that don't exist, a POP type whose endowment and utility name goods that don't exist, and a POP of an unknown type at coordinates with no hex, which are likewise not reported. `[[market]]` follows `[[deposit]]` as the pattern for an entry identified by coordinates (see [market.md](market.md)); `[[pop_type]]` follows `[[resource]]` as a named `rules/` kind with good references, and `[[pop]]` follows `[[deposit]]` as a placed thing with a type reference, coordinates and a per-hex index (see [pops.md](pops.md)).

## docs/goods.md — modify
- **Location:** "How goods are loaded", last paragraph ("Goods contain no references, ..."), as replaced by Step 03
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same paragraph

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Sections that name goods (a resource type's `produces`, a hex's `currency`, the keys of a market's `prices`, the keys of a POP type's `endowment` and `utility`, and more from Stage 02 onward) resolve them with `Definitions::resolve` against the goods registry, through `GoodsReader::defs()`. A good's `perishes_after` also decides when a POP's endowment of it perishes (see [pops.md](pops.md)), and when goods bought from a market, and currency received from one, perish (see [market.md](market.md)).

## docs/world.md — modify
- **Location:** intro paragraph, the second sentence ("It adds the clock and the goods that exist, and it can be stepped turn by turn.")
- **Change:** replace

It adds the clock, the POPs' holdings and the goods that exist, and it can be stepped turn by turn.

## docs/world.md — modify
- **Location:** "What a World holds", the field table: the `manifest`, `rules` row
- **Change:** replace

| `manifest`, `rules` | the setting | never (goods, resource types and POP types are fixed for the whole game) |

## docs/world.md — modify
- **Location:** "What a World holds", the field table, new rows after the `hex_stockpiles` row (as replaced by Step 02)
- **Change:** add

| `pops: Pops` | the setting's `StartState::pops` | not yet; world state because Step 07 changes a POP's size (see [pops.md](pops.md)) |
| `pop_stockpiles: IdVec<PopKind, Stockpile>` | one empty stockpile per POP at creation | goods enter by endowment and leave by perishing; Step 06's trade will take from and add to them |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, from "Public accessors are read-only" to the end of the paragraph (as replaced by Step 02), and the sentence before it ("There is one stockpile of **unowned goods** per hex ...")
- **Change:** replace both
- **Supersedes:** Step 02 (dummy-market), the same paragraph

The world owns the setting, so callers (the dev UI) have no lifetimes to manage. There is one stockpile of **unowned goods** per hex, in an `IdVec` indexed by `HexId`, and one stockpile per POP, the goods that POP owns, in an `IdVec` indexed by `PopId` (see [ids.md](ids.md)). Both use the same `Stockpile` type. Deposits have no owners in this Stage, so their output still goes to the hex stockpiles. Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)`, `hex_stockpile(hex)`, `pops()` and `pop_stockpile(pop)`. `market(hex)` returns the hex's `Market` view (its prices in the hex's currency and the exchange, at the current turn; see [market.md](market.md)); because it borrows the world, the world can't be stepped while a `Market` is alive. `hex_currency`, `market` and `hex_stockpile` panic with `is not from this list` for a hex from another map, and `pop_stockpile` for a POP from another world's `Pops` (a caller bug). The turn neither reads nor changes currencies or prices.

## docs/world.md — modify
- **Location:** "The turn", from the numbered list through the paragraph starting "So between calls the world is always at part 2"
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), item 2 of the numbered list (which itself superseded Step 02's)

A turn has three parts:

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good into its hex's stockpile. Then every POP receives its type's endowment times its size into its own stockpile (see [Endowment](#endowment)).
2. **Actors act:** nothing yet. Later Steps and Stages add POP demand and trade (Stage 02 Steps 05 and 06), industries, and the live market here. Trades through the dummy market (see [market.md](market.md)) will happen in this part once POP trade arrives; nothing calls the exchange during a step yet.
3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile and then from every POP stockpile.

Two calls drive the clock:

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 and runs turn 0's start, so a fresh world's deposits have emitted and its POPs already hold their first endowment. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing. Then it runs the end of the current turn (perishing: hexes in ID order, then POPs in ID order), advances the clock, and runs the start of the new turn (emission, then endowment).

So between calls the world is always at part 2: this turn's output and endowment are in the stockpiles and nothing has perished yet this turn. This is the state that tests, the checker, and the dev UI observe. If observation happened after the end of the turn, goods with `perishes_after = 1` would always look empty.

## docs/world.md — modify
- **Location:** "The turn", new subsection after "### Emission" and before "### Perishing"
- **Change:** add

### Endowment

`World::endow` (private) runs after `emit` and walks `pops.iter()` in POP ID order. For each POP it looks up the POP type in `rules.pop_types`, and for each `Endowment { good, amount }` adds `amount.saturating_mul(pop.size)` to the POP's stockpile in the group `PerishesAt::for_good(good.perishes, self.turn)`, recording `(good, total)`. `totals_by_good` turns the records into `TurnReport::endowed`. A type with an empty endowment adds nothing, and a world with no POPs behaves exactly as before POPs existed.

Deposits emit before POPs are endowed so the order of floating-point additions is fixed and `emitted` is unchanged by POPs. `endow` borrows `self.rules` and `self.pops` while mutating `self.pop_stockpiles`; they are separate fields, so Step 06 can likewise borrow a POP's stockpile mutably while a `Market` borrows the rules and prices. See [pops.md](pops.md) for POPs themselves.

## docs/world.md — modify
- **Location:** "The turn", "### Perishing", the whole subsection body
- **Change:** replace

`World::perish` (private) calls `Stockpile::remove_perishing(self.turn)` on every hex stockpile in hex ID order and then on every POP stockpile in POP ID order, and passes all the removed amounts through `totals_by_good`. The totals, hexes and POPs together, become `TurnReport::perished`.

## docs/world.md — modify
- **Location:** "The turn", "### Determinism", the first sentence ("No `HashMap` or `HashSet` is used. Deposits emit in ID order, ...")
- **Change:** replace

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, stockpiles perish hexes first and then POPs, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds and reports.

## docs/world.md — modify
- **Location:** "TurnReport", the whole section body
- **Change:** replace

```rust
pub struct TurnReport {
    pub turn: Turn,                         // the turn that started
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn, hexes and POPs
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted by deposits at the start of `turn`
    pub endowed: Vec<(GoodId, Quantity)>,   // received by POPs as endowment at the start of `turn`
}
```

Each list has one total per good, summed over every hex (and, for `perished` and `endowed`, every POP), in good ID order, and lists only amounts greater than zero. `emitted` and `endowed` are kept apart so deposit output and POP income can be told apart. There is no per-hex or per-POP breakdown; what each holds is in its stockpile. For the sample setting turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`; endowed 4100 `unskilled_labor`", and every later step perishes the 4100 labor along with the deposit output and emits and endows the same amounts again, because all three goods perish after one turn. Step 06 will extend the report with its own POP trade totals in the same style.

## docs/world.md — modify
- **Location:** "Stockpile", the first two sentences ("`Stockpile` is a general type, not specific to hexes. Hexes use it now; Stage 02 reuses it for what POPs own and Stage 03 for industries.")
- **Change:** replace

`Stockpile` is a general type, not specific to hexes. Hexes use it for unowned goods and POPs for what they own (see [pops.md](pops.md)); Stage 03 reuses it for industries.

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`", the example block and the sentence after it ("Entries are `<amount> <good name>` joined by ...")
- **Change:** replace

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor
Turn 1: perished 4100 unskilled_labor, 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor
```

The line lists `perished`, `emitted` and `endowed` in that order. Entries are `<amount> <good name>` joined by `, ` in report order (good ID order; `unskilled_labor` is the sample's first good, so it leads the `perished` list); an empty list prints `nothing`, so a setting without POPs ends every line with `; endowed nothing`.

## docs/world.md — modify
- **Location:** "Tests", the unit-tests bullet (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same bullet

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub` and `saturating_mul`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, zero, and `remove_all`), and `world.rs` (a world forced to `Turn::MAX`, for both `turns_valid` and `pops_placed`, refuses to step and leaves its turn, hex stockpiles and POP stockpiles unchanged). Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)).

## docs/world.md — modify
- **Location:** "Tests", the `crates/engine/tests/world.rs` bullet (as replaced by Step 01)
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same bullet

- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports (including the POPs' labor in `perished` and `endowed`), a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits (which also endow nothing), determinism (reports, hex stockpiles and POP stockpiles of two runs), the accessors, hex currencies being readable and unchanged by stepping, and the foreign-hex panics of `hex_stockpile` and `hex_currency`. POP endowment in the world (the sample, `pops_placed` with a durable and a fractional endowment, worlds with no POPs, the foreign-POP panic) is tested in `crates/engine/tests/pops.rs` (see [pops.md](pops.md)).

## docs/world.md — modify
- **Location:** "Not yet", the whole paragraph (as replaced by Step 02)
- **Change:** replace
- **Supersedes:** Step 02 (dummy-market), the same paragraph

Ownership of deposits, starting stockpiles in `start/` (for hexes or POPs), deposit stock, depletion and regeneration, any effect of `movable`, per-hex and per-POP report breakdowns, saving and loading a world, prices that change during play, changes to POPs during play (size from Step 07), and any behavior in the "actors act" part of the turn (demand and trading arrive in Steps 05 and 06).

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `turn_line` bullet
- **Change:** replace

- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn (see [world.md](world.md)).

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `good_rows` bullet
- **Change:** replace

- `good_rows(world)`: one `GoodRow { name, lasts, movable, held }` per good (`movable` is "movable" or "fixed"). `held` sums the good over every hex stockpile only; goods held by POPs are not included (POP stockpiles are shown from Step 08). It is computed in one pass over the hexes, adding each stockpile's totals into a per-good array.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet (as replaced by Step 03)
- **Change:** replace
- **Supersedes:** Step 03 (pop-types), the same bullet

- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets, POP types and POPs counts (`18 markets, 2 pop types, 16 pops` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `3 pop types, 4 pops` for `pops_placed`, zero elsewhere), and every expected turn line ends with its `endowed` list (`; endowed nothing` for settings without POPs). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines. It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/currency.rs, ...`
- **Supersedes:** Step 03 (pop-types), the same block

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/goods.rs        goods data model (see goods.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types and placed POPs (see pops.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, goods.rs, map.rs, market.rs, pops.rs, resources.rs, setting_loader.rs, world.rs
```

## crates/engine/src/setting/map.rs — code doc comment (not a docs/ change)
- **Location:** the doc comment on `pub(crate) fn coordinate`
- **Change:** replace (flagged for the Stage's consolidation, as Step 02 did; `update-docs` edits only `docs/`)
- **Supersedes:** Step 02 (dummy-market), the same flag

The comment still reads "Shared by hexes and deposits; the message names the entry's label." `MarketsReader::read` and `PopsReader::read` also use it. Suggested text:

```rust
/// Reads one coordinate key as an `i32`. A value outside `i32` is reported at the value.
/// Shared by hexes, deposits, markets and POPs; the message names the entry's label.
```
