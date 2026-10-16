# The world and the turn

A loaded setting only describes a world: its rules and what turn 0 looks like. A `World` (`crates/engine/src/world.rs`) is that setting running. It adds the clock, the POPs' holdings and the goods that exist, and it can be stepped turn by turn. The supporting types are `Turn` (`turn.rs`) and `Stockpile` (`stockpile.rs`). None of these modules knows about files; `world` depends only on the public setting types `LoadedSetting`, `Manifest`, and `Rules`.

## What a World holds

`World::new(setting: LoadedSetting)` consumes the loaded setting and takes it apart:

| Field | Source | Changes? |
|---|---|---|
| `manifest`, `rules` | the setting | never (goods, resource types and POP types are fixed for the whole game) |
| `map: HexMap`, `deposits: Deposits` | the setting's `StartState` | not yet; they are world state because later Stages change them (deposit ownership in Stage 04) |
| `turn: Turn` | starts at `Turn::ZERO` | advances by one per step |
| `hex_currencies: HexCurrencies` | the setting's `StartState::currencies` | not yet; world state because a later Stage changes a hex's currency (see [currency.md](currency.md)) |
| `hex_prices: HexPrices` | the setting's `StartState::prices` | not yet; world state because the live market (Stage 05) changes prices (see [market.md](market.md)) |
| `hex_stockpiles: IdVec<HexKind, Stockpile>` | empty at creation | goods enter by emission and leave by perishing; nothing in the turn trades them |
| `pops: Pops` | the setting's `StartState::pops` | every turn: each POP's size is set by growth after the trade round (see [growth.md](growth.md)); hexes and types never change |
| `pop_stockpiles: IdVec<PopKind, Stockpile>` | one empty stockpile per POP at creation | goods enter by endowment and purchase, and leave by sale, consumption and perishing (see [trade.md](trade.md)) |
| `pop_outcomes: IdVec<PopKind, PopOutcome>` | built by turn 0's trade round | rebuilt every turn by the trade round: each POP's demand, what it sold, bought, consumed and carried over, its subsistence index and growth rate, and its starting size (see [trade.md](trade.md)) |

The world owns the setting, so callers (the dev UI) have no lifetimes to manage. There is one stockpile of **unowned goods** per hex, in an `IdVec` indexed by `HexId`, and one stockpile per POP, the goods that POP owns, in an `IdVec` indexed by `PopId` (see [ids.md](ids.md)). Both use the same `Stockpile` type. Deposits have no owners yet, so their output still goes to the hex stockpiles. Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, `hex_currencies()`, `hex_currency(hex)`, `hex_prices()`, `market(hex)`, `hex_stockpile(hex)`, `pops()`, `pop_stockpile(pop)` and `pop_outcome(pop)`. `market(hex)` returns the hex's `Market` view (its prices in the hex's currency and the exchange, at the current turn; see [market.md](market.md)); because it borrows the world, the world can't be stepped while a `Market` is alive. `pop_outcome(pop)` returns the POP's `PopOutcome` from the current turn's trade round (see [trade.md](trade.md)). `hex_currency`, `market` and `hex_stockpile` panic with `is not from this list` for a hex from another map, and `pop_stockpile` and `pop_outcome` for a POP from another world's `Pops` (a caller bug). The turn reads prices through the trade round but changes neither currencies nor prices.

## The turn

