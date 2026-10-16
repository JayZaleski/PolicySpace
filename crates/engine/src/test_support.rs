//! Helpers shared by the engine's unit tests.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::demand::PriceList;
use crate::goods::{GoodId, Perishes};
use crate::map::HexId;
use crate::pops::{GrowthParams, PopType};
use crate::quantity::Quantity;
use crate::stockpile::PerishesAt;
use crate::turn::Turn;

/// The index of unskilled labor in the demand and growth tests' goods.
pub(crate) const LABOR: u32 = 0;
/// The index of grain in the demand and growth tests' goods.
pub(crate) const GRAIN: u32 = 1;
/// The index of copper in the demand and growth tests' goods.
pub(crate) const COPPER: u32 = 2;
/// The index of silver, the currency, in the demand and growth tests' goods.
pub(crate) const SILVER: u32 = 3;

/// A quantity of `value`, which must be valid.
pub(crate) fn q(value: f64) -> Quantity {
    Quantity::new(value).unwrap()
}

/// Whether two numbers are within 1e-9 of each other.
pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The good with the given index.
pub(crate) fn good(index: u32) -> GoodId {
    GoodId::from_index(index)
}

/// The hex with the given index.
pub(crate) fn hex(index: u32) -> HexId {
    HexId::from_index(index)
}

/// The end of turn `t`, when goods perish.
pub(crate) fn end(t: u64) -> PerishesAt {
    PerishesAt::EndOfTurn(Turn::new(t))
}

/// A good that perishes after `n` turns.
pub(crate) fn after(n: u32) -> Perishes {
    Perishes::After(NonZeroU32::new(n).unwrap())
}

/// Growth parameters that never change a POP's size.
pub(crate) fn no_growth() -> GrowthParams {
    GrowthParams::new(vec![0.0], 0.0, 0.0)
}

/// Hand-written prices: a currency good and a table of the other prices.
pub(crate) struct Prices {
    pub(crate) currency: GoodId,
    pub(crate) table: BTreeMap<GoodId, Quantity>,
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

/// Prices of the given (good index, price) pairs, with silver as the currency.
pub(crate) fn prices(list: &[(u32, f64)]) -> Prices {
    Prices {
        currency: good(SILVER),
        table: list.iter().map(|&(g, p)| (good(g), q(p))).collect(),
    }
}

/// Labor 1, grain 2, copper 5.
pub(crate) fn typical() -> Prices {
    prices(&[(LABOR, 1.0), (GRAIN, 2.0), (COPPER, 5.0)])
}

/// Labor subsistence 2 weight 0.2, grain subsistence 2 weight 0.8, endowed with 10 labor.
pub(crate) fn peasant() -> PopType {
    PopType::new(
        vec![(good(LABOR), q(10.0))],
        vec![(good(LABOR), q(2.0), 0.2), (good(GRAIN), q(2.0), 0.8)],
        no_growth(),
    )
}

/// A POP type with no endowment and the given (good index, subsistence, weight) utility.
pub(crate) fn type_of(utility: &[(u32, f64, f64)]) -> PopType {
    PopType::new(
        vec![],
        utility
            .iter()
            .map(|&(g, s, w)| (good(g), q(s), w))
            .collect(),
        no_growth(),
    )
}

/// Holdings of the given (good index, amount) pairs.
pub(crate) fn held(list: &[(u32, f64)]) -> Vec<(GoodId, Quantity)> {
    list.iter().map(|&(g, a)| (good(g), q(a))).collect()
}
