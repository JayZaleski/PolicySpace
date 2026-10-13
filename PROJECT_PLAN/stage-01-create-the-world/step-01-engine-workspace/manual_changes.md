# Step 01: engine-workspace — Manual Changes

No player-facing changes. This Step is developer infrastructure only.

If the manual has (or gains) a short "for developers" or "project structure" section, it can briefly mention:
- The project is a Rust workspace with an engine library and a developer UI program, and the engine never depends on the UI.
- Running `cargo run -p policyspace-dev-ui` prints the engine's name and version (a placeholder until a later Step adds a real UI).
- `python tools/check.py` runs formatting, clippy, and all tests.
- Link to `docs/workspace.md` and `docs/ids.md` for technical detail.

No example, diagram, or demo is needed.
