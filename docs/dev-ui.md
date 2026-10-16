# The developer UI

`policyspace-dev-ui` (binary `dev-ui`, in `crates/dev-ui/`) is a tool for developers and setting authors, not a player UI. It depends on the engine by path and uses only the engine's public interface (`load_setting`, `LoadedSetting`, `LoadErrors`, `World` with its `step`, `market`, `hex_currency`, `pops`, `pop_stockpile` and `pop_outcome`, `TurnReport`, `StepError`, `Stockpile`, `PerishesAt`, `Perishes`, `Quantity`, `Turn`, `HexMap`, `HexCoord`, `HexId`, `PopId`, `ENGINE_NAME`, `ENGINE_VERSION`). The engine knows nothing about it. The window is built with egui/eframe 0.36 (default features, which use the wgpu renderer), and the system folder picker comes from rfd 0.17. These are dependencies of this crate only.

## Commands

| Arguments | Behavior | Exit code |
|---|---|---|
| none | opens the window with no setting loaded | 0 when the window closes; 1 if it can't be created |
| `<SETTING_FOLDER>` | opens the window with that setting loaded (load errors are shown in the window) | as above |
| `check <SETTING_FOLDER> [--turns N]` | the headless setting checker; never opens a window (see [setting-loader.md](setting-loader.md) and [world.md](world.md)) | 0, 1 or 2 as described there |
| `--version` | prints `{ENGINE_NAME} {ENGINE_VERSION}` | 0 |
| anything else | prints `USAGE` to stderr | 2 |

`USAGE` is:

```
usage: dev-ui [SETTING_FOLDER]
       dev-ui check SETTING_FOLDER [--turns N]
       dev-ui --version
```

A folder argument that is the word `check` or starts with `-` is not a folder, so `dev-ui check`, `dev-ui --bogus` and `dev-ui check --turns 2` are usage errors. A folder literally named `check` can be passed as `./check`.

Do not add `#![windows_subsystem = "windows"]` to `main.rs`: on Windows it detaches the program from the console, and `check` and `--version` would print nothing.

## Module structure (`crates/dev-ui/src/`)

| Module | Purpose |
|---|---|
| `main.rs` | Thin dispatcher: collects `std::env::args_os()`, calls `parse_args`, and runs the chosen `Command`. Holds the crate docs. |
| `cli.rs` | `Command` (`Window { folder }`, `Check { path, turns }`, `Version`), `parse_args`, `parse_turns`, `USAGE`, `MAX_TURNS` (10,000). |
| `check.rs` | The headless checker only: `run` and `run_turns`. Its wording comes from `text`. |
| `text.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `count`, `turn_line`) and the window's panels (countdowns, deposit lines, stockpile lines, market lines, POP sections, goods, resource-type and POP-type rows, and the index, rate and share formatting), built from engine types. No egui types. |
| `geometry.rs` | Pure map math: hex to world to screen, fitting, zooming, culling, and finding the hex under a point. No egui types. |
| `session.rs` | `Session` and `Loaded`: the current folder, what loading it gave, the turn log, and the selected hex. Steps the world. No egui types. |
| `app.rs` | The eframe window: `run`, the panels, keyboard shortcuts, and the map canvas. Uses `geometry`, `session` and `text`. |
| `test_support.rs` | `#[cfg(test)]` helpers shared by the unit tests: `repo_path(relative)` (a path from the repository root) and `world_at(relative)` (loads a setting and returns its world at turn 0, panicking with the errors). |

Everything except `app.rs` is unit-tested without a window. The window is checked by hand.

## Loading and stepping: `Session`

`Session` holds:

- `folder: Option<PathBuf>`;
- `loaded: Option<Loaded>`;
- `log: Vec<String>`: every turn line since the last load, oldest first;
- `step_error: Option<String>`: the error from the last refused step since the last load;
- `selected: Option<HexCoord>`: the selected hex.

`Loaded` is `World(World)` (the setting loaded; the world is at turn 0 after a load and later after steps) or `Errors(Vec<String>)`. `Session::world()` returns the loaded world, or None when nothing is loaded or the setting has errors; use it wherever only the world case matters.

**Loading.**

