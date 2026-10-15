# Step 04: pops — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-04-pops/design.md` — the behavior this spec implements.
- `PROJECT_PLAN/stage-02-populate-the-world/step-03-pop-types/spec.md` — what Step 03 built (`PopType`, `PopTypes`, `PopTypesReader`); this Step places POPs of those types.
- `docs/setting-loader.md` — the loader's two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/resources.md` — `DepositsReader` and `Deposits` are the pattern for a placed thing in `start/` with a type reference, coordinates, and a per-hex index. `[[pop]]` follows it exactly.
- `docs/world.md` — `World`, the three-part turn, `Stockpile`, `PerishesAt`, `TurnReport`, the checker's turn line.
- `docs/dev-ui.md` — the checker's success line and the window's turn log.
- `docs/workspace.md` — the check command and test layout.
- Code: `crates/engine/src/resources.rs` (`Deposits`: the list to mirror), `crates/engine/src/setting/resources.rs` (`DepositsReader`: the reader to mirror), `crates/engine/src/setting/map.rs` (`MapReader::id`, `coordinate`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/setting/pops.rs` (`PopTypesReader`), `crates/engine/src/setting/mod.rs` (`WorldContent`, `StartState`, `load_setting`), `crates/engine/src/pops.rs`, `crates/engine/src/world.rs`, `crates/engine/src/stockpile.rs`, `crates/engine/src/quantity.rs` (`saturating_mul`), `crates/engine/src/id.rs` (`IdVec`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/world.rs` (helpers and the sample expectations), `crates/engine/tests/pops.rs`, `crates/engine/tests/resources.rs` (`every_deposit_error_is_reported_at_its_place`), `crates/dev-ui/src/text.rs` (`turn_line`), `crates/dev-ui/src/check.rs`, `crates/dev-ui/src/session.rs` (tests assert turn lines), `crates/dev-ui/tests/checker.rs`, `settings/sample/start/map.toml` and `deposits.toml`, `settings/sample/rules/pops.toml`.

## Approach

- **Rename Step 03's ID names first.** The POP *type* ID becomes `PopTypeKind` / `PopTypeId` (label stays `"pop type"`), freeing `PopKind` / `PopId` (label `"pop"`) for the placed POP. Rationale: "POP ID" means the placed POP everywhere from here on; a type ID called `PopId` would be a permanent trap. Nothing outside the engine crate uses the old names.
- **A `[[pop]]` section in `start/`**, read by a new `PopsReader` in `setting/pops.rs` (next to `PopTypesReader`, as `setting/resources.rs` holds both readers) that follows `DepositsReader`: `type`, `q`, `r`, `size` all read before any early return; duplicates checked by (type name as written, coordinates) in phase 1; the type and the hex resolved in phase 2. Rationale: POPs are state identified by hex and type, exactly like deposits by hex and resource.
- **Domain type `Pops` in `pops.rs`** mirrors `Deposits`: an `IdVec<PopKind, Pop>` plus a per-hex index. `Pop { hex, pop_type, size }`. `size` is a `Quantity` (finite, greater than 0; the loader guarantees it), so `amount.saturating_mul(size)` gives the endowment and Step 07 can multiply it by a growth rate. Rationale: one numeric representation for every continuous amount in the engine.
- **The World holds `pops: Pops` and `pop_stockpiles: IdVec<PopKind, Stockpile>`**, one stockpile per POP, mirroring `hex_stockpiles`. Rationale: the same shape as hexes; Step 06 can borrow a POP's stockpile mutably while the `Market` borrows the rules and prices (separate fields).
- **Start of turn: deposits emit, then POPs are endowed in ID order.** For each good in the POP's type's endowment, `amount × size` is added to the POP's stockpile with `PerishesAt::for_good(good.perishes, turn)`. **End of turn: hex stockpiles perish, then POP stockpiles**, both into one `perished` list.
- **`TurnReport` gains `endowed`**, the total endowed per good, after `emitted`. The checker's and the window's turn line gain `; endowed <list>`. The checker's success line gains `, N pops`.
- **New test settings `pops_placed` (turn behavior) and `pops_placed_errors`**, rather than adding POPs to `turns_valid`, whose exact turn lines many tests assert. `pops_one` gains a hex and one POP for the checker's `1 pop`.
- **The build runs as one.** The reader needs the `Pops` type, the world needs `StartState::pops`, and the dev UI needs `TurnReport::endowed`, so no package could be built and tested alone.

## Changes

### New

- Test settings `crates/engine/tests/settings/pops_placed/` and `pops_placed_errors/` (exact contents below).
- `crates/engine/tests/settings/pops_one/start/map.toml` and `start/pops.toml`.
- `settings/sample/start/pops.toml`.

### Modified

- `crates/engine/src/pops.rs` — rename `PopKind` → `PopTypeKind`, `PopId` → `PopTypeId`; add `PopKind`, `PopId`, `Pop`, `Pops`, unit tests. Module docs: POP types *and* the placed POPs; a POP is N identical individuals of one type on one hex, with its own stockpile in the World; at most one POP per type per hex; size is continuous.
- `crates/engine/src/setting/pops.rs` — use the renamed type names; rename the private `PopEntry` (the pop-type entry) to `PopTypeEntry`; add `PopTypesReader::defs()`; add `PopsReader`. Module docs: both section types.
- `crates/engine/src/setting/mod.rs` — `StartState.pops`, `WorldContent.pops`, the `pop` section type, `resolve`, `load_setting`. Module docs: `[[pop]]` is the fourth `start/` kind; a POP's `type` resolves against the POP types and its coordinates against the map in phase 2.
- `crates/engine/src/world.rs` — fields `pops`, `pop_stockpiles`; accessors `pops()`, `pop_stockpile(pop)`; `endow`; `perish` covers both; `TurnReport.endowed`; module docs (the turn's three parts as in `design.md`; the world also holds the POPs and one stockpile each; POP stockpiles start empty).
- `crates/engine/src/lib.rs` — the export list: `pub use pops::{Endowment, Pop, PopId, PopKind, PopType, PopTypeId, PopTypeKind, PopTypes, Pops, UtilityGood};`. Crate docs: POPs (groups of individuals of a POP type, placed on hexes) live in the `pops` module; each POP receives its type's endowment into its own stockpile every turn.
- `crates/dev-ui/src/text.rs` — `turn_line` gains the `endowed` part; its doc comment.
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/src/session.rs` — the test expectations (turn lines).
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 pops` (the sample `16 pops`, `pops_one` `1 hex` and `1 pop`); every expected turn line gains `; endowed ...`; a new test for `pops_placed`.
- `crates/engine/tests/pops.rs` — the placed-POP tests (Test Plan).
- `crates/engine/tests/world.rs` — the sample's expectations now include the POPs' labor in `perished`, plus `endowed`; determinism compares POP stockpiles too.
- `crates/engine/tests/resources.rs` — comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a `[[pop]]`).
- Test settings `resources_broken_file/start/s.toml` (append a `[[pop]]`, below), `pops_one` (new files).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- `settings/sample/start/README.md` — mention `pops.toml`. `settings/sample/rules/pops.toml` — replace the last header-comment line ("Nothing uses POP types yet: ...") with "POPs of these types are placed in start/pops.toml."

## Interfaces & Data

### `crates/engine/src/pops.rs`

Rename (keep the doc comments, update the wording):

```rust
/// Marker kind for POP types.
pub enum PopTypeKind {}
impl IdKind for PopTypeKind { const LABEL: &'static str = "pop type"; }
/// The ID of a POP type.
pub type PopTypeId = Id<PopTypeKind>;
pub type PopTypes = NamedList<PopTypeKind, PopType>;
```

Add:

```rust
use crate::id::IdVec;
use crate::map::{HexId, HexKind};

/// Marker kind for POPs.
pub enum PopKind {}
impl IdKind for PopKind {
    const LABEL: &'static str = "pop";
}
/// The ID of a POP. Valid only within the [`Pops`] that produced it.
pub type PopId = Id<PopKind>;

/// One POP: N identical individuals of one type on one hex. Its stockpile is world state
/// (see the `world` module), not part of this record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pop {
    /// The hex it lives on.
    pub hex: HexId,
    /// Its POP type.
    pub pop_type: PopTypeId,
    /// N, the number of individuals. Continuous; always finite and greater than 0.
    pub size: Quantity,
}

/// Every POP in a setting, in load order (which is also ID order). At most one POP of each
/// type on each hex.
///
/// `Pops::default()` is empty and was built with no map, so `on_hex` and `of_type_on_hex`
/// panic on it for any hex.
#[derive(Debug, Default)]
pub struct Pops {
    /// One entry per POP, by `PopId`.
    pops: IdVec<PopKind, Pop>,
    /// One entry per hex, by `HexId`: the POPs on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<PopId>>,
}

impl Pops {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map the POPs live on. Crate-private: only the setting loader
    /// builds POPs.
    ///
    /// # Panics
    /// Panics if a POP's hex index is `>= hex_count` (`"{hex:?} is not from a map of
    /// {hex_count} hexes"`), if two POPs have the same hex and type (`"two POPs of
    /// {pop_type:?} on {hex:?}"`; the loader rejects duplicates first), or if there are
    /// more than `u32::MAX` POPs.
    pub(crate) fn from_pops(pops: Vec<Pop>, hex_count: usize) -> Self;
    /// Number of POPs.
    pub fn len(&self) -> usize;
    /// True if the setting has no POPs.
    pub fn is_empty(&self) -> bool;
    /// The POP `id`. Panics (`is not from this list`) if `id` isn't from this list.
    pub fn get(&self, id: PopId) -> &Pop;
    /// Every POP as (ID, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (PopId, &Pop)> + '_;
    /// The POPs on `hex`, in ID order; empty if it has none. Panics if `hex` isn't from the
    /// map these POPs were built with.
    pub fn on_hex(&self, hex: HexId) -> &[PopId];
    /// The POP of `pop_type` on `hex`, if any. Panics like `on_hex`.
    pub fn of_type_on_hex(&self, hex: HexId, pop_type: PopTypeId) -> Option<PopId>;
}
```

`from_pops` follows `Deposits::from_deposits`; the same-type check walks `by_hex` after filling it (for each hex, no two IDs with the same `pop_type`; a simple nested loop is fine, the lists are short). `of_type_on_hex` is `on_hex(hex).iter().copied().find(..)`.

### `PopsReader` (`crates/engine/src/setting/pops.rs`)

```rust
use std::collections::HashMap;
use crate::map::HexCoord;
use crate::pops::{Pop, Pops};
use super::error::Location;
use super::map::{MapReader, coordinate};

