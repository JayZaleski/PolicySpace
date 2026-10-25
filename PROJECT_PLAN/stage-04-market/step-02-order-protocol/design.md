# Step 02: order-protocol

## Summary
This Step puts Step 01's order line into the decider protocol. A business's offers and bids stop being fixed quantities and become lines: one line per good it wants to sell and one per good it wants to buy, each carried across the boundary as plain data, checked against the view with Step 01's rules beside the usual good checks, and written and read losslessly on both sides. The arithmetic rule is changed only as far as the format forces: it sends the quantities it computes today as flat lines, so every existing test keeps its numbers. The trade round still runs against the dummy market, trading each line's quantity at the posted price; Step 04 replaces that with the crossing. The point of doing this first is that Steps 03 to 05 can then build on one order form for every actor, with the protocol, the Python client and the recordings already agreeing on it.

## Behavior

### A line in a decision
A decision's offers and bids are still two tables keyed by good name, with one entry per good, but each entry is now a line instead of an amount. A line in a decision carries four numbers:

- the **quantity** the business trades at the current price;
- the **slope**: how the quantity changes per unit of price;
- the **maximum** it would trade at any price;
- the **give-up price**: the last price at which it still trades (a seller sells nothing below it, a buyer buys nothing above it; zero is allowed).

**The engine anchors the line.** The decider does not send an anchor price. The engine anchors every line at the posted price of that good in the view the decision answers, which is the price the decider saw. This matches the vision's wording (a quantity at the current price and a slope), leaves no redundant field that could disagree with the view, and means a line kept on file from Step 06 on is anchored at the price the actor last looked at. Posted prices are always positive, so the anchor is always a valid one.

**All four numbers are required.** A line with a missing number is malformed and refused with a message saying what is missing, like any missing key in the protocol. There are no defaults, because a default that depends on another field (a maximum equal to the quantity, say) would be a new kind of rule for both readers, and a decider that forgot a field would silently get a different order instead of an error. A line without a give-up price is refused, as the vision requires.

### Validation
A decision is validated against its view as before, reading the whole decision and reporting every problem at its place. For each offer and each bid, two kinds of check apply, and both are reported where they fail:

- **The good checks stay as they are:** an unknown good, the hex's own currency, and a good with no posted price on the hex are each refused with the existing messages.
- **The line checks are Step 01's:** an offer (a supply line) with a negative slope, a bid (a demand line) with a positive slope, a quantity above the maximum, and any number that is not finite or is negative where it can't be. The refusal says what is wrong, in Step 01's words, so one set of rules and messages describes a malformed line wherever it is found.

A line that trades nothing (a maximum of zero, or a quantity of zero with a slope of zero) is well-formed and valid; it resolves to nothing. Asking for more than the business holds or can pay is still not an error: a line that promises more than the stockpile holds is a real request that is too big, and the trade round trims it as it trims an offer today.

A malformed line refuses the whole decision, as any other problem does, and the business idles for the turn under the host's existing failure semantics.

### The dummy round
Until Step 04 the market still posts fixed prices and trades any amount at them. A line is resolved by evaluating it at the posted price: because the line is anchored there, that is the line's own quantity, or zero when the give-up price excludes the posted price (a buyer whose give-up is below it, or a seller whose give-up is above it). The resulting amount is the business's offer or bid for the round, and everything after that is unchanged: offers are trimmed to holdings, bids are valued at the posted prices after the hex's sales and scaled by one proportion when they cost more than the cash held, and the business's outcome records the amount asked beside the amount done. Nothing about prices moves in this Step.

### The arithmetic rule
The rule plans exactly as it does now: the margin test, the fractions, the needs, the surplus and shortfall per good, and the cash budget are untouched. Only the form of what it sends changes. For each good it would have offered or bid, it sends a flat line: the quantity and the maximum are the amount it computes today, the slope is zero, and the give-up price is the posted price on both sides, so the line says "sell this much at or above the current price" or "buy this much at or below it". That is the honest translation of a fixed quantity at the posted price. Any other give-up (zero for sales, some large number for bids) would express a willingness the rule never had, and there is no natural finite ceiling for a bid. Under the dummy round each such line trades its full amount, so the rule's behaviour, its trace and the engine's figures for the sample are exactly today's. Goods with a zero amount, the currency and unpriced goods are still never sent. Rewriting the rule for the live market is Step 05.