- `Session::open(folder)` makes `folder` current and loads it, replacing whatever was loaded. `reload()` loads the current folder again from disk, and does nothing if no folder was chosen.
- Loading calls `load_setting`. On success it builds `World::new(setting)` (at turn 0, with turn 0's trade round and growth already run), and the log becomes exactly `[text::turn_line(&world, &report)]` for turn 0's `TurnReport`. The result is `Loaded::World(world)`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `text::error_lines(&errors, &folder)`, and the log is emptied.
- Either way `step_error` is cleared: only a load clears it.
- A failed load replaces the previous world, so nothing from an earlier load stays on screen. A folder that fails to load is still the current folder, so Reload retries it.
- Loading is synchronous on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds.

`text::error_lines` returns each error's `Display` in order, then `{n} error(s) found in {folder}`. `check::run` prints these lines to stderr and the window shows the same lines, so both always agree.

**Stepping.** `step(times)` calls `World::step` up to `times` times and pushes `text::turn_line` of each report onto the log. It stops at the first refused step and stores that `StepError`'s `Display` in `step_error`. It does nothing without a world and returns the number of steps taken. `STEP_MANY` (10) is the count the Step ×10 button and Shift+Space use. The engine offers no way to put a world at the maximum turn, so the refused-step path can't be reached from the dev UI and is not tested. The log stores formatted lines rather than reports, so it matches `check --turns` word for word and the window needs no formatting. It is never trimmed: it grows by one line per step until the next load.

**Selection.** The selection is stored as a `HexCoord`, never a `HexId`, because a `HexId` is an index into one map and may name a different hex (or none) after a reload, and `Deposits::on_hex`, `Pops::on_hex` and `World::hex_stockpile` panic on an out-of-range id.

- `select(Some(hex))` stores `map.coord(hex)`; `select(None)` clears. Without a world, `Some` is ignored.
- `selected()` returns `map.id(coord)` for the current world, so it is None when nothing is selected, nothing is loaded, or the coordinates aren't on the map.
- `open` always clears the selection.
- `reload` keeps it, unless the load succeeded and the new map has no hex at those coordinates (then it is cleared). A failed load keeps it, so the next successful reload selects the hex again if it still exists. This mirrors how Reload keeps pan and zoom.

## Map math: `geometry`

All map math is in `f64`. One world unit is a hex's circumradius, so a view's `scale` (screen pixels per world unit) is a hex's on-screen radius in pixels. `f32` (egui's type) can't hold hex centers at the `i32` extremes (about ±5.6 × 10^9 world units), so positions are converted to `f32` only after culling, when they are near the canvas.

Types:

- `Point { x, y }`: a point in world units or screen pixels.
- `Viewport { min, width, height }`: the canvas rectangle on screen.
- `View { center, scale }`: `center` is the world point drawn at the viewport's center. The app keeps a `View` for the loaded map.
- `HexShape`: `Dot`, `Hexagon` or `LabelledHexagon`, chosen by `View::hex_shape` from the scale.

Layout is the pointy-top orientation fixed in [map.md](map.md): `hex_center(coord)` is `x = √3 · (q + r/2)`, `y = 1.5 · r`, computed in `f64` (integer arithmetic would overflow). `hex_containing(point)` is its inverse, using cube rounding, and returns None if the coordinates don't fit in `i32`. `hex_corners(center, radius)` gives the six corners clockwise from the top-right one.

`View` operations:

- `View::fit(coords, viewport)` fits the bounding box of every hex inside the viewport, with a 10% margin (`FIT_FILL`), clamped to `MIN_SCALE..=FIT_MAX_SCALE`. An empty map gives center (0, 0) and `DEFAULT_SCALE`. A viewport smaller than 1 pixel counts as 1, so the result is always finite and positive. A map with coordinates at the `i32` extremes fits at a scale around 10^-7.
- `world_to_screen` and `screen_to_world` are inverses.
- `pan(dx, dy)` moves the map by screen pixels.
- `zoom_at(screen, factor, viewport)` multiplies the scale (clamped to `MIN_SCALE..=MAX_SCALE`) while keeping the world point under `screen` fixed.
- `hex_shape()` is `Dot` below `DOT_BELOW_PX` (3 px), `LabelledHexagon` from `LABEL_FROM_PX` (22 px), and `Hexagon` between.

