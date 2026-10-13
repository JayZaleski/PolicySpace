//! The map: the hex tiles of a setting's starting state.
//!
//! The map is the set of hex tiles in a setting's starting state. A hex carries only its
//! position for now; terrain and other attributes come later.
//!
//! - **Coordinates are axial** (`q`, `r`), see [`HexCoord`]. The engine fixes the
//!   orientation; settings don't choose it. Hexes are **pointy-top**, each value of `r` is
//!   a straight east-west row, `q` increases eastward, and `r` increases southward. Step
//!   07's dev UI draws the map this way.
//! - **A hex is identified by its coordinates.** There are no hex names. Anything saved
//!   long-term stores coordinates, never [`HexId`]s, which are valid only within the
//!   [`HexMap`] that produced them.
//! - **There are no shape rules.** Any set of distinct coordinates is a map (holes,
//!   islands, negative coordinates), and an empty map is valid.
//! - **Neighbors, distance and wrapping don't exist yet.** Wrapping is planned as an
//!   optional per-setting map value (a wrap width along `q`, off by default), possibly with
//!   a north-south rule (torus or pole crossing); the fixed orientation is what makes that
//!   possible. A true geodesic globe is not supported by axial coordinates.
//!
//! This module knows nothing about files: the setting loader builds a [`HexMap`] from a
//! checked setting, and everything else only reads it.

use std::collections::HashMap;
use std::fmt;

use crate::id::{Id, IdKind};

/// Marker kind for hexes.
pub enum HexKind {}

impl IdKind for HexKind {
    const LABEL: &'static str = "hex";
}

/// The ID of a hex.
pub type HexId = Id<HexKind>;

/// Axial coordinates of a hex (see the module docs for the orientation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HexCoord {
    /// Column: increases eastward.
    pub q: i32,
    /// Row: increases southward.
    pub r: i32,
}

impl HexCoord {
    /// Coordinates (q, r).
    pub const fn new(q: i32, r: i32) -> Self {
        HexCoord { q, r }
    }
}

/// Displays as `(q, r)`, e.g. `(1, -2)`. Used in error messages.
impl fmt::Display for HexCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.q, self.r)
    }
}

/// Every hex in a setting, in load order (which is also ID order).
#[derive(Debug, Default)]
pub struct HexMap {
    /// Indexed by `HexId::index`.
    coords: Vec<HexCoord>,
    /// Lookup only; never iterated (iteration order must not depend on hashing).
    by_coord: HashMap<HexCoord, HexId>,
}

impl HexMap {
    /// Builds the map, giving IDs 0, 1, 2, ... in the order given.
    /// Crate-private: only the setting loader builds a map.
    ///
    /// # Panics
    /// Panics if two entries have the same coordinates (the loader rejects duplicates
    /// first, so this is a programming error), or if there are more than u32::MAX hexes.
    pub(crate) fn from_coords(coords: Vec<HexCoord>) -> Self {
        let mut by_coord = HashMap::with_capacity(coords.len());
        for (index, coord) in coords.iter().enumerate() {
            let index = u32::try_from(index).expect("more than u32::MAX hexes");
            let previous = by_coord.insert(*coord, HexId::from_index(index));
            assert!(previous.is_none(), "duplicate hex at {coord}");
        }
        HexMap { coords, by_coord }
    }

    /// Number of hexes.
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// True if the setting defines no hexes.
    pub fn is_empty(&self) -> bool {
        self.coords.is_empty()
    }

    /// The hex at `coord`, or None if there is none.
    pub fn id(&self, coord: HexCoord) -> Option<HexId> {
        self.by_coord.get(&coord).copied()
    }

    /// The coordinates of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this map.
    pub fn coord(&self, id: HexId) -> HexCoord {
        match self.coords.get(id.index()) {
            Some(coord) => *coord,
            None => panic!(
                "{:?} is not from this map (it has {} hexes)",
                id,
                self.coords.len()
            ),
        }
    }

    /// Every hex as (ID, coordinates), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (HexId, HexCoord)> + '_ {
        self.coords.iter().enumerate().map(|(index, coord)| {
            // `from_coords` checked that every index fits in a u32.
            (HexId::from_index(index as u32), *coord)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> HexMap {
        HexMap::from_coords(vec![
            HexCoord::new(0, 0),
            HexCoord::new(1, -1),
            HexCoord::new(-3, 2),
        ])
    }

    #[test]
    fn from_coords_gives_sequential_ids_and_working_lookups() {
        let map = sample();
        assert_eq!(map.len(), 3);
        assert!(!map.is_empty());
        let listed: Vec<(usize, HexCoord)> =
            map.iter().map(|(id, coord)| (id.index(), coord)).collect();
        assert_eq!(
            listed,
            vec![
                (0, HexCoord::new(0, 0)),
                (1, HexCoord::new(1, -1)),
                (2, HexCoord::new(-3, 2)),
            ]
        );
        for (id, coord) in map.iter() {
            assert_eq!(map.id(coord), Some(id));
            assert_eq!(map.coord(id), coord);
        }
        assert_eq!(map.id(HexCoord::new(9, 9)), None);
    }

    #[test]
    fn the_default_map_is_empty() {
        let map = HexMap::default();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert_eq!(map.iter().count(), 0);
        assert_eq!(map.id(HexCoord::new(0, 0)), None);
        assert!(HexMap::from_coords(Vec::new()).is_empty());
    }

    #[test]
    fn coordinates_display_as_q_r() {
        assert_eq!(HexCoord::new(1, -2).to_string(), "(1, -2)");
        assert_eq!(HexCoord::new(0, 0).to_string(), "(0, 0)");
    }

    #[test]
    #[should_panic(expected = "is not from this map")]
    fn coord_panics_on_a_foreign_id() {
        sample().coord(HexId::from_index(3));
    }

    #[test]
    #[should_panic(expected = "duplicate hex")]
    fn from_coords_panics_on_duplicate_coordinates() {
        HexMap::from_coords(vec![
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 0),
        ]);
    }
}
