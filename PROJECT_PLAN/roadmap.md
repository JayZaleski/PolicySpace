# PolicySpace: Roadmap

## Stage 01: create-the-world
- Status: done
- Goal: A setting folder loads into a Rust engine as hexes, goods, and natural resources. A minimal turn advances the clock, resources emit their goods, and unused goods perish. A developer UI shows the hex grid and the world's data, and steps turns.

### Step 01: engine-workspace
- Status: done
- Tags: #engine #rust
- Description: Set up the Rust workspace with an engine library crate and a dev-UI binary crate. The engine never depends on the UI. Also set up the core ID types and the test setup that later Steps build on.

### Step 02: setting-loader
- Status: done
- Tags: #setting-data #engine
- Description: Define the setting folder layout and file format, split into persistent rules and starting state. Load a setting folder into the engine and report clear validation errors. Start a sample setting that later Steps extend.

### Step 03: goods-and-properties
- Status: done
- Tags: #goods #setting-data
- Description: Define goods in setting data. Properties are named in data (perishes after N periods, movable), and the engine knows only the property names. Movable has no effect yet. Add goods to the sample setting. Carry-over from Step 02: reference checking must also be skipped when discovery couldn't see some data files (missing folder, unreadable subfolder, bad file name), not only when a file is broken.

### Step 04: hex-map
- Status: done
- Tags: #hex-map #setting-data
- Description: Define hex tiles in the starting state, with only the data needed to place them on a grid. Terrain and other attributes come later. Add a map to the sample setting.

### Step 05: natural-resources
- Status: done
- Tags: #natural-resources #goods #hex-map #setting-data
- Description: Define resource types in the rules (which unimproved good they produce, and how much one deposit outputs per period) and place whole numbers of deposits on hexes in the starting state. Each deposit is a separate unit for a single future owner. There is no ownership, depletion, or regeneration yet. Add deposits to the sample setting.

### Step 06: turn-clock
- Status: done
- Tags: #turn-loop #goods #natural-resources
- Description: Add world state and the turn step: the clock advances, deposits emit their goods, and goods perish when they reach their period limit. Emitted goods are held somewhere that Stage 02 can replace with an owner. Where exactly is a design question for this Step.

### Step 07: dev-ui-map
- Status: done
- Tags: #dev-ui #hex-map
- Description: Build a developer UI that loads a setting folder and draws its hex grid. It's a developer tool, not a player UI, and talks to the engine only through the engine's public interface.

### Step 08: dev-ui-inspect-and-step
- Status: done
- Tags: #dev-ui #turn-loop
- Description: Let the developer inspect hexes, deposits, goods, and stockpiles in the dev UI, and step turns from it, so the minimal turn can be watched working.

## Stage 02: populate-the-world
- Status: done
- Goal: POPs live on hexes, sell unskilled labor to a dummy market and buy goods from it at fixed prices posted in the hex's currency good, consume according to Stone-Geary utility with leisure, and grow or shrink with their utility. POPs own their own goods only; deposits keep emitting into unowned stockpiles.

### Step 01: currency
- Status: done
- Tags: #currency #setting-data #hex-map
- Description: Designate one good per hex as its currency, in the starting state. Decide whether every hex must have one and what the designated good may be. The designation is state, not a rule, because it can change during the simulation. Add a currency good to the sample setting.

### Step 02: dummy-market
- Status: done
- Tags: #market #dummy-data #setting-data #currency
- Description: A per-hex dummy market that posts fixed prices in the hex's currency and buys and sells any amount at those prices. Decide where the dummy prices live in the setting folder. Add the first way to take goods out of a stockpile, including which perish group goes first. This is the seam Stage 05 replaces with the live market.

### Step 03: pop-types
- Status: done
- Tags: #pops #utility #setting-data #goods
- Description: Define POP types in the rules: the endowment each individual receives per turn (any goods, intended to be only unskilled labor; other goods exist for testing), and the Stone-Geary parameters (subsistence and share per named good, shares normalized by the engine). Leisure is an endowed good appearing in the utility function, so it needs no special case. Validate the parameters and check their references to goods. Add POP types to the sample setting.

### Step 04: pops
- Status: done
- Tags: #pops #setting-data #turn-loop
- Description: Place POPs in the starting state: hex, POP type, and size N. Each POP owns its own stockpile of goods. At the start of each turn every POP's endowment is emitted into its stockpile. POPs start with no cash. Add POPs to the sample setting.

