//! Reading setting files: bytes to text to a parsed TOML tree, with byte offsets turned
//! into line and column positions.

use std::fs;
use std::ops::Range;
use std::path::Path;

use toml::de::DeTable;

use super::error::{ErrorSink, Location};

/// The text of one setting file, with what is needed to turn byte offsets into
/// line and column positions.
#[derive(Debug)]
pub(crate) struct SourceFile {
    /// Path relative to the setting folder, with forward slashes.
    pub path: String,
    text: String,
    /// Byte offset of the start of each line.
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Wraps `text` (already stripped of any BOM) as the file at `path`.
    pub fn new(path: &str, text: String) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        SourceFile {
            path: path.to_string(),
            text,
            line_starts,
        }
    }

    /// The file's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The position of a byte offset: 1-based line, and a 1-based column counted in
    /// characters. An offset past the end is treated as the end of the file.
    pub fn location(&self, byte_offset: usize) -> Location {
        let offset = byte_offset.min(self.text.len());
        let line_index = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let line_start = self.line_starts[line_index];
        let column = match self.text.get(line_start..offset) {
            Some(before) => before.chars().count(),
            // Not on a character boundary (should not happen): fall back to bytes.
            None => offset - line_start,
        };
        Location {
            path: self.path.clone(),
            line: to_u32(line_index + 1),
            column: to_u32(column + 1),
        }
    }

    /// The position where `span` starts.
    pub fn location_of(&self, span: Range<usize>) -> Location {
        self.location(span.start)
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Reads `abs_path` (shown as `rel_path` in errors), checks it is UTF-8 and valid TOML,
/// and hands the parsed table to `use_table`, returning what it returns.
///
/// Returns `None`, having reported the problems, if the file can't be read, isn't UTF-8,
/// or has any TOML syntax error. In that case `use_table` is not called, so the file's
/// content isn't validated. The caller treats `None` as "this file was rejected".
pub(crate) fn read_toml<R>(
    abs_path: &Path,
    rel_path: &str,
    sink: &mut ErrorSink,
    use_table: impl FnOnce(&SourceFile, &DeTable<'_>, &mut ErrorSink) -> R,
) -> Option<R> {
    let mut bytes = match fs::read(abs_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            sink.in_file(rel_path, format!("could not read file: {error}"));
            return None;
        }
    };
    // Strip one leading UTF-8 byte order mark; every location is computed without it.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid_len = error.utf8_error().valid_up_to();
            let valid_prefix = String::from_utf8_lossy(&error.as_bytes()[..valid_len]).into_owned();
            let file = SourceFile::new(rel_path, valid_prefix);
            sink.at(&file.location(valid_len), "file is not valid UTF-8");
            return None;
        }
    };
    let file = SourceFile::new(rel_path, text);
    let (table, parse_errors) = DeTable::parse_recoverable(file.text());
    if !parse_errors.is_empty() {
        for error in &parse_errors {
            let message = format!("invalid TOML: {}", error.message());
            match error.span() {
                Some(span) => sink.at(&file.location_of(span), message),
                None => sink.in_file(rel_path, message),
            }
        }
        return None;
    }
    Some(use_table(&file, table.get_ref(), sink))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> SourceFile {
        SourceFile::new("rules/a.toml", text.to_string())
    }

    #[test]
    fn location_on_first_line() {
        let f = file("name = 1\nother = 2\n");
        let l = f.location(0);
        assert_eq!((l.line, l.column), (1, 1));
        let l = f.location(5);
        assert_eq!((l.line, l.column), (1, 6));
        assert_eq!(l.path, "rules/a.toml");
    }

    #[test]
    fn location_on_later_lines() {
        let f = file("a = 1\nbb = 2\n\nccc = 3\n");
        let l = f.location(6);
        assert_eq!((l.line, l.column), (2, 1));
        let l = f.location(11);
        assert_eq!((l.line, l.column), (2, 6));
        let l = f.location(13);
        assert_eq!((l.line, l.column), (3, 1));
        let l = f.location(14);
        assert_eq!((l.line, l.column), (4, 1));
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        // "é" is two bytes, "日" is three.
        let f = file("s = \"é日\" # x\n");
        let offset = f.text().find('#').unwrap();
        let l = f.location(offset);
        assert_eq!((l.line, l.column), (1, 10));
    }

    #[test]
    fn location_past_end_is_end_of_file() {
        let f = file("ab\ncd");
        let l = f.location(999);
        assert_eq!((l.line, l.column), (2, 3));
    }

    #[test]
    fn location_of_uses_span_start() {
        let f = file("ab\ncd");
        let l = f.location_of(3..5);
        assert_eq!((l.line, l.column), (2, 1));
    }

    #[test]
    fn read_toml_parses_valid_file_and_strips_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.toml");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"x = 1\n");
        fs::write(&path, bytes).unwrap();
        let mut sink = ErrorSink::new();
        let keys = read_toml(&path, "a.toml", &mut sink, |_, table, _| {
            table
                .keys()
                .map(|k| k.get_ref().to_string())
                .collect::<Vec<_>>()
        });
        assert_eq!(keys, Some(vec!["x".to_string()]));
        assert!(sink.is_empty());
    }

    #[test]
    fn read_toml_reports_invalid_utf8_at_first_bad_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.toml");
        fs::write(&path, b"a = 1\nb = \xFF\xFE\n").unwrap();
        let mut sink = ErrorSink::new();
        let result = read_toml(&path, "rules/a.toml", &mut sink, |_, _, _| ());
        assert!(result.is_none());
        let errors = sink.into_result(()).unwrap_err();
        assert_eq!(
            errors.to_string(),
            "rules/a.toml:2:5: file is not valid UTF-8"
        );
    }

    #[test]
    fn read_toml_reports_syntax_errors_and_skips_callback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.toml");
        fs::write(&path, "a = \nb = 2\n").unwrap();
        let mut sink = ErrorSink::new();
        let result = read_toml(&path, "a.toml", &mut sink, |_, _, _| panic!("not called"));
        assert!(result.is_none());
        let errors = sink.into_result(()).unwrap_err();
        assert!(
            errors
                .iter()
                .all(|e| e.message().starts_with("invalid TOML: "))
        );
        assert_eq!(errors.iter().next().unwrap().line(), Some(1));
    }

    #[test]
    fn read_toml_reports_unreadable_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = ErrorSink::new();
        let result = read_toml(
            &dir.path().join("missing.toml"),
            "missing.toml",
            &mut sink,
            |_, _, _| (),
        );
        assert!(result.is_none());
        let errors = sink.into_result(()).unwrap_err();
        let error = errors.iter().next().unwrap();
        assert_eq!(error.path(), Some("missing.toml"));
        assert!(error.message().starts_with("could not read file: "));
    }
}
