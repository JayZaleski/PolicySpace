# PolicySpace: Vision

## What it is

PolicySpace is a niche Grand Strategy game built on top of a realistic simulation of economics and politics. The simulation is agent-based: actors (POPs, businesses, industries, States, and others) make their own decisions, and the behavior of the economy emerges from those decisions rather than from hand-tuned aggregates.

Emergent outcomes are the whole point. The final game experience is deliberately not defined up front: defining it generates rules that derail what the project is actually for. The game is built slice by slice, each slice adding one system to a working simulation, and the player experience grows out of what the simulation does.

## Milestones

The first milestone is a **closed market**: a self-contained economy on each hex with a functioning market, saving, and investment. It is reached when the Saving and Investment Stage is complete, and it is tested on a small number of dense hexes, each with many deposits, POPs, and industries. Trade and migration come after that; they make hexes much less dense and bring space into the picture. Politics and policy are a later milestone still.

## Architecture principles

- **The engine is written in Rust** and is setting-agnostic. It can host many settings: space, the 19th century, ancient times, and others. Everything that belongs to a setting lives in data, not code.
- **A setting is a folder of data files.** It fully defines a world: the map, the goods and their properties, the natural resources, the parameters of utility and production functions, and later technology trees, starting States, and more. It is organized into many files. It separates persistent rules (constants that never change during play) from the starting state (what turn 0 looks like).
- **The engine is independent of the UI/UX.** The UI can be swapped out, most likely with a different UI per setting. During development, a developer UI lets the developer load a setting, inspect the world, and step turns. It is not a player UI.
- **Any actor is in principle playable**, but States are the intended player role.
- **Time is turn-based.** Each turn, actors act and the world updates.
- **Passive actors optimize; businesses decide.** POPs, natural resources, and industries are passive: each turn, each one optimizes a fixed objective (a utility function, a production function) with closed-form rules, given the information it has. Their behavior changes with their circumstances, but the underlying form is generic. Businesses hold the open-ended, discretionary decisions that don't reduce to a generic form. Making the simulation more nuanced is then a matter of making business decisions richer, and businesses are the first place LLMs and SystemOne AI models plug in. Those models do not enter the picture until the closed-form simulation works.

## Building principles

- **Dummy data now, live data later.** When a system needs a system that isn't built yet, its actors respond to static dummy data. When the later system arrives, the dummy data is replaced with live simulation data. For example, POPs and industries trade with a dummy market that posts fixed prices until the real market exists. The dummy market is the live market with two things missing: its prices never adjust, and it buys and sells any amount. Each Stage leaves clean seams where its stubs will be replaced.
- **Systems stay simpler than they sound** until a later system needs more. POPs sell unskilled labor to a dummy market long before anyone else demands it.

## Core concepts

### Goods

A good is anything an actor can own. This is deliberately abstract: skilled and unskilled labor, intermediate goods, consumption goods, currency, and the output of natural resources are all goods. A good does not produce anything by itself.

Goods are not pre-categorized as intermediate, consumption, or capital goods. A good can serve several roles; what a good is used for is decided by the utility functions and production functions that name it.

Each good has **properties** that define how it behaves. Properties are data in the setting folder; the engine knows only the property names. The first properties are:

- **Perishes after N periods.** Labor and unimproved resource output perish after one period. Durable goods never perish.
- **Movable or fixed in place.** This property exists from the start but has no effect until movement and trade between places are built, after the Market Stage.

Divisibility (whole units versus continuous quantities) is not needed yet.

The purpose of one unified notion of a good is that labor, resource output, intermediates, consumption goods, and currency can all be exchanged in the same market system, without designing a separate system for each type.

### Currency

Currency is a good designated as the one prices are posted in. It is held and exchanged like any other good. Each hex has its own designated currency, and the designation can change during the simulation, so it belongs to the starting state, not the persistent rules. The designated good can be a commodity, or later a synthetic good issued by a central bank, so the simulation can handle commodity and fiat money regimes alike. Until a central bank exists, currency is simply an in-game good that prices are quoted in.

### Natural resources

A natural resource (a copper deposit, arable land) sits on a hex and produces a fixed amount of a related *unimproved* good each period. That good cannot be moved, and it perishes at the end of the period if unused. This models unimproved resources: whoever holds a copper deposit receives unimproved copper each period but must combine it with labor and capital to extract and use it, or rent the resource to someone who will. Deposits have no owner until the Businesses Stage. Resource exhaustion may be added later.

### POPs

A POP is a group of N identical individuals living on a hex. Each individual produces a fixed amount of unskilled labor per turn and the POP has a fixed Stone-Geary utility function that names the goods it consumes, including leisure. Leisure is the POP consuming its own labor rather than selling it, so labor supply and demand for goods both come out of the same utility maximization. A POP's utility parameters are its own, or are assigned from a set of possibilities the setting defines.

A POP grows or declines gradually. Its growth rate is a function of its utility (probably a polynomial): zero when it is exactly at subsistence, positive above, negative below. A POP whose income can't cover its subsistence bundle spends everything it has, buying toward subsistence, and shrinks; it doesn't die at once. Growth changes N; the per-individual parameters don't change.

POPs own their inventory of goods, including currency and their own labor, from the start. They own businesses, and through them deposits and industries, only from the Saving and Investment Stage.

### Industries

An industry combines goods (including labor and unimproved resource output) into other goods according to a production function defined in the setting. It is passive: it optimizes its production function against the prices it sees. Industries have no owner until the Businesses Stage.

### Businesses

A business is the layer between POPs and the things they ultimately own. POPs own businesses, and businesses own natural resources and industries. Businesses hold the planning and strategic decisions, such as vertical and horizontal expansion, that are hard to reduce to a generic form. In their first version they are simple holders that collect what their industries and deposits earn, set operating budgets, and pay dividends. Expansion decisions come with the Saving and Investment Stage.

