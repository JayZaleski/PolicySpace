//! Reading the fields of one TOML table (a `[[section]]` entry, or the manifest).
//!
//! [`EntryReader`] records every problem it finds (missing keys, wrong types, bad names,
//! unknown keys) and keeps going, so one load can report all of them. Later Steps define
//! their section types with it.

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use super::error::{ErrorSink, Located, Location};
use super::source::SourceFile;

/// True if `name` is a valid text name: an ASCII lowercase letter first, then only ASCII
/// lowercase letters, digits, and `_`.
pub(crate) fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Where a "missing key" error goes.
enum MissingAt {
    /// At the entry's header.
    Entry,
    /// On the whole file (used by the manifest, which has no header).
    File,
}

/// Reads the keys of one table, reporting every problem into an [`ErrorSink`].
pub(crate) struct EntryReader<'a, 'i> {
    file: &'a SourceFile,
    table: &'a DeTable<'i>,
    entry: Location,
    missing_at: MissingAt,
    label: &'static str,
    read: Vec<String>,
    sink: &'a mut ErrorSink,
}

/// What an accessor found for a key.
enum Found<'a, 'i> {
    Value(&'a Spanned<DeValue<'i>>),
    /// The key is absent; already reported if it was required.
    Absent,
}

impl<'a, 'i> EntryReader<'a, 'i> {
    /// A reader for one `[[section]]` entry. `entry` is the position of its header.
    pub fn new(
        file: &'a SourceFile,
        table: &'a DeTable<'i>,
        entry: Location,
        label: &'static str,
        sink: &'a mut ErrorSink,
    ) -> Self {
        EntryReader {
            file,
            table,
            entry,
            missing_at: MissingAt::Entry,
            label,
            read: Vec::new(),
            sink,
        }
    }

    /// A reader for the manifest's root table (label `manifest`). Its location is the
    /// start of the file, and missing keys are reported on the whole file.
    pub fn for_manifest(
        file: &'a SourceFile,
        table: &'a DeTable<'i>,
        sink: &'a mut ErrorSink,
    ) -> Self {
        let entry = file.location(0);
        EntryReader {
            file,
            table,
            entry,
            missing_at: MissingAt::File,
            label: "manifest",
            read: Vec::new(),
            sink,
        }
    }

    /// The entry's header (or the start of the file, for the manifest).
    // Used by section types from Step 03 on; until then only by tests.
    #[allow(dead_code)]
    pub fn location(&self) -> &Location {
        &self.entry
    }

    /// The error collector, for section-specific checks and `Definitions`.
    pub fn errors(&mut self) -> &mut ErrorSink {
        self.sink
    }

    fn find(&mut self, key: &str, required: bool) -> Found<'a, 'i> {
        self.read.push(key.to_string());
        let table: &'a DeTable<'i> = self.table;
        for (k, v) in table.iter() {
            if k.get_ref().as_ref() == key {
                return Found::Value(v);
            }
        }
        if required {
            let message = format!("missing required key '{key}' in {}", self.label);
            match self.missing_at {
                MissingAt::Entry => self.sink.at(&self.entry, message),
                MissingAt::File => self.sink.in_file(&self.entry.path, message),
            }
        }
        Found::Absent
    }

    fn wrong_type(&mut self, key: &str, expected: &str, value: &Spanned<DeValue<'i>>) {
        let at = self.file.location_of(value.span());
        self.sink.at(
            &at,
            format!(
                "key '{key}' in {} must be {expected}, found {}",
                self.label,
                value.get_ref().type_str()
            ),
        );
    }

    fn string(&mut self, key: &str, required: bool) -> Option<Located<String>> {
        let Found::Value(value) = self.find(key, required) else {
            return None;
        };
        match value.get_ref().as_str() {
            Some(s) => Some(Located {
                value: s.to_string(),
                at: self.file.location_of(value.span()),
            }),
            None => {
                self.wrong_type(key, "a string", value);
                None
            }
        }
    }

