# Step 02: setting-loader — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-02-setting-loader/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-01-engine-workspace/spec.md` and `docs_changes.md`: what Step 01 built (workspace, lints, `tools/check.py`, the ID mechanism).
- `crates/engine/src/id.rs`: `IdKind`, `Id<K>`, `NameRegistry<K>`, `IdError`. This Step builds on them.
- `crates/engine/src/lib.rs`, `crates/dev-ui/src/main.rs`, `crates/dev-ui/tests/placeholder.rs`: the code this Step extends.
- `Cargo.toml` (workspace): lints are `missing_docs = "warn"` and clippy `all`, and `tools/check.py` runs clippy with `-D warnings`. Every public item, including the fields of public structs and enum variants, needs a doc comment.

## Approach
- **TOML is parsed with the `toml` crate (1.1), using only its parser.** `toml::de::DeTable::parse_recoverable` returns a tree in which every key and value carries its byte span, plus a list of all syntax errors in the file. This is the engine's first external dependency. Turn its default features off and enable only `parse` and `std`. No serde.
- **Files are read by walking the parsed tree by hand, through a small field-reader helper (`EntryReader`)**, not by serde derive. Serde stops at the first error, and the design requires every error to be reported in one load. The helper records every problem (missing keys, unknown keys, wrong types, bad names) and keeps going. Later Steps define their sections with it.
- **Every problem goes into one error collector (`ErrorSink`)** instead of being returned early. At the end of the load, the errors are sorted into a fixed order. If there are any, the load fails.
- **The loader is generic over a "content" type that declares its section types (`SettingContent` trait).** The real content type declares none in this Step. A test-only content type declares a "thing" section (in `rules/`) and a "placed thing" section (in `start/`, referring to a thing by name), so the shared rules can be tested now: duplicates across files, wrong folder, and references between files. Steps 03–05 add their section types to the real content type.
- **Two phases.** Phase 1 reads every file in order, and each section entry is handed to its section type's `read` function, which defines names and stores unresolved references. Phase 2 (`resolve`) checks references once every file has been read, so references can cross files.
- **A `Definitions<K>` helper** wraps `NameRegistry<K>` and remembers where each name was defined. A duplicate is reported at the new definition, naming the first one's location. A reference is resolved with an "unknown …" error at the reference's location.
- **Locations** are the path relative to the setting folder, always written with forward slashes, plus a 1-based line and column (the column counts characters, not bytes), computed from byte spans.
- **The dev UI becomes a setting checker** when given a path. It calls only the engine's public API.

## Changes
Create:
- `crates/engine/src/setting/mod.rs`: module docs; public `load_setting`, `LoadedSetting`, `Manifest`, `Rules`, `StartState`, `SUPPORTED_FORMAT_VERSION`; crate-private `Folder`, `SectionType`, `SettingContent`, `load_with`, and the real content type `WorldContent`.
- `crates/engine/src/setting/error.rs`: public `LoadError`, `LoadErrors`; crate-private `Location`, `Located<T>`, `ErrorSink`, and the error sort.
- `crates/engine/src/setting/source.rs`: crate-private `SourceFile` (relative path, text, line starts) with span-to-location conversion, and file reading (UTF-8 check, BOM strip, parse).
- `crates/engine/src/setting/discover.rs`: crate-private discovery of the setting folder's structure and of the `.toml` files in `rules/` and `start/`.
- `crates/engine/src/setting/reader.rs`: crate-private `EntryReader` and `valid_name`.
- `crates/engine/src/setting/defs.rs`: crate-private `Definitions<K>`.
- `crates/engine/src/setting/test_content.rs`: `#[cfg(test)]` test-only content type (`ThingKind`, `TestContent`) and the framework tests that use it.
- `crates/engine/tests/setting_loader.rs`: integration tests through the public API.
- Test setting folders under `crates/engine/tests/settings/` (listed under Test Plan).
- `settings/sample/setting.toml`, `settings/sample/README.md`, `settings/sample/rules/README.md`, `settings/sample/start/README.md`.
- `crates/dev-ui/tests/checker.rs`: integration tests for the setting checker.