impl PopTypesReader {
    /// The definitions, for resolving references to POP types in phase 2.
    pub fn defs(&self) -> &Definitions<PopTypeKind>;
}

/// One `[[pop]]` entry, waiting for phase 2.
struct PopEntry {
    /// The `[[pop]]` header.
    at: Location,
    pop_type: Located<String>,
    coord: HexCoord,
    /// `Quantity::ONE` is a placeholder if the size was invalid (the load fails anyway).
    size: Quantity,
}

/// Collects POPs during phase 1, and resolves them in phase 2.
#[derive(Default)]
pub(crate) struct PopsReader {
    entries: Vec<PopEntry>,
    /// (type name as written, coordinates) to the header of the first entry.
    /// Lookup only; never iterated.
    seen: HashMap<(String, HexCoord), Location>,
    /// Filled in phase 2.
    pops: Vec<Pop>,
}

impl PopsReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(&mut self, pop_types: &Definitions<PopTypeKind>, map: &MapReader, sink: &mut ErrorSink);
    /// Only after a successful load.
    pub fn finish(self, hex_count: usize) -> Pops;
}
```

`read`: `at = reader.location().clone()`. Read every key before any early return: `pop_type = reader.required_string("type")` (not name-checked: a bad value is simply an unknown POP type in phase 2); `q = coordinate(reader, "q")`; `r = coordinate(reader, "r")`; `size = reader.required_number("size").and_then(..)` accepting `Quantity::new(v.value).filter(|q| q.get() > 0.0)`, otherwise reporting `key 'size' in pop must be a finite number greater than 0` at `v.at` and giving `None`. Then `let (Some(pop_type), Some(q), Some(r)) = .. else { return; }` (an entry with a missing or invalid type or coordinate is not stored and takes no part in the duplicate check, like deposits). `coord = HexCoord::new(q, r)`; `key = (pop_type.value.clone(), coord)`; if `seen` has it, report `duplicate pop of '<type>' at <coord> (first defined at <first>)` at `at` and return. Otherwise insert and push `PopEntry { at, pop_type, coord, size: size.unwrap_or(Quantity::ONE) }`.

`resolve`: for each entry, `pop_type = pop_types.resolve(&entry.pop_type, sink)` (reports `unknown pop type '<name>'` at the value), `hex = map.id(entry.coord)`, reporting `no hex at <coord>` at `entry.at` if None. If both are `Some`, push `Pop { hex, pop_type, size: entry.size }`. Both checks always run, so an entry with an unknown type still gets its hex checked and the other way round.

`finish(hex_count)`: `Pops::from_pops(self.pops, hex_count)`.

### `setting/mod.rs`

- `StartState` gains `pub pops: Pops` (doc: every POP, in load order; see `crate::pops`).
- `WorldContent` gains `pub pops: PopsReader`; `section_types` gains `SectionType { header: "pop", folder: Folder::Start, read: |c, r| c.pops.read(r) }` after `pop_type`.
- `resolve`: add `self.pops.resolve(self.pop_types.defs(), &self.map, sink);` last.
- `load_setting`: `let pops = content.pops.finish(map.len());` after `map` is finished, next to `deposits`; `StartState { map, currencies, deposits, prices, pops }`.
- Update the comment above `resolve` (POPs refer to POP types and hexes) and the module docs (`[[pop]]`, phase 2, the "Adding a section type" list: POPs follow deposits as a placed thing in `start/` identified by type and coordinates, in `crate::pops` and `setting/pops.rs`).

### `crates/engine/src/world.rs`

```rust
pub struct TurnReport {
    pub turn: Turn,
    /// Total removed per good at the end of the previous turn from every hex and POP
    /// stockpile, in good ID order, only amounts > 0. Always empty for turn 0.
    pub perished: Vec<(GoodId, Quantity)>,
    /// Total emitted by deposits per good at the start of `turn`, in good ID order, only
    /// amounts > 0.
    pub emitted: Vec<(GoodId, Quantity)>,
    /// Total received by POPs as endowment per good at the start of `turn`, in good ID
    /// order, only amounts > 0.
    pub endowed: Vec<(GoodId, Quantity)>,
}

