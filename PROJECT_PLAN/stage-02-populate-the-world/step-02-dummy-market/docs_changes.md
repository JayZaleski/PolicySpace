# Step 02: dummy-market — Docs Changes

## docs/market.md — new file
- **Location:** whole file
- **Change:** add

# The dummy market

Source: `crates/engine/src/market.rs` (posted prices and the exchange) and `crates/engine/src/setting/market.rs` (reading `[[market]]` sections). The public types are re-exported from the engine crate root: `HexPrices`, `Market`, `TradeError`.

Each hex may have a **dummy market**: a stub that posts fixed prices in the hex's currency good (see [currency.md](currency.md)) and buys or sells any amount at those prices. It holds nothing of its own. Goods sold to it vanish, goods bought from it appear from nothing, and currency is created and destroyed the same way. There is no inventory and no limit on how much it trades.

The dummy market is the **seam** that Stage 05 replaces with the live market. Callers ask a `Market` for prices and trade through it; when the live market arrives, what stands behind `Market` changes, not the callers. Nothing trades yet: no actor calls the exchange during a turn, and the turn report has no market activity. Steps 05 and 06 (Stone-Geary demand and POP trade) are the first users. `market.rs` depends on `goods`, `id`, `map`, `quantity`, `stockpile` and `turn`, and knows nothing about files.

## Which goods are traded

- A good is traded on a hex **only if the hex's market posts a price for it**. A good with no posted price is "not traded here": the market neither buys nor sells it. So a good may exist in the rules before anything on a hex produces or wants it.
- A hex with no `[[market]]` entry is valid and trades nothing. Later Steps treat it the same as a hex where every good is unpriced.
- The hex's **currency** has price 1 by definition (`Quantity::ONE`). It is never stored in a price list (the loader rejects it) and can't be traded against itself.
- A price is a `Quantity` that is finite and greater than zero (the loader guarantees it). It is the number of units of the hex's currency that one unit of the good costs.
- Prices are starting state, not rules, because the live market will change them. Nothing changes them during play yet; stepping the world leaves them as loaded.

## Data model

### `HexPrices`

```rust
pub struct HexPrices { by_hex: IdVec<HexKind, Option<BTreeMap<GoodId, Quantity>>> }
```

One optional price list per hex, indexed by `HexId`, each list in good ID order. `None` means the hex has no `[[market]]` entry; `Some(empty)` means an entry with no prices. Both trade nothing; the difference exists so the checker can count markets and the dev UI (Step 08) can say "no market".

| Method | Result |
|---|---|
| `len()`, `is_empty()` | Number of hexes (equal to the map's hex count) |
| `market_count()` | Number of hexes that have a `[[market]]` entry, including ones with no prices |
| `has_market(hex)` | True if `hex` has a `[[market]]` entry |
| `get(hex, good) -> Option<Quantity>` | The posted price, or `None`. Knows nothing about the currency: use `Market::price` for the full answer |
| `posted(hex)` | Every posted `(GoodId, Quantity)` on `hex`, in good ID order; empty if none |

The hex methods panic with `is not from this list` for a hex from another map. `HexPrices::default()` is empty (no hexes). The constructor `from_vec` and the accessor `list(hex)` (the raw price list, used to build a `Market`) are crate-private, so a `HexPrices` always comes from a checked setting.

### `Market<'a>`

A per-hex view handed out by `World::market(hex)` (see [world.md](world.md)). It is `Copy` and borrows what it needs: the hex, the rules' `Goods` (for each good's perish property), the hex's currency, its price list, and the world's current turn.

| Method | Result |
|---|---|
| `hex()` | The hex this market is on |
| `currency()` | The good prices are posted in |
| `has_market()` | True if the hex has a `[[market]]` entry |
| `price(good) -> Option<Quantity>` | `Some(ONE)` for the currency (checked first), the posted price, or `None` if the good isn't traded here |
| `prices()` | The posted `(GoodId, Quantity)` pairs in good ID order; the currency isn't listed |
| `sell(&mut stockpile, good, amount) -> Result<Quantity, TradeError>` | Sells `amount` of `good`; returns the currency received |
| `sell_all(&mut stockpile, good) -> Result<(Quantity, Quantity), TradeError>` | Sells everything of `good` the stockpile holds; returns (amount sold, currency received) |
| `buy(&mut stockpile, good, amount) -> Result<Quantity, TradeError>` | Buys `amount` of `good`; returns the currency paid |

