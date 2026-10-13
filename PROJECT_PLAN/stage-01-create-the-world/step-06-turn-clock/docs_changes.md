# Step 06: turn-clock — Docs Changes

## docs/world.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# The world and the turn

A loaded setting only describes a world: its rules and what turn 0 looks like. A `World` (`crates/engine/src/world.rs`) is that setting running. It adds the clock and the goods that exist, and it can be stepped turn by turn. The supporting types are `Turn` (`turn.rs`) and `Stockpile` (`stockpile.rs`). None of these modules knows about files; `world` depends only on the public setting types `LoadedSetting`, `Manifest`, and `Rules`.

## What a World holds

`World::new(setting: LoadedSetting)` consumes the loaded setting and takes it apart:

| Field | Source | Changes? |
|---|---|---|
| `manifest`, `rules` | the setting | never (goods and resource types are fixed for the whole game) |
| `map: HexMap`, `deposits: Deposits` | the setting's `StartState` | not yet; they are world state because later Stages change them (deposit ownership in Stage 02) |
| `turn: Turn` | starts at `Turn::ZERO` | advances by one per step |
| `hex_stockpiles: Vec<Stockpile>` | empty at creation | goods enter by emission and leave by perishing |

The world owns the setting, so callers (the dev UI) have no lifetimes to manage. There is one stockpile of **unowned goods** per hex, in a `Vec` indexed by `HexId::index` (hex IDs are dense). Public accessors are read-only: `turn()`, `manifest()`, `rules()`, `map()`, `deposits()`, and `hex_stockpile(hex)`. `hex_stockpile` panics with `hex ... is not from this map` for a hex from another map (a caller bug).

## The turn

A turn has three parts:

1. **Start of turn:** every deposit emits its resource type's `output` of its `produces` good into its hex's stockpile.
2. **Actors act:** nothing yet. Later Stages add POPs, industries, and markets here.
3. **End of turn:** every group that perishes at the end of this turn is removed from every stockpile.

Two calls drive the clock:

- `World::new(setting) -> (World, TurnReport)` creates the world at turn 0 and runs turn 0's start. The report's `perished` is empty.
- `World::step(&mut self) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`), so a refused step changes nothing. Then it runs the end of the current turn (perishing, hexes in ID order), advances the clock, and runs the start of the new turn (emission).

So between calls the world is always at part 2: this turn's output is in the stockpiles and nothing has perished yet this turn. This is the state that tests, the checker, and the dev UI observe. If observation happened after the end of the turn, goods with `perishes_after = 1` would always look empty.

`StepError` has one variant, `ClockAtMaximum` (the clock is at `Turn::MAX`; the world is unchanged). It is unreachable in practice and implements `Display` and `std::error::Error`.

### Emission

`World::emit` (private) walks `deposits.iter()` in deposit ID order. For each deposit it looks up the resource type, computes `PerishesAt::for_good(good.perishes, self.turn)`, adds the type's `output` to the stockpile of the deposit's hex, and adds the same amount to a per-good total. Each deposit adds its output once; there is no multiplication by a count, so the order of floating-point additions is fixed. Several deposits of one good on a hex merge into one group. The totals become `TurnReport::emitted`.

Emission is the seam for Stage 02: once deposits have owners, an owned deposit's output goes to its owner's stockpile instead. The per-hex stockpile then either stays as the place for goods nobody owns or is removed.

### Perishing

`World::perish` (private) calls `Stockpile::remove_perishing(self.turn)` on every hex stockpile in hex ID order and merges the per-good totals (with `Quantity::saturating_add`). The totals become `TurnReport::perished`.

### Determinism

No `HashMap` or `HashSet` is used. Deposits emit in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times gives identical worlds and reports. `World` itself is not `PartialEq`, but `Stockpile` and `TurnReport` are, which is how tests compare two runs.

## TurnReport

```rust
pub struct TurnReport {
    pub turn: Turn,                         // the turn that started
    pub perished: Vec<(GoodId, Quantity)>,  // removed at the end of the previous turn
    pub emitted: Vec<(GoodId, Quantity)>,   // emitted at the start of `turn`
}
```

Each list has one total per good, summed over every hex, in good ID order, and lists only amounts greater than zero. There is no per-hex breakdown; what each hex holds is in its stockpile. For the sample setting turn 0 reports "perished nothing; emitted 10 `unimproved_copper` and 24 `unimproved_grain`", and every later step perishes and emits the same amounts, because both goods perish after one turn.

## Turn

`Turn` is a `u64` newtype (`Turn::ZERO`, `Turn::MAX`, `new`, `get`, `next`, `saturating_add`), `Ord`, `Default` (zero), and `Display` (the number). The type stops turn numbers being confused with other integers, and `next` returning `None` at `MAX` means the clock can never wrap.

## Stockpile

`Stockpile` is a general type, not specific to hexes. Hexes use it now; Stage 02 reuses it for what POPs own and Stage 03 for industries. It is a `BTreeMap<GoodId, BTreeMap<PerishesAt, Quantity>>`: sparse (only goods with a positive amount, only groups with an amount greater than zero) and ordered (good ID order, then soonest group first).

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
- `remove_perishing(turn)` removes every group at `EndOfTurn(t)` with `t <= turn` (not only `t == turn`, so no group can be stranded), drops goods left with no groups, and returns the totals removed per good.

There is no public way to change a stockpile, and no way yet to take goods out to use them. That arrives with Stage 02, which also decides which units are taken first. A `GoodId` does not say which `Goods` list it came from, so a stockpile simply holds the IDs it is given; the world only gives it IDs from its own rules.

