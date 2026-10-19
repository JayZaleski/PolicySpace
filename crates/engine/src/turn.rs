//! Turns: the clock's turn numbers.
//!
//! The clock counts turns as whole numbers from 0. A world starts at turn 0 and a step
//! advances it by one. [`Turn`] is a `u64` behind a type, so turn numbers can't be
//! confused with other integers and the clock can't wrap. A turn is written to JSON as a
//! whole number.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A turn number. Turn 0 is the first turn.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct Turn(u64);

impl Turn {
    /// Turn 0.
    pub const ZERO: Turn = Turn(0);
    /// The largest turn number. A world at this turn can't step.
    pub const MAX: Turn = Turn(u64::MAX);

    /// Turn `n`.
    pub const fn new(n: u64) -> Turn {
        Turn(n)
    }

    /// The turn number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next turn, or None at `MAX`.
    pub fn next(self) -> Option<Turn> {
        self.0.checked_add(1).map(Turn)
    }

    /// `self + n`, or `MAX` if that would overflow.
    pub fn saturating_add(self, n: u64) -> Turn {
        Turn(self.0.saturating_add(n))
    }
}

/// Displays the number: `0`, `7`.
impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_round_trip_through_json_as_whole_numbers() {
        for n in [0, 7, u64::MAX] {
            let turn = Turn::new(n);
            let text = serde_json::to_string(&turn).unwrap();
            assert_eq!(text, n.to_string());
            assert_eq!(serde_json::from_str::<Turn>(&text).unwrap(), turn);
        }
    }

    #[test]
    fn reading_refuses_a_fraction_a_negative_and_a_string() {
        for text in ["3.0", "-1", "\"3\""] {
            let error = serde_json::from_str::<Turn>(text).unwrap_err();
            assert!(!error.to_string().is_empty(), "{text} should be refused");
        }
    }

    #[test]
    fn new_and_get() {
        assert_eq!(Turn::new(7).get(), 7);
        assert_eq!(Turn::ZERO.get(), 0);
        assert_eq!(Turn::MAX.get(), u64::MAX);
    }

    #[test]
    fn next_advances_by_one_and_stops_at_max() {
        assert_eq!(Turn::ZERO.next(), Some(Turn::new(1)));
        assert_eq!(Turn::new(41).next(), Some(Turn::new(42)));
        assert_eq!(Turn::new(u64::MAX - 1).next(), Some(Turn::MAX));
        assert_eq!(Turn::MAX.next(), None);
    }

    #[test]
    fn saturating_add_is_normal_then_clamps_at_max() {
        assert_eq!(Turn::new(5).saturating_add(3), Turn::new(8));
        assert_eq!(Turn::new(5).saturating_add(0), Turn::new(5));
        assert_eq!(Turn::new(u64::MAX - 1).saturating_add(1), Turn::MAX);
        assert_eq!(Turn::new(u64::MAX - 1).saturating_add(2), Turn::MAX);
        assert_eq!(Turn::MAX.saturating_add(u64::MAX), Turn::MAX);
    }

    #[test]
    fn display_shows_the_number() {
        assert_eq!(Turn::ZERO.to_string(), "0");
        assert_eq!(Turn::new(7).to_string(), "7");
        assert_eq!(Turn::MAX.to_string(), "18446744073709551615");
    }

    #[test]
    fn turns_are_ordered_by_number() {
        assert!(Turn::new(1) < Turn::new(2));
        assert!(Turn::new(2) < Turn::MAX);
        assert!(Turn::ZERO <= Turn::ZERO);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Turn::default(), Turn::ZERO);
    }
}
