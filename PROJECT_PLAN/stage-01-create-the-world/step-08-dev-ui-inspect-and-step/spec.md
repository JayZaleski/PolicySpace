# Step 08: dev-ui-inspect-and-step — Spec

## Context
Read before starting:
- `PROJECT_PLAN/stage-01-create-the-world/step-08-dev-ui-inspect-and-step/design.md`: the approved design. Its Acceptance Criteria are the target, and its wording for panel text is binding.
- `PROJECT_PLAN/stage-01-create-the-world/step-07-dev-ui-map/spec.md` and `docs_changes.md`: how the dev UI is built (modules, `Session`, `DevApp`, `draw_map`, the egui 0.36 API notes, the test pitfalls).
- `PROJECT_PLAN/stage-01-create-the-world/step-06-turn-clock/docs_changes.md`: `World`, `TurnReport`, `StepError`, `Stockpile`, `PerishesAt`.
- Code this Step changes:
  - `crates/dev-ui/src/app.rs`, `crates/dev-ui/src/session.rs`, `crates/dev-ui/src/check.rs`, `crates/dev-ui/src/main.rs`
- Code this Step uses (read only, do not modify):
  - `crates/dev-ui/src/geometry.rs` (`hex_at`, `is_visible`, `hex_corners`, `HexShape`, `HOVER_SNAP_PX`)
  - `crates/engine/src/lib.rs` (the public API), `world.rs`, `stockpile.rs`, `resources.rs` (`Deposits::on_hex`, `of_type_on_hex`), `goods.rs` (`Perishes`), `turn.rs`, `quantity.rs`, `map.rs`
  - `crates/engine/tests/settings/README.md` (`turns_valid`, `hexes_valid`, `hexes_one`, `valid_minimal`), `settings/sample/`
