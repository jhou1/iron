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
        // ── abbreviations.rs ──
        "abbreviations.rs:115",
        "abbreviations.rs:217",
        "abbreviations.rs:249",
        // ── dashboard.rs ──
        "dashboard.rs:142",
        "dashboard.rs:221",
        "dashboard.rs:372",
        "dashboard.rs:402", // gauge bar: width-controlled, cannot overflow
        // ── goals.rs ──
        "goals.rs:214",
        "goals.rs:390",
        "goals.rs:458",
        // ── history.rs ──
        "history.rs:123",
        "history.rs:185",
        "history.rs:217",
        // ── log_entry.rs ──
        "log_entry.rs:185",
        "log_entry.rs:218",
        "log_entry.rs:244",
        "log_entry.rs:263",
        "log_entry.rs:434",
        "log_entry.rs:534",
        "log_entry.rs:570",
        "log_entry.rs:617",
        // ── mod.rs ──
        "mod.rs:104",
        // ── practices.rs ──
        "practices.rs:168",
        "practices.rs:250",
        "practices.rs:296",
        // ── quotes_screen.rs ──
        "quotes_screen.rs:113",
        "quotes_screen.rs:186",
        "quotes_screen.rs:215",
        // ── quick_log.rs ──
        "quick_log.rs:122",
        "quick_log.rs:142",
        "quick_log.rs:202",
        "quick_log.rs:217",
        "quick_log.rs:231",
        // ── trends.rs ──
        "trends.rs:138",
        "trends.rs:178",
        "trends.rs:195",
        "trends.rs:365",
        "trends.rs:373",
        "trends.rs:419",
        "trends.rs:431",
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
