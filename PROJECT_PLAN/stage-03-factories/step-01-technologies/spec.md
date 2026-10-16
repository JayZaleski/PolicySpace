# Step 01: technologies — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-01-technologies/design.md` — the behavior this spec implements, including what every number means.
- `PROJECT_PLAN/stage-02-populate-the-world/step-03-pop-types/spec.md` — how a named `rules/` kind with tables of good references was added; this Step follows the same shape.
- `docs/setting-loader.md` — the loader's two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/pops.md` — `PopTypesReader` keeps every entry for phase 2; `PopType` has private fields and a crate-private constructor that asserts its invariants. This Step copies both.
- `docs/ids.md` — `NamedList`, `IdKind`, `Id`.
- `docs/dev-ui.md` — the checker's success line.
- `docs/workspace.md` — the check command and test layout.
- Code: `crates/engine/src/setting/pops.rs` (`PopTypesReader`: the pattern), `crates/engine/src/setting/goods.rs` (how `perishes_after` is range-checked: the "at least 1" / "out of range" messages), `crates/engine/src/setting/reader.rs` (`EntryReader`, `required_number_table`, `positive_quantity`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/setting/mod.rs` (`WorldContent`, `Rules`, `load_setting`), `crates/engine/src/pops.rs` (domain pattern), `crates/engine/src/id.rs` (`NamedList`), `crates/engine/src/quantity.rs`, `crates/engine/src/lib.rs`, `crates/engine/src/test_support.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/pops.rs` (`types_of`, `every_pop_type_error_is_reported_at_its_place`), `crates/engine/tests/goods.rs` and `crates/engine/tests/market.rs` (the sample tests that change), `crates/dev-ui/src/check.rs`, `crates/dev-ui/src/text.rs` (`market_lines_on_the_sample`), `crates/dev-ui/tests/checker.rs`, `settings/sample/rules/goods.toml`, `settings/sample/rules/pops.toml` (header comment style), `settings/sample/start/markets.toml`.

## Approach

- **A `[[technology]]` section in `rules/`**, read by a new `TechnologiesReader` (`setting/technologies.rs`) that follows `PopTypesReader`: the name goes through `Definitions<TechnologyKind>`, every key is read before anything is defined, every entry is kept for phase 2 (so an entry with a bad name still reports its unknown goods), and the good names in `inputs`, `output`, `capital` and `construction` are resolved in phase 2. Rationale: technologies are persistent rules identified by name with tables of good references, exactly like POP types.
- **A shared `Bundle` type in a new module `crates/engine/src/bundle.rs`**: goods with amounts, kept sorted by good ID, no good twice. A technology's three tables are `Bundle`s, and the later Steps of this Stage (a factory's current capital bundle, the depreciation slice) reuse it. Rationale: one definition for a concept four Steps use. Only what this Step needs goes in (construction, lookup, iteration); scaling and arithmetic wait for Step 05.
- **Domain types in a new module `crates/engine/src/technologies.rs`**: `TechnologyKind`/`TechnologyId`, `Technology` (private fields, accessors, a crate-private constructor that asserts the invariants, like `PopType`), and `Technologies = NamedList<TechnologyKind, Technology>`. The module knows nothing about files.
- **Keys**: `name`, `inputs`, `output`, `capacity`, `capital`, `construction`, `construction_turns`, `lifetime`. All eight are required; `inputs` and `construction` may be empty tables (written explicitly, like a POP type's empty `endowment`). Rationale: the loader is strict everywhere; an explicit `{}` says the author meant it.
- **`construction_turns` and `lifetime` are `u32`**, read with `required_integer` and range-checked like a good's `perishes_after`: below 1 is `must be at least 1`, above `u32::MAX` is `is out of range`.
- **The output-among-inputs check runs in phase 1 by comparing names as written**, so it is reported even when the goods are unknown. Rationale: good names are unique, so name equality is ID equality; and it reports more in one load.
- **A new `EntryReader::required_quantity_table`**: `required_number_table` plus `positive_quantity` on every value, so the three bundles share one accessor and the message prefix (`key '<k>' in <key> of <label>`) is built in one place. `PopTypesReader` and `MarketsReader` are not switched to it (that is consolidation's job).
- **`Rules` gains `technologies: Technologies`**, and the checker's success line gains `, N technologies` at the end. `World` holds `Rules` whole, so it is unchanged.
- **The build runs as one.** The reader needs the domain types and the `EntryReader` addition, and every test needs loaded settings, so no package could be built and tested alone.

## Changes

### New

- `crates/engine/src/bundle.rs` — `Bundle` and unit tests.
- `crates/engine/src/technologies.rs` — domain types and unit tests.
- `crates/engine/src/setting/technologies.rs` — `TechnologiesReader`.
- `crates/engine/tests/technologies.rs` — integration tests (Test Plan).
- Test settings `crates/engine/tests/settings/technologies_valid/`, `technologies_one/`, `technologies_errors/` (exact contents below).
- `settings/sample/rules/technologies.toml`.

### Modified

- `crates/engine/src/lib.rs` — `pub mod bundle; pub use bundle::Bundle;` first (alphabetical: before `currency`); `pub mod technologies; pub use technologies::{Technologies, Technology, TechnologyId, TechnologyKind};` after `stockpile`, before `trade`. Crate docs: a sentence that technologies (the fixed-proportion recipes factories run, with their capital and construction bundles, construction time and lifetime) are rules in the `technologies` module, that a `Bundle` (the `bundle` module) is a list of goods with amounts, and that nothing runs them yet.
- `crates/engine/src/setting/reader.rs` — `required_quantity_table`; a unit test.
- `crates/engine/src/setting/mod.rs` — `mod technologies;`, `Rules.technologies`, `WorldContent.technologies`, the `technology` section type, `resolve`, `load_setting`. Module docs: technologies are the fourth `rules/` kind (`[[technology]]`); the goods in their `inputs`, `output`, `capital` and `construction` are resolved in phase 2; the "Adding a section type" list names `crate::technologies` / `setting/technologies.rs` as another named kind that keeps every entry for phase 2.
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 technologies` at the end, except the sample (`7 goods` and `, 3 technologies`); a new test for `technologies_one` (`1 technology`). The turn lines don't change.
- `crates/dev-ui/src/text.rs` — `market_lines_on_the_sample` gains a last line `tools: 12 silver` for the center (prices are listed in good ID order; `tools` is ID 6). `good_rows_list_every_good_with_properties_and_world_totals` gains the `tools` row (name, `0` in hexes, `0` in POPs).
- `crates/engine/tests/goods.rs` — the sample test becomes seven goods, with `(6, "tools", good(Perishes::Never, true))` last; rename it `the_sample_setting_has_its_seven_goods_in_order`.
- `crates/engine/tests/market.rs` — `the_sample_setting_posts_its_prices`: the center also prices `tools` at 12 and `center.prices().count()` is 6; `tools` joins `non_currency`; the east hex doesn't price it.
- `crates/engine/tests/resources.rs` — comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a technology whose goods don't exist).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- Test setting `resources_broken_file/rules/r.toml` (append a `[[technology]]`, below).
- `settings/sample/rules/goods.toml` — the `tools` good.
- `settings/sample/start/markets.toml` — the center prices `tools`.
- `settings/sample/rules/README.md` — mention `technologies.toml`.

## Interfaces & Data

### `crates/engine/src/bundle.rs`

```rust
//! Module docs: A bundle is a list of goods with amounts: a technology's inputs per unit of
//! output, its capital bundle and its construction bundle, and later a factory's current
//! capital. Kept sorted by good ID with no good twice, so two bundles with the same goods
//! and amounts are equal and lookups are cheap. Amounts are `Quantity` (finite, at least
//! 0); whether 0 is allowed is up to the owner (a technology's bundles are all positive; a
//! factory's capital may wear down to 0 later). Knows nothing about files.

use crate::goods::GoodId;
use crate::quantity::Quantity;

/// Goods with amounts, sorted by good ID, no good twice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    items: Vec<(GoodId, Quantity)>,
}

impl Bundle {
    /// Builds a bundle from (good, amount) pairs in any order, sorting them by good ID.
    /// Crate-private: only the loader (and later the engine's own arithmetic) builds bundles.
    ///
    /// # Panics
    /// Panics if a good appears twice (`"a good appears twice in a bundle"`).
    pub(crate) fn new(items: Vec<(GoodId, Quantity)>) -> Self;
    /// The (good, amount) pairs, in good ID order.
    pub fn as_slice(&self) -> &[(GoodId, Quantity)];
    /// Every (good, amount), in good ID order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_;
    /// The amount of `good`; zero if the bundle doesn't name it.
    pub fn amount_of(&self, good: GoodId) -> Quantity;
    /// True if the bundle names `good` (even with amount 0).
    pub fn contains(&self, good: GoodId) -> bool;
    /// Number of goods named.
    pub fn len(&self) -> usize;
    /// True if no good is named.
    pub fn is_empty(&self) -> bool;
}
```

`new`: `sort_by_key(|x| x.0)` then assert no two adjacent goods are equal. `amount_of` and `contains` may use `binary_search_by_key`.

### `crates/engine/src/technologies.rs`

```rust
//! Module docs: Technologies. A technology (`[[technology]]` in `rules/`) is a named recipe
//! that factories run, shared by every factory that uses it, as a POP type is shared by
//! POPs: the inputs one unit of output takes (fixed proportions, no substitution; the table
//! may be empty), the one output good, the capacity (the most output a factory makes in a
//! turn), the capital bundle (the goods a finished factory holds; at least one good), the
//! construction bundle (the goods spent to build a factory, consumed, not kept; may be
//! empty), the construction turns t and the lifetime t₁ (whole numbers of at least 1). The
//! output good is never among the inputs. Then state what the numbers mean, copied from
//! design.md's "What the numbers mean": depreciation of 1/t₁ of the capital bundle per
//! turn whether or not the factory ran, shutdown when it is exhausted; construction paying
//! 1/t of both bundles per turn for t turns; maintenance as the construction project run
//! for a fraction of its time (replenishing a share f costs f of both bundles, at most 1/t
//! per turn); and that t > t₁ is allowed (such a factory wears out faster than it can be
//! maintained). Nothing runs them yet: Steps 03 to 06 of this Stage do. Knows nothing
//! about files.

use crate::bundle::Bundle;
use crate::goods::GoodId;
use crate::id::{Id, IdKind, NamedList};
use crate::quantity::Quantity;

/// Marker kind for technologies.
pub enum TechnologyKind {}
impl IdKind for TechnologyKind {
    const LABEL: &'static str = "technology";
}
/// The ID of a technology.
pub type TechnologyId = Id<TechnologyKind>;

/// One technology: a fixed-proportion recipe and what a factory of it is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Technology {
    inputs: Bundle,           // per unit of output; every amount > 0; never names `output`
    output: GoodId,
    capacity: Quantity,       // > 0
    capital: Bundle,          // never empty; every amount > 0
    construction: Bundle,     // every amount > 0; may be empty
    construction_turns: u32,  // >= 1
    lifetime: u32,            // >= 1
}

impl Technology {
    /// Crate-private: only the loader builds technologies.
    ///
    /// # Panics
    /// Panics if `inputs` names `output` (`"a technology's output good may not be among
    /// its inputs"`), if `capital` is empty (`"a technology's capital bundle must name at
    /// least one good"`), if any amount in `inputs`, `capital` or `construction`, or
    /// `capacity`, is 0 (`"a technology's amounts must be greater than 0"`), or if
    /// `construction_turns` or `lifetime` is 0 (`"a technology's construction turns and
    /// lifetime must be at least 1"`). The loader guarantees all of this.
    pub(crate) fn new(
        inputs: Bundle,
        output: GoodId,
        capacity: Quantity,
        capital: Bundle,
        construction: Bundle,
        construction_turns: u32,
        lifetime: u32,
    ) -> Self;
    /// The inputs one unit of output takes. May be empty.
    pub fn inputs(&self) -> &Bundle;
    /// The good a factory of this technology makes.
    pub fn output(&self) -> GoodId;
    /// The most output a factory of this technology makes in one turn. Greater than 0.
    pub fn capacity(&self) -> Quantity;
    /// The goods a finished factory holds. Never empty.
    pub fn capital(&self) -> &Bundle;
    /// The goods spent to build a factory. May be empty.
    pub fn construction(&self) -> &Bundle;
    /// How many turns building a factory takes (t). At least 1.
    pub fn construction_turns(&self) -> u32;
    /// How many turns a factory's capital lasts (t₁). At least 1.
    pub fn lifetime(&self) -> u32;
}

