# Step 07: pop-growth — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-07-pop-growth/design.md` — the behavior this spec implements, the worked examples, and the Acceptance Criteria.
- `PROJECT_PLAN/stage-02-populate-the-world/step-06-pop-trade/spec.md` and `docs_changes.md` — the trade round as built (`Submission`, `PopOutcome`, `hex_round`, `World::trade`, the `pops_trade` test setting and its expected table).
- `PROJECT_PLAN/stage-02-populate-the-world/step-05-stone-geary-demand/spec.md` — `Demand`, `DemandedGood` (`held`, `quantity`, `subsistence`, `below_subsistence`), `income`, `subsistence_cost`, `supernumerary`.
- `PROJECT_PLAN/stage-02-populate-the-world/step-03-pop-types/spec.md` — the `[[pop_type]]` reader, the `EntryReader` accessors it added (`nested`, `required_table_of_tables`, `optional_number`, `has`), the `pops_errors` setting and its expected error list.
- `docs/setting-loader.md` (the two phases, `EntryReader`, error order), `docs/world.md` (the turn, `TurnReport`, the checker's turn line), `docs/workspace.md` (lints, the check command, test layout), `docs/dev-ui.md` (the turn log).
- Code: `crates/engine/src/pops.rs` (`PopType`, `PopType::new`, `UtilityGood`, `Pop`, `Pops`), `crates/engine/src/setting/pops.rs` (`PopTypesReader::read`, `PopTypeEntry`, `finish`), `crates/engine/src/setting/reader.rs` (`EntryReader`: `find`, `wrong_type`, `number_value`, `required_number_table`, `required_table_of_tables`, `nested`, `finish`, and its unit-test `run` helper), `crates/engine/src/demand.rs` (`Demand`, `demand`, and the test `Prices` price list), `crates/engine/src/trade.rs` (`hex_round` pass 4, `PopOutcome`), `crates/engine/src/world.rs` (`new`, `step`, `trade`, `TurnReport`), `crates/engine/src/quantity.rs`, `crates/engine/src/id.rs` (`IdVec::iter`, `get_mut`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/pops.rs`, `crates/engine/tests/trade.rs`, `crates/engine/tests/world.rs`, `crates/engine/tests/settings/README.md`, every test setting with a `[[pop_type]]` (listed under Changes), `crates/dev-ui/src/text.rs` (`turn_line`), `crates/dev-ui/src/session.rs` (tests assert turn lines), `crates/dev-ui/tests/checker.rs`, `settings/sample/rules/pops.toml`, `settings/sample/start/pops.toml`.

## Approach

- **A new pure module `crates/engine/src/growth.rs`** holds the math: `subsistence_index(&Demand) -> f64`, `growth_rate(&GrowthParams, index) -> f64`, and `grown_size(size, rate) -> Quantity`. Like `demand.rs`, it knows nothing about the World or files, and has unit tests of its own. Rationale: the index and the rate are formulas over numbers that already exist; a module of pure functions is the easiest thing to test and the easiest for Stage 05 to revisit (which numbers the index reads).
- **The growth parameters are a `GrowthParams` on `PopType`**, next to the utility goods, built by a crate-private constructor that asserts what the loader guarantees. `PopType::new` gains a third argument. Rationale: they are per-type rules read from the same `[[pop_type]]` entry, exactly like the utility.
- **The trade round's record-building pass computes each POP's index and rate** (from the demand it just computed and the submission's type) and stores them in the `PopOutcome` with the starting size. **The World applies the rate to N** in a new `grow` step after every hex has traded, in POP ID order, and sums the population. Rationale (decided with the user): the outcome is complete the moment it is created, with no placeholder or setter; growth is still applied only after all trading, so the behavior is exactly the design's.
- **Two small `EntryReader` accessors**: `required_sub_table(key)` returns a nested reader for a table-valued key (label `growth of pop_type`, missing keys reported at the key's location), and `required_number_array(key)` reads an array of numbers element by element. Rationale: the `growth` sub-table then gets the standard wrong-type, missing-key and unknown-key handling and message forms for free.
- **`TurnReport` gains `population: Quantity`**, the dev UI's `turn_line` appends `; population N`.
- **Test settings**: `pops_trade` and `pops_placed` get zero-growth parameters so their multi-turn expectations hold; a new `pops_growth` setting carries the growth cases; every `pops_errors` entry gains a valid `growth` line (so the existing expected error lines move down) and a new `rules/c.toml` there holds every growth error case.
- **The sample grows**, so its turn 1 and later numbers (endowed, sold, bought, consumed, perished slivers, the turn lines) change. The design's AC says the sample's POPs grow; the tests that asserted the sample is the same every turn now follow the sizes.
- **The build runs as one.** The loader, the growth module, the trade record, the World and every test setting depend on the same `PopType` change, so no package could compile and test alone.

## Changes

### New

- `crates/engine/src/growth.rs` — `subsistence_index`, `growth_rate`, `grown_size`, unit tests.
- `crates/engine/tests/growth.rs` — integration tests (Test Plan).
- `crates/engine/tests/settings/pops_growth/` — a test setting for growth (described under Interfaces & Data).
- `crates/engine/tests/settings/pops_errors/rules/c.toml` — every growth error case.

### Modified

- `crates/engine/src/pops.rs` — `GrowthParams` (struct, `new`, accessors), `PopType.growth` field and `growth()` accessor, `PopType::new(endowment, utility, growth)`, `Pops::set_size` (crate-private); module doc (a POP's size grows each turn by a rate from its type's parameters; see `growth`); `Pop.size` doc; unit tests updated for the new argument, plus `GrowthParams` tests.
- `crates/engine/src/setting/pops.rs` — `PopTypeEntry.growth: Option<GrowthParams>`, a private `read_growth`, `read` calls it, `finish` passes it; module doc sentence.
- `crates/engine/src/setting/reader.rs` — `required_sub_table`, `required_number_array`, a private `find_entry` that also returns the key's span; unit tests.
- `crates/engine/src/trade.rs` — `PopOutcome` gains `subsistence_index`, `growth_rate`, `starting_size` and accessors; `hex_round` pass 4 fills them; module doc sentence; its tests' `PopType::new` calls gain a growth argument.
- `crates/engine/src/demand.rs` — tests' `PopType::new` calls gain a growth argument; module doc: the `growth` module builds the subsistence index from a `Demand`.
- `crates/engine/src/world.rs` — `TurnReport.population`; `grow`; wiring in `new` and `step`; module, struct and field docs.
- `crates/engine/src/lib.rs` — `pub mod growth;` (alphabetical: after `goods`, before `id`) with `pub use growth::{grown_size, growth_rate, subsistence_index};`; add `GrowthParams` to the `pops` re-export; crate doc sentence.
- `crates/engine/tests/pops.rs`, `crates/engine/tests/trade.rs`, `crates/engine/tests/world.rs`, `crates/engine/tests/demand.rs` — updates listed in the Test Plan.
- Test settings: `pops_valid` (`rules/a.toml`, `rules/a/z.toml`, `rules/b.toml`), `pops_one/rules/pops.toml`, `pops_placed/rules/types.toml`, `pops_placed_errors/rules/types.toml`, `pops_trade/rules/pops.toml`, `pops_errors` (`rules/a.toml`, `rules/b.toml`, `start/p.toml`), `resources_broken_file/rules/r.toml` — every `[[pop_type]]` gains a `growth` sub-table (exact contents below).
- `crates/engine/tests/settings/README.md` — `pops_growth`, the growth parameters of the settings above, the new `pops_errors` cases.
- `crates/dev-ui/src/text.rs` — `turn_line` appends `; population {}`; doc comment.
- `crates/dev-ui/src/session.rs` — the two hardcoded turn lines in its tests.
- `crates/dev-ui/tests/checker.rs` — every hardcoded turn line.
- `settings/sample/rules/pops.toml` — a `[pop_type.growth]` table on each type, with comments; `settings/sample/start/pops.toml` — the header comment's "endowed 4100 unskilled_labor every turn" becomes "at turn 0; from then on the sizes grow or shrink (see rules/pops.toml)".

Nothing else changes: no `docs/`, no `manual.ipynb`, no `setting/mod.rs`, no `check.rs`, no `app.rs`, no `market.rs`, `stockpile.rs` or `demand.rs` logic.

## Interfaces & Data

### `crates/engine/src/pops.rs`

```rust
/// How a POP type's size responds to its subsistence index u (see the
/// [`growth`](crate::growth) module): a polynomial above subsistence, a linear decline
/// below, both clamped.
#[derive(Debug, Clone, PartialEq)]
pub struct GrowthParams {
    /// c₁, c₂, …: above subsistence the rate is c₁u + c₂u² + …. Never empty; every
    /// coefficient finite.
    coefficients: Vec<f64>,
    /// Below subsistence the rate is `decline × u`, so a POP with nothing (u = −1) shrinks
    /// by exactly this. Finite, in [0, 1).
    decline: f64,
    /// The cap on the rate. Finite, at least 0.
    max_growth: f64,
}

impl GrowthParams {
    /// Crate-private: only the loader (and tests) build parameters.
    ///
    /// # Panics
    /// Panics if `coefficients` is empty or holds a non-finite number
    /// (`"a POP type's growth coefficients must be a non-empty list of finite numbers"`),
    /// if `decline` is not finite or not in [0, 1) (`"a POP type's decline must be a finite
    /// number of at least 0 and less than 1"`), or if `max_growth` is not finite or is
    /// negative (`"a POP type's max_growth must be a finite number of at least 0"`).
    pub(crate) fn new(coefficients: Vec<f64>, decline: f64, max_growth: f64) -> Self
    pub fn coefficients(&self) -> &[f64]
    pub fn decline(&self) -> f64
    pub fn max_growth(&self) -> f64
}

pub struct PopType {
    endowment: Vec<Endowment>,
    utility: Vec<UtilityGood>,
    /// How the size responds to the subsistence index.
    growth: GrowthParams,
}

impl PopType {
    pub(crate) fn new(
        endowment: Vec<(GoodId, Quantity)>,
        utility: Vec<(GoodId, Quantity, f64)>,
        growth: GrowthParams,
    ) -> Self
    /// The growth parameters.
    pub fn growth(&self) -> &GrowthParams
}

impl Pops {
    /// Sets the size of `id`. Crate-private: only the World's growth step changes sizes.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub(crate) fn set_size(&mut self, id: PopId, size: Quantity)
}
```

`Pop.size` doc becomes: "N, the number of individuals. Continuous and finite; greater than 0 in the setting, and growth multiplies it by a factor above 0 each turn, so it stays positive (only floating-point underflow after thousands of turns of decline could reach 0)." Tests in `pops.rs` need a growth value for `PopType::new`: add a test helper `fn no_growth() -> GrowthParams { GrowthParams::new(vec![0.0], 0.0, 0.0) }` and pass it; `demand.rs` and `trade.rs` tests get the same helper.

### `crates/engine/src/growth.rs`

Module doc: POP growth. Each turn, after it has traded and consumed, a POP grows or shrinks by a rate that depends on its **subsistence index** u, a dimensionless measure built from the numbers its demand reports: 0 at exactly subsistence, positive above, negative (down to −1) below. The rate is a function of u with parameters on the POP type ([`GrowthParams`]): a polynomial with no constant term above subsistence, `decline × u` below, clamped to [−decline, max_growth]. Because `decline` is below 1, the rate is above −1 and N stays positive: the engine never removes a POP. The trade round ([`trade`](crate::trade)) records each POP's index and rate in its outcome; the World applies the rate to the size after every hex has traded. u is per individual (every input scales with N), so it doesn't depend on the POP's size. In this Stage the demanded and consumed quantities are the same; Stage 05 revisits which numbers the index reads once rationing exists. Knows nothing about the World or files.

```rust
use crate::demand::Demand;
use crate::pops::GrowthParams;
use crate::quantity::Quantity;

/// The subsistence index of a demand. Above subsistence (no good below it): (M − C) / C,
/// the extra subsistence bundles the income could buy, or 0 if C is 0. Below: the smallest
/// attainment (demanded quantity over subsistence need) over the goods with a positive
/// need, minus 1, so in [−1, 0). The two sides meet at 0.
pub fn subsistence_index(demand: &Demand) -> f64

/// The growth rate at `index`: the polynomial `c₁u + c₂u² + …` for `index >= 0`,
/// `decline × index` below, clamped to `[−decline, max_growth]`. Zero at index 0.
pub fn growth_rate(params: &GrowthParams, index: f64) -> f64

/// `size × (1 + rate)`, saturating at the largest finite quantity. `rate` is a rate from
/// [`growth_rate`], so it is above −1 and the result is finite and non-negative.
pub fn grown_size(size: Quantity, rate: f64) -> Quantity
```

`subsistence_index`: if `!demand.below_subsistence()`: `c = demand.subsistence_cost().get()`; if `c == 0.0` return `0.0`, else return `(demand.supernumerary() / c).clamp(0.0, f64::MAX)` (not below means M ≥ C, so this is already ≥ 0 and the lower bound only guards rounding; the upper bound keeps the index finite when C is tiny and M huge, so that the ratio overflows: an infinite index would make Horner start with `0 × ∞ = NaN`, and `grown_size` would panic). Else: `demand.goods().iter().filter(|g| g.subsistence > Quantity::ZERO).map(|g| g.quantity.get() / g.subsistence.get()).fold(f64::INFINITY, f64::min) - 1.0`. Below subsistence means some good has `quantity < subsistence`, so the filter is non-empty and the minimum is below 1; `debug_assert!((-1.0..0.0).contains(&index))`. Note `DemandedGood::subsistence` is already the type's amount times N, and `quantity` is also for the whole POP, so the ratio is per individual.

`growth_rate`: `let rate = if index < 0.0 { params.decline() * index } else { params.coefficients().iter().rev().fold(0.0, |acc, c| acc * index + c) * index };` (Horner; for `[c1, c2]` this is `c1·u + c2·u²`; at `index == 0.0` it is exactly 0; the index is always finite (see `subsistence_index`), so with finite coefficients it is never NaN, at worst ±infinity, which the clamp fixes) then `rate.clamp(-params.decline(), params.max_growth())` (`−decline ≤ 0 ≤ max_growth` always holds, so `clamp` can't panic).

`grown_size`: `size.saturating_mul(Quantity::new(1.0 + rate).expect("1 + rate is finite and positive"))`.

### `crates/engine/src/setting/reader.rs`

```rust
/// Reads `key` as a table and returns a reader for it, labelled `<key> of <label>` (for
/// example `growth of pop_type`), so its messages read `key 'decline' in growth of pop_type
/// must be a number, found string` and `unknown key 'cap' in growth of pop_type`. Missing
/// required keys inside it are reported at `key`'s own location (the key in an inline
/// entry, or its name in a `[section.key]` header). A missing `key` reports the usual
/// missing-key error; a value that isn't a table reports `key '<key>' in <label> must be a
/// table, found <type>`. Both return None. Call `finish` on the returned reader to report
/// its unknown keys.
pub fn required_sub_table(&mut self, key: &str) -> Option<EntryReader<'_, 'i>>

/// Reads `key` as an array of numbers (integers or floats, as `f64`). Returns the array's
/// location and every element in order: `Some(value)` for a number (as written, including
/// `inf` and `nan`; range checks belong to the caller), `None` for an element that isn't
/// one, which is reported at the element's position as `element <n> of key '<key>' in
/// <label> must be a number, found <type>` or `element <n> of key '<key>' in <label> is out
/// of range`, with `n` counted from 1. An empty array returns an empty list. A missing
/// `key` reports the usual missing-key error; a value that isn't an array reports
/// `key '<key>' in <label> must be an array, found <type>`. Both return None.
#[allow(clippy::type_complexity)]
pub fn required_number_array(&mut self, key: &str) -> Option<Located<Vec<Option<Located<f64>>>>>
```

Implementation notes: add a private `find_entry(&mut self, key, required) -> Option<(&'a Spanned<Cow<'i, str>>, &'a Spanned<DeValue<'i>>)>` (the key as `table.iter()` yields it, so `k.span()` works as in `finish`) that does what `find` does but returns the key too; `find` can call it. `required_sub_table`: `find_entry(key, true)`; `value.get_ref().as_table()` else `wrong_type(key, "a table", value)` and None; build `EntryReader { file: self.file, table, entry: self.file.location_of(k.span()), missing_at: MissingAt::Entry, label: format!("{key} of {}", self.label), read: Vec::new(), sink: &mut *self.sink }` (the same shape as `nested`). `required_number_array`: `find(key, true)`; `value.get_ref().as_array()` (`Option<&DeArray<'i>>`, a `Vec<Spanned<DeValue<'i>>>`) else `wrong_type(key, "an array", value)` and None; `at = self.file.location_of(value.span())`; for each `(i, element)`: `prefix = format!("element {} of key '{key}' in {}", i + 1, self.label)` and `self.number_value(&prefix, element)`. Existing accessors and messages are unchanged.

### `crates/engine/src/setting/pops.rs`

```rust
struct PopTypeEntry {
    ...existing fields...,
    /// None if the `growth` sub-table or anything in it was invalid (already reported).
    growth: Option<GrowthParams>,
}

/// Reads the `growth` sub-table of one `[[pop_type]]`: every key is read before anything
/// is decided, so one bad sub-table reports all of its problems.
fn read_growth(reader: &mut EntryReader<'_, '_>) -> Option<GrowthParams>
```

`read_growth`: `let mut sub = reader.required_sub_table("growth")?;` then, in this order, all before returning:

1. `coefficients`: `sub.required_number_array("growth")` → None → invalid. `Some(list)`: if `list.value.is_empty()` report `key 'growth' in {label} must have at least one coefficient` at `list.at` and invalid; else every element must be `Some(v)` with `v.value.is_finite()`; an element that is `None` is already reported; a `Some(v)` that isn't finite reports `element {n} of key 'growth' in {label} must be a finite number` at `v.at` (n from 1). Valid only if every element is valid.
2. `decline`: `sub.required_number("decline")` → `Some(v)` with `v.value.is_finite() && v.value >= 0.0 && v.value < 1.0`, else report `key 'decline' in {label} must be a finite number of at least 0 and less than 1` at `v.at` (only when the value was a number; a missing key or wrong type is already reported).
3. `max_growth`: `sub.required_number("max_growth")` → `Some(v)` with `v.value.is_finite() && v.value >= 0.0`, else report `key 'max_growth' in {label} must be a finite number of at least 0` at `v.at`.
4. `sub.finish()` (unknown keys).
5. If all three are valid, `Some(GrowthParams::new(coefficients, decline, max_growth))`, else None.

`{label}` is `sub.label()`, i.e. `growth of pop_type`. In `read`, call `let growth = read_growth(reader);` after the utility is read and before the name is defined (every key is read before defining, as now; the entry is pushed whatever `growth` is, so a type with a bad `growth` still takes part in the duplicate check). `finish`: `entry.growth.expect("internal error: a POP type's growth was invalid in a successful load")` passed to `PopType::new`. Module doc: `growth` is read with its own nested reader; its keys are fields (name-checked for unknown keys), unlike the data keys of `endowment` and `utility`.

### `crates/engine/src/trade.rs`

```rust
pub struct PopOutcome {
    ...existing fields...,
    /// The POP's subsistence index this turn (see the `growth` module).
    subsistence_index: f64,
    /// The growth rate the World applies to the size after the round.
    growth_rate: f64,
    /// The size the turn started with (the size the demand and the trade were for).
    starting_size: Quantity,
}

impl PopOutcome {
    /// The subsistence index: 0 at subsistence, positive above, down to −1 below.
    pub fn subsistence_index(&self) -> f64
    /// The growth rate applied to the size after this turn's round.
    pub fn growth_rate(&self) -> f64
    /// The size the turn started with; the POP's current size is the size after growth.
    pub fn starting_size(&self) -> Quantity
}
```

In `hex_round` pass 4, for each submission: `let subsistence_index = subsistence_index(&demand); let growth_rate = growth_rate(s.pop_type.growth(), subsistence_index);` and `starting_size: s.size`. Module doc: add that the record also carries the POP's subsistence index and growth rate, computed by the [`growth`](crate::growth) module from the demand and the type, and the starting size; the World applies the rate after every hex has traded.

### `crates/engine/src/world.rs`

```rust
pub struct TurnReport {
    ...existing fields...,
    /// The total size of every POP after growth in `turn`. Zero for a world with no POPs.
    pub population: Quantity,
}

impl World {
    /// Part 2 of the turn, after `trade`: every POP, in ID order, grows by the rate its
    /// outcome records (`size × (1 + rate)`). Returns the total population after growth.
    fn grow(&mut self) -> Quantity
}
```

`grow`: `let mut population = Quantity::ZERO; for (id, outcome) in self.pop_outcomes.iter() { let size = grown_size(self.pops.get(id).size, outcome.growth_rate()); self.pops.set_size(id, size); population = population.saturating_add(size); } population`. `pop_outcomes` is read and `pops` written as separate fields, so the borrows don't conflict. `new` and `step`: after `trade`, `let population = self.grow();` into the report. Docs: the `pops` field ("Their sizes change in `grow`"), `World::new` ("runs turn 0's start, turn 0's trade round and turn 0's growth"), `World::pop_outcome` (add "its subsistence index, growth rate and starting size"), the module doc (part 2 is the trade round, consumption and then growth; `new` runs turn 0's growth too; between calls every POP has traded, consumed and grown; determinism adds "POPs grow in ID order"), the `World` doc list (the world holds each POP's current size, grown each turn), and the Stage-02 bullet ("POP size in Step 07" → sizes now change).

### `crates/dev-ui/src/text.rs`

```rust
// "Turn {}: perished {}; emitted {}; endowed {}; sold {}; bought {}; consumed {}; population {}"
```

`population` is `report.population` with `Quantity`'s `Display` (so `0` for a world without POPs). Doc comment: add "and the total population after growth".

### The sample setting

`settings/sample/rules/pops.toml`: extend the header comment with the `growth` sub-table (three required keys: `growth`, a list of one or more polynomial coefficients c₁, c₂, … giving the rate c₁u + c₂u² + … above subsistence, where u is the subsistence index, the number of extra subsistence bundles the POP's income could buy; `decline`, a number from 0 up to but not including 1, the rate at u = −1 (a POP with nothing), applied linearly below subsistence; `max_growth`, the cap on growth; the rate is clamped to [−decline, max_growth]; `growth = [0]` and `decline = 0` freeze a type's size). Then add to **both** types, after their utility tables (comment wording free, values exact):

```toml
# Growth peaks at u = 1 (3% a turn), crosses zero at u = 2, and is capped at 5%. A POP
# with nothing shrinks 10% a turn. A peasant at these prices (u = 2/3) grows about 2.7%.
[pop_type.growth]
growth = [0.06, -0.03]
decline = 0.1
max_growth = 0.05
```

Expected turn 0: every market-hex peasant POP has index 2/3, rate `0.06 × 2/3 − 0.03 × 4/9 = 0.02666…` (= 0.04 − 0.04/3) and size `N₀ × 1.02666…`; the center artisan (POP 8) index 1, rate 0.03, size 10.3; the north-east peasant (POP 2, size 12) index −1, rate −0.1, size 10.8. `population` = 388 × 1.02666… + 10.3 + 10.8 ≈ 419.44667. After one step: the rates repeat (u depends on prices and the type only; the silver sliver carried over moves it by ~1e-15), so sizes are `N₀ × 1.02666…²`, 10.609 and 9.72; `endowed` at turn t is 10 × the previous report's `population`; `sold`, `bought` and `consumed` are the per-individual amounts (7.2, 3.6, 2.8 for a peasant; 8.5, 3 grain + 0.5 copper, 1.5 + 3 + 0.5 for the artisan; 10 consumed for the no-market peasant) times each POP's current size. The turn lines for turns 1 and 2 therefore differ from each other now; the builder pastes them from the checker.

### The `pops_growth` test setting (`crates/engine/tests/settings/pops_growth/`)

`setting.toml`: `name = "Pops Growth"`, `format_version = 1`.

`rules/goods.toml`, in this order (IDs 0–3): `labor` (`perishes_after = 1`), `grain` (`perishes_after = 3`), `copper` (durable), `silver` (durable).

`rules/pops.toml` (inline tables; `G = { growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }`):

| Type | Endowment | Utility | growth |
|---|---|---|---|
| `peasant` | labor 10 | labor { subsistence 2, share 0.2 }, grain { subsistence 2, share 0.8 } | G |
| `boomer` | labor 10 | labor { subsistence 1, share 1 } | `{ growth = [1], decline = 0.1, max_growth = 0.05 }` |
| `rich` | labor 10 | labor { subsistence 2.5, share 1 } | G |
| `richer` | labor 10 | labor { subsistence 1, share 1 } | G |
| `carefree` | labor 10 | labor { share 1 } | `{ growth = [0.06], decline = 0.1, max_growth = 0.05 }` |
| `wanting` | labor 10 | labor { share 1 }, copper { subsistence = 1 } | `{ growth = [0.06], decline = 0.1, max_growth = 0.05 }` |
| `static` | labor 10 | labor { subsistence 2, share 0.2 }, grain { subsistence 2, share 0.8 } | `{ growth = [0], decline = 0, max_growth = 0 }` |

`start/map.toml`: hexes (0, 0), (1, 0), (2, 0), every one with `currency = "silver"`. `start/markets.toml`: (0, 0) `labor = 1, grain = 2`; (1, 0) `labor = 1, grain = 4`; (2, 0) `labor = 1, grain = 10`. No hex prices copper.

`start/pops.toml`, in this order (IDs 0–9): 0 `peasant` (0, 0) size 20; 1 `boomer` (0, 0) 10; 2 `rich` (0, 0) 10; 3 `richer` (0, 0) 10; 4 `carefree` (0, 0) 10; 5 `wanting` (0, 0) 10; 6 `static` (0, 0) 10; 7 `peasant` (1, 0) 10; 8 `peasant` (2, 0) 11; 9 `static` (2, 0) 10.

Expected at turn 0 (M, C per POP; index u; rate; size after growth):

| POP | Why | u | rate | size |
|---|---|---|---|---|
| 0 peasant ×20 | M 200, C 120 | 2/3 | 0.02666… | 20.5333… |
| 1 boomer | M 10, C 1: polynomial 9, capped | 9 | 0.05 | 10.5 |
| 2 rich | M 10, C 2.5: 0.18 − 0.27 | 3 | −0.09 | 9.1 |
| 3 richer | M 10, C 1: 0.54 − 2.43 = −1.89, clamped | 9 | −0.1 | 9 |
| 4 carefree | C 0 | 0 | 0 | 10 |
| 5 wanting | copper untradable and unheld: attainment 0, whatever M is | −1 | −0.1 | 9 |
| 6 static | as POP 0, zero parameters | 2/3 | 0 | 10 |
| 7 peasant at grain 4 | M 100, C 100 | 0 | 0 | 10 |
| 8 peasant at grain 10 ×11 | M 110, C 242, below: M/C − 1 | −6/11 | −0.6/11 ≈ −0.054545 | 10.4 |
| 9 static at grain 10 | below, zero parameters | −6/11 | 0 | 10 |

`starting_size` is the size from the setting. `population` at turn 0 = 108.5333… (= 108 + 8/15). After one step every rate repeats, so the sizes are 20 × 1.02666…² ≈ 21.08089, 11.025, 8.281, 8.1, 10, 8.1, 10, 10, 10.4²/11 ≈ 9.832727, 10; `population` ≈ 106.41962; the turn's `endowed` is labor 10 × 108.5333… = 1085.333….

### The other test settings

Append to **every** `[[pop_type]]` entry, as its last line, `growth = { growth = [0], decline = 0, max_growth = 0 }` in: `pops_one/rules/pops.toml`, `pops_placed/rules/types.toml` (all three types), `pops_placed_errors/rules/types.toml`, `pops_trade/rules/pops.toml` (all six types). With zero parameters no size changes, so every existing expectation of those settings holds; `population` is 1 for `pops_one`, 12.5 for `pops_placed`, 11 for `pops_trade`.

`resources_broken_file/rules/r.toml`: the `ghost` type gains `growth = { growth = [0.03], decline = 0.1, max_growth = 0.05 }`; its expected errors stay exactly the one syntax error.

`pops_valid` (the loader reads real values, in both forms): `rules/a.toml` `farmer` gains the line `growth = { growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }`; `rules/a/z.toml` `fisher` gains, after its utility tables,
```toml
[pop_type.growth]
growth = [0.03]
decline = 0
max_growth = 1
```
`rules/b.toml` `idle` gains `growth = { growth = [0.02, 0.01, -0.005], decline = 0.5, max_growth = 0.2 }`.

`pops_errors`: append `growth = { growth = [0.03], decline = 0.1, max_growth = 0.05 }` as the **last line of every entry** in `rules/a.toml` (20 entries; the file becomes 118 lines), `rules/b.toml` and `start/p.toml`, changing nothing else. Each entry's lines move down by the number of entries before it. Add `rules/c.toml`, exactly (90 lines):

```toml
[[pop_type]]
name = "no_growth"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "growth_not_table"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = 5

[[pop_type]]
name = "growth_missing_keys"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = {}

[[pop_type]]
name = "growth_list_not_array"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = 0.03, decline = 0.1, max_growth = 0.05 }

[[pop_type]]
name = "growth_list_empty"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [], decline = 0.1, max_growth = 0.05 }

