# Currencies

Source: `crates/engine/src/currency.rs` (the data model) and `crates/engine/src/setting/map.rs` (reading the `currency` key of each `[[hex]]`). The public type is re-exported from the engine crate root: `HexCurrencies`.

Every hex designates one good as its **currency**: the good that prices on that hex are posted in. The designation is required, so every hex has exactly one currency and no code ever handles a hex without one. Different hexes may designate different goods. The dummy market posts its prices in the hex's currency (see [market.md](market.md)), and the trade round pays and is paid in it (see [trade.md](trade.md)).

The engine has no opinion about money. The only requirement is that the good exists in the setting's `rules/`: a perishable or non-movable good (even labor) is a valid currency, and there is no currency-specific good property.

## Why it is state, not a rule, and not part of the map

The designation is starting state (it is written in `start/`) because it can change during play, for example when a central bank issues a synthetic currency in a later Stage. For the same reason it is not stored in `HexMap`: the map is geometry and never changes, while currencies are per-hex state kept in a parallel table, like the hex stockpiles in `World`. Nothing changes a currency yet, and there is no API to change one.

## Data model

```rust
pub struct HexCurrencies { by_hex: IdVec<HexKind, GoodId> }
```

| Method | Result |
|---|---|
| `len()`, `is_empty()` | Number of hexes (equal to the map's hex count) |
| `get(hex) -> GoodId` | The currency of `hex`. Panics (`is not from this list`) if `hex` is not from the map these were built with |
| `iter() -> (HexId, GoodId)` | Every hex's currency, in hex ID order |

`HexCurrencies::default()` is empty (no hexes). The constructor `from_vec(Vec<GoodId>)` is crate-private, so a `HexCurrencies` always comes from a checked setting; entries are in hex ID order. `currency.rs` depends on `goods`, `map` and `id` and knows nothing about files.

A `GoodId` is only meaningful with the `Goods` list of the same setting; look the name up with `rules.goods.name(id)`.

## How currencies are loaded

The `currency` key is part of each `[[hex]]` entry, so `MapReader` (see [map.md](map.md)) reads it alongside `q` and `r`:

1. **Phase 1.** `MapReader::read` reads `currency` with `required_string`, before any early return, so it is never falsely reported as unknown. A missing key is reported at the header (`missing required key 'currency' in hex`); a non-string at the value (`key 'currency' in hex must be a string, found integer`). The value is not name-checked: like a resource type's `produces`, a badly spelled value is simply an unknown good in phase 2. The currency is kept unresolved (`Option<Located<String>>`) in the `currencies` vector, indexed like the hex coordinates. Only placed hexes push a currency: a hex with invalid coordinates is dropped entirely, so its currency is never checked. A hex whose currency is missing or not a string *is* still placed (with `None`), so duplicate-hex and deposit-reference checks keep working; the load fails anyway.
2. **Phase 2.** `WorldContent::resolve` calls `MapReader::resolve(goods.defs(), sink)` first, which resolves each placed hex's currency through `Definitions::<GoodKind>::resolve`. An unknown name is reported at the value as `unknown good 'tin'`. The good may be defined in any `rules/` file. The map must resolve before the markets, because `MarketsReader::resolve` reads the resolved currencies through `MapReader::currency(hex)` to reject a market that prices its own currency. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)), so no misleading "unknown good" is shown.
3. **Finish.** `MapReader::finish` returns `(HexMap, HexCurrencies)`. It is only called after a successful load, when every currency has resolved (it panics with an `internal error: ...` message otherwise). `load_setting` stores the currencies in `StartState::currencies`.

The currency for `HexId` n is the one written on the n-th hex loaded, so IDs and order follow the map and the same folder always gives the same result.

## Using currencies

```rust
let setting = load_setting(path)?;
let silver = setting.rules.goods.id("silver")?;
for (hex, good) in setting.start.currencies.iter() { /* hex ID order */ }

let (world, _) = World::new(setting);
let good = world.hex_currency(hex);        // GoodId
let all = world.hex_currencies();          // &HexCurrencies
let market = world.market(hex);            // market.currency() is the same good
```

`World::new` moves `StartState::currencies` into the world (see [world.md](world.md)). Stepping the world does not change it. The dev UI's hex panel shows the currency as the first line of the Market section (see [dev-ui.md](dev-ui.md)).

## Not yet

Changing a hex's currency during play (and what then happens to prices and holdings), any restriction on which goods may be currencies, a map-wide default currency, and exchange between the currencies of different hexes.

## The sample setting and tests

- The sample defines `silver` (durable, movable) in `settings/sample/rules/goods.toml`, and all 19 hexes in `settings/sample/start/map.toml` designate it. The file's header comment notes that hexes may differ.
- `currency.rs` unit tests: IDs in order, `get`, `iter`, the empty default, and the foreign-hex panic. They use the `good` and `hex` helpers from `crates/engine/src/test_support.rs` (see [workspace.md](workspace.md)).
- `crates/engine/tests/currency.rs`: the sample's hexes are all `silver`; `hexes_valid` gives `coin, shell, coin, shell` by hex ID; `valid_minimal` has no currencies; loading twice gives the same list.
- `crates/engine/tests/world.rs`: `hex_currency` on `turns_valid` (all `gold`) before and after three steps, the sample still `silver` after a step, and the foreign-hex panic.
- `crates/engine/tests/map.rs` (`every_hex_error_is_reported_at_its_place`): the missing key, the integer value, the wrongly cased `'Coin'`, the unknown `'tin'`, and a hex with invalid coordinates whose unknown currency is not reported.
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a hex with an unknown currency, and only the syntax error is reported.
- Test settings that designate currencies: `hexes_valid` (`coin`, `shell`), `hexes_one` and `hexes_errors` (`coin`), and every other setting with hexes (`deposits_errors`, `resources_one`, `resources_valid`, `turns_valid`, `markets_valid`, `pops_placed`, `pops_trade`, `pops_growth`, and the rest) names one of its existing goods. The dev UI's `session.rs` tests write temporary settings whose hexes use a good `coin`.
