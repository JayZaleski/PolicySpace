# Step 04: pops

## Summary
This Step puts POPs on the map. A POP is a group of N identical individuals of one POP type living on one hex, placed in the starting state. Each POP owns its own stockpile, which starts empty: POPs have no cash and no goods until they earn them. At the start of every turn, including turn 0, each POP receives its type's endowment times its size into its stockpile, and at the end of every turn its stockpile perishes like the hex stockpiles do. The turn report gains an `endowed` list so the Step can be watched working in the checker and the dev UI's turn log. Nothing consumes, trades, or grows yet: Step 05 computes demand, Step 06 trades, Step 07 changes N. The sample setting gains POPs on most of its hexes.

## Behavior

**In the setting folder.** A `[[pop]]` entry in `start/` has four required keys.

- `type`: the name of a POP type defined in `rules/`.
- `q`, `r`: the axial coordinates of a hex that exists in the map, read exactly as a deposit's are.
- `size`: N, the number of individuals. A number (whole or with decimals), finite and greater than 0.

Size is continuous, not a whole number. A POP is an aggregate: its endowment and (from Step 05) its demand are per-individual amounts scaled by N, and Step 07's growth multiplies N by a rate, which with whole numbers would freeze small POPs or need a rounding rule. Fractional individuals are no stranger than a deposit emitting 2.5 of a good. The dev UI may round N for display. The starting size must be positive; what happens to a POP that shrinks toward zero is Step 07's decision.

A hex holds at most one POP of each type. Two POPs of one type on one hex would see the same prices and the same per-individual endowment, so they would behave as one POP split in two; a second `[[pop]]` with the same type and coordinates is a duplicate error naming the first. A hex may hold POPs of several types, and a hex may hold none. POPs are state, so a `[[pop]]` in `rules/` gets the usual "belongs in start/" error.

**Errors**, each reported at the offending place: a missing `type`, `q`, `r` or `size`; a `type` that is not a string; a bad, missing or out-of-range coordinate; coordinates that match no hex; a `size` that is not a number, is 0 or negative, or is infinite or NaN; a duplicate POP (same type name as written and same coordinates); an unknown key. A `type` that matches no POP type is reported at the value as an unknown POP type in the loader's second phase, after every file has been read, so the type may be defined in any `rules/` file. Like other reference checks, it and the hex lookup are skipped when the loader could not see every file. Every key is read even when another is bad, so one entry with several problems reports all of them in one load. As with deposits, an entry whose `type` or coordinates are missing or invalid takes no part in the duplicate check; an invalid `size` does not stop the reference checks.

**In the engine.** POPs are a list in the starting state, like deposits: each has an ID given in load order (file order, so loading the same folder always gives the same result), a hex, a type, and a size. The list can be asked for the POPs on a hex, in ID order. The World takes the POPs over as state, because size changes from Step 07, and gives every POP its own stockpile, the same `Stockpile` type the hexes use. All POP stockpiles are empty when the world is created. The hex stockpiles stay as they are: deposits still emit into them, since deposits have no owners in this Stage. Through the World a caller can read the POP list, each POP's hex, type and size, and each POP's stockpile with the same totals and perish breakdown a hex stockpile offers.

**The turn.** The three parts stay, and POPs join two of them.

1. **Start of turn:** every deposit emits as before. Then every POP, in ID order, receives its endowment: for each good in its type's endowment, the per-individual amount times the POP's size, added to the POP's stockpile as a group that perishes when a unit of that good appearing this turn perishes. In the sample, unskilled labor perishes after one turn, so each turn's labor is gone at the end of that turn. Deposits emit before POPs so every existing report is unchanged, and the order of floating-point additions is fixed.
2. **Actors act:** still nothing.
3. **End of turn:** every group that perishes at the end of this turn is removed from every hex stockpile and then from every POP stockpile, in ID order.

Creating a world runs turn 0's start, so a fresh world's POPs already hold their first endowment. A step at the clock's maximum is still refused and changes nothing. A world with no POPs behaves exactly as before.

