# POPs

Source: `crates/engine/src/pops.rs` (the data model) and `crates/engine/src/setting/pops.rs` (reading `[[pop_type]]` and `[[pop]]` sections). The public types are re-exported from the engine crate root: `PopTypeKind`, `PopTypeId`, `PopType`, `PopTypes`, `Endowment`, `UtilityGood`, `GrowthParams`, `PopKind`, `PopId`, `Pop`, `Pops`.

A **POP** is a group of N identical individuals of one **POP type**, living on one hex. This document covers both:

- **POP types** describe one individual: the goods it receives at the start of every turn (its **endowment**), the Stone-Geary utility function it consumes by, as a **subsistence** amount and a **share** for each good it wants, and the **growth** parameters that say how a POP's size responds to its prosperity. POP types are persistent rules, defined in `rules/` and checked by the loader like resource types (see [resources.md](resources.md)).
- **POPs** are starting state, placed in `start/` like deposits: each names a type, a hex and a size. Each POP owns a stockpile in the running world, and at the start of every turn it receives its type's endowment times its size (see [Endowment in the turn](#endowment-in-the-turn) and [world.md](world.md)).

The names follow one rule: "POP" means the placed group, so `PopId` is a placed POP's ID and `PopTypeId` is a POP type's. Every turn, after the endowment, each POP trades and consumes in the trade round (see [trade.md](trade.md)): the hex's market computes its demand (see [demand.md](demand.md)) from its type's subsistence amounts and shares, sells what it doesn't keep and buys what it wants through the dummy market (see [market.md](market.md)), and the POP consumes its utility goods. Then each POP's size grows or shrinks by a rate from its type's growth parameters and how far above or below subsistence it was (see [growth.md](growth.md)).

## Concepts

- **Endowment.** The amount of each good one individual receives every turn. The intended design endows only unskilled labor. Any good may be endowed so that the POP system can be tested before production and the live market exist; the sample setting's comments say so. An endowment may be empty: such a type has no income and is always below subsistence unless every subsistence amount is zero (useful for testing the shrinking path).
- **Utility.** One entry per good the individual consumes. `subsistence` is the amount needed before anything else; `share` is the good's weight in spending beyond subsistence. A good with `share` 0 and a positive subsistence is a **pure necessity**: bought at its subsistence amount and never more.
- **Shares are weights.** The engine normalizes a type's shares so they sum to 1, and stores only the normalized shares. Setting authors never rebalance a file, and `0.1 + 0.2 + 0.7` can't fail on floating point.
- **Leisure** is not a special case: it is an endowed good that also appears in the utility, so it is consumed rather than sold. An endowed good not in the utility is sold entirely; a utility good not endowed is simply bought.
- **Growth.** A type also says how its POPs' size responds to prosperity: a `growth` sub-table with polynomial coefficients for the rate above subsistence, a `decline` for the rate below it, and a `max_growth` cap. The rate depends only on the POP's subsistence index, so every POP of a type at the same prices grows at the same rate whatever its size (see [growth.md](growth.md)).
- **No cross-checks.** The loader doesn't compare a good's endowment with its subsistence, or endowed goods with utility goods, and puts no restriction on which goods may be endowed or consumed (the currency included; its price is 1). Because a POP may buy more of an endowed good than it receives, a subsistence above the endowment is not necessarily unreachable. But for a good only the POP itself produces, such as labor, a subsistence at or above the endowment means the type can never reach subsistence.

## Data model

```rust
pub enum PopTypeKind {}                   // IdKind, label "pop type"
pub type PopTypeId = Id<PopTypeKind>;
pub struct Endowment { pub good: GoodId, pub amount: Quantity }
pub struct UtilityGood { pub good: GoodId, pub subsistence: Quantity, pub share: f64 }
pub struct GrowthParams { coefficients: Vec<f64>, decline: f64, max_growth: f64 }
pub struct PopType { endowment: Vec<Endowment>, utility: Vec<UtilityGood>, growth: GrowthParams }
pub type PopTypes = NamedList<PopTypeKind, PopType>;
```

