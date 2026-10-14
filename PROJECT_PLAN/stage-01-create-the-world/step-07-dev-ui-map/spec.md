# Step 07: dev-ui-map — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-07-dev-ui-map/design.md`: the approved design. Its Acceptance Criteria are the target.
- `PROJECT_PLAN/stage-01-create-the-world/step-06-turn-clock/docs_changes.md`: `World` (its accessors), and the setting checker's `--turns N` as it works today.
- `PROJECT_PLAN/stage-01-create-the-world/step-04-hex-map/docs_changes.md`: `HexCoord`, `HexMap`, and the fixed orientation (pointy-top, `r` rows east–west, `q` east, `r` south).
- Code this Step changes or moves:
  - `crates/dev-ui/Cargo.toml`, `crates/dev-ui/src/main.rs`
  - `crates/dev-ui/tests/checker.rs`, `crates/dev-ui/tests/placeholder.rs`
- Code this Step uses (read only, do not modify):
  - `crates/engine/src/lib.rs` (the public API), `crates/engine/src/world.rs`, `crates/engine/src/map.rs`, `crates/engine/src/setting/error.rs` (`LoadErrors`, `LoadError` `Display`), `crates/engine/src/setting/mod.rs` (`LoadedSetting`, `Manifest`)
  - `crates/engine/tests/settings/README.md` (what each test setting contains), `settings/sample/`
- `Cargo.toml` (workspace): lints `unsafe_code = "forbid"`, `missing_docs = "warn"`, clippy `all`; `tools/check.py` runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Follow the existing convention of a doc comment on every item, including private ones.
- `docs/` holds nothing for this Stage yet (it's written at the end of the Stage); the Step folders above are the reference.
- eframe/egui API: use version 0.36. Check the API against the crate source in the local cargo registry (`~/.cargo/registry/src/*/eframe-0.36.*`, `egui-0.36.*`) or docs.rs, rather than relying on memory: names such as the `App` trait method, scroll and zoom input, and painter shapes have changed between egui versions.

## Approach
- **egui/eframe 0.36 (default features, OpenGL "glow" backend) for the window, and rfd 0.17 for the system folder picker.** These are dependencies of `policyspace-dev-ui` only. The engine crate is not modified at all, and gets no new dependencies.
- **One binary crate, split into modules**, so most of the logic can be tested without a window:
  - `cli`: argument parsing and the usage message.
  - `check`: today's checker and `--turns`, moved from `main.rs` with identical output, plus the shared error-lines function.
  - `geometry`: pure map math (hex ↔ world ↔ screen, fitting, zooming, finding the hex under a point). No egui types, so it is unit-tested directly.
  - `session`: which folder is open and what loading it gave (a `World` or error lines), with open and reload. No egui types, so it is unit-tested directly.
  - `app`: the eframe window. Draws the panels and the map using `geometry` and `session`. Not tested automatically (the design puts automated window tests out of scope).
- **Map math is in `f64`**, with a "view" made of the world point at the canvas's center plus a scale (screen pixels per world unit). Coordinates at the `i32` extremes put hex centers around ±5.6 × 10⁹ world units, which `f32` (egui's type) can't hold precisely. Convert to `f32` only for hexes that survive culling, whose screen positions are near the canvas.
- **One world unit is a hex's circumradius**, so the view's scale is a hex's on-screen radius in pixels. This makes the thresholds (when to draw dots, when to show labels) simple comparisons against the scale.
- **The view lives in the app, not the session.** Reload never touches it, so after a successful reload (even with a failed load in between) the map stays at the same pan and zoom. Open and Fit clear it, and the map is fitted to the canvas the next time it's drawn (the canvas size is only known while drawing).
- **The error view and `check` share one function** that turns `LoadErrors` into lines, so the window shows exactly what `check` prints.
- **Loading is synchronous** on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds, and the window doesn't need to repaint while the system dialog is open.
- **One build, no packages.** `app` depends on `geometry`, `session`, and `check`, and is the largest part. The other modules are small, so a split would leave most of the work in Integration and still cost extra agents and a merge.

