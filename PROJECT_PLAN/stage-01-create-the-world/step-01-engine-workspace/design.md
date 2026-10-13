# Step 01: engine-workspace

## Summary
This Step lays the foundation every later Step builds on. It sets up a Rust workspace with two crates: an engine library that will hold the whole simulation, and a developer UI program that uses it. It adds the general mechanism the engine uses to identify things (goods, hexes, deposits, and later POPs and industries), a test setup with one command that checks the whole project, and a pinned Rust version. Nothing in the game world exists yet. The point is that later Steps can add systems without revisiting how the project is organized.

## Behavior

### Workspace and crates
- The project is one Rust workspace with two crates:
  - **Engine**: a library that holds the simulation. It is setting-agnostic and UI-agnostic.
  - **Dev UI**: a program for the developer that uses the engine.
- Dependencies run one way only. The dev UI depends on the engine; the engine never depends on the dev UI and doesn't know any UI exists. This keeps the UI swappable, as the vision requires (a different UI per setting, a player UI later).
- In this Step the dev UI is a placeholder. When run, it asks the engine for its name and version, prints them, and exits. That's enough to show the two crates are wired together correctly. Choosing a UI framework and drawing anything is Step 07's job.

### IDs
- Setting files refer to things by readable **text names** (for example `copper_ore`, `arable_land`).
- When a setting is loaded, the engine turns each text name into a compact **number ID**, and uses the numbers internally from then on.
- Each kind of thing has its **own kind of ID**. A good's ID and a hex's ID are different kinds, and the engine can't accidentally use one where the other is expected.
- The engine can always turn an ID back into its **text name**. Text names appear only at the edges: reading setting files, error messages, and the dev UI.
- IDs are **only meaningful inside one loaded world**. If the setting files change, the numbers may change too. Anything saved long-term (save files, when they arrive) must store text names, not raw numbers.
- This Step builds only the **general mechanism** for ID kinds and name lookup, with tests. It defines no concrete ID kinds. Each later Step adds the kinds it needs (goods in Step 03, hexes in Step 04, resource types and deposits in Step 05, and so on).
- Edge cases the mechanism handles:
  - The same text name registered twice for the same kind is an error, not a silent overwrite.
  - Looking up a text name that doesn't exist reports that it wasn't found, naming the missing text name.
  - The same text name may be used by two different kinds (a good and a resource type both called `copper`) without conflict.

### Determinism
- As a standing principle, the engine gives the same result from the same setting and the same inputs. Nothing in this Step can vary, so there is nothing to enforce yet. Later Steps that add randomness or ordering choices must keep to it.

### Tests and checks
- Each crate uses Rust's built-in tests.
- A dedicated folder holds small test settings. It starts empty; Step 02 adds the first ones.
- One command checks the whole project: formatting (rustfmt), common mistakes (clippy), and all tests. It passes on the finished Step.
- No automated testing on a server (CI) for now.

### Toolchain
- The project pins Rust 1.98 in a file in the repository, so every build uses the same version. Upgrading is a deliberate change to that file.

## Player-facing Changes
None for players. A developer can build the project, run the placeholder dev UI to see the engine's name and version, and run the single check command.

## Interactions
- **Step 02 (setting-loader)** uses the ID mechanism to turn the setting's text names into IDs, and adds the first test settings to the test-settings folder.
- **Steps 03–05** each add their own ID kinds (goods, hexes, resource types, deposits).
- **Step 07 (dev-ui-map)** replaces the dev UI placeholder with a real developer UI, still talking to the engine only through its public interface.

## Out of Scope
- Choosing or using a UI framework.
- The setting folder layout, file format, and loading.
- Any concrete ID kinds, world state, or turns.
- Save files.
- CI.

## Acceptance Criteria
- The project builds as one workspace containing an engine library crate and a dev UI program crate.
- The engine crate does not depend on the dev UI crate.
- Running the dev UI prints the engine's name and version and exits successfully.
- Tests show that the ID mechanism:
  - gives each registered text name an ID and turns it back into the same text name;
  - keeps different ID kinds from being mixed up (this is caught when the code is built, not at runtime);
  - rejects a duplicate text name within one kind;
  - reports a missing text name clearly;
  - allows the same text name in two different kinds.
- The test-settings folder exists.
- The Rust version is pinned to 1.98 in the repository.
- The single check command runs the formatting check, clippy, and all tests, and passes.