Free functions:

- `is_visible(view, viewport, center)` is the culling test: the center is within the viewport expanded by the drawn radius (`scale` for hexagons, `DOT_RADIUS_PX` for dots).
- `hex_at(map, view, viewport, screen)` finds the hovered hex. First it converts the point to a hex with `hex_containing` and looks it up with `HexMap::id`. If that finds nothing and hexes are dots, it takes the hex whose screen center is nearest within `HOVER_SNAP_PX`, with ties going to the lower `HexId` (a linear scan over `map.iter()`). Otherwise it returns None.

The tuning constants (`DOT_BELOW_PX`, `DOT_RADIUS_PX`, `LABEL_FROM_PX`, `HOVER_SNAP_PX`, `FIT_MAX_SCALE`, `FIT_FILL`, `MIN_SCALE`, `MAX_SCALE`, `DEFAULT_SCALE`) are public constants at the top of `geometry.rs`.

## Panel text: `text`

`text` builds every string the dev UI shows, without egui types, so it is unit-tested without a window. The `check` command and the window share its load-error, count and turn-line functions, so they always use the same wording. Every list comes out in ID order (goods, resource types, deposits, prices, the POPs on a hex, the goods in a POP's table, POP types), straight from the engine's ordered accessors, so the panels are deterministic.

**Numbers.** Sizes and amounts are written in full with `Quantity`'s `Display` (the f64 `Display`), as in `check --turns`; nothing is rounded. Only three ratios are rounded for display, by three public helpers:

- `format_index(index)`: the subsistence index to 4 fixed decimals: `0.6667`, `1.0000`, `0.0000`, `-1.0000`.
- `format_rate(rate)`: the growth rate as a signed percentage to 2 fixed decimals: `+2.67%`, `-10.00%`, `+0.00%`.
- `format_share(share)`: a normalized utility share rounded to 4 decimals, written with f64 `Display` so trailing zeros drop: `0.8`, `0.1`, `0.3333`, `1`, `0`.

Each of them, and the supernumerary income in a POP's demand line, adds `0.0` to the value before formatting: that turns `-0.0` into `0.0`, which would otherwise print as `-0.0000`, `-0.00%` or `-0`. Negative numbers use the ASCII `-`, like coordinates and turn lines. Where the rounded value hides detail, the window shows the full f64 value as hover text (the growth line and each utility line).

Shared with `check`:

- `error_lines(errors, folder)`: each error's `Display`, then `{n} error(s) found in {folder}`.
- `count(n, singular, plural)`: `1 hex`, `19 hexes`.
- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}; sold {list}; bought {list}; consumed {list}; population {n}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn, `sold`, `bought` and `consumed` are the POPs' totals from the turn's trade round, and `population` is the total POP size after growth, printed with `Quantity`'s `Display` (`0` for a world without POPs; see [world.md](world.md), [trade.md](trade.md) and [growth.md](growth.md)).

For the panels:

- `countdown(PerishesAt, now: Turn)`: for `EndOfTurn(t)` with `n = t - now` (saturating), "perishes this turn" (0), "perishes in 1 turn", or "perishes in {n} turns"; `Never` gives "never perishes".
- `lasts(Perishes)`: "perishes after 1 turn", "perishes after {n} turns", or "never perishes".
- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add` rather than multiplied, so it matches the hex stockpile's total, which emission builds by adding each deposit's output in turn.
- `stockpile_lines(world, hex)` and `pop_stockpile_lines(world, pop)`: one `StockLine { good, parts }` per good held, in good ID order. `good` is `{total} {good}`; `parts` has one `{amount}: {countdown}` per perish group, soonest first. Both wrap the private `stock_lines(world, &Stockpile)`, so a POP's stockpile is written exactly like a hex's. Use `stock_lines` for any future stockpile (an industry's, a business's).
- `good_rows(world)`: one `GoodRow { name, lasts, movable, in_hexes, in_pops }` per good (`movable` is "movable" or "fixed"). `in_hexes` sums the good over every hex stockpile (the unowned output of deposits) and `in_pops` over every POP stockpile (owned goods); they are kept apart because they are different things. Each is computed in one pass (over `world.map().iter()`, then over `world.pops().iter()`), adding each stockpile's totals into a per-good array with `Quantity::saturating_add`. A world without POPs has `"0"` in every `in_pops`.
- `resource_rows(world)`: one `ResourceRow { name, produces, output, deposits }` per resource type; deposits are counted in one pass over `world.deposits().iter()`.
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

## The window: `app`

`app::run(folder)` starts `eframe::run_native` with a 1280 × 800 window. If the window can't be created it prints `error: could not open the window: {error}` and returns exit code 1. `DevApp` holds `session`, `view: Option<View>` (None means "fit on the next draw", because the canvas size is only known while drawing), `hovered: Option<HexId>`, and `last_title` (so the title command is sent only when the title changes). It holds no selection state: that lives in `Session`. The view lives in the app, not the session, so Reload never touches it: after a successful reload, even with a failed load in between, the map stays at the same pan and zoom. Open and Fit set `view` to None; Open and Reload reset `hovered`.

Each frame (`eframe::App::ui`) runs in this order:

1. **Shortcuts** (`DevApp::shortcuts`), only when a world is loaded, before anything is drawn. Inside one `ui.input_mut`, it consumes `Shift+Space`, then `Space`, then `Esc`. Both Space keys are consumed every frame, even when the first matched: egui presses a focused button on Space, so a Space left in the queue would press a focused Step button as well as trigger the shortcut. Shift+Space is consumed first because `consume_key` ignores extra Shift. Shift+Space steps `STEP_MANY` times, Space steps once, Esc clears the selection. Because a held key repeats, holding Space keeps stepping.
2. **Top panel:** **Open setting…** (rfd's `pick_folder`; cancelling changes nothing), **Reload** (enabled when a folder is chosen), **Fit**, then **Step** (hover text "Space") and **Step ×10** (hover text "Shift+Space"), and the folder path. Step and Step ×10 are enabled whenever a world is loaded, including one with no hexes. Fit needs a world with at least one hex.
3. **Status bar:** with a world, `{name} · {n} hexes · turn {turn}` plus ` · hex (q, r)` while a hex is hovered (using `text::count` for the wording); with errors, `{n} error(s)`; otherwise `No setting loaded`. The turn changes as the world steps.
4. **Side panels** (only when a world is loaded, and shown before the central panel): `Panel::left("world")` and `Panel::right("hex")`, both resizable with a minimum of about 200 px (`SIDE_PANEL_MIN_WIDTH`). The world panel starts at about 320 px (`SIDE_PANEL_WIDTH`) and the hex panel at about 460 px (`HEX_PANEL_WIDTH`), wide enough for a POP's seven-column goods table at the sample's numbers.
5. **Central panel**, by state: no folder shows "No setting loaded." with an Open button; errors show a heading and every line in selectable monospace text inside a scroll area; a world with no hexes shows "This setting has no hexes."; otherwise `draw_map`. A click result from `draw_map` is applied with `session.select(..)` after the panel closure, because the world is borrowed from the session while drawing. The status bar and central panel match on `session.loaded()` because they also handle the errors and nothing-loaded cases; the other checks use `session.world()`.
6. **Title:** `{ENGINE_NAME} {ENGINE_VERSION} — Dev UI`, with ` — {setting name}` once a world is loaded.

**World panel (left)**, in one vertical `ScrollArea`, four collapsible sections (all open by default):

- *Turn log*: the refused-step error first, if any (in the theme's `error_fg_color`), then the log newest first in an inner `ScrollArea::both()` (id salt "turn log", max height 320 px) drawn with `show_rows` at one body-text line per row. Rows are non-wrapping labels, so every row has the same height and thousands of lines stay smooth.
- *Goods*: a striped `Grid` (Name, Lasts, Movable, In hexes, In POPs) from `text::good_rows`.
- *Resource types*: a striped `Grid` (Name, Produces, Output per deposit, Deposits) from `text::resource_rows`.
- *POP types*: a striped `Grid` (Name, Endowment, Utility, Growth, POPs, Total size) from `text::pop_type_rows`. The Endowment and Utility cells are `ui.vertical` stacks with one label per line; each utility label has the full share as hover text.

**Hex panel (right)**, in a `ScrollArea::both()`, so a wide goods table scrolls sideways instead of wrapping: for a world with no hexes, "This setting has no hexes."; with no selection, "Click a hex to inspect it."; otherwise, in order:

- the heading `Hex (q, r)`;
- "Deposits": `text::deposit_lines`, or "No deposits.";
- "Stockpile": the hex's stockpile;
- "Market": one label per line of `text::market_lines`;
- "POPs": "No POPs." if `world.pops().on_hex(hex)` is empty; otherwise one `CollapsingHeader` per POP, in POP ID order, open by default, headed by `PopSection::heading`. Inside it: the growth line (with `growth_hover` as hover text), the demand line, a striped seven-column `Grid` (Good, Held, Demanded, Subsistence, Sold, Bought, Consumed; id salt `("pop goods", pop index)`), the carried-over line, and a "Stockpile" subheading with the POP's stockpile. A row with `below_subsistence` draws all seven cells with `ui.colored_label` in the theme's `error_fg_color`, so a POP below subsistence and the good it lacks stand out; that is the only color the panel uses.

Each `CollapsingHeader` gets an explicit `id_salt(("pop", pop index))`. egui keys a header's open/closed state by its ID, which defaults to the heading text; the heading holds the size, which changes every step, so without the salt a collapsed section would reopen after each step.

Both stockpiles are drawn by the free function `stockpile_listing(ui, salt, lines)`: "Empty." for no lines, else each `StockLine.good` with its `parts` under `ui.indent(salt, ..)`. The hex passes the salt `"hex"` and a POP `("pop", pop index)`, so the indents of different stockpiles don't share an ID, and the two look identical.

The world between steps is after part 2 of the current turn (see [world.md](world.md)): every POP has been endowed, has traded, consumed and grown, and nothing has perished yet. So the POP sections show the outcome of the current turn's trade round (the turn in the status bar), each POP's size after that turn's growth, and its stockpile after consumption and before perishing. Both panels read the world directly each frame, so they update straight after a step or reload.

**`draw_map`** allocates the remaining area (click-and-drag sense), builds a `Viewport`, fits the view if it is None, and then:

1. Dragging pans. Scrolling over the canvas zooms toward the pointer by `exp(scroll_y · 0.0015)`, and egui's pinch zoom is applied too.
2. While the pointer is over the canvas and not dragging, `hovered = hex_at(...)`.
3. If the canvas was clicked and the pointer position is known, it returns `Some(hex_at(...))`: `Some(Some(hex))` selects (including dots within `HOVER_SNAP_PX`), `Some(None)` is a click on empty space and clears the selection. egui does not report a click for a drag, so panning never changes the selection.
4. It walks `map.iter()` once per frame, skips hexes that fail `is_visible`, and paints the rest with `paint_hex` by `hex_shape()`: a dot, a 1 px hexagon outline (`Shape::closed_line`), or an outline plus the label `q, r`. `paint_hex` takes the hex's screen center as a geometry `Point` (`f64`), computes the hexagon corners from it, and converts to egui's `f32` position only for painting. The hovered hex is filled with the theme's selection color. Colors come from the current egui visuals.
5. **Marks:** a visible hex drawn as `Hexagon` or `LabelledHexagon` can get two marks, in the foreground color, both of radius `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels (the cap keeps them inside the hexagon at small scales) and both off the label so they never cover the coordinates. A hex with at least one deposit (`HexLook::has_deposits`, from `!world.deposits().on_hex(id).is_empty()`) gets a filled circle at `center + (0, 0.55 · scale)`, below the label. A hex with at least one POP (`HexLook::has_pops`, from `!world.pops().on_hex(id).is_empty()`) gets a hollow circle (`circle_stroke`, 1 px) at `center − (0, 0.55 · scale)`, mirrored above the label. A hex can show both, either or neither. Dots get no marks. The map draws no other data: no coloring by population or anything else.
6. **Selected outline:** after the loop, in a second pass so neighbouring outlines don't paint over it, the selected hex is outlined in `visuals.selection.stroke.color` if it passes `is_visible`: a 3 px closed line through `hex_corners` for hexagons, or a 5 px-radius, 2 px circle stroke for dots. A hex that is hovered and selected keeps its hover fill with the outline on top.

