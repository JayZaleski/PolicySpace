//! Reading `[[resource]]` and `[[deposit]]` sections. The domain types live in
//! [`crate::resources`]; this module only turns setting entries into them.
//!
//! Resource types are a named kind, so they go through `Definitions` like goods. Deposits
//! have no names: they are identified by resource type and hex, so their reader checks its
//! own duplicates (like hexes). Both refer to other things by name or coordinates, and those
//! references are resolved in phase 2 (`WorldContent::resolve`). Deposits are created
//! unowned; the businesses reader then hands them out with [`DepositsReader::claim`], also
//! in phase 2.

use std::collections::HashMap;

use crate::businesses::BusinessId;
use crate::goods::{GoodId, GoodKind};
use crate::map::{HexCoord, HexId};
use crate::quantity::Quantity;
use crate::resources::{Deposit, Deposits, ResourceId, ResourceKind, ResourceType, ResourceTypes};

use super::defs::Definitions;
use super::error::{ErrorSink, Located, Location};
use super::map::{MapReader, coordinate};
use super::reader::EntryReader;

/// Collects resource types during phase 1, and resolves what they produce in phase 2.
#[derive(Default)]
pub(crate) struct ResourcesReader {
    defs: Definitions<ResourceKind>,
    /// Indexed by `ResourceId::index`; pushed exactly when `defs.define` returns an ID.
    /// None if the key is missing or not a string.
    produces: Vec<Option<Located<String>>>,
    /// Indexed like `produces`. `Quantity::ZERO` is a placeholder if the output is invalid.
    outputs: Vec<Quantity>,
    /// Filled in phase 2, indexed like `produces`.
    produces_ids: Vec<Option<GoodId>>,
}

impl ResourcesReader {
    /// Reads one `[[resource]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        // Read every key before defining anything, even when an earlier one failed, so
        // that no key is falsely reported as unknown.
        let name = reader.required_name("name");
        // Not name-checked: a bad value is simply an unknown good in phase 2.
        let produces = reader.required_string("produces");
        let output = reader
            .required_number("output")
            .and_then(|v| reader.positive_quantity("key 'output' in resource", v));

        // Define the name even if another key was invalid: the load fails anyway, but a
        // later duplicate must still be caught, and a deposit referring to this resource
        // must not look like a reference to an unknown resource.
        if let Some(name) = name
            && self.defs.define(&name, reader.errors()).is_some()
        {
            self.produces.push(produces);
            self.outputs.push(output.unwrap_or(Quantity::ZERO));
        }
    }

    /// Phase 2: resolves each `produces` against the goods.
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink) {
        self.produces_ids = self
            .produces
            .iter()
            .map(|produces| {
                produces
                    .as_ref()
                    .and_then(|produces| goods.resolve(produces, sink))
            })
            .collect();
    }

    /// The definitions, for resolving references to resource types in phase 2.
    pub fn defs(&self) -> &Definitions<ResourceKind> {
        &self.defs
    }

    /// The finished list. Only call this after a successful load, when phase 2 has run
    /// and resolved every `produces`.
    pub fn finish(self) -> ResourceTypes {
        let types = self
            .produces_ids
            .iter()
            .zip(&self.outputs)
            .map(|(produces, output)| ResourceType {
                produces: produces.expect(
                    "internal error: a resource's produces was not resolved in a successful load",
                ),
                output: *output,
            })
            .collect();
        ResourceTypes::from_parts(self.defs.into_registry(), types)
    }
}

/// The most deposits one `[[deposit]]` entry may place.
pub(crate) const MAX_DEPOSIT_COUNT: u32 = 1_000_000;

/// One `[[deposit]]` entry, waiting for phase 2.
struct DepositEntry {
    /// The `[[deposit]]` header.
    at: Location,
    resource: Located<String>,
    coord: HexCoord,
    count: u32,
}

/// Collects deposits during phase 1, and resolves and expands them in phase 2.
#[derive(Default)]
pub(crate) struct DepositsReader {
    entries: Vec<DepositEntry>,
    /// (resource name as written, coordinates) to the header of the first entry.
    /// Lookup only; never iterated.
    seen: HashMap<(String, HexCoord), Location>,
    /// Filled in phase 2.
    deposits: Vec<Deposit>,
}

