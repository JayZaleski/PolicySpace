# Step 03: pop-types — Manual Changes

Nothing new is visible when running the world: no POPs exist yet, the turn reports are unchanged, and the developer UI doesn't show POP types (that comes in Step 08). The change is to the setting format, plus one extra count in the checker's success line.

## What setting authors can now do

- Define **POP types** with `[[pop_type]]` sections in `rules/` (in `start/` it is an error). A POP type describes one individual of a POP: what it receives each turn and what it wants to consume. Mention that the header is `[[pop_type]]`: placed POPs, coming in the next Step, will use `[[pop]]`, so authors shouldn't confuse the two.
- Three required keys:
  - `name`: lowercase snake_case, unique among POP types (a POP type may share a name with a good).
  - `endowment`: `good = amount` pairs, the amount of each good one individual receives at the start of every turn. Amounts are numbers greater than 0. It may be empty (a type with no income, mainly for testing).
  - `utility`: one sub-table per good the individual consumes, each with two optional numbers that default to 0: `subsistence` (the amount needed before anything else) and `share` (the good's weight in spending beyond subsistence). Each listed good needs at least one of them positive, and at least one good needs a positive share.
- **Shares are weights, not percentages.** The engine scales them to sum to 1, so `1, 4, 5` means 10%, 40% and 50%, and authors never have to make them add up. This is worth stating plainly with an example.
- **Pure necessity:** a good with a subsistence and no share is bought up to its subsistence and never more.
- **Leisure:** there is no special leisure setting. A good that is endowed and also in the utility is kept and consumed (leisure); an endowed good not in the utility will be sold, and a wanted good that isn't endowed will be bought. These behaviors arrive in later Steps; say so, and say that nothing uses POP types yet.
- **Intended use vs. what's allowed:** the intended design endows only unskilled labor. Any good may be endowed so the POP system can be tested before production and the live market exist. The currency may appear like any other good.
- **A pitfall worth a sentence:** the engine doesn't compare subsistence with the endowment. For a good only the POP itself produces, such as labor, a subsistence at or above the endowment means the type can never reach subsistence.
- Both TOML forms work: inline tables (`endowment = { unskilled_labor = 10 }`) and sub-table headers (`[pop_type.utility.grain]`).
- New errors, with file, line and column: missing `name`, `endowment` or `utility`; a bad or duplicate name; `endowment` or `utility` not a table; an endowment amount that isn't a number above 0 (including `inf`); a utility entry that isn't a table; a negative or non-finite `subsistence` or `share`; a utility good with nothing positive; a utility with no positive share; an unknown good; an unknown key in the entry or in a utility good. As with other references, unknown-good errors are skipped when part of the setting couldn't be read.

## Where it fits

- **New subsection "POP types"** under "Writing a setting", after the "Markets" subsection that Step 02's draft adds (and before "Running a setting"). Add it to the Contents list. It should cover the keys, shares as weights, pure necessities, leisure, the intended labor-only endowment, a short TOML example, and the sample's two types.
- **"The folder layout"**: the `rules/` line in the layout block should mention POP types (`goods, resource types, POP types`), and the bullet listing which section types go in which folder should add `[[pop_type]]` to `rules/`.
- **"Naming rules"**: the sentence "Names of goods and resource types ..." should include POP types.
- **"Running a setting"**: no change needed beyond Step 02's draft; optionally note in part 2 ("Actors act") that POPs will act here once placed.
- **"Checking a setting"**: the sample success line gains a count. **This supersedes Step 02's manual draft**, which gave the line ending in `18 markets`; it is now `Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types`. The singular-wording note should add `1 pop type`. Optionally add a POP type error (for example `utility of pop_type needs at least one good with a positive share`) to the sample error listing, keeping the error count line consistent.
- **"The developer UI"**: no change; "Not available yet" may mention that POPs appear in a later Step.

## The sample setting

`settings/sample/rules/pops.toml` defines two types, both endowed with 10 `unskilled_labor` per individual per turn:

| Type | Good | Subsistence | Share as written | Share used |
|---|---|---|---|---|
| `peasant` | `unskilled_labor` (leisure) | 2 | 0.2 | 0.2 |
| | `grain` | 2 | 0.8 | 0.8 |
| `artisan` | `unskilled_labor` (leisure) | 1 | 1 | 0.1 |
| | `grain` | 2 | 4 | 0.4 |
| | `copper` | 0 | 5 | 0.5 |

Peasants keep some labor as leisure and spend mostly on grain; artisans keep less leisure and weight copper heavily. The artisan rows show weights being normalized. The file's comments explain every key.

## Links to docs/

- [docs/pops.md](docs/pops.md) (new), for the data model, normalization and the loading rules. Link it from the end of the new "POP types" subsection.
- [docs/goods.md](docs/goods.md), where the subsection mentions that endowment and utility name goods.

## Examples or diagrams

- A short TOML example of one type in the inline form, and the same or another type in the sub-table form, including a pure-necessity good (subsistence only).
- The table above is the clearest way to show weights becoming shares. No diagram or interactive demo is needed yet: until demand is computed (Step 05), there is nothing to plot. A small demo of how a type spends an income across its goods would fit once Step 05 lands.