/// Every technology in a setting, in load order (which is also ID order).
pub type Technologies = NamedList<TechnologyKind, Technology>;
```

Seven parameters is within clippy's `too_many_arguments` limit; don't add an options struct.

### `EntryReader` addition (`crates/engine/src/setting/reader.rs`)

```rust
/// Reads `key` as a table whose values are quantities: `required_number_table` with
/// `positive_quantity` applied to every value, using the same prefix (`key '<k>' in
/// <key> of <label>`). A value that isn't a number, is out of range, or isn't a finite
/// number greater than 0 is reported and returned as None, so the caller still sees the
/// key. Missing and non-table `key` are handled as for `required_number_table` (None).
#[allow(clippy::type_complexity)]
pub fn required_quantity_table(&mut self, key: &str)
    -> Option<Vec<(Located<String>, Option<Quantity>)>>;
```

Implement it on top of `required_number_table` (call it, then map each `Some(number)` through `positive_quantity` with the prefix `format!("key '{}' in {key} of {}", name.value, self.label)`); build the prefix before the mutable call to keep the borrow checker happy (clone the label into a local `String` first).

### `TechnologiesReader` (`crates/engine/src/setting/technologies.rs`)

```rust
//! Reading `[[technology]]` sections. Domain types in `crate::technologies` and
//! `crate::bundle`. Technologies are a named kind, so they go through `Definitions` like
//! POP types; the goods named in `inputs`, `output`, `capital` and `construction` are
//! resolved in phase 2. Every entry is kept for phase 2, even one whose name was bad, so
//! its unknown goods are still reported; only entries with an ID reach `finish`.

