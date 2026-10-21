# Workspace and tooling

The project is a Rust workspace with two crates, plus a Python package for the business deciders that the engine runs as separate processes (see [python.md](python.md)).

## Layout

```
Cargo.toml              workspace root (virtual manifest)
Cargo.lock              committed
rust-toolchain.toml     pins the Rust version
crates/
  engine/               policyspace-engine (library)
    src/lib.rs
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/bundle.rs       Bundle: goods with amounts (see technologies.md)
    src/businesses.rs   businesses and their per-hex index (see businesses.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/decider/        the decider protocol (view, decision, batch, JSON; see decider.md) and
                        the host that runs it each turn (see decider-host.md)
    src/demand.rs       Stone-Geary demand and the PriceList trait (see demand.md)
    src/error_list.rs   ErrorList, the never-empty error list behind LoadErrors, DecisionErrors
                        and ReplyErrors (see setting-loader.md)
    src/factories.rs    factories: hex, technology, owner, condition (see factories.md)
    src/goods.rs        goods data model (see goods.md)
    src/growth.rs       POP growth: subsistence index, growth rate, grown size (see growth.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types, growth parameters and placed POPs (see pops.md)
    src/production.rs   factory mechanics: run, wear, maintain, operate (see production.md)
    src/quantity.rs     the Quantity type and its JSON form
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number and its JSON form
    src/stockpile.rs    Stockpile and PerishesAt
    src/technologies.rs technologies, the recipes factories run (see technologies.md)
    src/trade.rs        the trade round, PopOutcome and BusinessTrade (see trade.md)
    src/world.rs        World, TurnReport, BusinessOutcome, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    src/test_support.rs helpers shared by the engine's unit tests (#[cfg(test)])
    tests/common/       helpers shared by the integration tests
    tests/business_trade.rs, businesses.rs, currency.rs, decider.rs, decider_host.rs,
          demand.rs, external_decider.rs, factories.rs, goods.rs, growth.rs, map.rs,
          market.rs, pops.rs, production.rs, recordings.rs, resources.rs,
          setting_loader.rs, technologies.rs, trade.rs, world.rs
    tests/recordings/   committed decider messages shared with the Python tests (see decider.md)
    tests/settings/     small setting folders used by engine tests
  dev-ui/               policyspace-dev-ui (binary `dev-ui`)
    src/                main.rs, cli.rs, check.rs, text/ (mod.rs, businesses.rs),
                        geometry.rs, session.rs, app.rs, test_support.rs (see dev-ui.md)
    tests/              checker.rs, cli.rs
python/                 the policyspace Python package (see python.md)
  pyproject.toml        project file: setuptools, no runtime dependencies, pytest settings
  policyspace/          protocol.py, arithmetic.py, deciders.py, __main__.py
  tests/                the Python tests: conftest.py, test_protocol.py, test_arithmetic.py,
                        test_loop.py
settings/sample/        the project's sample setting
tools/check.py          the single check command
requirements.txt        the virtual environment: jupyter, pytest, the package (editable)
.gitattributes          keeps tests/recordings/ at LF line endings
```

## Crates and the dependency rule

