# Step 03: pop-types

## Summary
A POP type describes one individual of a POP: the goods it receives each turn (its endowment) and the Stone-Geary utility function it consumes by, as a subsistence amount and a share for each good it wants. POP types are persistent rules, defined in `rules/` and checked by the loader like resource types: the numbers are validated and the good names are resolved in the loader's second phase. Leisure needs no special case: it is an endowed good that also appears in the utility, consumed rather than sold. Nothing uses POP types yet. Step 04 places POPs of these types, Step 05 computes their demand, and Step 06 trades. The sample setting gains two POP types.

## Behavior

**In the setting folder.** A `[[pop_type]]` entry in `rules/` has three required keys.

- `name`: the type's name, lowercase `snake_case`, unique among POP types. POP type names and good names are separate namespaces, as resource type names are.
- `endowment`: a table of good name = amount, the amount of each good one individual receives at the start of every turn. Each amount is a number (whole or with decimals), finite and greater than 0. The table may be empty: such a type has no income and is always below subsistence unless every subsistence amount is zero, which is useful for testing the shrinking path. The intended design is that the only endowed good is unskilled labor; the generalization exists so the POP system can be tested before production and the live market exist, and the sample's comments and the documentation say so.
- `utility`: a table with one sub-table per good the individual consumes. Each sub-table has two optional keys, `subsistence` (the amount the individual needs before anything else, finite, at least 0, default 0) and `share` (its weight among the goods bought beyond subsistence, finite, at least 0, default 0). A listed good with both values zero, or with neither key, does nothing and is an error. At least one good must have a positive share, so an empty `utility` is an error.

Both TOML table forms are accepted, inline (`endowment = { unskilled_labor = 10 }`) and as sub-tables (`[pop_type.utility.grain]`), as for a market's `prices`. A `[[pop_type]]` in `start/` gets the usual "belongs in rules/" error.

**Shares are weights.** The engine normalizes the shares of a type's utility goods so they sum to 1. Setting authors never have to rebalance a file to make shares add up, and a `0.1 + 0.2 + 0.7` file cannot fail on floating point. A good with `share = 0` but a positive subsistence is a pure necessity: it is bought at its subsistence amount and never more.

**Leisure and the endowment.** Leisure is an endowed good that also appears in the utility; the engine treats it like any other good. An endowed good that is not in the utility is sold entirely (from Step 06). A utility good that is not endowed is simply bought. There is no cross-check between a good's endowment and its subsistence: with the generalized endowment a POP may buy more of an endowed good than it receives, so a subsistence above the endowment is not necessarily unreachable. The documentation notes that, for a good only the POP itself produces, such as labor, a subsistence at or above the endowment means the type can never reach subsistence. The currency good may appear in the endowment or the utility like any other good; its price is 1.

**Errors**, each reported at the offending place: a missing `name`, `endowment` or `utility`; a bad or duplicate name (the message names the first definition); `endowment` or `utility` that is not a table; an endowment amount that is not a number, is 0 or negative, or is infinite or NaN; a utility entry that is not a table; a `subsistence` or `share` that is not a number, is negative, or is not finite; a utility good with nothing positive; a utility with no positive share; an unknown key in the entry or in a utility good's sub-table. A good name in `endowment` or `utility` that matches no good is reported as an unknown good in the loader's second phase, after every file has been read, so the good may be defined in any `rules/` file. Like other reference checks, it is skipped when the loader could not see every file. Every key is read even when another is bad, so one entry with several problems reports all of them in one load, and a type is defined by name even when its other keys are bad, so later duplicates are still caught.

**In the engine.** POP types are a named list in the rules, like resource types: each has an ID and a name, and a type holds its endowment as (good, amount) pairs in good ID order and its utility goods as (good, subsistence, normalized share) in good ID order. The normalized shares are what the engine stores; the weights as written are not kept. IDs follow load order, so loading the same folder always gives the same result. A setting with no POP types is valid. The turn is unchanged: nothing reads POP types during a step.

## Player-facing Changes
None visible yet. A setting author writes `[[pop_type]]` entries in `rules/`. The sample setting gains `rules/pops.toml` with two types, both endowed with 10 unskilled labor per individual per turn: `peasant`, which keeps some labor as leisure and spends mostly on grain, and `artisan`, which keeps less leisure and weights copper more. The file's comments explain the keys, that shares are weights, and that the endowment is generalized for testing while unskilled labor is the intended endowment. The checker's success line counts POP types alongside the other kinds of thing. The dev UI shows POPs in Step 08.

## Interactions
- **Goods (Stage 01 Step 03):** endowment and utility good names are references to goods, resolved against the goods registry like a resource type's `produces`.
- **Resource types (Stage 01 Step 05):** the model for a named rules entry with good references; POP types follow the same reader pattern.
- **Dummy market (Step 02):** the number-table reader added for prices serves the endowment table.
- **POPs (Step 04):** a POP names its type; at the start of each turn the type's endowment, times the POP's size, is emitted into the POP's stockpile. **Stone-Geary demand (Step 05):** takes a type's subsistence amounts and shares, the endowment and the prices, and must handle goods that are not traded on the hex. **POP trade (Step 06):** sells the endowment the POP doesn't keep. **Dev UI (Step 08):** shows each POP's type.
- **Roadmap:** Steps 04, 05, 06 and 08 are reworded from "labor" to "endowment".
- **Test settings:** existing test settings have no POP types and must keep loading unchanged.

## Out of Scope
- Placing POPs, emitting endowments, computing demand, trading, or growth (Steps 04 to 07).
- Per-POP parameters that differ from the type, or assigning parameters from a set of possibilities.
- Growth-rate parameters (Step 07).
- Any cross-check between endowment and subsistence, or between endowed goods and utility goods.
- Any restriction on which goods may be endowed or consumed, including the currency.
- Showing POP types in the dev UI (Step 08).

## Acceptance Criteria
- The sample setting loads without errors and reports two POP types, `peasant` and `artisan`, each endowed with 10 unskilled labor, with the expected subsistence amounts and shares that sum to 1 in the order of good IDs.
- A test setting with POP types across several files, both table forms, a multi-good endowment, an empty endowment, a pure-necessity good (share 0), and the currency in a utility loads, and the normalized shares of each type sum to 1 with the expected proportions.
- Each error case produces one error at the expected file, line and column with the expected text: missing keys, a bad and a duplicate name, non-table `endowment` and `utility`, an endowment amount that is a string, zero, negative or `inf`, a non-table utility entry, a negative or non-finite `subsistence` or `share`, a utility good with nothing positive, a utility with no positive share, an unknown good in `endowment` and in `utility`, an unknown key in the entry and in a sub-table, and a `[[pop_type]]` in `start/`.
- One entry with several problems reports all of them in one load.
- The unknown-good checks are skipped when another file in the setting is broken.
- The checker's success line counts POP types, and a setting with no POP types reports zero.
- Stepping a World is unchanged, and all existing tests pass unchanged.
