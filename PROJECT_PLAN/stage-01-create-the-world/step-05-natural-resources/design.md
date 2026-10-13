# Step 05: natural-resources

## Summary
This Step adds natural resources to the setting format, in two parts. **Resource types** go in `rules/`: each has a name, the one good it produces, and how much of it one deposit outputs per period (for example, one `copper_ore` deposit outputs 2.5 `unimproved_copper`). **Deposits** go in `start/`: the setting places a whole number of deposits of a resource type on a hex, so one hex can be richer in a resource than another. Each deposit is a separate unit that Stage 02 will give a single owner. There is no ownership yet, and nothing is produced until the turn clock exists in Step 06. This is the first Step whose data refers to other data (a resource type names a good, and a deposit names a resource type and a hex), so it is the first real use of the loader's reference check. It also adds resources and deposits to the sample setting, and adds two counts to the setting checker.

## Behavior

### Resource types
- Resource types are defined in `[[resource]]` sections. They belong in `rules/`, because what kinds of resource exist, and what one deposit of each produces, never change during play. A `[[resource]]` in `start/` is an error saying it belongs in `rules/`.
- Any file in `rules/`, at any depth, may hold any number of resource types. Resource types from every file are merged into one list.
- Each resource type has exactly three keys, all required:
  - `name`: lowercase `snake_case`, as for goods. The same resource type name defined twice is an error, even across files, and the error names both places. Resource type names and good names are separate, so a resource type may share a name with a good.
  - `produces`: the name of the good it produces. It must name a good the setting defines.
  - `output`: how much of that good **one deposit** produces per period. Output is a continuous quantity: any number greater than 0 and finite. Whole numbers (`4`) and decimals (`2.5`) are both accepted. `0`, negative numbers, `inf`, and `nan` are errors at the value.
- Each resource type produces exactly one good. Several resource types may produce the same good.
- The engine doesn't require the produced good to be unmovable or to perish after one period, though that is how the vision describes unimproved output. The setting decides, and the sample setting follows the convention.
- Resource types get their IDs in load order: the order files are read, then the order within each file. The same folder always gives the same IDs.
- A resource type with no deposits is valid.

### Deposits
- Deposits are placed by `[[deposit]]` sections. They belong in `start/`, because where resources lie and how many there are is part of what turn 0 looks like. A `[[deposit]]` in `rules/` is an error saying it belongs in `start/`.
- Any file in `start/`, at any depth, may hold any number of `[[deposit]]` entries. Entries from every file are merged.
- Each entry has these keys:
  - `resource` (required): the name of a resource type the setting defines.
  - `q` and `r` (required): the axial coordinates of the hex, written and checked exactly as for `[[hex]]` (whole numbers in the 32-bit signed range). There must be a hex at those coordinates.
  - `count` (optional, default 1): how many deposits of that type the hex holds. A whole number from 1 to 1,000,000. `0`, negative numbers, numbers above 1,000,000, and non-integers are errors at the value. The cap turns a typo such as `count = 4000000000` into a clear error instead of an attempt to create billions of deposits.
- An entry with `count = 3` places three separate deposits. They are identical when the game starts: each outputs the resource type's `output` per period. Each is its own unit with its own ID, so in Stage 02 each can have a different owner. A deposit has one owner; shared ownership of one deposit is not planned.
- A hex may hold deposits of several resource types. Each resource type appears in at most one entry per hex: a second `[[deposit]]` entry for the same type on the same hex is an error naming both places, because the author should raise `count` instead.
- Deposits have no names. They get IDs in load order: the order entries are read (files, then entries within each file), and within an entry, its `count` deposits one after another. The same folder always gives the same IDs.

Example:
```toml
# rules/resources.toml
[[resource]]
name = "copper_ore"
produces = "unimproved_copper"
output = 2.5

# start/deposits.toml
[[deposit]]          # two copper_ore deposits on (1, -1): 5 unimproved_copper per period in total
resource = "copper_ore"
q = 1
r = -1
count = 2

[[deposit]]          # count left out: one deposit
resource = "copper_ore"
q = -2
r = 1
```

### References
- Three references are checked after every file has been read, so each may point into any file: a resource type's good, a deposit entry's resource type, and a deposit entry's hex.
- Each unknown target is an error at the reference: an unknown good (at `produces`), an unknown resource type (at `resource`), or no hex at the entry's coordinates (at the `[[deposit]]` header, since the coordinates are two keys).
- Reference checking follows the existing rules from Steps 02 and 03. It is skipped when a file couldn't be read or discovery couldn't see some data files, because the target might be in a file the loader never saw. Only the earlier errors are reported then.

### Errors
Each is reported at the right file, line, and column.

Resource types:
- a missing `name`, or a name that isn't lowercase `snake_case`;
- the same resource type name in two files (the error names both places);
- a missing `produces` or `output`;
- a value of the wrong type (for example `output = "5"` or `produces = 3`);
- an `output` that isn't greater than 0 and finite;
- a `produces` naming a good the setting doesn't define;
- an unknown key;
- a `[[resource]]` in `start/`.

Deposits:
- a missing `resource`, `q`, or `r`;
- a value of the wrong type;
- a `q` or `r` outside the 32-bit range;
- a `count` that isn't a whole number from 1 to 1,000,000;
- a `resource` naming a resource type the setting doesn't define;
- coordinates with no hex;
- a second entry for the same resource type on the same hex (the error names both places);
- an unknown key;
- a `[[deposit]]` in `rules/`.

As with every section, all of an entry's problems are reported in one load. An entry whose `resource`, `q`, or `r` is missing or invalid isn't checked for duplicates, so it can't produce a misleading duplicate error. The load fails anyway.

