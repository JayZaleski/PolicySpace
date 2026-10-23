//! Order lines and the live market's arithmetic: lines, summed curves and the crossing.
//!
//! A [`Line`] is an actor's order for one good on one side of the market: a straight line
//! through an anchor (a price and the quantity wanted at it) with a slope, clipped to zero
//! and to a maximum, and zero beyond a give-up price. A demand line is zero above its
//! give-up price and a supply line zero below it; at the give-up price itself the line trades
//! its own quantity (the give-up is inclusive). A demand line's slope is at most zero and a
//! supply line's at least zero. [`Line::new`] refuses malformed numbers and reports every
//! problem at once.
//!
//! A [`Curve`] is the lines on one side of a good, summed: a sorted list of breakpoints
//! (price, quantity), linear between neighbours, constant before the first and after the
//! last. A jump is two points at one price (the quantity before, then after). Summing is
//! exact: the breakpoints are the lines' corners merged, not samples. At a jump price a
//! curve takes the side's inclusive value: a demand curve the value before the jump (buyers
//! still buy at their give-up price), a supply curve the value after it (sellers already
//! sell at theirs). So a demand curve is left-continuous, a supply curve right-continuous,
//! and excess demand (demand minus supply) never rises with the price.
//!
//! [`cross`] finds the price the market sets from a good's two curves and its current price:
//!
//! - **A clean crossing**: demand equals supply at one price; everyone trades their whole
//!   line there.
//! - **A crossing at a jump**: the sign of excess demand changes at a price where a curve
//!   jumps; the short side trades its whole line, the long side a common share of its line.
//! - **A range of equal prices**: demand equals supply, both positive, across a range; the
//!   price is the one in the range closest to the current price. At the range's edges a jump
//!   in a curve can leave one side wanting more (a buyer whose give-up price is the range's
//!   top still buys there), and that side then trades a share of its line.
//! - **Nothing would trade**: one side has no lines, or the sides never overlap; the price
//!   stays where it is, nothing trades, and each side reports what it wants at that price.
//!
//! A crossing at price zero is reported as is; the module sets no price floor. It
//! compares exact `f64` values and uses no tolerance. It knows nothing about the world,
//! holdings, goods or files. Nothing calls it yet: wiring it into the turn is later work.

use std::error::Error;
use std::fmt;

use crate::error_list::ErrorList;
use crate::quantity::Quantity;

/// Which side of a good's market an order is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Buying.
    Demand,
    /// Selling.
    Supply,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Demand => write!(f, "demand"),
            Side::Supply => write!(f, "supply"),
        }
    }
}

/// The numbers a line is built from, as the sender gives them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineParams {
    /// The price the line was drawn at; must be finite and greater than 0.
    pub anchor_price: f64,
    /// The quantity at the anchor price; finite, at least 0.
    pub quantity: f64,
    /// Quantity change per unit of price; finite; at most 0 for demand, at least 0 for supply.
    pub slope: f64,
    /// The most the actor trades at any price; finite, at least 0.
    pub maximum: f64,
    /// The last price the actor still trades at; finite, at least 0.
    pub give_up: f64,
}

/// One problem with a line's numbers.
#[derive(Debug, Clone, PartialEq)]
pub enum LineError {
    /// The anchor price is not a finite number greater than 0.
    AnchorPrice {
        /// The number given.
        got: f64,
    },
    /// The quantity is not a finite number of at least 0.
    Quantity {
        /// The number given.
        got: f64,
    },
    /// The slope is not finite.
    Slope {
        /// The number given.
        got: f64,
    },
    /// The slope goes the wrong way for the side.
    SlopeWrongWay {
        /// The line's side.
        side: Side,
        /// The number given.
        got: f64,
    },
    /// The maximum is not a finite number of at least 0.
    Maximum {
        /// The number given.
        got: f64,
    },
    /// The give-up price is not a finite number of at least 0.
    GiveUp {
        /// The number given.
        got: f64,
    },
    /// The quantity at the anchor price is above the maximum.
    QuantityAboveMaximum {
        /// The quantity given.
        quantity: f64,
        /// The maximum given.
        maximum: f64,
    },
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineError::AnchorPrice { got } => write!(
                f,
                "anchor price must be a finite number greater than 0 (got {got})"
            ),
            LineError::Quantity { got } => write!(
                f,
                "quantity must be a finite number of at least 0 (got {got})"
            ),
            LineError::Slope { got } => write!(f, "slope must be a finite number (got {got})"),
            LineError::SlopeWrongWay {
                side: Side::Demand,
                got,
            } => write!(f, "slope of a demand line must be at most 0 (got {got})"),
            LineError::SlopeWrongWay {
                side: Side::Supply,
                got,
            } => write!(f, "slope of a supply line must be at least 0 (got {got})"),
            LineError::Maximum { got } => write!(
                f,
                "maximum must be a finite number of at least 0 (got {got})"
            ),
            LineError::GiveUp { got } => write!(
                f,
                "give-up price must be a finite number of at least 0 (got {got})"
            ),
            LineError::QuantityAboveMaximum { quantity, maximum } => write!(
                f,
                "quantity at the anchor price ({quantity}) is above the maximum ({maximum})"
            ),
        }
    }
}

impl Error for LineError {}

/// Every problem with one line, in field order. Never empty.
pub type LineErrors = ErrorList<LineError>;

/// A well-formed order line: a straight line through the anchor, clipped to 0 and the
/// maximum, and zero beyond the give-up price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    side: Side,
    anchor_price: Quantity,
    quantity: Quantity,
    slope: f64,
    maximum: Quantity,
    give_up: Quantity,
}