### Serialization and the Python client
A line is written as a table of its four numbers, in a fixed key order, inside the offer or bid entry for its good, and read back exactly, under the protocol's lossless rule and its strict reading (unknown, missing and repeated keys and non-finite numbers refused). The Python client gains the same line as a dataclass, with the decision's offers and bids becoming tables of lines, read as strictly and written in the engine's layout, so a message the engine writes still comes back from Python byte for byte. The protocol's second implementation changes wherever the first does, and nowhere else.

### Recordings
The reply half of the sample's turn-0 recording is regenerated, since the rule now answers with lines. The request files, hand-written and machine-written alike, carry no decisions and don't change. The engine's fixture test and the Python test keep the recording honest as before, and the replay decider answers from the regenerated file. Engine tests that build decisions directly use lines in the new form.

## Player-facing Changes
None in the setting format, the turn, the report or the figures the sample produces. In the dev UI, each business's trade table keeps its Offered, Sold, Bid and Bought columns, where the offered and bid figures are the amount each line resolved to at the posted price; a refused decision's rows show each line's quantity, as what the decider asked for. The line's slope, maximum and give-up price aren't shown until Step 10.

## Interactions
- **Order lines** (Step 01): the line, its refusal rules and messages, and evaluating a line at a price are what this Step carries across the boundary and resolves with.
- **Business view and decisions** (Stage 03 Step 05): the decision's offers and bids change shape; validation gains the line checks at the same places; the two-layer JSON reading and the lossless rule apply to the new numbers.
- **Business script** (Stage 03 Step 06): the rule's output form changes; its plan does not. The Python protocol module mirrors the engine's format.
- **Decider host** (Stage 03 Step 07): a decision refused for a malformed line idles its business under the existing semantics; the replay decider reads the regenerated recording.
- **Business trade** (Stage 03 Step 08): resolving a decision now evaluates each line at the posted price before the round; trims, the cash cap and the outcome are unchanged.
- **Dev UI businesses** (Stage 03 Step 09): the trade table shows the resolved amounts, as described above.
- **Later Steps:** Step 03 draws POP lines in the same form; Step 04 settles every line at the crossing instead of the posted price; Step 05 rewrites the rule to send real lines and publishes the market's curves in the view; Step 06 keeps lines on file; Step 10 shows them.

## Out of Scope
- Moving any price, summing lines or using the crossing in the turn (Step 04).
- Drawing lines for POPs (Step 03), market data in the view and a rule that uses it (Step 05), orders staying on file (Step 06).
- Showing a line's slope, maximum or give-up price in the dev UI (Step 10).
- Bug 01 (the rule's bids disagreeing with its fractions by a speck): the rule's amounts are unchanged here, and Step 05 closes it.
- Any new verb, any change to the view, the sample setting, the turn report or the failure semantics.

## Acceptance Criteria
- A decision whose offers and bids are lines (quantity, slope, maximum, give-up price per good) reads from JSON, validates against its view and writes back identically; the engine anchors each line at the good's posted price in that view.
- A line with a missing number is refused as a format error naming the missing key. A sell line with a negative slope, a buy line with a positive slope, a quantity above the maximum, and a negative or non-finite number are each refused at the offer's or bid's place with Step 01's message; the good checks (unknown good, currency, no posted price) still apply and are reported alongside; one decision with several problems reports all of them. A line that trades nothing is valid.
- A valid line resolves in the dummy round to its quantity at the posted price: a flat line trades its amount; a bid whose give-up is below the posted price, or an offer whose give-up is above it, trades nothing. Offers above holdings and bids beyond cash are trimmed and recorded as before.
- The arithmetic rule sends flat lines with the quantity and maximum equal to today's amounts, slope zero and give-up equal to the posted price, and never sends a line for a zero amount, the currency or an unpriced good. The sample stepped under the real decider gives the same trades, cash, conditions, trace phrases and turn lines as before this Step.
- The Python client reads and writes lines in the engine's exact layout: the exactness request still comes back byte for byte, the regenerated sample recording round trips in pretty form, and the engine's fixture test confirms the recording's request equals the engine's and its reply validates with one valid answer per view.
- Every existing test passes, with expectations changed only where decisions are built or compared in the old form.
