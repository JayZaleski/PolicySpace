# Step 02: dummy-market

## Summary
Every hex may have a dummy market: a stub that posts fixed prices in the hex's currency good and buys or sells any amount at those prices. The prices are starting state in a `[[market]]` section. The market has no stockpile of its own: goods sold to it vanish, goods bought from it appear from nothing, and currency is created and destroyed the same way. This Step also adds the first way to take goods out of a stockpile, deciding that the soonest-perishing group goes first. It is the seam that Stage 05 replaces with the live market, and it gives Steps 05 and 06 the prices and the exchange that POP trade needs. No actor trades yet.

## Behavior

**In the setting folder.** A `[[market]]` entry in `start/` names a hex by its axial coordinates `q` and `r`, and posts prices in a `prices` table whose keys are good names and whose values are prices in the hex's currency. A price is a number (whole or with decimals) greater than zero and finite. Prices are state rather than rules because the live market changes them, so `[[market]]` in `rules/` is the usual "belongs in start/" error. Errors, each reported at the offending place: a missing, wrongly typed, or out-of-range coordinate; coordinates that match no hex; a second entry for the same hex (the message names the first); a price that is not a number, is zero or negative, or is infinite or NaN; a `prices` value that is not a table; an unknown key in the entry. A good name in `prices` that matches no good is reported as an unknown good in the loader's second phase, after every file has been read, and like other reference checks it is skipped when the loader could not see every file. Listing the hex's own currency in `prices` is an error, because its price is 1 by definition. That check is also made in the second phase, since it needs the hex's currency resolved. An entry with an empty `prices` table is valid and means the market trades nothing.

**Which goods are traded.** A good is traded on a hex only if the hex's market posts a price for it. A good with no posted price is not traded there: the market neither buys nor sells it. This keeps settings flexible: a good may exist in the rules before anything produces or wants it. A hex with no `[[market]]` entry is valid and trades nothing, which later Steps treat the same way as a hex where every good is unpriced. The hex's currency good is never traded against itself; asking its price gives 1.

**Prices in the engine.** The loaded starting state carries each hex's posted prices, and the World takes them over as per-hex state next to the currencies and the hex stockpiles. Anyone with the World can ask, for a hex and a good, the posted price in the hex's currency, and gets one of three answers: a price, 1 for the currency itself, or "not traded here." Prices are stored per hex in hex ID order and per good in good ID order, so loading the same folder always gives the same result. Nothing changes prices during play.

**Exchange.** The market exchanges goods against an actor's stockpile; it holds nothing itself.
- **Selling** removes the requested amount of the good from the stockpile and adds the amount times the price, in the hex's currency, to the same stockpile.
- **Buying** removes the amount times the price in currency from the stockpile and adds the requested amount of the good, which enters the stockpile as if it appeared in the current turn, so a perishable good bought in turn T perishes at the end of turn T plus its period limit minus one. Currency received from a sale enters the same way; if the currency is a perishable good, it perishes like any other unit that appeared this turn.
- Every trade is **all-or-nothing**. It is refused, and nothing changes, when the stockpile holds less of the good than is being sold, holds less currency than the purchase costs, the good has no posted price on that hex, or the good is the hex's own currency. The caller is told which of these happened. A trade of zero amount is accepted and changes nothing.
- There is also a way to **sell everything** of one good that the stockpile holds, so that a POP selling all the labor it does not keep as leisure never fails on a rounding difference.

**Taking goods out of a stockpile.** A stockpile can now give up an amount of a good. The amount is taken from the group that perishes soonest first, then from the next, so what would spoil first is used first and durable units are kept longest. Removing more than the stockpile holds is refused and takes nothing; there is no partial removal. Groups emptied by the removal disappear, and a good left with no groups disappears, so a stockpile stays sparse. A removal of zero changes nothing. The amount of currency a trade moves is the floating-point product of price and amount; the engine does no rounding.

**Determinism and the turn.** The turn's three parts are unchanged: no actor trades in this Step, so nothing calls the exchange during a step, and the turn report is unchanged. Trades, when later Steps make them, happen in the "actors act" part of the turn. Stepping the World does not change prices.

## Player-facing Changes
None visible yet. A setting author adds `[[market]]` entries to `start/`. The sample setting gains a `start/markets.toml`: most of its 19 hexes post the same prices in silver (unskilled labor 1, grain 2, copper 5); the center hex also prices unimproved grain at 0.5 and unimproved copper at 1; and one edge hex has no market. So the sample has a hex that trades everything, hexes that trade some goods, and a hex that trades nothing. Nothing in the sample's prices has to be consistent with production, since nothing is produced for sale yet. The checker's success line counts markets alongside the other kinds of thing. The dev UI shows a hex's prices in Step 08.

## Interactions
- **Goods (Stage 01 Step 03):** price keys are references to goods, resolved against the goods registry like a resource type's `produces`. The perish property decides how bought goods and received currency enter a stockpile.
- **Hex map (Stage 01 Step 04):** market coordinates resolve to a hex like a deposit's. The map's own data is unchanged.
- **Stockpile and World (Stage 01 Step 06):** the stockpile gains removal, soonest-perishing group first, and the World gains per-hex prices. Emission and perishing are unchanged.
- **Currency (Step 01):** prices are posted in the hex's currency; the currency's own price is 1 and it cannot be listed or traded.
- **Stone-Geary demand (Step 05):** takes the posted prices as input and must handle "not traded here." **POP trade (Step 06):** sells labor and buys goods through the exchange, in the "actors act" part of the turn. **Dev UI (Step 08):** displays prices.
- **Stage 05 (market):** replaces the posted prices and the exchange with live market outcomes. The `[[market]]` section may survive as starting prices or be dropped.
- **Test settings:** existing test settings have no markets and must keep loading unchanged.

## Out of Scope
- Changing prices during play, and any rule for what happens to prices when a hex's currency changes.
- A market stockpile, market inventories, or any limit on how much the market buys or sells.
- Any actor trading (Step 06), and market activity in the turn report.
- Starting stockpiles of goods or currency.
- Prices in a currency other than the hex's own, or exchange between currencies.
- Rounding or a smallest unit of currency.
- Showing markets or prices in the dev UI (Step 08).

## Acceptance Criteria
- The sample setting loads without errors; the center hex reports prices for all five non-currency goods, a typical hex reports prices for labor, grain and copper and "not traded" for the unimproved goods, and the edge hex without a market reports "not traded" for every good. Every hex reports 1 for silver.
- A test setting with markets across several files loads, each hex reports its own prices, and an entry with an empty `prices` table is accepted.
- Each error case produces one error at the expected file, line and column with the expected text: missing and bad coordinates, coordinates with no hex, a duplicate market for a hex, an unknown good in `prices`, a price that is a string, zero, negative or `inf`, `prices` that is not a table, the hex's currency listed, an unknown key, and a `[[market]]` in `rules/`.
- The unknown-good and currency-listed checks are skipped when another file in the setting is broken.
- Removing from a stockpile takes the soonest-perishing group first, removes groups and goods that reach zero, refuses an amount larger than the total without changing anything, and accepts zero.
- Selling a good adds the right amount of currency and removes the good; buying removes the currency and adds the good with the perish time of a unit appearing this turn; both refuse, with the right reason and no change, when goods or currency are short, when the good is unpriced on that hex, and when the good is the currency. Selling everything of a good empties it exactly.
- Stepping a World does not change any hex's prices, and existing tests, including every test setting without markets, pass unchanged.
