# Step 05: business-view-and-decisions — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-05-business-view-and-decisions/design.md` — the behavior this spec implements: what a view holds, the action vocabulary, every validation rule and its reason, the lossless rule, the batch and the recording.
- `PROJECT_PLAN/stage-03-factories/step-04-factory-mechanics/spec.md` — the previous Step's conventions (a pure module with its own tests, nothing wired into the turn), and the meaning of the run and maintenance fractions the decision carries.
- `docs/world.md` — the World, the three-part turn, business stockpiles and perish groups. `docs/market.md` — `HexPrices`, posted prices, the currency rule. `docs/ids.md` — `Id`, `NamedList`, why IDs are never persisted (the protocol uses names and ID numbers, never `Id` values). `docs/setting-loader.md` — the loader's error style (`LoadError`, `LoadErrors`: every problem, each at its place, in a fixed order), which the decision errors copy. `docs/workspace.md` — the check command, test layout, the shared test helpers.
- Code: `crates/engine/src/world.rs` (the accessors a view is built from: `turn`, `rules`, `businesses`, `factories`, `deposits`, `hex_currency`, `hex_prices`, `business_stockpile`; `World::market` as the model for a method that builds something from the world), `crates/engine/src/businesses.rs`, `crates/engine/src/factories.rs` (`owned_by`), `crates/engine/src/resources.rs` (`Deposits::owned_by`, `ResourceType`), `crates/engine/src/technologies.rs` (`Technology` accessors), `crates/engine/src/bundle.rs`, `crates/engine/src/stockpile.rs` (`iter`, `groups`, `total`, `PerishesAt`), `crates/engine/src/market.rs` (`HexPrices::posted`), `crates/engine/src/quantity.rs`, `crates/engine/src/turn.rs`, `crates/engine/src/goods.rs`, `crates/engine/src/id.rs` (`NamedList::id`, `name`), `crates/engine/src/setting/error.rs` (the error shapes to copy), `crates/engine/src/production.rs` (the fraction checks' wording), `crates/engine/src/test_support.rs`, `crates/engine/src/lib.rs`, `crates/engine/Cargo.toml`, `Cargo.toml`.
- Tests: `crates/engine/tests/common/mod.rs`, `crates/engine/tests/businesses.rs` (`business_id`, `business_totals`: how the sample's businesses are reached), `crates/engine/tests/factories.rs`, `crates/engine/tests/production.rs`, `crates/engine/tests/settings/README.md` (the `businesses_valid` and `production_valid` entries: the test settings this Step reuses, unchanged).
- `settings/sample/` (every file): the expected sample views below are computed from it.

## Approach

- **A new module folder `crates/engine/src/decider/`** with `mod.rs` (docs, re-exports, the JSON helpers and `FormatError`), `view.rs` (the view and how it is built from a World), `decision.rs` (the decision, its validation and errors) and `batch.rs` (request, reply, recording, reply validation). Rationale: the design's five parts are three groups of types; Step 07 adds the Decider trait, the external-process decider and the replay decider to the same folder.
- **JSON through `serde` (with `derive`) and `serde_json`**, two new dependencies of the engine crate. Every protocol type derives `Serialize` and `Deserialize`, with `#[serde(deny_unknown_fields)]` so a misspelled key is refused, as the setting loader refuses unknown keys. Rationale: the design chose JSON and the standard library for it; the derive keeps the format and the types in one place. `serde_json` isn't in `Cargo.lock` yet, so the first build fetches it (`cargo build` needs network access once).
- **Lossless numbers, defined where the types are.** `Quantity` gets hand-written `Serialize`/`Deserialize` in `quantity.rs`: written as a JSON number (serde_json writes the shortest text that reads back to the same `f64`, and with its `float_roundtrip` feature parses it back correctly rounded), read through `Quantity::new`, which refuses negative and non-finite values. `Turn` derives both in `turn.rs`: a newtype over `u64`, so it is a JSON integer; serde refuses a fraction or a negative number for a `u64`. Fractions (run, maintenance, condition) are `f64` fields checked after parsing (see "two layers" below). Rationale: the design's rule that the quantity type is the one place the written form lives; `u64` covers the largest turn exactly in JSON (Python reads it exactly too).
- **Name-keyed tables everywhere a key is unique**, as `BTreeMap`s: prices, stockpile goods, technologies, deposits, bundles, and (decided with the user, now in `design.md`) a decision's factories, offers and bids, and a view's factories. Factories are keyed by their ID number (`u32`); serde_json writes integer keys as quoted strings (`"0"`) and reads them back. A key written twice in the JSON is refused by a shared `deserialize_with` helper, since `BTreeMap` would otherwise keep the last value silently. Rationale: Python builds dicts, a table can't hold a duplicate, and alphabetical (or numeric) order is deterministic.
- **A hex with no market gives an empty `prices` table**, the same as a market with no prices. Rationale: to a decider the two are the same (nothing is tradable); one shape is simpler.
- **Decision numbers are checked in two layers.** `Decision::from_json` parses, then runs the same number checks validation runs (`number_problems`), so a parsed decision is well-formed as the design promises. `Decision::validate` runs them again together with the view checks, so a decision built in Rust (by a test now, by the dev UI in Stage 05) is checked too. Rationale: the design puts the number rules under Validation and also promises well-formed reads; both hold, and the check is cheap.
- **Validation takes the view and the goods list.** `Decision::validate(&self, view: &BusinessView, goods: &Goods)`: the view alone can't tell an unknown good from a good this hex doesn't price, and the design reports those differently. Step 07 has the World's `rules().goods`.
- **Reply validation returns one answer per business in the request**: `Nothing`, `Valid(&Decision)` or `Refused(DecisionErrors)`, after the reply-level checks (turn, unknown business, duplicate business), which refuse the whole reply and report every problem found. Rationale: Step 07 needs exactly this to resolve a turn; what it does with `Refused` is its failure semantics.
- **Nothing in the turn changes.** `World` gains one read-only method, `business_view`. No setting, report, or dev UI text changes.
- **The build runs as one.** Validation needs the view types, the batch needs both, the JSON helpers are shared, and the tests cross all three files; no package could be built and tested alone.

## Changes

### New

- `crates/engine/src/decider/mod.rs` — module docs, `pub mod view; pub mod decision; pub mod batch;`, re-exports, `FormatError`, the crate-private JSON helpers and the duplicate-key helper.
- `crates/engine/src/decider/view.rs` — `BusinessView` and its parts, `build` (from a World), JSON methods, value checks, unit tests.
- `crates/engine/src/decider/decision.rs` — `Decision`, `FactoryOrder`, `Place`, `DecisionError`, `DecisionErrors`, validation, JSON methods, unit tests.
- `crates/engine/src/decider/batch.rs` — `Request`, `Reply`, `Recording`, `ReplyError`, `ReplyErrors`, `Answer`, reply validation, JSON methods, unit tests.
- `crates/engine/tests/decider.rs` — integration tests (Test Plan).

### Modified

- `crates/engine/Cargo.toml` — `[dependencies]` gains `serde = { version = "1", features = ["derive"] }` and `serde_json = { version = "1", features = ["float_roundtrip"] }`. The `float_roundtrip` feature is required: without it serde_json's float parser is fast but not correctly rounded, and a value such as `123456789.123456789` reads back one unit in the last place off, breaking the lossless rule. `Cargo.lock` updates when cargo resolves them.
- `crates/engine/src/quantity.rs` — `impl Serialize for Quantity` and `impl<'de> Deserialize<'de> for Quantity`; module docs gain a sentence: a quantity is written to JSON as a plain number and read back through `new`, so the written form is defined here and nowhere else; unit tests.
- `crates/engine/src/turn.rs` — `#[derive(Serialize, Deserialize)]` on `Turn`; module docs: a turn is written to JSON as a whole number; unit tests.
- `crates/engine/src/world.rs` — `pub fn business_view(&self, business: BusinessId) -> BusinessView` (delegates to `decider::view::build`); module docs gain one bullet: a business's view of its circumstances at the current turn is taken with `business_view`, which reads and changes nothing (the decision point that serves it is Step 07's).
- `crates/engine/src/lib.rs` — `pub mod decider;` and `pub use decider::{Answer, BusinessView, Decision, DecisionErrors, FormatError, Recording, Reply, ReplyErrors, Request};` after `currency`, before `demand`. Crate docs: after the production sentence, add that the [`decider`] module defines the decider protocol as data: the view a business is served each turn (its hex's currency and posted prices, its stockpile by perish group, its cash, its factories, their technologies and its deposits, and nothing outside its circumstances), the decision it answers with (a run and a maintenance fraction per factory, offers, bids and an optional trace), the validation of a decision against its view, the request and reply that carry one turn's batch, and lossless JSON for all of it; no decider runs yet (Steps 06 and 07).

Nothing else changes: not the turn, `TurnReport`, `Stockpile`, `Market`, the loader, any setting (sample or test), the dev UI, `docs/`, or any existing test.

## Interfaces & Data

### `crates/engine/src/quantity.rs`

```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

impl Serialize for Quantity {
    // serializer.serialize_f64(self.0)
}

impl<'de> Deserialize<'de> for Quantity {
    // let value = f64::deserialize(deserializer)?;
    // Quantity::new(value).ok_or_else(|| de::Error::custom(format!(
    //     "a quantity must be a finite number of at least 0, not {value}")))
}
```

`f64::deserialize` accepts a JSON integer (`100`) as well as `100.0`. JSON has no NaN or infinity literals, so a non-finite value can only arrive as an overflowing literal such as `1e999`; serde_json refuses that as out of range before it reaches `new`. Either way the read fails with a message.

### `crates/engine/src/turn.rs`

`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)] pub struct Turn(u64);` — a serde newtype, written as the bare integer. `3.0`, `-1` and `"3"` are refused by serde's `u64` visitor with its own message (`invalid type: floating point ...`, and so on).

### `crates/engine/src/decider/mod.rs`

```rust
//! The decider protocol as data: what a business is told each turn, what it may answer,
//! how an answer is checked against what it was told, and how both travel as JSON.
//!
//! State the design: the view is taken at the start of "actors act", before any trading, so
//! the stockpile holds this turn's emission, last turn's output and the carried-over
//! currency; it holds only what concerns the business (its hex's currency and posted prices,
//! its stockpile by perish group, its cash, its factories by ID, their technologies once
//! each, its deposits per resource type), never other actors or hexes. A decision names its
//! business and carries, per factory, a run and a maintenance fraction (0 to 1; an
//! unmentioned factory is left alone), offers and bids per good, and an optional free-text
//! trace the engine stores and never acts on. Validation refuses nonsense (a factory not in
//! the view, an unknown good, a bad fraction or amount, the currency, an unpriced good) and
//! reports every problem at its place; asking for more than the business holds or can pay
//! is a valid request that Step 08 trims. One turn's exchange is a `Request` (every view)
//! and a `Reply` (any subset of decisions, in any order); a `Recording` keeps both. Every
//! type reads and writes JSON losslessly, keyed by setting names and factory ID numbers,
//! never by engine IDs. Nothing here runs a decider or touches the turn: Step 06 writes the
//! first script against this format, Step 07 runs the exchange, Step 08 resolves decisions.

pub mod batch;
pub mod decision;
pub mod view;

pub use batch::{Answer, Recording, Reply, ReplyError, ReplyErrors, Request};
pub use decision::{Decision, DecisionError, DecisionErrors, FactoryOrder, Place};
pub use view::{BusinessView, DepositsView, FactoryView, PerishGroup, TechnologyView};

/// Why a message couldn't be read from JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The text isn't the JSON the format expects: a syntax error, a wrong type, a missing
    /// or unknown key, a key written twice, a number out of its type's range. The parser's
    /// message, which ends with the line and column.
    Json(String),
    /// The JSON parsed, but holds values no message may: each problem at its place, in
    /// document order (`"factory #0: run must be ..."`, or in a batch
    /// `"center_farm: factory #0: run must be ..."`).
    Values(Vec<String>),
}
// Display: `Json(m)` writes `m`; `Values(v)` writes the lines joined by '\n' (no trailing
// newline, like `LoadErrors`). `impl std::error::Error for FormatError {}`.

/// Serializes `value` as compact JSON (one line).
pub(crate) fn to_json<T: Serialize>(value: &T) -> String;
/// Serializes `value` as indented JSON, for recordings people read and diff.
pub(crate) fn to_json_pretty<T: Serialize>(value: &T) -> String;
/// Parses `text`, mapping a parser error to `FormatError::Json(error.to_string())`.
pub(crate) fn from_json<T: DeserializeOwned>(text: &str) -> Result<T, FormatError>;
/// Turns a non-empty list of value problems into `Err(FormatError::Values(problems))`,
/// and an empty one into `Ok(())`.
pub(crate) fn checked(problems: Vec<String>) -> Result<(), FormatError>;

/// A `#[serde(deserialize_with = "no_duplicate_keys")]` helper for every `BTreeMap` field:
/// a `MapAccess` visitor that inserts each entry and fails with
/// `de::Error::custom(format!("the key '{key}' is written twice"))` when a key repeats.
/// Generic over `K: Deserialize + Ord + Display` and `V: Deserialize`.
pub(crate) fn no_duplicate_keys<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>;

/// True for a finite number from 0 to 1 (a run, maintenance or condition fraction).
pub(crate) fn is_fraction(value: f64) -> bool;
/// True for a finite number of at least 0 (an amount; the same rule as `Quantity::new`).
pub(crate) fn is_amount(value: f64) -> bool;
```

`to_json`/`to_json_pretty` use `serde_json::to_string(_pretty)` and `expect("the protocol's types always serialize")` (they hold only strings, finite numbers and maps). The three `pub(crate)` helpers keep `serde_json` out of the public API, so the library could be swapped in one file.

Every public struct below derives `Debug, Clone, PartialEq, Serialize, Deserialize` and carries `#[serde(deny_unknown_fields)]`. Every `BTreeMap` field carries `#[serde(deserialize_with = "crate::decider::no_duplicate_keys")]` (and `#[serde(default)]` where noted). Field order below is the JSON key order.

### `crates/engine/src/decider/view.rs`

```rust
/// What one business is told at the decision point: its circumstances and nothing else.
pub struct BusinessView {
    /// The current turn.
    pub turn: Turn,
    /// The business's name.
    pub business: String,
    /// The name of its hex's currency good.
    pub currency: String,
    /// The currency it holds: the same number as the currency's total in `stockpile`.
    pub cash: Quantity,
    /// The hex's posted prices by good name, in the currency. The currency itself isn't
    /// listed. Empty for a hex with no market, or a market that prices nothing.
    pub prices: BTreeMap<String, Quantity>,
    /// Every good the business holds, by name, split into perish groups, soonest first
    /// and the never-perishing group last. Only goods it holds; only groups above 0.
    pub stockpile: BTreeMap<String, Vec<PerishGroup>>,
    /// Its factories by ID number.
    pub factories: BTreeMap<u32, FactoryView>,
    /// The technologies its factories run, each once, by name, with every number the
    /// rules give it.
    pub technologies: BTreeMap<String, TechnologyView>,
    /// Its deposits, summed per resource type name. Only types it owns deposits of.
    pub deposits: BTreeMap<String, DepositsView>,
}

/// One perish group of a good in a stockpile.
pub struct PerishGroup {
    /// The amount in the group. Greater than 0 in a view the engine builds.
    pub amount: Quantity,
    /// The turn at whose end the group perishes, or None if it never does.
    pub perishes: Option<Turn>,
}

/// One factory as its business sees it.
pub struct FactoryView {
    /// The name of the technology it runs.
    pub technology: String,
    /// The share of the technology's capital bundle it holds: finite, 0 to 1.
    pub condition: f64,
}

/// A technology's numbers, as the rules give them.
pub struct TechnologyView {
    /// Inputs per unit of output, by good name. May be empty.
    pub inputs: BTreeMap<String, Quantity>,
    /// The output good's name.
    pub output: String,
    /// The most output a factory makes in one turn.
    pub capacity: Quantity,
    /// The capital bundle, by good name. Never empty.
    pub capital: BTreeMap<String, Quantity>,
    /// The construction bundle, by good name. May be empty.
    pub construction: BTreeMap<String, Quantity>,
    /// Turns to build a factory (t).
    pub construction_turns: u32,
    /// Turns a factory's capital lasts (t₁).
    pub lifetime: u32,
}

/// The deposits of one resource type a business owns.
pub struct DepositsView {
    /// How many it owns. At least 1.
    pub count: u32,
    /// The name of the good each deposit emits.
    pub produces: String,
    /// How much each emits per turn.
    pub output: Quantity,
}

impl BusinessView {
    /// Compact JSON, one line.
    pub fn to_json(&self) -> String;
    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String;
    /// Reads a view, refusing JSON that isn't the format or holds a condition that isn't a
    /// finite number from 0 to 1.
    pub fn from_json(text: &str) -> Result<BusinessView, FormatError>;
    /// Value problems a parse can't catch: for each factory whose condition isn't a
    /// fraction, `"factory #{id}: condition must be a finite number from 0 to 1 (got {v})"`.
    pub(crate) fn problems(&self) -> Vec<String>;
}

/// Builds `business`'s view of `world` at its current turn. Reads only. Called by
/// `World::business_view`.
///
/// # Panics
/// Panics if `business` isn't from `world`'s businesses (through the World's accessors).
pub(crate) fn build(world: &World, business: BusinessId) -> BusinessView;
```

`build`, step by step (names through `world.rules().goods.name(..)`, `.technologies.name(..)`, `.resources.name(..)`, `world.businesses().name(..)`):

1. `turn = world.turn()`; `business` name; `hex = world.businesses().get(business).hex`; `currency = world.hex_currency(hex)` as a name.
2. `prices`: `world.hex_prices().posted(hex)` (good ID order; the loader never prices the currency) into a map by name. `HexPrices::posted` yields nothing for a hex without a market, which gives the empty table.
3. `stockpile`: for each `(good, _)` of `world.business_stockpile(business).iter()`, the groups from `stockpile.groups(good)` in order, `PerishesAt::EndOfTurn(t)` → `Some(t)`, `Never` → `None`. `cash = stockpile.total(currency)`.
4. `factories`: for each id of `world.factories().owned_by(business)`: key `u32::try_from(id.index()).expect("factory IDs fit in a u32")`, `FactoryView { technology: name, condition: factory.condition }`.
5. `technologies`: the distinct `TechnologyId`s of those factories (a `BTreeSet`), each as `TechnologyView` from `world.rules().technologies.get(id)`: bundles through a private `fn named_bundle(goods: &Goods, bundle: &Bundle) -> BTreeMap<String, Quantity>`.
6. `deposits`: count `world.deposits().owned_by(business)` per `deposit.resource` (a `BTreeMap<ResourceId, u32>`), then each as `DepositsView { count, produces: name of resource.produces, output: resource.output }`.

### `crates/engine/src/decider/decision.rs`

```rust
/// A business's answer for one turn.
pub struct Decision {
    /// The business it answers for.
    pub business: String,
    /// Orders per factory, by ID number. A factory not listed is left alone (run 0,
    /// maintenance 0). Absent in JSON means empty.
    #[serde(default)]
    pub factories: BTreeMap<u32, FactoryOrder>,
    /// Amounts to sell, by good name. Absent in JSON means empty.
    #[serde(default)]
    pub offers: BTreeMap<String, f64>,
    /// Amounts to buy, by good name. Absent in JSON means empty.
    #[serde(default)]
    pub bids: BTreeMap<String, f64>,
    /// The decider's account of its choice, free text, any number of lines. Stored for the
    /// dev UI, never acted on. Absent in JSON when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

/// The two fractions for one factory. A fraction left out of the JSON is 0.
pub struct FactoryOrder {
    /// The share of capacity to use: finite, 0 to 1.
    #[serde(default)]
    pub run: f64,
    /// The share of the full capital bundle to replenish: finite, 0 to 1.
    #[serde(default)]
    pub maintenance: f64,
}

/// Where in a decision a problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The decision as a whole.
    Decision,
    /// The order for the factory with this ID number.
    Factory(u32),
    /// The offer of this good.
    Offer(String),
    /// The bid for this good.
    Bid(String),
}
// Display: `decision`, `factory #3`, `offer 'grain'`, `bid 'grain'`.

/// One problem found in a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionError { place: Place, message: String }
impl DecisionError {
    /// Where the problem is.
    pub fn place(&self) -> &Place;
    /// The message, without the place.
    pub fn message(&self) -> &str;
}
// Display: `"{place}: {message}"`. `impl Error`.

/// Every problem found in one refused decision, in document order. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionErrors(Vec<DecisionError>);
impl DecisionErrors {
    pub fn iter(&self) -> std::slice::Iter<'_, DecisionError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;  // always false, as LoadErrors
}
// `IntoIterator for &DecisionErrors`; Display: one error per line, no trailing newline;
// `impl Error`.

impl Decision {
    /// A decision that leaves every factory alone and trades nothing: the legitimate
    /// "do nothing".
    pub fn empty(business: impl Into<String>) -> Decision;

    /// Checks the decision against the view it answers and the setting's goods. Reads the
    /// whole decision and reports every problem, each at its place, in document order:
    /// the decision itself, then the factories by ID, then the offers, then the bids by
    /// name. Asking for more than the business holds or can pay is not a problem here.
    pub fn validate(&self, view: &BusinessView, goods: &Goods) -> Result<(), DecisionErrors>;

    pub fn to_json(&self) -> String;
    pub fn to_json_pretty(&self) -> String;
    /// Reads a decision, refusing JSON that isn't the format or holds a fraction or an
    /// amount that isn't a valid value (the same checks `validate` makes on numbers).
    pub fn from_json(text: &str) -> Result<Decision, FormatError>;

    /// The number problems, in document order: for each factory, a `run` or `maintenance`
    /// that isn't a fraction; for each offer and bid, an amount that isn't a valid quantity.
    pub(crate) fn number_problems(&self) -> Vec<DecisionError>;
}
```

`validate`, rule by rule, pushing into one `Vec<DecisionError>` and returning `Err(DecisionErrors(v))` if non-empty:

1. If `self.business != view.business`: `Place::Decision`, `"answers for '{self}' against the view of '{view}'"`.
2. For each `(id, order)` in `self.factories` (ascending): if `!view.factories.contains_key(id)`: `Factory(id)`, `"not in this business's view"`. Independently, if `!is_fraction(order.run)`: `"run must be a finite number from 0 to 1 (got {run})"`; likewise `"maintenance must be a finite number from 0 to 1 (got {m})"`. (An unknown factory with a bad fraction reports both.)
3. For each `(good, amount)` in `self.offers`, then in `self.bids`, with `Place::Offer`/`Place::Bid`: if `!is_amount(amount)`: `"amount must be a finite number of at least 0 (got {amount})"`. Independently, one of: `goods.id(good).is_err()` → `"unknown good"`; else `good == view.currency` → `"it is this hex's currency, which can't be traded against itself"`; else `!view.prices.contains_key(good)` → `"no price is posted for it on this hex"`. A good that passes all three is fine whatever the amount: holdings and cash are not checked.

`{run}` and `{amount}` use `f64`'s `Display` (`1.5`, `-0.1`, `NaN`, `inf`). `number_problems` is rules 2 and 3 without the view checks; `validate` calls it and interleaves (one loop per section, so the order is per place, as above). `from_json`: `from_json::<Decision>(text)?`, then `checked(number_problems.iter().map(ToString::to_string).collect())`.

### `crates/engine/src/decider/batch.rs`

```rust
/// One turn's views, served together: the engine's message to the deciders.
pub struct Request {
    /// The turn every view is for.
    pub turn: Turn,
    /// One view per business served, in business ID order when the engine builds it.
    pub views: Vec<BusinessView>,
}

/// One turn's decisions, collected together: the deciders' message back.
pub struct Reply {
    /// The turn the decisions answer. Must match the request's.
    pub turn: Turn,
    /// Decisions in any order, for any subset of the request's businesses. Absent in JSON
    /// means empty.
    #[serde(default)]
    pub decisions: Vec<Decision>,
}

/// A request and its reply, saved together: the recording Step 06's Python tests read and
/// Step 07's replay decider answers from.
pub struct Recording {
    /// The views served.
    pub request: Request,
    /// The decisions collected.
    pub reply: Reply,
}

/// Why a reply was refused as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyError {
    /// The reply's turn isn't the request's.
    TurnMismatch {
        /// The request's turn.
        request: Turn,
        /// The reply's turn.
        reply: Turn,
    },
    /// A decision for a business that isn't in the request.
    UnknownBusiness {
        /// The business named.
        name: String,
    },
    /// A second decision for one business.
    DuplicateBusiness {
        /// The business named twice.
        name: String,
    },
}
// Display: `the reply answers turn {reply}, but the request is for turn {request}`;
// `a decision for '{name}', which isn't in the request`; `two decisions for '{name}'`.
// `impl Error`.

/// Every reply-level problem, in order: the turn first, then the decisions in reply order.
/// Never empty. Same shape and impls as `DecisionErrors`.
pub struct ReplyErrors(Vec<ReplyError>);

/// What one business in the request got from the reply.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer<'a> {
    /// No decision: the business does nothing this turn.
    Nothing,
    /// A decision that passed validation against the business's view.
    Valid(&'a Decision),
    /// A decision that failed validation; what happens next is Step 07's.
    Refused(DecisionErrors),
}

impl Reply {
    /// Checks the reply against the request it answers. Refuses the whole reply, with
    /// every reply-level problem, when its turn isn't the request's, it names a business
    /// not in the request, or it names a business twice. Otherwise returns one `Answer` per
    /// view, in the request's order, each decision validated against its view.
    pub fn validate<'a>(&'a self, request: &Request, goods: &Goods) -> Result<Vec<Answer<'a>>, ReplyErrors>;
    pub fn to_json(&self) -> String;  pub fn to_json_pretty(&self) -> String;
    /// Reads a reply; value problems are each decision's, prefixed `"{business}: "`.
    pub fn from_json(text: &str) -> Result<Reply, FormatError>;
}
impl Request {
    pub fn to_json(&self) -> String;  pub fn to_json_pretty(&self) -> String;
    /// Reads a request; value problems are each view's, prefixed `"{business}: "`.
    pub fn from_json(text: &str) -> Result<Request, FormatError>;
}
impl Recording {
    pub fn to_json(&self) -> String;  pub fn to_json_pretty(&self) -> String;
    /// Reads a recording; value problems are the request's then the reply's.
    pub fn from_json(text: &str) -> Result<Recording, FormatError>;
}
```

`Reply::validate`: collect `ReplyError`s: `TurnMismatch` if `self.turn != request.turn`; then for each decision in order, `UnknownBusiness` if no view has that `business`, else `DuplicateBusiness` if an earlier decision had it (a `BTreeSet<&str>` of names seen). If any, `Err`. Otherwise for each view in request order: the decision with `decision.business == view.business` (at most one now) → `Nothing` if none, else `Valid(d)` or `Refused(errors)` from `d.validate(view, goods)`.

Pitfalls:

- `#[serde(deny_unknown_fields)]` with `#[serde(default)]` fields is fine; with `#[serde(deserialize_with)]` on a field, `default` needs the field's type to be `Default` (`BTreeMap` is).
- `deserialize_with` on a `BTreeMap<u32, _>`: serde_json parses the quoted key `"0"` into `u32` itself; a key `"abc"` fails with serde's message. The helper only tracks duplicates.
- `Option<Turn>` in `PerishGroup`: serde writes `null` for None; write it always (no `skip_serializing_if`), so every group has both keys.
- `f64` keys are never used; `f64` values with `PartialEq` are fine because every read value is finite after the checks.
- `missing_docs` is on: document every field, including the named fields of `ReplyError`'s variants.
- Don't import `serde_json` outside `mod.rs`.

