# Step 06: pop-trade — Manual Changes

POPs now trade and consume every turn. In the "actors act" part of the turn, each POP sells what it doesn't want to keep to its hex's market, buys what it wants with the proceeds, and then consumes its utility goods. A setting author sees this in `check --turns` and in the developer UI's turn log: every turn line gains three lists, `sold`, `bought` and `consumed`, after `endowed`. The setting format and the sample setting are unchanged. A POP's own trade can't be inspected in the window yet (Step 08).

## What users can now see

- **Three new lists on every turn line**, after `endowed`, each in goods order and showing only goods with an amount: `sold` (the total POPs sold on all hexes), `bought`, and `consumed`. An empty list prints `nothing`, so a setting without POPs ends every line with `; sold nothing; bought nothing; consumed nothing`.
- **A fresh world has already traded.** Turn 0's line already shows trade, because creating the world runs turn 0's trade round. Between steps, what the window and the checker show is the world after trading.
- **Labor no longer perishes in the sample.** POPs sell or consume all their labor in the round, so `perished` now lists only the deposit output from turn 1 on.

## How trade works, for setting authors

Explain in plain terms, without the formulas (those are in docs/demand.md and docs/trade.md):

- Each turn, after the endowment, each POP's **whole holdings** are its income: the endowment, plus anything durable it still holds, plus any currency left from earlier turns. The market works out what the POP wants to end up with from its type's subsistence amounts and shares and the hex's prices (Step 05's demand).
- **Sales first, then purchases.** On each hex every POP sells first, then every POP buys. With today's fixed-price market the order changes nothing; it's there so the later live market, which can run short, works fairly.
- **Consumption.** After trading, a POP consumes the goods in its utility, up to what it wanted. The labor a POP keeps is its leisure, consumed the same way. Goods outside its utility are never consumed: they are sold if the hex prices them, and kept if it doesn't.
- **Leftover currency stays.** A POP spends essentially all its income. Floating-point rounding can leave a tiny amount of currency, which stays in its stockpile and counts as income next turn. Nothing is rounded.
- **No market, no trade.** A POP on a hex with no market (or where its goods aren't priced) keeps what it has and consumes what it can. The sample's north-east peasant on (2, -2) consumes all its labor as leisure and goes without grain every turn.
- **Hints for authors** that follow from these rules: a durable good a POP is endowed with but doesn't want and can't sell piles up in its stockpile; if no good a type gives a share to is priced on its hex, its spare income is kept as currency and piles up too (for a durable currency); and a POP that can't afford its subsistence sells more labor than its leisure needs and still falls short on everything in the same proportion.

## Where it fits

- **"Running a setting"**:
  - Part 2 of the three-part turn ("Actors act") is no longer empty: it is now "**Trade and consume**", described as above. **This supersedes Step 04's manual draft**, which said part 2 was still empty and that "nothing uses what POPs hold yet", and Step 02's note that trading "will happen here once POPs arrive". Step 02's description of what the market does once actors trade (any amount at posted prices, bought goods start their perish countdown in the turn they're bought, soonest-perishing units leave first) now describes what actually happens.
  - Say that a new world has already run turn 0's trade, and that what the checker and window show between steps is the world after trading.
  - **Turn reports** gain `sold`, `bought` and `consumed`. Replace the code cell that prints the sample's turns 0 to 2 (**this supersedes the lines in Step 04's manual draft**) with:
    ```
    Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain
    Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain
    Turn 2: (the same as turn 1)
    ```
    (The cell should run the checker and print real output rather than hardcode it.) Explain where the numbers come from: each peasant with a market sells 7.2 of its 10 labor and buys 3.6 grain, the center artisans sell 8.5 labor each and buy grain and copper, and the 4100 − 2878.6 = 1221.4 labor not sold is consumed as leisure (including the no-market peasant's 120). Point out that `unskilled_labor` has left `perished`, because it's all sold or consumed during the turn, and that `copper` precedes `grain` because lists are in goods order.
- **"Checking a setting"**: if it shows `check --turns` output, use the new lines above. The success line is unchanged.
- **"The developer UI"**:
  - *Turn log*: it already says the log uses the same wording as `check --turns`, so it now shows the three new lists. No other wording change is needed.
  - *Goods*: the "amount held in the whole world" still counts only hex stockpiles. Step 04's note stands; after trading POPs hold little anyway.
  - *Not available yet*: what each POP sold, bought and consumed, its income, and whether it is below subsistence can't be inspected in the window yet (Step 08).
- **Natural follow-on**: Step 03's manual draft suggested a "how a type spends its income" demo once demand exists, and Step 05 deferred it until trade was visible. It fits now, in "Running a setting" or at the end of Step 03's "POP types" subsection (see below).

## Links to docs/

- [docs/trade.md](docs/trade.md) (new), from the trade description in "Running a setting": the trade round, sales before purchases, the purchase cap, consumption and leftover currency.
- [docs/demand.md](docs/demand.md) (Step 05's draft), for how a POP's wanted quantities are computed, including the below-subsistence rule.
- [docs/world.md](docs/world.md), for the turn and the turn report.

## Examples or diagrams

- **A worked example of one POP for one turn**, as a small table or a flow: the center peasant (size 60) is endowed 600 labor, sells 432 for 432 silver, buys 216 grain with it, and consumes 168 labor and 216 grain, ending with no labor, no grain and a sliver of silver at most. Next to it, the north-east peasant (size 12, no market) consumes its 120 labor and gets no grain.
- **An interactive or plotted demo of a POP's choices** (the one Steps 03 and 05 suggested): for one sample type, show what it sells, buys and consumes as the grain price varies (for example from 1 to 10 with labor at 1), marking where it falls below subsistence (for a peasant at 10 labor, above a grain price of 4). A static chart computed in the notebook from the same formulas, or a slider widget if the manual already uses them, would both work. The worked examples in docs/demand.md and docs/trade.md give checked values to match.
- No new diagram of the map is needed.