Modify:
- `crates/engine/Cargo.toml`: add `toml` and the dev-dependency `tempfile`.
- `crates/engine/src/lib.rs`: `pub mod setting;` and re-exports. Mention setting loading in the crate docs.
- `crates/engine/tests/settings/README.md`: describe the folders now in it (see Test Plan).
- `crates/dev-ui/src/main.rs`: argument handling and the setting checker.
- `Cargo.lock`: updated by cargo.

## Interfaces & Data

### Dependencies (`crates/engine/Cargo.toml`)
```toml
[dependencies]
toml = { version = "1.1", default-features = false, features = ["parse", "std"] }

[dev-dependencies]
tempfile = "3"
```
API used: `toml::de::{DeTable, DeValue, DeArray, DeInteger}`, `toml::Spanned` (`get_ref()`, `span()`), `DeTable::parse_recoverable(&str) -> (Spanned<DeTable>, Vec<toml::de::Error>)`, `toml::de::Error::{message(), span()}`, `DeValue::{as_str, as_integer, as_bool, as_table, as_array, type_str}`, and `DeInteger::{as_str, radix}` (convert with `i64::from_str_radix(int.as_str(), int.radix())`; failure means out of range). Without `preserve_order`, a `DeTable` iterates its keys in sorted order. That's deterministic and fine, because errors are sorted at the end anyway, and the order of an array's entries (`[[x]]` sections) is always source order.

### Public API (re-exported from the crate root)
```rust
pub mod setting;
pub use setting::{load_setting, LoadError, LoadErrors, LoadedSetting, Manifest, Rules, StartState, SUPPORTED_FORMAT_VERSION};
```

```rust
/// The only setting format version this engine reads.
pub const SUPPORTED_FORMAT_VERSION: i64 = 1;

/// Loads the setting folder at `path`. Either every check passes and a LoadedSetting is
/// returned, or every problem found is returned (sorted; see "Error order").
pub fn load_setting(path: impl AsRef<Path>) -> Result<LoadedSetting, LoadErrors>;

#[derive(Debug)]
pub struct LoadedSetting {
    /// The manifest (setting.toml).
    pub manifest: Manifest,
    /// Persistent rules (from rules/). Empty until Step 03.
    pub rules: Rules,
    /// Starting state (from start/). Empty until Step 04.
    pub start: StartState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Setting format version; always SUPPORTED_FORMAT_VERSION after a successful load.
    pub format_version: i64,
}

/// Persistent rules. No fields yet; Steps 03 and 05 add them.
#[derive(Debug, Default)]
pub struct Rules {}

/// Starting state (turn 0). No fields yet; Steps 04 and 05 add them.
#[derive(Debug, Default)]
pub struct StartState {}
```
Don't derive `Clone` or `PartialEq` on `LoadedSetting`, `Rules`, or `StartState`. `NameRegistry` implements neither, and later Steps put registries in these types.

```rust
/// One problem found while loading a setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError { path: Option<String>, line: Option<u32>, column: Option<u32>, message: String }

impl LoadError {
    /// Path relative to the setting folder, with forward slashes (None for problems with the folder itself).
    pub fn path(&self) -> Option<&str>;
    /// 1-based line, if the problem has a position.
    pub fn line(&self) -> Option<u32>;
    /// 1-based column in characters, if the problem has a position.
    pub fn column(&self) -> Option<u32>;
    /// The message, without the location.
    pub fn message(&self) -> &str;
}
```
`Display` for `LoadError`:
- path, line and column: `rules/metals.toml:14:3: message`
- path only: `rules/notes.txt: message`
- neither: `message`

(`line` and `column` are always both present or both absent.)

```rust
/// Every problem found in one failed load, in sorted order. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadErrors(Vec<LoadError>);

impl LoadErrors {
    /// The errors, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, LoadError>;
    /// Number of errors (at least 1).
    pub fn len(&self) -> usize;
}
```
- `Display` writes one error per line, separated by `\n`, with no trailing newline.
- Implement `std::error::Error`.
- Clippy's `len_without_is_empty` will fire: add `is_empty()` (always false, documented as such). Don't suppress the lint.

### Crate-private framework