/// Reads the optional `count` key. Absent gives 1. A count outside `1..=MAX_DEPOSIT_COUNT`
/// is reported at the value and gives None. A count that is not an integer is reported by
/// the reader and also gives 1 (the load fails anyway).
fn deposit_count(reader: &mut EntryReader<'_, '_>) -> Option<u32> {
    let Some(count) = reader.optional_integer("count") else {
        return Some(1);
    };
    if count.value < 1 {
        reader
            .errors()
            .at(&count.at, "key 'count' in deposit must be at least 1");
        return None;
    }
    match u32::try_from(count.value) {
        Ok(n) if n <= MAX_DEPOSIT_COUNT => Some(n),
        _ => {
            reader.errors().at(
                &count.at,
                format!("key 'count' in deposit is out of range (the most is {MAX_DEPOSIT_COUNT})"),
            );
            None
        }
    }
}

impl DepositsReader {
    /// Reads one `[[deposit]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        let at = reader.location().clone();
        // Read every key before returning, so none is falsely reported as unknown.
        let resource = reader.required_string("resource");
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
        let count = deposit_count(reader);

        // An entry with a missing or invalid resource or coordinate is not stored, so it
        // can't produce a duplicate error. The load fails anyway.
        let (Some(resource), Some(q), Some(r)) = (resource, q, r) else {
            return;
        };
        let coord = HexCoord::new(q, r);
        let key = (resource.value.clone(), coord);
        if let Some(first) = self.seen.get(&key) {
            let message = format!(
                "duplicate deposit of '{}' at {coord} (first defined at {first})",
                resource.value
            );
            reader.errors().at(&at, message);
            return;
        }
        self.seen.insert(key, at.clone());
        // With an invalid count, store 1, so phase 2 still checks the references.
        self.entries.push(DepositEntry {
            at,
            resource,
            coord,
            count: count.unwrap_or(1),
        });
    }

    /// Phase 2: resolves each entry's resource type and hex, and expands `count`.
    pub fn resolve(
        &mut self,
        resources: &Definitions<ResourceKind>,
        map: &MapReader,
        sink: &mut ErrorSink,
    ) {
        let mut total: u64 = 0;
        let mut too_many_reported = false;
        for entry in &self.entries {
            let resource = resources.resolve(&entry.resource, sink);
            let hex = map.id(entry.coord);
            if hex.is_none() {
                sink.at(&entry.at, format!("no hex at {}", entry.coord));
            }
            let (Some(resource), Some(hex)) = (resource, hex) else {
                continue;
            };
            if too_many_reported {
                continue;
            }
            total += u64::from(entry.count);
            if total > u64::from(u32::MAX) {
                sink.at(
                    &entry.at,
                    format!("too many deposits in total (the most is {})", u32::MAX),
                );
                too_many_reported = true;
                continue;
            }
            for _ in 0..entry.count {
                self.deposits.push(Deposit {
                    resource,
                    hex,
                    owner: None,
                });
            }
        }
    }

    /// Phase 2, after `resolve`: gives the first `count` unowned deposits of `resource` on
    /// `hex`, in ID order, to `owner`. If fewer than `count` are unowned, assigns none and
    /// returns `Err(available)`.
    pub fn claim(
        &mut self,
        hex: HexId,
        resource: ResourceId,
        count: u32,
        owner: BusinessId,
    ) -> Result<(), u32> {
        let matches = |deposit: &Deposit| {
            deposit.hex == hex && deposit.resource == resource && deposit.owner.is_none()
        };
        // The deposit total fits a `u32` (checked in `resolve`), so the count does too.
        let available = self.deposits.iter().filter(|d| matches(d)).count() as u32;
        if available < count {
            return Err(available);
        }
        for deposit in self
            .deposits
            .iter_mut()
            .filter(|d| matches(d))
            .take(count as usize)
        {
            deposit.owner = Some(owner);
        }
        Ok(())
    }

    /// The finished list. Only call this after a successful load.
    pub fn finish(self, hex_count: usize) -> Deposits {
        Deposits::from_deposits(self.deposits, hex_count)
    }
}

