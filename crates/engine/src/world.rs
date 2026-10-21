//! The running world and its turn clock.
//!
//! - A [`World`] is a running world, built from a [`LoadedSetting`]. The rules (goods,
//!   resource types, POP types) never change. The world holds what does: the clock, the
//!   hexes, the deposits (each with its owner, if any), the POPs (each with its current
//!   size, grown every turn), the businesses, each hex's currency, each hex's posted
//!   prices, one stockpile of unowned goods per hex, one stockpile per POP (empty when the
//!   world is created; a POP's first endowment arrives at the start of turn 0), one
//!   stockpile per business (filled from its starting inventory when the world is
//!   created), each POP's latest trade outcome (what it demanded, sold, bought and
//!   consumed), each business's answer at the current turn's decision point, and each
//!   business's outcome (what its trade round did and what its factories did). Hexes,
//!   currencies, deposits and businesses don't change yet, but they are world state
//!   because later Stages change them; a POP's size changes every turn as it grows, and a
//!   factory's condition changes every turn as it wears and is maintained. The
//!   posted prices don't change yet either; [`World::market`] hands out each hex's
//!   market view, the seam Stage 04 replaces.
//! - The world also holds the factories (each on its owner's hex, running one technology
//!   and holding a share of its capital bundle) as state. Their condition changes every
//!   turn: each business's factories are run, worn and maintained by the
//!   [`production`](crate::production) mechanics right after its hex's trade round, and the
//!   conditions are written back; their technology changes in later Stages.
//! - A business's view of its circumstances at the current turn is taken with
//!   [`World::business_view`], which reads and changes nothing (the decision point that
//!   serves it is `decide`).
//! - A turn has three parts:
//!   1. start of turn: every deposit emits its type's `output` of its good into its
//!      owner's stockpile if it has an owner and its hex's stockpile otherwise (the hex
//!      stockpile stays the home of unowned goods), then every POP receives its type's
//!      endowment times its size into its own stockpile;
//!   2. actors act: the decision point, then on every hex the trade round, consumption
//!      and operation, then growth. At the decision point every business's view goes to the
//!      decider its setting entry names, through the host the caller passes to `new` and
//!      `step` (see the [`decider::host`](crate::decider::host) module), and its answer is
//!      stored. Then on every hex each POP submits its parameters and holdings and each
//!      business the offers and bids its valid decision resolves to (none, for a business
//!      that decided nothing or was refused); the market computes the POPs' demand and
//!      settles every sale before any purchase, and each POP consumes (see the
//!      [`trade`](crate::trade) module). A business's bids are scaled by one proportion when
//!      they cost more than the currency it holds, and its fractions reach the factory
//!      mechanics untouched. Right after the hex's round each of its businesses operates
//!      its factories with the fractions its valid decision gives (0 and 0 for a factory it
//!      doesn't mention, and for every factory of a business that decided nothing or was
//!      refused: they only wear), and the conditions are written back. Then, once every hex
//!      has traded, every POP grows or shrinks by the rate its outcome records (see the
//!      [`growth`](crate::growth) module). Later Stages add the live market here;
//!   3. end of turn: whatever perishes at the end of this turn is removed, from the hex
//!      stockpiles, then the POP stockpiles and then the business stockpiles.
//! - [`World::new`] takes the host (and fails for a decider name the host doesn't hold),
//!   puts the world at turn 0, fills every business's stockpile from its starting
//!   inventory (each good perishing as if added in turn 0), and runs turn 0's start, turn
//!   0's decision point, turn 0's trade round, operation and growth. [`World::step`] ends
//!   the current turn, advances the clock, and starts the next one, decision point, trade
//!   round, operation and growth included. The world keeps each POP's outcome of the
//!   current turn's round ([`World::pop_outcome`]), each business's answer
//!   ([`World::business_answer`]) and each business's outcome
//!   ([`World::business_outcome`]). Between calls the world is always after part 2, where
//!   every business has answered, traded and operated, and every POP has traded, consumed
//!   and grown this turn, and nothing has perished yet. So a world that has just been
//!   created is already past turn 0's operation: under the idle host its factories have worn
//!   one slice.
//! - Deposits emit in deposit ID order, POPs are endowed in POP ID order, trade runs on
//!   hexes in hex ID order, POPs in ID order within a hex and then businesses in ID order,
//!   and goods in ID order within each, with every sale on the hex before every purchase
//!   (and POPs before businesses within each pass) and then POP consumption, then each
//!   business on the hex operates its factories in factory ID order, POPs grow in ID
//!   order, perishing goes through hexes, then POPs, then businesses in business ID order,
//!   deciders are taken in name order and businesses in ID order within a decider, and
//!   stockpiles iterate in a fixed order, so the same setting stepped the same number
//!   of times always gives the same world and reports.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::businesses::{BusinessId, BusinessKind, Businesses};
use crate::currency::HexCurrencies;
use crate::decider::{
    BusinessAnswer, BusinessView, DeciderHost, DecisionCounts, ReportedFailure, UnknownDecider,
    view,
};
use crate::factories::{Factories, FactoryId};
use crate::goods::{GoodId, Goods};
use crate::growth::grown_size;
use crate::id::IdVec;
use crate::map::{HexId, HexKind, HexMap};
use crate::market::{HexPrices, Market};
use crate::pops::{PopId, PopKind, Pops};
use crate::production::{self, FactoryRecord, Operation};
use crate::quantity::Quantity;
use crate::resources::Deposits;
use crate::setting::{LoadedSetting, Manifest, Rules};
use crate::stockpile::{PerishesAt, Stockpile};
use crate::trade::{
    BusinessSubmission, BusinessTrade, PopOutcome, RefusedTrade, RefusedTrader, Submission,
    hex_round,
};
use crate::turn::Turn;