**Locations (`error.rs`)**
```rust
pub(crate) struct Location { pub path: String, pub line: u32, pub column: u32 }   // Clone, Debug, PartialEq, Eq
impl Display for Location   // "rules/a.toml:3:9"
pub(crate) struct Located<T> { pub value: T, pub at: Location }          // Clone, Debug
pub(crate) struct ErrorSink { errors: Vec<LoadError> }
impl ErrorSink {
    pub fn new() -> Self;
    pub fn at(&mut self, at: &Location, message: impl Into<String>);        // positioned error
    pub fn in_file(&mut self, path: &str, message: impl Into<String>);      // path only
    pub fn general(&mut self, message: impl Into<String>);                  // no path
    pub fn is_empty(&self) -> bool;
    pub fn into_result<T>(self, value: T) -> Result<T, LoadErrors>;          // sorts; Ok only if empty
}
```

**Error order.** `into_result` stable-sorts errors by this key:
1. A group number based on the path: no path = 0; exactly `setting.toml` = 1; starts with `rules/` = 3; starts with `start/` = 4; anything else (other root entries) = 2.
2. The path string, in byte order.
3. The line, with `None` before any line.
4. The column.

The stable sort keeps errors at the same position in the order they were reported. This gives "by file in read order, then by position" from the design.

**Source files (`source.rs`)**
```rust
pub(crate) struct SourceFile { pub path: String, text: String, line_starts: Vec<usize> }
impl SourceFile {
    pub fn location(&self, byte_offset: usize) -> Location;   // 1-based line; column = chars from line start + 1
    pub fn location_of(&self, span: Range<usize>) -> Location; // uses span.start
}
/// Reads `abs_path` (shown as `rel_path` in errors) and parses it. Returns None, having
/// reported errors, if the file can't be read, isn't UTF-8, or has any TOML syntax error.
pub(crate) fn read_toml(abs_path: &Path, rel_path: &str, sink: &mut ErrorSink)
    -> Option<(SourceFile, /* owned parsed table */)>;
```
- Read the file as bytes.
  - An I/O failure is an `in_file` error: `could not read file: {io error}`.
  - Invalid UTF-8 is an error located at the first invalid byte: `Utf8Error::valid_up_to()`, with the line and column computed over the valid prefix. The message is `file is not valid UTF-8`.
- Strip one leading UTF-8 BOM (`\u{FEFF}`) before parsing. Compute every location on the stripped text.
- Parse with `DeTable::parse_recoverable`. Each parse error becomes `invalid TOML: {message}`, at `location(span.start)` when it has a span, or `in_file` when it doesn't. If there were any parse errors, return `None`, so the file's content isn't validated.
- The parsed tree borrows the text. Either call `make_owned()` on it, or structure the code so the tree is used while the text is still borrowed. This is the builder's choice; the signature above is indicative.
- `read_toml` also records whether the file was rejected (a read, UTF-8 or TOML error). `load_with` needs this to skip phase 2 (see below).

**Name syntax (`reader.rs`)**
```rust
/// True if `name` is a valid text name: an ASCII lowercase letter first, then only
/// ASCII lowercase letters, digits, and '_'.
pub(crate) fn valid_name(name: &str) -> bool;
```

**EntryReader (`reader.rs`)**
It reads one table: one `[[section]]` entry, or the manifest's root table.
```rust
pub(crate) struct EntryReader<'a, 'i> { /* &SourceFile, &DeTable<'i>, entry Location, label: &'static str, keys read so far, &mut ErrorSink */ }
impl EntryReader<'_, '_> {
    pub fn location(&self) -> &Location;                                   // the entry's header (or file start for the manifest)
    pub fn required_string(&mut self, key: &str) -> Option<Located<String>>;
    pub fn optional_string(&mut self, key: &str) -> Option<Located<String>>;
    pub fn required_name(&mut self, key: &str) -> Option<Located<String>>; // string + valid_name check
    pub fn required_integer(&mut self, key: &str) -> Option<Located<i64>>;
    pub fn optional_integer(&mut self, key: &str) -> Option<Located<i64>>;
    pub fn required_bool(&mut self, key: &str) -> Option<Located<bool>>;
    pub fn optional_bool(&mut self, key: &str) -> Option<Located<bool>>;
    pub fn errors(&mut self) -> &mut ErrorSink;                            // for section-specific checks and Definitions
    pub fn finish(self);                                                   // reports every key never asked for
}
```
- Each accessor marks the key as read, even when its value is invalid, so a wrong-type key isn't also reported as unknown.
- Each accessor returns `None` after reporting when the key is missing (required only), has the wrong type, is an invalid name, or is an out-of-range integer.
- `finish` reports each table key that no accessor asked for, at the key's span.
- The loader calls `finish` itself after a section's `read` function returns. Section code never calls it.
- The entry's location is the span of the array element's `Spanned<DeValue>`, which covers the `[[section]]` header. Missing-key errors use it. For the manifest root table, missing-key errors use `in_file("setting.toml", …)`.
- `label` is the section header name (`thing`) or `manifest`.