    fn integer(&mut self, key: &str, required: bool) -> Option<Located<i64>> {
        let Found::Value(value) = self.find(key, required) else {
            return None;
        };
        match value.get_ref().as_integer() {
            Some(int) => {
                let at = self.file.location_of(value.span());
                match i64::from_str_radix(int.as_str(), int.radix()) {
                    Ok(n) => Some(Located { value: n, at }),
                    Err(_) => {
                        self.sink.at(
                            &at,
                            format!("key '{key}' in {} is out of range", self.label),
                        );
                        None
                    }
                }
            }
            None => {
                self.wrong_type(key, "an integer", value);
                None
            }
        }
    }

    fn boolean(&mut self, key: &str, required: bool) -> Option<Located<bool>> {
        let Found::Value(value) = self.find(key, required) else {
            return None;
        };
        match value.get_ref().as_bool() {
            Some(b) => Some(Located {
                value: b,
                at: self.file.location_of(value.span()),
            }),
            None => {
                self.wrong_type(key, "a boolean", value);
                None
            }
        }
    }

    /// A string value that must be present.
    pub fn required_string(&mut self, key: &str) -> Option<Located<String>> {
        self.string(key, true)
    }

    /// A string value that may be absent.
    pub fn optional_string(&mut self, key: &str) -> Option<Located<String>> {
        self.string(key, false)
    }

    /// A string value that must be present and be a valid name (see [`valid_name`]).
    // Used by section types from Step 03 on; until then only by tests.
    #[allow(dead_code)]
    pub fn required_name(&mut self, key: &str) -> Option<Located<String>> {
        let name = self.required_string(key)?;
        if valid_name(&name.value) {
            Some(name)
        } else {
            self.sink.at(
                &name.at,
                format!(
                    "invalid name '{}' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)",
                    name.value
                ),
            );
            None
        }
    }

    /// An integer value that must be present.
    pub fn required_integer(&mut self, key: &str) -> Option<Located<i64>> {
        self.integer(key, true)
    }

    /// An integer value that may be absent.
    // Used by section types from Step 03 on; until then only by tests.
    #[allow(dead_code)]
    pub fn optional_integer(&mut self, key: &str) -> Option<Located<i64>> {
        self.integer(key, false)
    }

    /// A boolean value that must be present.
    // Used by section types from Step 03 on; until then only by tests.
    #[allow(dead_code)]
    pub fn required_bool(&mut self, key: &str) -> Option<Located<bool>> {
        self.boolean(key, true)
    }

    /// A boolean value that may be absent.
    // Used by section types from Step 03 on; until then only by tests.
    #[allow(dead_code)]
    pub fn optional_bool(&mut self, key: &str) -> Option<Located<bool>> {
        self.boolean(key, false)
    }

