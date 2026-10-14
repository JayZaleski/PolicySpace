//! Typed numeric IDs and the registries that hand them out.
//!
//! Setting files refer to most things (goods, deposits, ...) by text name. When a
//! setting is loaded, a [`NameRegistry`] turns each text name into a compact numeric
//! [`Id`], and the rest of the engine works with those IDs. Some kinds of thing, such as
//! hexes, have no name and are identified by other data (a hex by its coordinates); the
//! list that holds them hands out their IDs directly, in load order. An ID's index can be
//! used to index a `Vec` directly, and registries and lists iterate in load order, so the
//! engine stays deterministic.
//!
//! Rules:
//! - Names (and the other data that identifies a thing) live in setting files. The
//!   registry, or the list for kinds without names, converts them to IDs at load time.
//! - An ID is only meaningful inside the registry or list (the world) that created it.
//!   IDs must never be persisted: anything saved long-term stores text names, or the
//!   data that identifies the thing, not IDs.
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

    /// Creates an ID from a position in a list. Crate-private, like `from_index`.
    ///
    /// # Panics
    /// Panics if `index` is greater than `u32::MAX`.
    pub(crate) fn from_usize(index: usize) -> Self {
        match u32::try_from(index) {
            Ok(index) => Id::from_index(index),
            Err(_) => panic!("{} index {index} does not fit in a u32 ID", K::LABEL),
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
        let id = Id::from_usize(self.names.len());
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
        self.names
            .iter()
            .enumerate()
            .map(|(index, name)| (Id::from_usize(index), name.as_str()))
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

/// A list with one `T` per ID of kind `K`: element `i` belongs to the ID with index `i`.
///
/// Use it for per-ID data (per hex, per deposit, ...) so that lookups by ID panic with one
/// message when the ID isn't from this list. IDs are only valid inside the list (the world)
/// that produced them. The trait impls are written by hand, for the same reason as `Id`'s.
pub(crate) struct IdVec<K: IdKind, T> {
    items: Vec<T>,
    _kind: PhantomData<fn() -> K>,
}

impl<K: IdKind, T> IdVec<K, T> {
    /// Wraps `items`, giving IDs 0, 1, 2, ... in order.
    ///
    /// # Panics
    /// Panics if the last index doesn't fit in a `u32` ID.
    pub(crate) fn from_vec(items: Vec<T>) -> Self {
        if let Some(last) = items.len().checked_sub(1) {
            Id::<K>::from_usize(last);
        }
        IdVec {
            items,
            _kind: PhantomData,
        }
    }

    /// Adds `item` at the end and returns its new ID.
    // Only tests use this so far; remove `#[cfg(test)]` when a section type needs it.
    #[cfg(test)]
    pub(crate) fn push(&mut self, item: T) -> Id<K> {
        let id = Id::from_usize(self.items.len());
        self.items.push(item);
        id
    }

    /// Number of items.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// True if there are no items.
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The item for `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list (index out of range).
    pub(crate) fn get(&self, id: Id<K>) -> &T {
        match self.items.get(id.index()) {
            Some(item) => item,
            None => Self::foreign(id, self.items.len()),
        }
    }

    /// The item for `id`, mutably.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list (index out of range).
    pub(crate) fn get_mut(&mut self, id: Id<K>) -> &mut T {
        let len = self.items.len();
        match self.items.get_mut(id.index()) {
            Some(item) => item,
            None => Self::foreign(id, len),
        }
    }

    /// Every item as (ID, item), in index order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Id<K>, &T)> + '_ {
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| (Id::from_usize(index), item))
    }

    /// Every item, mutably, in index order.
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.items.iter_mut()
    }

    fn foreign(id: Id<K>, len: usize) -> ! {
        panic!(
            "{:?} is not from this list (it has {} {} entries)",
            id,
            len,
            K::LABEL
        )
    }
}

impl<K: IdKind, T> Default for IdVec<K, T> {
    fn default() -> Self {
        IdVec {
            items: Vec::new(),
            _kind: PhantomData,
        }
    }
}

impl<K: IdKind, T: Clone> Clone for IdVec<K, T> {
    fn clone(&self) -> Self {
        IdVec {
            items: self.items.clone(),
            _kind: PhantomData,
        }
    }
}

impl<K: IdKind, T: PartialEq> PartialEq for IdVec<K, T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<K: IdKind, T: fmt::Debug> fmt::Debug for IdVec<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.items).finish()
    }
}

/// A list of named things of kind `K`, each with its data: a [`NameRegistry`] plus one `T`
/// per name.
///
/// IDs are handed out in load order, and [`iter`](Self::iter) yields entries in that same
/// order (which is also ID order). An ID is only valid inside the list that produced it.
pub struct NamedList<K: IdKind, T> {
    names: NameRegistry<K>,
    items: IdVec<K, T>,
}

impl<K: IdKind, T> NamedList<K, T> {
    /// Builds the list from the registered names and one item per name, in the same order.
    /// Crate-private: only the setting loader builds named lists.
    ///
    /// # Panics
    /// Panics if `names` and `items` have different lengths (a programming error).
    pub(crate) fn from_parts(names: NameRegistry<K>, items: Vec<T>) -> Self {
        assert_eq!(
            names.len(),
            items.len(),
            "every {} name needs exactly one entry",
            K::LABEL
        );
        NamedList {
            names,
            items: IdVec::from_vec(items),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if the list has no entries.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ID of the entry named `name`, or [`IdError::UnknownName`].
    pub fn id(&self, name: &str) -> Result<Id<K>, IdError> {
        self.names.id(name)
    }

    /// The name of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn name(&self, id: Id<K>) -> &str {
        self.names.name(id)
    }

    /// The data of `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn get(&self, id: Id<K>) -> &T {
        self.items.get(id)
    }

    /// Every entry as (ID, name, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (Id<K>, &str, &T)> + '_ {
        self.names
            .iter()
            .zip(self.items.iter())
            .map(|((id, name), (_, item))| (id, name, item))
    }
}

impl<K: IdKind, T> Default for NamedList<K, T> {
    fn default() -> Self {
        NamedList {
            names: NameRegistry::new(),
            items: IdVec::default(),
        }
    }
}

impl<K: IdKind, T: fmt::Debug> fmt::Debug for NamedList<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedList")
            .field("kind", &K::LABEL)
            .field("names", &self.names)
            .field("items", &self.items)
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

    #[test]
    fn id_vec_push_hands_out_sequential_ids_and_get_returns_each_item() {
        let mut list: IdVec<TestGood, &str> = IdVec::default();
        assert!(list.is_empty());
        let a = list.push("a");
        let b = list.push("b");
        let c = list.push("c");
        assert_eq!((a.index(), b.index(), c.index()), (0, 1, 2));
        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
        assert_eq!((*list.get(a), *list.get(b), *list.get(c)), ("a", "b", "c"));
    }

    #[test]
    fn id_vec_get_mut_changes_the_right_item() {
        let mut list: IdVec<TestGood, i32> = IdVec::from_vec(vec![10, 20, 30]);
        let middle = Id::from_index(1);
        *list.get_mut(middle) += 5;
        assert_eq!(
            list.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
            [10, 25, 30]
        );
        for value in list.values_mut() {
            *value += 1;
        }
        assert_eq!(
            list.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
            [11, 26, 31]
        );
    }

    #[test]
    fn id_vec_iter_is_in_index_order() {
        let list: IdVec<TestGood, char> = IdVec::from_vec(vec!['x', 'y', 'z']);
        let listed: Vec<(usize, char)> = list.iter().map(|(id, c)| (id.index(), *c)).collect();
        assert_eq!(listed, vec![(0, 'x'), (1, 'y'), (2, 'z')]);
    }

    #[test]
    fn id_vec_from_vec_gives_the_same_ids_as_pushing() {
        let built: IdVec<TestGood, i32> = IdVec::from_vec(vec![7, 8, 9]);
        let mut pushed: IdVec<TestGood, i32> = IdVec::default();
        for value in [7, 8, 9] {
            pushed.push(value);
        }
        assert_eq!(built, pushed);
        for (id, value) in built.iter() {
            assert_eq!(pushed.get(id), value);
        }
    }

    #[test]
    fn id_vec_clone_and_equality_work() {
        let list: IdVec<TestGood, i32> = IdVec::from_vec(vec![1, 2]);
        let mut copy = list.clone();
        assert_eq!(list, copy);
        *copy.get_mut(Id::from_index(0)) = 99;
        assert_ne!(list, copy);
        assert_eq!(format!("{list:?}"), "[1, 2]");
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn id_vec_get_with_a_foreign_id_panics() {
        let list: IdVec<TestGood, i32> = IdVec::from_vec(vec![1, 2]);
        list.get(Id::from_index(2));
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn id_vec_get_mut_with_a_foreign_id_panics() {
        let mut list: IdVec<TestGood, i32> = IdVec::default();
        list.get_mut(Id::from_index(0));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "does not fit in a u32 ID")]
    fn from_usize_panics_beyond_u32_max() {
        Id::<TestGood>::from_usize(u32::MAX as usize + 1);
    }

    #[test]
    fn from_usize_matches_from_index() {
        assert_eq!(Id::<TestGood>::from_usize(4), Id::from_index(4));
    }

    fn named_sample() -> NamedList<TestGood, i32> {
        let mut names = NameRegistry::new();
        names.register("zinc").unwrap();
        names.register("copper").unwrap();
        NamedList::from_parts(names, vec![10, 20])
    }

    #[test]
    fn named_list_accessors_and_ids_in_order() {
        let list = named_sample();
        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        let copper = list.id("copper").unwrap();
        assert_eq!(copper.index(), 1);
        assert_eq!(list.name(copper), "copper");
        assert_eq!(*list.get(copper), 20);
        assert_eq!(list.id("zinc").unwrap().index(), 0);
    }

    #[test]
    fn the_default_named_list_is_empty() {
        let list: NamedList<TestGood, i32> = NamedList::default();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
        assert_eq!(list.iter().count(), 0);
    }

    #[test]
    fn named_list_unknown_name_is_an_error() {
        assert_eq!(
            named_sample().id("iron"),
            Err(IdError::UnknownName {
                kind: "good",
                name: "iron".to_string()
            })
        );
    }

    #[test]
    fn named_list_iter_is_in_load_order() {
        let list = named_sample();
        let listed: Vec<(usize, &str, i32)> = list
            .iter()
            .map(|(id, name, value)| (id.index(), name, *value))
            .collect();
        assert_eq!(listed, vec![(0, "zinc", 10), (1, "copper", 20)]);
    }

    /// An ID that is out of range for `named_sample()` (which has two entries).
    fn foreign_good() -> Id<TestGood> {
        Id::from_index(2)
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn named_list_get_panics_on_a_foreign_id() {
        named_sample().get(foreign_good());
    }

    #[test]
    #[should_panic(expected = "is not from this registry")]
    fn named_list_name_panics_on_a_foreign_id() {
        named_sample().name(foreign_good());
    }

    #[test]
    #[should_panic(expected = "exactly one entry")]
    fn named_list_from_parts_panics_on_mismatched_lengths() {
        let mut names: NameRegistry<TestGood> = NameRegistry::new();
        names.register("a").unwrap();
        NamedList::<TestGood, i32>::from_parts(names, Vec::new());
    }
}
