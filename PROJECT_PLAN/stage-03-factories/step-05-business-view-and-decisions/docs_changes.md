# Step 05: business-view-and-decisions — Docs Changes

## docs/decider.md — new file
- **Location:** whole file
- **Change:** add

# The decider protocol

Source: `crates/engine/src/decider/` — `mod.rs` (module docs, re-exports, `FormatError` and the shared JSON helpers), `view.rs` (`BusinessView` and how it is built from a World), `decision.rs` (`Decision`, its validation and errors) and `batch.rs` (`Request`, `Reply`, `Recording`, reply validation). Also the JSON forms of `Quantity` (`quantity.rs`) and `Turn` (`turn.rs`), and `World::business_view` (`world.rs`). The crate root re-exports `Answer`, `BusinessView`, `Decision`, `DecisionErrors`, `FormatError`, `Recording`, `Reply`, `ReplyErrors` and `Request`. The rest is public under `policyspace_engine::decider` (`FactoryOrder`, `Place`, `DecisionError`, `ReplyError`, `PerishGroup`, `FactoryView`, `TechnologyView`, `DepositsView`), and the submodules are public as `decider::view`, `decider::decision` and `decider::batch`.

A **decider** is whatever chooses what a business does each turn: a script in another process, a replayed recording, or a test. This module defines, as plain data, what the engine and a decider say to each other:

- the **view** a business is served each turn: the data it may know, and nothing more;
- the **decision** it answers with: the kinds of action it may take, with the bounds the engine enforces, and an optional trace;
- **validation** of a decision against the view it answers;
- the **batch** of one turn's exchange (every view out, every decision back) and the **recording** that keeps both;
- **lossless JSON** for all of it, keyed by setting names and factory ID numbers, never by engine IDs.

**Nothing runs a decider or touches the turn yet.** `World::business_view` builds a view on demand, and everything else is types and functions with their own tests. Step 06 of Stage 03 writes the first Python script against this format, Step 07 runs the exchange (building the request, transport, what a refused decision or a missing reply means, replaying recordings), Step 08 resolves valid decisions (trading, running and maintaining factories, trimming over-asking), and Step 09 shows decisions and traces in the dev UI.

## Concepts

