//! The running world and its turn clock.
//!
//! - A [`World`] is a running world, built from a [`LoadedSetting`]. The rules (goods,
//!   resource types) never change. The world holds what does: the clock, the hexes, the
//!   deposits, and one stockpile of unowned goods per hex. Hexes and deposits don't change
//!   yet, but they are world state because later Stages change them (deposit ownership in
//!   Stage 02).
//! - A turn has three parts:
//!   1. start of turn: every deposit emits its type's `output` of its good into its hex's
//!      stockpile;
//!   2. actors act: nothing yet, and later Stages add POPs, industries, and markets here;
//!   3. end of turn: whatever perishes at the end of this turn is removed.
//! - [`World::new`] puts the world at turn 0 and runs turn 0's start. [`World::step`] ends
//!   the current turn, advances the clock, and starts the next one. Between calls the world
//!   is always at part 2, where this turn's output is in the stockpiles and nothing has
//!   perished yet this turn.
//! - Deposits emit in deposit ID order, and stockpiles iterate in a fixed order, so the
//!   same setting stepped the same number of times always gives the same world and reports.
//! - Stage 02 will send an owned deposit's output to its owner instead of the hex
//!   stockpile.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::goods::GoodId;
use crate::id::IdVec;
use crate::map::{HexId, HexKind, HexMap};
use crate::quantity::Quantity;
use crate::resources::Deposits;
use crate::setting::{LoadedSetting, Manifest, Rules};
use crate::stockpile::{PerishesAt, Stockpile};
use crate::turn::Turn;

/// What happened in one step (or in creating the world).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnReport {
    /// The turn that started.
    pub turn: Turn,
    /// Total removed per good at the end of the previous turn, in good ID order, only
    /// amounts > 0. Always empty for turn 0.
    pub perished: Vec<(GoodId, Quantity)>,
    /// Total emitted per good at the start of `turn`, in good ID order, only amounts > 0.
    pub emitted: Vec<(GoodId, Quantity)>,
}

/// Why a step was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepError {
    /// The clock is at `Turn::MAX` and can't advance. The world is unchanged.
    ClockAtMaximum,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepError::ClockAtMaximum => write!(
                f,
                "the clock is at its maximum turn ({}) and can't advance",
                Turn::MAX
            ),
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
    /// One per hex, by `HexId`.
    hex_stockpiles: IdVec<HexKind, Stockpile>,
}

impl World {
    /// Builds a world from `setting` at turn 0 and runs turn 0's emission.
    /// Returns the world and turn 0's report.
    pub fn new(setting: LoadedSetting) -> (World, TurnReport) {
        let LoadedSetting {
            manifest,
            rules,
            start,
        } = setting;
        let hex_stockpiles = IdVec::from_vec(vec![Stockpile::new(); start.map.len()]);
        let mut world = World {
            manifest,
            rules,
            map: start.map,
            deposits: start.deposits,
            turn: Turn::ZERO,
            hex_stockpiles,
        };
        let emitted = world.emit();
        let report = TurnReport {
            turn: world.turn,
            perished: Vec::new(),
            emitted,
        };
        (world, report)
    }

    /// Ends the current turn, advances the clock, starts the next turn.
    /// At `Turn::MAX` returns `Err(StepError::ClockAtMaximum)` and changes nothing.
    pub fn step(&mut self) -> Result<TurnReport, StepError> {
        let next = self.turn.next().ok_or(StepError::ClockAtMaximum)?;
        let perished = self.perish();
        self.turn = next;
        let emitted = self.emit();
        Ok(TurnReport {
            turn: self.turn,
            perished,
            emitted,
        })
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

    /// The stockpile of unowned goods on `hex`.
    ///
    /// # Panics
    /// Panics if `hex` isn't from this world's map.
    pub fn hex_stockpile(&self, hex: HexId) -> &Stockpile {
        self.hex_stockpiles.get(hex)
    }

    /// Start of turn: every deposit, in ID order, adds its type's output to its hex's
    /// stockpile. Returns the total emitted per good, in good ID order.
    fn emit(&mut self) -> Vec<(GoodId, Quantity)> {
        let mut emitted = Vec::new();
        for (_, deposit) in self.deposits.iter() {
            let resource = self.rules.resources.get(deposit.resource);
            let good = resource.produces;
            let at = PerishesAt::for_good(self.rules.goods.get(good).perishes, self.turn);
            self.hex_stockpiles
                .get_mut(deposit.hex)
                .add(good, resource.output, at);
            emitted.push((good, resource.output));
        }
        totals_by_good(emitted)
    }

    /// End of turn: removes whatever perishes at the end of the current turn from every
    /// hex's stockpile. Returns the total removed per good, in good ID order.
    fn perish(&mut self) -> Vec<(GoodId, Quantity)> {
        let turn = self.turn;
        totals_by_good(
            self.hex_stockpiles
                .values_mut()
                .flat_map(|stockpile| stockpile.remove_perishing(turn)),
        )
    }
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

    fn turns_valid_world() -> World {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/settings/turns_valid");
        let setting = load_setting(&path).unwrap_or_else(|e| panic!("should load:\n{e}"));
        World::new(setting).0
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
        let mut world = turns_valid_world();
        world.step().unwrap();
        world.turn = Turn::MAX;
        let before: IdVec<HexKind, Stockpile> = world.hex_stockpiles.clone();
        assert_eq!(world.step(), Err(StepError::ClockAtMaximum));
        assert_eq!(world.turn(), Turn::MAX);
        assert_eq!(world.hex_stockpiles, before);
    }
}
