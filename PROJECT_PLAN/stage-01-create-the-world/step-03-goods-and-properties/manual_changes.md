# Step 03: goods-and-properties — Manual Changes

No change for players. Setting authors gain the first real content type, so this extends the "Writing a setting" section drafted in Step 02. That draft's note that no section types exist yet is now out of date and should be removed or replaced.

## What the manual should cover
- What a good is: anything an actor can own. The engine has no built-in goods or categories like labor or capital; settings define them all.
- How to define goods: `[[good]]` sections in `rules/`, in any file at any depth, any number per file; goods are merged and get IDs in load order. A `[[good]]` in `start/` is an error. A setting with no goods is valid.
- The fields: required `name` (lowercase snake_case, unique across the whole setting); optional `perishes_after` (whole number 1 or more; absent means it never perishes; explain with the turn-5 examples: 1 means gone when the turn ends, 3 means gone at the end of turn 7); optional `movable` (default true). State plainly that perishing and movement do not happen yet: perishing arrives with the turn clock, movement later. Unknown keys and wrong types are errors.
- The sample setting's five goods, as a table (name, perishes_after, movable, role).
- The setting checker's success line now ends with a goods count (`... (format version 1): 5 goods`); update any earlier example output from Step 02.
- Briefly, that reference errors are not shown when part of the setting could not be read (a missing folder, a broken or unreadable file), so authors should fix the structural problem first.

## Where it fits
Inside the Step 02 "Writing a setting" section, as a new subsection "Goods" after the folder and naming rules. Update the Step 02 note that said the sample setting is only a manifest.

## Links
- `docs/goods.md` for the data model and how goods are loaded.
- `docs/setting-loader.md` for error reporting and the checker.

## Example
Show the TOML example from the design (`unskilled_labor` with both properties, `copper` with only a name), plus one or two sample error lines (for example a duplicate good name naming both places, and `perishes_after = 0`). An interactive demo is not needed.
