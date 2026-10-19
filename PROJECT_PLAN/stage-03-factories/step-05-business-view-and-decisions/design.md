# Step 05: business-view-and-decisions

## Summary
This Step defines the decider protocol as engine data, sized to the minimal business. It has five parts: the view a business is served each turn (the data it may know and nothing more); the action vocabulary (the kinds of decision a business may answer with, each with the bounds the engine enforces, and an optional trace); validation of a decision against the view it answers; lossless serialization of both as plain data with no references into the engine; and the shape of a turn's batch (every view out, every decision back, in one exchange), which is also the recording format. Nothing is wired into the turn, no process is started and no Python is written: Step 06 writes the first script against this format, Step 07 runs the exchange, and Step 08 resolves the decisions. Everything here has its own tests.

## Behavior

### Where the view is taken
The decision point sits at the start of the "actors act" part of the turn, before any trading. So when a business is served its view, its stockpile already holds this turn's emission from its deposits (unimproved goods that perish at the end of this turn), the output its factories made last turn, and whatever currency it carried over. Building a view reads the world and changes nothing.

### The view
A business's view holds, and holds only, what concerns that business:

- **The turn number**, and **the business's name**.
- **Its hex's currency good**, by name, and **the hex's posted prices**: one price per good the hex's market trades, in the currency. A hex with no market posts no prices, and the view says so by having none.
- **Its stockpile, grouped by when the goods perish.** For each good it holds: the amount in each perish group, with the turn at whose end that group perishes, or that it never does. This is the engine's own representation of the stockpile, and it is what lets a decider tell the goods it must sell this turn from the goods it can keep. Totals are not sent separately; a decider sums the groups.
- **Its cash**: the amount of the hex's currency it holds, for convenience. It is the same number as the currency's total in the stockpile.
- **Its factories**, each by its ID number, with its condition and the name of the technology it runs.
- **The technologies its factories use**, each listed once by name with every number the rules give it: inputs per unit of output, the output good, capacity, the capital bundle, the construction bundle, construction turns and lifetime. Technologies are listed once rather than repeated under each factory, because a business with many identical factories should not receive its recipe many times.
- **Its deposits, summed per resource type**: for each type it owns deposits of, how many, the good a deposit emits and how much each emits per turn. Deposits of one type on one hex are identical and nothing addresses a single deposit yet; IDs can be added when a Step trades deposits.

Everything in the view is scoped to the business: its hex, its own assets and the prices it faces. Nothing about other businesses, POPs, other hexes, hex stockpiles or totals is in it, so a decider cannot act on anything it could not know.

### The action vocabulary
A decision names the business it answers for and carries:

- **Per factory, by ID number: a run fraction and a maintenance fraction**, each a number from 0 to 1 inclusive, with the meanings Step 04 gave them (the share of capacity to use, and the share of the full capital bundle to replenish). A fraction left out is 0. A factory the decision doesn't mention is left alone: run 0, maintenance 0. That is a legitimate choice, not a malformed decision, and it lets a decider send an empty decision; Step 08's record shows the factory idle.
- **Offers**: for each good the business wants to sell, one amount.
- **Bids**: for each good the business wants to buy, one amount.

The factories, the offers and the bids are each a table: keyed by the factory's ID number, or by the good's name, with one entry per key. A table can't hold the same key twice, so a decision can't list a factory or a good twice; a decider that builds its decision as a table (as Python does) gets this for free, and the engine refuses a file in which a key is written twice (see Serialization). The same shape is used for the view's factories.
- **A trace**, optional: one free-text string, as many lines as the decider likes, a decider's account of why it chose what it chose. The engine stores it for the dev UI (Step 09) and never acts on it. It is absent when the decider sends none.

Nothing here is a reference into the engine: goods, technologies, resource types and businesses go by their setting names, factories by their ID number.

### Validation
A decision is validated against the view it answers, and a decision that is nonsense is refused as a whole. Validation reads the whole decision before deciding, and reports every problem it found, each at its place (the factory or the good it concerns), in the loader's style. A decision is refused when it:

- names a factory that isn't in the view (not the business's, or no such factory);
- names a good that doesn't exist;
- gives a fraction that is negative, above 1, or not a number;
- gives an amount that isn't a valid quantity (negative, not finite, not a number);
- offers or bids the hex's own currency;
- offers or bids a good the hex posts no price for, since the market there can't trade it.

What happens to a refused decision (whether the business does nothing that turn, and whether the run stops) is Step 07's failure semantics, not this Step's.

