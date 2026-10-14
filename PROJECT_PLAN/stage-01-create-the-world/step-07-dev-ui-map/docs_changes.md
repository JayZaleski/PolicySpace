# Step 07: dev-ui-map — Docs Changes

## docs/dev-ui.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# The developer UI

`policyspace-dev-ui` (binary `dev-ui`, in `crates/dev-ui/`) is a tool for developers and setting authors, not a player UI. It depends on the engine by path and uses only the engine's public interface (`load_setting`, `World`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`). The engine knows nothing about it. The window is built with egui/eframe 0.36 (default features, which use the wgpu renderer), and the system folder picker comes from rfd 0.17. These are dependencies of this crate only.

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
| `check.rs` | The headless checker: `run`, the shared `error_lines`, and the `count`, `list`, `turn_line` and `run_turns` helpers. |
| `geometry.rs` | Pure map math: hex to world to screen, fitting, zooming, culling, and finding the hex under a point. No egui types. |
| `session.rs` | `Session` and `Loaded`: the current folder and what loading it gave. No egui types. |
| `app.rs` | The eframe window: `run`, the panels, and the map canvas. Uses `geometry`, `session` and `check`. |

Everything except `app.rs` is unit-tested without a window. The window is checked by hand.

## Loading: `Session`

`Session` holds `folder: Option<PathBuf>` and `loaded: Option<Loaded>`.

- `Session::open(folder)` makes `folder` current and loads it, replacing whatever was loaded. `reload()` loads the current folder again from disk, and does nothing if no folder was chosen.
- Loading calls `load_setting`. On success it builds `World::new(setting)` (at turn 0; the `TurnReport` is discarded for now) and stores `Loaded::World(world)`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `check::error_lines(&errors, &folder)`.
- A failed load replaces the previous world, so nothing from an earlier load stays on screen. A folder that fails to load is still the current folder, so Reload retries it.
- Loading is synchronous on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds.

`check::error_lines` returns each error's `Display` in order, then `{n} error(s) found in {folder}`. `check::run` prints these lines to stderr and the window shows the same lines, so both always agree.

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

## The window: `app`

`app::run(folder)` starts `eframe::run_native` with a 1280 × 800 window. If the window can't be created it prints `error: could not open the window: {error}` and returns exit code 1. `DevApp` holds:

- `session: Session`;
- `view: Option<View>`, where None means "fit on the next draw" (the canvas size is only known while drawing);
- `hovered: Option<HexId>`;
- `last_title`, so the viewport title command is sent only when the title changes.

The view lives in the app, not the session, so Reload never touches it: after a successful reload, even with a failed load in between, the map stays at the same pan and zoom. Open and Fit set `view` to None.

Each frame (`eframe::App::ui`) draws:

- **Top panel:** **Open setting…** (rfd's `pick_folder`; cancelling changes nothing), **Reload** (enabled when a folder is chosen), **Fit** (enabled when a world with at least one hex is loaded), and the folder path.
- **Status bar:** with a world, `{name} · {n} hexes · turn {turn}` plus ` · hex (q, r)` while a hex is hovered (using `check::count` for the wording); with errors, `{n} error(s)`; otherwise `No setting loaded`.
- **Central panel**, by state: no folder shows "No setting loaded." with an Open button; errors show a heading and every line in selectable monospace text inside a scroll area; a world with no hexes shows "This setting has no hexes."; otherwise `draw_map`.
- **Title:** `{ENGINE_NAME} {ENGINE_VERSION} — Dev UI`, with ` — {setting name}` appended once a world is loaded.

`draw_map` allocates the remaining area (click-and-drag sense), builds a `Viewport`, fits the view if it is None, and then:

1. Dragging pans. Scrolling over the canvas zooms toward the pointer by `exp(scroll_y · 0.0015)`, and egui's pinch zoom is applied too.
2. While the pointer is over the canvas and not dragging, `hovered = hex_at(...)`.
3. It walks `map.iter()` once per frame, skips hexes that fail `is_visible`, and paints the rest by `hex_shape()`: a dot, a 1 px hexagon outline (`Shape::closed_line`), or an outline plus the label `q, r`. The hovered hex is filled with the theme's selection color. Colors come from the current egui visuals.

One pass with culling is enough for maps of around 10,000 hexes; there is no caching. Clicking does nothing yet.

## Extending the window

Hex selection, inspection panels and turn stepping (Step 08) build on `DevApp`, the `hovered` hex, and the `World` held in `Session`. To draw something new per hex, add it to the loop in `draw_map` after the `is_visible` check, so it is only done for hexes on screen. Keep new map math in `geometry` (in `f64`, without egui types) so it can be unit-tested.

## Tests

- Unit tests in `cli.rs` (every accepted and rejected argument form), `check.rs` (`error_lines` for `many_errors`), `geometry.rs` (layout, round trips at the `i32` extremes, fitting empty, one-hex, sample and `hexes_valid` maps, zoom and pan, shape thresholds, `hex_at` including dots, and culling), and `session.rs` (open, reload after edits in a `tempfile` folder, failed loads, replacing one load with another).
- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`.
- `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.
- No test may run the binary with no arguments or with a single folder argument: those open a window and hang the run. The window forms are tested only through `parse_args`.
- The window itself is checked by hand.

## docs/setting-loader.md — modify (setting checker)
- **Location:** "Setting checker (dev UI)"
- **Change:** replace
- **Supersedes:** step-02-setting-loader/docs_changes.md (the checker's invocation and "no argument" row); step-06-turn-clock/docs_changes.md (the checker table, usage text, and `dev-ui <SETTING_FOLDER> --turns N` form)

The checker is now the `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)), implemented in `crates/dev-ui/src/check.rs`. It never opens a window, and it calls only the engine's public API.

| Arguments | Behavior | Exit code |
|---|---|---|
| `check <path>` | loads the setting; on success prints the success line to stdout | 0 |
| `check <path>` with errors | prints each error on its own line to stderr (see `check::error_lines`), then `<n> error(s) found in <path>` | 1 |
| `check <path> --turns N` | after the success line, creates a world, prints turn 0's line, then steps `N` times printing one line per turn (see [world.md](world.md)); `N` is a whole number from 0 to 10,000 (ASCII digits only, leading zeros allowed) | 0 |
| `check <path> --turns N` with errors | as for `check <path>` with errors; no turns are run | 1 |
| `--version` | prints `{ENGINE_NAME} {ENGINE_VERSION}` | 0 |
| anything else | prints the usage message to stderr: `usage: dev-ui [SETTING_FOLDER]`, `       dev-ui check SETTING_FOLDER [--turns N]`, `       dev-ui --version` on three lines | 2 |

The success line is as described by the earlier Steps: `Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>`. All arguments are parsed before anything is loaded. Run it with `cargo run -p policyspace-dev-ui -- check settings/sample`. The dev UI window shows the same error lines when a setting fails to load.

Also replace, in the "Tests" section, the sentence about `crates/dev-ui/tests/checker.rs` and `placeholder.rs` with: `crates/dev-ui/tests/checker.rs` runs `dev-ui check` against the sample and a broken setting; `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`" and "Tests" (the `checker.rs` bullet)
- **Change:** replace
- **Supersedes:** step-06-turn-clock/docs_changes.md (the invocation `dev-ui <SETTING_FOLDER> --turns N`, and the sentence saying Step 07 replaces the checker)

In "Setting checker: `--turns N`": replace the invocation with `dev-ui check <SETTING_FOLDER> --turns N`, and replace the last sentence ("This is deliberately minimal ...") with: This is deliberately minimal. The checker stays as the headless `check` mode of the dev UI (see [dev-ui.md](dev-ui.md)), and the dev UI window builds a `World` from a loaded setting at turn 0 and reads its map and turn. It does not step the world yet.

In "Tests": the `checker.rs` bullet's invocations now use the `check` form.

## docs/workspace.md — modify
- **Location:** Layout block; the `policyspace-dev-ui` bullet under "Crates and the dependency rule"; "Tests"
- **Change:** replace
- **Supersedes:** step-01-engine-workspace/docs_changes.md and step-02-setting-loader/docs_changes.md (the placeholder description of the dev UI, and `placeholder.rs`)

Layout block: under `crates/dev-ui/` list `src/` (`main.rs`, `cli.rs`, `check.rs`, `geometry.rs`, `session.rs`, `app.rs`) and `tests/` (`checker.rs`, `cli.rs`). Remove `placeholder.rs`.

Replace the dev UI bullet with: **`policyspace-dev-ui`** is the developer UI: an egui/eframe window that loads a setting folder, builds a world from it, and draws its hex map, plus a headless `check` mode that prints a setting's summary or errors. It depends on the engine by path and uses only the engine's public interface; the engine has no UI dependency. Its extra dependencies (`eframe`, `rfd`) belong to this crate only. See [dev-ui.md](dev-ui.md).

In "Tests": replace the `placeholder.rs` bullet with: `crates/dev-ui/tests/cli.rs` runs the built binary (`env!("CARGO_BIN_EXE_dev-ui")`) and checks `--version` and the usage errors. Add a note that no test may start the binary in a window form (no arguments or one folder), since that opens a window.

## docs/README.md — modify
- **Location:** the table of documents, after the `world.md` row
- **Change:** add
- **Supersedes:** none

| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads settings, the map math, the window, and the headless `check` mode |