[[pop_type]]
name = "growth_list_bad_elements"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.06, "x", inf], decline = 0.1, max_growth = 0.05 }

[[pop_type]]
name = "decline_negative"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = -0.1, max_growth = 0.05 }

[[pop_type]]
name = "decline_one"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = 1, max_growth = 0.05 }

[[pop_type]]
name = "decline_infinite"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = inf, max_growth = 0.05 }

[[pop_type]]
name = "decline_not_number"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = "x", max_growth = 0.05 }

[[pop_type]]
name = "max_growth_negative"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = 0.1, max_growth = -1 }

[[pop_type]]
name = "max_growth_infinite"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = 0.1, max_growth = inf }

[[pop_type]]
name = "growth_unknown_key"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [0.03], decline = 0.1, max_growth = 0.05, cap = 1 }

[[pop_type]]
name = "growth_sub_table_form"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[pop_type.growth]
decline = 0.1

[[pop_type]]
name = "ok"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
growth = { growth = [], decline = 2, max_growth = -1, cap = 1 }
```

The last entry duplicates `ok` from `rules/a.toml` and has four growth problems: it checks that one sub-table reports every problem in one load and that a type with a bad `growth` still takes part in the duplicate check.

Expected error lines for `pops_errors` (`help` is the usual invalid-name suffix; the `rules/a.toml` lines are the existing ones moved down, with the same columns):

```
rules/a.toml:7:1: missing required key 'name' in pop_type
rules/a.toml:12:1: missing required key 'endowment' in pop_type
rules/a.toml:17:1: missing required key 'utility' in pop_type
rules/a.toml:23:8: invalid name 'Bad Name' {help}
rules/a.toml:29:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)
rules/a.toml:36:13: key 'endowment' in pop_type must be a table, found integer
rules/a.toml:43:11: key 'utility' in pop_type must be a table, found string
rules/a.toml:48:23: key 'grain' in endowment of pop_type must be a number, found string
rules/a.toml:48:35: key 'coin' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:54:23: key 'grain' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:54:34: key 'coin' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:61:21: key 'grain' in utility of pop_type must be a table, found integer
rules/a.toml:67:37: key 'subsistence' in utility.grain of pop_type must be a finite number of at least 0
rules/a.toml:67:77: key 'subsistence' in utility.coin of pop_type must be a number, found string
rules/a.toml:73:31: key 'share' in utility.grain of pop_type must be a finite number of at least 0
rules/a.toml:79:36: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:85:36: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:88:1: utility of pop_type needs at least one good with a positive share
rules/a.toml:94:1: utility of pop_type needs at least one good with a positive share
rules/a.toml:102:15: unknown good 'tin'
rules/a.toml:103:13: unknown good 'tin'
rules/a.toml:109:34: unknown key 'weight' in utility.grain of pop_type
rules/a.toml:110:1: unknown key 'colour' in pop_type
rules/a.toml:114:8: invalid name 'Several' {help}
rules/a.toml:115:15: unknown good 'tin'
rules/a.toml:116:13: unknown good 'tin'
rules/a.toml:116:29: key 'share' in utility.tin of pop_type must be a finite number of at least 0
rules/a.toml:116:33: unknown key 'weight' in utility.tin of pop_type
rules/a.toml:116:47: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:117:1: unknown key 'colour' in pop_type
rules/b.toml:2:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)
rules/c.toml:1:1: missing required key 'growth' in pop_type
rules/c.toml:10:10: key 'growth' in pop_type must be a table, found integer
rules/c.toml:16:1: missing required key 'growth' in growth of pop_type
rules/c.toml:16:1: missing required key 'decline' in growth of pop_type
rules/c.toml:16:1: missing required key 'max_growth' in growth of pop_type
rules/c.toml:22:21: key 'growth' in growth of pop_type must be an array, found float
rules/c.toml:28:21: key 'growth' in growth of pop_type must have at least one coefficient
rules/c.toml:34:28: element 2 of key 'growth' in growth of pop_type must be a number, found string
rules/c.toml:34:33: element 3 of key 'growth' in growth of pop_type must be a finite number
rules/c.toml:40:39: key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1
rules/c.toml:46:39: key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1
rules/c.toml:52:39: key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1
rules/c.toml:58:39: key 'decline' in growth of pop_type must be a number, found string
rules/c.toml:64:57: key 'max_growth' in growth of pop_type must be a finite number of at least 0
rules/c.toml:70:57: key 'max_growth' in growth of pop_type must be a finite number of at least 0
rules/c.toml:76:63: unknown key 'cap' in growth of pop_type
rules/c.toml:83:11: missing required key 'growth' in growth of pop_type
rules/c.toml:83:11: missing required key 'max_growth' in growth of pop_type
rules/c.toml:87:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)
rules/c.toml:90:21: key 'growth' in growth of pop_type must have at least one coefficient
rules/c.toml:90:35: key 'decline' in growth of pop_type must be a finite number of at least 0 and less than 1
rules/c.toml:90:51: key 'max_growth' in growth of pop_type must be a finite number of at least 0
rules/c.toml:90:55: unknown key 'cap' in growth of pop_type
start/p.toml:1:3: section 'pop_type' belongs in rules/, not start/
```

Errors at the same position keep report order (the sink's sort is stable), so the three missing keys at `16:1` come out `growth`, `decline`, `max_growth` because `read_growth` reads them in that order. If the actual output differs, check the implementation against this spec before changing the list; the message forms are fixed. Columns of inline-table keys and values come from the TOML spans (the first character of the key or value); if `toml` reports a span differently than assumed here, adjust the columns only.

## Tasks

1. `pops.rs`: `GrowthParams`, `PopType.growth` and `growth()`, the new `PopType::new` argument, `Pops::set_size`, docs; update its unit tests and add the `GrowthParams` tests (Test Plan). Fix the `PopType::new` calls in `demand.rs` and `trade.rs` tests with a `no_growth()` helper. Register `GrowthParams` in `lib.rs`.
2. `growth.rs`: the three functions with docs and unit tests (Test Plan). Register in `lib.rs` with the re-exports and the crate-doc sentence; the `demand.rs` module-doc sentence.
3. `reader.rs`: `find_entry`, `required_sub_table`, `required_number_array`, docs, unit tests (Test Plan).
4. `setting/pops.rs`: `read_growth`, the entry field, `read` and `finish`, module doc.
5. Test settings: every `[[pop_type]]` gains its `growth` as specified; `pops_growth` and `pops_errors/rules/c.toml` as specified; `tests/settings/README.md` (add `pops_growth` with its types, hexes, prices and POPs; note the zero-growth parameters of `pops_one`, `pops_placed`, `pops_placed_errors` and `pops_trade`; the real values in `pops_valid`; the growth cases in `pops_errors`). The sample's `rules/pops.toml` and `start/pops.toml`.
6. `trade.rs`: the three `PopOutcome` fields and accessors, pass 4, module doc.
7. `world.rs`: `TurnReport.population`, `grow`, wiring in `new` and `step`, docs. The unit test `a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged` also compares every POP's size before and after the refused step.
8. `text.rs`: `turn_line` and its doc. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and `... check crates/engine/tests/settings/pops_placed --turns 1`, confirm every printed amount is within 1e-9 of the expectations in this spec, and paste the printed lines into `session.rs` and `checker.rs` (Test Plan).
9. `crates/engine/tests/growth.rs`, and the updates to `tests/pops.rs`, `tests/trade.rs`, `tests/world.rs` (Test Plan).
10. Run `.venv/Scripts/python tools/check.py` (fmt, clippy with `-D warnings`, all tests) and fix everything it reports. Every public item needs a doc comment.

## Test Plan

Tolerance: `close(a, b)` is `(a − b).abs() < 1e-9`. Compare every index, rate, size and population with `close`, never exactly, except where the spec says "exactly". Reuse the helper style of `tests/trade.rs` (`world_of`, `hex`, `good`, `pop_id`, `outcome`, `list`, `assert_list`); `tests/growth.rs` gets its own copies plus `size(world, index) -> f64` (`world.pops().get(pop_id(..)).size.get()`).

### Unit tests: `growth.rs`

Build demands with `demand()` and a hand-written `PriceList` as `demand.rs`'s tests do (copy its `Prices` struct and `prices` helper, with goods `LABOR 0`, `GRAIN 1`, `COPPER 2`, `SILVER 3` and `SILVER` as the currency); types with `PopType::new`; `G = GrowthParams::new(vec![0.06, -0.03], 0.1, 0.05)`.

| Test | Criterion | Setup → expectation |
|---|---|---|
| `index_above_subsistence_counts_extra_bundles` | AC 2 | peasant (labor 2/0.2, grain 2/0.8) holding labor 10 at labor 1, grain 2 → `2/3`; artisan-like (labor 1/1, grain 2/4, copper 0/5) at labor 1, grain 2, copper 5 → `1.0` |
| `index_is_zero_at_exactly_subsistence` | AC 4 | peasant, labor 10, grain at 4 → `0.0` exactly |
| `index_is_zero_when_subsistence_costs_nothing` | AC 6 | type labor { share 1 } holding labor 10 → `0.0`; type with no subsistence holding nothing (M 0, C 0) → `0.0` |
| `index_below_subsistence_is_the_shortfall` | AC 4 | peasant, labor 10, grain at 10 → `10/22 − 1` |
| `index_is_minus_one_for_an_unobtainable_need` | AC 6 | type labor { share 1 }, copper { subsistence 1 } holding labor 10 at labor 1 (copper unpriced) → `−1.0`; the same with labor 1000 → `−1.0`; peasant on a hex with no prices → `−1.0` |
| `index_takes_the_worst_attainment` | design | type labor { subsistence 2, share 1 }, copper { subsistence 1 } holding labor 10 and copper 0.5, copper unpriced → copper attainment 0.5, labor fine → `−0.5` |
| `index_stays_finite_when_the_ratio_overflows` | robustness | type labor { subsistence 1e-160, share 1 } holding labor 1e300 at labor 1, size 1e-160 → index is finite (`f64::MAX`); `growth_rate(G, index)` is exactly −0.1 (G's quadratic term sends the polynomial to −∞, and the clamp gives the floor), and `grown_size` doesn't panic |
| `index_does_not_depend_on_size` | design | peasant at sizes 1 and 50 (holding 10 and 500 labor) → equal indexes, above and below subsistence |
| `rate_follows_the_polynomial_above_subsistence` | AC 2 | `G`: at 2/3 → `0.04 − 0.04/3`; at 1 → 0.03; at 2 → 0 (close); at 3 → −0.09; single coefficient `[0.03]` with max 1 at 2 → 0.06 |
| `rate_is_zero_at_subsistence` | AC 4 | `G` and `[1, 2, 3]` at 0 → exactly `0.0` |
| `rate_is_capped_and_floored` | AC 5 | `[1]`, decline 0.1, max 0.05 at 9 → exactly 0.05; `G` at 9 (polynomial −1.89) → exactly −0.1; `[1e300, 1e300]` at 1e10 (overflows to infinity) → 0.05 |
| `rate_declines_linearly_below_subsistence` | AC 4 | decline 0.1: at −0.5 → −0.05; at −1 → exactly −0.1; at `10/22 − 1` → `−0.6/11` |
| `zero_parameters_give_a_zero_rate` | AC 7 | `[0]`, 0, 0 at −1, −0.5, 0, 0.5, 9 → 0 |
| `grown_size_multiplies_and_saturates` | AC 2 | 20 at `0.04 − 0.04/3` → 20.5333…; 12 at −0.1 → 10.8; 10 at 0 → exactly 10; `f64::MAX` at 0.05 → `f64::MAX` |

### Unit tests: `pops.rs`

`growth_params_are_stored` (coefficients, decline and max_growth come back as given; `PopType::growth()` returns them); `#[should_panic]` for empty coefficients, a `NaN` coefficient, `decline` of 1, `decline` of −0.1, `decline` of `inf`, `max_growth` of −1, each with the message in Interfaces & Data; `set_size_changes_one_pop` (on `sample_pops()`, set POP 2's size to 7, read it back, others unchanged).

