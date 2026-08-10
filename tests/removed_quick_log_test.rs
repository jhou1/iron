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
        "README.md",
        "README_zh.md",
        "CLAUDE.md",
        "docs/specs/dashboard-spec.md",
        "docs/specs/navigation-spec.md",
        "docs/specs/data-model-spec.md",
        "docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md",
        "locales/en.ftl",
        "locales/zh-CN.ftl",
    ] {
        let source = fs::read_to_string(path).unwrap();
        for removed in [
            "QuickLog",
            "quick_log",
            "crate::llm",
            "crate::config",
            "ureq",
            "toml =",
            "Quick Log",
            "快捷记录",
            "Abbreviations",
            "quicklog-",
            "key-quick-log",
            "abbreviations-",
        ] {
            assert!(
                !source.contains(removed),
                "{} still contains removed feature marker {:?}",
                path,
                removed
            );
        }
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
