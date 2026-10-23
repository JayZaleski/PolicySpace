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
- **Optimizers live in the engine; strategists live behind a decision boundary.** POPs, natural resources, and factories are passive: a POP optimizes a fixed utility function with closed-form rules, a deposit emits, a factory is pure technology that turns inputs into outputs by its production function. Businesses, and later States and political factions, are **discretionary actors**: they hold every decision that doesn't reduce to a generic form, from how much a factory runs to open-ended strategy. The engine does not decide for them. It serves each one the data it may see, receives its decisions as plain data, and resolves them under the world's rules. Who makes the decision, a script, a learned model, an LLM, or the player, is interchangeable. See [Deciders](#deciders).

## Building principles

- **Dummy data now, live data later.** When a system needs a system that isn't built yet, its actors respond to static dummy data. When the later system arrives, the dummy data is replaced with live simulation data. For example, POPs and businesses traded with a dummy market that posted fixed prices and bought and sold any amount, until the Market Stage built the live market and the fixed prices became the prices on turn 0. Now a placeholder dividend returns business profits to POPs until the Businesses Stage gives businesses real owners. Each Stage leaves clean seams where its stubs will be replaced.
- **Dummy deciders now, smarter deciders later.** The same principle applies on the other side of the decision boundary. A discretionary actor's first decider is the simplest script that produces sensible behavior; richer strategies, learned models, and LLMs replace it later without touching the engine. Dummy data is replaced inside the engine; deciders are replaced outside it.
- **The engine's tests run its deciders.** Business behavior lives in Python from the start, so the engine's test suite depends on the project's Python environment. There is no duplicate Rust rule for tests to fall back on: two copies of one strategy drift apart. Engine tests that don't test strategy can replay recorded decisions instead.
- **Systems stay simpler than they sound** until a later system needs more. POPs sold unskilled labor to a dummy market long before anyone else demanded it.
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

Currency is a good designated as the one prices are posted in. It is held and exchanged like any other good. Each hex has its own designated currency, and the designation can change during the simulation, so it belongs to the starting state, not the persistent rules. The designated good can be a commodity, or later a synthetic good issued by a central bank, so the simulation can handle commodity and fiat money regimes alike. Until a central bank exists, currency is simply an in-game good that prices are quoted in. Money changes hands once per turn, because an actor spends only what it held at the start of the turn, so the stock of currency on a hex caps how much can be traded there in a turn.

### Natural resources

A natural resource is a deposit that sits on a hex and produces a fixed amount of a related *unimproved* good each period. That good cannot be moved, and it perishes at the end of the period if unused. This models unimproved resources: whoever holds a deposit receives its unimproved good each period but must combine it with other inputs to make use of it. Deposits have no owner until the Factories Stage, when minimal businesses can own them. Resource exhaustion may be added later.

### POPs

A POP is a group of N identical individuals living on a hex. Each individual produces a fixed amount of unskilled labor per turn and the POP has a fixed Stone-Geary utility function that names the goods it consumes, including leisure. Leisure is the POP consuming its own labor rather than selling it, so labor supply and demand for goods both come out of the same utility maximization. Labor the POP offered and couldn't sell also counts as leisure: unemployment costs the POP its wages, not its rest. If watching the simulation shows unemployment should hurt more, the change is a utility penalty for a falling wage, not for idle hours. A POP's utility parameters are its own, or are assigned from a set of possibilities the setting defines.

A POP grows or declines gradually. Its growth rate is a function of its utility: zero when it is exactly at subsistence, positive above, negative below. A POP whose income can't cover its subsistence bundle spends everything it has, buying toward subsistence, and shrinks; it doesn't die at once. Growth changes N; the per-individual parameters don't change.

POPs own their inventory of goods, including currency and their own labor, from the start, and the setting gives them starting cash, because an actor spends only what it held at the start of the turn. They own businesses, and through them deposits and factories, only from the Businesses Stage.

**POPs are state that reacts, never actors that choose.** Everything a POP does is a deterministic function of its state and the setting's parameters, computed in the engine, including the financial and human-capital choices of the Saving and Investment Stage. The engine draws a POP's market orders from its preferences, its endowment, and its cash, and refreshes them under the same rules as every other actor's orders (see [Markets](#markets)), so POP demand is sticky the way habits are. If POPs ever respond to policy, that response follows the same pattern as growth: a closed-form function of the POP's circumstances, such as its utility against subsistence or the trend in its income, with parameters in the setting. The engine computes it and serves it to discretionary actors as data. Policy never changes POP opinion by fiat; it changes prices and incomes through the market, POP state follows, and opinion follows that. The content of that function is not designed yet and belongs to the politics milestone.

### Factories and technologies

A factory is a discrete asset on a hex that combines goods (including labor and unimproved resource output) into other goods. It is pure technology: it holds a production function and runs it. How much it runs, what it buys, and what it sells are decided by the business that owns it. Every factory has a business owner from the Factories Stage, and factories are tradable assets once businesses can buy and sell them.

**Technologies.** A technology is a named recipe in the setting's rules, shared by every factory that uses it, the way a POP type is shared by POPs. A factory points at one technology, and that pointer is state, so it can change. A recipe is **fixed-proportion**: so many units of each input per unit of output, with no substitution between inputs. Capacity belongs to the technology: a factory produces at most its technology's capacity per turn, and a business that wants more capacity needs more factories. Fixed proportions were chosen over smooth production functions because they keep the factory's physics honest: no unbounded output from one scarce input by piling on another, no shutdown over an input that should be an optional augmentation, and no instant swing in the input mix when a price moves.

**Substitution is retooling.** The economy still substitutes between inputs, but through business decisions rather than inside the production function. A business can transition a factory to a different technology, at a cost in goods and labor and over several turns, which is cheaper and faster than building a new factory. Which technologies a factory can transition to is data, so a setting can limit transitions to related technologies. Technology trees are lists of recipes and their transitions. Retooling arrives with the business decisions to build factories, after the Market Stage.

**Capital, depreciation, and maintenance.** A factory's capital is a bundle of goods inside it, defined by its technology. Construction turns that bundle plus a construction bundle (intended to be labor), over a number of turns, into a finished factory. Once finished, the factory loses one slice of its capital each turn, the bundle divided by the technology's lifetime, in the bundle's proportions, whether or not it ran. Because the capital only ever changes in its own proportions, a factory's whole state is one share of its technology's bundle: its condition. Every turn of output costs one full slice: a factory holding at least a slice produces at full capacity, one holding less is shut down and makes nothing, and wear continues either way, so a full, never-maintained factory runs exactly its lifetime. Maintenance is construction run for a fraction of its time: a business may replenish any share of the capital, paying that share of the capital and construction bundles, capped at one construction slice per turn, and this can revive a shut-down factory slice by slice. A technology whose construction takes longer than its lifetime wears faster than it can be maintained and cannot be revived; a setting may say that. Within a turn a factory runs, then wears, then is maintained, so steady maintenance of one slice holds it full. A business's factories run as if at the same moment: their output arrives after all have run, and a shortage of an input is shared among them proportionally rather than by order. Scrapping a factory takes labor and time and recovers the remaining bundle. The Factories Stage built factories that exist in the starting state, depreciation, shutdown, and maintenance. Construction, scrapping, retooling, and trading factories are business decisions for later Stages.

### Businesses

A business is the layer between POPs and the things they ultimately own. POPs own businesses, and businesses own natural resources and factories. Businesses are the market-facing actors: they decide how much their factories produce, buy the inputs, sell the output, and hold the cash. They also hold the planning and strategic decisions, such as vertical and horizontal expansion, that are hard to reduce to a generic form.

Businesses arrive in two steps. A **minimal business** exists from the Factories Stage. The **Businesses Stage** then adds POP ownership, operating budgets, and dividends. Expansion decisions come with the Saving and Investment Stage.

The minimal business:

- **Owns a set of factories and a set of deposits**, of any size, with one stockpile and one cash balance across them. A sample setting may give each business one factory, but the shape is sets, so vertical integration is expressible from the start. Owned deposits emit into the business's stockpile, and the business feeds its own factories directly; moving goods within one owner is not an exchange, so it doesn't go through the market. Surplus unimproved output is sold before it perishes and shortfalls are bought.
- **Sits on one hex** and owns only assets on it, until trade between places exists. Then a business is expected to operate across several hexes with this one as its headquarters.
- **Spends only what it held at the start of the turn**, like every actor. Each turn it sells last turn's output and any surplus, buys this turn's inputs and maintenance with the cash it started the turn with, then its factories run and the output goes into inventory for next turn. Labor perishes after one period, so it is bought and used in the same turn.
- **Starts with cash** in the starting state. Purchases are capped by that cash: when the bids cost more than the cash, every bid is scaled by one proportion, so a short business buys a smaller bundle in the shape it chose. The engine never changes a run level directly; a factory that finds fewer inputs runs less by the factory rules, which is how a cash shortage reaches production. There is no credit. Until the Businesses Stage gives businesses owners, a **placeholder dividend** keeps money circulating: each business keeps a cash reserve (a setting multiple of its usual spending) and pays the excess to the POPs on its hex by headcount. It is an engine rule standing in for ownership, and the setting can switch it off.
- **Decides by arithmetic, in a script.** The rule is simple: a factory runs at capacity and is maintained when the margin at current prices covers inputs and steady maintenance; otherwise it is left to run down. A shut-down factory of a technology that pays is maintained but not run until it holds a slice again. The business values its own unimproved output at the current price, offers every surplus at or above the price where the factory breaks even, bids for every shortfall at or below it (both as the lines with give-up prices the market takes, see [Markets](#markets)), and scales its whole plan down by one proportion when the bids would cost more than its cash. That rule is the business's first decider, a Python script speaking the protocol described under [Deciders](#deciders), not engine code. The engine serves the business its view, receives its fractions and orders, and applies the cash cap and the market's rules to whatever it receives.

The cash cap is the seam where operating budgets plug in, and maintenance is the seam where expansion plugs in. The decider is the seam where richer strategy plugs in.

### Deciders

A **decider** is whatever makes a discretionary actor's decisions. The engine defines the boundary; deciders live on the other side of it.

- **The engine owns consequences; deciders own choices.** Whatever a decider sends, the engine applies the world's rules to it: cash caps, capacity, stockpile contents, perish rules, market settlement. A decider can be naive or wrong and the world stays consistent. That is what lets deciders be swapped freely.
- **The view is the information set.** Each turn the engine serves each discretionary actor a view: the data that actor may know. An actor cannot act on anything the view doesn't contain. Views are scoped to the actor's circumstances (its hex, its own assets, its hex's public market data, and later its trading partners), never to the whole world, so a view's size does not grow with the size of the world. Hidden information is enforced by construction, and a script that peeks at aggregates it couldn't know is impossible rather than merely discouraged.
- **Decisions are plain data.** The view and the decision are both serializable: data with no references into the engine's internals, so they can cross a process boundary, be logged, and be replayed. The engine defines the **action vocabulary**: the kinds of decision each actor type can make, with the bounds the engine will enforce. It validates and resolves; it does not enumerate every legal choice. The vocabulary grows one Stage at a time (run and maintenance fractions per factory, offers, and bids now; construction, retooling, trading assets, budgets, dividends, taxes, and spending later), and every verb in it is resolved by engine rules. Asking for more than an actor holds or can pay is not an invalid decision but a real request that is too big: the engine trims it and records what was asked beside what was done.
- **Deciders are interchangeable.** A rule in a script, a learned model, an LLM, and a human at a UI are all deciders for the same actor through the same protocol. The player is not a special case: a player's State is an actor whose decider is the UI. The developer UI is a decider too; it can take over any actor and submit its decisions by hand, which is both a debugging tool and the first proof of the player path.
- **Python is the default home for deciders, not the only one.** The project's deciders are written in Python unless there is a reason not to, because that is where the simple scripts, the learned models, and the LLM calls all live most easily. Any language that speaks the protocol can host a decider, so a strategy that gets too slow in Python can move to compiled code without the engine noticing.
- **Which decider controls which actor is setting data.** Actors in the starting state name their decider there, and a name the running deciders don't provide is an error when the world is created, not a turn-time event. Discretionary actors enter and leave the simulation during play (businesses are founded and fail; later States form and collapse), so the setting will also give a rule for the decider a newly created actor gets. Each turn's request tells a decider which actors it currently controls; an actor that exits has its pending decisions dropped.
- **The engine never depends on a decider's memory.** A decider may remember what it likes between turns, but the simulation's state lives entirely in the engine, so a decider can be restarted or replaced between turns with no loss.
- **The protocol's shape is a performance rule.** The turn has one **decision point**, where every discretionary actor's view is served and every decision collected in one batched exchange, before trading. There is never a round trip per actor or per sub-decision, and no decider sees the result of one choice before making another in the same turn. Decider processes persist across turns rather than being started each turn. Quantities cross the boundary losslessly, in the engine's own representation rather than as floating-point approximations. The default scripts work over whole batches, not one actor at a time. With those rules the boundary costs about one extra pass over the actors' data per turn, and the decision point is parallel across hexes and deciders.
- **Attention is rationed.** A business's orders stay on file at the market, so a business is only served a view, and only decides, on turns when it is due to re-think: in the baseline fraction, or woken because a price that matters to it moved (see [Markets](#markets)). Whether a decider can ask to be woken next turn, so a strategy that needs attention every turn isn't cut off, is a design question for the Market Stage.
- **Decisions can explain themselves.** A decision may carry a trace: a decider's account of why it chose what it chose, in a form the developer UI can show. The engine stores it and never acts on it.
- **Failure has a defined meaning.** A broken decider degrades the simulation in a known way and never halts it. A decision the engine can't resolve is refused as a whole and idles that actor for the turn, kept with its errors so a developer can see why. A decider that errors, answers late, or sends a reply that isn't one idles every actor it controls for that turn, is reported in the turn's results, and is started again at the next decision point. A decider that can't start at all is a setting-load error, shown where a broken setting would be. Deciders that don't fail are never affected by one that does.

### Utility and production functions

Utility functions (POPs) and production functions (factories) name goods directly. Their parameters are defined in the setting folder.

### Markets

Each hex has one market, and every exchange of goods on the hex goes through it. The market has three jobs, and they fix its shape:

- **Settle trades.** Buyers and sellers of a good trade at one price, and that price moves up and down toward where supply meets demand. It doesn't have to get there within a turn.
- **Publish data actors can reason with.** For each good, the market knows roughly how much would be bought and sold at prices other than the current one, so an actor can work out how its own orders would move the price, and a later politics system can estimate who gains and who loses when a price changes.
- **Stay neutral.** The market settles whatever orders it receives by one mechanical rule. It never adjusts an outcome from a top-down economic rationale, such as raising a price because a market looks concentrated. Market power, gluts, shortages, and price swings all emerge from what actors actually order, and actors are free to order according to their own strategies.

The rules that follow from those jobs:

- **Orders are straight lines.** For each good it wants to buy or sell, an actor sends a linear demand or supply curve: a quantity at the current price and a slope, clipped so the quantity never goes below zero or above a maximum, with a **give-up price** at which the actor buys or sells nothing (zero is allowed). An order without a give-up price is rejected like any malformed decision. Lines are the one form every actor speaks, and they add up: the summed line's slope is what a seller needs to judge a supply restriction, and the area between the summed lines is the surplus a politics system needs.
- **POPs don't send orders; the engine draws them.** POPs are passive, so the engine turns a POP's Stone-Geary preferences, its endowment, and its cash into a line for each good at the current prices. It is an approximation of the true curve, good near the current price.
- **Each good is settled on its own.** A line for one good carries no information about other goods' prices, so the market finds each good's price taking the other prices as they stand. Cross-effects, such as a wage change moving bread demand, are corrected over the following turns, not within the turn.
- **Orders stay on file.** An actor's lines remain at the market until the actor replaces them. Each turn a baseline fraction of actors re-think and re-send, and any actor is woken to re-think when a price that matters to it has moved more than a threshold since that actor last looked. The fraction and the threshold are setting parameters and apply to POPs too. A calm market is sluggish; a turbulent one has everyone re-evaluating. This is the economy's lag knob.
- **The price is set at the crossing.** Each turn, for each good, the market sums the lines on file and sets the price where total demand meets total supply. Everyone trades along their own line at that price. There is no gradual movement toward clearing: lag comes from orders being out of date, not from the price rule. When a good has orders on only one side, its price stays where it was, nothing trades, and the unfilled side is recorded as a shortage or a glut.
- **You spend only what you held at the start of the turn.** This turn's earnings are spendable next turn. So every actor's budget is known exactly when its lines are drawn, and nothing an actor can spend depends on how everyone else's trades settled. If the crossing prices make an actor's basket cost more than its cash, every purchase is scaled by one proportion. There is no credit.
- **Mismatches come from stale orders.** A line on file may promise goods the actor no longer holds. The engine trims the trade to what is there, and the other side comes up short. Unsold durable goods are inventory, unsold labor is unemployment, unfilled bids are shortages, and all three are visible quantities. With constant fundamentals prices settle to the equilibrium and stay there.
- **Market data is public on the hex.** For every good: the current price, the summed demand and supply lines, how much traded, and how much went unfilled on each side. Every actor on the hex sees all of it in its view.
- **Policy lives in the market.** Taxes and subsidies, once States exist, are wedges between the price buyers pay and the price sellers receive, applied to the summed lines. The market applies the wedge, solves, and reports the revenue. Policy changes the market's inputs, not the actors' code.
- **Market power is a strategy, not a market feature.** Nothing in the market grants it or prevents it. A seller that restricts supply moves the price because fewer units were offered, and for no other reason. Deciders that pursue it come later.

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

Technologies, factories, depreciation, and the minimal business, as described under [Factories and technologies](#factories-and-technologies) and [Businesses](#businesses). Technologies are fixed-proportion recipes with a capacity, a capital bundle, a construction bundle and time, and a lifetime, defined in the rules. Factories exist in the starting state with a technology and a condition; they lose a slice of their capital each turn, shut down below one slice, and are revived by maintenance. Minimal businesses own sets of factories and deposits on their one hex, start with cash, trade fixed quantities with the dummy market on a one-turn lag, and maintain the factories that pay for it. Owned deposits emit into their business's stockpile; unowned deposits keep emitting into unowned stockpiles.

This Stage built the first [decider](#deciders) and the boundary it speaks through, sized to the minimal business and nothing more general: the view, the decision vocabulary, validation, lossless JSON messages, and a recording format for a turn's batch. The turn gained its decision point: once per turn the engine serves every business its view, a Python process that lives across turns returns its decisions, and the engine applies the cash cap, trades, and operates the factories. The business's arithmetic rule lives in that script, not in Rust, and the engine's tests run it; tests that don't test strategy replay recordings. Failure has the meaning given under Deciders. The sample setting has a real production chain on one hex, from deposits through factories, with a capital good for the bundles. The dev UI shows businesses, their decisions and traces, and their factories' condition and remaining life. Letting the developer take over a business and submit its decisions by hand waits for the Dev UI Stage. There is no POP ownership, budget, dividend, construction, scrapping, or retooling yet.

### Stage 4: Market

The live market described under [Markets](#markets), in a single closed economy with no external trade. The dummy market goes away: its fixed prices become the prices on turn 0, and from then on each good's price is set where the lines on file cross. POPs get starting cash in the setting. The engine draws POP lines from their preferences; the business script is rewritten to send lines with give-up prices (sell output at or above break-even, buy inputs at or below it), so the arithmetic rule does not survive the live market unchanged. Orders stay on file under the refresh fraction and the wake-up threshold, actors spend only what they held at the start of the turn, and the placeholder dividend returns business profits to the hex's POPs. Inventory, unemployment, and shortages appear as visible quantities, and the market's data is in every view on the hex. The test of correctness is that, with constant fundamentals, prices settle to the equilibrium we can calculate and stay there. A small dev-UI Step shows prices, the summed lines, and shortages and gluts moving over turns. Strategies that pursue market power are not part of this Stage; the market doesn't prevent them.

### Stage 5: Dev UI

Once the market makes the numbers worth looking at, the developer UI gets a Stage of its own. It answers three questions, none of which are decided here: what the dev UI should be good for from now on (an inspector of any object in the world, a way to watch the economy move over turns, or both); how UI back-and-forths run inside the development loop, when the developer sees something wrong and the agent fixes it; and how the agent itself can see and test the UI, so UI work can be built and reviewed like everything else. The dev UI also becomes a [decider](#deciders): it can take over any discretionary actor and submit its decisions by hand, the first proof of the player path.

### Stage 6: Businesses

Ownership arrives in full. POPs own businesses; businesses already own deposits and factories. Businesses collect their factories' and deposits' earnings, set operating budgets, and pay dividends, replacing the Market Stage's placeholder dividend. Expansion decisions wait for Saving and Investment.

### Stage 7: Saving and Investment

POPs own shares of businesses and receive dividends. POPs do financial planning on data the businesses provide: deposit at banks for interest, buy and sell stock. Businesses make expansion decisions, vertical and horizontal: constructing factories, retooling them to other technologies, scrapping them, and buying and selling them. POPs make human capital decisions that let them exchange unskilled labor for labor of different skills. Completing this Stage reaches the closed-market milestone.

## Beyond Stage 7

Directions the project will take later, in no particular order:

- A central bank and a synthetic currency, so fiat money regimes can be simulated alongside commodity money.
- States as actors: budgets, taxes, spending, and the first policy levers. Taxes and subsidies are modelled in the markets, as wedges on the summed lines.
- Politics: political factions as discretionary actors whose objectives are built from data about POPs, and the closed-form rule by which POPs respond to policy.
- Terrain, movement, and trade between places; open markets between economies. Hexes become much less dense when space matters.
- Technology trees and starting States in the setting folder.
- A player UI, and saving and loading.
- Learned models and LLMs as deciders, replacing the scripts behind businesses first and other discretionary actors later.
- Resource exhaustion.
- Deciders that pursue market power. The market already lets a seller move a price by what it offers; strategies that exploit this come later.
