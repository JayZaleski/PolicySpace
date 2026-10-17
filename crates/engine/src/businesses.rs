//! Businesses. A business (`[[business]]` in `start/`) is a named actor on one hex: it has
//! a decider name (the name of the decider that will control it; Step 07 resolves it, this
//! Step only stores it), a starting inventory (the goods, cash included, in its stockpile
//! when the world is created; after that the stockpile in the [`world`](crate::world)
//! module is the truth), and owns deposits on its hex (written on the deposit, see
//! [`resources`](crate::resources)). A business may own only deposits on its hex, and may
//! sit on a hex with no market. Business names are their own namespace. Nothing decides or
//! trades for a business yet. Knows nothing about files.

use crate::bundle::Bundle;
use crate::id::{Id, IdError, IdKind, IdVec, NameRegistry, NamedList};
use crate::map::{HexId, HexKind};

/// Marker kind for businesses.
pub enum BusinessKind {}

impl IdKind for BusinessKind {
    const LABEL: &'static str = "business";
}

/// The ID of a business. Valid only within the [`Businesses`] that produced it.
pub type BusinessId = Id<BusinessKind>;

/// One business. Its stockpile is world state, not part of this record.
#[derive(Debug, Clone, PartialEq)]
pub struct Business {
    /// The hex it sits on (its only hex until trade between hexes exists).
    pub hex: HexId,
    /// The name of the decider that controls it. A valid name; not resolved yet.
    pub decider: String,
    /// What its stockpile holds when the world is created, cash included. May be empty.
    /// Every amount is greater than 0. The World fills the stockpile from this at turn 0.
    pub starting_inventory: Bundle,
}

/// Every business in a setting, in load order (which is also ID order), with a per-hex
/// index. `Businesses::default()` is empty and was built with no map, so `on_hex` panics
/// on it for any hex.
#[derive(Debug, Default)]
pub struct Businesses {
    list: NamedList<BusinessKind, Business>,
    /// One entry per hex, by `HexId`: the businesses on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<BusinessId>>,
}

impl Businesses {
    /// Crate-private: only the loader builds businesses. `hex_count` is the number of
    /// hexes in the map.
    ///
    /// # Panics
    /// Panics if `names` and `businesses` differ in length (through `NamedList::from_parts`,
    /// `"every business name needs exactly one entry"`), if a business's hex index is
    /// `>= hex_count` (`"{hex:?} is not from a map of {hex_count} hexes"`), or if there are
    /// more than `u32::MAX` businesses.
    pub(crate) fn from_parts(
        names: NameRegistry<BusinessKind>,
        businesses: Vec<Business>,
        hex_count: usize,
    ) -> Self {
        let list = NamedList::from_parts(names, businesses);
        let mut by_hex: IdVec<HexKind, Vec<BusinessId>> =
            IdVec::from_vec(vec![Vec::new(); hex_count]);
        for (id, _, business) in list.iter() {
            assert!(
                business.hex.index() < hex_count,
                "{:?} is not from a map of {hex_count} hexes",
                business.hex
            );
            by_hex.get_mut(business.hex).push(id);
        }
        Businesses { list, by_hex }
    }

    /// Number of businesses.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// True if the setting has no businesses.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The ID of the business named `name`, or `IdError::UnknownName { kind: "business", .. }`.
    pub fn id(&self, name: &str) -> Result<BusinessId, IdError> {
        self.list.id(name)
    }

    /// The name of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list (`not from this registry`).
    pub fn name(&self, id: BusinessId) -> &str {
        self.list.name(id)
    }

    /// The business `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list (`is not from this list`).
    pub fn get(&self, id: BusinessId) -> &Business {
        self.list.get(id)
    }

