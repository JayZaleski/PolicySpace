//! Reading `[[hex]]` sections (phase 1). The domain types live in [`crate::map`]; this
//! module only turns setting entries into them.

use std::collections::HashMap;

use crate::map::{HexCoord, HexId, HexMap};

use super::error::Location;
use super::reader::EntryReader;

/// Collects hexes during phase 1.
#[derive(Default)]
pub(crate) struct MapReader {
    /// Indexed by the future `HexId::index`.
    coords: Vec<HexCoord>,
    /// Where each placed hex's `[[hex]]` header is; indexed like `coords`.
    locations: Vec<Location>,
    /// Coordinates to their index in `coords`. Lookup only; never iterated.
    by_coord: HashMap<HexCoord, usize>,
}

/// Reads one coordinate key as an `i32`. A value outside `i32` is reported at the value.
/// Shared by hexes and deposits; the message names the entry's label.
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

impl MapReader {
    /// Reads one `[[hex]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        let at = reader.location().clone();
        // Read both keys even if the first failed, so neither is falsely reported as
        // unknown.
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
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
    }

    /// The ID the hex at `coord` will have in the finished map, or None if there is none.
    pub fn id(&self, coord: HexCoord) -> Option<HexId> {
        let index = *self.by_coord.get(&coord)?;
        // `finish` gives each hex the ID of its index (and checks that it fits).
        Some(HexId::from_usize(index))
    }

    /// The finished map.
    pub fn finish(self) -> HexMap {
        HexMap::from_coords(self.coords)
    }
}