### Unit tests: `reader.rs` (the existing `run` helper)

| Test | Expectation |
|---|---|
| `sub_tables_read_with_their_own_label_and_location` | `g = { a = 1, z = 2 }\nq = 7\n`: `required_sub_table("g")` gives a reader with `label() == "g of thing"`; inside it `required_number("a")` is 1, `required_number("b")` is None, `finish()`; `required_sub_table("q")` is None; `required_sub_table("missing")` is None. Errors: `rules/t.toml:1:1: missing required key 'b' in g of thing`, `rules/t.toml:1:1: missing required key 'missing' in thing`, `rules/t.toml:1:14: unknown key 'z' in g of thing`, `rules/t.toml:2:5: key 'q' in thing must be a table, found integer` |
| `sub_tables_work_in_header_form` | `[g]\na = 1\n`: `required_sub_table("g")` reads `a` = 1 with no errors; a missing key inside is reported at `rules/t.toml:1:2` |
| `number_arrays_give_every_element_and_report_bad_ones` | `a = [1, 2.5, "x", 99999999999999999999, inf]\nb = []\nc = 7\n`: `required_number_array("a")` → `at` (1, 5), five elements: 1.0, 2.5, None, None, `f64::INFINITY`; `"b"` → `Some` with an empty list and `at` (2, 5); `"c"` → None; `"missing"` → None. Errors: `rules/t.toml:1:1: missing required key 'missing' in thing`, `rules/t.toml:1:14: element 3 of key 'a' in thing must be a number, found string`, `rules/t.toml:1:19: element 4 of key 'a' in thing is out of range`, `rules/t.toml:3:5: key 'c' in thing must be an array, found integer` |