## Changes
Create:
- `crates/dev-ui/src/cli.rs`: `Command`, `parse_args`, `parse_turns`, `USAGE`; unit tests.
- `crates/dev-ui/src/check.rs`: the checker moved from `main.rs` (`run`, `count`, `list`, `turn_line`, `run_turns`), plus `error_lines`; unit test for `error_lines`.
- `crates/dev-ui/src/geometry.rs`: `Point`, `Viewport`, `View`, `HexShape`, and the free functions and constants below; unit tests.
- `crates/dev-ui/src/session.rs`: `Session`, `Loaded`; unit tests.
- `crates/dev-ui/src/app.rs`: `run` and the eframe app.
- `crates/dev-ui/tests/cli.rs`: `--version` and usage-error tests (replaces `placeholder.rs`).

Modify:
- `crates/dev-ui/Cargo.toml`: dependencies `eframe = "0.36"` and `rfd = "0.17"`; dev-dependency `tempfile = "3"`.
- `crates/dev-ui/src/main.rs`: becomes a thin dispatcher (module declarations, crate docs, `main`).
- `crates/dev-ui/tests/checker.rs`: every invocation gains the `check` argument; `USAGE` updated.
- `Cargo.lock`: updated by cargo.

Delete:
- `crates/dev-ui/tests/placeholder.rs` (its test moves to `tests/cli.rs` as the `--version` test).

Do not modify anything under `crates/engine/`, `settings/`, or `tools/`.

## Interfaces & Data

### `main.rs`
Crate docs (`//!`) replace the placeholder docs. They must say: this is the developer UI, a tool for developers, not a player UI; it uses only the engine's public interface; and the forms, matching `USAGE`:
- `dev-ui [SETTING_FOLDER]` opens the window (with that setting loaded, if given);
- `dev-ui check SETTING_FOLDER [--turns N]` is the headless setting checker (describe its output and exit codes as the old docs did: 0 success, 1 setting errors, 2 wrong usage);
- `dev-ui --version` prints the engine's name and version.

```rust
mod app;
mod check;
mod cli;
mod geometry;
mod session;

fn main() -> ExitCode {
    // collect std::env::args_os().skip(1); parse_args; on None print USAGE to stderr, exit 2.
    // Version => println!("{ENGINE_NAME} {ENGINE_VERSION}"), success.
    // Check { path, turns } => check::run(&path, turns).
    // Window { folder } => app::run(folder).
}
```
**Pitfall:** do not add `#![windows_subsystem = "windows"]`. It would detach the program from the console on Windows, and `check` and `--version` would print nothing.

### `cli.rs`
```rust
/// What the command line asks for.
pub enum Command {
    /// Open the window, with this setting folder loaded if given.
    Window { folder: Option<PathBuf> },
    /// Run the headless setting checker, then `turns` turns if given.
    Check { path: OsString, turns: Option<u32> },
    /// Print the engine's name and version.
    Version,
}

/// The usage message (printed to stderr on wrong usage).
pub const USAGE: &str = "usage: dev-ui [SETTING_FOLDER]\n       dev-ui check SETTING_FOLDER [--turns N]\n       dev-ui --version";

/// The most turns `--turns` accepts.
pub const MAX_TURNS: u32 = 10_000;

/// Parses the arguments (without the program name), or None if they're wrong.
pub fn parse_args(args: &[OsString]) -> Option<Command>;

/// Parses the number after `--turns`: non-empty ASCII digits only, at most MAX_TURNS.
fn parse_turns(n: &OsStr) -> Option<u32>;  // moved unchanged from main.rs
```
Rules for `parse_args`, in this order:
- `[]` → `Window { folder: None }`.
- `["--version"]` → `Version`.
- `["check", folder]` → `Check { path, turns: None }`.
- `["check", folder, "--turns", n]` → `Check { path, turns: Some(parse_turns(n)?) }`.
- `[folder]` → `Window { folder: Some(folder.into()) }`.
- Anything else → `None`.
- A `folder` argument that is the word `check` or starts with `-` is not a folder: the whole parse is `None`. So `dev-ui check`, `dev-ui --bogus`, `dev-ui check --turns 2`, and `dev-ui check -x` are usage errors. (A folder literally named `check` can be passed as `./check`.)