A turn has three parts:

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good into its hex's stockpile. Then every POP receives its type's endowment times its size into its own stockpile (see [Endowment](#endowment)).
2. **Actors act:** the trade round and consumption, then growth (see [Trade round](#trade-round), [Growth](#growth), [trade.md](trade.md) and [growth.md](growth.md)). On every hex, each POP submits its parameters and holdings, the hex's market computes their demand and settles every sale before any purchase, and each POP consumes its utility goods. Then, once every hex has traded, every POP grows or shrinks by the rate its outcome records. Later Stages add industries and the live market here.
3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile and then from every POP stockpile.

Two calls drive the clock, and both run a turn through one private function:

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 with empty stockpiles and runs `run_turn` for turn 0, so a fresh world's deposits have emitted and its POPs have already been endowed, traded, consumed and grown. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing (sizes and outcomes included). Then it runs the end of the current turn (`perish`: hexes in ID order, then POPs in ID order), advances the clock, and runs `run_turn` for the new turn with what perished.
- `World::run_turn(&mut self, perished) -> TurnReport` (private) calls `emit`, `endow`, `trade` and `grow` in that order and builds the report from their results: `emitted` and `endowed` from the first two, `sold`, `bought` and `consumed` from the private `TradeTotals` that `trade` returns, `population` from `grow`, and `turn` from the clock.

So between calls the world is always **after part 2**: this turn's output is in the hex stockpiles, this turn's trade is done, each POP's stockpile holds what the round left, each POP's size is its size after growth, and nothing has perished yet this turn. This is the state that tests, the checker, and the dev UI observe. If observation happened after the end of the turn, goods with `perishes_after = 1` would always look empty.

`StepError` has one variant, `ClockAtMaximum` (the clock is at `Turn::MAX`; the world is unchanged). It is unreachable in practice and implements `Display` and `std::error::Error`.

### Emission

`World::emit` (private) walks `deposits.iter()` in deposit ID order. For each deposit it looks up the resource type, computes `PerishesAt::for_good(good.perishes, self.turn)`, adds the type's `output` to the stockpile of the deposit's hex, and records `(good, output)`. Each deposit adds its output once; there is no multiplication by a count, so the order of floating-point additions is fixed. Several deposits of one good on a hex merge into one group. The private `totals_by_good` helper then sums the recorded amounts per good (with `Quantity::saturating_add`) and returns them in good ID order, leaving out zero totals; these become `TurnReport::emitted`.

Emission is the seam for deposit ownership: once deposits have owners (Stage 04, businesses), an owned deposit's output goes to its owner's stockpile instead. The per-hex stockpile then either stays as the place for goods nobody owns or is removed.

### Endowment

`World::endow` (private) runs after `emit` and walks `pops.iter()` in POP ID order. For each POP it looks up the POP type in `rules.pop_types`, and for each `Endowment { good, amount }` adds `amount.saturating_mul(pop.size)` to the POP's stockpile in the group `PerishesAt::for_good(good.perishes, self.turn)`, recording `(good, total)`. `totals_by_good` turns the records into `TurnReport::endowed`. A type with an empty endowment adds nothing, and a world with no POPs behaves exactly as before POPs existed.

Deposits emit before POPs are endowed so the order of floating-point additions is fixed and `emitted` is unchanged by POPs. `endow` borrows `self.rules` and `self.pops` while mutating `self.pop_stockpiles`; they are separate fields, and the trade round relies on the same split (see [Trade round](#trade-round)). See [pops.md](pops.md) for POPs themselves.

### Trade round

`World::trade` (private) runs after `endow`. For each hex in hex ID order that has POPs, it builds the hex's `Market` from the world's fields, moves each POP's stockpile out into a `Submission` (`std::mem::take`), calls `trade::hex_round`, moves the stockpiles back, and stores each `PopOutcome` by `PopId`. Then it replaces `pop_outcomes` and returns the totals sold, bought and consumed through `totals_by_good` as a `TradeTotals { sold, bought, consumed }` (a private struct, each list in good ID order), which `run_turn` puts into the report. A refused trade can't happen after the demand and the purchase cap; if one does, it panics with `engine bug: the market on hex #<hex> refused a trade for POP #<pop>: <error>`. The loop reads `map`, `pops`, `rules`, `hex_currencies` and `hex_prices` and writes `pop_stockpiles`, as separate fields, so the borrows don't conflict. See [trade.md](trade.md) for the round itself.

### Growth

`World::grow` (private) runs after `trade`. For each POP in ID order it reads the growth rate from the POP's `PopOutcome`, sets the size to `growth::grown_size(size, rate)` (`size × (1 + rate)`, saturating) with the crate-private `Pops::set_size`, and adds the new size to a running total, which becomes the report's `population`. It reads `pop_outcomes` and writes `pops`, which are separate fields, so the borrows don't conflict. The rate was computed by the trade round from the POP's demand and its type's growth parameters; see [growth.md](growth.md) for the index and the rate.

### Perishing

`World::perish` (private) calls `Stockpile::remove_perishing(self.turn)` on every hex stockpile in hex ID order and then on every POP stockpile in POP ID order, and passes all the removed amounts through `totals_by_good`. The totals, hexes and POPs together, become `TurnReport::perished`.

### Determinism

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and goods in ID order within a POP, with every sale on a hex before any purchase and then consumption, POPs grow in ID order, stockpiles perish hexes first and then POPs, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds (POP sizes included), outcomes and reports. `World` itself is not `PartialEq`, but `Stockpile`, `PopOutcome` and `TurnReport` are, which is how tests compare two runs.

## TurnReport

```rust
pub struct TurnReport {
    pub turn: Turn,                         // the turn that started
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn, hexes and POPs
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted by deposits at the start of `turn`
    pub endowed: Vec<(GoodId, Quantity)>,   // received by POPs as endowment at the start of `turn`
    pub sold: Vec<(GoodId, Quantity)>,      // sold by POPs in the trade round of `turn`
    pub bought: Vec<(GoodId, Quantity)>,    // bought by POPs in the trade round of `turn`
    pub consumed: Vec<(GoodId, Quantity)>,  // consumed by POPs after the trade round of `turn`
    pub population: Quantity,               // total size of every POP after growth in `turn`
}
```

Each list has one total per good, summed over every hex (and, for `perished`, `endowed`, `sold`, `bought` and `consumed`, every POP), in good ID order, and lists only amounts greater than zero. `emitted` and `endowed` are kept apart so deposit output and POP income can be told apart. `population` is a single total, `Quantity::ZERO` for a world with no POPs. There is no per-hex or per-POP breakdown in the report: what each holds is in its stockpile, what each POP traded and how it grew is in its `PopOutcome` (`World::pop_outcome`), and its size is in `World::pops()`. For the sample setting turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`; endowed 4100 `unskilled_labor`; sold 2878.6 `unskilled_labor`; bought 5 `copper` and 1426.8 `grain`; consumed 1221.4 `unskilled_labor`, 5 `copper` and 1426.8 `grain`; population 419.4466666666666". Because the POPs grow, later turns endow, trade and consume more (each turn's `endowed` is 10 labor times the previous turn's `population`), and `perished` is the deposit output only: the POPs sell or consume all their labor in the round, so none is left to perish (apart from floating-point slivers, which the checker would print).

## Turn

`Turn` is a `u64` newtype (`Turn::ZERO`, `Turn::MAX`, `new`, `get`, `next`, `saturating_add`), `Ord`, `Default` (zero), and `Display` (the number). The type stops turn numbers being confused with other integers, and `next` returning `None` at `MAX` means the clock can never wrap.

## Stockpile

`Stockpile` is a general type, not specific to hexes. Hexes use it for unowned goods and POPs for what they own (see [pops.md](pops.md)); Stage 03 reuses it for industries. It is a `BTreeMap<GoodId, BTreeMap<PerishesAt, Quantity>>`: sparse (only goods with a positive amount, only groups with an amount greater than zero) and ordered (good ID order, then soonest group first).

Each good's amount is split into groups by when it perishes, so `perishes_after` works without tracking individual units:

```rust
pub enum PerishesAt { EndOfTurn(Turn), Never }   // derives Ord: EndOfTurn(t) by t, then Never
```

`PerishesAt::for_good(perishes, added_in)` gives `Never` for a durable good and `EndOfTurn(added_in + n - 1)` for `Perishes::After(n)` (saturating at `Turn::MAX`). That is the meaning of `perishes_after` fixed in [goods.md](goods.md): with `perishes_after = 1`, output emitted in turn 5 can be used in turn 5 and is gone when turn 5 ends; with 3 it is gone at the end of turn 7. A good with `perishes_after = 3` emitted at 5 per turn therefore holds 5, 10, 15 in turns 0 to 2, and 15 from then on, with 5 perishing and 5 emitted each step.

Reading (public):

| Method | Returns |
|---|---|
| `is_empty()` | true if it holds nothing |
| `iter()` | `(GoodId, Quantity)` totals, in good ID order |
| `total(good)` | the total of one good (zero if none) |
| `groups(good)` | `(PerishesAt, Quantity)`, soonest first (empty if none) |

Changing (`pub(crate)`, engine only):

- `add(good, amount, at)` merges `amount` into the group `at`; a zero amount changes nothing.
- `remove(good, amount) -> Result<(), Quantity>` takes `amount` out, **soonest-perishing group first**, then the next, so what would spoil first is used first and durable units are kept longest. It is all-or-nothing: if the stockpile holds less than `amount`, it returns `Err(held)` (the total held) and changes nothing; there is no partial removal. A zero amount is `Ok` and changes nothing. A group that reaches zero is dropped, and so is a good left with no groups, so the stockpile stays sparse. Because the check compares against the floating-point total of the groups, a tiny remainder from rounding can be left unremoved when the groups run out; that is accepted.
- `retain(good, keep) -> Quantity` removes everything of `good` above `keep`, soonest-perishing group first, and returns the amount removed (zero, changing nothing, if it holds `keep` or less). It walks the groups from the latest-perishing back, keeping whole groups while they fit in `keep`; the group where `keep` runs out is set to exactly the remainder (or dropped if that is zero), and every sooner group is dropped. So when the kept amount lies in one group, exactly `keep` remains, with no rounding sliver; `keep = 0` removes the good entirely (there is no separate "remove all" method).
- `remove_perishing(turn)` removes every group at `EndOfTurn(t)` with `t <= turn` (not only `t == turn`, so no group can be stranded), drops goods left with no groups, and returns the totals removed per good.

There is no public way to change a stockpile. Removal is used by the dummy market (see [market.md](market.md)), which is the only way code outside the engine can change one: a caller passes a stockpile it owns (in tests, a clone of a hex stockpile) to `Market::sell`, `sell_down_to`, `buy` or `buy_for`. Inside the engine, the trade round sells POP goods with `sell_down_to` (which uses `retain`) and removes consumed goods with `remove` (see [trade.md](trade.md)). A `GoodId` does not say which `Goods` list it came from, so a stockpile simply holds the IDs it is given; the world only gives it IDs from its own rules.

## Quantity

`Quantity`'s arithmetic is `saturating_add` and `saturating_mul`, which clamp at `f64::MAX` so the result of two finite values never becomes infinite; `checked_sub`, which returns `None` if the result would be negative and never returns `-0.0`; and `checked_div`, which returns `None` when dividing by zero and otherwise clamps at `f64::MAX` (zero divided by a positive value is zero). `Quantity::ONE` is one unit (the price of a hex's currency in its own market). Arithmetic is added only when a system needs it (see [resources.md](resources.md)); `checked_div` exists for Stone-Geary demand (see [demand.md](demand.md)).

## Setting checker: `--turns N`

See the table in [setting-loader.md](setting-loader.md). `dev-ui check <SETTING_FOLDER> --turns N` loads the setting, prints the success line, creates a `World`, prints turn 0's line, then steps `N` times, printing one line per turn:

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666
Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4194.466666666667 unskilled_labor; sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain; population 429.2982444444444
```

The line lists `perished`, `emitted`, `endowed`, `sold`, `bought` and `consumed` in that order, then `population`. Entries are `<amount> <good name>` joined by `, ` in report order (good ID order: `copper` comes before `grain` in the sample's goods); an empty list prints `nothing`, so a setting without POPs ends every line with `; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0`. The amounts are floating-point sums printed with `Quantity`'s `Display`, with nothing rounded, so they can print with a long tail of digits, as the sample's do from turn 1 on once its POPs have grown. The line is built by `text::turn_line`. The checker is the headless `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)). The dev UI window builds a `World` from a loaded setting, steps it with its Step buttons, and shows the same turn lines in its turn log as `check --turns` prints.

## Tests

- Unit tests: `turn.rs`, `quantity.rs` (including `checked_sub`, `saturating_mul` and `checked_div`), `stockpile.rs` (including removal: part of the soonest group, exactly a group, across groups, the whole total, more than held, and zero; and `retain`: from the soonest group first, exactly `keep` left in a single group, `keep = 0`, at or below `keep`, and `keep` on a group boundary), `world.rs` (a world forced to `Turn::MAX`, for both `turns_valid` and `pops_placed`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles, POP outcomes and POP sizes unchanged), `trade.rs` (see [trade.md](trade.md)) and `growth.rs` (see [growth.md](growth.md)). The stockpile tests take `q`, `good`, `after` and `end` from `crates/engine/src/test_support.rs`. Trading through the market is tested in `crates/engine/tests/market.rs` (see [market.md](market.md)), demand in `crates/engine/tests/demand.rs` (see [demand.md](demand.md)), the trade round in `crates/engine/tests/trade.rs`, and growth in `crates/engine/tests/growth.rs`.
- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports (the POPs' labor in `endowed`, 10 × the previous report's `population` after turn 0, and only the deposit output in `perished`, since POPs use up their labor), a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits (which also endow and trade nothing, with a population of 0), determinism (reports, hex stockpiles and POP stockpiles of two runs), the accessors, hex currencies being readable and unchanged by stepping, and the foreign-hex panics of `hex_stockpile` and `hex_currency`. POP endowment in the world is tested in `crates/engine/tests/pops.rs` (see [pops.md](pops.md)), the trade round in `crates/engine/tests/trade.rs` (see [trade.md](trade.md)), and growth in `crates/engine/tests/growth.rs` (see [growth.md](growth.md)).
- `turns_valid` (`crates/engine/tests/settings/turns_valid/`) has goods `daily` (perishes after 1), `stew` (after 3), and `gold` (durable, every hex's currency); resource types `field`, `meadow`, `pot`, and `mine`; three hexes; six deposits, with (0, 0) holding four, (1, 0) two, and (2, 0) none; and two markets in `start/markets.toml`, on (0, 0) and (1, 0), with (2, 0) having none. It has no POPs.
- `crates/dev-ui/tests/checker.rs` covers `check --turns`: output for the sample (turns 0, 1 and 2, compared exactly as printed), `--turns 0`, empty lists, decimal amounts, `pops_placed`'s lines, the usage errors, and a broken setting.

## Not yet

Ownership of deposits, starting stockpiles in `start/` (for hexes or POPs), deposit stock, depletion and regeneration, any effect of `movable`, per-hex and per-POP report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, and actors other than POPs in the "actors act" part of the turn.
