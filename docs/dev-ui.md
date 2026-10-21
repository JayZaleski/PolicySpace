# The developer UI

`policyspace-dev-ui` (binary `dev-ui`, in `crates/dev-ui/`) is a tool for developers and setting authors, not a player UI. It depends on the engine by path and uses only the engine's public interface (`load_setting`, `LoadedSetting`, `LoadErrors`, `World` with its `step`, `market`, `hex_currency`, `pops`, `pop_stockpile` and `pop_outcome`, `TurnReport`, `StepError`, `Stockpile`, `PerishesAt`, `Perishes`, `Quantity`, `Turn`, `HexMap`, `HexCoord`, `HexId`, `PopId`, `ENGINE_NAME`, `ENGINE_VERSION`). It also uses the decider host: `DeciderHost`, `ReportedFailure` and `DeciderFailure`, and `decider::python` (`host_for`, `HostError`) to start the setting's deciders; its tests add `ClosureDecider`, `Reply` and `PythonError` (see [decider-host.md](decider-host.md)). For businesses and factories it reads `World::businesses` (with `on_hex`), `World::factories`, `Deposits::owned_by`, `World::business_stockpile`, `World::business_answer` (a `BusinessAnswer`), `World::business_outcome` (its `BusinessTrade` and per-factory `FactoryRecord`s) and the rules' technologies, and it calls the engine's `slices_held` to count a factory's remaining life (see [production.md](production.md#lifetime-slices-and-shutdown)). The engine knows nothing about it. The window is built with egui/eframe 0.36 (default features, which use the wgpu renderer), and the system folder picker comes from rfd 0.17. These are dependencies of this crate only.

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
| `check.rs` | The headless checker only: `run` and `run_turns`. `run_turns` starts the setting's deciders and prints each turn's line and its decider failures. Its wording comes from `text`. |
| `text/mod.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `load_failure_lines`, `count`, `turn_line`, `failure_lines`) and the window's panels (countdowns, deposit lines, stockpile lines, market lines, POP sections, goods, resource-type and POP-type rows, and the index, rate and share formatting), built from engine types. No egui types. Re-exports the window-facing functions of `text/businesses.rs`. |
| `text/businesses.rs` | The business, factory and technology text: `business_section` (a hex's business sections), `technology_rows` and `business_rows` (the world panel's tables), and the helpers they use (`remaining_life`, `format_life`, `business_deposit_lines`, `business_stockpile_lines`). A child module of `text`, so it reaches `text`'s private helpers as `super::` items. No egui types. |
| `geometry.rs` | Pure map math: hex to world to screen, fitting, zooming, culling, and finding the hex under a point. No egui types. |
| `session.rs` | `Session`, `Loaded` and `Hosting`: the current folder, what loading it gave, the decider host of the loaded world, the turn log, and the selected hex. Steps the world. No egui types. |
| `app.rs` | The eframe window: `run`, the panels, keyboard shortcuts, and the map canvas. Uses `geometry`, `session` and `text`. |
| `test_support.rs` | `#[cfg(test)]` helpers shared by the unit tests: `repo_path(relative)` (a path from the repository root); `world_with(relative, &mut host)` (loads a setting and returns its world at turn 0 under `host`, panicking with the load errors or an unknown decider name); `world_at(relative)` (`world_with` under the idle host); and `python_world(relative)` (the world under the setting's own Python deciders from `python::host_for`, returned with its host so a test can step it; tests using it need the project's virtual environment). |

Everything except `app.rs` is unit-tested without a window. The window is checked by hand.

## Loading and stepping: `Session`

`Session` holds:

- `hosting: Hosting`: how each load builds its decider host;
- `folder: Option<PathBuf>`;
- `loaded: Option<Loaded>`;
- `host: Option<DeciderHost>`: the host of the loaded world, `None` whenever `loaded` isn't a world;
- `log: Vec<String>`: every turn line since the last load, oldest first, each followed by its decider failure lines;
- `step_error: Option<String>`: the error from the last refused step since the last load;
- `selected: Option<HexCoord>`: the selected hex.

