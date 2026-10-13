//! PolicySpace engine: a setting-agnostic, UI-agnostic simulation library.
//!
//! The engine never depends on any UI: user interfaces depend on the engine, never the
//! other way round. The engine knows nothing about any particular setting; settings
//! (the goods, the map, the rules of a world) are data that the engine loads. The engine
//! is deterministic: the same setting and the same inputs always give the same result.
//!
//! A setting is a folder of TOML files. [`load_setting`] reads and strictly checks one,
//! and returns a [`LoadedSetting`] or every problem found (see the [`setting`] module).
//! Goods are defined by the setting (see the [`goods`] module). The map is a set of hexes
//! in the setting's starting state (see the [`map`] module). Natural resources (resource
//! types in the rules, deposits on hexes in the starting state) are in the [`resources`]
//! module, and amounts of goods are [`Quantity`] values (see the [`quantity`] module).

pub mod goods;
pub use goods::{Good, GoodId, GoodKind, Goods, Perishes};
pub mod id;
pub use id::{Id, IdError, IdKind, NameRegistry};
pub mod map;
pub use map::{HexCoord, HexId, HexKind, HexMap};
pub mod quantity;
pub use quantity::Quantity;
pub mod resources;
pub use resources::{
    Deposit, DepositId, DepositKind, Deposits, ResourceId, ResourceKind, ResourceType,
    ResourceTypes,
};
pub mod setting;
pub use setting::{
    LoadError, LoadErrors, LoadedSetting, Manifest, Rules, SUPPORTED_FORMAT_VERSION, StartState,
    load_setting,
};

/// Human-readable name of the engine.
pub const ENGINE_NAME: &str = "PolicySpace Engine";
/// Engine version, taken from the crate's Cargo version.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