### `check.rs`
Move the checker from `main.rs` unchanged in behavior: the success line (`Loaded setting '<name>' (format version <n>): <goods>, <hexes>, <resources>, <deposits>`), the `--turns` lines, the error lines on stderr, and the exit codes. Keep `count`, `list`, `turn_line`, and `run_turns` as they are.

```rust
/// Loads the setting at `path` and prints the checker's output; runs `turns` turns if given.
/// Exit codes: 0 success, 1 setting errors (or a step error).
pub fn run(path: &OsStr, turns: Option<u32>) -> ExitCode;

/// The lines reported for a failed load: each error's `Display`, in order, then
/// `"{n} error(s) found in {folder.display()}"`. Used by `run` (printed to stderr)
/// and by the window's error view, so both show the same lines.
pub fn error_lines(errors: &LoadErrors, folder: &Path) -> Vec<String>;

/// "1 hex", "19 hexes": the count with the singular or plural word. (Now `pub(crate)`:
/// the window's status bar uses it too.)
pub(crate) fn count(n: usize, singular: &str, plural: &str) -> String;
```
`run` must print exactly what the old `main` printed for a failed load: the lines of `error_lines` to stderr, in order. Use `Path::new(path)` for the folder, as before.

### `geometry.rs`
Module docs: map math for the window, in `f64` world units where one unit is a hex's circumradius; pointy-top layout as fixed in Step 04; no egui types.