#[cfg(test)]
mod tests {
    use super::super::source::SourceFile;
    use super::*;
    use crate::id::{IdKind, NameRegistry};
    use toml::de::DeTable;

    /// Parses `text`, runs `deposit_count` on it as a "deposit" entry, and returns the
    /// result with the sorted error lines.
    fn run(text: &str) -> (Option<u32>, Vec<String>) {
        let file = SourceFile::new("start/t.toml", text.to_string());
        let (table, parse_errors) = DeTable::parse_recoverable(file.text());
        assert!(parse_errors.is_empty(), "test TOML must be valid");
        let mut sink = ErrorSink::new();
        let entry = file.location(0);
        let count = {
            let mut reader = EntryReader::new(&file, table.get_ref(), entry, "deposit", &mut sink);
            let count = deposit_count(&mut reader);
            reader.finish();
            count
        };
        (count, sink.into_lines())
    }

    #[test]
    fn an_absent_count_is_one() {
        assert_eq!(run(""), (Some(1), vec![]));
    }

    #[test]
    fn counts_from_one_to_the_maximum_are_accepted() {
        assert_eq!(run("count = 1\n"), (Some(1), vec![]));
        assert_eq!(run("count = 7\n"), (Some(7), vec![]));
        assert_eq!(run("count = 1000000\n"), (Some(1_000_000), vec![]));
    }

    #[test]
    fn a_count_above_the_maximum_is_out_of_range() {
        let (count, errors) = run("count = 1000001\n");
        assert_eq!(count, None);
        assert_eq!(
            errors,
            vec!["start/t.toml:1:9: key 'count' in deposit is out of range (the most is 1000000)"]
        );
        let (count, errors) = run("count = 99999999999\n");
        assert_eq!(count, None);
        assert_eq!(
            errors,
            vec!["start/t.toml:1:9: key 'count' in deposit is out of range (the most is 1000000)"]
        );
    }

    #[test]
    fn a_count_below_one_must_be_at_least_one() {
        for text in ["count = 0\n", "count = -1\n"] {
            let (count, errors) = run(text);
            assert_eq!(count, None, "{text}");
            assert_eq!(
                errors,
                vec!["start/t.toml:1:9: key 'count' in deposit must be at least 1"],
                "{text}"
            );
        }
    }

    fn ids<K: IdKind>(count: usize) -> Vec<crate::id::Id<K>> {
        let mut names: NameRegistry<K> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("n{i}")).unwrap())
            .collect()
    }

    fn owners(reader: &DepositsReader) -> Vec<Option<usize>> {
        reader
            .deposits
            .iter()
            .map(|d| d.owner.map(|b| b.index()))
            .collect()
    }

    #[test]
    fn claim_hands_out_the_first_unowned_deposits_in_id_order() {
        let r = ids::<ResourceKind>(2);
        let h = ids::<crate::map::HexKind>(2);
        let b = ids::<crate::businesses::BusinessKind>(2);
        let deposit = |resource: ResourceId| Deposit {
            resource,
            hex: h[0],
            owner: None,
        };
        // Deposits: 0 r0 on h0, 1 r1 on h0, 2 r0 on h0.
        let mut reader = DepositsReader {
            deposits: vec![deposit(r[0]), deposit(r[1]), deposit(r[0])],
            ..Default::default()
        };
        assert_eq!(reader.claim(h[0], r[0], 1, b[0]), Ok(()));
        assert_eq!(owners(&reader), vec![Some(0), None, None]);
        // Too many: nothing changes.
        assert_eq!(reader.claim(h[0], r[0], 2, b[1]), Err(1));
        assert_eq!(owners(&reader), vec![Some(0), None, None]);
        assert_eq!(reader.claim(h[0], r[0], 1, b[1]), Ok(()));
        assert_eq!(owners(&reader), vec![Some(0), None, Some(1)]);
        // No deposits of that type on that hex.
        assert_eq!(reader.claim(h[1], r[0], 1, b[1]), Err(0));
        assert_eq!(owners(&reader), vec![Some(0), None, Some(1)]);
    }
}