`Market::new` is crate-private. `World::market` builds one from shared borrows of the world, so the world can't be stepped while a `Market` is alive, and a `Market` always trades at the world's current turn. Code inside the engine (POP trade in Step 06) can build a `Market` from split borrows of the world's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`) while holding a `&mut` to an actor's stockpile in another field.

## The exchange

The market exchanges goods against a **caller-supplied stockpile**; the same stockpile pays and receives.

- **Selling** removes `amount` of the good from the stockpile (soonest-perishing group first, see [world.md](world.md)) and adds `amount × price` of the currency.
- **Selling everything** (`sell_all`) removes every unit of the good (`Stockpile::remove_all`) and adds `sold × price` of the currency. It exists so a POP that sells all the labor it doesn't keep as leisure never fails on a rounding difference between a computed amount and the amount held. A good not held sells `(0, 0)`.
- **Buying** removes `amount × price` of the currency (soonest-perishing group first) and adds `amount` of the good.

Anything added enters the stockpile **as if it appeared in the current turn**: its group is `PerishesAt::for_good(perishes, turn)`. So a perishable good bought in turn T perishes at the end of turn T + `perishes_after` − 1, and currency received from a sale perishes the same way if the currency is a perishable good.

The amount of currency is `amount.saturating_mul(price)`: the floating-point product, clamped at `f64::MAX`. The engine does no rounding and has no smallest unit of currency.

### Refusals

Every trade is **all-or-nothing**: a refused trade changes nothing. The checks run in this order, and each refusal is a `TradeError`:

| Variant | When |
|---|---|
| `IsCurrency { good }` | `good` is the hex's currency |
| `NotTraded { good }` | no price is posted for `good` on this hex |
| `NotEnoughGoods { good, held, wanted }` | `sell` of more than the stockpile holds |
| `NotEnoughCurrency { currency, held, needed }` | `buy` costing more currency than the stockpile holds |

`sell_all` can only fail with the first two. A zero amount of a traded good passes every check, changes nothing, and returns `Ok(ZERO)` (a zero amount of an untraded good or of the currency is still refused). `TradeError` implements `Display` and `std::error::Error`; IDs are written as `#index` because the error has no access to names:

```
good #2 is not traded on this hex
good #5 is this hex's currency and can't be traded against itself
not enough of good #2 to sell: holds 2, wants to sell 3
not enough currency (good #5) to buy: holds 1.5, needs 3
```

The internal `remove` that `sell` and `buy` call can't fail after the shortfall check; if it ever did, the trade is refused with the same variant rather than panicking.

## Setting format

A `[[market]]` belongs in `start/` (in `rules/` it gets the usual "belongs in start/" error). It has three required keys:

| Key | Meaning |
|---|---|
| `q`, `r` | The axial coordinates of a hex that exists in the map |
| `prices` | A table of `good name = price`. Each price is a number (whole or with decimals), finite and greater than 0 |

Both TOML forms of the table are accepted:

```toml
[[market]]
q = 0
r = 0
prices = { grain = 2, fish = 1.5 }

[[market]]
q = 1
r = 0

[market.prices]
grain = 3
coin = 0.25
```

At most one market per hex; a hex needs none. `prices = {}` is valid and means a market that trades nothing. Any key other than `q`, `r` and `prices` is unknown. A good may be priced on a hex whose currency is a different good (above, `coin` is priced on a hex whose currency is `shell`), but a hex's own currency may not be listed.

## How markets are loaded

`setting/market.rs` holds `MarketsReader`. `WorldContent` has a `markets: MarketsReader` field, and the `market` section type (folder `Start`, listed after `deposit`) calls `MarketsReader::read` for each entry. It follows `DepositsReader` (see [resources.md](resources.md)): an entry identified by coordinates, which checks its own duplicates and resolves its references in phase 2.

