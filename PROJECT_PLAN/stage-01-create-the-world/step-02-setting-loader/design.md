# Step 02: setting-loader

## Summary
This Step defines what a setting folder looks like and builds the engine's loader for it. A setting is a folder of TOML files, split into persistent rules and starting state, with a small manifest at the top. The loader reads every file and merges them by kind. It is strict: typos, stray files, and bad names are errors, and every error is reported at once with its file and position. This Step builds the framework only. It defines no kinds of content: Steps 03–05 add goods, hexes, and deposits to it. It also starts the sample setting that later Steps extend, and turns the dev UI placeholder into a simple setting checker, so a setting can be checked before the real dev UI arrives in Step 07.

## Behavior

### File format
- All setting data files are TOML: easy to read and edit by hand, comments allowed, and standard in Rust.

### Folder layout
```
<setting folder>/
  setting.toml     # the manifest (required)
  README.md        # optional notes
  rules/           # persistent rules (required, may be empty)
  start/           # starting state, what turn 0 looks like (required, may be empty)
```
- **Rules** are constants that never change during play, mostly definitions of kinds of things (what a "copper deposit" is). **Starting state** is the turn-0 value of everything that can change, mostly the actual things (this deposit, on this hex). Starting-state files refer to rules by name. Starting state is in effect a special case of a save: a later save is the same rules plus the state at a later turn.
- The setting root may contain only `setting.toml`, `README.md`, `rules/`, and `start/` (plus ignored files, below). Anything else is an error.
- A missing `setting.toml`, `rules/`, or `start/` is an error.

### The manifest
`setting.toml` holds:
- `name`: the setting's display name. Required.
- `description`: a short description. Optional.
- `format_version`: which version of the setting file format the setting uses. Required. Only `1` is supported. Any other value is a clear error; the engine never guesses. The format version changes only when the format changes in a way that breaks existing settings, not when optional keys are added.

Unknown keys in the manifest are errors.

### Reading `rules/` and `start/`
- Every `.toml` file in `rules/` and `start/` is read, including files in subfolders at any depth. Authors can organize files however they like, for example by theme (`rules/metals.toml`, `rules/food.toml`) or in nested folders (`rules/goods/metals.toml`).
- Files are read in sorted order of their full path within the setting, so the same folder always loads the same way, with the same IDs.
- What a file defines is given by its section headers (for example, `[[good]]` in a later Step), not by the file's name or folder. Any file may hold any number of sections of any type that belongs in its folder.
- Content from all files is merged by kind: every good from every file in `rules/` goes into one list of goods, and so on.
- Rules sections are allowed only in `rules/`, and starting-state sections only in `start/`. A section in the wrong folder is an error.
- The same name defined twice for one kind is an error, even across files. The error names both places.
- References from one thing to another (for example, a resource type naming the good it produces) are checked only after every file has been read, so a reference can point into any file.

### What counts as an error
- An unknown section header, or an unknown key inside a section.
- A name that isn't lowercase `snake_case`: it must start with a lowercase letter and contain only lowercase letters, digits, and underscores (`copper_ore`, `grain2`). This is the name check that Step 01 left to the loader.
- A file that isn't valid UTF-8 or isn't valid TOML.
- A file in the setting that isn't a `.toml` file, except for the allowed and ignored files below.

### Allowed and ignored files
- `README.md` is allowed in any folder of the setting: the root, `rules/`, `start/`, and any subfolder. The documentation tells setting authors to put their notes there. The loader doesn't read it.
- Ignored entirely: hidden files and folders (names starting with `.`, such as `.DS_Store` or `.gitkeep`), and the operating-system files `Thumbs.db` and `desktop.ini`. These are created by operating systems and tools, and shouldn't make a load fail.

### Error reporting
- The loader collects every error it can find and reports them all together, rather than stopping at the first.
- Each error gives the file's path relative to the setting folder, the line and column when there is one, and a clear message. For example: `rules/metals.toml:14:3: unknown key 'perishs' in good`.
- Errors are listed in a fixed order: by file, in the same order the files are read, then by position within the file.
- A file that isn't valid UTF-8 or isn't valid TOML reports only those problems; its contents aren't checked further, since a broken file would otherwise produce a cascade of misleading errors. For the same reason, if any file is broken this way, references between things aren't checked in that load, because the things a reference points to may be in the broken file.
- If there is any error, the load fails and no setting is produced. There is never a partly loaded setting.

### What a successful load produces
- A **loaded setting**: the manifest, the persistent rules, and the starting state. In this Step the rules and starting state are empty, because no kinds of content exist yet.
- A loaded setting is not a running world. Step 06 builds the world state from it.