## Quantity

`Quantity` gained its first arithmetic, `saturating_add`, which clamps at `f64::MAX` so the sum of two finite values never becomes infinite (see [resources.md](resources.md)). Nothing else was added.

## Setting checker: `--turns N`

See the table in [setting-loader.md](setting-loader.md). `dev-ui <SETTING_FOLDER> --turns N` loads the setting, prints the success line, creates a `World`, prints turn 0's line, then steps `N` times, printing one line per turn:

```
Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain
Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain
```

Entries are `<amount> <good name>` joined by `, ` in report order; an empty list prints `nothing`. This is deliberately minimal: Step 07 replaces the checker with a real developer UI, which builds on `World`.

## Tests

- Unit tests: `turn.rs`, `quantity.rs`, `stockpile.rs`, and `world.rs` (including a world forced to `Turn::MAX`, which refuses to step and stays unchanged).
- `crates/engine/tests/world.rs` runs the sample setting and `turns_valid` through the public API: the sample's stockpiles and reports, a 3-turn good building up and levelling off, a durable good growing and never perishing, deposits merging into one group, deposits not leaking into other hexes, settings with no deposits, determinism, the accessors, and the foreign-hex panic.
- `turns_valid` (`crates/engine/tests/settings/turns_valid/`) has goods `daily` (perishes after 1), `stew` (after 3), and `gold` (durable); resource types `field`, `meadow`, `pot`, and `mine`; three hexes; and six deposits, with (0, 0) holding four, (1, 0) two, and (2, 0) none.
- `crates/dev-ui/tests/checker.rs` covers `--turns`: output for the sample, `--turns 0`, empty lists, decimal amounts, the usage errors, and a broken setting.

## Not yet

Taking goods out of a stockpile, ownership of deposits and goods, starting stockpiles in `start/`, deposit stock, depletion and regeneration, any effect of `movable`, per-hex report breakdowns, saving and loading a world, and any behavior in the "actors act" part of the turn.

## docs/setting-loader.md — modify (setting checker table)
- **Location:** "Setting checker (dev UI)"
- **Change:** replace
- **Supersedes:** step-02-setting-loader/docs_changes.md (usage row); step-05-natural-resources/docs_changes.md (the success-line row stays as written there)

Add `--turns N` to the table of arguments and replace the "more than one" row:

| Arguments | Behavior | Exit code |
|---|---|---|
| one path, `--turns N` | after the success line, creates a world, prints turn 0's line, then steps `N` times printing one line per turn (see [world.md](world.md)); `N` is a whole number from 0 to 10,000 (ASCII digits only, leading zeros allowed) | 0 |
| one path with errors, `--turns N` | as for one path with errors; no turns are run | 1 |
| anything else (two arguments that are not `<path> --turns <N>`, `--turns` without a number, `N` out of range or not a number, extra arguments) | prints `usage: dev-ui [SETTING_FOLDER [--turns N]]` to stderr | 2 |

All arguments are parsed before anything is loaded. The no-argument form, the success line, and the failure output are unchanged without `--turns`.

## docs/resources.md — modify
- **Location:** second paragraph of the intro; "Using resources"; "Planned extensions"; the `Quantity` bullet under "Types"
- **Change:** replace
- **Supersedes:** step-05-natural-resources/docs_changes.md (the "Nothing is produced yet" sentence, the "no arithmetic yet" sentence, and the last sentence of "Planned extensions")

- Intro, second paragraph: replace the first sentence with: Each deposit emits its resource type's `output` of its good into its hex's stockpile at the start of every turn (see [world.md](world.md)). There is no ownership (Stage 02), and no deposit stock, depletion, or regeneration (later). The engine does not require the produced good to be unmovable or to perish after one period; the setting decides.
- `Quantity` bullet: replace "It has no arithmetic yet; Step 06 adds what it needs." with: Its only arithmetic is `saturating_add`, which clamps at `f64::MAX` so the result stays finite.
- "Using resources": add after the closing paragraph: Output is emitted per deposit per turn, in deposit ID order, so a hex's stockpile holds the sum over its deposits.
- "Planned extensions": replace the last sentence with: Each deposit already emits its resource type's `output` every turn (see [world.md](world.md)), with no depletion. Stage 02 gives each deposit one owner, and an owned deposit's output then goes to its owner instead of the hex stockpile.

## docs/goods.md — modify
- **Location:** the section on the `perishes_after` property
- **Change:** add
- **Supersedes:** none

Add: `perishes_after` takes effect in Step 06. A good with `perishes_after = N` that enters a stockpile in turn T perishes at the end of turn T+N-1, and a good without it never perishes. See `PerishesAt::for_good` and [world.md](world.md). `movable` still has no effect.

## docs/workspace.md — modify
- **Location:** Layout block
- **Change:** add
- **Supersedes:** none

Under `crates/engine/src/` add `turn.rs` (the `Turn` number), `stockpile.rs` (`Stockpile` and `PerishesAt`), and `world.rs` (`World`, `TurnReport`, `StepError`; see world.md). Under `crates/engine/tests/` add `world.rs`.

## docs/README.md — modify
- **Location:** the table of documents, after the `resources.md` row
- **Change:** add
- **Supersedes:** none

| [world.md](world.md) | The running `World`, the three-part turn, stockpiles with perish groups, turn reports, and the setting checker's `--turns` option |
