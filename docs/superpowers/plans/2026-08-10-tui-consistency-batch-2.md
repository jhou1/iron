# TUI Consistency Framework — Batch 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permanently remove Quick Log and its abbreviation/LLM stack, then migrate Log Entry and Dashboard to the shared TUI interaction framework.

**Architecture:** Database initialization performs a deliberate one-way removal of the `abbreviations` table, while application routing and dependency cleanup remove every live Quick Log path. Log Entry and Dashboard retain their domain workflows but replace local cursors, scrolling, status, and shortcut layout with Batch 1 framework primitives.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Fluent localization, existing `src/tui/framework` primitives, Cargo test/clippy/build gates.

## Global Constraints

- Batch 2 changes Log Entry and Dashboard behavior only; changes elsewhere are limited to Quick Log/LLM/Abbreviations removal, routing, shared framework extensions, tests, localization, and documentation.
- Quick Log is deleted rather than migrated or replaced.
- Existing abbreviation rows are permanently deleted; no backup or export is created.
- `Database::init_schema` must fail if `DROP TABLE IF EXISTS abbreviations` fails.
- Training logs, practices, goals, milestones, quotes, daily metrics, import, and export data remain unchanged.
- Log Entry keeps the existing new-log and History-edit workflows.
- Log Entry remains supported at `80x24`; Dashboard primary acceptance size is `80x35`.
- Direct rendering into smaller rectangles must not panic.
- Modes and input clear only after validation and required database mutation both succeed.
- Every changed behavior follows red-green-refactor and each task ends in an independently reviewable commit.
- Do not begin a later task while a targeted test for the current task is failing.

---

### Task 1: Drop the Abbreviations Schema and Remove Backend APIs

**Files:**
- Modify: `src/db.rs`
- Modify: `src/model.rs`
- Modify: `tests/db_test.rs`

**Interfaces:**
- Consumes: `Database::open(path: &Path) -> anyhow::Result<Database>` and existing schema initialization.
- Produces: database initialization with no `abbreviations` table and no abbreviation/parsed-LLM model or CRUD API.

- [ ] **Step 1: Add the destructive migration regression test**

Add imports for `rusqlite::Connection` and `tempfile::tempdir` to `tests/db_test.rs` if they are not already present, then add:

```rust
#[test]
fn opening_database_drops_abbreviations_and_preserves_training_data() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let legacy = Connection::open(&path).unwrap();
    legacy
        .execute_batch(
            "CREATE TABLE abbreviations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                short TEXT NOT NULL UNIQUE COLLATE NOCASE,
                full_name TEXT NOT NULL
             );
             INSERT INTO abbreviations (short, full_name) VALUES ('DL', 'Deadlift');",
        )
        .unwrap();
    drop(legacy);

    let db = Database::open(&path).unwrap();
    db.create_practice("Deadlift", PracticeType::Weighted)
        .unwrap();
    drop(db);

    let raw = Connection::open(&path).unwrap();
    let abbreviation_tables: i64 = raw
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='abbreviations'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let practices: i64 = raw
        .query_row("SELECT count(*) FROM practices", [], |row| row.get(0))
        .unwrap();

    assert_eq!(abbreviation_tables, 0);
    assert_eq!(practices, 1);
}
```

- [ ] **Step 2: Run the test and verify the legacy table survives before the fix**

Run:

```bash
cargo test opening_database_drops_abbreviations_and_preserves_training_data -- --nocapture
```

Expected: FAIL because `init_schema` currently recreates or retains `abbreviations`, so `abbreviation_tables` is `1`.

- [ ] **Step 3: Implement the one-way schema removal**

In `Database::init_schema`:

1. Remove the `CREATE TABLE IF NOT EXISTS abbreviations (...)` statement from the schema batch.
2. Add this checked operation after current tables are created and before reconciliation returns:

```rust
self.conn.execute("DROP TABLE IF EXISTS abbreviations", [])?;
```

Do not write `let _ = ...`; failure must propagate.

- [ ] **Step 4: Remove backend-only Quick Log types and APIs**

Apply these exact removals:

- Remove `Abbreviation` from the `crate::model` import in `src/db.rs`.
- Remove `create_abbreviation`, `list_abbreviations`, `update_abbreviation`, and `delete_abbreviation` from `Database`.
- Remove `Abbreviation`, `ParsedLog`, `RawParsedLog`, and `RawSet` from `src/model.rs`.
- Remove the four abbreviation CRUD/ordering/uniqueness tests from `tests/db_test.rs`.
- Keep `Deserialize` in `src/model.rs`; other persisted models still use it.

- [ ] **Step 5: Run backend and model tests**

Run:

```bash
cargo test --test db_test -- --nocapture
cargo test --test model_test -- --nocapture
```

