# Step 06: pop-trade

## Summary
The first behavior in the "actors act" part of the turn, shaped the way the live market will expect. Each POP hands its hex's market a submission: its type's Stone-Geary parameters, its size, and what it holds. The market posts its prices, computes each submission's demand with Step 05's function, and settles the trades at those prices: every POP's sales first, then every POP's purchases. Each POP then consumes the utility goods it ended up with. Currency left over by rounding stays in the POP's stockpile and is income next turn. The turn report gains POP totals for what was sold, bought, and consumed, and the World keeps each POP's last outcome so Step 07 can read its utility and Step 08 can show it. The dummy market still posts fixed prices and trades any amount; what this Step adds around it is the structure Stage 05 fills in with aggregation, rationing, and price adjustment, so that Stage touches the market's inside and not the POPs.

## Behavior

**Where it happens.** The turn's three parts stay, and part 2 gains its first content. Part 1 (deposits emit, POPs are endowed) is unchanged. Part 2 runs the trade round on every hex, in hex ID order, then consumption. Part 3 (perishing) is unchanged. Part 2 runs in the same call as part 1: stepping the world ends the current turn, advances the clock, runs the start of the next turn, then its trade round. Creating a world runs turn 0's start and turn 0's trade round. So between calls the world is after part 2: every POP has traded and consumed, and the stockpiles hold what is left. One report line covers one whole turn's trade.

**The submission.** Each POP on a hex submits three things to the hex's market: its type's utility goods (subsistence and normalized share per good, from Step 03), its size N, and its holdings, which are its whole stockpile. Durable goods and leftover currency in the stockpile are income like the endowment, as Step 05 already treats them. The POP does not compute anything itself; its submission is the parameters, not an answer. Every POP submits, including a POP on a hex with no market.

**The trade round.** The market, given the submissions of all POPs on its hex:
1. **Posts prices.** In this Stage they are the fixed prices from the starting state; the currency's price is 1 and an unpriced good is "not traded here", exactly as Step 02 answers.
2. **Computes demand** for each submission with Step 05's function: the quantity of every involved good the POP wants to end up with, from its parameters, size, holdings, and the posted prices. The below-subsistence rule and the untradable-goods rules are Step 05's and are not restated here.
3. **Settles sales.** For every POP in POP ID order, for every good in good ID order whose holding exceeds its demanded quantity: when the demanded quantity is zero, the POP sells everything it holds of the good, so a good it does not want never leaves a rounding sliver behind; otherwise it sells the difference. The proceeds in currency enter the POP's stockpile as Step 02 describes. Untradable goods are never sold: Step 05 demands them at their held amount, so there is nothing to sell.
4. **Settles purchases.** After every POP's sales, for every POP in POP ID order, for every good in good ID order whose demanded quantity exceeds its holding, the POP buys the difference, capped at what its currency affords. The budget balances only up to floating-point rounding, so without the cap the last purchase could be refused for a few units in the last decimal place; with it the shortfall lands on the last good in ID order and is negligible. A POP with no currency buys nothing. Bought goods enter the stockpile as Step 02 describes.

Sales settle before purchases across the whole hex, not per POP, because in the live market a POP rationed when selling has less income than it planned, and every purchase must see the income the sales actually produced. With the dummy market, which trades any amount, the order makes no visible difference; the structure is set now so Stage 05 does not have to change it.

**Consumption.** After the trade round on its hex, each POP consumes: of each good in its type's utility, the smaller of what it now holds and what it demanded. Consumed goods are removed from the stockpile, soonest-perishing group first, like any removal. Leisure is the endowed good kept and consumed this way; it needs no special case. Nothing else is consumed: a held good that is not in the utility stays in the stockpile, whether it is currency demanded as a parking place for income when nothing with a share is priced, a durable good the POP could not sell, or an untradable good. With the dummy market a POP always holds exactly what it demanded (up to the rounding cap) when it consumes; the "smaller of" rule is for Stage 05, where rationing can leave a POP short.