**Definitions (`defs.rs`)**
```rust
pub(crate) struct Definitions<K: IdKind> { registry: NameRegistry<K>, locations: Vec<Location> } // locations[id.index()]
impl<K: IdKind> Definitions<K> {
    pub fn new() -> Self;                        // plus a hand-written Default
    /// Registers the name. On a duplicate, reports at name.at, names the first definition's location, and returns None.
    pub fn define(&mut self, name: &Located<String>, sink: &mut ErrorSink) -> Option<Id<K>>;
    /// Looks up a reference. If unknown, reports at reference.at and returns None.
    pub fn resolve(&self, reference: &Located<String>, sink: &mut ErrorSink) -> Option<Id<K>>;
    pub fn registry(&self) -> &NameRegistry<K>;
    pub fn location(&self, id: Id<K>) -> &Location;
    pub fn into_registry(self) -> NameRegistry<K>;
}
```
- Use `IdError`'s `Display` for the base text of both messages.
- Some of these methods are used only by tests until Step 03. Allow `dead_code` narrowly on just those items, with a comment saying Step 03 uses them. Don't allow it module-wide.

**Section types and content (`mod.rs`)**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Folder { Rules, Start }   // dir_name(): "rules" / "start"

pub(crate) struct SectionType<C> {
    /// The [[header]] name.
    pub header: &'static str,
    /// The only folder this section may appear in.
    pub folder: Folder,
    /// Reads one entry into the content. Runs in phase 1, in file order then entry order.
    pub read: fn(&mut C, &mut EntryReader<'_, '_>),
}

pub(crate) trait SettingContent: Default {
    /// The section types this content accepts.
    fn section_types() -> Vec<SectionType<Self>>;
    /// Phase 2: resolve references once every file has been read.
    fn resolve(&mut self, sink: &mut ErrorSink);
}

/// The real content. No section types until Step 03.
#[derive(Default)]
pub(crate) struct WorldContent { pub rules: Rules, pub start: StartState }
// section_types() -> Vec::new(); resolve -> no-op

