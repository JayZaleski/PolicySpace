//! Goods: everything an actor can own.
//!
//! A good is anything an actor can own: labor, resource output, intermediate goods, and
//! consumption goods alike. There are no built-in goods and no categories such as "labor"
//! or "capital"; every good comes from the setting's `[[good]]` sections. The engine
//! understands exactly two properties of a good:
//!
//! - **`perishes`** ([`Perishes`]): when units of the good perish. A good either never
//!   perishes (a durable good), or each unit survives a whole number of turn-ends, counting
//!   the turn it appears in. Making goods perish happens with the turn clock, not here.
//! - **`movable`**: whether the good can be moved between places. It has no effect yet;
//!   movement and trade come after the Closed Market Stage.
//!
//! Goods are identified by [`GoodId`], which is valid only within the [`Goods`] list (the
//! setting) that produced it. This module knows nothing about files: the setting loader
//! builds a [`Goods`] from a checked setting, and everything else only reads it.

use std::num::NonZeroU32;

use crate::id::{Id, IdKind, NamedList};

/// Marker kind for goods.
pub enum GoodKind {}

impl IdKind for GoodKind {
    const LABEL: &'static str = "good";
}

/// The ID of a good.
pub type GoodId = Id<GoodKind>;

/// When units of a good perish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Perishes {
    /// Never perishes (a durable good). The default when `perishes_after` is absent.
    Never,
    /// A unit survives this many turn-ends, counting the turn it appears in:
    /// `After(1)` is usable in the turn it appears and gone when that turn ends;
    /// `After(3)` appearing in turn 5 is gone at the end of turn 7.
    After(NonZeroU32),
}

/// The properties of one good, with defaults already applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Good {
    /// When units of this good perish.
    pub perishes: Perishes,
    /// Whether the good can be moved between places. No effect yet (movement and trade
    /// come after the Closed Market Stage). Defaults to true.
    pub movable: bool,
}

/// Every good in a setting, in load order (which is also ID order).
pub type Goods = NamedList<GoodKind, Good>;
