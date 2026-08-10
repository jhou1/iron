use std::{fs, path::Path};

#[test]
fn removed_quick_log_has_no_live_source_or_dependency_entry() {
    for path in [
        "src/app.rs",
        "src/lib.rs",
        "src/main.rs",
        "src/tui/mod.rs",
        "src/tui/dashboard.rs",
        "Cargo.toml",
    ] {
        let source = fs::read_to_string(path).unwrap();
        for removed in [
            "QuickLog",
            "quick_log",
            "crate::llm",
            "crate::config",
            "ureq",
            "toml =",
        ] {
            assert!(
                !source.contains(removed),
                "{} still contains removed feature marker {:?}",
                path,
                removed
            );
        }
    }

    for removed_file in ["src/tui/quick_log.rs", "src/llm.rs", "src/config.rs"] {
        assert!(
            !Path::new(removed_file).exists(),
            "{removed_file} still exists"
        );
    }
}
