# Step 02: order-protocol — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-04-market/step-02-order-protocol/design.md` — the behavior this spec implements. Every rule there is required.
- `PROJECT_PLAN/stage-04-market/step-01-order-lines/docs_changes.md` — the description of the `orders` module (Step 01), which `docs/` doesn't hold yet. Read "Lines" fully: `LineParams`, `Line::new` and its messages, `Line::at` and the inclusive give-up price.
- `crates/engine/src/orders.rs` lines 40–300 — `Side`, `LineParams`, `LineError` (with its exact `Display` texts), `LineErrors`, `Line::new`, `Line::at`, `Line::params`.
- `docs/decider.md` — the protocol: the decision, validation and its message table, the batch, the JSON format, the two-layer reading, duplicate keys, the recordings and how to regenerate them.
- `docs/python.md` — the Python package: the dataclasses, strict reading, the engine-style writer, the arithmetic rule and its tests.
- `docs/trade.md`, section on business submissions and the cash cap — what the World hands the round and what it records.
- `docs/dev-ui.md`, "Businesses, factories and technologies" — the trade table of a valid and a refused decision.
- `docs/workspace.md` — crate layout, lints (`missing_docs`, clippy `-D warnings`), the test helpers, the check command, and the rule that tests never set environment variables in-process.
- Code: `crates/engine/src/decider/decision.rs` (all), `crates/engine/src/decider/mod.rs` (the module doc, `is_amount`, `no_duplicate_keys`), `crates/engine/src/decider/batch.rs` (`value_problems`, tests), `crates/engine/src/world.rs` lines 650–670 and 733–748 (`business_submission`, `resolve_amounts`), `crates/engine/src/trade.rs` lines 1–30 and 129–136 (module doc, `BusinessSubmission`), `crates/dev-ui/src/text/businesses.rs` lines 100–200 and 400–445 (`sent`, `trade_rows`, the test `decision` helper), `python/policyspace/protocol.py` (all), `python/policyspace/arithmetic.py` (all), `python/tests/test_protocol.py`, `python/tests/test_arithmetic.py`, `crates/engine/tests/decider.rs`, `crates/engine/tests/business_trade.rs` lines 1–80, `crates/engine/tests/recordings.rs`, `crates/engine/tests/recordings/README.md`.

## Approach
- **One new protocol type, `OrderLine`**, in `decider/decision.rs`: four plain `f64` fields (`quantity`, `slope`, `maximum`, `give_up`), all required, no defaults. `Decision.offers` and `Decision.bids` become `BTreeMap<String, OrderLine>`. Plain `f64`s (not `Quantity`) so that a bad number is reported by validation at its place together with every other problem, as amounts are today, instead of failing the parse at the first one. A missing field fails the parse with serde's `missing field` message, as the design requires.
- **The engine anchors the line and Step 01 checks it.** Validation builds a `LineParams` from the `OrderLine` plus an anchor price and calls `Line::new(side, &params)`: offers are `Side::Supply`, bids are `Side::Demand`. Each `LineError`'s `Display` becomes a `DecisionError` at the offer's or bid's place. So the protocol has no second copy of the line rules or messages.
- **The anchor is the good's posted price in the view**, when the view has one. When it doesn't (unknown good, the currency, an unpriced good), and at the format layer (`from_json`, where there is no view), the shape checks still run with a **stand-in anchor** of `1.0`. `Line::new`'s checks on the four fields don't read the anchor, and the anchor itself is always valid, so the stand-in reports exactly the four-field problems and nothing else. This is what lets one decision report a malformed line and a bad good together.
- **The dummy round evaluates each line at the posted price.** `World::business_submission` builds each line of a valid decision with the hex's posted price as its anchor and takes `Line::at(posted price)`: the line's own quantity, or zero when the give-up price excludes the posted price. Zero results are dropped as zero amounts are today. Under the dummy market the price at trade time equals the price in the view, so this is the design's "anchored at the price the decider saw". Nothing after that changes: trims to holdings, the cash cap, `BusinessTrade`.
- **`OrderLine::flat(amount, give_up)`** on both sides: slope 0, maximum = amount. The arithmetic rule uses it with the posted price as the give-up; engine and dev-UI tests use it to translate their fixed amounts.
- **Python mirrors the engine**: an `OrderLine` dataclass, a strict reader and an ordered writer for it, the decision's tables becoming tables of lines; the rule converts its computed amounts into flat lines as the last step, so its planning code is untouched.
- **The sample recording's reply half is regenerated** with the existing `POLICYSPACE_UPDATE_RECORDINGS` procedure. The request files don't change.
- **Build as one.** The Rust and Python sides meet at the regenerated recording and at tests that start the real Python decider, so neither side can pass its tests alone; no `## Packages`.

