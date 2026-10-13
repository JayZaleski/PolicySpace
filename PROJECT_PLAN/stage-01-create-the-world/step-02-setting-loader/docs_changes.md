# Step 02: setting-loader — Docs Changes

## docs/README.md — modify
- **Location:** the table of documents, after the `ids.md` row
- **Change:** add
- **Supersedes:** none

| [setting-loader.md](setting-loader.md) | The setting folder format, the loader's two phases, error reporting, the section-type framework, and the setting checker |

## docs/setting-loader.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# The setting loader

Source: `crates/engine/src/setting/`. The public API is re-exported from the engine crate root: `load_setting`, `LoadedSetting`, `Manifest`, `Rules`, `StartState`, `LoadError`, `LoadErrors`, `SUPPORTED_FORMAT_VERSION`.

A setting is a folder of TOML files that describes a world. The loader reads the folder, checks it strictly, and returns either a `LoadedSetting` or every problem it found. A loaded setting is not a running world; a later Step builds world state from it. This Step builds the framework only: it defines no section types (goods, hexes and so on), which later Steps add.

## Folder format

```
<setting folder>/
  setting.toml     manifest (required)
  README.md        optional notes
  rules/           persistent rules (required, may hold only a README.md)
  start/           starting state, what turn 0 looks like (required, same)
```

- **Rules** are constants that never change during play, mostly definitions of kinds of things. **Starting state** is the turn-0 value of everything that can change, and refers to rules by name.
- The root may hold only the four entries above. Anything else is an error, as is a missing manifest, `rules/` or `start/`, or an entry of the wrong kind (for example `rules` as a file).
- Every `.toml` file (case-sensitive extension) in `rules/` and `start/` is read, in subfolders at any depth. Files are read in byte order of their relative path (forward slashes), all of `rules/` before all of `start/`. This order decides ID order, so the same folder always loads the same way.
- What a file defines comes from its `[[section]]` headers, not its name or folder. Content from all files is merged by kind.
- `README.md` is allowed in any folder and never read. Ignored everywhere: names starting with `.` (not entered), `Thumbs.db`, `desktop.ini`. Any other file is an error.
- A leading UTF-8 BOM is stripped before parsing. Locations are computed on the stripped text.

### Manifest (`setting.toml`)

| Key | Required | Notes |
|---|---|---|
| `name` | yes | string, not empty after trimming |
| `description` | no | string |
| `format_version` | yes | integer; must equal `SUPPORTED_FORMAT_VERSION` (1). The version changes only when the format breaks existing settings |

Unknown keys are errors.

## Module structure (`crates/engine/src/setting/`)

| File | Contents |
|---|---|
| `mod.rs` | Public types, `Folder`, `SectionType`, `SettingContent`, `WorldContent`, `load_with`, `load_setting`, manifest reading, phase 1 section dispatch |
| `error.rs` | `LoadError`, `LoadErrors` (public); `Location`, `Located<T>`, `ErrorSink` and the error sort (crate-private) |
| `source.rs` | `SourceFile` (relative path, text, line starts, span-to-location) and `read_toml` |
| `discover.rs` | Folder-structure checks and discovery of the manifest and data files |
| `reader.rs` | `valid_name` and `EntryReader` |
| `defs.rs` | `Definitions<K>` |
| `test_content.rs` | `#[cfg(test)]` test-only content type and framework tests |

Everything except the items re-exported from the crate root is `pub(crate)`. The engine depends on exactly one external crate for this: `toml` (default features off, `parse` and `std` on, no serde), used for its parser only. `tempfile` is a dev-dependency.

## How loading works

`load_with::<C: SettingContent>(path)` is the generic loader. `load_setting` is `load_with::<WorldContent>` plus building a `LoadedSetting`.

1. **Setting folder.** A path that is not a folder gives one error with no file path.
2. **Discovery** (`discover.rs`). Checks the root, reports structure problems, and returns the manifest path and the sorted list of data files, each tagged with its `Folder`.
3. **Manifest.** Read with `read_toml` and an `EntryReader` labelled `manifest`. A `Manifest` is produced only if `name` and `format_version` are both valid.
4. **Phase 1.** For each data file in order, `read_toml` parses it. For each top-level key, `read_sections` looks it up among the content's section types:
   - unknown header: `unknown section '<key>'`;
   - known header in the wrong folder: `section '<key>' belongs in rules/, not start/` (contents not read);
   - value not an array of tables: `section '<key>' must be written as [[<key>]]`. The inline form `x = [{...}]` is the same TOML data and is accepted;
   - otherwise each entry gets an `EntryReader`, the section's `read` function runs, then `finish()` reports unread keys.
