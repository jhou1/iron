use std::{
    fs,
    path::{Path, PathBuf},
};

fn compact_alphanumeric(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalized_path(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

fn path_marker(path: &str) -> Option<&'static str> {
    for component in normalized_path(path).split('/') {
        let stem = component.split('.').next().unwrap_or(component);
        let compact = compact_alphanumeric(stem);
        if compact.contains("quicklog") {
            return Some("quick log path");
        }
        if compact == "llm" || compact.starts_with("llm") {
            return Some("LLM path");
        }
        if compact == "config" {
            return Some("config path");
        }
        if compact.contains("abbreviation") {
            return Some("abbreviation path");
        }
    }
    None
}

fn remove_exact_occurrences(mut source: String, fragment: &str, limit: usize) -> String {
    for _ in 0..limit {
        let Some(index) = source.find(fragment) else {
            break;
        };
        source.replace_range(index..index + fragment.len(), "");
    }
    source
}

fn without_allowed_migration_compatibility(path: &str, source: &str) -> String {
    let mut remaining = source.to_owned();
    match normalized_path(path).as_str() {
        "src/db.rs" => {
            remaining =
                remove_exact_occurrences(remaining, "DROP TABLE IF EXISTS abbreviations", 1);
        }
        "tests/db_test.rs" => {
            for (fragment, limit) in [
                (
                    "opening_database_drops_abbreviations_and_preserves_training_data",
                    1,
                ),
                ("CREATE TABLE abbreviations", 1),
                ("INSERT INTO abbreviations", 1),
                ("name='abbreviations'", 1),
                ("abbreviation_tables", 2),
            ] {
                remaining = remove_exact_occurrences(remaining, fragment, limit);
            }
        }
        _ => {}
    }
    remaining
}

fn contains_token(source: &str, expected: &str) -> bool {
    source
        .split(|character: char| !character.is_alphanumeric())
        .any(|token| token.eq_ignore_ascii_case(expected))
}

fn has_adjacent_tokens(source: &str, first: &str, second: &str) -> bool {
    let tokens: Vec<String> = source
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect();
    tokens
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

fn has_toml_dependency(path: &str, source: &str) -> bool {
    if normalized_path(path) != "cargo.toml" {
        return false;
    }
    source.lines().any(|line| {
        let trimmed = line.trim();
        if let Some((name, _)) = trimmed.split_once('=') {
            let dependency_name = name.split('.').next().unwrap_or(name);
            return compact_alphanumeric(dependency_name) == "toml";
        }
        trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && compact_alphanumeric(trimmed).ends_with("dependenciestoml")
    })
}

fn content_marker(path: &str, source: &str) -> Option<&'static str> {
    let source = without_allowed_migration_compatibility(path, source);
    if compact_alphanumeric(&source).contains("quicklog") {
        return Some("quick log content");
    }
    if source.contains("快捷记录") {
        return Some("translated quick log content");
    }
    if contains_token(&source, "llm") {
        return Some("LLM content");
    }
    if has_adjacent_tokens(&source, "mod", "config")
        || has_adjacent_tokens(&source, "crate", "config")
    {
        return Some("config module content");
    }
    if contains_token(&source, "ureq") {
        return Some("ureq dependency");
    }
    if has_toml_dependency(path, &source) {
        return Some("toml dependency");
    }
    if source.to_lowercase().contains("abbreviation") {
        return Some("abbreviation content");
    }
    None
}

fn removed_marker(path: &str, source: &str) -> Option<&'static str> {
    path_marker(path).or_else(|| content_marker(path, source))
}

