//! Stone-Geary demand: what a POP wants to end up with.
//!
//! [`demand`] is a pure function of a POP type's utility, the POP's size, what the POP
//! holds, and a hex's prices. It returns the quantity of every involved good (in the
//! utility, held, or both) that the POP wants to end up with, along with its income, the
//! cost of its subsistence bundle, and the supernumerary income (income minus that cost).
//!
//! - **Tradable and untradable goods.** A good is tradable if the prices have a price for it
//!   (the currency is priced at 1). The optimization runs over tradable goods only. An
//!   untradable good the POP holds is kept whole and adds nothing to income; an untradable
//!   utility good it doesn't hold gets zero. Shares are renormalized over the tradable
//!   utility goods.
//! - **At or above subsistence** (income at least the subsistence cost): every tradable
//!   utility good gets its subsistence amount (times the POP's size) plus its renormalized
//!   share of the supernumerary income, divided by its price. A tradable held good outside
//!   the utility gets zero: it is all for sale. A good with share 0 gets exactly its
//!   subsistence. If no tradable utility good has a positive share, the supernumerary income
//!   is demanded as currency.
//! - **Below subsistence**: every tradable utility good gets the same fraction (income over
//!   cost) of its subsistence amount, and everything else tradable gets zero.
//!
//! The budget balances up to floating-point rounding: the demanded tradable goods are worth
//! the income. The function does no rounding. Nothing calls it during the turn yet (the POP
//! trade step does). It knows nothing about the world or files; prices come in through the
//! [`PriceList`] trait and holdings as any list of goods and amounts.

use std::collections::BTreeMap;

use crate::goods::GoodId;
use crate::pops::PopType;
use crate::quantity::Quantity;

/// A hex's prices as the demand function needs them: which good is the currency, and the
/// price of any good in that currency. The dummy market implements this; the live market
/// will too.
pub trait PriceList {
    /// The good prices are posted in.
    fn currency(&self) -> GoodId;
    /// The price of `good` in the currency: [`Quantity::ONE`] for the currency itself,
    /// the posted price, or None if the good isn't traded here. Every price is > 0.
    fn price(&self, good: GoodId) -> Option<Quantity>;
}

/// One good in a demand: how much the POP holds, how much it wants to end up with, and
/// how much of it the POP's subsistence bundle needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemandedGood {
    /// The good.
    pub good: GoodId,
    /// How much the POP holds (the sum of the holdings passed in); zero if it holds none.
    pub held: Quantity,
    /// How much the POP wants to end up with.
    pub quantity: Quantity,
    /// The type's subsistence amount times N; zero if the good isn't in the utility.
    pub subsistence: Quantity,
}

impl DemandedGood {
    /// True if the demanded quantity doesn't reach the subsistence amount. Never true for
    /// a good with zero subsistence.
    pub fn below_subsistence(&self) -> bool {
        self.quantity < self.subsistence
    }
}

/// What a POP wants to end up with, and the numbers behind it. See [`demand`].
#[derive(Debug, Clone, PartialEq)]
pub struct Demand {
    /// In good ID order, each good once: every utility good, every held good, and the
    /// currency if supernumerary income was demanded as currency.
    goods: Vec<DemandedGood>,
    /// M: the value of every tradable held good at its price.
    income: Quantity,
    /// C: the value of the tradable part of the subsistence bundle (times N).
    subsistence_cost: Quantity,
    /// M - C. Finite; negative below subsistence.
    supernumerary: f64,
}

impl Demand {
    /// Every involved good, in good ID order.
    pub fn goods(&self) -> &[DemandedGood] {
        &self.goods
    }

    /// The record of `good`, or None if it isn't involved.
    pub fn get(&self, good: GoodId) -> Option<&DemandedGood> {
        self.goods
            .binary_search_by_key(&good, |g| g.good)
            .ok()
            .map(|i| &self.goods[i])
    }

    /// The demanded quantity of `good`; zero if it isn't involved.
    pub fn quantity_of(&self, good: GoodId) -> Quantity {
        self.get(good).map_or(Quantity::ZERO, |g| g.quantity)
    }

    /// Income M: the value of every tradable held good at its price.
    pub fn income(&self) -> Quantity {
        self.income
    }

    /// The subsistence cost C: the value of the tradable part of the subsistence bundle.
    pub fn subsistence_cost(&self) -> Quantity {
        self.subsistence_cost
    }

    /// Supernumerary income M - C; negative below subsistence.
    pub fn supernumerary(&self) -> f64 {
        self.supernumerary
    }

    /// True if any good is below subsistence.
    pub fn below_subsistence(&self) -> bool {
        self.goods.iter().any(DemandedGood::below_subsistence)
    }
}

/// The working record of one good while the demand is computed.
struct Entry {
    held: Quantity,
    subsistence: Quantity,
    share: f64,
    price: Option<Quantity>,
    quantity: Quantity,
}

