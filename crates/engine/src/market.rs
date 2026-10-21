//! The dummy market. Each hex may post fixed prices in its currency good and exchange goods
//! against any stockpile at those prices. It holds nothing: goods sold to it vanish, goods
//! bought appear from nothing, and currency likewise. A good with no posted price isn't
//! traded on that hex; the hex's currency has price 1 and can't be traded against itself.
//! A `Market` is a [`PriceList`], so the [`demand`](crate::demand) module can compute a POP's
//! demand against it. The trade round in the [`trade`](crate::trade) module uses the
//! exchange. This is the seam Stage 04 replaces with the live market. Prices are starting
//! state and nothing changes them yet. Knows nothing about files.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::demand::PriceList;
use crate::goods::{GoodId, Goods};
use crate::id::IdVec;
use crate::map::{HexId, HexKind};
use crate::quantity::Quantity;
use crate::stockpile::{PerishesAt, Stockpile};
use crate::turn::Turn;

/// Each hex's posted prices, by [`HexId`]. None for a hex with no `[[market]]` entry.
#[derive(Debug, Default)]
pub struct HexPrices {
    by_hex: IdVec<HexKind, Option<BTreeMap<GoodId, Quantity>>>,
}

impl HexPrices {
    /// One entry per hex, in hex ID order. Crate-private: only the loader builds this.
    ///
    /// # Panics
    /// Panics if there are more than `u32::MAX` entries (via `IdVec::from_vec`).
    pub(crate) fn from_vec(lists: Vec<Option<BTreeMap<GoodId, Quantity>>>) -> Self {
        HexPrices {
            by_hex: IdVec::from_vec(lists),
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

    /// Number of hexes that have a market (a `[[market]]` entry, even with no prices).
    pub fn market_count(&self) -> usize {
        self.by_hex
            .iter()
            .filter(|(_, list)| list.is_some())
            .count()
    }

    /// True if `hex` has a market.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these were built with.
    pub fn has_market(&self, hex: HexId) -> bool {
        self.by_hex.get(hex).is_some()
    }

    /// The posted price of `good` on `hex`, or None if none is posted. Knows nothing about
    /// the currency: use [`Market::price`] for the full answer.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these were built with.
    pub fn get(&self, hex: HexId, good: GoodId) -> Option<Quantity> {
        self.by_hex.get(hex).as_ref()?.get(&good).copied()
    }

    /// Every posted price on `hex` as (good, price), in good ID order; empty if none.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these were built with.
    pub fn posted(&self, hex: HexId) -> impl Iterator<Item = (GoodId, Quantity)> + '_ {
        self.by_hex
            .get(hex)
            .iter()
            .flat_map(|list| list.iter().map(|(good, price)| (*good, *price)))
    }

    /// The hex's price list, for building a [`Market`]. Crate-private.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these were built with.
    pub(crate) fn list(&self, hex: HexId) -> Option<&BTreeMap<GoodId, Quantity>> {
        self.by_hex.get(hex).as_ref()
    }
}

/// Why a trade was refused. The stockpile is unchanged in every case.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeError {
    /// No price is posted for the good on this hex.
    NotTraded {
        /// The good.
        good: GoodId,
    },
    /// The good is the hex's currency, which isn't traded against itself.
    IsCurrency {
        /// The good.
        good: GoodId,
    },
    /// Selling more of a good than the stockpile holds.
    NotEnoughGoods {
        /// The good.
        good: GoodId,
        /// How much the stockpile holds.
        held: Quantity,
        /// How much was to be sold.
        wanted: Quantity,
    },
    /// Buying costs more currency than the stockpile holds.
    NotEnoughCurrency {
        /// The hex's currency.
        currency: GoodId,
        /// How much currency the stockpile holds.
        held: Quantity,
        /// How much currency the purchase costs.
        needed: Quantity,
    },
}

impl fmt::Display for TradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeError::NotTraded { good } => {
                write!(f, "good #{} is not traded on this hex", good.index())
            }
            TradeError::IsCurrency { good } => write!(
                f,
                "good #{} is this hex's currency and can't be traded against itself",
                good.index()
            ),
            TradeError::NotEnoughGoods { good, held, wanted } => write!(
                f,
                "not enough of good #{} to sell: holds {held}, wants to sell {wanted}",
                good.index()
            ),
            TradeError::NotEnoughCurrency {
                currency,
                held,
                needed,
            } => write!(
                f,
                "not enough currency (good #{}) to buy: holds {held}, needs {needed}",
                currency.index()
            ),
        }
    }
}

impl Error for TradeError {}

