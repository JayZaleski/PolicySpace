//! Error types for setting loading: where a problem is, and the collection of all of them.

use std::error::Error;
use std::fmt;

/// A position in a setting file: the path relative to the setting folder (with forward
/// slashes), a 1-based line, and a 1-based column counted in characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Location {
    /// Path relative to the setting folder, with forward slashes.
    pub path: String,
    /// 1-based line.
    pub line: u32,
    /// 1-based column, in characters.
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path, self.line, self.column)
    }
}

/// A value read from a setting file, together with where it was written.
#[derive(Clone, Debug)]
pub(crate) struct Located<T> {
    /// The value.
    pub value: T,
    /// Where the value was written.
    pub at: Location,
}

/// One problem found while loading a setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    path: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    message: String,
}

impl LoadError {
    /// Path relative to the setting folder, with forward slashes (None for problems with
    /// the folder itself).
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// 1-based line, if the problem has a position.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// 1-based column in characters, if the problem has a position.
    pub fn column(&self) -> Option<u32> {
        self.column
    }

    /// The message, without the location.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.path, self.line, self.column) {
            (Some(path), Some(line), Some(column)) => {
                write!(f, "{path}:{line}:{column}: {}", self.message)
            }
            (Some(path), _, _) => write!(f, "{path}: {}", self.message),
            (None, _, _) => write!(f, "{}", self.message),
        }
    }
}

impl Error for LoadError {}

/// Every problem found in one failed load, in sorted order. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadErrors(Vec<LoadError>);

impl LoadErrors {
    /// The errors, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, LoadError> {
        self.0.iter()
    }

    /// Number of errors (at least 1).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always false: a `LoadErrors` holds at least one error.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a LoadErrors {
    type Item = &'a LoadError;
    type IntoIter = std::slice::Iter<'a, LoadError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl fmt::Display for LoadErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl Error for LoadErrors {}

/// Collects every problem found during a load, instead of stopping at the first.
#[derive(Debug, Default)]
pub(crate) struct ErrorSink {
    errors: Vec<LoadError>,
}

impl ErrorSink {
    /// An empty collector.
    pub fn new() -> Self {
        ErrorSink::default()
    }

    /// Reports a problem at a position in a file.
    pub fn at(&mut self, at: &Location, message: impl Into<String>) {
        self.errors.push(LoadError {
            path: Some(at.path.clone()),
            line: Some(at.line),
            column: Some(at.column),
            message: message.into(),
        });
    }

    /// Reports a problem with a whole file or folder (no position).
    pub fn in_file(&mut self, path: &str, message: impl Into<String>) {
        self.errors.push(LoadError {
            path: Some(path.to_string()),
            line: None,
            column: None,
            message: message.into(),
        });
    }

    /// Reports a problem that isn't tied to any file.
    pub fn general(&mut self, message: impl Into<String>) {
        self.errors.push(LoadError {
            path: None,
            line: None,
            column: None,
            message: message.into(),
        });
    }

    /// True if nothing has been reported.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Sorts the errors into their fixed order (see [`sort_key`]). Returns `Ok(value)`
    /// only if no error was reported.
    pub fn into_result<T>(mut self, value: T) -> Result<T, LoadErrors> {
        if self.errors.is_empty() {
            Ok(value)
        } else {
            // A stable sort keeps errors at the same position in the order they were
            // reported.
            self.errors.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
            Err(LoadErrors(self.errors))
        }
    }
}

