# Step 09: dev-ui-businesses — Docs Changes

This Step builds on earlier drafts in this Stage that are not in `docs/` yet: Step 04's `docs/production.md`, Step 05's `docs/decider.md`, Step 06's `docs/python.md`, Step 07's `docs/decider-host.md`, Steps 02 and 03's `docs/businesses.md` and `docs/factories.md`, and the edits Steps 01, 02, 07 and 08 make to `docs/dev-ui.md`, `docs/technologies.md`, `docs/workspace.md` and `docs/README.md`. Changes to those files below are relative to the latest draft of each section, and name it in **Supersedes**. Most of the change is in `docs/dev-ui.md`. The engine changes in one place: `production.rs`'s slice rule is now public as `slices_held`. The rest of the changes remove the "Step 09 shows this in the dev UI" notes that earlier drafts left in the "Not yet" sections.

## docs/dev-ui.md — modify
- **Location:** introduction, after the sentence Step 07 adds ("It also uses the decider host: ...")
- **Change:** add

For businesses and factories it reads `World::businesses` (with `on_hex`), `World::factories`, `Deposits::owned_by`, `World::business_stockpile`, `World::business_answer` (a `BusinessAnswer`), `World::business_outcome` (its `BusinessTrade` and per-factory `FactoryRecord`s) and the rules' technologies, and it calls the engine's `slices_held` to count a factory's remaining life (see [production.md](production.md#lifetime-slices-and-shutdown)).

## docs/dev-ui.md — modify
- **Location:** "Module structure", the table rows for `text.rs` and `test_support.rs`
- **Change:** replace those two rows
- **Supersedes:** Step 07 (decider-host), its replacement of the same two rows

| `text/mod.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `load_failure_lines`, `count`, `turn_line`, `failure_lines`) and the window's panels (countdowns, deposit lines, stockpile lines, market lines, POP sections, goods, resource-type and POP-type rows, and the index, rate and share formatting), built from engine types. No egui types. Re-exports the window-facing functions of `text/businesses.rs`. |
| `text/businesses.rs` | The business, factory and technology text: `business_section` (a hex's business sections), `technology_rows` and `business_rows` (the world panel's tables), and the helpers they use (`remaining_life`, `format_life`, `business_deposit_lines`, `business_stockpile_lines`). A child module of `text`, so it reaches `text`'s private helpers as `super::` items. No egui types. |
| `test_support.rs` | `#[cfg(test)]` helpers shared by the unit tests: `repo_path(relative)` (a path from the repository root); `world_with(relative, &mut host)` (loads a setting and returns its world at turn 0 under `host`, panicking with the load errors or an unknown decider name); `world_at(relative)` (`world_with` under the idle host); and `python_world(relative)` (the world under the setting's own Python deciders from `python::host_for`, returned with its host so a test can step it; tests using it need the project's virtual environment). |

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", first paragraph
- **Change:** replace