**Leftover currency.** Whatever currency a POP holds after consumption stays in its stockpile. If the currency is durable, as the sample's silver is, it survives the end of the turn and counts as income in the next turn's demand, as Step 05 already provides. If the currency perishes, it perishes by its own rule like any other good. The engine does no rounding anywhere.

**What the World records.** After the trade round the World keeps, for each POP, the outcome of this turn: the demand it was computed (income, subsistence cost, supernumerary income, and per good the held, demanded and subsistence quantities, as Step 05 reports them), and per good what the POP sold, bought, and consumed, plus the currency it carried over. A caller can read the latest outcome of any POP. Step 07 derives growth from the demand numbers; Step 08 displays them. The outcome is replaced every turn.

**The turn report** gains three lists after `endowed`, each in good ID order and listing only amounts greater than zero: `sold` (the total of each good POPs sold on all hexes), `bought`, and `consumed`. The checker's turn line and the dev UI's turn log print them in that order after the existing lists, with `nothing` for an empty list. For the sample at turn 1, `sold` is unskilled labor only, `bought` is grain and copper, and `consumed` is unskilled labor, grain and copper.

**Edge cases.**
- A hex with no market: every submission gets a demand of "keep everything held", nothing is sold or bought, and the POP consumes its endowed utility goods as leisure. The sample's north-east peasant consumes its whole labor endowment and goes without grain every turn.
- A hex with no POPs, or a world with no POPs: the trade round does nothing and the three lists are empty.
- A POP below subsistence: it sells more of its endowed utility good than its leisure subsistence allows and buys toward subsistence, exactly as Step 05's demand says; this Step just executes it.
- A type with an empty endowment and no holdings: income zero, nothing sold, nothing bought, nothing consumed.
- A trade the exchange refuses (not enough goods, not enough currency, unpriced good, the currency itself) cannot happen after the demand and the cap, because sales never exceed holdings, purchases are capped, untradable goods are not traded, and the currency itself is never traded: proceeds and spending move it, and because the budget balances, what a POP holds of it after its purchases is its demanded quantity up to rounding, whether that is the parking place or a utility that names the currency. If it happens anyway it is a bug, and the round stops with a message naming the hex, the POP and the refusal rather than silently continuing.

**Determinism.** Hexes in hex ID order, POPs in POP ID order within a hex, goods in good ID order within a POP, sales before purchases, then consumption: the same setting stepped the same number of times gives the same world, the same outcomes, and the same reports.

**Examples from the sample setting**, per individual (every number scales with N), at a typical hex's prices (labor 1, grain 2, copper 5).
- A peasant holds 10 labor; demand is labor 2.8, grain 3.6. It sells 7.2 labor for 7.2 silver, buys 3.6 grain for 7.2 silver, consumes 2.8 labor and 3.6 grain, and ends the turn holding nothing but a rounding sliver of silver, if any.
- An artisan at the center holds 10 labor; demand is labor 1.5, grain 3, copper 0.5. It sells 8.5 labor for 8.5 silver, buys 3 grain for 6 and 0.5 copper for 2.5, consumes 1.5 labor, 3 grain and 0.5 copper.
- Turn 2 for either looks the same, because labor perished and was endowed again and any leftover silver is a rounding sliver.

## Player-facing Changes
No setting file changes, and the sample setting is unchanged. `check --turns` and the dev UI's turn log show the three new lists after `endowed` on every turn line, so the sample can be watched trading from turn 0. Inspecting a POP's outcome in the dev UI is Step 08.

