# Step 05: stone-geary-demand

## Summary
The closed-form Stone-Geary demand as a pure function: given a hex's posted prices, what a POP holds, and its type's utility parameters, it returns the quantity of each good the POP wants to end up with, including the endowed goods it keeps as leisure. It handles the two cases the textbook formula doesn't: goods not traded on the hex, and income that can't cover the subsistence bundle. It also reports the income, the cost of the subsistence bundle, and which utility goods end up below subsistence, so Step 07 can build growth on them. Nothing calls it during a turn yet: Step 06 wires it into POP trade. It has its own tests and changes no setting data.

## Behavior

**Inputs.** Three things, all of which already exist in the engine.
- The hex's prices, as Step 02's market answers them: a posted price in the hex's currency, 1 for the currency itself, or "not traded here".
- What the POP holds: a list of goods and amounts. The function treats everything in this list as the POP's endowment for this turn and makes no assumption about where it came from. Step 06 decides what to pass; most likely the POP's whole stockpile, since POPs spend everything each turn. Durable goods and leftover currency that happen to be in the stockpile are income like anything else.
- The POP type's utility goods (subsistence amount and normalized share per good, from Step 03) and the POP's size N. Subsistence amounts are per individual, so the function works with the subsistence bundle times N. Because the demand is linear in N, computing per individual and multiplying by N gives the same answer; the spec may pick either.

**Tradable and untradable goods.** Every good involved (in the utility, held, or both) is either tradable on this hex (it has a price, or is the currency) or untradable. The optimization runs over the tradable goods only.
- An untradable good the POP holds is kept whole: its demand is the held amount. If it is in the utility it is consumed as leisure or the like; if not, it is simply held. It contributes nothing to income, since it can't be sold.
- An untradable utility good the POP doesn't hold has a demand of zero. It can't be bought.
- Among the tradable utility goods, the shares are renormalized to sum to 1 over that set. This is the exact conditional Stone-Geary demand when some quantities are fixed, not an approximation.

**Income and the subsistence bundle.**
- Income M is the value of every tradable held good at its price, with the currency at 1.
- The subsistence cost C is the sum over tradable utility goods of price times subsistence times N.
- Supernumerary income is M minus C. It may be negative.

**At or above subsistence (M ≥ C).** Each tradable utility good gets its subsistence times N plus its renormalized share of the supernumerary income divided by its price. A tradable held good that is not in the utility gets a demand of zero: it is all for sale. A pure necessity (share 0) gets exactly its subsistence. If no tradable utility good has a positive share (for instance the only share good is unpriced on this hex), the supernumerary income has nowhere to go and is demanded as currency, which is a good at price 1, so nothing is lost and the budget still balances.

**Below subsistence (M < C).** The POP spends everything, buying toward subsistence: every tradable utility good gets its subsistence times N, scaled by M divided by C. Every good gets the same fraction of its subsistence, including leisure, so a peasant facing a grain price spike sells more labor than its leisure subsistence allows and still falls short of grain in the same proportion. Tradable goods with no subsistence, and tradable held goods not in the utility, get zero. The rule is continuous at the boundary: when M equals C it gives exactly the subsistence bundle. C is positive whenever this case occurs, so there is no division by zero. A POP that holds nothing tradable has M of zero and, if C is positive, demands nothing tradable.

**The budget balances.** In every case the value of the demanded tradable goods at posted prices equals M, up to floating-point rounding. The function does no rounding and promises nothing about exact equality; Step 06 handles the difference when it trades.

**What the result reports.** In good ID order, the demanded quantity of every good involved, with zeros included so Step 06 can derive the trade plan: sell where holdings exceed demand, buy where demand exceeds holdings. Alongside the quantities: the income M, the subsistence cost C, the supernumerary income, and for each utility good whether its demanded quantity meets its subsistence times N. The POP is below subsistence if any utility good falls short. That covers both ways it can happen: income too small for the tradable bundle, and an untradable utility good the POP doesn't hold enough of (a peasant on the hex with no market has M and C both zero, but no grain). This Step does not define a utility index or a growth rate; Step 07 builds those from these numbers.

