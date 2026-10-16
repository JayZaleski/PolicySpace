//! Reading `[[market]]` sections. Domain types in [`crate::market`]. A market is identified
//! by its hex, so the reader checks its own duplicates (like deposits); its hex and the
//! goods in `prices` are resolved in phase 2, where it also checks that the hex's currency
//! isn't priced.

use std::collections::{BTreeMap, HashMap};

use crate::goods::{GoodId, GoodKind};
use crate::map::HexCoord;
use crate::market::HexPrices;
use crate::quantity::Quantity;

use super::defs::Definitions;
use super::error::{ErrorSink, Located, Location};
use super::map::{MapReader, coordinate};
use super::reader::EntryReader;

/// One `[[market]]` entry, waiting for phase 2.
struct MarketEntry {
    /// The `[[market]]` header.
    at: Location,
    coord: HexCoord,
    /// Only the valid prices, in table order.
    prices: Vec<(Located<String>, Quantity)>,
}

/// Collects markets during phase 1, and resolves them in phase 2.
#[derive(Default)]
pub(crate) struct MarketsReader {
    entries: Vec<MarketEntry>,
    /// Coordinates to the header of the first entry. Lookup only; never iterated.
    seen: HashMap<HexCoord, Location>,
    /// Filled in phase 2, by hex index.
    lists: Vec<Option<BTreeMap<GoodId, Quantity>>>,
}

impl MarketsReader {
    /// Reads one `[[market]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        let at = reader.location().clone();
        // Read every key before returning, so none is falsely reported as unknown.
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
        let pairs = reader.required_number_table("prices");

        let mut prices = Vec::new();
        for (name, number) in pairs.unwrap_or_default() {
            let Some(number) = number else {
                continue;
            };
            let prefix = format!("key '{}' in prices of market", name.value);
            if let Some(price) = reader.positive_quantity(&prefix, number) {
                prices.push((name, price));
            }
        }

        // An entry with a missing or invalid coordinate is not stored, so it can't produce
        // a duplicate error. The load fails anyway.
        let (Some(q), Some(r)) = (q, r) else {
            return;
        };
        let coord = HexCoord::new(q, r);
        if let Some(first) = self.seen.get(&coord) {
            let message = format!("duplicate market at {coord} (first defined at {first})");
            reader.errors().at(&at, message);
            return;
        }
        self.seen.insert(coord, at.clone());
        self.entries.push(MarketEntry { at, coord, prices });
    }

    /// Phase 2: resolves each entry's hex and the goods it prices, and checks that no
    /// market prices its hex's currency.
    pub fn resolve(
        &mut self,
        goods: &Definitions<GoodKind>,
        map: &MapReader,
        sink: &mut ErrorSink,
    ) {
        self.lists = vec![None; map.len()];
        for entry in &self.entries {
            let hex = map.id(entry.coord);
            if hex.is_none() {
                sink.at(&entry.at, format!("no hex at {}", entry.coord));
            }
            if let Some(hex) = hex {
                // An entry with no valid prices is still a market.
                self.lists[hex.index()].get_or_insert_with(BTreeMap::new);
            }
            for (name, price) in &entry.prices {
                // Always resolve, so unknown goods are reported even without a hex.
                let id = goods.resolve(name, sink);
                let (Some(hex), Some(id)) = (hex, id) else {
                    continue;
                };
                // If the hex's currency didn't resolve, its own error is already reported.
                if map.currency(hex) == Some(id) {
                    sink.at(
                        &name.at,
                        format!(
                            "'{}' is the currency at {}, so it can't have a price",
                            name.value, entry.coord
                        ),
                    );
                } else {
                    self.lists[hex.index()]
                        .get_or_insert_with(BTreeMap::new)
                        .insert(id, *price);
                }
            }
        }
    }

    /// The finished prices. Only call this after a successful load.
    pub fn finish(self) -> HexPrices {
        HexPrices::from_vec(self.lists)
    }
}
