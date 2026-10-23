# Step 01: order-lines

## Summary
This Step adds the order line, the one form every actor's market orders take from now on, and the arithmetic the live market runs on lines: evaluating a line at a price, summing the lines on one side of a good into one curve, and finding the price where the summed demand and supply curves cross. Everything is engine data and pure functions with their own tests; nothing is wired into the turn. Later Steps put lines into the decider protocol (Step 02), draw them for POPs (Step 03), settle trades on them (Step 04) and publish the summed curves in every view (Step 05).

## Behavior

### A line
A line is an actor's order for one good on one side of the market: a demand line says how much the actor buys at each price, a supply line how much it sells. A line is anchored at the price it was drawn at, and carries:

- the **anchor price**: the good's price when the line was drawn (a positive price, since it is a market price);
- the **quantity at the anchor price**;
- the **slope**: how much the quantity changes per unit of price. A demand line's quantity never rises with the price and a supply line's never falls with it, so a demand slope is at most zero and a supply slope at least zero. A slope of zero is allowed on both sides;
- the **maximum**: the most the actor would trade at any price;
- the **give-up price**: the last price at which the actor still trades. A buyer buys nothing above it; a seller sells nothing below it. Zero is allowed, so a seller with a give-up price of zero sells at any price, and a buyer with a give-up price of zero never buys.

The line is anchored at an explicit price, rather than at "the current price", because from Step 06 on a line stays on file while the price moves, and it must keep meaning what it meant when it was drawn.

**The quantity at a price** follows the straight line through the anchor with the given slope, clipped so it is never below zero and never above the maximum, and is zero beyond the give-up price regardless of what the line says. The give-up price is inclusive: at exactly that price the actor trades what the line says. So "sell everything at or above break-even" is a supply line with quantity equal to the holding, slope zero, maximum equal to the holding, and give-up price equal to break-even; nothing else is needed to express it. A line may therefore be a step, and the clipping and the give-up price can make the quantity jump at one price.

**Malformed lines** are refused, not fixed, in the spirit of the decider protocol: a demand line with a positive slope, a supply line with a negative slope, a quantity at the anchor above the maximum, and any number that isn't finite or that is negative where it can't be. The refusal says what is wrong. A line with a maximum of zero, or a quantity of zero at the anchor and a slope of zero, is well-formed and trades nothing; it isn't an error.

### A summed curve
The lines on one side of a good add up into one curve: at every price, the curve's quantity is the sum of the lines' quantities at that price. Because each line is piecewise linear with at most a few corners (where it reaches zero, where it reaches its maximum, and the give-up price), the sum is piecewise linear too, with at most a few corners per line, and it never goes the wrong way: a summed demand curve never rises with the price and a summed supply curve never falls with it.

The curve is represented as a **list of breakpoints**, each a price and a quantity, in rising price order. The quantity is linear between neighbouring breakpoints, constant before the first and after the last, and a jump is written as two breakpoints at the same price (the quantity before the jump, then after). Summing lines merges their breakpoints, so the result is exact, not sampled. The curve can be evaluated at any price, and the one-sided case is the empty sum: a curve of zero everywhere, written as no breakpoints. This list is the curve's only form: Step 05 publishes it in the business view as it is, and the dev UI draws it from the same list, so there is no second, simplified form to keep in sync. Anything an actor wants, such as the slope near the current price or the area between the curves, can be computed from the list.

### The crossing
Given a good's summed demand curve, summed supply curve and current price, the crossing finds the price the market sets and how much trades. Excess demand (demand minus supply) never rises with the price, so there is exactly one place where it changes from positive to negative, possibly a single price and possibly a range.