/// What happened in one step (or in creating the world).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnReport {
    /// The turn that started.
    pub turn: Turn,
    /// Total removed per good at the end of the previous turn from every hex, POP and
    /// business stockpile, in good ID order, only amounts > 0. Always empty for turn 0.
    pub perished: Vec<(GoodId, Quantity)>,
    /// Total emitted by deposits per good at the start of `turn` (into hex or business
    /// stockpiles), in good ID order, only amounts > 0.
    pub emitted: Vec<(GoodId, Quantity)>,
    /// Total received by POPs as endowment per good at the start of `turn`, in good ID
    /// order, only amounts > 0.
    pub endowed: Vec<(GoodId, Quantity)>,
    /// Total sold by POPs per good in the trade round of `turn`, on every hex, in good ID
    /// order, only amounts > 0.
    pub sold: Vec<(GoodId, Quantity)>,
    /// Total bought by POPs per good in the trade round of `turn`, likewise.
    pub bought: Vec<(GoodId, Quantity)>,
    /// Total consumed by POPs per good after the trade round of `turn`, likewise.
    pub consumed: Vec<(GoodId, Quantity)>,
    /// The total size of every POP after growth in `turn`. Zero for a world with no POPs.
    pub population: Quantity,
    /// Total sold by businesses per good in the round of `turn`, in good ID order, only
    /// amounts > 0.
    pub business_sold: Vec<(GoodId, Quantity)>,
    /// Total bought by businesses per good in the round of `turn`, likewise.
    pub business_bought: Vec<(GoodId, Quantity)>,
    /// Total output made by factories per good in `turn`, likewise.
    pub produced: Vec<(GoodId, Quantity)>,
    /// Total inputs and maintenance goods consumed by factories per good in `turn`,
    /// likewise.
    pub used: Vec<(GoodId, Quantity)>,
    /// How the businesses answered at the decision point of `turn`.
    pub decisions: DecisionCounts,
    /// Every decider that failed at the decision point of `turn`, in decider name order.
    pub decider_failures: Vec<ReportedFailure>,
}

/// What the current turn did for one business, after its hex's round and its operation.
#[derive(Debug, Clone, PartialEq)]
pub struct BusinessOutcome {
    /// What the trade round did.
    trade: BusinessTrade,
    /// One record per factory the business owns, in factory ID order.
    factories: Vec<(FactoryId, FactoryRecord)>,
}