/// The sort key for an error: path group, path, line (None first), column.
///
/// Groups: no path = 0; `setting.toml` = 1; `rules/...` = 3; `start/...` = 4; anything
/// else (other root entries) = 2. This gives "by file in read order, then by position".
fn sort_key(error: &LoadError) -> (u8, Option<&str>, Option<u32>, Option<u32>) {
    let group = match error.path.as_deref() {
        None => 0,
        Some("setting.toml") => 1,
        Some(path) if path.starts_with("rules/") => 3,
        Some(path) if path.starts_with("start/") => 4,
        Some(_) => 2,
    };
    (group, error.path.as_deref(), error.line, error.column)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(path: &str, line: u32, column: u32) -> Location {
        Location {
            path: path.to_string(),
            line,
            column,
        }
    }

    #[test]
    fn location_display() {
        assert_eq!(loc("rules/a.toml", 3, 9).to_string(), "rules/a.toml:3:9");
    }

    #[test]
    fn load_error_display_has_three_shapes() {
        let mut sink = ErrorSink::new();
        sink.at(&loc("rules/metals.toml", 14, 3), "full");
        sink.in_file("rules/notes.txt", "path only");
        sink.general("neither");
        let errors = sink.into_result(()).unwrap_err();
        let shown: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(
            shown,
            vec![
                "neither",
                "rules/metals.toml:14:3: full",
                "rules/notes.txt: path only",
            ]
        );
    }

    #[test]
    fn load_error_accessors() {
        let mut sink = ErrorSink::new();
        sink.at(&loc("setting.toml", 2, 5), "msg");
        sink.general("no path");
        let errors = sink.into_result(()).unwrap_err();
        let general = errors.iter().next().unwrap();
        assert_eq!(general.path(), None);
        assert_eq!(general.line(), None);
        assert_eq!(general.column(), None);
        let positioned = errors.iter().nth(1).unwrap();
        assert_eq!(positioned.path(), Some("setting.toml"));
        assert_eq!(positioned.line(), Some(2));
        assert_eq!(positioned.column(), Some(5));
        assert_eq!(positioned.message(), "msg");
    }

    #[test]
    fn load_errors_display_has_no_trailing_newline() {
        let mut sink = ErrorSink::new();
        sink.general("one");
        sink.general("two");
        let errors = sink.into_result(()).unwrap_err();
        assert_eq!(errors.to_string(), "one\ntwo");
        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn empty_sink_gives_ok() {
        let sink = ErrorSink::new();
        assert!(sink.is_empty());
        assert_eq!(sink.into_result(7), Ok(7));
    }

    #[test]
    fn errors_sort_by_group_path_line_column() {
        let mut sink = ErrorSink::new();
        sink.at(&loc("start/b.toml", 1, 1), "start b");
        sink.at(&loc("rules/b.toml", 2, 1), "rules b line 2");
        sink.at(&loc("rules/b.toml", 1, 7), "rules b 1:7");
        sink.at(&loc("rules/b.toml", 1, 2), "rules b 1:2");
        sink.in_file("rules/b.toml", "rules b file");
        sink.in_file("zzz.txt", "root zzz");
        sink.in_file("extra/", "root extra");
        sink.at(&loc("setting.toml", 1, 1), "manifest");
        sink.general("general");
        sink.in_file("rules/a.toml", "rules a");
        sink.in_file("start/", "start folder");
        let errors = sink.into_result(()).unwrap_err();
        let shown: Vec<&str> = errors.iter().map(|e| e.message()).collect();
        assert_eq!(
            shown,
            vec![
                "general",
                "manifest",
                "root extra",
                "root zzz",
                "rules a",
                "rules b file",
                "rules b 1:2",
                "rules b 1:7",
                "rules b line 2",
                "start folder",
                "start b",
            ]
        );
    }

    #[test]
    fn sort_is_stable_for_equal_keys() {
        let mut sink = ErrorSink::new();
        sink.at(&loc("rules/a.toml", 1, 1), "first");
        sink.at(&loc("rules/a.toml", 1, 1), "second");
        sink.at(&loc("rules/a.toml", 1, 1), "third");
        let errors = sink.into_result(()).unwrap_err();
        let shown: Vec<&str> = errors.iter().map(|e| e.message()).collect();
        assert_eq!(shown, vec!["first", "second", "third"]);
    }
}
