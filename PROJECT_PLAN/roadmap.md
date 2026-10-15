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
- Status: in-progress
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
- Status: specified
- Tags: #pops #setting-data #turn-loop
- Description: Place POPs in the starting state: hex, POP type, and size N. Each POP owns its own stockpile of goods. At the start of each turn every POP's endowment is emitted into its stockpile. POPs start with no cash. Add POPs to the sample setting.

### Step 05: stone-geary-demand
- Status: planned
- Tags: #pops #utility #engine
- Description: The closed-form Stone-Geary demand: given prices and the endowment, the quantity of each good a POP wants, including the endowed goods it keeps as leisure. Includes the below-subsistence rule: a POP whose full income can't cover its subsistence bundle spends everything, buying toward subsistence. A pure function with its own tests, not yet wired into the turn.

### Step 06: pop-trade
- Status: planned
- Tags: #pops #market #turn-loop
- Description: The first behavior in the "actors act" part of the turn: each POP computes its demand against the dummy market's prices, sells the endowment it doesn't keep as leisure, buys goods with the proceeds, and consumes them. The turn report gains POP totals.

### Step 07: pop-growth
- Status: planned
- Tags: #pops #turn-loop #setting-data
- Description: Each POP's size N grows or shrinks by a rate that is a function of its utility: zero at subsistence, positive above, negative below. The function's parameters live in the setting. Per-individual parameters don't change; next turn's endowment follows the new N.

### Step 08: dev-ui-pops
- Status: planned
- Tags: #dev-ui #pops #currency #market
- Description: Inspect a hex's currency, its posted prices, and its POPs in the dev UI: each POP's type, size, inventory, last turn's endowment sold, goods bought and consumed, utility, and growth. Show the POP totals in the turn log.

## Stage 03: industries
- Status: outline
- Goal: Add industries that combine goods, including labor and unimproved resource output, into other goods according to production functions, with technology and depreciation. Industries buy inputs from and sell output to the dummy market. They have no owners, budgets, or dividends yet.

## Stage 04: businesses
- Status: outline
- Goal: Ownership arrives. Businesses own deposits and industries, and POPs own businesses. Businesses collect their industries' and deposits' earnings, set operating budgets, and pay dividends. Expansion decisions wait for Stage 06.

## Stage 05: market
- Status: outline
- Goal: Demand, supply, market power, and settlement rules in a single closed economy with no external trade. The dummy market is replaced by live market outcomes, bringing POPs, industries, and businesses together.

## Stage 06: saving-and-investment
- Status: outline
- Goal: POPs own shares of businesses and receive dividends, deposit at banks for interest, and buy and sell stock. Businesses make vertical and horizontal expansion decisions. POPs make human capital decisions that exchange unskilled labor for skilled labor. Completing this Stage reaches the closed-market milestone.
