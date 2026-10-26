# Step 02: order-protocol — Docs Changes

All references to `orders.md` below are to the new file Step 01's `docs_changes.md` proposes (`step-01-order-lines/docs_changes.md`), which isn't in `docs/` yet; apply that first.

## docs/decider.md — modify
- **Location:** the opening "Source:" paragraph, the sentence listing what is public under `policyspace_engine::decider`
- **Change:** replace
- **Supersedes:** none

The rest is public under `policyspace_engine::decider` (`FactoryOrder`, `OrderLine`, `Place`, `DecisionError`, `ReplyError`, `PerishGroup`, `FactoryView`, `TechnologyView`, `DepositsView`), and the submodules are public as `decider::view`, `decider::decision` and `decider::batch`.

## docs/decider.md — modify
- **Location:** the paragraph starting "**The engine runs the exchange once per turn**", the sentence "The World then resolves each valid decision: ..."
- **Change:** replace
- **Supersedes:** none

The World then resolves each valid decision: each order line is evaluated at its good's posted price, the resulting offers and bids go through the hex's trade round, and the fractions go to the factory mechanics (see [trade.md](trade.md)).

## docs/decider.md — modify
- **Location:** "Concepts", the bullet "**Tables, not lists, wherever a key is unique.**"
- **Change:** replace
- **Supersedes:** none

- **Tables, not lists, wherever a key is unique.** Prices, stockpile goods, technologies, deposits, bundles, a view's factories, and a decision's factories, offers and bids are all `BTreeMap`s: by good, technology or resource-type name, or by factory number. A table can't hold one key twice, so a decision can't list a factory twice or send two lines for one good on one side; a Python decider that builds dicts gets this for free. Keys come out in sorted order (alphabetical, or numeric for factories), which keeps the JSON deterministic.

## docs/decider.md — modify
- **Location:** "The decision", the code block and the bullet list after it, up to (not including) "### Validation"
- **Change:** replace
- **Supersedes:** none

```rust
pub struct Decision {
    pub business: String,
    pub factories: BTreeMap<u32, FactoryOrder>,   // absent in JSON = empty
    pub offers: BTreeMap<String, OrderLine>,      // a line to sell, per good; absent = empty
    pub bids: BTreeMap<String, OrderLine>,        // a line to buy, per good; absent = empty
    pub trace: Option<String>,                    // absent in JSON when None
}
pub struct FactoryOrder { pub run: f64, pub maintenance: f64 }  // each defaults to 0
pub struct OrderLine {
    pub quantity: f64,  // what it trades at the posted price: finite, >= 0, <= maximum
    pub slope: f64,     // quantity change per unit of price: finite; >= 0 in an offer, <= 0 in a bid
    pub maximum: f64,   // the most it trades at any price: finite, >= 0
    pub give_up: f64,   // the last price it still trades at: finite, >= 0
}
impl OrderLine {
    pub fn flat(amount: f64, give_up: f64) -> OrderLine;     // slope 0, maximum = amount
    pub fn params(&self, anchor_price: f64) -> LineParams;   // Step 01's raw form, anchored
}
```

