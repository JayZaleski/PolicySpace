# The developer UI

`policyspace-dev-ui` (binary `dev-ui`, in `crates/dev-ui/`) is a tool for developers and setting authors, not a player UI. It depends on the engine by path and uses only the engine's public interface (`load_setting`, `World` and its `step`, `TurnReport`, `StepError`, `Stockpile`, `Deposits`, `Rules`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`). The engine knows nothing about it. The window is built with egui/eframe 0.36 (default features, which use the wgpu renderer), and the system folder picker comes from rfd 0.17. These are dependencies of this crate only.

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
| `text.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `count`, `turn_line`) and the window's panels (countdowns, deposit lines, stockpile lines, goods and resource-type rows), built from engine types. No egui types. |
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
- Loading calls `load_setting`. On success it builds `World::new(setting)` (at turn 0), and the log becomes exactly `[text::turn_line(&world, &report)]` for turn 0's `TurnReport`. The result is `Loaded::World(world)`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `text::error_lines(&errors, &folder)`, and the log is emptied.
- Either way `step_error` is cleared: only a load clears it.
- A failed load replaces the previous world, so nothing from an earlier load stays on screen. A folder that fails to load is still the current folder, so Reload retries it.
- Loading is synchronous on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds.

`text::error_lines` returns each error's `Display` in order, then `{n} error(s) found in {folder}`. `check::run` prints these lines to stderr and the window shows the same lines, so both always agree.

**Stepping.** `step(times)` calls `World::step` up to `times` times and pushes `text::turn_line` of each report onto the log. It stops at the first refused step and stores that `StepError`'s `Display` in `step_error`. It does nothing without a world and returns the number of steps taken. `STEP_MANY` (10) is the count the Step ×10 button and Shift+Space use. The engine offers no way to put a world at the maximum turn, so the refused-step path can't be reached from the dev UI and is not tested. The log stores formatted lines rather than reports, so it matches `check --turns` word for word and the window needs no formatting. It is never trimmed: it grows by one line per step until the next load.

**Selection.** The selection is stored as a `HexCoord`, never a `HexId`, because a `HexId` is an index into one map and may name a different hex (or none) after a reload, and `Deposits::on_hex` and `World::hex_stockpile` panic on an out-of-range id.

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

`text` builds every string the dev UI shows, without egui types, so it is unit-tested without a window. The `check` command and the window share its load-error, count and turn-line functions, so they always use the same wording. Amounts use `Quantity`'s `Display`. Goods, resource types and deposits come out in ID order (the setting's definition order), so the panels are deterministic.

Shared with `check`:

- `error_lines(errors, folder)`: each error's `Display`, then `{n} error(s) found in {folder}`.
- `count(n, singular, plural)`: `1 hex`, `19 hexes`.
- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}`, where a list is `<amount> <good>` joined by `, ` or `nothing`.

For the panels:

