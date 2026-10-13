# Step 01: engine-workspace — Docs Changes

## docs/README.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# PolicySpace developer documentation

These documents explain how the code works, for developers who want to modify the project. The user manual (`manual.ipynb`) links here for technical detail.

| Document | Covers |
|---|---|
| [workspace.md](workspace.md) | The Cargo workspace, the crates and their dependency rule, the pinned toolchain, the check command, and test layout |
| [ids.md](ids.md) | The ID mechanism: `Id<K>`, `IdKind`, `NameRegistry<K>`, and `IdError` |

## docs/workspace.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# Workspace and tooling

## Layout

```
Cargo.toml              workspace root (virtual manifest)
Cargo.lock              committed
rust-toolchain.toml     pins the Rust version
crates/
  engine/               policyspace-engine (library)
    src/lib.rs
    src/id.rs
    tests/settings/     small setting folders used by engine tests
  dev-ui/               policyspace-dev-ui (binary `dev-ui`)
    src/main.rs
    tests/placeholder.rs
tools/check.py          the single check command
```

## Crates and the dependency rule

- **`policyspace-engine`** is the simulation library. It is setting-agnostic (settings are data, not code) and UI-agnostic (it never knows a UI exists). It is meant to be deterministic: the same setting and the same inputs give the same result. Anything iterable must iterate in a defined order (for example registration order), never hash order.
- **`policyspace-dev-ui`** is the developer program. It depends on the engine by path and uses only the engine's public interface. Today it is a placeholder: `main` prints `"{ENGINE_NAME} {ENGINE_VERSION}"` (currently `PolicySpace Engine 0.1.0`) and exits 0. A later Step replaces it with a real developer UI.
- Dependencies run one way. The engine must never depend on the dev UI or on any UI library; Cargo rejects the reverse dependency as a cycle, so the structure enforces it.
- The engine has no external dependencies. Add one only when a Step needs it.
- The engine exposes `ENGINE_NAME` and `ENGINE_VERSION` (the latter from `CARGO_PKG_VERSION`) at the crate root.

## Shared configuration

The workspace `Cargo.toml` sets, for all crates (each crate inherits it with `.workspace = true` and `[lints] workspace = true`):

- version `0.1.0`, edition 2024, `rust-version = "1.98"`, `publish = false`, resolver 3;
- lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`, and clippy `all` as warnings.

Because the check command runs clippy with `-D warnings`, every public item needs a doc comment, including the named fields of public enum variants.

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
- `crates/dev-ui/tests/placeholder.rs` is an integration test that runs the built binary (`env!("CARGO_BIN_EXE_dev-ui")`) and checks it exits successfully and prints the engine's name and version.
- `crates/engine/tests/settings/` holds small setting folders for engine tests. It currently contains only a README; the setting loader Step adds the first settings. Each test setting should be minimal and named for what it tests.

## docs/ids.md — new file
- **Location:** new file
- **Change:** add
- **Supersedes:** none

# IDs and name registries

Source: `crates/engine/src/id.rs`, re-exported from the engine crate root (`Id`, `IdError`, `IdKind`, `NameRegistry`).

## Rules

- Setting files refer to things by readable text names (for example `copper_ore`).
- When a setting is loaded, a registry turns each text name into a compact numeric ID. The engine uses IDs internally from then on.
- Each kind of thing has its own ID type, so a good's ID cannot be used where a hex's ID is expected. This is a compile-time error.
- IDs are meaningful only inside the registry (world) that created them. They must never be persisted: save files and anything long-term store text names.
- Text names appear only at the edges: reading settings, error messages, and UIs.

## Types

- **`IdKind`** is a marker trait with one constant, `LABEL`, a lower-case human label (`"good"`, `"resource type"`) used in messages. Implement it on a zero-sized, uninhabited marker type, for example `pub enum GoodKind {}`.
- **`Id<K: IdKind>`** wraps a `u32` index and a `PhantomData<fn() -> K>` (which keeps `Id<K>` `Copy`, `Send` and `Sync` for any `K`). `Id::index()` returns the 0-based position as `usize`, so an ID can index a per-kind `Vec` directly. `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` and `Debug` are implemented by hand so they place no bounds on `K`; comparison and hashing use the index only. `Debug` prints `Id<good>(3)`. `Id::from_index` is crate-private: only registries create IDs.
- **`NameRegistry<K>`** maps names to IDs and back for one kind. IDs are handed out as `0, 1, 2...` in registration order, and `iter()` yields `(Id<K>, &str)` in that order, so iteration is deterministic. Internally it holds a `Vec<String>` (index = ID) and a `HashMap<String, Id<K>>` that is used only for lookup, never for iteration.
  - `register(name)` returns the new ID, or `IdError::DuplicateName` (leaving the registry unchanged). Any string is accepted, including an empty one; validating name syntax is the setting loader's job.
  - `id(name)` returns the ID, or `IdError::UnknownName`.
  - `name(id)` returns the text name. It panics if the ID did not come from this registry, which is a programming error.
  - `len()`, `is_empty()`, and a hand-written `Default`.
- **`IdError`** has `DuplicateName { kind, name }` and `UnknownName { kind, name }`, where `kind` is `K::LABEL`. `Display` gives `duplicate good name 'copper'` and `unknown good 'copper'`; `std::error::Error` is implemented by hand (no `thiserror`).

The same name may be registered in registries of two different kinds without conflict.

## Adding a new ID kind

1. Declare a marker type and implement `IdKind` with a `LABEL`.
2. Create a `NameRegistry<YourKind>` where the world is loaded and register names in a deterministic order (the order they appear in the setting files).
3. Use `Id<YourKind>` everywhere internally, and `registry.name(id)` only when producing text for errors or UIs.

No concrete kinds are defined yet; each Step defines the kinds it needs.

## Tests

Unit tests in `id.rs` cover registration order, round trips, duplicate and unknown-name errors and their messages, same name across kinds, the out-of-range panic, and the `Debug` format. A `compile_fail` doc test on `Id` shows that an `Id<HexKind>` cannot be passed where an `Id<GoodKind>` is expected.
