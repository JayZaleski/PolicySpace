# PolicySpace: Roadmap

## Stage 01: create-the-world
- Status: in-progress
- Goal: A setting folder loads into a Rust engine as hexes, goods, and natural resources. A minimal turn advances the clock, resources emit their goods, and unused goods perish. A developer UI shows the hex grid and the world's data, and steps turns.

### Step 01: engine-workspace
- Status: done
- Tags: #engine #rust
- Description: Set up the Rust workspace with an engine library crate and a dev-UI binary crate. The engine never depends on the UI. Also set up the core ID types and the test setup that later Steps build on.

### Step 02: setting-loader
- Status: specified
- Tags: #setting-data #engine
- Description: Define the setting folder layout and file format, split into persistent rules and starting state. Load a setting folder into the engine and report clear validation errors. Start a sample setting that later Steps extend.

### Step 03: goods-and-properties
- Status: planned
- Tags: #goods #setting-data
- Description: Define goods in setting data. Properties are named in data (perishes after N periods, movable), and the engine knows only the property names. Movable has no effect yet. Add goods to the sample setting.

### Step 04: hex-map
- Status: planned
- Tags: #hex-map #setting-data
- Description: Define hex tiles in the starting state, with only the data needed to place them on a grid. Terrain and other attributes come later. Add a map to the sample setting.

### Step 05: natural-resources
- Status: planned
- Tags: #natural-resources #goods #hex-map #setting-data
- Description: Define resource types in the rules (which unimproved good they produce and how much per period) and place deposits on hexes in the starting state. There is no ownership yet. Add deposits to the sample setting.

### Step 06: turn-clock
- Status: planned
- Tags: #turn-loop #goods #natural-resources
- Description: Add world state and the turn step: the clock advances, deposits emit their goods, and goods perish when they reach their period limit. Emitted goods are held somewhere that Stage 02 can replace with an owner. Where exactly is a design question for this Step.

### Step 07: dev-ui-map
- Status: planned
- Tags: #dev-ui #hex-map
- Description: Build a developer UI that loads a setting folder and draws its hex grid. It's a developer tool, not a player UI, and talks to the engine only through the engine's public interface.

### Step 08: dev-ui-inspect-and-step
- Status: planned
- Tags: #dev-ui #turn-loop
- Description: Let the developer inspect hexes, deposits, goods, and stockpiles in the dev UI, and step turns from it, so the minimal turn can be watched working.

## Stage 02: populate-the-world
- Status: outline
- Goal: Add POPs with utility functions, ownership, consumption, unskilled labor, and growth or decline. POPs own things, including natural resources, generate labor each turn, and consume against dummy data where industries and markets don't yet exist.

## Stage 03: industries
- Status: outline
- Goal: Add industries that combine goods, including labor and unimproved resource output, into other goods according to production functions, with technology, depreciation, operating budgets, and dividends.

## Stage 04: closed-market
- Status: outline
- Goal: Add currency, demand, supply, market power, and settlement rules in a single closed economy, replacing the dummy data from earlier Stages with live market outcomes.
