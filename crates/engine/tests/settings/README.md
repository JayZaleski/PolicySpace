# Test settings

Small setting folders used by the engine's setting-loader tests
(`crates/engine/tests/setting_loader.rs` and the framework tests in
`crates/engine/src/setting/`). Each is minimal and named for what it tests. Every
folder holds at least one file (often a `README.md`) so that git keeps it.

Valid settings:
- `valid_minimal`: the smallest valid setting.
- `valid_nested`: `.toml` files in nested subfolders, and `README.md` files everywhere.
- `goods_valid`: goods across several files and a nested folder, with and without property
  keys. Load order: `grain`, `copper`, `labor`, `tools`.
- `goods_one`: exactly one good (for the setting checker's `1 good`).

Folder-structure errors:
- `missing_manifest`, `missing_rules`, `missing_start`: a required entry is absent.
- `unexpected_root_entry`: a stray file and a stray folder in the setting root.
- `non_toml_file`: non-`.toml` files in `rules/` and `start/`.

File-content errors:
- `manifest_missing_fields`, `manifest_bad_values`: problems in `setting.toml`.
- `invalid_toml`: a file with TOML syntax errors.
- `unknown_section`: section headers the loader doesn't know (`widget`, `gadget`; made-up
  names, so they stay unknown as real section types are added).
- `goods_errors`: every kind of `[[good]]` error, including one good with several problems,
  a duplicate across files, and a `[[good]]` in `start/`.
- `many_errors`: several kinds of problem at once, to check that all are reported in order.

Framework settings. These use the test-only section types (a "thing" in `rules/` and a
"placed thing" in `start/`, defined in `crates/engine/src/setting/test_content.rs`), so
the real loader reports their sections as unknown:
- `framework_valid`: things across files, and placed things that refer to them.
- `framework_duplicate`: the same thing defined in two files.
- `framework_wrong_folder`: each section type in the wrong folder.
- `framework_bad_reference`: a placed thing referring to a thing that doesn't exist.
- `framework_field_errors`: several problems in one section, and a section written as a table.

Settings that git can't keep reliably (hidden files, invalid UTF-8, a byte order mark)
are built at runtime in the tests instead.