/// One good = amount table, waiting for phase 2.
struct BundleEntry {
    /// Every key in table order; None where the amount was invalid (already reported).
    amounts: Vec<(Located<String>, Option<Quantity>)>,
    /// Filled in phase 2, parallel to `amounts`.
    ids: Vec<Option<GoodId>>,
}

impl BundleEntry {
    /// From `required_quantity_table`'s result; a missing or non-table key gives an empty entry
    /// (the load fails anyway).
    fn from_pairs(pairs: Option<Vec<(Located<String>, Option<Quantity>)>>) -> Self;
    /// Phase 2: resolves every key against the goods, always.
    fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink);
    /// Only after a successful load: every good and amount `expect`ed.
    fn finish(&self) -> Bundle;
}

/// One `[[technology]]` entry, waiting for phase 2.
struct TechnologyEntry {
    /// None if the name was missing, invalid or a duplicate.
    id: Option<TechnologyId>,
    inputs: BundleEntry,
    /// None if missing or not a string.
    output: Option<Located<String>>,
    /// None if missing or invalid.
    capacity: Option<Quantity>,
    capital: BundleEntry,
    construction: BundleEntry,
    construction_turns: Option<u32>,
    lifetime: Option<u32>,
    /// Filled in phase 2.
    output_id: Option<GoodId>,
}