```rust
/// √3.
const SQRT_3: f64 = 1.732_050_807_568_877_2;

/// Below this on-screen hex radius (pixels), hexes are drawn as dots.
pub const DOT_BELOW_PX: f64 = 3.0;
/// The radius of a dot (pixels).
pub const DOT_RADIUS_PX: f64 = 1.5;
/// From this on-screen hex radius (pixels), hexes show their coordinates.
pub const LABEL_FROM_PX: f64 = 22.0;
/// When hexes are dots, hovering within this distance (pixels) of a dot's center counts.
pub const HOVER_SNAP_PX: f64 = 6.0;
/// Fitting never zooms in beyond this scale (so a one-hex map isn't enormous).
pub const FIT_MAX_SCALE: f64 = 48.0;
/// The fitted map fills this fraction of the canvas (a margin around it).
pub const FIT_FILL: f64 = 0.9;
/// Zoom limits (pixels per world unit).
pub const MIN_SCALE: f64 = 1e-12;
pub const MAX_SCALE: f64 = 400.0;
/// The scale of a fitted empty map, and the default.
pub const DEFAULT_SCALE: f64 = 32.0;

/// A point (world units or screen pixels, depending on use).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }

/// The canvas on screen, in pixels: top-left corner and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport { pub min: Point, pub width: f64, pub height: f64 }
// Viewport::center(&self) -> Point; Viewport::contains_with_margin(&self, p: Point, margin: f64) -> bool

/// What part of the world is on screen: `center` (a world point) is drawn at the
/// viewport's center, and `scale` is screen pixels per world unit (= hex radius in pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View { pub center: Point, pub scale: f64 }

/// The world position of a hex's center: x = √3 · (q + r/2), y = 1.5 · r.
pub fn hex_center(coord: HexCoord) -> Point;

/// The hex whose area contains world point `p`, or None if its coordinates don't fit in i32.
pub fn hex_containing(p: Point) -> Option<HexCoord>;

/// The six corners of a pointy-top hex of `radius` around `center`, clockwise from the
/// top-right one: corner i is at angle (60·i − 30)° from +x, with y pointing down.
pub fn hex_corners(center: Point, radius: f64) -> [Point; 6];

impl View {
    /// Fits every hex in `coords` inside `viewport`. Empty → center (0, 0), DEFAULT_SCALE.
    pub fn fit(coords: impl IntoIterator<Item = HexCoord>, viewport: Viewport) -> View;
    pub fn world_to_screen(&self, p: Point, viewport: Viewport) -> Point;
    pub fn screen_to_world(&self, p: Point, viewport: Viewport) -> Point;
    /// Moves the map by `dx, dy` screen pixels (dragging right moves the map right).
    pub fn pan(&mut self, dx: f64, dy: f64);
    /// Multiplies the scale by `factor` (clamped to MIN_SCALE..=MAX_SCALE), keeping the
    /// world point under `screen` fixed on screen.
    pub fn zoom_at(&mut self, screen: Point, factor: f64, viewport: Viewport);
    /// How hexes are drawn at this scale.
    pub fn hex_shape(&self) -> HexShape;
}

/// How hexes are drawn at the current scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexShape {
    /// Too small for a hexagon: a dot of DOT_RADIUS_PX.
    Dot,
    /// An outlined hexagon, without a label.
    Hexagon,
    /// An outlined hexagon with its coordinates inside.
    LabelledHexagon,
}

/// The hex under screen point `screen`, if any (see the rules below).
pub fn hex_at(map: &HexMap, view: &View, viewport: Viewport, screen: Point) -> Option<HexId>;

/// True if a hex centered at screen point `center` can be seen in `viewport` at this scale.
pub fn is_visible(view: &View, viewport: Viewport, center: Point) -> bool;
```
Logic notes:
- **`hex_center`:** compute in `f64` from `q as f64` and `r as f64`, never in integers (`q + r/2` overflows and truncates in `i32`).
- **`hex_containing`:** fractional axial `qf = (√3/3)·x − (1/3)·y`, `rf = (2/3)·y`. Round with cube rounding: `sf = −qf − rf`; round all three; whichever of the three has the largest rounding error is recomputed from the other two (`q = −r − s`, and so on). Return None if the rounded `q` or `r` is outside the `i32` range (check as `f64` before converting).
- **`View::fit`:** bounding box of every hex's extent: center x ± √3/2, center y ± 1. `center` = box center. `scale = min(vw / box_w, vh / box_h) · FIT_FILL`, then clamped to `MIN_SCALE..=FIT_MAX_SCALE`. Treat a viewport width or height below 1 as 1 (a minimised window), so the result is always finite and positive. For `hexes_valid`, which has coordinates at the `i32` extremes, `box_w` is about 7.4 × 10⁹, so the scale is around 10⁻⁷: far above `MIN_SCALE`.
- **`world_to_screen`:** `viewport.center() + (p − center) · scale`; `screen_to_world` is its inverse.
- **`zoom_at`:** take `w = screen_to_world(screen)`, change the scale, then set `center` so that `world_to_screen(w) == screen`.
- **`hex_shape`:** `Dot` if `scale < DOT_BELOW_PX`; `LabelledHexagon` if `scale >= LABEL_FROM_PX`; otherwise `Hexagon`.
- **`is_visible`:** the center is within the viewport expanded on every side by the drawn radius (`scale` for hexagons, `DOT_RADIUS_PX` for dots).
- **`hex_at` rules:**
  1. Exact: `hex_containing(screen_to_world(screen))`, then `map.id(coord)`. If found, return it.
  2. Otherwise, only when `hex_shape()` is `Dot`: the hex whose screen center is nearest to `screen` within `HOVER_SNAP_PX`. Ties go to the lower `HexId`, so the answer is deterministic. A linear scan over `map.iter()` is fine.
  3. Otherwise None.
- **Hover on a point outside the viewport** isn't geometry's concern; the app only calls `hex_at` while the pointer is over the canvas.

### `session.rs`
Module docs: what the window has loaded, separate from drawing so it can be tested without a window.