`text` builds every string the dev UI shows, without egui types, so it is unit-tested without a window. It is a directory module: `text/mod.rs` holds the `check` output and the hex, market, POP and world-table text, and `text/businesses.rs` the business, factory and technology text (see [Businesses, factories and technologies](#businesses-factories-and-technologies)). The child module re-exports only what `app.rs` uses (`business_section`, `business_rows`, `technology_rows`), because an unused re-export in this binary crate is an `unused_imports` warning and the check command runs clippy with `-D warnings`; its other items stay `pub` inside the module for its tests. The `check` command and the window share the load-error, count and turn-line functions, so they always use the same wording. Every list comes out in ID order (goods, resource types, deposits, prices, the POPs and businesses on a hex, the goods in a POP's table or a business's trade table, factories, technologies, POP types, businesses), straight from the engine's ordered accessors, so the panels are deterministic.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "**Numbers.**" paragraph, at the end of its first sentence ("... nothing is rounded.")
- **Change:** add after that sentence

The business text follows the same rule: fractions, conditions, wear and the cash-cap proportion are written with f64 `Display`, so a value such as the arithmetic decider's 0.12499999999999989 tools appears as sent.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "For the panels", the `deposit_lines(world, hex)` bullet
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its replacement of the bullet's last sentence

- `deposit_lines(world, hex)`: one line per resource type present on the hex: `{type} × {count}: {output} {good} each, {total} total`. The total is the output added to itself `count` times with `Quantity::saturating_add` rather than multiplied, the same way emission builds a stockpile by adding each deposit's output in turn. The lines list every deposit on the hex, owned or not; when a business owns some of them, the hex stockpile holds only the unowned deposits' output, so it is less than the line's total. The wording lives in the private `deposit_lines_from(world, count_of)`, which writes a line for each resource type whose `count_of(type)` is above 0; `deposit_lines` passes the count on the hex, and `business_deposit_lines` the count a business owns, so both read the same.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "For the panels", the `stockpile_lines(world, hex)` and `pop_stockpile_lines(world, pop)` bullet, its last sentence ("Use `stock_lines` for any future stockpile (an industry's, a business's).")
- **Change:** replace

`business_stockpile_lines(world, business)` wraps it too, so hex, POP and business stockpiles all read the same. Use `stock_lines` for any future stockpile.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "For the panels", the `good_rows(world)` bullet
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its addition at the end of the bullet

- `good_rows(world)`: one `GoodRow { name, lasts, movable, in_hexes, in_pops, in_businesses }` per good (`movable` is "movable" or "fixed"). `in_hexes` sums the good over every hex stockpile (the unowned output of deposits), `in_pops` over every POP stockpile (what POPs own), and `in_businesses` over every business stockpile (what businesses own: their owned deposits' output, their cash, what they bought and their factories' output). The three are kept apart because they are different things. Each is computed in one pass (over `world.map().iter()`, `world.pops().iter()` and `world.businesses().iter()`), adding each stockpile's totals into a per-good array with `Quantity::saturating_add`. A world without POPs has `"0"` in every `in_pops`, and one without businesses `"0"` in every `in_businesses`.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", new subsection at the end of the section (after the `pop_type_rows(world)` bullet)
- **Change:** add

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

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the "**World panel (left)**" paragraph and its list
- **Change:** replace the lead sentence and the Goods bullet, and add two bullets after the POP types bullet

**World panel (left)**, in one vertical `ScrollArea`, six collapsible sections (all open by default):

- *Goods*: a striped `Grid` (Name, Lasts, Movable, In hexes, In POPs, In businesses) from `text::good_rows`.

- *Technologies*: a striped nine-column `Grid` (Name, Recipe, Output, Capacity, Capital, Construction, Construction turns, Lifetime, Factories) from `text::technology_rows`. The Recipe, Capital and Construction cells are `ui.vertical` stacks with one label per line.
- *Businesses*: a striped six-column `Grid` (Name, Hex, Decider, Cash, Factories, Answer) from `text::business_rows`. A refused business's row draws all six cells in the theme's `error_fg_color`, so a refusal anywhere in the world shows without clicking hexes. The turn line doesn't print the report's decision counts, because this table shows the same thing per business.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the "**Hex panel (right)**" list, the "POPs" bullet
- **Change:** replace the bullet, and add a "Businesses" bullet after it

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

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the paragraph after the hex panel list ("Each `CollapsingHeader` gets an explicit `id_salt(("pop", pop index))`. ...") and the `stockpile_listing` paragraph after it
- **Change:** replace both paragraphs

Each repeated `CollapsingHeader` and `Grid` gets an explicit id salt from its item's index: `("pop", pop index)` for a POP's section, and `("business", business index)` for a business's, with the business's trade grid, factories grid and trace header salted the same way. egui keys a header's open/closed state by its ID, which defaults to the heading text; the headings hold the POP's size and the business's cash, which change every step, so without the salt a collapsed section would reopen after each step.

Every stockpile is drawn by the free function `stockpile_listing(ui, salt, lines)`: "Empty." for no lines, else each `StockLine.good` with its `parts` under `ui.indent(salt, ..)`. The hex passes the salt `"hex"`, a POP `("pop", pop index)` and a business `("business", business index)`, so the indents of different stockpiles don't share an ID, and they all look identical.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", the paragraph beginning "The world between steps is after part 2 of the current turn"
- **Change:** replace

The world between steps is after part 2 of the current turn (see [world.md](world.md)): every business has been served its view, answered, traded and operated its factories, every POP has been endowed, has traded, consumed and grown, and nothing has perished yet. So the POP and business sections show the current turn (the turn in the status bar): each POP's trade outcome, its size after growth and its stockpile after consumption; each business's answer at that turn's decision point, the trade and factory operation it led to, each factory's condition after that turn's wear and maintenance, and the business's stockpile after operation. Both panels read the world directly each frame, so they update straight after a step or reload.

## docs/dev-ui.md — modify
- **Location:** "The window: `app`", `draw_map` list, item 5 ("**Marks:** ...")
- **Change:** replace

5. **Marks:** a visible hex drawn as `Hexagon` or `LabelledHexagon` can get three marks, all in the foreground color, sized by one radius of `min(clamp(0.12 · scale, 2, 6), 0.35 · scale)` pixels (the cap keeps them inside the hexagon at small scales), and all placed on a ring of radius `offset = 0.55 · scale` around the center, off the label so they never cover the coordinates. A hex with at least one deposit (`HexLook::has_deposits`, from `!world.deposits().on_hex(id).is_empty()`) gets a filled circle at `center + (0, offset)`, below the label. A hex with at least one POP (`HexLook::has_pops`, from `!world.pops().on_hex(id).is_empty()`) gets a hollow circle (`circle_stroke`, 1 px) at `center − (0, offset)`, above the label. A hex with at least one business (`HexLook::has_businesses`, from `!world.businesses().on_hex(id).is_empty()`) gets a filled square of side `2 · radius` (`rect_filled`) at `center + (−0.866 · offset, 0.5 · offset)`, the lower left of the label, 60° round the ring from the deposit mark. A hex can show any combination of the three. Dots get no marks. The map draws no other data: no coloring by population or anything else.

## docs/dev-ui.md — modify
- **Location:** "Extending the window", the "**New hex-panel sections**" bullet and the "**New map marks**" bullet
- **Change:** replace both bullets

- **New hex-panel sections:** add them after "Businesses" in `hex_panel_contents`, built by a `text` function the way `pop_section` and `business_section` are. If `text/mod.rs` grows too long, put the new text in a child module beside `text/businesses.rs`, and re-export from `text` only what `app.rs` uses. Give any repeated `CollapsingHeader` or `Grid` an explicit id salt from the item's ID, never from text that changes between steps. Reuse `stock_lines` (through a public wrapper) and `stockpile_listing` for any stockpile, so every stockpile reads the same. Use color only where it carries information, from the theme's visuals (`error_fg_color`), never a hard-coded `Color32`.
- **New map marks:** add a field to `HexLook`, fill it in `draw_map`, and paint it in `paint_hex`'s hexagon arm, using the shared `offset` and `radius`. The marks sit on a ring around the label: the deposit mark below, the POP mark above, and the business mark at the lower left; the other three 60° positions on the ring are free.

## docs/dev-ui.md — modify
- **Location:** "Tests", first bullet, the opening ("Unit tests in `cli.rs` (every accepted and rejected argument form), `text.rs`, `geometry.rs` ...") and the sentence "The `text.rs` tests cover:"
- **Change:** replace `text.rs` with `text/mod.rs`, `text/businesses.rs` in the list of files, and replace "The `text.rs` tests cover:" with "The `text/mod.rs` tests cover:"

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text.rs` (now `text/mod.rs`) sub-bullet about goods' `in_hexes`
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its replacement of the same sub-bullet

  - goods' `in_hexes` totals on the sample (at turn 0 `0, 7.5, 16, 0, 0, 0, 0` in good ID order, only unowned deposit output; the same after steps; `tools` reads `0`, since nothing holds it) and `turns_valid`; `in_pops`: only silver on the sample, equal to the sum of every POP's silver, and all `"0"` on `turns_valid`; and `in_businesses`: on the sample under the idle host `0, 2.5, 8, 0, 0, 350, 0` at turn 0 and the per-good sums over every business stockpile after steps, under the `arithmetic` decider (with the project's Python) `12` grain, `2.5` copper and the per-good sums for silver and tools, and all `"0"` on `valid_minimal` and `turns_valid`;

## docs/dev-ui.md — modify
- **Location:** "Tests", first bullet, after the `text/mod.rs` sub-bullets (after the sub-bullet Step 08 adds) and before the paragraph "Numbers that come from floating-point growth ..."
- **Change:** add

  The `text/businesses.rs` tests cover:
  - `remaining_life` at its boundaries (exactly one slice, a hair under one, zero, lifetimes 3 and 24 whose slices aren't exact in floating point) and `format_life`'s wording;
  - remaining life agreeing with the engine: on `business_trade` under a closure decider that runs `works`'s two factories with no maintenance, stepped five times, each factory reads `n turns` for exactly as many further turns as it then produces output, and `shut down` from the turn of its last output;
  - the sample's three center businesses under the real `arithmetic` decider (with the project's Python) at turn 0: headings, decider and answer lines, a non-empty trace, owned deposit lines, the cash lines and proportion 1, every trade row, every factory row with its hover texts, and the stockpile; and after one step, the output sold and the new cash;
  - the same businesses under the idle host: answer nothing, no trace, no trade rows, factories at run and maintenance 0 with only their wear;
  - a refused decision (`business_trade` with an order for a factory the business doesn't own): the error lines, the trade rows with what was asked and 0 done, and the factories idle; and a refused decision naming a misspelt good, whose row comes after the known goods, with its trace shown;
  - a cash-short decision on `business_trade`: proportion `0.2`, the cut purchases, and the run done below the run asked;
  - `business_deposit_lines` and `business_stockpile_lines`, `technology_rows` on the sample, `business_trade` (`bakery`'s empty construction reads `nothing`, `smelter` has two factories) and `valid_minimal` (empty), and `business_rows` under both hosts and on `valid_minimal` (empty).

  Numbers that come from floating-point arithmetic (wear and conditions at lifetime 24, the decider's 0.12499999999999989 tools) are compared against the engine's own value formatted with `Display`.

## docs/dev-ui.md — modify
- **Location:** "Tests", the last bullet ("The window itself is checked by hand: ...")
- **Change:** replace

- The window itself is checked by hand: the layout, the hover texts, the error-colored rows and lines (a POP below subsistence, a refused business in the Businesses table and its section, a cash-short proportion, a shut-down factory), a collapsed POP or business section staying collapsed across a step, the collapsed Trace, the hex panel's sideways scroll, and the three map marks.

## docs/production.md — modify
- **Location:** "The model", "Wear", the paragraph beginning "**Rounding never changes a factory's life.**", its last sentence ("Running uses the same snapped slice count to decide whether a factory holds a slice.")
- **Change:** replace

Running uses the same snapped slice count to decide whether a factory holds a slice. The rule is public as `slices_held(condition, lifetime) -> f64`: `condition × lifetime`, snapped to the nearest whole number within `TOLERANCE`. A factory runs only when it is at least 1, and its `floor` is the number of further turns the factory can run with no maintenance. Callers outside the engine (the dev UI's remaining-life column) use it instead of copying the arithmetic, so they always agree with the engine about whether a factory runs.

## docs/production.md — modify
- **Location:** "Data model", the last bullet, as Step 08 replaced it ("**A cut is visible as `done < asked`** in either record. ...")
- **Change:** replace its last sentence ("Step 09 shows them in the dev UI.")
- **Supersedes:** Step 08 (business-trade), the same sentence

The dev UI shows them per business, one row per factory, with asked beside done (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## docs/production.md — modify
- **Location:** "Functions", the table
- **Change:** add a row at the end

| `slices_held(condition, lifetime)` | Counts the lifetime-slices a condition holds, snapped within `TOLERANCE` (see [Wear](#wear)). `run` uses it to decide eligibility and `wear` to take a slice | `f64` |

## docs/production.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), its replacement of the same section

The fractions come from a business's `Decision` (see [decider.md](decider.md)), which every business's decider sends at the decision point (see [decider-host.md](decider-host.md)); the first decider, the arithmetic rule, chooses them in Python with the same slice arithmetic as this module (see [python.md](python.md#orders-per-factory)). Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## docs/production.md — modify
- **Location:** "Tests", the `production.rs` unit tests bullet, after the sub-bullet "an unmaintained condition staying a whole number of slices;"
- **Change:** add

  - `slices_held` snapping within the tolerance: 8 for (1, 8), exactly 1 for (0.125, 8), below 1 for a hair under 0.125, 0 for (0, 8), exactly 2 for (2/3, 3) and exactly 24 for (1 − 1e-12, 24);

## docs/README.md — modify
- **Location:** the document table, the `production.md` row
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), the same row

| [production.md](production.md) | Factory mechanics as pure functions over one business's factories, called once per business in the turn: the run and maintenance fractions, running at a fraction of capacity with shortages shared proportionally (which is how the cash cap reaches the run level), shutdown below one lifetime-slice, wear with exact factory lifetimes, maintenance and its caps, the run-wear-maintain order, and the per-factory records (`Operation`, `RunRecord`, `MaintenanceRecord`, `FactoryRecord`, `operate`, and the public slice rule `slices_held`) |

## docs/README.md — modify
- **Location:** the document table, the `dev-ui.md` row
- **Change:** replace

| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads and steps settings under their deciders, the map math and the deposit, POP and business marks, the text it shows (turn lines, panels, POP sections, business sections with their trade and factory tables, technology and business tables), the window, and the headless `check` mode |

## docs/workspace.md — modify
- **Location:** the repository layout block, the `dev-ui/` `src/` entry
- **Change:** replace

```
    src/                main.rs, cli.rs, check.rs, text/ (mod.rs, businesses.rs),
                        geometry.rs, session.rs, app.rs, test_support.rs (see dev-ui.md)
```

## docs/businesses.md — modify
- **Location:** "Not yet", its first sentence
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), the same sentence

The dev UI shows each business on its hex (its decider and answer, owned deposits, cash through the round, trade, factories, trace and stockpile), lists every business in the world panel, and marks hexes with businesses on the map (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## docs/factories.md — modify
- **Location:** introduction, the second paragraph as Step 08 replaced it, its last sentence ("Showing factories in the dev UI is Step 09.")
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), the same sentence

The dev UI shows each business's factories with their condition, remaining life and this turn's record (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## docs/factories.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), the same section

Factories under construction, building, scrapping, retooling, and buying and selling factories are Stage 07. Factories on a hex other than their owner's wait for trade between hexes.

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the turn runs, wears and maintains them (see [production.md](production.md) and [trade.md](trade.md)). A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)), and the arithmetic rule tests each technology's margin at posted prices from those numbers (see [python.md](python.md#the-margin-test-per-technology)). The dev UI's world panel lists every technology with its numbers and the number of factories running it (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/decider.md — modify
- **Location:** "Not yet", its first sentence ("Showing decisions, traces and outcomes in the dev UI is Step 09 of Stage 03; the trace is kept in the business's stored answer, not in its outcome.")
- **Change:** replace
- **Supersedes:** Step 08 (business-trade), the same sentence

The trace is kept in the business's stored answer, not in its outcome; the dev UI shows it, with the decision and its outcome, in each business's section (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)).

## docs/decider-host.md — modify
- **Location:** "Concepts" (or wherever Step 07 placed it), the bullet "A refused decision is kept with its errors, so a developer can see what was sent and why it was refused (Step 09 shows it). It counts as deciding nothing."
- **Change:** replace

- A refused decision is kept with its errors, so a developer can see what was sent and why it was refused: the dev UI shows the errors, the offers and bids asked for, and the trace (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). It counts as deciding nothing.

## docs/decider-host.md — modify
- **Location:** "Not yet", its first sentence
- **Change:** remove
- **Supersedes:** Step 08 (business-trade), its replacement of the same sentence ("Showing each business's answer, its trace, its refusals and its outcome in the dev UI is Step 09 of Stage 03; ...")

If removing it leaves "Not yet" empty, replace the section body with: "The dev UI shows each business's answer, its refusals and trace, and its outcome (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). Taking over a business and sending its decisions from the window is Stage 05."

## docs/python.md — modify
- **Location:** "The arithmetic rule", "The trace", first sentence ("Every decision carries a trace, the only account of why the business did what it did (Step 09 shows it in the dev UI).")
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same sentence

Every decision carries a trace, the only account of why the business did what it did. The dev UI shows it, collapsed, in each business's section on the hex panel.

## docs/python.md — modify
- **Location:** "The arithmetic rule", the paragraph beginning "The slice arithmetic mirrors `production.rs` exactly", its first sentence
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same sentence

The slice arithmetic mirrors `production.rs` exactly, so "holds a slice" and the condition after wear are the engine's numbers: `slices(condition, lifetime)` is the engine's public `slices_held` (condition × lifetime, rounded when within `TOLERANCE = 1e-9` of a whole number), and `holds_a_slice`, `snap` (to 0 or 1 within the tolerance) and `after_wear` (one slice lost, stopping at zero) follow the engine's private helpers.

## docs/python.md — modify
- **Location:** "Not yet", its first sentence ("Showing traces in the dev UI is Step 09 of Stage 03.")
- **Change:** remove
- **Supersedes:** Step 08 (business-trade), its replacement of the same sentence
