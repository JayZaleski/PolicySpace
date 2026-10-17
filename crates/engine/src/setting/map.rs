//! Reading `[[hex]]` sections (phase 1). The domain types live in [`crate::map`] (the
//! hexes) and [`crate::currency`] (each hex's currency); this module only turns setting
//! entries into them. It reads the `currency` key of each hex too, keeps it unresolved in
//! phase 1, and resolves it against the goods in phase 2.
//!
//! It also holds the helpers shared by every section placed by coordinates: [`coordinate`],
//! and [`count`] for the placed sections that can have several copies (deposits and
//! factories).

use std::collections::HashMap;

use crate::currency::HexCurrencies;
use crate::goods::{GoodId, GoodKind};
use crate::map::{HexCoord, HexId, HexMap};

use super::defs::Definitions;
use super::error::{ErrorSink, Located, Location};
use super::reader::EntryReader;

/// The most copies one `[[deposit]]` or `[[factory]]` entry may place.
pub(crate) const MAX_COUNT: u32 = 1_000_000;

/// Collects hexes during phase 1.
#[derive(Default)]
pub(crate) struct MapReader {
    /// Indexed by the future `HexId::index`.
    coords: Vec<HexCoord>,
    /// Where each placed hex's `[[hex]]` header is; indexed like `coords`.
    locations: Vec<Location>,
    /// Coordinates to their index in `coords`. Lookup only; never iterated.
    by_coord: HashMap<HexCoord, usize>,
    /// Each placed hex's currency as written. None if the key was missing or not a string.
    currencies: Vec<Option<Located<String>>>,
    /// Filled in phase 2, indexed like `coords`.
    currency_ids: Vec<Option<GoodId>>,
}

/// Reads one coordinate key as an `i32`. A value outside `i32` is reported at the value.
/// Shared by every section placed by coordinates (hexes, deposits, markets, POPs); the
/// message names the entry's label.
pub(crate) fn coordinate(reader: &mut EntryReader<'_, '_>, key: &str) -> Option<i32> {
    let value = reader.required_integer(key)?;
    match i32::try_from(value.value) {
        Ok(n) => Some(n),
        Err(_) => {
            let message = format!("key '{key}' in {} is out of range", reader.label());
            reader.errors().at(&value.at, message);
            None
        }
    }
}

/// Reads the optional `count` key of a placed entry. Absent gives 1. A count outside
/// `1..=MAX_COUNT` is reported at the value and gives None: `key 'count' in <label> must be
/// at least 1` or `key 'count' in <label> is out of range (the most is 1000000)`. A count
/// that is not an integer is reported by the reader and also gives None.
pub(crate) fn count(reader: &mut EntryReader<'_, '_>) -> Option<u32> {
    let Some(count) = reader.optional_integer("count") else {
        // Absent gives 1; a wrong type was reported by the reader and gives None.
        return if reader.has("count") { None } else { Some(1) };
    };
    let label = reader.label().to_string();
    if count.value < 1 {
        let message = format!("key 'count' in {label} must be at least 1");
        reader.errors().at(&count.at, message);
        return None;
    }
    match u32::try_from(count.value) {
        Ok(n) if n <= MAX_COUNT => Some(n),
        _ => {
            let message =
                format!("key 'count' in {label} is out of range (the most is {MAX_COUNT})");
            reader.errors().at(&count.at, message);
            None
        }
    }
}

impl MapReader {
    /// Reads one `[[hex]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        let at = reader.location().clone();
        // Read every key even if an earlier one failed, so none is falsely reported as
        // unknown.
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
        // Not name-checked: a bad value is simply an unknown good in phase 2.
        let currency = reader.required_string("currency");
        // A hex with a missing or invalid coordinate is not placed, so it can't produce a
        // duplicate error. The load fails anyway.
        let (Some(q), Some(r)) = (q, r) else {
            return;
        };
        let coord = HexCoord::new(q, r);
        if let Some(&first) = self.by_coord.get(&coord) {
            reader.errors().at(
                &at,
                format!(
                    "duplicate hex at {coord} (first defined at {})",
                    self.locations[first]
                ),
            );
            return;
        }
        self.by_coord.insert(coord, self.coords.len());
        self.coords.push(coord);
        self.locations.push(at);
        self.currencies.push(currency);
    }

    /// The ID the hex at `coord` will have in the finished map, or None if there is none.
    pub fn id(&self, coord: HexCoord) -> Option<HexId> {
        let index = *self.by_coord.get(&coord)?;
        // `finish` gives each hex the ID of its index (and checks that it fits).
        Some(HexId::from_usize(index))
    }

    /// Number of placed hexes (the hex count of the finished map).
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// The resolved currency of a placed hex: None before phase 2, or if it didn't resolve.
    pub fn currency(&self, hex: HexId) -> Option<GoodId> {
        self.currency_ids.get(hex.index()).copied().flatten()
    }

    /// Phase 2: resolves each placed hex's currency against the goods. An unknown name is
    /// reported at the value (`unknown good '<name>'`) by `Definitions::resolve`.
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink) {
        self.currency_ids = self
            .currencies
            .iter()
            .map(|currency| {
                currency
                    .as_ref()
                    .and_then(|currency| goods.resolve(currency, sink))
            })
            .collect();
    }

    /// The finished map and currencies. Only call this after a successful load, when
    /// phase 2 has resolved every currency.
    pub fn finish(self) -> (HexMap, HexCurrencies) {
        let currencies = self
            .currency_ids
            .iter()
            .map(|currency| {
                currency.expect(
                    "internal error: a hex's currency was not resolved in a successful load",
                )
            })
            .collect();
        (
            HexMap::from_coords(self.coords),
            HexCurrencies::from_vec(currencies),
        )
    }
}