**The turn report** gains a third list, `endowed`: the total of each good received by all POPs at the start of the turn, in good ID order, listing only amounts greater than zero. It is separate from `emitted` so deposit output and POP endowment can be told apart, and so this Step is observable before Step 08's dev UI. The `perished` list now covers hex and POP stockpiles together. The checker's turn line and the dev UI's turn log print the new list after the other two: `Turn 1: perished 4100 unskilled_labor, 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor` (each list is in good ID order, and `unskilled_labor` is the sample's first good). An empty list prints `nothing`, as before. A type with an empty endowment contributes nothing.

## Player-facing Changes
None visible to a player yet. A setting author writes `[[pop]]` entries in `start/`. The sample setting gains `start/pops.toml`: `peasant` POPs of varying sizes on most of its 19 hexes, including the center and the north-east corner hex that has no market (so one POP's labor can never be sold, Step 06's "not traded here" case); an `artisan` POP at the center as well, the only hex that prices every good; and a few hexes with no POPs. The file's comments explain the keys, that size is continuous, and that one POP per type per hex is allowed. The checker's success line counts POPs alongside the other kinds of thing, and `check --turns` shows the `endowed` list. The dev UI's turn log shows the same line; inspecting POPs in the dev UI is Step 08.

## Interactions
- **POP types (Step 03):** a POP's `type` is a reference to a POP type, resolved like a deposit's `resource`; the type's endowment is what the POP receives.
- **Hex map (Stage 01 Step 04):** POP coordinates resolve to a hex like a deposit's. The map is unchanged.
- **Deposits (Stage 01 Step 05):** the model for a placed thing in `start/` with a type reference, coordinates, and a per-hex index; POPs follow the same reader and list pattern.
- **World and turn (Stage 01 Step 06):** the World gains the POP list and one stockpile per POP; emission and perishing reach POP stockpiles; the report gains `endowed`.
- **Dummy market (Step 02):** untouched. A POP's stockpile is what the market will exchange against from Step 06.
- **Stone-Geary demand (Step 05):** reads a POP's type and size. **POP trade (Step 06):** sells from and buys into the POP's stockpile in the "actors act" part, and extends the report with its own POP totals in the same style. **POP growth (Step 07):** changes a POP's size; next turn's endowment follows. **Dev UI (Step 08):** shows each POP's type, size and stockpile.
- **Checker and dev UI turn line:** gains the `endowed` list.
- **Test settings:** existing test settings have no POPs and must keep loading and stepping unchanged, with their turn lines unchanged except for the added `; endowed nothing`.

## Out of Scope
- Starting inventory or cash for POPs: every POP stockpile starts empty.
- Demand, trade, consumption, and growth (Steps 05 to 07).
- Any change to a POP's hex or type during play, migration, or POPs disappearing.
- Per-POP parameters that differ from the type.
- Deposit ownership, or any change to where deposits emit.
- Showing POPs in the dev UI (Step 08).

## Acceptance Criteria
- The sample setting loads without errors and reports its POPs with the expected hexes, types and sizes; the center hex lists a peasant and an artisan POP, the north-east corner hex lists a peasant POP, and a hex with no entry lists none.
- A fresh sample world is at turn 0, every POP's stockpile holds exactly its type's endowment times its size as one group perishing at the end of turn 0, the hex stockpiles hold what they held before this Step, and the report's `endowed` list gives the total unskilled labor over all POPs.
- Stepping the sample world perishes every POP's labor along with the deposit output, then endows the same amounts again; this repeats every step. Two worlds from the same setting stepped the same number of times are identical.
- A test setting with POPs across several files, two POPs of different types on one hex, a type with a multi-good endowment including a durable good (which accumulates in the POP's stockpile and never perishes), a type with an empty endowment, and a fractional size loads and steps as expected.
- Each error case produces one error at the expected file, line and column with the expected text: missing `type`, `q`, `r` and `size`; a non-string `type`; bad coordinates and coordinates with no hex; a `size` that is a string, zero, negative or `inf`; a duplicate POP; an unknown POP type; an unknown key; and a `[[pop]]` in `rules/`. One entry with several problems reports all of them in one load.
- The unknown-type and no-hex checks are skipped when another file in the setting is broken.
- The checker's success line counts POPs, and a setting with no POPs reports zero. `check --turns` prints `endowed` after `emitted` on every turn line, `nothing` when no POP received anything, and the dev UI's turn log shows the same lines.
- Every existing test passes, with turn-line expectations updated only by the added `endowed` part.
