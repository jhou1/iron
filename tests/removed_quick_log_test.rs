use std::{
    fs,
    path::{Path, PathBuf},
};

const REMOVED_MARKERS: &[&str] = &[
    "QuickLog",
    "quick_log",
    "crate::llm",
    "crate::config",
    "ureq",
    "toml =",
    "Quick Log",
    "快捷记录",
    "Abbreviation",
    "create_abbreviation",
    "list_abbreviations",
    "update_abbreviation",
    "delete_abbreviation",
    "quicklog-",
    "key-quick-log",
    "abbreviations-",
];

fn assert_no_removed_markers(path: &str, source: &str) {
    for removed in REMOVED_MARKERS {
        assert!(
            !source.contains(removed),
            "{} still contains removed feature marker {:?}",
            path,
            removed
        );
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
#[should_panic(expected = "still contains removed feature marker")]
fn banned_marker_scan_rejects_live_marker() {
    assert_no_removed_markers("src/fake.rs", "pub struct QuickLog;");
}

#[test]
fn removed_quick_log_has_no_live_source_or_dependency_entry() {
    let mut live_paths = vec![PathBuf::from("Cargo.toml")];
    for root in ["src", "locales", "tests"] {
        collect_files(Path::new(root), &mut live_paths);
    }
    live_paths.sort();

    for path in live_paths {
        if path == Path::new("tests/removed_quick_log_test.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        assert_no_removed_markers(&path.display().to_string(), &source);
    }

    for removed_file in [
        "src/tui/quick_log.rs",
        "src/llm.rs",
        "src/config.rs",
        "docs/specs/quick-log-spec.md",
        "docs/specs/abbreviations-spec.md",
        "docs/superpowers/specs/2026-05-12-quick-log-design.md",
        "docs/superpowers/plans/2026-05-12-quick-log-implementation.md",
    ] {
        assert!(
            !Path::new(removed_file).exists(),
            "{removed_file} still exists"
        );
    }
}
