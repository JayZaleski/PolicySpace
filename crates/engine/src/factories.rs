//! Factories. A factory (`[[factory]]` in `start/`) is a discrete asset on a hex that runs
//! one technology (see `technologies`) and belongs to exactly one business (see
//! `businesses`). It carries the state that changes during play: which technology it runs,
//! and its condition, the share of its technology's full capital bundle it holds (0 to 1;
//! the goods in the factory are the technology's capital bundle times the condition,
//! derived whenever needed, because depreciation and maintenance only ever change the
//! bundle in its own proportions). Factories have no names: deciders, the dev UI and later
//! decisions address them by ID. A factory sits on its owner's hex until trade between
//! hexes exists (the loader checks this). A hex and a business may hold any number of
//! factories. Nothing runs, wears, maintains or shuts down a factory yet (Steps 04 and 08),
//! and nothing changes a factory during play yet. Knows nothing about files.

use crate::businesses::BusinessId;
use crate::id::{Id, IdKind, IdVec};
use crate::map::{HexId, HexKind};
use crate::technologies::TechnologyId;

/// Marker kind for factories.
pub enum FactoryKind {}

impl IdKind for FactoryKind {
    const LABEL: &'static str = "factory";
}

/// The ID of a factory. Valid only within the [`Factories`] that produced it.
pub type FactoryId = Id<FactoryKind>;

/// One factory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Factory {
    /// The hex it sits on: its owner's hex, until trade between hexes exists.
    pub hex: HexId,
    /// The technology it runs. State: a later Stage can retool it.
    pub technology: TechnologyId,
    /// The business that owns it. Exactly one; every factory has an owner.
    pub owner: BusinessId,
    /// The share of its technology's full capital bundle it holds: finite, from 0 to 1.
    /// A setting places a factory with a condition greater than 0 (1 if none is written);
    /// once depreciation runs (Step 04) the engine may store 0.
    pub condition: f64,
}

/// Every factory in a setting, in load order (which is also ID order), with a per-hex
/// index. `Factories::default()` is empty and was built with no map, so
/// [`on_hex`](Self::on_hex) panics on it for any hex.
#[derive(Debug, Default)]
pub struct Factories {
    /// One entry per factory, by `FactoryId`.
    factories: IdVec<FactoryKind, Factory>,
    /// One entry per hex, by `HexId`: the factories on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<FactoryId>>,
}

impl Factories {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map. Crate-private: only the loader builds factories.
    ///
    /// # Panics
    /// Panics if a factory's hex index is `>= hex_count` (`"{hex:?} is not from a map of
    /// {hex_count} hexes"`), if a condition isn't a finite number from 0 to 1 (`"a
    /// factory's condition must be a finite number from 0 to 1"`), or if there are more
    /// than `u32::MAX` factories.
    pub(crate) fn from_factories(factories: Vec<Factory>, hex_count: usize) -> Self {
        let factories = IdVec::from_vec(factories);
        let mut by_hex: IdVec<HexKind, Vec<FactoryId>> =
            IdVec::from_vec(vec![Vec::new(); hex_count]);
        for (id, factory) in factories.iter() {
            assert!(
                factory.hex.index() < hex_count,
                "{:?} is not from a map of {hex_count} hexes",
                factory.hex
            );
            assert!(
                factory.condition.is_finite() && (0.0..=1.0).contains(&factory.condition),
                "a factory's condition must be a finite number from 0 to 1"
            );
            by_hex.get_mut(factory.hex).push(id);
        }
        Factories { factories, by_hex }
    }

