//! Reading `[[pop_type]]` sections. Domain types in `crate::pops`. POP types are a named
//! kind, so they go through `Definitions` like resource types; the goods named in
//! `endowment` and `utility` are resolved in phase 2. Every entry is kept for phase 2,
//! even one whose name was bad, so its unknown goods are still reported; only entries
//! with an ID reach `finish`.

use crate::goods::{GoodId, GoodKind};
use crate::pops::{PopId, PopKind, PopType, PopTypes};
use crate::quantity::Quantity;

use super::defs::Definitions;
use super::error::{ErrorSink, Located};
use super::reader::EntryReader;

/// One utility key and, if it was valid, its (subsistence, weight as written).
type UtilityEntry = (Located<String>, Option<(Quantity, f64)>);

/// One `[[pop_type]]` entry, waiting for phase 2.
struct PopEntry {
    /// None if the name was missing, invalid or a duplicate.
    id: Option<PopId>,
    /// Every endowment key, in table order; None where the amount was invalid.
    endowment: Vec<(Located<String>, Option<Quantity>)>,
    /// Every utility key, in table order; None where the sub-table or a value was invalid.
    utility: Vec<UtilityEntry>,
    /// Filled in phase 2, parallel to `endowment`.
    endowment_ids: Vec<Option<GoodId>>,
    /// Filled in phase 2, parallel to `utility`.
    utility_ids: Vec<Option<GoodId>>,
}

/// Collects POP types during phase 1, and resolves their goods in phase 2.
#[derive(Default)]
pub(crate) struct PopTypesReader {
    defs: Definitions<PopKind>,
    entries: Vec<PopEntry>,
}

/// What `subsistence` or `share` of a utility good turned out to be.
enum Amount {
    /// The key is absent (it defaults to 0).
    Absent,
    /// A finite number of at least 0.
    Valid(f64),
    /// Present but wrong; already reported.
    Invalid,
}

/// Reads the optional `subsistence` or `share` of one utility good.
fn amount(sub: &mut EntryReader<'_, '_>, key: &str) -> Amount {
    if !sub.has(key) {
        return Amount::Absent;
    }
    let Some(value) = sub.optional_number(key) else {
        return Amount::Invalid;
    };
    if value.value.is_finite() && value.value >= 0.0 {
        Amount::Valid(value.value)
    } else {
        let message = format!(
            "key '{key}' in {} must be a finite number of at least 0",
            sub.label()
        );
        sub.errors().at(&value.at, message);
        Amount::Invalid
    }
}

impl PopTypesReader {
    /// Reads one `[[pop_type]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        // Read every key before defining anything, even when an earlier one failed, so
        // that no key is falsely reported as unknown.
        let name = reader.required_name("name");

        let mut endowment = Vec::new();
        for (key, number) in reader
            .required_number_table("endowment")
            .unwrap_or_default()
        {
            let amount = number.and_then(|v| {
                let quantity = Quantity::new(v.value).filter(|q| q.get() > 0.0);
                if quantity.is_none() {
                    let message = format!(
                        "key '{}' in endowment of {} must be a finite number greater than 0",
                        key.value,
                        reader.label()
                    );
                    reader.errors().at(&v.at, message);
                }
                quantity
            });
            endowment.push((key, amount));
        }

        let table = reader.required_table_of_tables("utility");
        let utility_is_table = table.is_some();
        // True if some good has a positive share, or a share we couldn't read (its problem
        // is already reported, so it must not also trigger the "no positive share" error).
        let mut positive_or_unknown = false;
        let mut utility = Vec::new();
        for (key, sub_table) in table.unwrap_or_default() {
            let Some(sub_table) = sub_table else {
                positive_or_unknown = true;
                utility.push((key, None));
                continue;
            };
            let (subsistence, share) = {
                let mut sub = reader.nested("utility", &key, sub_table);
                let subsistence = amount(&mut sub, "subsistence");
                let share = amount(&mut sub, "share");
                sub.finish();
                (subsistence, share)
            };
            let (subsistence, share) = match (subsistence, share) {
                (Amount::Invalid, _) | (_, Amount::Invalid) => {
                    positive_or_unknown = true;
                    utility.push((key, None));
                    continue;
                }
                (subsistence, share) => (
                    match subsistence {
                        Amount::Valid(v) => v,
                        _ => 0.0,
                    },
                    match share {
                        Amount::Valid(v) => v,
                        _ => 0.0,
                    },
                ),
            };
            if subsistence == 0.0 && share == 0.0 {
                let message = format!(
                    "key '{}' in utility of {} must have a positive subsistence or share",
                    key.value,
                    reader.label()
                );
                reader.errors().at(&key.at, message);
                utility.push((key, None));
                continue;
            }
            if share > 0.0 {
                positive_or_unknown = true;
            }
            utility.push((
                key,
                Some((Quantity::new(subsistence).unwrap_or(Quantity::ZERO), share)),
            ));
        }
        if utility_is_table && !positive_or_unknown {
            let at = reader.location().clone();
            let message = format!(
                "utility of {} needs at least one good with a positive share",
                reader.label()
            );
            reader.errors().at(&at, message);
        }

        let id = name.and_then(|n| self.defs.define(&n, reader.errors()));
        self.entries.push(PopEntry {
            id,
            endowment,
            utility,
            endowment_ids: Vec::new(),
            utility_ids: Vec::new(),
        });
    }

    /// Phase 2: resolves every good named in an endowment or a utility, in every entry.
    pub fn resolve(&mut self, goods: &Definitions<GoodKind>, sink: &mut ErrorSink) {
        for entry in &mut self.entries {
            entry.endowment_ids = entry
                .endowment
                .iter()
                .map(|(name, _)| goods.resolve(name, sink))
                .collect();
            entry.utility_ids = entry
                .utility
                .iter()
                .map(|(name, _)| goods.resolve(name, sink))
                .collect();
        }
    }

    /// The finished list. Only call this after a successful load, when phase 2 has run and
    /// resolved every good.
    pub fn finish(self) -> PopTypes {
        const UNRESOLVED: &str =
            "internal error: a POP type's goods were not resolved in a successful load";
        let mut types = Vec::new();
        for entry in self.entries {
            if entry.id.is_none() {
                continue;
            }
            let endowment = entry
                .endowment
                .iter()
                .zip(&entry.endowment_ids)
                .map(|((_, amount), good)| (good.expect(UNRESOLVED), amount.expect(UNRESOLVED)))
                .collect();
            let utility = entry
                .utility
                .iter()
                .zip(&entry.utility_ids)
                .map(|((_, values), good)| {
                    let (subsistence, weight) = values.expect(UNRESOLVED);
                    (good.expect(UNRESOLVED), subsistence, weight)
                })
                .collect();
            types.push(PopType::new(endowment, utility));
        }
        PopTypes::from_parts(self.defs.into_registry(), types)
    }
}
