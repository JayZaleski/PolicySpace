# Step 08: business-trade

## Summary
This Step gives the business its turn. The decision point (Step 07) already serves every business its view and stores its answer; nothing acts on it, and factories never run or wear. This Step resolves the stored decisions: each business's offers and bids go through the hex round beside the POPs, with every sale on the hex settled before any purchase; purchases are capped by the cash the business holds after its sales; then its factories run, wear and are maintained with Step 04's functions, in Step 04's order, and the output enters its stockpile as this turn's goods, to be sold next turn. Labor bought this turn is used this turn and perishes at its end. The World keeps a per-business outcome (what was asked beside what was done, with Step 04's per-factory records), and the turn report and turn line gain business totals. The cash cap is the seam for operating budgets, maintenance the seam for expansion, and the decider the seam for richer strategy.

## Behavior

### Where it happens
The turn's three parts stay. Part 2 becomes: the decision point, then the hex round on every hex in hex ID order, then consumption and factory operation on that hex, then growth once every hex is done. Between calls the world is still after part 2: every business has traded and operated, every POP has traded, consumed and grown.

On each hex, the POPs and the businesses whose answer is a valid decision take part in one round together. The settlement passes stay as Step 06 of Stage 02 fixed them, with the businesses added behind the POPs:

1. **Sales.** POPs in POP ID order, as now; then businesses in business ID order, each selling its offers.
2. **Purchases.** After every sale on the hex: POPs in POP ID order, as now; then businesses in business ID order, each buying its bids under the cash cap.
3. **Consumption and operation.** POPs consume, as now. Then each business on the hex, in business ID order, operates its factories.

Every sale settles before any purchase across the whole hex, POPs and businesses alike, because in the live market a seller that is rationed has less income than it planned, and every purchase must see the income the sales produced. With the dummy market the order is invisible; POPs-then-businesses within a pass only fixes determinism. The order among businesses is likewise invisible: a business trades only against the market and operates only on its own stockpile, so nothing one business does can reach another in the same turn.

### Resolving a decision
A valid decision names offers and bids per good and two fractions per factory. Validation (Step 05) has already made sure every good is priced on the hex and isn't the currency, and every fraction is in range, so the engine only has to apply the world's limits:

- **Offers.** The business sells the amount offered; if it holds less, it sells everything it holds. Selling draws soonest-perishing groups first, as every removal does. The proceeds enter the business's stockpile as currency of this turn.
- **The cash cap.** After the hex's sales, the engine values the business's bids at the posted prices. If their total cost exceeds the currency the business holds, every bid is scaled by one proportion, the cash held over the total cost, and the business buys the scaled amounts. One proportion for every bid mirrors Step 04's proportional sharing of a shortage and the arithmetic rule's own budgeting (Step 06): a cash-short business buys a smaller bundle in the proportions it asked for, rather than all of one good and none of another in good ID order, which would spend cash on inputs its factories then can't use. Scaled purchases balance only up to floating-point rounding, so the last purchase is capped at the currency actually left, as POP purchases are; a business with no currency buys nothing.
- **The run level is not touched by the engine.** The fractions the decision carries are handed to Step 04's functions as they are. When the cash cap has cut the bids, the factories find fewer inputs and maintenance goods than they need, and Step 04's cuts reduce the run and maintenance fractions by the shortage, in the same proportion across the business's factories. That is how "when cash is short the run level is scaled down" is realised: through the one safety net that already exists, without the engine having to know what a bid is for or derive bids from fractions, which the protocol leaves to the decider.
- **Offering and bidding the same good** is allowed (Step 05) and needs no special case: the offer is sold in the sales pass and the bid bought in the purchase pass.
- **The exchange refusing a trade** is an engine bug, as for POPs: offers are trimmed to holdings, purchases are capped, and validation has excluded unpriced goods and the currency. The round stops with a message naming the hex, the business and the refusal rather than continuing silently.

### Operating the factories
After the hex's round and the POPs' consumption, each business operates its factories with Step 04's functions in Step 04's order: run, then wear, then maintain. Inputs and maintenance goods leave the business's stockpile; output enters it after every one of the business's factories has run, as goods added this turn, perishing by the good's own rule. So this turn's output is in the stockpile at next turn's decision point, where the view shows it and the business can offer it; that is the one-turn lag the vision describes. Labor bought in the purchase pass is in the stockpile when the factories run and perishes at the end of the turn like any labor.

A business on a hex with no market has no offers or bids (validation refuses them where nothing is priced) and operates its factories from whatever it holds: its deposits' emission and its starting inventory.

### Businesses that decided nothing
A business whose answer is nothing, or a refused decision, takes no part in the settlement passes and operates its factories at run 0 and maintenance 0. Its factories still wear: wear happens every turn whether or not a factory ran (Step 04), so an idle business's factories run down and eventually shut down. This is the first Step in which stepping a world changes a factory's condition, under any host, the idle host included.

### What the World records
After operating, the World keeps one outcome per business for the current turn, replaced every turn and readable by a caller:

- **Trade, per good:** the amount offered beside the amount sold, and the amount bid beside the amount bought, in good ID order; and the scaling proportion the cash cap applied, 1 when the bids were within the cash. A cut shows as asked above done, the way Step 04's records show a cut order.
- **Cash:** the currency held at the start of the round, after the sales pass and after the purchase pass.
- **Factories:** Step 04's record for each of the business's factories: the run fraction asked for and done, the inputs used and output made, the slice lost to wear, the maintenance fraction asked for and done with the goods it cost, and the condition before and after.

The decision itself, its trace and any validation errors are not copied into the outcome: they are already the business's stored answer (Step 07), and Step 09 shows the two side by side. A business that decided nothing has an outcome with nothing offered, bid, sold or bought, and factory records showing run 0, maintenance 0 and the wear.

### The turn report and turn line
The report gains four lists, each in good ID order and listing only amounts greater than zero, totalled over every business on every hex: what businesses **sold**, what they **bought**, what their factories **produced** (output made), and what their factories **used** (inputs and maintenance goods consumed). They are separate from the POP lists, so the POP numbers keep their meaning. The checker's turn line and the dev UI's turn log print them after `population`, in that order, each as a list or `nothing`, in the style of the existing lists.

### Determinism
Hexes in hex ID order; within a hex, POPs in ID order then businesses in ID order within each pass; goods in good ID order within a business; every sale before every purchase; then consumption, then operation in business ID order; then growth. The same setting stepped the same number of times under the same deciders gives the same stockpiles, conditions, outcomes and reports.

## Player-facing Changes
No setting file changes, and the sample setting is unchanged. The checker's `--turns` and the dev UI's turn log show the four business lists on every turn line, so the sample's chain can be watched running: the farm and the mine sell their output and buy what their deposits don't cover, the toolmaker buys copper and sells tools, and each maintains its factory. Inspecting a business's outcome and its factories in the dev UI is Step 09.

## Interactions
- **Business view and decisions** (Step 05): the valid decision resolved here; the rule that over-asking is valid and trimmed at resolution, with asked beside done recorded.
- **Decider host** (Step 07): the stored answers are the input; a refused or missing answer means an idle business. The decision point keeps its place before the round.
- **Factory mechanics** (Step 04): run, wear and maintain, in that order, called once per business after its hex's round; their records go into the outcome; their cuts are how a cash cut reaches the run level.
- **Business script** (Step 06): unchanged. Its plan already fits its budget, so the cash cap is a safety net under it. Its assumption that output is sold next turn is what the one-turn lag delivers.
- **Factories** (Step 03): conditions now change during play; a factory's `condition` is written back after operation.
- **Businesses** (Step 02): the stockpile trades, pays and receives; its cash is its currency.
- **Trade round** (Stage 02 Step 06) and **dummy market** (Stage 02 Step 02): the hex round gains a second kind of trader, keeping sales before purchases across everyone on the hex; the exchange is unchanged. The round stays the seam Stage 04 fills with rationing and price adjustment.
- **World and turn** (Stage 01 Step 06): part 2 gains operation; the World gains per-business outcomes; the report gains four lists.
- **Checker and dev UI turn line** (Stage 01 Steps 07 and 08): print the four lists.
- **Later Steps:** Step 09 shows each business's answer and outcome. Stage 04 breaks the dummy market's any-amount assumption; the resolution rules here (trim to holdings, cap to cash, share shortages) already cope with being sold or bought less than asked. Stage 06 plugs operating budgets into the cash cap; Stage 07 plugs expansion into maintenance.

## Out of Scope
- Showing outcomes, decisions, traces or factory records in the dev UI beyond the turn line: Step 09.
- Any change to the protocol, the view, validation, the Python package or the arithmetic rule.
- Scaling run or maintenance fractions in the engine, or any engine rule that derives bids from fractions.
- Operating budgets, dividends, POP ownership (Stage 06); construction, scrapping, retooling, trading factories or deposits (Stage 07).
- Rationing, price adjustment or any limit on what the market trades (Stage 04).
- Businesses trading with each other directly, or across hexes.
- Changes to the sample setting.

## Acceptance Criteria
- From a fresh sample world, stepped several turns under the real `arithmetic` decider: at turn 0 each center business sells its priced surplus and buys its shortfall as its decision says, its factory runs at capacity and is maintained, and its output is in its stockpile after the step; at turn 1 that output is offered and sold, and each business's cash is higher than the turn before. The farm's factory stays at full condition, the mine's climbs toward full within one construction slice a turn, and the toolmaker's stays full. No decider failure is reported, and two worlds from the same setting stepped the same number of times are identical, outcomes and reports included.
- Offers above holdings sell what is held, and the outcome shows the offer beside the smaller sale. A bid within cash is bought in full with proportion 1 and the cash figures in the outcome match the start, after-sales and after-purchases balances.
- A cash-short business (a test decision bidding beyond its cash after sales) has every bid scaled by the same proportion, shown in the outcome; its purchases spend at most its cash, with any rounding sliver left as currency; and its factory's run fraction done is cut by the same proportion through the input shortage, while the fraction asked is the one the decision sent.
- Sales before purchases: on a test hex with POPs and a business, every sale on the hex settles before any purchase, and the result equals what settling each trader alone would give with the dummy market.
- A business that decides nothing, or whose decision was refused, sells and buys nothing, its factories make nothing, and they wear one slice; after enough turns they are shut down. A business on a hex with no market operates its factories from its own stockpile and trades nothing.
- Labor bought in a turn is used by the factories that turn and never appears in `perished`, up to floating-point slivers (a decider whose bids don't agree exactly with its fractions can leave a speck of labor, which perishes and prints on the turn line); output appears in the stockpile only after operation and perishes by its good's rule.
- The report's four business lists are totals over all businesses in good ID order; `check --turns` and the dev UI's turn log print them after `population`, `nothing` when empty. Under the idle host every list is `nothing`.
- Existing tests pass with expectations changed only where this Step changes behavior: turn lines gain the four parts, and factory conditions after a step reflect wear.
