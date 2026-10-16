# PolicySpace: Vision

## What it is

PolicySpace is a niche Grand Strategy game built on top of a realistic simulation of economics and politics. The simulation is agent-based: actors (POPs, businesses, factories, States, and others) make their own decisions, and the behavior of the economy emerges from those decisions rather than from hand-tuned aggregates.

Emergent outcomes are the whole point. The final game experience is deliberately not defined up front: defining it generates rules that derail what the project is actually for. The game is built slice by slice, each slice adding one system to a working simulation, and the player experience grows out of what the simulation does.

## Milestones

The first milestone is a **closed market**: a self-contained economy on each hex with a functioning market, saving, and investment. It is reached when the Saving and Investment Stage is complete, and it is tested on a small number of dense hexes, each with many deposits, POPs, and factories. Trade and migration come after that; they make hexes much less dense and bring space into the picture. Politics and policy are a later milestone still.

## Architecture principles

- **The engine is written in Rust** and is setting-agnostic. It can host many settings: space, the 19th century, ancient times, and others. Everything that belongs to a setting lives in data, not code.
- **A setting is a folder of data files.** It fully defines a world: the map, the goods and their properties, the natural resources, the parameters of utility and production functions, and later technology trees, starting States, and more. It is organized into many files. It separates persistent rules (constants that never change during play) from the starting state (what turn 0 looks like).
- **The engine is independent of the UI/UX.** The UI can be swapped out, most likely with a different UI per setting. During development, a developer UI lets the developer load a setting, inspect the world, and step turns. It is not a player UI.
- **Any actor is in principle playable**, but States are the intended player role.
- **Time is turn-based.** Each turn, actors act and the world updates.
- **Passive actors optimize; businesses decide.** POPs, natural resources, and factories are passive. A POP optimizes a fixed utility function with closed-form rules, given the information it has; its behavior changes with its circumstances, but the underlying form is generic. A factory is pure technology: it holds a production function and turns inputs into outputs by it. The business that owns it combines prices with the production function's parameters and decides how much it runs, what to buy, and what to sell. Early on that decision is textbook profit maximization. The divide exists so that later Stages can make the decision richer, drawing on more than one factory's prices and parameters, without changing the factory. Businesses hold every decision that faces the market, from how much a factory produces to the open-ended, discretionary choices that don't reduce to a generic form. Making the simulation more nuanced is then a matter of making business decisions richer, and businesses are the first place LLMs and SystemOne AI models plug in. Those models do not enter the picture until the closed-form simulation works.

## Building principles

- **Dummy data now, live data later.** When a system needs a system that isn't built yet, its actors respond to static dummy data. When the later system arrives, the dummy data is replaced with live simulation data. For example, POPs and businesses trade with a dummy market that posts fixed prices until the real market exists. The dummy market is the live market with two things missing: its prices never adjust, and it buys and sells any amount. Each Stage leaves clean seams where its stubs will be replaced.
- **Systems stay simpler than they sound** until a later system needs more. POPs sell unskilled labor to a dummy market long before anyone else demands it.
- **Every Stage before the Dev UI Stage keeps a small dev-UI Step** that lets the developer inspect the new actors and watch a build work. UI polish waits for the Dev UI Stage, when the market has made the numbers worth looking at.

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

A natural resource is a deposit that sits on a hex and produces a fixed amount of a related *unimproved* good each period. That good cannot be moved, and it perishes at the end of the period if unused. This models unimproved resources: whoever holds a deposit receives its unimproved good each period but must combine it with other inputs to make use of it. Deposits have no owner until the Factories Stage, when minimal businesses can own them. Resource exhaustion may be added later.

### POPs

A POP is a group of N identical individuals living on a hex. Each individual produces a fixed amount of unskilled labor per turn and the POP has a fixed Stone-Geary utility function that names the goods it consumes, including leisure. Leisure is the POP consuming its own labor rather than selling it, so labor supply and demand for goods both come out of the same utility maximization. A POP's utility parameters are its own, or are assigned from a set of possibilities the setting defines.

A POP grows or declines gradually. Its growth rate is a function of its utility: zero when it is exactly at subsistence, positive above, negative below. A POP whose income can't cover its subsistence bundle spends everything it has, buying toward subsistence, and shrinks; it doesn't die at once. Growth changes N; the per-individual parameters don't change.

POPs own their inventory of goods, including currency and their own labor, from the start. They own businesses, and through them deposits and factories, only from the Businesses Stage.

### Factories and technologies

A factory is a discrete asset on a hex that combines goods (including labor and unimproved resource output) into other goods. It is pure technology: it holds a production function and runs it. How much it runs, what it buys, and what it sells are decided by the business that owns it. Every factory has a business owner from the Factories Stage, and factories are tradable assets once businesses can buy and sell them.