/// The generic loader. `load_setting` is load_with::<WorldContent> plus building LoadedSetting.
pub(crate) fn load_with<C: SettingContent>(path: &Path) -> Result<(Manifest, C), LoadErrors>;
```

### Loading algorithm (`load_with`)
1. **Setting folder.** If `path` doesn't exist or isn't a directory, report `general("setting folder '{path as given}' does not exist or is not a folder")` and return the errors immediately.
2. **Root entries (`discover.rs`).** List the root (follow symlinks; use `fs::metadata`).
   - **Ignored** anywhere in the setting: names starting with `.` (files and folders, which aren't entered), and files named exactly `Thumbs.db` or `desktop.ini`.
   - **Allowed at the root:** file `setting.toml`, file `README.md`, folder `rules`, folder `start`.
   - **Wrong kind:** `setting.toml` or `README.md` as a folder, or `rules`/`start` as a file, is an `in_file` error with message `expected a file, found a folder` or `expected a folder, found a file`. The path is the bare name.
   - **Anything else:**
     - a file: `in_file("<name>", "unexpected file in setting root (allowed: setting.toml, README.md, rules/, start/)")`;
     - a folder: `in_file("<name>/", "unexpected folder in setting root (allowed: setting.toml, README.md, rules/, start/)")`.
   - **Missing:**
     - `in_file("setting.toml", "missing manifest file")`;
     - `in_file("rules/", "missing required folder")`, and the same for `start/`.
   - An entry name that isn't valid UTF-8 is an `in_file` error at its lossy name: `file name is not valid UTF-8`.
   - Any I/O error listing a folder is an `in_file` error at that folder's path: `could not read folder: {error}`.
3. **Manifest.** If `setting.toml` exists, `read_toml` it. If that succeeds, read the root table with an `EntryReader` labeled `manifest`:
   - `name`: required string, not empty after trimming. If empty: `key 'name' in manifest must not be empty`, at the value.
   - `description`: optional string.
   - `format_version`: required integer. If it isn't `SUPPORTED_FORMAT_VERSION`: `unsupported format_version {n} (this engine supports format version 1)`, at the value.
   - `finish()`.
   - Produce a `Manifest` only if all three succeed.
4. **Discover data files.** For each of `rules/` and `start/` that exists, walk it recursively. Skip ignored entries. Files named exactly `README.md` are allowed and skipped. Files whose name ends in `.toml` (case-sensitive) are collected. Any other file is `in_file("<rel path>", "unexpected file (only .toml files and README.md are allowed here)")`. Collect each file's relative path with forward slashes, and sort each folder's list by that string's bytes. All `rules/` files come before all `start/` files.
5. **Phase 1.** For each data file in order: `read_toml`. On success, for each top-level `(key, value)` of the file:
   - **Known section type, right folder.** The value must be an array, and every element a table. That covers `[[x]]`; the equivalent `x = [{…}]` is accepted too, since it is the same TOML data. For each element in order, create an `EntryReader` (label = header), call the section's `read`, then `finish`.
     - If the value isn't an array: `section '{key}' must be written as [[{key}]]`, at the key.
     - If an element isn't a table: the same message, at the element.
   - **Known section type, wrong folder:** `section '{key}' belongs in {right}/, not {this}/`, at the key. Its contents aren't read.
   - **Not a known section type:** `unknown section '{key}'`, at the key.
6. **Phase 2.** If no file was rejected by `read_toml` (a read, UTF-8 or TOML error), call `content.resolve(&mut sink)`. Otherwise skip it, because definitions may be missing from the broken file. Other errors from phase 1 (unknown keys, bad names, duplicates) don't skip phase 2.
7. **Finish.** `sink.into_result((manifest, content))`. The manifest is `Some` whenever the sink is empty; unwrap it only inside the `Ok` branch, or restructure to avoid the unwrap.

### Message catalogue (exact text; tests assert on it)
| Situation | Message (after the location) |
|---|---|
| Missing key | `missing required key '{key}' in {label}` |
| Unknown key | `unknown key '{key}' in {label}` |
| Wrong type | `key '{key}' in {label} must be a {expected}, found {type_str}`, where `{expected}` is `string`, `integer` or `boolean`, and `{type_str}` is `DeValue::type_str()` |
| Integer out of range | `key '{key}' in {label} is out of range` |
| Invalid name | `invalid name '{name}' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)` |
| Duplicate definition | `{IdError::DuplicateName display} (first defined at {location})`, e.g. `duplicate thing name 'copper' (first defined at rules/a.toml:2:8)` |
| Unknown reference | `{IdError::UnknownName display}`, e.g. `unknown thing 'tin'` |
| Unknown section | `unknown section '{key}'` |
| Wrong folder | `section '{key}' belongs in rules/, not start/` (or the reverse) |
| Not an array of tables | `section '{key}' must be written as [[{key}]]` |
| Invalid TOML | `invalid TOML: {toml error message}` |
| Not UTF-8 | `file is not valid UTF-8` |
| Others | as given in the Loading algorithm |

### Test-only content (`test_content.rs`, `#[cfg(test)]`)
```rust
enum ThingKind {}            // IdKind, LABEL = "thing"
#[derive(Default)]
struct TestContent {
    things: Definitions<ThingKind>,
    placed: Vec<Located<String>>,          // unresolved references, in read order
    resolved: Vec<Id<ThingKind>>,          // filled by resolve
}
// section_types():
//   "thing"        in Rules: reads required_name("name"); defines it in `things`.
//   "placed_thing" in Start: reads required_name("thing") (stored in `placed`), optional_integer("count") and optional_bool("visible") (read and discarded; they exist to exercise the readers).
// resolve(): for each entry in `placed`, `things.resolve(..)`; push successes to `resolved`.
```

