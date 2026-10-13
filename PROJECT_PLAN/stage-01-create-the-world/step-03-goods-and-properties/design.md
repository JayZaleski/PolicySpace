# Step 03: goods-and-properties

## Summary
This Step adds goods to the setting format: the first real kind of content the loader reads. A good is anything an actor can own, and settings define their goods in `rules/`. Each good can have two properties: when it perishes, and whether it is movable. The engine knows what these two properties mean, but has no built-in goods and no categories such as "labor" or "capital"; every good comes from the setting. Movable has no effect yet, and perishing doesn't happen until the turn clock exists in Step 06. This Step also adds goods to the sample setting, reports a goods count from the setting checker, and fixes a gap left by Step 02 in when references between things are checked.

## Behavior

### Defining goods
- Goods are defined in `[[good]]` sections. They belong in `rules/`, because what goods exist never changes during play. A `[[good]]` in `start/` is an error saying it belongs in `rules/`.
- Any file in `rules/`, at any depth, may hold any number of goods. Goods from every file are merged into one list.
- Each good has a required `name`, which must be lowercase `snake_case` (the Step 02 name check).
- The same good name defined twice is an error, even across files, and the error names both places.
- Goods get their IDs in load order: the order files are read, then the order of goods within each file. The same folder always gives the same IDs.
- A setting with no goods is valid.

### Properties
The engine understands a fixed set of property keys, and each good sets its own values. Both are optional.

- **`perishes_after`**: how long a unit of the good lasts, as a whole number of 1 or more.
  - A unit survives N turn-ends, counting the turn it appears in. With `perishes_after = 1`, a unit that appears in turn 5 can be used during turn 5 and is gone when turn 5 ends. This is what labor and unimproved resource output use. With `perishes_after = 3`, a unit that appears in turn 5 is gone at the end of turn 7.
  - Leaving the key out means the good never perishes (a durable good). There is no special value meaning "never".
  - `0`, negative numbers, and values that aren't integers are errors at the value.
  - This Step fixes only the meaning. Step 06 makes perishing happen and decides how it tracks how old each unit is.
- **`movable`**: `true` or `false`; `true` if left out. It has no effect until movement and trade between places exist, after Stage 04. It is defined now so settings can state it from the start.

Any other key in a `[[good]]` is an unknown-key error, so a typo such as `perishes = 1` is reported rather than silently ignored. A wrong type (for example `movable = "yes"`) is an error at the value. As with every section, all of a good's problems are reported in one load.

Example:
```toml
[[good]]
name = "unskilled_labor"
perishes_after = 1
movable = false

[[good]]
name = "copper"          # durable and movable: only a name is needed
```

### What a loaded setting provides
- The loaded setting's rules hold the goods. Through the engine's public interface, a caller (the dev UI, and the world state in Step 06) can:
  - list every good in load order, which is also ID order;
  - look up a good's ID by name, and get a good's name from its ID;
  - read a good's two properties, with the defaults already applied. A caller sees "never perishes" or "movable", not "key missing".
- The engine doesn't record whether a property value was written out or came from its default.

### Reference checking after discovery problems (carried over from Step 02)
- Step 02 skips the reference check when a file is broken (unreadable, not UTF-8, or not valid TOML), because a reference may point into the broken file.
- This Step also skips it when discovery couldn't see some data files: when `rules/` or `start/` is missing, a subfolder can't be read, or a file name isn't valid UTF-8. In each case, the things a reference points to may be in files the loader never saw, so reference errors would be misleading.
- Other problems, such as unknown keys, bad names, duplicates, or a non-`.toml` file the loader could see but rejected, still don't skip the reference check.
- Goods don't refer to anything, so this is tested with the test-only section types from Step 02 (a "placed thing" referring to a "thing").

### The sample setting
- `settings/sample/rules/goods.toml` defines five goods, covering each combination of the properties and giving Step 05 resource outputs to point at:

| Good | `perishes_after` | `movable` | Role |
|---|---|---|---|
| `unskilled_labor` | 1 | false | labor (used from Stage 02) |
| `unimproved_copper` | 1 | false | output of copper deposits (Step 05) |
| `unimproved_grain` | 1 | false | output of arable land (Step 05) |
| `copper` | never | true | an intermediate good |
| `grain` | 3 | true | a perishable, movable good |

- The sample setting still loads without errors.

### The setting checker
- On success, the dev UI's setting checker adds a goods count to its line: `Loaded setting 'Sample Setting' (format version 1): 5 goods`. With exactly one good it says `1 good`; with none, `0 goods`.
- Later Steps add their own counts (hexes, deposits) to this line.
- Running with no argument, and the failure output, are unchanged.

### Test settings
- Small test settings cover goods: a valid set across several files (including nested folders), and the error cases below.

## Player-facing Changes
None for players. Setting authors can define goods and their properties, and the sample setting shows how. The setting checker reports how many goods a setting defines.

## Interactions
- **Step 01 (engine-workspace):** goods are the first concrete ID kind.
- **Step 02 (setting-loader):** `[[good]]` is the first real section type in the loader framework. This Step also extends Step 02's rule for when reference checking is skipped, and changes the setting checker's success line.
- **Step 05 (natural-resources):** resource types name the unimproved good they produce, by good name.
- **Step 06 (turn-clock):** uses `perishes_after` to make goods perish, and decides how age is tracked.
- **Steps 07–08 (dev UI):** list goods and show their properties.
- **Stage 02 onward:** POPs, labor, utility functions, and production functions name goods directly.

## Out of Scope
- Any effect of `movable`.
- Goods actually perishing, and tracking how old each unit is (Step 06).
- Resource types and the goods they produce (Step 05), including any check that resource output must be fixed in place and perish after one turn.
- Display names, descriptions, or units for goods.
- Divisibility (whole versus continuous quantities).
- Properties declared in data, or properties the engine doesn't act on.

## Acceptance Criteria
- The sample setting loads with the five goods in the table, in that order, with the listed properties.
- A valid test setting with goods across several files, including nested folders, loads. The goods get IDs in load order, and loading it twice gives the same IDs.
- For each good, the loaded setting gives its name, its ID by name, and its properties. A good with no property keys never perishes and is movable.
- Each of these is an error at the right file, line, and column:
  - a `[[good]]` with no `name`, or a name that isn't lowercase `snake_case`;
  - the same good name in two files (the error names both places);
  - a `[[good]]` in `start/` (the error says it belongs in `rules/`);
  - `perishes_after` of `0`, a negative number, or a non-integer;
  - `movable` that isn't `true` or `false`;
  - an unknown key in a `[[good]]`.
- One good with several problems reports all of them in one load.
- A setting with no goods loads.
- With the test-only section types, a placed thing whose thing can't be found does **not** get an "unknown thing" error when `rules/` or `start/` is missing, a subfolder can't be read, or a file name isn't valid UTF-8; only the discovery error is reported. It still gets the error when the only other problem is, for example, an unknown key or a non-`.toml` file.
- The dev UI with the sample setting's path prints `Loaded setting 'Sample Setting' (format version 1): 5 goods` and exits successfully. A setting with one good says `1 good`; one with none says `0 goods`.
- The dev UI with no argument, and with a broken setting, behaves as before.
- The single check command passes.