## Changes

### Engine
- `crates/engine/src/decider/decision.rs` — add `OrderLine` (struct, `flat`, `params`); change `Decision.offers`/`bids` to `BTreeMap<String, OrderLine>`; replace `amount_problem` with `line_problems`; update `validate` and `number_problems`; update the module doc comment ("offers and bids per good" → "an order line per good to sell and to buy"); update the unit tests (Test Plan).
- `crates/engine/src/decider/mod.rs` — re-export `OrderLine` from `decision`; remove `is_amount` (no longer used; clippy would flag it); update the module doc's description of a decision and of validation (line checks beside the good checks).
- `crates/engine/src/decider/batch.rs` — tests only: build lines where offers and bids are inserted; the expected `Values` messages for a bad offer/bid use the line messages.
- `crates/engine/src/world.rs` — replace `resolve_amounts` with `resolve_lines` (see Interfaces); `business_submission` calls it with the hex's prices and the side; update the module doc sentence about resolving offers and bids.
- `crates/engine/src/trade.rs` — doc comments only (module doc lines 14–16 and `BusinessSubmission`): the offers and bids are the amounts the World resolved from the decision's lines at the posted prices.
- `crates/engine/src/lib.rs` — crate doc: the decision carries "an order line per good to sell and per good to buy" instead of "offers, bids". No new re-export at the crate root (`FactoryOrder` isn't there either; `OrderLine` is reached as `policyspace_engine::decider::OrderLine`).
- `crates/engine/tests/decider.rs` — update `table`, `full_decision`, the hand-written decision text and the validation test; add line refusals through the public API (Test Plan).
- `crates/engine/tests/business_trade.rs` — the `decision` helper builds flat lines; add the dummy-round line tests (Test Plan).
- `crates/engine/tests/recordings/sample_turn0.recording.json` — regenerated (reply half). Not edited by hand.

### Dev UI
- `crates/dev-ui/src/text/businesses.rs` — `sent` reads the line's `quantity`; the doc comments of `BusinessSection.trade` / `TradeRow` say a refused decision's rows show each line's quantity; the test `decision` helper builds flat lines; tests updated (Test Plan).

### Python
- `python/policyspace/protocol.py` — add `OrderLine` (dataclass with `flat`); `Decision.offers`/`bids` become `dict[str, OrderLine]`; add `_line_from_data`, `_line_to_data`, `_lines`, `_lines_data`; use them in `decision_from_data` / `decision_to_data`; update the module docstring.
- `python/policyspace/arithmetic.py` — `decide_business` returns lines: a new `flat_lines(view, table)` turns the rule's amount tables into `OrderLine.flat(amount, float(view.price(good)))`; module docstring updated. `trade`, `needs`, `_value`, the budget and the trace are untouched.
- `python/tests/test_protocol.py`, `python/tests/test_arithmetic.py` — updated and extended (Test Plan).

### Not changed
`crates/engine/src/orders.rs`, `market.rs`, the trade round's logic, `production.rs`, the setting loader, the sample setting, every `*.request.json`, `crates/engine/tests/recordings/README.md`, `decider/view.rs`, `decider/host.rs`, `decider/replay.rs`, `decider/external.rs`, `decider/python.rs`, `crates/engine/tests/recordings.rs`, `external_decider.rs`, `decider_host.rs`, `docs/`, `manual.ipynb`.

## Interfaces & Data
All public items need doc comments (the lint is `-D warnings`).

### Engine: `decider/decision.rs`
```rust
use crate::orders::{Line, LineParams, Side};

/// One order line as a decision carries it: what the business trades at the posted price
/// of the good, how that changes with the price, the most it would trade, and the last
/// price at which it still trades. The engine anchors the line at the good's posted price
/// in the view the decision answers; the decider sends no anchor. All four numbers are
/// required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderLine {
    /// The quantity at the posted price: finite, at least 0, at most `maximum`.
    pub quantity: f64,
    /// Quantity change per unit of price: finite; at most 0 in a bid, at least 0 in an offer.
    pub slope: f64,
    /// The most the business trades at any price: finite, at least 0.
    pub maximum: f64,
    /// The last price it still trades at (an offer sells nothing below it, a bid buys
    /// nothing above it): finite, at least 0.
    pub give_up: f64,
}

impl OrderLine {
    /// A flat line: `amount` at every price the give-up allows (slope 0, maximum `amount`).
    /// "Sell this much at or above `give_up`" in an offer, "buy this much at or below it"
    /// in a bid.
    pub fn flat(amount: f64, give_up: f64) -> OrderLine;
    /// The line's numbers as Step 01's parameters, anchored at `anchor_price`.
    pub fn params(&self, anchor_price: f64) -> LineParams;
}

pub struct Decision {
    pub business: String,
    pub factories: BTreeMap<u32, FactoryOrder>,            // unchanged
    /// Lines to sell, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub offers: BTreeMap<String, OrderLine>,
    /// Lines to buy, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub bids: BTreeMap<String, OrderLine>,
    pub trace: Option<String>,                               // unchanged
}
```

**JSON.** A line is a table with keys in struct order: `{"quantity":8.0,"slope":0.0,"maximum":8.0,"give_up":0.5}`. The hand-written decision of `docs/decider.md` becomes:
```json
{"business": "center_farm", "factories": {"0": {"run": 1}}, "offers": {"unimproved_grain": {"quantity": 8, "slope": 0, "maximum": 8, "give_up": 0.5}}, "bids": {"unskilled_labor": {"quantity": 6, "slope": 0, "maximum": 6, "give_up": 1}}, "trace": "sell the grain"}
```
`Decision::empty("a").to_json()` is still `{"business":"a","factories":{},"offers":{},"bids":{}}`. A line missing a key fails the parse (`FormatError::Json`, serde's message contains ``missing field `give_up` ``); an unknown key in a line fails likewise (`deny_unknown_fields`); a repeated good name fails with `the key 'grain' is written twice` as today.

**Validation.** Replace `amount_problem` with:
```rust
/// The anchor used to check a line's shape when the good has no posted price (or there is
/// no view): any positive number does, because the shape checks never read it.
const STAND_IN_ANCHOR: f64 = 1.0;

/// The problems with one line, in Step 01's field order, each at `place`: Line::new's
/// errors with `anchor` as the anchor price, displayed with `LineError`'s `Display`.
fn line_problems(place: &Place, side: Side, line: &OrderLine, anchor: f64, problems: &mut Vec<DecisionError>);
```
In `validate`, the sections become `(&self.offers, Side::Supply, Place::Offer)` and `(&self.bids, Side::Demand, Place::Bid)`. For each entry, in this order: `line_problems` with `anchor = view.prices.get(good).map(Quantity::get).unwrap_or(STAND_IN_ANCHOR)`, then the existing good check (unknown good / currency / no posted price, first that applies). Report order for one decision is unchanged: the decision, factories by number, offers by name, bids by name.

Resulting messages (place as today, message from `LineError`):
| Problem | Message |
|---|---|
| quantity negative or not finite | `quantity must be a finite number of at least 0 (got -2)` |
| slope not finite | `slope must be a finite number (got NaN)` |
| offer with a negative slope | `slope of a supply line must be at least 0 (got -1.5)` |
| bid with a positive slope | `slope of a demand line must be at most 0 (got 1.5)` |
| maximum negative or not finite | `maximum must be a finite number of at least 0 (got inf)` |
| give-up negative or not finite | `give-up price must be a finite number of at least 0 (got -1)` |
| quantity above maximum (both valid) | `quantity at the anchor price (5) is above the maximum (4)` |

`number_problems` (the format layer, used by `Decision::from_json`, `Reply::from_json`, `Recording::from_json`) runs `line_problems` on every offer and bid with `STAND_IN_ANCHOR`, after the factory fraction checks, offers then bids, so a decision read from JSON with, say, a negative quantity is refused as `FormatError::Values(["offer 'grain': quantity must be a finite number of at least 0 (got -1)"])`. (JSON can't carry NaN or infinity, so in practice this layer catches negatives, wrong-way slopes and a quantity above its maximum.) Since the anchor is always valid, `LineError::AnchorPrice` can't be produced anywhere in this module; don't special-case it.

A line that trades nothing (maximum 0, or quantity 0 with slope 0) is valid. Holdings and cash are never checked.

### Engine: `world.rs`
```rust
/// A valid decision's lines on one side, each evaluated at its good's posted price on `hex`,
/// as (good, amount) in good ID order, leaving out zero amounts. A valid decision names
/// only goods with a posted price and holds only well-formed lines, so lookups and
/// `Line::new` are `expect`ed, as `resolve_amounts` did.
fn resolve_lines(
    goods: &Goods,
    prices: &HexPrices,
    hex: HexId,
    side: Side,
    lines: &BTreeMap<String, OrderLine>,
) -> Vec<(GoodId, Quantity)>;
```
For each `(name, line)`: `good = goods.id(name)`, `price = prices.get(hex, good)` (both `expect`ed), `amount = Line::new(side, &line.params(price.get())).expect(...).at(price)`. Keep amounts `> 0`, sort by good ID. `business_submission` passes `self.rules.goods`, `&self.hex_prices`, the business's hex, `Side::Supply` for offers and `Side::Demand` for bids. Because the anchor equals the evaluation price, `at` returns the line's quantity unless the give-up excludes the price (a bid with `give_up < price`, an offer with `give_up > price`), when it returns zero; the slope and the clip never act here. Say so in a comment: Step 04 replaces this with the crossing.

### Dev UI: `text/businesses.rs`
```rust
/// The quantity of the line `map` holds for `name`, f64 `Display`, "0" when it lacks the good.
fn sent(map: &BTreeMap<String, OrderLine>, name: &str) -> String;
```
Nothing else in `trade_rows` changes: a valid answer's rows come from `BusinessTrade` (the resolved amounts), a refused decision's rows from the stored decision's lines.

### Python: `protocol.py`
```python
@dataclass
class OrderLine:
    """One order line as a decision carries it: the quantity at the posted price, the slope,
    the maximum, and the give-up price. The engine anchors it at the good's posted price."""
    quantity: Number
    slope: Number
    maximum: Number
    give_up: Number

    @staticmethod
    def flat(amount: Number, give_up: Number) -> "OrderLine":
        """A flat line: `amount` at every price the give-up allows (slope 0.0, maximum
        `amount`)."""

@dataclass
class Decision:
    business: str
    factories: dict[int, FactoryOrder] = field(default_factory=dict)
    offers: dict[str, OrderLine] = field(default_factory=dict)
    bids: dict[str, OrderLine] = field(default_factory=dict)
    trace: str | None = None

_LINE_FIELDS = ("quantity", "slope", "maximum", "give_up")
def _line_from_data(data: object, what: str) -> OrderLine      # _record(data, what, _LINE_FIELDS); each field through _number(value, f"{what} {key}")
def _line_to_data(line: OrderLine) -> dict                      # the four keys in _LINE_FIELDS order
def _lines(value: object, what: str) -> dict[str, OrderLine]    # {good: _line_from_data(item, f"{what} '{good}'") for good, item in _table(value, f"{what}s").items()}
def _lines_data(table: dict[str, OrderLine]) -> dict            # sorted by good, each _line_to_data
```
`decision_from_data` uses `_lines(data.get("offers", {}), "offer")` and `_lines(data.get("bids", {}), "bid")`, so the messages read `unknown key 'x' in offer 'grain'`, `missing key 'give_up' in bid 'labor'`, `offer 'grain' quantity must be a number`, `offers must be a table`. `decision_to_data` writes `_lines_data(decision.offers)` and `_lines_data(decision.bids)` in the same key positions as today. Value ranges are still not checked in Python.

### Python: `arithmetic.py`
```python
def flat_lines(view: BusinessView, table: dict[str, float]) -> dict[str, OrderLine]:
    """Each amount as a flat line at the good's posted price: the quantity and the maximum
    are the amount, the slope is 0 and the give-up price is the posted price, so the line
    says "sell this much at or above the current price" or "buy this much at or below it".
    Every good in `table` has a posted price (`trade` skips the others)."""
    # OrderLine.flat(amount, float(view.price(good))) per good
```
`decide_business` ends with `Decision(view.business, orders, flat_lines(view, offers), flat_lines(view, bids), "\n".join(lines))`. The no-market early return is unchanged. `float(...)` on the price keeps every number in a line a `float`, as the amounts are.

## Tasks
1. `decision.rs`: add `OrderLine` with `flat` and `params`; change the two `Decision` fields; add `STAND_IN_ANCHOR` and `line_problems`; rewrite `validate`'s offer/bid loop and `number_problems`; delete `amount_problem`; update the module doc. Re-export `OrderLine` in `mod.rs`, delete `is_amount`, update `mod.rs`'s module doc and `lib.rs`'s crate doc.
2. `world.rs`: replace `resolve_amounts` with `resolve_lines`; update `business_submission` and the module doc. `trade.rs`: doc comments only.
3. Fix every compile error in the engine's unit tests (`decision.rs`, `batch.rs`) by building lines, then add the new `decision.rs` tests (Test Plan).
4. `crates/dev-ui/src/text/businesses.rs`: `sent`, doc comments, the test helper and tests.
5. Engine integration tests: `tests/decider.rs` and `tests/business_trade.rs` (Test Plan).
6. `protocol.py`: `OrderLine`, the readers and writers, the docstring. `arithmetic.py`: `flat_lines`, the return, the docstring.
7. Python tests: update and extend `test_protocol.py` and `test_arithmetic.py` (Test Plan).
8. Regenerate the sample recording's reply half (Test Plan, "Regenerating"). Confirm the recording's `request` half is byte-for-byte unchanged (`git diff` shows changes only inside `"reply"`).
9. Run `.venv/Scripts/python tools/check.py` and fix formatting, lints and failures until it passes.

## Test Plan
Run everything with `.venv/Scripts/python tools/check.py` (fmt, clippy `-D warnings`, `cargo test --workspace`, pytest). The tests that start the real Python decider (`external_decider.rs`, the sample test in `business_trade.rs`, the dev UI's `python_world` tests) and `recordings.rs` pass only after task 8.

**Regenerating the recording (task 8)**, from the project root, in this order. PowerShell:
```
$env:POLICYSPACE_UPDATE_RECORDINGS = "1"
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # rewrites the request half; the old reply no longer reads, so it writes an empty reply
.venv/Scripts/python -m pytest python/tests                              # rewrites the reply half from the rule
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # must pass
Remove-Item Env:POLICYSPACE_UPDATE_RECORDINGS
```
(Bash: `export POLICYSPACE_UPDATE_RECORDINGS=1` ... `unset POLICYSPACE_UPDATE_RECORDINGS`.) The first cargo run's reply check fails: expected. Never set the variable inside a test.

**Engine unit tests, `decision.rs`** (replace the `table` helper with `lines(&[(&str, OrderLine)])` plus `flat(amount, give_up)`; keep `full()` valid against `test_view`, whose prices are labor 1 and grain 2):
- An empty decision and a full decision (an offer line and a bid line) are valid; asking for far more than held or affordable is valid; a line that trades nothing (maximum 0; quantity 0 with slope 0) is valid; a sloped offer and a sloped bid with the right signs are valid. (AC 2)
- `every_refusal_is_reported_at_its_place`, rewritten: factories as today; offers `silver` (the currency, a valid line), `copper` (no posted price, a valid line) and `grain` with quantity −2 and slope −1 (two line problems: the quantity, then the wrong-way slope); bids `wheat` (unknown good, a valid line), `labor` with slope 1.5 and give-up −1 (two line problems) and `grain` with quantity 5 and maximum 4 (one problem). Assert the exact message list in document order: the four factory lines as today, then `offer 'copper': no price is posted for it on this hex`, `offer 'grain': quantity must be a finite number of at least 0 (got -2)`, `offer 'grain': slope of a supply line must be at least 0 (got -1)`, `offer 'silver': it is this hex's currency, which can't be traded against itself`, `bid 'grain': quantity at the anchor price (5) is above the maximum (4)`, `bid 'labor': slope of a demand line must be at most 0 (got 1.5)`, `bid 'labor': give-up price must be a finite number of at least 0 (got -1)`, `bid 'wheat': unknown good`; and the count, 12. (AC 2)
- A line problem and a good problem on one entry are both reported: an offer of `wheat` with a negative slope gives the slope message then `unknown good`. (AC 2)
- Round trips: a full decision with sloped lines and awkward numbers (`1.0/3.0`, `5e-324`, `f64::MAX` as a maximum) through both writers. (AC 1)
- Exact JSON: `Decision::empty("a")` unchanged; a decision with one flat offer writes `"offers":{"grain":{"quantity":8.0,"slope":0.0,"maximum":8.0,"give_up":2.0}}`. (AC 1)
- The hand-written decision above reads (integers become `f64`s) and `from_json(r#"{"business": "a"}"#)` is empty. (AC 1)
- Reading refuses: a line missing `give_up` (`FormatError::Json` containing `missing field`); an unknown key in a line (`unknown field`); a repeated good; a line whose `quantity` is a string (`invalid type`). `from_json` refuses with `Values` a negative quantity (`offer 'grain': quantity must be a finite number of at least 0 (got -1)`), a bid with slope 1 (`bid 'labor': slope of a demand line must be at most 0 (got 1)`) and quantity 5 with maximum 4 (`offer 'grain': quantity at the anchor price (5) is above the maximum (4)`); `1e999` and `NaN` inside a line are refused with a message. (AC 1, 2)
- `OrderLine::flat(8.0, 0.5)` has quantity 8, slope 0, maximum 8, give-up 0.5; `params(2.0)` carries the anchor and the four numbers. (AC 1)

**Engine unit tests, `batch.rs`:** keep every test; where an offer or bid is inserted use `OrderLine::flat`; the prefixed `Values` expectations become `a: offer 'grain': quantity must be a finite number of at least 0 (got -1)` and `a: bid 'labor': quantity must be a finite number of at least 0 (got -3)`.

**Engine integration tests, `tests/decider.rs`** (public API; `use policyspace_engine::decider::OrderLine`):
- `full_decision` and `a_decision_validates_against_the_sample_view` use flat lines with the posted prices as give-ups (unimproved_grain 0.5, unskilled_labor 1); the greedy decision is still valid. Add to that test: an offer of `unimproved_grain` with slope −1 is refused as `offer 'unimproved_grain': slope of a supply line must be at least 0 (got -1)`; a bid of `tools` with quantity 2 above maximum 1 is refused with the maximum message; a bid for `silver` with a negative slope reports the slope message and then the currency message. (AC 1, 2)
- `a_hand_written_decision_reads_and_validates` uses the new hand-written text above. (AC 1)
- Every message round-trips on the sample as today, with the new decisions. (AC 1)

**Engine integration tests, `tests/business_trade.rs`:** the `decision` helper keeps its `(&str, f64)` lists and builds `OrderLine::flat(amount, 0.0)` for offers and `OrderLine::flat(amount, f64::MAX)` for bids (lines that trade at any price), so every existing expectation stands unchanged. Add one test on the `business_trade` setting (prices labor 1, ore 2, iron 6, tools 10, bread 3), with a closure decider and `works`:
- a bid for `ore` with give-up 1.5 (below the posted 2) and an offer of `iron` with give-up 7 (above the posted 6) resolve to nothing: `bid()` and `offered()` don't list them, nothing is bought or sold of them;
- a bid for `ore` with give-up exactly 2 and an offer of `iron` with give-up exactly 6 trade their quantities (the give-up is inclusive);
- a sloped bid for `labor` (quantity 10, slope −3, maximum 20, give-up 5) resolves to 10 at the posted price 1, and a line with maximum above quantity resolves to its quantity;
- an offer above holdings is still trimmed to what is held and recorded as offered beside sold, and a bid beyond cash is still scaled and recorded (the existing `poor` expectations cover this; assert them in the new test too). (AC 3)

**Dev UI unit tests, `text/businesses.rs`:** the test `decision` helper builds lines as `business_trade.rs` does; `a_refused_decision_shows_a_misspelt_good_and_its_trace` keeps its expected table (the quantities); the steady-state trade table expectations are unchanged. (AC 3, Player-facing Changes)

**Python, `test_protocol.py`:**
- Reading refuses: a line missing `give_up` (`match="missing key 'give_up' in offer 'g'"`), an unknown key in a line, a line whose quantity is a string or a bool, a line that is a number instead of a table, `NaN` inside a line, a repeated good. (AC 5)
- Defaults: the empty decision's exact text is unchanged; `read_decision` of a decision with one line gives `OrderLine(8, 0, 8, 0.5)` with the parsed types kept. (AC 5)
- Written order: `write_decision` of a decision with lines writes each line as `{"quantity":...,"slope":...,"maximum":...,"give_up":...}` and the goods sorted. (AC 5)
- The exactness request still comes back byte for byte (it carries no decisions; keep the test). The regenerated sample recording round trips in pretty form. Every request file round trips. (AC 5)

**Python, `test_arithmetic.py`:**
- `test_the_sample_reply_is_the_recordings` unchanged in logic (it is the regeneration hook). (AC 4, 5)
- `test_the_sample_numbers`: compare quantities through a helper `quantities(table) -> dict[str, float]` with today's numbers, and assert every line is flat at the posted price: `slope == 0.0`, `maximum == quantity`, `give_up == view.price(good)` (read the sample request's views for the prices). (AC 4)
- The hand-written cases compare `quantities(decision.offers)` / `quantities(decision.bids)` against today's dicts. (AC 4)
- `test_every_decision_is_well_formed` adds: every line has `quantity > 0`, `slope == 0.0`, `maximum == quantity`, `give_up == float(view.price(good))`, and all four are `float`. (AC 4)

**Existing tests that must keep passing unchanged:** `recordings.rs` (after regeneration), `external_decider.rs` (the real decider's reply equals the recording's), the sample test in `business_trade.rs` (the same bought amounts), `decider_host.rs` (replays the regenerated recording), the dev UI's `python_world` tests and `checker.rs`, `test_loop.py`. Together these are AC 4's "the same trades, cash, conditions, trace phrases and turn lines", and AC 5's fixture check (`the_sample_recording_matches_the_engine`: request equal, canonical form, one valid answer per view).

Mapping: AC 1 → the round-trip, exact-JSON, hand-written and anchoring tests; AC 2 → the refusal tests in `decision.rs` and `tests/decider.rs`; AC 3 → the `business_trade.rs` dummy-round test and the dev UI tests; AC 4 → `test_arithmetic.py` and the unchanged Python-driven engine tests; AC 5 → `test_protocol.py` and `recordings.rs`; AC 6 → the whole check command.

## Constraints
- Don't change `orders.rs`, `market.rs`, the trade round's logic in `trade.rs`, `production.rs`, the setting loader, `settings/sample/`, any `*.request.json`, `recordings/README.md`, `view.rs`, `host.rs`, `replay.rs`, `external.rs`, `python.rs`, `recordings.rs`, `docs/` or the manual. The view, the sample setting, the turn report and the failure semantics are untouched.
- The sample recording is only ever rewritten by the regeneration procedure, never by hand.
- Keep every protocol type `Debug, Clone, PartialEq, Serialize, Deserialize` with `deny_unknown_fields`; every `BTreeMap` field keeps `no_duplicate_keys`. `serde_json` stays confined to `decider/mod.rs`.
- One source of line rules: the protocol never re-implements a check or a message that `Line::new` makes. Python checks structure and types only, never value ranges.
- Deterministic iteration (`BTreeMap`, sorted lists); no hash maps; no new dependencies on either side.
- Every public item has a doc comment; test-only code is `#[cfg(test)]`; `is_amount` is removed rather than left dead.
- Out of scope: moving prices, summing lines or the crossing in the turn (Step 04); POP lines (Step 03); market data in the view and a rule that uses it (Step 05); orders on file (Step 06); showing slope, maximum or give-up in the dev UI (Step 10); bug 01.

## Done When
All tasks complete, `.venv/Scripts/python tools/check.py` passes (formatting, clippy with `-D warnings`, every Rust test including the regenerated recording's fixture test and the tests that run the real Python decider, the Python tests), the sample recording's request half is unchanged, and every Acceptance Criterion in `design.md` is met by a test in the Test Plan.
