//! Integration tests for the dummy market: the `[[market]]` section, the prices a world
//! holds, and the exchange, through the engine's public API only.

mod common;

use std::path::Path;

use common::{failures_of, load, sample_setting, test_setting};
use policyspace_engine::{
    GoodId, HexCoord, HexId, PerishesAt, Quantity, Stockpile, Turn, TurnReport, World,
};

fn world_of(path: &Path) -> (World, TurnReport) {
    World::new(load(path))
}

fn hex(world: &World, q: i32, r: i32) -> HexId {
    world
        .map()
        .id(HexCoord { q, r })
        .unwrap_or_else(|| panic!("no hex at ({q}, {r})"))
}

fn good(world: &World, name: &str) -> GoodId {
    world.rules().goods.id(name).unwrap()
}

fn end(t: u64) -> PerishesAt {
    PerishesAt::EndOfTurn(Turn::new(t))
}

fn q(value: f64) -> Quantity {
    Quantity::new(value).unwrap()
}

/// One good's groups in a stockpile as (when it perishes, amount), soonest first.
fn groups(stockpile: &Stockpile, good: GoodId) -> Vec<(PerishesAt, f64)> {
    stockpile
        .groups(good)
        .map(|(at, amount)| (at, amount.get()))
        .collect()
}

/// A hex's posted prices as (good name, price), in good ID order.
fn posted(world: &World, q: i32, r: i32) -> Vec<(String, f64)> {
    world
        .hex_prices()
        .posted(hex(world, q, r))
        .map(|(g, price)| (world.rules().goods.name(g).to_string(), price.get()))
        .collect()
}

fn pair(name: &str, price: f64) -> (String, f64) {
    (name.to_string(), price)
}

#[test]
fn the_sample_setting_posts_its_prices() {
    let (world, _) = world_of(&sample_setting());
    let silver = good(&world, "silver");
    let labor = good(&world, "unskilled_labor");
    let grain = good(&world, "grain");
    let copper = good(&world, "copper");
    let unimproved_grain = good(&world, "unimproved_grain");
    let unimproved_copper = good(&world, "unimproved_copper");
    let non_currency = [labor, grain, copper, unimproved_grain, unimproved_copper];

    let center = world.market(hex(&world, 0, 0));
    assert!(center.has_market());
    assert_eq!(center.price(labor), Some(q(1.0)));
    assert_eq!(center.price(grain), Some(q(2.0)));
    assert_eq!(center.price(copper), Some(q(5.0)));
    assert_eq!(center.price(unimproved_grain), Some(q(0.5)));
    assert_eq!(center.price(unimproved_copper), Some(q(1.0)));
    assert_eq!(center.prices().count(), 5);

    let east = world.market(hex(&world, 1, 0));
    assert!(east.has_market());
    assert_eq!(east.price(labor), Some(q(1.0)));
    assert_eq!(east.price(grain), Some(q(2.0)));
    assert_eq!(east.price(copper), Some(q(5.0)));
    assert_eq!(east.price(unimproved_grain), None);
    assert_eq!(east.price(unimproved_copper), None);

    let corner = world.market(hex(&world, 2, -2));
    assert!(!corner.has_market());
    for g in non_currency {
        assert_eq!(corner.price(g), None);
    }
    assert_eq!(corner.prices().count(), 0);

    for (h, _) in world.map().iter() {
        let market = world.market(h);
        assert_eq!(market.hex(), h);
        assert_eq!(market.currency(), silver);
        assert_eq!(market.price(silver), Some(Quantity::ONE));
        // The currency is never listed among the posted prices.
        assert!(market.prices().all(|(g, _)| g != silver));
    }
    assert_eq!(world.hex_prices().market_count(), 18);
    assert_eq!(world.hex_prices().len(), 19);
}

