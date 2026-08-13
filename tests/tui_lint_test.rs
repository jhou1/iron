use std::collections::HashSet;
use std::path::Path;

fn starts_char_literal(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut probe = chars.clone();
    match probe.next() {
        Some('\\') => match probe.next() {
            Some('u') if probe.next() == Some('{') => {
                for character in probe.by_ref() {
                    if character == '}' {
                        return probe.next() == Some('\'');
                    }
                }
                false
            }
            Some('x') => probe.nth(1).is_some() && probe.next() == Some('\''),
            Some(_) => probe.next() == Some('\''),
            None => false,
        },
        Some(character) if character != '\n' && character != '\'' => probe.next() == Some('\''),
        _ => false,
    }
}

fn source_code_contains(source: &str, needle: &str) -> bool {
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut block_comment_depth = 0usize;
    let mut in_line_comment = false;
    let mut in_string = false;
    let mut in_char = false;
    let mut raw_string_hashes = None;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if in_line_comment {
            if character == '\n' {
                in_line_comment = false;
                code.push(character);
            }
            continue;
        }

        if block_comment_depth > 0 {
            match (character, chars.peek().copied()) {
                ('/', Some('*')) => {
                    chars.next();
                    block_comment_depth += 1;
                }
                ('*', Some('/')) => {
                    chars.next();
                    block_comment_depth -= 1;
                }
                ('\n', _) => code.push(character),
                _ => {}
            }
            continue;
        }

        if let Some(hash_count) = raw_string_hashes {
            if character == '"' {
                let mut probe = chars.clone();
                if (0..hash_count).all(|_| probe.next() == Some('#')) {
                    for _ in 0..hash_count {
                        chars.next();
                    }
                    raw_string_hashes = None;
                }
            } else if character == '\n' {
                code.push(character);
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            } else if character == '\n' {
                code.push(character);
            }
            continue;
        }

        if in_char {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                in_char = false;
            }
            continue;
        }

        if character == 'r' {
            let mut probe = chars.clone();
            let mut hash_count = 0;
            while probe.next_if_eq(&'#').is_some() {
                hash_count += 1;
            }
            if probe.next() == Some('"') {
                for _ in 0..=hash_count {
                    chars.next();
                }
                raw_string_hashes = Some(hash_count);
                continue;
            }
        }

        match (character, chars.peek().copied()) {
            ('/', Some('/')) => {
                chars.next();
                in_line_comment = true;
            }
            ('/', Some('*')) => {
                chars.next();
                block_comment_depth = 1;
            }
            ('"', _) => {
                in_string = true;
            }
            ('\'', _) if starts_char_literal(&chars) => {
                in_char = true;
            }
            _ => code.push(character),
        }
    }

    code.contains(needle)
}

fn rust_sources(root: &Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .unwrap_or_else(|_| panic!("failed to read {}", directory.display()))
        {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

fn paragraph_violations(root: &Path, allowlist: &HashSet<&str>) -> Vec<String> {
    let mut violations = Vec::new();
    for path in rust_sources(root) {
        let relative = path.strip_prefix(root).unwrap().to_string_lossy();
        let source = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        for (line_num_0, line) in lines.iter().enumerate() {
            if !line.trim().contains("Paragraph::new(") {
                continue;
            }
            let end = (line_num_0 + 4).min(lines.len());
            if lines[line_num_0..end].join(" ").contains(".wrap(") {
                continue;
            }
            let line_num = line_num_0 + 1;
            let key = format!("{}:{}", relative, line_num);
            if !allowlist.contains(key.as_str()) {
                violations.push(format!(
                    "  {}:{} — Paragraph::new() without .wrap(). Add .wrap(Wrap {{ trim: false }}) or allowlist with reason.",
                    relative, line_num
                ));
            }
        }
    }
    violations
}

#[test]
fn paragraph_lint_scans_nested_framework_sources() {
    let temp = tempfile::tempdir().unwrap();
    let nested = temp.path().join("framework");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        nested.join("bad_panel.rs"),
        "fn render() { Paragraph::new(variable_content); }\n",
    )
    .unwrap();

    let violations = paragraph_violations(temp.path(), &HashSet::new());

    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("framework/bad_panel.rs:1")),
        "{violations:?}"
    );
}

