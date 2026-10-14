# The setting loader

Source: `crates/engine/src/setting/`. The public API is re-exported from the engine crate root: `load_setting`, `LoadedSetting`, `Manifest`, `Rules`, `StartState`, `LoadError`, `LoadErrors`, `SUPPORTED_FORMAT_VERSION`.

A setting is a folder of TOML files that describes a world. The loader reads the folder, checks it strictly, and returns either a `LoadedSetting` or every problem it found. A loaded setting is not a running world; `World::new` builds one from it (see [world.md](world.md)). The loader defines four real section types so far: `[[good]]` and `[[resource]]` in `rules/` (see [goods.md](goods.md) and [resources.md](resources.md)) and `[[hex]]` and `[[deposit]]` in `start/` (see [map.md](map.md) and [resources.md](resources.md)); later Steps add more.

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
| `goods.rs` | `GoodsReader`: reads `[[good]]` entries and builds `Goods` |
| `map.rs` | `MapReader`: reads `[[hex]]` entries and builds `HexMap`. Its `coordinate` helper is shared with deposits, and `MapReader::id` resolves coordinates to a `HexId` |
| `resources.rs` | `ResourcesReader` and `DepositsReader`: read `[[resource]]` and `[[deposit]]` entries and build `ResourceTypes` and `Deposits` |
| `test_content.rs` | `#[cfg(test)]` test-only content type and framework tests |

Everything except the items re-exported from the crate root is `pub(crate)`. The engine depends on exactly one external crate for this: `toml` (default features off, `parse` and `std` on, no serde), used for its parser only. `tempfile` is a dev-dependency.

## How loading works

`load_with::<C: SettingContent>(path)` is the generic loader. `load_setting` is `load_with::<WorldContent>` plus building a `LoadedSetting`.

1. **Setting folder.** A path that is not a folder gives one error with no file path.
2. **Discovery** (`discover.rs`). Checks the root, reports structure problems, and returns the manifest path, the sorted list of data files (each tagged with its `Folder`), and `complete`, whether every data file could be seen.
3. **Manifest.** Read with `read_toml` and an `EntryReader` labelled `manifest`. A `Manifest` is produced only if `name` and `format_version` are both valid.
4. **Phase 1.** For each data file in order, `read_toml` parses it. For each top-level key, `read_sections` looks it up among the content's section types:
   - unknown header: `unknown section '<key>'`;
   - known header in the wrong folder: `section '<key>' belongs in rules/, not start/` (contents not read);
   - value not an array of tables: `section '<key>' must be written as [[<key>]]`. The inline form `x = [{...}]` is the same TOML data and is accepted;
   - otherwise each entry gets an `EntryReader`, the section's `read` function runs, then `finish()` reports unread keys.
5. **Phase 2.** `SettingContent::resolve` checks references now that every file has been read, so a reference can point into any file. It is skipped when the loader may not have seen every definition:
   - a file was rejected (could not be read, not UTF-8, or invalid TOML); or
   - discovery was incomplete (`Discovered::complete` is false): `rules/` or `start/` is missing or is not a folder, a subfolder could not be listed, or an entry's name is not valid UTF-8.

   In those cases an "unknown" error would be misleading, because the target may be in a file the loader never saw. Other phase 1 errors (unknown keys, bad names, duplicates, a non-`.toml` file the loader could see) do not skip phase 2.

   `WorldContent::resolve` is where real references are checked: it resolves each resource type's `produces` against the goods, then each deposit's `resource` against the resource types and its coordinates against the map (`MapReader::id`). A reader's `finish` that depends on phase 2 (`ResourcesReader::finish`) is only called after a successful load.
6. **Finish.** `ErrorSink::into_result` sorts the errors and returns `Err(LoadErrors)` if there are any. There is never a partly loaded setting.

`discover` computes `complete` by threading a boolean through `walk` and `list_dir`, which return false whenever a folder cannot be listed or an entry is left out; a missing data folder also clears it. `load_with` runs phase 2 only when no file was rejected and `discovered.complete` is true. The framework tests cover this with `TestContent`, whose "placed thing" refers to a "thing".

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

`WorldContent` is the real content type. It holds a `GoodsReader`, a `MapReader`, a `ResourcesReader`, and a `DepositsReader`, and returns four section types: `good` and `resource` in `rules/`, `hex` and `deposit` in `start/`. `load_setting` builds `Rules { goods, resources }` and `StartState { map, deposits }` from it (the map first, because `DepositsReader::finish` needs the hex count). Because the loader is generic over the content type, tests supply their own (`TestContent` in `test_content.rs`: a "thing" section in `rules/` and a "placed thing" in `start/` that refers to a thing by name) and so exercise the shared rules (duplicates across files, wrong folder, cross-file references) without real content.

### Adding a section type

1. Define the data and store it in `Rules` or `StartState` (add fields; they are plain structs, not `Clone`/`PartialEq` because name registries are not). Follow an existing reader as the pattern: one reader struct per section kind, stored on `WorldContent`, with a `finish` that produces the public type for `Rules` or `StartState`.
   - `[[good]]` (`Rules::goods`) is the worked example for a named kind in `rules/`; `[[resource]]` (`Rules::resources`) follows it.
   - `[[hex]]` (`StartState::map`) is the worked example for a starting-state section type and for a kind identified by something other than a name. Its reader keeps its own table of what makes an entry unique (coordinates) and checks duplicates itself, instead of using `Definitions`. `[[deposit]]` (`StartState::deposits`) follows it.
   - Domain types go in their own module under `crate::` (which knows nothing about files) and the reader in `setting/<name>.rs`.