- `run` and `maintenance` are the run fraction and the maintenance fraction of [production.md](production.md#two-instructions-per-factory): finite, 0 to 1 inclusive.
- **An offer or bid is an order line**, the one form every market order takes (see [orders.md](orders.md)): an offer is a supply line, a bid a demand line. A line says how much the business trades at the current price and how that changes with the price, up to a maximum, and the price beyond which it trades nothing (an offer sells nothing below its give-up price, a bid buys nothing above it; the give-up price itself is inclusive, and zero is allowed).
- **The engine anchors every line.** The decider sends no anchor price: the engine anchors each line at the good's posted price in the view the decision answers, the price the decider saw. Posted prices are always positive, so the anchor is always valid, and no field can disagree with the view.
- **All four numbers are required.** There are no defaults: a line missing a number is a format error naming the missing key, so a decider that forgets a field is told so instead of silently sending a different order.
- `OrderLine::flat(amount, give_up)` is "sell this much at or above `give_up`" in an offer and "buy this much at or below it" in a bid. `params(anchor)` turns a line into the `LineParams` of [orders.md](orders.md), which validation and the World pass to `Line::new`.
- The line's numbers are plain `f64`, not `Quantity`, so a bad number is reported at its place by validation (with every other problem) rather than failing the parse at the first one.
- The **trace** is free text, any number of lines: the decider's account of its choice. The engine stores it in the business's answer, for the dev UI, and never acts on it.

## docs/decider.md — modify
- **Location:** "Validation", from the table through the paragraph "The checks are independent: ..."
- **Change:** replace
- **Supersedes:** none

| Place | Problem | Message |
|---|---|---|
| `decision` | `business` isn't the view's | `answers for 'b' against the view of 'a'` |
| `factory #N` | not in the view (not this business's, or no such factory) | `not in this business's view` |
| `factory #N` | a fraction negative, above 1 or not finite | `run must be a finite number from 0 to 1 (got 1.5)`, likewise `maintenance` |
| `offer 'g'`, `bid 'g'` | quantity negative or not finite | `quantity must be a finite number of at least 0 (got -2)` |
| `offer 'g'`, `bid 'g'` | slope not finite | `slope must be a finite number (got NaN)` |
| `offer 'g'` | slope below 0 | `slope of a supply line must be at least 0 (got -1.5)` |
| `bid 'g'` | slope above 0 | `slope of a demand line must be at most 0 (got 1.5)` |
| `offer 'g'`, `bid 'g'` | maximum negative or not finite | `maximum must be a finite number of at least 0 (got inf)` |
| `offer 'g'`, `bid 'g'` | give-up price negative or not finite | `give-up price must be a finite number of at least 0 (got -1)` |
| `offer 'g'`, `bid 'g'` | quantity above the maximum (both valid numbers) | `quantity at the anchor price (5) is above the maximum (4)` |
| `offer 'g'`, `bid 'g'` | no such good | `unknown good` |
| `offer 'g'`, `bid 'g'` | the hex's currency | `it is this hex's currency, which can't be traded against itself` |
| `offer 'g'`, `bid 'g'` | no posted price on this hex | `no price is posted for it on this hex` |

**The line checks are Step 01's, not a copy.** For each offer and bid, the private `line_problems` builds the line's `LineParams` with `OrderLine::params` and calls `Line::new(side, ..)` (`Side::Supply` for offers, `Side::Demand` for bids); each `LineError` it returns becomes a `DecisionError` at the offer's or bid's place, with the `LineError`'s `Display` as its message (see [orders.md](orders.md#lineparams-and-linenew)). So the rules and wording for a malformed line live in one place, `orders.rs`, and the protocol never re-implements them. The anchor is the good's posted price in the view; when the view has none (an unknown good, the currency, an unpriced good) the private constant `STAND_IN_ANCHOR` (1.0) stands in. `Line::new`'s checks on the four fields never read the anchor, and the anchor is always valid, so the stand-in reports exactly the line's own problems; this is what lets one entry report a malformed line and a bad good together. `LineError::AnchorPrice` can therefore never appear in a decision's errors.

For each entry the line problems come first, in Step 01's field order, then the good check. The checks are independent: an unknown factory with a bad fraction reports both, and an offer with a wrong-way slope and an unknown good reports both. Of the three good checks only the first that applies is reported. Numbers in messages use `f64`'s `Display` (`1.5`, `-0.1`, `NaN`, `inf`). Holdings and cash are never checked. A line that trades nothing (a maximum of 0, or a quantity of 0 with a slope of 0) is valid; it resolves to nothing.

## docs/decider.md — modify
- **Location:** "Format", the hand-written decision and the line introducing it
- **Change:** replace
- **Supersedes:** none

A decision for it, written by hand: it runs the farm, offers its 8 unimproved grain at or above the posted 0.5, and bids for 6 labor at or below the posted 1:

```json
{"business": "center_farm", "factories": {"0": {"run": 1}}, "offers": {"unimproved_grain": {"quantity": 8, "slope": 0, "maximum": 8, "give_up": 0.5}}, "bids": {"unskilled_labor": {"quantity": 6, "slope": 0, "maximum": 6, "give_up": 1}}, "trace": "sell the grain"}
```

## docs/decider.md — modify
- **Location:** "Format", the bullet "A decision's `factories`, `offers` and `bids` may be left out ..."
- **Change:** replace
- **Supersedes:** none

- A decision's `factories`, `offers` and `bids` may be left out (empty), as may each fraction (0) and the trace. `Decision::empty("a").to_json()` is `{"business":"a","factories":{},"offers":{},"bids":{}}`; the trace key is written only when there is one. A reply's `decisions` may be left out too. **A line's four keys may not**: each line is a table with all four, written in the order `quantity`, `slope`, `maximum`, `give_up` (`"offers":{"grain":{"quantity":8.0,"slope":0.0,"maximum":8.0,"give_up":2.0}}`), and one missing is refused as ``missing field `give_up` ``. A line written as a bare number, the format before order lines, is refused as a wrong type.

## docs/decider.md — modify
- **Location:** "Reading in two layers", item 2 (**Values.**)
- **Change:** replace
- **Supersedes:** none

2. **Values.** The parsed value is checked, and every problem is collected into `FormatError::Values(lines)`: for a decision, the number checks of validation (`Decision::number_problems`: the factories' fractions, then every offer's and every bid's line checks, at their places, with the stand-in anchor since there is no view here); for a view, each factory condition that isn't a fraction (`factory #0: condition must be a finite number from 0 to 1 (got 1.5)`). JSON can't carry NaN or infinity, so for a line this layer catches negative numbers, wrong-way slopes and a quantity above its maximum. In a request or reply each line is prefixed with its business (`center_farm: offer 'grain': quantity must be a finite number of at least 0 (got -1)`); a recording lists the request's problems, then the reply's.

## docs/decider.md — modify
- **Location:** "Where serde_json lives", the last sentence ("`is_fraction` and `is_amount` in `mod.rs` are the shared number rules.")
- **Change:** replace
- **Supersedes:** none

`is_fraction` in `mod.rs` is the shared fraction rule; a line's numbers are checked by `Line::new` in `orders.rs`.

## docs/decider.md — modify
- **Location:** "Recordings", the table row for `sample_turn0.recording.json`
- **Change:** replace
- **Supersedes:** none

| `sample_turn0.recording.json` | the engine (`request` half) and the Python test (`reply` half) | pretty, trailing newline | the sample's turn-0 request for its three center businesses, and the arithmetic rule's reply (its offers and bids as flat lines at the posted prices) |

## docs/decider.md — modify
- **Location:** "Regenerating", the paragraph after the command block
- **Change:** replace
- **Supersedes:** none

Then unset the variable. The `cargo test` runs single-threaded so that the tests that write files don't run at the same time as each other. After the first step the reply check may fail (the reply is stale or empty): that is the signal to run the Python step. When the decision format itself changes, the old reply no longer reads at all, and the first step writes an empty reply in its place, which the Python step then fills. Hand-written requests carry no decisions, are edited by hand and are never regenerated.

## docs/decider.md — modify
- **Location:** "Extending", new bullet after "**A new verb in the decision**"
- **Change:** add
- **Supersedes:** none

- **A change to the order line** (a new field, say): add it to `OrderLine` and to `OrderLine::params`, and to `LineParams` and `Line::new` in `orders.rs` if the line's rules change, so the checks and messages stay in one place (see [orders.md](orders.md)). Decide whether a missing value is an error (as for the four fields today) or has a default that doesn't depend on other fields. Update `resolve_lines` in `world.rs` if the field changes what a line trades, and the Python `OrderLine` with its reader and writer, in the same key position.

## docs/decider.md — modify
- **Location:** "Not yet", the sentence starting "Verbs beyond the two fractions, offers and bids ..."
- **Change:** replace
- **Supersedes:** none

Verbs beyond the two fractions and the order lines (budgets, construction, retooling, trading assets) are later Stages. A line's give-up price and slope don't yet change what it trades, beyond the give-up price excluding the posted price: until the live market settles at the crossing, each line trades its quantity at the posted price (see [trade.md](trade.md#businesssubmission)). Market data in the view (the summed curves) and lines kept on file between turns are later Steps of Stage 04.

## docs/decider.md — modify
- **Location:** "Tests", the `decider/decision.rs` bullet
- **Change:** replace
- **Supersedes:** none

- `decider/decision.rs` unit tests (local builders `flat`, `line` and `lines`): empty and full decisions are valid; asking for too much is valid; sloped lines with the right signs and lines that trade nothing are valid; one decision with every kind of problem (bad fractions, the currency, an unpriced good, an unknown good, a negative quantity, wrong-way slopes on both sides, a negative give-up price, a quantity above its maximum) reports all twelve, at their places, in order; non-finite slope and maximum reported by field; a line problem and a good problem on one entry both reported; a decision for another business; round trips, including sloped lines with `1/3`, `5e-324` and `f64::MAX`; the empty decision's exact JSON and a line's exact key order; a hand-written decision; malformed JSON, wrong types, unknown keys (in a decision and in a line), duplicate goods, a line missing `give_up`, a line written as a bare number, bad fractions, and bad line values (`Values` at the place) each refused at the right layer; `OrderLine::flat` and `params`; the error displays.

## docs/decider.md — modify
- **Location:** "Tests", the `crates/engine/tests/decider.rs` bullet: the helper list and the sub-bullet "decisions validate against the farm's view ..."
- **Change:** replace
- **Supersedes:** none

- `crates/engine/tests/decider.rs`, through the public API only, using `common`'s `world_of`, `sample_setting`, `test_setting` and `q`, plus local helpers (`business`, `view`, `groups_of`, `qmap`, `lines` (flat lines from good, amount and give-up price), `order`, `factory_rows`, `deposit_rows`, `full_decision`):

and the sub-bullet:

  - decisions validate against the farm's view, including over-asking, with the out-of-view factory, currency and unpriced-good refusals, and the line refusals through the public API: a wrong-way offer slope, a bid quantity above its maximum, and a bid for the currency with a wrong-way slope reporting the slope message and then the currency message;

## docs/python.md — modify
- **Location:** "The messages", first paragraph
- **Change:** replace
- **Supersedes:** none

Every message of [decider.md](decider.md) is a `@dataclass` with the Rust type's name and field names: `BusinessView` with `PerishGroup`, `FactoryView`, `TechnologyView` and `DepositsView`; `Decision` with `FactoryOrder` and `OrderLine`; `Request`, `Reply` and `Recording`. Tables are `dict`s keyed by name, and factory tables are keyed by `int`. A decision's `offers` and `bids` are `dict[str, OrderLine]`, one line per good; an `OrderLine` has the four required fields `quantity`, `slope`, `maximum` and `give_up` (the engine anchors it at the good's posted price, see [decider.md](decider.md#the-decision)), and `OrderLine.flat(amount, give_up)` builds a line with slope `0.0` and maximum `amount`. Defaults follow the engine: a `Decision`'s `factories`, `offers` and `bids` default to empty and its `trace` to `None`, a `FactoryOrder`'s fractions to `0.0`, and `Decision.empty(business)` is the explicit "do nothing". A line's fields have no defaults.

## docs/python.md — modify
- **Location:** "Reading and writing", the paragraph "**Reading is as strict as the engine's.**", from "The keys that may be left out ..." to the end of the paragraph
- **Change:** replace
- **Supersedes:** none

The keys that may be left out are the ones the engine allows: a decision's `factories`, `offers`, `bids` and `trace`, a factory order's `run` and `maintenance`, and a reply's `decisions`; a line's four keys are all required. Messages about a line name its side and good: `missing key 'give_up' in offer 'grain'`, `unknown key 'anchor' in bid 'labor'`, `offer 'grain' quantity must be a number`, `offer 'grain' must be a table` (a bare number, the old form), and `offers must be a table`. Value ranges are not checked (a run of 2, a negative quantity or a wrong-way slope reads fine): that is the engine's validation (see [decider.md](decider.md#validation)).

## docs/python.md — modify
- **Location:** "Reading and writing", the paragraph "**Writing mirrors the engine.**"
- **Change:** replace
- **Supersedes:** none

**Writing mirrors the engine.** `*_to_data` builds each dict in the Rust field order, tables with their keys sorted (string keys by Python's default order, which equals the engine's byte order; factory keys numerically, written as decimal strings), `perishes` always written (`null` for "never"), each line's keys in the order `quantity`, `slope`, `maximum`, `give_up`, and a decision's `trace` key only when there is one. Dict iteration order is never relied on for output. The private helpers `_line_from_data`/`_line_to_data` read and write one line, and `_lines`/`_lines_data` a table of them, so the line's layout is defined in one place on this side too.

## docs/python.md — modify
- **Location:** "The arithmetic rule: `arithmetic.py`", first paragraph, the last sentence
- **Change:** replace
- **Supersedes:** none

It is the simplest rule that behaves sensibly under the dummy market (see [market.md](market.md)), where every offer sells at the posted price, and it plans in fixed amounts: only its last step turns them into order lines (see [Offers and bids](#offers-and-bids)).

## docs/python.md — modify
- **Location:** "Offers and bids", new paragraph after the first paragraph (before "The rule's bids should match exactly ...")
- **Change:** add
- **Supersedes:** none

**What it sends is flat lines.** The amounts are planned as plain numbers per good, and `flat_lines(view, table)` turns each into `OrderLine.flat(amount, float(view.price(good)))` as the last step of `decide_business`: the quantity and the maximum are the amount, the slope is 0, and the give-up price is the posted price. An offer then says "sell this much at or above the current price" and a bid "buy this much at or below it", the honest translation of a fixed quantity at the posted price; a give-up of 0 for sales or a large number for bids would express a willingness the rule doesn't have. Under the dummy round each such line trades its full amount, so the rule's plan, its trace and the engine's figures are those of a fixed-amount rule. `float(...)` keeps every number in a line a `float`. A rule that sends real sloped lines, using market data in the view, is a later Step of Stage 04.

## docs/python.md — modify
- **Location:** "Tests", the `test_protocol.py` and `test_arithmetic.py` bullets
- **Change:** replace
- **Supersedes:** none

- `test_protocol.py`: `format_float` against a table of engine layouts, and refusing non-finite values; `dumps` compact and pretty, and refusing booleans; strict reading (bad syntax, unknown, missing and duplicate keys, `NaN`, bad factory keys, booleans, string and float turns), and a line's own refusals with their messages (a missing `give_up`, an unknown key, a string or boolean number, a line that is a number, `offers` that isn't a table, a good repeated); defaults and the empty decision's exact text, and a read line keeping its parsed `int` and `float` types; sorted tables, with each line's keys in the engine's order; `OrderLine.flat`; **the exactness request comes back byte for byte**; the sample recording round trips in pretty form; every request file round trips.
- `test_arithmetic.py` (comparing amounts through a helper `quantities(table)`, the quantity of each line): the sample's reply equals the recording's, and its numbers (the three factories run at capacity, the farm bids grain and labor and maintains one slice, the mine catches up from 0.75, the toolmaker runs at a margin of 0.25), with every line flat at the posted price; one test per hand-written request (negative margin, unpriced input, cash short, revival, no market, seller), plus an unpriced bundle good and the currency in a bundle, built in code with `dataclasses.replace`; and an invariant over every request: no offer or bid names the currency or an unpriced good, every line has a quantity above 0, slope `0.0`, maximum equal to its quantity and give-up equal to the posted price, all four `float`s, every fraction is from 0 to 1, every factory in the view has an order, and every trace is non-empty.

## docs/trade.md — modify
- **Location:** opening paragraph, the sentence "Each business on the hex hands it a **business submission**: the offers and bids its decision resolves to, and its whole stockpile."
- **Change:** replace
- **Supersedes:** none

Each business on the hex hands it a **business submission**: the offers and bids its decision's order lines resolve to at the posted prices, and its whole stockpile.

## docs/trade.md — modify
- **Location:** "`BusinessSubmission`", the paragraph after the code block ("The World resolves it from the business's stored answer ...")
- **Change:** replace
- **Supersedes:** none

The World resolves it from the business's stored answer (see [decider-host.md](decider-host.md#answers-and-the-report)) in the private `World::business_submission`, through the private `resolve_lines(goods, prices, hex, side, lines)`, called once for the offers (`Side::Supply`) and once for the bids (`Side::Demand`). For `BusinessAnswer::Valid(decision)`, each order line in the decision's `offers` and `bids` (tables keyed by good name, see [decider.md](decider.md#the-decision)) becomes a `(GoodId, Quantity)`: the name is looked up in `rules.goods`, the good's posted price on the business's hex is read from `HexPrices`, the line is built with `Line::new(side, &line.params(price))`, anchored at that price, and evaluated there with `Line::at(price)` (see [orders.md](orders.md#the-lines-shape-lineat)). Validation guarantees the good is known, priced and the line well-formed, so each step is `expect`ed with the guarantee named. Zero amounts are dropped, and the list is sorted by `GoodId`, because the decision's `BTreeMap<String, _>` is in name order, not good ID order. For `Nothing` and `Refused { .. }` both lists are empty. An idle business therefore takes part in the round with nothing to trade: the result is the same as leaving it out, there is one code path, and every business gets an outcome with its cash figures.

**What a line trades under the dummy market.** The line is anchored at the very price it is evaluated at, and under the dummy market the price at trade time is the price the decider saw in its view. So `at` returns the line's own quantity, whatever its slope and maximum, unless the give-up price excludes the posted price, when it returns zero: a bid whose give-up price is below the posted price buys nothing, and an offer whose give-up price is above it sells nothing. The give-up price is inclusive, so a give-up equal to the posted price trades the quantity. Everything after resolution is unchanged: offers are trimmed to holdings and bids capped to cash, and `BusinessTrade` records the resolved amounts as offered and bid beside what was done. Settling every line at the crossing of the summed curves replaces this in a later Step of Stage 04.

## docs/trade.md — modify
- **Location:** "`BusinessTrade`", the table row for `offered()`, `bid()`
- **Change:** replace
- **Supersedes:** none

| `offered()`, `bid() -> &[(GoodId, Quantity)]` | The offers and bids as submitted: the amounts the decision's lines resolved to at the posted prices (good ID order, amounts > 0) |

## docs/trade.md — modify
- **Location:** "Tests", the `crates/engine/tests/business_trade.rs` bullet, the sentence after "two worlds stepped identically."
- **Change:** add (insert before "The last test runs the sample ...")
- **Supersedes:** none

The closure deciders' `decision` helper sends flat lines that trade at any price (offers with give-up 0, bids with give-up `f64::MAX`), so its amounts are what the round trades. `lines_resolve_at_the_posted_price_in_the_dummy_round` (helpers `fixed` and `with_lines`) checks the resolution itself: a bid whose give-up is below the posted price and an offer whose give-up is above it trade nothing and aren't listed as bid or offered; give-up prices equal to the posted price trade the quantity; a sloped bid and a line whose maximum is above its quantity resolve to the quantity; and an offer above holdings and bids beyond cash (proportion 0.2) are still trimmed and recorded as asked.

## docs/world.md — modify
- **Location:** "The turn", item 2 (**Actors act**), the clause "each business the offers and bids its valid decision resolves to (none for a business that decided nothing or was refused)"
- **Change:** replace
- **Supersedes:** none

each business the offers and bids its valid decision's order lines resolve to at the posted prices (none for a business that decided nothing or was refused)

## docs/world.md — modify
- **Location:** "Trade round and operation", the sentence "it resolves each business's answer into a `BusinessSubmission` with `business_submission` (moving its stockpile out)"
- **Change:** replace
- **Supersedes:** none

it resolves each business's answer into a `BusinessSubmission` with `business_submission`, which evaluates each of the decision's lines at its good's posted price on the business's hex (moving its stockpile out)

## docs/dev-ui.md — modify
- **Location:** "Businesses, factories and technologies", the `trade` sub-bullet of `business_section`
- **Change:** replace
- **Supersedes:** none

  - `trade`: one `TradeRow { good, offered, sold, bid, bought }` per good, merging the decision with what happened, so a cut shows as asked above done in one row. For a valid answer the goods are those the `BusinessTrade` offered or bid, in good ID order, and the cells are its `offered_of`, `sold_of`, `bid_of` and `bought_of`: offered and bid are the amounts the decision's lines resolved to at the posted prices (see [trade.md](trade.md#businesssubmission)). A refused decision has no trade, so its rows come from the stored `Decision`'s `offers` and `bids` maps, each cell the line's `quantity` as sent (the private `sent` helper), with sold and bought `0`: names the goods list knows come first in good ID order, then unknown names (a misspelt good) in name order, so what the decider asked for is always visible. A line's slope, maximum and give-up price aren't shown. A business that decided nothing has no rows.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text/businesses.rs` sub-bullet "a refused decision (`business_trade` with an order for a factory ...)"
- **Change:** replace
- **Supersedes:** none

  - a refused decision (`business_trade` with an order for a factory the business doesn't own): the error lines, the trade rows with each line's quantity asked and 0 done, and the factories idle; and a refused decision naming a misspelt good, whose row comes after the known goods, with its trace shown. The tests' `decision` helper builds flat lines that trade at any price, as `business_trade.rs` does;

## docs/market.md — modify
- **Location:** "Not yet", appended to the end of the section (after Step 01's added paragraph)
- **Change:** add
- **Supersedes:** none

Businesses already send order lines across the decider protocol (see [decider.md](decider.md#the-decision)), but the dummy market still trades them at the posted prices: each line is evaluated at the posted price, which gives its quantity or, when its give-up price excludes the posted price, nothing (see [trade.md](trade.md#businesssubmission)). POPs don't draw lines yet.

## docs/decider-host.md — modify
- **Location:** "Answers and the report", the bullet "A refused decision is kept with its errors, ..."
- **Change:** replace
- **Supersedes:** none

- A refused decision is kept with its errors, so a developer can see what was sent and why it was refused: the dev UI shows the errors, the quantity of each offer and bid line asked for, and the trace (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). It counts as deciding nothing. A malformed order line refuses the whole decision like any other problem.

## docs/README.md — modify
- **Location:** the documents table, the `decider.md` row
- **Change:** replace
- **Supersedes:** none

| [decider.md](decider.md) | The decider protocol as data: the `BusinessView` a business is served (built by `World::business_view`), the `Decision` it answers with (run and maintenance fractions per factory, an `OrderLine` per good to sell and to buy, anchored by the engine at the posted price, a trace), validation against the view and its errors (the line checks are `orders.rs`'s), the `Request`/`Reply`/`Recording` batch and reply validation, the lossless JSON format (name-keyed tables, quoted factory numbers, `Quantity` and `Turn` forms, two-layer reading, duplicate keys), one message per line as transport, and the committed recordings both test suites share and how to regenerate them |

## docs/README.md — modify
- **Location:** the documents table, the `trade.md` row, the phrase "business submissions resolved from decisions"
- **Change:** replace
- **Supersedes:** none

business submissions resolved from decisions' order lines at the posted prices

## docs/README.md — modify
- **Location:** the documents table, the `orders.md` row Step 01 adds
- **Change:** replace
- **Supersedes:** Step 01's `docs/README.md` row for `orders.md` (only the opening phrase changes)

| [orders.md](orders.md) | Order lines and the live market's arithmetic: `Line` and its checked construction from `LineParams` (every `LineError` at once; the decider protocol's line checks), the line's shape (clip to the maximum, inclusive give-up price, steps), `Curve` as an exact breakpoint list summed from lines with its side's convention at a jump, and `cross`: the clean, jump, range, no-trade and zero-price cases, the `Crossing` record with its shares and unfilled quantities, and how it is computed without a tolerance. Lines are carried in decisions and evaluated at the posted price; curves and the crossing aren't wired into the turn yet |

## docs/orders.md — modify
- **Location:** the paragraph starting "**Nothing calls it yet.**" (from Step 01's proposal)
- **Change:** replace
- **Supersedes:** Step 01's "**Nothing calls it yet.**" paragraph in `docs/orders.md`

**Who calls it.** The decider protocol carries a business's offers and bids as order lines and checks them with `Line::new`, reporting each `LineError` at the offer's or bid's place (see [decider.md](decider.md#validation)), and the World evaluates each line of a valid decision with `Line::at` at its good's posted price, which gives the line's quantity or zero (see [trade.md](trade.md#businesssubmission)). `Curve` and `cross` aren't called yet: the turn still trades at the dummy market's posted prices (see [market.md](market.md)). Later work draws lines for POPs, settles each good's trades on the crossing, and publishes the summed curves in the views.

## docs/orders.md — modify
- **Location:** "Not yet" (from Step 01's proposal)
- **Change:** replace
- **Supersedes:** Step 01's "Not yet" paragraph in `docs/orders.md`

POP lines, settlement on the crossing (business lines are carried in decisions but still evaluated at the posted price), a price floor, trimming to holdings and the cash cap on the crossing's results, curves in the views, orders kept on file between turns, and taxes or subsidies as a wedge between the curves.
