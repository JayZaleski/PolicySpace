# Step 02: dummy-market — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-02-dummy-market/design.md` — the behavior this spec implements.
- `PROJECT_PLAN/stage-02-populate-the-world/step-01-currency/spec.md` and `docs_changes.md` — how Step 01 added `HexCurrencies` next to the map; this Step follows the same shape for prices.
- `docs/setting-loader.md` — the loader's two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/world.md` — `World`, the turn, `Stockpile` and `PerishesAt`, `Quantity`.
- `docs/resources.md` — `DepositsReader` is the pattern for an entry identified by coordinates with references resolved in phase 2.
- `docs/dev-ui.md` — the checker's success line.
- `docs/workspace.md` — the check command and test layout.
- Code: `crates/engine/src/setting/resources.rs` (`DepositsReader`), `crates/engine/src/setting/map.rs` (`MapReader`, `coordinate`), `crates/engine/src/setting/reader.rs` (`EntryReader`), `crates/engine/src/setting/mod.rs` (`WorldContent`, `load_setting`, `StartState`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/currency.rs`, `crates/engine/src/stockpile.rs`, `crates/engine/src/quantity.rs`, `crates/engine/src/world.rs`, `crates/engine/src/id.rs` (`IdVec`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/world.rs` (test helpers to copy), `crates/engine/tests/resources.rs` (`every_deposit_error_is_reported_at_its_place`), `crates/dev-ui/src/check.rs`, `crates/dev-ui/tests/checker.rs`.

## Approach

- **A `[[market]]` section in `start/`**, read by a new `MarketsReader` (`setting/market.rs`) that follows `DepositsReader`: coordinates read with `coordinate`, duplicates checked by coordinates in phase 1, the hex and the good names resolved in phase 2. Rationale: prices are state and the section is the thing Stage 05 replaces.
- **Prices are stored per hex in a new public type `HexPrices`** (`crates/engine/src/market.rs`), one optional price list per hex by `HexId`, each list a `BTreeMap<GoodId, Quantity>`. `None` means the hex has no `[[market]]` entry; `Some(empty)` means an entry with no prices. Both trade nothing; the difference exists only so the checker can count markets and Step 08 can say "no market". A price is a `Quantity` (the loader guarantees it is finite and greater than zero).
- **The exchange is a per-hex view, `Market<'_>`**, handed out by `World::market(hex)`. It borrows the goods, the hex's currency, its price list and the current turn, and offers `price`, `prices`, `sell`, `sell_all`, and `buy`, each working on a caller-supplied `&mut Stockpile`. Its constructor is crate-private, so Step 06 can build it inside the World from split borrows while mutating POP stockpiles. Rationale: one object is the seam Stage 05 replaces; callers never change.
- **The currency's own price is 1** (`Quantity::ONE`), answered by `Market::price`; it is never in a price list (the loader rejects it) and never tradable (`TradeError::IsCurrency`).
- **`Stockpile` gains crate-private `remove` and `remove_all`**, taking the soonest-perishing group first, all-or-nothing. `add` stays as it is. Integration tests reach removal through the market, trading with a clone of a hex stockpile (`Stockpile` is `Clone` and public).
- **`Quantity` gains `ONE`, `checked_sub` and `saturating_mul`**, the only arithmetic the exchange needs.
- **`EntryReader` gains `required_number_table`** for the `prices` table, sharing the number parsing of `required_number`.
- **The checker's success line gains `, N markets`** (the number of `[[market]]` entries). Every expected line in the dev UI tests changes.
- **The build runs as one.** The loader needs `HexPrices`, and the exchange tests need loaded settings, so no package could be built and tested alone.

## Changes

### New

- `crates/engine/src/market.rs` — `HexPrices`, `Market`, `TradeError`, unit tests.
- `crates/engine/src/setting/market.rs` — `MarketsReader`.
- `crates/engine/tests/market.rs` — integration tests (Test Plan).
- Test settings `crates/engine/tests/settings/markets_valid/` and `markets_errors/` (exact contents below).
- `crates/engine/tests/settings/turns_valid/start/markets.toml`.
- `settings/sample/start/markets.toml`.

### Modified

- `crates/engine/src/lib.rs` — `pub mod market;` and `pub use market::{HexPrices, Market, TradeError};` after `map` (alphabetical). Crate docs: each hex may have a dummy market that posts prices in its currency and exchanges goods against a stockpile (see the `market` module).
- `crates/engine/src/quantity.rs` — `ONE`, `checked_sub`, `saturating_mul`, unit tests.
- `crates/engine/src/stockpile.rs` — `remove`, `remove_all`, unit tests. Module docs: goods now also leave by removal (soonest-perishing first), used by the market.
- `crates/engine/src/setting/reader.rs` — `required_number_table`; factor the number parsing out of `number` so both share it.
- `crates/engine/src/setting/map.rs` — `MapReader::len()` and `MapReader::currency(hex)`.
- `crates/engine/src/setting/mod.rs` — `mod market;`, `StartState.prices`, `WorldContent.markets`, the `market` section type, `resolve` order, `load_setting`. Module docs: phase 2 also resolves each market's hex and goods and checks that no market prices its hex's currency; `MapReader::resolve` must run before `MarketsReader::resolve`.
- `crates/engine/src/world.rs` — field `hex_prices`, accessors `hex_prices()` and `market(hex)`. Module docs: the world also holds each hex's posted prices, unchanged for now; the market view is the seam for Stage 05.
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 markets`, except the sample (`18 markets`) and `resources_one` (`1 market`).
- `crates/engine/tests/resources.rs` — comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a market); `resources_one` tests, if any assert on its section counts, are unchanged (markets are separate).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- Test settings: `resources_one/start/s.toml`, `resources_broken_file/start/s.toml`, `turns_valid` (new file).
- `settings/sample/start/README.md` (if it lists the files) and `settings/sample/setting.toml` description (unchanged unless it enumerates files).

## Interfaces & Data

### `crates/engine/src/quantity.rs`

```rust
/// One unit.
pub const ONE: Quantity;
/// `self - other`, or None if `other > self`. The result is normalized (never -0.0).
pub fn checked_sub(self, other: Quantity) -> Option<Quantity>;
/// `self * other`, clamped at `f64::MAX`.
pub fn saturating_mul(self, other: Quantity) -> Quantity;
```

### `crates/engine/src/stockpile.rs`

```rust
/// Removes `amount` of `good`, taking from the group that perishes soonest first.
/// All-or-nothing: if the stockpile holds less than `amount` of `good`, returns
/// `Err(held)` (the total it holds) and changes nothing. A zero amount is Ok and changes
/// nothing. Groups that reach zero are dropped, and so is a good left with no groups.
pub(crate) fn remove(&mut self, good: GoodId, amount: Quantity) -> Result<(), Quantity>;
/// Removes every unit of `good` and returns the total removed (zero if it held none).
pub(crate) fn remove_all(&mut self, good: GoodId) -> Quantity;
```

`remove` algorithm: if `amount == ZERO` return Ok. Let `held = self.total(good)`; if `held < amount` return `Err(held)`. Walk the good's groups in key order (soonest first) with `remaining = amount`: if `remaining >= group` remove the group and `remaining = remaining.checked_sub(group)`; otherwise set the group to `group.checked_sub(remaining)` (drop it if that is zero) and stop. If the groups run out while `remaining` is still a tiny positive value (floating-point rounding of the total), stop; that is acceptable. Finally drop the good if it has no groups. Never leave a zero group or an empty good behind.

### `crates/engine/src/market.rs`

```rust
//! Module docs: the dummy market. Each hex may post fixed prices in its currency good and
//! exchange goods against any stockpile at those prices. It holds nothing: goods sold to it
//! vanish, goods bought appear from nothing, and currency likewise. A good with no posted
//! price isn't traded on that hex; the hex's currency has price 1 and can't be traded
//! against itself. This is the seam Stage 05 replaces with the live market. Prices are
//! starting state and nothing changes them yet. Knows nothing about files.

use std::collections::BTreeMap;

/// Each hex's posted prices, by `HexId`. None for a hex with no `[[market]]` entry.
#[derive(Debug, Default)]
pub struct HexPrices {
    by_hex: IdVec<HexKind, Option<BTreeMap<GoodId, Quantity>>>,
}

impl HexPrices {
    /// One entry per hex, in hex ID order. Crate-private: only the loader builds this.
    pub(crate) fn from_vec(lists: Vec<Option<BTreeMap<GoodId, Quantity>>>) -> Self;
    /// Number of hexes.
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// Number of hexes that have a market (a `[[market]]` entry, even with no prices).
    pub fn market_count(&self) -> usize;
    /// True if `hex` has a market. Panics (`is not from this list`) on a foreign hex.
    pub fn has_market(&self, hex: HexId) -> bool;
    /// The posted price of `good` on `hex`, or None if none is posted. Knows nothing about
    /// the currency: use `Market::price` for the full answer. Panics on a foreign hex.
    pub fn get(&self, hex: HexId, good: GoodId) -> Option<Quantity>;
    /// Every posted price on `hex` as (good, price), in good ID order; empty if none.
    /// Panics on a foreign hex.
    pub fn posted(&self, hex: HexId) -> impl Iterator<Item = (GoodId, Quantity)> + '_;
    /// The hex's price list, for building a `Market`. Crate-private.
    pub(crate) fn list(&self, hex: HexId) -> Option<&BTreeMap<GoodId, Quantity>>;
}

/// Why a trade was refused. The stockpile is unchanged in every case.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeError {
    /// No price is posted for the good on this hex.
    NotTraded { good: GoodId },
    /// The good is the hex's currency, which isn't traded against itself.
    IsCurrency { good: GoodId },
    /// Selling more of a good than the stockpile holds.
    NotEnoughGoods { good: GoodId, held: Quantity, wanted: Quantity },
    /// Buying costs more currency than the stockpile holds.
    NotEnoughCurrency { currency: GoodId, held: Quantity, needed: Quantity },
}
// Display (ids as `#index`, amounts with Quantity's Display):
//   "good #2 is not traded on this hex"
//   "good #5 is this hex's currency and can't be traded against itself"
//   "not enough of good #2 to sell: holds 2, wants to sell 3"
//   "not enough currency (good #5) to buy: holds 1.5, needs 3"
impl std::error::Error for TradeError {}

