//! Finding the parts of a setting folder: the manifest, and the data files in `rules/`
//! and `start/`. Reports the folder-structure problems (missing or unexpected entries).

use std::fs;
use std::path::{Path, PathBuf};

use super::Folder;
use super::error::ErrorSink;

const ROOT_ALLOWED: &str = "allowed: setting.toml, README.md, rules/, start/";
const MANIFEST_NAME: &str = "setting.toml";

/// One `.toml` data file found in `rules/` or `start/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DataFile {
    /// Which top-level folder the file is in.
    pub folder: Folder,
    /// Path relative to the setting folder, with forward slashes.
    pub rel: String,
    /// Path on disk.
    pub abs: PathBuf,
}

/// What discovery found.
#[derive(Debug)]
pub(crate) struct Discovered {
    /// The manifest file, if `setting.toml` exists as a file.
    pub manifest: Option<PathBuf>,
    /// Every data file: all of `rules/` first, then all of `start/`, each sorted by the
    /// bytes of the relative path.
    pub data_files: Vec<DataFile>,
    /// False if discovery may have missed data files in `rules/` or `start/`.
    pub complete: bool,
}

/// True for names that are ignored anywhere in a setting: hidden entries (names starting
/// with `.`), and the operating-system files `Thumbs.db` and `desktop.ini`.
fn is_ignored(name: &str, is_dir: bool) -> bool {
    name.starts_with('.') || (!is_dir && (name == "Thumbs.db" || name == "desktop.ini"))
}

/// One directory entry, with its kind resolved (following symlinks).
struct Entry {
    name: String,
    is_dir: bool,
    abs: PathBuf,
}

