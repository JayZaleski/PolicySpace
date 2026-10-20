//! The trade round: the "actors act" part of the turn, for POPs and for businesses.
//!
//! Each POP submits its type's utility parameters, its size and its holdings to its hex's
//! market. The market posts its prices (fixed, this Stage), computes every POP submission's
//! demand with [`demand`](crate::demand), and settles every sale on the hex before any
//! purchase (so that in the live market every purchase sees the income the sales actually
//! produced): POP sales, then business sales, then POP purchases, then business purchases.
//! Then each POP consumes its utility goods up to what it demanded. The currency is never
//! traded: proceeds add it, purchases spend it, and what is left is a POP's demanded
//! quantity up to rounding. Hexes are handled by the World in hex ID order; within a hex
//! POPs go in POP ID order, then businesses in business ID order, and goods in good ID
//! order. Leftover currency stays in the stockpile. A refused trade is an engine bug.
//!
//! A business submits what the World resolved from its decision: offers and bids by good,
//! and its stockpile. It sells each offer up to what it holds, and buys its bids at posted
//! prices; when the bids cost more than the currency it holds after its sales, every bid is
//! scaled by one proportion (cash over cost), so what it buys keeps the shape it chose.
//! The round records what it did as a [`BusinessTrade`]. Operating the factories is not
//! part of the round: the World does that for each business right after the hex's round.
//!
//! A POP's record also carries its subsistence index and growth rate, computed by the
//! [`growth`](crate::growth) module from the demand and the type, and the size the turn
//! started with; the World applies the rate after every hex has traded. Stage 05 replaces
//! the inside of the round (adjusted prices, rationing); the submissions, the order,
//! consumption and the outcomes stay. Knows nothing about the World, deciders or files.

use crate::demand::{Demand, demand};
use crate::goods::GoodId;
use crate::growth::{growth_rate, subsistence_index};
use crate::market::{Market, TradeError};
use crate::pops::PopType;
use crate::quantity::Quantity;
use crate::stockpile::Stockpile;

/// One POP's submission to its hex's market: its type (the utility parameters), its size,
/// and its stockpile, moved out of the world for the round and moved back afterwards. The
/// round trades from and into the stockpile and consumption removes from it.
pub(crate) struct Submission<'a> {
    pub pop_type: &'a PopType,
    pub size: Quantity,
    pub stockpile: Stockpile,
}

/// What one turn's trade round did for one POP, kept by the World until the next turn.
#[derive(Debug, Clone, PartialEq)]
pub struct PopOutcome {
    /// The demand the market computed from the submission.
    demand: Demand,
    /// Per good sold, in good ID order, only amounts > 0.
    sold: Vec<(GoodId, Quantity)>,
    /// Per good bought, in good ID order, only amounts > 0.
    bought: Vec<(GoodId, Quantity)>,
    /// Per good consumed, in good ID order, only amounts > 0.
    consumed: Vec<(GoodId, Quantity)>,
    /// The hex's currency left in the stockpile after consumption.
    carried_over: Quantity,
    /// The POP's subsistence index this turn (see the `growth` module).
    subsistence_index: f64,
    /// The growth rate the World applies to the size after the round.
    growth_rate: f64,
    /// The size the turn started with (the size the demand and the trade were for).
    starting_size: Quantity,
}

/// The amount recorded for `good` in a list sorted by good; zero if none.
fn amount_of(list: &[(GoodId, Quantity)], good: GoodId) -> Quantity {
    list.iter()
        .find(|(g, _)| *g == good)
        .map_or(Quantity::ZERO, |(_, amount)| *amount)
}

impl PopOutcome {
    /// The demand the market computed from the POP's submission.
    pub fn demand(&self) -> &Demand {
        &self.demand
    }

    /// What the POP sold as (good, amount), in good ID order, only amounts > 0.
    pub fn sold(&self) -> &[(GoodId, Quantity)] {
        &self.sold
    }

    /// What the POP bought as (good, amount), in good ID order, only amounts > 0.
    pub fn bought(&self) -> &[(GoodId, Quantity)] {
        &self.bought
    }

    /// What the POP consumed as (good, amount), in good ID order, only amounts > 0.
    pub fn consumed(&self) -> &[(GoodId, Quantity)] {
        &self.consumed
    }

    /// The amount of `good` sold; zero if none.
    pub fn sold_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.sold, good)
    }

    /// The amount of `good` bought; zero if none.
    pub fn bought_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.bought, good)
    }

    /// The amount of `good` consumed; zero if none.
    pub fn consumed_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.consumed, good)
    }

    /// The hex's currency left in the POP's stockpile after consumption.
    pub fn carried_over(&self) -> Quantity {
        self.carried_over
    }

    /// The subsistence index: 0 at subsistence, positive above, down to −1 below.
    pub fn subsistence_index(&self) -> f64 {
        self.subsistence_index
    }

    /// The growth rate applied to the size after this turn's round.
    pub fn growth_rate(&self) -> f64 {
        self.growth_rate
    }

    /// The size the turn started with; the POP's current size is the size after growth.
    pub fn starting_size(&self) -> Quantity {
        self.starting_size
    }
}