- **`policyspace-engine`** is the simulation library. It is setting-agnostic (settings are data, not code) and UI-agnostic (it never knows a UI exists). It is meant to be deterministic: the same setting and the same inputs give the same result. Anything iterable must iterate in a defined order (for example registration order), never hash order.
- **`policyspace-dev-ui`** is the developer UI: an egui/eframe window that loads a setting folder, builds a world from it under the setting's deciders, draws its hex map, inspects hexes (their deposits, stockpile, market, POPs and businesses), goods, resource types, POP types, technologies and businesses, and steps turns, plus a headless `check` mode that prints a setting's summary or errors. It depends on the engine by path and uses only the engine's public interface; the engine has no UI dependency. Its extra dependencies (`eframe`, `rfd`) belong to this crate only. See [dev-ui.md](dev-ui.md).
- Dependencies run one way. The engine must never depend on the dev UI or on any UI library; Cargo rejects the reverse dependency as a cycle, so the structure enforces it.
- The engine's external dependencies are `toml` (parser only, no serde; see [setting-loader.md](setting-loader.md)), and `serde` (with `derive`) and `serde_json` for the decider protocol's JSON (see [decider.md](decider.md#json)), plus the dev-dependency `tempfile`. `serde_json` must keep its `float_roundtrip` feature: without it floats don't read back exactly, which breaks the protocol's lossless rule. `serde_json` is used only in `crates/engine/src/decider/mod.rs`. Add a dependency only when a Step needs it.
- **The Python package** `policyspace` in `python/` depends on nothing at runtime beyond the standard library, and on `pytest` for its tests. It talks to the engine only through the decider protocol's JSON (see [decider.md](decider.md)), never by importing or linking it, so the two sides meet only at the message format and the committed recordings both test. Add a Python dependency only when a Step needs it.
- The engine exposes `ENGINE_NAME` and `ENGINE_VERSION` (the latter from `CARGO_PKG_VERSION`) at the crate root.

## Shared configuration

The workspace `Cargo.toml` sets, for all crates (each crate inherits it with `.workspace = true` and `[lints] workspace = true`):

- version `0.1.0`, edition 2024, `rust-version = "1.98"`, `publish = false`, resolver 3;
- lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`, and clippy `all` as warnings.

Because the check command runs clippy with `-D warnings`, every public item needs a doc comment, including the named fields of public enum variants. Code that only tests use is marked `#[cfg(test)]` (not `#[allow(dead_code)]`), so clippy still catches truly dead code.

## Toolchain

`rust-toolchain.toml` pins Rust to exactly `1.98.1` with the `rustfmt` and `clippy` components. Upgrading the toolchain is a deliberate edit to this file.

## The Python environment

The project's Python lives in a virtual environment, `.venv`, created and filled from the project root:

```
python -m venv .venv
.venv/Scripts/python -m pip install -r requirements.txt
```

`requirements.txt` installs `jupyter` (for the manual), `pytest`, and the `policyspace` package in editable form (`-e ./python`; the path is relative to the project root, where the command runs). The first install fetches the package's build backend, so it needs network access once. Python 3.12 or later is required. `.gitignore` excludes `.venv/`, `__pycache__/`, `*.egg-info/` and `.pytest_cache/`.

The Rust side uses the environment too. The engine's `decider::python` module finds its interpreter relative to the engine crate (`.venv/Scripts/python.exe` on Windows, `.venv/bin/python` elsewhere) and starts `python -m policyspace <decider>` from it, so the dev UI needs the environment to load a setting with businesses, and `cargo test --workspace` needs it for the tests that run the real decider. Two environment variables change this for one run: `POLICYSPACE_PYTHON` points at another interpreter, and `POLICYSPACE_DECIDER_TIME_LIMIT` sets the time limit for each reply in seconds (`0` for none, to step through a decider in a debugger). See [decider-host.md](decider-host.md#finding-python-deciderpython).

## The check command

`python tools/check.py` (or `.venv/Scripts/python tools/check.py`) is the one command that checks the whole project. It is a standard-library-only script that works from any directory. It runs, in order, from the project root:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `python -m pytest python/tests`, with the interpreter that runs the script

It runs all four even if an earlier one fails, prints a pass/fail summary, and exits 0 only if all passed. If `cargo` isn't found it prints a message and exits 1. Fix formatting with `cargo fmt --all`. There is no CI yet; this command is the gate.

The fourth check uses the interpreter running the script (`sys.executable`), so run the script with `.venv/Scripts/python` to test against the virtual environment. Under an interpreter without pytest the check fails with Python's `No module named pytest`; `.venv/Scripts/python -m pip install -r requirements.txt` fixes it. The third check needs the environment as well: some engine and dev-UI tests start the project's Python as a decider, and fail (they are never skipped) with `the project's Python environment wasn't found at ...` when it is missing. With the environment missing, `cargo test -p policyspace-dev-ui` stops at its unit-test target; add `--no-fail-fast` to reach the checker tests too.

## Tests

- Unit tests live next to the code in `#[cfg(test)]` modules. The ID mechanism also has a `compile_fail` doc test (see [ids.md](ids.md)).
- The engine's unit tests share `crates/engine/src/test_support.rs`, declared in `lib.rs` as `#[cfg(test)] pub(crate) mod test_support;`. Everything in it is `pub(crate)`: `q(value)` (a `Quantity`), `close(a, b)` (within `1e-9`), `good(index)` and `hex(index)` (IDs from an index), `end(t)` (`PerishesAt::EndOfTurn`), `after(n)` (`Perishes::After`), `no_growth()` (growth parameters that never change a size), the good indexes `LABOR`, `GRAIN`, `COPPER` and `SILVER` that the demand and growth tests use, `Prices` (a hand-written `PriceList` over a `BTreeMap`, with `prices(list)` and `typical()` to build one), the POP type builders `peasant()` and `type_of(utility)`, and `held(list)` (holdings from index and amount pairs). Test modules import what they need from it (`use crate::test_support::{..}`); helpers that need a loaded world (such as `trade.rs`'s `hex(world, x)`) stay local. The decider module keeps its own shared unit-test helpers, `decider::view::test_view()` and `decider::decision::test_goods()` (both `#[cfg(test)] pub(crate)`), because they build protocol values rather than engine state (see [decider.md](decider.md#tests)).
- `crates/engine/tests/*.rs` are integration tests that use only the engine's public interface. They share `crates/engine/tests/common/mod.rs`, declared in each file with `mod common;` (it carries `#![allow(dead_code)]` because each file uses only some helpers): `test_setting(name)` and `sample_setting()` (paths), `load(path)` and `failures_of(name)` (a loaded setting, or a failing setting's error lines), `lines(errors)`, `idle()` (the idle decider host, for every `World::new` and `step` that doesn't test deciders), `world_of(path)` (a world built under the idle host, with its turn 0 report), `hex(world, q, r)`, `good(world, name)`, `q(value)`, `close(a, b)`, `end(t)`, `pop_id(index, world)` and `outcome(world, index)` (a POP and its current outcome by index), `named(world, list)` and `pair(name, amount)` (lists of goods as names and `f64`s), `assert_list(world, actual, expected)` (same names in the same order, amounts within `close`), `stockpile_totals(world, stockpile)` and `groups(stockpile, good)`. File-specific helpers stay in their files and call the shared ones.
- `crates/engine/tests/settings/` holds small setting folders for engine tests, described in [setting-loader.md](setting-loader.md). Each test setting should be minimal and named for what it tests.
- `crates/engine/tests/recordings/` holds committed decider protocol messages (a sample recording, an engine-written exactness request and hand-written requests) that `crates/engine/tests/recordings.rs` and the Python tests both read. They are compared byte for byte, so `.gitattributes` keeps them at LF line endings. The sample recording and the exactness request are regenerated with `POLICYSPACE_UPDATE_RECORDINGS` set (see [decider.md](decider.md#recordings)).
- The Python tests are in `python/tests/`, run by pytest from the virtual environment (`.venv/Scripts/python -m pytest python/tests`, or the check command). `conftest.py` locates the recordings folder and reads its files (see [python.md](python.md#tests)).
- `crates/dev-ui/tests/checker.rs` runs the built binary (`env!("CARGO_BIN_EXE_dev-ui")`) with `check` against the sample setting and broken ones. `crates/dev-ui/tests/cli.rs` checks `--version` and the usage errors. **No test may start the binary in a window form** (no arguments, or one folder): that opens a window and hangs the run.
- The dev UI's unit tests share `crates/dev-ui/src/test_support.rs` (see [dev-ui.md](dev-ui.md)).
- Tests never set or remove environment variables in the test process, because tests run in parallel in one process. Code that reads one has a `_from(value)` form that tests call instead (`python::interpreter_from`, `python::time_limit_from`), and tests of a child process set variables on its `Command`.
