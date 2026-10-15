# Step 08: dev-ui-pops — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-02-populate-the-world/step-08-dev-ui-pops/design.md` (the behavior, the exact wording, and the Acceptance Criteria).
- `docs/dev-ui.md` (the dev UI's modules and conventions; it describes the Stage 01 state of the code) and `docs/world.md`.
- `crates/dev-ui/src/text.rs` and `crates/dev-ui/src/app.rs` in full: the two files this Step changes. `crates/dev-ui/src/test_support.rs` for the test helpers.
- The engine's public interface this Step reads: `crates/engine/src/world.rs` (`World::market`, `hex_currency`, `pops`, `pop_stockpile`, `pop_outcome`), `crates/engine/src/trade.rs` (`PopOutcome`), `crates/engine/src/demand.rs` (`Demand`, `DemandedGood`), `crates/engine/src/pops.rs` (`PopType`, `Endowment`, `UtilityGood`, `GrowthParams`, `Pops`, `Pop`), `crates/engine/src/market.rs` (`Market`), `crates/engine/src/stockpile.rs` (`Stockpile`).
- `settings/sample/` (`rules/pops.toml`, `start/pops.toml`, `start/markets.toml`) and the engine test settings `crates/engine/tests/settings/pops_growth/` and `markets_valid/` (see `crates/engine/tests/settings/README.md`), which the tests use.

## Approach
- **Only the dev UI changes.** Every number the panels need is already public on `World`, `PopOutcome`, `Demand` and `PopType`. The engine crate, the `check` mode, `text::turn_line` and every setting stay as they are.
- **Text first, window second.** As in Stage 01, every string is built by pure functions in `text.rs` that take a `&World` and return strings or small row structs, and are unit-tested without a window. `app.rs` only lays them out. The window and the map mark are checked by hand at Code Review.
- **One stockpile formatter.** `stockpile_lines` is generalized to format any `&Stockpile`, so a POP's stockpile is written exactly like a hex's.
- **Information hierarchy, no new look.** The window keeps egui's default visuals. Inside a POP's section the heading carries what identifies the POP (type and size), two one-line summaries (growth, demand) come before the table, and the table carries the detail. Color is used only where it carries information: a goods row below subsistence in the theme's `error_fg_color`, like the refused-step error in the turn log. No other color, icon or decoration.
- **Per-POP sections keyed by POP ID.** A `CollapsingHeader`'s open/closed state is keyed by its ID; the heading text contains the size, which changes every step, so the sections get an explicit `id_salt` from the POP's ID. Otherwise a collapsed section would reopen after every step.
- **The seven-column goods table needs room.** The hex panel's scroll area scrolls both ways and the hex panel's default width rises to 460 px, so the sample's table fits at the default size and wider numbers scroll sideways instead of wrapping.
- **Numbers.** Sizes and amounts are written in full with `Quantity`'s `Display` (the f64 `Display`), as everywhere in the dev UI. Only three ratios are rounded for display: the subsistence index (4 fixed decimals), the growth rate (signed percentage, 2 fixed decimals), and normalized shares (rounded to 4 decimals, trailing zeros dropped). Negative numbers use the ordinary ASCII `-`, like coordinates and turn lines; the `−` in design.md is typography.
- **Determinism.** Every list comes out in ID order (prices, POPs on a hex, goods in a POP's table, POP types, goods rows), straight from the engine's ordered accessors.

## Changes

### `crates/dev-ui/src/text.rs`
- Imports: add `PopId`, `Stockpile` (and whatever else the new functions need) from `policyspace_engine`.
- Update the module doc: the panels now also show markets, POPs and POP types.
- Add `market_lines`, `format_index`, `format_rate`, `format_share`, `PopGoodRow`, `PopSection`, `pop_section`, `pop_stockpile_lines`, `UtilityLine`, `PopTypeRow`, `pop_type_rows`.
- Generalize `stockpile_lines` through a private `stock_lines(world, &Stockpile)`.
- `GoodRow`: replace `held` with `in_hexes` and `in_pops`; `good_rows` fills both.
- Tests: update the `good_rows` tests for the two columns; add the tests in the Test Plan.

### `crates/dev-ui/src/app.rs`
- Imports: the new `text` items; `Stroke` and `egui::vec2` are already there.
- Module doc: mention the market, POPs and POP types.
- Constant `HEX_PANEL_WIDTH: f32 = 460.0` (the hex panel's default width); the world panel keeps `SIDE_PANEL_WIDTH`.
- `side_panels`: the hex panel uses `HEX_PANEL_WIDTH` and `ScrollArea::both()`.
- `world_panel_contents`: the Goods grid gets five columns (Name, Lasts, Movable, In hexes, In POPs); a new "POP types" `CollapsingHeader` after "Resource types".
- `hex_panel_contents`: after the Stockpile section, a "Market" section and a "POPs" section. Factor the stockpile listing (good line plus indented parts, or "Empty.") into a helper used by both the hex stockpile and each POP's stockpile.
- `HexLook`: add `has_pops: bool`; `draw_map` fills it from `!world.pops().on_hex(id).is_empty()`; `paint_hex` draws the POP mark.

No other file changes. `Cargo.toml` is unchanged (no new dependencies).

## Interfaces & Data

All in `crates/dev-ui/src/text.rs`, public to the crate, no egui types. Names below are the engine's good and POP type names; `{currency}` is the name of the hex's currency good. Amounts are `Quantity` `Display`.

### Market
```rust
/// The Market section of `hex`: `"Currency: {currency}"`, then one line per posted price in
/// good ID order, `"{good}: {price} {currency}"` (the currency itself is never posted, so it
/// never appears); or, after the currency line, `"No prices posted."` for a market with no
/// prices, or `"No market."` for a hex with no market. Never empty.
pub fn market_lines(world: &World, hex: HexId) -> Vec<String>
```
Use `world.market(hex)`: `currency()`, `has_market()`, `prices()`.

### Number formatting
```rust
/// The subsistence index to 4 fixed decimals: "0.6667", "1.0000", "0.0000", "-1.0000".
pub fn format_index(index: f64) -> String
/// The growth rate as a signed percentage to 2 fixed decimals: "+2.67%", "-10.00%", "+0.00%".
pub fn format_rate(rate: f64) -> String
/// A normalized share rounded to 4 decimals with trailing zeros dropped: "0.8", "0.1",
/// "0.3333", "1", "0".
pub fn format_share(share: f64) -> String
```
Pitfalls: a value of `-0.0` would print as `-0.0000` / `-0.00%`; add `0.0` to the value first (`x + 0.0` turns `-0.0` into `0.0`) before formatting. `format_rate` is `format!("{:+.2}%", rate * 100.0 + 0.0)`. `format_share` is `(share * 10_000.0).round() / 10_000.0` written with f64 `Display` (which already drops trailing zeros: `0.8`, `1`, `0.3333`).

### POP sections
```rust
/// One row of a POP's goods table. Every field is formatted; `below_subsistence` flags a row
/// whose demanded quantity falls short of its subsistence (drawn in the error color).
#[derive(Debug, Clone, PartialEq)]
pub struct PopGoodRow {
    pub good: String,
    pub held: String,         // DemandedGood.held
    pub demanded: String,     // DemandedGood.quantity
    pub subsistence: String,  // DemandedGood.subsistence (already the type's amount × starting size)
    pub sold: String,         // PopOutcome::sold_of(good), "0" when nothing
    pub bought: String,       // PopOutcome::bought_of(good)
    pub consumed: String,     // PopOutcome::consumed_of(good)
    pub below_subsistence: bool, // DemandedGood::below_subsistence()
}

/// Everything the hex panel shows for one POP.
#[derive(Debug, Clone, PartialEq)]
pub struct PopSection {
    /// "{type} × {current size}", e.g. "peasant × 61.6".
    pub heading: String,
    /// "Started at {starting size}; index {format_index}; rate {format_rate}".
    pub growth: String,
    /// The hover text of the growth line: "index {index}; rate {rate}" with both as full
    /// f64 `Display` (e.g. "index 0.6666666666666666; rate 0.02666666666666667").
    pub growth_hover: String,
    /// "Income {M} {currency}; subsistence cost {C} {currency}; supernumerary {M-C} {currency}".
    /// M and C are `Quantity`; the supernumerary is an f64 written with f64 `Display`
    /// (so 240.0 writes "240", and a negative value keeps its "-").
    pub demand: String,
    /// One row per good in the recorded demand (`Demand::goods()`, already in good ID order).
    pub goods: Vec<PopGoodRow>,
    /// "Carried over: {amount} {currency}".
    pub carried_over: String,
    /// `pop_stockpile_lines(world, pop)`.
    pub stockpile: Vec<StockLine>,
}

/// The section for `pop` from the current turn's outcome and the POP's current size.
pub fn pop_section(world: &World, pop: PopId) -> PopSection

/// The stockpile of `pop`, written exactly like a hex's (`stockpile_lines`).
pub fn pop_stockpile_lines(world: &World, pop: PopId) -> Vec<StockLine>
```
Sources: `world.pops().get(pop)` (`pop_type`, `size`, `hex`), `world.rules().pop_types.name(..)`, `world.pop_outcome(pop)` (`demand()`, `sold_of`, `bought_of`, `consumed_of`, `carried_over()`, `subsistence_index()`, `growth_rate()`, `starting_size()`), `world.hex_currency(pop.hex)` for the currency name. The app iterates `world.pops().on_hex(hex)` (POP ID order) and calls `pop_section` for each.

`stockpile_lines(world, hex)` keeps its signature; it and `pop_stockpile_lines` both call a private `fn stock_lines(world: &World, stockpile: &Stockpile) -> Vec<StockLine>` holding the current body.

### Goods table
```rust
pub struct GoodRow {
    pub name: String,
    pub lasts: String,
    pub movable: &'static str,
    pub in_hexes: String, // total over every hex stockpile ("0" if none)
    pub in_pops: String,  // total over every POP stockpile ("0" if none)
}
pub fn good_rows(world: &World) -> Vec<GoodRow>
```
`in_pops` is summed the same way as `in_hexes`: one pass over `world.pops().iter()`, adding each `world.pop_stockpile(id)`'s totals into a per-good array with `saturating_add`.

### POP types table
```rust
/// One utility good of a POP type.
#[derive(Debug, Clone, PartialEq)]
pub struct UtilityLine {
    /// "{good}: subsistence {amount}, share {format_share}", e.g. "grain: subsistence 2, share 0.8".
    pub line: String,
    /// The hover text: the full normalized share with f64 `Display`, e.g. "0.8".
    pub share: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopTypeRow {
    pub name: String,
    /// One per endowed good, in good ID order: "{amount} {good}"; or exactly ["nothing"] for
    /// an empty endowment.
    pub endowment: Vec<String>,
    /// One per utility good, in good ID order.
    pub utility: Vec<UtilityLine>,
    /// "growth [{c1}, {c2}, …]; decline {d}; max {m}", every number with f64 `Display`,
    /// coefficients joined by ", ": "growth [0.06, -0.03]; decline 0.1; max 0.05".
    pub growth: String,
    /// The number of POPs of this type in the world.
    pub pops: usize,
    /// The sum of their current sizes (`saturating_add`), formatted; "0" with no POPs.
    pub total_size: String,
}

/// Every POP type, in POP type ID order.
pub fn pop_type_rows(world: &World) -> Vec<PopTypeRow>
```
Counts and total sizes come from one pass over `world.pops().iter()` into per-type arrays, like `resource_rows` counts deposits.

### Window (`app.rs`)
- **Hex panel**, after the Stockpile section, each preceded by `ui.add_space(4.0)`:
  - `ui.strong("Market")`, then one `ui.label` per line of `market_lines`.
  - `ui.strong("POPs")`. If `world.pops().on_hex(hex)` is empty: `ui.label("No POPs.")`. Else for each `pop` in it, `section = pop_section(world, pop)` and
    `CollapsingHeader::new(section.heading).id_salt(("pop", pop.index())).default_open(true).show(ui, |ui| { … })` containing, in order:
    1. `ui.label(section.growth).on_hover_text(section.growth_hover)`.
    2. `ui.label(section.demand)`.
    3. `Grid::new(("pop goods", pop.index())).num_columns(7).striped(true)` with the bold headings `Good`, `Held`, `Demanded`, `Subsistence`, `Sold`, `Bought`, `Consumed`, then one row per `PopGoodRow` in field order. A row with `below_subsistence` writes every one of its seven cells with `ui.colored_label(ui.visuals().error_fg_color, ..)`; other rows use `ui.label`.
    4. `ui.label(section.carried_over)`.
    5. `ui.strong("Stockpile")`, then the stockpile listing helper with `section.stockpile`.
  - The stockpile listing helper takes `ui`, an id salt (the hex panel passes `"hex"`, a POP section passes `("pop", pop.index())`) and the `Vec<StockLine>`: "Empty." when empty, else each `good` line with its `parts` under `ui.indent(salt, ..)`. The hex Stockpile section uses it too, so the two look identical.
- **World panel**: the Goods grid has `num_columns(5)` and headings `Name`, `Lasts`, `Movable`, `In hexes`, `In POPs`. After "Resource types", `CollapsingHeader::new("POP types").default_open(true)` with `Grid::new("pop types").num_columns(6).striped(true)`, headings `Name`, `Endowment`, `Utility`, `Growth`, `POPs`, `Total size`. The Endowment and Utility cells are `ui.vertical(|ui| ..)` with one `ui.label` per line; each utility label gets `.on_hover_text(line.share)`. POPs is `row.pops.to_string()`.
- **Side panels**: the right panel is `.default_size(HEX_PANEL_WIDTH)` and its contents are in `ScrollArea::both()`.
- **Map mark**: in `paint_hex`, compute the mark offset and radius once (the existing `0.55 · scale` offset and `clamp(0.12 · scale, 2, 6).min(0.35 · scale)` radius) for the `Hexagon | LabelledHexagon` arm. If `look.has_deposits`, the filled circle at `pos + (0, offset)` as now. If `look.has_pops`, `painter.circle_stroke(pos - egui::vec2(0.0, offset), radius, Stroke::new(1.0, look.foreground))`: a hollow circle of the same radius mirrored above the label. Dots get neither mark. Update the `draw_map` and `HexLook` docs.

## Tasks
1. In `text.rs`, refactor `stockpile_lines` into the private `stock_lines(world, &Stockpile)` plus the two public wrappers (`stockpile_lines`, `pop_stockpile_lines`). Existing tests still pass.
2. Add `format_index`, `format_rate`, `format_share` with the `-0.0` guard, and their tests.
3. Add `market_lines` and its tests (sample (0, 0) and (2, -2); `markets_valid` (0, 0), (2, 0) and (3, 0)).
4. Add `PopGoodRow`, `PopSection`, `pop_section`, and their tests on the sample's center peasant and artisan, the north-east peasant, and `pops_growth` POP 7.
5. Change `GoodRow` to `in_hexes` / `in_pops`, update `good_rows` and its tests, and add the sample test for the two columns.
6. Add `UtilityLine`, `PopTypeRow`, `pop_type_rows` and their tests (sample; `pops_valid` for the empty endowment).
7. In `app.rs`, add `HEX_PANEL_WIDTH`, switch the hex panel to it and to `ScrollArea::both()`.
8. Factor the stockpile listing helper out of `hex_panel_contents` and add the Market and POPs sections.
9. Update the Goods grid to five columns and add the POP types section to `world_panel_contents`.
10. Add `has_pops` to `HexLook`, fill it in `draw_map`, draw the hollow mark in `paint_hex`.
11. Update the module docs of `text.rs` and `app.rs`, and the doc comments of changed items.
12. Run the single check command, `.venv/Scripts/python tools/check.py` (see `docs/workspace.md`: format, clippy with `-D warnings`, the full test suite); fix anything it flags.

## Test Plan
All unit tests in `text.rs`'s `tests` module, loading settings with `test_support::world_at`. Where a value depends on floating-point arithmetic (grown sizes such as 61.6 and 10.3, the rounding sliver of carried-over silver), compare against the engine's own value formatted with `Display` (e.g. `format!("peasant × {}", world.pops().get(id).size)`) rather than a literal, so the test checks the wording and the source of the number, not the last digits. Exact integers (600, 168, 432, 216, 120) are compared as literals.

| Test | Acceptance Criterion |
|---|---|
| `market_lines` on sample (0, 0): `["Currency: silver", "unskilled_labor: 1 silver", "unimproved_copper: 1 silver", "unimproved_grain: 0.5 silver", "copper: 5 silver", "grain: 2 silver"]`; on (2, -2): `["Currency: silver", "No market."]` | hex (0, 0) market lines; hex (2, -2) `No market.` |
| `market_lines` on `markets_valid`: (0, 0) `["Currency: coin", "grain: 2 coin", "fish: 1.5 coin"]`, (2, 0) `["Currency: coin", "No prices posted."]`, (3, 0) `["Currency: coin", "No market."]` | `No prices posted.` |
| `format_index`: 2/3 → `0.6667`, 1 → `1.0000`, 0 → `0.0000`, -0.0 → `0.0000`, -1 → `-1.0000` | index formatting |
| `format_rate`: 0.026666… → `+2.67%`, 0.03 → `+3.00%`, -0.1 → `-10.00%`, 0 and -0.0 → `+0.00%` | rate formatting |
| `format_share`: 0.8 → `0.8`, 0.1 → `0.1`, 1/3 → `0.3333`, 1 → `1`, 0 → `0` | normalized shares |
| `pop_section` for the sample's center peasant (POP of type `peasant` on (0, 0), via `of_type_on_hex`): heading `peasant × {size}`; growth `Started at 60; index 0.6667; rate +2.67%`; demand `Income 600 silver; subsistence cost 360 silver; supernumerary 240 silver`; goods rows exactly `[unskilled_labor: held 600, demanded 168, subsistence 120, sold 432, bought 0, consumed 168, not below]`, `[grain: held 0, demanded 216, subsistence 120, sold 0, bought 216, consumed 216, not below]` (plus a `silver` row only if the outcome's demand lists one; assert the row set equals the demand's goods); `carried_over` starts with `Carried over: ` and ends with ` silver`, and `outcome.carried_over().get() < 1e-6`; stockpile is empty or has one line whose `parts` is `["{amount}: never perishes"]` | center peasant |
| `pop_section` for the sample's center artisan: heading `artisan × {size}`; growth `Started at 10; index 1.0000; rate +3.00%`; demand `Income 100 silver; subsistence cost 50 silver; supernumerary 50 silver`; rows: unskilled_labor sold 85, consumed 15; grain bought 30, consumed 30; copper bought 5, consumed 5, subsistence 0; none below | center artisan |
| `pop_section` for the north-east peasant (2, -2): heading `peasant × {size}` (size equals 12 × 0.9 to within 1e-9); growth `Started at 12; index -1.0000; rate -10.00%`; demand `Income 0 silver; subsistence cost 0 silver; supernumerary 0 silver`; unskilled_labor row held 120, demanded 120, consumed 120, sold 0, bought 0, not below; grain row demanded 0, subsistence 24, `below_subsistence == true`; carried over `Carried over: 0 silver`; stockpile empty | hex (2, -2); below-subsistence flag |
| `pop_section` after one `step()` on the sample: the north-east peasant's growth line reads `Started at {10.8…}` with the size the turn started with and the heading carries the new size (12 × 0.9 × 0.9 within 1e-9) | after Step |
| `pop_section` for `pops_growth` POP 7 (`PopId` index 7, the peasant on (1, 0)): growth `Started at 10; index 0.0000; rate +0.00%`, heading `peasant × 10` | exact subsistence |
| `growth_hover` of the center peasant equals `format!("index {}; rate {}", o.subsistence_index(), o.growth_rate())` and contains more digits than the growth line | hover shows full values |
| `good_rows` on the sample at turn 0 and after 3 steps: `in_hexes` = `["0", "10", "24", "0", "0", "0"]`, `in_pops` all `"0"` except silver, whose value equals the sum of every POP stockpile's silver total formatted | In hexes / In POPs |
| `good_rows` on `turns_valid` at turn 2: `in_hexes` matches the per-hex totals (existing test, renamed) and every `in_pops` is `"0"` | no POPs → In POPs empty |
| `pop_type_rows` on the sample: `peasant` (endowment `["10 unskilled_labor"]`, utility lines `unskilled_labor: subsistence 2, share 0.2` and `grain: subsistence 2, share 0.8`, growth `growth [0.06, -0.03]; decline 0.1; max 0.05`, pops 15, total_size = the formatted sum of the 15 peasants' sizes) and `artisan` (utility shares `0.1`, `0.4`, `0.5`, `copper: subsistence 0, share 0.5`, pops 1, total_size = the artisan's size formatted); the two total sizes added equal the turn-0 report's `population` within 1e-9 | POP types table |
| `pop_type_rows` on `pops_valid`: `idle` has endowment `["nothing"]` and pops 0, total_size `"0"` | empty endowment |
| `pop_stockpile_lines` on `pops_placed` after one `step()` (turn 1) for the hoarder (`PopId` index 1, size 2, endowed 4 labor and 1.5 gold per individual, no market, utility grain only): `[StockLine { good: "8 labor", parts: ["8: perishes this turn"] }, StockLine { good: "6 gold", parts: ["6: never perishes"] }]` (turn 0's labor perished, both turns' gold merged into one durable group); the existing hex `stockpile_lines` tests still pass unchanged | POP stockpile written like a hex's |

Run with `cargo test -p policyspace-dev-ui` and then the full workspace suite. The existing `crates/dev-ui/tests/checker.rs` and `cli.rs` must still pass unchanged (the turn line doesn't change).

Checked by hand at Code Review (not automated): the window layout, the hover texts, the error-colored rows, the collapsed state surviving a step, the hex panel's sideways scroll, and the map marks (filled below the label for deposits, hollow above it for POPs, none for dots).

## Constraints
- Do not change anything in `crates/engine/`, `settings/`, `check.rs`, `cli.rs`, `session.rs`, `geometry.rs`, or the integration tests in `crates/dev-ui/tests/`.
- Keep `text.rs` free of egui types and `app.rs` free of formatting logic beyond `to_string()` on counts.
- Follow the existing wording and conventions in `text.rs` and `app.rs`: doc comments on every public item, ID-order iteration, `saturating_add` for sums, `Quantity` `Display` for amounts, the theme's visuals for colors (no hard-coded `Color32` values).
- egui's defaults stay: no custom fonts, colors, spacing or styles; no coloring of hexes by any data; no mark beyond the POP mark.
- Out of scope (see design.md): any engine change, history over turns, charts, editing from the window, rounding of sizes or amounts, automated tests that drive the window.
- The single check command (`.venv/Scripts/python tools/check.py`) must stay clean: clippy runs with `-D warnings` under the workspace lints, so every public item needs a doc comment.

## Done When
All tasks complete, every test in the Test Plan passes along with the existing suite, the single check command passes, and every Acceptance Criterion in design.md is met (the hand-checked ones to be confirmed at Code Review).
