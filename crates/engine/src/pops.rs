//! POP types and the POPs placed on the map.
//!
//! - A **POP type** (`[[pop_type]]` in `rules/`) describes one individual: the goods it
//!   receives at the start of every turn (its endowment) and the Stone-Geary utility it
//!   consumes by, as a subsistence amount and a share for each good it wants. Leisure needs
//!   no special case: it is an endowed good that also appears in the utility, consumed
//!   rather than sold. The intended design endows only unskilled labor; any good may be
//!   endowed so the POP system can be tested before production and the live market exist.
//!   Shares are stored normalized (they sum to 1 over the type's utility goods); the
//!   weights as written in the setting aren't kept. A good with share 0 but a positive
//!   subsistence is a pure necessity. The [`demand`](crate::demand) module computes demand from the utility.
//! - A **POP** (`[[pop]]` in `start/`) is N identical individuals of one POP type on one
//!   hex. N (its size) is continuous: a POP is an aggregate, so its endowment scales with N
//!   and its size will later grow by a rate. A hex holds at most one POP of each type, and
//!   may hold POPs of several types or none. POPs have no names; they are identified in
//!   setting files by hex and type, and [`PopId`]s are valid only within the [`Pops`] that
//!   produced them. Each POP's stockpile is world state (see the [`world`](crate::world)
//!   module), not part of the [`Pop`] record; every turn the world adds the POP's type's
//!   endowment times its size to that stockpile.
//!
//! Knows nothing about files: the setting loader builds [`PopTypes`] and [`Pops`] from a
//! checked setting, and everything else only reads them.

use crate::goods::GoodId;
use crate::id::{Id, IdKind, IdVec, NamedList};
use crate::map::{HexId, HexKind};
use crate::quantity::Quantity;

/// Marker kind for POP types.
pub enum PopTypeKind {}

impl IdKind for PopTypeKind {
    const LABEL: &'static str = "pop type";
}

/// The ID of a POP type.
pub type PopTypeId = Id<PopTypeKind>;

/// One endowed good: what one individual receives of it at the start of every turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Endowment {
    /// The endowed good.
    pub good: GoodId,
    /// Always finite and greater than 0.
    pub amount: Quantity,
}

/// One good in a POP type's utility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtilityGood {
    /// The good the type wants.
    pub good: GoodId,
    /// The amount one individual needs before anything else. May be zero.
    pub subsistence: Quantity,
    /// Its share of spending beyond subsistence, normalized over the type's utility goods
    /// (the shares sum to 1). Zero for a pure necessity.
    pub share: f64,
}

/// One POP type: the per-individual endowment and utility parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PopType {
    /// Sorted by good, no good twice.
    endowment: Vec<Endowment>,
    /// Sorted by good, no good twice.
    utility: Vec<UtilityGood>,
}

impl PopType {
    /// Builds a type from (good, amount) endowment pairs and (good, subsistence, weight)
    /// utility triples in any order. Sorts both by good ID and normalizes the weights to
    /// shares that sum to 1. Crate-private: only the loader builds POP types.
    ///
    /// # Panics
    /// Panics if a good appears twice in either list, if a weight is negative or not
    /// finite, or if no weight is positive (the loader guarantees all of this).
    pub(crate) fn new(
        mut endowment: Vec<(GoodId, Quantity)>,
        mut utility: Vec<(GoodId, Quantity, f64)>,
    ) -> Self {
        endowment.sort_by_key(|x| x.0);
        utility.sort_by_key(|x| x.0);
        assert!(
            endowment.windows(2).all(|w| w[0].0 != w[1].0),
            "a good appears twice in a POP type's endowment"
        );
        assert!(
            utility.windows(2).all(|w| w[0].0 != w[1].0),
            "a good appears twice in a POP type's utility"
        );
        assert!(
            utility.iter().all(|x| x.2.is_finite() && x.2 >= 0.0),
            "a POP type's utility weights must be finite and at least 0"
        );
        let largest = utility.iter().map(|x| x.2).fold(0.0_f64, f64::max);
        assert!(
            largest > 0.0,
            "a POP type needs a utility good with a positive share"
        );
        // Scale by the largest weight first so the sum can't overflow to infinity.
        let total: f64 = utility.iter().map(|x| x.2 / largest).sum();
        PopType {
            endowment: endowment
                .into_iter()
                .map(|(good, amount)| Endowment { good, amount })
                .collect(),
            utility: utility
                .into_iter()
                .map(|(good, subsistence, weight)| UtilityGood {
                    good,
                    subsistence,
                    share: (weight / largest) / total,
                })
                .collect(),
        }
    }