1. **Phase 1, `read`.** Records the header's location, then reads `q` and `r` with the shared `coordinate` helper from `setting/map.rs` and `prices` with `EntryReader::required_number_table` (see [setting-loader.md](setting-loader.md)), all before any early return so no key is falsely reported as unknown. For each price that parsed as a number, `Quantity::new(v)` filtered to `> 0` must succeed, otherwise `key '<good>' in prices of market must be a finite number greater than 0` is reported at the value. Only valid prices are kept, unresolved, as `(Located<String>, Quantity)` in table order. A missing or non-table `prices` gives an empty list (the load fails anyway). An entry with a missing or invalid coordinate is not stored, so it can't cause a misleading duplicate error. A repeat of the same coordinates is reported at the second header as `duplicate market at (0, 0) (first defined at start/a.toml:1:1)` and not stored; the `seen: HashMap<HexCoord, Location>` behind this is used for lookup only.
2. **Phase 2, `resolve(goods, map, sink)`.** Creates one `None` list per placed hex (`MapReader::len`). For each entry, resolves the hex with `MapReader::id` (`no hex at (q, r)` at the header if none) and marks the hex as having a market (`Some(empty)`), even if none of its prices is valid. Each price's good name is resolved through `Definitions::<GoodKind>::resolve` (`unknown good 'tin'` at the key), always, so unknown goods are reported even when the hex is missing. If both resolve, the good is compared with the hex's resolved currency (`MapReader::currency`): a match is reported at the key as `'coin' is the currency at (6, 0), so it can't have a price`; otherwise the price goes into the hex's `BTreeMap`. If the hex's currency itself didn't resolve, the check is skipped (its own error is already reported). TOML forbids repeated keys in one table, so a good can't be priced twice on one hex.
3. **Finish.** `MarketsReader::finish` returns `HexPrices` from the phase-2 lists. `load_setting` calls it before `MapReader::finish` (which consumes the map reader) and stores the result in `StartState::prices`.

`WorldContent::resolve` must run `MapReader::resolve` before `MarketsReader::resolve`, because the currency check needs the hexes' resolved currencies. As for every reference, phase 2 is skipped when a file was rejected or discovery was incomplete, so the unknown-good and priced-currency checks are not made then (see [setting-loader.md](setting-loader.md)).

All `[[market]]` errors and their exact texts:

| Problem | Reported at | Message |
|---|---|---|
| `q` or `r` missing | header | `missing required key 'q' in market` |
| `q` or `r` not an integer | value | `key 'q' in market must be an integer, found float` |
| `q` or `r` outside `i32` | value | `key 'q' in market is out of range` |
| no hex at the coordinates | header | `no hex at (9, 9)` |
| a second market for a hex | header | `duplicate market at (0, 0) (first defined at <location>)` |
| `prices` missing | header | `missing required key 'prices' in market` |
| `prices` not a table | value | `key 'prices' in market must be a table, found integer` |
| a price that isn't a number | value | `key 'grain' in prices of market must be a number, found string` |
| a price that is zero, negative, `inf` or `nan` | value | `key 'grain' in prices of market must be a finite number greater than 0` |
| a good name that matches no good | key | `unknown good 'tin'` |
| the hex's currency is priced | key | `'coin' is the currency at (6, 0), so it can't have a price` |
| any other key | key | `unknown key 'colour' in market` |
| `[[market]]` in `rules/` | header | `section 'market' belongs in start/, not rules/` |

## Using markets

```rust
let (world, _) = World::new(load_setting(path)?);
let market = world.market(hex);                 // Market<'_>
let grain = world.rules().goods.id("grain")?;
match market.price(grain) {
    Some(price) => { /* in units of market.currency() */ }
    None => { /* not traded on this hex */ }
}