`Hosting` is `FromSetting` (one Python process per decider name the setting's businesses carry, through `python::host_for`; the window uses it), and, in tests only, `Idle` (`DeciderHost::idle()`) and `Custom(HostBuilder)` (a boxed `Fn(&LoadedSetting) -> Result<DeciderHost, HostError>`). `Idle` and `Custom` are `#[cfg(test)]` because nothing outside the tests builds them yet; Stage 05 lifts that when the dev UI becomes a decider. `Session::new(hosting)` takes it, and `Session` has no `Default`. The app calls `Session::new(Hosting::FromSetting)`.

`Loaded` is `World(World)` (the setting loaded; the world is at turn 0 after a load and later after steps) or `Errors(Vec<String>)`. `Session::world()` returns the loaded world, or None when nothing is loaded or the setting has errors; use it wherever only the world case matters.

**Loading.**

- `Session::open(folder)` makes `folder` current and loads it, replacing whatever was loaded. `reload()` loads the current folder again from disk, and does nothing if no folder was chosen.
- Loading first drops the previous host, which ends its processes, then calls `load_setting`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `text::error_lines(&errors, &folder)`, and the log is emptied. On success it builds the host from `hosting` and then `World::new(setting, &mut host)` (at turn 0, with turn 0's decision point, trade round, factory operation and growth already run). The log becomes `text::turn_line` of turn 0's `TurnReport`, followed by `text::failure_lines` for each of its decider failures, the host is kept, and the result is `Loaded::World(world)`.
- If the host can't be built (`HostError`: the Python environment is missing, the time-limit variable is malformed, or a decider exits as soon as it starts, such as for a name the Python package doesn't know), or `World::new` refuses a decider name (`UnknownDecider`), the load fails like a broken setting: `Loaded::Errors(text::load_failure_lines(&error.to_string(), folder))`, with the log emptied and no host. The window shows these lines in its error view.
- Either way `step_error` is cleared: only a load clears it.
- A failed load replaces the previous world, so nothing from an earlier load stays on screen. A folder that fails to load is still the current folder, so Reload retries it.
- Loading is synchronous on the UI thread, and the folder picker is rfd's blocking dialog. Settings load in milliseconds; starting the Python deciders takes a moment longer.

`text::error_lines` returns each error's `Display` in order, then `{n} error(s) found in {folder}`. `check::run` prints these lines to stderr and the window shows the same lines, so both always agree.

**Stepping.** `step(times)` calls `World::step(&mut host)` up to `times` times and pushes `text::turn_line` of each report onto the log, followed by `text::failure_lines` for each decider failure in it, so a failing script shows in the turn log as soon as it fails. A decider failure never stops stepping. It stops at the first refused step and stores that `StepError`'s `Display` in `step_error`. It does nothing without a world (and its host) and returns the number of steps taken. `STEP_MANY` (10) is the count the Step ×10 button and Shift+Space use. The engine offers no way to put a world at the maximum turn, so the refused-step path can't be reached from the dev UI and is not tested. The log stores formatted lines rather than reports, so it matches `check --turns` word for word and the window needs no formatting. It is never trimmed: it grows by one line per step until the next load.

**Selection.** The selection is stored as a `HexCoord`, never a `HexId`, because a `HexId` is an index into one map and may name a different hex (or none) after a reload, and `Deposits::on_hex`, `Pops::on_hex`, `Businesses::on_hex` and `World::hex_stockpile` panic on an out-of-range id.

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

`text` builds every string the dev UI shows, without egui types, so it is unit-tested without a window. It is a directory module: `text/mod.rs` holds the `check` output and the hex, market, POP and world-table text, and `text/businesses.rs` the business, factory and technology text (see [Businesses, factories and technologies](#businesses-factories-and-technologies)). The child module re-exports only what `app.rs` uses (`business_section`, `business_rows`, `technology_rows`), because an unused re-export in this binary crate is an `unused_imports` warning and the check command runs clippy with `-D warnings`; its other items stay `pub` inside the module for its tests. The `check` command and the window share the load-error, count and turn-line functions, so they always use the same wording. Every list comes out in ID order (goods, resource types, deposits, prices, the POPs and businesses on a hex, the goods in a POP's table or a business's trade table, factories, technologies, POP types, businesses), straight from the engine's ordered accessors, so the panels are deterministic.

**Numbers.** Sizes and amounts are written in full with `Quantity`'s `Display` (the f64 `Display`), as in `check --turns`; nothing is rounded. The business text follows the same rule: fractions, conditions, wear and the cash-cap proportion are written with f64 `Display`, so a value such as the arithmetic decider's 0.12499999999999989 tools appears as sent. Only three ratios are rounded for display, by three public helpers:

- `format_index(index)`: the subsistence index to 4 fixed decimals: `0.6667`, `1.0000`, `0.0000`, `-1.0000`.
- `format_rate(rate)`: the growth rate as a signed percentage to 2 fixed decimals: `+2.67%`, `-10.00%`, `+0.00%`.
- `format_share(share)`: a normalized utility share rounded to 4 decimals, written with f64 `Display` so trailing zeros drop: `0.8`, `0.1`, `0.3333`, `1`, `0`.

Each of them, and the supernumerary income in a POP's demand line, adds `0.0` to the value before formatting: that turns `-0.0` into `0.0`, which would otherwise print as `-0.0000`, `-0.00%` or `-0`. Negative numbers use the ASCII `-`, like coordinates and turn lines. Where the rounded value hides detail, the window shows the full f64 value as hover text (the growth line and each utility line).

Shared with `check`:

- `error_lines(errors, folder)`: each error's `Display`, then `{n} error(s) found in {folder}`.
- `load_failure_lines(message, folder)`: the lines of a setting that loaded but whose deciders couldn't start (or whose decider name the host doesn't hold): `message`'s lines (a `HostError` or `UnknownDecider` `Display`, which can span several lines when it quotes a script's error output), then `1 error(s) found in {folder}`, so the error view and the checker read as for a broken setting. The status bar counts the lines before the last, so a message that quotes error output shows more than `1 error(s)` there.
- `count(n, singular, plural)`: `1 hex`, `19 hexes`.
- `turn_line(world, report)`: `Turn {n}: perished {list}; emitted {list}; endowed {list}; sold {list}; bought {list}; consumed {list}; population {n}; businesses sold {list}; businesses bought {list}; produced {list}; used {list}`, where a list is `<amount> <good>` joined by `, ` or `nothing`. `endowed` is what POPs received at the start of the turn, `sold`, `bought` and `consumed` are the POPs' totals from the turn's trade round, and `population` is the total POP size after growth, printed with `Quantity`'s `Display` (`0` for a world without POPs). `businesses sold` and `businesses bought` are the businesses' totals from the same round, and `produced` and `used` the factories' output and the inputs and maintenance goods they consumed (see [world.md](world.md#turnreport), [trade.md](trade.md) and [growth.md](growth.md)). The report's `decisions` are not printed.
- `failure_lines(turn, failure)`: the lines logged for one decider failure: `Turn {turn}: decider '{name}' failed: {failure}` (the `DeciderFailure`'s one-line `Display`), then each line of its error-output tail as `    stderr: {line}`. For example `Turn 7: decider 'x' failed: no reply within the time limit of 2.5 s`. The log and `check --turns` print them right after the turn's line.

For the panels:

- `countdown(PerishesAt, now: Turn)`: for `EndOfTurn(t)` with `n = t - now` (saturating), "perishes this turn" (0), "perishes in 1 turn", or "perishes in {n} turns"; `Never` gives "never perishes".
- `lasts(Perishes)`: "perishes after 1 turn", "perishes after {n} turns", or "never perishes".
- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add` rather than multiplied, the same way emission builds a stockpile by adding each deposit's output in turn. The lines list every deposit on the hex, owned or not; when a business owns some of them, the hex stockpile holds only the unowned deposits' output, so it is less than the line's total. The wording lives in the private `deposit_lines_from(world, count_of)`, which writes a line for each resource type whose `count_of(type)` is above 0; `deposit_lines` passes the count on the hex, and `business_deposit_lines` the count a business owns, so both read the same.
- `stockpile_lines(world, hex)` and `pop_stockpile_lines(world, pop)`: one `StockLine { good, parts }` per good held, in good ID order. `good` is `{total} {good}`; `parts` has one `{amount}: {countdown}` per perish group, soonest first. Both wrap the private `stock_lines(world, &Stockpile)`, so a POP's stockpile is written exactly like a hex's. `business_stockpile_lines(world, business)` wraps it too, so hex, POP and business stockpiles all read the same. Use `stock_lines` for any future stockpile.
- `good_rows(world)`: one `GoodRow { name, lasts, movable, in_hexes, in_pops, in_businesses }` per good (`movable` is "movable" or "fixed"). `in_hexes` sums the good over every hex stockpile (the unowned output of deposits), `in_pops` over every POP stockpile (what POPs own), and `in_businesses` over every business stockpile (what businesses own: their owned deposits' output, their cash, what they bought and their factories' output). The three are kept apart because they are different things. Each is computed in one pass (over `world.map().iter()`, `world.pops().iter()` and `world.businesses().iter()`), adding each stockpile's totals into a per-good array with `Quantity::saturating_add`. A world without POPs has `"0"` in every `in_pops`, and one without businesses `"0"` in every `in_businesses`.
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

### Businesses, factories and technologies

`text/businesses.rs` builds the business sections of the hex panel and the world panel's Technologies and Businesses tables. Like the POP text, it reads the world as it stands between steps, after part 2 of the current turn: each business's stored answer (`World::business_answer`) and its outcome (`World::business_outcome`) are this turn's, the factory records hold this turn's run, wear and maintenance, and the stockpile is after operation (this turn's output, held to sell next turn).

- `remaining_life(condition, lifetime) -> Option<u32>`: the number of further turns a factory can run with no maintenance. It is `slices_held(condition, lifetime).floor()` when at least one slice is held, else `None` (shut down). It calls the engine's own rule, which snaps within `TOLERANCE`, rather than copying it, so the panel can never say a factory runs when the engine won't run it, or the reverse. It is applied to the condition *after* this turn's wear, so it counts the turns after the current one: a factory whose last output was this turn already reads shut down.
- `format_life(life)`: `shut down`, `1 turn` or `{n} turns`.
- `business_deposit_lines(world, business)`: the business's owned deposits (`world.deposits().owned_by(business)`, counted per resource type), worded exactly like `deposit_lines`; empty if it owns none.
- `business_stockpile_lines(world, business)`: `stock_lines` of the business's stockpile.
- `business_section(world, business) -> BusinessSection`: everything the hex panel shows for one business:
  - `heading`: `{name}: {cash} {currency}`, the cash being the business stockpile's total of its hex's currency (`center_farm: 85 silver`).
  - `decider`: `Decider: {name}`.
  - `answer`, `refused` and `errors`: `Answer: valid decision`, `Answer: refused, {n} error(s)` with each `DecisionError`'s `Display` in `errors` (`{place}: {message}`, as the engine reports it), or `Answer: nothing (no decision was sent, or its decider failed)`.
  - `deposits`: `business_deposit_lines`.
  - `cash`, `proportion` and `cash_short`: `Cash: {before} {currency} before; {after sales} after sales; {after purchases} after purchases;` and `proportion {p}`, from the outcome's `BusinessTrade`; `cash_short` is `p < 1`, which means the cash cap scaled the bids (see [trade.md](trade.md#the-cash-cap)).
  - `trade`: one `TradeRow { good, offered, sold, bid, bought }` per good, merging the decision with what happened, so a cut shows as asked above done in one row. For a valid answer the goods are those the `BusinessTrade` offered or bid, in good ID order, and the cells are its `offered_of`, `sold_of`, `bid_of` and `bought_of` (offered and bid are the decision's amounts as sent). A refused decision has no trade, so its rows come from the stored `Decision`'s `offers` and `bids` maps, with sold and bought `0`: names the goods list knows come first in good ID order, then unknown names (a misspelt good) in name order, so what the decider asked for is always visible. A business that decided nothing has no rows.
  - `factories`: one `FactoryRow` per record in `outcome.factories()` (factory ID order): `number` (the factory's ID number, the one a decision addresses it by), `technology`, `condition_before`, `condition_after`, `remaining_life` and `shut_down`, `run_asked`, `run_done`, `output` (`{amount} {good}`), `wear`, `maintenance_asked` and `maintenance_done`, plus three hover texts: `inputs` (`Inputs: {list}`, the goods the run took), `maintenance_cost` (`Maintenance cost: {list}`) and `capital` (`Capital: {list}`, the technology's capital bundle scaled by the condition after, which is the capital the factory holds, since capital only ever changes in the bundle's own proportions). The engine already records run and maintenance 0 for a factory the decision didn't mention and for every factory of a business that decided nothing or was refused, so the rows need no special case.
  - `trace`: the valid or refused decision's trace as sent, or `None`. The engine stores it and never acts on it.
  - `stockpile`: `business_stockpile_lines`.
- `technology_rows(world)`: one `TechnologyRow { name, recipe, output, capacity, capital, construction, construction_turns, lifetime, factories }` per technology, in technology ID order. `recipe` is one `{amount} {good} per unit` per input (or `["nothing"]`), `capital` and `construction` one `{amount} {good}` per good (`construction` may be `["nothing"]`), and `factories` the number of factories running it, counted in one pass over `world.factories().iter()`.
- `business_rows(world)`: one `BusinessRow { name, hex, decider, cash, factories, answer, refused }` per business, in business ID order. `hex` is the coordinates (`(0, 0)`), `cash` as in the section heading, `factories` the number it owns (one pass over the factories), and `answer` `nothing`, `valid` or `refused`.

## The window: `app`

`app::run(folder)` starts `eframe::run_native` with a 1280 × 800 window. If the window can't be created it prints `error: could not open the window: {error}` and returns exit code 1. `DevApp` holds `session`, `view: Option<View>` (None means "fit on the next draw", because the canvas size is only known while drawing), `hovered: Option<HexId>`, and `last_title` (so the title command is sent only when the title changes). It holds no selection state: that lives in `Session`. The view lives in the app, not the session, so Reload never touches it: after a successful reload, even with a failed load in between, the map stays at the same pan and zoom. Open and Fit set `view` to None; Open and Reload reset `hovered`.

Each frame (`eframe::App::ui`) runs in this order:

1. **Shortcuts** (`DevApp::shortcuts`), only when a world is loaded, before anything is drawn. Inside one `ui.input_mut`, it consumes `Shift+Space`, then `Space`, then `Esc`. Both Space keys are consumed every frame, even when the first matched: egui presses a focused button on Space, so a Space left in the queue would press a focused Step button as well as trigger the shortcut. Shift+Space is consumed first because `consume_key` ignores extra Shift. Shift+Space steps `STEP_MANY` times, Space steps once, Esc clears the selection. Because a held key repeats, holding Space keeps stepping.
2. **Top panel:** **Open setting…** (rfd's `pick_folder`; cancelling changes nothing), **Reload** (enabled when a folder is chosen), **Fit**, then **Step** (hover text "Space") and **Step ×10** (hover text "Shift+Space"), and the folder path. Step and Step ×10 are enabled whenever a world is loaded, including one with no hexes. Fit needs a world with at least one hex.
3. **Status bar:** with a world, `{name} · {n} hexes · turn {turn}` plus ` · hex (q, r)` while a hex is hovered (using `text::count` for the wording); with errors, `{n} error(s)`; otherwise `No setting loaded`. The turn changes as the world steps.
4. **Side panels** (only when a world is loaded, and shown before the central panel): `Panel::left("world")` and `Panel::right("hex")`, both resizable with a minimum of about 200 px (`SIDE_PANEL_MIN_WIDTH`). The world panel starts at about 320 px (`SIDE_PANEL_WIDTH`) and the hex panel at about 460 px (`HEX_PANEL_WIDTH`), wide enough for a POP's seven-column goods table at the sample's numbers.
5. **Central panel**, by state: no folder shows "No setting loaded." with an Open button; errors show a heading and every line in selectable monospace text inside a scroll area; a world with no hexes shows "This setting has no hexes."; otherwise `draw_map`. A click result from `draw_map` is applied with `session.select(..)` after the panel closure, because the world is borrowed from the session while drawing. The status bar and central panel match on `session.loaded()` because they also handle the errors and nothing-loaded cases; the other checks use `session.world()`.
6. **Title:** `{ENGINE_NAME} {ENGINE_VERSION} — Dev UI`, with ` — {setting name}` once a world is loaded.

**World panel (left)**, in one vertical `ScrollArea`, six collapsible sections (all open by default):

- *Turn log*: the refused-step error first, if any (in the theme's `error_fg_color`), then the log newest first in an inner `ScrollArea::both()` (id salt "turn log", max height 320 px) drawn with `show_rows` at one body-text line per row. Rows are non-wrapping labels, so every row has the same height and thousands of lines stay smooth.
- *Goods*: a striped `Grid` (Name, Lasts, Movable, In hexes, In POPs, In businesses) from `text::good_rows`.
- *Resource types*: a striped `Grid` (Name, Produces, Output per deposit, Deposits) from `text::resource_rows`.
- *POP types*: a striped `Grid` (Name, Endowment, Utility, Growth, POPs, Total size) from `text::pop_type_rows`. The Endowment and Utility cells are `ui.vertical` stacks with one label per line; each utility label has the full share as hover text.
- *Technologies*: a striped nine-column `Grid` (Name, Recipe, Output, Capacity, Capital, Construction, Construction turns, Lifetime, Factories) from `text::technology_rows`. The Recipe, Capital and Construction cells are `ui.vertical` stacks with one label per line.
- *Businesses*: a striped six-column `Grid` (Name, Hex, Decider, Cash, Factories, Answer) from `text::business_rows`. A refused business's row draws all six cells in the theme's `error_fg_color`, so a refusal anywhere in the world shows without clicking hexes. The turn line doesn't print the report's decision counts, because this table shows the same thing per business.

**Hex panel (right)**, in a `ScrollArea::both()`, so a wide goods table scrolls sideways instead of wrapping: for a world with no hexes, "This setting has no hexes."; with no selection, "Click a hex to inspect it."; otherwise, in order:

- the heading `Hex (q, r)`;
- "Deposits": `text::deposit_lines`, or "No deposits.";
- "Stockpile": the hex's stockpile;
- "Market": one label per line of `text::market_lines`;
- "POPs": "No POPs." if `world.pops().on_hex(hex)` is empty; otherwise one `CollapsingHeader` per POP, in POP ID order, open by default, headed by `PopSection::heading`. Inside it: the growth line (with `growth_hover` as hover text), the demand line, a striped seven-column `Grid` (Good, Held, Demanded, Subsistence, Sold, Bought, Consumed; id salt `("pop goods", pop index)`), the carried-over line, and a "Stockpile" subheading with the POP's stockpile. A row with `below_subsistence` draws all seven cells with `ui.colored_label` in the theme's `error_fg_color`, so a POP below subsistence and the good it lacks stand out.
- "Businesses": "No businesses." if `world.businesses().on_hex(hex)` is empty; otherwise one `CollapsingHeader` per business, in business ID order, open by default, headed by `BusinessSection::heading` (name and current cash). Inside it, in order:
  1. the decider line and the answer line, then each validation error; the answer line and the errors are in the error color when the decision was refused;
  2. "Deposits": the owned deposit lines, or "Owns no deposits.";
  3. one row with the cash line and the proportion, the proportion in the error color when `cash_short`;
  4. "Trade": a striped five-column `Grid` (Good, Offered, Sold, Bid, Bought; id salt `("business trade", business index)`), or "Nothing was traded." when there are no rows;
  5. "Factories": a striped eleven-column `Grid` (#, Technology, Condition before, Condition after, Remaining life, Run asked, Run done, Output, Wear, Maintenance asked, Maintenance done; id salt `("business factories", business index)`), or "No factories.". Remaining life is in the error color when the factory is shut down. Condition after has the capital as hover text, Output the inputs, and Maintenance done the maintenance cost;
  6. "Trace", when the decision carried one: a `CollapsingHeader` (id salt `("business trace", business index)`), collapsed by default, holding the trace as selectable monospace text;
  7. "Stockpile": the business's stockpile.

  The error color marks only what carries information: a POP below subsistence, a refused decision, a cash-short proportion and a shut-down factory.

Each repeated `CollapsingHeader` and `Grid` gets an explicit id salt from its item's index: `("pop", pop index)` for a POP's section, and `("business", business index)` for a business's, with the business's trade grid, factories grid and trace header salted the same way. egui keys a header's open/closed state by its ID, which defaults to the heading text; the headings hold the POP's size and the business's cash, which change every step, so without the salt a collapsed section would reopen after each step.

Every stockpile is drawn by the free function `stockpile_listing(ui, salt, lines)`: "Empty." for no lines, else each `StockLine.good` with its `parts` under `ui.indent(salt, ..)`. The hex passes the salt `"hex"`, a POP `("pop", pop index)` and a business `("business", business index)`, so the indents of different stockpiles don't share an ID, and they all look identical.

The world between steps is after part 2 of the current turn (see [world.md](world.md)): every business has been served its view, answered, traded and operated its factories, every POP has been endowed, has traded, consumed and grown, and nothing has perished yet. So the POP and business sections show the current turn (the turn in the status bar): each POP's trade outcome, its size after growth and its stockpile after consumption; each business's answer at that turn's decision point, the trade and factory operation it led to, each factory's condition after that turn's wear and maintenance, and the business's stockpile after operation. Both panels read the world directly each frame, so they update straight after a step or reload.

**`draw_map`** allocates the remaining area (click-and-drag sense), builds a `Viewport`, fits the view if it is None, and then:

1. Dragging pans. Scrolling over the canvas zooms toward the pointer by `exp(scroll_y · 0.0015)`, and egui's pinch zoom is applied too.
2. While the pointer is over the canvas and not dragging, `hovered = hex_at(...)`.
3. If the canvas was clicked and the pointer position is known, it returns `Some(hex_at(...))`: `Some(Some(hex))` selects (including dots within `HOVER_SNAP_PX`), `Some(None)` is a click on empty space and clears the selection. egui does not report a click for a drag, so panning never changes the selection.
4. It walks `map.iter()` once per frame, skips hexes that fail `is_visible`, and paints the rest with `paint_hex` by `hex_shape()`: a dot, a 1 px hexagon outline (`Shape::closed_line`), or an outline plus the label `q, r`. `paint_hex` takes the hex's screen center as a geometry `Point` (`f64`), computes the hexagon corners from it, and converts to egui's `f32` position only for painting. The hovered hex is filled with the theme's selection color. Colors come from the current egui visuals.
5. **Marks:** a visible hex drawn as `Hexagon` or `LabelledHexagon` can get three marks, all in the foreground color, sized by one radius of `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels (the cap keeps them inside the hexagon at small scales), and all placed on a ring of radius `offset = 0.55 · scale` around the center, off the label so they never cover the coordinates. A hex with at least one deposit (`HexLook::has_deposits`, from `!world.deposits().on_hex(id).is_empty()`) gets a filled circle at `center + (0, offset)`, below the label. A hex with at least one POP (`HexLook::has_pops`, from `!world.pops().on_hex(id).is_empty()`) gets a hollow circle (`circle_stroke`, 1 px) at `center − (0, offset)`, above the label. A hex with at least one business (`HexLook::has_businesses`, from `!world.businesses().on_hex(id).is_empty()`) gets a filled square of side `2 · radius` (`rect_filled`) at `center + (−0.866 · offset, 0.5 · offset)`, the lower left of the label, 60° round the ring from the deposit mark. A hex can show any combination of the three. Dots get no marks. The map draws no other data: no coloring by population or anything else.
6. **Selected outline:** after the loop, in a second pass so neighbouring outlines don't paint over it, the selected hex is outlined in `visuals.selection.stroke.color` if it passes `is_visible`: a 3 px closed line through `hex_corners` for hexagons, or a 5 px-radius, 2 px circle stroke for dots. A hex that is hovered and selected keeps its hover fill with the outline on top.

One pass with culling is enough for maps of around 10,000 hexes; there is no caching.

## Extending the window

- **New panel text:** add a pure function to `text` that takes a `&World` and returns strings or small row structs, unit-test it, and render it in `world_panel_contents` or `hex_panel_contents` in `app.rs`.
- **New per-hex drawing:** add it to the loop in `draw_map` after the `is_visible` check, so it is only done for hexes on screen. Anything that must not be overpainted by neighbours (like the selection outline) goes in a pass after the loop.
- **New state tied to a world** (for example a log): put it in `Session`, reset it where loading happens, and decide whether `open` and `reload` keep it. Never store a `HexId` across a load; store the `HexCoord`.
- **New keyboard shortcuts:** consume them in `DevApp::shortcuts`, which runs before any widget is drawn, and consume every key a focused widget would also react to.
- Keep new map math in `geometry` (in `f64`, without egui types) so it can be unit-tested.
- **New hex-panel sections:** add them after "Businesses" in `hex_panel_contents`, built by a `text` function the way `pop_section` and `business_section` are. If `text/mod.rs` grows too long, put the new text in a child module beside `text/businesses.rs`, and re-export from `text` only what `app.rs` uses. Give any repeated `CollapsingHeader` or `Grid` an explicit id salt from the item's ID, never from text that changes between steps. Reuse `stock_lines` (through a public wrapper) and `stockpile_listing` for any stockpile, so every stockpile reads the same. Use color only where it carries information, from the theme's visuals (`error_fg_color`), never a hard-coded `Color32`.
- **New map marks:** add a field to `HexLook`, fill it in `draw_map`, and paint it in `paint_hex`'s hexagon arm, using the shared `offset` and `radius`. The marks sit on a ring around the label: the deposit mark below, the POP mark above, and the business mark at the lower left; the other three 60° positions on the ring are free.
- **A different source of deciders** (Stage 05's dev UI as a decider): add a `Hosting` variant, or use `Custom` without its `#[cfg(test)]`, and build the host in `Hosting::build`. Keep the host in the session beside the world, and drop it whenever the world is replaced, so its processes end.

## Tests

- Unit tests in `cli.rs` (every accepted and rejected argument form), `text/mod.rs`, `text/businesses.rs`, `geometry.rs` (layout, round trips at the `i32` extremes, fitting, zoom and pan, shape thresholds, `hex_at` including dots, and culling), and `session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, the sample's exact turn 0 and turn 1 lines under the idle host (ending `; businesses sold nothing; businesses bought nothing; produced nothing; used nothing`), selection kept or cleared across reload and open, all with `Hosting::Idle`; a closure decider's failure logged at turn 0 and again at every step; a `Custom` host error shown as a load error with an empty log and no world; and, with the project's Python, the sample loading and stepping under its `arithmetic` decider with no failure lines and its turn 0 line showing the businesses' trade and production, and `production_valid`, whose businesses name `rule`, failing to load with the script's `unknown decider 'rule'` message). They load settings through `test_support`. The `text/mod.rs` tests cover:
  - `error_lines` for `many_errors`, countdown and `lasts` wording, and deposit and stockpile lines on the sample (the center hex's stockpile at turn 0 is only `4 unimproved_grain`, `4: perishes this turn`, its unowned deposit's output) and `turns_valid`;
  - goods' `in_hexes` totals on the sample (at turn 0 `0, 7.5, 16, 0, 0, 0, 0` in good ID order, only unowned deposit output; the same after steps; `tools` reads `0`, since nothing holds it) and `turns_valid`; `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`; and `in_businesses`: on the sample under the idle host `0, 2.5, 8, 0, 0, 350, 0` at turn 0 and the per-good sums over every business stockpile after steps, under the `arithmetic` decider (with the project's Python) `12` grain, `2.5` copper and the per-good sums for silver and tools, and all `"0"` on `valid_minimal` and `turns_valid`;
  - resource-type deposit counts;
  - `market_lines` on the sample (hex (0, 0)'s prices, ending with `tools: 12 silver`, and `No market.` on (2, -2)) and on `markets_valid` (prices, `No prices posted.`, `No market.`);
  - `format_index`, `format_rate` and `format_share`, including `-0.0` and exact subsistence;
  - `pop_section` for the sample's center peasant and artisan and the no-market north-east peasant (heading, growth, demand, every goods row, the below-subsistence flag on its grain row, carried-over and stockpile), after one step (the starting size is the previous size), for `pops_growth`'s POP at exact subsistence (`index 0.0000; rate +0.00%`), and the growth hover's full values;
  - `pop_type_rows` on the sample (endowments, normalized shares, growth parameters, counts, and total sizes adding up to the turn report's population) and on `pops_valid` (an empty endowment reads `nothing`);
  - `pop_stockpile_lines` on `pops_placed` after one step, written exactly like a hex stockpile;
  - `failure_lines` with and without an error-output tail, and `load_failure_lines` splitting a message into lines before the summary line;
  - `turn_line` with a hand-built `TurnReport` whose business lists are not empty, checking the four business parts and their order after `population`.

  The `text/businesses.rs` tests cover:
  - `remaining_life` at its boundaries (exactly one slice, a hair under one, zero, lifetimes 3 and 24 whose slices aren't exact in floating point) and `format_life`'s wording;
  - remaining life agreeing with the engine: on `business_trade` under a closure decider that runs `works`'s two factories with no maintenance, stepped five times, each factory reads `n turns` for exactly as many further turns as it then produces output, and `shut down` from the turn of its last output;
  - the sample's three center businesses under the real `arithmetic` decider (with the project's Python) at turn 0: headings, decider and answer lines, a non-empty trace, owned deposit lines, the cash lines and proportion 1, every trade row, every factory row with its hover texts, and the stockpile; and after one step, the output sold and the new cash;
  - the same businesses under the idle host: answer nothing, no trace, no trade rows, factories at run and maintenance 0 with only their wear;
  - a refused decision (`business_trade` with an order for a factory the business doesn't own): the error lines, the trade rows with what was asked and 0 done, and the factories idle; and a refused decision naming a misspelt good, whose row comes after the known goods, with its trace shown;
  - a cash-short decision on `business_trade`: proportion `0.2`, the cut purchases, and the run done below the run asked;
  - `business_deposit_lines` and `business_stockpile_lines`, `technology_rows` on the sample, `business_trade` (`bakery`'s empty construction reads `nothing`, `smelter` has two factories) and `valid_minimal` (empty), and `business_rows` under both hosts and on `valid_minimal` (empty).

  Numbers that come from floating-point arithmetic (sizes such as 61.6, the carried-over sliver, wear and conditions at lifetime 24, the decider's 0.12499999999999989 tools) are compared against the engine's own value formatted with `Display` rather than a literal, so the tests check the wording and the source of the number, not the last digits.
- `crates/dev-ui/tests/checker.rs` runs `dev-ui check ...` against the sample and broken settings, including `--turns`. Every expected success line includes the markets, POP types, POPs, technologies, businesses and factories counts (`18 markets, 2 pop types, 16 pops, 3 technologies, 3 businesses, 3 factories` for the sample, `1 market` for `resources_one`, `1 hex`, `1 pop type` and `1 pop` for `pops_one`, `1 technology` for `technologies_one`, `1 business` for `businesses_one`, `1 technology, 1 business, 1 factory` for `factories_one`, zero elsewhere), and every expected turn line ends with its `endowed`, `sold`, `bought` and `consumed` lists, its `population` and the four business lists (`; endowed nothing; sold nothing; bought nothing; consumed nothing; population 0; businesses sold nothing; businesses bought nothing; produced nothing; used nothing` for settings without POPs or businesses). A dedicated test checks `pops_placed`'s success line and its turn 0 and turn 1 lines (`consumed 35 labor; population 12.5`, with nothing sold or bought, since it has no markets, the same population every turn, since its types don't grow, and every business list `nothing`). The sample's turn 0, 1 and 2 lines, under the `arithmetic` decider, are compared exactly as printed, slivers included: turn 0 shows the businesses buying and producing with nothing sold yet, turns 1 and 2 show them selling last turn's output, and from turn 1 `perished` starts with a `0.0000000000000002220446049250313 unskilled_labor` sliver followed by only the unowned deposits' output (see [world.md](world.md#setting-checker---turns-n)). It has its own `repo_path`, because integration tests can't see the binary's `#[cfg(test)]` modules. The sample's `--turns` tests run the real `arithmetic` decider, so they need the project's Python; their exit-code assertions quote the child's stderr, so a missing environment fails with its message. `a_decider_that_wont_start_prints_an_error_and_exits_1` runs `production_valid --turns 1` (exit 1, only the success line on stdout, `unknown decider 'rule'` on stderr), and `turns_without_businesses_start_no_python` runs `pops_placed --turns 1` with `POLICYSPACE_PYTHON` pointing nowhere (set on the child's `Command`, never in the test process) and still passes.
- `crates/dev-ui/tests/cli.rs` covers `--version` and the usage errors.
- No test may run the binary with no arguments or with a single folder argument: those open a window and hang the run. The window forms are tested only through `parse_args`.
- The window itself is checked by hand: the layout, the hover texts, the error-colored rows and lines (a POP below subsistence, a refused business in the Businesses table and its section, a cash-short proportion, a shut-down factory), a collapsed POP or business section staying collapsed across a step, the collapsed Trace, the hex panel's sideways scroll, and the three map marks.