    /// Reports every key of the table that no accessor asked for, at the key's position.
    pub fn finish(self) {
        let EntryReader {
            file,
            table,
            label,
            read,
            sink,
            ..
        } = self;
        for (k, _) in table.iter() {
            let key: &str = k.get_ref().as_ref();
            if !read.iter().any(|r| r == key) {
                sink.at(
                    &file.location_of(k.span()),
                    format!("unknown key '{key}' in {label}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name_accepts_and_rejects() {
        for ok in ["copper_ore", "grain2", "a", "a_b_1"] {
            assert!(valid_name(ok), "{ok} should be valid");
        }
        for bad in [
            "Copper",
            "copper-ore",
            "2grain",
            "_copper",
            "",
            "c\u{f6}pper",
            "copper ore",
        ] {
            assert!(!valid_name(bad), "{bad:?} should be invalid");
        }
    }

    /// Parses `text`, runs `f` with a reader over the whole file as a "thing" entry, and
    /// returns the sorted error lines.
    fn run(text: &str, f: impl FnOnce(&mut EntryReader<'_, '_>)) -> Vec<String> {
        let file = SourceFile::new("rules/t.toml", text.to_string());
        let (table, parse_errors) = DeTable::parse_recoverable(file.text());
        assert!(parse_errors.is_empty(), "test TOML must be valid");
        let mut sink = ErrorSink::new();
        let entry = file.location(0);
        {
            let mut reader = EntryReader::new(&file, table.get_ref(), entry, "thing", &mut sink);
            f(&mut reader);
            reader.finish();
        }
        match sink.into_result(()) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn strings_present_missing_and_wrong_type() {
        let errors = run("a = \"x\"\nb = 5\n", |r| {
            let a = r.required_string("a").unwrap();
            assert_eq!(a.value, "x");
            assert_eq!((a.at.line, a.at.column), (1, 5));
            assert!(r.required_string("b").is_none());
            assert!(r.required_string("c").is_none());
            assert!(r.optional_string("d").is_none());
            assert!(r.optional_string("a").is_some());
        });
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'c' in thing",
                "rules/t.toml:2:5: key 'b' in thing must be a string, found integer",
            ]
        );
    }

    #[test]
    fn names_are_checked() {
        let errors = run("a = \"copper\"\nb = \"Copper\"\n", |r| {
            assert_eq!(r.required_name("a").unwrap().value, "copper");
            assert!(r.required_name("b").is_none());
        });
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:2:5: invalid name 'Copper' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)"
            ]
        );
    }

    #[test]
    fn integers_present_wrong_type_and_out_of_range() {
        let errors = run(
            "a = 7\nb = \"x\"\nc = 99999999999999999999\nd = 0x10\ne = -3\n",
            |r| {
                assert_eq!(r.required_integer("a").unwrap().value, 7);
                assert!(r.required_integer("b").is_none());
                assert!(r.optional_integer("c").is_none());
                assert_eq!(r.optional_integer("d").unwrap().value, 16);
                assert_eq!(r.optional_integer("e").unwrap().value, -3);
                assert!(r.optional_integer("missing").is_none());
                assert!(r.required_integer("missing2").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing2' in thing",
                "rules/t.toml:2:5: key 'b' in thing must be an integer, found string",
                "rules/t.toml:3:5: key 'c' in thing is out of range",
            ]
        );
    }

    #[test]
    fn booleans_present_wrong_type() {
        let errors = run("a = true\nb = 1\n", |r| {
            assert!(r.required_bool("a").unwrap().value);
            assert!(r.optional_bool("b").is_none());
            assert!(r.optional_bool("zzz").is_none());
            assert!(r.required_bool("yyy").is_none());
        });
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'yyy' in thing",
                "rules/t.toml:2:5: key 'b' in thing must be a boolean, found integer",
            ]
        );
    }

    #[test]
    fn finish_reports_only_unread_keys() {
        let errors = run("a = 1\nb = 2\nc = 3\n", |r| {
            r.optional_integer("a");
            // Reading a key with a wrong type still marks it as read.
            r.optional_string("b");
        });
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:2:5: key 'b' in thing must be a string, found integer",
                "rules/t.toml:3:1: unknown key 'c' in thing",
            ]
        );
    }

    #[test]
    fn manifest_reader_reports_missing_keys_on_the_file() {
        let file = SourceFile::new("setting.toml", "x = 1\n".to_string());
        let (table, _) = DeTable::parse_recoverable(file.text());
        let mut sink = ErrorSink::new();
        {
            let mut reader = EntryReader::for_manifest(&file, table.get_ref(), &mut sink);
            assert_eq!(reader.location().line, 1);
            assert!(reader.required_string("name").is_none());
            reader.finish();
        }
        let errors = sink.into_result(()).unwrap_err();
        let shown: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(
            shown,
            vec![
                "setting.toml: missing required key 'name' in manifest",
                "setting.toml:1:1: unknown key 'x' in manifest",
            ]
        );
    }
}