#[test]
fn markets_load_from_several_files_and_empty_prices_are_accepted() {
    let path = test_setting("markets_valid");
    let (world, _) = world_of(&path);
    assert_eq!(world.hex_prices().market_count(), 3);
    assert_eq!(
        posted(&world, 0, 0),
        vec![pair("grain", 2.0), pair("fish", 1.5)]
    );
    assert_eq!(
        posted(&world, 1, 0),
        vec![pair("coin", 0.25), pair("grain", 3.0)]
    );
    assert_eq!(posted(&world, 2, 0), Vec::<(String, f64)>::new());
    assert_eq!(posted(&world, 3, 0), Vec::<(String, f64)>::new());
    let has: Vec<bool> = (0..4)
        .map(|x| world.hex_prices().has_market(hex(&world, x, 0)))
        .collect();
    assert_eq!(has, vec![true, true, true, false]);

    // The hex with a market and no prices still has a market; the other has none.
    assert!(world.market(hex(&world, 2, 0)).has_market());
    assert!(!world.market(hex(&world, 3, 0)).has_market());

    // The currency of (1, 0) is `shell`, so `coin` may be priced there; (0, 0)'s is `coin`.
    let coin = good(&world, "coin");
    let shell = good(&world, "shell");
    assert_eq!(world.market(hex(&world, 1, 0)).price(coin), Some(q(0.25)));
    assert_eq!(
        world.market(hex(&world, 1, 0)).price(shell),
        Some(Quantity::ONE)
    );
    assert_eq!(
        world.market(hex(&world, 0, 0)).price(coin),
        Some(Quantity::ONE)
    );

    // Loading again gives the same lists.
    let (again, _) = world_of(&path);
    for x in 0..4 {
        assert_eq!(posted(&world, x, 0), posted(&again, x, 0));
    }
}

#[test]
fn every_market_error_is_reported_at_its_place() {
    let expected = vec![
        "rules/m.toml:1:3: section 'market' belongs in start/, not rules/",
        "start/a.toml:6:1: missing required key 'q' in market",
        "start/a.toml:10:1: missing required key 'r' in market",
        "start/a.toml:15:5: key 'q' in market must be an integer, found float",
        "start/a.toml:20:5: key 'q' in market is out of range",
        "start/a.toml:24:1: no hex at (9, 9)",
        "start/a.toml:32:20: key 'grain' in prices of market must be a number, found string",
        "start/a.toml:37:20: key 'grain' in prices of market must be a finite number greater than 0",
        "start/a.toml:42:20: key 'grain' in prices of market must be a finite number greater than 0",
        "start/a.toml:47:20: key 'grain' in prices of market must be a finite number greater than 0",
        "start/a.toml:52:10: key 'prices' in market must be a table, found integer",
        "start/a.toml:57:12: 'coin' is the currency at (6, 0), so it can't have a price",
        "start/a.toml:62:12: unknown good 'tin'",
        "start/a.toml:64:1: missing required key 'prices' in market",
        "start/a.toml:71:23: unknown good 'tin'",
        "start/a.toml:71:32: 'coin' is the currency at (1, 1), so it can't have a price",
        "start/a.toml:72:1: unknown key 'colour' in market",
        "start/b.toml:1:1: duplicate market at (0, 0) (first defined at start/a.toml:1:1)",
    ];
    assert_eq!(failures_of("markets_errors"), expected);
}

/// The `turns_valid` world: hex (0, 0) prices `daily` 2 and `stew` 4, hex (1, 0) prices
/// `daily` 0.5 and `stew` 3, and hex (2, 0) has no market.
fn turns_world() -> World {
    world_of(&test_setting("turns_valid")).0
}

/// A copy of a hex's stockpile, to trade with.
fn stock(world: &World, x: i32) -> Stockpile {
    world.hex_stockpile(hex(world, x, 0)).clone()
}