5. **Phase 2.** `SettingContent::resolve` checks references now that every file has been read, so a reference can point into any file. It is skipped if any file was rejected (could not be read, not UTF-8, or invalid TOML), because the missing definitions may be in the broken file. Other phase 1 errors (unknown keys, bad names, duplicates) do not skip phase 2.
6. **Finish.** `ErrorSink::into_result` sorts the errors and returns `Err(LoadErrors)` if there are any. There is never a partly loaded setting.

**Known gap (planned for Step 03):** phase 2 is skipped only when a file is rejected. It is not skipped when discovery could not see some data files (a missing `rules/` or `start/`, an unreadable subfolder, or a bad file name), so references into those files can produce misleading "unknown" errors alongside the structure error. Step 03 is to fix this by also skipping phase 2 in those cases.

### Section types and content

```rust
pub(crate) struct SectionType<C> {
    pub header: &'static str,       // the [[header]] name
    pub folder: Folder,             // the only folder it may appear in
    pub read: fn(&mut C, &mut EntryReader<'_, '_>),
}

pub(crate) trait SettingContent: Default {
    fn section_types() -> Vec<SectionType<Self>>;
    fn resolve(&mut self, sink: &mut ErrorSink);   // phase 2
}
```

`WorldContent` (holding `Rules` and `StartState`) is the real content type. It currently returns no section types, so any section header in a real setting is reported as unknown. Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

### Adding a section type

1. Define the data and store it in `Rules` or `StartState` (add fields; they are plain structs, not `Clone`/`PartialEq` because name registries are not).
2. Add a `SectionType` to `WorldContent::section_types`, naming its header, its one folder, and a `read` function.
3. In `read`, use the `EntryReader` accessors for fields, `Definitions<K>` to define names, and keep any references unresolved (as `Located<String>`).
4. Resolve references in `WorldContent::resolve`.

The `read` function never calls `finish`; the loader does. Remove the `#[allow(dead_code)]` markers on `SectionType` and on the `Definitions` methods that tests alone use today, once real code uses them.

## EntryReader

`EntryReader` reads one table: a `[[section]]` entry or the manifest's root table. It is hand-written rather than serde-based because serde stops at the first error, and the loader must report all of them.

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>`, where `Located` carries the value and its `Location`.
- Each accessor marks its key as read even when the value is invalid, so a wrong-type key is not also reported as unknown. On a problem it reports an error and returns `None`: missing required key, wrong type, invalid name, or out-of-range integer.
- `errors()` gives access to the `ErrorSink` for section-specific checks. `location()` is the entry's header (for the manifest, the file start).
- `finish()` reports every key no accessor asked for, as `unknown key '<key>' in <label>`, at the key.
- `valid_name(name)`: an ASCII lowercase letter, then only ASCII lowercase letters, digits and `_`. This is the name-syntax check that `ids.md` leaves to the loader.

Because the toml tables iterate in sorted key order and arrays in source order, reading is deterministic. Final error order does not depend on it anyway.

## Definitions

`Definitions<K: IdKind>` wraps a `NameRegistry<K>` and remembers where each name was defined (`locations[id.index()]`).

- `define(name, sink)` registers the name, returning its `Id<K>`. On a duplicate it reports at the new definition and names the first: `duplicate thing name 'copper' (first defined at rules/a.toml:2:8)`.
- `resolve(reference, sink)` looks a name up. If unknown it reports at the reference: `unknown thing 'tin'`.
- Also `registry()`, `location(id)`, `into_registry()`.

IDs are handed out in registration order, so they follow file read order.

## Errors and locations

- `Location` is a path relative to the setting folder (forward slashes) plus a 1-based line and a 1-based column counted in characters, computed from byte spans by `SourceFile::location`.
- `ErrorSink` collects errors with `at` (positioned), `in_file` (path only) and `general` (neither). Nothing returns early on an error; problems accumulate.
- `LoadError` has accessors `path()`, `line()`, `column()` and `message()`. `Display` is `rules/metals.toml:14:3: message`, `rules/notes.txt: message`, or just `message`.
- `LoadErrors` is never empty. It has `iter()`, `len()`, `is_empty()` (always false) and `IntoIterator for &LoadErrors`. `Display` writes one error per line with no trailing newline. It implements `std::error::Error`.
- **Order.** `into_result` stable-sorts by: path group (no path, then `setting.toml`, then other root entries, then `rules/`, then `start/`), path bytes, line (none first), column. Errors at the same position keep their reporting order.
- **Broken files.** `read_toml` reads bytes, checks UTF-8 (reporting at the first invalid byte), strips the BOM, and parses with `parse_recoverable`. Every syntax error is reported as `invalid TOML: <message>`, and the file's content is then not checked, to avoid a cascade of misleading errors.

Message texts are asserted exactly by the tests; change them deliberately.

## The sample setting and test settings

- `settings/sample/` at the repository root is the project's sample setting: a manifest plus `rules/` and `start/` holding only `README.md` files. Later Steps add content to it. It must always load without errors.
- `crates/engine/tests/settings/` holds small setting folders for the loader's tests, each named for what it tests (`valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, and the `framework_*` folders used with the test-only content type). Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## Setting checker (dev UI)