#[derive(Default)]
pub(crate) struct TechnologiesReader {
    defs: Definitions<TechnologyKind>,
    entries: Vec<TechnologyEntry>,
}

impl TechnologiesReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink);
    /// Only after a successful load.
    pub fn finish(self) -> Technologies;
}
```

A private helper `fn turns(reader: &mut EntryReader<'_, '_>, key: &str) -> Option<u32>`: `reader.required_integer(key)?`; then `u32::try_from(v.value).ok().filter(|n| *n >= 1)`; on `None` report at `v.at`: `key '<key>' in technology must be at least 1` if `v.value < 1`, else `key '<key>' in technology is out of range` (the same two message shapes `perishes_after` uses; the label comes from `reader.label()`). A non-integer value is already reported by `required_integer` and gives None.

`read`, in this order, all before defining anything:

1. `name = reader.required_name("name")`.
2. `inputs = BundleEntry::from_pairs(reader.required_quantity_table("inputs"))`.
3. `output = reader.required_string("output")` (not name-checked: a bad value is simply an unknown good in phase 2).
4. `capacity = reader.required_number("capacity").and_then(|v| reader.positive_quantity("key 'capacity' in technology", v))` (prefix built with `reader.label()`).
5. `capital_pairs = reader.required_quantity_table("capital")`. If it is `Some` and empty, report `capital of technology needs at least one good` at the entry header (`reader.location()`), the way `utility of pop_type needs at least one good with a positive share` is. Then `capital = BundleEntry::from_pairs(capital_pairs)`.
6. `construction = BundleEntry::from_pairs(reader.required_quantity_table("construction"))`.
7. `construction_turns = turns(reader, "construction_turns")`, `lifetime = turns(reader, "lifetime")`.
8. Output among inputs: if `output` is `Some`, for every input key whose `value == output.value`, report `key '<good>' in inputs of technology must not be the output good` at the key's location. Done whether or not the amount was valid, and whether or not the goods exist.
9. `id = name.and_then(|n| self.defs.define(&n, reader.errors()))`. Push the `TechnologyEntry` whatever `id` is.

`resolve`: for every entry, `inputs.resolve`, `output_id = output.as_ref().and_then(|o| goods.resolve(o, sink))`, `capital.resolve`, `construction.resolve`. Always, so unknown goods are reported even in an entry with a bad name or bad values.

`finish`: for each entry with `Some(id)`, in order (ID order, because `define` numbers them in order), `Technology::new(inputs.finish(), output_id.expect(..), capacity.expect(..), capital.finish(), construction.finish(), construction_turns.expect(..), lifetime.expect(..))` with the message `"internal error: a technology was not fully resolved in a successful load"`. `Technologies::from_parts(self.defs.into_registry(), list)`.

### `setting/mod.rs`

- `Rules` gains `pub technologies: Technologies` (doc: every technology the setting defines, in load order; see `crate::technologies`).
- `WorldContent` gains `pub technologies: TechnologiesReader`; `section_types` gains `SectionType { header: "technology", folder: Folder::Rules, read: |c, r| c.technologies.read(r) }` after `pop`.
- `resolve`: add `self.technologies.resolve(self.goods.defs(), sink);` last, and extend the comment.
- `load_setting`: `Rules { goods, resources, pop_types, technologies: content.technologies.finish() }`.

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.rules.technologies.len(), "technology", "technologies")`. The sample line becomes:
`Loaded setting 'Sample Setting' (format version 1): 7 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops, 3 technologies`. Every other expected line in `checker.rs` ends in `, 0 technologies`; the new `technologies_one` test expects `Loaded setting 'Technologies One' (format version 1): 1 good, 0 hexes, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops, 1 technology`.