impl Line {
    /// Checks every field and builds the line, or reports every problem in field order:
    /// anchor price, quantity, slope (not finite, then wrong way), maximum, give-up price,
    /// then quantity above maximum (only when both are valid numbers).
    pub fn new(side: Side, params: &LineParams) -> Result<Line, LineErrors> {
        let mut errors = Vec::new();
        let anchor_price = Quantity::new(params.anchor_price).filter(|p| p.get() > 0.0);
        if anchor_price.is_none() {
            errors.push(LineError::AnchorPrice {
                got: params.anchor_price,
            });
        }
        let quantity = Quantity::new(params.quantity);
        if quantity.is_none() {
            errors.push(LineError::Quantity {
                got: params.quantity,
            });
        }
        if !params.slope.is_finite() {
            errors.push(LineError::Slope { got: params.slope });
        } else if (side == Side::Demand && params.slope > 0.0)
            || (side == Side::Supply && params.slope < 0.0)
        {
            errors.push(LineError::SlopeWrongWay {
                side,
                got: params.slope,
            });
        }
        let maximum = Quantity::new(params.maximum);
        if maximum.is_none() {
            errors.push(LineError::Maximum {
                got: params.maximum,
            });
        }
        let give_up = Quantity::new(params.give_up);
        if give_up.is_none() {
            errors.push(LineError::GiveUp {
                got: params.give_up,
            });
        }
        if let (Some(quantity), Some(maximum)) = (quantity, maximum)
            && quantity > maximum
        {
            errors.push(LineError::QuantityAboveMaximum {
                quantity: quantity.get(),
                maximum: maximum.get(),
            });
        }
        match (anchor_price, quantity, maximum, give_up) {
            (Some(anchor_price), Some(quantity), Some(maximum), Some(give_up))
                if errors.is_empty() =>
            {
                Ok(Line {
                    side,
                    anchor_price,
                    quantity,
                    slope: params.slope,
                    maximum,
                    give_up,
                })
            }
            _ => Err(ErrorList::new(errors)),
        }
    }

    /// The line's side.
    pub fn side(&self) -> Side {
        self.side
    }

    /// The price the line was drawn at.
    pub fn anchor_price(&self) -> Quantity {
        self.anchor_price
    }

    /// The quantity at the anchor price, as given (already at most the maximum).
    pub fn quantity(&self) -> Quantity {
        self.quantity
    }

    /// The quantity change per unit of price.
    pub fn slope(&self) -> f64 {
        self.slope
    }

    /// The most the line trades at any price.
    pub fn maximum(&self) -> Quantity {
        self.maximum
    }

    /// The last price the actor still trades at.
    pub fn give_up(&self) -> Quantity {
        self.give_up
    }

    /// The numbers back, for the protocol and the UI.
    pub fn params(&self) -> LineParams {
        LineParams {
            anchor_price: self.anchor_price.get(),
            quantity: self.quantity.get(),
            slope: self.slope,
            maximum: self.maximum.get(),
            give_up: self.give_up.get(),
        }
    }

    /// The quantity at `price`: 0 beyond the give-up price (above it for demand, below it for
    /// supply); otherwise quantity + slope × (price − anchor_price), clipped to [0, maximum].
    pub fn at(&self, price: Quantity) -> Quantity {
        let live = match self.side {
            Side::Demand => price <= self.give_up,
            Side::Supply => price >= self.give_up,
        };
        if live {
            self.clipped(price)
        } else {
            Quantity::ZERO
        }
    }

    /// The straight line through the anchor, clipped to [0, maximum], ignoring the give-up.
    fn clipped(&self, price: Quantity) -> Quantity {
        let raw = self.quantity.get() + self.slope * (price.get() - self.anchor_price.get());
        let clipped = raw.max(0.0).min(self.maximum.get());
        Quantity::new(clipped).unwrap_or(Quantity::ZERO)
    }

    /// The quantity just below `price` (the left limit). Equals `at` except at a supply
    /// line's give-up price, where the line has not started yet.
    fn left(&self, price: Quantity) -> Quantity {
        match self.side {
            Side::Demand => self.at(price),
            Side::Supply => {
                if price <= self.give_up {
                    Quantity::ZERO
                } else {
                    self.clipped(price)
                }
            }
        }
    }

    /// The quantity just above `price` (the right limit). Equals `at` except at a demand
    /// line's give-up price, where the line has just ended.
    fn right(&self, price: Quantity) -> Quantity {
        match self.side {
            Side::Supply => self.at(price),
            Side::Demand => {
                if price >= self.give_up {
                    Quantity::ZERO
                } else {
                    self.clipped(price)
                }
            }
        }
    }

    /// Pushes the prices above 0 where this line's quantity changes direction while the
    /// line is live: the give-up price, where the raw line crosses 0, and where it crosses
    /// the maximum.
    fn push_corners(&self, prices: &mut Vec<f64>) {
        let give_up = self.give_up.get();
        let live = |price: f64| match self.side {
            Side::Demand => price <= give_up,
            Side::Supply => price >= give_up,
        };
        let mut candidates = vec![give_up];
        if self.slope != 0.0 {
            let anchor = self.anchor_price.get();
            candidates.push(anchor - self.quantity.get() / self.slope);
            candidates.push(anchor + (self.maximum.get() - self.quantity.get()) / self.slope);
        }
        for price in candidates {
            if price.is_finite() && price > 0.0 && live(price) {
                prices.push(price);
            }
        }
    }
}

/// The lines on one side of a good, summed: piecewise linear, as a breakpoint list.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    side: Side,
    points: Vec<(Quantity, Quantity)>,
}

impl Curve {
    /// A curve of zero everywhere (no lines).
    pub fn empty(side: Side) -> Curve {
        Curve {
            side,
            points: Vec::new(),
        }
    }

