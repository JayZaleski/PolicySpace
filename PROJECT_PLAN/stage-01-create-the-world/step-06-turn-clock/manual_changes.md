# Step 06: turn-clock — Manual Changes

No change for players. Setting authors and developers can now see a setting run: this extends the "Writing a setting" material from Steps 02 to 05 and the setting checker description.

## What the manual should cover
- What the world is: a loaded setting becomes a running world with a clock starting at turn 0. The rules stay fixed; the world holds the clock and the goods that exist.
- The turn, in plain terms: at the start of each turn every deposit produces its resource type's `output` of its good, in the stockpile of its hex; then (later) actors act; at the end of the turn goods that have reached their limit perish. Nothing acts yet, and deposits are unowned, so everything lands in an "unowned goods" stockpile on the hex.
- `perishes_after` now has an effect (replace any earlier note that it does nothing): with 1, output is usable during the turn it appears and gone when it ends; with 3, it is gone at the end of the third turn it exists. Goods without `perishes_after` last forever and accumulate. `movable` still has no effect.
- What a hex stockpile holds: the total of each good, and for each total, how much perishes at the end of which turn.
- Turn reports: each turn lists what perished and what was emitted, as totals per good over the whole map.
- The setting checker's new `--turns N` option (N from 0 to 10,000): prints one line per turn, and `--turns 0` prints only turn 0. Show how to read a line. Explain the usage error for bad values. Update the checker's usage message in earlier text to `dev-ui [SETTING_FOLDER [--turns N]]`.
- Update earlier text: the Step 05 notes that "nothing is produced yet" become "each deposit produces its output every turn" (still no ownership, depletion, or stock; the manual's planned-later list keeps stock, depletion, and regeneration as later features).

## Where it fits
A new subsection "Running a setting" at the end of "Writing a setting" (after "Natural resources"), since it is how an author checks that their resources behave as intended.

## Links
- `docs/world.md` for the turn, stockpiles, and reports.
- `docs/goods.md` for `perishes_after`, `docs/resources.md` for deposits and output, and `docs/setting-loader.md` for the checker.

## Example
Show the sample setting run with `--turns 2` (the three output lines from the design). A small worked table would help for `perishes_after = 3` with output 5 per turn: stockpile totals of 5, 10, 15, 15, 15 over turns 0 to 4, with 5 perishing from turn 3. A diagram of one turn as three boxes (emit, actors act, perish) would clarify when the world is observed (after the emit, before the perish). An interactive demo is not needed.