    /// The endowment, in good ID order. May be empty.
    pub fn endowment(&self) -> &[Endowment] {
        &self.endowment
    }

    /// The utility goods, in good ID order. Never empty; at least one share is positive.
    pub fn utility(&self) -> &[UtilityGood] {
        &self.utility
    }

    /// How much of `good` one individual receives per turn; zero if it isn't endowed.
    pub fn endowment_of(&self, good: GoodId) -> Quantity {
        match self.endowment.binary_search_by_key(&good, |e| e.good) {
            Ok(i) => self.endowment[i].amount,
            Err(_) => Quantity::ZERO,
        }
    }

    /// The utility parameters of `good`, or None if the type doesn't want it.
    pub fn utility_of(&self, good: GoodId) -> Option<&UtilityGood> {
        self.utility
            .binary_search_by_key(&good, |u| u.good)
            .ok()
            .map(|i| &self.utility[i])
    }
}

/// Every POP type in a setting, in load order (which is also ID order).
pub type PopTypes = NamedList<PopTypeKind, PopType>;

/// Marker kind for POPs.
pub enum PopKind {}

impl IdKind for PopKind {
    const LABEL: &'static str = "pop";
}

/// The ID of a POP. Valid only within the [`Pops`] that produced it.
pub type PopId = Id<PopKind>;

/// One POP: N identical individuals of one type on one hex. Its stockpile is world state
/// (see the `world` module), not part of this record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pop {
    /// The hex it lives on.
    pub hex: HexId,
    /// Its POP type.
    pub pop_type: PopTypeId,
    /// N, the number of individuals. Continuous; always finite and greater than 0.
    pub size: Quantity,
}

/// Every POP in a setting, in load order (which is also ID order). At most one POP of each
/// type on each hex.
///
/// `Pops::default()` is empty and was built with no map, so [`on_hex`](Self::on_hex) and
/// [`of_type_on_hex`](Self::of_type_on_hex) panic on it for any hex.
#[derive(Debug, Default)]
pub struct Pops {
    /// One entry per POP, by `PopId`.
    pops: IdVec<PopKind, Pop>,
    /// One entry per hex, by `HexId`: the POPs on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<PopId>>,
}

impl Pops {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map the POPs live on. Crate-private: only the setting loader
    /// builds POPs.
    ///
    /// # Panics
    /// Panics if a POP's hex index is `>= hex_count` (`"{hex:?} is not from a map of
    /// {hex_count} hexes"`), if two POPs have the same hex and type (`"two POPs of
    /// {pop_type:?} on {hex:?}"`; the loader rejects duplicates first), or if there are
    /// more than `u32::MAX` POPs.
    pub(crate) fn from_pops(pops: Vec<Pop>, hex_count: usize) -> Self {
        let pops = IdVec::from_vec(pops);
        let mut by_hex: IdVec<HexKind, Vec<PopId>> = IdVec::from_vec(vec![Vec::new(); hex_count]);
        for (id, pop) in pops.iter() {
            assert!(
                pop.hex.index() < hex_count,
                "{:?} is not from a map of {hex_count} hexes",
                pop.hex
            );
            by_hex.get_mut(pop.hex).push(id);
        }
        for (hex, ids) in by_hex.iter() {
            for (i, a) in ids.iter().enumerate() {
                for b in &ids[i + 1..] {
                    let pop_type = pops.get(*a).pop_type;
                    assert!(
                        pop_type != pops.get(*b).pop_type,
                        "two POPs of {pop_type:?} on {hex:?}"
                    );
                }
            }
        }
        Pops { pops, by_hex }
    }

    /// Number of POPs.
    pub fn len(&self) -> usize {
        self.pops.len()
    }

    /// True if the setting has no POPs.
    pub fn is_empty(&self) -> bool {
        self.pops.is_empty()
    }