### Sample setting (`settings/sample/`)
- `setting.toml`:
  ```toml
  name = "Sample Setting"
  description = "The project's sample setting. Each Step of Stage 01 adds to it."
  format_version = 1
  ```
- `README.md`: one short paragraph describing the sample setting and the folder layout, with `rules/` for persistent rules and `start/` for turn-0 state. It notes that `README.md` is the place for notes, since any other non-`.toml` file is an error.
- `rules/README.md`, `start/README.md`: one or two lines each, saying what belongs in the folder.

### Dev UI (`crates/dev-ui/src/main.rs`)
- `fn main() -> std::process::ExitCode`.
- Arguments (`std::env::args_os().skip(1)`):
  - **None:** print `{ENGINE_NAME} {ENGINE_VERSION}` to stdout and exit 0, unchanged.
  - **One:** `load_setting(path)`.
    - On `Ok`: print `Loaded setting '{name}' (format version {format_version})` to stdout and exit 0.
    - On `Err(errors)`: print each error on its own line to stderr, then `{n} error(s) found in {path}` (with `path.display()`), and exit 1.
  - **More than one:** print `usage: dev-ui [SETTING_FOLDER]` to stderr and exit 2.
- Update the crate docs: still a placeholder that Step 07 replaces, now with a setting-checker mode.
- Only the engine's public API is used.

## Tasks
1. Add the `toml` dependency and the `tempfile` dev-dependency to `crates/engine/Cargo.toml`. `cargo build` to update `Cargo.lock`.
2. `error.rs`: `Location`, `Located`, `LoadError` (+ `Display`), `LoadErrors` (+ `Display`, `Error`), `ErrorSink` with the sort. Unit tests.
3. `source.rs`: `SourceFile`, location computation, `read_toml` (bytes → UTF-8 → BOM strip → `parse_recoverable`). Unit tests.
4. `reader.rs`: `valid_name` and `EntryReader`. Unit tests.
5. `defs.rs`: `Definitions<K>`. Unit tests.
6. `discover.rs`: root checks and data-file discovery. Unit tests (with `tempfile`).
7. `mod.rs`: public types, `Folder`, `SectionType`, `SettingContent`, `WorldContent`, `load_with`, `load_setting`. Module docs explaining the folder layout, rules versus start, the two phases, and how a later Step adds a section type (add a `SectionType` to `WorldContent::section_types`, store data in `Rules`/`StartState`, and resolve in `resolve`).
8. Re-export from `lib.rs`. Update the crate docs.
9. `test_content.rs`: the test content type and the framework tests.
10. Create the test setting folders and update `crates/engine/tests/settings/README.md`.
11. `crates/engine/tests/setting_loader.rs`: integration tests.
12. Create `settings/sample/`.
13. Dev UI: argument handling and checker; `crates/dev-ui/tests/checker.rs`.
14. Run `python tools/check.py` and fix everything until it passes. Run `cargo run -p policyspace-dev-ui -- settings/sample` and a broken test setting, and check the output by eye.

## Test Plan

### Test settings (`crates/engine/tests/settings/`)
Every folder (and every `rules/` or `start/` in it) contains at least one file, so git keeps it. Use `README.md` where there's no other file. Comment-only `.toml` files are valid: they have no sections.