/// One hex's market, borrowed from a [`World`](crate::world::World): its posted prices and
/// the exchange.
#[derive(Debug, Clone, Copy)]
pub struct Market<'a> {
    hex: HexId,
    goods: &'a Goods,
    currency: GoodId,
    posted: Option<&'a BTreeMap<GoodId, Quantity>>,
    turn: Turn,
}

impl<'a> Market<'a> {
    /// Crate-private: `World::market` and the World's trade round build it.
    pub(crate) fn new(
        hex: HexId,
        goods: &'a Goods,
        currency: GoodId,
        posted: Option<&'a BTreeMap<GoodId, Quantity>>,
        turn: Turn,
    ) -> Self {
        Market {
            hex,
            goods,
            currency,
            posted,
            turn,
        }
    }

    /// The hex this market is on.
    pub fn hex(&self) -> HexId {
        self.hex
    }

    /// The good prices are posted in.
    pub fn currency(&self) -> GoodId {
        self.currency
    }

    /// True if the hex has a market (a `[[market]]` entry).
    pub fn has_market(&self) -> bool {
        self.posted.is_some()
    }

    /// The price of `good` in the currency: the posted price, [`Quantity::ONE`] for the
    /// currency itself, or None if the good isn't traded here.
    pub fn price(&self, good: GoodId) -> Option<Quantity> {
        if good == self.currency {
            return Some(Quantity::ONE);
        }
        self.posted?.get(&good).copied()
    }

    /// The posted prices as (good, price), in good ID order. The currency isn't listed.
    pub fn prices(&self) -> impl Iterator<Item = (GoodId, Quantity)> + '_ {
        self.posted
            .into_iter()
            .flat_map(|list| list.iter().map(|(good, price)| (*good, *price)))
    }

    /// The price of `good` for a trade, or why it can't be traded.
    fn trade_price(&self, good: GoodId) -> Result<Quantity, TradeError> {
        if good == self.currency {
            return Err(TradeError::IsCurrency { good });
        }
        self.posted
            .and_then(|list| list.get(&good).copied())
            .ok_or(TradeError::NotTraded { good })
    }

    /// When units of `good` added now perish.
    fn perishes_at(&self, good: GoodId) -> PerishesAt {
        PerishesAt::for_good(self.goods.get(good).perishes, self.turn)
    }

    /// Sells `amount` of `good` from `stockpile`: removes the good and adds
    /// `amount * price` of the currency. Returns the currency received.
    pub fn sell(
        &self,
        stockpile: &mut Stockpile,
        good: GoodId,
        amount: Quantity,
    ) -> Result<Quantity, TradeError> {
        let price = self.trade_price(good)?;
        let held = stockpile.total(good);
        if held < amount {
            return Err(TradeError::NotEnoughGoods {
                good,
                held,
                wanted: amount,
            });
        }
        if let Err(held) = stockpile.remove(good, amount) {
            // Can't happen after the check above; refuse rather than panic.
            return Err(TradeError::NotEnoughGoods {
                good,
                held,
                wanted: amount,
            });
        }
        let received = amount.saturating_mul(price);
        stockpile.add(self.currency, received, self.perishes_at(self.currency));
        Ok(received)
    }

    /// Sells everything of `good` above `keep` from `stockpile` (soonest-perishing first), so
    /// that `keep` remains, and adds the proceeds in currency. Returns (amount sold, currency
    /// received); (0, 0) if the stockpile holds `keep` or less. Same refusals as
    /// [`Market::sell`] except `NotEnoughGoods`.
    pub fn sell_down_to(
        &self,
        stockpile: &mut Stockpile,
        good: GoodId,
        keep: Quantity,
    ) -> Result<(Quantity, Quantity), TradeError> {
        let price = self.trade_price(good)?;
        let sold = stockpile.retain(good, keep);
        let received = sold.saturating_mul(price);
        stockpile.add(self.currency, received, self.perishes_at(self.currency));
        Ok((sold, received))
    }

    /// Buys `amount` of `good` into `stockpile`: removes `amount * price` of the currency
    /// and adds the good, perishing as a good added this turn does. Returns the currency
    /// paid.
    pub fn buy(
        &self,
        stockpile: &mut Stockpile,
        good: GoodId,
        amount: Quantity,
    ) -> Result<Quantity, TradeError> {
        let price = self.trade_price(good)?;
        let needed = amount.saturating_mul(price);
        let held = stockpile.total(self.currency);
        if held < needed {
            return Err(TradeError::NotEnoughCurrency {
                currency: self.currency,
                held,
                needed,
            });
        }
        if let Err(held) = stockpile.remove(self.currency, needed) {
            // Can't happen after the check above; refuse rather than panic.
            return Err(TradeError::NotEnoughCurrency {
                currency: self.currency,
                held,
                needed,
            });
        }
        stockpile.add(good, amount, self.perishes_at(good));
        Ok(needed)
    }

    /// Spends exactly `spend` of the currency from `stockpile` on `good`, adding
    /// `spend / price` of it, perishing as a good added this turn does. Returns the amount
    /// bought (zero for a zero spend). Refuses `NotEnoughCurrency` if the stockpile holds
    /// less than `spend`; same other refusals as [`Market::buy`].
    pub fn buy_for(
        &self,
        stockpile: &mut Stockpile,
        good: GoodId,
        spend: Quantity,
    ) -> Result<Quantity, TradeError> {
        let price = self.trade_price(good)?;
        let held = stockpile.total(self.currency);
        if held < spend {
            return Err(TradeError::NotEnoughCurrency {
                currency: self.currency,
                held,
                needed: spend,
            });
        }
        if let Err(held) = stockpile.remove(self.currency, spend) {
            // Can't happen after the check above; refuse rather than panic.
            return Err(TradeError::NotEnoughCurrency {
                currency: self.currency,
                held,
                needed: spend,
            });
        }
        let amount = spend.checked_div(price).unwrap_or(Quantity::ZERO);
        stockpile.add(good, amount, self.perishes_at(good));
        Ok(amount)
    }
}

