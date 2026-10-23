//! Integration tests for order lines, summed curves and the crossing, through the engine's
//! public API only.

mod common;

use common::{close, q};
use policyspace_engine::{Curve, Line, LineParams, Side, cross};

fn params(anchor_price: f64, quantity: f64, slope: f64, maximum: f64, give_up: f64) -> LineParams {
    LineParams {
        anchor_price,
        quantity,
        slope,
        maximum,
        give_up,
    }
}

#[test]
fn two_demand_lines_and_a_supply_line_cross_where_they_should() {
    // Demand: one line (10 at price 4, falling 1 per unit) and one step of 3 up to price 5.
    // Supply: 2 at price 4, rising 1 per unit.
    let buyers = [
        Line::new(Side::Demand, &params(4.0, 10.0, -1.0, 100.0, 100.0)).unwrap(),
        Line::new(Side::Demand, &params(4.0, 3.0, 0.0, 3.0, 5.0)).unwrap(),
    ];
    let sellers = [Line::new(Side::Supply, &params(4.0, 2.0, 1.0, 100.0, 0.0)).unwrap()];
    let demand = Curve::sum(Side::Demand, &buyers);
    let supply = Curve::sum(Side::Supply, &sellers);

    let points = demand.points();
    assert!(!points.is_empty());
    assert!(points.windows(2).all(|w| w[0].0 <= w[1].0), "sorted");
    assert!(
        points.iter().any(|(p, _)| *p == q(5.0)),
        "the step's jump is a breakpoint"
    );

    // Below 5: demand = (14 - p) + 3, supply = p - 2, so they cross at p = 9.5, above 5.
    // Above 5 only the line is left: 14 - p = p - 2 at p = 8.
    let crossing = cross(&demand, &supply, q(1.0));
    assert!(close(crossing.price.get(), 8.0), "{crossing:?}");
    assert!(close(crossing.traded.get(), 6.0), "{crossing:?}");
    assert!(close(crossing.demand_share(), 1.0));
    assert!(close(crossing.supply_share(), 1.0));
    assert!(crossing.trades());
}

#[test]
fn a_malformed_line_displays_one_problem_per_line() {
    let errors = Line::new(Side::Supply, &params(0.0, 5.0, -1.5, 4.0, -1.0)).unwrap_err();
    let text = errors.to_string();
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "anchor price must be a finite number greater than 0 (got 0)",
            "slope of a supply line must be at least 0 (got -1.5)",
            "give-up price must be a finite number of at least 0 (got -1)",
            "quantity at the anchor price (5) is above the maximum (4)",
        ]
    );
    assert_eq!(errors.len(), 4);
}

#[test]
fn with_no_lines_nothing_trades() {
    let demand = Curve::empty(Side::Demand);
    let supply = Curve::empty(Side::Supply);
    let crossing = cross(&demand, &supply, q(3.0));
    assert_eq!(crossing.price, q(3.0));
    assert!(!crossing.trades());
}