impl BusinessOutcome {
    /// What the trade round did for the business: what it offered, sold, bid and bought,
    /// the cash cap's proportion and its cash at each point.
    pub fn trade(&self) -> &BusinessTrade {
        &self.trade
    }

    /// What the mechanics did for each factory the business owns, in factory ID order.
    pub fn factories(&self) -> &[(FactoryId, FactoryRecord)] {
        &self.factories
    }

    /// What the mechanics did for `id`; None if the business doesn't own it.
    pub fn factory(&self, id: FactoryId) -> Option<&FactoryRecord> {
        self.factories
            .iter()
            .find(|(factory, _)| *factory == id)
            .map(|(_, record)| record)
    }
}

/// Why a step was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepError {
    /// The clock is at `Turn::MAX` and can't advance. The world is unchanged.
    ClockAtMaximum,
    /// A business names a decider the host doesn't hold. The world is unchanged.
    UnknownDecider(UnknownDecider),
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepError::ClockAtMaximum => write!(
                f,
                "the clock is at its maximum turn ({}) and can't advance",
                Turn::MAX
            ),
            StepError::UnknownDecider(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StepError {}

/// A running world.
#[derive(Debug)]
pub struct World {
    manifest: Manifest,
    rules: Rules,
    map: HexMap,
    deposits: Deposits,
    turn: Turn,
    /// One per hex, by `HexId`. Nothing changes it yet.
    hex_currencies: HexCurrencies,
    /// One per hex, by `HexId`. Nothing changes it yet.
    hex_prices: HexPrices,
    /// One per hex, by `HexId`.
    hex_stockpiles: IdVec<HexKind, Stockpile>,
    /// The POPs. Their sizes change in `grow`.
    pops: Pops,
    /// One per POP, by `PopId`. Empty when the world is created.
    pop_stockpiles: IdVec<PopKind, Stockpile>,
    /// One per POP, by `PopId`: the outcome of the current turn's trade round. Rebuilt
    /// every turn.
    pop_outcomes: IdVec<PopKind, PopOutcome>,
    /// The businesses. Nothing changes them yet.
    businesses: Businesses,
    /// One per business, by `BusinessId`. Holds the starting inventory when the world is
    /// created.
    business_stockpiles: IdVec<BusinessKind, Stockpile>,
    /// The factories. Their condition changes every turn (written back after operation);
    /// their technology is state because a later Stage changes it.
    factories: Factories,
    /// One per business, by `BusinessId`: its answer at the current turn's decision point.
    /// Rebuilt every turn.
    business_answers: IdVec<BusinessKind, BusinessAnswer>,
    /// One per business, by `BusinessId`: the outcome of the current turn's trade round and
    /// operation. Rebuilt every turn; empty only while the world is being created.
    business_outcomes: IdVec<BusinessKind, BusinessOutcome>,
}

/// The trade round's and the operation's totals per good, each in good ID order.
struct TradeTotals {
    sold: Vec<(GoodId, Quantity)>,
    bought: Vec<(GoodId, Quantity)>,
    consumed: Vec<(GoodId, Quantity)>,
    business_sold: Vec<(GoodId, Quantity)>,
    business_bought: Vec<(GoodId, Quantity)>,
    produced: Vec<(GoodId, Quantity)>,
    used: Vec<(GoodId, Quantity)>,
}

impl World {
    /// Builds a world from `setting` at turn 0 and runs turn 0's start, decision point,
    /// trade round, operation and growth, with `host` answering for the businesses. Fails, building
    /// nothing, if a business names a decider the host doesn't hold. Returns the world and
    /// turn 0's report.
    pub fn new(
        setting: LoadedSetting,
        host: &mut DeciderHost,
    ) -> Result<(World, TurnReport), UnknownDecider> {
        let LoadedSetting {
            manifest,
            rules,
            start,
        } = setting;
        host.check(&start.businesses)?;
        let hex_stockpiles = IdVec::from_vec(vec![Stockpile::new(); start.map.len()]);
        let pop_stockpiles = IdVec::from_vec(vec![Stockpile::new(); start.pops.len()]);
        let business_count = start.businesses.len();
        let business_stockpiles = IdVec::from_vec(
            start
                .businesses
                .iter()
                .map(|(_, _, business)| {
                    let mut stockpile = Stockpile::new();
                    for (good, amount) in business.starting_inventory.iter() {
                        let at = PerishesAt::for_good(rules.goods.get(good).perishes, Turn::ZERO);
                        stockpile.add(good, amount, at);
                    }
                    stockpile
                })
                .collect(),
        );
        let mut world = World {
            manifest,
            rules,
            map: start.map,
            deposits: start.deposits,
            turn: Turn::ZERO,
            hex_currencies: start.currencies,
            hex_prices: start.prices,
            hex_stockpiles,
            pops: start.pops,
            pop_stockpiles,
            pop_outcomes: IdVec::from_vec(Vec::new()),
            businesses: start.businesses,
            business_stockpiles,
            factories: start.factories,
            business_answers: IdVec::from_vec(vec![BusinessAnswer::Nothing; business_count]),
            business_outcomes: IdVec::from_vec(Vec::new()),
        };
        let report = world.run_turn(Vec::new(), host);
        Ok((world, report))
    }

    /// Ends the current turn, advances the clock, starts the next turn with `host`.
    /// At `Turn::MAX` returns `Err(StepError::ClockAtMaximum)`; if a business names a
    /// decider `host` doesn't hold, `Err(StepError::UnknownDecider)`. Either way nothing
    /// changes.
    pub fn step(&mut self, host: &mut DeciderHost) -> Result<TurnReport, StepError> {
        let next = self.turn.next().ok_or(StepError::ClockAtMaximum)?;
        host.check(&self.businesses)
            .map_err(StepError::UnknownDecider)?;
        let perished = self.perish();
        self.turn = next;
        Ok(self.run_turn(perished, host))
    }

    /// The current turn.
    pub fn turn(&self) -> Turn {
        self.turn
    }

    /// The setting's manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The rules (never change).
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    /// The hexes.
    pub fn map(&self) -> &HexMap {
        &self.map
    }

    /// The deposits.
    pub fn deposits(&self) -> &Deposits {
        &self.deposits
    }

    /// Every hex's currency.
    pub fn hex_currencies(&self) -> &HexCurrencies {
        &self.hex_currencies
    }

    /// The currency good of `hex`.
    ///
    /// # Panics
    /// Panics if `hex` isn't from this world's map.
    pub fn hex_currency(&self, hex: HexId) -> GoodId {
        self.hex_currencies.get(hex)
    }

    /// Every hex's posted prices.
    pub fn hex_prices(&self) -> &HexPrices {
        &self.hex_prices
    }

    /// The market of `hex`: its prices in the hex's currency and the exchange, at the
    /// current turn.
    ///
    /// # Panics
    /// Panics if `hex` isn't from this world's map.
    pub fn market(&self, hex: HexId) -> Market<'_> {
        Market::new(
            hex,
            &self.rules.goods,
            self.hex_currencies.get(hex),
            self.hex_prices.list(hex),
            self.turn,
        )
    }

    /// The stockpile of unowned goods on `hex`.
    ///
    /// # Panics
    /// Panics if `hex` isn't from this world's map.
    pub fn hex_stockpile(&self, hex: HexId) -> &Stockpile {
        self.hex_stockpiles.get(hex)
    }

    /// The POPs.
    pub fn pops(&self) -> &Pops {
        &self.pops
    }

    /// The stockpile of `pop`: the goods it owns.
    ///
    /// # Panics
    /// Panics if `pop` isn't from this world's POPs.
    pub fn pop_stockpile(&self, pop: PopId) -> &Stockpile {
        self.pop_stockpiles.get(pop)
    }

    /// The businesses.
    pub fn businesses(&self) -> &Businesses {
        &self.businesses
    }

    /// The factories.
    pub fn factories(&self) -> &Factories {
        &self.factories
    }

    /// The stockpile of `business`: the goods it owns.
    ///
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_stockpile(&self, business: BusinessId) -> &Stockpile {
        self.business_stockpiles.get(business)
    }

    /// `business`'s view of its circumstances at the current turn: its hex's currency and
    /// posted prices, its stockpile, its cash, its factories, their technologies and its
    /// deposits. Reads and changes nothing.
    ///
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_view(&self, business: BusinessId) -> BusinessView {
        view::build(self, business)
    }

    /// `business`'s answer at the current turn's decision point.
    ///
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_answer(&self, business: BusinessId) -> &BusinessAnswer {
        self.business_answers.get(business)
    }

    /// The outcome of the current turn for `business`: what its trade round did and what
    /// its factories did.
    ///
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_outcome(&self, business: BusinessId) -> &BusinessOutcome {
        self.business_outcomes.get(business)
    }

    /// The outcome of the current turn's trade round for `pop`: its demand, what it sold,
    /// bought and consumed, the currency it carried over, its subsistence index, its
    /// growth rate and its starting size.
    ///
    /// # Panics
    /// Panics if `pop` isn't from this world's POPs.
    pub fn pop_outcome(&self, pop: PopId) -> &PopOutcome {
        self.pop_outcomes.get(pop)
    }

    /// Runs the current turn's start (emission, endowment) and part 2 (the decision point,
    /// the trade round, consumption, operation, growth) and returns its report; `perished`
    /// is what the previous turn's end removed.
    fn run_turn(
        &mut self,
        perished: Vec<(GoodId, Quantity)>,
        host: &mut DeciderHost,
    ) -> TurnReport {
        let emitted = self.emit();
        let endowed = self.endow();
        let (decisions, decider_failures) = self.decide(host);
        let TradeTotals {
            sold,
            bought,
            consumed,
            business_sold,
            business_bought,
            produced,
            used,
        } = self.trade();
        let population = self.grow();
        TurnReport {
            turn: self.turn,
            perished,
            emitted,
            endowed,
            sold,
            bought,
            consumed,
            population,
            business_sold,
            business_bought,
            produced,
            used,
            decisions,
            decider_failures,
        }
    }

    /// Start of turn: every deposit, in ID order, adds its type's output to its owner's
    /// stockpile, or to its hex's if it has no owner. Returns the total emitted per good,
    /// in good ID order.
    fn emit(&mut self) -> Vec<(GoodId, Quantity)> {
        let mut emitted = Vec::new();
        for (_, deposit) in self.deposits.iter() {
            let resource = self.rules.resources.get(deposit.resource);
            let good = resource.produces;
            let at = PerishesAt::for_good(self.rules.goods.get(good).perishes, self.turn);
            let stockpile = match deposit.owner {
                Some(business) => self.business_stockpiles.get_mut(business),
                None => self.hex_stockpiles.get_mut(deposit.hex),
            };
            stockpile.add(good, resource.output, at);
            emitted.push((good, resource.output));
        }
        totals_by_good(emitted)
    }

    /// Start of turn, after `emit`: every POP, in ID order, receives its type's endowment
    /// times its size into its stockpile. Returns the total per good, in good ID order.
    fn endow(&mut self) -> Vec<(GoodId, Quantity)> {
        let rules = &self.rules;
        let mut endowed = Vec::new();
        for (id, pop) in self.pops.iter() {
            let pop_type = rules.pop_types.get(pop.pop_type);
            for endowment in pop_type.endowment() {
                let good = endowment.good;
                let total = endowment.amount.saturating_mul(pop.size);
                let at = PerishesAt::for_good(rules.goods.get(good).perishes, self.turn);
                self.pop_stockpiles.get_mut(id).add(good, total, at);
                endowed.push((good, total));
            }
        }
        totals_by_good(endowed)
    }

    /// Part 2 of the turn, after `endow`: the decision point. Stores every business's
    /// answer and returns the counts and the failures.
    fn decide(&mut self, host: &mut DeciderHost) -> (DecisionCounts, Vec<ReportedFailure>) {
        let point = host.decision_point(self);
        let mut counts = DecisionCounts::default();
        for answer in &point.answers {
            match answer {
                BusinessAnswer::Nothing => counts.nothing += 1,
                BusinessAnswer::Valid(_) => counts.valid += 1,
                BusinessAnswer::Refused { .. } => counts.refused += 1,
            }
        }
        self.business_answers = IdVec::from_vec(point.answers);
        (counts, point.failures)
    }

    /// Part 2 of the turn, after `decide`: on every hex in hex ID order, runs the trade round
    /// and consumption for the hex's POPs and businesses, then operates each of its
    /// businesses' factories, and records every POP's and every business's outcome. Returns
    /// the totals (sold, bought, consumed by POPs; sold and bought by businesses; produced
    /// and used by factories) per good, each in good ID order.
    fn trade(&mut self) -> TradeTotals {
        let mut pop_results: Vec<Option<PopOutcome>> = (0..self.pops.len()).map(|_| None).collect();
        let mut business_results: Vec<Option<BusinessOutcome>> =
            (0..self.businesses.len()).map(|_| None).collect();
        let mut sold = Vec::new();
        let mut bought = Vec::new();
        let mut consumed = Vec::new();
        let mut business_sold = Vec::new();
        let mut business_bought = Vec::new();
        let mut produced = Vec::new();
        let mut used = Vec::new();
        let hexes: Vec<HexId> = self.map.iter().map(|(hex, _)| hex).collect();
        for hex in hexes {
            let pop_ids: Vec<PopId> = self.pops.on_hex(hex).to_vec();
            let business_ids: Vec<BusinessId> = self.businesses.on_hex(hex).to_vec();
            if pop_ids.is_empty() && business_ids.is_empty() {
                continue;
            }
            let mut business_submissions: Vec<BusinessSubmission> = business_ids
                .iter()
                .map(|id| self.business_submission(*id))
                .collect();
            let market = Market::new(
                hex,
                &self.rules.goods,
                self.hex_currencies.get(hex),
                self.hex_prices.list(hex),
                self.turn,
            );
            let mut pop_submissions: Vec<Submission<'_>> = pop_ids
                .iter()
                .map(|id| {
                    let pop = self.pops.get(*id);
                    Submission {
                        pop_type: self.rules.pop_types.get(pop.pop_type),
                        size: pop.size,
                        stockpile: std::mem::take(self.pop_stockpiles.get_mut(*id)),
                    }
                })
                .collect();
            let round = match hex_round(&market, &mut pop_submissions, &mut business_submissions) {
                Ok(round) => round,
                Err(RefusedTrade { trader, error }) => match trader {
                    RefusedTrader::Pop(i) => panic!(
                        "engine bug: the market on hex #{} refused a trade for POP #{}: {error}",
                        hex.index(),
                        pop_ids[i].index()
                    ),
                    RefusedTrader::Business(i) => panic!(
                        "engine bug: the market on hex #{} refused a trade for business '{}': \
                         {error}",
                        hex.index(),
                        self.businesses.name(business_ids[i])
                    ),
                },
            };
            for ((id, submission), outcome) in pop_ids.iter().zip(pop_submissions).zip(round.pops) {
                *self.pop_stockpiles.get_mut(*id) = submission.stockpile;
                sold.extend_from_slice(outcome.sold());
                bought.extend_from_slice(outcome.bought());
                consumed.extend_from_slice(outcome.consumed());
                pop_results[id.index()] = Some(outcome);
            }
            for ((id, submission), trade) in business_ids
                .iter()
                .zip(business_submissions)
                .zip(round.businesses)
            {
                *self.business_stockpiles.get_mut(*id) = submission.stockpile;
                business_sold.extend_from_slice(trade.sold());
                business_bought.extend_from_slice(trade.bought());
                let records = self.operate_business(*id);
                for (factory, record) in &records {
                    let technology = self
                        .rules
                        .technologies
                        .get(self.factories.get(*factory).technology);
                    if record.run.output > Quantity::ZERO {
                        produced.push((technology.output(), record.run.output));
                    }
                    used.extend(record.run.inputs.iter());
                    used.extend(record.maintenance.cost.iter());
                }
                business_results[id.index()] = Some(BusinessOutcome {
                    trade,
                    factories: records,
                });
            }
        }
        self.pop_outcomes = IdVec::from_vec(
            pop_results
                .into_iter()
                .map(|o| o.expect("every POP is on exactly one hex"))
                .collect(),
        );
        self.business_outcomes = IdVec::from_vec(
            business_results
                .into_iter()
                .map(|o| o.expect("every business is on exactly one hex"))
                .collect(),
        );
        TradeTotals {
            sold: totals_by_good(sold),
            bought: totals_by_good(bought),
            consumed: totals_by_good(consumed),
            business_sold: totals_by_good(business_sold),
            business_bought: totals_by_good(business_bought),
            produced: totals_by_good(produced),
            used: totals_by_good(used),
        }
    }

    /// Resolves `business`'s answer into offers and bids by good ID, in good ID order, only
    /// amounts > 0; both empty unless the answer is `Valid`. Moves the stockpile out.
    fn business_submission(&mut self, business: BusinessId) -> BusinessSubmission {
        let (offers, bids) = match self.business_answers.get(business) {
            BusinessAnswer::Valid(decision) => (
                resolve_amounts(&self.rules.goods, &decision.offers),
                resolve_amounts(&self.rules.goods, &decision.bids),
            ),
            BusinessAnswer::Nothing | BusinessAnswer::Refused { .. } => (Vec::new(), Vec::new()),
        };
        BusinessSubmission {
            offers,
            bids,
            stockpile: std::mem::take(self.business_stockpiles.get_mut(business)),
        }
    }

    /// Operates `business`'s factories with the fractions its answer gives (0 and 0 for a
    /// factory it doesn't mention, and for every factory when it decided nothing or was
    /// refused), writes the conditions back, and returns the records in factory ID order.
    fn operate_business(&mut self, business: BusinessId) -> Vec<(FactoryId, FactoryRecord)> {
        let ids: Vec<FactoryId> = self.factories.owned_by(business).collect();
        if ids.is_empty() {
            return Vec::new();
        }
        let rules = &self.rules;
        let decision = match self.business_answers.get(business) {
            BusinessAnswer::Valid(decision) => Some(decision),
            BusinessAnswer::Nothing | BusinessAnswer::Refused { .. } => None,
        };
        let mut operations: Vec<Operation<'_>> = ids
            .iter()
            .map(|id| {
                let factory = self.factories.get(*id);
                let number = u32::try_from(id.index()).expect("factory IDs fit in a u32");
                let (run_fraction, maintenance_fraction) = decision
                    .and_then(|d| d.factories.get(&number))
                    .map_or((0.0, 0.0), |order| (order.run, order.maintenance));
                Operation {
                    technology: rules.technologies.get(factory.technology),
                    condition: factory.condition,
                    run_fraction,
                    maintenance_fraction,
                }
            })
            .collect();
        let mut stockpile = std::mem::take(self.business_stockpiles.get_mut(business));
        let records = production::operate(&mut operations, &mut stockpile, &rules.goods, self.turn);
        *self.business_stockpiles.get_mut(business) = stockpile;
        for (id, operation) in ids.iter().zip(&operations) {
            self.factories.set_condition(*id, operation.condition);
        }
        ids.into_iter().zip(records).collect()
    }

    /// Part 2 of the turn, after `trade`: every POP, in ID order, grows by the rate its
    /// outcome records (`size × (1 + rate)`). Returns the total population after growth.
    fn grow(&mut self) -> Quantity {
        let mut population = Quantity::ZERO;
        for (id, outcome) in self.pop_outcomes.iter() {
            let size = grown_size(self.pops.get(id).size, outcome.growth_rate());
            self.pops.set_size(id, size);
            population = population.saturating_add(size);
        }
        population
    }

    /// End of turn: removes whatever perishes at the end of the current turn from every
    /// hex's stockpile, then every POP's, then every business's. Returns the total removed
    /// per good, in good ID order.
    fn perish(&mut self) -> Vec<(GoodId, Quantity)> {
        let turn = self.turn;
        totals_by_good(
            self.hex_stockpiles
                .values_mut()
                .chain(self.pop_stockpiles.values_mut())
                .chain(self.business_stockpiles.values_mut())
                .flat_map(|stockpile| stockpile.remove_perishing(turn)),
        )
    }
}

