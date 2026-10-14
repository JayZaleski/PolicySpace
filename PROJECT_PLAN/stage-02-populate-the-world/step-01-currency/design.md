# Step 01: currency

## Summary
Every hex designates one good as its currency: the good that prices on that hex are posted in. The designation is part of the starting state, written on each hex, and the engine keeps it as per-hex state because it can change during the simulation. This Step adds nothing else: no prices, no trade, no way to change the designation. It gives Step 02's dummy market a currency to post prices in, and adds a currency good to the sample setting.

## Behavior

**In the setting folder.** Each `[[hex]]` in `start/` names its currency with a required `currency` key whose value is the name of a good defined in `rules/`. A hex with no `currency` key is an error, as is a value that is not a string. A value that matches no good (including a badly spelled name) is reported at the value as an unknown good, found in the loader's second phase once every file has been read, so the good may be defined in any `rules/` file. This is how the loader already treats a resource type's `produces` and a deposit's `resource`. Like other reference checks, it is skipped when the loader could not see every file.

**What the good may be.** The only requirement is that the good exists. The engine has no opinion about money: a perishable or non-movable good is a valid currency. A setting author who designates labor as currency gets exactly that.

**Every hex has one.** The designation is required, so there is no such thing as a hex without a currency, and later Steps never handle that case. Different hexes may designate different goods. An empty map is still valid.

**In the engine.** The map remains geometry only. The currency is a separate per-hex value: the loaded starting state carries one currency per hex, and the World takes it over as per-hex state alongside the hex stockpiles, read per hex. It is state rather than a rule because the vision says the designation can change during play (for example when a central bank issues a synthetic currency). Nothing in this Stage changes it, and there is no way to change it yet.

**Determinism and order.** Currencies are stored per hex in hex ID order, so loading the same folder always gives the same result. The turn is unchanged: emission, perishing, and reports do not depend on currencies.

## Player-facing Changes
None visible yet. A setting author writes `currency = "<good>"` on every hex. The sample setting gains a `silver` good (durable, movable) and every one of its 19 hexes designates it, with a comment noting that hexes may differ. The checker's success line is unchanged, since currencies are not a new kind of thing to count. The dev UI shows a hex's currency in Step 08.

## Interactions
- **Goods (Stage 01 Step 03):** the currency is a reference to a good, resolved against the goods registry like a resource type's `produces`.
- **Hex map (Stage 01 Step 04):** the key is read as part of each `[[hex]]` entry. The map's own data (coordinates, IDs, duplicate checks) is unchanged.
- **World (Stage 01 Step 06):** the World gains a per-hex currency next to its per-hex stockpiles. The turn does not use it.
- **Dummy market (Step 02):** posts prices in the hex's currency. **POP trade (Step 06):** pays and is paid in it. **Dev UI (Step 08):** displays it.
- **Test settings:** every existing test setting with hexes must gain a `currency` key so it keeps loading, or keeps reporting exactly the errors it is meant to.

## Out of Scope
- Changing a hex's currency during play, and any rule for what happens to prices or holdings when it changes.
- Starting stockpiles of currency or any other good.
- Prices, markets, and trade (Step 02 onward).
- Any currency-specific good property, or any restriction on which goods may be currencies.
- A map-wide default currency to avoid repeating the key on every hex.
- Showing the currency in the dev UI (Step 08) or in the checker's success line.

## Acceptance Criteria
- The sample setting loads without errors, and every one of its hexes reports `silver` as its currency.
- A test setting whose hexes name different currencies loads, and each hex reports its own.
- A `[[hex]]` with no `currency` key, one whose value is not a string, one whose value is a badly spelled name, and one naming a good that does not exist each produce one error at the expected file, line, and column, with the expected message text.
- The unknown-good check is skipped when another file in the setting is broken, so no misleading "unknown good" error is reported.
- A hex's currency is readable from a loaded setting and from a running World, and stepping the World does not change it.
- All existing tests pass after the test settings gain the key.