- `tools/check.py` runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Workspace lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`. Every item gets a doc comment, private ones included.
- egui/eframe 0.36.2 (already a dependency). Check API names against the crate source in `~/.cargo/registry/src/*/egui-0.36.2/src`, not memory. Confirmed there: `Panel::left(id)` / `Panel::right(id)` with `.resizable(bool)`, `.default_size(f32)`, `.min_size(f32)`; `ScrollArea::show_rows`, `.max_height`, `.id_salt`; `CollapsingHeader::new(..).default_open(true)`; `Grid::new(id).num_columns(n).striped(true)`; `Label::extend()`; `InputState::consume_key(Modifiers, Key)`; `Painter::circle_stroke`.

## Approach
- **Only the dev UI changes.** The engine already exposes everything needed (`World::step`, `TurnReport`, `hex_stockpile`, `Stockpile::iter`/`groups`, `Deposits`, `Rules`).
- **`Session` gains stepping, the turn log, the step error, and the selection**, so all of the design's state rules (Reload resets the log; selection survives Reload) are unit-tested without a window. `Loaded` is unchanged.
- **The log stores formatted lines** (`check::turn_line`, oldest first). Names never change during a world's life, the window needs strings anyway, and it guarantees the log matches `check --turns` word for word. The window shows it newest first.
- **The selection is stored as a `HexCoord`, not a `HexId`.** A `HexId` is an index into one map; after Reload it could name a different hex or none, and `Deposits::on_hex`/`World::hex_stockpile` panic on an out-of-range id. Coordinates are looked up in the current map each time.
- **New pure module `inspect`** (no egui types) builds every piece of panel text: the countdown, how long a good lasts, deposit lines, stockpile lines, goods rows, and resource-type rows. Unit-tested.
- **Keyboard shortcuts are handled at the very start of each frame, before any widget is drawn.** egui presses a focused button on Space (`context.rs`: `key_pressed(Key::Space)`), so after clicking Step, Space would otherwise both press the button and trigger the shortcut. Consuming the key first prevents this. `consume_key` ignores extra Shift, so Shift+Space must be checked before Space.
- **One build, no packages.** `app.rs` is most of the work and depends on both `session` and `inspect`; the other parts are small. A split would leave most of the work in Integration and cost extra agents and a merge.

## Changes
Create:
- `crates/dev-ui/src/inspect.rs`: panel text, with unit tests.

Modify:
- `crates/dev-ui/src/session.rs`: stepping, log, step error, selection; tests.
- `crates/dev-ui/src/check.rs`: `turn_line` becomes `pub(crate)`. No behavior change.
- `crates/dev-ui/src/app.rs`: shortcuts, Step buttons, the two side panels, click to select, deposit marks, selected-hex outline.
- `crates/dev-ui/src/main.rs`: `mod inspect;` and the crate docs' window bullet (the window also inspects hexes and steps turns).

Do not modify anything under `crates/engine/`, `settings/`, or `tools/`, nor `geometry.rs`, `cli.rs`, or the tests in `crates/dev-ui/tests/`.

## Interfaces & Data

### `check.rs`
```rust
/// The line printed for a turn. `pub(crate)`: the window's turn log uses it too.
pub(crate) fn turn_line(world: &World, report: &TurnReport) -> String;
```
Nothing else changes. `check` output must stay byte-for-byte the same.

### `session.rs`
Update the module docs: the session also steps the world, keeps the turn log, and remembers the selected hex.

```rust
/// How many steps Step ×10 takes.
pub const STEP_MANY: u32 = 10;

pub struct Session {
    folder: Option<PathBuf>,
    loaded: Option<Loaded>,
    /// Every turn line since the last load, oldest first (`check::turn_line`). Empty
    /// unless a world is loaded.
    log: Vec<String>,
    /// The error from the last refused step, if any, since the last load.
    step_error: Option<String>,
    /// The selected hex's coordinates. Kept across a failed load (see `reload`).
    selected: Option<HexCoord>,
}

impl Session {
    // new, open, reload, folder, loaded: as before, with the changes below.

    /// Steps the world up to `times` times, adding each report's line to the log. Stops at
    /// the first refused step, storing its error (`StepError`'s `Display`). Does nothing
    /// without a world. Returns the number of steps taken.
    pub fn step(&mut self, times: u32) -> u32;

    /// The turn log, oldest first.
    pub fn log(&self) -> &[String];

    /// The last refused step's error, if any, since the last load.
    pub fn step_error(&self) -> Option<&str>;

    /// Selects `hex` (a hex of the current world's map), or clears the selection with None.
    /// Without a world, `Some` is ignored and `None` clears.
    pub fn select(&mut self, hex: Option<HexId>);

    /// The selected hex in the current world's map; None when nothing is selected, no
    /// world is loaded, or the selected coordinates aren't on the map.
    pub fn selected(&self) -> Option<HexId>;
}
```
Rules:
- **Loading** (shared by `open` and `reload`): on success, `World::new(setting)` gives the world and turn 0's report; the log becomes exactly `[turn_line(&world, &report)]`. On failure, the log is emptied. Either way `step_error` becomes None. Remove the Step 07 comment "Step 08 will show the turn 0 report…".
- **`open`** always clears `selected` (even when the new folder is the same folder).
- **`reload`** keeps `selected` unless the load succeeded and the new map has no hex at those coordinates (`map.id(coord).is_none()`), in which case it becomes None. A failed load keeps it (design: a failed Reload doesn't clear the selection).
- **`select(Some(hex))`** stores `world.map().coord(hex)`.
- **`selected`** returns `map.id(coord)` for the current world.
- **`step`**: `for _ in 0..times { match world.step() { Ok(r) => push turn_line(world, &r), Err(e) => { step_error = Some(e.to_string()); break } } }`. A later successful step (impossible in practice) leaves `step_error` as it is; only a load clears it.

### `inspect.rs`
Module docs: the text the window's panels show, built from a `World` without egui types so it can be tested. Wording follows design.md. Amounts use `Quantity`'s `Display` (as `check` does).

```rust
/// When a group perishes, as a countdown from turn `now`:
/// `EndOfTurn(t)`: n = t − now (saturating at 0): 0 → "perishes this turn",
/// 1 → "perishes in 1 turn", n → "perishes in {n} turns"; `Never` → "never perishes".
pub fn countdown(at: PerishesAt, now: Turn) -> String;

/// How long a good lasts: "perishes after 1 turn", "perishes after {n} turns", or
/// "never perishes".
pub fn lasts(perishes: Perishes) -> String;

/// One line per resource type present on `hex`, in resource ID order:
/// "{type} × {count}: {output} {good} each, {total} total". Empty if it has no deposits.
pub fn deposit_lines(world: &World, hex: HexId) -> Vec<String>;

/// One good in a hex's stockpile.
#[derive(Debug, Clone, PartialEq)]
pub struct StockLine {
    /// "{total} {good}", e.g. "15 stew".
    pub good: String,
    /// One per group, soonest first: "{amount}: {countdown}", e.g. "5: perishes in 1 turn".
    pub parts: Vec<String>,
}

/// The stockpile of `hex`, in good ID order. Empty if the stockpile is empty.
pub fn stockpile_lines(world: &World, hex: HexId) -> Vec<StockLine>;

/// One row of the Goods table.
#[derive(Debug, Clone, PartialEq)]
pub struct GoodRow {
    pub name: String,
    /// `lasts(..)`.
    pub lasts: String,
    /// "movable" or "fixed".
    pub movable: &'static str,
    /// The world total over every hex stockpile, formatted (`Quantity` Display; "0" if none).
    pub held: String,
}

/// Every good, in good ID order.
pub fn good_rows(world: &World) -> Vec<GoodRow>;

/// One row of the Resource types table.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRow {
    pub name: String,
    /// The produced good's name.
    pub produces: String,
    /// Output per deposit, formatted.
    pub output: String,
    /// The number of deposits of this type in the world.
    pub deposits: usize,
}

/// Every resource type, in resource ID order.
pub fn resource_rows(world: &World) -> Vec<ResourceRow>;
```
Logic notes:
- `deposit_lines`: iterate `world.rules().resources.iter()` (ID order); for each type, `n = world.deposits().of_type_on_hex(hex, id).count()`; skip 0. `total` = `output` added to itself n times with `Quantity::saturating_add` starting from `Quantity::ZERO` (Quantity has no multiply; repeated addition matches how emission sums). Sample (0, 0) gives `["copper_ore × 1: 2.5 unimproved_copper each, 2.5 total", "arable_land × 3: 4 unimproved_grain each, 12 total"]` (copper_ore is defined first). The `×` is U+00D7.
- `stockpile_lines`: `stockpile.iter()` for goods and totals, `stockpile.groups(good)` for parts, `countdown(at, world.turn())`.
- `good_rows`: the held total sums `world.hex_stockpile(id).total(good)` over `world.map().iter()` with `saturating_add`. Called once per frame; one pass per good over the hexes is fine at 10,000 hexes. If it profiles badly, compute all goods in one pass over the hexes instead.
- `resource_rows`: count deposits by type with one pass over `world.deposits().iter()`.

### `app.rs`
Update the module docs: the window also has the world and hex panels and steps turns.

`DevApp` gets no new state: the selection lives in `Session`.

**Frame order in `eframe::App::ui`:**
1. `self.shortcuts(ui)`: only when a world is loaded. In `ui.input_mut(|i| ...)`: if `consume_key(Modifiers::SHIFT, Key::Space)` → `session.step(STEP_MANY)`; else if `consume_key(Modifiers::NONE, Key::Space)` → `session.step(1)`. Separately, `consume_key(Modifiers::NONE, Key::Escape)` → `session.select(None)`. This must run before any panel is drawn (see Approach). Consume every queued press of the key in the frame; `consume_key` returning true once per frame is acceptable (key repeat gives repeated steps while held, which is fine).
2. Top panel, bottom panel (as today).
3. When `Loaded::World`: `Panel::left("world")` then `Panel::right("hex")`, both `.resizable(true)`, default width about 320 px, min width about 200 px. Side panels must be shown before `CentralPanel`.
4. Central panel (as today, with the map changes below).
5. The title update (as today).

**Top panel:** after Fit, add **Step** and **Step ×10**, both enabled only when `Loaded::World` (including a world with no hexes), with hover text "Space" and "Shift+Space". Clicking calls `session.step(1)` / `session.step(STEP_MANY)`. Reload no longer sets anything about the selection itself (the session handles it); keep `hovered = None`. Open (both buttons) keeps setting `view = None` and `hovered = None`; the session clears the selection.

**World panel (left)**, all inside one `ScrollArea::vertical()`:
- `CollapsingHeader` "Turn log" (open by default):
  - If `session.step_error()` is Some, show it first, in the theme's error color (`visuals.error_fg_color`).
  - Then the log, newest first, in an inner `ScrollArea::both()` with `.id_salt("turn log")`, `.max_height(320.0)`, using `show_rows` with the height of one line of body text, so thousands of lines stay smooth. Row `i` shows `log[len − 1 − i]` as `Label::new(line).extend()` (no wrapping, so every row has the same height).
- `CollapsingHeader` "Goods" (open): a striped `Grid` with columns Name, Lasts, Movable, Held, one row per `good_rows` entry.
- `CollapsingHeader` "Resource types" (open): a striped `Grid` with columns Name, Produces, Output per deposit, Deposits.

**Hex panel (right)**, inside a `ScrollArea::vertical()`:
- World with no hexes: "This setting has no hexes."
- No selection: "Click a hex to inspect it."
- Otherwise: a heading `Hex (q, r)` (`HexCoord` Display); a "Deposits" subheading, then each `deposit_lines` line or "No deposits."; a "Stockpile" subheading, then for each `StockLine` its `good` line followed by its `parts` indented (`ui.indent`), or "Empty." if there are none.

**Map (`draw_map`)**: give it the selected hex and the deposits, and have it report a click. Suggested signature:
```rust
/// ... Returns Some(hex under the pointer, or None) if the canvas was clicked this frame.
fn draw_map(ui: &mut Ui, world: &World, view: &mut Option<View>, hovered: &mut Option<HexId>,
            selected: Option<HexId>) -> Option<Option<HexId>>;
```
The caller applies a click with `session.select(..)` after the panel closure (the world is borrowed from the session while drawing).
- **Click:** if `response.clicked()` and the pointer position is known, the result is `Some(hex_at(map, view, viewport, pointer))`: a hex (selecting it, also for dots within `HOVER_SNAP_PX`) or None (empty space clears). egui does not report a click for a drag, so panning never changes the selection.
- **Deposit marks:** for a visible hex with `!world.deposits().on_hex(id).is_empty()`, when the shape is `Hexagon` or `LabelledHexagon`: a filled circle in the foreground color at `center + (0, 0.55 · scale)` with radius `clamp(0.12 · scale, 2, 6)` px. It sits below the label so it never covers the coordinates. No mark for `Dot`.
- **Selected hex:** drawn in a second pass after the loop over all hexes, so neighbouring outlines don't paint over it, and only if it passes `is_visible`. Accent color: `visuals.selection.stroke.color`. Hexagon shapes: `Shape::closed_line` through `hex_corners(center, scale)` with a 3 px stroke. Dots: `circle_stroke` of radius 5 px, 2 px wide. A hex that is both hovered and selected keeps its hover fill and gets the outline on top.
- Everything else in `draw_map` stays as it is (pan, zoom, hover, culling, `f32` conversion only after `is_visible`).

**Status bar:** unchanged (it already shows the turn, which now changes).

## Tasks
1. In `check.rs`, make `turn_line` `pub(crate)` and update its doc. Run `cargo test -p policyspace-dev-ui` (all existing tests pass).
2. Create `inspect.rs` and declare it in `main.rs`. Write its unit tests.
3. Extend `session.rs` (log, step error, selection, `step`, `select`, `selected`, `STEP_MANY`) and its tests. Existing tests keep passing.
4. In `app.rs`: shortcuts at the start of the frame; Step and Step ×10 buttons.
5. In `app.rs`: the world panel and the hex panel.
6. In `app.rs`: click to select, deposit marks, and the selected-hex outline in `draw_map`.
7. Update the crate docs in `main.rs` and the module docs of `app.rs` and `session.rs`.
8. Launch the window by hand at least once (`cargo run -p policyspace-dev-ui -- settings/sample` from the repo root): select (0, 0), step, Step ×10, Space, Shift+Space, Esc. Close it. If the environment can't open a window, say so in the report.
9. Run `.venv/Scripts/python tools/check.py` (or `python tools/check.py`) and fix everything until it passes.

## Test Plan
Run everything with `.venv/Scripts/python tools/check.py`; `cargo test -p policyspace-dev-ui` runs this crate's tests alone. Load test settings from paths built on `env!("CARGO_MANIFEST_DIR")`, as the existing session tests do (`repo_path`).

**Pitfall (from Step 07): no test may run the `dev-ui` binary with no arguments or with a single folder argument.** That opens a window and hangs the run.

Unit tests in `inspect.rs`:
- `countdown`: `EndOfTurn(5)` at turn 5 → "perishes this turn"; at turn 4 → "perishes in 1 turn"; at turn 3 → "perishes in 2 turns"; `Never` → "never perishes". (Design: the countdown.)
- `lasts`: `After(1)` → "perishes after 1 turn"; `After(3)` → "perishes after 3 turns"; `Never` → "never perishes".
- `deposit_lines`: sample (0, 0) gives the two lines in the logic notes, in that order; a sample hex with no deposits (e.g. (2, 0), check it has none) gives an empty list. `turns_valid` (0, 0) gives `"field × 2: 2 daily each, 4 total"`, `"meadow × 1: 0.5 daily each, 0.5 total"`, `"pot × 1: 5 stew each, 5 total"`. (Design: hex panel deposits; acceptance criterion for (0, 0).)
- `stockpile_lines` on `turns_valid` stepped to turn 2:
  - (0, 0): `daily` "4.5 daily" with one part "4.5: perishes this turn"; `stew` "15 stew" with parts "5: perishes this turn", "5: perishes in 1 turn", "5: perishes in 2 turns", in good ID order (`daily` before `stew`).
  - (1, 0): "2 daily" with "2: perishes this turn"; "4.5 gold" with "4.5: never perishes".
  - (2, 0): empty.
  (Design: the `turns_valid` acceptance criterion.)
- `stockpile_lines` on the sample at turn 0, (0, 0): "2.5 unimproved_copper" and "12 unimproved_grain", each with one part "…: perishes this turn".
- `good_rows` on the sample: 5 rows in the setting's order; `unimproved_copper` held "10", `unimproved_grain` held "24", the other three "0"; `grain` lasts "perishes after 3 turns", movable "movable"; `unskilled_labor` movable "fixed"; `copper` "never perishes". After 3 steps the held values are the same.
- `resource_rows`: sample gives `copper_ore` (produces `unimproved_copper`, output "2.5", 4 deposits) and `arable_land` ("4", 6 deposits); `turns_valid` gives `field` 3, `meadow` 1, `pot` 1, `mine` 1.

Unit tests in `session.rs` (design: stepping, the log, Reload, selection):
- `open` on the sample: the log is exactly `["Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain"]`, `step_error()` is None, and `selected()` is None.
- `step(1)` returns 1, the world is at turn 1, the log has 2 lines, and the last is `"Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain"`. `step(STEP_MANY)` returns 10: turn 11, 12 lines, the last starting `"Turn 11: "`.
- Every log line equals what `check`'s `--turns` prints for the same turn: compare with `turn_line` on a separately built and stepped `World`, for `turns_valid` over 4 steps.
- A new session's `step(1)` returns 0 and the log stays empty; a session holding errors (`many_errors`) likewise.
- `reload` after stepping: turn 0, log of 1 line, `step_error` None.
- Selection:
  - `select(map.id((0,0)))` on the sample, then `selected()` is that hex; `reload` keeps it; `select(None)` clears it.
  - `open` (of the same or another folder) clears it.
  - In a `tempfile` setting (as in `reload_sees_edits_on_disk`) with hexes (0, 0) and (1, 0): select (1, 0); remove (1, 0) from `map.toml` and reload → `selected()` None; add it back and reload → still None (it was cleared).
  - Same tempfile: select (0, 0); break `setting.toml` and reload → errors, `selected()` None; fix it and reload → `selected()` is (0, 0) again.
  - `select(Some(..))` while holding errors leaves `selected()` None.
- The refused-step path (`StepError::ClockAtMaximum`) can't be reached from the dev UI (the engine offers no way to put a world at the maximum turn), so it isn't tested here; keep its code to the simple loop above. Say so in the report.

Existing tests: every test in `check.rs`, `cli.rs`, `geometry.rs`, `session.rs`, `tests/checker.rs`, and `tests/cli.rs` keeps passing unchanged (design: `check` mode unchanged).

Manual checks at Code Review (design: the window's Acceptance Criteria), run by the user:
- `dev-ui settings/sample`: click (0, 0) → thick accent outline; hex panel shows `Hex (0, 0)`, the two deposit lines, and the stockpile with "perishes this turn". Esc and clicking empty space clear it; dragging doesn't change it. Hexes with deposits show marks.
- Step, Step ×10, Space, Shift+Space: the log grows newest first, the status bar's turn matches the newest entry. Pressing Space after clicking Step steps exactly once.
- World panel: 5 goods with properties and held totals, 2 resource types with deposit counts.
- `turns_valid`: at turn 2, (0, 0) shows 15 `stew` split 5/5/5; (1, 0)'s `gold` grows 1.5 per step, "never perishes"; (2, 0) shows "No deposits." and "Empty."
- Reload after stepping: turn 0, one log line, selection kept. Remove the selected hex from the setting and Reload: selection cleared. Open another folder: selection and log cleared.
- `hexes_valid` fitted (dots): clicking near a dot selects it. `valid_minimal`: world panel shown, hex panel says "This setting has no hexes.", Step works. A broken setting: no side panels and Step buttons disabled.

## Constraints
- Do not modify `crates/engine/`, `settings/`, `tools/`, `geometry.rs`, `cli.rs`, or the files in `crates/dev-ui/tests/`. No new dependencies.
- `check` output stays byte-for-byte the same.
- `session` and `inspect` must not use egui or eframe types.
- Never keep a `HexId` across a load: the selection is stored as `HexCoord`, and `hovered` is reset on Open and Reload as today.
- No `unsafe`, no `#![windows_subsystem = "windows"]`, and no `HashMap` iteration in anything that decides what's shown or in what order.
- A doc comment on every item, following the existing style.
- Out of scope: auto-run, stepping by an arbitrary number, coloring hexes by data, map marks other than deposits, per-hex log breakdowns, saving the log, editing the world, multi-select, per-deposit inspection, automated tests that drive the window.

## Done When
All tasks are complete, every test in the Test Plan passes, `tools/check.py` passes, and every Acceptance Criterion in design.md is met (the window's criteria by the manual checks at Code Review).
