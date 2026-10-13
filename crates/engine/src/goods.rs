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

use crate::id::{Id, IdError, IdKind, NameRegistry};

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
#[derive(Debug, Default)]
pub struct Goods {
    names: NameRegistry<GoodKind>,
    /// Indexed by `GoodId::index`.
    goods: Vec<Good>,
}

impl Goods {
    /// Builds the list. Crate-private: only the setting loader builds goods.
    ///
    /// # Panics
    /// Panics if `names` and `goods` have different lengths (a programming error).
    pub(crate) fn from_parts(names: NameRegistry<GoodKind>, goods: Vec<Good>) -> Self {
        assert_eq!(
            names.len(),
            goods.len(),
            "every good name needs exactly one set of properties"
        );
        Goods { names, goods }
    }

    /// Number of goods.
    pub fn len(&self) -> usize {
        self.goods.len()
    }

    /// True if the setting defines no goods.
    pub fn is_empty(&self) -> bool {
        self.goods.is_empty()
    }

    /// The ID of the good named `name`, or [`IdError::UnknownName`].
    pub fn id(&self, name: &str) -> Result<GoodId, IdError> {
        self.names.id(name)
    }

    /// The name of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn name(&self, id: GoodId) -> &str {
        self.names.name(id)
    }

    /// The properties of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn get(&self, id: GoodId) -> &Good {
        match self.goods.get(id.index()) {
            Some(good) => good,
            None => panic!(
                "{:?} is not from this list (it has {} goods)",
                id,
                self.goods.len()
            ),
        }
    }

    /// Every good as (ID, name, properties), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, &str, &Good)> + '_ {
        self.names
            .iter()
            .zip(&self.goods)
            .map(|((id, name), good)| (id, name, good))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(n: u32) -> Perishes {
        Perishes::After(NonZeroU32::new(n).unwrap())
    }

    fn sample() -> Goods {
        let mut names = NameRegistry::new();
        names.register("labor").unwrap();
        names.register("copper").unwrap();
        let goods = vec![
            Good {
                perishes: after(1),
                movable: false,
            },
            Good {
                perishes: Perishes::Never,
                movable: true,
            },
        ];
        Goods::from_parts(names, goods)
    }

    #[test]
    fn from_parts_with_matching_lengths_gives_a_usable_list() {
        let goods = sample();
        assert_eq!(goods.len(), 2);
        assert!(!goods.is_empty());
        let labor = goods.id("labor").unwrap();
        assert_eq!(labor.index(), 0);
        assert_eq!(goods.name(labor), "labor");
        assert_eq!(goods.get(labor).perishes, after(1));
        assert!(!goods.get(labor).movable);
        let copper = goods.id("copper").unwrap();
        assert_eq!(goods.get(copper).perishes, Perishes::Never);
        assert!(goods.get(copper).movable);
    }

    #[test]
    fn the_default_list_is_empty() {
        let goods = Goods::default();
        assert_eq!(goods.len(), 0);
        assert!(goods.is_empty());
        assert_eq!(goods.iter().count(), 0);
    }

    #[test]
    fn unknown_names_are_an_error() {
        assert_eq!(
            sample().id("iron"),
            Err(IdError::UnknownName {
                kind: "good",
                name: "iron".to_string()
            })
        );
    }

    #[test]
    fn iter_is_in_load_order() {
        let goods = sample();
        let listed: Vec<(usize, &str, Good)> = goods
            .iter()
            .map(|(id, name, good)| (id.index(), name, *good))
            .collect();
        assert_eq!(
            listed,
            vec![
                (
                    0,
                    "labor",
                    Good {
                        perishes: after(1),
                        movable: false
                    }
                ),
                (
                    1,
                    "copper",
                    Good {
                        perishes: Perishes::Never,
                        movable: true
                    }
                ),
            ]
        );
    }

    /// An ID that is out of range for `sample()` (which has two goods).
    fn foreign_id() -> GoodId {
        let mut names: NameRegistry<GoodKind> = NameRegistry::new();
        names.register("a").unwrap();
        names.register("b").unwrap();
        names.register("c").unwrap()
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn get_panics_on_a_foreign_id() {
        sample().get(foreign_id());
    }

    #[test]
    #[should_panic(expected = "is not from this registry")]
    fn name_panics_on_a_foreign_id() {
        sample().name(foreign_id());
    }

    #[test]
    #[should_panic(expected = "exactly one set of properties")]
    fn from_parts_panics_on_mismatched_lengths() {
        let mut names = NameRegistry::new();
        names.register("labor").unwrap();
        Goods::from_parts(names, Vec::new());
    }
}
