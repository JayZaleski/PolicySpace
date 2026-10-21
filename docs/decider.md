# The decider protocol

Source: `crates/engine/src/decider/` — `mod.rs` (module docs, re-exports, `FormatError` and the shared JSON helpers), `view.rs` (`BusinessView` and how it is built from a World), `decision.rs` (`Decision`, its validation and errors) and `batch.rs` (`Request`, `Reply`, `Recording`, reply validation). Also the JSON forms of `Quantity` (`quantity.rs`) and `Turn` (`turn.rs`), and `World::business_view` (`world.rs`). The crate root re-exports `Answer`, `BusinessView`, `Decision`, `DecisionErrors`, `FormatError`, `Recording`, `Reply`, `ReplyErrors` and `Request`. The rest is public under `policyspace_engine::decider` (`FactoryOrder`, `Place`, `DecisionError`, `ReplyError`, `PerishGroup`, `FactoryView`, `TechnologyView`, `DepositsView`), and the submodules are public as `decider::view`, `decider::decision` and `decider::batch`. The runtime around the protocol (the host, the decision point and the deciders themselves) is in [decider-host.md](decider-host.md).

A **decider** is whatever chooses what a business does each turn: a script in another process, a replayed recording, or a test. This module defines, as plain data, what the engine and a decider say to each other:

- the **view** a business is served each turn: the data it may know, and nothing more;
- the **decision** it answers with: the kinds of action it may take, with the bounds the engine enforces, and an optional trace;
- **validation** of a decision against the view it answers;
- the **batch** of one turn's exchange (every view out, every decision back) and the **recording** that keeps both;
- **lossless JSON** for all of it, keyed by setting names and factory ID numbers, never by engine IDs.