### Step 05: stone-geary-demand
- Status: done
- Tags: #pops #utility #engine
- Description: The closed-form Stone-Geary demand: given prices and the endowment, the quantity of each good a POP wants, including the endowed goods it keeps as leisure. Includes the below-subsistence rule: a POP whose full income can't cover its subsistence bundle spends everything, buying toward subsistence. A pure function with its own tests, not yet wired into the turn.

### Step 06: pop-trade
- Status: done
- Tags: #pops #market #turn-loop
- Description: The first behavior in the "actors act" part of the turn, shaped as the live market expects. Each POP hands the hex's market a submission: its Stone-Geary parameters and its endowment. The market posts its prices (fixed, from the dummy data). Demand is computed from the submission against those prices; the POP sells the endowment it doesn't keep as leisure, then buys goods with the proceeds, so sales settle before purchases. The POP then consumes what it demanded. Currency left over by rounding stays in the stockpile and is income next turn. The turn report gains POP totals. Design question: whether the market or the POP computes demand from the submission (leaning market-side, so Stage 05 replaces market internals only).

### Step 07: pop-growth
- Status: done
- Tags: #pops #turn-loop #setting-data
- Description: Each POP's size N grows or shrinks by a rate that is a function of its utility: zero at subsistence, positive above, negative below. The function's parameters live in the setting. Per-individual parameters don't change; next turn's endowment follows the new N.

### Step 08: dev-ui-pops
- Status: done
- Tags: #dev-ui #pops #currency #market
- Description: Inspect a hex's currency, its posted prices, and its POPs in the dev UI: each POP's type, size, inventory, last turn's submission and trade outcome (endowment sold, goods bought and consumed, currency carried over), utility, and growth. Show the POP totals in the turn log.

## Stage 03: factories
- Status: in-progress
- Goal: Technologies, factories, depreciation, and the minimal business. Factories exist in the starting state with a technology and a capital bundle, consume a slice of it each turn and shut down when it's gone. Minimal businesses own sets of factories and deposits, start with cash, trade fixed quantities with the dummy market on a one-turn lag, and maintain the factories that pay for it. Owned deposits emit into their business's stockpile; unowned ones keep emitting into hex stockpiles. The sample setting gets a real production chain on one hex. No POP ownership, budgets, dividends, construction, scrapping, or retooling yet.

### Step 01: technologies
- Status: done
- Tags: #technologies #factories #setting-data #goods
- Description: Define technologies in the rules: a fixed-proportion recipe (so many units of each input per unit of output, no substitution), the output good, capacity per turn, the capital bundle of goods a factory of this technology holds, its lifetime in turns, and the labor maintenance takes. Validate the numbers and check the references to goods. Add a production chain to the sample setting (farm, mine, toolmaker) with a capital good for the bundles. Transitions between technologies and construction parameters wait for Stage 07.

### Step 02: businesses
- Status: done
- Tags: #businesses #natural-resources #setting-data #turn-loop #deciders
- Description: Define minimal businesses in the starting state: starting cash, the deposits the business owns, and the name of the decider that controls it (a name only; Step 07 resolves it). Each business gets an ID and one stockpile that holds its goods and cash. Rework emission so an owned deposit emits into its owner's stockpile while unowned deposits keep emitting into the hex stockpile; this replaces the ownership seam Stage 01 left in emission. Design question: whether a business is tied to one hex and may own only assets on it (expected yes, until trade exists). Add businesses to the sample setting.

### Step 03: factories
- Status: done
- Tags: #factories #businesses #setting-data #hex-map
- Description: Place factories in the starting state: hex, technology, owner business, and the current capital bundle. Each factory gets an ID, points at one technology (state, so it can change later), and has exactly one owner. Validate the references and the bundle. Add the chain's factories to the sample setting.

### Step 04: factory-mechanics
- Status: done
- Tags: #factories #production #capital #engine
- Description: Pure functions with their own tests, not yet wired into the turn. Running a factory at a given level: the level is clamped to the technology's capacity and to the inputs the business has on hand, inputs leave the business's stockpile and output enters it. Depreciation: each turn a factory consumes its capital bundle divided by its technology's lifetime, in the bundle's proportions, whether or not it ran, and shuts down when the bundle is exhausted. Maintenance: replenishing a share of the bundle costs that share of the capital and construction bundles, capped at one construction slice per turn. The choice of run level and maintenance share is not made here: it belongs to the business's decider (Step 06).

