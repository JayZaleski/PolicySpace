# Step 06: business-script

## Summary
This Step adds the project's first Python: a package holding a client that speaks Step 05's protocol (it reads one turn's batch of views and writes the batch of decisions) and the first business decider, the arithmetic rule. The rule is the simplest script that produces sensible behavior under the dummy market: a factory runs at capacity and is maintained when the margin at posted prices covers its inputs and maintenance, and is otherwise left to run down; the business sells what it holds and doesn't need, buys what it needs and doesn't hold, and scales its plan down to the cash it will have. The rule is a function over a whole batch, separate from the input/output loop that carries it. Everything is tested in Python against recorded views, from the project's virtual environment, and the one check command runs those tests with the Rust ones. Nothing in the engine runs the script yet: Step 07 starts it and drives the exchange, Step 08 resolves what it sends.

## Behavior

### The package
The Python code lives in a top-level `python/` folder as one package, `policyspace`, with a project file, installed in editable form into the project's virtual environment (`.venv`), so the module runs from anywhere and Step 07's test setup only needs to find the environment. It has no runtime dependencies: the standard library's JSON is enough, and it is lossless for this format (floats round-trip exactly and Python's integers cover the largest turn number). The test runner is pytest, added to `requirements.txt`.

The package separates three things, so each can change alone:

- **The protocol**: reading and writing Step 05's messages (views, decisions, requests, replies, recordings) in its exact JSON form. Every number read from the engine is written back unchanged.
- **The rule**: a function from a batch of views to a batch of decisions. Batch in, batch out is the shape the vision asks of every default script; with the sample's three businesses the work inside is plain, and nothing requires it to be vectorized.
- **The loop**: the script's command-line form. Started with a decider's name as its argument, it reads one compact request (one line of JSON) per turn on standard input, answers one compact reply per line on standard output, and stops when input ends. Step 05's compact writer already puts one message on one line, so this is the long-lived form Step 07 needs; Step 07 adds keeping the process alive, timeouts, and what the engine does when the script fails. Only the name `arithmetic` exists (it is what the sample's businesses name); any other name is an error at startup, so a misnamed decider fails at once rather than silently deciding nothing. Later rules are added as names.

A reply answers every business in the request, including businesses on hexes without a market, which get an empty decision. Decisions in the reply follow the request's order.

### The arithmetic rule
The rule plans each business from its view alone: posted prices, its stockpile, cash, factories, their technologies and condition. It never looks outside the view, and it remembers nothing between turns.

**The margin test, per technology.** One turn at capacity, at the hex's posted prices, earns capacity times the output's price and costs capacity times the inputs' prices plus one lifetime-slice of the capital bundle and of the construction bundle at prices (the steady maintenance that holds a factory's condition). The business values goods it produces itself (its deposits' output) at the posted price, as the vision says, so the test depends only on the technology and the prices, never on what the business happens to hold. A technology whose output, any of whose inputs, or any good in its capital or construction bundle has no posted price fails the test: the market there can't trade it, so the business could neither sell the output nor buy what running and maintenance need. When the margin is at least zero, every factory of that technology is **run at capacity and maintained**; break-even runs. Otherwise every factory of that technology is **left to run down**: run fraction 0, maintenance fraction 0.

**The maintenance fraction.** Wear comes before maintenance within a turn (Step 04), so the rule asks for what brings a factory back to full after this turn's wear: one minus its condition after wear, capped at one construction slice (one over the technology's construction turns), the most the engine allows in a turn. A full factory therefore gets exactly one lifetime-slice, holding it steady; a factory below full is caught up over a few turns. A shut-down factory (holding less than one lifetime-slice) of a technology that passes the test is maintained too, so it revives slice by slice as Step 04 allows; it is not run until it holds a full slice, since it would make nothing. The margin test counts only the steady slice, not the catch-up, so a factory below full is restored on the strength of its steady-state margin; that is accepted for a rule this simple.

**Offers and bids.** For each good, the business's need is the inputs for its planned runs plus the goods its planned maintenance costs, and what it holds is the stockpile's total over every perish group. It bids the shortfall and offers the surplus. Zero amounts, the hex's currency, and goods with no posted price are never offered or bid, so every decision passes Step 05's validation. Selling every priced surplus makes the offers "last turn's output and surplus unimproved goods" without the rule having to know which is which, and a business with nothing to run still sells what it holds.

**The cash budget.** Sales settle before purchases, and the dummy market buys everything offered, so the business will have its cash plus the value of its offers at posted prices before it buys. When the bids would cost more than that, the rule scales the run and maintenance fractions of all the business's factories by one common proportion, the budget over the bids' cost, and recomputes its needs, offers and bids from the scaled plan. Scaling needs down can only shrink bids and grow offers, so one pass gives a plan within the budget. One proportion for every factory and both fractions is the simplest rule and mirrors Step 04's proportional sharing of a shortage; the engine's own cash cap (Step 08) remains the safety net, not the normal path.