/// One hex's market, borrowed from a `World`: its posted prices and the exchange.
#[derive(Debug, Clone, Copy)]
pub struct Market<'a> {
    hex: HexId,
    goods: &'a Goods,
    currency: GoodId,
    posted: Option<&'a BTreeMap<GoodId, Quantity>>,
    turn: Turn,
}

impl<'a> Market<'a> {
    /// Crate-private: `World::market` builds it, and later Steps build it inside the World.
    pub(crate) fn new(hex: HexId, goods: &'a Goods, currency: GoodId,
                      posted: Option<&'a BTreeMap<GoodId, Quantity>>, turn: Turn) -> Self;
    pub fn hex(&self) -> HexId;
    /// The good prices are posted in.
    pub fn currency(&self) -> GoodId;
    /// True if the hex has a market (`[[market]]` entry).
    pub fn has_market(&self) -> bool;
    /// The price of `good` in the currency: the posted price, `Quantity::ONE` for the
    /// currency itself, or None if the good isn't traded here.
    pub fn price(&self, good: GoodId) -> Option<Quantity>;
    /// The posted prices as (good, price), in good ID order. The currency isn't listed.
    pub fn prices(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_;
    /// Sells `amount` of `good` from `stockpile`: removes the good and adds
    /// `amount * price` of the currency. Returns the currency received.
    pub fn sell(&self, stockpile: &mut Stockpile, good: GoodId, amount: Quantity)
        -> Result<Quantity, TradeError>;
    /// Sells everything of `good` the stockpile holds. Returns (amount sold, currency
    /// received); (0, 0) if it held none. Same refusals as `sell` except NotEnoughGoods.
    pub fn sell_all(&self, stockpile: &mut Stockpile, good: GoodId)
        -> Result<(Quantity, Quantity), TradeError>;
    /// Buys `amount` of `good` into `stockpile`: removes `amount * price` of the currency
    /// and adds the good with `PerishesAt::for_good(good.perishes, turn)`. Returns the
    /// currency paid.
    pub fn buy(&self, stockpile: &mut Stockpile, good: GoodId, amount: Quantity)
        -> Result<Quantity, TradeError>;
}
```

Rules shared by `sell`, `sell_all`, `buy`, checked in this order: `good == currency` → `IsCurrency`; no posted price → `NotTraded`. Then `sell`: `held = stockpile.total(good)`; if `held < amount` → `NotEnoughGoods`; `stockpile.remove(good, amount)` (its `Err` can't happen after the check; treat it as `NotEnoughGoods` anyway, don't `unwrap`); `received = amount.saturating_mul(price)`; `stockpile.add(currency, received, PerishesAt::for_good(goods.get(currency).perishes, turn))`. `sell_all`: `sold = stockpile.remove_all(good)`, then the same add. `buy`: `needed = amount.saturating_mul(price)`; `held = stockpile.total(currency)`; if `held < needed` → `NotEnoughCurrency`; `stockpile.remove(currency, needed)`; `stockpile.add(good, amount, PerishesAt::for_good(goods.get(good).perishes, turn))`. A zero amount passes every check and changes nothing (`add` and `remove` ignore zero); it returns `Ok(ZERO)`. `price` returns `Some(ONE)` for the currency **before** looking at the list.

### `crates/engine/src/world.rs`

```rust
/// One per hex, by `HexId`. Nothing changes it yet.
hex_prices: HexPrices,

/// Every hex's posted prices.
pub fn hex_prices(&self) -> &HexPrices;
/// The market of `hex`: its prices in the hex's currency and the exchange, at the current
/// turn. Panics (`is not from this list`) if `hex` isn't from this world's map.
pub fn market(&self, hex: HexId) -> Market<'_>;
```

`World::new` moves `start.prices` into the field. `emit`, `perish`, `step`, `TurnReport` unchanged.

### `EntryReader::required_number_table` (`crates/engine/src/setting/reader.rs`)

```rust
/// Reads `key` as a table whose values are numbers. Returns every (key, value) pair in the
/// table's order, each key with its location. A value that isn't a number or is out of
/// range is reported (`key '<k>' in <key> of <label> must be a number, found <type>`,
/// `key '<k>' in <key> of <label> is out of range`) and returned as None, so the caller
/// still sees the key. A missing key reports the usual missing-key error; a value that
/// isn't a table reports `key '<key>' in <label> must be a table, found <type>`. Both
/// return None.
pub fn required_number_table(&mut self, key: &str)
    -> Option<Vec<(Located<String>, Option<Located<f64>>)>>;
```

Values go through the same parsing as `required_number` (integer or float; `inf`/`nan` returned as written; integer beyond `i64` is out of range). Refactor `number` into a helper that takes the message prefix (`key 'output' in resource` vs `key 'grain' in prices of market`) and a `&Spanned<DeValue>`, and call it from both. Keys of the nested table are **not** name-checked and are not tracked as read/unknown (the table's keys are data, not fields). `toml::de::DeValue::as_table()` gives the nested `DeTable`; its keys are `Spanned`, so `file.location_of(k.span())` locates them.

### `MapReader` additions (`crates/engine/src/setting/map.rs`)

```rust
/// Number of placed hexes (the hex count of the finished map).
pub fn len(&self) -> usize;
/// The resolved currency of a placed hex: None before phase 2, or if it didn't resolve.
pub fn currency(&self, hex: HexId) -> Option<GoodId>;
```

### `MarketsReader` (`crates/engine/src/setting/market.rs`)

```rust
//! Reading `[[market]]` sections. Domain types in `crate::market`. A market is identified
//! by its hex, so the reader checks its own duplicates (like deposits); its hex and the
//! goods in `prices` are resolved in phase 2, where it also checks that the hex's currency
//! isn't priced.

struct MarketEntry {
    at: Location,                                 // the [[market]] header
    coord: HexCoord,
    prices: Vec<(Located<String>, Quantity)>,     // only the valid prices, in table order
}

#[derive(Default)]
pub(crate) struct MarketsReader {
    entries: Vec<MarketEntry>,
    seen: HashMap<HexCoord, Location>,            // lookup only
    lists: Vec<Option<BTreeMap<GoodId, Quantity>>>, // filled in phase 2, by hex index
}

impl MarketsReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, map: &MapReader, sink: &mut ErrorSink);
    /// Only after a successful load.
    pub fn finish(self) -> HexPrices;
}
```

`read`: take `at`; read `q`, `r` (`coordinate`) and `prices` (`required_number_table`) before any early return. For each pair with a `Some` value: accept `Quantity::new(v).filter(|q| q.get() > 0.0)`, otherwise report `key '<k>' in prices of market must be a finite number greater than 0` at the value. If `q` or `r` is `None`, return (not stored, as deposits do). A missing or non-table `prices` gives an empty list (the load fails anyway). Duplicate coordinates: `duplicate market at (q, r) (first defined at <loc>)` at the header, not stored. Otherwise record in `seen` and push.

`resolve`: `self.lists = vec![None; map.len()]`. For each entry: `hex = map.id(coord)`, reporting `no hex at (q, r)` at the header if None. For each `(name, price)`: `id = goods.resolve(name, sink)` (always, even without a hex, so unknown goods are reported). If both `hex` and `id` exist: if `map.currency(hex) == Some(id)`, report `'<name>' is the currency at (q, r), so it can't have a price` at `name.at`; else insert into the hex's list (create `Some(BTreeMap)` first; an entry with no valid prices still creates `Some(empty)`). The hex's currency is `None` if it didn't resolve; then skip the check (its own error is already reported). TOML forbids duplicate keys in one table, so a good can't appear twice in one list.