/// Lists a folder's entries sorted by name. Problems with individual entries are
/// reported (at `rel_dir` joined with the entry's name) and the entry is left out.
/// `rel_dir` is the folder's relative path with forward slashes ("" for the root).
/// Returns the entries, and whether the listing is complete: false if the folder couldn't
/// be listed (then there are no entries), or if any entry had to be left out.
fn list_dir(abs_dir: &Path, rel_dir: &str, sink: &mut ErrorSink) -> (Vec<Entry>, bool) {
    let report_folder_error = |sink: &mut ErrorSink, error: std::io::Error| {
        let message = format!("could not read folder: {error}");
        if rel_dir.is_empty() {
            sink.general(message);
        } else {
            sink.in_file(rel_dir, message);
        }
    };
    let read_dir = match fs::read_dir(abs_dir) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            report_folder_error(sink, error);
            return (Vec::new(), false);
        }
    };
    let mut complete = true;
    let mut entries = Vec::new();
    for item in read_dir {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                report_folder_error(sink, error);
                complete = false;
                continue;
            }
        };
        let os_name = item.file_name();
        let Some(name) = os_name.to_str() else {
            let lossy = os_name.to_string_lossy();
            sink.in_file(&join(rel_dir, &lossy), "file name is not valid UTF-8");
            complete = false;
            continue;
        };
        let abs = item.path();
        match fs::metadata(&abs) {
            Ok(metadata) => entries.push(Entry {
                name: name.to_string(),
                is_dir: metadata.is_dir(),
                abs,
            }),
            Err(error) => {
                sink.in_file(
                    &join(rel_dir, name),
                    format!("could not read file information: {error}"),
                );
                complete = false;
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    (entries, complete)
}

fn join(rel_dir: &str, name: &str) -> String {
    if rel_dir.is_empty() {
        name.to_string()
    } else {
        format!("{rel_dir}/{name}")
    }
}

/// Examines the setting folder at `root`, which must exist and be a folder.
pub(crate) fn discover(root: &Path, sink: &mut ErrorSink) -> Discovered {
    let mut manifest = None;
    let mut have_manifest_entry = false;
    let mut data_files = Vec::new();
    let mut have_folder = [false, false];
    let mut folders: [Option<PathBuf>; 2] = [None, None];

    let (root_entries, mut complete) = list_dir(root, "", sink);
    for entry in root_entries {
        if is_ignored(&entry.name, entry.is_dir) {
            continue;
        }
        match (entry.name.as_str(), entry.is_dir) {
            (MANIFEST_NAME, false) => {
                have_manifest_entry = true;
                manifest = Some(entry.abs);
            }
            (MANIFEST_NAME, true) => {
                have_manifest_entry = true;
                sink.in_file(MANIFEST_NAME, "expected a file, found a folder");
            }
            ("README.md", false) => {}
            ("README.md", true) => {
                sink.in_file("README.md", "expected a file, found a folder");
            }
            (name, is_dir) if Folder::from_dir_name(name).is_some() => {
                let folder = Folder::from_dir_name(name).unwrap_or(Folder::Rules);
                have_folder[folder.order()] = true;
                if is_dir {
                    folders[folder.order()] = Some(entry.abs);
                } else {
                    sink.in_file(name, "expected a folder, found a file");
                }
            }
            (name, false) => sink.in_file(
                name,
                format!("unexpected file in setting root ({ROOT_ALLOWED})"),
            ),
            (name, true) => sink.in_file(
                &format!("{name}/"),
                format!("unexpected folder in setting root ({ROOT_ALLOWED})"),
            ),
        }
    }

    if !have_manifest_entry {
        sink.in_file(MANIFEST_NAME, "missing manifest file");
    }
    for folder in [Folder::Rules, Folder::Start] {
        if !have_folder[folder.order()] {
            sink.in_file(
                &format!("{}/", folder.dir_name()),
                "missing required folder",
            );
        }
    }

    for folder in [Folder::Rules, Folder::Start] {
        match &folders[folder.order()] {
            Some(abs) => {
                let mut found = Vec::new();
                complete &= walk(abs, folder.dir_name(), folder, &mut found, sink);
                found.sort_by(|a, b| a.rel.cmp(&b.rel));
                data_files.extend(found);
            }
            // Missing, or a file instead of a folder: its data files can't be seen.
            None => complete = false,
        }
    }

    Discovered {
        manifest,
        data_files,
        complete,
    }
}

/// Walks a data folder recursively, collecting `.toml` files and reporting other files.
/// Returns false if some folder couldn't be listed or some entry had to be left out.
fn walk(
    abs_dir: &Path,
    rel_dir: &str,
    folder: Folder,
    found: &mut Vec<DataFile>,
    sink: &mut ErrorSink,
) -> bool {
    let (entries, mut complete) = list_dir(abs_dir, rel_dir, sink);
    for entry in entries {
        if is_ignored(&entry.name, entry.is_dir) {
            continue;
        }
        let rel = join(rel_dir, &entry.name);
        if entry.is_dir {
            complete &= walk(&entry.abs, &rel, folder, found, sink);
        } else if entry.name == "README.md" {
            // Allowed, and not read.
        } else if entry.name.ends_with(".toml") {
            found.push(DataFile {
                folder,
                rel,
                abs: entry.abs,
            });
        } else {
            sink.in_file(
                &rel,
                "unexpected file (only .toml files and README.md are allowed here)",
            );
        }
    }
    complete
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    fn errors_of(root: &Path) -> (Discovered, Vec<String>) {
        let mut sink = ErrorSink::new();
        let found = discover(root, &mut sink);
        let shown = match sink.into_result(()) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.iter().map(|e| e.to_string()).collect(),
        };
        (found, shown)
    }

    #[test]
    fn nested_valid_setting_is_listed_in_sorted_full_path_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/settings/valid_nested");
        let (found, errors) = errors_of(&root);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(found.manifest.is_some());
        let rels: Vec<&str> = found.data_files.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(
            rels,
            vec![
                "rules/a.toml",
                "rules/a/z.toml",
                "rules/b.toml",
                "start/x/y/deep.toml"
            ]
        );
        assert_eq!(found.data_files[0].folder, Folder::Rules);
        assert_eq!(found.data_files[3].folder, Folder::Start);
    }

    #[test]
    fn missing_entries_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let (found, errors) = errors_of(dir.path());
        assert!(found.manifest.is_none());
        assert_eq!(
            errors,
            vec![
                "setting.toml: missing manifest file",
                "rules/: missing required folder",
                "start/: missing required folder",
            ]
        );
    }

    #[test]
    fn wrong_kinds_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("setting.toml")).unwrap();
        fs::create_dir(root.join("README.md")).unwrap();
        touch(root, "rules");
        touch(root, "start");
        let (_, errors) = errors_of(root);
        assert_eq!(
            errors,
            vec![
                "setting.toml: expected a file, found a folder",
                "README.md: expected a file, found a folder",
                "rules: expected a folder, found a file",
                "start: expected a folder, found a file",
            ]
        );
    }

    #[test]
    fn unexpected_root_entries_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(root, "setting.toml");
        touch(root, "rules/README.md");
        touch(root, "start/README.md");
        touch(root, "notes.txt");
        touch(root, "extra/README.md");
        let (_, errors) = errors_of(root);
        assert_eq!(
            errors,
            vec![
                "extra/: unexpected folder in setting root (allowed: setting.toml, README.md, rules/, start/)",
                "notes.txt: unexpected file in setting root (allowed: setting.toml, README.md, rules/, start/)",
            ]
        );
    }

    #[test]
    fn ignored_entries_are_skipped_everywhere() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(root, "setting.toml");
        for base in ["", "rules/", "start/"] {
            touch(root, &format!("{base}.DS_Store"));
            touch(root, &format!("{base}.hidden/junk.txt"));
            touch(root, &format!("{base}Thumbs.db"));
            touch(root, &format!("{base}desktop.ini"));
        }
        let (found, errors) = errors_of(root);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(found.data_files.is_empty());
    }

    /// A minimal complete setting folder: the manifest and both data folders.
    fn minimal(root: &Path) {
        touch(root, "setting.toml");
        touch(root, "rules/README.md");
        touch(root, "start/README.md");
    }

    #[test]
    fn discovery_is_complete_when_every_data_file_was_seen() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/settings");
        for name in [
            "valid_nested",
            "missing_manifest",
            "unexpected_root_entry",
            "non_toml_file",
        ] {
            let (found, _) = errors_of(&root.join(name));
            assert!(found.complete, "{name} should be complete");
        }
    }

    #[test]
    fn discovery_is_incomplete_when_a_data_folder_is_missing_or_not_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "setting.toml");
        touch(dir.path(), "start/README.md");
        assert!(!errors_of(dir.path()).0.complete, "no rules/");

        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "setting.toml");
        touch(dir.path(), "rules/README.md");
        assert!(!errors_of(dir.path()).0.complete, "no start/");

        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "setting.toml");
        touch(dir.path(), "start/README.md");
        touch(dir.path(), "rules");
        assert!(!errors_of(dir.path()).0.complete, "rules as a file");
    }

    #[test]
    fn walking_a_folder_that_cannot_be_listed_is_reported_and_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nowhere");
        let mut sink = ErrorSink::new();
        let mut found = Vec::new();
        let complete = walk(&missing, "rules/sub", Folder::Rules, &mut found, &mut sink);
        assert!(!complete);
        assert!(found.is_empty());
        let errors = sink.into_result(()).unwrap_err();
        let shown: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(shown.len(), 1);
        assert!(
            shown[0].starts_with("rules/sub: could not read folder: "),
            "{shown:?}"
        );
    }

    /// Creates `rules/<bad name>.toml` where the name is not valid UTF-8, then checks
    /// discovery reports it and is incomplete.
    fn check_bad_file_name(bad_name: std::ffi::OsString) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        minimal(root);
        let path = root.join("rules").join(&bad_name);
        if let Err(error) = fs::write(&path, "") {
            panic!(
                "this platform refused to create a file whose name is not valid UTF-8 ({error}); \
                 the non-UTF-8 file name test cannot run here"
            );
        }
        let (found, errors) = errors_of(root);
        assert!(!found.complete);
        assert!(found.data_files.is_empty());
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("rules/bad")
                && errors[0].ends_with(": file name is not valid UTF-8"),
            "{errors:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_file_name_that_is_not_utf8_is_reported_and_incomplete() {
        use std::os::unix::ffi::OsStrExt;
        check_bad_file_name(std::ffi::OsStr::from_bytes(b"bad\xFF.toml").to_os_string());
    }

    #[cfg(windows)]
    #[test]
    fn a_file_name_that_is_not_utf8_is_reported_and_incomplete() {
        use std::os::windows::ffi::OsStringExt;
        // 0xD800 is an unpaired surrogate: valid on Windows file systems, not valid UTF-16.
        let units: Vec<u16> = "bad"
            .encode_utf16()
            .chain([0xD800])
            .chain(".toml".encode_utf16())
            .collect();
        check_bad_file_name(std::ffi::OsString::from_wide(&units));
    }

    #[test]
    fn non_toml_files_are_reported_and_readme_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(root, "setting.toml");
        touch(root, "rules/README.md");
        touch(root, "rules/notes.txt");
        touch(root, "rules/sub/README.md");
        touch(root, "rules/sub/A.TOML");
        touch(root, "start/sub/data.json");
        let (found, errors) = errors_of(root);
        assert!(found.data_files.is_empty());
        assert_eq!(
            errors,
            vec![
                "rules/notes.txt: unexpected file (only .toml files and README.md are allowed here)",
                "rules/sub/A.TOML: unexpected file (only .toml files and README.md are allowed here)",
                "start/sub/data.json: unexpected file (only .toml files and README.md are allowed here)",
            ]
        );
    }
}
