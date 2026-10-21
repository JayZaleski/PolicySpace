# Businesses

Source: `crates/engine/src/businesses.rs` (the data model), `crates/engine/src/setting/businesses.rs` (reading `[[business]]` sections), deposit ownership in `crates/engine/src/resources.rs` and `crates/engine/src/setting/resources.rs`, and the business stockpiles, answers and outcomes in `crates/engine/src/world.rs`. The public types are re-exported from the engine crate root: `BusinessKind`, `BusinessId`, `Business`, `Businesses`.

A **business** is a named actor on one hex. It has a **starting inventory** (the goods, cash included, in its stockpile when the world is created), owns some of the **deposits** on its hex, owns **factories** (see [factories.md](factories.md)), and names the **decider** that controls it. Businesses are starting state, defined in `start/` and checked by the loader. In the running world each business has its own stockpile, and an owned deposit emits into its owner's stockpile instead of the hex's.

Factories name a business as their owner, and must sit on its hex. Every turn, at the decision point, each business is served its view and its decider's answer is validated and stored in the world (see [decider.md](decider.md) for the view and the decision, [decider-host.md](decider-host.md) for the exchange, and [python.md](python.md) for the first decider, the arithmetic rule). Then, in its hex's trade round, a business with a valid decision sells its offers and buys its bids from and into its stockpile, and right after the round it runs, wears and maintains its factories at the fractions the decision gives (see [trade.md](trade.md)). A business that decided nothing, or whose decision was refused, trades nothing and its factories only wear.

## Concepts