### A framework for later Steps
- This Step defines no section types. Until Step 03 adds the first one, any section header in a setting is reported as unknown.
- Each later Step (goods in Step 03, hexes in Step 04, resource types and deposits in Step 05) adds its own section types to the loader. It decides whether each belongs in `rules/` or `start/`, and adds its own checks. File discovery, merging, duplicate detection, name checking, and error reporting are shared and don't need to be rebuilt.
- Names in setting files become IDs using Step 01's ID mechanism.

### The sample setting
- `settings/sample/` at the repository root is the project's sample setting. In this Step it has a manifest and empty `rules/` and `start/` folders, each with a `README.md` so git keeps the folders. Later Steps add goods, a map, and deposits to it.
- It loads without errors.

### Test settings
- `crates/engine/tests/settings/` gets small setting folders for the loader's tests: one valid setting, and one minimal setting for each error case. Each is named for what it tests.
- Real settings have no section types until Step 03, so the framework's shared rules (duplicate names across files, sections in the wrong folder, and references between files) are tested with a made-up pair of section types that exists only in tests: a "thing" defined in `rules/`, and a "placed thing" in `start/` that refers to a thing by name. The loader takes its list of section types as an input, so tests can supply this pair while real loading uses the real list.

### The setting checker
- The dev UI program stays a placeholder, but gains one use:
  - **Run with no argument**: it behaves as before, printing the engine's name and version.
  - **Run with a path to a setting folder**: it loads the setting. On success, it prints the setting's name and format version and exits successfully. On failure, it prints every error and exits with a failure code.
- Step 07 replaces the dev UI with a real developer UI. Checking a setting from the command line may or may not survive that; Step 07 decides.

## Player-facing Changes
None for players. Setting authors and developers get:
- a defined setting folder format, with a sample setting to copy;
- clear, complete error lists when a setting has problems;
- a command that checks a setting folder: the dev UI program, run with the folder's path.

## Interactions
- **Step 01 (engine-workspace):** text names in setting files become IDs through the ID mechanism. This Step adds the name-syntax check that Step 01 left to the loader. It also replaces the dev UI placeholder's behavior, keeping the no-argument behavior, and adds the first test settings to the test-settings folder.
- **Steps 03–05 (goods-and-properties, hex-map, natural-resources):** each adds its own section types to the loader and its content to the sample setting.
- **Step 06 (turn-clock):** builds world state from a loaded setting.
- **Step 07 (dev-ui-map):** loads a setting folder through the loader, and replaces the setting checker.

## Out of Scope
- Any kinds of content: goods, hexes, resource types, and deposits belong to Steps 03–05.
- World state and turns.
- Loading from anywhere other than a folder on disk, such as archives or downloads.
- Mods layered on top of a setting.
- File formats other than TOML.
- "Did you mean…?" suggestions in error messages. A later Step can add them when references between things exist.
- Extra manifest fields such as authors or a setting version.

## Acceptance Criteria
- The sample setting at `settings/sample/` loads without errors.
- A valid test setting loads, and the loaded setting has the manifest's name, description, and format version.
- A setting with `.toml` files in nested subfolders of `rules/` and `start/` loads, and the files are read in sorted full-path order.
- Each of these produces an error naming the file, with the line and column where they apply:
  - missing `setting.toml`, `rules/`, or `start/`;
  - an unexpected file or folder at the setting root;
  - a missing `name` or `format_version`, or an unsupported `format_version`;
  - an unknown key in the manifest;
  - a file that isn't valid TOML;
  - a file that isn't valid UTF-8;
  - an unknown section header in `rules/` or `start/`;
  - a non-`.toml` file in `rules/` or `start/`.
- Using the test-only section types:
  - a thing defined in one file and a placed thing in another file that refers to it load correctly, and things get IDs in the order their files are read;
  - the same thing name defined in two files is an error that names both places;
  - a thing section in `start/` is an error saying it belongs in `rules/`;
  - a placed thing referring to a thing that doesn't exist is an error at the reference;
  - one section with several problems (an unknown key, a bad name, a value of the wrong type) reports all of them.
- The name-syntax check accepts `copper_ore` and `grain2`, and rejects names such as `Copper`, `copper-ore`, `2grain`, `_copper`, and the empty name.
- `README.md` files in the root, `rules/`, `start/`, and subfolders don't cause errors.
- Hidden files and folders, `Thumbs.db`, and `desktop.ini` are ignored.
- A setting with several problems reports all of them in one load, in file order and then position order, and produces no loaded setting.
- Loading the same setting twice gives the same result.
- The dev UI with no argument prints the engine's name and version and exits successfully.
- The dev UI with the sample setting's path prints its name and format version and exits successfully.
- The dev UI with a broken setting's path prints all of its errors and exits with a failure code.
- The single check command passes.