`finish`: `HexPrices::from_vec(self.lists)`.

### `setting/mod.rs`

- `StartState` gains `pub prices: HexPrices` (doc: each hex's posted prices, by hex ID; see `crate::market`).
- `WorldContent` gains `pub markets: MarketsReader`; `section_types` gains `SectionType { header: "market", folder: Folder::Start, read: |c, r| c.markets.read(r) }` after `deposit`.
- `resolve`: `map.resolve` first (it fills the currency IDs the market check needs), then resources, deposits, and `self.markets.resolve(self.goods.defs(), &self.map, sink)`.
- `load_setting`: `let prices = content.markets.finish();` before `content.map.finish()` consumes the map reader (markets don't need it after `resolve`), and `StartState { map, currencies, deposits, prices }`.

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.start.prices.market_count(), "market", "markets")`. The sample line becomes:
`Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets`. The other expected lines in `checker.rs` end in `, 0 markets`, except `resources_one`: `..., 1 resource, 1 deposit, 1 market`.

### Setting file format

A `[[market]]` in `start/` has three required keys: `q`, `r` (integers, the coordinates of a hex), and `prices`, a table whose keys are good names and whose values are numbers greater than zero. Both `prices = { grain = 2 }` and a `[market.prices]` sub-table are accepted (same TOML data). Any other key is unknown. At most one market per hex. A hex needs no market.

### Test setting contents

Write these files exactly; line numbers matter for the expected errors.

**`markets_valid/`** — `setting.toml`: name `Markets Test`, `format_version = 1`. `rules/goods.toml`: goods `coin`, `shell`, `grain`, `fish` in that order (name only). `start/hexes.toml`: hexes (0, 0) `coin`, (1, 0) `shell`, (2, 0) `coin`, (3, 0) `coin`. `start/a.toml`:
```toml
[[market]]
q = 0
r = 0
prices = { grain = 2, fish = 1.5 }
```
`start/a/z.toml` (sub-table form; `coin` is a good but not this hex's currency, so it may be priced):
```toml
[[market]]
q = 1
r = 0

[market.prices]
grain = 3
coin = 0.25
```
`start/b.toml`:
```toml
[[market]]
q = 2
r = 0
prices = {}
```
(3, 0) has no market. Expected: hex 0 posts `[(grain, 2), (fish, 1.5)]`, hex 1 `[(coin, 0.25), (grain, 3)]`, hex 2 `[]`, hex 3 none; `market_count() == 3`; `has_market` true, true, true, false.

**`markets_errors/`** — `setting.toml`: name `Markets Errors`, `format_version = 1`. `rules/goods.toml`: goods `coin`, `grain`. `rules/m.toml`:
```toml
[[market]]
q = 0
r = 0
prices = {}
```
`start/hexes.toml`: hexes (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (1, 1), all `currency = "coin"`. `start/a.toml` (72 lines):
```toml
[[market]]
q = 0
r = 0
prices = { grain = 2 }

[[market]]
r = 0
prices = { grain = 2 }

[[market]]
q = 0
prices = { grain = 2 }

[[market]]
q = 1.5
r = 0
prices = { grain = 2 }

[[market]]
q = 2147483648
r = 0
prices = { grain = 2 }

[[market]]
q = 9
r = 9
prices = { grain = 2 }

[[market]]
q = 1
r = 0
prices = { grain = "2" }

[[market]]
q = 2
r = 0
prices = { grain = 0 }

[[market]]
q = 3
r = 0
prices = { grain = -1 }

[[market]]
q = 4
r = 0
prices = { grain = inf }

[[market]]
q = 5
r = 0
prices = 7

[[market]]
q = 6
r = 0
prices = { coin = 1 }

[[market]]
q = 7
r = 0
prices = { tin = 3 }

[[market]]
q = 8
r = 0

[[market]]
q = 1
r = 1
prices = { grain = 2, tin = 1, coin = 2 }
colour = "red"
```
`start/b.toml`:
```toml
[[market]]
q = 0
r = 0
prices = {}
```
Expected error lines:
```
rules/m.toml:1:3: section 'market' belongs in start/, not rules/
start/a.toml:6:1: missing required key 'q' in market
start/a.toml:10:1: missing required key 'r' in market
start/a.toml:15:5: key 'q' in market must be an integer, found float
start/a.toml:20:5: key 'q' in market is out of range
start/a.toml:24:1: no hex at (9, 9)
start/a.toml:32:20: key 'grain' in prices of market must be a number, found string
start/a.toml:37:20: key 'grain' in prices of market must be a finite number greater than 0
start/a.toml:42:20: key 'grain' in prices of market must be a finite number greater than 0
start/a.toml:47:20: key 'grain' in prices of market must be a finite number greater than 0
start/a.toml:52:10: key 'prices' in market must be a table, found integer
start/a.toml:57:12: 'coin' is the currency at (6, 0), so it can't have a price
start/a.toml:62:12: unknown good 'tin'
start/a.toml:64:1: missing required key 'prices' in market
start/a.toml:71:23: unknown good 'tin'
start/a.toml:71:32: 'coin' is the currency at (1, 1), so it can't have a price
start/a.toml:72:1: unknown key 'colour' in market
start/b.toml:1:1: duplicate market at (0, 0) (first defined at start/a.toml:1:1)
```
If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader` message forms are fixed. The columns of inline-table keys and values come from the TOML spans; if `toml` reports a key's span differently than assumed here (column of the key's first character), adjust the columns only.

