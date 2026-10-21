//! Stockpiles: amounts of goods, grouped by when they perish.
//!
//! - A stockpile holds amounts of goods. It is a general type: each hex has a stockpile of
//!   unowned goods, and each POP and each business has its own (see the
//!   [`world`](crate::world) module).
//! - Each good's amount is split into groups by when it perishes, so `perishes_after`
//!   works without tracking individual units. A good with `perishes_after = N` added in
//!   turn T perishes at the end of turn T+N−1. A durable good's amount is one group that
//!   never perishes. Amounts of a good that perish at the same time are merged.
//! - A stockpile lists only goods it holds a positive amount of.
//! - Only the engine changes stockpiles. Goods enter by emission and leave by perishing.
//!   They also leave by removal, soonest-perishing group first, which the
//!   [`market`](crate::market) uses to sell goods and pay currency, by sale down to a kept
//!   amount, and by consumption (see the [`trade`](crate::trade) module).

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

    /// Removes `amount` of `good`, taking from the group that perishes soonest first.
    /// All-or-nothing: if the stockpile holds less than `amount` of `good`, returns
    /// `Err(held)` (the total it holds) and changes nothing. A zero amount is Ok and changes
    /// nothing. Groups that reach zero are dropped, and so is a good left with no groups.
    /// Crate-private.
    pub(crate) fn remove(&mut self, good: GoodId, amount: Quantity) -> Result<(), Quantity> {
        if amount == Quantity::ZERO {
            return Ok(());
        }
        let held = self.total(good);
        if held < amount {
            return Err(held);
        }
        if let Some(groups) = self.goods.get_mut(&good) {
            let mut remaining = amount;
            let order: Vec<PerishesAt> = groups.keys().copied().collect();
            for at in order {
                if remaining == Quantity::ZERO {
                    break;
                }
                let Some(group) = groups.get(&at).copied() else {
                    continue;
                };
                if remaining >= group {
                    groups.remove(&at);
                    remaining = remaining.checked_sub(group).unwrap_or(Quantity::ZERO);
                } else {
                    match group.checked_sub(remaining) {
                        Some(left) if left > Quantity::ZERO => {
                            groups.insert(at, left);
                        }
                        _ => {
                            groups.remove(&at);
                        }
                    }
                    remaining = Quantity::ZERO;
                }
            }
            // If the groups ran out while `remaining` is a tiny positive value (rounding
            // of the total), that is acceptable.
            if groups.is_empty() {
                self.goods.remove(&good);
            }
        }
        Ok(())
    }

    /// Removes everything of `good` above `keep`, taking from the group that perishes
    /// soonest first, and returns the amount removed (zero, changing nothing, if it holds
    /// `keep` or less). When the kept amount lies in a single group, exactly `keep`
    /// remains. Crate-private.
    pub(crate) fn retain(&mut self, good: GoodId, keep: Quantity) -> Quantity {
        if self.total(good) <= keep {
            return Quantity::ZERO;
        }
        let Some(groups) = self.goods.get_mut(&good) else {
            return Quantity::ZERO;
        };
        let mut removed = Quantity::ZERO;
        let mut kept = Quantity::ZERO;
        let mut boundary_found = false;
        let order: Vec<PerishesAt> = groups.keys().rev().copied().collect();
        for at in order {
            let Some(group) = groups.get(&at).copied() else {
                continue;
            };
            if boundary_found {
                groups.remove(&at);
                removed = removed.saturating_add(group);
            } else if kept.saturating_add(group) <= keep {
                kept = kept.saturating_add(group);
            } else {
                boundary_found = true;
                let new_amount = keep.checked_sub(kept).unwrap_or(Quantity::ZERO);
                if new_amount > Quantity::ZERO {
                    groups.insert(at, new_amount);
                } else {
                    groups.remove(&at);
                }
                removed =
                    removed.saturating_add(group.checked_sub(new_amount).unwrap_or(Quantity::ZERO));
            }
        }
        if groups.is_empty() {
            self.goods.remove(&good);
        }
        removed
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
    use super::*;
    use crate::test_support::{after, end, good, q};

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

    fn mixed() -> Stockpile {
        let mut s = Stockpile::new();
        s.add(good(0), q(4.0), end(1));
        s.add(good(0), q(3.0), end(3));
        s.add(good(0), q(2.0), PerishesAt::Never);
        s.add(good(1), q(1.0), end(1));
        s
    }

    #[test]
    fn remove_part_of_the_soonest_group_keeps_the_rest() {
        let mut s = mixed();
        assert_eq!(s.remove(good(0), q(1.5)), Ok(()));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(
            groups,
            vec![
                (end(1), q(2.5)),
                (end(3), q(3.0)),
                (PerishesAt::Never, q(2.0))
            ]
        );
        assert_eq!(s.total(good(1)), q(1.0));
    }

    #[test]
    fn remove_exactly_a_group_drops_it() {
        let mut s = mixed();
        assert_eq!(s.remove(good(0), q(4.0)), Ok(()));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(3), q(3.0)), (PerishesAt::Never, q(2.0))]);
    }

    #[test]
    fn remove_across_groups_empties_the_first_and_reduces_the_second() {
        let mut s = mixed();
        assert_eq!(s.remove(good(0), q(5.0)), Ok(()));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(3), q(2.0)), (PerishesAt::Never, q(2.0))]);
    }

    #[test]
    fn remove_the_whole_total_drops_the_good() {
        let mut s = mixed();
        assert_eq!(s.remove(good(0), q(9.0)), Ok(()));
        assert_eq!(s.groups(good(0)).count(), 0);
        assert_eq!(s.total(good(0)), Quantity::ZERO);
        let listed: Vec<_> = s.iter().collect();
        assert_eq!(listed, vec![(good(1), q(1.0))]);
    }

    #[test]
    fn remove_more_than_held_fails_and_changes_nothing() {
        let mut s = mixed();
        let before = s.clone();
        assert_eq!(s.remove(good(0), q(9.5)), Err(q(9.0)));
        assert_eq!(s, before);
        assert_eq!(s.remove(good(7), q(1.0)), Err(Quantity::ZERO));
        assert_eq!(s, before);
    }

    #[test]
    fn remove_zero_is_ok_and_changes_nothing() {
        let mut s = mixed();
        let before = s.clone();
        assert_eq!(s.remove(good(0), Quantity::ZERO), Ok(()));
        assert_eq!(s.remove(good(7), Quantity::ZERO), Ok(()));
        assert_eq!(s, before);
    }

    #[test]
    fn retain_removes_everything_above_keep_from_the_soonest_group_first() {
        let mut s = Stockpile::new();
        s.add(good(0), q(4.0), end(1));
        s.add(good(0), q(6.0), end(2));
        assert_eq!(s.retain(good(0), q(7.0)), q(3.0));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(1), q(1.0)), (end(2), q(6.0))]);
    }

    #[test]
    fn retain_leaves_exactly_keep_in_a_single_group() {
        let mut s = Stockpile::new();
        s.add(good(0), q(10.0), end(1));
        let removed = s.retain(good(0), q(2.8));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(1), q(2.8))]);
        assert_eq!(removed, q(10.0).checked_sub(q(2.8)).unwrap());
    }

    #[test]
    fn retain_with_zero_keep_removes_the_good() {
        let mut s = mixed();
        assert_eq!(s.retain(good(0), Quantity::ZERO), q(9.0));
        assert_eq!(s.groups(good(0)).count(), 0);
        let listed: Vec<_> = s.iter().collect();
        assert_eq!(listed, vec![(good(1), q(1.0))]);
    }

    #[test]
    fn retain_at_or_below_keep_changes_nothing() {
        let mut s = mixed();
        let before = s.clone();
        assert_eq!(s.retain(good(0), q(9.0)), Quantity::ZERO);
        assert_eq!(s, before);
        assert_eq!(s.retain(good(0), q(12.0)), Quantity::ZERO);
        assert_eq!(s.retain(good(7), q(1.0)), Quantity::ZERO);
        assert_eq!(s, before);
    }

    #[test]
    fn retain_drops_whole_groups_when_keep_falls_on_a_boundary() {
        let mut s = Stockpile::new();
        s.add(good(0), q(4.0), end(1));
        s.add(good(0), q(6.0), end(2));
        assert_eq!(s.retain(good(0), q(6.0)), q(4.0));
        let groups: Vec<_> = s.groups(good(0)).collect();
        assert_eq!(groups, vec![(end(2), q(6.0))]);
    }
}
