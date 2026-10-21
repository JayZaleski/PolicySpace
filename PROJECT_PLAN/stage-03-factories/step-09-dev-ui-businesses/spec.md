# Step 09: dev-ui-businesses — Spec

## Context
Read before starting:
- This Step's `design.md` (the wording, the order of every section, and the Acceptance Criteria the Test Plan maps to).
- `docs/dev-ui.md` (the dev UI's modules, the `text` conventions, the window's panels, the marks, and "Extending the window"), `docs/world.md`, `docs/trade.md`.
- The Stage's earlier drafts, for what was built: `PROJECT_PLAN/stage-03-factories/step-08-business-trade/docs_changes.md` (the business round, `BusinessTrade`, `BusinessOutcome`, the turn line) and `step-07-decider-host/docs_changes.md` (`BusinessAnswer`, `DeciderHost`, `python::host_for`).
- Code, all of it: `crates/dev-ui/src/text.rs`, `crates/dev-ui/src/app.rs`, `crates/dev-ui/src/session.rs`, `crates/dev-ui/src/test_support.rs`.
- Engine code the text module reads: `crates/engine/src/world.rs` (`World::businesses`, `factories`, `business_stockpile`, `business_answer`, `business_outcome`, `BusinessOutcome`, `business_submission`, `operate_business`), `crates/engine/src/trade.rs` (`BusinessTrade`), `crates/engine/src/production.rs` (`FactoryRecord`, `RunRecord`, `MaintenanceRecord`, the private `slices`, `condition_of`, `snap`, `TOLERANCE`, and `run`'s eligibility test), `crates/engine/src/decider/host.rs` (`BusinessAnswer`), `crates/engine/src/decider/decision.rs` (`Decision`, `FactoryOrder`, `DecisionError`, `DecisionErrors`, `Place`), `crates/engine/src/businesses.rs`, `crates/engine/src/factories.rs`, `crates/engine/src/technologies.rs`, `crates/engine/src/bundle.rs`, `crates/engine/src/resources.rs` (`Deposits::owned_by`, `of_type_on_hex`), `crates/engine/src/stockpile.rs`, `crates/engine/src/lib.rs`.
- Engine tests to copy decisions and numbers from: `crates/engine/tests/business_trade.rs` (`rule`, `decision`, `steady_decisions`, `a_cash_short_business_is_scaled_and_its_run_is_cut`, `a_refused_decision_idles_its_business`, `the_sample_chain_runs_under_the_arithmetic_decider`) and `crates/engine/tests/settings/README.md` (the `business_trade` setting).
- Settings: `settings/sample/` (`rules/technologies.toml`, `start/businesses.toml`, `start/factories.toml`), `crates/engine/tests/settings/business_trade/`, `crates/engine/tests/settings/valid_minimal/`.

## Approach
- **One engine addition, nothing else in the engine.** The private `slices` helper in `production.rs` becomes the public `slices_held(condition, lifetime)`, re-exported from the crate root, so the panel's remaining-life count calls the engine's own eligibility rule (snapping within `TOLERANCE`) instead of copying it. Its behavior and every existing engine test are unchanged.
- **All new panel text is pure functions in `text`**, as the earlier dev-ui Steps did: each returns strings or small row structs built from engine types, with no egui, so every line the window shows is unit-tested without a window. `app.rs` only renders them.
- **`text` becomes a directory module.** `text.rs` moves to `text/mod.rs` unchanged (apart from one new Goods column and a shared deposit-line helper), and the new business, factory, technology and remaining-life text lives in `text/businesses.rs`, re-exported from `text`, so callers keep writing `text::business_section`. The existing file is already 1,100 lines; the new text and its tests are about as much again.
- **Nothing is rounded.** Fractions, conditions, wear and the proportion are f64 `Display`, amounts are `Quantity` `Display`, exactly as the rest of the dev UI; no new format helper. The row detail (inputs, maintenance cost, capital bundle) is hover text on three cells.
- **The trade table merges the decision and the outcome.** A valid decision's rows come from `BusinessTrade` (its `offered` and `bid` are the decision's amounts as sent, resolved to good IDs; `sold` and `bought` are what happened). A refused decision has no trade, so its rows come from the stored `Decision`'s `offers` and `bids` maps with sold and bought `0`. A business that decided nothing has no rows.
- **Factory rows come straight from the outcome's records.** The engine already passes run and maintenance 0 for an unmentioned factory and for every factory of a business that decided nothing or was refused, so the rows need no special case.
- **The session is unchanged.** The window already steps the world under the setting's deciders; the panels read the world each frame.
- **No parallel build.** The window code depends on the text module's new types, and the engine change is one function, so the work is one build.