One pass with culling is enough for maps of around 10,000 hexes; there is no caching.

## Extending the window

- **New panel text:** add a pure function to `text` that takes a `&World` and returns strings or small row structs, unit-test it, and render it in `world_panel_contents` or `hex_panel_contents` in `app.rs`.
- **New per-hex drawing:** add it to the loop in `draw_map` after the `is_visible` check, so it is only done for hexes on screen. Anything that must not be overpainted by neighbours (like the selection outline) goes in a pass after the loop.
- **New state tied to a world** (for example a log): put it in `Session`, reset it where loading happens, and decide whether `open` and `reload` keep it. Never store a `HexId` across a load; store the `HexCoord`.
- **New keyboard shortcuts:** consume them in `DevApp::shortcuts`, which runs before any widget is drawn, and consume every key a focused widget would also react to.
- Keep new map math in `geometry` (in `f64`, without egui types) so it can be unit-tested.
- **New hex-panel sections** (a later Stage's industries or businesses): add them after "POPs" in `hex_panel_contents`, built by a `text` function the way `pop_section` is. Give any repeated `CollapsingHeader` or `Grid` an explicit id salt from the item's ID, never from text that changes between steps. Reuse `stock_lines` (through a public wrapper) and `stockpile_listing` for any stockpile, so every stockpile reads the same. Use color only where it carries information, from the theme's visuals (`error_fg_color`), never a hard-coded `Color32`.
- **New map marks:** add a field to `HexLook`, fill it in `draw_map`, and paint it in `paint_hex`'s hexagon arm, using the shared offset and radius; the space below the label holds the deposit mark and the space above it the POP mark.

## Tests

- Unit tests in `cli.rs` (every accepted and rejected argument form), `text.rs`, `geometry.rs` (layout, round trips at the `i32` extremes, fitting, zoom and pan, shape thresholds, `hex_at` including dots, and culling), and `session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, the sample's exact turn 0 and turn 1 lines, selection kept or cleared across reload and open). They load settings through `test_support`. The `text.rs` tests cover:
  - `error_lines` for `many_errors`, countdown and `lasts` wording, and deposit and stockpile lines on the sample and `turns_valid`;
  - goods' `in_hexes` totals on the sample (at turn 0 and after steps) and `turns_valid`, and `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`;
  - resource-type deposit counts;
  - `market_lines` on the sample (hex (0, 0)'s prices, and `No market.` on (2, -2)) and on `markets_valid` (prices, `No prices posted.`, `No market.`);
  - `format_index`, `format_rate` and `format_share`, including `-0.0` and exact subsistence;
  - `pop_section` for the sample's center peasant and artisan and the no-market north-east peasant (heading, growth, demand, every goods row, the below-subsistence flag on its grain row, carried-over and stockpile), after one step (the starting size is the previous size), for `pops_growth`'s POP at exact subsistence (`index 0.0000; rate +0.00%`), and the growth hover's full values;
  - `pop_type_rows` on the sample (endowments, normalized shares, growth parameters, counts, and total sizes adding up to the turn report's population) and on `pops_valid` (an empty endowment reads `nothing`);
  - `pop_stockpile_lines` on `pops_placed` after one step, written exactly like a hex stockpile.

  Numbers that come from floating-point growth (sizes such as 61.6, the carried-over sliver) are compared against the engine's own value formatted with `Display` rather than a literal, so the tests check the wording and the source of the number, not the last digits.
- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets, POP types and POPs counts (`18 markets, 2 pop types, 16 pops` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists and its `population` (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0` for settings without POPs). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines (`consumed 35 labor; population 12.5`, with nothing sold or bought, since it has no markets, and the same population every turn, since its types don't grow). The sample's turn 0, 1 and 2 lines all differ, because its POPs grow, and are compared exactly as printed. It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules.
- `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.
- No test may run the binary with no arguments or with a single folder argument: those open a window and hang the run. The window forms are tested only through `parse_args`.
- The window itself is checked by hand: the layout, the hover texts, the error-colored rows, a collapsed POP section staying collapsed across a step, the hex panel's sideways scroll, and the map marks.
