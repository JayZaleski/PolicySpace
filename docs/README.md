# PolicySpace developer documentation

These documents explain how the code works, for developers who want to modify the project. The user manual (`manual.ipynb`) links here for technical detail.

| Document | Covers |
|---|---|
| [workspace.md](workspace.md) | The Cargo workspace, the crates and their dependency rule, the pinned toolchain, the check command, and test layout including the shared test helpers |
| [ids.md](ids.md) | The ID mechanism: `Id<K>`, `IdKind`, `NameRegistry<K>`, `IdVec<K, T>`, `NamedList<K, T>`, and `IdError` |
| [setting-loader.md](setting-loader.md) | The setting folder format, the loader's two phases, error reporting, the section-type framework, `EntryReader` and its accessors, and the setting checker |
| [goods.md](goods.md) | What a good is, its two properties, how `[[good]]` sections are loaded, and how to add a property |
| [map.md](map.md) | Hex coordinates and orientation, `HexMap`, how `[[hex]]` sections are loaded, and how to add a hex attribute |
| [currency.md](currency.md) | Each hex's currency good: `HexCurrencies`, why it is per-hex state and not part of the map, and how the `currency` key of `[[hex]]` is read and resolved |
| [resources.md](resources.md) | Resource types and deposits, the `Quantity` type, how `[[resource]]` and `[[deposit]]` sections are loaded and their references checked |
| [market.md](market.md) | The dummy market: each hex's posted prices (`HexPrices`), the `Market` exchange and its `TradeError` refusals, and how `[[market]]` sections are loaded and checked |
| [pops.md](pops.md) | POP types (the endowment, the Stone-Geary utility parameters and how shares are normalized, and the growth parameters) and placed POPs (`Pops`, continuous size, one POP per type per hex), how `[[pop_type]]` and `[[pop]]` sections are loaded and checked, and the per-turn endowment |
| [demand.md](demand.md) | Stone-Geary demand: the pure `demand` function, the `PriceList` trait the market implements, tradable and untradable goods, the at-or-above and below-subsistence rules, and the `Demand` result the trade round and growth build on |
| [trade.md](trade.md) | The trade round in part 2 of the turn: POP submissions, demand computed by the market, sales before purchases across a hex, the purchase cap, consumption, leftover currency, and the per-POP `PopOutcome` the World records |
| [growth.md](growth.md) | POP growth in part 2 of the turn: the subsistence index built from a POP's demand, the growth rate from its type's `growth` parameters (polynomial above subsistence, linear decline below, clamped), and how the World applies it to POP sizes |
| [world.md](world.md) | The running `World`, the three-part turn (emission and POP endowment, the trade round and POP growth, perishing), hex and POP stockpiles with perish groups and removal, per-POP trade outcomes, turn reports with the total population, and the setting checker's `--turns` option |
| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads and steps settings, the map math, the text it shows (turn lines, panels, POP sections), the window, and the headless `check` mode |
