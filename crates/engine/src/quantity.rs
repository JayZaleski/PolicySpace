//! Quantities: amounts of goods.
//!
//! A quantity is an amount of a good (or, here, a deposit's output per period). Quantities
//! are continuous. They're stored as `f64` for now, behind this type, so every system uses
//! one representation that can be swapped later (for example, to fixed decimal for
//! cross-platform determinism) in one place. A [`Quantity`] is always finite and never
//! negative. Arithmetic is added when a system needs it; so far, `saturating_add`,
//! `checked_sub` and `saturating_mul`.

use std::fmt;

/// An amount of a good: a finite, non-negative number.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Quantity(f64);

impl Quantity {
    /// Zero.
    pub const ZERO: Quantity = Quantity(0.0);

    /// One unit.
    pub const ONE: Quantity = Quantity(1.0);

    /// The quantity `value`, or None if it is negative, NaN, or infinite.
    /// `-0.0` becomes `0.0`.
    pub fn new(value: f64) -> Option<Quantity> {
        if value.is_finite() && value >= 0.0 {
            // Adding 0.0 turns -0.0 into 0.0 and leaves every other value unchanged.
            Some(Quantity(value + 0.0))
        } else {
            None
        }
    }

    /// The value as an `f64`.
    pub fn get(self) -> f64 {
        self.0
    }

    /// `self + other`, clamped to the largest finite `f64` so the result stays finite.
    pub fn saturating_add(self, other: Quantity) -> Quantity {
        // Both values are finite and non-negative, so only overflow to infinity needs
        // the clamp.
        Quantity((self.0 + other.0).min(f64::MAX))
    }

    /// `self - other`, or None if `other > self`. The result is normalized (never -0.0).
    pub fn checked_sub(self, other: Quantity) -> Option<Quantity> {
        if other.0 > self.0 {
            None
        } else {
            // Both values are finite and `other <= self`, so the difference is finite and
            // non-negative. Adding 0.0 guards against -0.0.
            Some(Quantity(self.0 - other.0 + 0.0))
        }
    }

    /// `self * other`, clamped at `f64::MAX`.
    pub fn saturating_mul(self, other: Quantity) -> Quantity {
        // Both values are finite and non-negative, so only overflow to infinity needs the
        // clamp. (Zero times a finite value is zero, never NaN.)
        Quantity((self.0 * other.0).min(f64::MAX))
    }
}

/// Displays the number as `f64` does: `4`, `2.5`, `0`.
impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_finite_non_negative_values() {
        for value in [0.0, 2.5, 1e300, 4.0] {
            assert_eq!(Quantity::new(value).map(Quantity::get), Some(value));
        }
    }

    #[test]
    fn new_rejects_negative_nan_and_infinite_values() {
        for value in [-1.0, -1e-300, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Quantity::new(value), None, "{value} should be rejected");
        }
    }

    #[test]
    fn negative_zero_becomes_zero() {
        let q = Quantity::new(-0.0).unwrap();
        assert_eq!(q.to_string(), "0");
        assert!(q.get().is_sign_positive());
        assert_eq!(q, Quantity::ZERO);
    }

    #[test]
    fn display_matches_f64() {
        assert_eq!(Quantity::new(4.0).unwrap().to_string(), "4");
        assert_eq!(Quantity::new(2.5).unwrap().to_string(), "2.5");
        assert_eq!(Quantity::ZERO.to_string(), "0");
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Quantity::default(), Quantity::ZERO);
        assert_eq!(Quantity::default().get(), 0.0);
    }

    #[test]
    fn quantities_compare_by_value() {
        assert!(Quantity::new(1.0).unwrap() < Quantity::new(2.0).unwrap());
    }

    #[test]
    fn saturating_add_sums_normal_values() {
        let a = Quantity::new(2.5).unwrap();
        let b = Quantity::new(4.0).unwrap();
        assert_eq!(a.saturating_add(b).get(), 6.5);
        assert_eq!(a.saturating_add(Quantity::ZERO), a);
        assert_eq!(Quantity::ZERO.saturating_add(a), a);
    }

    #[test]
    fn saturating_add_clamps_at_the_largest_finite_value() {
        let max = Quantity::new(f64::MAX).unwrap();
        let sum = max.saturating_add(max);
        assert_eq!(sum.get(), f64::MAX);
        assert!(sum.get().is_finite());
    }

    #[test]
    fn one_is_one() {
        assert_eq!(Quantity::ONE.get(), 1.0);
    }

    #[test]
    fn checked_sub_subtracts_normal_values() {
        let a = Quantity::new(4.5).unwrap();
        let b = Quantity::new(1.5).unwrap();
        assert_eq!(a.checked_sub(b).map(Quantity::get), Some(3.0));
        assert_eq!(a.checked_sub(Quantity::ZERO), Some(a));
    }

    #[test]
    fn checked_sub_of_equal_values_is_positive_zero() {
        let a = Quantity::new(2.5).unwrap();
        let zero = a.checked_sub(a).unwrap();
        assert_eq!(zero, Quantity::ZERO);
        assert!(zero.get().is_sign_positive());
    }

    #[test]
    fn checked_sub_of_a_larger_value_is_none() {
        let a = Quantity::new(1.0).unwrap();
        let b = Quantity::new(2.0).unwrap();
        assert_eq!(a.checked_sub(b), None);
    }

    #[test]
    fn saturating_mul_multiplies_normal_values() {
        let a = Quantity::new(1.5).unwrap();
        let b = Quantity::new(4.0).unwrap();
        assert_eq!(a.saturating_mul(b).get(), 6.0);
        assert_eq!(a.saturating_mul(Quantity::ZERO), Quantity::ZERO);
        assert_eq!(a.saturating_mul(Quantity::ONE), a);
    }

    #[test]
    fn saturating_mul_clamps_at_the_largest_finite_value() {
        let max = Quantity::new(f64::MAX).unwrap();
        let product = max.saturating_mul(Quantity::new(2.0).unwrap());
        assert_eq!(product.get(), f64::MAX);
    }
}