### Integration tests: `crates/engine/tests/pops.rs`

- `the_sample_setting_has_two_pop_types`: also assert both types' `growth()`: coefficients `[0.06, -0.03]` (close, elementwise), `decline` 0.1, `max_growth` 0.05 (**AC 1**).
- `pop_types_across_files_and_both_forms_get_ids_in_load_order`: also assert `farmer` `[0.06, -0.03]`/0.1/0.05, `fisher` `[0.03]`/0/1 (header form), `idle` `[0.02, 0.01, -0.005]`/0.5/0.2.
- `every_pop_type_error_is_reported_at_its_place`: the expected list becomes the one in Interfaces & Data (**AC 8**).
- `stepping_the_sample_re_endows_and_the_labor_is_used_up`: `endowed` at step t is `unskilled_labor` 10 × the previous report's `population` (keep the previous report; close), no longer 4100. Rename to `stepping_the_sample_re_endows_by_the_grown_sizes`.
- `the_pops_placed_world_endows_durable_and_perishable_goods`: also assert every report's `population` is exactly 12.5 and every POP's size is unchanged after 3 steps.
- `a_world_with_no_pops_has_none_and_endows_nothing`: also assert `population == Quantity::ZERO` at every turn.

### Integration tests: `crates/engine/tests/growth.rs`