#[test]
fn selling_removes_the_good_and_adds_currency() {
    let world = turns_world();
    let daily = good(&world, "daily");
    let gold = good(&world, "gold");
    let market = world.market(hex(&world, 1, 0));
    // At turn 0 hex (1, 0) holds `daily` 2 at end(0) and `gold` 1.5 that never perishes.
    let mut pile = stock(&world, 1);

    assert_eq!(market.sell(&mut pile, daily, q(1.5)), Ok(q(0.75)));
    assert_eq!(groups(&pile, daily), vec![(end(0), 0.5)]);
    assert_eq!(pile.total(gold), q(2.25));
    assert_eq!(groups(&pile, gold), vec![(PerishesAt::Never, 2.25)]);

    // Selling more than is held is refused and changes nothing.
    let mut pile = stock(&world, 1);
    let before = pile.clone();
    assert_eq!(
        market.sell(&mut pile, daily, q(3.0)),
        Err(policyspace_engine::TradeError::NotEnoughGoods {
            good: daily,
            held: q(2.0),
            wanted: q(3.0)
        })
    );
    assert_eq!(pile, before);

    // Selling everything.
    let mut pile = stock(&world, 1);
    assert_eq!(market.sell_all(&mut pile, daily), Ok((q(2.0), q(1.0))));
    assert_eq!(pile.total(daily), Quantity::ZERO);
    assert_eq!(pile.groups(daily).count(), 0);
    assert_eq!(pile.total(gold), q(2.5));
    // Selling everything of a good that isn't held trades nothing.
    assert_eq!(
        market.sell_all(&mut pile, daily),
        Ok((Quantity::ZERO, Quantity::ZERO))
    );
    assert_eq!(pile.total(gold), q(2.5));
}

#[test]
fn buying_removes_currency_and_adds_the_good() {
    let mut world = turns_world();
    let daily = good(&world, "daily");
    let stew = good(&world, "stew");
    let gold = good(&world, "gold");
    let market = world.market(hex(&world, 1, 0));
    let mut pile = stock(&world, 1);

    // 0.5 stew costs 1.5 gold: all the gold the hex holds.
    assert_eq!(market.buy(&mut pile, stew, q(0.5)), Ok(q(1.5)));
    assert_eq!(pile.total(gold), Quantity::ZERO);
    assert!(pile.iter().all(|(g, _)| g != gold));
    // `stew` perishes after 3 turns, so a unit bought in turn 0 lasts to the end of turn 2.
    assert_eq!(groups(&pile, stew), vec![(end(2), 0.5)]);

    // Not enough currency: refused, unchanged.
    let mut pile = stock(&world, 1);
    let before = pile.clone();
    assert_eq!(
        market.buy(&mut pile, stew, q(1.0)),
        Err(policyspace_engine::TradeError::NotEnoughCurrency {
            currency: gold,
            held: q(1.5),
            needed: q(3.0)
        })
    );
    assert_eq!(pile, before);

    // After one step the new unit's perish time follows the current turn.
    world.step().unwrap();
    let mut pile = stock(&world, 1);
    assert_eq!(pile.total(gold), q(3.0));
    let market = world.market(hex(&world, 0, 0));
    assert_eq!(market.buy(&mut pile, daily, q(1.0)), Ok(q(2.0)));
    assert_eq!(groups(&pile, daily), vec![(end(1), 3.0)]);
    assert_eq!(pile.total(gold), q(1.0));
}

#[test]
fn untraded_goods_and_the_currency_are_refused() {
    let world = turns_world();
    let daily = good(&world, "daily");
    let gold = good(&world, "gold");

    // Hex (2, 0) has no market: nothing is traded, but the currency still costs 1.
    let none = world.market(hex(&world, 2, 0));
    assert!(!none.has_market());
    assert_eq!(none.price(daily), None);
    assert_eq!(none.price(gold), Some(Quantity::ONE));
    let mut pile = stock(&world, 1);
    let before = pile.clone();
    let not_traded = Err(policyspace_engine::TradeError::NotTraded { good: daily });
    assert_eq!(none.sell(&mut pile, daily, q(1.0)), not_traded);
    assert_eq!(
        none.sell_all(&mut pile, daily),
        Err(policyspace_engine::TradeError::NotTraded { good: daily })
    );
    assert_eq!(
        none.buy(&mut pile, daily, q(1.0)),
        Err(policyspace_engine::TradeError::NotTraded { good: daily })
    );
    assert_eq!(pile, before);

    // The currency is never traded against itself.
    let market = world.market(hex(&world, 0, 0));
    let mut pile = stock(&world, 1);
    let before = pile.clone();
    let is_currency = || Err(policyspace_engine::TradeError::IsCurrency { good: gold });
    assert_eq!(market.sell(&mut pile, gold, q(1.0)), is_currency());
    assert_eq!(market.buy(&mut pile, gold, q(1.0)), is_currency());
    assert_eq!(
        market.sell_all(&mut pile, gold),
        Err(policyspace_engine::TradeError::IsCurrency { good: gold })
    );
    assert_eq!(pile, before);
}