### Setting file format

A `[[technology]]` in `rules/` has eight required keys: `name` (a valid name, unique among technologies), `inputs` (a table of good name = number, each finite and greater than 0; may be empty; the output good may not appear), `output` (a good name), `capacity` (a finite number greater than 0), `capital` (a table like `inputs`, with at least one good), `construction` (a table like `inputs`; may be empty), `construction_turns` and `lifetime` (whole numbers from 1 to 4294967295). Both inline tables and sub-tables (`[technology.inputs]`, `[technology.capital]`, `[technology.construction]`) are accepted. Any other key is unknown. Good names are resolved in phase 2.

### Test setting contents

Write these files exactly; line numbers and columns matter for the expected errors. Every test setting has a `setting.toml` with `format_version = 1`, and a `start/README.md` when `start/` would otherwise be empty.

**`technologies_valid/`** — `setting.toml`: name `Technologies Test`. `rules/goods.toml`: goods `labor`, `ore`, `iron`, `tools`, `bread` in that order (name only; read after every technology file, so references point forward). `rules/a.toml`:
```toml
[[technology]]
name = "smelter"
inputs = { ore = 2, labor = 1 }
output = "iron"
capacity = 10
capital = { tools = 5 }
construction = { labor = 20 }
construction_turns = 2
lifetime = 10
```
`rules/a/z.toml` (sub-table forms, including empty ones; a capital bundle written out of ID order, with a decimal):
```toml
[[technology]]
name = "well"
output = "bread"
capacity = 0.5
construction_turns = 1
lifetime = 1

[technology.inputs]

[technology.capital]
tools = 1.5
labor = 3

[technology.construction]
```
`rules/b.toml` (an empty inline construction; the output good also in the capital bundle, which is allowed; construction turns equal to the lifetime):
```toml
[[technology]]
name = "toolmaker"
inputs = { iron = 1 }
output = "tools"
capacity = 3
capital = { tools = 2, iron = 4 }
construction = {}
construction_turns = 3
lifetime = 3
```
Expected (good IDs: labor 0, ore 1, iron 2, tools 3, bread 4): `smelter` = ID 0, inputs `[(labor, 1), (ore, 2)]`, output `iron`, capacity 10, capital `[(tools, 5)]`, construction `[(labor, 20)]`, 2, 10; `well` = ID 1, inputs `[]`, output `bread`, capacity 0.5, capital `[(labor, 3), (tools, 1.5)]`, construction `[]`, 1, 1; `toolmaker` = ID 2, inputs `[(iron, 1)]`, output `tools`, capacity 3, capital `[(iron, 4), (tools, 2)]`, construction `[]`, 3, 3.

**`technologies_one/`** — `setting.toml`: name `Technologies One`. `rules/goods.toml`: good `bread`. `rules/technologies.toml`: one `[[technology]]` `oven`: `inputs = {}`, `output = "bread"`, `capacity = 1`, `capital = { bread = 1 }`, `construction = {}`, `construction_turns = 1`, `lifetime = 1`. `start/README.md` only.