## Interactions
- **Dummy market (Step 02):** the source of prices and the exchange the settlement uses (sell, sell everything, buy). The market gains the trade round around them; the posted prices and the exchange's rules are unchanged.
- **POP types (Step 03):** the utility parameters in the submission.
- **POPs (Step 04):** the size and the stockpile in the submission; the stockpile is what settlement and consumption change.
- **Stone-Geary demand (Step 05):** the only thing the market uses to turn a submission into quantities. Unchanged.
- **World and turn (Stage 01 Step 06):** part 2 gains the trade round and consumption; the report gains three lists; the World gains per-POP outcomes.
- **POP growth (Step 07):** reads each POP's recorded demand numbers. **Dev UI (Step 08):** shows each POP's outcome and the new report lists.
- **Stage 03 (industries):** submit through the same market round in their own form. **Stage 05 (market):** replaces the inside of the trade round: posted prices become adjusted prices, settlement gains rationing, and the aggregate curves are built from the submissions. The submission, the sales-before-purchases order, consumption, and the recorded outcomes stay.
- **Checker and dev UI turn line:** gain the three lists.
- **Test settings:** existing test settings keep loading and stepping. Those without POPs step unchanged, with their turn lines extended only by `; sold nothing; bought nothing; consumed nothing`. Those with POPs (`pops_placed`, `pops_one`) now have their POPs consume their utility goods, so their stockpiles and turn lines change accordingly; neither has a market, so nothing is sold or bought. The `pops_placed` hoarder type's utility changes from gold (its hex's currency, which it would otherwise consume every turn) to grain, which is unpriced there, so its gold is kept as the parking case and keeps accumulating, as Step 04's test expects.

## Out of Scope
- Price adjustment, rationing, market inventory, or any limit on what the market trades (Stage 05).
- Aggregating submissions into one demand per type; the market handles each submission on its own in this Stage.
- Growth or any change to N (Step 07), and showing outcomes in the dev UI (Step 08).
- Saving: POPs spend their whole income except for rounding leftovers.
- Rounding rules or a smallest unit of currency.
- Any actor other than POPs trading, and deposit ownership.
- Changes to setting files or the sample setting (test settings may change, as Interactions says), or to the checker beyond the turn line.

## Acceptance Criteria
- A fresh sample world has already run turn 0's trade round: a typical-hex peasant POP's stockpile holds no labor and no grain, and at most a rounding sliver of silver; its recorded outcome shows 7.2 labor sold, 3.6 grain bought, and 2.8 labor and 3.6 grain consumed per individual, times its size.
- The center artisan POP's outcome shows 8.5 labor sold and 3 grain and 0.5 copper bought and consumed per individual, times its size.
- The north-east peasant POP (no market) sells and buys nothing, consumes its whole labor endowment, and its recorded demand reports grain below subsistence.
- Stepping the sample world repeats the same outcomes every turn up to floating-point rounding (the silver sliver carried over is income next turn), and two worlds from the same setting stepped the same number of times are identical, outcomes and reports included.
- Every POP's purchases are capped at its currency: in a test where the budget's floating-point rounding would leave the last purchase a sliver short, the purchase is reduced rather than refused, and the leftover currency is in the stockpile and counts as income next turn.
- Currency carried over counts as income: a test POP that starts a turn with currency in its stockpile has a demand with that currency included in its income and buys accordingly.
- On a test hex with two POPs, every sale on the hex is settled before any purchase, and the result is the same as the per-POP order would give with the dummy market.
- A POP below subsistence (a test with grain at 10) sells and buys the scaled bundle Step 05 prescribes and consumes exactly what it bought and kept.
- Consumption removes only utility goods, up to the demanded quantity: a durable held good outside the utility and currency demanded as a parking place both stay in the stockpile.
- The report's `sold`, `bought` and `consumed` lists are totals over all POPs in good ID order; `check --turns` prints them after `endowed` on every turn line, `nothing` when empty, and the dev UI's turn log shows the same lines.
- A world with no POPs, and every existing test setting without POPs, step unchanged apart from the three added `nothing` parts. `pops_placed` and `pops_one` step as before except that their POPs consume their utility goods: in `pops_placed` the workers consume all their labor, the hoarder keeps its labor and its accumulating gold, and `consumed` is the workers' labor.