let mut stock = world.hex_stockpile(hex).clone(); // any Stockpile works
let received = market.sell(&mut stock, grain, amount)?;
let all_prices = world.hex_prices();              // &HexPrices
```

`Stockpile` is public and `Clone`, but its changing methods are crate-private, so code outside the engine can only trade with a stockpile it obtained from the world (as the tests do with clones of hex stockpiles). Inside the engine, trading happens in the "actors act" part of the turn once Step 06 adds POP trade.

## Not yet

Changing prices during play (and what happens to them when a hex's currency changes), a market stockpile or any limit on volume, prices in another hex's currency or exchange between currencies, rounding or a smallest unit of currency, any actor trading and market activity in the turn report, and showing markets in the dev UI (Stage 02 Step 08). Stage 05 replaces the posted prices and the exchange with live market outcomes; the `[[market]]` section may survive as starting prices or be dropped then.

## The sample setting and tests

- `settings/sample/start/markets.toml` holds 18 markets, one per hex of the map in the map's order, skipping (2, -2), which has none. Every market prices `unskilled_labor` 1, `grain` 2 and `copper` 5 in `silver`; the center (0, 0) also prices `unimproved_grain` 0.5 and `unimproved_copper` 1 (the only hex that trades resource output). So the sample has a hex that trades every non-currency good, hexes that trade some goods, and a hex that trades nothing. The prices need not be consistent with production, since nothing is produced for sale yet. The file's header comment explains the keys.
- `market.rs` unit tests: `HexPrices` order, `get`, `posted`, `market_count`, the empty default, the foreign-hex panic, and the `TradeError` messages.
- `crates/engine/tests/market.rs`: the sample's prices on the center, a typical hex and the edge hex without a market (and `silver` at 1 everywhere, 18 markets); `markets_valid` (prices across files and both table forms, an empty `prices`, a hex without a market, the same result on a second load); every `markets_errors` error at its place; selling, `sell_all` and buying on `turns_valid` (amounts, perish groups of what was added, refusals leaving the stockpile unchanged); the soonest-perishing group sold first; zero amounts; prices unchanged by stepping; and the foreign-hex panic of `World::market`.
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a market that prices goods that don't exist, and only the syntax error is reported.
- Test settings: `markets_valid`, `markets_errors`, `turns_valid/start/markets.toml` ((0, 0) prices `daily` 2 and `stew` 4, (1, 0) `daily` 0.5 and `stew` 3, (2, 0) no market, `gold` the currency), and `resources_one` (one market with empty prices, for the checker's `1 market`). Every other test setting has no markets and loads as before.

## docs/README.md — modify
- **Location:** the document table, new row after the `currency.md` row (added by Step 01)
- **Change:** add

| [market.md](market.md) | The dummy market: each hex's posted prices (`HexPrices`), the `Market` exchange and its `TradeError` refusals, and how `[[market]]` sections are loaded and checked |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row
- **Change:** replace

| [world.md](world.md) | The running `World`, the three-part turn, stockpiles with perish groups and removal, turn reports, and the setting checker's `--turns` option |

## docs/setting-loader.md — modify
- **Location:** intro, the sentence starting "The loader defines four real section types so far"
- **Change:** replace

The loader defines five real section types so far: `[[good]]` and `[[resource]]` in `rules/` (see [goods.md](goods.md) and [resources.md](resources.md)) and `[[hex]]`, `[[deposit]]` and `[[market]]` in `start/` (see [map.md](map.md), [resources.md](resources.md) and [market.md](market.md)); later Steps add more.

## docs/setting-loader.md — modify
- **Location:** "Module structure", the `map.rs` row of the table
- **Change:** replace
- **Supersedes:** Step 01 (currency), the `map.rs` row

| `map.rs` | `MapReader`: reads `[[hex]]` entries, including each hex's `currency`, and builds `HexMap` and `HexCurrencies`. Its `coordinate` helper is shared with deposits and markets. `MapReader::id` resolves coordinates to a `HexId`, `MapReader::len` gives the hex count, and `MapReader::currency` gives a hex's resolved currency after phase 2 |

## docs/setting-loader.md — modify
- **Location:** "Module structure", new row after the `map.rs` row
- **Change:** add

| `market.rs` | `MarketsReader`: reads `[[market]]` entries, resolves their hexes and goods in phase 2, checks that no market prices its hex's currency, and builds `HexPrices` (see [market.md](market.md)) |

## docs/setting-loader.md — modify
- **Location:** "How loading works", step 5, the paragraph starting "`WorldContent::resolve` is where real references are checked"
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

   `WorldContent::resolve` is where real references are checked: it resolves each hex's currency against the goods (`MapReader::resolve`), each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`), and finally each market's coordinates against the map and the goods in its `prices` against the goods (`MarketsReader::resolve`), checking that no market prices its hex's currency. The map must resolve before the markets, because that check needs the hexes' resolved currencies (`MapReader::currency`); otherwise the order does not affect the result, because errors are sorted. A reader's `finish` that depends on phase 2 (`MapReader::finish`, `ResourcesReader::finish`, `MarketsReader::finish`) is only called after a successful load.

