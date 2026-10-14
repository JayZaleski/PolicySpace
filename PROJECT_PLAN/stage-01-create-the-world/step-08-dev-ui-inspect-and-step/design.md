# Step 08: dev-ui-inspect-and-step

## Summary
This Step finishes Stage 01's developer UI. In the window from Step 07, the developer can select a hex and inspect its deposits and stockpile, see the world's goods, resource types, and a log of every turn report, and step turns one at a time or ten at a time. With this, the minimal turn from Step 06 can be watched working: deposits emit, goods build up, and goods perish on schedule. The engine already makes everything this needs public, so only the dev UI changes.

## Behavior

### Selecting a hex
- Clicking a hex selects it. When hexes are drawn as dots, a click near a dot selects it, within the same snap distance that hovering uses (Step 07).
- The selected hex is drawn with a thick outline in the theme's accent color, so it can be told apart from the hover highlight. A hex can be hovered and selected at once.
- Clicking empty space, or pressing Esc, clears the selection. Clicking the selected hex again keeps it selected.
- Dragging to pan never changes the selection.
- **Reload** keeps the selection if the reloaded map has a hex with the same coordinates, and clears it otherwise. This matches Reload keeping the pan and zoom: a setting author can edit a hex's deposits, Reload, and see the result at once.
- A Reload that fails (the window shows errors, so nothing is selected) doesn't clear the selection. The next Reload that succeeds selects that hex again if it still exists, just as the pan and zoom survive a failed load in between.
- **Opening a different folder** clears the selection.