impl PriceList for Market<'_> {
    fn currency(&self) -> GoodId {
        Market::currency(self)
    }

    fn price(&self, good: GoodId) -> Option<Quantity> {
        Market::price(self, good)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{good, hex, q};

    fn prices() -> HexPrices {
        let mut first = BTreeMap::new();
        first.insert(good(2), q(1.5));
        first.insert(good(0), q(2.0));
        HexPrices::from_vec(vec![Some(first), None, Some(BTreeMap::new())])
    }

    #[test]
    fn from_vec_gives_ids_in_order_and_counts_markets() {
        let prices = prices();
        assert_eq!(prices.len(), 3);
        assert!(!prices.is_empty());
        assert_eq!(prices.market_count(), 2);
        assert!(prices.has_market(hex(0)));
        assert!(!prices.has_market(hex(1)));
        assert!(prices.has_market(hex(2)));
    }

    #[test]
    fn get_and_posted_answer_per_hex() {
        let prices = prices();
        assert_eq!(prices.get(hex(0), good(0)), Some(q(2.0)));
        assert_eq!(prices.get(hex(0), good(2)), Some(q(1.5)));
        assert_eq!(prices.get(hex(0), good(1)), None);
        assert_eq!(prices.get(hex(1), good(0)), None);
        assert_eq!(prices.get(hex(2), good(0)), None);
        // In good ID order, whatever the insertion order.
        let posted: Vec<_> = prices.posted(hex(0)).collect();
        assert_eq!(posted, vec![(good(0), q(2.0)), (good(2), q(1.5))]);
        assert_eq!(prices.posted(hex(1)).count(), 0);
        assert_eq!(prices.posted(hex(2)).count(), 0);
        assert!(prices.list(hex(1)).is_none());
        assert!(prices.list(hex(2)).is_some());
    }

    #[test]
    fn default_is_empty() {
        let prices = HexPrices::default();
        assert_eq!(prices.len(), 0);
        assert!(prices.is_empty());
        assert_eq!(prices.market_count(), 0);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn get_with_a_foreign_hex_panics() {
        prices().get(hex(3), good(0));
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn has_market_with_a_foreign_hex_panics() {
        prices().has_market(hex(3));
    }

    #[test]
    fn trade_errors_display() {
        assert_eq!(
            TradeError::NotTraded { good: good(2) }.to_string(),
            "good #2 is not traded on this hex"
        );
        assert_eq!(
            TradeError::IsCurrency { good: good(5) }.to_string(),
            "good #5 is this hex's currency and can't be traded against itself"
        );
        assert_eq!(
            TradeError::NotEnoughGoods {
                good: good(2),
                held: q(2.0),
                wanted: q(3.0)
            }
            .to_string(),
            "not enough of good #2 to sell: holds 2, wants to sell 3"
        );
        assert_eq!(
            TradeError::NotEnoughCurrency {
                currency: good(5),
                held: q(1.5),
                needed: q(3.0)
            }
            .to_string(),
            "not enough currency (good #5) to buy: holds 1.5, needs 3"
        );
    }
}