fn assert_no_removed_markers(path: &str, source: &str) {
    if let Some(marker) = removed_marker(path, source) {
        panic!("{path} still contains removed feature marker {marker:?}");
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
fn banned_marker_scan_rejects_live_content_and_path_variants() {
    let cases = [
        ("quick log type", "src/fake.rs", "pub struct QuickLog;"),
        ("quick log module", "src/fake.rs", "mod quick_log;"),
        (
            "quick log locale key",
            "locales/en.ftl",
            "quick-log-title = Log now",
        ),
        (
            "compact quick log locale key",
            "locales/en.ftl",
            "quicklog-title = Log now",
        ),
        (
            "translated quick log locale value",
            "locales/zh-CN.ftl",
            "nav-item = 快捷记录",
        ),
        ("llm module", "src/lib.rs", "mod llm;"),
        ("spaced llm import", "src/lib.rs", "use crate :: LLM;"),
        ("config module", "src/lib.rs", "mod config ;"),
        ("spaced config import", "src/lib.rs", "use crate :: CONFIG;"),
        ("uppercase HTTP dependency", "Cargo.toml", "UREQ = \"2\""),
        (
            "compact TOML dependency",
            "Cargo.toml",
            "ToMl={ version = \"0.8\" }",
        ),
        (
            "workspace TOML dependency",
            "Cargo.toml",
            "toml.workspace = true",
        ),
        (
            "singular abbreviation model",
            "src/model.rs",
            "struct abbreviation;",
        ),
        (
            "plural abbreviation model",
            "src/model.rs",
            "struct ABBREVIATIONS;",
        ),
        (
            "create abbreviation API",
            "src/db.rs",
            "fn create_abbreviation() {}",
        ),
        (
            "list abbreviation API",
            "src/db.rs",
            "fn list_abbreviations() {}",
        ),
        (
            "update abbreviation API",
            "src/db.rs",
            "fn update_abbreviation() {}",
        ),
        (
            "delete abbreviation API",
            "src/db.rs",
            "fn delete_abbreviation() {}",
        ),
        (
            "abbreviation locale key",
            "locales/en.ftl",
            "abbreviations-title = Expansions",
        ),
        (
            "abbreviation table",
            "src/db.rs",
            "CREATE TABLE abbreviations (id INTEGER);",
        ),
        (
            "extra abbreviation in migration source",
            "src/db.rs",
            "DROP TABLE IF EXISTS abbreviations;\nfn abbreviations_ui() {}",
        ),
        (
            "extra abbreviation in migration test",
            "tests/db_test.rs",
            "CREATE TABLE abbreviations (id INTEGER);\nfn create_abbreviation() {}",
        ),
        ("nested llm path", "src/LLM/mod.rs", "pub fn send() {}"),
        (
            "nested config path",
            "src\\CONFIG\\mod.rs",
            "pub fn load() {}",
        ),
        (
            "quick log file path",
            "src/tui/quick_log.rs",
            "pub fn render() {}",
        ),
        (
            "hyphenated quick log path",
            "src/tui/quick-log/screen.rs",
            "pub fn render() {}",
        ),
        (
            "compact quick log path",
            "locales/quicklog.ftl",
            "title = Log",
        ),
        (
            "abbreviation path",
            "src/abbreviations/mod.rs",
            "pub fn expand() {}",
        ),
    ];

    for (label, path, source) in cases {
        assert!(
            removed_marker(path, source).is_some(),
            "audit accepted {label}: {path}: {source}"
        );
    }
}

#[test]
fn exact_migration_compatibility_occurrences_are_allowed() {
    for (path, source) in [
        (
            "src/db.rs",
            ".execute(\"DROP TABLE IF EXISTS abbreviations\", [])?;",
        ),
        (
            "tests/db_test.rs",
            "fn opening_database_drops_abbreviations_and_preserves_training_data() {\n\
             CREATE TABLE abbreviations (id INTEGER);\n\
             INSERT INTO abbreviations (short) VALUES ('DL');\n\
             let abbreviation_tables = query(\"name='abbreviations'\");\n\
             assert_eq!(abbreviation_tables, 0);\n\
             }",
        ),
    ] {
        assert_eq!(removed_marker(path, source), None, "{path}: {source}");
    }
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