/// A decision's amounts by good name as (good, amount) in good ID order, leaving out zero
/// amounts. A valid decision names known goods with valid amounts.
fn resolve_amounts(goods: &Goods, amounts: &BTreeMap<String, f64>) -> Vec<(GoodId, Quantity)> {
    let mut resolved: Vec<(GoodId, Quantity)> = amounts
        .iter()
        .map(|(name, amount)| {
            (
                goods.id(name).expect("a valid decision names known goods"),
                Quantity::new(*amount).expect("a valid decision has valid amounts"),
            )
        })
        .filter(|(_, amount)| *amount > Quantity::ZERO)
        .collect();
    resolved.sort_by_key(|(good, _)| *good);
    resolved
}

/// Sums `amounts` per good (saturating) and returns the totals in good ID order, leaving
/// out goods whose total is zero.
fn totals_by_good(
    amounts: impl IntoIterator<Item = (GoodId, Quantity)>,
) -> Vec<(GoodId, Quantity)> {
    let mut totals: BTreeMap<GoodId, Quantity> = BTreeMap::new();
    for (good, amount) in amounts {
        let total = totals.entry(good).or_insert(Quantity::ZERO);
        *total = total.saturating_add(amount);
    }
    totals
        .into_iter()
        .filter(|(_, amount)| *amount > Quantity::ZERO)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::setting::load_setting;

    fn world_of(setting_name: &str) -> World {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/settings")
            .join(setting_name);
        let setting = load_setting(&path).unwrap_or_else(|e| panic!("should load:\n{e}"));
        World::new(setting, &mut DeciderHost::idle()).unwrap().0
    }

    fn turns_valid_world() -> World {
        world_of("turns_valid")
    }

    #[test]
    fn step_error_display() {
        assert_eq!(
            StepError::ClockAtMaximum.to_string(),
            "the clock is at its maximum turn (18446744073709551615) and can't advance"
        );
    }

    #[test]
    fn a_world_at_the_clock_maximum_refuses_to_step_and_is_unchanged() {
        for mut world in [
            turns_valid_world(),
            world_of("pops_placed"),
            world_of("businesses_valid"),
        ] {
            world.step(&mut DeciderHost::idle()).unwrap();
            world.turn = Turn::MAX;
            let before: IdVec<HexKind, Stockpile> = world.hex_stockpiles.clone();
            let pops_before: IdVec<PopKind, Stockpile> = world.pop_stockpiles.clone();
            let businesses_before: IdVec<BusinessKind, Stockpile> =
                world.business_stockpiles.clone();
            let sizes_before: Vec<Quantity> = world.pops.iter().map(|(_, p)| p.size).collect();
            let outcomes_before: IdVec<PopKind, PopOutcome> = world.pop_outcomes.clone();
            let answers_before: IdVec<BusinessKind, BusinessAnswer> =
                world.business_answers.clone();
            let business_outcomes_before: IdVec<BusinessKind, BusinessOutcome> =
                world.business_outcomes.clone();
            let conditions_before: Vec<f64> =
                world.factories.iter().map(|(_, f)| f.condition).collect();
            assert_eq!(
                world.step(&mut DeciderHost::idle()),
                Err(StepError::ClockAtMaximum)
            );
            assert_eq!(world.turn(), Turn::MAX);
            assert_eq!(world.hex_stockpiles, before);
            assert_eq!(world.pop_stockpiles, pops_before);
            assert_eq!(world.business_stockpiles, businesses_before);
            assert_eq!(world.pop_outcomes, outcomes_before);
            assert_eq!(world.business_answers, answers_before);
            assert_eq!(world.business_outcomes, business_outcomes_before);
            let conditions_after: Vec<f64> =
                world.factories.iter().map(|(_, f)| f.condition).collect();
            assert_eq!(conditions_after, conditions_before);
            let sizes_after: Vec<Quantity> = world.pops.iter().map(|(_, p)| p.size).collect();
            assert_eq!(sizes_after, sizes_before);
        }
    }
}