### Step 05: business-view-and-decisions
- Status: done
- Tags: #deciders #businesses #engine
- Description: The decider protocol as engine data, sized to the minimal business. The view a business is served each turn: its hex's posted prices, its stockpile and cash, its factories with their technology's numbers and bundle state, and its deposits; nothing outside its circumstances. The action vocabulary: per-factory run level and maintenance share, offers, and bids, each with the bounds the engine enforces, and an optional trace. Validation of a decision against the view it answers. Lossless serialization of both, with no references into the engine, and a recording format for a turn's batch of views and decisions. No Python yet; everything has its own tests.

### Step 06: business-script
- Status: specified
- Tags: #deciders #businesses #python
- Description: The project's Python package: a client that reads a batch of views and writes a batch of decisions in Step 05's format, and the first business decider, the arithmetic rule: a factory runs at capacity and is maintained when the margin at posted prices covers inputs and maintenance, and is otherwise left to run down; offers of last turn's output and surplus unimproved goods, bids for the inputs and maintenance that implies. The rule works over whole batches, not one business at a time. Python tests against recorded views, run from the project's virtual environment.

### Step 07: decider-host
- Status: planned
- Tags: #deciders #turn-loop #engine #python
- Description: The decision point in the engine: once per turn, every business's view is served and every decision collected in one batched exchange. A Decider abstraction with two implementations: the external-process decider, which starts the Python script once, keeps it alive across turns, and exchanges one batch per turn; and the replay decider, which answers from a recording so engine tests that don't test strategy don't need Python. Failure semantics: what the engine does when a decider errors, answers late, or sends a decision it can't resolve. Resolving the decider names businesses give in the setting, and the test setup that finds the project's Python environment. The engine never depends on a decider's memory.

### Step 08: business-trade
- Status: planned
- Tags: #businesses #factories #market #turn-loop #deciders
- Description: The business's turn. After the decision point, each business's offers and bids go through the same hex round as POPs: sales settle before purchases, purchases are capped by cash on hand after sales, and when cash is short the run level is scaled down. Then its factories run at the resolved levels, depreciate, and are maintained by the chosen shares, and the output goes to inventory for next turn. Labor is bought and used in the same turn. A per-business outcome records the decision, its trace, and what was traded, and the turn report gains business totals. The cash cap is the seam for operating budgets; maintenance is the seam for expansion; the decider is the seam for richer strategy.

### Step 09: dev-ui-businesses
- Status: planned
- Tags: #dev-ui #businesses #factories #deciders
- Description: Inspect a hex's businesses in the dev UI: cash, stockpile, owned deposits, decider, last turn's decision with its trace, and the trade outcome. Inspect its factories: technology, capital bundle and remaining life, run level, and whether it has shut down. Show the business totals in the turn log. Taking over a business and submitting its decisions by hand waits for Stage 05.

## Stage 04: market
- Status: outline
- Goal: The live market of the vision's Markets section, in a single closed economy with no external trade. It brings POPs, factories, and minimal businesses together: the dummy market's infinite counterparty is removed, bids and offers are settled against each other with proportional rationing, sales before purchases, and prices adjust between turns toward clearing using the aggregate curves at a speed set in the setting. Inventory, unemployment, and shortages become visible quantities. The test of correctness is that, with constant fundamentals, prices converge over turns to the known equilibrium. Market power is not part of this Stage.

## Stage 05: dev-ui
- Status: outline
- Goal: Once the market makes the numbers worth looking at, the developer UI gets a Stage of its own. It decides what the dev UI should be good for from now on (an inspector of any object in the world, a way to watch the economy move over turns, or both), how UI back-and-forths run inside the development loop when the developer sees something wrong and the agent fixes it, and how the agent itself can see and test the UI so UI work can be built and reviewed like everything else. The dev UI becomes a decider: it can take over any discretionary actor and submit its decisions by hand, the first proof of the player path.

## Stage 06: businesses
- Status: outline
- Goal: Ownership arrives in full. POPs own businesses; businesses already own deposits and factories. Businesses collect their factories' and deposits' earnings, set operating budgets, and pay dividends. Budgets and dividends are new verbs in the decider vocabulary, resolved by engine rules, not new engine decisions. Expansion decisions wait for Stage 07.

## Stage 07: saving-and-investment
- Status: outline
- Goal: POPs own shares of businesses and receive dividends, deposit at banks for interest, and buy and sell stock. Businesses make vertical and horizontal expansion decisions through their deciders: constructing factories, retooling them to other technologies, scrapping them, and buying and selling them. POPs make human capital decisions that exchange unskilled labor for skilled labor. Completing this Stage reaches the closed-market milestone.
