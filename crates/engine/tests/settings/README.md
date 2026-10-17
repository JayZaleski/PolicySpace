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
  Goods: `coin` and `shell`, designated as the currencies of those hexes in that order
  (`coin`, `shell`, `coin`, `shell`).
- `hexes_one`: exactly one hex (for the setting checker's `1 hex`), with one good, `coin`,
  as its currency.
- `resources_valid`: resource types and deposits across several files and nested folders,
  with the goods and hexes they refer to defined in files read after them. Resource
  types: `copper_ore`, `iron_ore`, `farmland`, `unused`. Deposits: 2 `copper_ore` and 1
  `farmland` at (0, 0), then 3 `iron_ore` at (1, 0).
- `resources_one`: one good, one hex, one resource type, one deposit and one market with
  empty prices (for the setting checker's `1 resource`, `1 deposit` and `1 market`).
- `turns_valid`: for the turn clock tests (`crates/engine/tests/world.rs`). Goods: `daily`
  (perishes after 1), `stew` (perishes after 3), `gold` (never perishes). Resource types:
  `field` (2 `daily`), `meadow` (0.5 `daily`), `pot` (5 `stew`), `mine` (1.5 `gold`).
  Hexes: (0, 0), (1, 0), (2, 0). Deposits: 2 `field`, 1 `meadow` and 1 `pot` at (0, 0),
  then 1 `field` and 1 `mine` at (1, 0); (2, 0) has none.
  Markets (in `start/markets.toml`): (0, 0) prices `daily` 2 and `stew` 4; (1, 0) prices
  `daily` 0.5 and `stew` 3; (2, 0) has no market. `gold` is every hex's currency.
- `markets_valid`: markets across several files and a nested folder, in both the inline
  `prices = { ... }` form and the `[market.prices]` sub-table form, with one market that has
  empty prices and one hex with no market. Goods: `coin`, `shell`, `grain`, `fish`. Hexes:
  (0, 0) `coin`, (1, 0) `shell`, (2, 0) `coin`, (3, 0) `coin`. Prices: (0, 0) `grain` 2 and
  `fish` 1.5; (1, 0) `grain` 3 and `coin` 0.25 (`coin` is not that hex's currency, so it may
  be priced); (2, 0) none; (3, 0) has no market.
- `pops_valid`: POP types across several files and a nested folder, in the inline form and
  the sub-table form (`[pop_type.endowment]`, `[pop_type.utility.<good>]`), with the goods
  and a hex (whose currency is `coin`) in files read after them. Goods: `coin`, `labor`,
  `grain`, `fish`. POP types: `farmer` (endows `labor`), `fisher` (endows `fish` and
  `labor` written out of ID order; a pure-necessity `fish`, and the currency `coin` in its
  utility), `idle` (empty endowment). Every type has real growth parameters, in both forms:
  `farmer` `[0.06, -0.03]`, decline 0.1, max_growth 0.05 (inline); `fisher` `[0.03]`,
  decline 0, max_growth 1 (the `[pop_type.growth]` table); `idle` `[0.02, 0.01, -0.005]`,
  decline 0.5, max_growth 0.2 (inline).
- `technologies_valid`: technologies across several files and a nested folder
  (`rules/a.toml`, `rules/a/z.toml`, `rules/b.toml`), in the inline form and the sub-table
  form (`[technology.inputs]`, `[technology.capital]`, `[technology.construction]`, some
  empty), with the goods in `rules/goods.toml`, read after every technology file. Goods
  (IDs 0-4): `labor`, `ore`, `iron`, `tools`, `bread`. Technologies: `smelter` (inputs 2
  `ore` and 1 `labor` written out of ID order, output `iron`, capacity 10, capital 5 `tools`,
  construction 20 `labor`, 2 turns, lifetime 10), `well` (no inputs, output `bread`, capacity
  0.5, capital 1.5 `tools` and 3 `labor` written out of ID order, no construction, 1, 1),
  `toolmaker` (input 1 `iron`, output `tools`, capacity 3, capital 2 `tools` and 4 `iron`,
  empty inline construction, 3 turns, lifetime 3).
- `technologies_one`: one good, `bread`, and one technology, `oven` (for the setting
  checker's `1 technology`).
- `businesses_valid`: businesses across several files and a nested folder (`start/a.toml`,
  `start/b/c.toml`, then `start/z.toml` with the hexes, deposits and market they refer to),
  in the inline form and the sub-table form (`[business.inventory]`, `[business.deposits]`),
  for the businesses tests (`crates/engine/tests/businesses.rs`). Goods (IDs 0-3): `coin`,
  `labor` (perishes after 1, unmovable), `grain` (perishes after 3), `ore`. Resource types:
  `field` (2 `grain`), `vein` (1.5 `ore`). Hexes (0, 0) and (1, 0), both with currency
  `coin`; deposits 0-2 `field` and 3 `vein` at (0, 0), 4 `field` at (1, 0); a market at
  (0, 0) only. Businesses: 0 `farm_a` at (0, 0) (inventory 4 `labor` and 10 `coin` written
  out of ID order; owns 2 `field`), 1 `farm_b` at (0, 0) (5 `coin`; owns 1 `vein` and 1
  `field`, written out of resource ID order), 2 `idle` at (1, 0) (empty inventory, no
  deposits, a hex with no market). Deposit owners: `[0, 0, 1, 1, none]`.
- `businesses_one`: one good, one hex and one business (for the setting checker's
  `1 business`).
- `pops_one`: one good, one POP type, one hex and one POP (for the setting checker's
  `1 pop type`, `1 hex` and `1 pop`). Its type has zero growth parameters.
- `pops_placed`: POPs across several files and a nested folder (`start/a.toml`, then
  `start/b/c.toml`), with a deposit, for the turn behavior. Goods: `labor` (perishes after 1
  turn, unmovable), `grain` (perishes after 1 turn), `gold` (never perishes). Hexes (0, 0),
  (1, 0), (2, 0), each with currency `gold`; a `field` (output 2 `grain`) at (2, 0). POP
  types: `worker` (endows 10 `labor`), `hoarder` (endows 4 `labor` and 1.5 `gold`), `idle`
  (empty endowment). POPs: 0 `worker` at (0, 0) size 3; 1 `hoarder` at (0, 0) size 2; 2
  `worker` at (1, 0) size 0.5 (fractional); 3 `idle` at (1, 0) size 7; (2, 0) has none. The
  hoarder's utility is `grain`, so its `gold` is parked and accumulates and its `labor` is
  not consumed; the workers consume their `labor` (their utility). No hex has a market.
  Every type has zero growth parameters (`growth = [0]`, `decline = 0`, `max_growth = 0`),
  so no size changes and the population is 12.5 every turn.
- `pops_trade`: for the POP trade round (`crates/engine/tests/trade.rs` and the unit tests
  in `crates/engine/src/trade.rs`). Goods (IDs 0-4): `labor` (perishes after 1), `grain`
  (perishes after 3), `copper`, `silver`, `gem` (durable). Hexes (0, 0) to (4, 0), every one
  with currency `silver`. Markets: (0, 0) `labor` 1, `grain` 2, `copper` 5; (1, 0) `labor` 1,
  `grain` 10; (2, 0) none; (3, 0) `labor` 1, `grain` 2; (4, 0) `labor` 1, `grain` 2,
  `copper` 5. POP types: `peasant` (labor 10; labor and grain), `artisan` (labor 10; labor,
  grain, copper), `saver` (labor 10 and silver 5; as peasant), `collector` (labor 10 and gem
  1; as peasant), `miser` (labor 10; labor with share 0, copper), `idle` (empty endowment;
  grain). POPs: 0 `peasant` at (0, 0) size 2; 1 `artisan` at (0, 0) size 1; 2 `peasant` at
  (1, 0) size 1; 3 `idle` at (1, 0) size 3; 4 `peasant` at (2, 0) size 1; 5 `saver`, 6
  `miser`, 7 `collector` at (3, 0) size 1; (4, 0) has a market and no POPs. Every type has
  zero growth parameters, so no size changes and the population is 11 every turn.
- `pops_growth`: for POP growth (`crates/engine/tests/growth.rs`). Goods (IDs 0-3): `labor`
  (perishes after 1), `grain` (perishes after 3), `copper`, `silver` (durable). Hexes (0, 0),
  (1, 0), (2, 0), every one with currency `silver`. Markets: (0, 0) `labor` 1, `grain` 2;
  (1, 0) `labor` 1, `grain` 4; (2, 0) `labor` 1, `grain` 10; no hex prices `copper`. With
  G = `{ growth = [0.06, -0.03], decline = 0.1, max_growth = 0.05 }`, POP types (every one
  endows 10 `labor`): `peasant` (labor subsistence 2 share 0.2, grain subsistence 2 share
  0.8; G), `boomer` (labor subsistence 1; `[1]`, decline 0.1, max_growth 0.05), `rich`
  (labor subsistence 2.5; G), `richer` (labor subsistence 1; G), `carefree` (labor share 1,
  no subsistence; `[0.06]`, 0.1, 0.05), `wanting` (labor share 1, copper subsistence 1;
  `[0.06]`, 0.1, 0.05), `static` (as `peasant`; `[0]`, 0, 0). POPs (IDs 0-9): 0 `peasant` at
  (0, 0) size 20; 1 `boomer`, 2 `rich`, 3 `richer`, 4 `carefree`, 5 `wanting`, 6 `static`,
  all at (0, 0) size 10; 7 `peasant` at (1, 0) size 10; 8 `peasant` at (2, 0) size 11; 9
  `static` at (2, 0) size 10. At turn 0 the (index, rate, size after growth) are: 0 (2/3,
  0.02666, 20.5333); 1 (9, capped 0.05, 10.5); 2 (3, -0.09, 9.1); 3 (9, floored -0.1, 9); 4
  (0, 0, 10: subsistence costs nothing); 5 (-1, -0.1, 9: copper can't be obtained); 6 (2/3, 0,
  10); 7 (0, 0, 10: income exactly covers subsistence); 8 (-6/11, -0.6/11, 10.4: below
  subsistence); 9 (-6/11, 0, 10). The population is 108.5333 at turn 0.

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
  typed or out-of-range coordinate, a missing, wrongly typed or unknown currency, an unknown
  key, one hex with several problems, a duplicate across files, and a `[[hex]]` in `rules/`.
  Its one good is `coin`. A hex with invalid coordinates is not placed, so its unknown
  currency is not reported.
- `resources_errors`: every kind of `[[resource]]` error, including a missing, invalid or
  wrongly typed key, an output that is not a finite number above 0, an unknown good, one
  resource with several problems, a duplicate across files, and a `[[resource]]` in
  `start/`.
- `deposits_errors`: every kind of `[[deposit]]` error, including a missing, wrongly typed
  or out-of-range key, bad counts (0, negative, over 1,000,000, a float), an unknown
  resource, a coordinate with no hex, one deposit with several problems, a duplicate across
  files, and a `[[deposit]]` in `rules/`.
- `markets_errors`: every kind of `[[market]]` error, including a missing, wrongly typed or
  out-of-range coordinate, a missing or non-table `prices`, prices that are not numbers or
  not finite and above 0, an unknown good, a priced currency, a coordinate with no hex, one
  market with several problems, an unknown key, a duplicate across files, and a `[[market]]`
  in `rules/`. Its goods are `coin` (every hex's currency) and `grain`.
- `pops_errors`: every kind of `[[pop_type]]` error, including a missing `name`,
  `endowment` or `utility`, a bad or duplicate name, a non-table `endowment` or `utility`,
  endowment amounts that are a string, zero, negative or infinite, a non-table utility entry,
  a negative or non-finite `subsistence` or `share`, a utility good with nothing positive, a
  utility with no positive share, unknown goods, unknown keys in the entry and in a
  sub-table, one entry with several problems, a duplicate across files, and a `[[pop_type]]`
  in `start/`. Every entry in `rules/a.toml`, `rules/b.toml` and `start/p.toml` has a valid
  `growth` line as its last line, and `rules/c.toml` holds every `growth` error: a missing
  or non-table `growth`, missing keys inside it (inline and in the `[pop_type.growth]` form),
  a `growth` list that is a number, empty, or holds a string or `inf`, a `decline` that is
  negative, 1, infinite or a string, a `max_growth` that is negative or infinite, an unknown
  key in the sub-table, and one entry (a duplicate of `ok`) with four growth problems. Its
  goods are `coin` and `grain`.
- `pops_placed_errors`: every kind of `[[pop]]` error, including a missing `type`, `q`, `r`
  or `size`, a wrongly typed `type`, `q` or `size`, an out-of-range `q`, a coordinate with
  no hex, a size that is zero, negative or infinite, an unknown POP type, an unknown key, one
  entry with four problems, a duplicate across files, and a `[[pop]]` in `rules/`. Its goods
  are `coin` and `labor`.
- `technologies_errors`: every kind of `[[technology]]` error, including a missing `name`,
  `inputs`, `output`, `capacity`, `capital`, `construction`, `construction_turns` or
  `lifetime`, a bad or duplicate name, wrongly typed values, amounts that are a string,
  zero, negative, `inf` or `nan`, turns and lifetimes below 1 or beyond `u32` (and a
  non-integer), an empty `capital`, the output good among the inputs, unknown goods, unknown
  keys, one entry (`Several`) with many problems, a duplicate across files, and a
  `[[technology]]` in `start/`. Its goods are `coin`, `grain` and `labor`.
- `businesses_errors`: every kind of `[[business]]` error, including a missing `name`, `q`,
  `r`, `inventory`, `deposits` or `decider`, a bad or duplicate name, wrongly typed and
  out-of-range values, inventory amounts that are a string, zero, negative, `inf` or `nan`,
  deposit counts below 1, beyond `u32` and non-integer, a bad decider name, unknown goods and
  resource types, a coordinate with no hex, more deposits claimed than the hex has unowned
  (in the same file and in a later one), unknown keys, one entry (`Several`) with many
  problems, a duplicate across files, and a `[[business]]` in `rules/`. Its goods are `coin`
  and `grain`, with one `field` resource type and two `field` deposits at (0, 0).
- `resources_broken_file`: a broken TOML file next to resources that refer to a good and a
  hex that don't exist, plus a hex whose currency is an unknown good, a market that prices
  goods that don't exist, a POP type whose endowment and utility name goods that don't
  exist, a technology whose goods don't exist, a POP of an unknown type at a coordinate
  with no hex, and a business whose good and resource type don't exist on a hex that
  doesn't exist: only the syntax error is reported.
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
