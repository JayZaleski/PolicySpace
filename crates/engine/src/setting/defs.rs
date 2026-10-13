//! Definitions of named things: a name registry that also remembers where each name was
//! defined, so duplicate and unknown-name errors can point at the right places.

use crate::id::{Id, IdKind, NameRegistry};

use super::error::{ErrorSink, Located, Location};

/// A [`NameRegistry`] plus the location of each name's definition.
pub(crate) struct Definitions<K: IdKind> {
    registry: NameRegistry<K>,
    /// Indexed by `Id::index`.
    locations: Vec<Location>,
}

impl<K: IdKind> Definitions<K> {
    /// An empty set of definitions.
    pub fn new() -> Self {
        Definitions {
            registry: NameRegistry::new(),
            locations: Vec::new(),
        }
    }

    /// Registers the name. On a duplicate, reports an error at the new definition that
    /// names the first one's location, and returns `None`.
    pub fn define(&mut self, name: &Located<String>, sink: &mut ErrorSink) -> Option<Id<K>> {
        match self.registry.register(&name.value) {
            Ok(id) => {
                self.locations.push(name.at.clone());
                Some(id)
            }
            Err(error) => {
                // A duplicate means the name is already registered, so it has a location.
                let first = self
                    .registry
                    .id(&name.value)
                    .ok()
                    .and_then(|id| self.locations.get(id.index()));
                let message = match first {
                    Some(first) => format!("{error} (first defined at {first})"),
                    None => error.to_string(),
                };
                sink.at(&name.at, message);
                None
            }
        }
    }

    /// Looks up a reference. If the name is unknown, reports an error at the reference
    /// and returns `None`.
    pub fn resolve(&self, reference: &Located<String>, sink: &mut ErrorSink) -> Option<Id<K>> {
        match self.registry.id(&reference.value) {
            Ok(id) => Some(id),
            Err(error) => {
                sink.at(&reference.at, error.to_string());
                None
            }
        }
    }

    /// The registry of names defined so far.
    // Not used by any real section type yet; until then only by tests.
    #[allow(dead_code)]
    pub fn registry(&self) -> &NameRegistry<K> {
        &self.registry
    }

    /// Where `id` was defined.
    // Not used by any real section type yet; until then only by tests.
    #[allow(dead_code)]
    pub fn location(&self, id: Id<K>) -> &Location {
        &self.locations[id.index()]
    }

    /// Gives up the location information and returns just the registry.
    pub fn into_registry(self) -> NameRegistry<K> {
        self.registry
    }
}

impl<K: IdKind> Default for Definitions<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum ThingKind {}
    impl IdKind for ThingKind {
        const LABEL: &'static str = "thing";
    }

    fn named(name: &str, path: &str, line: u32, column: u32) -> Located<String> {
        Located {
            value: name.to_string(),
            at: Location {
                path: path.to_string(),
                line,
                column,
            },
        }
    }

    #[test]
    fn define_gives_sequential_ids_and_remembers_locations() {
        let mut defs: Definitions<ThingKind> = Definitions::new();
        let mut sink = ErrorSink::new();
        let a = defs
            .define(&named("iron", "rules/a.toml", 2, 8), &mut sink)
            .unwrap();
        let b = defs
            .define(&named("tin", "rules/b.toml", 5, 8), &mut sink)
            .unwrap();
        assert!(sink.is_empty());
        assert_eq!((a.index(), b.index()), (0, 1));
        assert_eq!(defs.location(a).to_string(), "rules/a.toml:2:8");
        assert_eq!(defs.location(b).to_string(), "rules/b.toml:5:8");
        assert_eq!(defs.registry().name(b), "tin");
        assert_eq!(defs.into_registry().len(), 2);
    }

    #[test]
    fn duplicate_is_reported_at_the_new_definition_naming_the_first() {
        let mut defs: Definitions<ThingKind> = Definitions::new();
        let mut sink = ErrorSink::new();
        defs.define(&named("copper", "rules/a.toml", 2, 8), &mut sink)
            .unwrap();
        let again = defs.define(&named("copper", "rules/b.toml", 4, 8), &mut sink);
        assert!(again.is_none());
        let errors = sink.into_result(()).unwrap_err();
        assert_eq!(
            errors.to_string(),
            "rules/b.toml:4:8: duplicate thing name 'copper' (first defined at rules/a.toml:2:8)"
        );
    }

    #[test]
    fn resolve_finds_known_names_and_reports_unknown_ones_at_the_reference() {
        let mut defs: Definitions<ThingKind> = Definitions::new();
        let mut sink = ErrorSink::new();
        let copper = defs
            .define(&named("copper", "rules/a.toml", 2, 8), &mut sink)
            .unwrap();
        assert_eq!(
            defs.resolve(&named("copper", "start/p.toml", 3, 9), &mut sink),
            Some(copper)
        );
        assert!(sink.is_empty());
        assert!(
            defs.resolve(&named("tin", "start/p.toml", 6, 9), &mut sink)
                .is_none()
        );
        let errors = sink.into_result(()).unwrap_err();
        assert_eq!(errors.to_string(), "start/p.toml:6:9: unknown thing 'tin'");
    }
}