**`technologies_errors/`** — `setting.toml`: name `Technologies Errors`. `rules/goods.toml`: goods `coin`, `grain`, `labor`. `start/t.toml`:
```toml
[[technology]]
name = "misplaced"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1
```
`rules/b.toml`:
```toml
[[technology]]
name = "ok"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1
```
`rules/a.toml` (194 lines):
```toml
[[technology]]
name = "ok"
inputs = { grain = 1, labor = 0.5 }
output = "coin"
capacity = 2
capital = { labor = 1 }
construction = { labor = 2 }
construction_turns = 1
lifetime = 4

[[technology]]
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_inputs"
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_output"
inputs = {}
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_capacity"
inputs = {}
output = "grain"
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_capital"
inputs = {}
output = "grain"
capacity = 1
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_construction"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction_turns = 1
lifetime = 1

[[technology]]
name = "no_turns"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
lifetime = 1

[[technology]]
name = "no_lifetime"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1

[[technology]]
name = "Bad Name"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "ok"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "wrong_types"
inputs = 5
output = 3
capacity = "x"
capital = "tools"
construction = 7
construction_turns = 2.5
lifetime = "y"

[[technology]]
name = "bad_amounts"
inputs = { grain = "2", labor = 0 }
output = "coin"
capacity = 0
capital = { labor = -1, coin = inf }
construction = { grain = nan }
construction_turns = 1
lifetime = 1

[[technology]]
name = "bad_turns"
inputs = {}
output = "grain"
capacity = inf
capital = { labor = 1 }
construction = {}
construction_turns = 0
lifetime = -1

[[technology]]
name = "huge_turns"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 4294967296
lifetime = 99999999999999999999

[[technology]]
name = "empty_capital"
inputs = {}
output = "grain"
capacity = 1
capital = {}
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "self_input"
inputs = { grain = 1, labor = 2 }
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1

[[technology]]
name = "unknown_goods"
inputs = { tin = 1 }
output = "bronze"
capacity = 1
capital = { tin = 2 }
construction = { tin = 3 }
construction_turns = 1
lifetime = 1

[[technology]]
name = "unknown_keys"
inputs = {}
output = "grain"
capacity = 1
capital = { labor = 1 }
construction = {}
construction_turns = 1
lifetime = 1
colour = "red"
speed = 3

[[technology]]
name = "Several"
inputs = { tin = 1, grain = 0 }
output = "grain"
capacity = 0
capital = {}
construction = { tin = -1 }
construction_turns = 0
lifetime = 1.5
colour = "red"
```
Expected error lines (`help` is the usual invalid-name suffix, see `tests/resources.rs`; `positive` is `must be a finite number greater than 0`):
```
rules/a.toml:11:1: missing required key 'name' in technology
rules/a.toml:20:1: missing required key 'inputs' in technology
rules/a.toml:29:1: missing required key 'output' in technology
rules/a.toml:38:1: missing required key 'capacity' in technology
rules/a.toml:47:1: missing required key 'capital' in technology
rules/a.toml:56:1: missing required key 'construction' in technology
rules/a.toml:65:1: missing required key 'construction_turns' in technology
rules/a.toml:74:1: missing required key 'lifetime' in technology
rules/a.toml:84:8: invalid name 'Bad Name' {help}
rules/a.toml:94:8: duplicate technology name 'ok' (first defined at rules/a.toml:2:8)
rules/a.toml:105:10: key 'inputs' in technology must be a table, found integer
rules/a.toml:106:10: key 'output' in technology must be a string, found integer
rules/a.toml:107:12: key 'capacity' in technology must be a number, found string
rules/a.toml:108:11: key 'capital' in technology must be a table, found string
rules/a.toml:109:16: key 'construction' in technology must be a table, found integer
rules/a.toml:110:22: key 'construction_turns' in technology must be an integer, found float
rules/a.toml:111:12: key 'lifetime' in technology must be an integer, found string
rules/a.toml:115:20: key 'grain' in inputs of technology must be a number, found string
rules/a.toml:115:33: key 'labor' in inputs of technology {positive}
rules/a.toml:117:12: key 'capacity' in technology {positive}
rules/a.toml:118:21: key 'labor' in capital of technology {positive}
rules/a.toml:118:32: key 'coin' in capital of technology {positive}
rules/a.toml:119:26: key 'grain' in construction of technology {positive}
rules/a.toml:127:12: key 'capacity' in technology {positive}
rules/a.toml:130:22: key 'construction_turns' in technology must be at least 1
rules/a.toml:131:12: key 'lifetime' in technology must be at least 1
rules/a.toml:140:22: key 'construction_turns' in technology is out of range
rules/a.toml:141:12: key 'lifetime' in technology is out of range
rules/a.toml:143:1: capital of technology needs at least one good
rules/a.toml:155:12: key 'grain' in inputs of technology must not be the output good
rules/a.toml:165:12: unknown good 'tin'
rules/a.toml:166:10: unknown good 'bronze'
rules/a.toml:168:13: unknown good 'tin'
rules/a.toml:169:18: unknown good 'tin'
rules/a.toml:182:1: unknown key 'colour' in technology
rules/a.toml:183:1: unknown key 'speed' in technology
rules/a.toml:185:1: capital of technology needs at least one good
rules/a.toml:186:8: invalid name 'Several' {help}
rules/a.toml:187:12: unknown good 'tin'
rules/a.toml:187:21: key 'grain' in inputs of technology must not be the output good
rules/a.toml:187:29: key 'grain' in inputs of technology {positive}
rules/a.toml:189:12: key 'capacity' in technology {positive}
rules/a.toml:191:18: unknown good 'tin'
rules/a.toml:191:24: key 'tin' in construction of technology {positive}
rules/a.toml:192:22: key 'construction_turns' in technology must be at least 1
rules/a.toml:193:12: key 'lifetime' in technology must be an integer, found float
rules/a.toml:194:1: unknown key 'colour' in technology
rules/b.toml:2:8: duplicate technology name 'ok' (first defined at rules/a.toml:2:8)
start/t.toml:1:3: section 'technology' belongs in rules/, not start/
```
If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader` and `Definitions` message forms are fixed. The columns come from the TOML spans (column of the key's or value's first character); if `toml` reports a span differently than assumed, adjust the columns only. Note that `huge_turns`'s `lifetime` is reported by `required_integer` (beyond `i64`) and its `construction_turns` by `turns` (beyond `u32`), with the same message shape; that the `Several` entry (line 185 on) reports its unknown goods although its name is invalid; and that an input whose amount is invalid is still checked against the output good.

**`resources_broken_file/rules/r.toml`**: append
```toml

