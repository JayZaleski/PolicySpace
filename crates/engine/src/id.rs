//! Typed numeric IDs and the registries that hand them out.
//!
//! Setting files refer to things (goods, hexes, deposits, ...) by text name. When a
//! setting is loaded, a [`NameRegistry`] turns each text name into a compact numeric
//! [`Id`], and the rest of the engine works with those IDs. An ID's index can be used
//! to index a `Vec` directly, and registries iterate in registration order, so the
//! engine stays deterministic.
//!
//! Rules:
//! - Text names live in setting files. The registry converts them to IDs at load time.
//! - An ID is only meaningful inside the registry (the world) that created it. IDs must
//!   never be persisted: anything saved long-term stores text names, not IDs.
//! - Each kind of thing has its own marker type implementing [`IdKind`], so IDs of
//!   different kinds are different types and cannot be mixed up. Each later Step
//!   defines its own kinds; this module defines none.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Marker trait for a kind of identified thing (good, hex, deposit, ...).
/// Implemented by zero-sized marker types, e.g. `pub enum GoodKind {}`.
pub trait IdKind: 'static {
    /// Lower-case human label used in messages, e.g. "good", "resource type".
    const LABEL: &'static str;
}

/// A compact numeric ID for one thing of kind `K`.
///
/// IDs of different kinds are different types, so mixing them up fails to compile:
///
/// ```compile_fail
/// use policyspace_engine::{Id, IdKind, NameRegistry};
///
/// enum GoodKind {}
/// impl IdKind for GoodKind {
///     const LABEL: &'static str = "good";
/// }
/// enum HexKind {}
/// impl IdKind for HexKind {
///     const LABEL: &'static str = "hex";
/// }
///
/// fn takes_good(_id: Id<GoodKind>) {}
///
/// let mut hexes: NameRegistry<HexKind> = NameRegistry::new();
/// let hex = hexes.register("origin").unwrap();
/// takes_good(hex); // error: expected `Id<GoodKind>`, found `Id<HexKind>`
/// ```
pub struct Id<K: IdKind> {
    index: u32,
    _kind: PhantomData<fn() -> K>,
}

impl<K: IdKind> Id<K> {
    /// Creates an ID from a raw index. Crate-private: only registries hand out IDs.
    pub(crate) fn from_index(index: u32) -> Self {
        Id {
            index,
            _kind: PhantomData,
        }
    }

    /// The ID's position in its registry (0-based), for indexing per-kind Vecs.
    pub fn index(self) -> usize {
        self.index as usize
    }
}

// The trait impls below are written by hand: `#[derive]` would add bounds such as
// `K: Clone`, which the uninhabited marker types don't satisfy.

impl<K: IdKind> Clone for Id<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: IdKind> Copy for Id<K> {}

impl<K: IdKind> PartialEq for Id<K> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<K: IdKind> Eq for Id<K> {}

impl<K: IdKind> Hash for Id<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<K: IdKind> PartialOrd for Id<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: IdKind> Ord for Id<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl<K: IdKind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", K::LABEL, self.index)
    }
}

/// Errors from name lookups and registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// The name was already registered for this kind.
    DuplicateName {
        /// The label of the kind (`IdKind::LABEL`).
        kind: &'static str,
        /// The name that was registered twice.
        name: String,
    },
    /// No thing of this kind has this name.
    UnknownName {
        /// The label of the kind (`IdKind::LABEL`).
        kind: &'static str,
        /// The name that wasn't found.
        name: String,
    },
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::DuplicateName { kind, name } => write!(f, "duplicate {kind} name '{name}'"),
            IdError::UnknownName { kind, name } => write!(f, "unknown {kind} '{name}'"),
        }
    }
}

impl Error for IdError {}

/// Two-way map between text names and IDs for one kind.
///
/// IDs are handed out as `0, 1, 2, ...` in registration order, and [`iter`](Self::iter)
/// yields entries in that same order (never hash order).
pub struct NameRegistry<K: IdKind> {
    /// Index = `Id::index`.
    names: Vec<String>,
    by_name: HashMap<String, Id<K>>,
}