    /// Every business as (ID, name, business), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (BusinessId, &str, &Business)> + '_ {
        self.list.iter()
    }

    /// The businesses on `hex`, in ID order; empty if none.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map the list was built with.
    pub fn on_hex(&self, hex: HexId) -> &[BusinessId] {
        self.by_hex.get(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goods::GoodId;
    use crate::quantity::Quantity;

    fn hex_ids(count: usize) -> Vec<HexId> {
        let mut names: NameRegistry<HexKind> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("h{i}")).unwrap())
            .collect()
    }

    fn registry(names: &[&str]) -> NameRegistry<BusinessKind> {
        let mut registry = NameRegistry::new();
        for name in names {
            registry.register(name).unwrap();
        }
        registry
    }

    fn business(hex: HexId, decider: &str, inventory: &[(u32, f64)]) -> Business {
        Business {
            hex,
            decider: decider.to_string(),
            starting_inventory: Bundle::new(
                inventory
                    .iter()
                    .map(|&(g, a)| (GoodId::from_index(g), Quantity::new(a).unwrap()))
                    .collect(),
            ),
        }
    }

    /// Three hexes; businesses: 0 a@h0, 1 b@h2, 2 c@h0.
    fn sample_businesses() -> Businesses {
        let h = hex_ids(3);
        Businesses::from_parts(
            registry(&["a", "b", "c"]),
            vec![
                business(h[0], "rule", &[(1, 10.0)]),
                business(h[2], "other", &[]),
                business(h[0], "rule", &[(0, 2.0), (2, 3.0)]),
            ],
            3,
        )
    }

    fn indexes(ids: &[BusinessId]) -> Vec<usize> {
        ids.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn businesses_get_sequential_ids_in_the_order_given() {
        let businesses = sample_businesses();
        assert_eq!(businesses.len(), 3);
        assert!(!businesses.is_empty());
        let listed: Vec<(usize, String, usize, String, usize)> = businesses
            .iter()
            .map(|(id, name, b)| {
                (
                    id.index(),
                    name.to_string(),
                    b.hex.index(),
                    b.decider.clone(),
                    b.starting_inventory.len(),
                )
            })
            .collect();
        assert_eq!(
            listed,
            vec![
                (0, "a".to_string(), 0, "rule".to_string(), 1),
                (1, "b".to_string(), 2, "other".to_string(), 0),
                (2, "c".to_string(), 0, "rule".to_string(), 2),
            ]
        );
    }

    #[test]
    fn id_name_and_get_round_trip() {
        let businesses = sample_businesses();
        let b = businesses.id("b").unwrap();
        assert_eq!(b.index(), 1);
        assert_eq!(businesses.name(b), "b");
        assert_eq!(businesses.get(b).decider, "other");
        assert!(matches!(
            businesses.id("zzz"),
            Err(IdError::UnknownName {
                kind: "business",
                ..
            })
        ));
    }

    #[test]
    fn on_hex_lists_businesses_in_id_order() {
        let businesses = sample_businesses();
        let h = hex_ids(3);
        assert_eq!(indexes(businesses.on_hex(h[0])), vec![0, 2]);
        assert!(businesses.on_hex(h[1]).is_empty());
        assert_eq!(indexes(businesses.on_hex(h[2])), vec![1]);
    }

    #[test]
    fn the_default_list_is_empty() {
        let businesses = Businesses::default();
        assert_eq!(businesses.len(), 0);
        assert!(businesses.is_empty());
        assert_eq!(businesses.iter().count(), 0);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn get_panics_on_a_foreign_id() {
        let foreign = registry(&["a", "b"]).id("b").unwrap();
        Businesses::default().get(foreign);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_a_hex_beyond_the_map() {
        sample_businesses().on_hex(hex_ids(4)[3]);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_the_default_list() {
        Businesses::default().on_hex(hex_ids(1)[0]);
    }

    #[test]
    #[should_panic(expected = "is not from a map of 2 hexes")]
    fn from_parts_panics_on_a_hex_beyond_hex_count() {
        let h = hex_ids(3);
        Businesses::from_parts(registry(&["a"]), vec![business(h[2], "rule", &[])], 2);
    }

    #[test]
    #[should_panic(expected = "every business name needs exactly one entry")]
    fn from_parts_panics_on_mismatched_lengths() {
        let h = hex_ids(1);
        Businesses::from_parts(registry(&["a", "b"]), vec![business(h[0], "rule", &[])], 1);
    }
}
