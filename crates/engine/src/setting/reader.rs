//! Reading the fields of one TOML table (a `[[section]]` entry, or the manifest).
//!
//! [`EntryReader`] records every problem it finds (missing keys, wrong types, bad names,
//! unknown keys) and keeps going, so one load can report all of them. Later Steps define
//! their section types with it.

use std::borrow::Cow;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use super::error::{ErrorSink, Located, Location};
use super::source::SourceFile;
use crate::quantity::Quantity;

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
    label: String,
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
        label: &str,
        sink: &'a mut ErrorSink,
    ) -> Self {
        EntryReader {
            file,
            table,
            entry,
            missing_at: MissingAt::Entry,
            label: label.to_string(),
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
            label: "manifest".to_string(),
            read: Vec::new(),
            sink,
        }
    }

    /// The entry's header (or the start of the file, for the manifest).
    pub fn location(&self) -> &Location {
        &self.entry
    }

    /// The label used in messages ("good", "hex", "manifest", ...).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The error collector, for section-specific checks and `Definitions`.
    pub fn errors(&mut self) -> &mut ErrorSink {
        self.sink
    }

    fn find(&mut self, key: &str, required: bool) -> Found<'a, 'i> {
        match self.find_entry(key, required) {
            Some((_, value)) => Found::Value(value),
            None => Found::Absent,
        }
    }

    /// Like `find`, but also returns the key as the table holds it (so its span is known).
    #[allow(clippy::type_complexity)]
    fn find_entry(
        &mut self,
        key: &str,
        required: bool,
    ) -> Option<(&'a Spanned<Cow<'i, str>>, &'a Spanned<DeValue<'i>>)> {
        self.read.push(key.to_string());
        let table: &'a DeTable<'i> = self.table;
        for (k, v) in table.iter() {
            if k.get_ref().as_ref() == key {
                return Some((k, v));
            }
        }
        if required {
            let message = format!("missing required key '{key}' in {}", self.label);
            match self.missing_at {
                MissingAt::Entry => self.sink.at(&self.entry, message),
                MissingAt::File => self.sink.in_file(&self.entry.path, message),
            }
        }
        None
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
        let prefix = format!("key '{key}' in {}", self.label);
        self.integer_value(&prefix, value)
    }

    /// Parses `value` as an integer (`i64`). `prefix` starts the message of any problem, as
    /// for `number_value`: a non-integer is `<prefix> must be an integer, found <type>`, an
    /// integer beyond `i64` is `<prefix> is out of range`.
    fn integer_value(
        &mut self,
        prefix: &str,
        value: &Spanned<DeValue<'i>>,
    ) -> Option<Located<i64>> {
        let at = self.file.location_of(value.span());
        match value.get_ref().as_integer() {
            Some(int) => match i64::from_str_radix(int.as_str(), int.radix()) {
                Ok(n) => Some(Located { value: n, at }),
                Err(_) => {
                    self.sink.at(&at, format!("{prefix} is out of range"));
                    None
                }
            },
            None => {
                self.sink.at(
                    &at,
                    format!(
                        "{prefix} must be an integer, found {}",
                        value.get_ref().type_str()
                    ),
                );
                None
            }
        }
    }

    /// A number: an integer or a float, as `f64`. Reported if `required` and missing.
    fn number(&mut self, key: &str, required: bool) -> Option<Located<f64>> {
        let Found::Value(value) = self.find(key, required) else {
            return None;
        };
        let prefix = format!("key '{key}' in {}", self.label);
        self.number_value(&prefix, value)
    }

    /// Parses `value` as a number (an integer or a float, as `f64`). `prefix` starts the
    /// message of any problem: `key 'output' in resource` or `key 'grain' in prices of market`.
    fn number_value(&mut self, prefix: &str, value: &Spanned<DeValue<'i>>) -> Option<Located<f64>> {
        let at = self.file.location_of(value.span());
        let parsed = match value.get_ref() {
            DeValue::Integer(int) => i64::from_str_radix(int.as_str(), int.radix())
                .ok()
                .map(|n| n as f64),
            DeValue::Float(float) => float.as_str().parse::<f64>().ok(),
            other => {
                self.sink.at(
                    &at,
                    format!("{prefix} must be a number, found {}", other.type_str()),
                );
                return None;
            }
        };
        match parsed {
            Some(n) => Some(Located { value: n, at }),
            None => {
                self.sink.at(&at, format!("{prefix} is out of range"));
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
    pub fn optional_integer(&mut self, key: &str) -> Option<Located<i64>> {
        self.integer(key, false)
    }

    /// A number that must be present: an integer or a float, as `f64`. The value is
    /// returned as written (including `inf` and `nan`); range checks belong to the caller.
    pub fn required_number(&mut self, key: &str) -> Option<Located<f64>> {
        self.number(key, true)
    }

    /// A number that may be absent. Absent and wrong-type both give None (the latter is
    /// reported); use [`has`](Self::has) to tell them apart.
    pub fn optional_number(&mut self, key: &str) -> Option<Located<f64>> {
        self.number(key, false)
    }

    /// Checks that `value` is a finite number greater than 0 and returns it as a
    /// [`Quantity`]; otherwise reports `<prefix> must be a finite number greater than 0` at
    /// the value and returns None. `prefix` is the same text `number_value` uses: `key
    /// 'output' in resource`, `key 'grain' in prices of market`.
    pub fn positive_quantity(&mut self, prefix: &str, value: Located<f64>) -> Option<Quantity> {
        let quantity = Quantity::new(value.value).filter(|q| q.get() > 0.0);
        if quantity.is_none() {
            let message = format!("{prefix} must be a finite number greater than 0");
            self.sink.at(&value.at, message);
        }
        quantity
    }

    /// Checks that `value` is a finite number of at least 0 and returns it; otherwise
    /// reports `<prefix> must be a finite number of at least 0` at the value and returns
    /// None. `prefix` is as for [`positive_quantity`](Self::positive_quantity).
    pub fn non_negative_number(&mut self, prefix: &str, value: Located<f64>) -> Option<f64> {
        if value.value.is_finite() && value.value >= 0.0 {
            Some(value.value)
        } else {
            let message = format!("{prefix} must be a finite number of at least 0");
            self.sink.at(&value.at, message);
            None
        }
    }

    /// True if the table has `key`. Doesn't mark the key as read.
    pub fn has(&self, key: &str) -> bool {
        self.table.iter().any(|(k, _)| k.get_ref().as_ref() == key)
    }

    /// Reads `key` as a table whose values are numbers. Returns every (key, value) pair in the
    /// table's order, each key with its location. A value that isn't a number or is out of
    /// range is reported (`key '<k>' in <key> of <label> must be a number, found <type>`,
    /// `key '<k>' in <key> of <label> is out of range`) and returned as None, so the caller
    /// still sees the key. A missing key reports the usual missing-key error; a value that
    /// isn't a table reports `key '<key>' in <label> must be a table, found <type>`. Both
    /// return None. The nested table's keys are data, not fields: they are not name-checked
    /// and not tracked as read or unknown.
    #[allow(clippy::type_complexity)]
    pub fn required_number_table(
        &mut self,
        key: &str,
    ) -> Option<Vec<(Located<String>, Option<Located<f64>>)>> {
        let Found::Value(value) = self.find(key, true) else {
            return None;
        };
        let Some(table) = value.get_ref().as_table() else {
            self.wrong_type(key, "a table", value);
            return None;
        };
        let mut pairs = Vec::new();
        for (k, v) in table.iter() {
            let name = Located {
                value: k.get_ref().to_string(),
                at: self.file.location_of(k.span()),
            };
            let prefix = format!("key '{}' in {key} of {}", name.value, self.label);
            let number = self.number_value(&prefix, v);
            pairs.push((name, number));
        }
        Some(pairs)
    }

    /// Reads `key` as a table whose values are integers: like `required_number_table`, but
    /// each value is parsed as `required_integer` parses one. Returns every key with its
    /// location, in the table's order, and `None` where the value isn't an integer or is out
    /// of range (reported as `key '<k>' in <key> of <label> must be an integer, found <type>`
    /// or `... is out of range`). A missing key gives the usual missing-key error and a
    /// non-table value `key '<key>' in <label> must be a table, found <type>`; both return
    /// None. The table's keys are data: not name-checked, not tracked as read or unknown.
    #[allow(clippy::type_complexity)]
    pub fn required_integer_table(
        &mut self,
        key: &str,
    ) -> Option<Vec<(Located<String>, Option<Located<i64>>)>> {
        let Found::Value(value) = self.find(key, true) else {
            return None;
        };
        let Some(table) = value.get_ref().as_table() else {
            self.wrong_type(key, "a table", value);
            return None;
        };
        let mut pairs = Vec::new();
        for (k, v) in table.iter() {
            let name = Located {
                value: k.get_ref().to_string(),
                at: self.file.location_of(k.span()),
            };
            let prefix = format!("key '{}' in {key} of {}", name.value, self.label);
            let integer = self.integer_value(&prefix, v);
            pairs.push((name, integer));
        }
        Some(pairs)
    }

    /// Reads `key` as a table whose values are quantities: `required_number_table` with
    /// `positive_quantity` applied to every value, using the same prefix (`key '<k>' in
    /// <key> of <label>`). A value that isn't a number, is out of range, or isn't a finite
    /// number greater than 0 is reported and returned as None, so the caller still sees the
    /// key. Missing and non-table `key` are handled as for `required_number_table` (None).
    #[allow(clippy::type_complexity)]
    pub fn required_quantity_table(
        &mut self,
        key: &str,
    ) -> Option<Vec<(Located<String>, Option<Quantity>)>> {
        let label = self.label.clone();
        let pairs = self.required_number_table(key)?;
        Some(
            pairs
                .into_iter()
                .map(|(name, number)| {
                    let prefix = format!("key '{}' in {key} of {label}", name.value);
                    let quantity = number.and_then(|v| self.positive_quantity(&prefix, v));
                    (name, quantity)
                })
                .collect(),
        )
    }

    /// Reads `key` as a table whose values are tables. Returns every (name, table) pair in
    /// the table's order, each name with its location. A value that isn't a table is
    /// reported (`key '<k>' in <key> of <label> must be a table, found <type>`) and returned
    /// as None, so the caller still sees the name. A missing key reports the usual
    /// missing-key error; a value that isn't a table reports `key '<key>' in <label> must
    /// be a table, found <type>`. Both return None. The nested table's keys are data, not
    /// fields: not name-checked, not tracked as read or unknown. Read each sub-table with
    /// [`nested`](Self::nested).
    #[allow(clippy::type_complexity)]
    pub fn required_table_of_tables(
        &mut self,
        key: &str,
    ) -> Option<Vec<(Located<String>, Option<&'a DeTable<'i>>)>> {
        let Found::Value(value) = self.find(key, true) else {
            return None;
        };
        let Some(table) = value.get_ref().as_table() else {
            self.wrong_type(key, "a table", value);
            return None;
        };
        let mut pairs = Vec::new();
        for (k, v) in table.iter() {
            let name = Located {
                value: k.get_ref().to_string(),
                at: self.file.location_of(k.span()),
            };
            let sub = v.get_ref().as_table();
            if sub.is_none() {
                let at = self.file.location_of(v.span());
                self.sink.at(
                    &at,
                    format!(
                        "key '{}' in {key} of {} must be a table, found {}",
                        name.value,
                        self.label,
                        v.get_ref().type_str()
                    ),
                );
            }
            pairs.push((name, sub));
        }
        Some(pairs)
    }

    /// A reader for one sub-table returned by
    /// [`required_table_of_tables`](Self::required_table_of_tables): `key` is the outer key and `name` the sub-table's
    /// key. Its label is `<key>.<name> of <label>` (for example `utility.grain of
    /// pop_type`), so its messages read `key 'share' in utility.grain of pop_type must be
    /// a number, found string`. Missing required keys are reported at `name`'s location.
    /// Call `finish` on it to report its unknown keys.
    pub fn nested(
        &mut self,
        key: &str,
        name: &Located<String>,
        table: &'a DeTable<'i>,
    ) -> EntryReader<'_, 'i> {
        EntryReader {
            file: self.file,
            table,
            entry: name.at.clone(),
            missing_at: MissingAt::Entry,
            label: format!("{key}.{} of {}", name.value, self.label),
            read: Vec::new(),
            sink: &mut *self.sink,
        }
    }

    /// Reads `key` as a table and returns a reader for it, labelled `<key> of <label>` (for
    /// example `growth of pop_type`), so its messages read `key 'decline' in growth of
    /// pop_type must be a number, found string` and `unknown key 'cap' in growth of
    /// pop_type`. Missing required keys inside it are reported at `key`'s own location (the
    /// key in an inline entry, or its name in a `[section.key]` header). A missing `key`
    /// reports the usual missing-key error; a value that isn't a table reports `key '<key>'
    /// in <label> must be a table, found <type>`. Both return None. Call `finish` on the
    /// returned reader to report its unknown keys.
    pub fn required_sub_table(&mut self, key: &str) -> Option<EntryReader<'_, 'i>> {
        let (k, value) = self.find_entry(key, true)?;
        let Some(table) = value.get_ref().as_table() else {
            self.wrong_type(key, "a table", value);
            return None;
        };
        Some(EntryReader {
            file: self.file,
            table,
            entry: self.file.location_of(k.span()),
            missing_at: MissingAt::Entry,
            label: format!("{key} of {}", self.label),
            read: Vec::new(),
            sink: &mut *self.sink,
        })
    }

    /// Reads `key` as an array of numbers (integers or floats, as `f64`). Returns the array's
    /// location and every element in order: `Some(value)` for a number (as written, including
    /// `inf` and `nan`; range checks belong to the caller), `None` for an element that isn't
    /// one, which is reported at the element's position as `element <n> of key '<key>' in
    /// <label> must be a number, found <type>` or `element <n> of key '<key>' in <label> is
    /// out of range`, with `n` counted from 1. An empty array returns an empty list. A
    /// missing `key` reports the usual missing-key error; a value that isn't an array
    /// reports `key '<key>' in <label> must be an array, found <type>`. Both return None.
    #[allow(clippy::type_complexity)]
    pub fn required_number_array(
        &mut self,
        key: &str,
    ) -> Option<Located<Vec<Option<Located<f64>>>>> {
        let Found::Value(value) = self.find(key, true) else {
            return None;
        };
        let Some(array) = value.get_ref().as_array() else {
            self.wrong_type(key, "an array", value);
            return None;
        };
        let at = self.file.location_of(value.span());
        let mut elements = Vec::new();
        for (i, element) in array.iter().enumerate() {
            let prefix = format!("element {} of key '{key}' in {}", i + 1, self.label);
            elements.push(self.number_value(&prefix, element));
        }
        Some(Located {
            value: elements,
            at,
        })
    }

    /// A boolean value that must be present.
    // Only tests use this so far; remove `#[cfg(test)]` when a section type needs it.
    #[cfg(test)]
    pub fn required_bool(&mut self, key: &str) -> Option<Located<bool>> {
        self.boolean(key, true)
    }

    /// A boolean value that may be absent.
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
        sink.into_lines()
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
    fn numbers_integers_floats_and_special_values() {
        let errors = run(
            "a = 5
b = 2.5
c = 1_000.5
d = -3
e = inf
f = nan
g = \"5\"
h = 99999999999999999999
i = -inf
",
            |r| {
                assert_eq!(r.required_number("a").unwrap().value, 5.0);
                assert_eq!(r.required_number("b").unwrap().value, 2.5);
                assert_eq!(r.required_number("c").unwrap().value, 1000.5);
                assert_eq!(r.required_number("d").unwrap().value, -3.0);
                assert_eq!(r.required_number("e").unwrap().value, f64::INFINITY);
                assert!(r.required_number("f").unwrap().value.is_nan());
                assert!(r.required_number("g").is_none());
                assert!(r.required_number("h").is_none());
                assert_eq!(r.required_number("i").unwrap().value, f64::NEG_INFINITY);
                assert!(r.required_number("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:7:5: key 'g' in thing must be a number, found string",
                "rules/t.toml:8:5: key 'h' in thing is out of range",
            ]
        );
    }

    #[test]
    fn positive_quantity_accepts_positive_and_reports_the_rest() {
        let errors = run(
            "a = 2.5
b = -1
c = 0
d = inf
e = nan
",
            |r| {
                let read = |r: &mut EntryReader<'_, '_>, key: &str| {
                    let value = r.required_number(key).unwrap();
                    r.positive_quantity(&format!("key '{key}' in thing"), value)
                };
                assert_eq!(read(r, "a"), Some(Quantity::new(2.5).unwrap()));
                for key in ["b", "c", "d", "e"] {
                    assert!(read(r, key).is_none(), "{key}");
                }
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:2:5: key 'b' in thing must be a finite number greater than 0",
                "rules/t.toml:3:5: key 'c' in thing must be a finite number greater than 0",
                "rules/t.toml:4:5: key 'd' in thing must be a finite number greater than 0",
                "rules/t.toml:5:5: key 'e' in thing must be a finite number greater than 0",
            ]
        );
    }

    #[test]
    fn non_negative_number_accepts_zero_and_up_and_reports_the_rest() {
        let errors = run(
            "a = 2.5
b = -1
c = 0
d = inf
e = nan
",
            |r| {
                let read = |r: &mut EntryReader<'_, '_>, key: &str| {
                    let value = r.required_number(key).unwrap();
                    r.non_negative_number(&format!("key '{key}' in thing"), value)
                };
                assert_eq!(read(r, "a"), Some(2.5));
                assert_eq!(read(r, "c"), Some(0.0));
                for key in ["b", "d", "e"] {
                    assert!(read(r, key).is_none(), "{key}");
                }
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:2:5: key 'b' in thing must be a finite number of at least 0",
                "rules/t.toml:4:5: key 'd' in thing must be a finite number of at least 0",
                "rules/t.toml:5:5: key 'e' in thing must be a finite number of at least 0",
            ]
        );
    }

    #[test]
    fn number_tables_give_every_key_and_report_bad_values() {
        let errors = run(
            "p = { a = 1, b = 2.5, c = \"x\", d = 99999999999999999999, e = inf }
q = 7
",
            |r| {
                let pairs = r.required_number_table("p").unwrap();
                let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.value.as_str()).collect();
                assert_eq!(keys, vec!["a", "b", "c", "d", "e"]);
                assert_eq!(pairs[0].1.as_ref().unwrap().value, 1.0);
                assert_eq!(pairs[1].1.as_ref().unwrap().value, 2.5);
                assert!(pairs[2].1.is_none());
                assert!(pairs[3].1.is_none());
                assert_eq!(pairs[4].1.as_ref().unwrap().value, f64::INFINITY);
                assert_eq!((pairs[0].0.at.line, pairs[0].0.at.column), (1, 7));
                assert!(r.required_number_table("q").is_none());
                assert!(r.required_number_table("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:27: key 'c' in p of thing must be a number, found string",
                "rules/t.toml:1:36: key 'd' in p of thing is out of range",
                "rules/t.toml:2:5: key 'q' in thing must be a table, found integer",
            ]
        );
    }

    #[test]
    fn integer_tables_give_every_key_and_report_bad_values() {
        let errors = run(
            "p = { a = 1, b = -2, c = 2.5, d = \"x\", e = 99999999999999999999 }
q = 7
",
            |r| {
                let pairs = r.required_integer_table("p").unwrap();
                let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.value.as_str()).collect();
                assert_eq!(keys, vec!["a", "b", "c", "d", "e"]);
                assert_eq!(pairs[0].1.as_ref().unwrap().value, 1);
                assert_eq!(pairs[1].1.as_ref().unwrap().value, -2);
                assert!(pairs[2].1.is_none());
                assert!(pairs[3].1.is_none());
                assert!(pairs[4].1.is_none());
                assert_eq!((pairs[0].0.at.line, pairs[0].0.at.column), (1, 7));
                assert!(r.required_integer_table("q").is_none());
                assert!(r.required_integer_table("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:26: key 'c' in p of thing must be an integer, found float",
                "rules/t.toml:1:35: key 'd' in p of thing must be an integer, found string",
                "rules/t.toml:1:44: key 'e' in p of thing is out of range",
                "rules/t.toml:2:5: key 'q' in thing must be a table, found integer",
            ]
        );
    }

    #[test]
    fn number_tables_work_as_sub_tables_and_empty() {
        let errors = run(
            "[p]
x = 3

[q]
",
            |r| {
                let pairs = r.required_number_table("p").unwrap();
                assert_eq!(pairs.len(), 1);
                assert_eq!(pairs[0].0.value, "x");
                assert_eq!(r.required_number_table("q").unwrap().len(), 0);
            },
        );
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn quantity_tables_give_quantities_and_report_the_rest() {
        let errors = run(
            "p = { a = 1, b = 2.5, c = \"x\", d = 0, e = inf }
q = 7
",
            |r| {
                let pairs = r.required_quantity_table("p").unwrap();
                let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.value.as_str()).collect();
                assert_eq!(keys, vec!["a", "b", "c", "d", "e"]);
                assert_eq!(pairs[0].1, Quantity::new(1.0));
                assert_eq!(pairs[1].1, Quantity::new(2.5));
                assert!(pairs[2].1.is_none());
                assert!(pairs[3].1.is_none());
                assert!(pairs[4].1.is_none());
                assert!(r.required_quantity_table("q").is_none());
                assert!(r.required_quantity_table("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:27: key 'c' in p of thing must be a number, found string",
                "rules/t.toml:1:36: key 'd' in p of thing must be a finite number greater than 0",
                "rules/t.toml:1:43: key 'e' in p of thing must be a finite number greater than 0",
                "rules/t.toml:2:5: key 'q' in thing must be a table, found integer",
            ]
        );
    }

    #[test]
    fn optional_numbers_present_absent_and_wrong_type() {
        let errors = run("a = 2\nb = \"x\"\n", |r| {
            assert_eq!(r.optional_number("a").unwrap().value, 2.0);
            assert!(r.optional_number("b").is_none());
            assert!(r.optional_number("missing").is_none());
        });
        assert_eq!(
            errors,
            vec!["rules/t.toml:2:5: key 'b' in thing must be a number, found string"]
        );
    }

    #[test]
    fn has_does_not_mark_a_key_as_read() {
        let errors = run("a = 1\nb = 2\n", |r| {
            assert!(r.has("a"));
            assert!(!r.has("zzz"));
            r.optional_integer("b");
        });
        assert_eq!(errors, vec!["rules/t.toml:1:1: unknown key 'a' in thing"]);
    }

    #[test]
    fn tables_of_tables_give_every_name_and_report_non_tables() {
        let errors = run(
            "p = { a = { n = 1 }, b = 5, c = {} }
q = 7
",
            |r| {
                let pairs = r.required_table_of_tables("p").unwrap();
                let names: Vec<&str> = pairs.iter().map(|(k, _)| k.value.as_str()).collect();
                assert_eq!(names, vec!["a", "b", "c"]);
                assert!(pairs[0].1.is_some());
                assert!(pairs[1].1.is_none());
                assert_eq!(pairs[2].1.unwrap().len(), 0);
                assert_eq!((pairs[0].0.at.line, pairs[0].0.at.column), (1, 7));
                assert!(r.required_table_of_tables("q").is_none());
                assert!(r.required_table_of_tables("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:26: key 'b' in p of thing must be a table, found integer",
                "rules/t.toml:2:5: key 'q' in thing must be a table, found integer",
            ]
        );
    }

    #[test]
    fn tables_of_tables_work_in_sub_table_form() {
        let errors = run(
            "[p.x]
n = 1

[p.y]
",
            |r| {
                let pairs = r.required_table_of_tables("p").unwrap();
                assert_eq!(pairs.len(), 2);
                assert_eq!(pairs[0].0.value, "x");
                assert_eq!(pairs[1].0.value, "y");
                assert!(pairs.iter().all(|(_, t)| t.is_some()));
            },
        );
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn nested_readers_report_with_the_nested_label_and_location() {
        let errors = run(
            "[p.x]
a = \"s\"
z = 1
",
            |r| {
                let pairs = r.required_table_of_tables("p").unwrap();
                let (name, table) = &pairs[0];
                let mut sub = r.nested("p", name, table.unwrap());
                assert_eq!(sub.label(), "p.x of thing");
                assert!(sub.optional_number("a").is_none());
                assert!(sub.required_number("b").is_none());
                sub.finish();
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:4: missing required key 'b' in p.x of thing",
                "rules/t.toml:2:5: key 'a' in p.x of thing must be a number, found string",
                "rules/t.toml:3:1: unknown key 'z' in p.x of thing",
            ]
        );
    }

    #[test]
    fn label_is_the_section_label() {
        run("", |r| assert_eq!(r.label(), "thing"));
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
        let shown = sink.into_lines();
        assert_eq!(
            shown,
            vec![
                "setting.toml: missing required key 'name' in manifest",
                "setting.toml:1:1: unknown key 'x' in manifest",
            ]
        );
    }

    #[test]
    fn sub_tables_read_with_their_own_label_and_location() {
        let errors = run("g = { a = 1, z = 2 }\nq = 7\n", |r| {
            {
                let mut sub = r.required_sub_table("g").unwrap();
                assert_eq!(sub.label(), "g of thing");
                assert_eq!(sub.required_number("a").unwrap().value, 1.0);
                assert!(sub.required_number("b").is_none());
                sub.finish();
            }
            assert!(r.required_sub_table("q").is_none());
            assert!(r.required_sub_table("missing").is_none());
        });
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'b' in g of thing",
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:14: unknown key 'z' in g of thing",
                "rules/t.toml:2:5: key 'q' in thing must be a table, found integer",
            ]
        );
    }

    #[test]
    fn sub_tables_work_in_header_form() {
        let errors = run("[g]\na = 1\n", |r| {
            let mut sub = r.required_sub_table("g").unwrap();
            assert_eq!(sub.required_number("a").unwrap().value, 1.0);
            assert!(sub.required_number("b").is_none());
            sub.finish();
        });
        assert_eq!(
            errors,
            vec!["rules/t.toml:1:2: missing required key 'b' in g of thing"]
        );
    }

    #[test]
    fn number_arrays_give_every_element_and_report_bad_ones() {
        let errors = run(
            "a = [1, 2.5, \"x\", 99999999999999999999, inf]\nb = []\nc = 7\n",
            |r| {
                let a = r.required_number_array("a").unwrap();
                assert_eq!((a.at.line, a.at.column), (1, 5));
                let values: Vec<Option<f64>> = a
                    .value
                    .iter()
                    .map(|e| e.as_ref().map(|e| e.value))
                    .collect();
                assert_eq!(
                    values,
                    vec![Some(1.0), Some(2.5), None, None, Some(f64::INFINITY)]
                );
                let b = r.required_number_array("b").unwrap();
                assert!(b.value.is_empty());
                assert_eq!((b.at.line, b.at.column), (2, 5));
                assert!(r.required_number_array("c").is_none());
                assert!(r.required_number_array("missing").is_none());
            },
        );
        assert_eq!(
            errors,
            vec![
                "rules/t.toml:1:1: missing required key 'missing' in thing",
                "rules/t.toml:1:14: element 3 of key 'a' in thing must be a number, found string",
                "rules/t.toml:1:19: element 4 of key 'a' in thing is out of range",
                "rules/t.toml:3:5: key 'c' in thing must be an array, found integer",
            ]
        );
    }
}
