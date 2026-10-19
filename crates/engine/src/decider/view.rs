//! The view: what one business is told at the decision point, and how it is built from a
//! [`World`].
//!
//! A view holds the business's circumstances and nothing else: its hex's currency and
//! posted prices, its stockpile split into perish groups, its cash, its factories (by ID
//! number) with their conditions, the technologies those factories run (each once, with
//! every number the rules give it) and its deposits summed per resource type. Everything
//! is named by setting names, so the view reads and writes as JSON without engine IDs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::bundle::Bundle;
use crate::businesses::BusinessId;
use crate::decider::{self, FormatError};
use crate::goods::Goods;
use crate::quantity::Quantity;
use crate::resources::ResourceId;
use crate::stockpile::PerishesAt;
use crate::technologies::TechnologyId;
use crate::turn::Turn;
use crate::world::World;

/// What one business is told at the decision point: its circumstances and nothing else.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessView {
    /// The current turn.
    pub turn: Turn,
    /// The business's name.
    pub business: String,
    /// The name of its hex's currency good.
    pub currency: String,
    /// The currency it holds: the same number as the currency's total in `stockpile`.
    pub cash: Quantity,
    /// The hex's posted prices by good name, in the currency. The currency itself isn't
    /// listed. Empty for a hex with no market, or a market that prices nothing.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub prices: BTreeMap<String, Quantity>,
    /// Every good the business holds, by name, split into perish groups, soonest first
    /// and the never-perishing group last. Only goods it holds; only groups above 0.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub stockpile: BTreeMap<String, Vec<PerishGroup>>,
    /// Its factories by ID number.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub factories: BTreeMap<u32, FactoryView>,
    /// The technologies its factories run, each once, by name, with every number the
    /// rules give it.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub technologies: BTreeMap<String, TechnologyView>,
    /// Its deposits, summed per resource type name. Only types it owns deposits of.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub deposits: BTreeMap<String, DepositsView>,
}

/// One perish group of a good in a stockpile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerishGroup {
    /// The amount in the group. Greater than 0 in a view the engine builds.
    pub amount: Quantity,
    /// The turn at whose end the group perishes, or None if it never does.
    pub perishes: Option<Turn>,
}

/// One factory as its business sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryView {
    /// The name of the technology it runs.
    pub technology: String,
    /// The share of the technology's capital bundle it holds: finite, 0 to 1.
    pub condition: f64,
}

/// A technology's numbers, as the rules give them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TechnologyView {
    /// Inputs per unit of output, by good name. May be empty.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub inputs: BTreeMap<String, Quantity>,
    /// The output good's name.
    pub output: String,
    /// The most output a factory makes in one turn.
    pub capacity: Quantity,
    /// The capital bundle, by good name. Never empty.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub capital: BTreeMap<String, Quantity>,
    /// The construction bundle, by good name. May be empty.
    #[serde(deserialize_with = "crate::decider::no_duplicate_keys")]
    pub construction: BTreeMap<String, Quantity>,
    /// Turns to build a factory (t).
    pub construction_turns: u32,
    /// Turns a factory's capital lasts (t₁).
    pub lifetime: u32,
}

/// The deposits of one resource type a business owns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepositsView {
    /// How many it owns. At least 1.
    pub count: u32,
    /// The name of the good each deposit emits.
    pub produces: String,
    /// How much each emits per turn.
    pub output: Quantity,
}

impl BusinessView {
    /// Compact JSON, one line.
    pub fn to_json(&self) -> String {
        decider::to_json(self)
    }

    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String {
        decider::to_json_pretty(self)
    }

    /// Reads a view, refusing JSON that isn't the format or holds a condition that isn't a
    /// finite number from 0 to 1.
    pub fn from_json(text: &str) -> Result<BusinessView, FormatError> {
        let view: BusinessView = decider::from_json(text)?;
        decider::checked(view.problems())?;
        Ok(view)
    }

    /// Value problems a parse can't catch: for each factory whose condition isn't a
    /// fraction, `"factory #{id}: condition must be a finite number from 0 to 1 (got {v})"`.
    pub(crate) fn problems(&self) -> Vec<String> {
        self.factories
            .iter()
            .filter(|(_, factory)| !decider::is_fraction(factory.condition))
            .map(|(id, factory)| {
                format!(
                    "factory #{id}: condition must be a finite number from 0 to 1 (got {})",
                    factory.condition
                )
            })
            .collect()
    }
}