    /// The sum of `lines`, all of which must be on `side`.
    ///
    /// # Panics
    /// Panics if a line is on the other side: that is a programming error
    /// (`a <side> line can't be summed into a <side> curve`).
    pub fn sum<'a>(side: Side, lines: impl IntoIterator<Item = &'a Line>) -> Curve {
        let lines: Vec<&Line> = lines.into_iter().collect();
        for line in &lines {
            assert!(
                line.side == side,
                "a {} line can't be summed into a {side} curve",
                line.side
            );
        }
        if lines.is_empty() {
            return Curve::empty(side);
        }
        let mut candidates = vec![0.0];
        for line in &lines {
            line.push_corners(&mut candidates);
        }
        candidates.sort_by(f64::total_cmp);
        candidates.dedup();
        let mut points: Vec<(Quantity, Quantity)> = Vec::new();
        for (i, &candidate) in candidates.iter().enumerate() {
            let price = Quantity::new(candidate).unwrap_or(Quantity::ZERO);
            let total = |f: &dyn Fn(&Line) -> Quantity| {
                lines
                    .iter()
                    .fold(Quantity::ZERO, |sum, line| sum.saturating_add(f(line)))
            };
            let at = total(&|line| line.at(price));
            let after = total(&|line| line.right(price));
            let before = if i == 0 {
                at
            } else {
                total(&|line| line.left(price))
            };
            let mut push = |quantity: Quantity| {
                if points.last() != Some(&(price, quantity)) {
                    points.push((price, quantity));
                }
            };
            if before != after {
                push(before);
                push(after);
            } else {
                push(at);
            }
        }
        Curve { side, points }
    }

    /// The curve's side.
    pub fn side(&self) -> Side {
        self.side
    }

    /// The breakpoints in rising price order: (price, quantity). Linear between neighbours,
    /// constant before the first and after the last, a jump as two points at one price (the
    /// quantity before the jump, then after). Empty for the empty curve. Never two consecutive
    /// points with the same price and quantity, and never more than two at one price.
    pub fn points(&self) -> &[(Quantity, Quantity)] {
        &self.points
    }

    /// True if the curve has no points: zero everywhere.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The quantity at `price`, with the side's convention at a jump: the value before the
    /// jump for demand, after it for supply.
    pub fn at(&self, price: Quantity) -> Quantity {
        match self.side {
            Side::Demand => self.before(price),
            Side::Supply => self.after(price),
        }
    }

    /// The quantity just below `price` (the left limit); equals `at` except at a supply jump.
    pub fn before(&self, price: Quantity) -> Quantity {
        self.limit(price, true)
    }

    /// The quantity just above `price` (the right limit); equals `at` except at a demand jump.
    pub fn after(&self, price: Quantity) -> Quantity {
        self.limit(price, false)
    }

    /// The left (`left` true) or right limit at `price`.
    fn limit(&self, price: Quantity, left: bool) -> Quantity {
        let points = &self.points;
        let (Some(first), Some(last)) = (points.first(), points.last()) else {
            return Quantity::ZERO;
        };
        // The points at exactly `price` are points[from..to] (none, one, or two).
        let from = points.partition_point(|point| point.0 < price);
        let to = points.partition_point(|point| point.0 <= price);
        if from < to {
            return if left {
                points[from].1
            } else {
                points[to - 1].1
            };
        }
        if from == 0 {
            return first.1;
        }
        if from == points.len() {
            return last.1;
        }
        let (x1, y1) = points[from - 1];
        let (x2, y2) = points[from];
        let t = (price.get() - x1.get()) / (x2.get() - x1.get());
        let value = y1.get() + (y2.get() - y1.get()) * t;
        let (low, high) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Quantity::new(value.clamp(low.get(), high.get())).unwrap_or(low)
    }
}

/// What the market sets for one good from its two summed curves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// The price set: the crossing price, or the current price when nothing trades.
    pub price: Quantity,
    /// How much changes hands at that price (zero when nothing trades).
    pub traded: Quantity,
    /// What the demand curve wants at `price` (its unfilled part is `demand - traded`).
    pub demand: Quantity,
    /// What the supply curve wants at `price`.
    pub supply: Quantity,
}

impl Crossing {
    /// True if anything changes hands.
    pub fn trades(&self) -> bool {
        self.traded > Quantity::ZERO
    }

    /// The share of its line every buyer trades: traded / demand, or 1 when demand is 0.
    pub fn demand_share(&self) -> f64 {
        share(self.traded, self.demand)
    }

    /// The share of its line every seller trades: traded / supply, or 1 when supply is 0.
    pub fn supply_share(&self) -> f64 {
        share(self.traded, self.supply)
    }

    /// What buyers wanted and did not get: demand - traded.
    pub fn unfilled_demand(&self) -> Quantity {
        self.demand
            .checked_sub(self.traded)
            .unwrap_or(Quantity::ZERO)
    }

    /// What sellers offered and did not sell: supply - traded.
    pub fn unfilled_supply(&self) -> Quantity {
        self.supply
            .checked_sub(self.traded)
            .unwrap_or(Quantity::ZERO)
    }
}

/// `traded / wanted`, or 1 when nothing is wanted.
fn share(traded: Quantity, wanted: Quantity) -> f64 {
    if wanted == Quantity::ZERO {
        1.0
    } else {
        traded.get() / wanted.get()
    }
}

/// The smaller of two quantities.
fn smaller(a: Quantity, b: Quantity) -> Quantity {
    if a <= b { a } else { b }
}

/// Excess demand (demand minus supply) at one breakpoint price: its left limit, its value
/// and its right limit.
#[derive(Clone, Copy)]
struct Excess {
    left: f64,
    value: f64,
    right: f64,
}