| Folder | Contents |
|---|---|
| `valid_minimal` | manifest with `name`, `description`, `format_version = 1`; `rules/README.md`, `start/README.md` |
| `valid_nested` | valid manifest; comment-only `.toml` files at `rules/b.toml`, `rules/a.toml`, `rules/a/z.toml`, `start/x/y/deep.toml`; `README.md` at root, in `rules/`, `start/`, `rules/a/` |
| `missing_manifest` | `rules/README.md`, `start/README.md` only |
| `missing_rules` | manifest, `start/README.md` |
| `missing_start` | manifest, `rules/README.md` |
| `unexpected_root_entry` | valid setting plus root file `notes.txt` and root folder `extra/` (containing `README.md`) |
| `manifest_missing_fields` | manifest with only `description`, so `name` and `format_version` are missing |
| `manifest_bad_values` | `name = ""`, `format_version = 2`, `description = 5`, and an unknown key `author = "x"` |
| `invalid_toml` | valid manifest; `rules/broken.toml` with two separate syntax errors on different lines |
| `unknown_section` | valid manifest; `rules/goods.toml` with a `[[good]]` section; `start/map.toml` with a `[[hex]]` section |
| `non_toml_file` | valid manifest; `rules/notes.txt`, `start/sub/data.json` |
| `many_errors` | missing `start/`; manifest `format_version = 3`; `rules/a.toml` with an unknown section; `rules/b.txt` |
| `framework_valid` | `rules/b.toml`: thing `copper`; `rules/a.toml`: thing `iron`; `rules/a/z.toml`: thing `tin`; `start/places.toml`: placed things referring to `copper` and `tin`, one with `count = 3` and `visible = true` |
| `framework_duplicate` | `rules/a.toml` and `rules/b.toml` both define thing `copper` |
| `framework_wrong_folder` | a `[[thing]]` in `start/s.toml` and a `[[placed_thing]]` in `rules/r.toml` |
| `framework_bad_reference` | thing `copper` in `rules/`; placed thing referring to `bronze` in `start/` |
| `framework_field_errors` | one `[[thing]]` entry with `name = "Copper"` and an unknown key `colour = "red"`; one `[[placed_thing]]` with `thing = 5` and `count = "three"`; one `[[thing]]` with no `name`; `rules/x.toml` with `thing = { name = "a" }` |

Settings that need files git won't keep reliably (hidden files, `Thumbs.db`, `desktop.ini`, invalid UTF-8 bytes, a BOM) are built at runtime in a `tempfile::TempDir`.

### Tests
Engine unit tests live in the `setting` modules; framework tests that use `TestContent` live in `test_content.rs`. Engine integration tests are in `crates/engine/tests/setting_loader.rs`, and dev UI tests in `crates/dev-ui/tests/checker.rs`. Locate the test settings with `Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/settings/<name>")`. Assert on exact `Display` strings where practical, including exact line and column.