    /// Number of factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// True if the setting has no factories.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// The factory `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn get(&self, id: FactoryId) -> &Factory {
        self.factories.get(id)
    }

    /// Every factory as (ID, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (FactoryId, &Factory)> + '_ {
        self.factories.iter()
    }

    /// The factories on `hex`, in ID order; empty if it has none.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these factories were built with.
    pub fn on_hex(&self, hex: HexId) -> &[FactoryId] {
        self.by_hex.get(hex)
    }

    /// The factories owned by `business`, in ID order (a filter over every factory).
    pub fn owned_by(&self, business: BusinessId) -> impl Iterator<Item = FactoryId> + '_ {
        self.factories
            .iter()
            .filter(move |(_, factory)| factory.owner == business)
            .map(|(id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::businesses::BusinessKind;
    use crate::id::NameRegistry;
    use crate::technologies::TechnologyKind;

    fn ids<K: IdKind>(count: usize) -> Vec<Id<K>> {
        let mut names: NameRegistry<K> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("n{i}")).unwrap())
            .collect()
    }

    fn factory(hex: HexId, technology: TechnologyId, owner: BusinessId, condition: f64) -> Factory {
        Factory {
            hex,
            technology,
            owner,
            condition,
        }
    }

    /// Three hexes, two technologies, two businesses; factories: 0 t0 b0 @h0 (1.0),
    /// 1 t1 b1 @h0 (0.5), 2 t0 b0 @h2 (0.25), 3 t0 b0 @h0 (1.0).
    fn sample_factories() -> Factories {
        let h = ids::<HexKind>(3);
        let t = ids::<TechnologyKind>(2);
        let b = ids::<BusinessKind>(2);
        Factories::from_factories(
            vec![
                factory(h[0], t[0], b[0], 1.0),
                factory(h[0], t[1], b[1], 0.5),
                factory(h[2], t[0], b[0], 0.25),
                factory(h[0], t[0], b[0], 1.0),
            ],
            3,
        )
    }

    fn indexes(ids: &[FactoryId]) -> Vec<usize> {
        ids.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn factories_get_sequential_ids_in_the_order_given() {
        let factories = sample_factories();
        assert_eq!(factories.len(), 4);
        assert!(!factories.is_empty());
        let listed: Vec<(usize, usize, usize, usize, f64)> = factories
            .iter()
            .map(|(id, f)| {
                (
                    id.index(),
                    f.hex.index(),
                    f.technology.index(),
                    f.owner.index(),
                    f.condition,
                )
            })
            .collect();
        assert_eq!(
            listed,
            vec![
                (0, 0, 0, 0, 1.0),
                (1, 0, 1, 1, 0.5),
                (2, 2, 0, 0, 0.25),
                (3, 0, 0, 0, 1.0)
            ]
        );
        let (id, _) = factories.iter().nth(2).unwrap();
        assert_eq!(factories.get(id).hex.index(), 2);
        assert_eq!(factories.get(id).condition, 0.25);
    }

    #[test]
    fn on_hex_lists_factories_in_id_order() {
        let factories = sample_factories();
        let h = ids::<HexKind>(3);
        assert_eq!(indexes(factories.on_hex(h[0])), vec![0, 1, 3]);
        assert!(factories.on_hex(h[1]).is_empty());
        assert_eq!(indexes(factories.on_hex(h[2])), vec![2]);
    }

    #[test]
    fn owned_by_lists_only_that_owners_factories_in_id_order() {
        let factories = sample_factories();
        let b = ids::<BusinessKind>(3);
        let owned = |business| -> Vec<usize> {
            factories.owned_by(business).map(|id| id.index()).collect()
        };
        assert_eq!(owned(b[0]), vec![0, 2, 3]);
        assert_eq!(owned(b[1]), vec![1]);
        assert!(owned(b[2]).is_empty());
    }

    #[test]
    fn the_default_list_is_empty() {
        let factories = Factories::default();
        assert_eq!(factories.len(), 0);
        assert!(factories.is_empty());
        assert_eq!(factories.iter().count(), 0);
    }

    #[test]
    fn a_condition_of_exactly_0_and_exactly_1_is_accepted() {
        let h = ids::<HexKind>(1);
        let t = ids::<TechnologyKind>(1);
        let b = ids::<BusinessKind>(1);
        let factories = Factories::from_factories(
            vec![
                factory(h[0], t[0], b[0], 0.0),
                factory(h[0], t[0], b[0], 1.0),
            ],
            1,
        );
        assert_eq!(factories.len(), 2);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn get_panics_on_a_foreign_id() {
        sample_factories().get(FactoryId::from_index(4));
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_a_hex_beyond_the_map() {
        sample_factories().on_hex(HexId::from_index(3));
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_the_default_list() {
        Factories::default().on_hex(HexId::from_index(0));
    }

    #[test]
    #[should_panic(expected = "is not from a map of 2 hexes")]
    fn from_factories_panics_on_a_hex_beyond_hex_count() {
        let h = ids::<HexKind>(3);
        let t = ids::<TechnologyKind>(1);
        let b = ids::<BusinessKind>(1);
        Factories::from_factories(vec![factory(h[2], t[0], b[0], 1.0)], 2);
    }

    fn build_with_condition(condition: f64) {
        let h = ids::<HexKind>(1);
        let t = ids::<TechnologyKind>(1);
        let b = ids::<BusinessKind>(1);
        Factories::from_factories(vec![factory(h[0], t[0], b[0], condition)], 1);
    }

    #[test]
    #[should_panic(expected = "a factory's condition must be a finite number from 0 to 1")]
    fn from_factories_panics_on_a_condition_above_1() {
        build_with_condition(1.5);
    }

    #[test]
    #[should_panic(expected = "a factory's condition must be a finite number from 0 to 1")]
    fn from_factories_panics_on_a_negative_condition() {
        build_with_condition(-0.1);
    }

    #[test]
    #[should_panic(expected = "a factory's condition must be a finite number from 0 to 1")]
    fn from_factories_panics_on_a_nan_condition() {
        build_with_condition(f64::NAN);
    }
}