    /// The POP `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn get(&self, id: PopId) -> &Pop {
        self.pops.get(id)
    }

    /// Every POP as (ID, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (PopId, &Pop)> + '_ {
        self.pops.iter()
    }

    /// The POPs on `hex`, in ID order; empty if it has none.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these POPs were built with.
    pub fn on_hex(&self, hex: HexId) -> &[PopId] {
        self.by_hex.get(hex)
    }

    /// The POP of `pop_type` on `hex`, if any.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these POPs were built with.
    pub fn of_type_on_hex(&self, hex: HexId, pop_type: PopTypeId) -> Option<PopId> {
        self.on_hex(hex)
            .iter()
            .copied()
            .find(|id| self.pops.get(*id).pop_type == pop_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::NameRegistry;

    fn good(index: u32) -> GoodId {
        GoodId::from_index(index)
    }

    fn q(value: f64) -> Quantity {
        Quantity::new(value).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn shares_are_normalized_and_sorted_by_good() {
        let t = PopType::new(
            vec![(good(4), q(1.0)), (good(1), q(2.0))],
            vec![(good(3), q(0.0), 3.0), (good(2), q(5.0), 1.0)],
        );
        let utility = t.utility();
        assert_eq!(utility.len(), 2);
        assert_eq!(utility[0].good, good(2));
        assert_eq!(utility[0].subsistence, q(5.0));
        assert!(close(utility[0].share, 0.25));
        assert_eq!(utility[1].good, good(3));
        assert!(close(utility[1].share, 0.75));
        assert!(close(utility.iter().map(|u| u.share).sum::<f64>(), 1.0));
        let endowment = t.endowment();
        assert_eq!(endowment[0].good, good(1));
        assert_eq!(endowment[0].amount, q(2.0));
        assert_eq!(endowment[1].good, good(4));
    }

    #[test]
    fn huge_weights_do_not_overflow() {
        let t = PopType::new(
            vec![],
            vec![(good(0), q(0.0), 1e308), (good(1), q(0.0), 1e308)],
        );
        assert!(close(t.utility()[0].share, 0.5));
        assert!(close(t.utility()[1].share, 0.5));
    }

    #[test]
    fn a_single_positive_weight_gives_share_one() {
        let t = PopType::new(vec![], vec![(good(0), q(0.0), 7.5)]);
        assert!(close(t.utility()[0].share, 1.0));
    }

    #[test]
    fn a_zero_weight_good_stays_with_share_zero() {
        let t = PopType::new(vec![], vec![(good(0), q(1.0), 0.0), (good(1), q(0.0), 2.0)]);
        assert_eq!(t.utility().len(), 2);
        assert_eq!(t.utility()[0].share, 0.0);
        assert!(close(t.utility()[1].share, 1.0));
    }

    #[test]
    fn an_empty_endowment_is_fine() {
        let t = PopType::new(vec![], vec![(good(0), q(0.0), 1.0)]);
        assert!(t.endowment().is_empty());
        assert_eq!(t.endowment_of(good(0)), Quantity::ZERO);
    }

    #[test]
    fn endowment_of_gives_the_amount_or_zero() {
        let t = PopType::new(
            vec![(good(5), q(8.0)), (good(2), q(3.0))],
            vec![(good(0), q(0.0), 1.0)],
        );
        assert_eq!(t.endowment_of(good(5)), q(8.0));
        assert_eq!(t.endowment_of(good(2)), q(3.0));
        assert_eq!(t.endowment_of(good(3)), Quantity::ZERO);
    }

    #[test]
    fn utility_of_finds_a_good_or_gives_none() {
        let t = PopType::new(vec![], vec![(good(5), q(1.0), 1.0), (good(2), q(0.0), 1.0)]);
        assert_eq!(t.utility_of(good(5)).unwrap().subsistence, q(1.0));
        assert!(close(t.utility_of(good(2)).unwrap().share, 0.5));
        assert!(t.utility_of(good(3)).is_none());
    }

    #[test]
    #[should_panic(expected = "a good appears twice in a POP type's endowment")]
    fn a_duplicate_endowment_good_panics() {
        PopType::new(
            vec![(good(1), q(1.0)), (good(1), q(2.0))],
            vec![(good(0), q(0.0), 1.0)],
        );
    }

    #[test]
    #[should_panic(expected = "a good appears twice in a POP type's utility")]
    fn a_duplicate_utility_good_panics() {
        PopType::new(vec![], vec![(good(1), q(0.0), 1.0), (good(1), q(0.0), 2.0)]);
    }

    #[test]
    #[should_panic(expected = "a POP type needs a utility good with a positive share")]
    fn no_positive_weight_panics() {
        PopType::new(vec![], vec![(good(1), q(2.0), 0.0)]);
    }

    // --- Pops ---

    fn type_ids(count: usize) -> Vec<PopTypeId> {
        let mut names: NameRegistry<PopTypeKind> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("t{i}")).unwrap())
            .collect()
    }

    fn hex_ids(count: usize) -> Vec<HexId> {
        let mut names: NameRegistry<HexKind> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("h{i}")).unwrap())
            .collect()
    }

    fn pop(hex: HexId, pop_type: PopTypeId, size: f64) -> Pop {
        Pop {
            hex,
            pop_type,
            size: q(size),
        }
    }

    /// Three hexes, two types; POPs: 0 t0@h0, 1 t1@h0, 2 t0@h2, 3 t1@h2.
    fn sample_pops() -> Pops {
        let t = type_ids(2);
        let h = hex_ids(3);
        Pops::from_pops(
            vec![
                pop(h[0], t[0], 1.0),
                pop(h[0], t[1], 2.0),
                pop(h[2], t[0], 3.5),
                pop(h[2], t[1], 4.0),
            ],
            3,
        )
    }

    fn indexes(ids: &[PopId]) -> Vec<usize> {
        ids.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn pops_get_sequential_ids_in_the_order_given() {
        let pops = sample_pops();
        assert_eq!(pops.len(), 4);
        assert!(!pops.is_empty());
        let listed: Vec<(usize, usize, usize, f64)> = pops
            .iter()
            .map(|(id, p)| (id.index(), p.hex.index(), p.pop_type.index(), p.size.get()))
            .collect();
        assert_eq!(
            listed,
            vec![
                (0, 0, 0, 1.0),
                (1, 0, 1, 2.0),
                (2, 2, 0, 3.5),
                (3, 2, 1, 4.0)
            ]
        );
        let (id, _) = pops.iter().nth(2).unwrap();
        assert_eq!(pops.get(id).hex.index(), 2);
    }

    #[test]
    fn on_hex_lists_pops_in_id_order() {
        let pops = sample_pops();
        let h = hex_ids(3);
        assert_eq!(indexes(pops.on_hex(h[0])), vec![0, 1]);
        assert!(pops.on_hex(h[1]).is_empty());
        assert_eq!(indexes(pops.on_hex(h[2])), vec![2, 3]);
    }

    #[test]
    fn of_type_on_hex_finds_the_one_pop_or_none() {
        let pops = sample_pops();
        let h = hex_ids(3);
        let t = type_ids(2);
        assert_eq!(
            pops.of_type_on_hex(h[0], t[0]).map(|id| id.index()),
            Some(0)
        );
        assert_eq!(
            pops.of_type_on_hex(h[0], t[1]).map(|id| id.index()),
            Some(1)
        );
        assert_eq!(
            pops.of_type_on_hex(h[2], t[1]).map(|id| id.index()),
            Some(3)
        );
        assert!(pops.of_type_on_hex(h[1], t[0]).is_none());
    }

    #[test]
    fn the_default_pop_list_is_empty() {
        let pops = Pops::default();
        assert_eq!(pops.len(), 0);
        assert!(pops.is_empty());
        assert_eq!(pops.iter().count(), 0);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn pop_get_panics_on_a_foreign_id() {
        let mut names: NameRegistry<PopKind> = NameRegistry::new();
        names.register("a").unwrap();
        let foreign = names.register("b").unwrap();
        Pops::default().get(foreign);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn pops_on_hex_panics_on_a_hex_beyond_the_map() {
        sample_pops().on_hex(hex_ids(4)[3]);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn pops_on_hex_panics_on_the_default_list() {
        Pops::default().on_hex(hex_ids(1)[0]);
    }

    #[test]
    #[should_panic(expected = "is not from a map of 2 hexes")]
    fn from_pops_panics_on_a_hex_beyond_hex_count() {
        let t = type_ids(1);
        let h = hex_ids(3);
        Pops::from_pops(vec![pop(h[2], t[0], 1.0)], 2);
    }

    #[test]
    #[should_panic(expected = "two POPs of")]
    fn from_pops_panics_on_two_pops_of_one_type_on_one_hex() {
        let t = type_ids(2);
        let h = hex_ids(1);
        Pops::from_pops(
            vec![
                pop(h[0], t[0], 1.0),
                pop(h[0], t[1], 1.0),
                pop(h[0], t[0], 2.0),
            ],
            1,
        );
    }
}