2. Add a `SectionType` to `WorldContent::section_types`, naming its header, its one folder, and a `read` function.
3. In `read`, use the `EntryReader` accessors for fields, `Definitions<K>` to define names, and keep any references unresolved (as `Located<String>`). Resolve them in `WorldContent::resolve`: a name through `Definitions::resolve`, a hex through `MapReader::id`. Resources and deposits are the worked example.

The `read` function never calls `finish`; the loader does. Items that only tests use today (`Definitions::registry`, `Definitions::location`, `EntryReader::required_bool`) are marked `#[cfg(test)]`; remove the attribute when a section type needs them.

## EntryReader

`EntryReader` reads one table: a `[[section]]` entry or the manifest's root table. It is hand-written rather than serde-based because serde stops at the first error, and the loader must report all of them.

- Accessors: `required_string`, `optional_string`, `required_name` (string plus `valid_name`), `required_integer`, `optional_integer`, `required_number`, `required_bool`, `optional_bool`. Each returns `Option<Located<T>>`, where `Located` carries the value and its `Location`. `required_number` accepts an integer or a float and returns an `f64`; `inf` and `nan` are returned as written (the caller does the range check), an integer beyond `i64` is reported as out of range, and other types are reported as `must be a number, found <type>`.
- Each accessor marks its key as read even when the value is invalid, so a wrong-type key is not also reported as unknown. On a problem it reports an error and returns `None`: missing required key, wrong type, invalid name, or out-of-range integer.
- `errors()` gives access to the `ErrorSink` for section-specific checks. `location()` is the entry's header (for the manifest, the file start). `label()` is the entry label used in messages (for example `good`, `deposit`, `manifest`).
- `finish()` reports every key no accessor asked for, as `unknown key '<key>' in <label>`, at the key.
- `valid_name(name)`: an ASCII lowercase letter, then only ASCII lowercase letters, digits and `_`. This is the name-syntax check that [ids.md](ids.md) leaves to the loader.

Because the toml tables iterate in sorted key order and arrays in source order, reading is deterministic. Final error order does not depend on it anyway.

## Definitions

`Definitions<K: IdKind>` wraps a `NameRegistry<K>` and remembers where each name was defined (`locations[id.index()]`).

- `define(name, sink)` registers the name, returning its `Id<K>`. On a duplicate it reports at the new definition and names the first: `duplicate thing name 'copper' (first defined at rules/a.toml:2:8)`.
- `resolve(reference, sink)` looks a name up. If unknown it reports at the reference: `unknown thing 'tin'`.
- Also `into_registry()`, and (test-only) `registry()` and `location(id)`.

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

- `settings/sample/` at the repository root is the project's sample setting: a manifest, `rules/goods.toml` (five goods, see [goods.md](goods.md)), `rules/resources.toml` (two resource types) and `start/map.toml` (19 hexes, see [map.md](map.md)), `start/deposits.toml` (10 deposits, see [resources.md](resources.md)), plus `README.md` files. It must always load without errors.
- `crates/engine/tests/settings/` holds small setting folders for the engine's tests, each named for what it tests; its `README.md` lists every one. They include `valid_minimal`, `valid_nested`, `missing_manifest`, `invalid_toml`, `many_errors`, `goods_valid`, `goods_one`, `goods_errors`, `hexes_valid`, `hexes_one`, `hexes_errors`, `resources_valid`, `resources_one`, `resources_errors`, `deposits_errors`, `resources_broken_file`, `resources_missing_start`, `turns_valid`, and the `framework_*` folders used with the test-only content type. Each directory contains at least one file so git keeps it. Settings that git cannot keep reliably (hidden files, `Thumbs.db`, invalid UTF-8, a BOM) are built at runtime in a `tempfile::TempDir`.

## Setting checker (dev UI)

The checker is the `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)), implemented in `crates/dev-ui/src/check.rs`. It never opens a window, and it calls only the engine's public API.

| Arguments | Behavior | Exit code |
|---|---|---|
| `check <path>` | loads the setting; on success prints the success line to stdout | 0 |
| `check <path>` with errors | prints each error on its own line to stderr (see `text::error_lines`), then `<n> error(s) found in <path>` | 1 |
| `check <path> --turns N` | after the success line, creates a world, prints turn 0's line, then steps `N` times printing one line per turn (see [world.md](world.md)); `N` is a whole number from 0 to 10,000 (ASCII digits only, leading zeros allowed) | 0 |
| `check <path> --turns N` with errors | as for `check <path>` with errors; no turns are run | 1 |
| `--version` | prints `{ENGINE_NAME} {ENGINE_VERSION}` | 0 |
| anything else | prints the usage message to stderr: `usage: dev-ui [SETTING_FOLDER]`, `       dev-ui check SETTING_FOLDER [--turns N]`, `       dev-ui --version` on three lines | 2 |

The success line is `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>`, for example `5 goods, 19 hexes, 2 resources, 10 deposits` or `1 good, 1 hex, 1 resource, 1 deposit`. The `count(n, singular, plural)` helper in `text.rs` does the wording; later Steps append their own counts. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

## Tests

- Unit tests sit in each `setting` module. Framework tests that use `TestContent` are in `test_content.rs`. Tests that need the errors as text lines use the `#[cfg(test)]` helpers `ErrorSink::into_lines` and `LoadErrors::lines`.
- `crates/engine/tests/setting_loader.rs` and the other files in `crates/engine/tests/` load the test settings through the public API and assert on exact `Display` strings, including line and column (see [workspace.md](workspace.md) for the shared helpers).
- `crates/dev-ui/tests/checker.rs` runs `dev-ui check` against the sample and a broken setting; `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.