pub struct World {
    // existing fields, then:
    /// The POPs. Nothing changes them yet (Step 07 changes their size).
    pops: Pops,
    /// One per POP, by `PopId`. Empty when the world is created.
    pop_stockpiles: IdVec<PopKind, Stockpile>,
}

impl World {
    /// The POPs.
    pub fn pops(&self) -> &Pops;
    /// The stockpile of `pop`: the goods it owns. Panics (`is not from this list`) if `pop`
    /// isn't from this world's POPs.
    pub fn pop_stockpile(&self, pop: PopId) -> &Stockpile;

    /// Start of turn, after `emit`: every POP, in ID order, receives its type's endowment
    /// times its size into its stockpile. Returns the total per good, in good ID order.
    fn endow(&mut self) -> Vec<(GoodId, Quantity)>;
}
```

- `World::new`: `pop_stockpiles = IdVec::from_vec(vec![Stockpile::new(); start.pops.len()])`, `pops: start.pops`; after `emit`, `let endowed = world.endow();`; the report carries both.
- `step`: `perished = self.perish()`, advance, `emitted = self.emit()`, `endowed = self.endow()`.
- `endow`: for `(id, pop)` in `self.pops.iter()`: `let pop_type = self.rules.pop_types.get(pop.pop_type);` for each `Endowment { good, amount }` in `pop_type.endowment()`: `let total = amount.saturating_mul(pop.size); let at = PerishesAt::for_good(self.rules.goods.get(good).perishes, self.turn); self.pop_stockpiles.get_mut(id).add(good, total, at); endowed.push((good, total));`. Return `totals_by_good(endowed)`. (Borrowing: collect what `emit` does today, iterating `self.pops` while mutating `self.pop_stockpiles` is fine because they are separate fields; if the borrow checker objects to `self.rules` in the loop, bind `let rules = &self.rules;` first.) A type with an empty endowment adds nothing.
- `perish`: `totals_by_good(hex removals.chain(pop removals))`, hex stockpiles first, each in ID order (`values_mut()` on each `IdVec`).
- Unit test `a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged`: also compare `pop_stockpiles` before and after (use `turns_valid`, which has no POPs, and additionally a `pops_placed` world).

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.start.pops.len(), "pop", "pops")`. The sample line becomes:
`Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops`. Every other expected line in `checker.rs` ends in `, 0 pops`, except `pops_one`: `Loaded setting 'Pops One' (format version 1): 1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 1 pop type, 1 pop`, and the new `pops_placed` test: `Loaded setting 'Pops Placed' (format version 1): 3 goods, 3 hexes, 1 resource, 1 deposit, 0 markets, 3 pop types, 4 pops`.