impl Entry {
    fn new(subsistence: Quantity, share: f64) -> Entry {
        Entry {
            held: Quantity::ZERO,
            subsistence,
            share,
            price: None,
            quantity: Quantity::ZERO,
        }
    }
}

/// The Stone-Geary demand of a POP of `pop_type` with `size` individuals that holds
/// `holdings` (goods and amounts, in any order; a good listed twice is summed) on a hex
/// with `prices`. Reads its inputs and changes nothing.
pub fn demand(
    pop_type: &PopType,
    size: Quantity,
    holdings: impl IntoIterator<Item = (GoodId, Quantity)>,
    prices: &impl PriceList,
) -> Demand {
    // 1. Collect the involved goods.
    let mut entries: BTreeMap<GoodId, Entry> = BTreeMap::new();
    for u in pop_type.utility() {
        entries.insert(
            u.good,
            Entry::new(u.subsistence.saturating_mul(size), u.share),
        );
    }
    for (good, amount) in holdings {
        let entry = entries
            .entry(good)
            .or_insert_with(|| Entry::new(Quantity::ZERO, 0.0));
        entry.held = entry.held.saturating_add(amount);
    }
    for (good, entry) in entries.iter_mut() {
        entry.price = prices.price(*good);
    }

    // 2. Totals over the tradable entries.
    let mut income = Quantity::ZERO;
    let mut subsistence_cost = Quantity::ZERO;
    let mut total_share = 0.0_f64;
    for entry in entries.values() {
        if let Some(price) = entry.price {
            income = income.saturating_add(entry.held.saturating_mul(price));
            subsistence_cost =
                subsistence_cost.saturating_add(entry.subsistence.saturating_mul(price));
            total_share += entry.share;
        }
    }
    let supernumerary = income.get() - subsistence_cost.get();

    // 3. Quantities.
    let extra = income.checked_sub(subsistence_cost);
    // Only used below subsistence, where C > M >= 0, so C > 0 and the division is defined.
    let fraction = income
        .checked_div(subsistence_cost)
        .unwrap_or(Quantity::ZERO);
    for entry in entries.values_mut() {
        let Some(price) = entry.price else {
            // Untradable: kept whole, whether or not it is in the utility.
            entry.quantity = entry.held;
            continue;
        };
        entry.quantity = match extra {
            Some(extra) => {
                let mut quantity = entry.subsistence;
                if entry.share > 0.0 && total_share > 0.0 {
                    // The renormalized share is in (0, 1], so `new` can't fail; the price is
                    // > 0 by the trait's contract, so the division is defined.
                    let part = Quantity::new(entry.share / total_share).unwrap_or(Quantity::ZERO);
                    let spend = extra
                        .saturating_mul(part)
                        .checked_div(price)
                        .unwrap_or(Quantity::ZERO);
                    quantity = quantity.saturating_add(spend);
                }
                quantity
            }
            None => entry.subsistence.saturating_mul(fraction),
        };
    }

    // 4. Currency fallback: with nowhere to put the surplus, demand it as currency.
    if let Some(extra) = extra
        && total_share == 0.0
        && extra > Quantity::ZERO
    {
        let entry = entries
            .entry(prices.currency())
            .or_insert_with(|| Entry::new(Quantity::ZERO, 0.0));
        entry.quantity = entry.quantity.saturating_add(extra);
    }

    // 5. Build the result in good ID order.
    Demand {
        goods: entries
            .into_iter()
            .map(|(good, e)| DemandedGood {
                good,
                held: e.held,
                quantity: e.quantity,
                subsistence: e.subsistence,
            })
            .collect(),
        income,
        subsistence_cost,
        supernumerary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABOR: u32 = 0;
    const GRAIN: u32 = 1;
    const COPPER: u32 = 2;
    const SILVER: u32 = 3;

    fn good(index: u32) -> GoodId {
        GoodId::from_index(index)
    }

    fn q(value: f64) -> Quantity {
        Quantity::new(value).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// Hand-written prices: a currency good and a table of the other prices.
    struct Prices {
        currency: GoodId,
        table: BTreeMap<GoodId, Quantity>,
    }

    impl PriceList for Prices {
        fn currency(&self) -> GoodId {
            self.currency
        }

        fn price(&self, good: GoodId) -> Option<Quantity> {
            if good == self.currency {
                Some(Quantity::ONE)
            } else {
                self.table.get(&good).copied()
            }
        }
    }

    fn prices(list: &[(u32, f64)]) -> Prices {
        Prices {
            currency: good(SILVER),
            table: list.iter().map(|&(g, p)| (good(g), q(p))).collect(),
        }
    }

    /// Labor 1, grain 2, copper 5.
    fn typical() -> Prices {
        prices(&[(LABOR, 1.0), (GRAIN, 2.0), (COPPER, 5.0)])
    }

    /// Labor subsistence 2 weight 0.2, grain subsistence 2 weight 0.8.
    fn peasant() -> PopType {
        PopType::new(
            vec![(good(LABOR), q(10.0))],
            vec![(good(LABOR), q(2.0), 0.2), (good(GRAIN), q(2.0), 0.8)],
        )
    }

    fn type_of(utility: &[(u32, f64, f64)]) -> PopType {
        PopType::new(
            vec![],
            utility
                .iter()
                .map(|&(g, s, w)| (good(g), q(s), w))
                .collect(),
        )
    }

    fn held(list: &[(u32, f64)]) -> Vec<(GoodId, Quantity)> {
        list.iter().map(|&(g, a)| (good(g), q(a))).collect()
    }

    /// Asserts the demanded goods are exactly `expected` (good index, quantity), in order.
    fn assert_quantities(d: &Demand, expected: &[(u32, f64)]) {
        let actual: Vec<(usize, f64)> = d
            .goods()
            .iter()
            .map(|g| (g.good.index(), g.quantity.get()))
            .collect();
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for ((ag, aq), (eg, eq)) in actual.iter().zip(expected) {
            assert_eq!(*ag, *eg as usize, "{actual:?} vs {expected:?}");
            assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
        }
    }

    fn assert_balances(d: &Demand, p: &Prices) {
        let spent: f64 = d
            .goods()
            .iter()
            .filter_map(|g| p.price(g.good).map(|price| g.quantity.get() * price.get()))
            .sum();
        let m = d.income().get();
        assert!(
            (spent - m).abs() < 1e-9 * m.max(1.0),
            "spends {spent} of income {m}"
        );
    }

    #[test]
    fn below_subsistence_scales_the_bundle() {
        let p = prices(&[(LABOR, 1.0), (GRAIN, 10.0)]);
        let d = demand(&peasant(), q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert!(close(d.income().get(), 10.0));
        assert!(close(d.subsistence_cost().get(), 22.0));
        assert!(close(d.supernumerary(), -12.0));
        assert_quantities(&d, &[(LABOR, 10.0 / 11.0), (GRAIN, 10.0 / 11.0)]);
        assert!(d.goods().iter().all(DemandedGood::below_subsistence));
        assert!(d.below_subsistence());
        assert_balances(&d, &p);
    }

    #[test]
    fn at_exactly_subsistence_gives_the_bundle() {
        let p = prices(&[(LABOR, 1.0), (GRAIN, 4.0)]);
        let d = demand(&peasant(), q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert!(close(d.income().get(), 10.0));
        assert!(close(d.subsistence_cost().get(), 10.0));
        assert!(close(d.supernumerary(), 0.0));
        assert_quantities(&d, &[(LABOR, 2.0), (GRAIN, 2.0)]);
        assert!(!d.below_subsistence());
        assert_balances(&d, &p);
    }

    #[test]
    fn an_untradable_utility_good_renormalizes_the_shares() {
        let p = prices(&[(LABOR, 1.0)]);
        let d = demand(&peasant(), q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert!(close(d.income().get(), 10.0));
        assert!(close(d.subsistence_cost().get(), 2.0));
        assert_quantities(&d, &[(LABOR, 10.0), (GRAIN, 0.0)]);
        assert!(!d.get(good(LABOR)).unwrap().below_subsistence());
        assert!(d.get(good(GRAIN)).unwrap().below_subsistence());
        assert_balances(&d, &p);
    }

    #[test]
    fn an_untradable_held_good_is_kept_whole() {
        let p = prices(&[(GRAIN, 2.0)]);
        let d = demand(
            &peasant(),
            q(1.0),
            held(&[(LABOR, 10.0), (SILVER, 6.0)]),
            &p,
        );
        assert!(close(d.income().get(), 6.0));
        assert!(close(d.subsistence_cost().get(), 4.0));
        assert_quantities(&d, &[(LABOR, 10.0), (GRAIN, 3.0), (SILVER, 0.0)]);
        assert!(!d.below_subsistence());
        assert_balances(&d, &p);
    }

    #[test]
    fn no_positive_tradable_share_demands_currency() {
        let t = type_of(&[(LABOR, 2.0, 0.0), (GRAIN, 0.0, 1.0)]);
        let p = prices(&[(LABOR, 1.0)]);
        let d = demand(&t, q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert!(close(d.income().get(), 10.0));
        assert!(close(d.subsistence_cost().get(), 2.0));
        assert_quantities(&d, &[(LABOR, 2.0), (GRAIN, 0.0), (SILVER, 8.0)]);
        assert!(!d.get(good(GRAIN)).unwrap().below_subsistence());
        assert!(!d.below_subsistence());
        assert_eq!(d.get(good(SILVER)).unwrap().held, Quantity::ZERO);
        assert_balances(&d, &p);
    }

    #[test]
    fn the_fallback_adds_nothing_when_there_is_no_surplus() {
        let t = type_of(&[(LABOR, 2.0, 0.0), (GRAIN, 0.0, 1.0)]);
        let p = prices(&[(LABOR, 1.0)]);
        let d = demand(&t, q(1.0), held(&[(LABOR, 2.0)]), &p);
        assert_quantities(&d, &[(LABOR, 2.0), (GRAIN, 0.0)]);
        assert!(d.get(good(SILVER)).is_none());
        assert_balances(&d, &p);
    }

    #[test]
    fn a_utility_that_names_the_currency_prices_it_at_one() {
        let t = type_of(&[(LABOR, 0.0, 0.5), (SILVER, 1.0, 0.5)]);
        let p = prices(&[(LABOR, 2.0)]);
        let d = demand(&t, q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert!(close(d.income().get(), 20.0));
        assert!(close(d.subsistence_cost().get(), 1.0));
        assert!(close(d.supernumerary(), 19.0));
        assert_quantities(&d, &[(LABOR, 4.75), (SILVER, 10.5)]);
        assert_balances(&d, &p);
    }

    #[test]
    fn held_currency_is_income() {
        let p = typical();
        let d = demand(
            &peasant(),
            q(1.0),
            held(&[(LABOR, 10.0), (SILVER, 5.0)]),
            &p,
        );
        assert!(close(d.income().get(), 15.0));
        assert_quantities(&d, &[(LABOR, 3.8), (GRAIN, 5.6), (SILVER, 0.0)]);
        assert_balances(&d, &p);
    }

    #[test]
    fn a_tradable_held_good_outside_the_utility_is_all_for_sale() {
        let p = typical();
        let d = demand(
            &peasant(),
            q(1.0),
            held(&[(LABOR, 10.0), (COPPER, 1.0)]),
            &p,
        );
        assert!(close(d.income().get(), 15.0));
        let copper = d.get(good(COPPER)).unwrap();
        assert_eq!(copper.quantity, Quantity::ZERO);
        assert_eq!(copper.held, q(1.0));
        assert_balances(&d, &p);
    }

    #[test]
    fn a_pure_necessity_gets_exactly_its_subsistence() {
        let t = type_of(&[(LABOR, 3.0, 0.0), (GRAIN, 0.0, 1.0)]);
        let p = typical();
        let d = demand(&t, q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert_eq!(d.quantity_of(good(LABOR)), q(3.0));
        assert!(close(d.quantity_of(good(GRAIN)).get(), 3.5));
        assert_balances(&d, &p);
    }

    #[test]
    fn holding_nothing_demands_nothing_tradable() {
        let p = typical();
        let d = demand(&peasant(), q(1.0), Vec::new(), &p);
        assert_eq!(d.income(), Quantity::ZERO);
        assert!(close(d.subsistence_cost().get(), 6.0));
        assert!(close(d.supernumerary(), -6.0));
        assert_quantities(&d, &[(LABOR, 0.0), (GRAIN, 0.0)]);
        assert!(d.goods().iter().all(DemandedGood::below_subsistence));
        assert_balances(&d, &p);
    }

    #[test]
    fn duplicate_holdings_are_summed() {
        let p = typical();
        let split = demand(&peasant(), q(1.0), held(&[(LABOR, 4.0), (LABOR, 6.0)]), &p);
        let whole = demand(&peasant(), q(1.0), held(&[(LABOR, 10.0)]), &p);
        assert_eq!(split, whole);
        assert_eq!(split.get(good(LABOR)).unwrap().held, q(10.0));
    }

    #[test]
    fn a_zero_size_demands_nothing() {
        let p = typical();
        let d = demand(&peasant(), Quantity::ZERO, Vec::new(), &p);
        assert_quantities(&d, &[(LABOR, 0.0), (GRAIN, 0.0)]);
        assert_eq!(d.subsistence_cost(), Quantity::ZERO);
        assert!(!d.below_subsistence());
    }

    #[test]
    fn accessors_find_goods_or_give_zero() {
        let p = typical();
        let d = demand(&peasant(), q(1.0), held(&[(LABOR, 10.0)]), &p);
        let grain = d.get(good(GRAIN)).unwrap();
        assert_eq!(grain.good, good(GRAIN));
        assert_eq!(grain.held, Quantity::ZERO);
        assert_eq!(grain.subsistence, q(2.0));
        assert!(close(d.quantity_of(good(GRAIN)).get(), 3.6));
        assert!(d.get(good(COPPER)).is_none());
        assert_eq!(d.quantity_of(good(COPPER)), Quantity::ZERO);
    }
}
