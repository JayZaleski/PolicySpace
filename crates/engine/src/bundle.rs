//! A bundle is a list of goods with amounts: a technology's inputs per unit of output, its
//! capital bundle and its construction bundle, and later a factory's current capital. Kept
//! sorted by good ID with no good twice, so two bundles with the same goods and amounts are
//! equal and lookups are cheap. Amounts are `Quantity` (finite, at least 0); whether 0 is
//! allowed is up to the owner (a technology's bundles are all positive; a factory's capital
//! may wear down to 0 later). Knows nothing about files.

use crate::goods::GoodId;
use crate::quantity::Quantity;

/// Goods with amounts, sorted by good ID, no good twice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    items: Vec<(GoodId, Quantity)>,
}

impl Bundle {
    /// Builds a bundle from (good, amount) pairs in any order, sorting them by good ID.
    /// Crate-private: only the loader (and later the engine's own arithmetic) builds bundles.
    ///
    /// # Panics
    /// Panics if a good appears twice (`"a good appears twice in a bundle"`).
    pub(crate) fn new(mut items: Vec<(GoodId, Quantity)>) -> Self {
        items.sort_by_key(|x| x.0);
        assert!(
            items.windows(2).all(|w| w[0].0 != w[1].0),
            "a good appears twice in a bundle"
        );
        Bundle { items }
    }

    /// The (good, amount) pairs, in good ID order.
    pub fn as_slice(&self) -> &[(GoodId, Quantity)] {
        &self.items
    }

    /// Every (good, amount), in good ID order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_ {
        self.items.iter().copied()
    }

    /// The amount of `good`; zero if the bundle doesn't name it.
    pub fn amount_of(&self, good: GoodId) -> Quantity {
        match self.items.binary_search_by_key(&good, |x| x.0) {
            Ok(i) => self.items[i].1,
            Err(_) => Quantity::ZERO,
        }
    }

    /// True if the bundle names `good` (even with amount 0).
    pub fn contains(&self, good: GoodId) -> bool {
        self.items.binary_search_by_key(&good, |x| x.0).is_ok()
    }

    /// Number of goods named.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if no good is named.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good(i: u32) -> GoodId {
        GoodId::from_index(i)
    }

    fn q(v: f64) -> Quantity {
        Quantity::new(v).unwrap()
    }

    #[test]
    fn pairs_given_out_of_order_come_back_sorted_by_good() {
        let bundle = Bundle::new(vec![
            (good(3), q(1.0)),
            (good(0), q(2.5)),
            (good(2), q(4.0)),
        ]);
        assert_eq!(
            bundle.as_slice(),
            &[(good(0), q(2.5)), (good(2), q(4.0)), (good(3), q(1.0))]
        );
        assert_eq!(
            bundle.iter().collect::<Vec<_>>(),
            vec![(good(0), q(2.5)), (good(2), q(4.0)), (good(3), q(1.0))]
        );
    }

    #[test]
    fn amount_of_gives_the_amount_or_zero() {
        let bundle = Bundle::new(vec![(good(3), q(1.0)), (good(0), q(2.5))]);
        assert_eq!(bundle.amount_of(good(0)), q(2.5));
        assert_eq!(bundle.amount_of(good(3)), q(1.0));
        assert_eq!(bundle.amount_of(good(1)), Quantity::ZERO);
        assert_eq!(bundle.amount_of(good(9)), Quantity::ZERO);
    }

    #[test]
    fn contains_names_goods_even_with_amount_zero() {
        let bundle = Bundle::new(vec![(good(1), q(2.0)), (good(2), Quantity::ZERO)]);
        assert!(bundle.contains(good(1)));
        assert!(bundle.contains(good(2)));
        assert!(!bundle.contains(good(0)));
        assert!(!bundle.contains(good(3)));
    }

    #[test]
    fn len_and_is_empty() {
        let bundle = Bundle::new(vec![(good(1), q(2.0)), (good(0), q(1.0))]);
        assert_eq!(bundle.len(), 2);
        assert!(!bundle.is_empty());
        let empty = Bundle::new(Vec::new());
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn default_is_empty() {
        let bundle = Bundle::default();
        assert!(bundle.is_empty());
        assert_eq!(bundle, Bundle::new(Vec::new()));
    }

    #[test]
    #[should_panic(expected = "a good appears twice in a bundle")]
    fn a_duplicate_good_panics() {
        Bundle::new(vec![
            (good(1), q(1.0)),
            (good(0), q(1.0)),
            (good(1), q(2.0)),
        ]);
    }
}