| Test | Criterion | Expectation |
|---|---|---|
| `a_fresh_sample_world_has_grown_at_turn_0` | AC 2 | sample: for every peasant POP except POP 2: `subsistence_index` 2/3, `growth_rate` `0.04 − 0.04/3`, `starting_size` the setting's size, current size `starting × (1 + rate)`; POP 8: 1, 0.03, 10, 10.3; POP 2: −1, −0.1, 12, 10.8; `report.population` equals the sum of the current sizes and ≈ 419.44667 |
| `stepping_the_sample_grows_again` | AC 3 | after one step: POP 2 size 9.72, POP 0 size `20 × (1 + r)²`, POP 8 10.609; `report.endowed` is `unskilled_labor` 10 × turn 0's `population`; each POP's `sold`/`bought`/`consumed` per unit of `starting_size` equals turn 0's per unit of its starting size (close); after a second step POP 2 is 8.748 |
| `two_sample_worlds_grow_identically` | AC 3 | two sample worlds stepped 3 times: equal reports; after each step every POP's size (`assert_eq!`), outcome and stockpile equal between the worlds |
| `the_pops_growth_table_at_turn_0` | AC 4, 5, 6, 7 | every row of the table (index, rate, starting size, size); `population` ≈ 108.5333… |
| `the_pops_growth_table_after_a_step` | AC 3 | the sizes listed after one step; `population` ≈ 106.41962; `endowed` labor ≈ 1085.333… |
| `a_pop_at_subsistence_keeps_its_size` | AC 4 | POP 7: index exactly 0, rate exactly 0, size exactly 10 at turn 0 and after 2 steps |
| `a_poor_pop_shrinks_by_decline_times_its_shortfall` | AC 4 | POP 8: index `10/22 − 1`, rate `0.1 × (10/22 − 1)`, size 10.4 |
| `the_cap_and_the_floor_bound_the_polynomial` | AC 5 | POP 1 rate exactly 0.05, size 10.5; POP 2 rate −0.09, size 9.1; POP 3 rate exactly −0.1, size 9 |
| `zero_cost_and_unobtainable_needs` | AC 6 | POP 4: index 0, rate 0, size 10; POP 5: index exactly −1, rate exactly −0.1, size 9, `demand().income()` 100 |
| `zero_parameters_never_change_size` | AC 7 | POPs 6 and 9: rate exactly 0, size exactly 10 at turn 0 and after 3 steps, although POP 6's index is 2/3 and POP 9's is −6/11 |
| `worlds_without_pops_report_zero_population` | AC 9 | `turns_valid`, `valid_minimal`: `population == Quantity::ZERO` at turn 0 and after 2 steps |
| `zero_growth_settings_keep_their_sizes_and_totals` | AC 10 | `pops_trade`: every size unchanged after 3 steps, `population` exactly 11 every turn; `pops_placed`: 12.5 |

