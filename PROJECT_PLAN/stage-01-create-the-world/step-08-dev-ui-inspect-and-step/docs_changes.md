# Step 08: dev-ui-inspect-and-step — Docs Changes

All changes build on Step 07's proposed `docs/dev-ui.md` (new file, not yet applied). Apply Step 07's version first, then these edits.

## docs/dev-ui.md — modify (intro and module table)
- **Location:** the intro paragraph; the "Module structure" table
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the intro's engine-API list, and the `check.rs`, `session.rs` and `app.rs` rows; the new `inspect.rs` row is added)

Intro: replace "(`load_setting`, `World`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`)" with "(`load_setting`, `World` and its `step`, `TurnReport`, `StepError`, `Stockpile`, `Deposits`, `Rules`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`)".

Module table rows:

| Module | Purpose |
|---|---|
| `check.rs` | The headless checker: `run`, the shared `error_lines`, and the `count`, `list`, `run_turns` helpers. `turn_line` is `pub(crate)` because the window's turn log uses it too. |
| `inspect.rs` | The text the side panels show (countdowns, deposit lines, stockpile lines, goods and resource-type rows), built from a `World`. No egui types. |
| `session.rs` | `Session` and `Loaded`: the current folder, what loading it gave, the turn log, and the selected hex. Steps the world. No egui types. |
| `app.rs` | The eframe window: `run`, the panels, keyboard shortcuts, and the map canvas. Uses `geometry`, `session`, `inspect` and `check`. |

## docs/dev-ui.md — modify (Loading: `Session`)
- **Location:** section "Loading: `Session`"
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the "Loading: `Session`" section)

## Loading and stepping: `Session`

`Session` holds:

- `folder: Option<PathBuf>`;
- `loaded: Option<Loaded>`;
- `log: Vec<String>`: every turn line since the last load, oldest first;
- `step_error: Option<String>`: the error from the last refused step since the last load;
- `selected: Option<HexCoord>`: the selected hex.

**Loading.**

- `Session::open(folder)` makes `folder` current and loads it, replacing whatever was loaded. `reload()` loads the current folder again from disk, and does nothing if no folder was chosen.
- Loading calls `load_setting`. On success it builds `World::new(setting)` (at turn 0), and the log becomes exactly `[check::turn_line(&world, &report)]` for turn 0's `TurnReport`. The result is `Loaded::World(world)`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `check::error_lines(&errors, &folder)`, and the log is emptied.
- Either way `step_error` is cleared: only a load clears it.
- A failed load replaces the previous world, so nothing from an earlier load stays on screen. A folder that fails to load is still the current folder, so Reload retries it.
- Loading is synchronous on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds.

`check::error_lines` returns each error's `Display` in order, then `{n} error(s) found in {folder}`. `check::run` prints these lines to stderr and the window shows the same lines, so both always agree.

**Stepping.** `step(times)` calls `World::step` up to `times` times and pushes `check::turn_line` of each report onto the log. It stops at the first refused step and stores that `StepError`'s `Display` in `step_error`. It does nothing without a world and returns the number of steps taken. `STEP_MANY` (10) is the count the Step ×10 button and Shift+Space use. The engine offers no way to put a world at the maximum turn, so the refused-step path can't be reached from the dev UI and is not tested. The log stores formatted lines rather than reports, so it matches `check --turns` word for word and the window needs no formatting. It is never trimmed: it grows by one line per step until the next load.

**Selection.** The selection is stored as a `HexCoord`, never a `HexId`, because a `HexId` is an index into one map and may name a different hex (or none) after a reload, and `Deposits::on_hex` and `World::hex_stockpile` panic on an out-of-range id.

- `select(Some(hex))` stores `map.coord(hex)`; `select(None)` clears. Without a world, `Some` is ignored.
- `selected()` returns `map.id(coord)` for the current world, so it is None when nothing is selected, nothing is loaded, or the coordinates aren't on the map.
- `open` always clears the selection.
- `reload` keeps it, unless the load succeeded and the new map has no hex at those coordinates (then it is cleared). A failed load keeps it, so the next successful reload selects the hex again if it still exists. This mirrors how Reload keeps pan and zoom.

## docs/dev-ui.md — new section
- **Location:** new section "Panel text: `inspect`", after "Map math: `geometry`"
- **Change:** add
- **Supersedes:** none

## Panel text: `inspect`

`inspect` builds every string the side panels show, from a `&World`, without egui types, so it is unit-tested without a window. Amounts use `Quantity`'s `Display`, as `check` does. Goods, resource types and deposits come out in ID order (the setting's definition order), so the panels are deterministic.

