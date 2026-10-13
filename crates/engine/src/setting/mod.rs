//! Loading a setting: a folder of TOML files that describes a world.
//!
//! The engine knows nothing about any particular setting. A setting is data: the goods,
//! the map, and the rules of a world, written as TOML files in a folder. [`load_setting`]
//! reads such a folder, checks it strictly, and returns a [`LoadedSetting`] (the manifest,
//! the persistent rules, and the starting state). A loaded setting is not a running
//! world: later Steps build the world state from it.
//!
//! # Folder layout
//! ```text
//! <setting folder>/
//!   setting.toml     the manifest (required)
//!   README.md        optional notes
//!   rules/           persistent rules (required, may hold only a README.md)
//!   start/           starting state: what turn 0 looks like (required, same)
//! ```
//! - **Rules** are constants that never change during play, mostly definitions of kinds
//!   of things. **Starting state** is the turn-0 value of everything that can change.
//!   Starting-state files refer to rules by name.
//! - The root may hold only the entries above. Anything else is an error.
//! - Every `.toml` file in `rules/` and `start/` is read, in subfolders at any depth,
//!   in sorted order of the full relative path (with forward slashes), all of `rules/`
//!   first. What a file defines is given by its `[[section]]` headers, not by its name.
//!   A `README.md` is allowed in any folder and never read. Hidden entries (names that
//!   start with `.`), `Thumbs.db` and `desktop.ini` are ignored. Any other file is an
//!   error.
//!
//! # Strictness and error reporting
//! Unknown sections, unknown keys, bad names, wrong types, duplicate names and unknown
//! references are all errors. The loader collects every problem it can find and returns
//! them all together in a [`LoadErrors`], in a fixed order (by file in read order, then
//! by position), each with the file's path, line and column. A file that can't be read,
//! isn't UTF-8, or isn't valid TOML reports only that; its content isn't checked, and if
//! any file is broken this way, references between things aren't checked either. They
//! aren't checked either if discovery couldn't see every data file (a missing `rules/` or
//! `start/`, a folder that can't be listed, or a file name that isn't valid UTF-8), since
//! the thing a reference points to may be in a file the loader never saw. Other problems
//! (unknown keys, bad names, duplicates, files rejected for their extension) don't skip
//! the check. If there is any error, no setting is produced.
//!
//! # How loading works
//! - **Phase 1** reads every file in order. Each `[[section]]` entry is handed to its
//!   section type's `read` function (see `SectionType`), which reads the entry's fields
//!   with an `EntryReader`, defines names with `Definitions`, and stores any references
//!   unresolved.
//! - **Phase 2** (`SettingContent::resolve`) checks the references once every file has
//!   been read, so a reference can point into any file.
//!
//! # Adding a section type (for later Steps)
//! 1. Define the data, and store it in [`Rules`] or [`StartState`]. Goods are the worked
//!    example: the domain types are in [`crate::goods`] (which knows nothing about
//!    files), the reader is in `setting/goods.rs`, and the finished list is
//!    `Rules::goods`.
//! 2. Add a `SectionType` to `WorldContent::section_types`, naming its header, the one
//!    folder it belongs in, and a `read` function.
//! 3. Check references in `WorldContent::resolve`.
//!
//! File discovery, merging, duplicate detection, name checking and error reporting are
//! shared and need not be rebuilt.

mod defs;
mod discover;
mod error;
mod goods;
mod reader;
mod source;
#[cfg(test)]
mod test_content;

use std::path::Path;

use toml::de::DeTable;

pub use error::{LoadError, LoadErrors};

use crate::goods::Goods;
use error::{ErrorSink, Location};
use goods::GoodsReader;
use reader::EntryReader;
use source::{SourceFile, read_toml};

/// The only setting format version this engine reads.
pub const SUPPORTED_FORMAT_VERSION: i64 = 1;