`crates/dev-ui/src/main.rs` calls only the engine's public API:

| Arguments | Behavior | Exit code |
|---|---|---|
| none | prints `{ENGINE_NAME} {ENGINE_VERSION}` | 0 |
| one path | loads the setting; on success prints `Loaded setting '<name>' (format version <n>)` to stdout | 0 |
| one path, errors | prints each error on its own line to stderr, then `<n> error(s) found in <path>` | 1 |
| more than one | prints `usage: dev-ui [SETTING_FOLDER]` to stderr | 2 |

Run it with `cargo run -p policyspace-dev-ui -- settings/sample`. It is still a placeholder; a later Step replaces the dev UI and decides whether the checker survives.

## Tests

- Unit tests sit in each `setting` module. Framework tests that use `TestContent` are in `test_content.rs`.
- `crates/engine/tests/setting_loader.rs` loads the test settings through the public API and asserts on exact `Display` strings, including line and column.
- `crates/dev-ui/tests/checker.rs` runs the built binary against the sample and a broken setting. `placeholder.rs` still covers the no-argument case.

## docs/workspace.md — modify
- **Location:** "Crates and the dependency rule" and "Layout" and "Tests" sections
- **Change:** replace
- **Supersedes:** step-01-engine-workspace/docs_changes.md (the workspace.md text about the layout, the dev UI placeholder, no external dependencies, and test settings)

Layout block, replacing the `crates/` and `tools/` lines:

```
crates/
  engine/               policyspace-engine (library)
    src/lib.rs
    src/id.rs
    src/setting/        the setting loader (see setting-loader.md)
    tests/setting_loader.rs
    tests/settings/     small setting folders used by engine tests
  dev-ui/               policyspace-dev-ui (binary `dev-ui`)
    src/main.rs
    tests/placeholder.rs
    tests/checker.rs
settings/sample/        the project's sample setting
tools/check.py          the single check command
```

Replace the dev UI bullet with: **`policyspace-dev-ui`** is the developer program. It depends on the engine by path and uses only the engine's public interface. Today it is a placeholder with one use: with no argument it prints `"{ENGINE_NAME} {ENGINE_VERSION}"` and exits 0, and with a setting folder path it acts as a setting checker (see [setting-loader.md](setting-loader.md)). A later Step replaces it with a real developer UI.

Replace the "no external dependencies" bullet with: The engine's only external dependency is `toml` (parser only, no serde; see [setting-loader.md](setting-loader.md)), plus the dev-dependency `tempfile`. Add a dependency only when a Step needs it.

Replace the last Tests bullet with: `crates/engine/tests/settings/` holds small setting folders for engine tests, described in [setting-loader.md](setting-loader.md). Each test setting should be minimal and named for what it tests. Add `crates/dev-ui/tests/checker.rs` to the dev UI test description: it runs the binary against the sample setting and a broken one.

## docs/ids.md — modify
- **Location:** "Rules" section and "Adding a new ID kind"
- **Change:** add
- **Supersedes:** none

Add to "Adding a new ID kind", step 2: Registration happens in the setting loader, through `Definitions<K>`, which wraps the registry and records where each name was defined; see [setting-loader.md](setting-loader.md). Name syntax (lowercase `snake_case`) is checked by the loader's `valid_name` before a name reaches the registry.