/// One business's submission to its hex's market: its offers and bids as the World resolved
/// them from its decision (good ID order, only amounts > 0; both empty for a business that
/// decided nothing or was refused), and its stockpile, moved out for the round.
pub(crate) struct BusinessSubmission {
    pub offers: Vec<(GoodId, Quantity)>,
    pub bids: Vec<(GoodId, Quantity)>,
    pub stockpile: Stockpile,
}

/// What one turn's round did for one business.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessTrade {
    /// As submitted.
    offered: Vec<(GoodId, Quantity)>,
    /// Per good sold, in the offers' order, only amounts > 0.
    sold: Vec<(GoodId, Quantity)>,
    /// As submitted.
    bid: Vec<(GoodId, Quantity)>,
    /// Per good bought, in the bids' order, only amounts > 0.
    bought: Vec<(GoodId, Quantity)>,
    /// The cash cap's scaling of every bid; 1 when the bids were within the cash.
    proportion: f64,
    /// The currency held at the start of the round.
    cash_before: Quantity,
    /// The currency held after the whole sales pass.
    cash_after_sales: Quantity,
    /// The currency held after the whole purchase pass.
    cash_after_purchases: Quantity,
}

impl BusinessTrade {
    /// What the business offered as (good, amount), as submitted.
    pub fn offered(&self) -> &[(GoodId, Quantity)] {
        &self.offered
    }

    /// What the business sold as (good, amount), only amounts > 0.
    pub fn sold(&self) -> &[(GoodId, Quantity)] {
        &self.sold
    }

    /// What the business bid for as (good, amount), as submitted.
    pub fn bid(&self) -> &[(GoodId, Quantity)] {
        &self.bid
    }

    /// What the business bought as (good, amount), only amounts > 0.
    pub fn bought(&self) -> &[(GoodId, Quantity)] {
        &self.bought
    }

    /// The amount of `good` offered; zero if none.
    pub fn offered_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.offered, good)
    }

    /// The amount of `good` sold; zero if none.
    pub fn sold_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.sold, good)
    }

    /// The amount of `good` bid for; zero if none.
    pub fn bid_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.bid, good)
    }

    /// The amount of `good` bought; zero if none.
    pub fn bought_of(&self, good: GoodId) -> Quantity {
        amount_of(&self.bought, good)
    }

    /// The scaling the cash cap applied to every bid: 1 when the bids were within the cash,
    /// 0 for a business with no currency.
    pub fn proportion(&self) -> f64 {
        self.proportion
    }

    /// The currency held at the start of the round.
    pub fn cash_before(&self) -> Quantity {
        self.cash_before
    }

    /// The currency held after the sales.
    pub fn cash_after_sales(&self) -> Quantity {
        self.cash_after_sales
    }

    /// The currency held after the purchases.
    pub fn cash_after_purchases(&self) -> Quantity {
        self.cash_after_purchases
    }
}

/// The results of one hex's round, each in submission order.
pub(crate) struct HexRound {
    pub pops: Vec<PopOutcome>,
    pub businesses: Vec<BusinessTrade>,
}

/// Which submission the exchange refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusedTrader {
    /// The POP submission at this index.
    Pop(usize),
    /// The business submission at this index.
    Business(usize),
}

/// The exchange refused a trade during the round: an engine bug, since the demand, the
/// offers' cap and the purchase caps make every trade acceptable. The stockpiles are left
/// as they were at the refusal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RefusedTrade {
    /// Which submission, by index in the slice passed to `hex_round`.
    pub trader: RefusedTrader,
    pub error: TradeError,
}