Expected: all remaining tests pass, including the new destructive migration test.

- [ ] **Step 6: Confirm no backend abbreviation API remains**

Run:

```bash
rg -n "Abbreviation|ParsedLog|RawParsedLog|RawSet|create_abbreviation|list_abbreviations|update_abbreviation|delete_abbreviation|CREATE TABLE IF NOT EXISTS abbreviations" src/db.rs src/model.rs tests/db_test.rs
```

Expected: no matches. Separately confirm `DROP TABLE IF EXISTS abbreviations` appears exactly once in `src/db.rs`.

- [ ] **Step 7: Commit the backend removal**

```bash
git add src/db.rs src/model.rs tests/db_test.rs
git commit -m "refactor(db): remove Quick Log abbreviation data"
```

---

### Task 2: Remove Quick Log Runtime, LLM Configuration, and Dependencies

**Files:**
- Create: `tests/removed_quick_log_test.rs`
- Delete: `src/tui/quick_log.rs`
- Delete: `src/llm.rs`
- Delete: `src/config.rs`
- Delete: `tests/llm_test.rs`
- Delete: `tests/config_test.rs`
- Modify: `src/app.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/dashboard.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `tests/tui_lint_test.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock` through Cargo

**Interfaces:**
- Consumes: the backend removal from Task 1.
- Produces: an application with no Quick Log route, LLM client, LLM configuration loader, background polling, or direct `ureq`/`toml` dependency.

- [ ] **Step 1: Add a live-source absence test and observe RED**

Create `tests/removed_quick_log_test.rs`:

```rust
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
        assert!(!Path::new(removed_file).exists(), "{removed_file} still exists");
    }
}
```

Run:

```bash
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: FAIL on the first live Quick Log marker.

- [ ] **Step 2: Remove modules and route variants**

Use `apply_patch` deletions for the three source modules and two dedicated integration tests. Then make these routing changes:

```rust
// src/tui/mod.rs
pub enum Screen {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
    Goals,
    Quotes,
}
```

- Remove `pub mod quick_log`.
- Remove `mod config`, `mod llm`, `pub mod config`, and `pub mod llm` from binary/library roots.
- Remove `QuickLogScreen`, `Config`, and `Duration` imports from `src/app.rs`.
- Remove Quick Log construction and every `Screen::QuickLog` match arm.
- Remove Dashboard `KeyCode::Char('w')` navigation and the rendered `[w]` command. Leave the remaining Dashboard footer structure intact until Task 8.

- [ ] **Step 3: Restore a blocking application event loop**

Replace the poll/background-result branch with the existing key-event flow driven directly by:

```rust
if let Event::Key(key) = event::read()? {
    if key.kind != event::KeyEventKind::Press {
        continue;
    }
    // existing Ctrl+C, routing, and Action handling
}
```

No timer, receiver, background result, or periodic redraw path remains.

- [ ] **Step 4: Remove dependencies and regenerate the lockfile**

Remove these direct dependency lines from `Cargo.toml`:

```toml
ureq = { version = "3", features = ["json"] }
toml = "0.8"
```

Run:

```bash
cargo check
```

Expected: compilation succeeds and Cargo updates `Cargo.lock` so `ureq` and its now-unreachable transitive packages disappear. `dirs`, `serde`, and `serde_json` remain.

- [ ] **Step 5: Remove Quick Log from lint inventories**

Delete `"quick_log.rs"` from `required_screens` and remove any Quick Log line-number allowlist entries from `tests/tui_lint_test.rs`.

- [ ] **Step 6: Run the removal and application tests**

Run:

```bash
cargo test --test removed_quick_log_test -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test tui::dashboard::tests -- --nocapture
cargo check
```

Expected: all pass with no compile warnings.

- [ ] **Step 7: Commit the runtime removal**

```bash
git add Cargo.toml Cargo.lock src/app.rs src/lib.rs src/main.rs src/tui/mod.rs src/tui/dashboard.rs tests/tui_lint_test.rs tests/removed_quick_log_test.rs
git add -u src/config.rs src/llm.rs src/tui/quick_log.rs tests/config_test.rs tests/llm_test.rs
git commit -m "refactor: remove Quick Log runtime"
```

---

### Task 3: Remove Quick Log and Abbreviations Documentation and Localization

