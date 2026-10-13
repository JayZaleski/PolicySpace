//! Integration tests for the setting loader, through the engine's public API only.
//! (The framework rules that need the test-only section types are tested inside the
//! crate, in `src/setting/test_content.rs`.)

use std::fs;
use std::path::{Path, PathBuf};

use policyspace_engine::{LoadErrors, SUPPORTED_FORMAT_VERSION, load_setting};

fn test_setting(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/settings")
        .join(name)
}

fn lines(errors: &LoadErrors) -> Vec<String> {
    errors.iter().map(|e| e.to_string()).collect()
}

fn failure_of(path: &Path) -> LoadErrors {
    match load_setting(path) {
        Ok(_) => panic!("{} should not load", path.display()),
        Err(errors) => errors,
    }
}

fn failures_of(name: &str) -> Vec<String> {
    lines(&failure_of(&test_setting(name)))
}

const MANIFEST: &str = "name = \"Temp\"\nformat_version = 1\n";

/// Creates a minimal valid setting in a temp folder.
fn temp_setting() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("setting.toml"), MANIFEST).unwrap();
    fs::create_dir(dir.path().join("rules")).unwrap();
    fs::create_dir(dir.path().join("start")).unwrap();
    dir
}

#[test]
fn sample_setting_loads() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../settings/sample");
    let setting = load_setting(&path).expect("the sample setting should load");
    assert_eq!(setting.manifest.name, "Sample Setting");
    assert_eq!(setting.manifest.format_version, 1);
    assert_eq!(setting.manifest.format_version, SUPPORTED_FORMAT_VERSION);
    assert_eq!(setting.start.map.len(), 19);
    assert_eq!(setting.rules.resources.len(), 2);
    assert_eq!(setting.start.deposits.len(), 10);
}

#[test]
fn valid_minimal_loads_with_manifest_fields() {
    let setting = load_setting(test_setting("valid_minimal")).unwrap();
    assert_eq!(setting.manifest.name, "Minimal Setting");
    assert_eq!(
        setting.manifest.description.as_deref(),
        Some("The smallest valid setting.")
    );
    assert_eq!(setting.manifest.format_version, 1);
}

#[test]
fn valid_nested_loads() {
    assert!(load_setting(test_setting("valid_nested")).is_ok());
}

#[test]
fn missing_manifest_rules_and_start() {
    assert_eq!(
        failures_of("missing_manifest"),
        vec!["setting.toml: missing manifest file"]
    );
    assert_eq!(
        failures_of("missing_rules"),
        vec!["rules/: missing required folder"]
    );
    assert_eq!(
        failures_of("missing_start"),
        vec!["start/: missing required folder"]
    );
}

#[test]
fn missing_setting_folder_is_reported() {
    let errors = failure_of(&test_setting("does_not_exist"));
    assert_eq!(errors.len(), 1);
    let error = errors.iter().next().unwrap();
    assert_eq!(error.path(), None);
    assert!(
        error.message().starts_with("setting folder '")
            && error
                .message()
                .ends_with("' does not exist or is not a folder"),
        "{error}"
    );
}

#[test]
fn unexpected_root_entries() {
    assert_eq!(
        failures_of("unexpected_root_entry"),
        vec![
            "extra/: unexpected folder in setting root (allowed: setting.toml, README.md, rules/, start/)",
            "notes.txt: unexpected file in setting root (allowed: setting.toml, README.md, rules/, start/)",
        ]
    );
}

#[test]
fn manifest_problems() {
    assert_eq!(
        failures_of("manifest_missing_fields"),
        vec![
            "setting.toml: missing required key 'name' in manifest",
            "setting.toml: missing required key 'format_version' in manifest",
        ]
    );
    assert_eq!(
        failures_of("manifest_bad_values"),
        vec![
            "setting.toml:1:8: key 'name' in manifest must not be empty",
            "setting.toml:2:15: key 'description' in manifest must be a string, found integer",
            "setting.toml:3:18: unsupported format_version 2 (this engine supports format version 1)",
            "setting.toml:4:1: unknown key 'author' in manifest",
        ]
    );
}

#[test]
fn invalid_toml_reports_each_syntax_error_and_nothing_else() {
    assert_eq!(
        failures_of("invalid_toml"),
        vec![
            "rules/broken.toml:2:8: invalid TOML: string values must be quoted, expected literal string",
            "rules/broken.toml:4:10: invalid TOML: string values must be quoted, expected literal string",
        ]
    );
}

#[test]
fn invalid_utf8_is_reported_at_the_first_bad_byte() {
    let dir = temp_setting();
    fs::write(dir.path().join("rules/bad.toml"), b"# fine\n# \xFF\xFE\n").unwrap();
    assert_eq!(
        lines(&failure_of(dir.path())),
        vec!["rules/bad.toml:2:3: file is not valid UTF-8"]
    );
}

#[test]
fn unknown_sections() {
    assert_eq!(
        failures_of("unknown_section"),
        vec![
            "rules/widgets.toml:1:3: unknown section 'widget'",
            "start/gadgets.toml:3:3: unknown section 'gadget'",
        ]
    );
}

#[test]
fn non_toml_files_are_reported() {
    assert_eq!(
        failures_of("non_toml_file"),
        vec![
            "rules/notes.txt: unexpected file (only .toml files and README.md are allowed here)",
            "start/sub/data.json: unexpected file (only .toml files and README.md are allowed here)",
        ]
    );
}

#[test]
fn ignored_files_do_not_cause_errors_and_a_bom_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut manifest = vec![0xEF, 0xBB, 0xBF];
    manifest.extend_from_slice(MANIFEST.as_bytes());
    fs::write(root.join("setting.toml"), manifest).unwrap();
    fs::create_dir(root.join("rules")).unwrap();
    fs::create_dir(root.join("start")).unwrap();
    for base in ["", "rules/", "start/"] {
        fs::write(root.join(format!("{base}.DS_Store")), "x").unwrap();
        fs::create_dir(root.join(format!("{base}.hidden"))).unwrap();
        fs::write(root.join(format!("{base}.hidden/junk.txt")), "x").unwrap();
        fs::write(root.join(format!("{base}Thumbs.db")), "x").unwrap();
        fs::write(root.join(format!("{base}desktop.ini")), "x").unwrap();
    }
    let setting = load_setting(root).unwrap_or_else(|errors| panic!("should load:\n{errors}"));
    assert_eq!(setting.manifest.name, "Temp");
}

#[test]
fn many_errors_are_all_reported_in_order_and_no_setting_is_produced() {
    let path = test_setting("many_errors");
    let result = load_setting(&path);
    let errors = result.expect_err("many_errors must not load");
    assert_eq!(
        lines(&errors),
        vec![
            "setting.toml:2:18: unsupported format_version 3 (this engine supports format version 1)",
            "rules/a.toml:1:3: unknown section 'widget'",
            "rules/b.txt: unexpected file (only .toml files and README.md are allowed here)",
            "start/: missing required folder",
        ]
    );
    assert_eq!(errors.len(), 4);
    assert_eq!(
        errors.to_string(),
        lines(&errors).join("\n"),
        "Display writes one error per line"
    );
}

#[test]
fn loading_the_same_setting_twice_gives_the_same_result() {
    let valid = test_setting("valid_nested");
    assert_eq!(
        format!("{:?}", load_setting(&valid)),
        format!("{:?}", load_setting(&valid))
    );
    let broken = test_setting("many_errors");
    assert_eq!(failure_of(&broken), failure_of(&broken));
}