```rust
/// What the last load of the current folder gave.
pub enum Loaded {
    /// The setting loaded; its world is at turn 0.
    World(World),
    /// The setting has errors: the lines from `check::error_lines`.
    Errors(Vec<String>),
}

/// The current folder (if any) and what loading it gave.
#[derive(Default)]
pub struct Session { folder: Option<PathBuf>, loaded: Option<Loaded> }

impl Session {
    /// No folder chosen.
    pub fn new() -> Session;
    /// Makes `folder` the current folder and loads it (replacing whatever was loaded).
    pub fn open(&mut self, folder: PathBuf);
    /// Loads the current folder again, from disk. Does nothing if no folder was chosen.
    pub fn reload(&mut self);
    /// The current folder.
    pub fn folder(&self) -> Option<&Path>;
    /// What the last load gave; None if no folder was chosen.
    pub fn loaded(&self) -> Option<&Loaded>;
}
```
- Loading: `load_setting(&folder)`; on `Ok(setting)`, `let (world, _report) = World::new(setting)` (Step 08 will show the report); on `Err(errors)`, `Loaded::Errors(check::error_lines(&errors, &folder))`.
- A failed load replaces the previous world. Nothing from an earlier load is kept.
- If `open` is given a folder that fails to load, it's still the current folder, so Reload retries it.

### `app.rs`
```rust
/// Opens the window, with `folder` loaded if given, and runs until it's closed.
/// If the window can't be created, prints `error: could not open the window: {error}`
/// to stderr and returns exit code 1.
pub fn run(folder: Option<PathBuf>) -> ExitCode;
```
The app state: `session: Session`, `view: Option<View>` (None = fit on next draw), `hovered: Option<HexId>`, and the last title sent (so the title is only sent when it changes).

Window:
- `NativeOptions` with an initial inner size of 1280 × 800 and a title.
- Title: `"{ENGINE_NAME} {ENGINE_VERSION} — Dev UI"`; once a world is loaded, append `" — {manifest.name}"`. While errors are shown, use the plain title. Change it with the viewport title command, only when it differs from the last one sent.

Top panel (one row of controls):
- **Open setting…**: `rfd::FileDialog::new().set_title("Open setting folder").pick_folder()`. On `Some(folder)`: `session.open(folder)`, `view = None`, `hovered = None`. On `None` (cancelled): nothing changes.
- **Reload**: enabled only when `session.folder()` is Some. Calls `session.reload()` and sets `hovered = None`. Does **not** touch `view`.
- **Fit**: enabled only when a world with at least one hex is loaded. Sets `view = None`.
- The current folder's path as a label, if any.

