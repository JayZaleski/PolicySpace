//! POP growth. Each turn, after it has traded and consumed, a POP grows or shrinks by a
//! rate that depends on its **subsistence index** u, a dimensionless measure built from the
//! numbers its demand reports: 0 at exactly subsistence, positive above, negative (down to
//! −1) below. The rate is a function of u with parameters on the POP type
//! ([`GrowthParams`]): a polynomial with no constant term above subsistence, `decline × u`
//! below, clamped to [−decline, max_growth]. Because `decline` is below 1, the rate is above
//! −1 and N stays positive: the engine never removes a POP. The trade round
//! ([`trade`](crate::trade)) records each POP's index and rate in its outcome; the World
//! applies the rate to the size after every hex has traded. u is per individual (every input
//! scales with N), so it doesn't depend on the POP's size. In this Stage the demanded and
//! consumed quantities are the same; Stage 04 revisits which numbers the index reads once
//! rationing exists. Knows nothing about the World or files.

use crate::demand::Demand;
use crate::pops::GrowthParams;
use crate::quantity::Quantity;

/// The subsistence index of a demand. Above subsistence (no good below it): (M − C) / C,
/// the extra subsistence bundles the income could buy, or 0 if C is 0, capped at
/// `f64::MAX` so it is always finite (C tiny and M huge can make the ratio overflow).
/// Below: the smallest attainment (demanded quantity over subsistence need) over the goods
/// with a positive need, minus 1, so in [−1, 0). The two sides meet at 0.
pub fn subsistence_index(demand: &Demand) -> f64 {
    if !demand.below_subsistence() {
        let cost = demand.subsistence_cost().get();
        if cost == 0.0 {
            return 0.0;
        }
        // Not below subsistence means M >= C, so this is already >= 0; the lower bound
        // only guards rounding. The upper bound keeps the index finite when the ratio
        // overflows: an infinite index would make Horner start with 0 × ∞ = NaN in
        // `growth_rate`, and `grown_size` would then panic.
        return (demand.supernumerary() / cost).clamp(0.0, f64::MAX);
    }
    let index = demand
        .goods()
        .iter()
        .filter(|g| g.subsistence > Quantity::ZERO)
        .map(|g| g.quantity.get() / g.subsistence.get())
        .fold(f64::INFINITY, f64::min)
        - 1.0;
    debug_assert!((-1.0..0.0).contains(&index));
    index
}

/// The growth rate at `index`: the polynomial `c₁u + c₂u² + …` for `index >= 0`,
/// `decline × index` below, clamped to `[−decline, max_growth]`. Zero at index 0. The
/// index is finite ([`subsistence_index`] caps it), so the polynomial can overflow to
/// infinity but never produces NaN, and the clamp turns it into a finite rate.
pub fn growth_rate(params: &GrowthParams, index: f64) -> f64 {
    let rate = if index < 0.0 {
        params.decline() * index
    } else {
        // Horner's method; for [c1, c2] this is c1·u + c2·u².
        params
            .coefficients()
            .iter()
            .rev()
            .fold(0.0, |acc, c| acc * index + c)
            * index
    };
    // −decline <= 0 <= max_growth always holds, so clamp can't panic.
    rate.clamp(-params.decline(), params.max_growth())
}