- `countdown(PerishesAt, now: Turn)`: for `EndOfTurn(t)` with `n = t - now` (saturating), "perishes this turn" (0), "perishes in 1 turn", or "perishes in {n} turns"; `Never` gives "never perishes".
- `lasts(Perishes)`: "perishes after 1 turn", "perishes after {n} turns", or "never perishes".
- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add` (`Quantity` has no multiply).
- `stockpile_lines(world, hex)`: one `StockLine { good, parts }` per good held, in good ID order. `good` is `{total} {good}`; `parts` has one `{amount}: {countdown}` per perish group, soonest first.
- `good_rows(world)`: one `GoodRow { name, lasts, movable, held }` per good (`movable` is "movable" or "fixed"). `held` sums the good over every hex stockpile; it is computed in one pass over the hexes, adding each stockpile's totals into a per-good array.
- `resource_rows(world)`: one `ResourceRow { name, produces, output, deposits }` per resource type; deposits are counted in one pass over `world.deposits().iter()`.

## The window: `app`

`app::run(folder)` starts `eframe::run_native` with a 1280 × 800 window. If the window can't be created it prints `error: could not open the window: {error}` and returns exit code 1. `DevApp` holds `session`, `view: Option<View>` (None means "fit on the next draw", because the canvas size is only known while drawing), `hovered: Option<HexId>`, and `last_title` (so the title command is sent only when the title changes). It holds no selection state: that lives in `Session`. The view lives in the app, not the session, so Reload never touches it: after a successful reload, even with a failed load in between, the map stays at the same pan and zoom. Open and Fit set `view` to None; Open and Reload reset `hovered`.

Each frame (`eframe::App::ui`) runs in this order:

1. **Shortcuts** (`DevApp::shortcuts`), only when a world is loaded, before anything is drawn. Inside one `ui.input_mut`, it consumes `Shift+Space`, then `Space`, then `Esc`. Both Space keys are consumed every frame, even when the first matched: egui presses a focused button on Space, so a Space left in the queue would press a focused Step button as well as trigger the shortcut. Shift+Space is consumed first because `consume_key` ignores extra Shift. Shift+Space steps `STEP_MANY` times, Space steps once, Esc clears the selection. Because a held key repeats, holding Space keeps stepping.
2. **Top panel:** **Open setting…** (rfd's `pick_folder`; cancelling changes nothing), **Reload** (enabled when a folder is chosen), **Fit**, then **Step** (hover text "Space") and **Step ×10** (hover text "Shift+Space"), and the folder path. Step and Step ×10 are enabled whenever a world is loaded, including one with no hexes. Fit needs a world with at least one hex.
3. **Status bar:** with a world, `{name} · {n} hexes · turn {turn}` plus ` · hex (q, r)` while a hex is hovered (using `text::count` for the wording); with errors, `{n} error(s)`; otherwise `No setting loaded`. The turn changes as the world steps.
4. **Side panels** (only when a world is loaded, and shown before the central panel): `Panel::left("world")` and `Panel::right("hex")`, both resizable (default about 320 px, minimum about 200 px).
5. **Central panel**, by state: no folder shows "No setting loaded." with an Open button; errors show a heading and every line in selectable monospace text inside a scroll area; a world with no hexes shows "This setting has no hexes."; otherwise `draw_map`. A click result from `draw_map` is applied with `session.select(..)` after the panel closure, because the world is borrowed from the session while drawing. The status bar and central panel match on `session.loaded()` because they also handle the errors and nothing-loaded cases; the other checks use `session.world()`.
6. **Title:** `{ENGINE_NAME} {ENGINE_VERSION} — Dev UI`, with ` — {setting name}` once a world is loaded.

**World panel (left)**, in one vertical `ScrollArea`, three collapsible sections (all open by default):

- *Turn log*: the refused-step error first, if any (in the theme's `error_fg_color`), then the log newest first in an inner `ScrollArea::both()` (id salt "turn log", max height 320 px) drawn with `show_rows` at one body-text line per row. Rows are non-wrapping labels, so every row has the same height and thousands of lines stay smooth.
- *Goods*: a striped `Grid` (Name, Lasts, Movable, Held) from `text::good_rows`.
- *Resource types*: a striped `Grid` (Name, Produces, Output per deposit, Deposits) from `text::resource_rows`.

**Hex panel (right)**, in a vertical `ScrollArea`: for a world with no hexes, "This setting has no hexes."; with no selection, "Click a hex to inspect it."; otherwise the heading `Hex (q, r)`, a "Deposits" subheading with `text::deposit_lines` (or "No deposits."), and a "Stockpile" subheading listing each `StockLine.good` with its `parts` indented (or "Empty."). Both panels read the world directly each frame, so they update straight after a step or reload.

**`draw_map`** allocates the remaining area (click-and-drag sense), builds a `Viewport`, fits the view if it is None, and then:

1. Dragging pans. Scrolling over the canvas zooms toward the pointer by `exp(scroll_y · 0.0015)`, and egui's pinch zoom is applied too.
2. While the pointer is over the canvas and not dragging, `hovered = hex_at(...)`.
3. If the canvas was clicked and the pointer position is known, it returns `Some(hex_at(...))`: `Some(Some(hex))` selects (including dots within `HOVER_SNAP_PX`), `Some(None)` is a click on empty space and clears the selection. egui does not report a click for a drag, so panning never changes the selection.
4. It walks `map.iter()` once per frame, skips hexes that fail `is_visible`, and paints the rest with `paint_hex` by `hex_shape()`: a dot, a 1 px hexagon outline (`Shape::closed_line`), or an outline plus the label `q, r`. `paint_hex` takes the hex's screen center as a geometry `Point` (`f64`), computes the hexagon corners from it, and converts to egui's `f32` position only for painting. The hovered hex is filled with the theme's selection color. Colors come from the current egui visuals.
5. **Deposit mark:** a visible hex with at least one deposit (`!world.deposits().on_hex(id).is_empty()`) drawn as `Hexagon` or `LabelledHexagon` gets a filled circle in the foreground color at `center + (0, 0.55 · scale)`, below the label. Its radius is `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels; the cap keeps the mark inside the hexagon at small scales. Dots get no mark.
6. **Selected outline:** after the loop, in a second pass so neighbouring outlines don't paint over it, the selected hex is outlined in `visuals.selection.stroke.color` if it passes `is_visible`: a 3 px closed line through `hex_corners` for hexagons, or a 5 px-radius, 2 px circle stroke for dots. A hex that is hovered and selected keeps its hover fill with the outline on top.

One pass with culling is enough for maps of around 10,000 hexes; there is no caching.

## Extending the window

- **New panel text:** add a pure function to `text` that takes a `&World` and returns strings or small row structs, unit-test it, and render it in `world_panel_contents` or `hex_panel_contents` in `app.rs`.
- **New per-hex drawing:** add it to the loop in `draw_map` after the `is_visible` check, so it is only done for hexes on screen. Anything that must not be overpainted by neighbours (like the selection outline) goes in a pass after the loop.
- **New state tied to a world** (for example a log): put it in `Session`, reset it where loading happens, and decide whether `open` and `reload` keep it. Never store a `HexId` across a load; store the `HexCoord`.
- **New keyboard shortcuts:** consume them in `DevApp::shortcuts`, which runs before any widget is drawn, and consume every key a focused widget would also react to.
- Keep new map math in `geometry` (in `f64`, without egui types) so it can be unit-tested.
- The Stage 02 additions (owners, POPs and their stockpiles) belong in the hex panel and world panel the same way.

## Tests

- Unit tests in `cli.rs` (every accepted and rejected argument form), `text.rs` (`error_lines` for `many_errors`, countdown and `lasts` wording, deposit and stockpile lines on the sample and `turns_valid`, goods' world totals, resource-type deposit counts), `geometry.rs` (layout, round trips at the `i32` extremes, fitting, zoom and pan, shape thresholds, `hex_at` including dots, and culling), and `session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, selection kept or cleared across reload and open). They load settings through `test_support`.
- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules.
- `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.
- No test may run the binary with no arguments or with a single folder argument: those open a window and hang the run. The window forms are tested only through `parse_args`.
- The window itself is checked by hand.
