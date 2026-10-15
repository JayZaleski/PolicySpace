# Step 08: dev-ui-pops — Docs Changes

All changes are to `docs/dev-ui.md`. Only the dev UI changed in this Step (`crates/dev-ui/src/text.rs` and `crates/dev-ui/src/app.rs`). The engine, the `check` mode, `text::turn_line`, the integration tests and every setting are unchanged, so the turn-line and `checker.rs` texts drafted by Steps 04, 06 and 07 still apply as written.

## docs/dev-ui.md — modify
- **Location:** intro, the parenthesised list of engine items the dev UI uses
- **Change:** replace the list
- **Supersedes:** Step 02 (dummy-market), the same list

(`load_setting`, `World` and its `step`, `market`, `hex_currency`, `pops`, `pop_stockpile` and `pop_outcome`, `TurnReport`, `StepError`, `Stockpile`, `Deposits`, `Market`, `Pops`, `PopId`, `PopOutcome` with its `Demand`, `Rules` with its `PopType`s, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`)

## docs/dev-ui.md — modify
- **Location:** "Module structure", the `text.rs` row of the table
- **Change:** replace the row

| `text.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `count`, `turn_line`) and the window's panels (countdowns, deposit lines, stockpile lines, market lines, POP sections, goods, resource-type and POP-type rows, and the index, rate and share formatting), built from engine types. No egui types. |

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the opening paragraph
- **Change:** replace