### Updates to existing tests

- `tests/trade.rs`:
  - `the_sample_report_totals_pops_in_good_id_order`: turn 0 unchanged. Turn 1's totals are no longer equal to turn 0's; instead compute them from the POPs' sizes after turn 0 (`s` = the sum of market-hex peasant sizes, `a` = the artisan's, `n` = POP 2's): `sold = [unskilled_labor 7.2 s + 8.5 a]`, `bought = [copper 0.5 a, grain 3.6 s + 3 a]`, `consumed = [unskilled_labor 2.8 s + 1.5 a + 10 n, copper 0.5 a, grain 3.6 s + 3 a]` (close). The perished assertion stays.
  - `stepping_repeats_the_outcomes_and_two_worlds_agree`: the two-worlds equality stays; "matches its turn-0 outcome" becomes "matches per individual": divide each list amount and `demand().income()` by the outcome's `starting_size` before comparing with turn 0's per-individual numbers. Rename to `stepping_repeats_the_outcomes_per_individual_and_two_worlds_agree`.
  - `the_pops_trade_setting_matches_the_table`, `currency_carried_over_is_income_next_turn`, `held_goods_outside_the_utility_stay_or_sell`: unchanged (zero growth).
  - `worlds_without_pops_report_nothing_traded`: also `population == Quantity::ZERO`.