| # | Test | Acceptance Criterion |
|---|---|---|
| 1 | `load_setting("<repo>/settings/sample")` is `Ok`, with name `Sample Setting` and format version 1 (path via `CARGO_MANIFEST_DIR/../../settings/sample`). | sample setting loads |
| 2 | `valid_minimal` loads. Its manifest has the expected name, `Some(description)`, and `format_version == 1`. | valid test setting loads with manifest fields |
| 3 | `valid_nested` loads. A discovery unit test on it returns `rules/a.toml, rules/a/z.toml, rules/b.toml, start/x/y/deep.toml`, in that order. | nested subfolders, sorted full-path order; README.md allowed |
| 4 | `missing_manifest`, `missing_rules`, `missing_start` each give exactly the expected error. | missing setting.toml / rules / start |
| 5 | `unexpected_root_entry` gives `notes.txt: unexpected file …` and `extra/: unexpected folder …`. | unexpected root entry |
| 6 | `manifest_missing_fields` gives both missing-key errors. `manifest_bad_values` gives: empty name, unsupported version, `description` wrong type, and unknown key `author`, each at its exact line and column. | missing name/format_version, unsupported version, unknown manifest key |
| 7 | `invalid_toml` gives two `invalid TOML: …` errors with different line numbers, and no other errors for that file. | invalid TOML |
| 8 | A temp setting with `rules/bad.toml` containing valid text, then the bytes `0xFF 0xFE` on line 2, gives `rules/bad.toml:2:{col}: file is not valid UTF-8`. | invalid UTF-8 |
| 9 | `unknown_section` gives `unknown section 'good'` (in rules) and `unknown section 'hex'` (in start), with positions. | unknown section header |
| 10 | `non_toml_file` gives the two unexpected-file errors, with paths `rules/notes.txt` and `start/sub/data.json`. | non-.toml file in rules/start |
| 11 | `valid_name` accepts `copper_ore`, `grain2`, `a`, `a_b_1`, and rejects `Copper`, `copper-ore`, `2grain`, `_copper`, ``, `cöpper`, `copper ore`. | name-syntax check |
| 12 | A temp setting containing `.DS_Store`, `.hidden/` (with a `junk.txt` inside), `Thumbs.db`, `desktop.ini` at the root and in `rules/` and `start/`, and a `setting.toml` starting with a UTF-8 BOM, loads `Ok`. | ignored files; README.md (covered by 3) |
| 13 | `many_errors` gives every expected error, and the full list equals the expected `Vec<String>` in the sorted order: manifest, then `rules/a.toml`, then `rules/b.txt`, then `start/` (group 4). Check the result is `Err`. | all errors in one load, in order; no loaded setting |
| 14 | Loading `framework_valid` twice gives identical `{:?}` output. Loading `many_errors` twice gives equal `LoadErrors`. | same setting twice gives the same result |
| 15 | `load_with::<TestContent>(framework_valid)` is `Ok`. Things are registered in the order `iron` (a.toml), `tin` (a/z.toml), `copper` (b.toml), with IDs 0, 1, 2. `resolved` holds the IDs of `copper` and `tin`, in file order. | test-only: cross-file references; IDs in file-read order |
| 16 | `framework_duplicate` gives `rules/b.toml:L:C: duplicate thing name 'copper' (first defined at rules/a.toml:L:C)`. | test-only: duplicate across files names both places |
| 17 | `framework_wrong_folder` gives `section 'thing' belongs in rules/, not start/` and `section 'placed_thing' belongs in start/, not rules/`. | test-only: wrong folder |
| 18 | `framework_bad_reference` gives `start/…:L:C: unknown thing 'bronze'`, located at the `thing = "bronze"` value. | test-only: unknown reference at the reference |
| 19 | `framework_field_errors` gives all of: invalid name `Copper`, unknown key `colour`, `thing` wrong type (integer), `count` wrong type (string), missing `name` (at the header), and `section 'thing' must be written as [[thing]]`. In particular, the bad-name and unknown-key errors in the same entry are both reported. | test-only: several problems in one section all reported |
| 20 | Phase 2 is skipped after a broken file. A temp setting with `framework_bad_reference`'s content plus `rules/zz.toml` containing a syntax error gives only the TOML error, with no `unknown thing` error. | broken files don't cascade (design: error reporting) |
| 21 | Unit tests: `SourceFile::location` (first line, later lines, a multi-byte character before the column); `LoadError` `Display` for all three shapes; `ErrorSink` sort order across the groups (including `None` line before a line, and stability); `Definitions` define/duplicate/resolve; each `EntryReader` accessor (present, missing, wrong type, out-of-range integer such as `99999999999999999999`); `finish` reports only unread keys. | supports the above |
| 22 | Dev UI with no arguments: the existing `placeholder.rs` test still passes, unchanged. | dev UI no-argument behavior |
| 23 | Dev UI with the sample path: exit 0, and stdout is `Loaded setting 'Sample Setting' (format version 1)`. | checker on the sample |
| 24 | Dev UI with `crates/engine/tests/settings/many_errors`: exit code 1. stderr contains every expected error line and ends with `N error(s) found in …`. | checker on a broken setting |
| 25 | Dev UI with two arguments: exit code 2 and the usage line. | (checker robustness) |
| 26 | `python tools/check.py` exits 0. | single check command passes |

Run everything with `python tools/check.py` (or `.venv/Scripts/python tools/check.py`).

## Constraints
- External crates: only `toml` (features `parse` and `std`, no defaults) and the dev-dependency `tempfile`. No serde, `thiserror`, `anyhow`, `walkdir`, or anything else.
- The dev UI uses only the engine's public API. The engine must not depend on the dev UI or on any UI library.
- Determinism: never iterate a `HashMap` or `HashSet` to produce output or IDs. File order, entry order, and error order are as specified above.
- No real section types (good, hex, resource type, deposit), and no fields in `Rules` or `StartState`. Those belong to Steps 03–05.
- `Definitions`, `EntryReader`, `SectionType`, `SettingContent`, and `load_with` stay `pub(crate)`.
- No `unwrap`/`expect` on anything a setting file controls. Panics are only for programming errors.
- Don't touch `PROJECT_PLAN/`, `CLAUDE.md`, `.claude/`, `docs/`, `manual.ipynb`, `manual.html`, `tools/`, or `crates/engine/src/id.rs`. If `id.rs` seems to need a change, stop and report it.
- Don't commit.
- Out of scope: world state, turns, "did you mean" suggestions, non-folder sources, mod layering, and formats other than TOML.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` exits 0, and every Acceptance Criterion in design.md is met.