#[test]
fn ownership_source_check_rejects_non_code_evidence() {
    let comments_only = r#"
        // render_scrollable_list_panel(
        /* render_scrollable_list_panel( */
    "#;
    let strings_only = r###"
        const ORDINARY: &str = "render_scrollable_list_panel(";
        const RAW: &str = r#"render_scrollable_list_panel("#;
    "###;

    assert!(!source_code_contains(
        comments_only,
        "render_scrollable_list_panel("
    ));
    assert!(!source_code_contains(
        strings_only,
        "render_scrollable_list_panel("
    ));
    assert!(!source_code_contains("const MARKER: char = '(';", "("));
    assert!(source_code_contains(
        "render_scrollable_list_panel(frame, area);",
        "render_scrollable_list_panel("
    ));
    assert!(source_code_contains(
        "fn render<'a>(frame: &'a mut Frame) { render_scrollable_list_panel(frame, area); }",
        "render_scrollable_list_panel("
    ));
}

/// Scans all TUI source files and verifies that every `Paragraph::new(`
/// rendering user-entered or variable-length content uses `.wrap(`.
///
/// Fixed UI elements (titles, footers, shortcuts, single-line labels) are
/// allowlisted by file:line. If you add a new Paragraph without `.wrap()`,
/// either add `.wrap(Wrap { trim: false })` or add the line to the allowlist
/// with a comment explaining why wrapping is unnecessary.
#[test]
fn paragraph_widgets_must_wrap_user_content() {
    // Lines that are known-safe: fixed UI text that cannot overflow.
    // Format: "filename:line_number"
    // When adding entries, include a brief reason.
    let allowlist: HashSet<&str> = [
        "dashboard.rs:136",
        "dashboard.rs:410",
        "dashboard.rs:430",
        "dashboard.rs:465",
        "log_entry.rs:544",
        "log_entry.rs:736",
        "log_entry.rs:835",
    ]
    .into_iter()
    .collect();

    let tui_dir = Path::new("src/tui");
    let violations = paragraph_violations(tui_dir, &allowlist);

    // Also check that allowlist entries still correspond to actual Paragraph::new( lines
    // (catch stale entries after refactors)
    let mut stale = Vec::new();
    for &entry in &allowlist {
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        let filename = parts[0];
        let line_num: usize = parts[1].parse().unwrap();
        let path = tui_dir.join(filename);
        if !path.exists() {
            stale.push(format!("  {} — file does not exist", entry));
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            stale.push(format!(
                "  {} — line number out of range (file has {} lines)",
                entry,
                lines.len()
            ));
            continue;
        }
        if !lines[line_num - 1].contains("Paragraph::new(") {
            stale.push(format!(
                "  {} — no Paragraph::new( on this line (found: {:?}). Update or remove from allowlist.",
                entry,
                lines[line_num - 1].trim()
            ));
        }
    }

    let mut msg = String::new();
    if !violations.is_empty() {
        msg.push_str(&format!(
            "Paragraph widgets without .wrap() found (see CLAUDE.md constraint):\n{}\n",
            violations.join("\n")
        ));
    }
    if !stale.is_empty() {
        msg.push_str(&format!(
            "Stale allowlist entries (line numbers shifted after edits):\n{}\n",
            stale.join("\n")
        ));
    }
    assert!(msg.is_empty(), "\n{}", msg);
}

/// Ensures that main content screens use bordered blocks for visual consistency.
/// Every screen that displays a primary list or content area should wrap it in
/// a Block with Borders::ALL.
#[test]
fn screens_must_have_bordered_blocks() {
    let tui_dir = Path::new("src/tui");
    let required_screens = ["goals.rs", "trends.rs", "history.rs", "log_entry.rs"];

    let mut missing = Vec::new();
    for filename in &required_screens {
        let path = tui_dir.join(filename);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read {}", path.display()));
        if !source.contains(".borders(Borders::ALL)") {
            missing.push(format!("  {} — missing .borders(Borders::ALL)", filename));
        }
    }

    assert!(
        missing.is_empty(),
        "\nScreens missing bordered blocks for visual consistency:\n{}\n",
        missing.join("\n")
    );

    for (filename, screen_name) in [
        ("practices.rs", "Practices"),
        ("quotes_screen.rs", "Quotes"),
    ] {
        let source = std::fs::read_to_string(tui_dir.join(filename)).unwrap();
        assert!(
            source_code_contains(&source, "render_scrollable_list_panel("),
            "{screen_name} must delegate its bordered list to ScrollableListPanel"
        );
    }

    let list_panel_source =
        std::fs::read_to_string(tui_dir.join("framework/list_panel.rs")).unwrap();
    assert!(
        source_code_contains(&list_panel_source, ".borders(Borders::ALL)"),
        "ScrollableListPanel must own the delegated bordered-list contract"
    );
}