/// A setting that loaded without errors.
#[derive(Debug)]
pub struct LoadedSetting {
    /// The manifest (setting.toml).
    pub manifest: Manifest,
    /// Persistent rules (from rules/).
    pub rules: Rules,
    /// Starting state (from start/). Empty until Step 04.
    pub start: StartState,
}

/// The contents of `setting.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Setting format version; always [`SUPPORTED_FORMAT_VERSION`] after a successful load.
    pub format_version: i64,
}

/// Persistent rules. Step 05 onward add more fields.
#[derive(Debug, Default)]
pub struct Rules {
    /// Every good the setting defines, in load order.
    pub goods: Goods,
}

/// Starting state (turn 0). No fields yet; Steps 04 and 05 add them.
#[derive(Debug, Default)]
pub struct StartState {}

/// Loads the setting folder at `path`. Either every check passes and a [`LoadedSetting`]
/// is returned, or every problem found is returned (sorted; see the module docs).
pub fn load_setting(path: impl AsRef<Path>) -> Result<LoadedSetting, LoadErrors> {
    let (manifest, content) = load_with::<WorldContent>(path.as_ref())?;
    Ok(LoadedSetting {
        manifest,
        rules: Rules {
            goods: content.goods.finish(),
        },
        start: content.start,
    })
}

/// The two top-level folders that hold data files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Folder {
    /// `rules/`: persistent rules.
    Rules,
    /// `start/`: starting state.
    Start,
}

impl Folder {
    /// The folder's name on disk.
    pub fn dir_name(self) -> &'static str {
        match self {
            Folder::Rules => "rules",
            Folder::Start => "start",
        }
    }

    /// The folder with this name, if any.
    pub fn from_dir_name(name: &str) -> Option<Folder> {
        match name {
            "rules" => Some(Folder::Rules),
            "start" => Some(Folder::Start),
            _ => None,
        }
    }

    /// 0 for `rules`, 1 for `start`: the read order, and an index for small arrays.
    pub fn order(self) -> usize {
        match self {
            Folder::Rules => 0,
            Folder::Start => 1,
        }
    }
}

/// One kind of `[[section]]` that a setting file may contain.
pub(crate) struct SectionType<C> {
    /// The `[[header]]` name.
    pub header: &'static str,
    /// The only folder this section may appear in.
    pub folder: Folder,
    /// Reads one entry into the content. Runs in phase 1, in file order then entry order.
    pub read: fn(&mut C, &mut EntryReader<'_, '_>),
}

/// What a setting's data files are loaded into.
pub(crate) trait SettingContent: Default {
    /// The section types this content accepts.
    fn section_types() -> Vec<SectionType<Self>>;
    /// Phase 2: resolve references once every file has been read.
    fn resolve(&mut self, sink: &mut ErrorSink);
}

/// The real content. Steps 04 onward add more section types.
#[derive(Default)]
pub(crate) struct WorldContent {
    pub goods: GoodsReader,
    pub start: StartState,
}

impl SettingContent for WorldContent {
    fn section_types() -> Vec<SectionType<Self>> {
        vec![SectionType {
            header: "good",
            folder: Folder::Rules,
            read: |content, reader| content.goods.read(reader),
        }]
    }

    // Goods refer to nothing, so there is nothing to check yet.
    fn resolve(&mut self, _sink: &mut ErrorSink) {}
}

/// Turns a sink that holds errors into the `Err` result.
fn fail<T>(sink: ErrorSink) -> Result<T, LoadErrors> {
    match sink.into_result(()) {
        Err(errors) => Err(errors),
        Ok(()) => unreachable!("fail() is only called after an error was reported"),
    }
}