**Files:**
- Delete: `docs/specs/quick-log-spec.md`
- Delete: `docs/specs/abbreviations-spec.md`
- Delete: `docs/superpowers/specs/2026-05-12-quick-log-design.md`
- Delete: `docs/superpowers/plans/2026-05-12-quick-log-implementation.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `CLAUDE.md`
- Modify: `docs/specs/dashboard-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/specs/data-model-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`
- Modify: `tests/removed_quick_log_test.rs`

**Interfaces:**
- Consumes: the removed runtime from Task 2.
- Produces: active documentation and locale catalogs that advertise only shipped behavior.

- [ ] **Step 1: Extend the absence test to active docs and locales**

Add these paths to the source loop in `tests/removed_quick_log_test.rs`:

```rust
"README.md",
"README_zh.md",
"CLAUDE.md",
"docs/specs/dashboard-spec.md",
"docs/specs/navigation-spec.md",
"docs/specs/data-model-spec.md",
"docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md",
"locales/en.ftl",
"locales/zh-CN.ftl",
```

Add `"Quick Log"`, `"快捷记录"`, `"Abbreviations"`, `"quicklog-"`, `"key-quick-log"`, and `"abbreviations-"` to the removed marker list. Add the four dedicated documentation paths to the `removed_file` list.

Run:

```bash
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: FAIL because the active docs/locales and dedicated documents still contain removed behavior.

- [ ] **Step 2: Delete dedicated feature documents**

Delete the two active specs and the two historical documents dedicated entirely to Quick Log. Do not delete the approved Batch 2 design or this plan; they are the audit record authorizing removal.

- [ ] **Step 3: Remove active documentation claims**

Make these explicit edits:

- Remove Quick Log from the feature lists and Dashboard shortcut tables in both READMEs.
- Remove the Quick Log usage sections from both READMEs.
- Remove Quick Log/Abbreviations modes and keys from `dashboard-spec.md`.
- Remove Quick Log from the navigation tree and framework batch descriptions.
- Remove the Abbreviation data model section.
- Update the original consistency framework design with a short supersession note linking to `2026-08-10-tui-consistency-batch-2-design.md`; remove its live Batch 2 Quick Log acceptance requirements.
- Remove any Quick Log development command or module guidance from `CLAUDE.md`.

- [ ] **Step 4: Remove locale keys symmetrically**

Delete these groups from both locale files:

- `key-quick-log`
- every `quicklog-*` key
- every `abbreviations-*` key

Do not renumber or reword unrelated localization keys.

- [ ] **Step 5: Run absence and localization parity tests**

Run:

```bash
cargo test --test removed_quick_log_test -- --nocapture
cargo test --test i18n_test -- --nocapture
```

Expected: both pass.

- [ ] **Step 6: Audit remaining historical mentions**

Run:

```bash
rg -n -i -g '!removed_quick_log_test.rs' "quick.?log|quicklog|abbreviation|LLM config|ureq" README.md README_zh.md CLAUDE.md src tests locales docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
```

Expected: no live feature matches. Matches in the approved Batch 2 design/plan are allowed because they document the removal decision.

- [ ] **Step 7: Commit the documentation removal**

```bash
git add README.md README_zh.md CLAUDE.md docs/specs/dashboard-spec.md docs/specs/navigation-spec.md docs/specs/data-model-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md locales/en.ftl locales/zh-CN.ftl tests/removed_quick_log_test.rs
git add -u docs/specs/quick-log-spec.md docs/specs/abbreviations-spec.md docs/superpowers/specs/2026-05-12-quick-log-design.md docs/superpowers/plans/2026-05-12-quick-log-implementation.md
git commit -m "docs: remove Quick Log and abbreviations"
```

---

### Task 4: Add Filtered Text Input and Migrate Log Entry Practice Selection

**Files:**
- Modify: `src/tui/framework/input.rs`
- Modify: `src/tui/log_entry.rs`

**Interfaces:**
- Consumes: `TextInput`, `ViewportState`, `VisualSpan`, `move_selection`, `EdgeBehavior`, `Shortcut`, `shortcut_height`, and `render_shortcuts`.
- Produces: `TextInput::handle_key_if<F>(&mut self, key: KeyEvent, accepts: F) -> bool where F: Fn(char) -> bool`, plus a shared-framework practice selector.

- [ ] **Step 1: Add a failing filtered-input unit test**

In `src/tui/framework/input.rs` add:

```rust
#[test]
fn filtered_input_rejects_invalid_characters_without_blocking_navigation() {
    let mut input = TextInput::from("12");
    input.handle_key_if(key(KeyCode::Char('x')), |character| character.is_ascii_digit());
    input.handle_key_if(key(KeyCode::Left), |character| character.is_ascii_digit());
    input.handle_key_if(key(KeyCode::Backspace), |character| character.is_ascii_digit());

    assert_eq!(input.value(), "2");
    assert_eq!(input.cursor(), 0);
}
```

Run:

```bash
cargo test filtered_input_rejects_invalid_characters_without_blocking_navigation -- --nocapture
```

Expected: compile FAIL because `handle_key_if` does not exist.

- [ ] **Step 2: Implement `TextInput::handle_key_if`**

Add:

