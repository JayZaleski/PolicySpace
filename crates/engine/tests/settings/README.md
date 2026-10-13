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
- `hexes_valid`: hexes across several files and a nested folder, with negative coordinates
  and the extremes of `i32`. Load order: (0, 0), (1, -1), (-3, 2), (i32::MAX, i32::MIN).
- `hexes_one`: exactly one hex (for the setting checker's `1 hex`).
- `resources_valid`: resource types and deposits across several files and nested folders,
  with the goods and hexes they refer to defined in files read after them. Resource
  types: `copper_ore`, `iron_ore`, `farmland`, `unused`. Deposits: 2 `copper_ore` and 1
  `farmland` at (0, 0), then 3 `iron_ore` at (1, 0).
- `resources_one`: one good, one hex, one resource type and one deposit (for the setting
  checker's `1 resource` and `1 deposit`).
- `turns_valid`: for the turn clock tests (`crates/engine/tests/world.rs`). Goods: `daily`
  (perishes after 1), `stew` (perishes after 3), `gold` (never perishes). Resource types:
  `field` (2 `daily`), `meadow` (0.5 `daily`), `pot` (5 `stew`), `mine` (1.5 `gold`).
  Hexes: (0, 0), (1, 0), (2, 0). Deposits: 2 `field`, 1 `meadow` and 1 `pot` at (0, 0),
  then 1 `field` and 1 `mine` at (1, 0); (2, 0) has none.

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
- `hexes_errors`: every kind of `[[hex]]` error, including hexes with a missing, wrongly
  typed or out-of-range coordinate, an unknown key, one hex with several problems, a
  duplicate across files, and a `[[hex]]` in `rules/`.
- `resources_errors`: every kind of `[[resource]]` error, including a missing, invalid or
  wrongly typed key, an output that is not a finite number above 0, an unknown good, one
  resource with several problems, a duplicate across files, and a `[[resource]]` in
  `start/`.
- `deposits_errors`: every kind of `[[deposit]]` error, including a missing, wrongly typed
  or out-of-range key, bad counts (0, negative, over 1,000,000, a float), an unknown
  resource, a coordinate with no hex, one deposit with several problems, a duplicate across
  files, and a `[[deposit]]` in `rules/`.
- `resources_broken_file`: a broken TOML file next to resources that refer to a good and a
  hex that don't exist: only the syntax error is reported.
- `resources_missing_start`: no `start/` folder, and a resource that refers to a good that
  doesn't exist: only the missing folder is reported.
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