- **A clean crossing.** Where demand and supply are equal and positive at one price, that is the price, and everyone on both sides trades the full quantity of their own line at it.
- **A crossing at a jump.** Where the sign changes at a price at which one curve jumps (a step order, or a line reaching its maximum or give-up price), that price is the price, and the two sides disagree on quantity. The side that wants less trades its full quantity; the side that wants more is cut to match by **one proportion**, so every actor on the long side trades the same share of its line's quantity. The crossing reports the price, the traded quantity and that proportion for each side (one of them is 1). This is the project's usual way of trimming a request that is too big, and it means a step order is honoured exactly for as long as the other side can absorb it.
- **A range of equal prices.** Where demand and supply are equal and positive across a whole range of prices (both curves flat there), the price is the one in that range closest to the current price. It is deterministic and moves the price only as far as it must; nothing in the market moves a price without a reason in the orders.
- **Nothing would trade.** When one side has no lines, or both have lines but there is no price at which both quantities are positive (every buyer gives up below where every seller starts), the price stays where it is and nothing trades. The crossing reports the unchanged price, zero traded, and what each side wants at that price as the unfilled quantity (at most one side is positive). The one-sided case and the never-overlapping case follow the same rule on purpose: a price move with no trade behind it would be a signal the orders don't support, and the unfilled quantities already carry the signal. How the unfilled quantities become shortages and gluts in the world is Step 04's business.
- **A crossing at price zero.** Sellers may supply more than buyers want even for free, so the sign change can fall at a price of zero. The crossing reports that honestly. Keeping prices positive in the world is Step 04's decision, since that's where a crossing becomes a world price; this Step doesn't invent a floor.

The crossing works on the summed curves only, never on the individual lines, so the market's rule is mechanical and the same whatever the lines came from, as the vision's neutrality rule requires. The current price is an input so that the range and no-trade rules can refer to it; the crossing never reads anything else about the world.

## Player-facing Changes
None. Nothing is wired into the turn, the setting format doesn't change, and the dev UI doesn't change. Prices still come from the dummy market until Step 04.

## Interactions
- **Quantities and prices.** Quantities, prices and the maximum are the engine's quantity type (finite, never negative); the anchor price is positive, like every market price. The slope is a signed rate, not a quantity.
- **The dummy market and the trade round** are untouched. Nothing calls the new functions yet.
- **Step 02 (order-protocol)** carries lines across the decider boundary and validates them against the view; the malformed-line refusals here are what that validation reports, alongside the usual good checks.
- **Step 03 (pop-orders)** draws POP lines in this form.
- **Step 04 (settlement)** sums each good's lines, calls the crossing, and trades every actor along its own line at the result, applying the long side's proportion and the holdings trim. It also decides the price floor and what the no-trade result means for the world.
- **Step 05 (business-orders)** publishes the summed curves as the breakpoint lists defined here.
- **Step 10 (dev-ui-market)** draws them.

## Out of Scope
- Putting lines into the decision vocabulary, the JSON protocol or the Python client (Step 02).
- Drawing lines for POPs (Step 03), settling trades or changing any price in the world (Step 04), publishing curves in views (Step 05), orders staying on file (Step 06).
- Trimming a line to what the actor actually holds, and the cash cap on purchases: those depend on the actor's stockpile, and belong to settlement (Step 04).
- A price floor, and what the world does when the crossing is at zero (Step 04).
- Taxes and subsidies as wedges between the summed curves; that shape is kept in mind but not built.
- Any simplified or sampled form of a curve.

## Acceptance Criteria
- A line can be built from an anchor price, a quantity, a slope, a maximum and a give-up price, and the quantity it gives at any price follows the line, never goes below zero or above the maximum, is zero beyond the give-up price, and equals the line's own quantity at the give-up price itself.
- A demand line with a positive slope, a supply line with a negative slope, an anchor quantity above the maximum, and a non-finite or wrongly negative number are each refused with a message saying what is wrong; a line that trades nothing is accepted.
- Summing lines gives a breakpoint list that evaluates, at every price, to the sum of the lines' quantities, including at every breakpoint and on both sides of every jump; summing no lines gives the empty curve; a summed demand curve never rises and a summed supply curve never falls with the price.
- The crossing finds the clean crossing price and quantity; at a jump it returns the jump price, the smaller quantity and a proportion below 1 for the long side only; across a range of equal prices it returns the price closest to the current price; with one side empty, or two sides that never overlap, it returns the current price, zero traded and the unfilled quantities; and it reports a crossing at zero as zero.
- Tests cover each of those cases, including step orders, lines that reach their maximum, several lines whose corners coincide, and a mix of demand and supply steps at the same price.