- **One hex.** A business sits on one hex and may own only deposits and factories on that hex. Once trade between hexes exists, businesses are expected to operate across several hexes with this one as their headquarters. A hex may hold any number of businesses, and a business may sit on a hex with no market (it then trades nothing, but still operates its factories from what it holds).
- **Named.** Businesses are the first named kind in the starting state: each has a name, unique among businesses, because factories refer to their owner by name (see [factories.md](factories.md)) and the decider protocol names businesses in its messages. Business names are their own namespace, separate from goods, resource types, POP types and technologies.
- **Cash is a good.** There is no separate cash field. A business's cash is the amount of its hex's currency in its stockpile, so its starting cash is the currency it lists in its inventory, zero if it lists none (see [currency.md](currency.md)). A business that starts with no cash is valid; the engine doesn't police economics.
- **Deposit ownership** is written on the business, not on the deposit: the business says how many deposits of each resource type it owns, and the loader hands out that many not-yet-owned deposits of that type on its hex. Deposits of one type on one hex are identical, so which units a business gets doesn't matter, but the assignment is deterministic: deposit ID order, businesses taken in load order. Each deposit has at most one owner. Nothing changes ownership during play yet.
- **Decider name.** Required and a valid name. All the businesses naming one decider share it: each turn it receives one request with all their views. The name is resolved against the decider host when a world is created and at every step, and a business naming a decider the host doesn't hold is an error (`UnknownDecider`). The dev UI and the checker build the host from the names themselves, starting `python -m policyspace <name>` for each, so a name the Python package doesn't know is a load error there (see [decider-host.md](decider-host.md#finding-python-deciderpython)). The loader checks only that it is a valid name.

## Data model

```rust
pub enum BusinessKind {}                    // IdKind, label "business"
pub type BusinessId = Id<BusinessKind>;
pub struct Business {
    pub hex: HexId,                         // its only hex until trade between hexes exists
    pub decider: String,                    // a valid name; resolved against the decider host by the World
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

`Businesses::from_parts(names, businesses, hex_count)` is crate-private: only the loader builds businesses. It builds the `NamedList` (panicking with `every business name needs exactly one entry` if the lengths differ), then fills `by_hex`, panicking if a business's hex index is `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`) or if there are more than `u32::MAX` businesses. `Businesses::default()` is empty and was built with no map, so `on_hex` panics on it for any hex. The fields are private. `businesses.rs` depends on `bundle`, `id` and `map` and knows nothing about files; it does not depend on `resources` or `factories`, which depend on it for `BusinessId` and `BusinessKind`.

### Deposit ownership

```rust
pub struct Deposit {
    pub resource: ResourceId,
    pub hex: HexId,
    pub owner: Option<BusinessId>,   // set by the loader; nothing changes it during play yet
}
```

`Deposits::owned_by(business)` returns an iterator over the `DepositId`s that business owns, in ID order, from a per-owner index (`by_owner`) built once at load beside the per-hex index; a deposit with no owner is in no owner's list. It panics for a business not from the businesses the list was built with. `Deposits` lives in the World as state, so a running world reads ownership with `world.deposits().get(id).owner` or `world.deposits().owned_by(business)`. See [resources.md](resources.md). `Factories::owned_by` works the same way for factories (see [factories.md](factories.md)).

## Setting format

A `[[business]]` belongs in `start/` (in `rules/` it gets `section 'business' belongs in start/, not rules/`). Six keys, all required:

| Key | Meaning |
|---|---|
| `name` | Lowercase `snake_case`, unique among businesses. A separate namespace from goods, resource types, POP types and technologies |
| `q`, `r` | Integer axial coordinates of a hex in the map, read as a deposit's or POP's are |
| `inventory` | A table of `good name = amount`, each amount a finite number greater than 0. May be empty (`{}`) but must be written. The hex's currency listed here is the business's starting cash |
| `deposits` | A table of `resource type name = count`, each count a whole number of at least 1 (and at most 4294967295). May be empty (`{}`) but must be written. The business gets that many not-yet-owned deposits of that type on its hex |
| `decider` | A valid name. Resolved when a world is created, not by the loader (see [decider-host.md](decider-host.md)) |

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

Any other key is an error. `[[deposit]]` entries are unchanged: ownership is written only on the business. A business's factories are written as `[[factory]]` entries naming it as `owner` (see [factories.md](factories.md)).

## How businesses are loaded

`setting/businesses.rs` holds `BusinessesReader`, which follows `TechnologiesReader` (see [technologies.md](technologies.md#how-technologies-are-loaded)) for its name and its good references and `PopsReader` (see [pops.md](pops.md#how-pops-are-loaded)) for its coordinates. `WorldContent` has a `businesses: BusinessesReader` field, and the `business` section type (folder `Start`, listed after `technology`) calls `BusinessesReader::read` for each entry. Businesses are a named kind, so the reader uses `Definitions<BusinessKind>`.

1. **Phase 1, `read`.** Records the header's location, then reads every key before anything is defined, so no key is falsely reported as unknown and one entry with several problems reports all of them. In order:
   - `name` with `required_name`.
   - `q` and `r` with the shared `coordinate` helper from `setting/map.rs`.
   - `inventory` with `EntryReader::required_quantity_table` (prefix `key '<good>' in inventory of business`). Every key is kept, with `None` for a bad amount, so its good is still resolved. A missing or non-table `inventory` is reported there and gives an empty list (the load fails anyway).
   - `deposits` with the private `read_deposits`, which calls `EntryReader::required_integer_table("deposits")` (see [setting-loader.md](setting-loader.md#entryreader)) and then passes each count through `EntryReader::at_least_one` with the prefix `key '<k>' in deposits of business` and no upper bound: below 1 is `key '<k>' in deposits of business must be at least 1` and beyond a `u32` is `key '<k>' in deposits of business is out of range`, both at the value. Every key is kept, with `None` for a bad count, so its resource type is still resolved.
   - `decider` with `required_name`.
   - Finally the name is defined with `Definitions::define` (a duplicate is reported at the second name as `duplicate business name 'ok' (first defined at <location>)`).
   - **Every entry is kept** for phase 2 as a `BusinessEntry` with an optional `BusinessId` (`None` if the name was missing, invalid or a duplicate), so an entry with a bad name still has its unknown goods and resource types reported.
2. **Phase 2, `resolve(goods, resources, map, deposits, sink)`.** For every entry, in load order:
   - resolves each `inventory` key through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key) and each `deposits` key through `Definitions::<ResourceKind>::resolve` (`unknown resource 'meadow'` at the key), always;
   - resolves the coordinates through `MapReader::id` (`no hex at (9, 9)` at the header);
   - then, only if the entry has an ID and a hex, **claims its deposits**: for each `deposits` key whose count and resource type are both valid, `DepositsReader::claim(hex, resource, count, id)`. If the hex has fewer unowned deposits of that type than claimed (counting what earlier businesses took), nothing is assigned for that key and the error `key '<k>' in deposits of business claims more deposits than the hex has unowned (<count> claimed, <available> available)` is reported at the key.

   `WorldContent::resolve` calls this after `DepositsReader::resolve` has expanded the deposit list the claims walk, and before `FactoriesReader::resolve`, which needs the businesses' resolved hexes. Goods, resource types and hexes may be defined in any file. As for every reference, phase 2 (and with it deposit assignment) is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)).
3. **Finish.** `BusinessesReader::finish(hex_count)` skips entries without an ID, builds each `Business` (the starting inventory with `Bundle::new`, which sorts it by good ID), and returns `Businesses::from_parts`. It is only called after a successful load and panics with `internal error: a business was not fully resolved in a successful load` otherwise. `load_setting` calls it after `PopsReader::finish` and before `DepositsReader::finish` and `FactoriesReader::finish`, which take the business count for their per-owner indexes, and stores the result in `StartState::businesses`.

`DepositsReader::claim(hex, resource, count, owner) -> Result<(), u32>` (crate-private) first counts the deposits of `resource` on `hex` with no owner. If there are fewer than `count`, it returns `Err(available)` and changes nothing; otherwise it sets `owner` on the first `count` of them in deposit ID order. Because claims run in load order, the same folder always gives the same owners.

Two accessors serve the factories reader, which resolves after the businesses: `BusinessesReader::defs()` gives the business `Definitions`, for resolving a factory's `owner`, and `BusinessesReader::hex_of(id) -> Option<(HexId, HexCoord)>` gives the resolved hex of business `id` and the coordinates it was written with, or None if its coordinates were missing, invalid or matched no hex (already reported). Entries without an ID are skipped, so `hex_of` searches the entries for the ID rather than indexing them. See [factories.md](factories.md#how-factories-are-loaded).

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

- **Creation.** `World::new(setting, &mut host)` first checks that the host holds every business's decider name, then fills each business's stockpile from its starting inventory before turn 0 runs. Each good goes in as one group that perishes by the good's own rule as if added in turn 0 (`PerishesAt::for_good(perishes, Turn::ZERO)`), so a perishable good listed there (labor, say) is gone at the end of turn 0, and a durable currency stays. Turn 0's emission then lands on top.
- **Emission.** At the start of every turn, every deposit emits in deposit ID order, into its owner's stockpile when it has one and into its hex's stockpile when it doesn't. The hex stockpile stays the home of goods nobody owns.
- **Decision point.** After emission and endowment, the business's view is served to its decider and the answer is stored (`World::business_answer(business)`, a `BusinessAnswer`: nothing, a valid decision, or a refused decision with its errors; see [decider-host.md](decider-host.md#answers-and-the-report)).
- **Trade and operation.** In part 2 of the turn the business takes part in its hex's round behind the POPs: its offers are sold (trimmed to what it holds) before any purchase on the hex, and its bids are bought under the **cash cap**, one proportion scaling every bid when they cost more than its currency. Its cash is the amount of its hex's currency in its stockpile. Right after the round its factories run, wear and are maintained, taking inputs and maintenance goods from its stockpile and adding their output to it, to be sold next turn (see [trade.md](trade.md)).
- **Outcome.** `World::business_outcome(business)` returns what the current turn did for it: a `BusinessTrade` (offered beside sold, bid beside bought, the proportion, its cash before the round, after the sales and after the purchases) and one `FactoryRecord` per factory it owns (see [trade.md](trade.md#businesstrade) and [production.md](production.md#data-model)).
- **Perishing.** At the end of every turn, perishing covers hex stockpiles, then POP stockpiles, then business stockpiles, each in ID order.
- **Report.** `emitted` is one total over every deposit, owned or not, and `perished` includes what perished from business stockpiles. The business lists `business_sold`, `business_bought`, `produced` and `used` total every business and factory (see [world.md](world.md#turnreport)). Nothing changes a business or a deposit's owner during play.

## Using businesses

```rust
let mut host = DeciderHost::idle();                      // or python::host_for(&setting)? (see decider-host.md)
let (world, _report) = World::new(load_setting(path)?, &mut host)?;
let center = world.map().id(HexCoord::new(0, 0)).unwrap();
for &id in world.businesses().on_hex(center) {          // ID order
    let name = world.businesses().name(id);
    let business = world.businesses().get(id);
    let decider = business.decider.as_str();             // the name its host resolves
    let answer = world.business_answer(id);              // its answer at this turn's decision point
    let outcome = world.business_outcome(id);            // its trade and its factories' records this turn
    let cash_after = outcome.trade().cash_after_purchases();
    let stock = world.business_stockpile(id);
    let cash = stock.total(world.hex_currency(business.hex));
    let owned: Vec<DepositId> = world.deposits().owned_by(id).collect();
    let factories: Vec<FactoryId> = world.factories().owned_by(id).collect();
    let view = world.business_view(id);                  // what its decider is served (see decider.md)
}
let farm = world.businesses().id("center_farm")?;
```

The dev UI shows each business on its hex (its decider and answer, owned deposits, cash through the round, trade, factories, trace and stockpile), lists every business in the world panel, and marks hexes with businesses on the map (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## Not yet

Changing ownership during play, businesses on several hexes, founding or failing businesses, POP ownership of businesses, operating budgets and dividends (Stage 06), and building, scrapping or retooling factories (Stage 07) are later Stages.

## The sample setting and tests

- `settings/sample/start/businesses.toml` places three businesses on the center hex (0, 0), one per technology in the chain (see [technologies.md](technologies.md)), every one with decider `arithmetic`, the name of the arithmetic rule in the Python package's registry (see [python.md](python.md#the-loop-and-the-command-line-deciderspy-__main__py)):

  | Business (ID) | Starting inventory | Deposits | Owns deposit IDs |
  |---|---|---|---|
  | `center_farm` (0) | 100 `silver` | 2 `arable_land` | 0, 1 |
  | `center_mine` (1) | 50 `silver` | 1 `copper_ore` | 3 |
  | `center_toolmaker` (2) | 200 `silver` | none | none |

  The center's third `arable_land` deposit (ID 2) and every deposit on other hexes stay unowned, so the center has owned and unowned deposits of one type, which exercises the ID-order assignment. The starting silver is a few turns of inputs at the center's prices. Each business owns one factory on the center hex, running the technology it is named for (see [factories.md](factories.md)). Under the idle host, a fresh sample world's `center_farm` holds 8 `unimproved_grain` and 100 `silver`, `center_mine` 2.5 `unimproved_copper` and 50 `silver`, `center_toolmaker` 200 `silver`, and the center hex stockpile only the unowned deposit's 4 `unimproved_grain`; every turn after looks the same, and the POP numbers in the turn reports are unchanged by the businesses. Under the `arithmetic` decider the three businesses form a chain: the farm and the mine buy what their deposits don't cover and sell their output, the toolmaker buys copper and sells tools, and each maintains its factory (see [trade.md](trade.md#worked-examples) for the numbers). The file's header comment explains every key, the ownership rules and the numbers.
- `businesses.rs` unit tests: sequential IDs in the order given, `id`/`name` round trips, `get`, `iter` in order, `on_hex` in ID order and empty for a hex with none, the empty default, and the panics (foreign ID, a hex beyond the map, the default list, `from_parts` with a hex beyond `hex_count` or mismatched lengths).
- `resources.rs` unit tests: `owned_by` lists only that owner's deposits, in ID order, and nothing for a business that owns none; `from_deposits` panics for an owner beyond `business_count`, and `owned_by` on the default list. `setting/resources.rs`: `claim` hands out the first unowned deposits in ID order, changes nothing when short (returning how many were available), and returns `Err(0)` on a hex with none of that type. `setting/reader.rs`: `required_integer_table` and `at_least_one` (see [setting-loader.md](setting-loader.md)). `setting/businesses.rs`: `read_deposits` (a count of 1 and the largest `u32` accepted; 0, a negative, a count beyond `u32` and a float reported; a missing table). `world.rs`: the clock-maximum test also runs `businesses_valid` and checks the business stockpiles, answers and outcomes are unchanged.
- `crates/engine/tests/businesses.rs`: the sample's three businesses, their deposits and owners; a fresh sample world's business and hex stockpiles, and three steps keeping them, with two runs identical; `businesses_valid` (businesses across `start/a.toml`, `start/b/c.toml` and a later `start/z.toml` holding the hexes and deposits they refer to, both table forms, two businesses sharing the unowned deposits of one type on one hex, an empty inventory and no deposits on a hex with no market, a perishable `labor` gone after turn 0 and a durable `coin` kept), loaded twice with the same result and stepped through turn 3; every `businesses_errors` error at its place, including one entry (`Several`) with many problems and a second business running out of deposits in a later file; `valid_minimal` and `turns_valid` with no businesses and every deposit unowned; `businesses_one`; and `business_stockpile` panicking on a business from another world. These run under the idle host, so no business trades; the business round is tested in `crates/engine/tests/business_trade.rs` (see [trade.md](trade.md#tests)).
- Test settings: `businesses_valid` (its businesses `farm_a` and `farm_b` name the decider `rule` and `idle` names `other_rule`, which the decider-host tests use), `businesses_one` (one good, one hex and one business, for the checker's `1 business`), `businesses_errors`, and a business with an unknown good and resource type on a hex that doesn't exist in `resources_broken_file` (only the syntax error is reported). `crates/engine/tests/settings/README.md` lists them with their expected values.