/// `size × (1 + rate)`, saturating at the largest finite quantity. `rate` is a rate from
/// [`growth_rate`], so it is above −1 and the result is finite and non-negative.
pub fn grown_size(size: Quantity, rate: f64) -> Quantity {
    size.saturating_mul(Quantity::new(1.0 + rate).expect("1 + rate is finite and positive"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demand::demand;
    use crate::pops::PopType;
    use crate::test_support::{
        COPPER, GRAIN, LABOR, Prices, close, held, no_growth, peasant, prices, q, type_of, typical,
    };

    fn g() -> GrowthParams {
        GrowthParams::new(vec![0.06, -0.03], 0.1, 0.05)
    }

    fn index_of(pop_type: &PopType, size: f64, holdings: &[(u32, f64)], prices: &Prices) -> f64 {
        subsistence_index(&demand(pop_type, q(size), held(holdings), prices))
    }

    #[test]
    fn index_above_subsistence_counts_extra_bundles() {
        let u = index_of(&peasant(), 1.0, &[(LABOR, 10.0)], &typical());
        assert!(close(u, 2.0 / 3.0), "{u}");
        let artisan = type_of(&[(LABOR, 1.0, 1.0), (GRAIN, 2.0, 4.0), (COPPER, 0.0, 5.0)]);
        let u = index_of(&artisan, 1.0, &[(LABOR, 10.0)], &typical());
        assert!(close(u, 1.0), "{u}");
    }

    #[test]
    fn index_is_zero_at_exactly_subsistence() {
        let prices = prices(&[(LABOR, 1.0), (GRAIN, 4.0)]);
        let u = index_of(&peasant(), 1.0, &[(LABOR, 10.0)], &prices);
        assert_eq!(u, 0.0);
    }

    #[test]
    fn index_is_zero_when_subsistence_costs_nothing() {
        let free = type_of(&[(LABOR, 0.0, 1.0)]);
        assert_eq!(index_of(&free, 1.0, &[(LABOR, 10.0)], &typical()), 0.0);
        assert_eq!(index_of(&free, 1.0, &[], &typical()), 0.0);
    }

    #[test]
    fn index_below_subsistence_is_the_shortfall() {
        let prices = prices(&[(LABOR, 1.0), (GRAIN, 10.0)]);
        let u = index_of(&peasant(), 1.0, &[(LABOR, 10.0)], &prices);
        assert!(close(u, 10.0 / 22.0 - 1.0), "{u}");
    }

    #[test]
    fn index_is_minus_one_for_an_unobtainable_need() {
        let wanting = type_of(&[(LABOR, 0.0, 1.0), (COPPER, 1.0, 0.0)]);
        let labor_only = prices(&[(LABOR, 1.0)]);
        assert_eq!(index_of(&wanting, 1.0, &[(LABOR, 10.0)], &labor_only), -1.0);
        assert_eq!(
            index_of(&wanting, 1.0, &[(LABOR, 1000.0)], &labor_only),
            -1.0
        );
        let none = prices(&[]);
        assert_eq!(index_of(&peasant(), 1.0, &[(LABOR, 10.0)], &none), -1.0);
    }

    #[test]
    fn index_takes_the_worst_attainment() {
        let t = type_of(&[(LABOR, 2.0, 1.0), (COPPER, 1.0, 0.0)]);
        let labor_only = prices(&[(LABOR, 1.0)]);
        let u = index_of(&t, 1.0, &[(LABOR, 10.0), (COPPER, 0.5)], &labor_only);
        assert!(close(u, -0.5), "{u}");
    }

    #[test]
    fn index_stays_finite_when_the_ratio_overflows() {
        let tiny = type_of(&[(LABOR, 1e-160, 1.0)]);
        let prices = prices(&[(LABOR, 1.0)]);
        let u = index_of(&tiny, 1e-160, &[(LABOR, 1e300)], &prices);
        assert!(u.is_finite(), "{u}");
        let rate = growth_rate(&g(), u);
        assert_eq!(rate, -0.1);
        let _ = grown_size(q(10.0), rate);
    }

    #[test]
    fn index_does_not_depend_on_size() {
        let above = typical();
        let below = prices(&[(LABOR, 1.0), (GRAIN, 10.0)]);
        for p in [&above, &below] {
            let one = index_of(&peasant(), 1.0, &[(LABOR, 10.0)], p);
            let fifty = index_of(&peasant(), 50.0, &[(LABOR, 500.0)], p);
            assert!(close(one, fifty), "{one} vs {fifty}");
        }
    }

    #[test]
    fn rate_follows_the_polynomial_above_subsistence() {
        let p = g();
        assert!(close(growth_rate(&p, 2.0 / 3.0), 0.04 - 0.04 / 3.0));
        assert!(close(growth_rate(&p, 1.0), 0.03));
        assert!(close(growth_rate(&p, 2.0), 0.0));
        assert!(close(growth_rate(&p, 3.0), -0.09));
        let linear = GrowthParams::new(vec![0.03], 0.1, 1.0);
        assert!(close(growth_rate(&linear, 2.0), 0.06));
    }

    #[test]
    fn rate_is_zero_at_subsistence() {
        assert_eq!(growth_rate(&g(), 0.0), 0.0);
        let p = GrowthParams::new(vec![1.0, 2.0, 3.0], 0.1, 5.0);
        assert_eq!(growth_rate(&p, 0.0), 0.0);
    }

    #[test]
    fn rate_is_capped_and_floored() {
        let capped = GrowthParams::new(vec![1.0], 0.1, 0.05);
        assert_eq!(growth_rate(&capped, 9.0), 0.05);
        assert_eq!(growth_rate(&g(), 9.0), -0.1);
        let huge = GrowthParams::new(vec![1e300, 1e300], 0.1, 0.05);
        assert_eq!(growth_rate(&huge, 1e10), 0.05);
    }

    #[test]
    fn rate_declines_linearly_below_subsistence() {
        let p = g();
        assert!(close(growth_rate(&p, -0.5), -0.05));
        assert_eq!(growth_rate(&p, -1.0), -0.1);
        assert!(close(growth_rate(&p, 10.0 / 22.0 - 1.0), -0.6 / 11.0));
    }

    #[test]
    fn zero_parameters_give_a_zero_rate() {
        let p = no_growth();
        for u in [-1.0, -0.5, 0.0, 0.5, 9.0] {
            assert_eq!(growth_rate(&p, u), 0.0, "u = {u}");
        }
    }

    #[test]
    fn grown_size_multiplies_and_saturates() {
        let rate = 0.04 - 0.04 / 3.0;
        assert!(close(grown_size(q(20.0), rate).get(), 20.5333333333));
        assert!(close(grown_size(q(12.0), -0.1).get(), 10.8));
        assert_eq!(grown_size(q(10.0), 0.0), q(10.0));
        assert_eq!(grown_size(q(f64::MAX), 0.05), q(f64::MAX));
    }
}
