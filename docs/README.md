# PolicySpace developer documentation

These documents explain how the code works, for developers who want to modify the project. The user manual (`manual.ipynb`) links here for technical detail.

| Document | Covers |
|---|---|
| [workspace.md](workspace.md) | The Cargo workspace, the crates and their dependency rule, the pinned toolchain, the check command, and test layout |
| [ids.md](ids.md) | The ID mechanism: `Id<K>`, `IdKind`, `NameRegistry<K>`, `IdVec<K, T>`, `NamedList<K, T>`, and `IdError` |
| [setting-loader.md](setting-loader.md) | The setting folder format, the loader's two phases, error reporting, the section-type framework, and the setting checker |
| [goods.md](goods.md) | What a good is, its two properties, how `[[good]]` sections are loaded, and how to add a property |
| [map.md](map.md) | Hex coordinates and orientation, `HexMap`, how `[[hex]]` sections are loaded, and how to add a hex attribute |
| [resources.md](resources.md) | Resource types and deposits, the `Quantity` type, how `[[resource]]` and `[[deposit]]` sections are loaded and their references checked |
| [world.md](world.md) | The running `World`, the three-part turn, stockpiles with perish groups, turn reports, and the setting checker's `--turns` option |
| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads and steps settings, the map math, the text it shows, the window, and the headless `check` mode |