- `tests/world.rs`: `sample_setting_steps`: `endowed` is 10 × the previous report's `population` (keep the previous report), not 4100. `reports_match_the_expected_numbers`, `worlds_without_deposits_stay_empty`: also assert `population == Quantity::ZERO` wherever they assert the other lists are empty. `identical_runs_give_identical_results`: unchanged (reports include `population`).
- `tests/demand.rs`: `the_recorded_demand_matches_the_pure_function` read the POP's current size, which after turn 0's growth is 61.6; it now reads `pop_outcome(..).starting_size()` (asserted 60) and passes that to the pure `demand` call. For the same reason `tests/trade.rs` `a_fresh_sample_world_has_traded_turn_0` and `the_no_market_peasant_consumes_its_labor_and_goes_without_grain`, and `tests/pops.rs` `a_fresh_sample_world_endows_every_pop`, take the size the round was for from `starting_size()` instead of `Pop.size` (the setting's sizes are still asserted exactly).
- `crates/dev-ui/src/session.rs` tests: the turn 0 line gains `; population <value>` where the value is the checker's print of ≈ 419.44667; the turn 1 line changes throughout (`endowed` ≈ 4194.4667, `sold`, `bought`, `consumed` from the sizes, `perished` still `10 unimproved_copper, 24 unimproved_grain` unless a sliver prints). Paste the printed lines.
- `crates/dev-ui/tests/checker.rs`: the `pops_placed` lines gain `; population 12.5`; `valid_minimal` and `turns_valid` lines gain `; population 0`; the sample test no longer has one `steady` line for turns 1 and 2: paste the three printed lines.

Run: `cargo test -p policyspace-engine`, `cargo test -p policyspace-dev-ui`, then `.venv/Scripts/python tools/check.py` for the gate.

## Constraints

- Follow the crate's conventions: doc comments on every public item (clippy runs with `-D warnings` and `missing_docs = "warn"`), every amount a `Quantity` with its checked and saturating arithmetic, no rounding anywhere. The index and the rate are `f64` because they can be negative.
- `growth.rs` imports only `demand`, `pops` and `quantity`; it knows nothing about `World`, `Stockpile`, `Market` or files. `trade.rs` additionally imports `growth`. `GrowthParams::new`, `PopType::new` and `Pops::set_size` stay `pub(crate)`.
- Don't change existing `EntryReader` message texts, `setting/mod.rs`, `check.rs`, `app.rs`, `market.rs`, `stockpile.rs`, the `demand` function, `docs/` or `manual.ipynb`. The checker changes only through `text::turn_line`.
- Don't remove POPs, round N, add a minimum size, read anything but the demand for the index, show growth in the dev UI (Step 08), or handle rationing (Stage 05).
- Don't commit.

## Done When

All tasks complete, `tools/check.py` passes (fmt, clippy, every test in the workspace), every test in the Test Plan passes, and every Acceptance Criterion in design.md is met.