/// Builds `business`'s view of `world` at its current turn. Reads only. Called by
/// `World::business_view`.
///
/// # Panics
/// Panics if `business` isn't from `world`'s businesses (through the World's accessors).
pub(crate) fn build(world: &World, business: BusinessId) -> BusinessView {
    let rules = world.rules();
    let hex = world.businesses().get(business).hex;
    let currency = world.hex_currency(hex);

    let prices = world
        .hex_prices()
        .posted(hex)
        .map(|(good, price)| (rules.goods.name(good).to_string(), price))
        .collect();

    let held = world.business_stockpile(business);
    let stockpile = held
        .iter()
        .map(|(good, _)| {
            let groups = held
                .groups(good)
                .map(|(at, amount)| PerishGroup {
                    amount,
                    perishes: match at {
                        PerishesAt::EndOfTurn(turn) => Some(turn),
                        PerishesAt::Never => None,
                    },
                })
                .collect();
            (rules.goods.name(good).to_string(), groups)
        })
        .collect();

    let mut factories = BTreeMap::new();
    let mut technology_ids: BTreeSet<TechnologyId> = BTreeSet::new();
    for id in world.factories().owned_by(business) {
        let factory = world.factories().get(id);
        factories.insert(
            u32::try_from(id.index()).expect("factory IDs fit in a u32"),
            FactoryView {
                technology: rules.technologies.name(factory.technology).to_string(),
                condition: factory.condition,
            },
        );
        technology_ids.insert(factory.technology);
    }

    let technologies = technology_ids
        .into_iter()
        .map(|id| {
            let technology = rules.technologies.get(id);
            (
                rules.technologies.name(id).to_string(),
                TechnologyView {
                    inputs: named_bundle(&rules.goods, technology.inputs()),
                    output: rules.goods.name(technology.output()).to_string(),
                    capacity: technology.capacity(),
                    capital: named_bundle(&rules.goods, technology.capital()),
                    construction: named_bundle(&rules.goods, technology.construction()),
                    construction_turns: technology.construction_turns(),
                    lifetime: technology.lifetime(),
                },
            )
        })
        .collect();

    let mut counts: BTreeMap<ResourceId, u32> = BTreeMap::new();
    for id in world.deposits().owned_by(business) {
        *counts.entry(world.deposits().get(id).resource).or_insert(0) += 1;
    }
    let deposits = counts
        .into_iter()
        .map(|(id, count)| {
            let resource = rules.resources.get(id);
            (
                rules.resources.name(id).to_string(),
                DepositsView {
                    count,
                    produces: rules.goods.name(resource.produces).to_string(),
                    output: resource.output,
                },
            )
        })
        .collect();

    BusinessView {
        turn: world.turn(),
        business: world.businesses().name(business).to_string(),
        currency: rules.goods.name(currency).to_string(),
        cash: held.total(currency),
        prices,
        stockpile,
        factories,
        technologies,
        deposits,
    }
}

/// A bundle as good name to amount.
fn named_bundle(goods: &Goods, bundle: &Bundle) -> BTreeMap<String, Quantity> {
    bundle
        .iter()
        .map(|(good, amount)| (goods.name(good).to_string(), amount))
        .collect()
}