## docs/setting-loader.md — modify
- **Location:** "Section types and content", the paragraph starting "`WorldContent` is the real content type"
- **Change:** replace
- **Supersedes:** Step 01 (currency), the sentence starting "`load_setting` builds `Rules { goods, resources }`"

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, a `DepositsReader` and a `MarketsReader`, and returns five section types: `good` and `resource` in `rules/`, `hex`, `deposit` and `market` in `start/`. `load_setting` builds `Rules { goods, resources }` and `StartState { map, currencies, deposits, prices }` from it. `MarketsReader::finish` runs first, because `MapReader::finish` consumes the map reader; `MapReader::finish` returns the map and the currencies together and comes before `DepositsReader::finish`, which needs the hex count. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 1, the `[[hex]]` bullet
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same bullet

   - `[[hex]]` (`StartState::map`) is the worked example for a starting-state section type and for a kind identified by something other than a name. Its reader keeps its own table of what makes an entry unique (coordinates) and checks duplicates itself, instead of using `Definitions`. `[[deposit]]` (`StartState::deposits`) and `[[market]]` (`StartState::prices`) follow it. A `[[hex]]` also carries a reference, its `currency`, which is stored separately in `StartState::currencies` (see [currency.md](currency.md)). `[[market]]` is the example for per-hex state stored one entry per hex by `HexId`, with `None` for hexes that have no entry (see [market.md](market.md)).

## docs/setting-loader.md — modify
- **Location:** "Adding a section type", step 3, the sentence "Resources and deposits are the worked example." (as replaced by Step 01: "Resources, deposits and hex currencies are the worked examples.")
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same sentence

Resources, deposits, hex currencies and markets are the worked examples. Markets also show a check that depends on another reader's phase-2 result (the hex's resolved currency), which fixes the order of the calls in `WorldContent::resolve`.

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first bullet (the list of accessors)
- **Change:** replace

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `required_number_table`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>`, where `Located` carries the value and its `Location`. `required_number` accepts an integer or a float and returns an `f64`; `inf` and `nan` are returned as written (the caller does the range check), an integer beyond `i64` is reported as out of range, and other types are reported as `must be a number, found <type>`.
- `required_number_table(key)` reads a table whose values are numbers (used for a market's `prices`) and returns `Option<Vec<(Located<String>, Option<Located<f64>>)>>`: every key with its location, in the table's order, and its value parsed exactly as `required_number` parses one (both share the private `number_value` helper, which takes the message prefix). A bad value is reported as `key '<k>' in <key> of <label> must be a number, found <type>` or `... is out of range` and returned as `None`, so the caller still sees the key. A missing key gives the usual missing-key error and a non-table value `key '<key>' in <label> must be a table, found <type>`; both return `None`. The nested table's keys are data, not fields: they are not name-checked and not tracked as read or unknown. Both `key = { ... }` and a `[section.key]` sub-table are accepted.

## docs/setting-loader.md — modify
- **Location:** "The sample setting and test settings", both bullets
- **Change:** replace
- **Supersedes:** Step 01 (currency), the first bullet

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (six goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types), `start/map.toml` (19 hexes, each designating `silver` as its currency, see [map.md](map.md) and [currency.md](currency.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), `start/markets.toml` (18 markets posting prices in `silver`, see [market.md](market.md)), plus `README.md` files. It must always load without errors.
- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `markets_valid`, `markets_errors`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the paragraph starting "The success line is"
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>, <markets>`, for example `6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets` or `1 good, 1 hex, 1 resource, 1 deposit, 1 market`. Markets are counted with `HexPrices::market_count`: every `[[market]]` entry, including one with empty `prices`. Currencies are not counted: there is one per hex, not a new kind of thing. The `count(n, singular, plural)` helper in `text.rs` does the wording; later Steps append their own counts. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

## docs/map.md — modify
- **Location:** "How hexes are loaded", the paragraph starting "Duplicates are detected by the reader's own coordinate table"
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

Duplicates are detected by the reader's own coordinate table, not by `Definitions`, which is keyed by names. The only reference in a hex is its currency: `MapReader::resolve` (phase 2) resolves each placed hex's currency against the goods. Other readers query the map reader while resolving their own references: `MapReader::id(coord)` gives the `HexId` a coordinate will have in the finished map (deposits and markets use it), `MapReader::len()` gives the number of placed hexes (markets size their per-hex lists with it), and `MapReader::currency(hex)` gives a placed hex's resolved currency, or `None` before phase 2 or if it didn't resolve (markets use it to reject a priced currency, so `MapReader::resolve` must run first). The `coordinate` helper that reads `q` and `r` is shared by hexes, deposits and markets; its message names the entry's label.

