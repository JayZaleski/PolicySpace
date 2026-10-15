# Step 03: pop-types — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-02-populate-the-world/step-03-pop-types/design.md` — the behavior this spec implements.
- `PROJECT_PLAN/stage-02-populate-the-world/step-02-dummy-market/spec.md` — how Step 02 added a number-table reader and a section type; this Step follows the same shape.
- `docs/setting-loader.md` — the loader's two phases, `EntryReader`, `Definitions`, error order, "Adding a section type".
- `docs/resources.md` — `ResourcesReader` is the pattern for a named `rules/` entry whose good references resolve in phase 2.
- `docs/ids.md` — `NamedList`, `IdKind`, `Id`.
- `docs/dev-ui.md` — the checker's success line.
- `docs/workspace.md` — the check command and test layout.
- Code: `crates/engine/src/setting/resources.rs` (`ResourcesReader`), `crates/engine/src/setting/market.rs` (`MarketsReader`, use of `required_number_table`), `crates/engine/src/setting/reader.rs` (`EntryReader`), `crates/engine/src/setting/defs.rs`, `crates/engine/src/setting/mod.rs` (`WorldContent`, `Rules`, `load_setting`), `crates/engine/src/resources.rs` (domain pattern), `crates/engine/src/id.rs` (`NamedList`), `crates/engine/src/quantity.rs`, `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/resources.rs` (`types_of`, `every_resource_error_is_reported_at_its_place`), `crates/dev-ui/src/check.rs`, `crates/dev-ui/tests/checker.rs`, `settings/sample/rules/goods.toml`, `settings/sample/start/markets.toml` (the sample's prices, for choosing sensible parameters).

## Approach

- **A `[[pop_type]]` section in `rules/`** (not `[[pop]]`: that header is kept for Step 04's placed POPs), read by a new `PopTypesReader` (`setting/pops.rs`) that follows `ResourcesReader`: the name goes through `Definitions<PopKind>`, every key is read before anything is defined, and the good names in `endowment` and `utility` are resolved in phase 2. Rationale: POP types are persistent rules identified by name, exactly like resource types.
- **Domain types in a new public module `crates/engine/src/pops.rs`**: `PopKind`/`PopId`, `Endowment`, `UtilityGood`, `PopType`, and `PopTypes = NamedList<PopKind, PopType>`. The module knows nothing about files. Step 04's placed POPs can join this module later. Rationale: mirrors `resources.rs`.
- **`PopType` keeps its lists sorted by good ID, with shares already normalized.** A crate-private constructor `PopType::new` sorts and normalizes, so the loader stays thin and normalization is unit-tested in the domain module. Shares are `f64` (dimensionless weights); subsistence amounts and endowment amounts are `Quantity`.
- **`EntryReader` gains three small accessors** for the `utility` table of tables: `optional_number`, `has`, `required_table_of_tables`, and a `nested` sub-reader whose label is `utility.<good> of pop_type`, so a utility good's sub-table gets the standard wrong-type, missing-key and unknown-key handling and messages. The reader's `label` becomes a `String` to allow that. Rationale: reuses the existing machinery instead of hand-parsing a `DeTable` in the pops reader.
- **Every entry is resolved in phase 2, even one whose name was bad or duplicate**, so an unknown good in such an entry is still reported. Only entries that got an ID reach `finish`. (Resource types drop such entries; POP types are stricter because the design asks for every problem of an entry in one load.)
- **`Rules` gains `pop_types: PopTypes`**, and the checker's success line gains `, N pop types` at the end. `World` holds `Rules` whole, so it is unchanged.
- **The build runs as one.** The reader needs the `EntryReader` additions and the domain type, and every test needs loaded settings, so no package could be built and tested alone.

## Changes

### New

- `crates/engine/src/pops.rs` — domain types and unit tests.
- `crates/engine/src/setting/pops.rs` — `PopTypesReader`.
- `crates/engine/tests/pops.rs` — integration tests (Test Plan).
- Test settings `crates/engine/tests/settings/pops_valid/`, `pops_errors/`, `pops_one/` (exact contents below).
- `settings/sample/rules/pops.toml`.

### Modified

- `crates/engine/src/lib.rs` — `pub mod pops;` and `pub use pops::{Endowment, PopId, PopKind, PopType, PopTypes, UtilityGood};` after `map`/`market` (alphabetical: `pops` goes after `market`, before `quantity`). Crate docs: a sentence that POP types (the per-individual endowment and Stone-Geary utility parameters) are rules in the `pops` module; nothing uses them yet.
- `crates/engine/src/setting/reader.rs` — `label: String`; `new` takes `label: &str`; `label()` returns `&str`; new `optional_number`, `has`, `required_table_of_tables`, `nested`; unit tests.
- `crates/engine/src/setting/mod.rs` — `mod pops;`, `Rules.pop_types`, `WorldContent.pop_types`, the `pop_type` section type, `resolve`, `load_setting`. Module docs: POP types are the third `rules/` kind (`[[pop_type]]`), their endowment and utility goods are resolved in phase 2; the "Adding a section type" list mentions `crate::pops` / `setting/pops.rs` as another named-kind example.
- `crates/dev-ui/src/check.rs` — the success line.
- `crates/dev-ui/tests/checker.rs` — every expected success line gains `, 0 pop types`, except the sample (`2 pop types`); a new test for `pops_one` (`1 pop type`).
- `crates/engine/tests/resources.rs` — comment in `references_are_not_checked_after_a_broken_file_or_incomplete_discovery` (the setting now also holds a POP type with unknown goods).
- `crates/engine/tests/settings/README.md` — the new and changed settings.
- Test setting `resources_broken_file/rules/r.toml` (append a `[[pop_type]]`, below).
- `settings/sample/rules/README.md` — mention `pops.toml`.

## Interfaces & Data

### `crates/engine/src/pops.rs`

```rust
//! Module docs: POP types. A POP type describes one individual of a POP (Step 04): the
//! goods it receives at the start of every turn (its endowment) and the Stone-Geary
//! utility it consumes by, as a subsistence amount and a share for each good it wants.
//! Leisure needs no special case: it is an endowed good that also appears in the utility,
//! consumed rather than sold. The intended design endows only unskilled labor; any good
//! may be endowed so the POP system can be tested before production and the live market
//! exist. Shares are stored normalized (they sum to 1 over the type's utility goods); the
//! weights as written in the setting aren't kept. A good with share 0 but a positive
//! subsistence is a pure necessity. Nothing uses POP types yet: Step 04 places POPs of
//! these types, Step 05 computes their demand. Knows nothing about files.

use crate::goods::GoodId;
use crate::id::{Id, IdKind, NamedList};
use crate::quantity::Quantity;

/// Marker kind for POP types.
pub enum PopKind {}
impl IdKind for PopKind {
    const LABEL: &'static str = "pop type";
}
/// The ID of a POP type.
pub type PopId = Id<PopKind>;

/// One endowed good: what one individual receives of it at the start of every turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Endowment {
    pub good: GoodId,
    /// Always finite and greater than 0.
    pub amount: Quantity,
}

/// One good in a POP type's utility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtilityGood {
    pub good: GoodId,
    /// The amount one individual needs before anything else. May be zero.
    pub subsistence: Quantity,
    /// Its share of spending beyond subsistence, normalized over the type's utility goods
    /// (the shares sum to 1). Zero for a pure necessity.
    pub share: f64,
}

/// One POP type: the per-individual endowment and utility parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PopType {
    endowment: Vec<Endowment>,      // sorted by good, no good twice
    utility: Vec<UtilityGood>,      // sorted by good, no good twice
}

impl PopType {
    /// Builds a type from (good, amount) endowment pairs and (good, subsistence, weight)
    /// utility triples in any order. Sorts both by good ID and normalizes the weights to
    /// shares that sum to 1. Crate-private: only the loader builds POP types.
    ///
    /// # Panics
    /// Panics if a good appears twice in either list, if a weight is negative or not
    /// finite, or if no weight is positive (the loader guarantees all of this).
    pub(crate) fn new(endowment: Vec<(GoodId, Quantity)>, utility: Vec<(GoodId, Quantity, f64)>) -> Self;
    /// The endowment, in good ID order. May be empty.
    pub fn endowment(&self) -> &[Endowment];
    /// The utility goods, in good ID order. Never empty; at least one share is positive.
    pub fn utility(&self) -> &[UtilityGood];
    /// How much of `good` one individual receives per turn; zero if it isn't endowed.
    pub fn endowment_of(&self, good: GoodId) -> Quantity;
    /// The utility parameters of `good`, or None if the type doesn't want it.
    pub fn utility_of(&self, good: GoodId) -> Option<&UtilityGood>;
}

/// Every POP type in a setting, in load order (which is also ID order).
pub type PopTypes = NamedList<PopKind, PopType>;
```

`new`: sort each list with `sort_by_key(|x| x.0)` (`Id` is `Ord`), assert no two adjacent goods are equal (`"a good appears twice in a POP type's endowment"` / `"... utility"`), assert every weight is finite and `>= 0.0`, compute `total = sum(weights)`, assert `total > 0.0` (`"a POP type needs a utility good with a positive share"`), and store `share = weight / total`. `endowment_of` and `utility_of` may use `binary_search_by_key`.

### `EntryReader` additions (`crates/engine/src/setting/reader.rs`)

```rust
/// A number that may be absent. Absent and wrong-type both give None (the latter is
/// reported); use `has` to tell them apart.
pub fn optional_number(&mut self, key: &str) -> Option<Located<f64>>;

/// True if the table has `key`. Doesn't mark the key as read.
pub fn has(&self, key: &str) -> bool;

/// Reads `key` as a table whose values are tables. Returns every (name, table) pair in
/// the table's order, each name with its location. A value that isn't a table is reported
/// (`key '<k>' in <key> of <label> must be a table, found <type>`) and returned as None,
/// so the caller still sees the name. A missing key reports the usual missing-key error;
/// a value that isn't a table reports `key '<key>' in <label> must be a table, found
/// <type>`. Both return None. The nested table's keys are data, not fields: not
/// name-checked, not tracked as read or unknown.
#[allow(clippy::type_complexity)]
pub fn required_table_of_tables(&mut self, key: &str)
    -> Option<Vec<(Located<String>, Option<&'a DeTable<'i>>)>>;

/// A reader for one sub-table returned by `required_table_of_tables`: `key` is the outer
/// key and `name` the sub-table's key. Its label is `<key>.<name> of <label>` (for example
/// `utility.grain of pop_type`), so its messages read `key 'share' in utility.grain of
/// pop_type must be a number, found string` and `unknown key 'weight' in utility.grain of
/// pop_type`. Missing required keys are reported at `name`'s location. Call `finish` on
/// it to report its unknown keys.
pub fn nested(&mut self, key: &str, name: &Located<String>, table: &'a DeTable<'i>)
    -> EntryReader<'_, 'i>;
```

Implementation notes: `number(key, required)` gets a `required` flag like `string`/`integer`; `required_number` passes true. `nested` builds `EntryReader { file: self.file, table, entry: name.at.clone(), missing_at: MissingAt::Entry, label: format!("{key}.{} of {}", name.value, self.label), read: Vec::new(), sink: &mut *self.sink }`. Change `label: &'static str` to `label: String`, `new(.., label: &str, ..)` (`to_string()` it), and `label(&self) -> &str`; `mod.rs` passes `section.header` unchanged, `map::coordinate` compiles unchanged. `required_table_of_tables` mirrors `required_number_table`, with `as_table()` on each value and `wrong_type`-style messages built by hand for the nested values (the `key '<k>' in <key> of <label>` prefix).

### `PopTypesReader` (`crates/engine/src/setting/pops.rs`)

```rust
//! Reading `[[pop_type]]` sections. Domain types in `crate::pops`. POP types are a named
//! kind, so they go through `Definitions` like resource types; the goods named in
//! `endowment` and `utility` are resolved in phase 2. Every entry is kept for phase 2,
//! even one whose name was bad, so its unknown goods are still reported; only entries
//! with an ID reach `finish`.

struct PopEntry {
    /// None if the name was missing, invalid or a duplicate.
    id: Option<PopId>,
    /// Every endowment key, in table order; None where the amount was invalid.
    endowment: Vec<(Located<String>, Option<Quantity>)>,
    /// Every utility key, in table order; None where the sub-table or a value was invalid.
    /// The f64 is the weight as written.
    utility: Vec<(Located<String>, Option<(Quantity, f64)>)>,
    /// Filled in phase 2, parallel to `endowment` and `utility`.
    endowment_ids: Vec<Option<GoodId>>,
    utility_ids: Vec<Option<GoodId>>,
}

#[derive(Default)]
pub(crate) struct PopTypesReader {
    defs: Definitions<PopKind>,
    entries: Vec<PopEntry>,
}

impl PopTypesReader {
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>);
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink);
    /// Only after a successful load.
    pub fn finish(self) -> PopTypes;
}
```

`read`, in this order, all before defining anything:

1. `name = reader.required_name("name")`.
2. `endowment = reader.required_number_table("endowment")`. For each `(key, number)`: `None` → push `(key, None)` (already reported). `Some(v)` → `Quantity::new(v.value).filter(|q| q.get() > 0.0)`; on `None` report `key '<k>' in endowment of pop_type must be a finite number greater than 0` at `v.at` and push `(key, None)`. A missing or non-table `endowment` gives an empty list (the load fails anyway).
3. `utility = reader.required_table_of_tables("utility")`. For each `(key, table)`: `None` → push `(key, None)`. `Some(table)` → `let mut sub = reader.nested("utility", &key, table);` read `subsistence` and `share` with a private helper `fn amount(sub: &mut EntryReader, key: &str) -> Amount` where `enum Amount { Absent, Valid(f64), Invalid }`: `Absent` if `!sub.has(key)`; else `sub.optional_number(key)` → `None` is `Invalid` (reported by the reader); `Some(v)` with `v.value.is_finite() && v.value >= 0.0` is `Valid`, otherwise report `key '<key>' in utility.<k> of pop_type must be a finite number of at least 0` at `v.at` and `Invalid`. Then `sub.finish()` (unknown keys). Then: if either is `Invalid`, push `(key, None)` and treat this good as "share unknown". Otherwise `subsistence = Absent → 0`, `share = Absent → 0`; if both are 0 report `key '<k>' in utility of pop_type must have a positive subsistence or share` at `key.at` and push `(key, None)`; else push `(key, Some((Quantity::new(subsistence).unwrap_or(ZERO), share)))`.
4. If `utility` was `Some` (a table) and no good has a positive share or an unknown share, report `utility of pop_type needs at least one good with a positive share` at the entry header (`reader.location()`). An empty table therefore reports this. A missing or non-table `utility` doesn't (its own error is reported).
5. `id = name.and_then(|n| self.defs.define(&n, reader.errors()))`. Push the `PopEntry` whatever `id` is.

The pops reader builds its two messages with `reader.label()` so they read `... of pop_type`; the nested reader's messages come from `EntryReader`.

`resolve`: for every entry, `endowment_ids = endowment.iter().map(|(name, _)| goods.resolve(name, sink)).collect()`, same for `utility_ids`. Always, so unknown goods are reported even in an entry with a bad name or bad values.

`finish`: for each entry with `Some(id)`, in order (they are in ID order because `define` numbers them in order), build `PopType::new(pairs, triples)` where every good ID and value is `expect`ed (`"internal error: a POP type's goods were not resolved in a successful load"`). `PopTypes::from_parts(self.defs.into_registry(), types)`.

### `setting/mod.rs`

- `Rules` gains `pub pop_types: PopTypes` (doc: every POP type the setting defines, in load order; see `crate::pops`).
- `WorldContent` gains `pub pop_types: PopTypesReader`; `section_types` gains `SectionType { header: "pop_type", folder: Folder::Rules, read: |c, r| c.pop_types.read(r) }` after `market`.
- `resolve`: add `self.pop_types.resolve(self.goods.defs(), sink);` last.
- `load_setting`: `Rules { goods, resources, pop_types: content.pop_types.finish() }`.

### Checker (`crates/dev-ui/src/check.rs`)

Append `, {}` with `count(setting.rules.pop_types.len(), "pop type", "pop types")`. The sample line becomes:
`Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types`. Every other expected line in `checker.rs` ends in `, 0 pop types`; the new `pops_one` test expects `Loaded setting 'Pops One' (format version 1): 1 good, 0 hexes, 0 resources, 0 deposits, 0 markets, 1 pop type`.

### Setting file format

A `[[pop_type]]` in `rules/` has three required keys: `name` (a valid name, unique among POP types), `endowment` (a table of good name = number, each finite and greater than 0; may be empty) and `utility` (a table of good name = sub-table; each sub-table has optional `subsistence` and `share`, each a finite number of at least 0, default 0; a good with both zero is an error; at least one good needs a positive share). Both inline tables and sub-tables (`[pop_type.endowment]`, `[pop_type.utility.grain]`) are accepted. Any other key in the entry or in a sub-table is unknown. Good names are resolved in phase 2.

### Test setting contents

Write these files exactly; line numbers and columns matter for the expected errors. Every test setting has a `setting.toml` with `format_version = 1`.

**`pops_valid/`** — `setting.toml`: name `Pops Test`. `rules/goods.toml`: goods `coin`, `labor`, `grain`, `fish` in that order (name only; read after every POP file, so references point forward). `start/hexes.toml`: one hex (0, 0) with `currency = "coin"` (so `coin` is a currency good, and nothing stops it appearing in a utility). `rules/a.toml`:
```toml
[[pop_type]]
name = "farmer"
endowment = { labor = 10 }
utility = { grain = { subsistence = 2, share = 3 }, labor = { subsistence = 1, share = 1 } }
```
`rules/a/z.toml` (sub-table forms, multi-good endowment written out of ID order, a pure necessity, the currency in the utility):
```toml
[[pop_type]]
name = "fisher"

[pop_type.endowment]
fish = 2
labor = 8

[pop_type.utility.fish]
subsistence = 1

[pop_type.utility.coin]
share = 0.5

[pop_type.utility.grain]
share = 1.5
```
`rules/b.toml` (empty endowment):
```toml
[[pop_type]]
name = "idle"
endowment = {}
utility = { grain = { share = 1 } }
```
Expected (good IDs: coin 0, labor 1, grain 2, fish 3): `farmer` = ID 0, endowment `[(labor, 10)]`, utility `[(labor, 1, 0.25), (grain, 2, 0.75)]`; `fisher` = ID 1, endowment `[(labor, 8), (fish, 2)]`, utility `[(coin, 0, 0.25), (grain, 0, 0.75), (fish, 1, 0)]`; `idle` = ID 2, endowment `[]`, utility `[(grain, 0, 1)]`.

**`pops_one/`** — `setting.toml`: name `Pops One`. `rules/goods.toml`: good `grain`. `rules/pops.toml`: one `[[pop_type]]` `eater`, `endowment = { grain = 1 }`, `utility = { grain = { share = 1 } }`. `start/README.md` only.

**`pops_errors/`** — `setting.toml`: name `Pops Errors`. `rules/goods.toml`: goods `coin`, `grain`. `start/p.toml`:
```toml
[[pop_type]]
name = "misplaced"
endowment = {}
utility = { grain = { share = 1 } }
```
`rules/b.toml`:
```toml
[[pop_type]]
name = "ok"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }
```
`rules/a.toml` (98 lines):
```toml
[[pop_type]]
name = "ok"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[[pop_type]]
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "no_endowment"
utility = { grain = { share = 1 } }

[[pop_type]]
name = "no_utility"
endowment = { grain = 1 }

[[pop_type]]
name = "Bad Name"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "ok"
endowment = { grain = 1 }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "endowment_not_table"
endowment = 5
utility = { grain = { share = 1 } }

[[pop_type]]
name = "utility_not_table"
endowment = { grain = 1 }
utility = "grain"

[[pop_type]]
name = "bad_amounts"
endowment = { grain = "2", coin = 0 }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "more_bad_amounts"
endowment = { grain = -1, coin = inf }
utility = { grain = { share = 1 } }

[[pop_type]]
name = "utility_entry_not_table"
endowment = { grain = 1 }
utility = { grain = 2, coin = { share = 1 } }

[[pop_type]]
name = "bad_subsistence"
endowment = { grain = 1 }
utility = { grain = { subsistence = -1, share = 1 }, coin = { subsistence = "x", share = 1 } }

[[pop_type]]
name = "bad_share"
endowment = { grain = 1 }
utility = { grain = { share = nan }, coin = { share = 1 } }

[[pop_type]]
name = "nothing_positive"
endowment = { grain = 1 }
utility = { grain = { share = 1 }, coin = {} }

[[pop_type]]
name = "zero_values"
endowment = { grain = 1 }
utility = { grain = { share = 1 }, coin = { subsistence = 0, share = 0 } }

[[pop_type]]
name = "no_positive_share"
endowment = { grain = 1 }
utility = { grain = { subsistence = 2 } }

[[pop_type]]
name = "empty_utility"
endowment = { grain = 1 }
utility = {}

[[pop_type]]
name = "unknown_goods"
endowment = { tin = 1 }
utility = { tin = { share = 1 } }

[[pop_type]]
name = "unknown_keys"
endowment = { grain = 1 }
utility = { grain = { share = 1, weight = 2 } }
colour = "red"

[[pop_type]]
name = "Several"
endowment = { tin = 1 }
utility = { tin = { share = -1, weight = 2 }, coin = {} }
colour = "red"
```
Expected error lines (`help` is the usual invalid-name suffix, see `tests/resources.rs`):
```
rules/a.toml:6:1: missing required key 'name' in pop_type
rules/a.toml:10:1: missing required key 'endowment' in pop_type
rules/a.toml:14:1: missing required key 'utility' in pop_type
rules/a.toml:19:8: invalid name 'Bad Name' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)
rules/a.toml:24:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)
rules/a.toml:30:13: key 'endowment' in pop_type must be a table, found integer
rules/a.toml:36:11: key 'utility' in pop_type must be a table, found string
rules/a.toml:40:23: key 'grain' in endowment of pop_type must be a number, found string
rules/a.toml:40:35: key 'coin' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:45:23: key 'grain' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:45:34: key 'coin' in endowment of pop_type must be a finite number greater than 0
rules/a.toml:51:21: key 'grain' in utility of pop_type must be a table, found integer
rules/a.toml:56:37: key 'subsistence' in utility.grain of pop_type must be a finite number of at least 0
rules/a.toml:56:77: key 'subsistence' in utility.coin of pop_type must be a number, found string
rules/a.toml:61:31: key 'share' in utility.grain of pop_type must be a finite number of at least 0
rules/a.toml:66:36: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:71:36: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:73:1: utility of pop_type needs at least one good with a positive share
rules/a.toml:78:1: utility of pop_type needs at least one good with a positive share
rules/a.toml:85:15: unknown good 'tin'
rules/a.toml:86:13: unknown good 'tin'
rules/a.toml:91:34: unknown key 'weight' in utility.grain of pop_type
rules/a.toml:92:1: unknown key 'colour' in pop_type
rules/a.toml:95:8: invalid name 'Several' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)
rules/a.toml:96:15: unknown good 'tin'
rules/a.toml:97:13: unknown good 'tin'
rules/a.toml:97:29: key 'share' in utility.tin of pop_type must be a finite number of at least 0
rules/a.toml:97:33: unknown key 'weight' in utility.tin of pop_type
rules/a.toml:97:47: key 'coin' in utility of pop_type must have a positive subsistence or share
rules/a.toml:98:1: unknown key 'colour' in pop_type
rules/b.toml:2:8: duplicate pop type name 'ok' (first defined at rules/a.toml:2:8)
start/p.toml:1:3: section 'pop_type' belongs in rules/, not start/
```
If the actual output differs, check the implementation against this spec before changing the list; the `EntryReader` and `Definitions` message forms are fixed. The columns of inline-table keys and values come from the TOML spans (column of the key's or value's first character); if `toml` reports a span differently than assumed, adjust the columns only. Note that the `Several` entry (line 94 on) reports its unknown goods although its name is invalid, and that `tin`'s invalid share means no "needs at least one good with a positive share" error for that entry.

**`resources_broken_file/rules/r.toml`**: append
```toml

[[pop_type]]
name = "ghost"
endowment = { tin = 1 }
utility = { nothing = { share = 1 } }
```
The expected errors stay exactly the one syntax error.

**Sample setting** — `settings/sample/rules/pops.toml`, with a header comment in the style of `resources.toml` explaining the three keys, that `subsistence` and `share` default to 0, that shares are weights the engine normalizes to sum to 1 (so `1, 4, 5` means 10%, 40%, 50%), that a good with share 0 and positive subsistence is bought only at subsistence, that an endowed good that is also in the utility is kept as leisure (unskilled labor here), that unskilled labor is the intended endowment and any good is allowed only so the POP system can be tested before production and the live market exist, and that the amounts are per individual per turn. Then:
```toml
# Peasants keep a fifth of their spare income as leisure and spend the rest on grain.
# Shares here already sum to 1; the engine would normalize them anyway.
[[pop_type]]
name = "peasant"
endowment = { unskilled_labor = 10 }

[pop_type.utility.unskilled_labor]
subsistence = 2
share = 0.2

[pop_type.utility.grain]
subsistence = 2
share = 0.8

# Artisans keep less leisure and weight copper heavily. The shares are weights: 1, 4 and 5
# become 0.1, 0.4 and 0.5.
[[pop_type]]
name = "artisan"
endowment = { unskilled_labor = 10 }

[pop_type.utility.unskilled_labor]
subsistence = 1
share = 1

[pop_type.utility.grain]
subsistence = 2
share = 4

[pop_type.utility.copper]
share = 5
```
(Comment wording is free; the values are exact.) Expected (good IDs: unskilled_labor 0, copper 3, grain 4): `peasant` = ID 0, endowment `[(unskilled_labor, 10)]`, utility `[(unskilled_labor, 2, 0.2), (grain, 2, 0.8)]`; `artisan` = ID 1, endowment `[(unskilled_labor, 10)]`, utility `[(unskilled_labor, 1, 0.1), (copper, 0, 0.5), (grain, 2, 0.4)]`. `settings/sample/rules/README.md`: add that POP types are in `pops.toml`.

## Tasks

1. `pops.rs`: domain types, `PopType::new`, accessors, unit tests (sorting, normalization, `endowment_of`, `utility_of`, the three panics). Export from `lib.rs`.
2. `reader.rs`: `label: String`, `optional_number`, `has`, `required_table_of_tables`, `nested`; unit tests (see Test Plan).
3. `setting/pops.rs`: `PopTypesReader`. `setting/mod.rs`: wiring and docs.
4. `check.rs` success line; `checker.rs` expectations and the `pops_one` test.
5. Test settings (`pops_valid`, `pops_one`, `pops_errors`, `resources_broken_file`), `tests/settings/README.md`, the sample's `pops.toml` and `rules/README.md`.
6. `crates/engine/tests/pops.rs` (Test Plan); the comment in `tests/resources.rs`.
7. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the new success line and unchanged turn lines.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). Compare shares with a tolerance (`(a - b).abs() < 1e-9`), never exact equality.

**Unit tests.** `pops.rs`: weights 3 and 1 given out of ID order come back sorted by good as shares 0.75 and 0.25 (**AC "shares sum to 1 in ID order"**); a single positive weight gives share 1; a zero-weight good stays with share 0; an empty endowment is fine; `endowment_of` gives the amount or zero; `utility_of` finds a good or gives None; `#[should_panic]` for a duplicate endowment good, a duplicate utility good, and no positive weight. `reader.rs` (pattern of the existing `run` helper): `optional_number` present, absent (no error) and wrong type (error, None); `has` true/false and not marking the key as read (`finish` still reports it as unknown); `required_table_of_tables` with a table of tables (names, locations, one non-table value reported and None), a non-table value, a missing key, and the sub-table form (`[p.x]`); `nested` reads a sub-table and reports `key 'a' in p.x of thing must be a number, found string`, `unknown key 'z' in p.x of thing`, and a missing required key at the sub-table's name location.

**`crates/engine/tests/pops.rs`** (`mod common;`). A `types_of(&LoadedSetting)` helper like `tests/resources.rs`'s returns each type as (index, name, endowment as `Vec<(good name, f64)>`, utility as `Vec<(good name, f64, f64)>`), and an `assert_types` helper compares them with the share tolerance.
- **AC "sample loads with two POP types"**: load the sample; `pop_types.len() == 2`; the types as listed above; every type's shares sum to 1 within tolerance; `pop_types.id("peasant")` is ID 0.
- **AC "test setting with several files, both forms, ..."**: load `pops_valid`; the three types as listed above; shares sum to 1 per type; `endowment_of(labor)` is 8 for `fisher` and zero for `idle`; `utility_of(fish)` for `fisher` has subsistence 1 and share 0; `utility_of(coin)` for `farmer` is None; loading twice gives equal `types_of`.
- **AC "each error case"**: `failures_of("pops_errors")` equals the expected list (follow `every_resource_error_is_reported_at_its_place`).
- **AC "one entry with several problems"**: covered by the `Several` entry in the list above (assert the list as a whole; no separate test needed).
- **AC "unknown-good checks skipped after a broken file"**: the existing assertion on `resources_broken_file` in `tests/resources.rs` keeps passing; add the comment.
- **AC "a setting with no POP types"**: `valid_minimal` loads with `pop_types.is_empty()` and `len() == 0`; `pop_types.id("missing")` is `Err(IdError::UnknownName { kind: "pop type", name })`.
- **AC "stepping unchanged"**: the existing `tests/world.rs` and `checker.rs` turn tests pass unchanged.

**Dev UI**: `checker.rs` passes with the updated lines, including `1 pop type` for `pops_one` and `0 pop types` elsewhere (**AC "checker counts POP types; zero reported"**).

## Constraints

- Follow the existing conventions: every key read before any early return; message texts come from `EntryReader` and `Definitions` where they exist, and the new texts in this spec are exact (tests assert on them); domain modules know nothing about files; no `HashMap` iteration (lookup only); doc comments on every public item, including named fields (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`.
- `PopType::new` and `NamedList::from_parts` stay `pub(crate)`. `PopType`'s fields stay private.
- Do not change `World`, the turn, `TurnReport`, the dev UI window, `HexMap`, `HexCurrencies`, `HexPrices`, `Market`, or the checker's output beyond the POP type count. Do not change existing `EntryReader` message texts.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop.
- Out of scope: placing POPs, emitting endowments, demand, trade, growth, any cross-check between endowment and subsistence, any restriction on which goods may be endowed or consumed, showing POP types in the dev UI window.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample setting loads with the expected checker line, and every Acceptance Criterion in `design.md` is met.
