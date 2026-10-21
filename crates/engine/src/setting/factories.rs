//! Reading `[[factory]]` sections. Domain types in `crate::factories`. Factories have no
//! names and no duplicate check: any number of identical factories may sit on one hex. A
//! factory refers to a technology and a business by name and to its hex by coordinates;
//! all three are resolved in phase 2, where the loader also checks that the hex is the
//! owner's (through `BusinessesReader::hex_of`) and expands `count` into that many
//! factories, like deposits. Every entry is kept for phase 2, even one with a bad
//! condition or count, so its references are still checked; only entries whose every key
//! is valid and whose references all resolve produce factories.

use crate::factories::{Factories, Factory};
use crate::map::HexCoord;
use crate::technologies::TechnologyKind;

use super::businesses::BusinessesReader;
use super::defs::Definitions;
use super::error::{ErrorSink, Located, Location};
use super::map::{MapReader, coordinate, count};
use super::reader::EntryReader;

/// One `[[factory]]` entry, waiting for phase 2.
struct FactoryEntry {
    /// The `[[factory]]` header.
    at: Location,
    /// None if `q` or `r` was missing or invalid (already reported).
    coord: Option<HexCoord>,
    /// None if missing or not a string (already reported).
    technology: Option<Located<String>>,
    /// None if missing or not a string (already reported).
    owner: Option<Located<String>>,
    /// 1 if absent; None if invalid (already reported).
    condition: Option<f64>,
    /// 1 if absent; None if invalid (already reported).
    count: Option<u32>,
}

/// Collects factories during phase 1, and resolves and expands them in phase 2.
#[derive(Default)]
pub(crate) struct FactoriesReader {
    entries: Vec<FactoryEntry>,
    /// Filled in phase 2.
    factories: Vec<Factory>,
}

/// Reads the optional `condition` key: the share of the technology's capital bundle the
/// factory holds. Absent gives 1. A value that is not a finite number greater than 0 and
/// at most 1 is reported at the value and gives None; so does a non-number (reported by the
/// reader).
fn condition(reader: &mut EntryReader<'_, '_>) -> Option<f64> {
    if !reader.has("condition") {
        return Some(1.0);
    }
    let value = reader.optional_number("condition")?;
    if value.value.is_finite() && value.value > 0.0 && value.value <= 1.0 {
        Some(value.value)
    } else {
        let message = format!(
            "key 'condition' in {} must be a finite number greater than 0 and at most 1",
            reader.label()
        );
        reader.errors().at(&value.at, message);
        None
    }
}

impl FactoriesReader {
    /// Reads one `[[factory]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        let at = reader.location().clone();
        // Read every key before deciding anything, even when an earlier one failed, so
        // none is falsely reported as unknown.
        let q = coordinate(reader, "q");
        let r = coordinate(reader, "r");
        // Not name-checked: a bad value is simply an unknown name in phase 2.
        let technology = reader.required_string("technology");
        let owner = reader.required_string("owner");
        let condition = condition(reader);
        let count = count(reader);