```rust
pub fn handle_key_if<F>(&mut self, key: KeyEvent, accepts: F) -> bool
where
    F: Fn(char) -> bool,
{
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    if let KeyCode::Char(character) = key.code {
        if !control && !accepts(character) {
            return false;
        }
    }
    self.handle_key(key)
}
```

Run the new framework test and all framework tests.

- [ ] **Step 3: Add Log Entry selector regression tests before migration**

Inside `src/tui/log_entry.rs` tests, add a `render_log_entry` helper using `framework::test_support::render_to_string`, then add:

```rust
#[test]
fn long_filter_and_last_practice_remain_visible() {
    let db = Database::open_in_memory().unwrap();
    for number in 1..=40 {
        db.create_practice(&format!("Practice {number:02}"), PracticeType::Weighted)
            .unwrap();
    }
    let mut screen = LogEntryScreen::new(&db).unwrap();
    screen.filtering = true;
    screen.filter_text = "练习".repeat(60);
    screen.filter_cursor = screen.filter_text.len();
    let filtered = render_log_entry(&mut screen, 80, 24);
    assert!(filtered.contains('_'), "{filtered}");

    screen.filter_text.clear();
    screen.filter_cursor = 0;
    screen.apply_filter();
    screen.filtering = false;
    for _ in 1..40 {
        screen.handle_key(key(KeyCode::Down), &db);
    }
    let last = render_log_entry(&mut screen, 80, 24);
    assert!(last.contains("Practice 40"), "{last}");
    assert!(last.contains('↑'), "{last}");
}
```

Run this exact current-state version once as the characterization baseline. During Step 4 replace the two `filter_text`/`filter_cursor` setup blocks with `screen.filter = TextInput::from("练习".repeat(60));` and `screen.filter.clear();` respectively.

- [ ] **Step 4: Replace selector-local state and helpers**

Apply this state mapping in `LogEntryScreen` and both constructors:

```rust
filter_text: String + filter_cursor: usize -> filter: TextInput
scroll: usize + list_height: usize -> list_viewport: ViewportState
```

Then:

- Render the active filter with `filter.visible_spans(filter_area.width, 2, ACCENT)`.
- Use `filter.handle_key` in filtering mode.
- Use `move_selection(..., EdgeBehavior::Clamp)` for `j/k` and arrow navigation.
- Set the real list viewport height after calculating the bordered inner area and header row.
- Reveal `VisualSpan { start: selected, height: 1 }` and clamp to filtered item count.
- Append the viewport overflow suffix to the practice-list title.
- Truncate practice name/type columns to the real inner width.
- Replace the selector footer with `active_select_shortcuts`, `shortcut_height`, and `render_shortcuts`.
- Remove local `adjust_scroll` and the selector’s manual cursor-editing branches.

- [ ] **Step 5: Run selector and existing Log Entry tests**

```bash
cargo test long_filter_and_last_practice_remain_visible -- --nocapture
cargo test tui::log_entry::tests -- --nocapture
cargo test tui::framework -- --nocapture
```

Expected: all pass.

- [ ] **Step 6: Confirm selector duplicate code is gone**

```bash
rg -n "filter_cursor|filter_text|fn adjust_scroll|list_height|fn handle_text_input" src/tui/log_entry.rs
```

Expected: no matches.

- [ ] **Step 7: Commit selector migration**

```bash
git add src/tui/framework/input.rs src/tui/log_entry.rs
git commit -m "refactor(log-entry): migrate practice selection interactions"
```

---

### Task 5: Migrate Every Log Entry Field to `TextInput`

**Files:**
- Modify: `src/tui/log_entry.rs`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: `TextInput::value`, `set`, `clear`, `cursor`, `visible_spans`, `handle_key`, and `handle_key_if`.
- Produces: a Log Entry form with no local string/cursor pair.

- [ ] **Step 1: Add failing long-field and invalid-date tests**

Add tests that construct an `EnterLog` screen with a weighted practice and assert bounded cursor rendering for every field:

```rust
#[test]
fn every_long_log_input_keeps_its_cursor_visible() {
    let db = Database::open_in_memory().unwrap();
    let practice = db
        .create_practice("Deadlift", PracticeType::Weighted)
        .unwrap();
    let mut screen = LogEntryScreen::new(&db).unwrap();
    screen.phase = Phase::EnterLog;
    screen.chosen_practice = Some(practice);
    let long = "练习".repeat(80);
    for focus in [
        FocusSection::Sets,
        FocusSection::WarmUp,
        FocusSection::CoolDown,
        FocusSection::Rpe,
        FocusSection::Note,
    ] {
        screen.focus = focus;
        match focus {
            FocusSection::Sets => screen.field1.set(&long),
            FocusSection::WarmUp => screen.warm_up.set(&long),
            FocusSection::CoolDown => screen.cool_down.set(&long),
            FocusSection::Rpe => screen.rpe_input.set("99"),
            FocusSection::Note => screen.note.set(&long),
        }
        let output = render_log_entry(&mut screen, 80, 24);
        assert!(output.contains('_'), "focus={focus:?}\n{output}");
    }
}

#[test]
fn invalid_date_keeps_date_editor_and_input() {
    let db = Database::open_in_memory().unwrap();
    let practice = db
        .create_practice("Deadlift", PracticeType::Weighted)
        .unwrap();
    let mut screen = LogEntryScreen::new(&db).unwrap();
    screen.phase = Phase::EnterLog;
    screen.chosen_practice = Some(practice);
    screen.editing_date = true;
    screen.date_input = TextInput::from("2026-99-99");

    screen.handle_key(key(KeyCode::Enter), &db);

    assert!(screen.editing_date);
    assert_eq!(screen.date_input.value(), "2026-99-99");
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
}
```

Run the tests before production migration. Expected: compile FAIL until the fields become `TextInput`; the existing invalid-date behavior would also close the editor.

- [ ] **Step 2: Replace all remaining string/cursor pairs**

Apply this exact mapping:

```text
field1 + field1_cursor             -> field1: TextInput
field2 + field2_cursor             -> field2: TextInput
warm_up + warm_up_cursor           -> warm_up: TextInput
cool_down + cool_down_cursor       -> cool_down: TextInput
rpe_input + rpe_cursor             -> rpe_input: TextInput
note + note_cursor                 -> note: TextInput
date_input + date_input_cursor     -> date_input: TextInput
```

Construct prefilled edit values with `TextInput::from`; construct empty values with `TextInput::new`.

- [ ] **Step 3: Migrate rendering to bounded shared spans**

For every active field, build its prefix and call:

```rust
spans.extend(input.visible_spans(area.width, prefix_width, ACCENT));
```

For inactive fields, render `input.value()` inside a wrapped or width-bounded paragraph. Remove every direct `split_at(cursor)` and the compatibility `visible_input_spans` import from Log Entry.

- [ ] **Step 4: Migrate field key handling without changing allowed characters**

- Set fields accept only ASCII digits and `.` through `handle_key_if`.
- Date accepts only ASCII digits and `-`.
- RPE accepts only ASCII digits and at most two characters.
- Warm-up, cool-down, and note call ordinary `handle_key`.
- Preserve `Tab`, `Enter`, set selection/edit/delete, `Ctrl+S`, and exit-confirmation branches before forwarding a key to the active input.
- Replace direct string mutation with `set`, `clear`, `value`, and `trimmed`.
- Update `commit_set` and `save_log` parsing to read `input.value()`.

- [ ] **Step 5: Keep invalid date editing open**

Use this transition:

```rust
match NaiveDate::parse_from_str(self.date_input.value(), "%Y-%m-%d") {
    Ok(_) => {
        self.log_date = self.date_input.value().to_owned();
        self.date_input.clear();
        self.editing_date = false;
    }
    Err(_) => {
        self.status_msg = Some((tr("log-invalid-date"), true));
    }
}
```

Add these exact locale entries:

```ftl
# locales/en.ftl
log-invalid-date = Invalid date; use YYYY-MM-DD

# locales/zh-CN.ftl
log-invalid-date = 日期无效，请使用 YYYY-MM-DD
```

- [ ] **Step 6: Update existing cursor tests to shared accessors**

Replace assertions such as:

```rust
assert_eq!(screen.field2_cursor, 2);
```

with:

```rust
assert_eq!(screen.field2.cursor(), 2);
```

Keep the existing sets navigation/edit tests; they remain behavior contracts.

- [ ] **Step 7: Run Log Entry tests and duplicate-state audit**

```bash
cargo test tui::log_entry::tests -- --nocapture
rg -n "field1_cursor|field2_cursor|warm_up_cursor|cool_down_cursor|rpe_cursor|note_cursor|date_input_cursor|visible_input_spans|split_at\(" src/tui/log_entry.rs
```

Expected: tests pass and the search has no matches.

- [ ] **Step 8: Commit field migration**

```bash
git add src/tui/log_entry.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "refactor(log-entry): use shared text input state"
```

---

### Task 6: Finish Log Entry Status, Shortcut, Error, and Small-Rectangle Behavior

**Files:**
- Modify: `src/tui/log_entry.rs`

**Interfaces:**
- Consumes: shared status/shortcut components and the fully migrated form from Task 5.
- Produces: complete active-mode commands, stable failure context, and panic-free direct rendering.

- [ ] **Step 1: Add failure-context and layout regression tests**

Add these tests:

```rust
#[test]
fn invalid_rpe_retains_complete_form_and_focus() {
    let db = Database::open_in_memory().unwrap();
    let practice = db
        .create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    let mut screen = LogEntryScreen::new(&db).unwrap();
    screen.phase = Phase::EnterLog;
    screen.chosen_practice = Some(practice);
    screen.sets.push(SetData::Bodyweight { reps: 10 });
    screen.note.set("keep me");
    screen.rpe_input.set("99");
    screen.focus = FocusSection::Rpe;

    let action = screen.save_log(&db);

    assert!(matches!(action, Action::None));
    assert_eq!(screen.note.value(), "keep me");
    assert_eq!(screen.rpe_input.value(), "99");
    assert_eq!(screen.focus, FocusSection::Rpe);
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
}

#[test]
fn log_entry_renders_small_rectangle_without_panicking() {
    let db = Database::open_in_memory().unwrap();
    let mut screen = LogEntryScreen::new(&db).unwrap();
    let output = render_log_entry(&mut screen, 24, 10);
    assert!(!output.is_empty());
}

#[test]
fn log_entry_shortcuts_include_every_active_command_at_supported_width() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    let mut screen = LogEntryScreen::new(&db).unwrap();
    let output = render_log_entry(&mut screen, 80, 24);
    for command in ["[j/k]", "[/]", "[Enter]", "[Esc]"] {
        assert!(output.contains(command), "missing {command}\n{output}");
    }
}
```

Run them before the layout change. Expected: the small-render or complete-shortcut test fails if the current fixed layout clips content; invalid RPE must remain a passing characterization.

- [ ] **Step 2: Centralize active shortcuts**

Add domain methods returning `Vec<Shortcut>` for:

- practice selection
- normal set entry
- selected-set editing/navigation
- date editing
- exit confirmation

Compute `shortcut_height` before each phase layout split, reserve that height, and render with `render_shortcuts`. Confirmation modes show only confirmation/cancel commands.

- [ ] **Step 3: Reserve status and clamp all derived rectangles**

- Keep a dedicated one-row status constraint in both phases.
- Use saturating arithmetic for set-table, notes, date editor, and footer areas.
- Do not create any `Rect` whose right or bottom edge exceeds its parent; use `fit_rect` for overlays.
- Wrap or truncate variable content inside real inner widths.

- [ ] **Step 4: Verify database write failure retains the form**

Add a file-database test that opens a second raw Rusqlite connection and executes `BEGIN EXCLUSIVE` before `save_log`. Assert `Action::None`, unchanged note/sets/editing identity/return target, and an error status. Roll back the raw lock at test end.

Run:

```bash
cargo test tui::log_entry::tests -- --nocapture
```

Expected: all Log Entry tests pass.

- [ ] **Step 5: Run lint and framework regressions**

```bash
cargo test --test tui_lint_test -- --nocapture
cargo test tui::framework -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass with zero warnings.

- [ ] **Step 6: Commit Log Entry completion**

```bash
git add src/tui/log_entry.rs tests/tui_lint_test.rs
git commit -m "feat(log-entry): complete shared interaction migration"
```

---

### Task 7: Migrate Dashboard HRV Input and Failure Semantics

**Files:**
- Modify: `src/tui/dashboard.rs`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: `TextInput` and reserved `StatusMessage` rendering.
- Produces: Dashboard HRV editing that retains mode/input on validation or database failure.

- [ ] **Step 1: Add HRV state-transition tests**

Add a `key` helper and these Dashboard tests:

```rust
#[test]
fn invalid_hrv_retains_input_mode_text_and_error() {
    let db = Database::open_in_memory().unwrap();
    let mut screen = DashboardScreen::new(&db, false).unwrap();
    screen.handle_key(key(KeyCode::Char('v')), &db);
    screen.hrv_input.set("101");

    screen.handle_key(key(KeyCode::Enter), &db);

    assert_eq!(screen.mode, DashboardMode::HrvInput);
    assert_eq!(screen.hrv_input.value(), "101");
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
}