**The engine runs the exchange once per turn** and acts on it. At the decision point, through a host of named deciders the caller builds (a Python process, a replayed recording, or a Rust closure), every business's view is served and its decision validated and stored in the World (see [decider-host.md](decider-host.md)). The World then resolves each valid decision: the offers and bids go through the hex's trade round, and the fractions go to the factory mechanics (see [trade.md](trade.md)). The other side of the protocol exists in Python: the `policyspace` package reads and writes these messages byte for byte and holds the first decider, the arithmetic rule (see [python.md](python.md)). The two sides are tied together by committed recordings that both test suites read (see [Recordings](#recordings)). The dev UI shows each business's decision, its trace and its outcome (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## Concepts

- **Where the view is taken.** The decision point is the start of the "actors act" part of the turn, before any trading (see [world.md](world.md#the-turn)). So a business's stockpile then holds this turn's emission from its deposits (unimproved goods that perish at the end of this turn), the output its factories made last turn, and the currency it carried over. Building a view reads the world and changes nothing.
- **Scoped to the business.** A view holds the business's hex's currency and posted prices, its own stockpile, cash, factories, their technologies and its deposits. Nothing about other businesses, POPs, other hexes, hex stockpiles or totals is in it, so a decider can't act on anything it couldn't know.
- **Names, not IDs.** Goods, technologies, resource types and businesses go by their setting names. Factories have no names, so they go by their ID number (`FactoryId::index()` as a `u32`), which is the factory's position in load order and stable for a given setting. No `Id<K>` value ever appears in a message (see [ids.md](ids.md)), and nothing in the module holds a reference into the World.
- **Tables, not lists, wherever a key is unique.** Prices, stockpile goods, technologies, deposits, bundles, a view's factories, and a decision's factories, offers and bids are all `BTreeMap`s: by good, technology or resource-type name, or by factory number. A table can't hold one key twice, so a decision can't list a factory or a good twice; a Python decider that builds dicts gets this for free. Keys come out in sorted order (alphabetical, or numeric for factories), which keeps the JSON deterministic.
- **Asking for too much is not an error.** Offering more of a good than the stockpile holds, bidding for more than cash can pay, or offering and bidding the same good is a valid request. The trade round trims it when it resolves it: an offer sells at most what is held, and bids that cost more than the cash held after the hex's sales are all scaled by one proportion; the factory mechanics then cut runs short of goods. The business's outcome records what was asked beside what was done (see [trade.md](trade.md#the-cash-cap) and [production.md](production.md#a-safety-net-not-the-normal-path)). Validation refuses only nonsense.
- **Doing nothing is legitimate.** A factory a decision doesn't mention is run at 0 and maintained at 0, so it only wears; a fraction left out is 0; a business with no decision in a reply trades nothing that turn and its factories only wear. `Decision::empty(name)` is the explicit "do nothing".

## The view

```rust
pub struct BusinessView {
    pub turn: Turn,
    pub business: String,                                  // the business's name
    pub currency: String,                                  // its hex's currency good
    pub cash: Quantity,                                    // = the currency's total in `stockpile`
    pub prices: BTreeMap<String, Quantity>,                // the hex's posted prices, in the currency
    pub stockpile: BTreeMap<String, Vec<PerishGroup>>,     // every good held, by perish group
    pub factories: BTreeMap<u32, FactoryView>,             // by factory ID number
    pub technologies: BTreeMap<String, TechnologyView>,    // the factories' technologies, once each
    pub deposits: BTreeMap<String, DepositsView>,          // per resource type
}
pub struct PerishGroup { pub amount: Quantity, pub perishes: Option<Turn> }
pub struct FactoryView { pub technology: String, pub condition: f64 }
pub struct TechnologyView {
    pub inputs: BTreeMap<String, Quantity>,       // per unit of output; may be empty
    pub output: String,
    pub capacity: Quantity,
    pub capital: BTreeMap<String, Quantity>,      // never empty
    pub construction: BTreeMap<String, Quantity>, // may be empty
    pub construction_turns: u32,
    pub lifetime: u32,
}
pub struct DepositsView { pub count: u32, pub produces: String, pub output: Quantity }
```

`World::business_view(business) -> BusinessView` builds a view of the world at its current turn (it delegates to the crate-private `decider::view::build`). It panics if `business` isn't from this world's businesses. It reads only the World's public accessors:

1. `turn`, the business's name, its hex (`businesses().get(business).hex`) and that hex's currency (`hex_currency(hex)`), as a name.
2. `prices` from `hex_prices().posted(hex)`, by good name. The loader never prices the currency, so it isn't listed. A hex with no market posts nothing, which gives an empty table, the same shape as a market that prices nothing: to a decider both mean nothing is tradable.
3. `stockpile` from `business_stockpile(business)`: one entry per good held, each with its perish groups from `Stockpile::groups` in order (soonest first, the never-perishing group last, only groups above 0). `PerishesAt::EndOfTurn(t)` becomes `perishes: Some(t)` and `Never` becomes `None`. Totals aren't sent; a decider sums the groups. `cash` is `stockpile.total(currency)`, sent separately for convenience.
4. `factories` from `factories().owned_by(business)`: key `u32::try_from(id.index())`, value the technology's name and the condition.
5. `technologies`: the distinct technologies of those factories, each once with every number the rules give it (see [technologies.md](technologies.md)), bundles as name-to-amount tables. Listing them once keeps a business with many identical factories from receiving its recipe many times.
6. `deposits`: `deposits().owned_by(business)` counted per resource type, each with the good it produces and one deposit's output per turn. Deposits of one type on one hex are identical and nothing addresses a single deposit yet, so they are summed.

Because the world is observed after part 2 of a turn, `business_view` called between steps shows the business after this turn's trade and operation (this turn's output held, conditions after wear and maintenance), not what its decider was served at this turn's decision point; that is next turn's view, before next turn's emission (see [world.md](world.md#what-a-world-holds)).

## The decision

```rust
pub struct Decision {
    pub business: String,
    pub factories: BTreeMap<u32, FactoryOrder>,   // absent in JSON = empty
    pub offers: BTreeMap<String, f64>,            // amount to sell, per good; absent = empty
    pub bids: BTreeMap<String, f64>,              // amount to buy, per good; absent = empty
    pub trace: Option<String>,                    // absent in JSON when None
}
pub struct FactoryOrder { pub run: f64, pub maintenance: f64 }  // each defaults to 0
```

- `run` and `maintenance` are the run fraction and the maintenance fraction of [production.md](production.md#two-instructions-per-factory): finite, 0 to 1 inclusive.
- Offer and bid amounts are plain `f64`, not `Quantity`, so a bad amount is reported at its place by validation (with every other problem) rather than failing the parse at the first one.
- The **trace** is free text, any number of lines: the decider's account of its choice. The engine stores it in the business's answer, for the dev UI, and never acts on it.

### Validation

`Decision::validate(&self, view: &BusinessView, goods: &Goods) -> Result<(), DecisionErrors>` checks a decision against the view it answers and the setting's goods (`world.rules().goods`). The goods list is needed because the view alone can't tell an unknown good from one this hex doesn't price, and the two are reported differently. It reads the whole decision and reports every problem, each at its place, in document order: the decision itself, then the factories by number, then the offers, then the bids by name.

| Place | Problem | Message |
|---|---|---|
| `decision` | `business` isn't the view's | `answers for 'b' against the view of 'a'` |
| `factory #N` | not in the view (not this business's, or no such factory) | `not in this business's view` |
| `factory #N` | a fraction negative, above 1 or not finite | `run must be a finite number from 0 to 1 (got 1.5)`, likewise `maintenance` |
| `offer 'g'`, `bid 'g'` | amount negative or not finite | `amount must be a finite number of at least 0 (got -2)` |
| `offer 'g'`, `bid 'g'` | no such good | `unknown good` |
| `offer 'g'`, `bid 'g'` | the hex's currency | `it is this hex's currency, which can't be traded against itself` |
| `offer 'g'`, `bid 'g'` | no posted price on this hex | `no price is posted for it on this hex` |

The checks are independent: an unknown factory with a bad fraction reports both, and an offer with a bad amount and an unknown good reports both. Of the three good checks only the first that applies is reported. Numbers in messages use `f64`'s `Display` (`1.5`, `-0.1`, `NaN`, `inf`). Holdings and cash are never checked.

The errors copy the loader's style (see [setting-loader.md](setting-loader.md)):

- `Place` is `Decision`, `Factory(u32)`, `Offer(String)` or `Bid(String)`, displayed as `decision`, `factory #3`, `offer 'grain'`, `bid 'grain'`.
- `DecisionError` has `place()` and `message()`, and displays as `{place}: {message}`.
- `DecisionErrors` is `ErrorList<DecisionError>`, the engine's one never-empty error list (`crates/engine/src/error_list.rs`, re-exported as `ErrorList`; `LoadErrors` and `ReplyErrors` are aliases of the same type). It has `iter`, `len`, `is_empty` (always false) and `IntoIterator for &DecisionErrors`, and displays one error per line with no trailing newline. Both it and `DecisionError` implement `std::error::Error`.

A refused decision idles its business for the turn and is kept with its errors; the run goes on (see [decider-host.md](decider-host.md#failures)).

## The batch

```rust
pub struct Request { pub turn: Turn, pub views: Vec<BusinessView> }       // the engine's message
pub struct Reply { pub turn: Turn, pub decisions: Vec<Decision> }        // the deciders' answer; absent decisions = empty
pub struct Recording { pub request: Request, pub reply: Reply }          // one turn, saved
pub enum Answer<'a> {
    Nothing,
    Valid(&'a Decision),
    Refused { decision: &'a Decision, errors: DecisionErrors },
}
```

The turn has one decision point, where every business's view is served and every decision collected in one exchange. A reply may hold decisions for any subset of the request's businesses, in any order, because a decider may control any set of businesses, and the host splits one turn among several deciders, one request each (see [decider-host.md](decider-host.md#the-decision-point)).

`Reply::validate(&self, request, goods) -> Result<Vec<Answer>, ReplyErrors>` first makes the reply-level checks and, if any fail, refuses the whole reply with every problem found, in order (the turn first, then the decisions in reply order):

- `TurnMismatch { request, reply }` — `the reply answers turn 1, but the request is for turn 0`;
- `UnknownBusiness { name }` — `a decision for 'nobody', which isn't in the request`;
- `DuplicateBusiness { name }` — `two decisions for 'center_mine'`.

It collects the request's business names into a `BTreeSet` and the decisions into a `BTreeMap` by business name in one pass over the reply (a name not in the set is `UnknownBusiness`, checked first; a second decision for one name is `DuplicateBusiness`), so the checks don't rescan the lists per decision. `ReplyErrors` is `ErrorList<ReplyError>`, with the same shape and impls as `DecisionErrors`. Otherwise it returns one `Answer` per view, in the request's order: `Nothing` for a business with no decision, `Valid(&decision)` or `Refused { decision, errors }` from `Decision::validate` against that business's view, the refused decision carried beside its errors so the caller needn't search the reply for it. A refused decision doesn't refuse the reply.

A **recording** is a request and its reply saved together. The Python tests read recordings to drive the deciders, and the replay decider answers from them, so engine tests that don't test strategy need no Python (see [decider-host.md](decider-host.md#the-replay-decider)). The committed recordings are described in [Recordings](#recordings).

Nothing checks that a request is consistent in itself (each view's turn against the request's, or two views of one business). A request the engine builds can't have either problem. A request read from a file is never trusted on its own: the replay decider answers only when the engine's own request for that turn equals the recorded one.

## JSON

Every message type has `to_json()` (compact, one line), `to_json_pretty()` (indented, for recordings people read and diff) and `from_json(text) -> Result<Self, FormatError>`. JSON was chosen because Python reads and writes it with nothing installed, every language has it, it is readable by eye, and a saved turn can be diffed.

### Format

Field order in the structs is the key order in the JSON. The farm's view at the sample's turn-0 decision point (the request in `sample_turn0.recording.json`), as `to_json_pretty` writes it:

```json
{
  "turn": 0,
  "business": "center_farm",
  "currency": "silver",
  "cash": 100.0,
  "prices": {
    "copper": 5.0,
    "grain": 2.0,
    "tools": 12.0,
    "unimproved_copper": 1.0,
    "unimproved_grain": 0.5,
    "unskilled_labor": 1.0
  },
  "stockpile": {
    "silver": [
      {
        "amount": 100.0,
        "perishes": null
      }
    ],
    "unimproved_grain": [
      {
        "amount": 8.0,
        "perishes": 0
      }
    ]
  },
  "factories": {
    "0": {
      "technology": "farm",
      "condition": 1.0
    }
  },
  "technologies": {
    "farm": {
      "inputs": {
        "unimproved_grain": 1.0,
        "unskilled_labor": 0.5
      },
      "output": "grain",
      "capacity": 12.0,
      "capital": {
        "tools": 4.0
      },
      "construction": {
        "unskilled_labor": 8.0
      },
      "construction_turns": 2,
      "lifetime": 8
    }
  },
  "deposits": {
    "arable_land": {
      "count": 2,
      "produces": "unimproved_grain",
      "output": 4.0
    }
  }
}
```

A decision for it, written by hand:

```json
{"business": "center_farm", "factories": {"0": {"run": 1}}, "offers": {"unimproved_grain": 8}, "bids": {"unskilled_labor": 6}, "trace": "sell the grain"}
```

- **Factory keys are quoted.** JSON requires a table's keys to be text, so a factory number appears as `"0"`. serde_json writes `u32` keys that way and reads them back as numbers; a decider that copies the key from the view into its decision needs to know nothing about this. A key that isn't a number (`"x"`) is refused.
- **`perishes` is always written**, as a turn number or `null` for "never perishes", so every group has both keys. A hand-written view that leaves the key out reads as never perishing (serde fills a missing `Option` with `None`).
- A decision's `factories`, `offers` and `bids` may be left out (empty), as may each fraction (0) and the trace. `Decision::empty("a").to_json()` is `{"business":"a","factories":{},"offers":{},"bids":{}}`; the trace key is written only when there is one. A reply's `decisions` may be left out too.
- Every struct carries `#[serde(deny_unknown_fields)]`, so a misspelled key (`"offer"`, `"cach"`) is refused, as the setting loader refuses unknown keys.

### The lossless rule

Quantities, fractions and turn numbers come back from a round trip exactly as they went in, for every value the engine can hold (`f64::MAX`, `5e-324`, `1.0 / 3.0`, `Turn::MAX`).

- **`Quantity`** has hand-written `Serialize` and `Deserialize` in `quantity.rs`, the one place its written form is defined. It is written as a plain JSON number (serde_json writes the shortest text that reads back to the same `f64`) and read as any JSON number, integers included, through `Quantity::new`, which refuses a negative value with `a quantity must be a finite number of at least 0, not -1` and turns `-0` into 0. JSON has no NaN or infinity literals, and an overflowing literal such as `1e999` is refused by the parser as out of range. If the engine ever changes its number representation, the JSON form changes in `quantity.rs` and nowhere else.
- **`Turn`** derives `Serialize` and `Deserialize` as a newtype over `u64`, so it is a bare JSON integer. serde refuses `3.0`, `-1` and `"3"`. `u64` holds the largest turn exactly, and Python reads it exactly too.
- **Fractions** (`run`, `maintenance`, `condition`) are `f64` fields, checked after parsing.
- **`serde_json` needs its `float_roundtrip` feature.** Without it, serde_json's float parser is fast but not correctly rounded, and a value such as `123456789.123456789` reads back one unit in the last place off. Don't drop the feature from `crates/engine/Cargo.toml`.

### Reading in two layers

`from_json` parses with serde, then runs value checks a parse can't make, so a message read from JSON is either well-formed or refused with a clear message:

1. **Parse.** Any serde error (bad syntax, a wrong type, a missing or unknown key, a key written twice, a negative quantity, a fractional turn, a number out of range) becomes `FormatError::Json(message)`, the parser's message, which ends with the line and column. Serde stops at the first such error.
2. **Values.** The parsed value is checked, and every problem is collected into `FormatError::Values(lines)`: for a decision, the number checks of validation (`Decision::number_problems`: fractions and amounts, at their places); for a view, each factory condition that isn't a fraction (`factory #0: condition must be a finite number from 0 to 1 (got 1.5)`). In a request or reply each line is prefixed with its business (`center_farm: offer 'grain': amount must be ...`); a recording lists the request's problems, then the reply's.

`FormatError` displays `Json` as its message and `Values` as its lines joined by newlines, with no trailing newline, and implements `std::error::Error`. `Decision::validate` runs the number checks again with the view checks, so a decision built in Rust is checked too.

**Duplicate keys.** A `BTreeMap` would silently keep the last of two entries with one key. Every map field instead uses `#[serde(deserialize_with = "crate::decider::no_duplicate_keys")]`, a `MapAccess` visitor that fails with `the key 'grain' is written twice`.

**Why serde here and not in the loader.** The setting loader is hand-written because it must report every error in a file, and serde stops at the first (see [setting-loader.md](setting-loader.md#entryreader)). A decision is produced by a program, not edited by a person, so a format error is a bug in the decider, and the first one is enough. The value problems a decider is likely to make are still all reported, by the second layer.

### Where serde_json lives

`serde_json` is used only in `decider/mod.rs`, through three crate-private helpers: `to_json`, `to_json_pretty` (both `expect` success, since the types hold only strings, finite numbers and maps) and `from_json` (mapping the parser's error to `FormatError::Json`), plus `checked(problems)`, which turns a non-empty problem list into `FormatError::Values`. So the JSON library could be swapped in one file. `serde`'s derives and attributes appear on every protocol type; `Quantity` has no serde attribute beyond its hand-written impls. `is_fraction` and `is_amount` in `mod.rs` are the shared number rules.

## The Python side

The deciders are Python processes, and `python/policyspace/protocol.py` is this format's other implementation (see [python.md](python.md#the-protocol-protocolpy)). It holds the same messages as dataclasses with the same field names, reads them as strictly (unknown, missing and repeated keys, wrong types, `NaN` and booleans refused), and leaves value checks to the engine's validation. Its writer reproduces serde_json's layout, both compact and pretty, so a message the engine writes comes back from Python byte for byte.

**Transport is one message per line.** `to_json` writes a message on one line, so a request is one line in and a reply is one line out. `python -m policyspace <decider>` answers each request line with one reply line until its input ends (see [python.md](python.md#the-loop-and-the-command-line-deciderspy-__main__py)). A reply from it answers every business in the request, in the request's order, with an empty decision for a business on a hex without a market. The engine's external-process decider starts the process once, keeps it alive across turns, and drives this exchange with a time limit per turn (see [decider-host.md](decider-host.md#the-external-process-decider)).

## Recordings

`crates/engine/tests/recordings/` holds committed protocol messages that the engine's tests and the Python tests both read, and that the replay decider answers from in the engine's tests. Its `README.md` describes the files. There are three kinds:

| File | Written by | Form | Purpose |
|---|---|---|---|
| `sample_turn0.recording.json` | the engine (`request` half) and the Python test (`reply` half) | pretty, trailing newline | the sample's turn-0 request for its three center businesses, and the arithmetic rule's reply |
| `exactness.request.json` | the engine | compact, one line plus a newline | extreme and awkward numbers (`Turn::MAX`, `f64::MAX`, `5e-324`, `1/3`, `1e-5`, `1e16` and others Python's own writer lays out differently); Python must write it back byte for byte |
| `negative_margin`, `unpriced_input`, `cash_short`, `revival`, `no_market`, `seller` (`.request.json`) | hand | pretty | one business each, for cases the sample doesn't reach; the Python tests assert what the rule does with them |

`.gitattributes` pins the folder to LF line endings (`crates/engine/tests/recordings/** text eol=lf`), because the tests compare these files byte for byte and git on Windows would otherwise check them out with CRLF.

`crates/engine/tests/recordings.rs` keeps the files honest, through the public API and `tests/common` only:

- `the_sample_recording_matches_the_engine` builds the sample world under a closure decider registered as `arithmetic` that keeps a copy of the request it is served and answers nothing, so the request is exactly the one turn 0's decision point builds (the loaded state plus emission, one `business_view` per business in ID order). It can't build the request from the world afterwards, because a world is already past turn 0's trade and operation when `World::new` returns. It checks the file's request equals it, the file is in the engine's canonical pretty form (which also proves Python's pretty writer matches), and the file's reply validates against the request with one `Answer::Valid` per view, in the request's order. If the sample setting, the view or the rule changes, this test fails, so the recording can't drift from the engine. `crates/engine/tests/decider_host.rs` replays the same file through the World (three valid answers at turn 0), `crates/engine/tests/external_decider.rs` checks that the real `arithmetic` process answers the sample's turn 0 with the recorded reply, and `crates/engine/tests/business_trade.rs` checks the trades and factory runs that reply leads to.
- `the_exactness_request_is_the_engines` builds the exactness request by hand and checks the file equals its `to_json()` plus a newline. The literal `123456789.123456789` is written `123_456_789.123_456_79`, the same `f64`, because clippy's `excessive_precision` lint rejects the longer form.
- `hand_written_requests_parse` reads every `*.request.json` in name order: each parses, has at least one view, and every view's turn equals the request's. It expects exactly seven files (six when regenerating, when it skips the exactness file another test is writing).

### Regenerating

When the sample setting, the protocol or the rule changes, regenerate the two machine-written files under one environment variable, `POLICYSPACE_UPDATE_RECORDINGS`. With it set, the engine's tests rewrite the request half of the sample recording (keeping the reply they find) and the exactness request, and the Python sample test rewrites the reply half. Without it every test only checks. From the project root:

```
set POLICYSPACE_UPDATE_RECORDINGS=1      (PowerShell: $env:POLICYSPACE_UPDATE_RECORDINGS = "1")
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # rewrites the engine's halves
.venv/Scripts/python -m pytest python/tests                              # rewrites the rule's reply
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # confirms
```

Then unset the variable. The `cargo test` runs single-threaded so that the tests that write files don't run at the same time as each other. After the first step the reply check may fail (the reply is stale or empty): that is the signal to run the Python step. Hand-written requests are edited by hand and never regenerated.

## Extending

- **A new field in the view**: add it to `BusinessView` (or a part), fill it in `view::build` from the World's public accessors, and keep it scoped to the business. Use names or factory numbers, never `Id` values. A `BTreeMap` field needs the `no_duplicate_keys` attribute, and any value a parse can't check goes into `BusinessView::problems`.
- **A new verb in the decision**: add a field with `#[serde(default)]` so existing deciders' JSON still reads, add its number checks to `number_problems` and its view checks to `validate` with a new `Place` variant if it has its own place, and resolve it in the World: `World::business_submission` for a verb that trades, `World::operate_business` for one that reaches the factories, or a new step of the turn (see [trade.md](trade.md)). Asking for more than the business has stays valid, and the resolution trims it and records asked beside done.
- **A new message**: derive `Debug, Clone, PartialEq, Serialize, Deserialize` with `deny_unknown_fields`, give it `to_json`, `to_json_pretty` and `from_json` through the `mod.rs` helpers, and prefix its parts' value problems with where they come from.
- **Any change to the format** has a second implementation to update: `python/policyspace/protocol.py` (the dataclass, its reader and its writer, with the field in the same key position; see [python.md](python.md#extending)). Then regenerate the recordings (see [Regenerating](#regenerating)); the round-trip and fixture tests fail until both sides agree.

## Not yet

The trace is kept in the business's stored answer, not in its outcome; the dev UI shows it, with the decision and its outcome, in each business's section (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). Verbs beyond the two fractions, offers and bids (budgets, construction, retooling, trading assets) are later Stages. Per-deposit IDs and structured traces are not planned yet. Running the exchange is described in [decider-host.md](decider-host.md), and resolving decisions in [trade.md](trade.md).

## Tests

- `quantity.rs` and `turn.rs` unit tests, through `serde_json` directly: quantities `0`, `0.1`, `1/3`, `1e-300`, `5e-324`, `f64::MAX` and `123456789.123456789` round trip exactly; a JSON integer reads as a quantity; `-1`, `1e999` and `"1"` are refused with a message and `-0.0` reads as zero; turns `0`, `7` and `u64::MAX` round trip as bare integers; `3.0`, `-1` and `"3"` are refused.
- `decider/view.rs` unit tests on a hand-built view: round trips through both writers; keys are names and quoted factory numbers, `perishes` is written as `null` or a number, prices are alphabetical, and no `Id<` text appears; extreme values round trip exactly; reading refuses a bad condition (`Values`), a negative cash, an unknown key, a key written twice, a non-numeric factory key and malformed JSON (`Json`).
- `decider/decision.rs` unit tests: empty and full decisions are valid; asking for too much is valid; one decision with every kind of problem reports them all, at their places, in order; a decision for another business; round trips; the empty decision's exact JSON; a hand-written decision; malformed JSON, wrong types, unknown keys, duplicate keys, bad fractions and amounts each refused at the right layer; the error displays.
- `decider/batch.rs` unit tests: a subset in any order is accepted and the rest get `Nothing`; an empty reply; each reply-level refusal alone and all three together; an invalid decision refused without refusing the reply, with the refused decision carried in the `Answer`; round trips including `Turn::MAX`; a reply with no `decisions` key; value problems prefixed with the business in requests, replies and recordings.
- `error_list.rs` unit tests: `ErrorList::new` panics on an empty list; `Display` joins with newlines and no trailing newline; `iter`, `len` and `into_iter` agree.
- The decider's unit tests share two `#[cfg(test)] pub(crate)` helpers in the module itself: `view::test_view()` (a hand-built view of business `a` at turn 3 with currency `silver`, prices for labor and grain, two `farm` factories and one deposit type) and `decision::test_goods()` (goods `labor`, `grain`, `copper`, `silver`).
- `crates/engine/tests/decider.rs`, through the public API only, using `common`'s `world_of`, `sample_setting`, `test_setting` and `q`, plus local helpers (`business`, `view`, `groups_of`, `qmap`, `table`, `order`, `factory_rows`, `deposit_rows`, `full_decision`):
  - the sample's three center businesses see their own prices, perish groups, cash, factories, technologies and deposits at turn 0, and the farm's emission perishes at turn 0's end and is re-emitted for turn 1; their views are built from the world after `World::new` (idle host), so each factory's condition is one slice below the setting's (farm 0.875, mine 0.625, toolmaker 23/24; `production_valid`'s `works` 0.75, 0.25 and 2/3), compared to within 1e-9;
  - `businesses_valid`'s `idle` (no market) sees no prices and nothing else; `farm_a` and `farm_b` see their own groups and deposits;
  - `production_valid`'s `works` sees two technologies for three factories;
  - building every view twice changes no stockpile or condition and gives equal views;
  - decisions validate against the farm's view, including over-asking, with the out-of-view factory, currency and unpriced-good refusals;
  - every sample view, decision, request, reply and recording round trips through both writers, and a hand-written decision reads and validates;
  - a reply is checked against its request (subset in another order, wrong turn, unknown business, duplicate business).
- `crates/engine/tests/recordings.rs` checks the committed recordings against the engine: the sample recording's request and canonical form, and its reply's validity; the exactness request; and that every hand-written request parses (see [Recordings](#recordings)). The Python tests read the same files (see [python.md](python.md#tests)).
- The host, the decision point and the three deciders have their own tests, listed in [decider-host.md](decider-host.md#tests).

No test setting was added for the protocol: the tests reuse the sample, `businesses_valid` and `production_valid` unchanged.