| `PopType` method | Result |
|---|---|
| `endowment() -> &[Endowment]` | The endowment in good ID order, no good twice. May be empty. Each `amount` is finite and greater than 0 |
| `utility() -> &[UtilityGood]` | The utility goods in good ID order, no good twice. Never empty; the shares sum to 1 and at least one is positive. `subsistence` may be zero |
| `growth() -> &GrowthParams` | The growth parameters: `coefficients()` (never empty, every one finite), `decline()` (in [0, 1)) and `max_growth()` (at least 0). See [growth.md](growth.md) |
| `endowment_of(good) -> Quantity` | One individual's endowment of `good`, or `Quantity::ZERO` if it isn't endowed |
| `utility_of(good) -> Option<&UtilityGood>` | The utility parameters of `good`, or `None` if the type doesn't want it |

Both lookups are binary searches on the sorted lists. `share` is a plain `f64` (a proportion, not an amount of a good); it is always finite and in `[0, 1]`.

`PopTypes` is a `NamedList` (see [ids.md](ids.md)): `len`, `is_empty`, `id(name)`, `name(id)`, `get(id)` and `iter()` as `(PopTypeId, &str, &PopType)` in load order, which is also ID order. An unknown name gives `IdError::UnknownName { kind: "pop type", .. }`. A setting with no POP types is valid and gives an empty list. `pops.rs` depends on `goods`, `id`, `map` and `quantity` and knows nothing about files.

### Construction and normalization

`PopType::new(endowment, utility, growth)` is crate-private: only the loader builds POP types. It takes `(GoodId, Quantity)` endowment pairs and `(GoodId, Quantity, f64)` utility triples (good, subsistence, weight as written) in any order, plus a `GrowthParams`, sorts both lists by good ID, and normalizes the weights:

1. Find the largest weight.
2. Divide every weight by it, so each scaled weight is in `[0, 1]`, and sum the scaled weights. Scaling first keeps the sum finite even for weights near `f64::MAX`.
3. Each share is `(weight / largest) / total`.

`new` panics if a good appears twice in either list, if a weight is negative or not finite, or if no weight is positive. `GrowthParams::new` (also crate-private) panics if the coefficients are empty or not finite, if `decline` is not in [0, 1), or if `max_growth` is negative or not finite. The loader guarantees all of these, so a panic is an internal bug. Unit tests that need a type whose size never changes build it with `test_support::no_growth()`, which is `GrowthParams::new(vec![0.0], 0.0, 0.0)` (see [workspace.md](workspace.md)).

## Setting format

