# Step 03: factories

## Summary
This Step places factories in the starting state. A factory is a discrete asset on a hex that runs one technology and belongs to exactly one business. It carries the state that changes during play: which technology it runs, and how much of its technology's capital bundle it still holds. The loader validates the references and the numbers, gives each factory an ID, and the World takes the factories over as state. Nothing runs yet: production, depreciation and maintenance are Step 04's arithmetic, and Step 08 wires them into the turn. The sample setting gains the chain's three factories on the center hex.

## Behavior

### What a factory is
A factory is state, defined by `[[factory]]` entries in `start/` (one in `rules/` gets the usual "belongs in start/" error). Factories are unnamed, like deposits and POPs: nothing refers to a factory by name. Deciders address them by ID in the view (Step 05), the dev UI shows them under their business (Step 09), and later decisions about a factory happen in play, by ID. A name would add a namespace and a uniqueness check for nothing that uses them; one can be added if a later Step needs it. Factory IDs follow load order, so the same folder always gives the same IDs.

A factory has:

- **A hex**, given by axial coordinates read exactly as a deposit's, POP's or business's. The hex is the factory's own, not derived from its owner, because businesses will operate across several hexes once trade between hexes exists. Until then a business may own only assets on its own hex, as Step 02 decided for deposits, so the loader rejects a factory whose hex isn't its owner's hex. That check is to be relaxed when trade arrives; writing the coordinates now means the data doesn't change then.
- **A technology**, by name. The pointer is state, so a later Stage can retool the factory to another technology, but this Step never changes it.
- **An owner**, a business by name. Exactly one, and required: a factory without an owner is not a thing this project has (every factory has a business owner from this Stage on).
- **A condition**: the share of its technology's full capital bundle the factory holds, a number greater than 0 and at most 1. It is optional, and a factory with no condition written is full (1), the common case. The capital is written as one number rather than a table of goods because Step 01's rules only ever change the bundle in its own proportions: depreciation takes a slice of the whole bundle, and maintenance replenishes a share of the whole bundle. So a factory's capital is always some share of its technology's bundle, the share is the only real state, and a table of goods could only express states the mechanics never produce. The engine derives the goods in the factory from the technology's capital bundle and the condition whenever it needs them. A condition of exactly 0 is rejected in the setting: a factory that starts exhausted isn't a case worth placing, and what an exhausted factory is (shut down, and whether maintenance can revive it) is Step 04's question. Once Step 04 runs depreciation, the engine's state may hold 0.
- **A count**, optional, default 1: the entry places that many identical factories (same hex, technology, owner and condition), each with its own ID, in the same range as a deposit's count. Capacity belongs to the technology, so a business that wants more output needs more factories, and those are usually identical; one entry per factory would be most tedious exactly where factories multiply.

A hex may hold any number of factories, and a business any number of factories, of any technologies. Nothing checks that a factory makes economic sense: that its inputs can be had on the hex, that the owner owns matching deposits, or that the technology is maintainable. The engine doesn't police economics.

### Validation
Every key is read before anything is decided, so one entry with several problems reports all of them in one load. Errors, each at its place: a missing key; a bad, missing or out-of-range coordinate; coordinates that match no hex; an unknown technology; an unknown business; a hex that isn't the owner's hex; a condition that isn't a number, or is zero, negative, above 1, infinite or NaN; a count that isn't a whole number or is outside the deposit count's range; an unknown key. The technology, the owner and the hex are references, resolved after every file has been read, so they may be defined in any file; like every reference check, they are skipped when the loader couldn't see every file. The hex-is-the-owner's check needs both the hex and the owner to have resolved, and is reported at the entry. A bad condition or count doesn't stop the reference checks. A setting with no factories is valid.

### In the engine
Factories are a list in the starting state, like deposits: each has an ID, a hex, a technology, an owner and a condition. The World takes the list over as state, because the technology and the condition change in later Steps. Through the World a caller can read every factory in ID order, the factories on a hex, and the factories a business owns, each in ID order. Nothing in the turn touches factories yet: they don't run, wear, or appear in the report. The checker's success line counts factories.

### Sample setting
One factory per business on the center hex, each running the technology its business is named for: the farm business's factory runs the farm, the mine business's the mine, and the toolmaker business's the toolmaker. At least one of them is placed below full condition, so the key is exercised and Step 04 has a worn factory from turn 0. The deposits on other hexes get no factory. The exact numbers are for the spec. The file's header comment explains every key, including what the condition means, in the style of the other sample files.

## Player-facing Changes
None in the UI. Setting authors get a new section, `[[factory]]`, in `start/`, with clear errors. The sample setting gains `start/factories.toml`. The checker's success line counts factories. Inspecting factories in the dev UI is Step 09.

## Interactions
- **Technologies** (Step 01): a factory names its technology, and its capital is a share of that technology's capital bundle.
- **Businesses** (Step 02): a factory names its owner, and must sit on its owner's hex until trade exists. Factories are the first thing that refers to a business by name, the reason businesses were named.
- **Deposits** (Stage 01 Step 05): the model for an unnamed, placed asset with an optional count, a per-hex index and an owner; factories reuse the count's rules and range.
- **Setting loader** (Stage 01 Step 02) and **POPs** (Stage 02 Step 04): coordinates and the "no hex at" error, every key read before anything is decided, references resolved after every file has been read.
- **World and turn** (Stage 01 Step 06): the factory list becomes world state; the turn is unchanged.
- **Later Steps:** Step 04 (factory mechanics) runs, depreciates and maintains a factory from its technology and condition. Step 05 serves each business its factories in the view, by ID. Step 08 wires the mechanics into the turn. Step 09 shows factories in the dev UI. Stage 07 changes the technology pointer, builds, scraps and sells factories.

## Out of Scope
- Running a factory, depreciation, maintenance, shutting down and what an exhausted factory is: Step 04.
- Factories under construction, scrapping, retooling, buying and selling factories: Stage 07.
- Factories without an owner, or on a hex other than their owner's.
- Any check that a factory is economically sensible.
- Any change to a factory during play. Any turn behavior or report change.
- Showing factories in the dev UI: Step 09.

## Acceptance Criteria
- The sample setting loads with three factories on the center hex, one per business with the expected technology, owner and condition, in load order, and a fresh sample world holds them unchanged after any number of turns; two worlds from the same setting stepped the same number of times are identical.
- A test setting with factories across several files, an entry with a count above 1 giving that many factories with consecutive IDs, a factory with no condition written (full), one with a condition below 1, two businesses on one hex each owning factories of different technologies, and references to a business and a hex defined in later files (technologies are rules, so they are always read first) loads with the expected values and the same IDs on a second load; the World answers the factories on each hex and owned by each business in ID order.
- Each error case produces one error at the expected place with the expected text: every missing key, bad coordinates and coordinates with no hex, an unknown technology, an unknown business, a factory on a hex that isn't its owner's, a condition that isn't a number or is zero, negative, above 1, infinite or NaN, a count that isn't a whole number or is out of range, an unknown key, and a `[[factory]]` in `rules/`. One entry with several problems reports all of them in one load.
- Reference checks, including the hex-is-the-owner's check, are skipped when another file is broken or discovery was incomplete.
- The checker's success line counts factories, zero for settings without them. Every existing test passes, with expectations changed only by the added factory count.
