# Step 07: dev-ui-map

## Summary
This Step replaces the placeholder `dev-ui` program with a real developer UI: a desktop window, built with egui/eframe, that loads a setting folder, builds a running world from it, and draws its hex grid. The developer can pan and zoom the map, see each hex's coordinates, and reload the setting after editing its files without restarting. The command-line setting checker from Steps 02–06 stays as a headless `check` mode, so scripts and tests that can't open a window keep working. The UI talks to the engine only through the engine's public interface, and the engine still knows nothing about any UI. Step 08 builds inspection and turn stepping on top of this window.

## Behavior

### Commands
- `dev-ui <SETTING_FOLDER>` opens the window with that setting loaded.
- `dev-ui` with no arguments opens the window with no setting loaded, showing an **Open setting…** button.
- `dev-ui check <SETTING_FOLDER> [--turns N]` is the setting checker as it works today: the same success line, error lines, `--turns` output, and exit codes (0 success, 1 setting errors, 2 wrong usage). It never opens a window. Only the word `check` is new.
- `dev-ui --version` prints the engine's name and version (what `dev-ui` with no arguments printed before) and exits successfully.
- Any other arguments (for example `check` without a folder, a bad `--turns`, two folders, an unknown flag) print a usage message that shows every form above, and exit with code 2. No window opens.

### Loading a setting
- A setting can be loaded from the command line, or from the window with **Open setting…**, which shows the system folder picker. Cancelling the picker changes nothing.
- On a successful load, the window builds a world from the setting (the same world Step 06 provides, at turn 0) and shows its map.
- **Reload** re-reads the currently loaded folder from disk and builds a fresh world at turn 0. It's for setting authors: edit a file, click Reload, see the result. Reload is unavailable when no folder has been chosen. It keeps the current pan and zoom, so the developer stays looking at the same place; opening a different folder fits the new map to the window.
- If a load fails, the window shows the full list of errors (the same lines `check` prints, with file, line, and column) and no map. Nothing from an earlier load stays on screen, so what's shown always matches the files on disk. After fixing the files, Reload tries again.
- A setting given on the command line that has errors opens the window showing those errors, so it can be fixed and reloaded.
- The window title shows the engine's name and version, and the setting's name once one is loaded.

### The map
- Each hex is drawn as an outlined pointy-top hexagon, in the orientation fixed in Step 04: each value of `r` is an east–west row, `q` increases eastward, and `r` increases southward.
- When a map is first shown, the whole map is fitted to the window.
- Dragging pans the map. The scroll wheel zooms in and out, toward the point under the cursor. A **Fit** button fits the whole map to the window again.
- When hexes are large enough on screen for text to be readable, each shows its coordinates inside it, written `q, r` (for example `1, -2`). When zoomed out further, the labels are hidden.
- Hovering over a hex highlights it and shows its coordinates in the status bar. Hovering over empty space highlights nothing. Clicking does nothing yet; selecting and inspecting hexes is Step 08.
- A status bar shows the setting's name, the number of hexes, the current turn (always 0 in this Step), and the hovered hex's coordinates.

### Unusual maps
- A setting with no hexes loads and shows the message "This setting has no hexes." in place of the map.
- Maps of any shape draw correctly: holes, separate islands, and negative coordinates.
- Coordinates anywhere in the 32-bit range (about ±2.1 billion) draw without errors. When a map spans huge distances, fitting it makes each hex far smaller than a pixel. Hexes too small to draw as hexagons are drawn as dots, so no hex ever disappears from view. Zooming in shows them as hexagons again.
- Hexes outside the visible area aren't drawn, so large maps stay smooth to pan and zoom. As a guide, a map of 10,000 hexes should pan and zoom without noticeable lag.

### Testing
- The map's geometry is tested automatically, without opening a window: where each hex lands on screen for a given pan and zoom, which hex (if any) is under a given screen point, and fitting a map (including an empty map, a one-hex map, and one with coordinates at the 32-bit extremes) to a window.
- The `check` mode keeps its existing tests, changed only to use the new `check` form. `--version` and the usage errors are tested.
- The window itself is checked by hand at Code Review, against the acceptance criteria below.

## Player-facing Changes
None for players. The developer gets a window that shows a setting's map, can open and reload settings in it, and sees load errors there. The command-line checker is now `dev-ui check <folder> [--turns N]`, and the version print is now `dev-ui --version`.

## Interactions
- **Step 01 (engine-workspace):** the dev-ui crate stops being a placeholder. Dependencies still run one way: the dev UI depends on the engine, and the engine gains no UI dependency.
- **Step 02 (setting-loader):** the UI loads settings with the engine's loader and shows its errors as they are. The checker moves under `check`.
- **Step 04 (hex-map):** the map is drawn from the hexes' axial coordinates in the fixed pointy-top orientation.
- **Step 06 (turn-clock):** the window builds a world from the loaded setting and reads its map and turn. `--turns` moves under `check` unchanged.
- **Step 08 (dev-ui-inspect-and-step):** adds clicking a hex to inspect it, panels for hexes, deposits, goods, and stockpiles, and stepping turns, all on the world and hover logic this Step builds.

## Out of Scope
- Showing deposits, goods, stockpiles, or turn reports, selecting hexes, and stepping turns (Step 08).
- Terrain, colors by data, or any hex attribute beyond coordinates.
- Neighbors, distance, and wrapping.
- Remembering the last opened folder or the view between runs.
- Automated tests that drive the window itself.
- A player UI, and saving or loading a world.

## Acceptance Criteria
- `dev-ui settings/sample` opens a window showing the sample's 19 hexes as a pointy-top hexagon of radius 2, fitted to the window, with rows running east–west and `r` increasing southward.
- Dragging pans the map, scrolling zooms toward the cursor, and Fit restores the whole-map view.
- Zoomed in, each hex shows its coordinates (`0, 0` at the center); zoomed out, labels are hidden.
- Hovering a hex highlights it and shows its coordinates in the status bar; hovering empty space shows none. The status bar shows the setting's name, `19 hexes`, and turn 0.
- `dev-ui` with no arguments opens a window with Open setting…. Choosing the sample folder loads it; cancelling the picker changes nothing.
- After a setting file is edited to add an error, Reload shows the error list (matching `check`'s lines) and no map. After the error is fixed, Reload shows the map again, at the same pan and zoom.
- `dev-ui` with a broken setting folder opens the window showing its errors.
- A setting with no hexes shows "This setting has no hexes."
- The `hexes_valid` test setting, which has coordinates at the 32-bit extremes, opens without errors or crashes, and every hex is visible as a hexagon or a dot.
- `dev-ui check <folder>` and `dev-ui check <folder> --turns N` give exactly the output and exit codes the checker gave before this Step.
- `dev-ui --version` prints the engine's name and version and exits successfully. Wrong arguments print the usage message showing every form, and exit with code 2 without opening a window.
- Automated tests cover hex-to-screen positions, finding the hex under a point, and fitting (empty, one-hex, and extreme-coordinate maps).
- The engine crate has no new dependencies.
- The single check command passes.