**Technologies.** A technology is a named recipe in the setting's rules, shared by every factory that uses it, the way a POP type is shared by POPs. A factory points at one technology, and that pointer is state, so it can change. A recipe is **fixed-proportion**: so many units of each input per unit of output, with no substitution between inputs. Capacity belongs to the technology: a factory produces at most its technology's capacity per turn, and a business that wants more capacity needs more factories. Fixed proportions were chosen over smooth production functions because they keep the factory's physics honest: no unbounded output from one scarce input by piling on another, no shutdown over an input that should be an optional augmentation, and no instant swing in the input mix when a price moves.

**Substitution is retooling.** The economy still substitutes between inputs, but through business decisions rather than inside the production function. A business can transition a factory to a different technology, at a cost in goods and labor and over several turns, which is cheaper and faster than building a new factory. Which technologies a factory can transition to is data, so a setting can limit transitions to related technologies. Technology trees are lists of recipes and their transitions. Retooling arrives with the business decisions to build factories, after the Market Stage.

**Capital, depreciation, and maintenance.** A factory's capital is a bundle of goods inside it. Construction turns that bundle plus labor, over a number of turns, into a finished factory. Once finished, the factory consumes a fixed slice of its construction bundle each turn, the bundle divided by the technology's lifetime, in the bundle's proportions, whether or not it ran. It produces at full capacity until the bundle is exhausted and then shuts down. Maintenance mirrors construction at a smaller scale: the depreciated slice plus some labor, added back each turn. Scrapping a factory takes labor and time and recovers the remaining bundle. The Factories Stage builds factories that already exist in the starting state, depreciation, shutdown, and maintenance. Construction, scrapping, retooling, and trading factories are business decisions for later Stages.

### Businesses

A business is the layer between POPs and the things they ultimately own. POPs own businesses, and businesses own natural resources and factories. Businesses are the market-facing actors: they decide how much their factories produce, buy the inputs, sell the output, and hold the cash. They also hold the planning and strategic decisions, such as vertical and horizontal expansion, that are hard to reduce to a generic form.

Businesses arrive in two steps. A **minimal business** exists from the Factories Stage. The **Businesses Stage** then adds POP ownership, operating budgets, and dividends. Expansion decisions come with the Saving and Investment Stage.

The minimal business:

- **Owns a set of factories and a set of deposits**, of any size, with one stockpile and one cash balance across them. A sample setting may give each business one factory, but the shape is sets, so vertical integration is expressible from the start. Owned deposits emit into the business's stockpile, and the business feeds its own factories directly; moving goods within one owner is not an exchange, so it doesn't go through the market. Surplus unimproved output is sold before it perishes and shortfalls are bought.
- **Runs on a one-turn lag.** Each turn it sells last turn's output and any surplus, then buys this turn's inputs and maintenance, then its factories run and the output goes into inventory for next turn. Labor perishes after one period, so it is bought and used in the same turn.
- **Starts with cash** in the starting state. Purchases are capped by cash on hand after sales; when cash is short, the run level is scaled down (with fixed proportions, the run level is the smaller of capacity and cash divided by the per-unit input cost). There is no credit. Profits accumulate as cash until dividends exist.
- **Decides by arithmetic** under the dummy market: a factory runs at capacity and is maintained when the margin at posted prices covers inputs and maintenance; otherwise it is left to run down. The business values its own unimproved output at the posted price.

The cash cap is the seam where operating budgets plug in, and maintenance is the seam where expansion plugs in.

### Utility and production functions

Utility functions (POPs) and production functions (factories) name goods directly. Their parameters are defined in the setting folder.

### Markets

Each hex has one market, and every exchange of goods on the hex goes through it. The market works by **submission and aggregation**, not by search:

- **Actors submit parameters, not answers.** Each actor hands the market its demand and supply in one of a small set of forms the market knows how to add up. POPs submit their Stone-Geary parameters and their endowment. Factories submit nothing: they don't face the market. Businesses submit fixed quantities: offers of the output and surplus they hold, and bids for the inputs and maintenance of the run level they chose. Richer forms of submission are deferred to the Market Stage or to the market-power work after it. The market never asks an actor "how much would you want at this price?", so the cost of a turn grows with the number of actors only through one pass over them, and the market's own work grows with the number of goods.
- **The market aggregates exactly.** Stone-Geary demand is linear in income, so every POP of one type on a hex collapses into a single demand function with their sizes summed, and the aggregate demand across types has a closed form. The market works on aggregate curves, never on individual actors.
- **Prices are fixed within a turn and adjust between turns.** At the start of trading the market posts one price per good in the hex's currency. Actors trade at those prices. After trading, the market uses the aggregate curves to compute where each price would have cleared and moves next turn's price part of the way toward it, at an adjustment speed set in the setting. It never iterates to equilibrium within a turn.
- **Disequilibrium is real and persists.** When bids and offers at the posted price don't match, a rationing rule decides who trades and how much (proportionally, by default). Unsold durable goods are inventory, unsold labor perishes as unemployment, and unfilled bids are shortages. All three are visible quantities in the simulation. Gluts, shortages, and price cycles are emergent, and with constant fundamentals prices converge over turns to the equilibrium.
- **Settlement order matters.** An actor that is rationed when selling has less income than it planned. Sales settle before purchases, and currency left unspent, because a good was short or by rounding, carries over as income next turn.
- **Policy lives in the market.** Taxes and subsidies, once States exist, are wedges between the price buyers pay and the price sellers receive, applied to the aggregate curves. The market applies the wedge, solves, and reports the revenue. Policy changes the market's inputs, not the actors' code.
- **Market power comes later.** The first live market is price-taking. How sellers come to influence prices is for a later Stage.

