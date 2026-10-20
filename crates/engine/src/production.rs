//! Factory mechanics: what happens when a business runs a factory at a fraction of its
//! capacity, how a factory wears each turn, and what maintaining it costs. Pure functions
//! over one business's factories, each with its own tests. The turn calls [`operate`] once
//! per business, right after its hex's trade round, with the fractions the business's valid
//! decision gives (0 and 0 for a factory it doesn't mention, and for every factory of a
//! business that decided nothing or was refused, which then only wear).
//!
//! - **Two fractions.** A business's decider chooses, for each factory, a run fraction and
//!   a maintenance fraction, each 0 to 1.
//! - **Running.** A factory that holds at least one lifetime-slice of capital runs at its
//!   capacity times the run fraction. Below one slice it is shut down and makes nothing, so
//!   every turn of output costs exactly one slice. Inputs leave the stockpile soonest-
//!   perishing first, and output enters it only after every factory has run, as goods of
//!   the current turn.
//! - **Shared shortages.** A business's factories run as if at the same moment, so a
//!   shortage is shared: each factory is cut by the smallest held/asked proportion over the
//!   goods it needs, in one pass, and the leftovers stay in the stockpile. The cuts are a
//!   safety net, not the normal path.
//! - **Wear.** Every factory loses one lifetime-slice of condition per turn, whether or not
//!   it ran, stopping at zero. The arithmetic is in whole slices, so a never-maintained
//!   factory's life is exact.
//! - **Maintenance.** Maintaining a fraction f costs f of the capital bundle and f of the
//!   construction bundle. The fraction is capped at one construction-turn's slice, then at
//!   full condition, then by the goods on hand, shared like inputs. A factory whose
//!   construction takes longer than its lifetime can't be revived from zero.
//! - **Order.** Run, wear, maintain.
//!
//! The module knows nothing about the world, `Factories` or files: the caller moves each
//! business's stockpile out, builds one [`Operation`] per factory and writes the conditions
//! back.

use std::collections::BTreeMap;

use crate::bundle::Bundle;
use crate::goods::{GoodId, Goods};
use crate::quantity::Quantity;
use crate::stockpile::{PerishesAt, Stockpile};
use crate::technologies::Technology;
use crate::turn::Turn;

/// How close to a whole number of slices, or to 0 or 1, a condition must be to count as
/// exactly that. Conditions are decimal numbers and a lifetime-slice need not be exact, so
/// without this a factory could keep a speck of condition after its last turn.
const TOLERANCE: f64 = 1e-9;

/// One factory as the mechanics see it: its technology, its condition going in (and, after
/// `wear`, `maintain` or `operate`, coming out) and the fractions its business asked for.
/// The caller builds one per factory of a business and writes `condition` back afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operation<'a> {
    /// The technology the factory runs.
    pub technology: &'a Technology,
    /// The share of the technology's capital bundle the factory holds: finite, 0 to 1.
    /// Read by every function; changed by `wear` and `maintain`.
    pub condition: f64,
    /// How much of the capacity to use this turn: finite, 0 to 1.
    pub run_fraction: f64,
    /// How much of the full capital bundle to replenish this turn: finite, 0 to 1.
    pub maintenance_fraction: f64,
}

/// What running did for one factory.
#[derive(Debug, Clone, PartialEq)]
pub struct RunRecord {
    /// The run fraction the business asked for.
    pub asked: f64,
    /// The fraction actually run: `asked`, or less after a cut for a missing input, or 0
    /// for a factory below one slice. Below `asked` shows a cut order.
    pub done: f64,
    /// The inputs taken from the stockpile, only amounts > 0 (empty when nothing ran).
    pub inputs: Bundle,
    /// The output made: capacity times `done`.
    pub output: Quantity,
}

/// What maintenance did for one factory.
#[derive(Debug, Clone, PartialEq)]
pub struct MaintenanceRecord {
    /// The maintenance fraction the business asked for.
    pub asked: f64,
    /// The fraction actually replenished, after the construction-slice cap, the
    /// full-condition cap and the goods on hand. Below `asked` shows a cut order.
    pub done: f64,
    /// The goods taken from the stockpile: `done` of the capital bundle plus `done` of the
    /// construction bundle, only amounts > 0 (empty when nothing was done).
    pub cost: Bundle,
}

/// What one turn's mechanics did for one factory, in the order they happened.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryRecord {
    /// The condition before anything happened.
    pub condition_before: f64,
    /// Running.
    pub run: RunRecord,
    /// The condition lost to wear: one lifetime-slice, or what was left if less.
    pub wear: f64,
    /// Maintenance.
    pub maintenance: MaintenanceRecord,
    /// The condition after maintenance: what the caller writes back to the factory.
    pub condition_after: f64,
}

