use std::collections::HashSet;
use std::path::Path;

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
        "dashboard.rs:152",
        "dashboard.rs:274",
        "dashboard.rs:461",
        "dashboard.rs:481",
        "dashboard.rs:516",
        "goals.rs:451",
        "goals.rs:572",
        "history.rs:133",
        "history.rs:231",
        "history.rs:277",
        "log_entry.rs:214",
        "log_entry.rs:264",
        "log_entry.rs:294",
        "log_entry.rs:493",
        "log_entry.rs:669",
        "log_entry.rs:715",
        "log_entry.rs:820",
        "mod.rs:108",
        "practices.rs:194",
        "practices.rs:293",
        "practices.rs:378",
        "quotes_screen.rs:122",
        "quotes_screen.rs:224",
        "quotes_screen.rs:271",
        "trends.rs:99",
        "trends.rs:135",
        "trends.rs:184",
        "trends.rs:249",
        "trends.rs:257",
        "trends.rs:317",
    ].into_iter().collect();

    let tui_dir = Path::new("src/tui");
    let mut violations = Vec::new();

    for entry in std::fs::read_dir(tui_dir).expect("failed to read src/tui/") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e != "rs").unwrap_or(true) {
            continue;
        }
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        let source = std::fs::read_to_string(&path).unwrap();

        for (line_num_0, line) in source.lines().enumerate() {
            let line_num = line_num_0 + 1; // 1-based to match editors/grep (file is 0-indexed in read)
            let trimmed = line.trim();

            if !trimmed.contains("Paragraph::new(") {
                continue;
            }
            // Check if this line or the surrounding context has .wrap(
            // We check the line itself and up to 3 lines after it (for chained calls)
            let lines_vec: Vec<&str> = source.lines().collect();
            let end = (line_num_0 + 4).min(lines_vec.len());
            let context: String = lines_vec[line_num_0..end].join(" ");

            if context.contains(".wrap(") {
                continue;
            }

            let key = format!("{}:{}", filename, line_num);
            if allowlist.contains(key.as_str()) {
                continue;
            }

            violations.push(format!(
                "  {}:{} — Paragraph::new() without .wrap(). Add .wrap(Wrap {{ trim: false }}) or allowlist with reason.",
                filename, line_num
            ));
        }
    }

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
            stale.push(format!("  {} — line number out of range (file has {} lines)", entry, lines.len()));
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
    let required_screens = [
        "goals.rs",
        "trends.rs",
        "history.rs",
        "practices.rs",
        "quotes_screen.rs",
        "quick_log.rs",
        "log_entry.rs",
    ];

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
}