**The trace.** Every decision carries a trace, since it is the only account of why the business did what it did and Step 09 shows it: per factory, the margin arithmetic and the verdict; then the budget, and the scaling proportion if the plan was cut.

### Tests and recorded views
The Python tests drive the rule with recorded views, in Step 05's recording format:

- **The sample at turn 0**: a committed recording of the request the engine builds for the sample's three center businesses, with the reply the rule is expected to give. A Rust test keeps it honest: it loads the sample, builds the same request, checks it equals the file's, and validates the file's reply against it. If the sample changes, that test fails and the recording is regenerated, so it cannot drift from the engine. The same file is ready for Step 07's replay decider.
- **Hand-written requests** for the cases the sample doesn't reach: a technology that fails the margin test, a cash-short business with two factories (the uniform scaling), a shut-down factory being revived, a business on a hex with no market, an unpriced input, and a business with no factories that only sells. The Rust test checks these for format only.
- **Lossless round trips**: every message Python reads and writes comes back as the engine's own compact JSON, using the values Step 05 chose to exercise exactness.
- **The loop**: the script, started from the virtual environment with `arithmetic`, answers a request line with the expected reply line and stops at the end of input; an unknown name fails at startup.

`tools/check.py` gains a fourth check that runs the Python tests from the virtual environment, so one command still checks the whole project.

## Player-facing Changes
None in the UI, the turn, the report or the setting format. The project gains a `python/` folder and a Python test suite, and the check command runs it. Setting up the environment now also installs the package (the setup command in `CLAUDE.md` stays as it is; the project file and `requirements.txt` carry the additions).

## Interactions
- **Business view and decisions** (Step 05): the protocol the client reads and writes, the validation every decision the rule sends must pass, and the recording format the tests use.
- **Factory mechanics** (Step 04): the order run, wear, maintain; the shut-down threshold; the maintenance cap; and the principle that the rule asks only for what it can cover.
- **Technologies** (Step 01): the numbers the margin test and the needs are computed from.
- **Businesses** (Step 02): the decider name `arithmetic` the sample's businesses carry is the script's argument.
- **Dummy market** (Stage 02 Step 02): the posted prices the rule plans at, and the assumption that every offer sells.
- **Workspace** (Stage 01 Step 01): the check command and the virtual environment.
- **Later Steps:** Step 07 starts the script with the decider's name, drives the line-per-turn exchange, and replays the committed recording. Step 08 resolves the decisions and applies the cash cap. Step 09 shows the trace. Stage 04's live market breaks the assumption that every offer sells; the rule stays valid, since over-asking is trimmed by the engine.

## Out of Scope
- Starting the script from the engine, keeping it alive, timeouts, and what a failed or late reply means: Step 07.
- Resolving decisions, trading, running and maintaining factories in the turn: Step 08.
- Any strategy beyond the arithmetic rule: ranking factories by margin, protecting maintenance over running, holding inventory, or using memory between turns.
- Vectorized or parallel evaluation of a batch.
- Any Python dependency beyond pytest for tests.
- Any change to the engine's protocol, the sample setting, the turn, the report or the dev UI.

## Acceptance Criteria
- The package installs in editable form into the virtual environment, and `.venv/Scripts/python -m policyspace arithmetic` answers a request line on standard input with one reply line on standard output and exits when input ends; an unknown decider name fails at startup with a message.
- On the sample's turn-0 recording, the rule runs all three factories at capacity: the farm bids the unimproved grain and labor it lacks and maintains one lifetime-slice; the mine maintains more than one slice to catch up from its starting condition, within one construction slice; the toolmaker, at a thin but non-negative margin, runs and maintains. Every decision validates against its view in the engine, and the reply equals the recording's.
- A technology with a negative margin, or an unpriced input or output, gets run 0 and maintenance 0 for every factory of it, and the business still offers its priced surplus. A shut-down factory of a profitable technology is maintained but not run. A business on a hex with no market gets an empty decision.
- A cash-short business with two factories has both factories' run and maintenance fractions scaled by the same proportion, its bids' cost is within its cash plus its offers' value, and the trace shows the proportion.
- Every message Python reads from the engine's JSON is written back identical, including the exactness values from Step 05.
- The Rust fixture test confirms the committed sample recording's request equals the engine's and its reply validates; the hand-written requests parse.
- `.venv/Scripts/python tools/check.py` runs the Python tests and passes, and every existing Rust test passes unchanged.