## docs/world.md — modify
- **Location:** "What a World holds", the field table
- **Change:** add a row after the `hex_currencies: HexCurrencies` row (added by Step 01)

| `hex_prices: HexPrices` | the setting's `StartState::prices` | not yet; world state because the live market (Stage 05) changes prices (see [market.md](market.md)) |

## docs/world.md — modify
- **Location:** "What a World holds", the field table, the `hex_stockpiles` row
- **Change:** replace

| `hex_stockpiles: IdVec<HexKind, Stockpile>` | empty at creation | goods enter by emission and leave by perishing (the market can trade with copies of them, but nothing in the turn does yet) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, from "Public accessors are read-only" to the end of the paragraph
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)` and `hex_stockpile(hex)`. `market(hex)` returns the hex's `Market` view (its prices in the hex's currency and the exchange, at the current turn; see [market.md](market.md)); because it borrows the world, the world can't be stepped while a `Market` is alive. `hex_currency`, `market` and `hex_stockpile` panic with `is not from this list` for a hex from another map (a caller bug). The turn neither reads nor changes currencies or prices.

## docs/world.md — modify
- **Location:** "The turn", item 2 of the numbered list
- **Change:** replace

2. **Actors act:** nothing yet. Later Stages add POPs, industries, and markets here. Trades through the dummy market (see [market.md](market.md)) will happen in this part once POP trade arrives (Stage 02 Step 06); nothing calls the exchange during a step yet, so the turn report is unchanged.

## docs/world.md — modify
- **Location:** "Stockpile", from "Changing (`pub(crate)`, engine only):" through the paragraph ending "the world only gives it IDs from its own rules."
- **Change:** replace

Changing (`pub(crate)`, engine only):

- `add(good, amount, at)` merges `amount` into the group `at`; a zero amount changes nothing.
- `remove(good, amount) -> Result<(), Quantity>` takes `amount` out, **soonest-perishing group first**, then the next, so what would spoil first is used first and durable units are kept longest. It is all-or-nothing: if the stockpile holds less than `amount`, it returns `Err(held)` (the total held) and changes nothing; there is no partial removal. A zero amount is `Ok` and changes nothing. A group that reaches zero is dropped, and so is a good left with no groups, so the stockpile stays sparse. Because the check compares against the floating-point total of the groups, a tiny remainder from rounding can be left unremoved when the groups run out; that is accepted.
- `remove_all(good) -> Quantity` removes every unit of `good` and returns the total removed (zero if none was held).
- `remove_perishing(turn)` removes every group at `EndOfTurn(t)` with `t <= turn` (not only `t == turn`, so no group can be stranded), drops goods left with no groups, and returns the totals removed per good.

There is no public way to change a stockpile. Removal is used by the dummy market (see [market.md](market.md)), which is the only way code outside the engine can change one: a caller passes a stockpile it owns (in tests, a clone of a hex stockpile) to `Market::sell`, `sell_all` or `buy`. A `GoodId` does not say which `Goods` list it came from, so a stockpile simply holds the IDs it is given; the world only gives it IDs from its own rules.

## docs/world.md — modify
- **Location:** "Quantity", the whole section body
- **Change:** replace

`Quantity`'s arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result of two finite values never becomes infinite, and `checked_sub`, which returns `None` if the result would be negative and never returns `-0.0`. `Quantity::ONE` is one unit (the price of a hex's currency in its own market). Arithmetic is added only when a system needs it (see [resources.md](resources.md)).

## docs/world.md — modify
- **Location:** "Tests", the unit-tests bullet
- **Change:** replace

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub` and `saturating_mul`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, zero, and `remove_all`), and `world.rs` (including a world forced to `Turn::MAX`, which refuses to step and stays unchanged). Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)).

## docs/world.md — modify
- **Location:** "Tests", the `turns_valid` bullet
- **Change:** replace

