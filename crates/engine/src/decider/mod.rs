//! The decider protocol as data: what a business is told each turn, what it may answer,
//! how an answer is checked against what it was told, and how both travel as JSON.
//!
//! The view is taken at the start of "actors act", before any trading, so the stockpile
//! holds this turn's emission, last turn's output and the carried-over currency; it holds
//! only what concerns the business (its hex's currency and posted prices, its stockpile by
//! perish group, its cash, its factories by ID, their technologies once each, its deposits
//! per resource type), never other actors or hexes. A decision names its business and
//! carries, per factory, a run and a maintenance fraction (0 to 1; an unmentioned factory
//! is left alone), offers and bids per good, and an optional free-text trace the engine
//! stores and never acts on. Validation refuses nonsense (a factory not in the view, an
//! unknown good, a bad fraction or amount, the currency, an unpriced good) and reports
//! every problem at its place; asking for more than the business holds or can pay is a
//! valid request that Step 08 trims. One turn's exchange is a [`Request`] (every view) and
//! a [`Reply`] (any subset of decisions, in any order); a [`Recording`] keeps both. Every
//! type reads and writes JSON losslessly, keyed by setting names and factory ID numbers,
//! never by engine IDs. The [`host`] module runs the exchange once per turn through a
//! [`DeciderHost`] of named [`Decider`]s (an external process, a replayed recording, or a
//! Rust closure), stores each business's [`BusinessAnswer`] and reports every
//! [`DeciderFailure`]; Step 08 resolves the valid decisions.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, DeserializeOwned, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

pub mod batch;
pub mod decision;
pub mod external;
pub mod host;
pub mod python;
pub mod replay;
pub mod view;

pub use batch::{Answer, Recording, Reply, ReplyError, ReplyErrors, Request};
pub use decision::{Decision, DecisionError, DecisionErrors, FactoryOrder, Place};
pub use external::{ExternalCommand, ExternalDecider, ProcessConfig, StartError};
pub use host::{
    BusinessAnswer, ClosureDecider, Decider, DeciderFailure, DeciderHost, DecisionCounts,
    ReportedFailure, UnknownDecider,
};
pub use python::{HostError, PythonError};
pub use replay::{ReplayDecider, ReplayError};
pub use view::{BusinessView, DepositsView, FactoryView, PerishGroup, TechnologyView};

/// Why a message couldn't be read from JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The text isn't the JSON the format expects: a syntax error, a wrong type, a missing
    /// or unknown key, a key written twice, a number out of its type's range. The parser's
    /// message, which ends with the line and column.
    Json(String),
    /// The JSON parsed, but holds values no message may: each problem at its place, in
    /// document order (`"factory #0: run must be ..."`, or in a batch
    /// `"center_farm: factory #0: run must be ..."`).
    Values(Vec<String>),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Json(message) => write!(f, "{message}"),
            FormatError::Values(lines) => write!(f, "{}", lines.join("\n")),
        }
    }
}

impl Error for FormatError {}

/// Serializes `value` as compact JSON (one line).
pub(crate) fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("the protocol's types always serialize")
}

/// Serializes `value` as indented JSON, for recordings people read and diff.
pub(crate) fn to_json_pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("the protocol's types always serialize")
}

/// Parses `text`, mapping a parser error to `FormatError::Json(error.to_string())`.
pub(crate) fn from_json<T: DeserializeOwned>(text: &str) -> Result<T, FormatError> {
    serde_json::from_str(text).map_err(|error| FormatError::Json(error.to_string()))
}

/// Turns a non-empty list of value problems into `Err(FormatError::Values(problems))`,
/// and an empty one into `Ok(())`.
pub(crate) fn checked(problems: Vec<String>) -> Result<(), FormatError> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(FormatError::Values(problems))
    }
}

/// A `#[serde(deserialize_with = "no_duplicate_keys")]` helper for every `BTreeMap` field:
/// inserts each entry and fails when a key repeats, since `BTreeMap` would otherwise keep
/// the last value silently.
pub(crate) fn no_duplicate_keys<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + fmt::Display,
    V: Deserialize<'de>,
{
    struct UniqueKeys<K, V>(PhantomData<fn() -> (K, V)>);

    impl<'de, K, V> Visitor<'de> for UniqueKeys<K, V>
    where
        K: Deserialize<'de> + Ord + fmt::Display,
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<K, V>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "a table")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<K, V>()? {
                if map.contains_key(&key) {
                    return Err(de::Error::custom(format!(
                        "the key '{key}' is written twice"
                    )));
                }
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(UniqueKeys(PhantomData))
}

/// True for a finite number from 0 to 1 (a run, maintenance or condition fraction).
pub(crate) fn is_fraction(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// True for a finite number of at least 0 (an amount; the same rule as `Quantity::new`).
pub(crate) fn is_amount(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}