### Utility and production functions

Utility functions (POPs) and production functions (industries) name goods directly. Their parameters are defined in the setting folder.

### Markets

Each hex has one market, and every exchange of goods on the hex goes through it. The market works by **submission and aggregation**, not by search:

- **Actors submit parameters, not answers.** Each actor hands the market its demand and supply in one of a small set of forms the market knows how to add up. POPs submit their Stone-Geary parameters and their endowment. Industries submit their production parameters and capacity. Businesses, whose decisions are discretionary, submit the result of those decisions as a curve: a fixed quantity, a reservation price, or a markup. The market never asks an actor "how much would you want at this price?", so the cost of a turn grows with the number of actors only through one pass over them, and the market's own work grows with the number of goods.
- **The market aggregates exactly.** Stone-Geary demand is linear in income, so every POP of one type on a hex collapses into a single demand function with their sizes summed, and the aggregate demand and supply across types has a closed form. Where industries make supply nonlinear in prices, the market works on the aggregate curves, never on individual actors.
- **Prices are fixed within a turn and adjust between turns.** At the start of trading the market posts one price per good in the hex's currency. Actors trade at those prices. After trading, the market uses the aggregate curves to compute where each price would have cleared and moves next turn's price part of the way toward it, at an adjustment speed set in the setting. It never iterates to equilibrium within a turn.
- **Disequilibrium is real and persists.** When bids and offers at the posted price don't match, a rationing rule decides who trades and how much (proportionally, by default). Unsold durable goods are inventory, unsold labor perishes as unemployment, and unfilled bids are shortages. All three are visible quantities in the simulation. Gluts, shortages, and price cycles are emergent, and with constant fundamentals prices converge over turns to the equilibrium.
- **Settlement order matters.** An actor that is rationed when selling has less income than it planned. Sales settle before purchases, and currency left unspent, because a good was short or by rounding, carries over as income next turn.
- **Policy lives in the market.** Taxes and subsidies, once States exist, are wedges between the price buyers pay and the price sellers receive, applied to the aggregate curves. The market applies the wedge, solves, and reports the revenue. Policy changes the market's inputs, not the actors' code.
- **Market power comes later.** The first live market is price-taking. Its natural extension is to let sellers post their own prices instead of taking the market's, which gives businesses the lever to withhold supply or mark up.

## Stages

The roadmap currently runs to the Saving and Investment Stage. Later directions are listed below without order or commitment.

### Stage 1: Create the World

Setting files define persistent parameters and starting state. The world has hex tiles, natural resources on those tiles, and goods with their properties. A developer UI displays the hexes on a grid and the world's data, and steps turns.

- Hexes need only the data to display them on a grid. Position does not matter yet; terrain and similar attributes come later, when movement and trade need them.
- There is no ownership yet.
- A minimal turn exists from the start: the clock steps, resources emit their goods, and unused goods perish. Every later Stage adds behavior to an already-ticking clock.

### Stage 2: Populate the World

POPs with Stone-Geary utility functions that include leisure, unskilled labor, consumption, and growth or decline. POPs trade with a dummy market that posts fixed prices in the hex's designated currency good: they sell labor into it and buy goods from it, and their supply and demand follow from their utility functions. A POP's trade is structured as the live market expects: the POP's parameters and endowment are its submission, prices are posted, then the POP trades at those prices. They start with no cash and spend their whole income every turn; currency left over by rounding carries over as next turn's income, and saving comes later. Stone-Geary demand is closed form, so no approximations are needed; simplified heuristics (budget shares, share of labor sold) are an option later if the full rules turn out to be slow.

POPs own their own goods, but there is no ownership of deposits or industries in this Stage. Deposits keep emitting into unowned stockpiles.

### Stage 3: Industries

Technology, production, and depreciation. Industries combine goods, including labor and unimproved resource output, into other goods according to production functions, buying inputs from and selling output to the dummy market. They have no owners, budgets, or dividends yet.

### Stage 4: Businesses

Ownership arrives. Businesses own deposits and industries, and POPs own businesses. Businesses collect their industries' and deposits' earnings, set operating budgets, and pay dividends. They are simple in this Stage; expansion decisions wait for Saving and Investment.

### Stage 5: Market

The live market described under [Markets](#markets), in a single closed economy with no external trade. It brings POPs, industries, and businesses together: the dummy market's infinite counterparty is removed, bids and offers are settled against each other with rationing, and prices adjust between turns toward clearing using the aggregate curves. Inventory, unemployment, and shortages appear. The test of correctness is that, with constant fundamentals, prices converge over turns to the known equilibrium. Market power is not part of this Stage.

### Stage 6: Saving and Investment

POPs own shares of businesses and receive dividends. POPs do financial planning on data the businesses provide: deposit at banks for interest, buy and sell stock. Businesses make expansion decisions, vertical and horizontal. POPs make human capital decisions that let them exchange unskilled labor for labor of different skills. Completing this Stage reaches the closed-market milestone.

## Beyond Stage 6

Directions the project will take later, in no particular order:

- A central bank and a synthetic currency, so fiat money regimes can be simulated alongside commodity money.
- States as actors: budgets, taxes, spending, and the first policy levers. Taxes and subsidies are modelled in the markets, as wedges on the aggregate curves.
- Politics: how POPs form interests and pressure the State.
- Terrain, movement, and trade between places; open markets between economies. Hexes become much less dense when space matters.
- Technology trees and starting States in the setting folder.
- A player UI, and saving and loading.
- LLM and SystemOne AI models controlling businesses first, then other non-player actors.
- Resource exhaustion.
- Market power: sellers posting their own prices instead of taking the market's.