Bottom panel (status bar), one line:
- World loaded: `"{name} · {count(n, "hex", "hexes")} · turn {turn}"`, plus `" · hex {coord}"` (`HexCoord`'s `Display`, `(q, r)`) while a hex is hovered.
- Errors: `"{n} error(s)"`, where n is the number of error lines minus the summary line.
- Nothing loaded: `"No setting loaded"`.

Central panel, by state:
- **No folder chosen:** a centered message "No setting loaded." with an **Open setting…** button that behaves like the top-panel one.
- **Errors:** a heading "The setting could not be loaded", then every line in monospace, inside a scroll area, selectable so it can be copied. No map is drawn.
- **World with no hexes:** a centered message "This setting has no hexes."
- **World with hexes:** the map canvas:
  - Allocate the whole remaining area, sensing click-and-drag. Build a `Viewport` from its rect.
  - If `view` is None, set it to `View::fit(map coords, viewport)`.
  - While dragging, call `view.pan` with the pointer delta.
  - While the pointer is over the canvas, apply scroll-wheel input as zoom toward the pointer: `factor = exp(scroll_y · 0.0015)` (up = zoom in). Also apply egui's pinch zoom factor if it isn't 1. Request a repaint when the view changes.
  - Hover: `hovered = hex_at(...)` while the pointer is over the canvas and not dragging; otherwise None.
  - Draw every hex with `is_visible` true, in `map.iter()` order, according to `view.hex_shape()`:
    - `Dot`: a filled circle of `DOT_RADIUS_PX` in the foreground color.
    - `Hexagon` and `LabelledHexagon`: a closed outline through `hex_corners(center, scale)`, 1 px stroke in the foreground color.
    - The hovered hex: filled with the theme's selection color (dots: a dot of radius 3 px in that color).
    - `LabelledHexagon`: the text `"{q}, {r}"` centered on the hex, font size `min(scale · 0.3, 14)`, in the foreground color.
  - Take colors from the current egui visuals so the map reads correctly in light and dark themes.
  - Convert to egui's `f32` positions only after `is_visible` passes.

Performance: one pass over the map per frame, with culling, is enough for the design's guide of 10,000 hexes. No caching is needed.

## Tasks
1. Add `eframe = "0.36"` and `rfd = "0.17"` to `crates/dev-ui/Cargo.toml` `[dependencies]`, and `tempfile = "3"` to `[dev-dependencies]`. Run `cargo build -p policyspace-dev-ui` to confirm they resolve and build with the pinned toolchain.
2. Create `cli.rs` (moving `parse_turns` and `MAX_TURNS` from `main.rs`), with unit tests.
3. Create `check.rs`: move `count`, `list`, `turn_line`, `run_turns` and the load-and-print code from `main.rs` into `run`; add `error_lines` and use it in `run`. Add the unit test.
4. Rewrite `main.rs` as the dispatcher, with the new crate docs. Temporarily make `Command::Window` print `USAGE` and exit 2 until task 8 lands, so the crate builds.
5. Update `tests/checker.rs` to the `check` form and the new `USAGE`. Replace `tests/placeholder.rs` with `tests/cli.rs`. Run `cargo test -p policyspace-dev-ui`: every checker test must pass with only the `check` argument added.
6. Create `geometry.rs` with its unit tests.
7. Create `session.rs` with its unit tests.
8. Create `app.rs` and wire `Command::Window` to `app::run`.
9. Launch the window by hand at least once (`cargo run -p policyspace-dev-ui -- ../../settings/sample` from the crate, or `cargo run -p policyspace-dev-ui -- settings/sample` from the repo root) to confirm it opens, draws the sample, pans, zooms, and hovers. Close it. If the environment can't open a window, say so in the report instead.
10. Run `python tools/check.py` (or `.venv/Scripts/python tools/check.py`) and fix everything until it passes.

## Test Plan
Run everything with `python tools/check.py`; `cargo test -p policyspace-dev-ui` runs this crate's tests alone.

**Pitfall: no test may run the `dev-ui` binary with no arguments, or with a single folder argument.** Those open a window, which hangs the test run. The window forms are tested only through `parse_args` unit tests.

Unit tests in `cli.rs` (design: Commands):
- `[]` → `Window { folder: None }`; `["settings/sample"]` → `Window { folder: Some(..) }`.
- `["--version"]` → `Version`.
- `["check", "x"]` → `Check { turns: None }`; `["check", "x", "--turns", "7"]` → `turns: Some(7)`.
- None for: `["check"]`, `["--bogus"]`, `["-x"]`, `["check", "-x"]`, `["check", "--turns", "2"]`, `["a", "b"]`, `["--version", "x"]`, `["check", "x", "--turns"]`, `["check", "x", "--turns", "10001"]`, `["check", "x", "--steps", "1"]`.

Unit test in `check.rs` (design: load failure shows the same lines as `check`):
- `error_lines` for `crates/engine/tests/settings/many_errors` (via `load_setting` on a path built from `env!("CARGO_MANIFEST_DIR")`) gives the four error lines listed in `tests/checker.rs` and then `4 error(s) found in <path>`.

Integration tests in `tests/checker.rs` (design: `check` keeps the old output and exit codes): every existing test, unchanged except that each invocation is prefixed with `check` and `USAGE` is the new text. Keep the existing cases for bad `--turns` values (prefixed with `check`), and `two_arguments_print_usage_and_exit_2` (`a b`, with no `check`).

Integration tests in `tests/cli.rs` (design: `--version` and usage):
- `--version` prints `"{ENGINE_NAME} {ENGINE_VERSION}"` and exits 0 (the old placeholder test).
- `check`, `--bogus`, `check -x`, `--version extra` each print `USAGE` to stderr, nothing to stdout, and exit 2.

Unit tests in `geometry.rs` (design: Testing):
- `hex_center` of (0, 0) is (0, 0); of (1, 0) is (√3, 0); of (0, 1) is (√3/2, 1.5); of (i32::MAX, i32::MIN) is finite and matches the formula in `f64`.
- `hex_containing(hex_center(c)) == Some(c)` for a spread of coordinates including negatives and both `i32` extremes; a point slightly inside each of a hex's six corners maps to that hex; a world point far beyond the `i32` range gives None.
- `hex_corners` gives six points at distance `radius`, with one directly above the center (pointy-top).
- `world_to_screen` and `screen_to_world` are inverses; `zoom_at` keeps the world point under the cursor in place, and clamps at `MIN_SCALE` and `MAX_SCALE`; `pan` moves the screen position of a world point by exactly the delta.
- `View::fit`: an empty map gives center (0, 0) and `DEFAULT_SCALE`; a one-hex map is centered on that hex at `FIT_MAX_SCALE`; the sample's 19 hexes all have screen centers inside the viewport; `hexes_valid`'s coordinates (including the `i32` extremes) give a finite, positive scale with every hex visible; a zero-size viewport gives a finite, positive scale.
- `hex_shape` switches at `DOT_BELOW_PX` and `LABEL_FROM_PX`.
- `hex_at` on a small map built by loading a test setting (`hexes_valid`, `settings/sample`): the screen center of each hex finds that hex; a point between islands finds None; at a dot-sized scale, a point within `HOVER_SNAP_PX` of a dot finds it, and one farther away finds None.
- `is_visible`: centers inside, just outside (within the radius), and far outside the viewport.

Unit tests in `session.rs` (design: Loading a setting):
- A new session has no folder and nothing loaded; `reload` on it does nothing.
- `open` on `settings/sample` gives a world with 19 hexes at turn 0.
- `open` on `many_errors` gives `Errors` equal to `check::error_lines` for it, and the folder is still current.
- Reload sees edits on disk: in a `tempfile` folder, write a valid setting (`setting.toml` with `name` and `format_version = 1`, `rules/README.md`, `start/map.toml` with one `[[hex]]`); open it (1 hex); add a second hex and reload (2 hexes); break `setting.toml` and reload (`Errors`, no world); fix it and reload (world again).
- `open` on a different folder replaces what was loaded.

Manual checks at Code Review (design: the window's Acceptance Criteria), run by the user:
- `dev-ui settings/sample`: 19 hexes fitted, pointy-top, `r` rows east–west and increasing southward; drag, scroll, Fit; labels when zoomed in, `0, 0` at the center; hover highlight and status bar.
- `dev-ui` with no folder: Open setting…, cancel, and choose the sample.
- Edit a file to add an error, Reload (errors, no map), fix it, Reload (map at the same pan and zoom).
- A broken folder on the command line; `valid_minimal` ("This setting has no hexes."); `hexes_valid` (every hex visible as a hexagon or a dot).

## Constraints
- Do not modify the engine crate (`crates/engine/`), `settings/`, or `tools/`. Everything the window needs is already public (`load_setting`, `World`, `HexMap`, `HexCoord`, `LoadErrors`, `ENGINE_NAME`, `ENGINE_VERSION`).
- `check`'s output must stay byte-for-byte what the checker printed before this Step (apart from the new usage message).
- `geometry` and `session` must not use egui or eframe types.
- No `unsafe`, no `#![windows_subsystem = "windows"]`, and no `HashMap` iteration in anything that decides what's drawn or hovered (ties in `hex_at` go to the lower `HexId`).
- A doc comment on every item, following the existing style.
- Out of scope (Step 08 or later): showing deposits, goods, stockpiles, or turn reports; clicking or selecting hexes; stepping turns; remembering the folder or view between runs; automated tests that drive the window; terrain, neighbors, wrapping.

## Done When
All tasks are complete, every test in the Test Plan passes, `python tools/check.py` passes, and every Acceptance Criterion in design.md is met (the window's criteria by the manual checks at Code Review).
