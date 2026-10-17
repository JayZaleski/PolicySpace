//! Reading `[[business]]` sections. Domain types in `crate::businesses`. Businesses are a
//! named kind in `start/`, so they go through `Definitions` like technologies; the goods in
//! `inventory`, the resource types in `deposits` and the hex are resolved in phase 2, where
//! each business also claims its deposits from `DepositsReader`. Every entry is kept for
//! phase 2, even one whose name was bad, so its unknown goods and resource types are still
//! reported; only entries with an ID reach `finish`, and only they claim deposits.
//! Factories refer to businesses by name and must sit on their owner's hex, which the
//! factories reader checks through `BusinessesReader::hex_of`.

use crate::bundle::Bundle;
use crate::businesses::{Business, BusinessId, BusinessKind, Businesses};
use crate::goods::{GoodId, GoodKind};
use crate::map::{HexCoord, HexId};
use crate::quantity::Quantity;
use crate::resources::{ResourceId, ResourceKind};

use super::defs::Definitions;
use super::error::{ErrorSink, Located, Location};
use super::map::{MapReader, coordinate};
use super::reader::EntryReader;
use super::resources::DepositsReader;

/// Every `inventory` key with its amount if it was valid (from `required_quantity_table`).
type Pairs = Vec<(Located<String>, Option<Quantity>)>;
/// Every `deposits` key with its count if it was valid.
type Counts = Vec<(Located<String>, Option<u32>)>;

/// One `[[business]]` entry, waiting for phase 2.
struct BusinessEntry {
    /// The `[[business]]` header.
    at: Location,
    /// None if the name was missing, invalid or a duplicate.
    id: Option<BusinessId>,
    /// None if `q` or `r` was missing or invalid (already reported).
    coord: Option<HexCoord>,
    inventory: Pairs,
    deposits: Counts,
    /// None if missing or invalid (already reported).
    decider: Option<Located<String>>,
    /// Filled in phase 2.
    hex: Option<HexId>,
    /// Filled in phase 2, parallel to `inventory`.
    inventory_ids: Vec<Option<GoodId>>,
    /// Filled in phase 2, parallel to `deposits`.
    deposit_ids: Vec<Option<ResourceId>>,
}

/// Collects businesses during phase 1, and resolves them and claims their deposits in
/// phase 2.
#[derive(Default)]
pub(crate) struct BusinessesReader {
    defs: Definitions<BusinessKind>,
    entries: Vec<BusinessEntry>,
}

/// Reads the required `deposits` table of resource type = count. A count below 1 or beyond
/// a `u32` is reported at the value and becomes None, as does one the reader already
/// reported; the keys are always kept, so phase 2 still checks the resource types.
fn read_deposits(reader: &mut EntryReader<'_, '_>) -> Counts {
    let label = reader.label().to_string();
    let mut counts = Vec::new();
    for (key, value) in reader
        .required_integer_table("deposits")
        .unwrap_or_default()
    {
        let count = value.and_then(|v| {
            if v.value < 1 {
                let message = format!(
                    "key '{}' in deposits of {label} must be at least 1",
                    key.value
                );
                reader.errors().at(&v.at, message);
                return None;
            }
            match u32::try_from(v.value) {
                Ok(n) => Some(n),
                Err(_) => {
                    let message =
                        format!("key '{}' in deposits of {label} is out of range", key.value);
                    reader.errors().at(&v.at, message);
                    None
                }
            }
        });
        counts.push((key, count));
    }
    counts
}

impl BusinessesReader {
    /// Reads one `[[business]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        // Read every key before defining anything, even when an earlier one failed, so
        // that no key is falsely reported as unknown.
        let at = reader.location().clone();
        let name = reader.required_name("name");
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
        // A missing or non-table key is reported there and gives an empty list; the load
        // fails anyway.
        let inventory = reader
            .required_quantity_table("inventory")
            .unwrap_or_default();
        let deposits = read_deposits(reader);
        let decider = reader.required_name("decider");

