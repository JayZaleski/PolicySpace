//! Currencies: each hex designates one good as its currency, the good that prices on that
//! hex are posted in. The designation is starting state (it can change during play, so it
//! is not a rule), one per hex, in hex ID order. The engine has no opinion about which good
//! may be a currency. Nothing changes a currency yet. This module knows nothing about
//! files: the loader builds [`HexCurrencies`] from a checked setting.

use crate::goods::GoodId;
use crate::id::IdVec;
use crate::map::{HexId, HexKind};

/// The currency good of every hex, by [`HexId`].
#[derive(Debug, Default)]
pub struct HexCurrencies {
    by_hex: IdVec<HexKind, GoodId>,
}

impl HexCurrencies {
    /// One entry per hex, in hex ID order. Crate-private: only the loader builds this.
    ///
    /// # Panics
    /// Panics if there are more than `u32::MAX` entries (via `IdVec::from_vec`).
    pub(crate) fn from_vec(currencies: Vec<GoodId>) -> Self {
        HexCurrencies {
            by_hex: IdVec::from_vec(currencies),
        }
    }

    /// Number of hexes.
    pub fn len(&self) -> usize {
        self.by_hex.len()
    }

    /// True if there are no hexes.
    pub fn is_empty(&self) -> bool {
        self.by_hex.is_empty()
    }

    /// The currency of `hex`.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these were built with.
    pub fn get(&self, hex: HexId) -> GoodId {
        *self.by_hex.get(hex)
    }

    /// Every hex's currency as (hex, good), in hex ID order.
    pub fn iter(&self) -> impl Iterator<Item = (HexId, GoodId)> + '_ {
        self.by_hex.iter().map(|(hex, good)| (hex, *good))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;

    fn good(index: usize) -> GoodId {
        GoodId::from_usize(index)
    }

    fn hex(index: usize) -> HexId {
        Id::from_usize(index)
    }

    #[test]
    fn from_vec_gives_ids_in_order() {
        let currencies = HexCurrencies::from_vec(vec![good(2), good(0), good(2)]);
        assert_eq!(currencies.len(), 3);
        assert!(!currencies.is_empty());
        assert_eq!(currencies.get(hex(0)), good(2));
        assert_eq!(currencies.get(hex(1)), good(0));
        assert_eq!(currencies.get(hex(2)), good(2));
    }

    #[test]
    fn iter_lists_every_hex_in_order() {
        let currencies = HexCurrencies::from_vec(vec![good(1), good(0)]);
        let listed: Vec<(HexId, GoodId)> = currencies.iter().collect();
        assert_eq!(listed, vec![(hex(0), good(1)), (hex(1), good(0))]);
    }

    #[test]
    fn default_is_empty() {
        let currencies = HexCurrencies::default();
        assert_eq!(currencies.len(), 0);
        assert!(currencies.is_empty());
        assert_eq!(currencies.iter().count(), 0);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn get_with_a_foreign_hex_panics() {
        let currencies = HexCurrencies::from_vec(vec![good(0)]);
        currencies.get(hex(1));
    }
}