### Turn line (`crates/dev-ui/src/text.rs`)

```rust
format!("Turn {}: perished {}; emitted {}; endowed {}", report.turn, list(..perished), list(..emitted), list(..endowed))
```

Expected lines (every list in good ID order; `unskilled_labor` is the sample's good 0):

- Sample turn 0: `Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor`
- Sample turn N ≥ 1: `Turn N: perished 4100 unskilled_labor, 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor`
- `valid_minimal`: `Turn 0: perished nothing; emitted nothing; endowed nothing`, `Turn 1: perished nothing; emitted nothing; endowed nothing`
- `turns_valid` turn 3: `Turn 3: perished 6.5 daily, 5 stew; emitted 6.5 daily, 5 stew, 1.5 gold; endowed nothing`
- `pops_placed` turn 0: `Turn 0: perished nothing; emitted 2 grain; endowed 43 labor, 3 gold`; turn N ≥ 1: `Turn N: perished 43 labor, 2 grain; emitted 2 grain; endowed 43 labor, 3 gold`

Update every turn-line expectation in `checker.rs` and `session.rs` accordingly (the `steady` string in `turns_prints_a_line_for_each_turn`, `empty_lists_print_nothing_and_decimals_print_as_is`, `open_gives_a_log_of_turn_0_and_nothing_selected`, `step_adds_one_log_line_per_turn`). `the_log_matches_the_checkers_turn_lines` compares against `turn_line` itself and needs no literal change.

### Setting file format

A `[[pop]]` in `start/` has four required keys: `type` (the name of a POP type), `q`, `r` (integers, the coordinates of a hex), and `size` (a number, finite and greater than 0; decimals allowed). Any other key is unknown. At most one POP per type per hex; a hex may hold POPs of several types or none.

### Test setting contents

Write these files exactly; line numbers and columns matter for the expected errors. Every test setting has a `setting.toml` with `format_version = 1`.

**`pops_placed/`** — `setting.toml`: name `Pops Placed`. `rules/goods.toml`:
```toml
[[good]]
name = "labor"
perishes_after = 1
movable = false

[[good]]
name = "grain"
perishes_after = 1

[[good]]
name = "gold"
```
`rules/resources.toml`: one resource `field`, `produces = "grain"`, `output = 2`. `rules/types.toml`:
```toml
[[pop_type]]
name = "worker"
endowment = { labor = 10 }
utility = { labor = { share = 1 } }

[[pop_type]]
name = "hoarder"
endowment = { labor = 4, gold = 1.5 }
utility = { gold = { share = 1 } }

[[pop_type]]
name = "idle"
endowment = {}
utility = { grain = { share = 1 } }
```
`start/map.toml`: hexes (0, 0), (1, 0), (2, 0), each `currency = "gold"`. `start/deposits.toml`: one `field` at (2, 0). `start/a.toml`:
```toml
[[pop]]
type = "worker"
q = 0
r = 0
size = 3

[[pop]]
type = "hoarder"
q = 0
r = 0
size = 2
```
`start/b/c.toml` (a nested folder, read after `start/a.toml`; a fractional size):
```toml
[[pop]]
type = "worker"
q = 1
r = 0
size = 0.5

[[pop]]
type = "idle"
q = 1
r = 0
size = 7
```
Expected (good IDs: labor 0, grain 1, gold 2; type IDs: worker 0, hoarder 1, idle 2): POP 0 `worker` at (0, 0) size 3; POP 1 `hoarder` at (0, 0) size 2; POP 2 `worker` at (1, 0) size 0.5; POP 3 `idle` at (1, 0) size 7. `on_hex`: (0, 0) → [0, 1]; (1, 0) → [2, 3]; (2, 0) → []. `of_type_on_hex((0, 0), hoarder)` → Some(1); `of_type_on_hex((1, 0), hoarder)` → None.

World at turn 0: POP 0 holds `labor` 30 in one group at `end(0)`; POP 1 holds `labor` 8 at `end(0)` and `gold` 3 at `Never`; POP 2 holds `labor` 5 at `end(0)`; POP 3 is empty; hex (2, 0) holds `grain` 2 at `end(0)`; hexes (0, 0) and (1, 0) are empty. Report: `perished` empty, `emitted` `[(grain, 2)]`, `endowed` `[(labor, 43), (gold, 3)]`. After each step to turn t: `perished` `[(labor, 43), (grain, 2)]`, `emitted` and `endowed` unchanged; POP 0 holds `labor` 30 at `end(t)`; POP 1's `gold` is one `Never` group of `3 × (t + 1)`; POP 3 stays empty; the hex stockpiles are as before this Step.

**`pops_placed_errors/`** — `setting.toml`: name `Pops Placed Errors`. `rules/goods.toml`: goods `coin`, `labor` (name only). `rules/p.toml`:
```toml
[[pop]]
type = "worker"
q = 0
r = 0
size = 1
```
`rules/types.toml`: one `[[pop_type]]` `worker`, `endowment = { labor = 1 }`, `utility = { labor = { share = 1 } }`. `start/hexes.toml`: hexes (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), all `currency = "coin"`. `start/b.toml`:
```toml
[[pop]]
type = "worker"
q = 0
r = 0
size = 1
```
`start/a.toml` (93 lines):
```toml
[[pop]]
type = "worker"
q = 0
r = 0
size = 10

[[pop]]
q = 0
r = 0
size = 10

[[pop]]
type = "worker"
r = 0
size = 10

[[pop]]
type = "worker"
q = 0
size = 10

[[pop]]
type = "worker"
q = 1
r = 0

[[pop]]
type = 5
q = 2
r = 0
size = 10

[[pop]]
type = "worker"
q = 1.5
r = 0
size = 10

[[pop]]
type = "worker"
q = 2147483648
r = 0
size = 10

[[pop]]
type = "worker"
q = 9
r = 9
size = 10

[[pop]]
type = "worker"
q = 3
r = 0
size = "10"

[[pop]]
type = "worker"
q = 4
r = 0
size = 0

[[pop]]
type = "worker"
q = 5
r = 0
size = -2

[[pop]]
type = "worker"
q = 6
r = 0
size = inf

[[pop]]
type = "ghost"
q = 7
r = 0
size = 10

[[pop]]
type = "worker"
q = 8
r = 0
size = 10
colour = "red"

[[pop]]
type = "ghost"
q = 10
r = 10
size = nan
colour = "red"
```
Expected error lines:
```
rules/p.toml:1:3: section 'pop' belongs in start/, not rules/
start/a.toml:7:1: missing required key 'type' in pop
start/a.toml:12:1: missing required key 'q' in pop
start/a.toml:17:1: missing required key 'r' in pop
start/a.toml:22:1: missing required key 'size' in pop
start/a.toml:28:8: key 'type' in pop must be a string, found integer
start/a.toml:35:5: key 'q' in pop must be an integer, found float
start/a.toml:41:5: key 'q' in pop is out of range
start/a.toml:45:1: no hex at (9, 9)
start/a.toml:55:8: key 'size' in pop must be a number, found string
start/a.toml:61:8: key 'size' in pop must be a finite number greater than 0
start/a.toml:67:8: key 'size' in pop must be a finite number greater than 0
start/a.toml:73:8: key 'size' in pop must be a finite number greater than 0
start/a.toml:76:8: unknown pop type 'ghost'
start/a.toml:86:1: unknown key 'colour' in pop
start/a.toml:88:1: no hex at (10, 10)
start/a.toml:89:8: unknown pop type 'ghost'
start/a.toml:92:8: key 'size' in pop must be a finite number greater than 0
start/a.toml:93:1: unknown key 'colour' in pop
start/b.toml:1:1: duplicate pop of 'worker' at (0, 0) (first defined at start/a.toml:1:1)
```
If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader`, `coordinate` and `Definitions` message forms are fixed. Note that the entry at line 22 (no `size`) is stored with the placeholder size, so it still counts for duplicates and reference checks; the entries at lines 27, 33 and 39 (bad `type` or `q`) are not stored, so they produce no `no hex` or duplicate errors; and the last entry reports four problems in one load.

**`pops_one/`** — add `start/map.toml` (one hex (0, 0), `currency = "grain"`) and `start/pops.toml` (one `[[pop]]`, `type = "eater"`, (0, 0), `size = 1`). Keep `start/README.md`.

**`resources_broken_file/start/s.toml`**: append
```toml

[[pop]]
type = "nobody"
q = 9
r = 9
size = 1
```
The expected errors stay exactly the one syntax error.

**Sample setting** — `settings/sample/start/pops.toml`, with a header comment in the style of `deposits.toml` explaining the four keys (`type` names a POP type from `rules/pops.toml`; `q`, `r` as for deposits; `size` is N, the number of individuals, a number greater than 0, decimals allowed, because a POP is an aggregate whose endowment and demand scale with N and whose size will grow by a rate), that a hex holds at most one POP of each type and may hold several types or none, that every POP starts with nothing (its first endowment arrives at the start of turn 0), and that the POPs are listed in the map's order (row by row, north to south, west to east). Then exactly these entries, in this order (comment wording is free; the values are exact):

| ID | type | q | r | size | note |
|---|---|---|---|---|---|
| 0 | peasant | 0 | -2 | 20 | |
| 1 | peasant | 1 | -2 | 15 | |
| 2 | peasant | 2 | -2 | 12 | comment: this hex has no market, so this POP's labor can never be sold |
| 3 | peasant | -1 | -1 | 25 | |
| 4 | peasant | 0 | -1 | 30 | |
| 5 | peasant | 1 | -1 | 35 | |
| 6 | peasant | -1 | 0 | 40 | |
| 7 | peasant | 0 | 0 | 60 | comment: the center holds two POPs |
| 8 | artisan | 0 | 0 | 10 | comment: the only hex that prices every good |
| 9 | peasant | 1 | 0 | 35 | |
| 10 | peasant | 2 | 0 | 18 | |
| 11 | peasant | -2 | 1 | 22 | |
| 12 | peasant | -1 | 1 | 28 | |
| 13 | peasant | 0 | 1 | 24 | |
| 14 | peasant | -1 | 2 | 16 | |
| 15 | peasant | 0 | 2 | 20 | |

Hexes with no POP: (2, -1), (-2, 0), (1, 1), (-2, 2); note each in a comment at its place in the order. Total size 410, so `endowed` is `4100 unskilled_labor` every turn. `settings/sample/start/README.md`: add that the POPs are in `pops.toml`.

## Tasks

1. `pops.rs`: rename `PopKind`/`PopId` to `PopTypeKind`/`PopTypeId` here, in `setting/pops.rs` (also `PopEntry` → `PopTypeEntry`), `lib.rs` and `tests/pops.rs`; the suite still passes.
2. `pops.rs`: `PopKind`, `PopId`, `Pop`, `Pops`, module docs, unit tests (Test Plan). Export from `lib.rs`.
3. `setting/pops.rs`: `PopTypesReader::defs`, `PopsReader`, module docs. `setting/mod.rs`: wiring and docs.
4. `world.rs`: `pops`, `pop_stockpiles`, `pops()`, `pop_stockpile`, `endow`, `perish`, `TurnReport.endowed`, docs, the unit test.
5. `text.rs` turn line; `check.rs` success line; `session.rs` and `checker.rs` expectations; the `pops_placed` checker test.
6. Test settings (`pops_placed`, `pops_placed_errors`, `pops_one`, `resources_broken_file`), `tests/settings/README.md`, the sample's `start/pops.toml`, `start/README.md` and the comment line in `rules/pops.toml`.
7. `crates/engine/tests/pops.rs` and `tests/world.rs` (Test Plan); the comment in `tests/resources.rs`.
8. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the success line and the turn lines above.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`).

