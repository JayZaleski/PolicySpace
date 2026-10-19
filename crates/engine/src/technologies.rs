//! Technologies. A technology (`[[technology]]` in `rules/`) is a named recipe that
//! factories run, shared by every factory that uses it, as a POP type is shared by POPs:
//! the inputs one unit of output takes (fixed proportions, no substitution; the table may
//! be empty), the one output good, the capacity (the most output a factory makes in a
//! turn), the capital bundle (the goods a finished factory holds; at least one good), the
//! construction bundle (the goods spent to build a factory, consumed, not kept; may be
//! empty), the construction turns t and the lifetime t₁ (whole numbers of at least 1). The
//! output good is never among the inputs.
//!
//! What the numbers mean:
//! - **Depreciation.** Every turn a factory loses 1/t₁ of its capital bundle, whether or
//!   not it ran. When the capital is exhausted the factory shuts down.
//! - **Construction.** Building a factory pays 1/t of both the capital bundle and the
//!   construction bundle each turn, for t turns. The construction bundle is consumed; the
//!   capital bundle becomes the factory's capital.
//! - **Maintenance.** Maintenance is the construction project run for a fraction of its
//!   time: replenishing a share f of the capital costs f of both bundles, and at most 1/t
//!   of both can be paid per turn.
//! - **t > t₁ is allowed.** Such a factory wears out faster than it can be maintained.
//!
//! The [`production`](crate::production) module runs, wears and maintains factories by
//! these numbers, as pure functions; Step 08 of this Stage wires it into the turn. Knows
//! nothing about files: the setting loader builds [`Technologies`] from a checked setting,
//! and everything else only reads them.

use crate::bundle::Bundle;
use crate::goods::GoodId;
use crate::id::{Id, IdKind, NamedList};
use crate::quantity::Quantity;

/// Marker kind for technologies.
pub enum TechnologyKind {}

impl IdKind for TechnologyKind {
    const LABEL: &'static str = "technology";
}

/// The ID of a technology.
pub type TechnologyId = Id<TechnologyKind>;

/// One technology: a fixed-proportion recipe and what a factory of it is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Technology {
    /// Per unit of output; every amount > 0; never names `output`.
    inputs: Bundle,
    /// The good a factory of this technology makes.
    output: GoodId,
    /// The most output per turn; > 0.
    capacity: Quantity,
    /// The goods a finished factory holds; never empty; every amount > 0.
    capital: Bundle,
    /// The goods spent to build a factory; every amount > 0; may be empty.
    construction: Bundle,
    /// Turns to build a factory; >= 1.
    construction_turns: u32,
    /// Turns a factory's capital lasts; >= 1.
    lifetime: u32,
}

impl Technology {
    /// Crate-private: only the loader builds technologies.
    ///
    /// # Panics
    /// Panics if `inputs` names `output` (`"a technology's output good may not be among
    /// its inputs"`), if `capital` is empty (`"a technology's capital bundle must name at
    /// least one good"`), if any amount in `inputs`, `capital` or `construction`, or
    /// `capacity`, is 0 (`"a technology's amounts must be greater than 0"`), or if
    /// `construction_turns` or `lifetime` is 0 (`"a technology's construction turns and
    /// lifetime must be at least 1"`). The loader guarantees all of this.
    pub(crate) fn new(
        inputs: Bundle,
        output: GoodId,
        capacity: Quantity,
        capital: Bundle,
        construction: Bundle,
        construction_turns: u32,
        lifetime: u32,
    ) -> Self {
        assert!(
            !inputs.contains(output),
            "a technology's output good may not be among its inputs"
        );
        assert!(
            !capital.is_empty(),
            "a technology's capital bundle must name at least one good"
        );
        let positive = |q: Quantity| q.get() > 0.0;
        assert!(
            positive(capacity)
                && [&inputs, &capital, &construction]
                    .iter()
                    .all(|b| b.iter().all(|(_, amount)| positive(amount))),
            "a technology's amounts must be greater than 0"
        );
        assert!(
            construction_turns >= 1 && lifetime >= 1,
            "a technology's construction turns and lifetime must be at least 1"
        );
        Technology {
            inputs,
            output,
            capacity,
            capital,
            construction,
            construction_turns,
            lifetime,
        }
    }

    /// The inputs one unit of output takes. May be empty.
    pub fn inputs(&self) -> &Bundle {
        &self.inputs
    }

    /// The good a factory of this technology makes.
    pub fn output(&self) -> GoodId {
        self.output
    }

    /// The most output a factory of this technology makes in one turn. Greater than 0.
    pub fn capacity(&self) -> Quantity {
        self.capacity
    }

