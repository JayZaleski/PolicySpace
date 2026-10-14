# Step 08: dev-ui-inspect-and-step — Manual Changes

No change for players. Setting authors and developers can now inspect a loaded world and step turns in the developer UI.

## What the manual should cover
- **Selecting a hex:** click a hex to select it (a thick accent-colored outline, distinct from the hover highlight). When hexes are drawn as dots, a click near a dot selects it. Click empty space or press Esc to clear. Dragging to pan never changes the selection.
- **Deposit marks:** hexes with at least one deposit show a small mark when drawn as hexagons (not as dots), so the author can see where to click.
- **Stepping:** new **Step** and **Step ×10** buttons in the top bar, with shortcuts Space and Shift+Space. They work whenever a setting loaded without errors, even one with no hexes. The status bar turn follows. Mention that the world starts at turn 0 on every load.
- **The world panel (left):** a turn log (every turn report since the load, turn 0 included, newest first, same wording as `check --turns`), a Goods table (name, how long it lasts, movable or fixed, amount held in the whole world), and a Resource types table (name, produced good, output per deposit, deposit count). Sections collapse; panels can be resized.
- **The hex panel (right):** for the selected hex, its coordinates, its deposits grouped by type (for example `arable_land × 3: 4 unimproved_grain each, 12 total`), and its stockpile with each good's total and the parts of it with a countdown to when they perish (`perishes this turn`, `perishes in N turns`, `never perishes`). Explain how to read the countdown, with the `turns_valid` example at turn 2 (hex (0, 0) holds 15 `stew`: 5 perish this turn, 5 in 1 turn, 5 in 2 turns). Messages: "Click a hex to inspect it.", "No deposits.", "Empty.", and "This setting has no hexes."
- **Reload and Open:** Reload returns to turn 0 with a log of one line and keeps the selection if the reloaded map still has a hex at those coordinates (so an author can edit deposits, Reload, and see the result at once). A failed Reload doesn't lose the selection. Opening a different folder clears the selection and log. A setting with errors still shows the error list with no side panels and disabled Step buttons.
- **Update earlier text:** this supersedes Step 07's manual notes "Clicking does nothing yet", "current turn (always 0 for now)", and "Not yet: showing deposits, goods, or stockpiles, selecting hexes, and stepping turns". Nothing is removed from the controls (pan, zoom, Fit, hover, Reload) or from `check`.
- **Not available:** auto-run, stepping by another number, coloring hexes by data, editing the world, and selecting several hexes.

## Where it fits
Extend the "The developer UI" subsection (added by Step 07) in the "Writing a setting" material, with new short parts: "Inspecting a hex", "Stepping turns", and "The world panel". An author's workflow paragraph is natural: edit deposits or goods, Reload, step a few turns, and check stockpiles. The "Running a setting" subsection (Step 06) can mention that the window shows the same turn lines as `check --turns`.

## Links
- `docs/dev-ui.md` for the session, the panel text, the window, and shortcuts.
- `docs/world.md` for turns, stockpiles and perishing, and `docs/resources.md` (if present in `docs/`) or the relevant resource doc for deposits.
- `docs/goods.md` (if present) for the goods' properties.

## Example
A screenshot of the window with the sample setting, hex (0, 0) selected, and a few steps taken would be best: it would show the outline, deposit marks, both panels, and the turn log. A second screenshot or a small table walking through the `turns_valid` stew stockpile over turns 0 to 3 would show how the countdown reads. An interactive demo is not needed.
