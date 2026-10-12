# PolicySpace: Vision

## What it is

PolicySpace is a niche Grand Strategy game built on top of a realistic simulation of economics and politics. The simulation is agent-based: actors (POPs, industries, States, and others) make their own decisions, and the behavior of the economy emerges from those decisions rather than from hand-tuned aggregates.

Emergent outcomes are the whole point. The final game experience is deliberately not defined up front: defining it generates rules that derail what the project is actually for. The game is built slice by slice, each slice adding one system to a working simulation, and the player experience grows out of what the simulation does.

## Architecture principles

- **The engine is written in Rust** and is setting-agnostic. It can host many settings: space, the 19th century, ancient times, and others. Everything that belongs to a setting lives in data, not code.
- **A setting is a folder of data files.** It fully defines a world: the map, the goods and their properties, the natural resources, the parameters of utility and production functions, and later technology trees, starting States, and more. It is organized into many files. It separates persistent rules (constants that never change during play) from the starting state (what turn 0 looks like).
- **The engine is independent of the UI/UX.** The UI can be swapped out, most likely with a different UI per setting. During development, a developer UI lets the developer load a setting, inspect the world, and step turns. It is not a player UI.
- **Any actor is in principle playable**, but States are the intended player role.
- **Time is turn-based.** Each turn, actors act and the world updates.
- **Non-player actors use closed-form decision rules first.** LLMs and SystemOne AI models will plug into the engine later to control non-player actors, but they do not enter the picture until the closed-form simulation works.

## Building principles

- **Dummy data now, live data later.** When a system needs a system that isn't built yet, its actors respond to static dummy data. When the later system arrives, the dummy data is replaced with live simulation data. For example, POPs consume against dummy prices until markets exist. Each Stage leaves clean seams where its stubs will be replaced.
- **Systems stay simpler than they sound** until a later system needs more. POPs generate unskilled labor each turn long before anyone trades it.

## Core concepts

### Goods

A good is anything an actor can own. This is deliberately abstract: skilled and unskilled labor, intermediate goods, consumption goods, and the output of natural resources are all goods. A good does not produce anything by itself.

Goods are not pre-categorized as intermediate, consumption, or capital goods. A good can serve several roles; what a good is used for is decided by the utility functions and production functions that name it.

Each good has **properties** that define how it behaves. Properties are data in the setting folder; the engine knows only the property names. The first properties are:

- **Perishes after N periods.** Labor and unimproved resource output perish after one period. Durable goods never perish.
- **Movable or fixed in place.** This property exists from the start but has no effect until movement and trade between places are built, after the Closed Market Stage.

Divisibility (whole units versus continuous quantities) is not needed yet.

The purpose of one unified notion of a good is that labor, resource output, intermediates, and consumption goods can all be exchanged in the same market system, without designing a separate system for each type.

### Natural resources

A natural resource (a copper deposit, arable land) sits on a hex and produces a fixed amount of a related *unimproved* good each period for its owner. That good cannot be moved, and it perishes at the end of the period if unused. This models unimproved resources: an owner of a copper deposit receives unimproved copper each period but must combine it with labor and capital to extract and use it, or rent the resource to someone who will. Resource exhaustion may be added later.

### Utility and production functions

Utility functions (POPs) and production functions (industries) name goods directly. Their parameters are defined in the setting folder. Each POP has its own utility function parameters, or is assigned a set from a number of possibilities defined in the setting.

## Stages

The roadmap currently stops at the Closed Market Stage. Later directions are listed below without order or commitment.

### Stage 1: Create the World

Setting files define persistent parameters and starting state. The world has hex tiles, natural resources on those tiles, and goods with their properties. A developer UI displays the hexes on a grid and the world's data, and steps turns.

- Hexes need only the data to display them on a grid. Position does not matter yet; terrain and similar attributes come later, when movement and trade need them.
- There is no ownership yet. It arrives with POPs in Stage 2.
- A minimal turn exists from the start: the clock steps, resources emit their goods, and unused goods perish. Every later Stage adds behavior to an already-ticking clock.

### Stage 2: Populate the World

POPs, utility functions, ownership, consumption, unskilled labor, and growth/decline. POPs own things (including natural resources), generate unskilled labor each turn, and consume according to their utility functions, against dummy data where industries and markets don't yet exist.

### Stage 3: Industries

Technology, production, depreciation, operating budgets, and dividends. Industries combine goods (including labor and unimproved resource output) into other goods according to production functions.

### Stage 4: Closed Market

Currency, demand, supply, market power, and settlement rules. A single closed economy with no external trade. The dummy data from earlier Stages is replaced by live market outcomes, bringing POPs, industries, and the market together.

## Beyond Stage 4

Directions the project will take later, in no particular order:

- States as actors: budgets, taxes, spending, and the first policy levers.
- Politics: how POPs form interests and pressure the State.
- Terrain, movement, and trade between places; open markets between economies.
- Technology trees and starting States in the setting folder.
- A player UI, and saving and loading.
- LLM and SystemOne AI models controlling non-player actors.
