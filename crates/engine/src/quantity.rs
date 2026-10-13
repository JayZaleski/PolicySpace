//! Quantities: amounts of goods.
//!
//! A quantity is an amount of a good (or, here, a deposit's output per period). Quantities
//! are continuous. They're stored as `f64` for now, behind this type, so every system uses
//! one representation that can be swapped later (for example, to fixed decimal for
//! cross-platform determinism) in one place. A [`Quantity`] is always finite and never
//! negative. Arithmetic is added when a system needs it.

use std::fmt;

/// An amount of a good: a finite, non-negative number.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Quantity(f64);

impl Quantity {
    /// Zero.
    pub const ZERO: Quantity = Quantity(0.0);

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
}