### Stepping turns
- The top bar gains **Step** and **Step ×10** buttons, next to Reload and Fit. Their keyboard shortcuts are **Space** and **Shift+Space**.
- They're available only when a world is loaded, including a world with no hexes.
- **Step** steps the world once and adds the report to the log. **Step ×10** steps ten times, adding ten reports.
- If the engine refuses a step (the clock is at its maximum turn), the world is unchanged, stepping stops (Step ×10 doesn't try the remaining steps), and the error is shown at the top of the turn log. In practice this is unreachable.
- **Reload** builds a fresh world at turn 0, as in Step 07, and starts a new log containing only turn 0's report. **Opening a folder** does the same.
- The status bar keeps showing the current turn, as in Step 07.

### Map marks
- A hex with at least one deposit gets a small mark inside it, so the developer can see where to click.
- Marks are drawn only when hexes are drawn as hexagons. When hexes are drawn as dots there's no room, so there are no marks.
- The map shows nothing else new: no coloring by stockpile or other data.

### The two panels
When a world is loaded, the window has two side panels around the map: the **world panel** on the left and the **hex panel** on the right. Each can be resized. They aren't shown when nothing is loaded or when the setting has errors. For a world with no hexes, the world panel is shown and the hex panel says "This setting has no hexes."

Amounts are written the same way as in the `check --turns` output. Goods, resource types, and deposits appear in ID order (the order they're defined in the setting), so the panels are always the same for the same world.

### World panel (left)
Three sections, each of which can be collapsed:

1. **Turn log:** every turn report since the setting was loaded, turn 0 included, newest first. Each entry has the same wording as a `check --turns` line, for example `Turn 3: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain`. A refused step's error appears above the newest entry. The log keeps every report until the next load. It stays smooth to scroll with thousands of entries.
2. **Goods:** one row per good, with:
   - its name;
   - how long it lasts: `perishes after N turns` (`after 1 turn` for one) or `never perishes`;
   - whether it's movable (`movable` or `fixed`);
   - how much of it the whole world holds now, summed over every stockpile.
3. **Resource types:** one row per resource type, with its name, the good it produces, its output per deposit, and the number of deposits of that type in the world.

### Hex panel (right)
- With no hex selected: "Click a hex to inspect it."
- With a hex selected, it shows:
  - its **coordinates**, written `(q, r)` as in the status bar;
  - its **deposits**, grouped by resource type, one line per type, for example `arable_land × 3: 4 unimproved_grain each, 12 total`. A hex with no deposits shows "No deposits."
  - its **stockpile**: each good it holds, with its total, and under it each part of that total with when it perishes, soonest first. An empty stockpile shows "Empty."
- **When a part perishes** is shown only as a countdown, counted from the current turn:
  - `perishes this turn`: it's removed at the end of the current turn;
  - `perishes in N turns` (`in 1 turn` for one): it's removed at the end of the turn N turns from now;
  - `never perishes`: a good that never perishes.
  - Example: in `turns_valid` at turn 2, hex (0, 0) holds 15 `stew`. 5 perishes this turn, 5 in 1 turn, and 5 in 2 turns.
- Both panels update straight after every step and reload.

### Testing
- As in Step 07, the logic is kept apart from the window code so it can be tested without opening a window:
  - stepping and Step ×10, and the log they build;
  - Reload and Open starting a new log at turn 0;
  - the selection after Reload (kept when the coordinates still exist, cleared when they don't) and after Open (cleared);
  - the deposit grouping and its wording;
  - the countdown wording;
  - the goods' world totals and the resource types' deposit counts.
- The window itself is checked by hand at Code Review, against the acceptance criteria below.

## Player-facing Changes
None for players. In the dev UI, the developer can:
- click a hex to inspect its deposits and stockpile;
- see the world's goods, resource types, and the full log of turn reports;
- step turns with Step and Step ×10 (Space and Shift+Space).

Hexes with deposits are marked on the map.

## Interactions
- **Step 07 (dev-ui-map):**
  - Builds on its window: the session and Reload, hover and snap distance, the top bar, and the status bar.
  - The turn-0 report that Step 07 discarded is now the first log entry.
  - The `check` mode is unchanged.
- **Step 06 (turn-clock):**
  - Uses its public interface: stepping and its error, turn reports, and hex stockpiles with their perish breakdown.
  - The log uses the same wording as `check --turns`.
- **Step 05 (natural-resources):** deposits and resource types are shown, grouped by type.
- **Step 03 (goods-and-properties):** each good's properties are shown. `movable` is shown but still has no effect.
- **Step 04 (hex-map):** selection uses the hex coordinates and lookup.
- **Stage 02 (populate-the-world):** will add owners, POPs, and their stockpiles. The panels are where those will be inspected.

## Out of Scope
- Running turns continuously (auto-run), and stepping by an arbitrary number.
- Coloring hexes by data, and any map marks other than deposits.
- Per-hex breakdowns in the turn log, and saving or exporting the log.
- Editing the world or the setting from the window.
- Selecting several hexes, and inspecting individual deposits (they're grouped by type).
- Any change to the engine, the `check` mode, or the sample setting.
- Automated tests that drive the window itself.

## Acceptance Criteria
- With `settings/sample` open, clicking hex (0, 0) selects it with a thick accent outline, and the hex panel shows:
  - `(0, 0)`;
  - `arable_land × 3: 4 unimproved_grain each, 12 total` and a `copper_ore` line;
  - a stockpile of 12 `unimproved_grain` and 2.5 `unimproved_copper`, each marked `perishes this turn`.
- Clicking empty space or pressing Esc clears the selection, and the hex panel says "Click a hex to inspect it." Dragging never changes the selection.
- Hexes with deposits show a mark when drawn as hexagons. With `hexes_valid` fitted (hexes drawn as dots), clicking near a dot selects it.
- At turn 0 the turn log has one entry, the sample's turn 0 line. Step adds `Turn 1: …` at the top, and Space does the same. Step ×10 and Shift+Space each add ten entries, and the status bar's turn matches the newest entry.
- The world panel lists the sample's 5 goods with their properties and world totals (10 `unimproved_copper` and 24 `unimproved_grain` at every turn, 0 for the rest), and its 2 resource types with output and deposit counts (4 `copper_ore`, 6 `arable_land`).
- With `turns_valid` open, stepping to turn 2 and selecting (0, 0) shows 15 `stew`: 5 perishing this turn, 5 in 1 turn, and 5 in 2 turns. From then on the total stays at 15. Hex (1, 0)'s `gold` grows by 1.5 every step and shows `never perishes`. Hex (2, 0) shows "No deposits." and "Empty."
- After stepping, Reload returns to turn 0 with a log of only turn 0's line, and keeps the selected hex. If the selected hex is removed from the setting's map before Reload, the selection is cleared. Opening a different folder clears the selection and the log.
- `valid_minimal` (no hexes) shows the world panel and "This setting has no hexes.", and Step works there. A setting with errors shows neither panel nor the Step buttons.
- Automated tests cover stepping, the log, selection across Reload and Open, the deposit and countdown wording, world totals, and deposit counts.
- The engine crate and the `check` mode are unchanged.
- The single check command passes.