## Changes

### `crates/engine/src/production.rs`
- Rename the private `fn slices(condition: f64, lifetime: u32) -> f64` to `pub fn slices_held(condition: f64, lifetime: u32) -> f64` and update its two call sites (`run`'s eligibility test, `wear`). Doc comment (clippy requires one): the number of lifetime-slices a factory of a technology with `lifetime` holds at `condition` (`condition × lifetime`), snapped to the nearest whole number when within `TOLERANCE` of it; a factory runs only when this is at least 1, and `floor` of it is the number of further turns it can run with no maintenance. Add this sentence to the module docs' "Running" bullet: the rule is public as `slices_held` so a UI can show remaining life with the engine's own arithmetic.
- Add one unit test in the module's `tests`: `slices_held` gives 8 for (1.0, 8), 1 for (0.125, 8), a value below 1 for (0.125 − 1e-6, 8), 0 for (0.0, 8), exactly 2 for (2.0 / 3.0, 3), and exactly 24 for (1.0 − 1e-12, 24).

### `crates/engine/src/lib.rs`
- `pub use production::{FactoryRecord, MaintenanceRecord, Operation, RunRecord, operate, slices_held};`. Nothing else.

### `crates/dev-ui/src/text.rs` → `crates/dev-ui/src/text/mod.rs`
- Move the file (content unchanged except as below). Add `mod businesses;` and `pub use businesses::{...}` for every public item listed under Interfaces & Data for that file.
- Module docs: add businesses, factories and technologies to the list of what the panels show, and "Step 02–09 designs".
- `GoodRow` gains `pub in_businesses: String` after `in_pops`: the good's total over every business stockpile, "0" if none. `good_rows` fills it with a third pass, over `world.businesses().iter()` and `world.business_stockpile(id).iter()`, the way `in_pops` is filled.
- Extract the body of `deposit_lines` into a private `fn deposit_lines_from(world: &World, count_of: impl Fn(ResourceId) -> usize) -> Vec<String>` that writes one line per resource type with a count above 0, in resource ID order, with the same wording and the same added-not-multiplied total. `deposit_lines(world, hex)` calls it with `|id| world.deposits().of_type_on_hex(hex, id).count()`. (The child module uses it for a business's owned deposits.) Keep the private `list` and `stock_lines` helpers as they are; the child module can call them.

### `crates/dev-ui/src/text/businesses.rs` (new)
Everything under Interfaces & Data for this file, with its unit tests (Test Plan). It imports `super::{StockLine, count, list, stock_lines, deposit_lines_from}` and the engine types it needs.

### `crates/dev-ui/src/test_support.rs`
- Add `pub fn world_with(relative: &str, host: &mut DeciderHost) -> World`: loads the setting at `relative` (panicking with the errors) and returns `World::new(setting, host)`'s world, panicking with the error if the host doesn't hold a name. `world_at` becomes `world_with(relative, &mut DeciderHost::idle())`.
- Add `pub fn python_world(relative: &str) -> (World, DeciderHost)`: loads the setting, builds the host with `policyspace_engine::decider::python::host_for(&setting)` (panicking with its error, as `session.rs`'s Python test does), builds the world and returns both so a test can step it. Document that tests using it need the project's virtual environment, like `session.rs`'s `the_sample_loads_and_steps_under_its_python_deciders`.

### `crates/dev-ui/src/app.rs`
- Module docs: add businesses to what the hex panel shows and technologies and businesses to the world panel; "Step 07 to 09 specs".
- Imports: the new `text` items (`business_rows`, `business_section`, `technology_rows`), and egui's `Rect` and `Vec2` if the square mark needs them.
- **World panel** (`world_panel_contents`), in this order after the existing sections:
  - The Goods grid gets a sixth column, heading "In businesses", cell `row.in_businesses`; `num_columns(6)`.
  - A `CollapsingHeader::new("Technologies")`, open by default, holding `Grid::new("technologies").num_columns(9).striped(true)`: headings "Name", "Recipe", "Output", "Capacity", "Capital", "Construction", "Construction turns", "Lifetime", "Factories"; the Recipe, Capital and Construction cells are `ui.vertical` stacks of one label per line, like the POP types table's Endowment cell; the rest are labels (`factories` via `to_string()`).
  - A `CollapsingHeader::new("Businesses")`, open by default, holding `Grid::new("businesses").num_columns(6).striped(true)`: headings "Name", "Hex", "Decider", "Cash", "Factories", "Answer"; a row with `refused` draws all six cells with `ui.colored_label(ui.visuals().error_fg_color, ..)`, otherwise `ui.label`.
- **Hex panel** (`hex_panel_contents`), after the POPs loop:
  - `ui.add_space(4.0); ui.strong("Businesses");`. If `world.businesses().on_hex(hex)` is empty, `ui.label("No businesses.")`.
  - Otherwise, for each `&business` in it (ID order), `let section = business_section(world, business);` and a `CollapsingHeader::new(section.heading).id_salt(("business", business.index())).default_open(true)` whose body is, in order:
    1. `ui.label(section.decider)`; then the answer line: `ui.colored_label(error color, section.answer)` when `section.refused`, else `ui.label(section.answer)`; then each line of `section.errors` as a `colored_label` in the error color (empty unless refused).
    2. `ui.strong("Deposits")`; `section.deposits` as labels, or `ui.label("Owns no deposits.")` when empty.
    3. One `ui.horizontal` row: `ui.label(section.cash)` then `section.proportion` as a `colored_label` in the error color when `section.cash_short`, else a plain label.
    4. `ui.strong("Trade")`; if `section.trade` is empty, `ui.label("Nothing was traded.")`; else `Grid::new(("business trade", business.index())).num_columns(5).striped(true)` with headings "Good", "Offered", "Sold", "Bid", "Bought" and one row per `TradeRow`.
    5. `ui.strong("Factories")`; if `section.factories` is empty, `ui.label("No factories.")`; else `Grid::new(("business factories", business.index())).num_columns(11).striped(true)` with headings "#", "Technology", "Condition before", "Condition after", "Remaining life", "Run asked", "Run done", "Output", "Wear", "Maintenance asked", "Maintenance done". Per `FactoryRow`: `number` (`to_string()`), `technology`, `condition_before`, `condition_after` with `.on_hover_text(row.capital)`, `remaining_life` as a `colored_label` in the error color when `row.shut_down` else a label, `run_asked`, `run_done`, `output` with `.on_hover_text(row.inputs)`, `wear`, `maintenance_asked`, `maintenance_done` with `.on_hover_text(row.maintenance_cost)`.
    6. If `section.trace` is `Some(trace)`: `CollapsingHeader::new("Trace").id_salt(("business trace", business.index())).default_open(false)` holding `ui.add(Label::new(RichText::new(trace).monospace()).selectable(true))`.
    7. `ui.strong("Stockpile")`; `stockpile_listing(ui, ("business", business.index()), section.stockpile)`.
  - Every repeated header and grid has an explicit id salt from the business's index, never from the heading (which holds the cash and changes every step).
- **Map mark.** `HexLook` gains `has_businesses: bool`, filled in `draw_map` from `!world.businesses().on_hex(id).is_empty()`. In `paint_hex`'s hexagon arm, after the POP mark, when `has_businesses`: a filled square of side `2 · radius` (the shared `radius`) centered at `pos + vec2(-0.866 · offset, 0.5 · offset)` (the shared `offset`; lower-left of the label, 60° round the same ring from the deposit mark below it, so a hex can show any combination of the three), in `look.foreground`, drawn with `painter.rect_filled(Rect::from_center_size(center, Vec2::splat(2.0 * radius)), 0.0, look.foreground)` (adapt to egui 0.36's signature if it differs). Dots get no marks, as before. Update `draw_map`'s and `HexLook`'s docs.

### `crates/dev-ui/src/session.rs`, `cli.rs`, `check.rs`, `geometry.rs`, `main.rs`, `tests/`
- Unchanged. (`check` output and the turn line don't change.)

## Interfaces & Data

All in `crates/dev-ui/src/text/businesses.rs`, re-exported from `text`. Every string uses `Quantity`/f64 `Display`; lists are joined with `", "`; nothing is rounded. Good names come from `world.rules().goods.name(..)`, technology names from `world.rules().technologies.name(..)`, business names from `world.businesses().name(..)`. The currency name of a business is `world.rules().goods.name(world.hex_currency(business.hex))`.

```rust
/// Remaining life of a factory at `condition` for a technology with `lifetime`:
/// `Some(n)` with `n = slices_held(condition, lifetime).floor() as u32` when at least one
/// slice is held, else `None` (shut down: the engine won't run it).
pub fn remaining_life(condition: f64, lifetime: u32) -> Option<u32>;

/// "shut down" for None, "1 turn" for Some(1), "{n} turns" otherwise (`count(n, "turn", "turns")`).
pub fn format_life(life: Option<u32>) -> String;

/// The owned deposits of `business`, one line per resource type in resource ID order,
/// worded exactly like `deposit_lines` ("{type} × {count}: {output} {good} each, {total}
/// total"); empty if it owns none. Counts come from `world.deposits().owned_by(business)`
/// grouped by `deposit.resource`, through `deposit_lines_from`.
pub fn business_deposit_lines(world: &World, business: BusinessId) -> Vec<String>;

/// `stock_lines(world, world.business_stockpile(business))`: written exactly like a hex's.
pub fn business_stockpile_lines(world: &World, business: BusinessId) -> Vec<StockLine>;

/// One row of a business's trade table.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRow {
    pub good: String,     // the good's name (or, in a refused decision, the name as sent)
    pub offered: String,  // "0" when nothing
    pub sold: String,
    pub bid: String,
    pub bought: String,
}

/// One row of a business's factories table.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryRow {
    pub number: u32,               // the factory's ID number: `id.index() as u32`, as the decider addresses it
    pub technology: String,
    pub condition_before: String,  // record.condition_before
    pub condition_after: String,   // record.condition_after
    pub remaining_life: String,    // format_life(remaining_life(record.condition_after, lifetime))
    pub shut_down: bool,           // remaining_life(..).is_none()
    pub run_asked: String,         // record.run.asked
    pub run_done: String,          // record.run.done
    pub output: String,            // "{record.run.output} {output good}"
    pub wear: String,              // record.wear
    pub maintenance_asked: String, // record.maintenance.asked
    pub maintenance_done: String,  // record.maintenance.done
    pub inputs: String,            // hover on Output: "Inputs: {list of record.run.inputs}" ("Inputs: nothing" when empty)
    pub maintenance_cost: String,  // hover on Maintenance done: "Maintenance cost: {list of record.maintenance.cost}" ("… nothing" when empty)
    pub capital: String,           // hover on Condition after: "Capital: {list of technology.capital().scaled(Quantity::new(record.condition_after))}" (every good of the bundle, even at amount 0)
}

/// Everything the hex panel shows for one business.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessSection {
    pub heading: String,       // "{name}: {cash} {currency}", cash = business stockpile's total of the currency, e.g. "center_farm: 85 silver"
    pub decider: String,       // "Decider: {business.decider}"
    pub answer: String,        // "Answer: valid decision" | "Answer: refused, {count(n, "error", "errors")}" | "Answer: nothing (no decision was sent, or its decider failed)"
    pub refused: bool,
    pub errors: Vec<String>,   // each DecisionError's Display in order ("{place}: {message}"); empty unless refused
    pub deposits: Vec<String>, // business_deposit_lines
    pub cash: String,          // "Cash: {cash_before} {currency} before; {cash_after_sales} after sales; {cash_after_purchases} after purchases;"
    pub proportion: String,    // "proportion {trade.proportion()}" (f64 Display: "1", "0.2", "0")
    pub cash_short: bool,      // trade.proportion() < 1.0
    pub trade: Vec<TradeRow>,
    pub factories: Vec<FactoryRow>,
    pub trace: Option<String>, // the decision's trace as sent (valid or refused); None when absent or nothing
    pub stockpile: Vec<StockLine>,
}

pub fn business_section(world: &World, business: BusinessId) -> BusinessSection;
```

`business_section` reads `world.businesses().get(business)`, `world.business_answer(business)`, `world.business_outcome(business)` (its `trade()` and `factories()`), `world.business_stockpile(business)`, `world.factories().get(id)` and `world.rules().technologies.get(..)` per record.

**Trade rows**, by answer:
- `Valid(_)`: the goods are the union of `trade.offered()` and `trade.bid()` IDs (sold ⊆ offered and bought ⊆ bid), in good ID order; cells from `offered_of`, `sold_of`, `bid_of`, `bought_of`.
- `Refused { decision, .. }`: the names are the union of `decision.offers` and `decision.bids` keys. Names the goods list knows (`goods.id(name).is_ok()`) come first in good ID order, then unknown names in name order (so a refused bid for a misspelt good is still shown). `offered` and `bid` are the decision's f64 values written with f64 `Display`, as sent (`"0"` when the map lacks the good); `sold` and `bought` are `"0"`.
- `Nothing`: empty.

**Factory rows:** one per `(id, record)` in `outcome.factories()` (already factory ID order); `lifetime` and `output good` from the factory's technology.

**Trace:** `Some(trace.clone())` from the valid or refused decision's `trace`; `None` otherwise.

```rust
/// One row of the Technologies table.
#[derive(Debug, Clone, PartialEq)]
pub struct TechnologyRow {
    pub name: String,
    pub recipe: Vec<String>,       // per input in good ID order: "{amount} {good} per unit"; exactly ["nothing"] when inputs are empty
    pub output: String,            // the output good's name
    pub capacity: String,
    pub capital: Vec<String>,      // per good: "{amount} {good}" (never empty)
    pub construction: Vec<String>, // per good: "{amount} {good}"; exactly ["nothing"] when empty
    pub construction_turns: String,
    pub lifetime: String,
    pub factories: usize,          // factories running it, counted in one pass over world.factories().iter()
}
pub fn technology_rows(world: &World) -> Vec<TechnologyRow>; // technology ID order

/// One row of the Businesses table.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessRow {
    pub name: String,
    pub hex: String,           // world.map().coord(business.hex).to_string(): "(0, 0)"
    pub decider: String,
    pub cash: String,          // "{total of the currency in its stockpile} {currency}"
    pub factories: usize,      // factories it owns, counted in one pass over world.factories().iter()
    pub answer: &'static str,  // "nothing" | "valid" | "refused"
    pub refused: bool,
}
pub fn business_rows(world: &World) -> Vec<BusinessRow>; // business ID order
```

## Tasks
1. Engine: rename `slices` to `pub fn slices_held` in `production.rs` with its doc comment and the module-doc sentence, update the call sites, add the unit test, re-export from `lib.rs`. Run `cargo test -p policyspace-engine`: everything passes unchanged.
2. Move `crates/dev-ui/src/text.rs` to `crates/dev-ui/src/text/mod.rs`; add `mod businesses;` and the re-exports; add `in_businesses` to `GoodRow` and `good_rows`; extract `deposit_lines_from`. Update the existing `good_rows` tests to assert `in_businesses` (see Test Plan).
3. `test_support.rs`: `world_with` and `python_world`.
4. `text/businesses.rs`: `remaining_life`, `format_life`, `business_deposit_lines`, `business_stockpile_lines`, with tests.
5. `text/businesses.rs`: `TradeRow`, `FactoryRow`, `BusinessSection`, `business_section`, with tests (arithmetic decider at turn 0 and after a step, idle host, refused, cash-short, life agreement).
6. `text/businesses.rs`: `TechnologyRow`, `technology_rows`, `BusinessRow`, `business_rows`, with tests.
7. `app.rs`: the Goods column, the Technologies and Businesses sections, the hex panel's Businesses section, the map mark, the module docs.
8. Run the single check command (`.venv/Scripts/python tools/check.py`): fmt, clippy with `-D warnings`, every test. Open the window on `settings/sample` once to confirm it builds and draws (close it again; don't automate it).

## Test Plan
Unit tests in `crates/dev-ui/src/text/businesses.rs` unless stated. Numbers that come from floating-point arithmetic other than those the design's Acceptance Criteria state as literals are compared against the engine's own value formatted with `Display` (e.g. `record.wear.to_string()`), not against literals. `sample_python()` is `python_world("settings/sample")`; `sample_idle()` is `world_at("settings/sample")`; `trade_world(host)` is `world_with("crates/engine/tests/settings/business_trade", host)`. Helpers copy `rule`, `decision` and `steady` from `crates/engine/tests/business_trade.rs`.

| Test | Acceptance Criterion |
|---|---|
| `remaining_life_at_the_boundaries`: (1.0, 8) → Some(8); (0.125, 8) → Some(1); (0.125 − 1e-6, 8) → None; (0.0, 8) → None; (2.0 / 3.0, 3) → Some(2); (1.0 − 1e-12, 24) → Some(24); (0.5, 3) → Some(1). `format_life`: None → "shut down", Some(1) → "1 turn", Some(8) → "8 turns". | shut down / remaining-life rule |
| `remaining_life_counts_the_further_turns_a_factory_produces`: `trade_world` with a closure host whose `rule` answers only for `works`: factories (0: run 1, maintenance 0), (1: run 1, maintenance 0), offers iron 15 and bread 10, bids ore 20 and labor 18 (no decisions for `poor` and `remote`). At turn 0 the rows for `works` read remaining life "3 turns" (smelter, lifetime 4) and "2 turns" (bakery, lifetime 3), `shut_down` false. Step 5 times; after each step read the rows again: the smelter's `run.output` (from the outcome record) is > 0 at turns 1, 2 and 3 and 0 at turns 4 and 5, the bakery's > 0 at turns 1 and 2 and 0 at 3, 4 and 5. The life is counted after the turn's wear, so it is the number of further producing turns: the smelter reads "3 turns", "2 turns", "1 turn" at turns 0, 1, 2 and "shut down" (`shut_down` true) from turn 3 on, the turn of its last output; the bakery reads "2 turns", "1 turn" at turns 0, 1 and "shut down" from turn 2 on. | remaining-life count agrees with further producing turns |
| `the_sample_sections_at_turn_0_under_the_arithmetic_decider`: `sample_python()`, hex (0, 0): `on_hex` gives three businesses whose sections have headings `center_farm: 85 silver`, `center_mine: 37 silver`, `center_toolmaker: 152.25 silver`, decider `Decider: arithmetic`, answer `Answer: valid decision`, `refused` false, empty errors, `trace` Some and non-empty. Farm: deposits `["arable_land × 2: 4 unimproved_grain each, 8 total"]`; cash `Cash: 100 silver before; 100 after sales; 85 after purchases;`, proportion `proportion 1`, not short; trade rows exactly, in good ID order, `unskilled_labor 0/0/7/7`, `unimproved_grain 0/0/4/4`, `tools 0/0/0.5/0.5` (cells offered/sold/bid/bought); one factory row: number 0, `farm`, condition `1`/`1`, `8 turns`, run `1`/`1`, output `12 grain`, wear `0.125`, maintenance `0.125`/`0.125`, inputs `Inputs: 6 unskilled_labor, 12 unimproved_grain`, cost `Maintenance cost: 1 unskilled_labor, 0.5 tools`, capital `Capital: 4 tools`; stockpile `good`s `["12 grain", "85 silver"]`, the silver part `["85: never perishes"]`. Mine: deposits `["copper_ore × 1: 2.5 unimproved_copper each, 2.5 total"]`; cash 50/50/37; bought `unskilled_labor` 4 and `tools` 0.75; factory 1 `mine`, condition `0.75`/`1`, `8 turns`, output `2.5 copper`, wear `0.125`, maintenance `0.375`/`0.375`. Toolmaker: deposits empty; cash 200/200/152.25; bought copper 8, unskilled_labor 6.25, tools 0.125; factory 2 `toolmaker`, condition `1`/`1`, `24 turns`, output `4 tools`, wear and maintenance equal to the record's values' `Display`. | AC 1, 2, 3 |
| `the_sample_sections_after_one_step`: step `sample_python()` once: farm's trade row `grain` offered `12` sold `12`; mine's `copper` `2.5`/`2.5`; toolmaker's `tools` `3.875`/`3.875`; the cash lines end with `94 after purchases;`, `43.5 after purchases;`, `152.5 after purchases;`; headings `center_farm: 94 silver` etc. | AC 4 (Reload is the session's existing behavior) |
| `the_sample_sections_under_the_idle_host`: `sample_idle()`: every section's answer is `Answer: nothing (no decision was sent, or its decider failed)`, `trace` None, `trade` empty, headings `center_farm: 100 silver`, `center_mine: 50 silver`, `center_toolmaker: 200 silver`; cash lines `Cash: 100 silver before; 100 after sales; 100 after purchases;` with `proportion 1`; factory rows run `0`/`0`, output `0 grain` etc., maintenance `0`/`0`, inputs `Inputs: nothing`, cost `Maintenance cost: nothing`; wear and conditions after equal the records' `Display` (farm 0.125 → 0.875, life `7 turns`; mine → `5 turns`; toolmaker → `23 turns`); the farm's stockpile `good`s are `["8 unimproved_grain", "100 silver"]`. | AC 5 (decided nothing), AC 1 wording |
| `a_refused_decision_shows_its_errors_and_what_was_asked`: `trade_world` with the host from the engine's `a_refused_decision_idles_its_business` (steady, plus factory 99 for `poor`). `poor`'s section: `refused` true, answer `Answer: refused, 1 error`, `errors` equal to the stored `DecisionErrors` iterated with `to_string()` and the one line starts with `factory #99: `; trade rows `labor 0/0/10/0` and `tools 0/0/4/0`; factory row 2 run `0`/`0`, maintenance `0`/`0`, wear `0.25`, condition `1`/`0.75`, life `3 turns`; trace None (the engine helper sends none). `business_rows` gives `poor` answer `refused`, `refused` true; `works` `valid`. | AC 5 (refused), AC 9 (refused row) |
| `a_cash_short_decision_shows_the_proportion_and_the_cut_run`: `trade_world(rule(steady))`: `poor`'s cash `Cash: 10 coin before; 10 after sales; 0 after purchases;`, proportion `proportion 0.2`, `cash_short` true; trade rows `labor 0/0/10/2`, `tools 0/0/4/0.8`; factory row run `1`/`0.2`, output `2 iron`, inputs `Inputs: 2 labor, 4 ore`. `works`'s `cash_short` false, `proportion 1`. | AC 5 (cash-short) |
| `business_deposit_lines_and_stockpile_lines`: on `sample_idle()`: farm's deposit lines as above, mine's, toolmaker's empty; `business_stockpile_lines` for the mine equals `[StockLine { good: "2.5 unimproved_copper", parts: ["2.5: perishes this turn"] }, StockLine { good: "50 silver", parts: ["50: never perishes"] }]`. | AC 1, 2 |
| `good_rows_in_businesses` (in `text/mod.rs`'s tests, extending `good_rows_list_every_good_with_properties_and_world_totals`): on `sample_idle()` at turn 0 `in_businesses` is `["0", "2.5", "8", "0", "0", "350", "0"]`; after the three steps it equals the per-good sums over `world.businesses().iter()` of `business_stockpile(id).total(good)` formatted. In a new test on `sample_python()`: `grain` "12", `copper` "2.5", and `silver` and `tools` equal the same sums. On `turns_valid` and `valid_minimal` every `in_businesses` is "0". | AC 7, AC 11 |
| `technology_rows_on_the_sample`: exactly three rows: `farm` recipe `["0.5 unskilled_labor per unit", "1 unimproved_grain per unit"]`, output `grain`, capacity `12`, capital `["4 tools"]`, construction `["8 unskilled_labor"]`, `2`, `8`, factories 1; `mine` `["1 unskilled_labor per unit", "1 unimproved_copper per unit"]`, `copper`, `2.5`, `["2 tools"]`, `["4 unskilled_labor"]`, `2`, `8`, 1; `toolmaker` `["1.5 unskilled_labor per unit", "2 copper per unit"]`, `tools`, `4`, `["3 tools"]`, `["6 unskilled_labor"]`, `3`, `24`, 1. On `business_trade`: `bakery`'s construction is `["nothing"]` and `smelter` has 2 factories. On `valid_minimal`: empty. | AC 8, AC 11 |
| `business_rows_on_the_sample`: `sample_python()`: three rows in order, hex `(0, 0)`, decider `arithmetic`, cash `85 silver`, `37 silver`, `152.25 silver`, factories 1, answer `valid`, `refused` false. `sample_idle()`: cash `100 silver` etc., answer `nothing`. `valid_minimal`: empty. | AC 8, AC 11 |
| Engine: `slices_held` unit test (Changes). | AC 12 |

Run: `cargo test -p policyspace-dev-ui` (the Python tests need the project's `.venv`, as the existing session test does), `cargo test -p policyspace-engine`, then `.venv/Scripts/python tools/check.py`. The existing `tests/checker.rs` and `tests/cli.rs` must pass unchanged.

Checked by hand at Code Review (AC 6, 9's color, 10, and the layout): `cargo run -p policyspace-dev-ui -- settings/sample`, select (0, 0) and (2, −2), step, reload, look at the marks and the tables.

## Constraints
- Follow the dev UI's conventions: no egui types in `text` or `test_support`; every list in ID order from the engine's ordered accessors; colors only from `ui.visuals().error_fg_color`, never a hard-coded `Color32`; explicit id salts from the business index on every repeated header and grid; `#[cfg(test)]` on test-only code (not `#[allow(dead_code)]`); doc comments on every public item (clippy runs with `-D warnings`).
- Don't change `session.rs`, `cli.rs`, `check.rs`, `geometry.rs`, `main.rs`, `turn_line`, any setting, the Python package, or any engine file other than `production.rs` and `lib.rs` as described. Existing tests keep passing without edits except the `good_rows` tests named above.
- Don't round any number; don't add format helpers.
- Keep `stock_lines`, `list` and `deposit_lines_from` private to `text` (the child module reaches them as `super::` items).
- Out of scope: taking over a business or submitting decisions, showing the served view, history or charts, printing the decision counts in the turn log, any other map mark or hex coloring, tests that drive the window.
- Don't edit `docs/` or `manual.ipynb`; `step-docs-draft` does that from this spec.

## Done When
All tasks complete, every test in the Test Plan passes, `.venv/Scripts/python tools/check.py` passes, the window opens on `settings/sample` and shows the new sections and mark, and every Acceptance Criterion in `design.md` is met.