#[test]
fn long_hrv_input_keeps_cursor_visible() {
    let db = Database::open_in_memory().unwrap();
    let mut screen = DashboardScreen::new(&db, false).unwrap();
    screen.mode = DashboardMode::HrvInput;
    screen.hrv_input = TextInput::from("9".repeat(100));

    let output = render_dashboard(&screen, 80, 35);

    assert!(output.contains('_'), "{output}");
}
```

Before migration, express setup through the current `String` where necessary, run once, and observe the invalid-input test FAIL because the current code silently leaves no error and may close after a database failure.

- [ ] **Step 2: Replace Dashboard HRV state with `TextInput`**

Change:

```rust
hrv_input: String
```

to:

```rust
hrv_input: TextInput
```

Initialize with `TextInput::new`, preload with `set`, render through `visible_spans`, and accept only ASCII digits through `handle_key_if`.

- [ ] **Step 3: Implement strict success-only exit semantics**

Use this control flow:

```rust
let parsed = self.hrv_input.value().parse::<i32>();
match parsed {
    Ok(hrv) if (0..=100).contains(&hrv) => {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        match db.set_daily_hrv(&today, hrv) {
            Ok(()) => {
                self.hrv_today = Some(hrv);
                self.hrv_input.clear();
                self.mode = DashboardMode::Normal;
            }
            Err(error) => {
                self.status_msg = Some((
                    tr_args("status-save-error", &[("msg", FluentValue::from(error.to_string()))]),
                    true,
                ));
            }
        }
    }
    _ => self.status_msg = Some((tr("dashboard-hrv-invalid"), true)),
}
```

Add these exact locale entries:

```ftl
# locales/en.ftl
dashboard-hrv-invalid = HRV must be a number from 0 to 100

# locales/zh-CN.ftl
dashboard-hrv-invalid = HRV 必须是 0 到 100 之间的数字
```

- [ ] **Step 4: Add and pass a locked-database failure test**

Use a tempfile database and a second `rusqlite::Connection` with `BEGIN EXCLUSIVE`; submit a valid HRV value and assert mode/input remain with an error. Roll back the lock explicitly.

- [ ] **Step 5: Run Dashboard and localization tests**

```bash
cargo test tui::dashboard::tests -- --nocapture
cargo test --test i18n_test -- --nocapture
```

Expected: all pass.

- [ ] **Step 6: Commit HRV migration**

```bash
git add src/tui/dashboard.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "refactor(dashboard): migrate HRV input state"
```

---

### Task 8: Make Dashboard Layout and Shortcuts Responsive

**Files:**
- Modify: `src/tui/dashboard.rs`
- Modify: `tests/tui_lint_test.rs`

**Interfaces:**
- Consumes: `Shortcut`, `shortcut_height`, `render_shortcuts`, `render_status_line`, `truncate_to_width`, and Ratatui layout constraints.
- Produces: a bounded Dashboard at `80x35`, panic-free smaller direct renders, and no stale Quick Log command.

- [ ] **Step 1: Replace the layout-only test with rendered behavior tests**

Add a `render_dashboard` helper using `render_to_string` and these tests:

```rust
#[test]
fn dashboard_renders_complete_remaining_navigation_at_80_by_35() {
    let db = Database::open_in_memory().unwrap();
    let screen = DashboardScreen::new(&db, false).unwrap();
    let output = render_dashboard(&screen, 80, 35);

    for command in ["[l]", "[h]", "[t]", "[e]", "[g]", "[Q]", "[v]", "[Esc]"] {
        assert!(output.contains(command), "missing {command}\n{output}");
    }
    assert!(!output.contains("[w]"), "{output}");
}

#[test]
fn dashboard_renders_small_rectangle_without_panicking() {
    let db = Database::open_in_memory().unwrap();
    let screen = DashboardScreen::new(&db, false).unwrap();
    let output = render_dashboard(&screen, 40, 16);
    assert!(!output.is_empty());
}
```

Also add a fixture containing a long practice name, long goal title, and long quote, then render at `80x35` and assert Dashboard sections plus the footer remain present.

Run before implementation. Expected: at least the complete navigation or long-content test fails with the fixed footer/layout.

- [ ] **Step 2: Centralize active Dashboard shortcuts**

Add:

```rust
fn active_shortcuts(&self) -> Vec<Shortcut> {
    match self.mode {
        DashboardMode::Normal => vec![
            Shortcut::new("l", tr("key-log")),
            Shortcut::new("h", tr("key-history")),
            Shortcut::new("t", tr("key-trends")),
            Shortcut::new("e", tr("key-practices")),
            Shortcut::new("g", tr("key-goals")),
            Shortcut::new("Q", tr("key-quotes")),
            Shortcut::new("v", tr("key-hrv")),
            Shortcut::new("Esc", tr("key-quit")),
        ],
        DashboardMode::ConfirmQuit => vec![
            Shortcut::new("y", tr("key-yes")),
            Shortcut::new("any", tr("key-cancel")),
        ],
        DashboardMode::HrvInput => vec![
            Shortcut::new("Enter", tr("key-confirm")),
            Shortcut::new("Esc", tr("key-cancel")),
        ],
    }
}
```

Use the existing `key-hrv` locale. Remove the hand-built grouped footer.

- [ ] **Step 3: Reserve dynamic footer height before content layout**

At render start:

```rust
let shortcuts = self.active_shortcuts();
let shortcut_rows = shortcut_height(&shortcuts, area.width);
```

Use `Constraint::Length(shortcut_rows)` for the footer and render it with `render_shortcuts`.

- [ ] **Step 4: Clamp content heights to available space**

- Reserve header, heatmap, summary, status, and shortcut rows first.
- Give the Recent/Goals region the remaining `Constraint::Min(1)` height instead of a content-derived fixed height that can exceed the frame.
- Allow Ratatui to shrink the heatmap/summary regions safely for direct small renders.
- Use saturating subtraction for all inner heights and gauge widths.
- Truncate Recent practice names and goal titles to the real inner width before adding fixed metric/gauge suffixes.
- Keep Summary quote wrapping inside the summary block.
- Skip rendering rows when their target rectangle is empty.

- [ ] **Step 5: Verify quit and HRV modes advertise only active commands**

Add tests entering `ConfirmQuit` and `HrvInput`, render, and assert inactive navigation commands are absent while confirmation/cancel commands are present.

- [ ] **Step 6: Run Dashboard, lint, and full TUI tests**

```bash
cargo test tui::dashboard::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test tui:: -- --nocapture
```

Expected: all pass.

- [ ] **Step 7: Commit Dashboard migration**

```bash
git add src/tui/dashboard.rs tests/tui_lint_test.rs
git commit -m "feat(dashboard): use responsive shared interactions"
```

---

### Task 9: Update Active Documentation and Run Final Release Gates

**Files:**
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `CLAUDE.md`
- Modify: `docs/specs/log-entry-spec.md`
- Modify: `docs/specs/dashboard-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed runtime behavior from Tasks 1–8.
- Produces: accurate user/developer documentation and a verified Batch 2 handoff.

