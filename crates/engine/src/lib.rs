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
//! Each hex designates a currency good (see the [`currency`] module), and may have a
//! dummy market that posts prices in that currency and exchanges goods against a stockpile
//! (see the [`market`] module). POP types (the per-individual endowment and Stone-Geary
//! utility parameters) are rules in the [`pops`] module. POPs (groups of individuals of a POP
//! type, placed on hexes) live in the same module; each POP receives its type's endowment
//! into its own stockpile every turn. The [`demand`] module computes a POP's Stone-Geary
//! demand from its type's utility, its holdings and a hex's prices, as a pure function. The
//! [`trade`] module runs each hex's trade round every turn: every POP submits its parameters
//! and holdings, the market computes demand and settles sales then purchases, and the POP
//! consumes; each POP's outcome is recorded. The [`growth`] module turns a POP's demand into
//! a subsistence index and a growth rate from its type's parameters, and the world applies
//! the rate to the POP's size after every hex has traded.
//! Technologies (the fixed-proportion recipes factories run, with their capital and
//! construction bundles, construction time and lifetime) are rules in the [`technologies`]
//! module, and a [`Bundle`] (the [`bundle`] module) is a list of goods with amounts; the
//! [`production`] module runs them. Businesses (the [`businesses`] module) are named actors in the
//! starting state, each on one hex with a decider name, a starting inventory and the
//! deposits it owns, each with its own stockpile in the world; an owned deposit emits into
//! its owner's stockpile; nothing decides or trades for them yet. Factories (the [`factories`]
//! module) are placed in the starting state, each on its owner's hex, running one
//! technology and holding a share (its condition) of that technology's capital bundle; the
//! [`production`] module gives them their mechanics as pure functions over one business's
//! factories and stockpile (running at a fraction of capacity with inputs shared when short,
//! wear of one lifetime-slice per turn with shutdown below one slice, and maintenance that
//! replenishes a fraction of the capital bundle at the cost of that fraction of the capital
//! and construction bundles); nothing in the turn calls them yet. The [`decider`] module
//! defines the decider protocol as data: the view a business is served each turn (its hex's
//! currency and posted prices, its stockpile by perish group, its cash, its factories, their
//! technologies and its deposits, and nothing outside its circumstances), the decision it
//! answers with (a run and a maintenance fraction per factory, offers, bids and an optional
//! trace), the validation of a decision against its view, the request and reply that carry
//! one turn's batch, and lossless JSON for all of it; a [`DeciderHost`] of named deciders (a
//! Python process the engine starts and keeps alive, a replayed recording, or a closure)
//! answers every business's view once per turn at the decision point, and the world stores
//! each business's answer; nothing resolves the decisions yet (Step 08).
//!
//! A running world is built from a loaded setting and stepped turn by turn (see the
//! [`world`] module). Each hex holds a stockpile of goods (see the [`stockpile`] module),
//! and turn numbers are [`Turn`] values (see the [`turn`] module).

pub mod bundle;
pub use bundle::Bundle;
pub mod businesses;
pub use businesses::{Business, BusinessId, BusinessKind, Businesses};
pub mod currency;
pub use currency::HexCurrencies;
pub mod decider;
pub use decider::{
    Answer, BusinessView, Decision, DecisionErrors, FormatError, Recording, Reply, ReplyErrors,
    Request,
};
pub use decider::{
    BusinessAnswer, Decider, DeciderFailure, DeciderHost, ReportedFailure, UnknownDecider,
};
pub mod demand;
pub use demand::{Demand, DemandedGood, PriceList, demand};
pub mod factories;
pub use factories::{Factories, Factory, FactoryId, FactoryKind};
pub mod goods;
pub use goods::{Good, GoodId, GoodKind, Goods, Perishes};
pub mod growth;
pub use growth::{grown_size, growth_rate, subsistence_index};
pub mod id;
pub use id::{Id, IdError, IdKind, NameRegistry, NamedList};
pub mod map;
pub use map::{HexCoord, HexId, HexKind, HexMap};
pub mod market;
pub use market::{HexPrices, Market, TradeError};
pub mod pops;
pub use pops::{
    Endowment, GrowthParams, Pop, PopId, PopKind, PopType, PopTypeId, PopTypeKind, PopTypes, Pops,
    UtilityGood,
};
pub mod production;
pub use production::{FactoryRecord, MaintenanceRecord, Operation, RunRecord, operate};
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
pub mod stockpile;
pub use stockpile::{PerishesAt, Stockpile};
pub mod technologies;
pub use technologies::{Technologies, Technology, TechnologyId, TechnologyKind};
pub mod trade;
pub use trade::PopOutcome;
pub mod turn;
pub use turn::Turn;
pub mod world;
pub use world::{StepError, TurnReport, World};
#[cfg(test)]
pub(crate) mod test_support;

/// Human-readable name of the engine.
pub const ENGINE_NAME: &str = "PolicySpace Engine";
/// Engine version, taken from the crate's Cargo version.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