/// Finds the price where `demand` and `supply` cross, given the good's `current` price.
///
/// # Panics
/// Panics if `demand` is not a demand curve or `supply` not a supply curve.
pub fn cross(demand: &Curve, supply: &Curve, current: Quantity) -> Crossing {
    assert!(
        demand.side == Side::Demand,
        "cross needs a demand curve as its first curve"
    );
    assert!(
        supply.side == Side::Supply,
        "cross needs a supply curve as its second curve"
    );
    let no_trade = || Crossing {
        price: current,
        traded: Quantity::ZERO,
        demand: demand.at(current),
        supply: supply.at(current),
    };

    // The merged breakpoint prices of both curves, and excess demand at each.
    let mut prices = vec![0.0];
    for point in demand.points.iter().chain(supply.points.iter()) {
        prices.push(point.0.get());
    }
    prices.sort_by(f64::total_cmp);
    prices.dedup();
    let excess: Vec<Excess> = prices
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let price = Quantity::new(p).unwrap_or(Quantity::ZERO);
            let value = demand.at(price).get() - supply.at(price).get();
            Excess {
                left: if i == 0 {
                    value
                } else {
                    demand.before(price).get() - supply.before(price).get()
                },
                value,
                right: demand.after(price).get() - supply.after(price).get(),
            }
        })
        .collect();

    // p_c: the supremum of the prices where excess demand is positive (0 if none).
    // p_n: the infimum of the prices where it is negative (None if none).
    // Excess demand never rises, so the positive prices come first, then the zeros, then the
    // negatives. Each breakpoint is a point piece; between neighbours it is linear from the
    // right limit of the lower to the left limit of the upper; after the last it is constant.
    let mut p_c = 0.0_f64;
    let mut p_n: Option<f64> = None;
    // True if p_n is a root found strictly inside a linear piece, not a breakpoint price.
    let mut interior_root = false;
    let last = prices.len() - 1;
    for i in 0..=last {
        let x1 = prices[i];
        if excess[i].value > 0.0 {
            p_c = p_c.max(x1);
        } else if excess[i].value < 0.0 && p_n.is_none() {
            p_n = Some(x1);
        }
        let a = excess[i].right;
        if i == last {
            if a > 0.0 {
                // Demand above supply at every price: can't happen, since every demand
                // line ends. Nothing sensible to trade.
                return no_trade();
            }
            if a < 0.0 && p_n.is_none() {
                p_n = Some(x1);
            }
            continue;
        }
        let x2 = prices[i + 1];
        let b = excess[i + 1].left;
        if a > 0.0 && b < 0.0 {
            let t = a / (a - b);
            let root = (x1 + t * (x2 - x1)).clamp(x1, x2);
            p_c = p_c.max(root);
            if p_n.is_none() {
                p_n = Some(root);
                interior_root = root > x1 && root < x2;
            }
        } else if a > 0.0 {
            p_c = p_c.max(x2);
        } else if b < 0.0 && p_n.is_none() {
            p_n = Some(x1);
        }
    }

    let gap = match p_n {
        None => true,
        Some(n) => p_c < n,
    };
    let (price, demanded, supplied) = if gap {
        // Demand equals supply across the gap: take the price in it closest to `current`.
        // At an edge a jump can leave one side wanting more; `at` is the inclusive value.
        let c = current.get();
        let price = if c <= p_c {
            Quantity::new(p_c).unwrap_or(current)
        } else if let Some(n) = p_n.filter(|&n| c >= n) {
            Quantity::new(n).unwrap_or(current)
        } else {
            current
        };
        (price, demand.at(price), supply.at(price))
    } else {
        // A single sign change.
        let price = Quantity::new(p_n.unwrap_or(p_c)).unwrap_or(current);
        let (demanded, supplied) = (demand.at(price), supply.at(price));
        if interior_root {
            // The curves are equal here by construction; interpolating them separately can
            // leave them a floating-point step apart, so report one value for both.
            let both = smaller(demanded, supplied);
            (price, both, both)
        } else {
            (price, demanded, supplied)
        }
    };
    let traded = smaller(demanded, supplied);
    if traded == Quantity::ZERO {
        return no_trade();
    }
    Crossing {
        price,
        traded,
        demand: demanded,
        supply: supplied,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{close, q};

    fn lp(anchor_price: f64, quantity: f64, slope: f64, maximum: f64, give_up: f64) -> LineParams {
        LineParams {
            anchor_price,
            quantity,
            slope,
            maximum,
            give_up,
        }
    }

    fn dl(params: LineParams) -> Line {
        Line::new(Side::Demand, &params).unwrap()
    }

    fn sl(params: LineParams) -> Line {
        Line::new(Side::Supply, &params).unwrap()
    }

    fn pts(curve: &Curve) -> Vec<(f64, f64)> {
        curve
            .points()
            .iter()
            .map(|(p, v)| (p.get(), v.get()))
            .collect()
    }

    fn at(line: &Line, price: f64) -> f64 {
        line.at(q(price)).get()
    }

    fn problems(side: Side, params: LineParams) -> Vec<String> {
        Line::new(side, &params).unwrap_err().lines()
    }

    fn dcurve(params: &[LineParams]) -> (Vec<Line>, Curve) {
        let lines: Vec<Line> = params.iter().map(|p| dl(*p)).collect();
        let curve = Curve::sum(Side::Demand, &lines);
        (lines, curve)
    }

    fn scurve(params: &[LineParams]) -> (Vec<Line>, Curve) {
        let lines: Vec<Line> = params.iter().map(|p| sl(*p)).collect();
        let curve = Curve::sum(Side::Supply, &lines);
        (lines, curve)
    }

    /// Prices to check a curve at: 0, a large price, every breakpoint, points near it on
    /// both sides, and the midpoints between breakpoints.
    fn sweep(curve: &Curve) -> Vec<f64> {
        let mut prices = vec![0.0, 1000.0];
        let mut previous = 0.0;
        for (p, _) in pts(curve) {
            prices.push(p);
            prices.push(p + 1e-3);
            if p > 1e-3 {
                prices.push(p - 1e-3);
            }
            prices.push((p + previous) / 2.0);
            previous = p;
        }
        prices
    }

    fn assert_sums(curve: &Curve, lines: &[Line]) {
        for price in sweep(curve) {
            let want: f64 = lines.iter().map(|l| at(l, price)).sum();
            let got = curve.at(q(price)).get();
            assert!(close(got, want), "at {price}: curve {got}, lines {want}");
        }
    }

    fn assert_well_formed(curve: &Curve) {
        let points = pts(curve);
        for pair in points.windows(2) {
            assert!(pair[0].0 <= pair[1].0, "prices rise: {points:?}");
            assert!(pair[0] != pair[1], "no repeated point: {points:?}");
        }
        for window in points.windows(3) {
            assert!(
                !(window[0].0 == window[1].0 && window[1].0 == window[2].0),
                "at most two points at one price: {points:?}"
            );
        }
    }

    // Lines

    #[test]
    fn a_demand_line_follows_its_slope_and_is_clipped() {
        let line = dl(lp(2.0, 10.0, -2.0, 12.0, 6.0));
        for (price, want) in [
            (0.0, 12.0),
            (1.0, 12.0),
            (2.0, 10.0),
            (6.0, 2.0),
            (6.5, 0.0),
            (7.0, 0.0),
        ] {
            assert!(close(at(&line, price), want), "at {price}");
        }
        let steep = dl(lp(2.0, 10.0, -5.0, 12.0, 100.0));
        assert!(close(at(&steep, 3.0), 5.0));
        assert_eq!(at(&steep, 4.0), 0.0);
        assert_eq!(at(&steep, 50.0), 0.0);
    }

    #[test]
    fn a_supply_line_follows_its_slope_and_is_clipped() {
        let line = sl(lp(2.0, 10.0, 4.0, 20.0, 1.0));
        for (price, want) in [
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, 6.0),
            (2.0, 10.0),
            (4.5, 20.0),
            (100.0, 20.0),
        ] {
            assert!(close(at(&line, price), want), "at {price}");
        }
        let low = sl(lp(5.0, 3.0, 4.0, 20.0, 0.0));
        assert_eq!(at(&low, 0.0), 0.0);
        assert_eq!(at(&low, 4.0), 0.0);
        assert!(close(at(&low, 6.0), 7.0));
    }

    #[test]
    fn the_give_up_price_trades_the_lines_own_quantity() {
        let demand = dl(lp(2.0, 10.0, -2.0, 12.0, 6.0));
        assert!(close(at(&demand, 6.0), 2.0));
        assert_eq!(at(&demand, 6.000_001), 0.0);
        let supply = sl(lp(2.0, 10.0, 4.0, 20.0, 3.0));
        assert!(close(at(&supply, 3.0), 14.0));
        assert_eq!(at(&supply, 2.999_999), 0.0);
        // A demand give-up below the anchor: zero at the anchor.
        let below = dl(lp(5.0, 10.0, -1.0, 20.0, 3.0));
        assert_eq!(at(&below, 5.0), 0.0);
        assert!(close(at(&below, 3.0), 12.0));
    }

    #[test]
    fn a_step_line_jumps_at_its_give_up_price() {
        let step = sl(lp(3.0, 8.0, 0.0, 8.0, 3.0));
        assert_eq!(at(&step, 2.999), 0.0);
        assert_eq!(at(&step, 3.0), 8.0);
        assert_eq!(at(&step, 1000.0), 8.0);
    }

    #[test]
    fn a_give_up_price_of_zero() {
        let seller = sl(lp(1.0, 5.0, 1.0, 10.0, 0.0));
        assert!(close(at(&seller, 0.0), 4.0));
        let buyer = dl(lp(1.0, 5.0, -1.0, 10.0, 0.0));
        assert!(close(at(&buyer, 0.0), 6.0));
        assert_eq!(at(&buyer, 0.001), 0.0);
        assert_eq!(at(&buyer, 5.0), 0.0);
    }

    #[test]
    fn new_refuses_malformed_numbers_with_messages() {
        let ok = lp(2.0, 10.0, -2.0, 12.0, 6.0);
        for (anchor, got) in [(0.0, "0"), (-1.0, "-1"), (f64::NAN, "NaN")] {
            let params = LineParams {
                anchor_price: anchor,
                ..ok
            };
            assert_eq!(
                problems(Side::Demand, params),
                [format!(
                    "anchor price must be a finite number greater than 0 (got {got})"
                )]
            );
        }
        assert_eq!(
            problems(
                Side::Demand,
                LineParams {
                    quantity: -2.0,
                    ..ok
                }
            ),
            ["quantity must be a finite number of at least 0 (got -2)"]
        );
        assert_eq!(
            problems(
                Side::Demand,
                LineParams {
                    slope: f64::NAN,
                    ..ok
                }
            ),
            ["slope must be a finite number (got NaN)"]
        );
        assert_eq!(
            problems(
                Side::Demand,
                LineParams {
                    slope: f64::INFINITY,
                    ..ok
                }
            ),
            ["slope must be a finite number (got inf)"]
        );
        assert_eq!(
            problems(Side::Demand, LineParams { slope: 1.5, ..ok }),
            ["slope of a demand line must be at most 0 (got 1.5)"]
        );
        assert_eq!(
            problems(Side::Supply, LineParams { slope: -1.5, ..ok }),
            ["slope of a supply line must be at least 0 (got -1.5)"]
        );
        assert_eq!(
            problems(
                Side::Demand,
                LineParams {
                    maximum: f64::INFINITY,
                    ..ok
                }
            ),
            ["maximum must be a finite number of at least 0 (got inf)"]
        );
        assert_eq!(
            problems(
                Side::Demand,
                LineParams {
                    give_up: -1.0,
                    ..ok
                }
            ),
            ["give-up price must be a finite number of at least 0 (got -1)"]
        );
        assert_eq!(
            problems(Side::Demand, lp(2.0, 5.0, -1.0, 4.0, 6.0)),
            ["quantity at the anchor price (5) is above the maximum (4)"]
        );
    }

    #[test]
    fn new_reports_every_problem_in_field_order() {
        let all = lp(-1.0, -2.0, f64::NAN, f64::INFINITY, -1.0);
        assert_eq!(
            problems(Side::Demand, all),
            [
                "anchor price must be a finite number greater than 0 (got -1)",
                "quantity must be a finite number of at least 0 (got -2)",
                "slope must be a finite number (got NaN)",
                "maximum must be a finite number of at least 0 (got inf)",
                "give-up price must be a finite number of at least 0 (got -1)",
            ]
        );
        assert_eq!(
            problems(Side::Demand, lp(2.0, 5.0, 1.5, 4.0, 6.0)),
            [
                "slope of a demand line must be at most 0 (got 1.5)",
                "quantity at the anchor price (5) is above the maximum (4)",
            ]
        );
    }

    #[test]
    fn a_line_that_trades_nothing_is_accepted() {
        let no_maximum = dl(lp(1.0, 0.0, -1.0, 0.0, 5.0));
        let flat_zero = sl(lp(1.0, 0.0, 0.0, 5.0, 0.0));
        for price in [0.0, 0.5, 1.0, 3.0, 100.0] {
            assert_eq!(at(&no_maximum, price), 0.0);
            assert_eq!(at(&flat_zero, price), 0.0);
        }
    }

    #[test]
    fn params_and_accessors_return_what_was_given() {
        let params = lp(2.0, 10.0, -2.0, 12.0, 6.0);
        let line = dl(params);
        assert_eq!(line.params(), params);
        assert_eq!(line.side(), Side::Demand);
        assert_eq!(line.anchor_price(), q(2.0));
        assert_eq!(line.quantity(), q(10.0));
        assert_eq!(line.slope(), -2.0);
        assert_eq!(line.maximum(), q(12.0));
        assert_eq!(line.give_up(), q(6.0));
        assert_eq!(sl(lp(1.0, 1.0, 1.0, 1.0, 1.0)).side(), Side::Supply);
    }

    // Curves

    #[test]
    fn a_sum_of_no_lines_is_the_empty_curve() {
        let none: [Line; 0] = [];
        let curve = Curve::sum(Side::Demand, &none);
        assert_eq!(curve, Curve::empty(Side::Demand));
        assert!(curve.is_empty());
        assert!(curve.points().is_empty());
        assert_eq!(curve.side(), Side::Demand);
        for price in [0.0, 3.0, 1000.0] {
            assert_eq!(curve.at(q(price)), Quantity::ZERO);
            assert_eq!(curve.before(q(price)), Quantity::ZERO);
            assert_eq!(curve.after(q(price)), Quantity::ZERO);
        }
        assert!(Curve::empty(Side::Supply).is_empty());
    }

    #[test]
    fn one_demand_line_makes_its_corners() {
        let (lines, curve) = dcurve(&[lp(2.0, 10.0, -2.0, 12.0, 6.0)]);
        assert_eq!(
            pts(&curve),
            [(0.0, 12.0), (1.0, 12.0), (6.0, 2.0), (6.0, 0.0)]
        );
        assert_sums(&curve, &lines);
        assert_eq!(curve.before(q(6.0)), q(2.0));
        assert_eq!(curve.at(q(6.0)), q(2.0));
        assert_eq!(curve.after(q(6.0)), q(0.0));
        assert!(!curve.is_empty());
    }

    #[test]
    fn one_supply_step_makes_a_jump() {
        let (lines, curve) = scurve(&[lp(3.0, 8.0, 0.0, 8.0, 3.0)]);
        assert_eq!(pts(&curve), [(0.0, 0.0), (3.0, 0.0), (3.0, 8.0)]);
        assert_eq!(curve.before(q(3.0)), q(0.0));
        assert_eq!(curve.at(q(3.0)), q(8.0));
        assert_eq!(curve.after(q(3.0)), q(8.0));
        assert_sums(&curve, &lines);
    }

    #[test]
    fn a_demand_give_up_at_zero_jumps_at_zero() {
        let (lines, curve) = dcurve(&[lp(1.0, 5.0, -1.0, 10.0, 0.0)]);
        assert_eq!(pts(&curve), [(0.0, 6.0), (0.0, 0.0)]);
        assert_eq!(curve.at(q(0.0)), q(6.0));
        assert_eq!(curve.after(q(0.0)), q(0.0));
        assert_sums(&curve, &lines);
    }

    #[test]
    fn lines_with_the_same_give_up_make_one_jump_of_the_summed_size() {
        let (lines, curve) = dcurve(&[lp(1.0, 4.0, 0.0, 4.0, 6.0), lp(1.0, 3.0, 0.0, 3.0, 6.0)]);
        assert_eq!(pts(&curve), [(0.0, 7.0), (6.0, 7.0), (6.0, 0.0)]);
        assert_sums(&curve, &lines);
        let (lines, curve) = scurve(&[lp(1.0, 4.0, 0.0, 4.0, 6.0), lp(1.0, 3.0, 0.0, 3.0, 6.0)]);
        assert_eq!(pts(&curve), [(0.0, 0.0), (6.0, 0.0), (6.0, 7.0)]);
        assert_sums(&curve, &lines);
    }

    #[test]
    fn lines_with_different_give_ups_make_two_jumps() {
        let (lines, curve) = dcurve(&[lp(1.0, 4.0, 0.0, 4.0, 6.0), lp(1.0, 3.0, 0.0, 3.0, 8.0)]);
        assert_eq!(
            pts(&curve),
            [(0.0, 7.0), (6.0, 7.0), (6.0, 3.0), (8.0, 3.0), (8.0, 0.0)]
        );
        assert_sums(&curve, &lines);
    }

    #[test]
    fn mixed_supply_lines_sum_exactly() {
        let (lines, curve) = scurve(&[
            lp(2.0, 10.0, 4.0, 20.0, 1.0),
            lp(3.0, 8.0, 0.0, 8.0, 3.0),
            lp(5.0, 2.0, 1.0, 6.0, 0.0),
            lp(4.0, 0.0, 2.0, 5.0, 2.5),
        ]);
        assert_sums(&curve, &lines);
        assert_well_formed(&curve);
        // Left and right limits at the step's jump.
        assert!(curve.at(q(3.0)).get() > curve.before(q(3.0)).get());
        assert_eq!(curve.at(q(3.0)), curve.after(q(3.0)));
    }

    #[test]
    fn mixed_demand_lines_sum_exactly() {
        let (lines, curve) = dcurve(&[
            lp(2.0, 10.0, -2.0, 12.0, 6.0),
            lp(3.0, 5.0, -1.0, 8.0, 9.0),
            lp(4.0, 3.0, 0.0, 3.0, 4.0),
            lp(1.0, 4.0, -3.0, 4.0, 2.0),
        ]);
        assert_sums(&curve, &lines);
        assert_well_formed(&curve);
        assert_eq!(curve.at(q(4.0)), curve.before(q(4.0)));
        assert!(curve.after(q(4.0)).get() < curve.at(q(4.0)).get());
    }

    #[test]
    fn summed_curves_never_go_the_wrong_way() {
        let (_, demand) = dcurve(&[
            lp(2.0, 10.0, -2.0, 12.0, 6.0),
            lp(3.0, 5.0, -1.0, 8.0, 9.0),
            lp(4.0, 3.0, 0.0, 3.0, 4.0),
            lp(1.0, 4.0, -3.0, 4.0, 2.0),
        ]);
        let (_, supply) = scurve(&[
            lp(2.0, 10.0, 4.0, 20.0, 1.0),
            lp(3.0, 8.0, 0.0, 8.0, 3.0),
            lp(5.0, 2.0, 1.0, 6.0, 0.0),
        ]);
        let mut last_demand = f64::INFINITY;
        let mut last_supply = 0.0;
        for i in 0..=800 {
            let price = q(f64::from(i) * 0.05);
            let d = demand.at(price).get();
            let s = supply.at(price).get();
            assert!(d <= last_demand, "demand rose at {price}");
            assert!(s >= last_supply, "supply fell at {price}");
            last_demand = d;
            last_supply = s;
        }
    }

    #[test]
    fn points_are_well_formed() {
        for params in [
            vec![lp(2.0, 10.0, -2.0, 12.0, 6.0)],
            vec![lp(1.0, 4.0, 0.0, 4.0, 6.0), lp(1.0, 3.0, 0.0, 3.0, 6.0)],
            vec![lp(1.0, 0.0, 0.0, 0.0, 3.0)],
            vec![lp(1.0, 5.0, -1.0, 10.0, 0.0), lp(3.0, 3.0, -1.0, 3.0, 3.0)],
        ] {
            assert_well_formed(&dcurve(&params).1);
            let supply: Vec<LineParams> = params
                .iter()
                .map(|p| LineParams {
                    slope: -p.slope,
                    ..*p
                })
                .collect();
            assert_well_formed(&scurve(&supply).1);
        }
    }

    #[test]
    fn lines_that_trade_nothing_sum_to_a_single_zero_point() {
        let (_, curve) = dcurve(&[lp(1.0, 0.0, 0.0, 0.0, 5.0)]);
        assert_eq!(pts(&curve), [(0.0, 0.0), (5.0, 0.0)]);
        assert!(!curve.is_empty());
    }

    #[test]
    #[should_panic(expected = "a supply line can't be summed into a demand curve")]
    fn summing_a_line_of_the_wrong_side_panics() {
        let line = sl(lp(1.0, 1.0, 0.0, 1.0, 0.0));
        Curve::sum(Side::Demand, [&line]);
    }

    // Crossings

    fn cross_of(demand: &[LineParams], supply: &[LineParams], current: f64) -> Crossing {
        cross(&dcurve(demand).1, &scurve(supply).1, q(current))
    }

    fn assert_crossing(c: &Crossing, price: f64, traded: f64) {
        assert!(close(c.price.get(), price), "price {} not {price}", c.price);
        assert!(
            close(c.traded.get(), traded),
            "traded {} not {traded}",
            c.traded
        );
    }

    #[test]
    fn a_clean_crossing_ignores_the_current_price() {
        let demand = [lp(4.0, 10.0, -1.0, 100.0, 100.0)];
        let supply = [lp(4.0, 2.0, 1.0, 100.0, 0.0)];
        for current in [1.0, 8.0, 100.0] {
            let c = cross_of(&demand, &supply, current);
            assert_crossing(&c, 8.0, 6.0);
            assert!(close(c.demand.get(), 6.0));
            assert!(close(c.supply.get(), 6.0));
            assert!(close(c.demand_share(), 1.0));
            assert!(close(c.supply_share(), 1.0));
            assert!(c.trades());
        }
    }

    #[test]
    fn a_crossing_at_a_demand_step_cuts_the_buyers() {
        let c = cross_of(
            &[lp(5.0, 10.0, 0.0, 10.0, 5.0)],
            &[lp(5.0, 4.0, 1.0, 100.0, 0.0)],
            2.0,
        );
        assert_crossing(&c, 5.0, 4.0);
        assert!(close(c.demand.get(), 10.0));
        assert!(close(c.supply.get(), 4.0));
        assert!(close(c.demand_share(), 0.4));
        assert!(close(c.supply_share(), 1.0));
        assert!(close(c.unfilled_demand().get(), 6.0));
        assert!(close(c.unfilled_supply().get(), 0.0));
    }

    #[test]
    fn a_crossing_at_a_supply_step_cuts_the_sellers() {
        let c = cross_of(
            &[lp(5.0, 4.0, -1.0, 100.0, 100.0)],
            &[lp(5.0, 10.0, 0.0, 10.0, 5.0)],
            9.0,
        );
        assert_crossing(&c, 5.0, 4.0);
        assert!(close(c.supply_share(), 0.4));
        assert!(close(c.demand_share(), 1.0));
        assert!(close(c.unfilled_supply().get(), 6.0));
    }

    #[test]
    fn steps_at_the_same_price_trade_the_smaller() {
        let c = cross_of(
            &[lp(5.0, 10.0, 0.0, 10.0, 5.0)],
            &[lp(5.0, 6.0, 0.0, 6.0, 5.0)],
            1.0,
        );
        assert_crossing(&c, 5.0, 6.0);
        assert!(close(c.demand_share(), 0.6));
        assert!(close(c.supply_share(), 1.0));
        let c = cross_of(
            &[lp(5.0, 6.0, 0.0, 6.0, 5.0)],
            &[lp(5.0, 10.0, 0.0, 10.0, 5.0)],
            9.0,
        );
        assert_crossing(&c, 5.0, 6.0);
        assert!(close(c.demand_share(), 1.0));
        assert!(close(c.supply_share(), 0.6));
        // Equal steps: both shares 1.
        let c = cross_of(
            &[lp(5.0, 6.0, 0.0, 6.0, 5.0)],
            &[lp(5.0, 6.0, 0.0, 6.0, 5.0)],
            9.0,
        );
        assert_crossing(&c, 5.0, 6.0);
        assert_eq!(c.demand_share(), 1.0);
        assert_eq!(c.supply_share(), 1.0);
    }

    #[test]
    fn a_line_reaching_its_maximum_is_a_corner_not_a_jump() {
        // Supply reaches its maximum 6 at price 4 and stays flat; demand crosses it there.
        let c = cross_of(
            &[lp(4.0, 10.0, -1.0, 100.0, 100.0)],
            &[lp(4.0, 6.0, 1.0, 6.0, 0.0)],
            1.0,
        );
        assert_crossing(&c, 8.0, 6.0);
        assert!(close(c.demand_share(), 1.0));
        assert!(close(c.supply_share(), 1.0));
    }

    #[test]
    fn a_range_of_equal_prices_picks_the_one_closest_to_the_current_price() {
        let demand = [lp(1.0, 10.0, 0.0, 10.0, 8.0)];
        let supply = [lp(1.0, 10.0, 0.0, 10.0, 5.0)];
        for (current, price) in [(3.0, 5.0), (5.0, 5.0), (6.0, 6.0), (8.0, 8.0), (9.0, 8.0)] {
            let c = cross_of(&demand, &supply, current);
            assert_crossing(&c, price, 10.0);
        }
    }

    #[test]
    fn a_clean_crossing_with_awkward_numbers_leaves_no_sliver() {
        let c = cross_of(
            &[lp(1.0, 7.0, -1.0 / 3.0, 100.0, 100.0)],
            &[lp(1.0, 1.0, 2.0 / 7.0, 100.0, 0.0)],
            5.0,
        );
        assert!(c.trades());
        assert_eq!(c.demand, c.traded);
        assert_eq!(c.supply, c.traded);
        assert_eq!(c.demand_share(), 1.0);
        assert_eq!(c.supply_share(), 1.0);
        assert_eq!(c.unfilled_demand(), Quantity::ZERO);
        assert_eq!(c.unfilled_supply(), Quantity::ZERO);
    }

    #[test]
    fn a_range_edge_with_a_demand_jump_cuts_the_buyers() {
        // Buyers of 10 give up at 8 and buyers of 3 at 5; sellers of 10 sell from 5.
        let c = cross_of(
            &[lp(1.0, 10.0, 0.0, 10.0, 8.0), lp(1.0, 3.0, 0.0, 3.0, 5.0)],
            &[lp(1.0, 10.0, 0.0, 10.0, 5.0)],
            3.0,
        );
        assert_crossing(&c, 5.0, 10.0);
        assert!(close(c.demand.get(), 13.0));
        assert!(close(c.supply.get(), 10.0));
        assert!(close(c.demand_share(), 10.0 / 13.0));
        assert!(close(c.supply_share(), 1.0));
        assert!(close(c.unfilled_demand().get(), 3.0));
    }

    #[test]
    fn a_range_edge_with_a_supply_jump_cuts_the_sellers() {
        let c = cross_of(
            &[lp(1.0, 10.0, 0.0, 10.0, 8.0)],
            &[lp(1.0, 10.0, 0.0, 10.0, 5.0), lp(1.0, 3.0, 0.0, 3.0, 8.0)],
            9.0,
        );
        assert_crossing(&c, 8.0, 10.0);
        assert!(close(c.supply.get(), 13.0));
        assert!(close(c.demand.get(), 10.0));
        assert!(close(c.supply_share(), 10.0 / 13.0));
        assert!(close(c.demand_share(), 1.0));
        assert!(close(c.unfilled_supply().get(), 3.0));
    }

    #[test]
    fn a_range_open_at_infinity_keeps_the_current_price() {
        let demand = [lp(1.0, 10.0, 0.0, 10.0, 1e9)];
        let supply = [lp(1.0, 10.0, 0.0, 10.0, 0.0)];
        for current in [0.0, 0.5, 7.0, 1e6] {
            assert_crossing(&cross_of(&demand, &supply, current), current, 10.0);
        }
    }

    #[test]
    fn buyers_and_sellers_that_never_overlap_keep_the_price_and_trade_nothing() {
        let (_, demand) = dcurve(&[lp(1.0, 10.0, 0.0, 10.0, 5.0)]);
        let (_, supply) = scurve(&[lp(9.0, 10.0, 0.0, 10.0, 8.0)]);
        let c = cross(&demand, &supply, q(3.0));
        assert_eq!(c.price, q(3.0));
        assert_eq!(c.traded, Quantity::ZERO);
        assert_eq!(c.demand, demand.at(q(3.0)));
        assert!(c.demand.get() > 0.0);
        assert_eq!(c.supply, Quantity::ZERO);
        assert!(!c.trades());
        let c = cross(&demand, &supply, q(6.0));
        assert_eq!(c.price, q(6.0));
        assert_eq!((c.traded, c.demand, c.supply), (q(0.0), q(0.0), q(0.0)));
        let c = cross(&demand, &supply, q(9.0));
        assert_eq!(c.price, q(9.0));
        assert_eq!(c.traded, Quantity::ZERO);
        assert_eq!(c.demand, Quantity::ZERO);
        assert_eq!(c.supply, supply.at(q(9.0)));
        assert!(c.supply.get() > 0.0);
    }

    #[test]
    fn one_empty_side_keeps_the_price_and_trade_nothing() {
        let (_, demand) = dcurve(&[lp(1.0, 10.0, 0.0, 10.0, 5.0)]);
        let c = cross(&demand, &Curve::empty(Side::Supply), q(4.0));
        assert_eq!(c.price, q(4.0));
        assert_eq!(c.traded, Quantity::ZERO);
        assert_eq!(c.demand, demand.at(q(4.0)));
        assert_eq!(c.supply, Quantity::ZERO);
        let c = cross(&demand, &Curve::empty(Side::Supply), q(8.0));
        assert_eq!((c.price, c.traded, c.demand), (q(8.0), q(0.0), q(0.0)));

        let (_, supply) = scurve(&[lp(1.0, 10.0, 0.0, 10.0, 0.0)]);
        let c = cross(&Curve::empty(Side::Demand), &supply, q(4.0));
        assert_eq!(c.price, q(4.0));
        assert_eq!(c.traded, Quantity::ZERO);
        assert_eq!(c.demand, Quantity::ZERO);
        assert_eq!(c.supply, supply.at(q(4.0)));

        let c = cross(
            &Curve::empty(Side::Demand),
            &Curve::empty(Side::Supply),
            q(4.0),
        );
        assert_eq!(c.price, q(4.0));
        assert_eq!((c.traded, c.demand, c.supply), (q(0.0), q(0.0), q(0.0)));
    }

    #[test]
    fn touching_at_zero_quantity_trades_nothing() {
        // Demand gives up at 5; supply is 0 at its give-up price 5 and rises after.
        let demand = [lp(1.0, 10.0, 0.0, 10.0, 5.0)];
        let supply = [lp(5.0, 0.0, 1.0, 100.0, 5.0)];
        for current in [3.0, 5.0, 7.0] {
            let c = cross_of(&demand, &supply, current);
            assert_eq!(c.price, q(current));
            assert_eq!(c.traded, Quantity::ZERO);
        }
    }

    #[test]
    fn a_crossing_at_zero_is_reported_as_zero() {
        // Sellers sell 8 even for free; buyers want 5 up to price 10.
        let c = cross_of(
            &[lp(1.0, 5.0, 0.0, 5.0, 10.0)],
            &[lp(1.0, 8.0, 0.0, 8.0, 0.0)],
            4.0,
        );
        assert_crossing(&c, 0.0, 5.0);
        assert!(close(c.demand.get(), 5.0));
        assert!(close(c.supply.get(), 8.0));
        assert!(c.supply_share() < 1.0);
        assert!(close(c.demand_share(), 1.0));
    }

    #[test]
    fn a_sloped_crossing_between_corners_is_found_exactly_enough() {
        // Two demand lines against two supply lines, crossing on a sloped stretch.
        let c = cross_of(
            &[lp(5.0, 6.0, -1.0, 9.0, 12.0), lp(3.0, 4.0, -2.0, 4.0, 3.0)],
            &[lp(5.0, 4.0, 1.0, 12.0, 1.0), lp(6.0, 5.0, 0.5, 8.0, 2.0)],
            1.0,
        );
        assert!(c.trades());
        assert!(close(c.demand.get(), c.supply.get()));
        assert!(close(c.demand_share(), 1.0));
    }

    #[test]
    #[should_panic]
    fn cross_with_the_sides_swapped_panics() {
        let (_, demand) = dcurve(&[lp(1.0, 1.0, 0.0, 1.0, 1.0)]);
        let (_, supply) = scurve(&[lp(1.0, 1.0, 0.0, 1.0, 1.0)]);
        cross(&supply, &demand, q(1.0));
    }

    #[test]
    fn crossing_methods_are_total() {
        let nothing_wanted = Crossing {
            price: q(1.0),
            traded: q(0.0),
            demand: q(0.0),
            supply: q(0.0),
        };
        assert_eq!(nothing_wanted.demand_share(), 1.0);
        assert_eq!(nothing_wanted.supply_share(), 1.0);
        assert!(!nothing_wanted.trades());
        let odd = Crossing {
            price: q(1.0),
            traded: q(5.0),
            demand: q(5.0),
            supply: q(3.0),
        };
        assert_eq!(odd.unfilled_demand(), q(0.0));
        assert_eq!(odd.unfilled_supply(), q(0.0));
    }

    #[test]
    fn side_displays_as_a_word() {
        assert_eq!(Side::Demand.to_string(), "demand");
        assert_eq!(Side::Supply.to_string(), "supply");
    }
}