### What a loaded setting provides
Resource types are part of the loaded setting's rules, and deposits are part of its starting state. Through the engine's public interface, a caller (the dev UI, and the world state in Step 06) can:
- list every resource type in load order, which is also ID order; look up a resource type's ID by name and its name by ID; and get the good it produces and its output per deposit;
- list every deposit in load order, which is also ID order, with its resource type and hex;
- list the deposits on a given hex, and the deposits of a given resource type on a given hex (empty when there are none);
- get the number of resource types and the number of deposits.

The number of deposits counts individual deposits, not entries: an entry with `count = 3` adds three.

### The sample setting
- `settings/sample/rules/resources.toml` defines two resource types:

| Resource type | Produces | Output per deposit |
|---|---|---|
| `copper_ore` | `unimproved_copper` | 2.5 |
| `arable_land` | `unimproved_grain` | 4 |

- `settings/sample/start/deposits.toml` has six entries on the 19-hex map, placing 10 deposits:

| Hex | Resource type | `count` |
|---|---|---|
| (0, 0) | `arable_land` | 3 |
| (0, 0) | `copper_ore` | (left out: 1) |
| (−1, 0) | `arable_land` | 2 |
| (1, 0) | `arable_land` | (left out: 1) |
| (1, −1) | `copper_ore` | 2 |
| (−2, 1) | `copper_ore` | (left out: 1) |

- This shows two resource types on one hex, several deposits of one type on a hex, `count` left out, and a decimal output.
- Comments in each file explain resource types, deposits, and `count`.
- The sample setting still loads without errors.

### The setting checker
- On success, the dev UI's setting checker adds resource and deposit counts after the hex count: `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits`. Singulars are `1 resource` and `1 deposit`; with none, `0 resources` and `0 deposits`.
- Running with no argument, and the failure output, are unchanged.

### Test settings
- Small test settings cover resources and deposits:
  - a valid set across several files, including nested folders, with: a hex holding deposits of two types; an entry with `count` above 1 and one without `count`; a resource type with no deposits; two resource types producing the same good; and both whole-number and decimal outputs;
  - a setting with one resource type and one deposit;
  - settings that trigger each error above, including the reference errors.

## Player-facing Changes
None for players. Setting authors can define resource types and place deposits on the map, and the sample setting shows how. The setting checker reports how many resource types and deposits a setting defines.

## Interactions
- **Step 01 (engine-workspace):** resource types and deposits are new ID kinds.
- **Step 02 (setting-loader):** the first real references, so the reference-checking phase does real work for the first time.
- **Step 03 (goods-and-properties):** `produces` names a good. The skip rules for reference checking added there apply here.
- **Step 04 (hex-map):** deposits are placed by hex coordinates, written and range-checked like `[[hex]]`. Step 04 expected "a deposit on coordinates with no hex" to be an error here.
- **Step 06 (turn-clock):** each turn, every deposit emits its resource type's `output` of its good.
- **Steps 07–08 (dev UI):** show deposits on the map and let the developer inspect them.
- **Stage 02 (populate-the-world):** each deposit gets one owner. Several owners on one hex own separate deposits.

## Out of Scope
- Producing anything: emitting output each turn is Step 06.
- Ownership of deposits (Stage 02).
- **Stock, depletion, and regeneration.** A resource type may later get an optional `stock` (how much a fresh deposit holds); leaving it out will mean the deposit never depletes, so settings written now stay valid. Regeneration (stock that grows back each period, as for forests or fisheries) will be added with it, also as optional data on the resource type. These are postponed because it isn't yet clear whether stock should fall by what a deposit emits or by what is actually used. Unused output perishes, so depleting by emission would drain a deposit nobody works. Depleting by use needs production to consume the unimproved good (Stage 03). A deposit's remaining stock would be world state, tracked per deposit.
- Deposits of one type on one hex that differ from each other at the start (different outputs).
- Checks that a resource type's good is unmovable or perishes after one period.
- A resource type producing more than one good.
- Display names, descriptions, or units for resource types.
- Divisibility rules for goods. Output is a continuous number, but nothing yet decides how quantities of goods are held.

## Acceptance Criteria
- The sample setting loads with the two resource types in the table (goods and outputs as listed) and the 10 deposits from the six entries, in load order, on the listed hexes.
- A valid test setting with resource types and deposits across several files, including nested folders, loads. Resource types and deposits get IDs in load order (an entry's `count` deposits consecutively), and loading it twice gives the same IDs.
- For each resource type, the loaded setting gives its name, its ID by name, the good it produces, and its output. For each deposit, it gives its resource type and hex.
- An entry with `count = 3` gives three deposits; an entry without `count` gives one.
- Listing a hex's deposits gives every deposit on it, and nothing for a hex with none. Listing the deposits of one type on a hex gives only those.
- A whole-number output and a decimal output both load with the right values.
- A setting with no resource types and no deposits loads.
- Each error listed under Errors is reported at the right file, line, and column. The duplicate errors name both places.
- Unknown goods, unknown resource types, and coordinates with no hex are each reported at the reference, even when the target would be defined in a later file. They are not reported when a file couldn't be read or discovery was incomplete.
- One resource type or deposit entry with several problems reports all of them in one load. An entry with a missing or invalid `resource`, `q`, or `r` causes no duplicate error.
- The dev UI with the sample setting's path prints `Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits` and exits successfully. A setting with one of each says `1 resource` and `1 deposit`; one with none says `0 resources` and `0 deposits`.
- The dev UI with no argument, and with a broken setting, behaves as before.
- The single check command passes.