/// A checked fraction as a quantity.
fn fraction(value: f64) -> Quantity {
    Quantity::new(value).expect("a checked fraction is a valid quantity")
}

/// Asserts every condition is a finite number from 0 to 1, and, as asked, every run
/// fraction and every maintenance fraction.
fn check(operations: &[Operation<'_>], run_fractions: bool, maintenance_fractions: bool) {
    let valid = |value: f64| value.is_finite() && (0.0..=1.0).contains(&value);
    for operation in operations {
        assert!(
            valid(operation.condition),
            "a factory's condition must be a finite number from 0 to 1"
        );
        if run_fractions {
            assert!(
                valid(operation.run_fraction),
                "a run fraction must be a finite number from 0 to 1"
            );
        }
        if maintenance_fractions {
            assert!(
                valid(operation.maintenance_fraction),
                "a maintenance fraction must be a finite number from 0 to 1"
            );
        }
    }
}

/// A condition as a number of lifetime-slices, snapped to the nearest whole number when
/// within `TOLERANCE` of it.
fn slices(condition: f64, lifetime: u32) -> f64 {
    let exact = condition * f64::from(lifetime);
    let rounded = exact.round();
    if (exact - rounded).abs() < TOLERANCE {
        rounded
    } else {
        exact
    }
}

/// The condition of a factory holding `slices` lifetime-slices, snapped.
fn condition_of(slices: f64, lifetime: u32) -> f64 {
    snap(slices / f64::from(lifetime))
}

/// A condition within `TOLERANCE` of 0 becomes 0, within `TOLERANCE` of 1 becomes 1.
fn snap(condition: f64) -> f64 {
    if condition < TOLERANCE {
        0.0
    } else if condition > 1.0 - TOLERANCE {
        1.0
    } else {
        condition
    }
}

/// For every good with a need above 0, the share of it the stockpile holds, at most 1. A
/// good with no need isn't listed (its proportion is 1).
fn proportions(needs: &BTreeMap<GoodId, Quantity>, stockpile: &Stockpile) -> BTreeMap<GoodId, f64> {
    needs
        .iter()
        .filter(|(_, need)| need.get() > 0.0)
        .map(|(good, need)| (*good, (stockpile.total(*good).get() / need.get()).min(1.0)))
        .collect()
}

/// The smallest proportion over the goods `bundle` names: 1 if it names none, or none are
/// listed.
fn cut(bundle: &Bundle, proportions: &BTreeMap<GoodId, f64>) -> f64 {
    bundle
        .iter()
        .filter_map(|(good, _)| proportions.get(&good).copied())
        .fold(1.0, f64::min)
}

/// Adds each amount in `bundle`, times `factor`, into `needs`.
fn add_needs(needs: &mut BTreeMap<GoodId, Quantity>, bundle: &Bundle, factor: Quantity) {
    for (good, amount) in bundle.scaled(factor).iter() {
        let need = needs.entry(good).or_insert(Quantity::ZERO);
        *need = need.saturating_add(amount);
    }
}

/// Removes `amount` of `good`; if the stockpile holds a floating-point speck less, takes
/// what it holds. A zero amount is skipped.
fn take(stockpile: &mut Stockpile, good: GoodId, amount: Quantity) {
    if amount == Quantity::ZERO {
        return;
    }
    if let Err(held) = stockpile.remove(good, amount) {
        debug_assert!(
            amount.get() - held.get() < TOLERANCE * amount.get().max(1.0),
            "a shared shortage can only be short by rounding"
        );
        // `held` is what the stockpile holds, so this can't fail.
        let _ = stockpile.remove(good, held);
    }
}

/// The same bundle without zero amounts.
fn positive(bundle: &Bundle) -> Bundle {
    Bundle::new(
        bundle
            .iter()
            .filter(|(_, amount)| amount.get() > 0.0)
            .collect(),
    )
}

/// Runs one business's factories together. Each factory holding at least one lifetime-slice
/// runs at capacity times its run fraction, cut by the smallest held/asked proportion over
/// its inputs when the factories together need more of a good than `stockpile` holds (one
/// pass; the leftover of a good freed by a cut stays). Inputs leave `stockpile` soonest-
/// perishing first; every output enters it after every factory has run, as goods added in
/// `turn` (perishing by the good's rule in `goods`). Conditions don't change. Returns one
/// record per operation, in order; the records don't depend on the order of the slice.
///
/// # Panics
/// Panics if a condition or run fraction isn't a finite number from 0 to 1.
pub fn run(
    operations: &[Operation<'_>],
    stockpile: &mut Stockpile,
    goods: &Goods,
    turn: Turn,
) -> Vec<RunRecord> {
    check(operations, true, false);
    let eligible: Vec<bool> = operations
        .iter()
        .map(|o| slices(o.condition, o.technology.lifetime()) >= 1.0)
        .collect();

    let mut needs = BTreeMap::new();
    for (operation, &eligible) in operations.iter().zip(&eligible) {
        if eligible {
            let wanted = operation
                .technology
                .capacity()
                .saturating_mul(fraction(operation.run_fraction));
            add_needs(&mut needs, operation.technology.inputs(), wanted);
        }
    }
    let proportions = proportions(&needs, stockpile);

    let mut used = Vec::with_capacity(operations.len());
    let mut records = Vec::with_capacity(operations.len());
    for (operation, &eligible) in operations.iter().zip(&eligible) {
        let technology = operation.technology;
        let done = if eligible {
            operation.run_fraction * cut(technology.inputs(), &proportions)
        } else {
            0.0
        };
        let output = technology.capacity().saturating_mul(fraction(done));
        let inputs = technology.inputs().scaled(output);
        records.push(RunRecord {
            asked: operation.run_fraction,
            done,
            inputs: positive(&inputs),
            output,
        });
        used.push(inputs);
    }

    for inputs in &used {
        for (good, amount) in inputs.iter() {
            take(stockpile, good, amount);
        }
    }
    for (operation, record) in operations.iter().zip(&records) {
        if record.output > Quantity::ZERO {
            let output = operation.technology.output();
            stockpile.add(
                output,
                record.output,
                PerishesAt::for_good(goods.get(output).perishes, turn),
            );
        }
    }
    records
}

/// Wears one business's factories: each loses one lifetime-slice of condition, or what it
/// has left if less, whether or not it ran. Returns the condition each lost, in order.
///
/// # Panics
/// Panics if a condition isn't a finite number from 0 to 1.
pub fn wear(operations: &mut [Operation<'_>]) -> Vec<f64> {
    check(operations, false, false);
    operations
        .iter_mut()
        .map(|operation| {
            let lifetime = operation.technology.lifetime();
            let held = slices(operation.condition, lifetime);
            let lost = held.min(1.0);
            operation.condition = condition_of(held - lost, lifetime);
            lost / f64::from(lifetime)
        })
        .collect()
}

/// Maintains one business's factories together. Each factory's fraction is reduced to at
/// most one construction-turn's slice, then to at most what fills it, then by the smallest
/// held/asked proportion over the goods it needs (its capital and construction bundles times
/// the fraction) when the factories together need more than `stockpile` holds. The goods
/// leave `stockpile` soonest-perishing first and each condition rises by the fraction done.
/// Returns one record per operation, in order.
///
/// # Panics
/// Panics if a condition or maintenance fraction isn't a finite number from 0 to 1.
pub fn maintain(
    operations: &mut [Operation<'_>],
    stockpile: &mut Stockpile,
) -> Vec<MaintenanceRecord> {
    check(operations, false, true);
    let mut wants = Vec::with_capacity(operations.len());
    let mut needs = BTreeMap::new();
    for operation in operations.iter() {
        let technology = operation.technology;
        let room = snap((1.0 - operation.condition).max(0.0));
        let want = operation
            .maintenance_fraction
            .min(1.0 / f64::from(technology.construction_turns()))
            .min(room);
        let per_unit = technology.capital().merged(technology.construction());
        add_needs(&mut needs, &per_unit, fraction(want));
        wants.push((want, per_unit));
    }
    let proportions = proportions(&needs, stockpile);

    let mut records = Vec::with_capacity(operations.len());
    let mut costs = Vec::with_capacity(operations.len());
    for (operation, (want, per_unit)) in operations.iter_mut().zip(&wants) {
        let done = want * cut(per_unit, &proportions);
        let cost = per_unit.scaled(fraction(done));
        operation.condition = snap(operation.condition + done);
        records.push(MaintenanceRecord {
            asked: operation.maintenance_fraction,
            done,
            cost: positive(&cost),
        });
        costs.push(cost);
    }
    for cost in &costs {
        for (good, amount) in cost.iter() {
            take(stockpile, good, amount);
        }
    }
    records
}

/// One turn of mechanics for one business: `run`, then `wear`, then `maintain`, in that
/// order (so a full, never-maintained factory runs exactly its lifetime, and replenishing
/// one lifetime-slice each turn holds a factory at full condition). Returns one record per
/// operation, in order, with the conditions before and after.
///
/// # Panics
/// As the three functions.
pub fn operate(
    operations: &mut [Operation<'_>],
    stockpile: &mut Stockpile,
    goods: &Goods,
    turn: Turn,
) -> Vec<FactoryRecord> {
    let before: Vec<f64> = operations.iter().map(|o| o.condition).collect();
    let runs = run(operations, stockpile, goods, turn);
    let wears = wear(operations);
    let maintenances = maintain(operations, stockpile);
    let mut records = Vec::with_capacity(operations.len());
    for (i, ((run, wear), maintenance)) in runs.into_iter().zip(wears).zip(maintenances).enumerate()
    {
        records.push(FactoryRecord {
            condition_before: before[i],
            run,
            wear,
            maintenance,
            condition_after: operations[i].condition,
        });
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goods::{Good, Perishes};
    use crate::id::{NameRegistry, NamedList};
    use crate::test_support::{COPPER, GRAIN, LABOR, SILVER, after, close, end, good, q};

    const TOOLS: u32 = 4;

    fn goods() -> Goods {
        let mut names = NameRegistry::new();
        for name in ["labor", "grain", "copper", "silver", "tools"] {
            names.register(name).unwrap();
        }
        let of = |perishes| Good {
            perishes,
            movable: true,
        };
        NamedList::from_parts(
            names,
            vec![
                of(after(1)),
                of(after(3)),
                of(Perishes::Never),
                of(Perishes::Never),
                of(Perishes::Never),
            ],
        )
    }

    fn bundle(items: &[(u32, f64)]) -> Bundle {
        Bundle::new(items.iter().map(|&(g, a)| (good(g), q(a))).collect())
    }

    fn tech(
        inputs: &[(u32, f64)],
        output: u32,
        capacity: f64,
        capital: &[(u32, f64)],
        construction: &[(u32, f64)],
        construction_turns: u32,
        lifetime: u32,
    ) -> Technology {
        Technology::new(
            bundle(inputs),
            good(output),
            q(capacity),
            bundle(capital),
            bundle(construction),
            construction_turns,
            lifetime,
        )
    }

    /// Inputs grain 2 and labor 1, output copper, capacity 10, capital tools 4,
    /// construction labor 8, t 2, L 4.
    fn smelter() -> Technology {
        tech(
            &[(GRAIN, 2.0), (LABOR, 1.0)],
            COPPER,
            10.0,
            &[(TOOLS, 4.0)],
            &[(LABOR, 8.0)],
            2,
            4,
        )
    }

    /// No inputs, output copper, capacity 10, capital tools 4, construction labor 8.
    fn spring(construction_turns: u32, lifetime: u32) -> Technology {
        tech(
            &[],
            COPPER,
            10.0,
            &[(TOOLS, 4.0)],
            &[(LABOR, 8.0)],
            construction_turns,
            lifetime,
        )
    }

    fn stock(items: &[(u32, f64)]) -> Stockpile {
        let mut stockpile = Stockpile::new();
        for &(g, a) in items {
            stockpile.add(good(g), q(a), PerishesAt::Never);
        }
        stockpile
    }

    fn op(technology: &Technology, condition: f64, run: f64, maintain: f64) -> Operation<'_> {
        Operation {
            technology,
            condition,
            run_fraction: run,
            maintenance_fraction: maintain,
        }
    }

    fn total(stockpile: &Stockpile, g: u32) -> f64 {
        stockpile.total(good(g)).get()
    }

    fn assert_bundle(actual: &Bundle, expected: &[(u32, f64)]) {
        let actual: Vec<(usize, f64)> = actual.iter().map(|(g, a)| (g.index(), a.get())).collect();
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for ((ag, aa), (eg, ea)) in actual.iter().zip(expected) {
            assert_eq!(*ag, *eg as usize, "{actual:?} vs {expected:?}");
            assert!(close(*aa, *ea), "{actual:?} vs {expected:?}");
        }
    }

    #[test]
    fn a_factory_runs_at_capacity_times_the_fraction() {
        let t = smelter();
        let ops = [op(&t, 1.0, 0.5, 0.0)];
        let mut stockpile = stock(&[(GRAIN, 30.0), (LABOR, 13.0)]);
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        assert_eq!(records.len(), 1);
        assert!(close(records[0].asked, 0.5));
        assert!(close(records[0].done, 0.5));
        assert_bundle(&records[0].inputs, &[(LABOR, 5.0), (GRAIN, 10.0)]);
        assert!(close(records[0].output.get(), 5.0));
        assert!(close(total(&stockpile, GRAIN), 20.0));
        assert!(close(total(&stockpile, LABOR), 8.0));
        assert!(close(total(&stockpile, COPPER), 5.0));
        let groups: Vec<_> = stockpile.groups(good(COPPER)).collect();
        assert_eq!(groups, vec![(PerishesAt::Never, q(5.0))]);
        assert_eq!(ops[0].condition, 1.0);
    }

    #[test]
    fn output_perishes_by_its_goods_rule_from_the_current_turn() {
        let t = tech(&[], GRAIN, 2.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let ops = [op(&t, 1.0, 1.0, 0.0)];
        let mut stockpile = Stockpile::new();
        run(&ops, &mut stockpile, &goods(), Turn::new(4));
        let groups: Vec<_> = stockpile.groups(good(GRAIN)).collect();
        assert_eq!(groups, vec![(end(6), q(2.0))]);
    }

    #[test]
    fn inputs_leave_soonest_perishing_groups_first() {
        let t = tech(&[(LABOR, 1.0)], COPPER, 5.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let ops = [op(&t, 1.0, 1.0, 0.0)];
        let mut stockpile = Stockpile::new();
        stockpile.add(good(LABOR), q(4.0), end(0));
        stockpile.add(good(LABOR), q(6.0), end(5));
        run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        let groups: Vec<_> = stockpile.groups(good(LABOR)).collect();
        assert_eq!(groups, vec![(end(5), q(5.0))]);
    }

    #[test]
    fn output_is_delivered_after_every_factory_has_run() {
        let maker = tech(&[], GRAIN, 4.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let user = tech(&[(GRAIN, 1.0)], COPPER, 4.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let ops = [op(&maker, 1.0, 1.0, 0.0), op(&user, 1.0, 1.0, 0.0)];
        let mut stockpile = Stockpile::new();
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        assert!(close(records[0].done, 1.0));
        assert!(close(records[1].done, 0.0));
        assert!(records[1].inputs.is_empty());
        assert!(close(total(&stockpile, GRAIN), 4.0));
        assert!(close(total(&stockpile, COPPER), 0.0));

        let reversed = [ops[1], ops[0]];
        let mut other = Stockpile::new();
        let again = run(&reversed, &mut other, &goods(), Turn::ZERO);
        assert_eq!(again, vec![records[1].clone(), records[0].clone()]);
        assert_eq!(other, stockpile);
    }

    #[test]
    fn a_technology_with_no_inputs_runs_on_nothing() {
        let t = spring(1, 4);
        let ops = [op(&t, 1.0, 1.0, 0.0)];
        let mut stockpile = Stockpile::new();
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        assert!(close(records[0].done, 1.0));
        assert!(records[0].inputs.is_empty());
        assert!(close(records[0].output.get(), 10.0));
    }

    #[test]
    fn a_shortage_is_shared_proportionally_whatever_the_order() {
        let t = tech(&[(LABOR, 1.0)], COPPER, 10.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let ops = [op(&t, 1.0, 1.0, 0.0), op(&t, 1.0, 1.0, 0.0)];
        let mut stockpile = stock(&[(LABOR, 5.0)]);
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        for record in &records {
            assert!(close(record.asked, 1.0));
            assert!(close(record.done, 0.25));
            assert!(close(record.output.get(), 2.5));
            assert_bundle(&record.inputs, &[(LABOR, 2.5)]);
        }
        assert!(close(total(&stockpile, LABOR), 0.0));
        assert!(close(total(&stockpile, COPPER), 5.0));

        let reversed = [ops[1], ops[0]];
        let mut other = stock(&[(LABOR, 5.0)]);
        let again = run(&reversed, &mut other, &goods(), Turn::ZERO);
        let mut flipped = records.clone();
        flipped.reverse();
        assert_eq!(again, flipped);
    }

    #[test]
    fn a_factory_short_of_two_goods_is_cut_by_the_worse_shortage_and_the_leftover_stays() {
        let a = tech(
            &[(GRAIN, 2.0), (LABOR, 1.0)],
            COPPER,
            10.0,
            &[(TOOLS, 1.0)],
            &[],
            1,
            4,
        );
        let b = tech(&[(LABOR, 1.0)], SILVER, 10.0, &[(TOOLS, 1.0)], &[], 1, 4);
        let ops = [op(&a, 1.0, 1.0, 0.0), op(&b, 1.0, 1.0, 0.0)];

        let mut stockpile = stock(&[(GRAIN, 10.0), (LABOR, 10.0)]);
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        assert!(close(records[0].done, 0.5));
        assert_bundle(&records[0].inputs, &[(LABOR, 5.0), (GRAIN, 10.0)]);
        assert!(close(records[1].done, 0.5));
        assert_bundle(&records[1].inputs, &[(LABOR, 5.0)]);
        assert!(close(total(&stockpile, GRAIN), 0.0));
        assert!(close(total(&stockpile, LABOR), 0.0));

        let mut stockpile = stock(&[(GRAIN, 5.0), (LABOR, 20.0)]);
        let records = run(&ops, &mut stockpile, &goods(), Turn::ZERO);
        assert!(close(records[0].done, 0.25));
        assert_bundle(&records[0].inputs, &[(LABOR, 2.5), (GRAIN, 5.0)]);
        assert!(close(records[1].done, 1.0));
        assert_bundle(&records[1].inputs, &[(LABOR, 10.0)]);
        assert!(close(total(&stockpile, LABOR), 7.5));
        assert!(close(total(&stockpile, GRAIN), 0.0));
    }

    #[test]
    fn wear_takes_one_slice_run_or_not_and_stops_at_zero() {
        let t = spring(1, 4);
        for run_fraction in [1.0, 0.0] {
            let mut ops = [op(&t, 1.0, run_fraction, 0.0)];
            let mut stockpile = stock(&[(TOOLS, 10.0)]);
            let records = operate(&mut ops, &mut stockpile, &goods(), Turn::ZERO);
            assert!(close(records[0].wear, 0.25));
            assert!(close(ops[0].condition, 0.75));
        }
        for (condition, lost) in [(0.25, 0.25), (0.0, 0.0), (0.1, 0.1)] {
            let mut ops = [op(&t, condition, 1.0, 0.0)];
            let wears = wear(&mut ops);
            assert!(close(wears[0], lost), "{condition}: {wears:?}");
            assert_eq!(ops[0].condition, 0.0);
        }
    }

    #[test]
    fn a_factory_below_one_slice_makes_nothing() {
        let t = spring(1, 4);
        let g = goods();
        for condition in [0.0, 0.2] {
            let ops = [op(&t, condition, 1.0, 0.0)];
            let records = run(&ops, &mut Stockpile::new(), &g, Turn::ZERO);
            assert!(close(records[0].done, 0.0));
            assert!(close(records[0].output.get(), 0.0));
        }

        let mut ops = [op(&t, 0.375, 1.0, 0.0)];
        let mut stockpile = Stockpile::new();
        let records = operate(&mut ops, &mut stockpile, &g, Turn::ZERO);
        assert!(close(records[0].run.done, 1.0));
        assert!(close(ops[0].condition, 0.125));
        let records = operate(&mut ops, &mut stockpile, &g, Turn::new(1));
        assert!(close(records[0].run.done, 0.0));
        assert!(close(records[0].run.output.get(), 0.0));
        assert_eq!(ops[0].condition, 0.0);
    }

    #[test]
    fn a_full_factory_runs_exactly_its_lifetime() {
        let g = goods();
        for lifetime in [1, 2, 3, 7, 24, 100] {
            let t = spring(1, lifetime);
            let mut ops = [op(&t, 1.0, 1.0, 0.0)];
            let mut stockpile = Stockpile::new();
            for turn in 0..lifetime {
                if turn == lifetime - 1 {
                    assert!(
                        close(ops[0].condition, 1.0 / f64::from(lifetime)),
                        "L {lifetime}"
                    );
                }
                let records = operate(&mut ops, &mut stockpile, &g, Turn::new(u64::from(turn)));
                assert!(close(records[0].run.output.get(), 10.0), "L {lifetime}");
            }
            assert_eq!(ops[0].condition, 0.0, "L {lifetime}");
            let records = operate(&mut ops, &mut stockpile, &g, Turn::new(1000));
            assert!(close(records[0].run.output.get(), 0.0), "L {lifetime}");
        }
    }

    #[test]
    fn an_unmaintained_condition_stays_a_whole_number_of_slices() {
        let t = spring(1, 3);
        let mut ops = [op(&t, 1.0, 0.0, 0.0)];
        wear(&mut ops);
        assert_eq!(ops[0].condition, 2.0 / 3.0);
        wear(&mut ops);
        assert_eq!(ops[0].condition, 1.0 / 3.0);
        wear(&mut ops);
        assert_eq!(ops[0].condition, 0.0);
    }

    #[test]
    fn maintenance_costs_the_fraction_of_both_bundles_and_raises_the_condition() {
        let t = smelter();
        let mut ops = [op(&t, 0.5, 0.0, 0.25)];
        let mut stockpile = stock(&[(TOOLS, 10.0), (LABOR, 10.0)]);
        let records = maintain(&mut ops, &mut stockpile);
        assert!(close(records[0].asked, 0.25));
        assert!(close(records[0].done, 0.25));
        assert_bundle(&records[0].cost, &[(LABOR, 2.0), (TOOLS, 1.0)]);
        assert!(close(ops[0].condition, 0.75));
        assert!(close(total(&stockpile, TOOLS), 9.0));
        assert!(close(total(&stockpile, LABOR), 8.0));
    }

    #[test]
    fn maintenance_is_capped_by_the_construction_slice_then_by_full_condition() {
        let t = smelter();
        let mut stockpile = stock(&[(TOOLS, 100.0), (LABOR, 100.0)]);
        let mut ops = [op(&t, 0.25, 0.0, 1.0)];
        let records = maintain(&mut ops, &mut stockpile);
        assert!(close(records[0].done, 0.5));
        assert!(close(ops[0].condition, 0.75));

        let mut ops = [op(&t, 0.75, 0.0, 1.0)];
        let records = maintain(&mut ops, &mut stockpile);
        assert!(close(records[0].done, 0.25));
        assert_eq!(ops[0].condition, 1.0);

        let one_turn = tech(
            &[(GRAIN, 2.0), (LABOR, 1.0)],
            COPPER,
            10.0,
            &[(TOOLS, 4.0)],
            &[(LABOR, 8.0)],
            1,
            4,
        );
        let mut ops = [op(&one_turn, 0.75, 0.0, 0.1)];
        let records = maintain(&mut ops, &mut stockpile);
        assert!(close(records[0].done, 0.1));
        assert!(close(ops[0].condition, 0.85));
    }

    #[test]
    fn maintenance_shortage_is_shared_proportionally_and_by_the_worst_good() {
        let t = smelter();
        let mut ops = [op(&t, 0.5, 0.0, 0.5), op(&t, 0.5, 0.0, 0.5)];
        let mut stockpile = stock(&[(TOOLS, 2.0), (LABOR, 8.0)]);
        let records = maintain(&mut ops, &mut stockpile);
        for (record, operation) in records.iter().zip(&ops) {
            assert!(close(record.done, 0.25));
            assert_bundle(&record.cost, &[(LABOR, 2.0), (TOOLS, 1.0)]);
            assert!(close(operation.condition, 0.75));
        }
        assert!(close(total(&stockpile, TOOLS), 0.0));
        assert!(close(total(&stockpile, LABOR), 4.0));
    }

    #[test]
    fn a_shut_down_factory_comes_back_slice_by_slice() {
        let g = goods();

        let t = spring(2, 4);
        let mut stockpile = stock(&[(TOOLS, 100.0), (LABOR, 100.0)]);
        let mut ops = [op(&t, 0.0, 1.0, 0.5)];
        let records = operate(&mut ops, &mut stockpile, &g, Turn::ZERO);
        assert!(close(records[0].run.done, 0.0));
        assert!(close(ops[0].condition, 0.5));
        ops[0].maintenance_fraction = 0.0;
        let records = operate(&mut ops, &mut stockpile, &g, Turn::new(1));
        assert!(close(records[0].run.done, 1.0));

        let t = spring(4, 4);
        let mut ops = [op(&t, 0.0, 1.0, 1.0)];
        let records = operate(&mut ops, &mut stockpile, &g, Turn::ZERO);
        assert!(close(records[0].maintenance.done, 0.25));
        for turn in 1..5 {
            let records = operate(&mut ops, &mut stockpile, &g, Turn::new(turn));
            assert!(close(records[0].run.done, 1.0), "turn {turn}");
            assert!(close(ops[0].condition, 0.25), "turn {turn}");
        }

        let t = spring(8, 4);
        let mut ops = [op(&t, 0.0, 1.0, 1.0)];
        for turn in 0..5 {
            let records = operate(&mut ops, &mut stockpile, &g, Turn::new(turn));
            assert!(close(records[0].run.done, 0.0), "turn {turn}");
            assert!(close(records[0].condition_after, 0.125), "turn {turn}");
        }
    }

    #[test]
    fn steady_maintenance_of_one_slice_holds_a_factory_full() {
        let t = tech(&[], COPPER, 10.0, &[(TOOLS, 3.0)], &[], 1, 3);
        let mut ops = [op(&t, 1.0, 1.0, 1.0 / 3.0)];
        let mut stockpile = stock(&[(TOOLS, 1000.0)]);
        let g = goods();
        for turn in 0..30 {
            let records = operate(&mut ops, &mut stockpile, &g, Turn::new(turn));
            assert!(close(records[0].run.done, 1.0), "turn {turn}");
            assert!(close(records[0].run.output.get(), 10.0), "turn {turn}");
            assert!(close(records[0].maintenance.done, 1.0 / 3.0), "turn {turn}");
            assert_eq!(records[0].condition_after, 1.0, "turn {turn}");
        }
    }

    #[test]
    fn run_takes_its_inputs_before_maintenance_takes_its_goods() {
        let t = tech(
            &[(LABOR, 1.0)],
            COPPER,
            10.0,
            &[(TOOLS, 4.0)],
            &[(LABOR, 8.0)],
            1,
            4,
        );
        let mut ops = [op(&t, 0.75, 1.0, 0.25)];
        let mut stockpile = stock(&[(LABOR, 10.0), (TOOLS, 10.0)]);
        let records = operate(&mut ops, &mut stockpile, &goods(), Turn::ZERO);
        assert!(close(records[0].run.done, 1.0));
        assert!(close(records[0].maintenance.asked, 0.25));
        assert!(close(records[0].maintenance.done, 0.0));
        assert!(records[0].maintenance.cost.is_empty());
        assert!(close(total(&stockpile, TOOLS), 10.0));
        assert!(close(total(&stockpile, LABOR), 0.0));
    }

    #[test]
    fn the_record_shows_asked_above_done_on_a_cut() {
        let a = tech(
            &[(GRAIN, 2.0), (LABOR, 1.0)],
            COPPER,
            10.0,
            &[(TOOLS, 4.0)],
            &[(LABOR, 8.0)],
            2,
            4,
        );
        let b = tech(&[(LABOR, 1.0)], SILVER, 10.0, &[(TOOLS, 2.0)], &[], 1, 4);
        let mut ops = [op(&a, 1.0, 1.0, 0.5), op(&b, 1.0, 1.0, 0.25)];
        let mut stockpile = stock(&[(GRAIN, 5.0), (LABOR, 20.0), (TOOLS, 2.0)]);
        let records = operate(&mut ops, &mut stockpile, &goods(), Turn::ZERO);

        // A is cut by grain to 0.25 of its run; asks for 0.5 of maintenance but the room
        // left after wear is 0.25, which the stock covers.
        assert!(close(records[0].condition_before, 1.0));
        assert!(close(records[0].run.asked, 1.0));
        assert!(close(records[0].run.done, 0.25));
        assert!(close(records[0].wear, 0.25));
        assert!(close(records[0].maintenance.asked, 0.5));
        assert!(close(records[0].maintenance.done, 0.25));
        assert!(close(records[0].condition_after, 1.0));

        // B is not cut anywhere, so every asked equals its done.
        assert!(close(records[1].condition_before, 1.0));
        assert!(close(records[1].run.asked, 1.0));
        assert!(close(records[1].run.done, 1.0));
        assert!(close(records[1].wear, 0.25));
        assert!(close(records[1].maintenance.asked, 0.25));
        assert!(close(records[1].maintenance.done, 0.25));
        assert!(close(records[1].condition_after, 1.0));

        assert!(close(total(&stockpile, GRAIN), 0.0));
        assert!(close(total(&stockpile, LABOR), 5.5));
        assert!(close(total(&stockpile, TOOLS), 0.5));
        assert!(close(total(&stockpile, COPPER), 2.5));
        assert!(close(total(&stockpile, SILVER), 10.0));
    }

    #[test]
    #[should_panic(expected = "a run fraction must be a finite number from 0 to 1")]
    fn a_run_fraction_above_one_panics() {
        let t = spring(1, 4);
        run(
            &[op(&t, 1.0, 1.5, 0.0)],
            &mut Stockpile::new(),
            &goods(),
            Turn::ZERO,
        );
    }

    #[test]
    #[should_panic(expected = "a run fraction must be a finite number from 0 to 1")]
    fn a_negative_run_fraction_panics() {
        let t = spring(1, 4);
        run(
            &[op(&t, 1.0, -0.1, 0.0)],
            &mut Stockpile::new(),
            &goods(),
            Turn::ZERO,
        );
    }

    #[test]
    #[should_panic(expected = "a run fraction must be a finite number from 0 to 1")]
    fn a_nan_run_fraction_panics() {
        let t = spring(1, 4);
        run(
            &[op(&t, 1.0, f64::NAN, 0.0)],
            &mut Stockpile::new(),
            &goods(),
            Turn::ZERO,
        );
    }

    #[test]
    #[should_panic(expected = "a maintenance fraction must be a finite number from 0 to 1")]
    fn a_maintenance_fraction_above_one_panics() {
        let t = spring(1, 4);
        maintain(&mut [op(&t, 1.0, 0.0, 2.0)], &mut Stockpile::new());
    }

    #[test]
    #[should_panic(expected = "a factory's condition must be a finite number from 0 to 1")]
    fn a_condition_above_one_panics() {
        let t = spring(1, 4);
        wear(&mut [op(&t, 1.5, 0.0, 0.0)]);
    }
}
