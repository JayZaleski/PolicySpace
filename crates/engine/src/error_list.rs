//! One never-empty list of errors, shared by every place that reports "every problem found"
//! instead of stopping at the first.
//!
//! [`ErrorList`] is the one type behind three public names:
//! [`LoadErrors`](crate::LoadErrors) (every problem in one failed setting load, in sorted
//! order), [`DecisionErrors`](crate::DecisionErrors) (every problem in one refused decision,
//! in document order) and [`ReplyErrors`](crate::ReplyErrors) (every problem that refuses a
//! whole reply). Each is a type alias, so they differ only in the error type they hold. A
//! list is never empty: the code that builds one has found at least one problem.

use std::error::Error;
use std::fmt;

/// Every problem found in one failed operation, in the order the operation defines. Never
/// empty. Displays one error per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorList<E>(Vec<E>);

impl<E> ErrorList<E> {
    /// Wraps `errors`. Crate-private: only the code that found the problems builds a list.
    ///
    /// # Panics
    /// Panics if `errors` is empty (`"an error list holds at least one error"`).
    pub(crate) fn new(errors: Vec<E>) -> Self {
        assert!(!errors.is_empty(), "an error list holds at least one error");
        ErrorList(errors)
    }

    /// The errors, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, E> {
        self.0.iter()
    }

    /// Number of errors (at least 1).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always false: an `ErrorList` holds at least one error.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
impl<E: fmt::Display> ErrorList<E> {
    /// Each error as one line of text, in order. For tests.
    pub(crate) fn lines(&self) -> Vec<String> {
        self.0.iter().map(|e| e.to_string()).collect()
    }
}

impl<'a, E> IntoIterator for &'a ErrorList<E> {
    type Item = &'a E;
    type IntoIter = std::slice::Iter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<E: fmt::Display> fmt::Display for ErrorList<E> {
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

impl<E: fmt::Debug + fmt::Display> Error for ErrorList<E> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "an error list holds at least one error")]
    fn new_panics_on_an_empty_list() {
        ErrorList::<String>::new(Vec::new());
    }

    #[test]
    fn display_joins_the_errors_with_newlines_and_no_trailing_newline() {
        let one = ErrorList::new(vec!["first".to_string()]);
        assert_eq!(one.to_string(), "first");
        let three = ErrorList::new(vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ]);
        assert_eq!(three.to_string(), "first\nsecond\nthird");
        assert_eq!(three.lines(), ["first", "second", "third"]);
    }

    #[test]
    fn iter_len_and_into_iter_agree() {
        let list = ErrorList::new(vec![1, 2, 3]);
        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
        assert_eq!(list.iter().copied().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!((&list).into_iter().copied().collect::<Vec<_>>(), [1, 2, 3]);
    }
}