/// The generic loader. [`load_setting`] is `load_with::<WorldContent>` plus building the
/// [`LoadedSetting`]. Tests supply their own content type.
pub(crate) fn load_with<C: SettingContent>(path: &Path) -> Result<(Manifest, C), LoadErrors> {
    let mut sink = ErrorSink::new();
    if !path.is_dir() {
        sink.general(format!(
            "setting folder '{}' does not exist or is not a folder",
            path.display()
        ));
        return fail(sink);
    }

    let discovered = discover::discover(path, &mut sink);
    // True if any file couldn't be read, wasn't UTF-8, or wasn't valid TOML.
    let mut rejected = false;

    // The manifest.
    let mut manifest = None;
    if let Some(abs) = &discovered.manifest {
        match read_toml(abs, "setting.toml", &mut sink, read_manifest) {
            Some(read) => manifest = read,
            None => rejected = true,
        }
    }

    // Phase 1: read every data file, in order.
    let section_types = C::section_types();
    let mut content = C::default();
    for data_file in &discovered.data_files {
        let read = read_toml(
            &data_file.abs,
            &data_file.rel,
            &mut sink,
            |file, table, sink| {
                read_sections(
                    file,
                    table,
                    data_file.folder,
                    &section_types,
                    &mut content,
                    sink,
                )
            },
        );
        if read.is_none() {
            rejected = true;
        }
    }

    // Phase 2: check references, unless a broken file or a file discovery never saw may
    // have hidden some definitions.
    if !rejected && discovered.complete {
        content.resolve(&mut sink);
    }

    let Some(manifest) = manifest else {
        if sink.is_empty() {
            sink.general("internal error: the manifest was not read but no error was reported");
        }
        return fail(sink);
    };
    sink.into_result((manifest, content))
}

/// Reads the manifest's root table. Returns the manifest only if it is fully valid.
fn read_manifest(file: &SourceFile, table: &DeTable<'_>, sink: &mut ErrorSink) -> Option<Manifest> {
    let mut reader = EntryReader::for_manifest(file, table, sink);
    let name = reader.required_string("name");
    let description = reader.optional_string("description");
    let format_version = reader.required_integer("format_version");

    let name = name.and_then(|name| {
        if name.value.trim().is_empty() {
            reader
                .errors()
                .at(&name.at, "key 'name' in manifest must not be empty");
            None
        } else {
            Some(name.value)
        }
    });
    let format_version = format_version.and_then(|version| {
        if version.value == SUPPORTED_FORMAT_VERSION {
            Some(version.value)
        } else {
            reader.errors().at(
                &version.at,
                format!(
                    "unsupported format_version {} (this engine supports format version {SUPPORTED_FORMAT_VERSION})",
                    version.value
                ),
            );
            None
        }
    });
    reader.finish();

    Some(Manifest {
        name: name?,
        description: description.map(|d| d.value),
        format_version: format_version?,
    })
}

/// Phase 1 for one parsed data file: hands each section entry to its section type.
fn read_sections<C>(
    file: &SourceFile,
    table: &DeTable<'_>,
    folder: Folder,
    section_types: &[SectionType<C>],
    content: &mut C,
    sink: &mut ErrorSink,
) {
    for (key, value) in table.iter() {
        let header: &str = key.get_ref().as_ref();
        let key_at = file.location_of(key.span());
        let Some(section) = section_types.iter().find(|s| s.header == header) else {
            sink.at(&key_at, format!("unknown section '{header}'"));
            continue;
        };
        if section.folder != folder {
            sink.at(
                &key_at,
                format!(
                    "section '{header}' belongs in {}/, not {}/",
                    section.folder.dir_name(),
                    folder.dir_name()
                ),
            );
            continue;
        }
        let not_array_of_tables = format!("section '{header}' must be written as [[{header}]]");
        let Some(entries) = value.get_ref().as_array() else {
            sink.at(&key_at, not_array_of_tables);
            continue;
        };
        for entry in entries.iter() {
            let Some(entry_table) = entry.get_ref().as_table() else {
                let at: Location = file.location_of(entry.span());
                sink.at(&at, not_array_of_tables.clone());
                continue;
            };
            let entry_at = file.location_of(entry.span());
            let mut reader = EntryReader::new(file, entry_table, entry_at, section.header, sink);
            (section.read)(content, &mut reader);
            reader.finish();
        }
    }
}
