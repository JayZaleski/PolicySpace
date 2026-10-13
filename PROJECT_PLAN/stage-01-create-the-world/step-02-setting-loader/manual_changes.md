# Step 02: setting-loader — Manual Changes

No change for players. Setting authors and developers gain a defined setting format and a checking command, so this belongs in a section for setting authors (new, titled something like "Writing a setting"), or in the "for developers" section if the manual has one by then.

## What the manual should cover
- What a setting is: a folder of TOML files, split into `rules/` (things that never change during play) and `start/` (what turn 0 looks like), plus a small `setting.toml` manifest. Explain the manifest's keys (`name`, optional `description`, `format_version` = 1) and what "unknown keys are errors" means for authors.
- Folder rules in plain terms: files can be organized freely, nested at any depth; they are read in sorted path order; what a file defines comes from its section headers, not its name; `README.md` is the place for notes; hidden files, `Thumbs.db` and `desktop.ini` are ignored; any other non-`.toml` file is an error.
- Naming rules: lowercase letters, digits and underscores, starting with a letter (`copper_ore`, `grain2`).
- How strictness helps: the loader reports every problem in one go, with file, line and column, and never produces a half-loaded setting. Show two or three sample error lines.
- How to check a setting: run the dev UI with the folder's path (`cargo run -p policyspace-dev-ui -- settings/sample`), and what the output and exit codes mean. The no-argument form still prints the engine's name and version.
- Note that no section types (goods, hexes, deposits) exist yet, so the sample setting is only a manifest; later Steps add them, and this section should be extended then. Pointing to `settings/sample/` as the setting to copy.

## Where it fits
A new section after any project-structure or developer material from Step 01. If Step 01's manual note on `cargo run -p policyspace-dev-ui` was added, update it: the program now also checks a setting folder when given a path.

## Links
- `docs/setting-loader.md` for the format, error rules, and how to add section types.
- `docs/ids.md` for how names become IDs.

## Example
A short example of a minimal valid setting (the sample's `setting.toml`) and a short example error listing from a broken setting would help. An interactive demo is not needed.
