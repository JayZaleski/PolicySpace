# Step 02: dummy-market — Manual Changes

Nothing new is visible when running the world: no one trades yet, the turn reports are unchanged, and the developer UI doesn't show prices (that comes in Step 08). The change is to the setting format, plus one extra count in the checker's success line.

## What setting authors can now do

- Add **markets** with `[[market]]` sections in `start/` (in `rules/` it is an error, because prices are state the live market will change later). Each names a hex by `q` and `r` and gives a `prices` table of `good name = price`.
- A price is a number (whole or decimal) greater than 0: how many units of **the hex's currency** one unit of the good costs. This connects to the currency introduced in Step 01: prices are always in the hex's own currency, the currency itself always costs 1, and listing it in `prices` is an error.
- **A good is traded on a hex only if that hex's market lists a price for it.** A good without a price there isn't bought or sold on that hex. A hex with no market at all is valid and trades nothing. `prices = {}` is a market that trades nothing.
- At most one market per hex. A good may be priced on a hex whose currency is a different good.
- What the market does once actors trade (from Step 06): it buys and sells any amount at its posted prices, has no stock of its own, and never runs out. Selling to it removes the good and pays currency; buying removes currency and adds the good. A bought good starts its perish countdown in the turn it is bought, and when goods are taken out, the units that perish soonest go first. It is a stand-in: a later Stage replaces it with a real market whose prices move. Worth one or two sentences so authors know why prices matter, while saying clearly that nothing trades yet.
- The format accepts both `prices = { grain = 2 }` and a `[market.prices]` sub-table.
- New errors, with file, line and column: missing or bad `q`/`r`, no hex at the coordinates, a second market on the same hex, `prices` missing or not a table, a price that isn't a number or isn't greater than 0 (including `inf`), an unknown good in `prices`, the hex's currency listed (`'coin' is the currency at (6, 0), so it can't have a price`), and an unknown key. As with other references, the unknown-good and currency checks are skipped when part of the setting could not be read.

## Where it fits

- **New subsection "Markets"** under "Writing a setting", after "Natural resources" (and before "Running a setting"). Add it to the Contents list. It should cover the keys, the "only priced goods are traded" rule, the currency rule, a short TOML example, and the sample's markets.
- **"The folder layout"**: the `start/` line in the layout block should mention markets (`the map, the deposits, the markets`), and the bullet listing which section types go in which folder should add `[[market]]` to `start/`.
- **"Running a setting"**: in the three-part turn, part 2 ("Actors act") can note that trading with markets will happen here once POPs arrive; nothing acts yet.
- **"Checking a setting"**: the sample success line gains a count. **This supersedes Step 01's manual draft**, which gave the line as `6 goods, 19 hexes, 2 resources, 10 deposits`; it is now `Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets`. The singular-wording note should add `1 market`. Optionally add a market error (for example the priced-currency error) to the sample error listing, keeping the error count line consistent.
- **"The developer UI"**: no change; markets aren't shown yet. "Not available yet" may mention that prices appear in a later Step.

## The sample setting

`settings/sample/start/markets.toml` gives 18 of the 19 hexes a market, with prices in `silver`:

| Hexes | `unskilled_labor` | `grain` | `copper` | `unimproved_grain` | `unimproved_copper` |
|---|---|---|---|---|---|
| center `(0, 0)` | 1 | 2 | 5 | 0.5 | 1 |
| every other hex except `(2, -2)` | 1 | 2 | 5 | not traded | not traded |
| `(2, -2)` (north-east corner) | no market | | | | |

So the sample has a hex that trades every good, hexes that trade some, and one that trades nothing. Mention that the prices are placeholders and needn't match production, since nothing is produced for sale yet.

## Links to docs/

- [docs/market.md](docs/market.md) (new), for prices, the exchange, and the loading rules. Link it from the end of the new "Markets" subsection.
- [docs/currency.md](docs/currency.md) (from Step 01), where the subsection explains that prices are in the hex's currency.

## Examples or diagrams

- A short TOML example with two markets: one priced inline, and one showing a good left unpriced (not traded there). The `[market.prices]` sub-table form can be shown as a second variant.
- Optionally, the sample map diagram (already drawn in "The map"/"Natural resources") could mark the hex without a market and the center hex that trades resource output, or a small table like the one above is enough. A table is probably clearer than a new diagram; no interactive demo is needed until trading happens.