`text` builds every string the dev UI shows, without egui types, so it is unit-tested without a window. The `check` command and the window share its load-error, count and turn-line functions, so they always use the same wording. Every list comes out in ID order (goods, resource types, deposits, prices, the POPs on a hex, the goods in a POP's table, POP types), straight from the engine's ordered accessors, so the panels are deterministic.

**Numbers.** Sizes and amounts are written in full with `Quantity`'s `Display` (the f64 `Display`), as in `check --turns`; nothing is rounded. Only three ratios are rounded for display, by three public helpers:

- `format_index(index)`: the subsistence index to 4 fixed decimals: `0.6667`, `1.0000`, `0.0000`, `-1.0000`.
- `format_rate(rate)`: the growth rate as a signed percentage to 2 fixed decimals: `+2.67%`, `-10.00%`, `+0.00%`.
- `format_share(share)`: a normalized utility share rounded to 4 decimals, written with f64 `Display` so trailing zeros drop: `0.8`, `0.1`, `0.3333`, `1`, `0`.

Each of them, and the supernumerary income in a POP's demand line, adds `0.0` to the value before formatting: that turns `-0.0` into `0.0`, which would otherwise print as `-0.0000`, `-0.00%` or `-0`. Negative numbers use the ASCII `-`, like coordinates and turn lines. Where the rounded value hides detail, the window shows the full f64 value as hover text (the growth line and each utility line).

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `stockpile_lines` bullet
- **Change:** replace

- `stockpile_lines(world, hex)` and `pop_stockpile_lines(world, pop)`: one `StockLine { good, parts }` per good held, in good ID order. `good` is `{total} {good}`; `parts` has one `{amount}: {countdown}` per perish group, soonest first. Both wrap the private `stock_lines(world, &Stockpile)`, so a POP's stockpile is written exactly like a hex's. Use `stock_lines` for any future stockpile (an industry's, a business's).

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", the `good_rows` bullet
- **Change:** replace
- **Supersedes:** Step 04 (pops), the same bullet

- `good_rows(world)`: one `GoodRow { name, lasts, movable, in_hexes, in_pops }` per good (`movable` is "movable" or "fixed"). `in_hexes` sums the good over every hex stockpile (the unowned output of deposits) and `in_pops` over every POP stockpile (owned goods); they are kept apart because they are different things. Each is computed in one pass (over `world.map().iter()`, then over `world.pops().iter()`), adding each stockpile's totals into a per-good array with `Quantity::saturating_add`. A world without POPs has `"0"` in every `in_pops`.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", new bullets after the `resource_rows` bullet
- **Change:** add

- `market_lines(world, hex)`: the hex panel's Market section, from `world.market(hex)`. The first line is always `Currency: {currency}` (every hex has a currency). Then one line per posted price in good ID order, `{good}: {price} {currency}` (`grain: 2 silver`); the currency itself is never posted, so it never appears. A hex whose market posts no prices gets `No prices posted.` after the currency line, and a hex with no market (`!has_market()`) gets `No market.`. Never empty.
- `pop_section(world, pop)`: a `PopSection` with everything the hex panel shows for one POP, from `world.pops().get(pop)` (type, current size, hex), `world.pop_outcome(pop)` (the current turn's `PopOutcome` and its `Demand`; see [trade.md](trade.md), [demand.md](demand.md) and [growth.md](growth.md)), and `world.hex_currency(hex)` for the currency name:
  - `heading`: `{type} × {current size}`, e.g. `peasant × 61.6`.
  - `growth`: `Started at {starting size}; index {format_index}; rate {format_rate}`, e.g. `Started at 60; index 0.6667; rate +2.67%`. The starting size is `PopOutcome::starting_size()`, the size the turn began with; the heading has the size after growth.
  - `growth_hover`: `index {index}; rate {rate}` with both in full f64 `Display`.
  - `demand`: `Income {M} {currency}; subsistence cost {C} {currency}; supernumerary {M−C} {currency}`. M and C are `Quantity`s; the supernumerary is the `Demand`'s f64, so a negative value keeps its sign.
  - `goods`: one `PopGoodRow { good, held, demanded, subsistence, sold, bought, consumed, below_subsistence }` per good in `Demand::goods()` (every utility good, every held good, and the currency when income was parked in it), already in good ID order. `held`, `demanded` (the quantity the POP wanted to end up with) and `subsistence` (the type's amount times the starting size, 0 outside the utility) come from the `DemandedGood`; `sold`, `bought` and `consumed` from `PopOutcome::sold_of`, `bought_of` and `consumed_of` (`0` when nothing happened). `below_subsistence` is `DemandedGood::below_subsistence()`.
  - `carried_over`: `Carried over: {amount} {currency}`, the currency left after consumption (`PopOutcome::carried_over()`).
  - `stockpile`: `pop_stockpile_lines(world, pop)`.
- `pop_type_rows(world)`: one `PopTypeRow { name, endowment, utility, growth, pops, total_size }` per POP type, in POP type ID order.
  - `endowment`: one `{amount} {good}` per endowed good in good ID order, or exactly `["nothing"]` for an empty endowment.
  - `utility`: one `UtilityLine { line, share }` per utility good in good ID order. `line` is `{good}: subsistence {amount}, share {format_share}` (`grain: subsistence 2, share 0.8`), using the engine's normalized shares (an artisan's weights 1, 4, 5 read `0.1`, `0.4`, `0.5`); `share` is the full normalized share, used as hover text.
  - `growth`: the parameters as the setting gave them, every number in f64 `Display`: `growth [0.06, -0.03]; decline 0.1; max 0.05`.
  - `pops` and `total_size`: the number of POPs of the type and the sum of their current sizes (`saturating_add`; `"0"` with no POPs), counted in one pass over `world.pops().iter()`, like `resource_rows` counts deposits. The total sizes over all types add up to the turn line's `population`.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", item 4 of the per-frame list (**Side panels**)
- **Change:** replace

4. **Side panels** (only when a world is loaded, and shown before the central panel): `Panel::left("world")` and `Panel::right("hex")`, both resizable with a minimum of about 200 px (`SIDE_PANEL_MIN_WIDTH`). The world panel starts at about 320 px (`SIDE_PANEL_WIDTH`) and the hex panel at about 460 px (`HEX_PANEL_WIDTH`), wide enough for a POP's seven-column goods table at the sample's numbers.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the **World panel (left)** paragraph and its bullets
- **Change:** replace

**World panel (left)**, in one vertical `ScrollArea`, four collapsible sections (all open by default):

- *Turn log*: the refused-step error first, if any (in the theme's `error_fg_color`), then the log newest first in an inner `ScrollArea::both()` (id salt "turn log", max height 320 px) drawn with `show_rows` at one body-text line per row. Rows are non-wrapping labels, so every row has the same height and thousands of lines stay smooth.
- *Goods*: a striped `Grid` (Name, Lasts, Movable, In hexes, In POPs) from `text::good_rows`.
- *Resource types*: a striped `Grid` (Name, Produces, Output per deposit, Deposits) from `text::resource_rows`.
- *POP types*: a striped `Grid` (Name, Endowment, Utility, Growth, POPs, Total size) from `text::pop_type_rows`. The Endowment and Utility cells are `ui.vertical` stacks with one label per line; each utility label has the full share as hover text.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the **Hex panel (right)** paragraph
- **Change:** replace

**Hex panel (right)**, in a `ScrollArea::both()`, so a wide goods table scrolls sideways instead of wrapping: for a world with no hexes, "This setting has no hexes."; with no selection, "Click a hex to inspect it."; otherwise, in order:

- the heading `Hex (q, r)`;
- "Deposits": `text::deposit_lines`, or "No deposits.";
- "Stockpile": the hex's stockpile;
- "Market": one label per line of `text::market_lines`;
- "POPs": "No POPs." if `world.pops().on_hex(hex)` is empty; otherwise one `CollapsingHeader` per POP, in POP ID order, open by default, headed by `PopSection::heading`. Inside it: the growth line (with `growth_hover` as hover text), the demand line, a striped seven-column `Grid` (Good, Held, Demanded, Subsistence, Sold, Bought, Consumed; id salt `("pop goods", pop index)`), the carried-over line, and a "Stockpile" subheading with the POP's stockpile. A row with `below_subsistence` draws all seven cells with `ui.colored_label` in the theme's `error_fg_color`, so a POP below subsistence and the good it lacks stand out; that is the only color the panel uses.

Each `CollapsingHeader` gets an explicit `id_salt(("pop", pop index))`. egui keys a header's open/closed state by its ID, which defaults to the heading text; the heading holds the size, which changes every step, so without the salt a collapsed section would reopen after each step.

Both stockpiles are drawn by the free function `stockpile_listing(ui, salt, lines)`: "Empty." for no lines, else each `StockLine.good` with its `parts` under `ui.indent(salt, ..)`. The hex passes the salt `"hex"` and a POP `("pop", pop index)`, so the indents of different stockpiles don't share an ID, and the two look identical.

The world between steps is after part 2 of the current turn (see [world.md](world.md)): every POP has been endowed, has traded, consumed and grown, and nothing has perished yet. So the POP sections show the outcome of the current turn's trade round (the turn in the status bar), each POP's size after that turn's growth, and its stockpile after consumption and before perishing. Both panels read the world directly each frame, so they update straight after a step or reload.

## docs/dev-ui.md — modify
- **Location:** "`draw_map`", item 5 (**Deposit mark**)
- **Change:** replace

5. **Marks:** a visible hex drawn as `Hexagon` or `LabelledHexagon` can get two marks, in the foreground color, both of radius `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels (the cap keeps them inside the hexagon at small scales) and both off the label so they never cover the coordinates. A hex with at least one deposit (`HexLook::has_deposits`, from `!world.deposits().on_hex(id).is_empty()`) gets a filled circle at `center + (0, 0.55 · scale)`, below the label. A hex with at least one POP (`HexLook::has_pops`, from `!world.pops().on_hex(id).is_empty()`) gets a hollow circle (`circle_stroke`, 1 px) at `center − (0, 0.55 · scale)`, mirrored above the label. A hex can show both, either or neither. Dots get no marks. The map draws no other data: no coloring by population or anything else.

## docs/dev-ui.md — modify
- **Location:** "Extending the window", the last bullet ("The Stage 02 additions ...")
- **Change:** replace

- **New hex-panel sections** (a later Stage's industries or businesses): add them after "POPs" in `hex_panel_contents`, built by a `text` function the way `pop_section` is. Give any repeated `CollapsingHeader` or `Grid` an explicit id salt from the item's ID, never from text that changes between steps. Reuse `stock_lines` (through a public wrapper) and `stockpile_listing` for any stockpile, so every stockpile reads the same. Use color only where it carries information, from the theme's visuals (`error_fg_color`), never a hard-coded `Color32`.
- **New map marks:** add a field to `HexLook`, fill it in `draw_map`, and paint it in `paint_hex`'s hexagon arm, using the shared offset and radius; the space below the label holds the deposit mark and the space above it the POP mark.

## docs/dev-ui.md — modify
- **Location:** "Tests", the first bullet (the unit tests)
- **Change:** replace

- Unit tests in `cli.rs` (every accepted and rejected argument form), `text.rs`, `geometry.rs` (layout, round trips at the `i32` extremes, fitting, zoom and pan, shape thresholds, `hex_at` including dots, and culling), and `session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, selection kept or cleared across reload and open). They load settings through `test_support`. The `text.rs` tests cover:
  - `error_lines` for `many_errors`, countdown and `lasts` wording, and deposit and stockpile lines on the sample and `turns_valid`;
  - goods' `in_hexes` totals on the sample (at turn 0 and after steps) and `turns_valid`, and `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`;
  - resource-type deposit counts;
  - `market_lines` on the sample (hex (0, 0)'s prices, and `No market.` on (2, -2)) and on `markets_valid` (prices, `No prices posted.`, `No market.`);
  - `format_index`, `format_rate` and `format_share`, including `-0.0` and exact subsistence;
  - `pop_section` for the sample's center peasant and artisan and the no-market north-east peasant (heading, growth, demand, every goods row, the below-subsistence flag on its grain row, carried-over and stockpile), after one step (the starting size is the previous size), for `pops_growth`'s POP at exact subsistence (`index 0.0000; rate +0.00%`), and the growth hover's full values;
  - `pop_type_rows` on the sample (endowments, normalized shares, growth parameters, counts, and total sizes adding up to the turn report's population) and on `pops_valid` (an empty endowment reads `nothing`);
  - `pop_stockpile_lines` on `pops_placed` after one step, written exactly like a hex stockpile.

  Numbers that come from floating-point growth (sizes such as 61.6, the carried-over sliver) are compared against the engine's own value formatted with `Display` rather than a literal, so the tests check the wording and the source of the number, not the last digits.

## docs/dev-ui.md — modify
- **Location:** "Tests", the last bullet ("The window itself is checked by hand.")
- **Change:** replace

- The window itself is checked by hand: the layout, the hover texts, the error-colored rows, a collapsed POP section staying collapsed across a step, the hex panel's sideways scroll, and the map marks.