**`turns_valid/start/markets.toml`** (new; `gold` is the currency):
```toml
[[market]]
q = 0
r = 0
prices = { daily = 2, stew = 4 }

[[market]]
q = 1
r = 0
prices = { daily = 0.5, stew = 3 }
```
(2, 0) has no market.

**`resources_one/start/s.toml`**: append a `[[market]]` at (0, 0) with `prices = {}` (`ore` is its currency, so nothing can be priced). The checker says `1 market`.

**`resources_broken_file/start/s.toml`**: append a `[[market]]` at (0, 0) with `prices = { tin = 1, nothing = 2 }`. The expected errors stay exactly the one syntax error.

**Sample setting** — `settings/sample/start/markets.toml`, with a header comment in the style of `deposits.toml` explaining the keys, that prices are in the hex's currency (`silver`), that a good without a price isn't traded on that hex, that a hex may have no market, and that the currency can't be priced. Then 18 `[[market]]` entries, one per hex of `map.toml` in the map's order, **skipping (2, -2)** (note the skip in a comment). Every entry prices `unskilled_labor = 1`, `grain = 2`, `copper = 5`; the center (0, 0) also prices `unimproved_grain = 0.5` and `unimproved_copper = 1` (comment: the only hex that trades resource output). `settings/sample/start/README.md`: add the file if it lists them.

