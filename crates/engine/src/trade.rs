//! The trade round: the "actors act" part of the turn for POPs.
//!
//! Each POP submits its type's utility parameters, its size and its holdings to its hex's
//! market. The market posts its prices (fixed, this Stage), computes every submission's
//! demand with [`demand`](crate::demand), and settles every POP's sales before any POP's
//! purchases (so that in the live market every purchase sees the income the sales actually
//! produced). Then each POP consumes its utility goods up to what it demanded. The currency
//! is never traded: proceeds add it, purchases spend it, and what is left is its demanded
//! quantity up to rounding. Hexes are handled by the World in hex ID order; within a hex
//! POPs go in POP ID order and goods in good ID order. Leftover currency stays in the
//! stockpile. A refused trade is an engine bug. The record also carries the POP's
//! subsistence index and growth rate, computed by the [`growth`](crate::growth) module from
//! the demand and the type, and the size the turn started with; the World applies the rate
//! after every hex has traded. Stage 05 replaces the inside of the round (adjusted prices,
//! rationing); the submission, the order, consumption and the outcome stay. Knows nothing
//! about the World or files.

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

/// The exchange refused a trade during the round: an engine bug, since the demand and the
/// purchase cap make every trade acceptable. The stockpiles are left as they were at the
/// refusal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RefusedTrade {
    /// Index of the submission in the slice passed to `hex_round`.
    pub submission: usize,
    pub error: TradeError,
}

/// Runs one hex's trade round and consumption for `submissions` (the hex's POPs in POP ID
/// order). Returns one outcome per submission, in the same order.
pub(crate) fn hex_round(
    market: &Market<'_>,
    submissions: &mut [Submission<'_>],
) -> Result<Vec<PopOutcome>, RefusedTrade> {
    // 1. Demand.
    let demands: Vec<Demand> = submissions
        .iter()
        .map(|s| demand(s.pop_type, s.size, s.stockpile.iter(), market))
        .collect();

    // 2. Every POP's sales, before any purchase.
    let mut sold = Vec::with_capacity(submissions.len());
    for (i, s) in submissions.iter_mut().enumerate() {
        let done =
            settle_sales(market, &demands[i], &mut s.stockpile).map_err(|error| RefusedTrade {
                submission: i,
                error,
            })?;
        sold.push(done);
    }

    // 3. Every POP's purchases.
    let mut bought = Vec::with_capacity(submissions.len());
    for (i, s) in submissions.iter_mut().enumerate() {
        let done = settle_purchases(market, &demands[i], &mut s.stockpile).map_err(|error| {
            RefusedTrade {
                submission: i,
                error,
            }
        })?;
        bought.push(done);
    }

    // 4. Consumption and the record.
    let mut outcomes = Vec::with_capacity(submissions.len());
    for (((s, demand), sold), bought) in submissions.iter_mut().zip(demands).zip(sold).zip(bought) {
        let consumed = consume(s.pop_type, &demand, &mut s.stockpile);
        let carried_over = s.stockpile.total(market.currency());
        let subsistence_index = subsistence_index(&demand);
        let growth_rate = growth_rate(s.pop_type.growth(), subsistence_index);
        outcomes.push(PopOutcome {
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
    Ok(outcomes)
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
        World::new(setting).0
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
        let outcomes = hex_round(&market, &mut subs).unwrap();
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
        let outcomes = hex_round(&market, &mut together).unwrap();
        assert_eq!(outcomes.len(), 2);
        for (i, name, size) in [(0, "peasant", 2.0), (1, "artisan", 1.0)] {
            let mut alone = vec![submission(name, size)];
            let alone_outcomes = hex_round(&market, &mut alone).unwrap();
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
        let outcomes = hex_round(&market, &mut subs).unwrap();
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
        assert_eq!(hex_round(&market, &mut []), Ok(vec![]));
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
        let outcomes = hex_round(&market, &mut subs).unwrap();
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
        let outcomes = hex_round(&market, &mut subs).unwrap();
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
}