- `countdown(PerishesAt, now: Turn)`: for `EndOfTurn(t)` with `n = t - now` (saturating), "perishes this turn" (0), "perishes in 1 turn", or "perishes in {n} turns"; `Never` gives "never perishes".
- `lasts(Perishes)`: "perishes after 1 turn", "perishes after {n} turns", or "never perishes".
- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add` (`Quantity` has no multiply).
- `stockpile_lines(world, hex)`: one `StockLine { good, parts }` per good held, in good ID order. `good` is `{total} {good}`; `parts` has one `{amount}: {countdown}` per perish group, soonest first.
- `good_rows(world)`: one `GoodRow { name, lasts, movable, held }` per good. `held` sums the good over every hex stockpile. It does one pass over the hexes per good, each frame; if that profiles badly, compute all goods in one pass.
- `resource_rows(world)`: one `ResourceRow { name, produces, output, deposits }` per resource type; deposits are counted in one pass over `world.deposits().iter()`.

## docs/dev-ui.md — modify (The window)
- **Location:** section "The window: `app`" (from "`DevApp` holds" to the end of the `draw_map` list, and the closing "One pass" paragraph)
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the "Each frame" list, the `draw_map` description, and "Clicking does nothing yet")

`DevApp` holds `session`, `view: Option<View>` (None means "fit on the next draw"), `hovered: Option<HexId>`, and `last_title`. It holds no selection state: that lives in `Session`. The view lives in the app, not the session, so Reload never touches it. Open and Fit set `view` to None; Open and Reload reset `hovered`.

Each frame (`eframe::App::ui`) runs in this order:

1. **Shortcuts** (`DevApp::shortcuts`), only when a world is loaded, before anything is drawn. Inside one `ui.input_mut`, it consumes `Shift+Space`, then `Space`, then `Esc`. Both Space keys are consumed every frame, even when the first matched: egui presses a focused button on Space, so a Space left in the queue would press a focused Step button as well as trigger the shortcut. Shift+Space is consumed first because `consume_key` ignores extra Shift. Shift+Space steps `STEP_MANY` times, Space steps once, Esc clears the selection. Because a held key repeats, holding Space keeps stepping.
2. **Top panel:** **Open setting…**, **Reload**, **Fit**, then **Step** (hover text "Space") and **Step ×10** (hover text "Shift+Space"), and the folder path. Step and Step ×10 are enabled whenever a world is loaded, including one with no hexes. Fit needs at least one hex.
3. **Status bar:** unchanged. With a world it shows `{name} · {n} hexes · turn {turn}`, plus ` · hex (q, r)` while a hex is hovered. The turn now changes as the world steps.
4. **Side panels** (only for `Loaded::World`, and shown before the central panel): `Panel::left("world")` and `Panel::right("hex")`, both resizable (default about 320 px, minimum about 200 px).
5. **Central panel**, by state: no folder shows "No setting loaded." with an Open button; errors show every line in selectable monospace text; a world with no hexes shows "This setting has no hexes."; otherwise `draw_map`. A click result from `draw_map` is applied with `session.select(..)` after the panel closure, because the world is borrowed from the session while drawing.
6. **Title:** `{ENGINE_NAME} {ENGINE_VERSION} — Dev UI`, with ` — {setting name}` once a world is loaded.

**World panel (left)**, in one vertical `ScrollArea`, three collapsible sections (all open by default):

- *Turn log*: the refused-step error first, if any (in the theme's `error_fg_color`), then the log newest first in an inner `ScrollArea::both()` (id salt "turn log", max height 320 px) drawn with `show_rows` at one body-text line per row. Rows are non-wrapping labels, so every row has the same height and thousands of lines stay smooth.
- *Goods*: a striped `Grid` (Name, Lasts, Movable, Held) from `inspect::good_rows`.
- *Resource types*: a striped `Grid` (Name, Produces, Output per deposit, Deposits) from `inspect::resource_rows`.

**Hex panel (right)**, in a vertical `ScrollArea`: for a world with no hexes, "This setting has no hexes."; with no selection, "Click a hex to inspect it."; otherwise the heading `Hex (q, r)`, a "Deposits" subheading with `inspect::deposit_lines` (or "No deposits."), and a "Stockpile" subheading listing each `StockLine.good` with its `parts` indented (or "Empty."). Both panels read the world directly each frame, so they update straight after a step or reload.

**`draw_map`** allocates the remaining area (click-and-drag sense), builds a `Viewport`, fits the view if it is None, and then:

1. Dragging pans. Scrolling over the canvas zooms toward the pointer by `exp(scroll_y · 0.0015)`, and egui's pinch zoom is applied too.
2. While the pointer is over the canvas and not dragging, `hovered = hex_at(...)`.
3. If the canvas was clicked and the pointer position is known, it returns `Some(hex_at(...))`: `Some(Some(hex))` selects (including dots within `HOVER_SNAP_PX`), `Some(None)` is a click on empty space and clears the selection. egui does not report a click for a drag, so panning never changes the selection.
4. It walks `map.iter()` once per frame, skips hexes that fail `is_visible`, and paints the rest by `hex_shape()`: a dot, a 1 px hexagon outline (`Shape::closed_line`), or an outline plus the label `q, r`. The hovered hex is filled with the theme's selection color. Colors come from the current egui visuals.
5. **Deposit mark:** a visible hex with at least one deposit (`!world.deposits().on_hex(id).is_empty()`) drawn as `Hexagon` or `LabelledHexagon` gets a filled circle in the foreground color at `center + (0, 0.55 · scale)`, below the label. Its radius is `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels; the cap keeps the mark inside the hexagon at small scales. Dots get no mark.
6. **Selected outline:** after the loop, in a second pass so neighbouring outlines don't paint over it, the selected hex is outlined in `visuals.selection.stroke.color` if it passes `is_visible`: a 3 px closed line through `hex_corners` for hexagons, or a 5 px-radius, 2 px circle stroke for dots. A hex that is hovered and selected keeps its hover fill with the outline on top.