        // Keep the entry whatever failed, so phase 2 still checks its references.
        self.entries.push(FactoryEntry {
            at,
            coord: q.zip(r).map(|(q, r)| HexCoord::new(q, r)),
            technology,
            owner,
            condition,
            count,
        });
    }

    /// Phase 2: resolves each entry's technology, owner and hex, checks the hex is the
    /// owner's, and expands `count`. Run after `BusinessesReader::resolve`, which fills in
    /// the hexes of the businesses.
    pub fn resolve(
        &mut self,
        technologies: &Definitions<TechnologyKind>,
        businesses: &BusinessesReader,
        map: &MapReader,
        sink: &mut ErrorSink,
    ) {
        let mut total: u64 = 0;
        let mut too_many_reported = false;
        for entry in &self.entries {
            let technology = entry
                .technology
                .as_ref()
                .and_then(|technology| technologies.resolve(technology, sink));
            let owner = entry
                .owner
                .as_ref()
                .and_then(|owner| businesses.defs().resolve(owner, sink));
            let hex = entry.coord.and_then(|coord| map.id(coord));
            if let (Some(coord), None) = (entry.coord, hex) {
                sink.at(&entry.at, format!("no hex at {coord}"));
            }

            // The factory must sit on its owner's hex. If the owner's own hex didn't
            // resolve, that entry already reported it, and there is nothing to check.
            let mut off_owners_hex = false;
            if let (Some(owner), Some(hex), Some(coord), Some(owner_name)) =
                (owner, hex, entry.coord, entry.owner.as_ref())
                && let Some((owner_hex, owner_coord)) = businesses.hex_of(owner)
                && owner_hex != hex
            {
                sink.at(
                    &entry.at,
                    format!(
                        "factory at {coord} is not on the hex of its owner '{}' at {owner_coord}",
                        owner_name.value
                    ),
                );
                off_owners_hex = true;
            }

            let (Some(technology), Some(owner), Some(hex), Some(condition), Some(count)) =
                (technology, owner, hex, entry.condition, entry.count)
            else {
                continue;
            };
            if off_owners_hex || too_many_reported {
                continue;
            }
            total += u64::from(count);
            if total > u64::from(u32::MAX) {
                sink.at(
                    &entry.at,
                    format!("too many factories in total (the most is {})", u32::MAX),
                );
                too_many_reported = true;
                continue;
            }
            for _ in 0..count {
                self.factories.push(Factory {
                    hex,
                    technology,
                    owner,
                    condition,
                });
            }
        }
    }

    /// The finished list. Only call this after a successful load.
    pub fn finish(self, hex_count: usize, business_count: usize) -> Factories {
        Factories::from_factories(self.factories, hex_count, business_count)
    }
}

#[cfg(test)]
mod tests {
    use super::super::source::SourceFile;
    use super::*;
    use toml::de::DeTable;

    /// Parses `text`, runs `condition` on it as a "factory" entry, and returns the result
    /// with the sorted error lines.
    fn run(text: &str) -> (Option<f64>, Vec<String>) {
        let file = SourceFile::new("start/t.toml", text.to_string());
        let (table, parse_errors) = DeTable::parse_recoverable(file.text());
        assert!(parse_errors.is_empty(), "test TOML must be valid");
        let mut sink = ErrorSink::new();
        let entry = file.location(0);
        let value = {
            let mut reader = EntryReader::new(&file, table.get_ref(), entry, "factory", &mut sink);
            let value = condition(&mut reader);
            reader.finish();
            value
        };
        (value, sink.into_lines())
    }

    const BAD_RANGE: &str = "start/t.toml:1:13: key 'condition' in factory must be a finite number greater than 0 and at most 1";

    #[test]
    fn an_absent_condition_is_one() {
        assert_eq!(run(""), (Some(1.0), vec![]));
    }

    #[test]
    fn conditions_above_0_and_up_to_1_are_accepted() {
        assert_eq!(run("condition = 0.5\n"), (Some(0.5), vec![]));
        assert_eq!(run("condition = 1\n"), (Some(1.0), vec![]));
        assert_eq!(run("condition = 1.0\n"), (Some(1.0), vec![]));
        assert_eq!(run("condition = 0.0001\n"), (Some(0.0001), vec![]));
    }

    #[test]
    fn conditions_outside_the_range_are_reported() {
        for text in [
            "condition = 0\n",
            "condition = -0.5\n",
            "condition = 1.5\n",
            "condition = inf\n",
            "condition = nan\n",
        ] {
            assert_eq!(run(text), (None, vec![BAD_RANGE.to_string()]), "{text}");
        }
    }

    #[test]
    fn a_condition_that_is_not_a_number_is_reported() {
        assert_eq!(
            run("condition = \"full\"\n"),
            (
                None,
                vec![
                    "start/t.toml:1:13: key 'condition' in factory must be a number, found string"
                        .to_string()
                ]
            )
        );
    }
}