- `turns_valid` (`crates/engine/tests/settings/turns_valid/`) has goods `daily` (perishes after 1), `stew` (after 3), and `gold` (durable, every hex's currency); resource types `field`, `meadow`, `pot`, and `mine`; three hexes; six deposits, with (0, 0) holding four, (1, 0) two, and (2, 0) none; and two markets in `start/markets.toml`, on (0, 0) and (1, 0), with (2, 0) having none.

## docs/world.md — modify
- **Location:** "Not yet", the whole paragraph
- **Change:** replace

Ownership of deposits and goods, starting stockpiles in `start/`, deposit stock, depletion and regeneration, any effect of `movable`, per-hex report breakdowns, saving and loading a world, prices that change during play, and any behavior in the "actors act" part of the turn (including trading).

## docs/resources.md — modify
- **Location:** "Types", the `Quantity` bullet under `quantity.rs:`
- **Change:** replace

- `Quantity` wraps an `f64` and is always finite and never negative. `Quantity::new(f64) -> Option<Quantity>` rejects negative, NaN, and infinite values and turns `-0.0` into `0.0`; `get()` returns the `f64`; `ZERO` and `Default` are zero, and `ONE` is one; `Display` prints like `f64` (`4`, `2.5`). It derives `PartialEq` and `PartialOrd` but not `Eq`, `Ord`, or `Hash`. Its arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result stays finite, and `checked_sub`, which returns `None` instead of a negative result. Arithmetic is added only when a system needs it. Every system should use this type for amounts of goods, so the representation can be swapped later (for example to fixed decimal for cross-platform determinism) in one place.

## docs/resources.md — modify
- **Location:** the paragraph starting "References are checked in `WorldContent::resolve`" (after the `DepositsReader` list)
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

References are checked in `WorldContent::resolve`, so they may point into any file: hex currencies and resource types against goods, then deposits against resource types and the map. As for all references, phase 2 is skipped when a file was rejected or discovery was incomplete (see [setting-loader.md](setting-loader.md)). `resources_broken_file` also holds a hex with an unknown currency and a market that prices goods that don't exist, which are likewise not reported. `[[market]]` follows `[[deposit]]` as the pattern for an entry identified by coordinates (see [market.md](market.md)).

## docs/goods.md — modify
- **Location:** "How goods are loaded", last paragraph ("Goods contain no references, ...")
- **Change:** replace
- **Supersedes:** Step 01 (currency), the same paragraph

Goods contain no references, so `GoodsReader` has nothing to resolve in phase 2. Sections that name goods (a resource type's `produces`, a hex's `currency`, the keys of a market's `prices`, and more from Stage 02 onward) resolve them with `Definitions::resolve` against the goods registry, through `GoodsReader::defs()`. A good's `perishes_after` also decides when goods bought from a market, and currency received from one, perish (see [market.md](market.md)).

## docs/dev-ui.md — modify
- **Location:** intro, the parenthesised list of engine items the dev UI uses
- **Change:** replace the list

(`load_setting`, `World` and its `step`, `TurnReport`, `StepError`, `Stockpile`, `Deposits`, `HexPrices`, `Rules`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`)

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `deposit_lines` bullet
- **Change:** replace

- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add`, so the order of floating-point additions matches emission (one addition per deposit).

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet
- **Change:** replace

- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets count (`18 markets` for the sample, `1 market` for `resources_one`, `0 markets` elsewhere). It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules.

## docs/workspace.md — modify
- **Location:** the repository layout block, the engine's `src/` and `tests/` lines
- **Change:** replace the lines from `src/id.rs` through `tests/goods.rs, ...`
- **Supersedes:** Step 01 (currency), the same block

```
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/goods.rs        goods data model (see goods.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, goods.rs, map.rs, market.rs, resources.rs, setting_loader.rs, world.rs
```

## crates/engine/src/setting/map.rs — code doc comment (not a docs/ change)
- **Location:** the doc comment on `pub(crate) fn coordinate`
- **Change:** replace (flagged for the Stage's consolidation; `update-docs` edits only `docs/`, so this belongs in `consolidate.md` or a later build)

The comment still reads "Shared by hexes and deposits; the message names the entry's label." `MarketsReader::read` now also uses it. Suggested text:

```rust
/// Reads one coordinate key as an `i32`. A value outside `i32` is reported at the value.
/// Shared by hexes, deposits and markets; the message names the entry's label.
```