## Tasks

1. `quantity.rs`: `ONE`, `checked_sub`, `saturating_mul`, unit tests.
2. `stockpile.rs`: `remove`, `remove_all`, unit tests; module docs.
3. `market.rs`: `HexPrices`, `TradeError`, `Market`; unit tests for `HexPrices` (order, `get`, `posted`, `market_count`, `default` empty, foreign-hex panic) and for `TradeError` Display. Export from `lib.rs`.
4. `reader.rs`: `required_number_table` and the shared number parsing. Unit test it with the pattern of `resources.rs`'s `run` helper (a table value, a non-table, a string value, an out-of-range integer, a missing key).
5. `map.rs`: `len`, `currency`. `setting/market.rs`: `MarketsReader`. `setting/mod.rs`: wiring and docs.
6. `world.rs`: `hex_prices`, `market`, docs.
7. `check.rs` success line; `checker.rs` expectations.
8. Test settings (`markets_valid`, `markets_errors`, `turns_valid`, `resources_one`, `resources_broken_file`), `tests/settings/README.md`, the sample's `markets.toml`.
9. `crates/engine/tests/market.rs` (Test Plan); the comment in `tests/resources.rs`.
10. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the new success line and unchanged turn lines.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`).

**Unit tests.** `quantity.rs`: `checked_sub` normal, equal (zero, positive sign), larger subtrahend → None; `saturating_mul` normal and clamped at `f64::MAX`. `stockpile.rs` (**AC "removing from a stockpile"**): removing part of the soonest group leaves the rest and later groups; removing exactly a group drops it; removing across two groups empties the first and reduces the second; removing the whole total drops the good; removing more than held returns `Err(held)` and the stockpile equals its clone from before; zero is `Ok` and unchanged; `remove_all` returns the total and drops the good, and returns zero for a good not held. `market.rs`: `HexPrices` as listed in Task 3.

**`crates/engine/tests/market.rs`** (`mod common;`; copy the `world_of`, `hex`, `good`, `end` helpers from `tests/world.rs`). Goods in `turns_valid`: `daily` (after 1) = ID 0, `stew` (after 3) = ID 1, `gold` (never) = ID 2. At turn 0, hex (1, 0) holds `daily` 2 at `end(0)` and `gold` 1.5 at `Never`; hex (0, 0) holds `daily` 4.5 and `stew` 5 at `end(2)`. Tests trade with `world.hex_stockpile(h).clone()`.
- **AC "sample prices"**: load the sample into a World. Center (0, 0): `price` of all five non-currency goods is `Some` with the values above; `has_market`. Hex (1, 0): labor 1, grain 2, copper 5, unimproved grain and copper `None`. Hex (2, -2): `has_market()` false, every non-currency good `None`. Every hex: `price(silver) == Some(ONE)`; `hex_prices().market_count() == 18`.
- **AC "test setting with markets across files, empty prices accepted"**: load `markets_valid`; `posted` per hex and `market_count` as listed above; loading twice gives the same lists.
- **AC "each error case"**: `failures_of("markets_errors")` equals the expected list (put this in `tests/market.rs`, following `every_deposit_error_is_reported_at_its_place`).
- **AC "checks skipped when a file is broken"**: the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing; add the comment.
- **AC "selling"**: at (1, 0) sell 1.5 `daily` → `Ok(0.75)`; stockpile: `daily` groups `[(end(0), 0.5)]`, `gold` total 2.25 in one `Never` group. Sell 3 `daily` → `Err(NotEnoughGoods { held: 2, wanted: 3 })`, stockpile equals its clone. `sell_all(daily)` on a fresh clone → `Ok((2, 1))`, `daily` total zero and `groups` empty, `gold` 2.5.
- **AC "buying"**: at (1, 0) buy 0.5 `stew` → `Ok(1.5)`; `gold` gone (total zero, not in `iter`), `stew` groups `[(end(2), 0.5)]`. Buy 1 `stew` on a fresh clone → `Err(NotEnoughCurrency { currency: gold, held: 1.5, needed: 3 })`, unchanged. After one `step`, buy 1 `daily` at (0, 0) with a stockpile that holds gold (clone (1, 0)'s at turn 1, which holds 3 gold): `daily` group at `end(1)`.
- **AC "refusals"**: at (2, 0) `sell`, `sell_all` and `buy` of `daily` → `NotTraded`; at (0, 0) `sell`/`buy` of `gold` → `IsCurrency`; stockpile unchanged each time. `price(daily)` at (2, 0) is `None`, `price(gold)` is `Some(ONE)`.
- **Soonest group first through the market**: step the `turns_valid` world once; clone (0, 0)'s stockpile (`stew` groups `[(end(2), 5), (end(3), 5)]`); sell 7 `stew` → `stew` groups `[(end(3), 3)]`.
- **Zero amounts**: `sell(daily, ZERO)` and `buy(stew, ZERO)` at (1, 0) → `Ok(ZERO)`, unchanged.
- **AC "stepping doesn't change prices"**: step the sample world 3 times; every hex's `posted` list equals the list before stepping. `market(hex)` with a hex from another world's map panics with `is not from this list`.
- `TradeError` Display strings as listed.

**Dev UI**: `checker.rs` passes with the updated lines, including `1 market` for `resources_one`. No new dev UI tests.

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); `BTreeMap` for ordered data; doc comments on every public item (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`.
- `Stockpile::add`, `remove`, `remove_all` stay `pub(crate)`. `IdVec` stays `pub(crate)`.
- Do not change `HexMap`, `HexCurrencies`, the turn, `TurnReport`, emission or perishing, the dev UI window, or the checker's output beyond the markets count.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: changing prices during play, a market stockpile, any actor trading, starting stockpiles, rounding, showing markets in the dev UI window.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line, and every Acceptance Criterion in `design.md` is met.
