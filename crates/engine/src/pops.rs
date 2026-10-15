//! POP types. A POP type describes one individual of a POP (Step 04): the goods it
//! receives at the start of every turn (its endowment) and the Stone-Geary utility it
//! consumes by, as a subsistence amount and a share for each good it wants. Leisure needs
//! no special case: it is an endowed good that also appears in the utility, consumed
//! rather than sold. The intended design endows only unskilled labor; any good may be
//! endowed so the POP system can be tested before production and the live market exist.
//! Shares are stored normalized (they sum to 1 over the type's utility goods); the weights
//! as written in the setting aren't kept. A good with share 0 but a positive subsistence
//! is a pure necessity. Nothing uses POP types yet: Step 04 places POPs of these types,
//! Step 05 computes their demand. Knows nothing about files.

use crate::goods::GoodId;
use crate::id::{Id, IdKind, NamedList};
use crate::quantity::Quantity;

/// Marker kind for POP types.
pub enum PopKind {}

impl IdKind for PopKind {
    const LABEL: &'static str = "pop type";
}

/// The ID of a POP type.
pub type PopId = Id<PopKind>;

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
pub type PopTypes = NamedList<PopKind, PopType>;

#[cfg(test)]
mod tests {
    use super::*;

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
}