**Unit tests.** `pops.rs` (`Pops`, following `resources.rs`'s `Deposits` tests with `NameRegistry`-made IDs): sequential IDs in the order given; `on_hex` lists in ID order and is empty for a hex with none; `of_type_on_hex` finds the one POP or gives None; the default list is empty; `get` panics on a foreign ID; `on_hex` panics on a hex beyond the map and on the default list; `from_pops` panics on a hex beyond `hex_count` and on two POPs of one type on one hex. `world.rs`: the clock-maximum test also checks `pop_stockpiles` (see above).

**`crates/engine/tests/pops.rs`** (add to the existing file; helpers: `world_of`, `hex`, `good`, `end` as in `tests/world.rs`; a `pops_of(&LoadedSetting) -> Vec<(usize, String, HexCoord, f64)>` helper giving (index, type name, coordinates, size); a `pop_groups(&World, PopId, good name)` helper like `tests/world.rs`'s `groups`).
- **AC "sample loads with its POPs"**: load the sample; `pops.len() == 16`; `pops_of` equals the table above; the center hex (0, 0) lists POP IDs `[7, 8]` with types `peasant` then `artisan`; (2, -2) lists `[2]` (a `peasant`); (2, -1) lists nothing; `of_type_on_hex((0, 0), artisan)` is `Some(8)`.
- **AC "fresh sample world"**: `World::new(sample)`: turn 0; every POP's stockpile holds exactly `unskilled_labor` = `10 × size` in one group at `end(0)` and nothing else; hex (0, 0)'s stockpile is `[unimproved_copper 2.5, unimproved_grain 12]` as before; the report's `endowed` is `[(unskilled_labor, 4100)]`, `emitted` unchanged, `perished` empty.
- **AC "stepping the sample"**: step 3 times; each report has `perished` `[(unskilled_labor, 4100), (unimproved_copper, 10), (unimproved_grain, 24)]`, `emitted` as before and `endowed` `[(unskilled_labor, 4100)]`; after each step POP 7 holds `unskilled_labor` 600 at `end(t)` only. Two sample worlds stepped 3 times give equal reports and equal `pop_stockpile` for every POP (also add the POP comparison to `identical_runs_give_identical_results` in `tests/world.rs`).
- **AC "test setting with several files, two types on one hex, a durable endowment, an empty endowment, a fractional size"**: load `pops_placed`; `pops_of` and `on_hex`/`of_type_on_hex` as listed above; the world at turn 0 and after steps to turns 1..=3 as listed above (groups and totals per POP, hex (2, 0)'s `grain`, the reports); loading twice gives equal `pops_of`.
- **AC "each error case"**: `failures_of("pops_placed_errors")` equals the expected list.
- **AC "one entry with several problems"**: covered by the last entry in that list.
- **AC "checks skipped after a broken file"**: the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing; extend its comment.
- **AC "a world with no POPs behaves as before"**: `valid_minimal` and `turns_valid` worlds have `pops().is_empty()` and `endowed` empty on turn 0 and after 3 steps; the existing `tests/world.rs` assertions on `turns_valid` pass unchanged (add `assert!(report.endowed.is_empty())` where the reports are checked).
- `pop_stockpile` with a `PopId` from another world's `Pops` panics with `is not from this list` (take an ID with index ≥ 4 from the sample and use it on the `pops_placed` world).

**`crates/engine/tests/world.rs`**: `sample_setting_at_turn_0` and `sample_setting_steps` updated for `perished` (labor first) and `endowed`; `worlds_without_deposits_stay_empty` also asserts `endowed` empty.

**Dev UI**: `checker.rs` passes with the updated success and turn lines (**AC "checker counts POPs; zero reported; `endowed` on every line; `nothing` when none"**), including the new `pops_placed` test (success line, then `--turns 1` gives the two lines above). `session.rs` tests pass with the updated lines (the window's log is the same `turn_line`).

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader`, `coordinate` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`.
- `Pops::from_pops`, `Stockpile::add`, `IdVec` stay `pub(crate)`. `Pop`'s fields are public (like `Deposit`'s); `Pops`'s are private.
- Do not change `HexMap`, `HexCurrencies`, `HexPrices`, `Market`, `PopType`, deposits or where they emit, the dev UI window's panels (the Goods table's `Held` column keeps summing hex stockpiles only; Step 08 shows POPs), or the checker's output beyond the POP count and the `endowed` part.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: starting inventory or cash for POPs, demand, trade, consumption, growth, changing a POP's hex, type or size, deposit ownership, showing POPs in the dev UI window.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line and turn lines, and every Acceptance Criterion in `design.md` is met.