- **Where the view is taken.** The decision point is the start of the "actors act" part of the turn, before any trading (see [world.md](world.md#the-turn)). So a business's stockpile then holds this turn's emission from its deposits (unimproved goods that perish at the end of this turn), the output its factories made last turn, and the currency it carried over. Building a view reads the world and changes nothing.
- **Scoped to the business.** A view holds the business's hex's currency and posted prices, its own stockpile, cash, factories, their technologies and its deposits. Nothing about other businesses, POPs, other hexes, hex stockpiles or totals is in it, so a decider can't act on anything it couldn't know.
- **Names, not IDs.** Goods, technologies, resource types and businesses go by their setting names. Factories have no names, so they go by their ID number (`FactoryId::index()` as a `u32`), which is the factory's position in load order and stable for a given setting. No `Id<K>` value ever appears in a message (see [ids.md](ids.md)), and nothing in the module holds a reference into the World.
- **Tables, not lists, wherever a key is unique.** Prices, stockpile goods, technologies, deposits, bundles, a view's factories, and a decision's factories, offers and bids are all `BTreeMap`s: by good, technology or resource-type name, or by factory number. A table can't hold one key twice, so a decision can't list a factory or a good twice; a Python decider that builds dicts gets this for free. Keys come out in sorted order (alphabetical, or numeric for factories), which keeps the JSON deterministic.
- **Asking for too much is not an error.** Offering more of a good than the stockpile holds, bidding for more than cash can pay, or offering and bidding the same good is a valid request. Step 08 trims it when it resolves it and records what was asked beside what was done, as the factory mechanics do for a short input (see [production.md](production.md#a-safety-net-not-the-normal-path)). Validation refuses only nonsense.
- **Doing nothing is legitimate.** A factory a decision doesn't mention is left alone (run 0, maintenance 0); a fraction left out is 0; a business with no decision in a reply does nothing that turn. `Decision::empty(name)` is the explicit "do nothing".

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
- The **trace** is free text, any number of lines: the decider's account of its choice. The engine stores it for the dev UI and never acts on it.

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
- `DecisionErrors` is never empty; it has `iter`, `len`, `is_empty` (always false) and `IntoIterator for &DecisionErrors`, and displays one error per line with no trailing newline. Both implement `std::error::Error`.

What happens to a refused decision (the business doing nothing that turn, or the run stopping) is Step 07's.

## The batch

```rust
pub struct Request { pub turn: Turn, pub views: Vec<BusinessView> }       // the engine's message
pub struct Reply { pub turn: Turn, pub decisions: Vec<Decision> }        // the deciders' answer; absent decisions = empty
pub struct Recording { pub request: Request, pub reply: Reply }          // one turn, saved
pub enum Answer<'a> { Nothing, Valid(&'a Decision), Refused(DecisionErrors) }
```

The turn has one decision point, where every business's view is served and every decision collected in one exchange. A reply may hold decisions for any subset of the request's businesses, in any order, because a decider may control any set of businesses and Step 07 may split one turn among several deciders.

`Reply::validate(&self, request, goods) -> Result<Vec<Answer>, ReplyErrors>` first makes the reply-level checks and, if any fail, refuses the whole reply with every problem found, in order (the turn first, then the decisions in reply order):

- `TurnMismatch { request, reply }` — `the reply answers turn 1, but the request is for turn 0`;
- `UnknownBusiness { name }` — `a decision for 'nobody', which isn't in the request`;
- `DuplicateBusiness { name }` — `two decisions for 'center_mine'`.

`ReplyErrors` has the same shape and impls as `DecisionErrors`. Otherwise it returns one `Answer` per view, in the request's order: `Nothing` for a business with no decision, `Valid(&decision)` or `Refused(errors)` from `Decision::validate` against that business's view. A refused decision doesn't refuse the reply.

A **recording** is a request and its reply saved together. Step 06's Python tests read recordings to drive the script, and Step 07's replay decider answers from them, so engine tests that don't test strategy need no Python.

Nothing checks that a request is consistent in itself (each view's turn against the request's, or two views of one business). A request the engine builds can't have either problem; one read from a recording file can, and Step 07 has to check it or trust only engine-built requests.

## JSON

Every message type has `to_json()` (compact, one line), `to_json_pretty()` (indented, for recordings people read and diff) and `from_json(text) -> Result<Self, FormatError>`. JSON was chosen because Python reads and writes it with nothing installed, every language has it, it is readable by eye, and a saved turn can be diffed.

### Format

Field order in the structs is the key order in the JSON. The farm's view in a fresh sample world, as `to_json_pretty` writes it:

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

## Extending

- **A new field in the view**: add it to `BusinessView` (or a part), fill it in `view::build` from the World's public accessors, and keep it scoped to the business. Use names or factory numbers, never `Id` values. A `BTreeMap` field needs the `no_duplicate_keys` attribute, and any value a parse can't check goes into `BusinessView::problems`.
- **A new verb in the decision**: add a field with `#[serde(default)]` so existing deciders' JSON still reads, add its number checks to `number_problems` and its view checks to `validate` with a new `Place` variant if it has its own place, and resolve it in Step 08's resolution code. Asking for more than the business has stays valid.
- **A new message**: derive `Debug, Clone, PartialEq, Serialize, Deserialize` with `deny_unknown_fields`, give it `to_json`, `to_json_pretty` and `from_json` through the `mod.rs` helpers, and prefix its parts' value problems with where they come from.

## Not yet

Building a request from every business, the decision point in the turn, transport between processes, resolving decider names, what a refused decision or a missing reply means, checking requests read from recordings, and the replay decider are Step 07 of Stage 03. The first Python decider is Step 06. Resolving decisions (trading, running and maintaining factories, trimming over-asking, the cash cap, keeping the trace in the business's record) is Step 08, and showing decisions and traces in the dev UI is Step 09. Verbs beyond the two fractions, offers and bids (budgets, construction, retooling, trading assets) are later Stages. Per-deposit IDs and structured traces are not planned yet.

## Tests

- `quantity.rs` and `turn.rs` unit tests, through `serde_json` directly: quantities `0`, `0.1`, `1/3`, `1e-300`, `5e-324`, `f64::MAX` and `123456789.123456789` round trip exactly; a JSON integer reads as a quantity; `-1`, `1e999` and `"1"` are refused with a message and `-0.0` reads as zero; turns `0`, `7` and `u64::MAX` round trip as bare integers; `3.0`, `-1` and `"3"` are refused.
- `decider/view.rs` unit tests on a hand-built view: round trips through both writers; keys are names and quoted factory numbers, `perishes` is written as `null` or a number, prices are alphabetical, and no `Id<` text appears; extreme values round trip exactly; reading refuses a bad condition (`Values`), a negative cash, an unknown key, a key written twice, a non-numeric factory key and malformed JSON (`Json`).
- `decider/decision.rs` unit tests: empty and full decisions are valid; asking for too much is valid; one decision with every kind of problem reports them all, at their places, in order; a decision for another business; round trips; the empty decision's exact JSON; a hand-written decision; malformed JSON, wrong types, unknown keys, duplicate keys, bad fractions and amounts each refused at the right layer; the error displays.
- `decider/batch.rs` unit tests: a subset in any order is accepted and the rest get `Nothing`; an empty reply; each reply-level refusal alone and all three together; an invalid decision refused without refusing the reply; round trips including `Turn::MAX`; a reply with no `decisions` key; value problems prefixed with the business in requests, replies and recordings.
- The decider's unit tests share two `#[cfg(test)] pub(crate)` helpers in the module itself: `view::test_view()` (a hand-built view of business `a` at turn 3 with currency `silver`, prices for labor and grain, two `farm` factories and one deposit type) and `decision::test_goods()` (goods `labor`, `grain`, `copper`, `silver`).
- `crates/engine/tests/decider.rs`, through the public API only, using `common`'s `world_of`, `sample_setting`, `test_setting` and `q`, plus local helpers (`business`, `view`, `groups_of`, `qmap`, `table`, `order`, `factory_rows`, `deposit_rows`, `full_decision`):
  - the sample's three center businesses see their own prices, perish groups, cash, factories, technologies and deposits at turn 0, and the farm's emission perishes at turn 0's end and is re-emitted for turn 1;
  - `businesses_valid`'s `idle` (no market) sees no prices and nothing else; `farm_a` and `farm_b` see their own groups and deposits;
  - `production_valid`'s `works` sees two technologies for three factories;
  - building every view twice changes no stockpile or condition and gives equal views;
  - decisions validate against the farm's view, including over-asking, with the out-of-view factory, currency and unpriced-good refusals;
  - every sample view, decision, request, reply and recording round trips through both writers, and a hand-written decision reads and validates;
  - a reply is checked against its request (subset in another order, wrong turn, unknown business, duplicate business).

No test setting was added: the tests reuse the sample, `businesses_valid` and `production_valid` unchanged.

## docs/README.md — modify
- **Location:** the document table, new row after `production.md` (the row Step 04 adds)
- **Change:** add

| [decider.md](decider.md) | The decider protocol as data: the `BusinessView` a business is served (built by `World::business_view`), the `Decision` it answers with (run and maintenance fractions per factory, offers, bids, a trace), validation against the view and its errors, the `Request`/`Reply`/`Recording` batch and reply validation, and the lossless JSON format (name-keyed tables, quoted factory numbers, `Quantity` and `Turn` forms, two-layer reading, duplicate keys) |

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries
- **Change:** add one line after `src/currency.rs`, before `src/demand.rs`

```
    src/decider/        the decider protocol: view, decision, batch, JSON (see decider.md)
```

## docs/workspace.md — modify
- **Location:** the repository layout block, engine `src/` entries, the `src/quantity.rs` and `src/turn.rs` lines
- **Change:** replace

```
    src/quantity.rs     the Quantity type and its JSON form
```
and
```
    src/turn.rs         the Turn number and its JSON form
```

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same entry

```
    tests/businesses.rs, currency.rs, decider.rs, demand.rs, factories.rs, goods.rs,
          growth.rs, map.rs, market.rs, pops.rs, production.rs, resources.rs,
          setting_loader.rs, technologies.rs, trade.rs, world.rs
```

## docs/workspace.md — modify
- **Location:** "Crates and the dependency rule", the bullet "The engine's only external dependency is `toml` ..."
- **Change:** replace

- The engine's external dependencies are `toml` (parser only, no serde; see [setting-loader.md](setting-loader.md)), and `serde` (with `derive`) and `serde_json` for the decider protocol's JSON (see [decider.md](decider.md#json)), plus the dev-dependency `tempfile`. `serde_json` must keep its `float_roundtrip` feature: without it floats don't read back exactly, which breaks the protocol's lossless rule. `serde_json` is used only in `crates/engine/src/decider/mod.rs`. Add a dependency only when a Step needs it.

## docs/workspace.md — modify
- **Location:** "Tests", the bullet about `crates/engine/src/test_support.rs`, at the end
- **Change:** add

The decider module keeps its own shared unit-test helpers, `decider::view::test_view()` and `decider::decision::test_goods()` (both `#[cfg(test)] pub(crate)`), because they build protocol values rather than engine state (see [decider.md](decider.md#tests)).

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, new sentence after the accessor sentence (the one Step 03 replaces, ending "... and `factories()`. `factories()` returns the `Factories`, ...")
- **Change:** add

`business_view(business)` builds a business's view of its circumstances at the current turn, the data a decider is served (see [decider.md](decider.md#the-view)). It reads and changes nothing, and panics for a business from another world.

## docs/world.md — modify
- **Location:** "Turn", at the end of the paragraph
- **Change:** add

It derives serde's `Serialize` and `Deserialize` as a newtype, so in JSON a turn is a bare whole number; a fraction, a negative number or a string is refused (see [decider.md](decider.md#the-lossless-rule)).

## docs/world.md — modify
- **Location:** "Quantity", at the end of the paragraph
- **Change:** add

`quantity.rs` also defines the JSON form, in hand-written `Serialize` and `Deserialize` impls: a quantity is written as a plain number, the shortest text that reads back to the same `f64`, and read through `Quantity::new`, so a negative or non-finite value is refused. This is the one place the written form is defined (see [decider.md](decider.md#the-lossless-rule)).

## docs/world.md — modify
- **Location:** "Tests", first bullet, the `turn.rs` and `quantity.rs` items
- **Change:** replace `` `turn.rs`, `quantity.rs` (including `checked_sub`, `saturating_mul` and `checked_div`) `` with

`turn.rs` (including its JSON form), `quantity.rs` (including `checked_sub`, `saturating_mul`, `checked_div` and the exact JSON round trip)

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same section

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, businesses acting in the "actors act" part of the turn (they exist and hold goods, and `business_view` builds the view a decider would be served, see [decider.md](decider.md), but nothing serves views or collects decisions until Step 07 of Stage 03, and nothing resolves them until Step 08), and factories running, wearing or being maintained in the turn (the mechanics exist as pure functions, see [production.md](production.md), and Step 08 of Stage 03 calls them; until then factories are held as state, unchanged).

## docs/ids.md — modify
- **Location:** "Rules", the bullet "IDs are meaningful only inside the registry or list (the world) that created them. They must never be persisted: ...", at the end
- **Change:** add

The decider protocol follows this rule: its messages carry setting names, and factories, which have no names, by their ID number (`FactoryId::index()`, their load order), never `Id<K>` values (see [decider.md](decider.md#concepts)).

## docs/ids.md — modify
- **Location:** "Rules", the bullet "Text names appear only at the edges: reading settings, error messages, and UIs."
- **Change:** replace

- Text names appear only at the edges: reading settings, error messages, UIs, and the decider protocol's messages (see [decider.md](decider.md)).

## docs/setting-loader.md — modify
- **Location:** "EntryReader", the first paragraph, after the sentence "It is hand-written rather than serde-based because serde stops at the first error, and the loader must report all of them."
- **Change:** add

The decider protocol's JSON does use serde, with a second pass that collects every value problem (see [decider.md](decider.md#reading-in-two-layers)); the loader stays hand-written.

## docs/production.md — modify
- **Location:** "The model", "Two instructions per factory", the sentence "The decision validation in Step 05 makes sure they arrive in range." (in the file Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same sentence in its new file

A decision's fractions are checked to be finite and from 0 to 1 when it is read from JSON and again when it is validated against its view (see [decider.md](decider.md#validation)), so they arrive in range.

## docs/production.md — modify
- **Location:** "Not yet" (in the file Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the section in its new file

Calling the mechanics in the turn, keeping the per-business records in the World, and anything in the turn report are Step 08. The fractions come from a business's `Decision` (see [decider.md](decider.md)), whose format and validation exist; the first decider is Step 06 (its rule never asks for more than it can cover) and collecting decisions in the turn is Step 07. Showing the records in the dev UI is Step 09. Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## docs/businesses.md — modify
- **Location:** introduction, the second paragraph ("Nothing decides or trades for a business yet. ...")
- **Change:** replace
- **Supersedes:** Step 03 (factories), its replacement of the same paragraph

Nothing decides or trades for a business yet. Factories (see [factories.md](factories.md)) name a business as their owner, and must sit on its hex. The view a business would be served and the decision it would answer with exist as data, with validation and JSON (see [decider.md](decider.md)); the first decider (Step 06) and the decider host (Step 07) will choose what each business does, and business trade (Step 08) will trade from and into the business stockpile. Until then a business only holds its inventory, receives the output of its deposits, and owns factories that don't run yet.

## docs/businesses.md — modify
- **Location:** "Using businesses", the code block, inside the `for` loop after the `factories` line (which Step 03 adds)
- **Change:** add

```rust
    let view = world.business_view(id);                  // what its decider is served (see decider.md)
```

## docs/businesses.md — modify
- **Location:** "Not yet"
- **Change:** replace its first sentence
- **Supersedes:** Step 04 (factory-mechanics), the same sentence

The factory mechanics a business's factories will follow exist as pure functions (see [production.md](production.md)), and its view and decisions exist as data (see [decider.md](decider.md)). The following are later Steps of Stage 03: the first decider (Step 06), running the exchange and resolving decider names (Step 07), business trade, running its factories in the turn and the use of its cash (Step 08), and showing businesses, their factories and their decisions in the dev UI (Step 09).

## docs/factories.md — modify
- **Location:** introduction, the second paragraph
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), its replacement of the same paragraph

The mechanics of running, wear and maintenance exist as pure functions over one business's factories (see [production.md](production.md)), but nothing in the turn calls them yet. Each business's view lists its factories by ID number with their condition and technology, and a decision addresses them by the same number (see [decider.md](decider.md)). Step 08 wires the mechanics into the turn, and Step 09 shows factories in the dev UI. Until then, factories are loaded, checked and held in the world, unchanged.

## docs/factories.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same section

Running a factory, depreciation, maintenance and shutting down exist as pure functions (see [production.md](production.md)) but aren't called in the turn, and the view and decision that carry a factory's condition and orders exist as data (see [decider.md](decider.md)). The following are later Steps of Stage 03: the mechanics in the turn (Step 08) and showing factories in the dev UI (Step 09). Factories under construction, building, scrapping, retooling, and buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes. Nothing changes a factory during play yet.

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the factory mechanics of running, depreciation and maintenance exist as pure functions (see [production.md](production.md)). A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)). The first decider (Step 06), the decider host (Step 07), business trade with factories running in the turn (Step 08), and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.
