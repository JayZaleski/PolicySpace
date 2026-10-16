# Workspace and tooling

## Layout

```
Cargo.toml              workspace root (virtual manifest)
Cargo.lock              committed
rust-toolchain.toml     pins the Rust version
crates/
  engine/               policyspace-engine (library)
    src/lib.rs
    src/id.rs           IDs, name registries, IdVec, NamedList (see ids.md)
    src/currency.rs     each hex's currency good (see currency.md)
    src/demand.rs       Stone-Geary demand and the PriceList trait (see demand.md)
    src/goods.rs        goods data model (see goods.md)
    src/growth.rs       POP growth: subsistence index, growth rate, grown size (see growth.md)
    src/map.rs          hex map data model (see map.md)
    src/market.rs       the dummy market: posted prices and the exchange (see market.md)
    src/pops.rs         POP types, growth parameters and placed POPs (see pops.md)
    src/quantity.rs     the Quantity type
    src/resources.rs    resource types and deposits (see resources.md)
    src/turn.rs         the Turn number
    src/stockpile.rs    Stockpile and PerishesAt
    src/trade.rs        the trade round and PopOutcome (see trade.md)
    src/world.rs        World, TurnReport, StepError (see world.md)
    src/setting/        the setting loader (see setting-loader.md)
    src/test_support.rs helpers shared by the engine's unit tests (#[cfg(test)])
    tests/common/       helpers shared by the integration tests
    tests/currency.rs, demand.rs, goods.rs, growth.rs, map.rs, market.rs, pops.rs,
          resources.rs, setting_loader.rs, trade.rs, world.rs
    tests/settings/     small setting folders used by engine tests
  dev-ui/               policyspace-dev-ui (binary `dev-ui`)
    src/                main.rs, cli.rs, check.rs, text.rs, geometry.rs, session.rs,
                        app.rs, test_support.rs (see dev-ui.md)
    tests/              checker.rs, cli.rs
settings/sample/        the project's sample setting
tools/check.py          the single check command
```

## Crates and the dependency rule

- **`policyspace-engine`** is the simulation library. It is setting-agnostic (settings are data, not code) and UI-agnostic (it never knows a UI exists). It is meant to be deterministic: the same setting and the same inputs give the same result. Anything iterable must iterate in a defined order (for example registration order), never hash order.
- **`policyspace-dev-ui`** is the developer UI: an egui/eframe window that loads a setting folder, builds a world from it, draws its hex map, inspects hexes (their deposits, stockpile, market and POPs), goods, resource types and POP types, and steps turns, plus a headless `check` mode that prints a setting's summary or errors. It depends on the engine by path and uses only the engine's public interface; the engine has no UI dependency. Its extra dependencies (`eframe`, `rfd`) belong to this crate only. See [dev-ui.md](dev-ui.md).
- Dependencies run one way. The engine must never depend on the dev UI or on any UI library; Cargo rejects the reverse dependency as a cycle, so the structure enforces it.
- The engine's only external dependency is `toml` (parser only, no serde; see [setting-loader.md](setting-loader.md)), plus the dev-dependency `tempfile`. Add a dependency only when a Step needs it.
- The engine exposes `ENGINE_NAME` and `ENGINE_VERSION` (the latter from `CARGO_PKG_VERSION`) at the crate root.

## Shared configuration

The workspace `Cargo.toml` sets, for all crates (each crate inherits it with `.workspace = true` and `[lints] workspace = true`):

- version `0.1.0`, edition 2024, `rust-version = "1.98"`, `publish = false`, resolver 3;
- lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`, and clippy `all` as warnings.

Because the check command runs clippy with `-D warnings`, every public item needs a doc comment, including the named fields of public enum variants. Code that only tests use is marked `#[cfg(test)]` (not `#[allow(dead_code)]`), so clippy still catches truly dead code.

## Toolchain

`rust-toolchain.toml` pins Rust to exactly `1.98.1` with the `rustfmt` and `clippy` components. Upgrading the toolchain is a deliberate edit to this file.

## The check command

`python tools/check.py` (or `.venv/Scripts/python tools/check.py`) is the one command that checks the whole project. It is a standard-library-only script that works from any directory. It runs, in order:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

It runs all three even if an earlier one fails, prints a pass/fail summary, and exits 0 only if all passed. If `cargo` isn't found it prints a message and exits 1. Fix formatting with `cargo fmt --all`. There is no CI yet; this command is the gate.

## Tests

- Unit tests live next to the code in `#[cfg(test)]` modules. The ID mechanism also has a `compile_fail` doc test (see [ids.md](ids.md)).
- The engine's unit tests share `crates/engine/src/test_support.rs`, declared in `lib.rs` as `#[cfg(test)] pub(crate) mod test_support;`. Everything in it is `pub(crate)`: `q(value)` (a `Quantity`), `close(a, b)` (within `1e-9`), `good(index)` and `hex(index)` (IDs from an index), `end(t)` (`PerishesAt::EndOfTurn`), `after(n)` (`Perishes::After`), `no_growth()` (growth parameters that never change a size), the good indexes `LABOR`, `GRAIN`, `COPPER` and `SILVER` that the demand and growth tests use, `Prices` (a hand-written `PriceList` over a `BTreeMap`, with `prices(list)` and `typical()` to build one), the POP type builders `peasant()` and `type_of(utility)`, and `held(list)` (holdings from index and amount pairs). Test modules import what they need from it (`use crate::test_support::{..}`); helpers that need a loaded world (such as `trade.rs`'s `hex(world, x)`) stay local.
- `crates/engine/tests/*.rs` are integration tests that use only the engine's public interface. They share `crates/engine/tests/common/mod.rs`, declared in each file with `mod common;` (it carries `#![allow(dead_code)]` because each file uses only some helpers): `test_setting(name)` and `sample_setting()` (paths), `load(path)` and `failures_of(name)` (a loaded setting, or a failing setting's error lines), `lines(errors)`, `world_of(path)` (a world with its turn 0 report), `hex(world, q, r)`, `good(world, name)`, `q(value)`, `close(a, b)`, `end(t)`, `pop_id(index, world)` and `outcome(world, index)` (a POP and its current outcome by index), `named(world, list)` and `pair(name, amount)` (lists of goods as names and `f64`s), `assert_list(world, actual, expected)` (same names in the same order, amounts within `close`), `stockpile_totals(world, stockpile)` and `groups(stockpile, good)`. File-specific helpers stay in their files and call the shared ones.
- `crates/engine/tests/settings/` holds small setting folders for engine tests, described in [setting-loader.md](setting-loader.md). Each test setting should be minimal and named for what it tests.
- `crates/dev-ui/tests/checker.rs` runs the built binary (`env!("CARGO_BIN_EXE_dev-ui")`) with `check` against the sample setting and broken ones. `crates/dev-ui/tests/cli.rs` checks `--version` and the usage errors. **No test may start the binary in a window form** (no arguments, or one folder): that opens a window and hangs the run.
- The dev UI's unit tests share `crates/dev-ui/src/test_support.rs` (see [dev-ui.md](dev-ui.md)).