/// A hand-built view shared by the decider's unit tests: turn 3, business `a`, currency
/// `silver`, cash 10, prices labor 1 and grain 2, silver and grain (6 in two groups) held,
/// factories 0 (`farm`, 1.0) and 1 (`farm`, 0.5), one technology and one deposit type.
#[cfg(test)]
pub(crate) fn test_view() -> BusinessView {
    use crate::test_support::q;

    let map = |items: &[(&str, f64)]| -> BTreeMap<String, Quantity> {
        items.iter().map(|(n, a)| (n.to_string(), q(*a))).collect()
    };
    BusinessView {
        turn: Turn::new(3),
        business: "a".to_string(),
        currency: "silver".to_string(),
        cash: q(10.0),
        prices: map(&[("labor", 1.0), ("grain", 2.0)]),
        stockpile: BTreeMap::from([
            (
                "silver".to_string(),
                vec![PerishGroup {
                    amount: q(10.0),
                    perishes: None,
                }],
            ),
            (
                "grain".to_string(),
                vec![
                    PerishGroup {
                        amount: q(4.0),
                        perishes: Some(Turn::new(3)),
                    },
                    PerishGroup {
                        amount: q(2.0),
                        perishes: Some(Turn::new(5)),
                    },
                ],
            ),
        ]),
        factories: BTreeMap::from([
            (
                0,
                FactoryView {
                    technology: "farm".to_string(),
                    condition: 1.0,
                },
            ),
            (
                1,
                FactoryView {
                    technology: "farm".to_string(),
                    condition: 0.5,
                },
            ),
        ]),
        technologies: BTreeMap::from([(
            "farm".to_string(),
            TechnologyView {
                inputs: map(&[("grain", 1.0), ("labor", 0.5)]),
                output: "bread".to_string(),
                capacity: q(12.0),
                capital: map(&[("tools", 4.0)]),
                construction: map(&[("labor", 8.0)]),
                construction_turns: 2,
                lifetime: 8,
            },
        )]),
        deposits: BTreeMap::from([(
            "field".to_string(),
            DepositsView {
                count: 2,
                produces: "grain".to_string(),
                output: q(4.0),
            },
        )]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::q;

    #[test]
    fn a_view_round_trips_through_json() {
        let v = test_view();
        assert_eq!(BusinessView::from_json(&v.to_json()).unwrap(), v);
        assert_eq!(BusinessView::from_json(&v.to_json_pretty()).unwrap(), v);
    }

    #[test]
    fn json_keys_are_names_and_id_numbers() {
        let text = test_view().to_json();
        assert!(text.contains(r#""business":"a""#), "{text}");
        assert!(
            text.contains(
                r#""factories":{"0":{"technology":"farm","condition":1.0},"1":{"technology":"farm","condition":0.5}}"#
            ),
            "{text}"
        );
        assert!(text.contains(r#""perishes":null"#), "{text}");
        assert!(text.contains(r#""perishes":3"#), "{text}");
        assert!(
            text.contains(r#""prices":{"grain":2.0,"labor":1.0}"#),
            "{text}"
        );
        assert!(!text.contains("Id<"), "{text}");
    }

    #[test]
    fn extreme_values_round_trip_exactly() {
        let mut v = test_view();
        v.turn = Turn::MAX;
        v.cash = q(f64::MAX);
        v.prices.insert("grain".to_string(), q(1e-300));
        v.stockpile.insert(
            "grain".to_string(),
            vec![PerishGroup {
                amount: q(5e-324),
                perishes: Some(Turn::MAX),
            }],
        );
        v.factories.get_mut(&0).unwrap().condition = 1.0 / 3.0;
        assert_eq!(BusinessView::from_json(&v.to_json()).unwrap(), v);
        assert_eq!(BusinessView::from_json(&v.to_json_pretty()).unwrap(), v);
    }

    #[test]
    fn reading_refuses_bad_values_with_a_message() {
        let good = test_view().to_json();

        let bad_condition = good.replacen(r#""condition":1.0"#, r#""condition":1.5"#, 1);
        assert_eq!(
            BusinessView::from_json(&bad_condition),
            Err(FormatError::Values(vec![
                "factory #0: condition must be a finite number from 0 to 1 (got 1.5)".to_string()
            ]))
        );

        let negative_cash = good.replacen(r#""cash":10.0"#, r#""cash":-10.0"#, 1);
        match BusinessView::from_json(&negative_cash) {
            Err(FormatError::Json(m)) => {
                assert!(
                    m.contains("a quantity must be a finite number of at least 0"),
                    "{m}"
                )
            }
            other => panic!("{other:?}"),
        }

        let unknown_key = good.replacen(r#""cash""#, r#""cach""#, 1);
        match BusinessView::from_json(&unknown_key) {
            Err(FormatError::Json(m)) => assert!(m.contains("unknown field"), "{m}"),
            other => panic!("{other:?}"),
        }

        let twice = good.replacen(r#""grain":2.0"#, r#""grain":2.0,"grain":3.0"#, 1);
        match BusinessView::from_json(&twice) {
            Err(FormatError::Json(m)) => {
                assert!(m.contains("the key 'grain' is written twice"), "{m}")
            }
            other => panic!("{other:?}"),
        }

        let bad_key = good.replacen(r#""factories":{"0""#, r#""factories":{"x""#, 1);
        assert!(matches!(
            BusinessView::from_json(&bad_key),
            Err(FormatError::Json(_))
        ));

        assert!(matches!(
            BusinessView::from_json("{"),
            Err(FormatError::Json(_))
        ));
    }
}
