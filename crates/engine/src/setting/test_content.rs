//! A test-only content type that exercises the shared loader rules before any real
//! section types exist: a "thing" defined in `rules/`, and a "placed thing" in `start/`
//! that refers to a thing by name.

use std::path::{Path, PathBuf};

use crate::id::{Id, IdKind};

use super::defs::Definitions;
use super::error::{ErrorSink, Located};
use super::reader::EntryReader;
use super::{Folder, SectionType, SettingContent, load_with};

pub(crate) enum ThingKind {}
impl IdKind for ThingKind {
    const LABEL: &'static str = "thing";
}

#[derive(Default)]
pub(crate) struct TestContent {
    pub things: Definitions<ThingKind>,
    /// Unresolved references, in read order.
    pub placed: Vec<Located<String>>,
    /// Filled by `resolve`.
    pub resolved: Vec<Id<ThingKind>>,
}

fn read_thing(content: &mut TestContent, reader: &mut EntryReader<'_, '_>) {
    if let Some(name) = reader.required_name("name") {
        content.things.define(&name, reader.errors());
    }
}

fn read_placed_thing(content: &mut TestContent, reader: &mut EntryReader<'_, '_>) {
    if let Some(thing) = reader.required_name("thing") {
        content.placed.push(thing);
    }
    // Read and discarded: they exist to exercise the readers.
    reader.optional_integer("count");
    reader.optional_bool("visible");
}

impl SettingContent for TestContent {
    fn section_types() -> Vec<SectionType<Self>> {
        vec![
            SectionType {
                header: "thing",
                folder: Folder::Rules,
                read: read_thing,
            },
            SectionType {
                header: "placed_thing",
                folder: Folder::Start,
                read: read_placed_thing,
            },
        ]
    }

    fn resolve(&mut self, sink: &mut ErrorSink) {
        for reference in &self.placed {
            if let Some(id) = self.things.resolve(reference, sink) {
                self.resolved.push(id);
            }
        }
    }
}

fn test_setting(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/settings")
        .join(name)
}

/// Loads a test setting with `TestContent`, returning the error lines on failure.
fn errors_of(name: &str) -> Vec<String> {
    match load_with::<TestContent>(&test_setting(name)) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.iter().map(|e| e.to_string()).collect(),
    }
}

#[test]
fn cross_file_references_resolve_and_ids_follow_file_read_order() {
    let (manifest, content) = load_with::<TestContent>(&test_setting("framework_valid"))
        .unwrap_or_else(|errors| panic!("framework_valid should load:\n{errors}"));
    assert_eq!(manifest.format_version, 1);
    let registry = content.things.registry();
    let names: Vec<(usize, &str)> = registry.iter().map(|(id, n)| (id.index(), n)).collect();
    assert_eq!(names, vec![(0, "iron"), (1, "tin"), (2, "copper")]);
    // Placed things refer to copper and tin, in file order.
    let resolved: Vec<&str> = content
        .resolved
        .iter()
        .map(|&id| registry.name(id))
        .collect();
    assert_eq!(resolved, vec!["copper", "tin"]);
    assert_eq!(content.resolved[0].index(), 2);
    assert_eq!(content.resolved[1].index(), 1);
}

#[test]
fn loading_the_same_setting_twice_gives_the_same_registry_and_ids() {
    let path = test_setting("framework_valid");
    let load = || {
        load_with::<TestContent>(&path)
            .unwrap_or_else(|errors| panic!("framework_valid should load:\n{errors}"))
            .1
    };
    let first = load();
    let second = load();
    let list = |content: &TestContent| -> Vec<(usize, String)> {
        content
            .things
            .registry()
            .iter()
            .map(|(id, n)| (id.index(), n.to_string()))
            .collect()
    };
    assert_eq!(list(&first), list(&second));
    assert_eq!(first.resolved, second.resolved);
}

#[test]
fn duplicate_across_files_names_both_places() {
    assert_eq!(
        errors_of("framework_duplicate"),
        vec!["rules/b.toml:2:8: duplicate thing name 'copper' (first defined at rules/a.toml:2:8)"]
    );
}

#[test]
fn sections_in_the_wrong_folder_are_errors() {
    assert_eq!(
        errors_of("framework_wrong_folder"),
        vec![
            "rules/r.toml:1:3: section 'placed_thing' belongs in start/, not rules/",
            "start/s.toml:1:3: section 'thing' belongs in rules/, not start/",
        ]
    );
}

#[test]
fn unknown_reference_is_reported_at_the_reference() {
    assert_eq!(
        errors_of("framework_bad_reference"),
        vec!["start/places.toml:2:9: unknown thing 'bronze'"]
    );
}

#[test]
fn several_problems_in_one_section_are_all_reported() {
    assert_eq!(
        errors_of("framework_field_errors"),
        vec![
            "rules/a.toml:2:8: invalid name 'Copper' (names must start with a lowercase letter and contain only lowercase letters, digits, and underscores)",
            "rules/a.toml:3:1: unknown key 'colour' in thing",
            "rules/b.toml:1:1: missing required key 'name' in thing",
            "rules/x.toml:1:1: section 'thing' must be written as [[thing]]",
            "start/p.toml:2:9: key 'thing' in placed_thing must be a string, found integer",
            "start/p.toml:3:9: key 'count' in placed_thing must be an integer, found string",
        ]
    );
}

#[test]
fn phase_two_is_skipped_after_a_broken_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("setting.toml"),
        "name = \"T\"\nformat_version = 1\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("rules")).unwrap();
    std::fs::create_dir_all(root.join("start")).unwrap();
    std::fs::write(
        root.join("rules/copper.toml"),
        "[[thing]]\nname = \"copper\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("start/places.toml"),
        "[[placed_thing]]\nthing = \"bronze\"\n",
    )
    .unwrap();
    std::fs::write(root.join("rules/zz.toml"), "[[thing\nname = \n").unwrap();
    let errors = match load_with::<TestContent>(root) {
        Ok(_) => panic!("should fail"),
        Err(errors) => errors,
    };
    let shown: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
    assert_eq!(
        shown,
        vec!["rules/zz.toml:1:8: invalid TOML: unclosed array table, expected `]]`"]
    );
}