**Examples from the sample setting**, with N of 1 and a typical hex's prices (labor 1, grain 2, copper 5).
- A peasant (10 labor; labor subsistence 2 share 0.2; grain subsistence 2 share 0.8): M is 10, C is 6, supernumerary 4. Demand: labor 2.8, grain 3.6. It would sell 7.2 labor for 7.2 silver and buy 3.6 grain with it.
- An artisan (10 labor; labor subsistence 1 share 0.1; grain subsistence 2 share 0.4; copper share 0.5): M is 10, C is 5 (labor 1 at price 1, grain 2 at price 2; copper has no subsistence), supernumerary 5. Demand: labor 1.5, grain 3, copper 0.5. It would sell 8.5 labor for 8.5 silver and spend 6 on grain and 2.5 on copper.
- A peasant where grain costs 10: C is 22, so the fraction is 10/22. Demand: labor and grain each about 0.909, below subsistence on both.
- A peasant on the north-east hex with no market: everything is untradable. Demand: labor 10 (kept whole), grain 0. M and C are zero, and grain is below subsistence.

**Purity and determinism.** The function reads its inputs and returns a result. It changes no stockpile, makes no trade, and touches no world state. The turn is unchanged: the three parts, the report, and every existing turn line are exactly as Step 04 left them.

## Player-facing Changes
None. No setting file changes, and the sample setting is unchanged. The checker's output and the dev UI are unchanged. Step 08 displays the numbers this Step computes once Step 06 produces them each turn.

## Interactions
- **Dummy market (Step 02):** the source of prices, and of the three-way answer (price, 1 for the currency, not traded) that drives the tradable/untradable split.
- **POP types (Step 03):** the subsistence amounts and normalized shares. The renormalization over tradable goods reuses the same idea the loader applies to the whole set.
- **POPs (Step 04):** the size N, and the stockpile Step 06 will pass as holdings.
- **POP trade (Step 06):** the only caller. It computes demand against the hex's market, sells holdings above demand, buys goods below it, and consumes.
- **POP growth (Step 07):** defines its utility index and growth rate from the reported income, subsistence cost, supernumerary income, and per-good subsistence status.
- **Dev UI (Step 08):** shows each POP's utility and trade once Steps 06 and 07 produce them.
- **Quantity:** the arithmetic (division, scaling, sums of products) may need a few more operations on the engine's quantity type.

## Out of Scope
- Calling the function during a turn, trading, or consuming (Step 06).
- A utility index, growth rate, or any treatment of what "below subsistence" does to a POP (Step 07).
- Any rounding, smallest unit, or guarantee that the trade plan is exactly affordable after floating-point arithmetic (Step 06 handles the difference).
- Priorities among subsistence goods, saving, or any demand that isn't the closed form.
- Changes to setting files, the sample setting, the checker, or the dev UI.

## Acceptance Criteria
- With the sample's typical prices and N of 1, a peasant's demand is labor 2.8 and grain 3.6 with income 10, subsistence cost 6 and supernumerary 4; an artisan's demand is labor 1.5, grain 3 and copper 0.5 with income 10, subsistence cost 5, supernumerary 5 and no good below subsistence.
- Demand for N of 50 is 50 times the demand for N of 1, for both types.
- The value of the demanded tradable goods equals the income, within floating-point tolerance, in every test case including below subsistence.
- Below subsistence (a peasant with grain at 10): both goods are scaled to 10/22 of their subsistence, both are reported below subsistence, and supernumerary income is negative. At exactly M equal to C the result is the subsistence bundle and nothing is below subsistence.
- A tradable held good not in the utility gets demand zero (all for sale). A pure necessity gets exactly its subsistence.
- Untradable goods: a held one is demanded at its held amount and adds nothing to income; an unheld one gets zero and is reported below subsistence if it has a positive subsistence; the shares of the remaining tradable goods are renormalized. A POP on a hex with no market demands exactly what it holds.
- When no tradable utility good has a positive share, the supernumerary income is demanded as currency and the budget still balances.
- Currency held counts as income at price 1; a utility that names the currency treats it as a good at price 1.
- Holdings of nothing give income zero and, with a positive subsistence cost, zero demand for every tradable good.
- The function changes nothing it is given, and every existing test passes unchanged.