## Tasks

1. `Cargo.toml` dependencies; `Quantity` and `Turn` serde impls with unit tests (round trip of `0`, `0.1`, `1.0/3.0`, `1e-300`, `5e-324`, `f64::MAX`, `123456789.123456789`; a JSON integer reads as a quantity; `-1`, `-0.0` (reads as zero), `1e999`, `"1"` refused with a message; `Turn` round trip of `0`, `7`, `u64::MAX`; `3.0`, `-1`, `"3"` refused).
2. `decider/mod.rs`: docs, `FormatError`, helpers; `lib.rs` module and re-exports; crate docs.
3. `decider/view.rs`: types, `build`, JSON methods, `problems`; `World::business_view`; `world.rs` docs.
4. `decider/decision.rs`: types, errors, `validate`, `number_problems`, JSON methods.
5. `decider/batch.rs`: types, errors, `Reply::validate`, JSON methods.
6. Unit tests in the three files (Test Plan).
7. `crates/engine/tests/decider.rs` (Test Plan).
8. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the output is unchanged.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py` (or `cargo test --workspace`). Compare quantities and fractions with `assert_eq!` where the spec guarantees exactness (every JSON round trip; the sample's whole-number and half values), and `close` where a value passes through arithmetic the engine doesn't guarantee exact. Test names are suggestions.

### Unit tests

**`quantity.rs`, `turn.rs`** — Task 1's lists, through `serde_json::to_string` / `from_str` directly.

**`decider/view.rs`** — Local helper `view()` builds a `BusinessView` by struct literal: turn 3, business `a`, currency `silver`, cash 10, prices labor 1 and grain 2, stockpile silver `[{10, None}]` and grain `[{4, Some(3)}, {2, Some(5)}]`, factories 0 (`farm`, 1.0) and 1 (`farm`, 0.5), one technology `farm` (inputs grain 1 and labor 0.5, output `bread`, capacity 12, capital tools 4, construction labor 8, 2, 8), deposits `field` (2, grain, 4).

- `a_view_round_trips_through_json`: `from_json(&v.to_json()) == v`, and the same through `to_json_pretty`.
- `json_keys_are_names_and_id_numbers`: `to_json` contains `"business":"a"`, `"factories":{"0":{"technology":"farm","condition":1.0},"1":{...}}`, `"perishes":null` and `"perishes":3`, `"prices":{"grain":2.0,"labor":1.0}` (alphabetical), and no `Id<` text.
- `extreme_values_round_trip_exactly`: a view with cash `f64::MAX`, a price `1e-300`, a group amount `5e-324` with `perishes: Some(Turn::MAX)`, turn `Turn::MAX`, and a condition `1.0/3.0` reads back equal.
- `reading_refuses_bad_values_with_a_message`: a condition of `1.5` → `Err(FormatError::Values(v))` with `v == ["factory #0: condition must be a finite number from 0 to 1 (got 1.5)"]`; a negative cash → `Err(FormatError::Json(m))` with `m` containing `a quantity must be a finite number of at least 0`; an unknown key `"cach"` → `Json` containing `unknown field`; a prices table with `"grain"` twice → `Json` containing `the key 'grain' is written twice`; a factory key `"x"` → `Json`; `{` → `Json`.

**`decider/decision.rs`** — Local helpers: `goods()` builds a `Goods` (`NamedList::from_parts` over a `NameRegistry`) of `labor`, `grain`, `copper`, `silver`; `view()` as above but with factories 0 and 1 only needing IDs (the rest can be minimal); `order(run, maintenance)`.

- `an_empty_decision_is_valid` and `a_full_decision_is_valid`: `Decision::empty("a")`; factories `{0: (0.5, 0.25)}`, offers `{grain: 3}`, bids `{labor: 2}`, trace `Some("because")` → `Ok(())`.
- `asking_for_too_much_is_valid`: offers `{grain: 1e6}` (the view holds 6), bids `{labor: 1e9}` (cash 10), and offering and bidding `grain` at once → `Ok(())`.
- `every_refusal_is_reported_at_its_place`: one decision with factories `{1: (NaN, 0), 2: (1.5, -0.1)}`, offers `{silver: 1, copper: 1, grain: -2}`, bids `{wheat: 1, labor: inf}` against `view()` gives, in this order, the lines: `factory #1: run must be a finite number from 0 to 1 (got NaN)`, `factory #2: not in this business's view`, `factory #2: run must be a finite number from 0 to 1 (got 1.5)`, `factory #2: maintenance must be a finite number from 0 to 1 (got -0.1)`, `offer 'copper': no price is posted for it on this hex`, `offer 'grain': amount must be a finite number of at least 0 (got -2)`, `offer 'silver': it is this hex's currency, which can't be traded against itself`, `bid 'labor': amount must be a finite number of at least 0 (got inf)`, `bid 'wheat': unknown good`. (Offers and bids come in name order because they are `BTreeMap`s.)
- `a_decision_for_another_business_is_refused`: `Decision::empty("b")` against `a`'s view → one line `decision: answers for 'b' against the view of 'a'`.
- `a_decision_round_trips_through_json` (full decision, both writers; trace with two lines) and `an_empty_decision_writes_empty_tables_and_no_trace`: `Decision::empty("a").to_json() == r#"{"business":"a","factories":{},"offers":{},"bids":{}}"#`.
- `a_hand_written_decision_reads_as_expected`: `{"business": "a", "factories": {"0": {"run": 1}}, "offers": {"grain": 8}, "bids": {"labor": 6}, "trace": "sell the grain"}` reads to factories `{0: (1.0, 0.0)}`, offers `{grain: 8.0}`, bids `{labor: 6.0}`, trace `Some`; `{"business": "a"}` reads as `Decision::empty("a")`.
- `reading_refuses_malformed_json_with_a_message`: each of these is `Err` and its `to_string()` contains the text given: `{"business": ` (`Json`, `EOF` or `expected`: assert only that it's `Json` and non-empty); `{"business": 5}` (`invalid type`); `{"business":"a","offer":{}}` (`unknown field`); `{"business":"a","offers":{"grain":1,"grain":2}}` (`the key 'grain' is written twice`); `{"business":"a","factories":{"0":{"run":"all"}}}` (`invalid type`); `{"business":"a","offers":{"grain":-1}}` → `FormatError::Values(["offer 'grain': amount must be a finite number of at least 0 (got -1)"])`; `{"business":"a","factories":{"0":{"run":2}}}` → `Values(["factory #0: run must be a finite number from 0 to 1 (got 2)"])`; `{"business":"a","offers":{"grain":1e999}}` and `{"business":"a","offers":{"grain":NaN}}` → `Err` (whichever layer catches it; assert `is_err()` and a non-empty message).
- `error_displays`: `Place` and `DecisionError` display as specified; `DecisionErrors` joins lines with `\n` and no trailing newline.

**`decider/batch.rs`** — Helpers: `request()` with views for `a`, `b`, `c` at turn 4 (minimal views, currency `silver`, no prices; the `view()` of `decision.rs` can be shared through a `#[cfg(test)] pub(crate)` helper in `view.rs`); `goods()` as above.

- `a_subset_in_any_order_is_accepted`: reply turn 4 with decisions `[c, a]` → `Ok([Valid(&a), Nothing, Valid(&c)])`.
- `an_empty_reply_means_everyone_does_nothing`: `Reply { turn: 4, decisions: [] }` → three `Nothing`.
- `a_wrong_turn_is_refused`, `an_unknown_business_is_refused`, `a_duplicate_business_is_refused`: one `ReplyError` each, with the Display texts of the Interfaces section; `every_reply_problem_is_reported`: turn 5 with `[a, z, a]` → `[TurnMismatch{4,5}, UnknownBusiness z, DuplicateBusiness a]`.
- `an_invalid_decision_is_refused_without_refusing_the_reply`: `[a with factories {9: (0,0)}, c]` → `Ok([Refused(errs), Nothing, Valid(&c)])` with `errs` one line `factory #9: not in this business's view`.
- `request_reply_and_recording_round_trip`: each through both writers; a `Request` with `Turn::MAX`; `Reply::from_json(r#"{"turn": 4}"#)` reads as an empty reply; value problems in a reply are prefixed: `{"turn":4,"decisions":[{"business":"a","offers":{"grain":-1}}]}` → `Values(["a: offer 'grain': amount must be a finite number of at least 0 (got -1)"])`.

### `crates/engine/tests/decider.rs` (`mod common;`)

Helpers: `business(&World, name) -> BusinessId` (`world.businesses().id(name).unwrap()`); `view(&World, name) -> BusinessView`; `groups_of(&BusinessView, good) -> Vec<(f64, Option<u64>)>`.

- **AC 1 (the sample's views).** `the_sample_businesses_see_their_own_circumstances`: load the sample, build the world (turn 0), take the three views. Common: `turn == Turn::ZERO`, `currency == "silver"`, `prices` == `{copper: 5, grain: 2, tools: 12, unimproved_copper: 1, unimproved_grain: 0.5, unskilled_labor: 1}`. `center_farm`: stockpile `silver [(100, None)]`, `unimproved_grain [(8, Some(0))]`, cash 100, factories `{0: (farm, 1.0)}`, technologies `{farm}` with inputs `{unimproved_grain: 1, unskilled_labor: 0.5}`, output `grain`, capacity 12, capital `{tools: 4}`, construction `{unskilled_labor: 8}`, 2, 8; deposits `{arable_land: (2, unimproved_grain, 4)}`. `center_mine`: `silver [(50, None)]`, `unimproved_copper [(2.5, Some(0))]`, cash 50, factories `{1: (mine, 0.75)}`, technologies `{mine}` (inputs `{unimproved_copper: 1, unskilled_labor: 1}`, `copper`, 2.5, `{tools: 2}`, `{unskilled_labor: 4}`, 2, 8), deposits `{copper_ore: (1, unimproved_copper, 2.5)}`. `center_toolmaker`: `silver [(200, None)]`, cash 200, factories `{2: (toolmaker, 1.0)}`, technologies `{toolmaker}` (inputs `{copper: 2, unskilled_labor: 1.5}`, `tools`, 4, `{tools: 3}`, `{unskilled_labor: 6}`, 3, 24), deposits empty. Each view's `stockpile` has exactly the goods listed, each `factories` exactly one entry, each `technologies` exactly one. After `step()`, `center_farm`'s view has turn 1 and `unimproved_grain [(8, Some(1))]` (turn 0's emission perished).
- **AC 1 (no market, several of a kind).** `a_business_on_a_hex_with_no_market_sees_no_prices`: `businesses_valid` world: `idle`'s view has empty `prices`, empty `stockpile`, cash 0, no factories, no technologies, no deposits, currency `coin`; `farm_a`'s has prices `{grain: 2, ore: 3}`, stockpile `coin [(10, None)]`, `labor [(4, Some(0))]`, `grain [(4, Some(2))]`, deposits `{field: (2, grain, 2)}`; `farm_b`'s stockpile `coin [(5, None)]`, `ore [(1.5, None)]`, `grain [(2, Some(2))]`, deposits `{field: (1, grain, 2), vein: (1, ore, 1.5)}`. `a_technology_is_listed_once_however_many_factories_run_it`: `production_valid` world, `works`: factories `{0: (smelter, 1.0), 1: (smelter, 0.5), 2: (bakery, 1.0)}`, technologies exactly `{bakery, smelter}`, stockpile `coin [(50, None)]`, `labor [(13, Some(0))]`, `ore [(30, Some(2))]`, `tools [(3, None)]`, cash 50.
- **AC 1 (the world is unchanged).** `building_views_changes_nothing`: sample world; clone every business's stockpile (`Stockpile: Clone + PartialEq`) and collect every factory's condition; build every view twice; the stockpiles and conditions compare equal, and the two views of each business are equal.
- **AC 2.** `a_decision_validates_against_the_sample_view`: `center_farm`'s view and `world.rules().goods`: a decision with factories `{0: (1, 0.125)}`, offers `{unimproved_grain: 8}`, bids `{unskilled_labor: 6}`, trace → `Ok`; `Decision::empty("center_farm")` → `Ok`; offers `{unimproved_grain: 1000}` and bids `{tools: 1000}` → `Ok` (too much is not nonsense); factories `{1: (1, 0)}` (the mine's factory) → one line `factory #1: not in this business's view`; offers `{silver: 1}` → the currency line; `idle`'s view in `businesses_valid` with bids `{grain: 1}` → `bid 'grain': no price is posted for it on this hex`.
- **AC 3.** `every_message_round_trips_on_the_sample`: each sample view, a full decision for each, a `Request` of the three views at turn 0, a `Reply` of the three decisions, and a `Recording` of both: `from_json(to_json) == original` and the same for `to_json_pretty`; the recording's pretty JSON contains the line `"business": "center_farm"`. `a_hand_written_decision_reads_and_validates`: the JSON of the unit test, with `"center_farm"` and the sample's goods, reads and validates `Ok` against `center_farm`'s view.
- **AC 4.** `a_reply_is_checked_against_its_request`: request of the three sample views at turn 0; reply `[center_toolmaker's, center_farm's]` → `[Valid, Nothing, Valid]`; turn 1 → `Err` with one `TurnMismatch`; a decision for `nobody` → `UnknownBusiness`; two for `center_mine` → `DuplicateBusiness`.
- **AC 5.** No other test changes; the dev UI checker's lines for the sample are unchanged (the existing `checker.rs` tests and the manual run in Task 8).

## Constraints

- Follow the existing conventions: doc comments on every public item and every named field (clippy runs with `-D warnings`); `#[cfg(test)]` rather than `#[allow(dead_code)]`; `cargo fmt`; deterministic iteration (`BTreeMap`/`BTreeSet`, never `HashMap`).
- `serde_json` is used only in `decider/mod.rs` (and the `quantity.rs`/`turn.rs` unit tests). `serde` derives and attributes may appear in any protocol type. No `serde` attribute on `Quantity` beyond the hand-written impls.
- The protocol carries setting names and factory ID numbers (`u32`), never `Id<K>` values. Nothing in the `decider` module holds a reference into the World.
- `World::business_view` is read-only (`&self`). Do not change the turn, `TurnReport`, `Stockpile`, `Market`, `HexPrices`, `Factories`, the loader, the sample setting, any test setting, the dev UI, or any existing test.
- Do not edit `docs/` or `manual.ipynb`; documentation is drafted later in the loop. (`docs/workspace.md`'s "only external dependency is `toml`" sentence becomes false; `step-docs-draft` will propose the fix.)
- Out of scope: any Python, process or transport (Steps 06, 07); building the request from every business and wiring the decision point into the turn (Step 07); resolving decisions, trimming over-asking, the cash cap, the per-business record (Step 08); what a refused decision or missing reply means (Step 07); any verb beyond the two fractions, offers and bids; per-deposit IDs; structured traces; the dev UI (Step 09).

## Notes from the build

Recorded after Code Review, for Step 07 (which reads recordings from files) and the docs:

- The `float_roundtrip` feature of `serde_json` is required for the lossless rule; the dependency line above was amended during the build.
- A view's `perishes` key may be left out of hand-written JSON and reads as "never perishes" (serde fills a missing `Option` with `None`); engine-written views always write it. If recordings from untrusted files matter, Step 07 should require the key.
- Nothing checks that a request is consistent (a view's turn against the request's turn, or two views naming one business). Engine-built requests can't have either problem; Step 07 should check requests it reads from recordings, or state that only engine-built requests are trusted.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes, every test in the Test Plan is written and passes, the sample's checker output is unchanged, and every Acceptance Criterion in `design.md` is met.