- [ ] **Step 1: Document only implemented behavior**

Update the active documents with:

- Log Entry shared input, visible cursor, list overflow, failure retention, and active-mode shortcut behavior.
- Dashboard remaining navigation, responsive layout, HRV failure retention, status, and dynamic shortcut behavior.
- The permanent abbreviation-table removal and absence of Quick Log.
- The new Batch 2 completion state in the broader framework design.

Do not document a future multi-line input, replacement Quick Log, or Batch 3 behavior as shipped.

- [ ] **Step 2: Run formatting and static analysis**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: zero warnings and zero errors.

- [ ] **Step 3: Run all tests and build**

```bash
cargo test
cargo build
```

Expected: every unit, database, integration, i18n, removal, and TUI lint test passes; build exits zero.

- [ ] **Step 4: Run final removal and duplicate-code audits**

```bash
rg -n -g '!removed_quick_log_test.rs' "QuickLog|quick_log|quicklog-|key-quick-log|Abbreviation|create_abbreviation|list_abbreviations|update_abbreviation|delete_abbreviation|crate::llm|crate::config|ureq|toml =" src tests locales Cargo.toml README.md README_zh.md CLAUDE.md docs/specs
rg -n "filter_cursor|field1_cursor|field2_cursor|warm_up_cursor|cool_down_cursor|rpe_cursor|note_cursor|date_input_cursor|fn adjust_scroll|visible_input_spans" src/tui/log_entry.rs src/tui/dashboard.rs
git diff --check 085a1e2..HEAD
git status --short
```

Expected:

- both source/reference searches have no matches
- the worktree is clean after the final documentation commit
- the approved design and implementation plan remain as the intentional audit record for the removal

- [ ] **Step 5: Commit final documentation**

```bash
git add README.md README_zh.md CLAUDE.md docs/specs/log-entry-spec.md docs/specs/dashboard-spec.md docs/specs/navigation-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Batch 2 behavior"
```

- [ ] **Step 6: Request read-only code review**

Ask the reviewer to compare base `085a1e2` with final HEAD and report only Critical, Important, and Minor findings. Require explicit verification of:

- destructive table removal without unrelated data loss
- absence of live Quick Log/LLM/Abbreviation code
- Log Entry failure-state retention and bounded inputs
- Dashboard HRV failure-state retention and responsive layout
- release gates and clean scope

Resolve every valid Critical or Important finding with a new failing regression test before changing implementation, then rerun all release gates.

- [ ] **Step 7: Prepare the manual acceptance handoff**

Provide the built binary and ask the user to verify:

1. Quick Log, `w`, LLM configuration, and Abbreviations are absent.
2. An existing database opens, preserves training data, and no longer has the `abbreviations` table.
3. Log Entry reaches the first/last practice and keeps long Unicode cursors visible.
4. Log Entry validation/database failures retain form, focus, and return target.
5. Dashboard renders Heatmap, Summary, Recent, Goals, Quote, HRV, status, and every remaining shortcut at `80x35`.
6. Invalid/failed HRV saves retain input mode and text.
7. Quit confirmation shows only its active actions.

Do not begin any later batch until the user explicitly accepts Batch 2.
