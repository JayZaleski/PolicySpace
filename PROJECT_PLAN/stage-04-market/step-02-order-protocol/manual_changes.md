# Step 02: order-protocol — Manual Changes

This is a small change. Nothing a setting author writes changes, and the sample gives the same numbers as before: the same trades, cash, factory conditions, traces and turn lines. What changes is how a decision describes its offers and bids, and the manual's wording about decisions should follow it.

## What users can now see or do
- **A decision's offers and bids are now order lines.** For each good a business sells or buys, its decider sends one line: how much it trades at the current price, how that changes with the price (the slope), the most it would trade at any price, and a **give-up price** past which it trades nothing (a seller sells nothing below it, a buyer buys nothing above it). The engine ties the line to the price the business saw in its view, so the decider doesn't send a price. All four numbers are required.
- **The arithmetic rule plans exactly as before** and sends each amount as a flat line: "sell this much at or above the current price", "buy this much at or below it". So its behaviour, its trace and the sample's figures don't change.
- **The market still trades at the posted prices** until a later Step of this Stage. Each line trades its quantity at the posted price, or nothing if its give-up price rules the posted price out (a buyer whose give-up is below it, a seller whose give-up is above it). Slopes and maximums don't do anything yet. Trimming offers to what is held and the cash cut on bids work as before.
- **A new reason for refusal:** a malformed line refuses the whole decision, and the business does nothing that turn. Examples are an offer whose quantity falls as the price rises, a bid whose quantity rises with the price, a quantity above the maximum, or a negative number. The developer UI lists each such error with its place, like the existing ones.
- **Developer UI trade table:** the columns don't change. For a valid decision, Offered and Bid are what each line came to at the posted price. For a refused decision, they are each line's quantity as sent. The slope, maximum and give-up price aren't shown yet (a later Step of this Stage shows them).
- Nothing was removed.

## Where it fits
- **"Businesses" → "How the arithmetic rule decides":** in the opening sentence that defines a decision ("what to offer for sale, what to bid for, ..."), say that each offer and bid is a line with a quantity at the current price, a slope, a maximum and a give-up price. After the bullet about selling the surplus and bidding for the shortfall, add a sentence saying the rule sends these as flat lines at the posted price. The trace example stays as it is.
- **"Running a setting" → "How businesses trade and produce":** the sentence defining a decision should match the one above. Add a malformed line to the examples of a refused decision ("a factory it doesn't own, or a good its hex doesn't price"). Add one or two sentences before the bullets: for now each line is traded at the posted price, at its quantity, unless its give-up price rules the posted price out. The sample table and the cash-cut example stay as they are.
- **"The developer UI" → "Inspecting a hex":** in the trade-table bullet, say that Offered and Bid are what each line came to at the posted price, that a refused decision shows each line's quantity, and that a line's other numbers aren't shown yet. In the bullet about refused decisions, a malformed line can be added as an example error. The sample panel text doesn't change.
- **"Checking a setting":** the sentence listing what makes a decision break the rules ("a factory it doesn't own, a good with no price") could add "a malformed order line". This is optional.
- No new section is needed.

## Links to docs/
- [docs/decider.md](docs/decider.md): the decision, the order line in a decision, the validation table with the line messages, and the JSON format with its hand-written example. Link it where the manual already points readers to it for the decision.
- [docs/orders.md](docs/orders.md) (new this Stage, from Step 01): the line's shape and its rules. Link it where order lines are first described.
- [docs/trade.md](docs/trade.md): how a line becomes an amount in the trade round under the dummy market. It is already linked from "How businesses trade and produce".
- [docs/python.md](docs/python.md): the rule's flat lines, for readers writing their own decider.

## Examples, diagrams, demos
- An optional short example in "How businesses trade and produce": the sample farm's tool bid is written as a line (buy 0.5 tools, slope 0, maximum 0.5, give-up 12, the posted price) and so trades its full 0.5. Next to it, the same bid with a give-up price of 10 buys nothing. This shows the give-up price without implying that slopes do anything yet.
- No diagram or interactive demo for now. A chart of a sloped line belongs with the Steps that make slopes matter (settlement at the crossing, and the rule that sends real lines). Until then, a chart would suggest behaviour the market doesn't have.

## Earlier Steps
Step 01's `manual_changes.md` said the Stage had no user-facing changes yet. That is still true of Step 01, and nothing in it is superseded. This Step is the first in Stage 04 to touch the manual, and it is the first place where `docs/orders.md` is worth linking from the manual.