    /// The goods a finished factory holds. Never empty.
    pub fn capital(&self) -> &Bundle {
        &self.capital
    }

    /// The goods spent to build a factory. May be empty.
    pub fn construction(&self) -> &Bundle {
        &self.construction
    }

    /// How many turns building a factory takes (t). At least 1.
    pub fn construction_turns(&self) -> u32 {
        self.construction_turns
    }

    /// How many turns a factory's capital lasts (t₁). At least 1.
    pub fn lifetime(&self) -> u32 {
        self.lifetime
    }
}

/// Every technology in a setting, in load order (which is also ID order).
pub type Technologies = NamedList<TechnologyKind, Technology>;

#[cfg(test)]
mod tests {
    use super::*;

    fn good(i: u32) -> GoodId {
        GoodId::from_index(i)
    }

    fn q(v: f64) -> Quantity {
        Quantity::new(v).unwrap()
    }

    fn bundle(items: &[(u32, f64)]) -> Bundle {
        Bundle::new(items.iter().map(|&(g, a)| (good(g), q(a))).collect())
    }

    /// A technology whose output is good 2, with every other parameter given.
    fn make(
        inputs: Bundle,
        capacity: f64,
        capital: Bundle,
        construction: Bundle,
        construction_turns: u32,
        lifetime: u32,
    ) -> Technology {
        Technology::new(
            inputs,
            good(2),
            Quantity::new(capacity).unwrap(),
            capital,
            construction,
            construction_turns,
            lifetime,
        )
    }

    #[test]
    fn a_technology_stores_every_field() {
        let tech = Technology::new(
            bundle(&[(1, 2.0), (0, 1.0)]),
            good(2),
            q(10.0),
            bundle(&[(3, 5.0)]),
            bundle(&[(0, 20.0)]),
            2,
            10,
        );
        assert_eq!(tech.inputs(), &bundle(&[(0, 1.0), (1, 2.0)]));
        assert_eq!(tech.output(), good(2));
        assert_eq!(tech.capacity(), q(10.0));
        assert_eq!(tech.capital(), &bundle(&[(3, 5.0)]));
        assert_eq!(tech.construction(), &bundle(&[(0, 20.0)]));
        assert_eq!(tech.construction_turns(), 2);
        assert_eq!(tech.lifetime(), 10);
    }

    #[test]
    fn the_output_may_be_in_the_capital_and_inputs_may_be_empty() {
        let tech = make(
            Bundle::default(),
            1.0,
            bundle(&[(2, 1.0)]),
            Bundle::default(),
            3,
            3,
        );
        assert!(tech.inputs().is_empty());
        assert!(tech.capital().contains(good(2)));
        assert!(tech.construction().is_empty());
    }

    #[test]
    #[should_panic(expected = "a technology's output good may not be among its inputs")]
    fn the_output_among_the_inputs_panics() {
        make(
            bundle(&[(2, 1.0)]),
            1.0,
            bundle(&[(0, 1.0)]),
            Bundle::default(),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's capital bundle must name at least one good")]
    fn an_empty_capital_bundle_panics() {
        make(
            Bundle::default(),
            1.0,
            Bundle::default(),
            Bundle::default(),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's amounts must be greater than 0")]
    fn a_zero_input_amount_panics() {
        make(
            Bundle::new(vec![(good(0), Quantity::ZERO)]),
            1.0,
            bundle(&[(0, 1.0)]),
            Bundle::default(),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's amounts must be greater than 0")]
    fn a_zero_capital_amount_panics() {
        make(
            Bundle::default(),
            1.0,
            Bundle::new(vec![(good(0), Quantity::ZERO)]),
            Bundle::default(),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's amounts must be greater than 0")]
    fn a_zero_construction_amount_panics() {
        make(
            Bundle::default(),
            1.0,
            bundle(&[(0, 1.0)]),
            Bundle::new(vec![(good(0), Quantity::ZERO)]),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's amounts must be greater than 0")]
    fn a_zero_capacity_panics() {
        make(
            Bundle::default(),
            0.0,
            bundle(&[(0, 1.0)]),
            Bundle::default(),
            1,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's construction turns and lifetime must be at least 1")]
    fn zero_construction_turns_panics() {
        make(
            Bundle::default(),
            1.0,
            bundle(&[(0, 1.0)]),
            Bundle::default(),
            0,
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a technology's construction turns and lifetime must be at least 1")]
    fn a_zero_lifetime_panics() {
        make(
            Bundle::default(),
            1.0,
            bundle(&[(0, 1.0)]),
            Bundle::default(),
            1,
            0,
        );
    }
}