**Asking for too much is not a validation error.** Offering more of a good than the stockpile holds, or bidding for more than cash can pay, is a real request that is too big, not nonsense. Such a decision is valid; Step 08 trims it when it resolves it (the cash cap can't be checked until sales have settled anyway) and records what was asked beside what was done, as Step 04 does for a factory whose inputs run short. One rule for every quantity keeps the safety net uniform, and a decider that overshoots by rounding doesn't lose its whole turn. Offering and bidding the same good is likewise allowed: the engine doesn't police economics.

### Serialization
Views, decisions and batches are written as JSON and read back from it. JSON is chosen because Python reads and writes it with nothing installed, every language has it, it is readable by eye, and a saved turn can be diffed. The engine gains the standard Rust JSON library as a dependency for this.

**The lossless rule.** Quantities, fractions and turn numbers are written exactly and read back exactly: the number that comes out of a round trip is the number that went in, for every value the engine can hold. The engine's quantity type is the one place the written form is defined, so if the engine ever changes its number representation, the format changes there and nowhere else. Reading rejects anything that isn't a valid value of its kind (a quantity that is negative or not finite, a turn that isn't a whole number, a fraction outside 0 to 1), and refuses a table in which a key is written twice (a factory or a good listed twice) rather than silently keeping one of the two entries, so a decision read from JSON is either well-formed or refused with a clear message; validation against the view then covers the rest. JSON requires a table's keys to be text, so a factory's ID number appears in quotes in the file; the engine reads it back as a number, and a decider that copies the key from the view into its decision needs to know nothing about this.

### The batch
The turn has one decision point, where every business's view is served and every decision collected in one exchange, so the two messages have a shape:

- **The request**: the turn number and a list of views.
- **The reply**: the turn number and a list of decisions, in any order, for any subset of the businesses in the request. A business with no decision in the reply does nothing this turn, for the same reason an unmentioned factory is left alone.

A reply is refused as a whole when its turn number doesn't match the request's, when it carries a decision for a business that wasn't in the request, or when it carries two decisions for one business. Otherwise each decision is validated against its business's view as above. Any subset is allowed because a decider may control any set of businesses, and Step 07 may split one turn's businesses among several deciders.

**A recording of a turn** is a request and its reply saved together. Step 06's Python tests read recordings to drive the script, and Step 07's replay decider answers from them so engine tests that don't test strategy need no Python.

How the request and reply travel between processes (pipes, framing, timeouts, keeping a process alive across turns) is Step 07's.

## Player-facing Changes
None. Nothing in the turn, the report, the setting format or the dev UI changes. The view, the decision, validation and the batch are observable only through their tests until Steps 06 to 09.

## Interactions
- **Businesses** (Step 02): the view is built for a business from its name, hex, stockpile and deposits. The decider name a business carries is not used here; Step 07 resolves it.
- **Factories** (Step 03): factories appear in the view by ID with their condition and technology, and decisions address them by the same ID.
- **Factory mechanics** (Step 04): the two fractions and their bounds are the ones Step 04 defined, and the "trim and record asked beside done" principle for over-asking is Step 04's safety net extended to offers and bids.
- **Technologies** (Step 01): the numbers listed per technology in the view.
- **Dummy market** (Stage 02 Step 02) and **currency** (Stage 02 Step 01): the posted prices and currency in the view, and the rule that a good with no posted price can't be offered or bid.
- **Stockpiles** (Stage 01 Step 06): the perish groups the view lists.
- **World and turn** (Stage 01 Step 06): the view is read from the World at the decision point; nothing in the turn changes yet.
- **Later Steps:** Step 06 writes the Python client and the arithmetic rule against this format. Step 07 runs the exchange, resolves decider names, defines what a refused decision means, and replays recordings. Step 08 resolves valid decisions, trimming over-asking, and keeps the trace in the business's record. Step 09 shows the decision and its trace in the dev UI.

## Out of Scope
- Any Python, any process, any transport between the engine and a decider: Steps 06 and 07.
- Wiring the decision point into the turn, and resolving decisions (trading, running, maintaining, the cash cap): Steps 07 and 08.
- What the engine does with a refused decision or a missing reply: Step 07.
- Verbs beyond run fraction, maintenance fraction, offers and bids (budgets, construction, retooling, trading assets): later Stages.
- Per-deposit IDs in the view, and anything in the view beyond the business's own circumstances.
- Structured traces.
- Any change to the sample setting, the turn, the report or the dev UI.

## Acceptance Criteria
- From a fresh sample world, each of the three center businesses' views holds the expected turn number, name, currency, the center's posted prices, its stockpile by perish group (starting silver as never perishing, its deposits' emission perishing at the end of turn 0), its cash, its factories with their conditions and technology names, each technology once with the rules' numbers, and its deposits per type (two arable land for the farm, one copper ore for the mine, none for the toolmaker). A business on a hex with no market gets a view with no prices. Nothing outside the business appears in any view, and building a view leaves the world unchanged.
- A decision with fractions for some factories, offers and bids for priced goods, and a trace is valid against its view, and so is an empty decision. Each refusal case is reported at its place with the expected message: a factory not in the view, an unknown good, a fraction below 0, above 1 or not a number, an amount that isn't a valid quantity, an offer or bid for the currency, and an offer or bid for an unpriced good; one decision with several problems reports all of them. An offer above the holdings and a bid beyond cash are valid.
- Every view, decision, request and reply written to JSON and read back is equal to the original, including quantities, fractions and turn numbers chosen to exercise exactness (values with no exact short decimal form, very large and very small values, and the largest turn number). A hand-written JSON decision reads as expected, and malformed JSON, a wrong type, an unknown key, a key written twice, a negative quantity and a non-finite number are each refused with a clear message.
- A reply with a wrong turn number, a decision for a business not in the request, or two decisions for one business is refused; a reply covering a subset of the request's businesses in a different order is accepted, and the businesses it leaves out count as having decided nothing.
- Every existing test passes unchanged: no setting, turn, report or dev UI text changes.