One pass with culling is enough for maps of around 10,000 hexes; there is no caching.

## docs/dev-ui.md — modify (Extending the window)
- **Location:** section "Extending the window"
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the "Extending the window" section)

## Extending the window

- **New panel text:** add a pure function to `inspect` that takes a `&World` and returns strings or small row structs, unit-test it, and render it in `world_panel_contents` or `hex_panel_contents` in `app.rs`.
- **New per-hex drawing:** add it to the loop in `draw_map` after the `is_visible` check, so it is only done for hexes on screen. Anything that must not be overpainted by neighbours (like the selection outline) goes in a pass after the loop.
- **New state tied to a world** (for example a log): put it in `Session`, reset it where loading happens, and decide whether `open` and `reload` keep it. Never store a `HexId` across a load; store the `HexCoord`.
- **New keyboard shortcuts:** consume them in `DevApp::shortcuts`, which runs before any widget is drawn, and consume every key a focused widget would also react to.
- Keep new map math in `geometry` (in `f64`, without egui types) so it can be unit-tested.
- The Stage 02 additions (owners, POPs and their stockpiles) belong in the hex panel and world panel the same way.

## docs/dev-ui.md — modify (Tests)
- **Location:** section "Tests"
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the first bullet of "Tests")

First bullet: Unit tests in `cli.rs` (every accepted and rejected argument form), `check.rs` (`error_lines` for `many_errors`), `geometry.rs` (layout, round trips at the `i32` extremes, fitting, zoom and pan, shape thresholds, `hex_at` including dots, and culling), `inspect.rs` (countdown and `lasts` wording, deposit and stockpile lines on the sample and `turns_valid`, goods' world totals, resource-type deposit counts), and `session.rs` (open, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, selection kept or cleared across reload and open).

## docs/world.md — modify
- **Location:** the sentence in "Setting checker: `--turns N`" about the dev UI window
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the closing sentence ending "It does not step the world yet.")

The checker stays as the headless `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)). The dev UI window builds a `World` from a loaded setting, steps it with its Step buttons, and shows the same turn lines in its turn log (`check::turn_line`) as `check --turns` prints.

## docs/workspace.md — modify
- **Location:** Layout block under `crates/dev-ui/`; the `policyspace-dev-ui` bullet
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the `src/` file list and the dev UI bullet)

Layout block: the `src/` list is `main.rs`, `cli.rs`, `check.rs`, `geometry.rs`, `inspect.rs`, `session.rs`, `app.rs`.

Dev UI bullet: **`policyspace-dev-ui`** is the developer UI: an egui/eframe window that loads a setting folder, builds a world from it, draws its hex map, inspects hexes, goods and resource types, and steps turns, plus a headless `check` mode that prints a setting's summary or errors. It depends on the engine by path and uses only the engine's public interface; the engine has no UI dependency. Its extra dependencies (`eframe`, `rfd`) belong to this crate only. See [dev-ui.md](dev-ui.md).

## docs/README.md — modify
- **Location:** the `dev-ui.md` row added by Step 07
- **Change:** replace
- **Supersedes:** step-07-dev-ui-map/docs_changes.md (the `dev-ui.md` row)

| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads and steps settings, the map math, the panel text, the window, and the headless `check` mode |
