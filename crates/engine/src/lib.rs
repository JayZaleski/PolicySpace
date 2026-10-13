//! PolicySpace engine: a setting-agnostic, UI-agnostic simulation library.
//!
//! The engine never depends on any UI: user interfaces depend on the engine, never the
//! other way round. The engine knows nothing about any particular setting; settings
//! (the goods, the map, the rules of a world) are data that the engine loads. The engine
//! is deterministic: the same setting and the same inputs always give the same result.

pub mod id;
pub use id::{Id, IdError, IdKind, NameRegistry};

/// Human-readable name of the engine.
pub const ENGINE_NAME: &str = "PolicySpace Engine";
/// Engine version, taken from the crate's Cargo version.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