[[technology]]
name = "ghost"
inputs = { tin = 1 }
output = "nothing"
capacity = 1
capital = { tin = 1 }
construction = {}
construction_turns = 1
lifetime = 1
```
The expected errors stay exactly the one syntax error.

### Sample setting

**`settings/sample/rules/goods.toml`**: append, after `silver`, with a comment that it is the capital good factories are made of (see `technologies.toml`), durable and movable, so only a name is needed:
```toml
[[good]]
name = "tools"
```
Good IDs become: unskilled_labor 0, unimproved_copper 1, unimproved_grain 2, copper 3, grain 4, silver 5, tools 6.

**`settings/sample/start/markets.toml`**: the center's prices become
```toml
prices = { unskilled_labor = 1, grain = 2, copper = 5, unimproved_grain = 0.5, unimproved_copper = 1, tools = 12 }
```
and the comment above it says the center is also the only hex that trades tools. No other market changes.

**`settings/sample/rules/technologies.toml`**, with a header comment in the style of `pops.toml` that explains every key (the eight keys and their rules, as in "Setting file format" above), states what the numbers mean (the depreciation, construction and maintenance rules from `design.md`'s "What the numbers mean", including that steady maintenance keeps up with depreciation only when `construction_turns <= lifetime`, and that nothing runs factories yet), and explains the numbers chosen: at the center's prices (labor 1, grain 2, copper 5, unimproved grain 0.5, unimproved copper 1, tools 12), per turn at capacity the farm nets 5 on revenue 24, the mine 4 on 12.5 and the toolmaker 0.25 on 48 (revenue minus inputs minus one lifetime's share of capital plus construction), and the capacities match what the center's deposits emit each turn (12 unimproved grain, 2.5 unimproved copper). Then:
```toml
# A farm turns the grain deposits' output into grain.
[[technology]]
name = "farm"
inputs = { unimproved_grain = 1, unskilled_labor = 0.5 }
output = "grain"
capacity = 12
capital = { tools = 4 }
construction = { unskilled_labor = 8 }
construction_turns = 2
lifetime = 8

# A mine turns the copper deposit's output into copper.
[[technology]]
name = "mine"
inputs = { unimproved_copper = 1, unskilled_labor = 1 }
output = "copper"
capacity = 2.5
capital = { tools = 2 }
construction = { unskilled_labor = 4 }
construction_turns = 2
lifetime = 8

