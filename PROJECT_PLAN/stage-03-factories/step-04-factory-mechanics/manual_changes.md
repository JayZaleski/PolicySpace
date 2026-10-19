# Step 04: factory-mechanics — Manual Changes

No user-facing changes: nothing in the turn, the turn reports, the setting format or the developer UI changes. The new factory mechanics can't be observed until Step 08 calls them in the turn.

One note for whoever writes the Technologies subsection (Step 01's draft, "What the numbers will mean"). This Step settles rules that setting authors pick numbers against, so that subsection's plain-language rules should match them. It still adds no new section and should still say that nothing runs yet:

- A factory shuts down once it holds less than one turn's worth of wear (1/lifetime of its capital), not only when its capital is completely used up. Each turn it runs costs exactly one such slice, and a leftover part-slice produces nothing. Condition doesn't slow a factory: it runs at full capacity until it shuts down. A full factory that is never maintained runs exactly `lifetime` turns.
- If `construction_turns` is greater than `lifetime`, Step 01's draft says the factory "wears out faster than it can be maintained". Add that once such a factory is exhausted, it can't be brought back by maintenance.
- Optional: Step 01's suggested chart of capital over time can show the shutdown threshold as a line at 1/lifetime, and the t > t₁ curve staying below it.

Link: [docs/production.md](docs/production.md) for the exact rules (running, shared shortages, wear, maintenance caps, the run-wear-maintain order).
