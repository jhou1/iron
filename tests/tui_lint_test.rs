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
        "abbreviations.rs:116", // title + column headers (fixed text)
        "abbreviations.rs:218", // action area: short input lines with visible_input_spans
        "abbreviations.rs:252", // shortcuts bar (fixed text)
        // ── dashboard.rs ──
        "dashboard.rs:179",     // logo (fixed text)
        "dashboard.rs:192",     // view label (single-line fixed UI element showing current view)
        "dashboard.rs:221",     // quote line (already has .wrap on next line)
        "dashboard.rs:257",     // HRV single value line
        "dashboard.rs:325",     // footer shortcuts bar
        "dashboard.rs:457",     // "press [a] to add" hint (fixed text)
        "dashboard.rs:557",     // quotes edit input line (single line, bounded by modal)
        "dashboard.rs:566",     // quotes delete confirm (fixed text)
        "dashboard.rs:580",     // quotes modal shortcuts bar
        // ── goals.rs ──
        "goals.rs:221",         // title (fixed text)
        "goals.rs:312",         // goals list (already has .wrap on next line)
        "goals.rs:370",         // footer (fixed text)
        "goals.rs:464",         // gauge line (fixed width progress bar)
        "goals.rs:471",         // "no milestones" hint (fixed text)
        "goals.rs:540",         // milestone list (already has .scroll, bounded modal)
        "goals.rs:584",         // modal footer (fixed text)
        // ── history.rs ──
        "history.rs:141",       // title (fixed text)
        "history.rs:176",       // shortcuts bar (fixed text)
        "history.rs:212",       // empty state message (fixed text)
        "history.rs:257",       // entry list (columnar layout with computed widths)
        // ── log_entry.rs ──
        "log_entry.rs:191",     // title (fixed text)
        "log_entry.rs:226",     // filter input (short, bounded)
        "log_entry.rs:253",     // practice list (columnar layout)
        "log_entry.rs:274",     // footer (fixed text)
        "log_entry.rs:447",     // title (fixed text)
        "log_entry.rs:475",     // date line (fixed format YYYY-MM-DD)
        "log_entry.rs:555",     // sets display (formatted numbers)
        "log_entry.rs:587",     // total line (formatted number)
        "log_entry.rs:626",     // footer (fixed text)
        "log_entry.rs:926",     // warmup/cooldown title (fixed text)
        "log_entry.rs:939",     // warmup input (uses visible_input_spans)
        "log_entry.rs:952",     // cooldown input (uses visible_input_spans)
        "log_entry.rs:964",     // footer (fixed text)
        "log_entry.rs:1074",    // note title (fixed text)
        "log_entry.rs:1103",    // summary lines (formatted, bounded)
        "log_entry.rs:1115",    // note input (uses visible_input_spans inside Block)
        "log_entry.rs:1129",    // footer (fixed text)
        // ── mod.rs ──
        "mod.rs:96",            // status line (single line)
        "mod.rs:138",           // help overlay (bounded by overlay box)
        // ── practices.rs ──
        "practices.rs:120",     // title + column headers (fixed text)
        "practices.rs:164",     // practice list (columnar layout, names are short)
        "practices.rs:246",     // action area: input with visible_input_spans
        "practices.rs:294",     // shortcuts bar (fixed text)
        // ── quick_log.rs ──
        "quick_log.rs:124",     // title (fixed text)
        "quick_log.rs:140",     // shortcuts bar (fixed text)
        "quick_log.rs:225",     // "no config" message (fixed text, inside bordered block)
        "quick_log.rs:240",     // spinner message (fixed text)
        "quick_log.rs:254",     // "no results" message (fixed text)
        // ── trends.rs ──
        "trends.rs:131",        // title (fixed text)
        "trends.rs:156",        // filter input (short, bounded)
        "trends.rs:180",        // practice list (names are short identifiers)
        "trends.rs:198",        // footer (fixed text)
        "trends.rs:374",        // chart title (fixed text)
        "trends.rs:381",        // subtitle (fixed text)
        "trends.rs:389",        // "no data" message (fixed text)
        "trends.rs:436",        // stats line (formatted numbers)
        "trends.rs:448",        // footer (fixed text)
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