/// Runs one hex's trade round and consumption for `pops` (the hex's POPs in POP ID order)
/// and `businesses` (its businesses in business ID order): every POP's demand; every sale
/// (POPs, then businesses); every purchase (POPs, then businesses); POP consumption.
/// Returns one result per submission, in the same order.
pub(crate) fn hex_round(
    market: &Market<'_>,
    pops: &mut [Submission<'_>],
    businesses: &mut [BusinessSubmission],
) -> Result<HexRound, RefusedTrade> {
    // 1. Demand.
    let demands: Vec<Demand> = pops
        .iter()
        .map(|s| demand(s.pop_type, s.size, s.stockpile.iter(), market))
        .collect();
    let cash_before: Vec<Quantity> = businesses
        .iter()
        .map(|b| b.stockpile.total(market.currency()))
        .collect();

    // 2. Every sale on the hex, before any purchase: POPs, then businesses.
    let mut pops_sold = Vec::with_capacity(pops.len());
    for (i, s) in pops.iter_mut().enumerate() {
        let done =
            settle_sales(market, &demands[i], &mut s.stockpile).map_err(|error| RefusedTrade {
                trader: RefusedTrader::Pop(i),
                error,
            })?;
        pops_sold.push(done);
    }
    let mut businesses_sold = Vec::with_capacity(businesses.len());
    for (i, b) in businesses.iter_mut().enumerate() {
        let done =
            settle_offers(market, &b.offers, &mut b.stockpile).map_err(|error| RefusedTrade {
                trader: RefusedTrader::Business(i),
                error,
            })?;
        businesses_sold.push(done);
    }
    let cash_after_sales: Vec<Quantity> = businesses
        .iter()
        .map(|b| b.stockpile.total(market.currency()))
        .collect();

    // 3. Every purchase: POPs, then businesses.
    let mut pops_bought = Vec::with_capacity(pops.len());
    for (i, s) in pops.iter_mut().enumerate() {
        let done = settle_purchases(market, &demands[i], &mut s.stockpile).map_err(|error| {
            RefusedTrade {
                trader: RefusedTrader::Pop(i),
                error,
            }
        })?;
        pops_bought.push(done);
    }
    let mut businesses_bought = Vec::with_capacity(businesses.len());
    for (i, b) in businesses.iter_mut().enumerate() {
        let done =
            settle_bids(market, &b.bids, &mut b.stockpile).map_err(|error| RefusedTrade {
                trader: RefusedTrader::Business(i),
                error,
            })?;
        businesses_bought.push(done);
    }

    // 4. POP consumption and the records.
    let mut pop_outcomes = Vec::with_capacity(pops.len());
    for (((s, demand), sold), bought) in
        pops.iter_mut().zip(demands).zip(pops_sold).zip(pops_bought)
    {
        let consumed = consume(s.pop_type, &demand, &mut s.stockpile);
        let carried_over = s.stockpile.total(market.currency());
        let subsistence_index = subsistence_index(&demand);
        let growth_rate = growth_rate(s.pop_type.growth(), subsistence_index);
        pop_outcomes.push(PopOutcome {
            demand,
            sold,
            bought,
            consumed,
            carried_over,
            subsistence_index,
            growth_rate,
            starting_size: s.size,
        });
    }
    let mut business_trades = Vec::with_capacity(businesses.len());
    for (i, b) in businesses.iter().enumerate() {
        let (bought, proportion) = businesses_bought[i].clone();
        business_trades.push(BusinessTrade {
            offered: b.offers.clone(),
            sold: businesses_sold[i].clone(),
            bid: b.bids.clone(),
            bought,
            proportion,
            cash_before: cash_before[i],
            cash_after_sales: cash_after_sales[i],
            cash_after_purchases: b.stockpile.total(market.currency()),
        });
    }
    Ok(HexRound {
        pops: pop_outcomes,
        businesses: business_trades,
    })
}

/// Sells each offered amount, or everything held if that is less. Returns (good, sold) in
/// the offers' order, only amounts > 0.
fn settle_offers(
    market: &Market<'_>,
    offers: &[(GoodId, Quantity)],
    stockpile: &mut Stockpile,
) -> Result<Vec<(GoodId, Quantity)>, TradeError> {
    let mut sold_list = Vec::new();
    for &(good, offer) in offers {
        let held = stockpile.total(good);
        let amount = if offer < held { offer } else { held };
        if amount > Quantity::ZERO {
            market.sell(stockpile, good, amount)?;
            sold_list.push((good, amount));
        }
    }
    Ok(sold_list)
}

/// Values the bids at posted prices; scales every bid by `cash / cost` when the cost exceeds
/// the currency held (1 otherwise); buys each scaled amount, capped at the currency left.
/// Returns ((good, bought) in the bids' order, only amounts > 0; the proportion).
fn settle_bids(
    market: &Market<'_>,
    bids: &[(GoodId, Quantity)],
    stockpile: &mut Stockpile,
) -> Result<(Vec<(GoodId, Quantity)>, f64), TradeError> {
    let mut cost = Quantity::ZERO;
    for &(good, bid) in bids {
        let price = market.price(good).ok_or(TradeError::NotTraded { good })?;
        cost = cost.saturating_add(bid.saturating_mul(price));
    }
    let cash = stockpile.total(market.currency());
    // `cost > cash` implies `cost > 0`, so there is no division by zero; no cash gives 0.
    let proportion = if cost > cash {
        cash.get() / cost.get()
    } else {
        1.0
    };
    let scale = Quantity::new(proportion).expect("a proportion is a valid quantity");
    let mut bought_list = Vec::new();
    for &(good, bid) in bids {
        let wanted = bid.saturating_mul(scale);
        if wanted == Quantity::ZERO {
            continue;
        }
        let price = market.price(good).ok_or(TradeError::NotTraded { good })?;
        let needed = wanted.saturating_mul(price);
        let held = stockpile.total(market.currency());
        let bought = if needed <= held {
            market.buy(stockpile, good, wanted)?;
            wanted
        } else {
            // A rounding sliver: spend what is left.
            market.buy_for(stockpile, good, held)?
        };
        if bought > Quantity::ZERO {
            bought_list.push((good, bought));
        }
    }
    Ok((bought_list, proportion))
}

/// Sells, for every good in the demand except the currency, whatever the stockpile holds
/// above the demanded quantity. Returns (good, amount sold) in good ID order, amounts > 0.
fn settle_sales(
    market: &Market<'_>,
    demand: &Demand,
    stockpile: &mut Stockpile,
) -> Result<Vec<(GoodId, Quantity)>, TradeError> {
    let mut sold_list = Vec::new();
    for g in demand.goods() {
        if g.good == market.currency() {
            continue;
        }
        let held = stockpile.total(g.good);
        if held > g.quantity {
            let (sold, _) = market.sell_down_to(stockpile, g.good, g.quantity)?;
            if sold > Quantity::ZERO {
                sold_list.push((g.good, sold));
            }
        }
    }
    Ok(sold_list)
}

/// Buys, for every good in the demand except the currency, the difference between the
/// demanded quantity and what the stockpile holds, capped at what its currency affords.
/// Returns (good, amount bought) in good ID order, amounts > 0.
fn settle_purchases(
    market: &Market<'_>,
    demand: &Demand,
    stockpile: &mut Stockpile,
) -> Result<Vec<(GoodId, Quantity)>, TradeError> {
    let mut bought_list = Vec::new();
    for g in demand.goods() {
        if g.good == market.currency() {
            continue;
        }
        let held = stockpile.total(g.good);
        if g.quantity <= held {
            continue;
        }
        let diff = g.quantity.checked_sub(held).unwrap_or(Quantity::ZERO);
        let price = market
            .price(g.good)
            .ok_or(TradeError::NotTraded { good: g.good })?;
        let cost = diff.saturating_mul(price);
        let currency_held = stockpile.total(market.currency());
        let bought = if cost <= currency_held {
            market.buy(stockpile, g.good, diff)?;
            diff
        } else {
            market.buy_for(stockpile, g.good, currency_held)?
        };
        if bought > Quantity::ZERO {
            bought_list.push((g.good, bought));
        }
    }
    Ok(bought_list)
}

/// Consumes, for every good in the type's utility, the smaller of what the stockpile holds
/// and what was demanded, soonest-perishing first. Returns (good, amount) in good ID
/// order, amounts > 0.
fn consume(
    pop_type: &PopType,
    demand: &Demand,
    stockpile: &mut Stockpile,
) -> Vec<(GoodId, Quantity)> {
    let mut consumed = Vec::new();
    for u in pop_type.utility() {
        let held = stockpile.total(u.good);
        let wanted = demand.quantity_of(u.good);
        let amount = if held < wanted { held } else { wanted };
        if amount > Quantity::ZERO {
            assert!(
                stockpile.remove(u.good, amount).is_ok(),
                "engine bug: a POP consumed more than its stockpile holds"
            );
            consumed.push((u.good, amount));
        }
    }
    consumed
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::map::{HexCoord, HexId};
    use crate::setting::load_setting;
    use crate::stockpile::PerishesAt;
    use crate::test_support::{close, no_growth, q};
    use crate::turn::Turn;
    use crate::world::World;

    fn trade_world() -> World {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/settings/pops_trade");
        let setting = load_setting(&path).unwrap_or_else(|e| panic!("should load:\n{e}"));
        World::new(setting, &mut crate::decider::DeciderHost::idle())
            .unwrap()
            .0
    }

    fn hex(world: &World, x: i32) -> HexId {
        world.map().id(HexCoord { q: x, r: 0 }).unwrap()
    }

    fn good(world: &World, name: &str) -> GoodId {
        world.rules().goods.id(name).unwrap()
    }

    fn pop_type<'a>(world: &'a World, name: &str) -> &'a PopType {
        let types = &world.rules().pop_types;
        types.get(types.id(name).unwrap())
    }

    /// A stockpile holding `items`. Labor lasts one turn, so a unit held in turn 0 perishes
    /// at the end of turn 0; everything else is durable.
    fn stock(items: &[(GoodId, f64)], world: &World) -> Stockpile {
        let labor = good(world, "labor");
        let mut stockpile = Stockpile::new();
        for (g, amount) in items {
            let at = if *g == labor {
                PerishesAt::EndOfTurn(Turn::ZERO)
            } else {
                PerishesAt::Never
            };
            stockpile.add(*g, q(*amount), at);
        }
        stockpile
    }

    fn named(world: &World, list: &[(GoodId, Quantity)]) -> Vec<(String, f64)> {
        list.iter()
            .map(|(g, a)| (world.rules().goods.name(*g).to_string(), a.get()))
            .collect()
    }

    fn assert_list(world: &World, actual: &[(GoodId, Quantity)], expected: &[(&str, f64)]) {
        let actual = named(world, actual);
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for ((an, aq), (en, eq)) in actual.iter().zip(expected) {
            assert_eq!(an, en, "{actual:?} vs {expected:?}");
            assert!(close(*aq, *eq), "{actual:?} vs {expected:?}");
        }
    }

    fn total_of(stockpile: &Stockpile, g: GoodId) -> f64 {
        stockpile.total(g).get()
    }

    #[test]
    fn a_single_peasant_sells_buys_and_consumes() {
        let world = trade_world();
        let (labor, grain, silver) = (
            good(&world, "labor"),
            good(&world, "grain"),
            good(&world, "silver"),
        );
        let market = world.market(hex(&world, 0));
        let mut subs = vec![Submission {
            pop_type: pop_type(&world, "peasant"),
            size: q(1.0),
            stockpile: stock(&[(labor, 10.0)], &world),
        }];
        let outcomes = hex_round(&market, &mut subs, &mut []).unwrap().pops;
        assert_eq!(outcomes.len(), 1);
        let o = &outcomes[0];
        assert_list(&world, o.sold(), &[("labor", 7.2)]);
        assert_list(&world, o.bought(), &[("grain", 3.6)]);
        assert_list(&world, o.consumed(), &[("labor", 2.8), ("grain", 3.6)]);
        assert!(close(o.demand().income().get(), 10.0));
        assert!(close(o.sold_of(labor).get(), 7.2));
        assert!(close(o.bought_of(grain).get(), 3.6));
        assert!(close(o.consumed_of(grain).get(), 3.6));
        assert_eq!(o.sold_of(grain), Quantity::ZERO);
        let stockpile = &subs[0].stockpile;
        assert_eq!(stockpile.total(labor), Quantity::ZERO);
        assert_eq!(stockpile.total(grain), Quantity::ZERO);
        assert!(total_of(stockpile, silver) < 1e-9);
        assert_eq!(o.carried_over(), stockpile.total(silver));
    }

    #[test]
    fn sales_on_the_hex_settle_before_purchases() {
        let world = trade_world();
        let (labor, copper) = (good(&world, "labor"), good(&world, "copper"));
        let market = world.market(hex(&world, 0));
        let submission = |name: &str, size: f64| Submission {
            pop_type: pop_type(&world, name),
            size: q(size),
            stockpile: stock(&[(labor, 10.0 * size)], &world),
        };
        let mut together = vec![submission("peasant", 2.0), submission("artisan", 1.0)];
        let outcomes = hex_round(&market, &mut together, &mut []).unwrap().pops;
        assert_eq!(outcomes.len(), 2);
        for (i, name, size) in [(0, "peasant", 2.0), (1, "artisan", 1.0)] {
            let mut alone = vec![submission(name, size)];
            let alone_outcomes = hex_round(&market, &mut alone, &mut []).unwrap().pops;
            assert_eq!(outcomes[i], alone_outcomes[0], "{name}");
            assert_eq!(together[i].stockpile, alone[0].stockpile, "{name}");
        }
        // Submission order: the first outcome is the peasant (no copper), the second the
        // artisan.
        assert_eq!(outcomes[0].bought_of(copper), Quantity::ZERO);
        assert!(outcomes[1].bought_of(copper) > Quantity::ZERO);
    }

    #[test]
    fn a_hex_without_a_market_trades_nothing() {
        let world = trade_world();
        let (labor, grain) = (good(&world, "labor"), good(&world, "grain"));
        let market = world.market(hex(&world, 2));
        let mut subs = vec![Submission {
            pop_type: pop_type(&world, "peasant"),
            size: q(1.0),
            stockpile: stock(&[(labor, 10.0)], &world),
        }];
        let outcomes = hex_round(&market, &mut subs, &mut []).unwrap().pops;
        let o = &outcomes[0];
        assert!(o.sold().is_empty());
        assert!(o.bought().is_empty());
        assert_list(&world, o.consumed(), &[("labor", 10.0)]);
        let d = o.demand().get(grain).unwrap();
        assert_eq!(d.quantity, Quantity::ZERO);
        assert!(d.below_subsistence());
        assert!(subs[0].stockpile.is_empty());
    }

    #[test]
    fn an_empty_slice_gives_no_outcomes() {
        let world = trade_world();
        let market = world.market(hex(&world, 0));
        let round = hex_round(&market, &mut [], &mut []).unwrap();
        assert!(round.pops.is_empty());
        assert!(round.businesses.is_empty());
    }

    #[test]
    fn a_zero_demand_sells_everything() {
        let world = trade_world();
        let (labor, copper) = (good(&world, "labor"), good(&world, "copper"));
        let market = world.market(hex(&world, 0));
        let mut subs = vec![Submission {
            pop_type: pop_type(&world, "peasant"),
            size: q(1.0),
            stockpile: stock(&[(labor, 10.0), (copper, 1.0)], &world),
        }];
        let outcomes = hex_round(&market, &mut subs, &mut []).unwrap().pops;
        assert_eq!(outcomes[0].sold_of(copper), q(1.0));
        assert_eq!(subs[0].stockpile.total(copper), Quantity::ZERO);
    }

    /// A type that wants only grain, spending everything on it.
    fn grain_only(world: &World) -> PopType {
        PopType::new(
            vec![],
            vec![(good(world, "grain"), Quantity::ZERO, 1.0)],
            no_growth(),
        )
    }

    #[test]
    fn the_purchase_cap_reduces_rather_than_refuses() {
        let world = trade_world();
        let (grain, silver) = (good(&world, "grain"), good(&world, "silver"));
        let market = world.market(hex(&world, 0));
        let pop_type = grain_only(&world);
        let d = demand(&pop_type, q(1.0), [(silver, q(6.0))], &market);
        assert_eq!(d.quantity_of(grain), q(3.0));
        let mut stockpile = stock(&[(silver, 6.0 - 1e-9)], &world);
        let bought = settle_purchases(&market, &d, &mut stockpile).unwrap();
        assert_list(&world, &bought, &[("grain", 2.9999999995)]);
        assert_eq!(stockpile.total(silver), Quantity::ZERO);
    }

    #[test]
    fn a_rounding_surplus_stays_as_currency() {
        let world = trade_world();
        let (grain, silver) = (good(&world, "grain"), good(&world, "silver"));
        let market = world.market(hex(&world, 0));
        let pop_type = grain_only(&world);
        let d = demand(&pop_type, q(1.0), [(silver, q(6.0))], &market);
        assert_eq!(d.quantity_of(grain), q(3.0));
        let mut stockpile = stock(&[(silver, 6.0 + 1e-9)], &world);
        let bought = settle_purchases(&market, &d, &mut stockpile).unwrap();
        assert_list(&world, &bought, &[("grain", 3.0)]);
        assert!((total_of(&stockpile, silver) - 1e-9).abs() < 1e-12);
    }

    #[test]
    fn a_pop_with_no_currency_buys_nothing() {
        let world = trade_world();
        let labor = good(&world, "labor");
        let market = world.market(hex(&world, 0));
        let d = demand(
            pop_type(&world, "peasant"),
            q(1.0),
            [(labor, q(10.0))],
            &market,
        );
        let mut stockpile = Stockpile::new();
        assert_eq!(settle_purchases(&market, &d, &mut stockpile), Ok(vec![]));
        assert!(stockpile.is_empty());
    }

    /// The peasant demand from labor 10 at hex (0, 0): labor 2.8, grain 3.6.
    fn peasant_demand(world: &World) -> Demand {
        let market = world.market(hex(world, 0));
        demand(
            pop_type(world, "peasant"),
            q(1.0),
            [(good(world, "labor"), q(10.0))],
            &market,
        )
    }

    #[test]
    fn consumption_takes_only_utility_goods_up_to_the_demand() {
        let world = trade_world();
        let (labor, grain, copper) = (
            good(&world, "labor"),
            good(&world, "grain"),
            good(&world, "copper"),
        );
        let d = peasant_demand(&world);
        let mut stockpile = stock(&[(labor, 2.8), (grain, 5.0), (copper, 1.0)], &world);
        let consumed = consume(pop_type(&world, "peasant"), &d, &mut stockpile);
        assert_list(&world, &consumed, &[("labor", 2.8), ("grain", 3.6)]);
        assert_eq!(stockpile.total(labor), Quantity::ZERO);
        assert!(close(total_of(&stockpile, grain), 1.4));
        assert_eq!(stockpile.total(copper), q(1.0));
    }

    #[test]
    fn consumption_takes_what_is_held_when_short() {
        let world = trade_world();
        let grain = good(&world, "grain");
        let d = peasant_demand(&world);
        let mut stockpile = stock(&[(grain, 1.0)], &world);
        let consumed = consume(pop_type(&world, "peasant"), &d, &mut stockpile);
        assert_list(&world, &consumed, &[("grain", 1.0)]);
        assert!(stockpile.is_empty());
    }

    #[test]
    fn the_currency_is_never_traded() {
        let world = trade_world();
        let (labor, silver) = (good(&world, "labor"), good(&world, "silver"));
        let market = world.market(hex(&world, 3));
        let pop_type = PopType::new(
            vec![],
            vec![(labor, Quantity::ZERO, 0.5), (silver, q(1.0), 0.5)],
            no_growth(),
        );
        let mut subs = vec![Submission {
            pop_type: &pop_type,
            size: q(1.0),
            stockpile: stock(&[(labor, 10.0)], &world),
        }];
        let outcomes = hex_round(&market, &mut subs, &mut []).unwrap().pops;
        let o = &outcomes[0];
        assert_list(&world, o.sold(), &[("labor", 5.5)]);
        assert!(o.bought().is_empty());
        assert_list(&world, o.consumed(), &[("labor", 4.5), ("silver", 5.5)]);
        assert!(total_of(&subs[0].stockpile, silver) < 1e-9);
    }

    #[test]
    fn settle_sales_reports_a_refusal() {
        let world = trade_world();
        let (labor, copper, silver) = (
            good(&world, "labor"),
            good(&world, "copper"),
            good(&world, "silver"),
        );
        let priced = world.market(hex(&world, 0));
        let bare = world.market(hex(&world, 3));
        let holdings = [(labor, 10.0), (copper, 2.0)];
        let d = demand(
            pop_type(&world, "peasant"),
            q(1.0),
            holdings.iter().map(|(g, a)| (*g, q(*a))),
            &priced,
        );
        let mut stockpile = stock(&holdings, &world);
        assert_eq!(
            settle_sales(&bare, &d, &mut stockpile),
            Err(TradeError::NotTraded { good: copper })
        );
        // The labor sale before the refusal went through: the income is 20, so 4.8 labor
        // is kept and 5.2 sold.
        assert!(close(total_of(&stockpile, labor), 4.8));
        assert!(close(total_of(&stockpile, silver), 5.2));
        assert_eq!(stockpile.total(copper), q(2.0));
    }

    /// A business submission with `stockpile` held (durable goods; see `stock`).
    fn business(
        offers: &[(GoodId, f64)],
        bids: &[(GoodId, f64)],
        stockpile: Stockpile,
    ) -> BusinessSubmission {
        let list = |items: &[(GoodId, f64)]| items.iter().map(|(g, a)| (*g, q(*a))).collect();
        BusinessSubmission {
            offers: list(offers),
            bids: list(bids),
            stockpile,
        }
    }

    /// Runs the round for the businesses alone on hex `x`.
    fn business_round(
        world: &World,
        x: i32,
        businesses: &mut [BusinessSubmission],
    ) -> Result<Vec<BusinessTrade>, RefusedTrade> {
        let market = world.market(hex(world, x));
        hex_round(&market, &mut [], businesses).map(|round| round.businesses)
    }

    #[test]
    fn a_business_sells_its_offers_and_buys_its_bids() {
        let world = trade_world();
        let (copper, grain, silver) = (
            good(&world, "copper"),
            good(&world, "grain"),
            good(&world, "silver"),
        );
        let mut subs = vec![business(
            &[(copper, 2.0)],
            &[(grain, 3.0)],
            stock(&[(silver, 10.0), (copper, 2.0)], &world),
        )];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        let t = &trades[0];
        assert_list(&world, t.offered(), &[("copper", 2.0)]);
        assert_list(&world, t.sold(), &[("copper", 2.0)]);
        assert_list(&world, t.bid(), &[("grain", 3.0)]);
        assert_list(&world, t.bought(), &[("grain", 3.0)]);
        assert!(close(t.sold_of(copper).get(), 2.0));
        assert!(close(t.bought_of(grain).get(), 3.0));
        assert!(close(t.offered_of(copper).get(), 2.0));
        assert!(close(t.bid_of(grain).get(), 3.0));
        assert_eq!(t.proportion(), 1.0);
        assert!(close(t.cash_before().get(), 10.0));
        assert!(close(t.cash_after_sales().get(), 20.0));
        assert!(close(t.cash_after_purchases().get(), 14.0));
        let stockpile = &subs[0].stockpile;
        assert_eq!(stockpile.total(copper), Quantity::ZERO);
        assert!(close(total_of(stockpile, grain), 3.0));
        assert!(close(total_of(stockpile, silver), 14.0));
        // Bought in turn 0, grain lasts 3 turns: it perishes at the end of turn 2.
        let groups: Vec<_> = stockpile.groups(grain).collect();
        assert_eq!(groups, vec![(PerishesAt::EndOfTurn(Turn::new(2)), q(3.0))]);
    }

    #[test]
    fn an_offer_above_holdings_sells_what_is_held() {
        let world = trade_world();
        let (labor, copper, silver) = (
            good(&world, "labor"),
            good(&world, "copper"),
            good(&world, "silver"),
        );
        let mut subs = vec![business(
            &[(labor, 1.0), (copper, 5.0)],
            &[],
            stock(&[(copper, 2.0)], &world),
        )];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        let t = &trades[0];
        assert_list(&world, t.sold(), &[("copper", 2.0)]);
        assert!(close(t.offered_of(copper).get(), 5.0));
        assert_eq!(t.sold_of(labor), Quantity::ZERO);
        assert!(t.sold().iter().all(|(g, _)| *g != labor));
        assert!(close(total_of(&subs[0].stockpile, silver), 10.0));
    }

    #[test]
    fn bids_beyond_cash_are_scaled_by_one_proportion() {
        let world = trade_world();
        let (grain, copper, silver) = (
            good(&world, "grain"),
            good(&world, "copper"),
            good(&world, "silver"),
        );
        let mut subs = vec![business(
            &[],
            &[(grain, 10.0), (copper, 6.0)],
            stock(&[(silver, 10.0)], &world),
        )];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        let t = &trades[0];
        assert!(close(t.proportion(), 0.2));
        assert_list(&world, t.bought(), &[("grain", 2.0), ("copper", 1.2)]);
        assert_list(&world, t.bid(), &[("grain", 10.0), ("copper", 6.0)]);
        assert!(total_of(&subs[0].stockpile, silver) < 1e-9);
        assert!(close(t.cash_before().get(), 10.0));
        assert!(close(t.cash_after_sales().get(), 10.0));
        assert!(t.cash_after_purchases().get() < 1e-9);
    }

    #[test]
    fn a_business_with_no_currency_buys_nothing() {
        let world = trade_world();
        let grain = good(&world, "grain");
        let mut subs = vec![business(&[], &[(grain, 1.0)], Stockpile::new())];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        assert!(trades[0].bought().is_empty());
        assert_eq!(trades[0].proportion(), 0.0);
        assert!(subs[0].stockpile.is_empty());
    }

    #[test]
    fn a_rounding_sliver_caps_the_last_purchase() {
        let world = trade_world();
        let (grain, silver) = (good(&world, "grain"), good(&world, "silver"));
        let mut subs = vec![business(
            &[],
            &[(grain, 3.0)],
            stock(&[(silver, 6.0 - 1e-9)], &world),
        )];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        let t = &trades[0];
        assert_eq!(t.proportion(), (6.0 - 1e-9) / 6.0);
        assert!(close(t.bought_of(grain).get(), 3.0));
        assert!(total_of(&subs[0].stockpile, silver) < 1e-9);
        assert!(close(total_of(&subs[0].stockpile, grain), 3.0));
    }

    #[test]
    fn empty_submissions_trade_nothing() {
        let world = trade_world();
        let silver = good(&world, "silver");
        let mut subs = vec![business(&[], &[], stock(&[(silver, 7.0)], &world))];
        let trades = business_round(&world, 0, &mut subs).unwrap();
        let t = &trades[0];
        assert!(t.offered().is_empty());
        assert!(t.sold().is_empty());
        assert!(t.bid().is_empty());
        assert!(t.bought().is_empty());
        assert_eq!(t.proportion(), 1.0);
        assert_eq!(t.cash_before(), q(7.0));
        assert_eq!(t.cash_after_sales(), q(7.0));
        assert_eq!(t.cash_after_purchases(), q(7.0));
    }

    #[test]
    fn pops_and_businesses_together_equal_each_alone() {
        let world = trade_world();
        let (labor, copper, grain, silver) = (
            good(&world, "labor"),
            good(&world, "copper"),
            good(&world, "grain"),
            good(&world, "silver"),
        );
        let market = world.market(hex(&world, 0));
        let peasant = || Submission {
            pop_type: pop_type(&world, "peasant"),
            size: q(1.0),
            stockpile: stock(&[(labor, 10.0)], &world),
        };
        let shop = || {
            business(
                &[(copper, 2.0)],
                &[(grain, 3.0)],
                stock(&[(silver, 10.0), (copper, 2.0)], &world),
            )
        };
        let mut pops = vec![peasant()];
        let mut shops = vec![shop()];
        let both = hex_round(&market, &mut pops, &mut shops).unwrap();

        let mut pops_alone = vec![peasant()];
        let pop_round = hex_round(&market, &mut pops_alone, &mut []).unwrap();
        let mut shops_alone = vec![shop()];
        let shop_round = hex_round(&market, &mut [], &mut shops_alone).unwrap();

        assert_eq!(both.pops, pop_round.pops);
        assert_eq!(pops[0].stockpile, pops_alone[0].stockpile);
        assert_eq!(both.businesses, shop_round.businesses);
        assert_eq!(shops[0].stockpile, shops_alone[0].stockpile);
    }

    #[test]
    fn a_refused_business_trade_names_the_business() {
        let world = trade_world();
        let (grain, silver) = (good(&world, "grain"), good(&world, "silver"));
        let mut subs = vec![business(
            &[],
            &[(grain, 1.0)],
            stock(&[(silver, 10.0)], &world),
        )];
        let market = world.market(hex(&world, 2));
        let refused = hex_round(&market, &mut [], &mut subs).err().unwrap();
        assert_eq!(
            refused,
            RefusedTrade {
                trader: RefusedTrader::Business(0),
                error: TradeError::NotTraded { good: grain },
            }
        );
    }
}
