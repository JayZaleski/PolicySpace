//! Reading `[[technology]]` sections. Domain types in `crate::technologies` and
//! `crate::bundle`. Technologies are a named kind, so they go through `Definitions` like
//! POP types; the goods named in `inputs`, `output`, `capital` and `construction` are
//! resolved in phase 2. Every entry is kept for phase 2, even one whose name was bad, so
//! its unknown goods are still reported; only entries with an ID reach `finish`.

use crate::bundle::Bundle;
use crate::goods::{GoodId, GoodKind};
use crate::quantity::Quantity;
use crate::technologies::{Technologies, Technology, TechnologyId, TechnologyKind};

use super::defs::Definitions;
use super::error::{ErrorSink, Located};
use super::reader::EntryReader;

/// What `required_quantity_table` gives: every key, with its amount if it was valid.
type Pairs = Vec<(Located<String>, Option<Quantity>)>;

/// One good = amount table, waiting for phase 2.
struct BundleEntry {
    /// Every key in table order; None where the amount was invalid (already reported).
    amounts: Pairs,
    /// Filled in phase 2, parallel to `amounts`.
    ids: Vec<Option<GoodId>>,
}

impl BundleEntry {
    /// From `required_quantity_table`'s result; a missing or non-table key gives an empty
    /// entry (the load fails anyway).
    fn from_pairs(pairs: Option<Pairs>) -> Self {
        BundleEntry {
            amounts: pairs.unwrap_or_default(),
            ids: Vec::new(),
        }
    }

    /// Phase 2: resolves every key against the goods, always.
    fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink) {
        self.ids = self
            .amounts
            .iter()
            .map(|(name, _)| goods.resolve(name, sink))
            .collect();
    }

    /// Only after a successful load: every good and amount `expect`ed.
    fn finish(&self) -> Bundle {
        const UNRESOLVED: &str =
            "internal error: a technology's goods were not resolved in a successful load";
        Bundle::new(
            self.amounts
                .iter()
                .zip(&self.ids)
                .map(|((_, amount), good)| (good.expect(UNRESOLVED), amount.expect(UNRESOLVED)))
                .collect(),
        )
    }
}

/// One `[[technology]]` entry, waiting for phase 2.
struct TechnologyEntry {
    /// None if the name was missing, invalid or a duplicate.
    id: Option<TechnologyId>,
    inputs: BundleEntry,
    /// None if missing or not a string.
    output: Option<Located<String>>,
    /// None if missing or invalid.
    capacity: Option<Quantity>,
    capital: BundleEntry,
    construction: BundleEntry,
    construction_turns: Option<u32>,
    lifetime: Option<u32>,
    /// Filled in phase 2.
    output_id: Option<GoodId>,
}

/// Collects technologies during phase 1, and resolves their goods in phase 2.
#[derive(Default)]
pub(crate) struct TechnologiesReader {
    defs: Definitions<TechnologyKind>,
    entries: Vec<TechnologyEntry>,
}

/// Reads a whole number of at least 1 that fits a `u32`; anything else is reported.
fn turns(reader: &mut EntryReader<'_, '_>, key: &str) -> Option<u32> {
    let v = reader.required_integer(key)?;
    let turns = u32::try_from(v.value).ok().filter(|n| *n >= 1);
    if turns.is_none() {
        let label = reader.label().to_string();
        let message = if v.value < 1 {
            format!("key '{key}' in {label} must be at least 1")
        } else {
            format!("key '{key}' in {label} is out of range")
        };
        reader.errors().at(&v.at, message);
    }
    turns
}

impl TechnologiesReader {
    /// Reads one `[[technology]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        // Read every key before defining anything, even when an earlier one failed, so
        // that no key is falsely reported as unknown.
        let name = reader.required_name("name");
        let inputs = BundleEntry::from_pairs(reader.required_quantity_table("inputs"));
        // Not name-checked: a bad value is simply an unknown good in phase 2.
        let output = reader.required_string("output");
        let capacity = reader.required_number("capacity").and_then(|v| {
            let prefix = format!("key 'capacity' in {}", reader.label());
            reader.positive_quantity(&prefix, v)
        });
        let capital_pairs = reader.required_quantity_table("capital");
        if capital_pairs.as_ref().is_some_and(|pairs| pairs.is_empty()) {
            let at = reader.location().clone();
            let message = format!("capital of {} needs at least one good", reader.label());
            reader.errors().at(&at, message);
        }
        let capital = BundleEntry::from_pairs(capital_pairs);
        let construction = BundleEntry::from_pairs(reader.required_quantity_table("construction"));
        let construction_turns = turns(reader, "construction_turns");
        let lifetime = turns(reader, "lifetime");

        // The output good may not be among the inputs. Names are compared as written (good
        // names are unique, so equal names are the same good), so this is reported whether
        // or not the goods exist and whether or not the amount was valid.
        if let Some(output) = &output {
            for (key, _) in &inputs.amounts {
                if key.value == output.value {
                    let message = format!(
                        "key '{}' in inputs of {} must not be the output good",
                        key.value,
                        reader.label()
                    );
                    reader.errors().at(&key.at, message);
                }
            }
        }

        let id = name.and_then(|n| self.defs.define(&n, reader.errors()));
        self.entries.push(TechnologyEntry {
            id,
            inputs,
            output,
            capacity,
            capital,
            construction,
            construction_turns,
            lifetime,
            output_id: None,
        });
    }

    /// Phase 2: resolves every good named in every entry, always, so unknown goods are
    /// reported even in an entry with a bad name or bad values.
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink) {
        for entry in &mut self.entries {
            entry.inputs.resolve(goods, sink);
            entry.output_id = entry
                .output
                .as_ref()
                .and_then(|output| goods.resolve(output, sink));
            entry.capital.resolve(goods, sink);
            entry.construction.resolve(goods, sink);
        }
    }

    /// The definitions, for resolving references to technologies in phase 2.
    pub fn defs(&self) -> &Definitions<TechnologyKind> {
        &self.defs
    }

    /// The finished list. Only call this after a successful load, when phase 2 has run and
    /// resolved every good.
    pub fn finish(self) -> Technologies {
        const UNRESOLVED: &str =
            "internal error: a technology was not fully resolved in a successful load";
        let mut technologies = Vec::new();
        for entry in self.entries {
            if entry.id.is_none() {
                continue;
            }
            technologies.push(Technology::new(
                entry.inputs.finish(),
                entry.output_id.expect(UNRESOLVED),
                entry.capacity.expect(UNRESOLVED),
                entry.capital.finish(),
                entry.construction.finish(),
                entry.construction_turns.expect(UNRESOLVED),
                entry.lifetime.expect(UNRESOLVED),
            ));
        }
        Technologies::from_parts(self.defs.into_registry(), technologies)
    }
}