## Stages

The roadmap currently runs to the Saving and Investment Stage. Later directions are listed below without order or commitment.

### Stage 1: Create the World

Setting files define persistent parameters and starting state. The world has hex tiles, natural resources on those tiles, and goods with their properties. A developer UI displays the hexes on a grid and the world's data, and steps turns.

- Hexes need only the data to display them on a grid. Position does not matter yet; terrain and similar attributes come later, when movement and trade need them.
- There is no ownership yet.
- A minimal turn exists from the start: the clock steps, resources emit their goods, and unused goods perish. Every later Stage adds behavior to an already-ticking clock.

### Stage 2: Populate the World

POPs with Stone-Geary utility functions that include leisure, unskilled labor, consumption, and growth or decline. POPs trade with a dummy market that posts fixed prices in the hex's designated currency good: they sell labor into it and buy goods from it, and their supply and demand follow from their utility functions. A POP's trade is structured as the live market expects: the POP's parameters and endowment are its submission, prices are posted, then the POP trades at those prices. They start with no cash and spend their whole income every turn; currency left over by rounding carries over as next turn's income, and saving comes later. Stone-Geary demand is closed form, so no approximations are needed; simplified heuristics (budget shares, share of labor sold) are an option later if the full rules turn out to be slow.

POPs own their own goods, but there is no ownership of deposits or factories in this Stage. Deposits keep emitting into unowned stockpiles.

### Stage 3: Factories

Technologies, factories, depreciation, and the minimal business, as described under [Factories and technologies](#factories-and-technologies) and [Businesses](#businesses). Technologies are fixed-proportion recipes with a capacity, a capital bundle, and a lifetime, defined in the rules. Factories exist in the starting state with a technology and a current capital bundle; they consume a slice of the bundle each turn and shut down when it's gone. Minimal businesses own sets of factories and deposits, start with cash, trade fixed quantities with the dummy market on a one-turn lag, and maintain the factories that pay for it. Owned deposits emit into their business's stockpile; unowned deposits keep emitting into unowned stockpiles. The sample setting gets a real production chain on one hex, from deposits through factories, with a capital good for the bundles. A dev-UI Step shows businesses and factories. There is no POP ownership, budget, dividend, construction, scrapping, or retooling yet.

### Stage 4: Market

The live market described under [Markets](#markets), in a single closed economy with no external trade. It brings POPs, factories, and minimal businesses together: the dummy market's infinite counterparty is removed, bids and offers are settled against each other with rationing, and prices adjust between turns toward clearing. Inventory, unemployment, and shortages appear. The test of correctness is that, with constant fundamentals, prices converge over turns to the known equilibrium. Market power is not part of this Stage.

### Stage 5: Dev UI

Once the market makes the numbers worth looking at, the developer UI gets a Stage of its own. It answers three questions, none of which are decided here: what the dev UI should be good for from now on (an inspector of any object in the world, a way to watch the economy move over turns, or both); how UI back-and-forths run inside the development loop, when the developer sees something wrong and the agent fixes it; and how the agent itself can see and test the UI, so UI work can be built and reviewed like everything else.

### Stage 6: Businesses

Ownership arrives in full. POPs own businesses; businesses already own deposits and factories. Businesses collect their factories' and deposits' earnings, set operating budgets, and pay dividends. Expansion decisions wait for Saving and Investment.

### Stage 7: Saving and Investment

POPs own shares of businesses and receive dividends. POPs do financial planning on data the businesses provide: deposit at banks for interest, buy and sell stock. Businesses make expansion decisions, vertical and horizontal: constructing factories, retooling them to other technologies, scrapping them, and buying and selling them. POPs make human capital decisions that let them exchange unskilled labor for labor of different skills. Completing this Stage reaches the closed-market milestone.

## Beyond Stage 7

Directions the project will take later, in no particular order:

- A central bank and a synthetic currency, so fiat money regimes can be simulated alongside commodity money.
- States as actors: budgets, taxes, spending, and the first policy levers. Taxes and subsidies are modelled in the markets, as wedges on the aggregate curves.
- Politics: how POPs form interests and pressure the State.
- Terrain, movement, and trade between places; open markets between economies. Hexes become much less dense when space matters.
- Technology trees and starting States in the setting folder.
- A player UI, and saving and loading.
- LLM and SystemOne AI models controlling businesses first, then other non-player actors.
- Resource exhaustion.
- Market power: sellers influencing prices instead of taking the market's.
