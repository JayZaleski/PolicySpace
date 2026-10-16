# Step 01: technologies

## Summary
This Step defines technologies in the setting's rules: the named recipes that factories run, shared by every factory that uses one, the way a POP type is shared by POPs. A technology says what a factory of that kind makes and from what, how much it can make per turn, what it is made of (its capital bundle), what it took to build (its construction bundle and construction time), and how long it lasts. The engine loads and validates them and checks their references to goods. Nothing runs yet: factories, production, depreciation and maintenance are later Steps of this Stage, but their arithmetic depends on the numbers defined here, so this Step records what each number means.

## Behavior

### What a technology is
A technology is persistent data in `rules/`, with a name unique among technologies (technology names, good names, resource type names and POP type names are separate namespaces). Every factory points at one technology; that pointer is state (Step 03) so it can change later, but the recipe itself never changes during play.

A technology has:

- **Inputs.** A table of good = amount: how many units of each input one unit of output takes. The recipe is fixed-proportion: to make X units of output a factory needs X times each amount, with no substitution between inputs. The table may be empty (a technology that makes its output from nothing is a legitimate test case; the engine doesn't police economics, as with an empty POP endowment). The output good may not also be an input: a recipe that consumes its own product is almost certainly a mistake, so the loader rejects it.
- **Output.** One good.
- **Capacity.** The most output a factory of this technology can make in one turn. Capacity belongs to the technology, so a business that wants more needs more factories.
- **Capital bundle.** A table of good = amount: the goods a finished factory holds. It must name at least one good, because a factory with nothing to wear out would never shut down, silently breaking the rule below.
- **Construction bundle.** A table of good = amount: the goods spent to build a factory of this technology. They are consumed and don't become part of the factory. The intended use is labor, but it is a general table, like a POP type's endowment, because the engine has no notion of labor. It may be empty: a technology whose factories take nothing to build or maintain beyond the bundle itself is a legitimate test case.
- **Construction turns** (t) and **lifetime** (t₁). Whole numbers of at least 1.

### Validation
Every amount in the inputs, capital bundle and construction bundle, and the capacity, is a number, finite and greater than 0, like a resource type's `output` and a POP type's endowment amounts. Construction turns and lifetime are whole numbers of at least 1. Good names are references, resolved in the loader's second phase, so a good may be defined in any `rules/` file, and an unknown good is reported at the name. As for every section, every key of an entry is read before anything is decided, so one entry with several problems reports all of them in one load, each at its place, and a technology with a bad name still takes part in the duplicate check. A `[[technology]]` in `start/` gets the usual "belongs in rules/" error. Technology IDs follow load order, so the same folder always gives the same IDs. A setting with no technologies is valid.

### What the numbers mean
These rules are the meaning of a technology's numbers. The later Steps of this Stage (production, depreciation and maintenance, business trade) implement them; this Step only records them, so the data shape and the behavior agree.

- **Depreciation.** Each turn, whether or not it ran, a factory consumes 1/t₁ of its technology's capital bundle, in the bundle's proportions. A factory that starts full is empty after t₁ turns and shuts down when its bundle is exhausted.
- **Construction.** Building a factory pays 1/t of the capital bundle and 1/t of the construction bundle each turn for t turns. The capital slices accumulate into the factory; the construction slices are spent. The factory can't run until construction is complete. Construction itself is a business decision for Stage 07; the parameters are recorded now because maintenance is defined in terms of them.
- **Maintenance.** Maintenance is the construction project run for a fraction of its time, while the factory keeps running. Replenishing a share f of the capital bundle costs f of the capital bundle plus f of the construction bundle. It is continuous: a business may replenish any share it chooses, and pays exactly that share. The per-turn construction slices are a cap on how much maintenance work can be done in one turn, not a minimum: a factory can regain at most 1/t of its bundle per turn, and bringing one from 95% to 100% costs 5% of each bundle.
- **A consequence, not an error.** A factory loses 1/t₁ of its bundle per turn and can regain at most 1/t per turn, so steady maintenance keeps up with depreciation only when t ≤ t₁. A technology with t > t₁ wears out faster than it can be maintained. That is a legitimate thing for a setting to say, so the loader doesn't reject or warn about it.

### Sample setting
The sample gets a production chain on the center hex, which already holds grain and copper deposits and is the only hex that trades resource output:

- A new durable, movable good, `tools`, the capital good for the bundles, and a price for it on the center hex's market, so the setting stays coherent when businesses start trading it in Step 06.
- Three technologies: a farm (unimproved grain and labor into grain), a mine (unimproved copper and labor into copper), and a toolmaker (copper and labor into tools). Each has a capital bundle of tools, a construction bundle of labor, and t ≤ t₁.
- The numbers are chosen so that, at the center hex's posted prices, the farm and the mine run at a clear profit and the toolmaker's margin is thin. Step 06 then has both a running and a marginal case to show. The exact numbers are for the spec.
- The file's header comment explains every key and the meaning of the numbers, in the style of the other sample files.

## Player-facing Changes
None in the UI. Setting authors get a new section, `[[technology]]`, in `rules/`, with clear errors for bad values and unknown goods. The sample setting gains a capital good, its price, and three technologies.

## Interactions
- **Goods** (Stage 01 Step 03): inputs, output, capital and construction bundles name goods, resolved like a POP type's endowment and utility goods.
- **Setting loader** (Stage 01 Step 02) and **POP types** (Stage 02 Step 03): technologies follow the `[[pop_type]]` pattern, a named `rules/` kind with tables keyed by good name, every key read before anything is decided, references resolved in phase 2.
- **Dummy market** (Stage 02 Step 02): the sample's center market gets a price for `tools`.
- **Later Steps of this Stage:** Step 03 (factories) points each factory at a technology and holds its current capital bundle, which is compared with the technology's full bundle. Step 04 (production) uses the inputs, output and capacity. Step 05 (depreciation and maintenance) uses the capital bundle, construction bundle, construction turns and lifetime by the rules above. Step 06 (business trade) uses all of them to decide and trade.

## Out of Scope
- Factories, production, depreciation, maintenance and business decisions: later Steps of this Stage. This Step adds no turn behavior.
- Construction of new factories, scrapping, and transitions (retooling) between technologies, including which transitions a technology allows: Stage 07. Construction turns and the construction bundle are recorded now only because maintenance is defined by them.
- Several output goods per technology, by-products, or substitution between inputs.
- Any check that a technology is economically sensible (profitable, maintainable, or that its inputs can be had).
- Showing technologies in the dev UI: Step 07 of this Stage.

## Acceptance Criteria
- The sample setting loads, with `tools` among its goods, a price for it on the center hex, and the three technologies with the inputs, outputs, capacities, bundles, construction turns and lifetimes the spec gives, in load order.
- A test setting with technologies across several files, in both TOML table forms, with goods defined in a file read after the technologies, an empty input table and an empty construction bundle, loads with the expected values and the same IDs on a second load.
- A test setting with every error (missing or non-table keys, a bad or duplicate name, amounts that are zero, negative, not finite or not numbers, construction turns or lifetime below 1 or not whole, an empty capital bundle, the output good among the inputs, unknown goods, unknown keys, a `[[technology]]` in `start/`) reports each error at its place with the exact message, including one entry with several problems reporting all of them.
- A setting with no technologies is valid. Reference errors in technologies are not reported when a file is broken or discovery was incomplete, like every other reference.
- Every existing test still passes: no existing setting, report or dev UI text changes except for the sample's new good and price.
