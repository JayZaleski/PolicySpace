//! Stockpiles: amounts of goods, grouped by when they perish.
//!
//! - A stockpile holds amounts of goods. It is a general type: each hex has a stockpile of
//!   unowned goods (see the [`world`](crate::world) module), and later Stages give POPs and
//!   industries their own.
//! - Each good's amount is split into groups by when it perishes, so `perishes_after`
//!   works without tracking individual units. A good with `perishes_after = N` added in
//!   turn T perishes at the end of turn T+N−1. A durable good's amount is one group that
//!   never perishes. Amounts of a good that perish at the same time are merged.
//! - A stockpile lists only goods it holds a positive amount of.
//! - Only the engine changes stockpiles. Goods enter by emission and leave by perishing;
//!   taking goods out to use them comes with Stage 02.

use std::collections::BTreeMap;

use crate::goods::{GoodId, Perishes};
use crate::quantity::Quantity;
use crate::turn::Turn;

/// When a group of a good's units perishes.
/// Ordered soonest first: every `EndOfTurn` (by turn) before `Never`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PerishesAt {
    /// Removed when this turn ends.
    EndOfTurn(Turn),
    /// Never perishes (a durable good).
    Never,
}

impl PerishesAt {
    /// When units of a good with `perishes` that are added in turn `added_in` perish:
    /// `Never` for `Perishes::Never`; `EndOfTurn(added_in + n - 1)` for `Perishes::After(n)`,
    /// saturating at `Turn::MAX` (a world at `Turn::MAX` can't step, so it never comes).
    pub fn for_good(perishes: Perishes, added_in: Turn) -> PerishesAt {
        match perishes {
            Perishes::Never => PerishesAt::Never,
            Perishes::After(n) => {
                PerishesAt::EndOfTurn(added_in.saturating_add(u64::from(n.get()) - 1))
            }
        }
    }
}

/// Amounts of goods, each grouped by when it perishes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stockpile {
    /// Only goods with at least one group; only groups with an amount > 0.
    goods: BTreeMap<GoodId, BTreeMap<PerishesAt, Quantity>>,
}

impl Stockpile {
    /// An empty stockpile.
    pub fn new() -> Stockpile {
        Stockpile::default()
    }