impl<K: IdKind> NameRegistry<K> {
    /// An empty registry.
    pub fn new() -> Self {
        NameRegistry {
            names: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// Registers `name` and returns its new ID (the next index in order).
    /// Errors with [`IdError::DuplicateName`] if `name` is already registered for this
    /// kind; the registry is left unchanged. Any string is accepted, including an empty
    /// one: validating name syntax belongs to the setting loader.
    pub fn register(&mut self, name: &str) -> Result<Id<K>, IdError> {
        if self.by_name.contains_key(name) {
            return Err(IdError::DuplicateName {
                kind: K::LABEL,
                name: name.to_string(),
            });
        }
        let index = u32::try_from(self.names.len()).expect("name registry exceeded u32::MAX names");
        let id = Id::from_index(index);
        self.names.push(name.to_string());
        self.by_name.insert(name.to_string(), id);
        Ok(id)
    }

    /// Looks up the ID for `name`.
    /// Errors with [`IdError::UnknownName`] if `name` isn't registered.
    pub fn id(&self, name: &str) -> Result<Id<K>, IdError> {
        self.by_name
            .get(name)
            .copied()
            .ok_or_else(|| IdError::UnknownName {
                kind: K::LABEL,
                name: name.to_string(),
            })
    }

    /// The text name for `id`.
    ///
    /// # Panics
    /// Panics if `id` didn't come from this registry (index out of range). That is a
    /// programming error, since IDs are only valid inside their own registry.
    pub fn name(&self, id: Id<K>) -> &str {
        match self.names.get(id.index()) {
            Some(name) => name,
            None => panic!(
                "{:?} is not from this registry (it has {} {} names)",
                id,
                self.names.len(),
                K::LABEL
            ),
        }
    }

    /// Number of registered names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// True if nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// All (ID, name) pairs in registration order.
    pub fn iter(&self) -> impl Iterator<Item = (Id<K>, &str)> + '_ {
        self.names.iter().enumerate().map(|(index, name)| {
            let index = u32::try_from(index).expect("name registry exceeded u32::MAX names");
            (Id::from_index(index), name.as_str())
        })
    }
}

impl<K: IdKind> Default for NameRegistry<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: IdKind> fmt::Debug for NameRegistry<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NameRegistry")
            .field("kind", &K::LABEL)
            .field("names", &self.names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum TestGood {}
    impl IdKind for TestGood {
        const LABEL: &'static str = "good";
    }

    enum TestHex {}
    impl IdKind for TestHex {
        const LABEL: &'static str = "hex";
    }

    #[test]
    fn register_gives_sequential_ids_and_round_trips() {
        let mut reg: NameRegistry<TestGood> = NameRegistry::new();
        let a = reg.register("a").unwrap();
        let b = reg.register("b").unwrap();
        let c = reg.register("c").unwrap();
        assert_eq!((a.index(), b.index(), c.index()), (0, 1, 2));
        assert_eq!(reg.name(a), "a");
        assert_eq!(reg.name(b), "b");
        assert_eq!(reg.name(c), "c");
        assert_eq!(reg.id("a").unwrap(), a);
        assert_eq!(reg.id("b").unwrap(), b);
        assert_eq!(reg.id("c").unwrap(), c);
    }

    #[test]
    fn iter_is_in_registration_order_and_len_is_correct() {
        let mut reg: NameRegistry<TestGood> = NameRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert_eq!(reg.iter().count(), 0);
        let z = reg.register("zinc").unwrap();
        let a = reg.register("aluminium").unwrap();
        let m = reg.register("magnesium").unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 3);
        let items: Vec<_> = reg.iter().collect();
        assert_eq!(items, vec![(z, "zinc"), (a, "aluminium"), (m, "magnesium")]);
    }

    #[test]
    fn duplicate_name_is_rejected_and_registry_unchanged() {
        let mut reg: NameRegistry<TestGood> = NameRegistry::new();
        let copper = reg.register("copper").unwrap();
        let err = reg.register("copper").unwrap_err();
        assert_eq!(
            err,
            IdError::DuplicateName {
                kind: "good",
                name: "copper".to_string()
            }
        );
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.id("copper").unwrap(), copper);
    }

    #[test]
    fn unknown_name_and_error_messages() {
        let reg: NameRegistry<TestGood> = NameRegistry::new();
        let err = reg.id("missing").unwrap_err();
        assert_eq!(
            err,
            IdError::UnknownName {
                kind: "good",
                name: "missing".to_string()
            }
        );
        assert_eq!(err.to_string(), "unknown good 'missing'");

        let dup = IdError::DuplicateName {
            kind: "good",
            name: "copper".to_string(),
        };
        assert_eq!(dup.to_string(), "duplicate good name 'copper'");
    }

    #[test]
    fn same_name_in_different_kinds_is_allowed() {
        let mut goods: NameRegistry<TestGood> = NameRegistry::new();
        let mut hexes: NameRegistry<TestHex> = NameRegistry::new();
        assert!(goods.register("copper").is_ok());
        assert!(hexes.register("copper").is_ok());
    }

    #[test]
    #[should_panic(expected = "not from this registry")]
    fn name_with_out_of_range_id_panics() {
        let mut reg: NameRegistry<TestGood> = NameRegistry::new();
        reg.register("a").unwrap();
        let bad: Id<TestGood> = Id::from_index(5);
        reg.name(bad);
    }

    #[test]
    fn debug_output_uses_label() {
        let id: Id<TestGood> = Id::from_index(3);
        assert_eq!(format!("{id:?}"), "Id<good>(3)");
    }
}
