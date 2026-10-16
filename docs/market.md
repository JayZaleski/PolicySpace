# The dummy market

Source: `crates/engine/src/market.rs` (posted prices and the exchange) and `crates/engine/src/setting/market.rs` (reading `[[market]]` sections). The public types are re-exported from the engine crate root: `HexPrices`, `Market`, `TradeError`.

Each hex may have a **dummy market**: a stub that posts fixed prices in the hex's currency good (see [currency.md](currency.md)) and buys or sells any amount at those prices. It holds nothing of its own. Goods sold to it vanish, goods bought from it appear from nothing, and currency is created and destroyed the same way. There is no inventory and no limit on how much it trades.

The dummy market is the **seam** that Stage 05 replaces with the live market. Callers ask a `Market` for prices and trade through it; when the live market arrives, what stands behind `Market` changes, not the callers. `Market` implements the `PriceList` trait (see [Market as a price list](#market-as-a-price-list)), so Stone-Geary demand (see [demand.md](demand.md)) can be computed against it. The trade round (see [trade.md](trade.md)) is the exchange's user: every turn, on every hex, it computes each POP's demand against the market and settles the POP's sales with `sell_down_to` and its purchases with `buy` and `buy_for`. `market.rs` depends on `demand` (for the trait), `goods`, `id`, `map`, `quantity`, `stockpile` and `turn`, and knows nothing about files.

## Which goods are traded

- A good is traded on a hex **only if the hex's market posts a price for it**. A good with no posted price is "not traded here": the market neither buys nor sells it. So a good may exist in the rules before anything on a hex produces or wants it.
- A hex with no `[[market]]` entry is valid and trades nothing. The trade round treats it the same as a hex where every good is unpriced.
- The hex's **currency** has price 1 by definition (`Quantity::ONE`). It is never stored in a price list (the loader rejects it) and can't be traded against itself.
- A price is a `Quantity` that is finite and greater than zero (the loader guarantees it). It is the number of units of the hex's currency that one unit of the good costs.
- Prices are starting state, not rules, because the live market will change them. Nothing changes them during play yet; stepping the world leaves them as loaded.

## Data model

### `HexPrices`

```rust
pub struct HexPrices { by_hex: IdVec<HexKind, Option<BTreeMap<GoodId, Quantity>>> }
```

One optional price list per hex, indexed by `HexId`, each list in good ID order. `None` means the hex has no `[[market]]` entry; `Some(empty)` means an entry with no prices. Both trade nothing; the difference exists so the checker can count markets and the dev UI can say "No market." or "No prices posted." (see [dev-ui.md](dev-ui.md)).

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
| `sell_down_to(&mut stockpile, good, keep) -> Result<(Quantity, Quantity), TradeError>` | Sells everything of `good` above `keep`, so that `keep` remains; returns (amount sold, currency received), `(0, 0)` if the stockpile holds `keep` or less. With `keep` of `Quantity::ZERO` it sells everything |
| `buy(&mut stockpile, good, amount) -> Result<Quantity, TradeError>` | Buys `amount` of `good`; returns the currency paid |
| `buy_for(&mut stockpile, good, spend) -> Result<Quantity, TradeError>` | Spends exactly `spend` of the currency on `good`; returns the amount bought (`spend / price`) |

`Market::new` is crate-private. `World::market` builds one from shared borrows of the world, so the world can't be stepped while a `Market` is alive, and a `Market` always trades at the world's current turn. `World::trade` (the trade round, see [trade.md](trade.md)) builds a `Market` from split borrows of the world's fields (`rules.goods`, `hex_currencies`, `hex_prices`, `turn`) while it moves POP stockpiles out of another field and back.

### Market as a price list

`Market<'_>` implements `PriceList` from the `demand` module by forwarding to its inherent methods: `PriceList::currency` is `Market::currency`, and `PriceList::price` is `Market::price`, with the same three-way answer. The inherent methods stay, so existing callers are unaffected; inside the impl they are called by their qualified path (`Market::price(self, good)`) so the trait methods don't call themselves. A replacement market should implement `PriceList` the same way, keeping every price greater than 0.

## The exchange

The market exchanges goods against a **caller-supplied stockpile**; the same stockpile pays and receives.

- **Selling** (`sell`) removes `amount` of the good from the stockpile (soonest-perishing group first, see [world.md](world.md)) and adds `amount × price` of the currency.
- **Selling down to an amount** (`sell_down_to`) removes everything above `keep` (`Stockpile::retain`, soonest-perishing group first) and adds `sold × price` of the currency. When the kept amount lies in a single perish group, exactly `keep` remains; selling `held − keep` with `sell` could leave a remainder one floating-point unit off, which would then show up in reports as a sliver such as `2.8e-14`. **Selling everything** is `sell_down_to(stockpile, good, Quantity::ZERO)`: it removes every unit of the good and returns the total sold and the currency received, or `(0, 0)` for a good not held. The trade round uses `sell_down_to` for every sale; there is no separate "sell all" method.
- **Buying** (`buy`) removes `amount × price` of the currency (soonest-perishing group first) and adds `amount` of the good.
- **Buying for an amount of currency** (`buy_for`) removes exactly `spend` of the currency and adds `spend / price` of the good. The trade round uses it to cap a purchase at the currency held: buying `held / price` with `buy` could still be refused when `(held / price) × price` rounds above `held`. A zero spend buys nothing.

Anything added enters the stockpile **as if it appeared in the current turn**: its group is `PerishesAt::for_good(perishes, turn)`. So a perishable good bought in turn T perishes at the end of turn T + `perishes_after` − 1, and currency received from a sale perishes the same way if the currency is a perishable good.

The amount of currency is `amount.saturating_mul(price)`: the floating-point product, clamped at `f64::MAX`. The engine does no rounding and has no smallest unit of currency.

### Refusals

Every trade is **all-or-nothing**: a refused trade changes nothing. The checks run in this order, and each refusal is a `TradeError`:

| Variant | When |
|---|---|
| `IsCurrency { good }` | `good` is the hex's currency |
| `NotTraded { good }` | no price is posted for `good` on this hex |
| `NotEnoughGoods { good, held, wanted }` | `sell` of more than the stockpile holds |
| `NotEnoughCurrency { currency, held, needed }` | `buy` costing more currency than the stockpile holds, or `buy_for` with `spend` (reported as `needed`) above the currency held |

`sell_down_to` can only fail with the first two. A zero amount (or zero spend) of a traded good passes every check, changes nothing, and returns zero (a zero amount of an untraded good or of the currency is still refused). `TradeError` implements `Display` and `std::error::Error`; IDs are written as `#index` because the error has no access to names:

```
good #2 is not traded on this hex
good #5 is this hex's currency and can't be traded against itself
not enough of good #2 to sell: holds 2, wants to sell 3
not enough currency (good #5) to buy: holds 1.5, needs 3
```

The internal `remove` that `sell`, `buy` and `buy_for` call can't fail after the shortfall check; if it ever did, the trade is refused with the same variant rather than panicking.

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

1. **Phase 1, `read`.** Records the header's location, then reads `q` and `r` with the shared `coordinate` helper from `setting/map.rs` and `prices` with `EntryReader::required_number_table` (see [setting-loader.md](setting-loader.md)), all before any early return so no key is falsely reported as unknown. Each price that parsed as a number goes through `EntryReader::positive_quantity` with the prefix `key '<good>' in prices of market`, which reports `key '<good>' in prices of market must be a finite number greater than 0` at the value if it is zero, negative, `inf` or `nan`. Only valid prices are kept, unresolved, as `(Located<String>, Quantity)` in table order. A missing or non-table `prices` gives an empty list (the load fails anyway). An entry with a missing or invalid coordinate is not stored, so it can't cause a misleading duplicate error. A repeat of the same coordinates is reported at the second header as `duplicate market at (0, 0) (first defined at start/a.toml:1:1)` and not stored; the `seen: HashMap<HexCoord, Location>` behind this is used for lookup only.
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
let (sold, received) = market.sell_down_to(&mut stock, grain, Quantity::ZERO)?; // sell everything
let all_prices = world.hex_prices();              // &HexPrices
```

`Stockpile` is public and `Clone`, but its changing methods are crate-private, so code outside the engine can only trade with a stockpile it obtained from the world (as the tests do with clones of hex stockpiles). Inside the engine, POPs trade in the "actors act" part of every turn (see [trade.md](trade.md)); hex stockpiles are never traded during the turn.

## Not yet

Changing prices during play (and what happens to them when a hex's currency changes), a market stockpile, rationing or any limit on volume, prices in another hex's currency or exchange between currencies, rounding or a smallest unit of currency, and actors other than POPs trading. Stage 05 replaces the posted prices and the exchange with live market outcomes inside the trade round; the `[[market]]` section may survive as starting prices or be dropped then.

## The sample setting and tests

- `settings/sample/start/markets.toml` holds 18 markets, one per hex of the map in the map's order, skipping (2, -2), which has none. Every market prices `unskilled_labor` 1, `grain` 2 and `copper` 5 in `silver`; the center (0, 0) also prices `unimproved_grain` 0.5 and `unimproved_copper` 1 (the only hex that trades resource output). So the sample has a hex that trades every non-currency good, hexes that trade some goods, and a hex that trades nothing. The prices need not be consistent with production, since nothing is produced for sale yet. The file's header comment explains the keys.
- `market.rs` unit tests: `HexPrices` order, `get`, `posted`, `market_count`, the empty default, the foreign-hex panics, and the `TradeError` messages. They use the `good`, `hex` and `q` helpers from `crates/engine/src/test_support.rs`.
- `crates/engine/tests/market.rs`: the sample's prices on the center, a typical hex and the edge hex without a market (and `silver` at 1 everywhere, 18 markets); `markets_valid` (prices across files and both table forms, an empty `prices`, a hex without a market, the same result on a second load); every `markets_errors` error at its place; selling, selling everything with `sell_down_to(.., Quantity::ZERO)` and buying on `turns_valid` (amounts, perish groups of what was added, refusals leaving the stockpile unchanged); `sell_down_to` (leaving exactly the kept amount in its original group, selling nothing at or below it, a zero `keep` selling everything and raising the currency by `held × price`, refusing an untraded good and the currency); `buy_for` (spending exactly, the perish group of the good bought, refusing a shortfall, a zero spend); the soonest-perishing group sold first; zero amounts; prices unchanged by stepping; and the foreign-hex panic of `World::market`. The trade round is tested in `crates/engine/tests/trade.rs` (see [trade.md](trade.md)).
- `crates/engine/tests/resources.rs`: `resources_broken_file` also holds a market that prices goods that don't exist, and only the syntax error is reported.
- Test settings: `markets_valid`, `markets_errors`, `turns_valid/start/markets.toml` ((0, 0) prices `daily` 2 and `stew` 4, (1, 0) `daily` 0.5 and `stew` 3, (2, 0) no market, `gold` the currency), `resources_one` (one market with empty prices, for the checker's `1 market`), and the POP settings `pops_trade` and `pops_growth` (see [trade.md](trade.md) and [growth.md](growth.md)).