#[test]
fn the_soonest_perishing_group_is_sold_first() {
    let mut world = turns_world();
    world.step().unwrap();
    let stew = good(&world, "stew");
    let mut pile = stock(&world, 0);
    assert_eq!(groups(&pile, stew), vec![(end(2), 5.0), (end(3), 5.0)]);
    let market = world.market(hex(&world, 0, 0));
    assert_eq!(market.sell(&mut pile, stew, q(7.0)), Ok(q(28.0)));
    assert_eq!(groups(&pile, stew), vec![(end(3), 3.0)]);
}

#[test]
fn zero_amounts_trade_nothing() {
    let world = turns_world();
    let daily = good(&world, "daily");
    let stew = good(&world, "stew");
    let market = world.market(hex(&world, 1, 0));
    let mut pile = stock(&world, 1);
    let before = pile.clone();
    assert_eq!(
        market.sell(&mut pile, daily, Quantity::ZERO),
        Ok(Quantity::ZERO)
    );
    assert_eq!(
        market.buy(&mut pile, stew, Quantity::ZERO),
        Ok(Quantity::ZERO)
    );
    assert_eq!(pile, before);
}

#[test]
fn stepping_does_not_change_prices() {
    let (mut world, _) = world_of(&sample_setting());
    let lists = |world: &World| -> Vec<Vec<(GoodId, f64)>> {
        world
            .map()
            .iter()
            .map(|(h, _)| {
                world
                    .hex_prices()
                    .posted(h)
                    .map(|(g, p)| (g, p.get()))
                    .collect()
            })
            .collect()
    };
    let before = lists(&world);
    let markets: Vec<bool> = world
        .map()
        .iter()
        .map(|(h, _)| world.hex_prices().has_market(h))
        .collect();
    for _ in 0..3 {
        world.step().unwrap();
    }
    assert_eq!(lists(&world), before);
    let after: Vec<bool> = world
        .map()
        .iter()
        .map(|(h, _)| world.hex_prices().has_market(h))
        .collect();
    assert_eq!(after, markets);
}

#[test]
#[should_panic(expected = "is not from this list")]
fn the_market_of_a_foreign_hex_panics() {
    let (big, _) = world_of(&sample_setting());
    let (small, _) = world_of(&test_setting("turns_valid"));
    let foreign = hex(&big, 0, 2);
    small.market(foreign);
}

#[test]
fn trade_errors_display() {
    use policyspace_engine::TradeError;
    let world = turns_world();
    let daily = good(&world, "daily");
    let gold = good(&world, "gold");
    assert_eq!(
        TradeError::NotTraded { good: daily }.to_string(),
        "good #0 is not traded on this hex"
    );
    assert_eq!(
        TradeError::IsCurrency { good: gold }.to_string(),
        "good #2 is this hex's currency and can't be traded against itself"
    );
    assert_eq!(
        TradeError::NotEnoughGoods {
            good: daily,
            held: q(2.0),
            wanted: q(3.0)
        }
        .to_string(),
        "not enough of good #0 to sell: holds 2, wants to sell 3"
    );
    assert_eq!(
        TradeError::NotEnoughCurrency {
            currency: gold,
            held: q(1.5),
            needed: q(3.0)
        }
        .to_string(),
        "not enough currency (good #2) to buy: holds 1.5, needs 3"
    );
}