        let id = name.and_then(|n| self.defs.define(&n, reader.errors()));
        self.entries.push(BusinessEntry {
            at,
            id,
            coord: q.zip(r).map(|(q, r)| HexCoord::new(q, r)),
            inventory,
            deposits,
            decider,
            hex: None,
            inventory_ids: Vec::new(),
            deposit_ids: Vec::new(),
        });
    }

    /// Phase 2: resolves every good, resource type and hex named in every entry, always, so
    /// unknown references are reported even in an entry with a bad name or bad values; then
    /// each entry with an ID and a valid hex claims its deposits, in load order.
    pub fn resolve(
        &mut self,
        goods: &Definitions<GoodKind>,
        resources: &Definitions<ResourceKind>,
        map: &MapReader,
        deposits: &mut DepositsReader,
        sink: &mut ErrorSink,
    ) {
        for entry in &mut self.entries {
            entry.inventory_ids = entry
                .inventory
                .iter()
                .map(|(name, _)| goods.resolve(name, sink))
                .collect();
            entry.deposit_ids = entry
                .deposits
                .iter()
                .map(|(name, _)| resources.resolve(name, sink))
                .collect();
            entry.hex = entry.coord.and_then(|coord| {
                let hex = map.id(coord);
                if hex.is_none() {
                    sink.at(&entry.at, format!("no hex at {coord}"));
                }
                hex
            });

            let (Some(id), Some(hex)) = (entry.id, entry.hex) else {
                continue;
            };
            for ((key, count), resource) in entry.deposits.iter().zip(&entry.deposit_ids) {
                let (Some(count), Some(resource)) = (*count, *resource) else {
                    continue;
                };
                if let Err(available) = deposits.claim(hex, resource, count, id) {
                    let message = format!(
                        "key '{}' in deposits of business claims more deposits than the hex has unowned ({count} claimed, {available} available)",
                        key.value
                    );
                    sink.at(&key.at, message);
                }
            }
        }
    }

    /// The definitions, for resolving references to businesses in phase 2.
    pub fn defs(&self) -> &Definitions<BusinessKind> {
        &self.defs
    }

    /// After `resolve`: the hex of the business `id` and the coordinates it was written with,
    /// or None if its coordinates were missing or invalid or matched no hex (already
    /// reported). Factories use it to check that they sit on their owner's hex.
    pub fn hex_of(&self, id: BusinessId) -> Option<(HexId, HexCoord)> {
        // Entries without an ID are skipped, so IDs can't index the list directly.
        let entry = self.entries.iter().find(|entry| entry.id == Some(id))?;
        entry.hex.zip(entry.coord)
    }

    /// The finished list. Only call this after a successful load, when phase 2 has run and
    /// resolved everything.
    pub fn finish(self, hex_count: usize) -> Businesses {
        const UNRESOLVED: &str =
            "internal error: a business was not fully resolved in a successful load";
        let mut list = Vec::new();
        for entry in self.entries {
            if entry.id.is_none() {
                continue;
            }
            let inventory = entry
                .inventory
                .iter()
                .zip(&entry.inventory_ids)
                .map(|((_, amount), good)| (good.expect(UNRESOLVED), amount.expect(UNRESOLVED)))
                .collect();
            list.push(Business {
                hex: entry.hex.expect(UNRESOLVED),
                decider: entry.decider.expect(UNRESOLVED).value,
                starting_inventory: Bundle::new(inventory),
            });
        }
        Businesses::from_parts(self.defs.into_registry(), list, hex_count)
    }
}

#[cfg(test)]
mod tests {
    use super::super::source::SourceFile;
    use super::*;
    use toml::de::DeTable;

    /// Parses `text`, runs `read_deposits` on it as a "business" entry, and returns the
    /// counts (key, value) with the sorted error lines.
    fn run(text: &str) -> (Vec<(String, Option<u32>)>, Vec<String>) {
        let file = SourceFile::new("start/t.toml", text.to_string());
        let (table, parse_errors) = DeTable::parse_recoverable(file.text());
        assert!(parse_errors.is_empty(), "test TOML must be valid");
        let mut sink = ErrorSink::new();
        let entry = file.location(0);
        let counts = {
            let mut reader = EntryReader::new(&file, table.get_ref(), entry, "business", &mut sink);
            let counts = read_deposits(&mut reader);
            reader.finish();
            counts
        };
        let counts = counts.into_iter().map(|(k, v)| (k.value, v)).collect();
        (counts, sink.into_lines())
    }

    #[test]
    fn deposit_counts_are_whole_numbers_of_at_least_one_that_fit_a_u32() {
        let (counts, errors) =
            run("deposits = { a = 1, b = 0, c = -3, d = 4294967296, e = 2.5 }\n");
        assert_eq!(
            counts,
            vec![
                ("a".to_string(), Some(1)),
                ("b".to_string(), None),
                ("c".to_string(), None),
                ("d".to_string(), None),
                ("e".to_string(), None),
            ]
        );
        assert_eq!(
            errors,
            vec![
                "start/t.toml:1:25: key 'b' in deposits of business must be at least 1",
                "start/t.toml:1:32: key 'c' in deposits of business must be at least 1",
                "start/t.toml:1:40: key 'd' in deposits of business is out of range",
                "start/t.toml:1:56: key 'e' in deposits of business must be an integer, found float",
            ]
        );
    }

    #[test]
    fn the_largest_u32_count_is_accepted() {
        let (counts, errors) = run("deposits = { a = 4294967295 }\n");
        assert_eq!(counts, vec![("a".to_string(), Some(u32::MAX))]);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn a_missing_deposits_key_gives_an_empty_list_and_the_missing_key_error() {
        let (counts, errors) = run("");
        assert!(counts.is_empty());
        assert_eq!(
            errors,
            vec!["start/t.toml:1:1: missing required key 'deposits' in business"]
        );
    }
}