A `[[pop_type]]` belongs in `rules/` (in `start/` it gets the usual `section 'pop_type' belongs in rules/, not start/`). The header is `[[pop_type]]`; `[[pop]]` places POPs of these types in `start/` (see [Placed POPs](#placed-pops)). Four keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among POP types. POP type names and good names are separate namespaces |
| `endowment` | A table of `good name = amount`. Each amount is a number (whole or decimal), finite and greater than 0. May be empty (`{}`) |
| `utility` | A table of `good name = sub-table`. Each sub-table has two optional keys, `subsistence` and `share`, each a finite number of at least 0 that defaults to 0. A good needs at least one of them positive, and at least one good needs a positive `share` |
| `growth` | A sub-table with three required keys: `growth`, a non-empty array of finite numbers (the polynomial coefficients c₁, c₂, … of the rate above subsistence); `decline`, a finite number of at least 0 and less than 1 (the rate at u = −1, applied linearly below subsistence); and `max_growth`, a finite number of at least 0 (the cap on the rate). See [growth.md](growth.md) |

Both TOML table forms are accepted for `endowment`, for `utility`, for each utility good and for `growth`:

```toml
[[pop_type]]
name = "farmer"
endowment = { labor = 10 }
utility = { grain = { subsistence = 2, share = 3 }, labor = { subsistence = 1, share = 1 } }
growth = { growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }

[[pop_type]]
name = "fisher"

[pop_type.endowment]
fish = 2
labor = 8

[pop_type.utility.fish]
subsistence = 1        # share 0: a pure necessity

[pop_type.utility.coin]
share = 0.5

[pop_type.growth]
growth = [0.03]        # plain linear growth
decline = 0            # never shrinks
max_growth = 1
```

TOML forbids repeated keys in one table, so a good can't appear twice in an endowment or a utility.

## How POP types are loaded

`setting/pops.rs` holds `PopTypesReader` (and `PopsReader`, see [How POPs are loaded](#how-pops-are-loaded)). `WorldContent` has a `pop_types: PopTypesReader` field, and the `pop_type` section type (folder `Rules`, listed after `market`) calls `PopTypesReader::read` for each entry. POP types are a named kind, so the reader uses `Definitions<PopTypeKind>` like `ResourcesReader`, and resolves good names in phase 2. `PopTypesReader::defs()` exposes those definitions so `PopsReader` can resolve a POP's `type` against them.

1. **Phase 1, `read`.** Every key is read before anything is defined, so no key is falsely reported as unknown and one entry with several problems reports all of them.
   - `name` with `required_name`.
   - `endowment` with `EntryReader::required_number_table` (see [setting-loader.md](setting-loader.md)). Each value that parsed as a number goes through `EntryReader::positive_quantity` with the prefix `key '<good>' in endowment of pop_type`, which reports `key 'grain' in endowment of pop_type must be a finite number greater than 0` at the value otherwise. Every key is kept, with `None` for a bad amount, so its good is still resolved.
   - `utility` with `EntryReader::required_table_of_tables`. A value that isn't a sub-table is reported there and kept as `None`. Each sub-table is read with `EntryReader::nested("utility", name, table)`, whose label is `utility.<good> of pop_type`. The private `amount` helper reads `subsistence` and `share`: it uses `has` to tell an absent key (default 0) from a present one, then `optional_number`, then `EntryReader::non_negative_number`, which checks the value is finite and at least 0 (`key 'share' in utility.grain of pop_type must be a finite number of at least 0`, at the value). The nested reader's `finish` reports unknown keys in the sub-table. A good whose subsistence and share are both 0 (or both absent) is reported at its key as `key 'coin' in utility of pop_type must have a positive subsistence or share`.
   - If `utility` was a table and no good has a positive share, `utility of pop_type needs at least one good with a positive share` is reported at the `[[pop_type]]` header. A good that couldn't be read (not a table, or a bad `subsistence` or `share`) counts as possibly positive, so an entry isn't given this error on top of the one already reported. A missing or non-table `utility` doesn't get it either.
   - `growth` with the private `read_growth`, which calls `EntryReader::required_sub_table("growth")` (see [setting-loader.md](setting-loader.md#entryreader)) and reads, in this order and all before deciding anything: `growth` with `required_number_array` (an empty array is reported at the array as `key 'growth' in growth of pop_type must have at least one coefficient`; an element that is a number but not finite at the element as `element 3 of key 'growth' in growth of pop_type must be a finite number`), `decline` with `required_number` and its own range check (`must be a finite number of at least 0 and less than 1`), `max_growth` with `required_number` and `non_negative_number`, and then the sub-reader's `finish` for unknown keys. Its keys are fields (name-checked, unknown ones reported), unlike the data keys of `endowment` and `utility`. Missing keys inside are reported at the `growth` key (or at its name in a `[pop_type.growth]` header). The entry's `growth: Option<GrowthParams>` is `None` if anything in the sub-table was bad. The name is still defined, so a type with a bad `growth` takes part in the duplicate check.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)`). The name is defined even when other keys are bad, so later duplicates are still caught.
   - **Every entry is kept** for phase 2, as a `PopTypeEntry` with an optional `PopTypeId` (`None` if the name was missing, invalid or a duplicate). Unlike `ResourcesReader`, which stores data only when the name defines, this lets an entry with a bad name still have its unknown goods reported.
2. **Phase 2, `resolve(goods, sink)`.** For every entry, resolves each endowment key and each utility key through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key). The good may be defined in any `rules/` file. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `PopTypesReader::finish` skips entries without an ID, builds each `PopType::new` from the resolved goods, the valid values and the entry's `GrowthParams`, and returns `PopTypes::from_parts`. It is only called after a successful load, when every good resolved and every value was valid; it panics with `internal error: a POP type's goods were not resolved in a successful load` otherwise, and a `None` growth panics with `internal error: a POP type's growth was invalid in a successful load`. `load_setting` stores the result in `Rules::pop_types`.

IDs follow load order (file read order, then the order of entries in each file), so the same folder always gives the same result.

All `[[pop_type]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `name`, `endowment`, `utility` or `growth` missing | header | `missing required key 'endowment' in pop_type` |
| a bad name | value | `invalid name 'Bad Name' (names must start with a lowercase letter ...)` |
| a duplicate name | value | `duplicate pop type name 'ok' (first defined at <location>)` |
| `endowment`, `utility` or `growth` not a table | value | `key 'utility' in pop_type must be a table, found string` |
| an endowment amount that isn't a number | value | `key 'grain' in endowment of pop_type must be a number, found string` |
| an endowment amount that is zero, negative, `inf` or `nan` | value | `key 'grain' in endowment of pop_type must be a finite number greater than 0` |
| a utility entry that isn't a table | value | `key 'grain' in utility of pop_type must be a table, found integer` |
| a `subsistence` or `share` that isn't a number | value | `key 'subsistence' in utility.coin of pop_type must be a number, found string` |
| a `subsistence` or `share` that is negative or not finite | value | `key 'share' in utility.grain of pop_type must be a finite number of at least 0` |
| a utility good with nothing positive | key | `key 'coin' in utility of pop_type must have a positive subsistence or share` |
| no utility good with a positive share | header | `utility of pop_type needs at least one good with a positive share` |
| a good name that matches no good | key | `unknown good 'tin'` |
| an unknown key in a utility good | key | `unknown key 'weight' in utility.grain of pop_type` |
| `growth`, `decline` or `max_growth` missing inside `growth` | the `growth` key (or its name in a `[pop_type.growth]` header) | `missing required key 'decline' in growth of pop_type` |
| the `growth` list not an array | value | `key 'growth' in growth of pop_type must be an array, found float` |
| the `growth` list empty | value | `key 'growth' in growth of pop_type must have at least one coefficient` |
| a coefficient that isn't a number, or an integer beyond `i64` | element | `element 2 of key 'growth' in growth of pop_type must be a number, found string` (or `... is out of range`) |
| a coefficient that is `inf` or `nan` | element | `element 3 of key 'growth' in growth of pop_type must be a finite number` |
| `decline` or `max_growth` not a number | value | `key 'decline' in growth of pop_type must be a number, found string` |
| `decline` negative, 1 or more, or not finite | value | `key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1` |
| `max_growth` negative or not finite | value | `key 'max_growth' in growth of pop_type must be a finite number of at least 0` |
| an unknown key in `growth` | key | `unknown key 'cap' in growth of pop_type` |
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

To compute what a POP of a type wants from its holdings and a hex's prices, pass the type, a size, the holdings and a `PriceList` to `demand` (see [demand.md](demand.md)). `peasant.growth()` gives the type's `GrowthParams`, and `growth_rate(peasant.growth(), u)` gives the rate at any index (see [growth.md](growth.md)).

## Placed POPs

A POP is N identical individuals of one type on one hex. Its endowment and its demand (see [demand.md](demand.md)) are per-individual amounts scaled by N. **Size is continuous**, a `Quantity` rather than an integer: growth multiplies N by a rate every turn (see [growth.md](growth.md)), which with whole numbers would freeze small POPs or need a rounding rule. Fractional individuals are no stranger than a deposit emitting 2.5 of a good, and a UI may round N for display.

**At most one POP of each type per hex.** Two POPs of one type on one hex would see the same prices and get the same per-individual endowment, so they would behave as one POP split in two. A hex may hold POPs of several types, or none.

POPs have no names. They are identified in setting files by (type, hex), and a `PopId` is valid only within the `Pops` list that produced it. A POP's **stockpile** is not part of the `Pop` record: it is world state, held by the `World` (see [Endowment in the turn](#endowment-in-the-turn)).

### Data model

```rust
pub enum PopKind {}                       // IdKind, label "pop"
pub type PopId = Id<PopKind>;
pub struct Pop {
    pub hex: HexId,
    pub pop_type: PopTypeId,
    pub size: Quantity,                   // N; finite, greater than 0, changed each turn by growth
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

`Pops::from_pops(pops, hex_count)` is crate-private: only the loader builds POPs. It numbers the POPs 0, 1, 2... in the order given, fills `by_hex`, and panics if a hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), if two POPs share a hex and a type (`"two POPs of {pop_type:?} on {hex:?}"`; the loader rejects duplicates first, so this is an internal bug), or if there are more than `u32::MAX` POPs. `Pops::default()` is empty and was built with no map, so `on_hex` and `of_type_on_hex` panic on it for any hex. `Pops::set_size(id, size)` is crate-private too: only the World's growth step changes a size. A size is greater than 0 in the setting and is multiplied by a factor above 0 each turn, so it stays positive (only floating-point underflow after thousands of turns of decline could reach 0).

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

1. **Phase 1, `read`.** Records the header's location, then reads every key before any early return, so no key is falsely reported as unknown and one entry with several problems reports all of them: `type` with `required_string` (not name-checked: a bad value is simply an unknown POP type in phase 2), `q` and `r` with the shared `coordinate` helper from `setting/map.rs`, and `size` with `required_number` followed by `EntryReader::positive_quantity("key 'size' in pop", ..)` (otherwise `key 'size' in pop must be a finite number greater than 0` at the value).
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

The `World` takes the `Pops` over as state (`World::pops()`), because growth changes their sizes every turn, and gives every POP its own `Stockpile` in `pop_stockpiles: IdVec<PopKind, Stockpile>`, read with `World::pop_stockpile(pop)`. Every POP stockpile is empty when the world is created: POPs have no starting cash or goods.

At the start of every turn, including turn 0, after deposits emit, the private `World::endow` walks the POPs in ID order. For each `Endowment { good, amount }` of the POP's type it adds `amount.saturating_mul(size)` to the POP's stockpile, in the group `PerishesAt::for_good(good.perishes, turn)`. So an endowed good with `perishes_after = 1` (the sample's `unskilled_labor`) would be gone at the end of the turn it arrives if the POP kept it, and a durable endowed good accumulates. A type with an empty endowment adds nothing. The totals per good become `TurnReport::endowed`. At the end of the turn, POP stockpiles perish after the hex stockpiles. See [world.md](world.md) for the turn as a whole.

### Trade and consumption in the turn

Right after the endowment, the trade round runs on every hex (see [trade.md](trade.md)). Each POP submits its type, its size and its whole stockpile; the market computes its demand, settles its sales (after every POP's sales on the hex, its purchases), and the POP consumes, of each utility good, the smaller of what it holds and what it demanded. Goods outside the utility are never consumed. Leftover currency stays in the stockpile and is income next turn. So between calls a POP's stockpile holds only what the round left: in the sample, a typical POP holds at most a sliver of `silver`, and the no-market peasant on (2, -2) holds nothing, having consumed its whole labor endowment. The World keeps each POP's outcome of the current turn, read with `World::pop_outcome(pop)`.

### Growth in the turn

After every hex has traded, each POP's size becomes `size × (1 + rate)`, where the rate comes from its subsistence index this turn and its type's growth parameters (see [growth.md](growth.md)). The next turn's endowment and demand use the new size. So between calls `world.pops().get(id).size` is the size after growth, and the size this turn's endowment and trade were for is `world.pop_outcome(id).starting_size()`. In the sample, a peasant on a market hex grows about 2.67% a turn, the center artisan 3%, and the north-east peasant, which can never get grain, shrinks 10% a turn (12 → 10.8 → 9.72).

### Using POPs

```rust
let (world, report) = World::new(load_setting(path)?);
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
let labor = world.rules().goods.id("unskilled_labor")?;
for &id in world.pops().on_hex(center) {                // ID order
    let pop = world.pops().get(id);
    let type_name = world.rules().pop_types.name(pop.pop_type);
    let left = world.pop_stockpile(id);                 // what the trade round left
    let outcome = world.pop_outcome(id);                // this turn's trade and growth
    let labor_endowed = outcome.demand().get(labor).map(|g| g.held);
    let labor_sold = outcome.sold_of(labor);
    let size_now = pop.size;                            // after growth
    let size_then = outcome.starting_size();            // what the turn's numbers were for
}
let artisan = world.rules().pop_types.id("artisan")?;
let artisans_here = world.pops().of_type_on_hex(center, artisan);   // Option<PopId>
```

The dev UI shows every POP type in its world panel and every POP of the selected hex in its hex panel (see [dev-ui.md](dev-ui.md)).

## Not yet

Removing POPs that shrink, a minimum size, starting stockpiles or cash for POPs, saving beyond rounding leftovers, changing a POP's hex or type during play, migration, and per-POP parameters that differ from the type (growth included).

## The sample setting and tests

- `settings/sample/rules/pops.toml` defines two types, both endowed with 10 `unskilled_labor` per individual per turn and both with `growth = [0.06, -0.03]`, `decline = 0.1`, `max_growth = 0.05` in a `[pop_type.growth]` table. `peasant` (ID 0): `unskilled_labor` subsistence 2, share 0.2; `grain` subsistence 2, share 0.8. `artisan` (ID 1): written with weights 1, 4 and 5, normalized to `unskilled_labor` subsistence 1, share 0.1; `copper` share 0.5 (no subsistence); `grain` subsistence 2, share 0.4. Its header comment explains the keys (including the subsistence index, the polynomial, the decline, the clamp, and that `growth = [0]` with `decline = 0` freezes a type), that shares are weights, leisure, and that unskilled labor is the intended endowment, and points to `start/pops.toml`.
- `settings/sample/start/pops.toml` places 16 POPs, listed in the map's order (row by row, north to south, west to east): a `peasant` on 15 of the 19 hexes, with sizes from 12 to 60, and an `artisan` of size 10 on the center hex (0, 0), the only hex that prices every good. The center therefore holds two POPs (IDs 7 and 8). The north-east corner (2, -2), which has no market, holds a `peasant` (ID 2) whose labor can never be sold and which never gets grain, so it shrinks every turn. Hexes (2, -1), (-2, 0), (1, 1) and (-2, 2) have no POPs. The sizes total 410 in the setting, so POPs are endowed `4100 unskilled_labor` at turn 0; from then on the sizes grow or shrink. The file's header comment explains the keys, continuous size, the one-POP-per-type-per-hex rule, that POPs start with nothing, and that sizes change each turn.
- `pops.rs` unit tests: normalization and sorting, huge weights that don't overflow, a single weight giving share 1, a zero-weight good kept with share 0, an empty endowment, `endowment_of` and `utility_of`, and the three `PopType::new` panics; `GrowthParams` stored and returned by `PopType::growth()` and each `GrowthParams::new` panic; for `Pops`, sequential IDs, `on_hex` in ID order and empty for a hex with none, `of_type_on_hex`, `set_size`, the empty default, and the panics (foreign ID, hex beyond the map, the default list, `from_pops` with a hex beyond `hex_count` or two POPs of one type on one hex). They use `good`, `q`, `close` and `no_growth` from `crates/engine/src/test_support.rs`.
- `crates/engine/tests/pops.rs`: the sample's two types with their normalized shares (summing to 1) and their growth parameters; `pops_valid` with IDs in load order, both table forms (for `growth` too), a multi-good endowment written out of ID order, an empty endowment, a pure necessity, the currency in a utility, each type's growth parameters, and the same result on a second load; every `pops_errors` error at its place, including every `growth` error; `valid_minimal` with no POP types. For placed POPs: the sample's 16 POPs with their hexes, types and sizes and its center, corner and empty hexes; a fresh sample world in which every POP's recorded demand shows `10 × starting_size` labor held, every POP stockpile holds no labor and at most a sliver of silver, and the report's `endowed` is 4100 labor; steps that re-endow by the grown sizes (each turn's `endowed` is 10 × the previous report's `population`), with the labor used up rather than perishing; `pops_placed` loaded and stepped (the workers consume their labor, the hoarder keeps its labor and its growing gold, every size unchanged and the population 12.5 every turn); every `pops_placed_errors` error at its place; worlds with no POPs endowing and trading nothing, with a population of 0; and `pop_stockpile` panicking on a POP from another world. Growth itself is tested in `crates/engine/tests/growth.rs` (see [growth.md](growth.md)).
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a POP type whose endowment and utility name goods that don't exist, and a `[[pop]]` of an unknown type at coordinates with no hex; only the syntax error is reported.
- Test settings: `pops_valid` (goods `coin`, `labor`, `grain`, `fish`; types `farmer`, `fisher`, `idle` across `rules/a.toml`, `rules/a/z.toml` and `rules/b.toml`, with the goods in a file read after them; real growth parameters, `fisher`'s in the `[pop_type.growth]` form); `pops_one` (one good, one type, one hex and one POP, for the checker's `1 pop type`, `1 hex` and `1 pop`); `pops_errors` (every entry in `rules/a.toml`, `rules/b.toml` and `start/p.toml` ends with a valid `growth` line, and `rules/c.toml` holds every `growth` error, including one entry, a duplicate of `ok`, with four growth problems); `pops_placed` (goods `labor` and `grain`, both perishing after 1 turn, and durable `gold`; types `worker` endowing 10 `labor` with a `labor` utility, `hoarder` endowing 4 `labor` and 1.5 `gold` with a `grain` utility, and `idle` with an empty endowment; hexes (0, 0), (1, 0), (2, 0), each with currency `gold` and none with a market, and a `field` deposit on (2, 0); POPs `worker` 3 and `hoarder` 2 on (0, 0) in `start/a.toml`, `worker` 0.5 and `idle` 7 on (1, 0) in `start/b/c.toml`. With no market, the workers consume their labor and the hoarder, whose utility good it doesn't hold, keeps its labor and its gold, which accumulates); `pops_placed_errors` (every `[[pop]]` error in the table above, one entry with four problems, a duplicate across files, and a `[[pop]]` in `rules/`); `pops_trade`, for the trade round (see [trade.md](trade.md)); and `pops_growth`, for growth (see [growth.md](growth.md)). `pops_one`, `pops_placed`, `pops_placed_errors` and `pops_trade` give every type zero growth parameters (`growth = [0]`, `decline = 0`, `max_growth = 0`), so their sizes never change. `crates/engine/tests/settings/README.md` lists every POP of each with its expected numbers.