    /// True if it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.goods.is_empty()
    }

    /// Every good held, with its total, in good ID order.
    pub fn iter(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_ {
        self.goods.iter().map(|(good, groups)| {
            let total = groups
                .values()
                .fold(Quantity::ZERO, |sum, amount| sum.saturating_add(*amount));
            (*good, total)
        })
    }

    /// The total of `good`; zero if it holds none.
    pub fn total(&self, good: GoodId) -> Quantity {
        self.groups(good).fold(Quantity::ZERO, |sum, (_, amount)| {
            sum.saturating_add(amount)
        })
    }

    /// `good`'s groups as (when it perishes, amount), soonest first; empty if it holds none.
    pub fn groups(&self, good: GoodId) -> impl Iterator<Item = (PerishesAt, Quantity)> + '_ {
        self.goods
            .get(&good)
            .into_iter()
            .flat_map(|groups| groups.iter().map(|(at, amount)| (*at, *amount)))
    }

    /// Adds `amount` of `good` to the group that perishes at `at` (merging with it).
    /// A zero amount changes nothing. Crate-private.
    pub(crate) fn add(&mut self, good: GoodId, amount: Quantity, at: PerishesAt) {
        if amount == Quantity::ZERO {
            return;
        }
        let group = self
            .goods
            .entry(good)
            .or_default()
            .entry(at)
            .or_insert(Quantity::ZERO);
        *group = group.saturating_add(amount);
    }

    /// Removes every group that perishes at the end of `turn` or earlier, drops goods left
    /// with no groups, and returns the total removed per good, in good ID order, listing
    /// only goods with an amount > 0. Crate-private.
    pub(crate) fn remove_perishing(&mut self, turn: Turn) -> Vec<(GoodId, Quantity)> {
        let mut removed = Vec::new();
        for (good, groups) in self.goods.iter_mut() {
            let perishing: Vec<PerishesAt> = groups
                .keys()
                .copied()
                .filter(|at| matches!(at, PerishesAt::EndOfTurn(t) if *t <= turn))
                .collect();
            let mut total = Quantity::ZERO;
            for at in perishing {
                if let Some(amount) = groups.remove(&at) {
                    total = total.saturating_add(amount);
                }
            }
            if total > Quantity::ZERO {
                removed.push((*good, total));
            }
        }
        self.goods.retain(|_, groups| !groups.is_empty());
        removed
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    fn q(value: f64) -> Quantity {
        Quantity::new(value).unwrap()
    }

    fn good(index: u32) -> GoodId {
        GoodId::from_index(index)
    }

    fn after(n: u32) -> Perishes {
        Perishes::After(NonZeroU32::new(n).unwrap())
    }

    fn end(t: u64) -> PerishesAt {
        PerishesAt::EndOfTurn(Turn::new(t))
    }

    #[test]
    fn for_good_computes_the_last_turn() {
        assert_eq!(
            PerishesAt::for_good(Perishes::Never, Turn::new(5)),
            PerishesAt::Never
        );
        assert_eq!(PerishesAt::for_good(after(1), Turn::new(5)), end(5));
        assert_eq!(PerishesAt::for_good(after(3), Turn::new(5)), end(7));
        assert_eq!(
            PerishesAt::for_good(after(3), Turn::new(u64::MAX - 1)),
            PerishesAt::EndOfTurn(Turn::MAX)
        );
        assert_eq!(
            PerishesAt::for_good(after(3), Turn::MAX),
            PerishesAt::EndOfTurn(Turn::MAX)
        );
    }

    #[test]
    fn perishes_at_orders_soonest_first_with_never_last() {
        assert!(end(1) < end(2));
        assert!(end(2) < PerishesAt::Never);
        assert!(PerishesAt::EndOfTurn(Turn::MAX) < PerishesAt::Never);
    }

    #[test]
    fn a_new_stockpile_is_empty() {
        let s = Stockpile::new();
        assert!(s.is_empty());
        assert_eq!(s.iter().count(), 0);
        assert_eq!(s.total(good(0)), Quantity::ZERO);
        assert_eq!(s.groups(good(0)).count(), 0);
    }

    #[test]
    fn add_merges_amounts_with_the_same_perish_time() {
        let mut s = Stockpile::new();
        s.add(good(0), q(2.0), end(3));
        s.add(good(0), q(0.5), end(3));
        assert!(!s.is_empty());
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(3), q(2.5))]);
        assert_eq!(s.total(good(0)), q(2.5));
    }

    #[test]
    fn adding_zero_changes_nothing() {
        let mut s = Stockpile::new();
        s.add(good(0), Quantity::ZERO, end(3));
        assert!(s.is_empty());
        assert_eq!(s, Stockpile::new());
    }

    #[test]
    fn iter_lists_goods_in_id_order_with_totals() {
        let mut s = Stockpile::new();
        s.add(good(2), q(1.0), PerishesAt::Never);
        s.add(good(0), q(4.0), end(1));
        s.add(good(0), q(1.0), end(2));
        let listed: Vec<_> = s.iter().collect();
        assert_eq!(listed, vec![(good(0), q(5.0)), (good(2), q(1.0))]);
    }

    #[test]
    fn groups_are_listed_soonest_first_and_never_last() {
        let mut s = Stockpile::new();
        s.add(good(1), q(3.0), PerishesAt::Never);
        s.add(good(1), q(2.0), end(9));
        s.add(good(1), q(1.0), end(4));
        let groups: Vec<_> = s.groups(good(1)).collect();
        assert_eq!(
            groups,
            vec![
                (end(4), q(1.0)),
                (end(9), q(2.0)),
                (PerishesAt::Never, q(3.0))
            ]
        );
        assert_eq!(s.total(good(1)), q(6.0));
        assert_eq!(s.groups(good(0)).count(), 0);
        assert_eq!(s.total(good(0)), Quantity::ZERO);
    }

    #[test]
    fn remove_perishing_removes_groups_at_or_before_the_turn() {
        let mut s = Stockpile::new();
        s.add(good(3), q(7.0), end(2));
        s.add(good(1), q(1.0), end(1));
        s.add(good(1), q(2.0), end(2));
        s.add(good(1), q(4.0), end(3));
        s.add(good(1), q(8.0), PerishesAt::Never);
        s.add(good(2), q(16.0), end(5));
        let removed = s.remove_perishing(Turn::new(2));
        assert_eq!(removed, vec![(good(1), q(3.0)), (good(3), q(7.0))]);
        // good 3 is gone, good 2 untouched, good 1 keeps its later and durable groups.
        let listed: Vec<_> = s.iter().collect();
        assert_eq!(listed, vec![(good(1), q(12.0)), (good(2), q(16.0))]);
        let groups: Vec<_> = s.groups(good(1)).collect();
        assert_eq!(groups, vec![(end(3), q(4.0)), (PerishesAt::Never, q(8.0))]);
        assert_eq!(s.groups(good(3)).count(), 0);
    }

    #[test]
    fn remove_perishing_lists_no_zeros_and_can_empty_the_stockpile() {
        let mut s = Stockpile::new();
        s.add(good(0), q(1.0), PerishesAt::Never);
        s.add(good(1), q(2.0), end(0));
        assert_eq!(s.remove_perishing(Turn::new(0)), vec![(good(1), q(2.0))]);
        assert_eq!(s.remove_perishing(Turn::new(100)), Vec::new());
        let mut only_perishing = Stockpile::new();
        only_perishing.add(good(1), q(2.0), end(0));
        only_perishing.remove_perishing(Turn::new(0));
        assert!(only_perishing.is_empty());
    }
}
