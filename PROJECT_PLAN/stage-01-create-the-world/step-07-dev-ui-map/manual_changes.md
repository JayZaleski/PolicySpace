# Step 07: dev-ui-map — Manual Changes

No change for players. Setting authors and developers get a window that shows a setting's map, and the command line changes.

## What the manual should cover
- The developer UI window: `dev-ui <folder>` opens it with a setting loaded, and `dev-ui` alone opens it with an **Open setting…** button (system folder picker; cancelling changes nothing).
- Reading the map: each hex is a pointy-top hexagon, rows run east-west, `q` increases eastward and `r` increases southward. The sample setting shows as a hexagon of 19 hexes (radius 2).
- Controls: drag to pan, scroll to zoom toward the cursor, **Fit** to show the whole map again. When zoomed in far enough each hex shows its coordinates as `q, r` (the center of the sample is `0, 0`). Hovering highlights a hex and shows its coordinates in the status bar. Clicking does nothing yet.
- The status bar: setting name, hex count, current turn (always 0 for now), and the hovered hex.
- The author's loop: edit a setting file, click **Reload**, see the result. Reload keeps the current pan and zoom. If the setting has errors, the window shows the full error list (the same lines `check` prints) and no map. Fix the files and Reload again. A broken folder given on the command line opens the window showing its errors.
- Unusual maps: an empty map shows "This setting has no hexes."; holes, islands, and negative coordinates draw correctly; for maps spanning huge distances, hexes too small to see are drawn as dots, and zooming in turns them back into hexagons.
- Changed commands (update every earlier mention): the setting checker is now `dev-ui check <folder> [--turns N]` with the same output and exit codes (0 success, 1 setting errors, 2 wrong usage); it never opens a window and is the form to use in scripts. `dev-ui --version` prints the engine's name and version. Wrong arguments print a three-line usage message and exit with 2. This supersedes the earlier `manual_changes.md` text that gives the checker's invocation as `dev-ui <folder>` or `cargo run -p policyspace-dev-ui -- <folder>`, the usage message `dev-ui [SETTING_FOLDER [--turns N]]` (Step 06), the "no argument prints the version" note (Steps 01 and 02), and the "placeholder" description of the dev UI.
- Not yet: showing deposits, goods, or stockpiles, selecting hexes, and stepping turns (Step 08).

## Where it fits
A new subsection "The developer UI" in the "Writing a setting" material, after "Running a setting" (Step 06), since it is how an author sees their map. The checker examples in earlier subsections just change to the `check` form. Any project-structure note on the dev UI should say it is a real tool now.

## Links
- `docs/dev-ui.md` for the commands, modules, and map math.
- `docs/map.md` for the hex orientation, `docs/setting-loader.md` for the checker and error reporting, and `docs/world.md` for the world the window builds.

## Example
A screenshot of the window showing the sample setting would help (map, top controls, status bar), as would a screenshot of the error view. A short annotated diagram of the hex axes (`q` east, `r` south, a few labelled hexes) would clarify the orientation. An interactive demo is not needed.