# A toolmaker turns copper into tools, the capital good every factory here is made of.
# Its margin is thin: it covers inputs and maintenance with little to spare.
[[technology]]
name = "toolmaker"
inputs = { copper = 2, unskilled_labor = 1.5 }
output = "tools"
capacity = 4
capital = { tools = 3 }
construction = { unskilled_labor = 6 }
construction_turns = 3
lifetime = 24
```
(Comment wording is free; the values are exact.) Expected: `farm` = ID 0, inputs `[(unskilled_labor, 0.5), (unimproved_grain, 1)]`, output `grain`, capacity 12, capital `[(tools, 4)]`, construction `[(unskilled_labor, 8)]`, 2, 8; `mine` = ID 1, inputs `[(unskilled_labor, 1), (unimproved_copper, 1)]`, output `copper`, capacity 2.5, capital `[(tools, 2)]`, construction `[(unskilled_labor, 4)]`, 2, 8; `toolmaker` = ID 2, inputs `[(unskilled_labor, 1.5), (copper, 2)]`, output `tools`, capacity 4, capital `[(tools, 3)]`, construction `[(unskilled_labor, 6)]`, 3, 24. `settings/sample/rules/README.md`: add that technologies are in `technologies.toml`.

The sample's turn lines don't change: no POP wants `tools`, nothing is produced yet, and the new good is never emitted.

## Tasks

1. `bundle.rs`: `Bundle`, unit tests. Export from `lib.rs`.
2. `technologies.rs`: domain types, `Technology::new`, accessors, unit tests. Export from `lib.rs`; crate docs.
3. `reader.rs`: `required_quantity_table`; unit test.
4. `setting/technologies.rs`: `TechnologiesReader`. `setting/mod.rs`: wiring and docs.
5. `check.rs` success line; `checker.rs` expectations and the `technologies_one` test; `text.rs` `market_lines_on_the_sample`.
6. Test settings (`technologies_valid`, `technologies_one`, `technologies_errors`, `resources_broken_file`), `tests/settings/README.md`; the sample's `goods.toml`, `markets.toml`, `technologies.toml` and `rules/README.md`.
7. `crates/engine/tests/technologies.rs` (Test Plan); the sample tests in `tests/goods.rs` and `tests/market.rs`; the comment in `tests/resources.rs`.
8. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the new success line and unchanged turn lines.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). All amounts in these tests are exactly representable, so compare them with `assert_eq!` on `Quantity` or `f64`.

**Unit tests.** `bundle.rs`: pairs given out of order come back sorted by good; `amount_of` gives the amount or zero; `contains`; `len`/`is_empty`; `Bundle::default()` is empty; `#[should_panic]` for a duplicate good. `technologies.rs`: a technology stores every field and the accessors return them; `#[should_panic]` (with the exact message) for the output among the inputs, an empty capital bundle, a zero amount in each of inputs, capital and construction, a zero capacity, zero construction turns and zero lifetime. `reader.rs` (pattern of the existing `run` helper): `required_quantity_table` on `p = { a = 1, b = 2.5, c = "x", d = 0, e = inf }`, `q = 7` and a missing key returns `a` and `b` as quantities and the rest as None, and reports `key 'c' in p of thing must be a number, found string`, `key 'd' in p of thing must be a finite number greater than 0`, `key 'e' in p of thing must be a finite number greater than 0`, `key 'q' in thing must be a table, found integer` and the missing-key error.

**`crates/engine/tests/technologies.rs`** (`mod common;`). A `techs_of(&LoadedSetting)` helper like `tests/pops.rs`'s `types_of` returns each technology as (index, name, inputs as `Vec<(good name, f64)>`, output name, capacity, capital, construction, construction turns, lifetime); compare whole lists with `assert_eq!`.
- **AC "the sample setting loads with tools, its price and the three technologies"**: load the sample; `goods.id("tools")` is ID 6; `technologies.len() == 3`; `techs_of` equals the list above; `technologies.id("farm")` is ID 0; the center market prices `tools` at 12 (the `tests/market.rs` change covers the price in detail).
- **AC "test setting across several files, both forms, forward references, empty tables"**: load `technologies_valid`; `techs_of` equals the list above; `well`'s `inputs().is_empty()` and `construction().is_empty()`; `smelter`'s `inputs().amount_of(ore)` is 2 and `amount_of(bread)` is zero; `capital().contains(tools)`; loading twice gives equal `techs_of`.
- **AC "each error case, one entry with several problems"**: `failures_of("technologies_errors")` equals the expected list (follow `every_pop_type_error_is_reported_at_its_place`; the `Several` entry covers "several problems", so no separate test).
- **AC "no technologies is valid; references not checked after a broken file"**: `valid_minimal` loads with `technologies.is_empty()`, `len() == 0`, and `technologies.id("missing")` is `Err(IdError::UnknownName { kind: "technology", name })`; the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing (add the comment).
- **AC "every existing test still passes"**: `tests/goods.rs` and `tests/market.rs` updated for the sample's new good and price; `tests/world.rs`, `tests/trade.rs`, `tests/growth.rs` and the dev UI tests pass unchanged apart from the success lines and the center's market lines.

**Dev UI**: `checker.rs` passes with the updated lines, including `1 technology` for `technologies_one` and `0 technologies` elsewhere; `text.rs`'s `market_lines_on_the_sample` passes with the `tools` line.

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`.
- `Bundle::new`, `Technology::new` and `NamedList::from_parts` stay `pub(crate)`. `Bundle`'s and `Technology`'s fields stay private.
- Do not change `World`, the turn, `TurnReport`, the dev UI window, `HexMap`, `HexCurrencies`, `HexPrices`, `Market`, `PopTypesReader`, `MarketsReader`, or the checker's output beyond the technology count. Do not change existing `EntryReader` message texts.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: factories, production, depreciation, maintenance, construction, business decisions, any economic sanity check (profitability, `construction_turns <= lifetime`, whether inputs can be had, whether capital goods are durable), bundle arithmetic (scaling, adding, subtracting), showing technologies in the dev UI window.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line and unchanged turn lines, and every Acceptance Criterion in `design.md` is met.
