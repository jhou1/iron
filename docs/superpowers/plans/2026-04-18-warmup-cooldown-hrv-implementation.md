# Warm-up, Cool-down & HRV Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add warm-up/cool-down text fields to training logs and a daily HRV score input on the dashboard.

**Architecture:** Two changes: (1) extend the `logs` table with nullable `warm_up`/`cool_down` TEXT columns, threading the new fields through model, db, log entry UI, history detail, and export/import; (2) add a new `daily_metrics` table for once-per-day HRV scores (0-100), with dashboard display and input, plus export/import support.

**Tech Stack:** Rust, ratatui, rusqlite, serde, chrono, fluent (i18n)

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/model.rs` | Modify | Add `warm_up`/`cool_down` to `Log`, add `DailyMetrics` struct |
| `src/db.rs` | Modify | Schema migration, update log CRUD, add daily_metrics CRUD |
| `src/tui/log_entry.rs` | Modify | Add `EnterWarmUpCoolDown` phase between sets and note |
| `src/tui/dashboard.rs` | Modify | Add HRV display row, `HrvInput` mode, `[v]` keybinding |
| `src/tui/history.rs` | Modify | Show warm-up/cool-down in detail pane |
| `src/export.rs` | Modify | Add warm_up/cool_down to ExportLog, add ExportDailyMetrics |
| `locales/en.ftl` | Modify | Add i18n keys for warm-up, cool-down, HRV |
| `locales/zh-CN.ftl` | Modify | Add Chinese translations |
| `tests/db_test.rs` | Modify | Add tests for warm_up/cool_down in log CRUD, daily HRV CRUD |
| `tests/export_import_integration_test.rs` | Modify | Add test for warm_up/cool_down and daily_metrics round-trip |
| `README.md` | Modify | Document new features |
| `Cargo.toml` | Modify | Bump version to 0.4.0 |

---

### Task 1: Model — Add warm_up/cool_down to Log, add DailyMetrics

**Files:**
- Modify: `src/model.rs:68-74` (Log struct)
- Modify: `src/model.rs` (add DailyMetrics after Quote struct, line ~165)

- [ ] **Step 1: Add warm_up and cool_down fields to the Log struct**

In `src/model.rs`, update the `Log` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: i64,
    pub practice_id: i64,
    pub logged_at: NaiveDateTime,
    pub note: Option<String>,
    pub warm_up: Option<String>,
    pub cool_down: Option<String>,
}
```

- [ ] **Step 2: Add the DailyMetrics struct**

Add after the `Quote` struct at the end of `src/model.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DailyMetrics {
    pub id: i64,
    pub date: String,
    pub hrv: Option<i32>,
}
```

- [ ] **Step 3: Run cargo check to verify compilation**

Run: `cargo check 2>&1 | head -30`

Expected: Compilation errors in `db.rs` and other files that construct `Log` without the new fields. This is expected — we'll fix those in Task 2.

- [ ] **Step 4: Commit**

```bash
git add src/model.rs
git commit -m "feat: add warm_up/cool_down to Log, add DailyMetrics struct"
```

---

### Task 2: Database — Schema migration and log CRUD updates

**Files:**
- Modify: `src/db.rs:49-108` (init_schema)
- Modify: `src/db.rs:178-234` (create_log, create_log_at, update_log)
- Modify: `src/db.rs:270-520` (list_logs_all, list_logs_recent, list_logs_for_practice, export_all)

- [ ] **Step 1: Write failing tests for warm_up/cool_down in log CRUD**

Add to `tests/db_test.rs`:

```rust
#[test]
fn create_log_with_warmup_cooldown() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![SetData::Weighted { weight: 60.0, reps: 10 }];
    db.create_log(practice_id, &sets, Some("Good session"), Some("5 min jump rope"), Some("Static stretches")).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].log.warm_up, Some("5 min jump rope".to_string()));
    assert_eq!(entries[0].log.cool_down, Some("Static stretches".to_string()));
}

#[test]
fn update_log_warmup_cooldown() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Squat", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![SetData::Weighted { weight: 60.0, reps: 10 }];
    db.create_log(practice_id, &sets, None, None, None).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    let log_id = entries[0].log.id;

    db.update_log(log_id, &sets, Some("Updated"), None, Some("Foam rolling"), Some("Cool down walk")).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    assert_eq!(entries[0].log.warm_up, Some("Foam rolling".to_string()));
    assert_eq!(entries[0].log.cool_down, Some("Cool down walk".to_string()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test create_log_with_warmup_cooldown update_log_warmup_cooldown 2>&1 | tail -20`

Expected: Compilation errors — `create_log` and `update_log` don't accept the new parameters yet.

- [ ] **Step 3: Add schema migration in init_schema**

In `src/db.rs`, add the `daily_metrics` table to the main `CREATE TABLE IF NOT EXISTS` batch (inside `execute_batch`), after the `quotes` table:

```sql
CREATE TABLE IF NOT EXISTS daily_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    hrv INTEGER
);
```

Then add migration lines after the existing milestones migrations (around line 106):

```rust
let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN warm_up TEXT", []);
let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN cool_down TEXT", []);
```

- [ ] **Step 4: Update create_log to accept warm_up and cool_down**

Update the `create_log` method signature and body:

```rust
#[allow(dead_code)]
pub fn create_log(
    &self,
    practice_id: i64,
    sets: &[SetData],
    note: Option<&str>,
    warm_up: Option<&str>,
    cool_down: Option<&str>,
) -> Result<i64> {
    let now = Local::now().naive_local();
    self.conn.execute(
        "INSERT INTO logs (practice_id, logged_at, note, warm_up, cool_down) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![practice_id, now.to_string(), note, warm_up, cool_down],
    )?;
    let log_id = self.conn.last_insert_rowid();
    self.insert_sets(log_id, sets)?;
    Ok(log_id)
}
```

- [ ] **Step 5: Update create_log_at to accept warm_up and cool_down**

```rust
pub fn create_log_at(
    &self,
    practice_id: i64,
    logged_at: &NaiveDateTime,
    sets: &[SetData],
    note: Option<&str>,
    warm_up: Option<&str>,
    cool_down: Option<&str>,
) -> Result<i64> {
    self.conn.execute(
        "INSERT INTO logs (practice_id, logged_at, note, warm_up, cool_down) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![practice_id, logged_at.to_string(), note, warm_up, cool_down],
    )?;
    let log_id = self.conn.last_insert_rowid();
    self.insert_sets(log_id, sets)?;
    Ok(log_id)
}
```

- [ ] **Step 6: Update update_log to accept warm_up and cool_down**

```rust
pub fn update_log(
    &self,
    log_id: i64,
    sets: &[SetData],
    note: Option<&str>,
    logged_at: Option<&NaiveDateTime>,
    warm_up: Option<&str>,
    cool_down: Option<&str>,
) -> Result<()> {
    if let Some(dt) = logged_at {
        self.conn.execute(
            "UPDATE logs SET note = ?1, logged_at = ?2, warm_up = ?3, cool_down = ?4 WHERE id = ?5",
            params![note, dt.to_string(), warm_up, cool_down, log_id],
        )?;
    } else {
        self.conn.execute(
            "UPDATE logs SET note = ?1, warm_up = ?2, cool_down = ?3 WHERE id = ?4",
            params![note, warm_up, cool_down, log_id],
        )?;
    }
    self.conn
        .execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
    self.insert_sets(log_id, sets)?;
    Ok(())
}
```

- [ ] **Step 7: Update all list_logs helpers to read warm_up/cool_down**

Each of `list_logs_all`, `list_logs_recent`, `list_logs_for_practice`, and `export_all` follow the same pattern. Update their SELECT to include `l.warm_up, l.cool_down` and update the `query_map` closure to read them, and populate the `Log` struct. Example for `list_logs_all`:

Change the SQL to:
```sql
SELECT l.id, l.practice_id, l.logged_at, l.note, l.warm_up, l.cool_down, p.name, p.practice_type
FROM logs l
JOIN practices p ON l.practice_id = p.id
ORDER BY l.logged_at DESC
```

Update `query_map` to read 8 columns:
```rust
let rows = stmt.query_map([], |row| {
    Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, i64>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, Option<String>>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, String>(7)?,
    ))
})?;
```

And destructure with the new fields:
```rust
let (log_id, practice_id, logged_at_str, note, warm_up, cool_down, practice_name, pt_str) = row?;
```

And build the `Log` with:
```rust
log: Log {
    id: log_id,
    practice_id,
    logged_at,
    note,
    warm_up,
    cool_down,
},
```

Apply the same pattern to `list_logs_recent`, `list_logs_for_practice`, and `export_all`.

- [ ] **Step 8: Fix all existing callers of create_log, create_log_at, and update_log**

In `src/tui/log_entry.rs` (around line 791), update the `handle_enter_note` save calls. These will be fully updated in Task 5, but for now pass `None, None` for warm_up/cool_down to make things compile:

For `update_log` call:
```rust
let _ = db.update_log(log_id, &self.sets, note, Some(&datetime), None, None);
```

For `create_log_at` call:
```rust
let _ = db.create_log_at(practice.id, &datetime, &self.sets, note, None, None);
```

In `tests/db_test.rs`, update existing test calls:
- `create_log(practice_id, &sets, Some("Felt good"))` → `create_log(practice_id, &sets, Some("Felt good"), None, None)`
- `create_log(practice_id, &sets, None)` → `create_log(practice_id, &sets, None, None, None)`
- `update_log(log_id, &new_sets, Some("Updated"), None)` → `update_log(log_id, &new_sets, Some("Updated"), None, None, None)`

In `tests/export_import_integration_test.rs`, update all `create_log_at` calls to add `, None, None` at the end.

In `src/export.rs`, update the `import_from_json` function's `create_log_at` call (around line 250):
```rust
db.create_log_at(
    practice_id,
    &logged_at,
    &sets,
    log.note.as_deref(),
    None,
    None,
)?;
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `cargo test 2>&1 | tail -20`

Expected: All tests pass, including the two new ones.

- [ ] **Step 10: Commit**

```bash
git add src/db.rs src/model.rs src/tui/log_entry.rs src/export.rs tests/db_test.rs tests/export_import_integration_test.rs
git commit -m "feat: add warm_up/cool_down to logs schema and CRUD"
```

---

### Task 3: Database — Daily HRV CRUD

**Files:**
- Modify: `src/db.rs` (add new methods after Quote CRUD section)
- Modify: `tests/db_test.rs`

- [ ] **Step 1: Write failing tests for daily HRV CRUD**

Add to `tests/db_test.rs`:

```rust
use ironcli::model::DailyMetrics;

#[test]
fn set_and_get_daily_hrv() {
    let db = Database::open_in_memory().unwrap();

    // No HRV for today initially
    let hrv = db.get_daily_hrv("2026-04-18").unwrap();
    assert_eq!(hrv, None);

    // Set HRV
    db.set_daily_hrv("2026-04-18", 72).unwrap();
    let hrv = db.get_daily_hrv("2026-04-18").unwrap();
    assert_eq!(hrv, Some(72));

    // Update HRV (upsert)
    db.set_daily_hrv("2026-04-18", 68).unwrap();
    let hrv = db.get_daily_hrv("2026-04-18").unwrap();
    assert_eq!(hrv, Some(68));
}

#[test]
fn list_daily_metrics() {
    let db = Database::open_in_memory().unwrap();
    db.set_daily_hrv("2026-04-17", 65).unwrap();
    db.set_daily_hrv("2026-04-18", 72).unwrap();

    let metrics = db.list_daily_metrics().unwrap();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].date, "2026-04-17");
    assert_eq!(metrics[0].hrv, Some(65));
    assert_eq!(metrics[1].date, "2026-04-18");
    assert_eq!(metrics[1].hrv, Some(72));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test set_and_get_daily_hrv list_daily_metrics 2>&1 | tail -20`

Expected: Compilation errors — methods don't exist yet.

- [ ] **Step 3: Implement the three daily HRV methods**

Add to `src/db.rs`, after the Quote CRUD section (around line 764):

```rust
// ── Daily Metrics CRUD ───────────────────────────────────────────

pub fn get_daily_hrv(&self, date: &str) -> Result<Option<i32>> {
    let mut stmt = self.conn.prepare(
        "SELECT hrv FROM daily_metrics WHERE date = ?1",
    )?;
    let result = stmt.query_row(params![date], |row| row.get::<_, Option<i32>>(0));
    match result {
        Ok(hrv) => Ok(hrv),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set_daily_hrv(&self, date: &str, hrv: i32) -> Result<()> {
    self.conn.execute(
        "INSERT INTO daily_metrics (date, hrv) VALUES (?1, ?2)
         ON CONFLICT(date) DO UPDATE SET hrv = ?2",
        params![date, hrv],
    )?;
    Ok(())
}

pub fn list_daily_metrics(&self) -> Result<Vec<crate::model::DailyMetrics>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, date, hrv FROM daily_metrics ORDER BY date",
    )?;
    let metrics = stmt
        .query_map([], |row| {
            Ok(crate::model::DailyMetrics {
                id: row.get(0)?,
                date: row.get(1)?,
                hrv: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(metrics)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test set_and_get_daily_hrv list_daily_metrics 2>&1 | tail -20`

Expected: Both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/db.rs tests/db_test.rs
git commit -m "feat: add daily HRV CRUD methods"
```

---

### Task 4: I18n — Add translation keys

**Files:**
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

- [ ] **Step 1: Add English translation keys**

Add to `locales/en.ftl` in the appropriate sections:

In the `# ── Log Entry ──` section, after `log-note-optional`:
```
log-warmup-label = Warm-up
log-cooldown-label = Cool-down
log-warmup-cooldown-title = Log { $name } — Warm-up & Cool-down
```

In the `# ── Dashboard ──` section, after `dashboard-no-quotes-modal`:
```
dashboard-hrv-label = HRV
dashboard-hrv-edit-hint = [v] edit
dashboard-hrv-record-hint = [v] record
dashboard-hrv-input-hint = (0-100, Enter to save, Esc to cancel)
```

In the `# ── Keyboard labels ──` section, after `key-no`:
```
key-hrv = HRV
key-next = Next
```

In the `# ── History ──` section, after `history-note`:
```
history-warmup = Warm-up: { $text }
history-cooldown = Cool-down: { $text }
```

- [ ] **Step 2: Add Chinese translation keys**

Add to `locales/zh-CN.ftl` in the matching sections:

In the `# ── 记录条目 ──` section, after `log-note-optional`:
```
log-warmup-label = 热身
log-cooldown-label = 放松
log-warmup-cooldown-title = 记录 { $name } — 热身与放松
```

In the `# ── 仪表盘 ──` section, after `dashboard-no-quotes-modal`:
```
dashboard-hrv-label = HRV
dashboard-hrv-edit-hint = [v] 编辑
dashboard-hrv-record-hint = [v] 记录
dashboard-hrv-input-hint = (0-100, Enter 保存, Esc 取消)
```

In the `# ── 按键标签 ──` section, after `key-no`:
```
key-hrv = HRV
key-next = 下一步
```

In the `# ── 历史 ──` section, after `history-note`:
```
history-warmup = 热身：{ $text }
history-cooldown = 放松：{ $text }
```

- [ ] **Step 3: Run i18n tests to verify translations parse**

Run: `cargo test --test i18n_test 2>&1 | tail -10`

Expected: All i18n tests pass.

- [ ] **Step 4: Commit**

```bash
git add locales/en.ftl locales/zh-CN.ftl
git commit -m "feat: add i18n keys for warm-up, cool-down, and HRV"
```

---

### Task 5: Log Entry UI — Add EnterWarmUpCoolDown phase

**Files:**
- Modify: `src/tui/log_entry.rs`

- [ ] **Step 1: Add the new phase enum variant and state fields**

Add `EnterWarmUpCoolDown` to the `Phase` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
enum Phase {
    SelectPractice,
    EnterSets,
    EnterWarmUpCoolDown,
    EnterNote,
}
```

Add new fields to `LogEntryScreen`:

```rust
pub struct LogEntryScreen {
    // ... existing fields ...
    warm_up: String,
    warm_up_cursor: usize,
    cool_down: String,
    cool_down_cursor: usize,
    warmup_cooldown_active: usize, // 0 = warm_up, 1 = cool_down
}
```

- [ ] **Step 2: Initialize the new fields in new() and from_existing()**

In `new()`, add to the struct initialization:

```rust
warm_up: String::new(),
warm_up_cursor: 0,
cool_down: String::new(),
cool_down_cursor: 0,
warmup_cooldown_active: 0,
```

In `from_existing()`, pre-fill from the existing log entry:

```rust
let warm_up = log_entry.log.warm_up.clone().unwrap_or_default();
let cool_down = log_entry.log.cool_down.clone().unwrap_or_default();
```

And in the struct initialization:

```rust
warm_up_cursor: warm_up.len(),
warm_up,
cool_down_cursor: cool_down.len(),
cool_down,
warmup_cooldown_active: 0,
```

- [ ] **Step 3: Update render() and handle_key() to dispatch the new phase**

In `render()`:

```rust
pub fn render(&self, frame: &mut Frame) {
    match self.phase {
        Phase::SelectPractice => self.render_select_practice(frame),
        Phase::EnterSets => self.render_enter_sets(frame),
        Phase::EnterWarmUpCoolDown => self.render_warmup_cooldown(frame),
        Phase::EnterNote => self.render_enter_note(frame),
    }
}
```

In `handle_key()`:

```rust
pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
    match self.phase {
        Phase::SelectPractice => self.handle_select_practice(key),
        Phase::EnterSets => self.handle_enter_sets(key),
        Phase::EnterWarmUpCoolDown => self.handle_warmup_cooldown(key, db),
        Phase::EnterNote => self.handle_enter_note(key, db),
    }
}
```

- [ ] **Step 4: Update the Ctrl+S transition in handle_enter_sets**

Change the Ctrl+S handler in `handle_enter_sets` (around line 508) to go to `EnterWarmUpCoolDown` instead of `EnterNote`:

```rust
if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
    if !self.sets.is_empty() {
        self.phase = Phase::EnterWarmUpCoolDown;
        self.warmup_cooldown_active = 0;
    }
    return Action::None;
}
```

- [ ] **Step 5: Implement render_warmup_cooldown()**

```rust
fn render_warmup_cooldown(&self, frame: &mut Frame) {
    let area = frame.area();
    let practice = self.chosen_practice.as_ref().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // spacer
            Constraint::Length(1), // warm-up input
            Constraint::Length(1), // cool-down input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // footer
            Constraint::Min(0),   // spacer absorbs excess
        ])
        .split(area);

    // Title
    let title = Line::from(Span::styled(
        format!(" {}", tr_args("log-warmup-cooldown-title", &[
            ("name", FluentValue::from(practice.name.clone())),
        ])),
        Style::default().fg(ACCENT).bold(),
    ));
    frame.render_widget(Paragraph::new(title), chunks[0]);

    // Warm-up input
    let wu_active = self.warmup_cooldown_active == 0;
    let wu_color = if wu_active { ACCENT } else { Color::White };
    let (wu_before, wu_after) = self.warm_up.split_at(self.warm_up_cursor);
    let warmup_line = Line::from(vec![
        Span::styled(format!("  {}: ", tr("log-warmup-label")), Style::default().fg(Color::Gray)),
        Span::styled(wu_before.to_string(), Style::default().fg(wu_color)),
        if wu_active {
            Span::styled("\u{2588}", Style::default().fg(wu_color))
        } else {
            Span::raw("")
        },
        Span::styled(wu_after.to_string(), Style::default().fg(wu_color)),
    ]);
    frame.render_widget(Paragraph::new(warmup_line), chunks[2]);

    // Cool-down input
    let cd_active = self.warmup_cooldown_active == 1;
    let cd_color = if cd_active { ACCENT } else { Color::White };
    let (cd_before, cd_after) = self.cool_down.split_at(self.cool_down_cursor);
    let cooldown_line = Line::from(vec![
        Span::styled(format!("  {}: ", tr("log-cooldown-label")), Style::default().fg(Color::Gray)),
        Span::styled(cd_before.to_string(), Style::default().fg(cd_color)),
        if cd_active {
            Span::styled("\u{2588}", Style::default().fg(cd_color))
        } else {
            Span::raw("")
        },
        Span::styled(cd_after.to_string(), Style::default().fg(cd_color)),
    ]);
    frame.render_widget(Paragraph::new(cooldown_line), chunks[3]);

    // Footer
    let footer = Line::from(vec![
        Span::styled(" [Tab]", Style::default().fg(ACCENT)),
        Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::Gray)),
        Span::styled("[Enter]", Style::default().fg(ACCENT)),
        Span::styled(format!(" {}  ", tr("key-next")), Style::default().fg(Color::Gray)),
        Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
        Span::styled(format!(" {}  ", tr("key-save")), Style::default().fg(Color::Gray)),
        Span::styled("[Esc]", Style::default().fg(ACCENT)),
        Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[5]);
}
```

- [ ] **Step 6: Implement handle_warmup_cooldown()**

```rust
fn handle_warmup_cooldown(&mut self, key: KeyEvent, _db: &Database) -> Action {
    // Get current field references
    let (text, cursor) = if self.warmup_cooldown_active == 0 {
        (&mut self.warm_up, &mut self.warm_up_cursor)
    } else {
        (&mut self.cool_down, &mut self.cool_down_cursor)
    };

    // Ctrl+S to save immediately (skip note)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        self.phase = Phase::EnterNote;
        self.note_cursor = self.note.len();
        return Action::None;
    }

    // Emacs-style cursor navigation
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b')
        || key.code == KeyCode::Left
    {
        if *cursor > 0 {
            *cursor = text[..*cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        return Action::None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
        || key.code == KeyCode::Right
    {
        if *cursor < text.len() {
            *cursor = text[*cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| *cursor + i)
                .unwrap_or(text.len());
        }
        return Action::None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a')
        || key.code == KeyCode::Home
    {
        *cursor = 0;
        return Action::None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e')
        || key.code == KeyCode::End
    {
        *cursor = text.len();
        return Action::None;
    }

    match key.code {
        KeyCode::Esc => Action::Navigate(self.return_to.clone()),
        KeyCode::Tab => {
            self.warmup_cooldown_active = if self.warmup_cooldown_active == 0 { 1 } else { 0 };
            Action::None
        }
        KeyCode::Enter => {
            self.phase = Phase::EnterNote;
            self.note_cursor = self.note.len();
            Action::None
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                let prev = text[..*cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                text.remove(prev);
                *cursor = prev;
            }
            Action::None
        }
        KeyCode::Char(c) => {
            text.insert(*cursor, c);
            *cursor += c.len_utf8();
            Action::None
        }
        _ => Action::None,
    }
}
```

- [ ] **Step 7: Update handle_enter_note to pass warm_up/cool_down when saving**

In `handle_enter_note`, update the save logic (around line 790):

```rust
KeyCode::Enter => {
    let practice = self.chosen_practice.as_ref().unwrap();
    let note = if self.note.is_empty() {
        None
    } else {
        Some(self.note.as_str())
    };
    let warm_up = if self.warm_up.is_empty() {
        None
    } else {
        Some(self.warm_up.as_str())
    };
    let cool_down = if self.cool_down.is_empty() {
        None
    } else {
        Some(self.cool_down.as_str())
    };
    let date = NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let datetime = date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    if let Some(log_id) = self.editing_log_id {
        let _ = db.update_log(log_id, &self.sets, note, Some(&datetime), warm_up, cool_down);
    } else {
        let _ = db.create_log_at(practice.id, &datetime, &self.sets, note, warm_up, cool_down);
    }
    Action::Navigate(self.return_to.clone())
}
```

- [ ] **Step 8: Run cargo check and cargo test**

Run: `cargo test 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/tui/log_entry.rs
git commit -m "feat: add warm-up/cool-down phase to log entry flow"
```

---

### Task 6: History Screen — Show warm-up/cool-down in detail

**Files:**
- Modify: `src/tui/history.rs:181-232` (render_detail method)

- [ ] **Step 1: Add warm-up and cool-down display in render_detail**

In `render_detail()`, after the sets loop and before the note display (around line 222), add:

```rust
if let Some(warm_up) = &entry.log.warm_up {
    lines.push(Line::from(Span::styled(
        format!("    {}", tr_args("history-warmup", &[
            ("text", FluentValue::from(warm_up.clone())),
        ])),
        Style::default().fg(Color::Gray),
    )));
}

if let Some(cool_down) = &entry.log.cool_down {
    lines.push(Line::from(Span::styled(
        format!("    {}", tr_args("history-cooldown", &[
            ("text", FluentValue::from(cool_down.clone())),
        ])),
        Style::default().fg(Color::Gray),
    )));
}
```

Also add the import at the top of the file if not already present:

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Increase the detail pane height to accommodate the new lines**

In `render()`, change the detail pane constraint from `Constraint::Length(4)` to `Constraint::Length(6)` (around line 67):

```rust
Constraint::Length(6),           // detail pane
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check 2>&1 | tail -10`

Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add src/tui/history.rs
git commit -m "feat: show warm-up/cool-down in history detail pane"
```

---

### Task 7: Dashboard — HRV display and input

**Files:**
- Modify: `src/tui/dashboard.rs`

- [ ] **Step 1: Add HrvInput to DashboardMode enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardMode {
    Normal,
    Goals,
    AddGoal,
    AddMilestone,
    EditItem,
    EditDate,
    ConfirmDelete,
    QuotesManage,
    QuotesEdit,
    HrvInput,
}
```

- [ ] **Step 2: Add HRV state fields to DashboardScreen**

Add to the struct:

```rust
pub struct DashboardScreen {
    // ... existing fields ...
    hrv_today: Option<i32>,
    hrv_input: String,
}
```

- [ ] **Step 3: Initialize HRV fields in new() and refresh()**

In `new()`:

```rust
let today = chrono::Local::now().format("%Y-%m-%d").to_string();
let hrv_today = db.get_daily_hrv(&today)?;
```

Add to the struct initialization:

```rust
hrv_today,
hrv_input: String::new(),
```

In `refresh()`, add:

```rust
let today = chrono::Local::now().format("%Y-%m-%d").to_string();
self.hrv_today = db.get_daily_hrv(&today)?;
```

- [ ] **Step 4: Add HRV row to the dashboard layout**

In `render()`, add a new `Constraint::Length(1)` for the HRV row between the quote box and the split panes. Update the layout constraints:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),            // [0] title
        Constraint::Length(7),            // [1] heatmap header ASCII art
        Constraint::Length(10),           // [2] heatmap
        Constraint::Length(quote_height), // [3] daily quote box
        Constraint::Length(1),            // [4] HRV row
        Constraint::Length(pane_height),  // [5] split panes
        Constraint::Length(1),            // [6] footer
        Constraint::Min(0),              // [7] spacer absorbs excess at bottom
    ])
    .split(area);
```

Update all subsequent `chunks[N]` references: what was `chunks[4]` (split panes) becomes `chunks[5]`, `chunks[5]` (footer) becomes `chunks[6]`.

- [ ] **Step 5: Render the HRV row**

Add the HRV row rendering after the quote box rendering (at the new `chunks[4]`):

```rust
// ── HRV row ──
let hrv_area = Rect {
    x: chunks[4].x + 1,
    y: chunks[4].y,
    width: chunks[4].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
    height: chunks[4].height,
};
let hrv_line = if self.mode == DashboardMode::HrvInput {
    Line::from(vec![
        Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
        Span::styled(&self.hrv_input, Style::default().fg(ACCENT)),
        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
        Span::styled(format!("  {}", tr("dashboard-hrv-input-hint")), Style::default().fg(Color::Gray)),
    ])
} else if let Some(hrv) = self.hrv_today {
    Line::from(vec![
        Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
        Span::styled(format!("{}", hrv), Style::default().fg(GREEN)),
        Span::styled(format!("  {}", tr("dashboard-hrv-edit-hint")), Style::default().fg(Color::DarkGray)),
    ])
} else {
    Line::from(vec![
        Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
        Span::styled("--", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("  {}", tr("dashboard-hrv-record-hint")), Style::default().fg(Color::DarkGray)),
    ])
};
frame.render_widget(Paragraph::new(hrv_line), hrv_area);
```

- [ ] **Step 6: Add [v] keybinding to handle_normal**

In `handle_normal()`, add the `v` key handler before `KeyCode::Char('q')`:

```rust
KeyCode::Char('v') => {
    self.hrv_input = self.hrv_today.map(|v| v.to_string()).unwrap_or_default();
    self.mode = DashboardMode::HrvInput;
    Action::None
}
```

- [ ] **Step 7: Add HrvInput handler dispatch in handle_key**

In `handle_key()`, add the new mode:

```rust
DashboardMode::HrvInput => self.handle_hrv_input(key, db),
```

- [ ] **Step 8: Implement handle_hrv_input**

```rust
fn handle_hrv_input(&mut self, key: KeyEvent, db: &Database) -> Action {
    match key.code {
        KeyCode::Enter => {
            if let Ok(hrv) = self.hrv_input.parse::<i32>() {
                if (0..=100).contains(&hrv) {
                    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                    let _ = db.set_daily_hrv(&today, hrv);
                    self.hrv_today = Some(hrv);
                    self.mode = DashboardMode::Normal;
                }
            }
            Action::None
        }
        KeyCode::Esc => {
            self.hrv_input.clear();
            self.mode = DashboardMode::Normal;
            Action::None
        }
        KeyCode::Backspace => {
            self.hrv_input.pop();
            Action::None
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            self.hrv_input.push(c);
            Action::None
        }
        _ => Action::None,
    }
}
```

- [ ] **Step 9: Add [v] to the dashboard footer**

In `handle_normal`'s footer rendering (the `if self.mode == DashboardMode::Normal` block), add before the `[q]` quit entry:

```rust
Span::styled("[v]", Style::default().fg(ACCENT)),
Span::styled(format!(" {}  ", tr("key-hrv")), Style::default().fg(Color::Gray)),
```

- [ ] **Step 10: Run cargo check**

Run: `cargo check 2>&1 | tail -10`

Expected: Compiles successfully.

- [ ] **Step 11: Commit**

```bash
git add src/tui/dashboard.rs
git commit -m "feat: add HRV display and input on dashboard"
```

---

### Task 8: Export/Import — warm_up, cool_down, and daily_metrics

**Files:**
- Modify: `src/export.rs`
- Modify: `tests/export_import_integration_test.rs`

- [ ] **Step 1: Write failing test for warm_up/cool_down export round-trip**

Add to `tests/export_import_integration_test.rs`:

```rust
#[test]
fn warmup_cooldown_survive_export_import() {
    let source = TestDb::new();
    let db = &source.db;

    let bench = db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let t1 = dt("2026-04-18 10:00:00");
    let sets = vec![SetData::Weighted { weight: 60.0, reps: 10 }];

    db.create_log_at(bench.id, &t1, &sets, Some("Good"), Some("Jump rope"), Some("Stretches")).unwrap();

    let export_path = source.export_path();
    export_to_json(db, Some(export_path.clone())).unwrap();

    let target = TestDb::new();
    import_from_json(&target.db, &export_path).unwrap();

    let entries = target.db.export_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].log.warm_up, Some("Jump rope".to_string()));
    assert_eq!(entries[0].log.cool_down, Some("Stretches".to_string()));
}
```

- [ ] **Step 2: Write failing test for daily_metrics export round-trip**

Add to `tests/export_import_integration_test.rs`:

```rust
#[test]
fn daily_metrics_survive_export_import() {
    let source = TestDb::new();
    let db = &source.db;

    db.set_daily_hrv("2026-04-17", 65).unwrap();
    db.set_daily_hrv("2026-04-18", 72).unwrap();

    let export_path = source.export_path();
    export_to_json(db, Some(export_path.clone())).unwrap();

    let target = TestDb::new();
    import_from_json(&target.db, &export_path).unwrap();

    let metrics = target.db.list_daily_metrics().unwrap();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].date, "2026-04-17");
    assert_eq!(metrics[0].hrv, Some(65));
    assert_eq!(metrics[1].date, "2026-04-18");
    assert_eq!(metrics[1].hrv, Some(72));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test warmup_cooldown_survive daily_metrics_survive 2>&1 | tail -20`

Expected: Compilation errors or assertion failures — export doesn't handle the new fields yet.

- [ ] **Step 4: Update ExportLog struct**

Add the new fields to `ExportLog`:

```rust
#[derive(Serialize, Deserialize)]
pub struct ExportLog {
    pub id: i64,
    pub practice: String,
    pub logged_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub warm_up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cool_down: Option<String>,
    pub sets: Vec<ExportSet>,
}
```

- [ ] **Step 5: Add ExportDailyMetrics struct and update ExportData**

After `ExportQuote`, add:

```rust
#[derive(Serialize, Deserialize)]
pub struct ExportDailyMetrics {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv: Option<i32>,
}
```

Update `ExportData`:

```rust
#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub practices: Vec<ExportPractice>,
    pub logs: Vec<ExportLog>,
    #[serde(default)]
    pub goals: Vec<ExportGoal>,
    #[serde(default)]
    pub quotes: Vec<ExportQuote>,
    #[serde(default)]
    pub daily_metrics: Vec<ExportDailyMetrics>,
}
```

- [ ] **Step 6: Update export_to_json to include warm_up/cool_down and daily_metrics**

In the `export_logs` mapping (around line 124), add `warm_up` and `cool_down`:

```rust
ExportLog {
    id: entry.log.id,
    practice: entry.practice_name.clone(),
    logged_at: entry.log.logged_at.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
    note: entry.log.note.clone(),
    warm_up: entry.log.warm_up.clone(),
    cool_down: entry.log.cool_down.clone(),
    sets,
}
```

After the quotes export section, add daily_metrics export:

```rust
let daily_metrics_list = db.list_daily_metrics()?;
let export_daily_metrics: Vec<ExportDailyMetrics> = daily_metrics_list
    .iter()
    .map(|m| ExportDailyMetrics {
        date: m.date.clone(),
        hrv: m.hrv,
    })
    .collect();
```

Update the `ExportData` struct construction to include:

```rust
daily_metrics: export_daily_metrics,
```

- [ ] **Step 7: Update import_from_json to handle warm_up/cool_down and daily_metrics**

Update the `create_log_at` call in the import loop to pass warm_up and cool_down:

```rust
db.create_log_at(
    practice_id,
    &logged_at,
    &sets,
    log.note.as_deref(),
    log.warm_up.as_deref(),
    log.cool_down.as_deref(),
)?;
```

After the quotes import section, add daily_metrics import:

```rust
for dm in &data.daily_metrics {
    if let Some(hrv) = dm.hrv {
        let _ = db.set_daily_hrv(&dm.date, hrv);
    }
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test 2>&1 | tail -20`

Expected: All tests pass, including the two new export/import tests.

- [ ] **Step 9: Commit**

```bash
git add src/export.rs tests/export_import_integration_test.rs
git commit -m "feat: add warm-up/cool-down and daily metrics to export/import"
```

---

### Task 9: Version bump and README update

**Files:**
- Modify: `Cargo.toml:3` (version)
- Modify: `README.md`

- [ ] **Step 1: Bump version to 0.4.0**

In `Cargo.toml`, change:

```toml
version = "0.4.0"
```

- [ ] **Step 2: Update README.md features list**

Add to the features list (after "Daily motivational quotes"):

```markdown
- **Warm-up & cool-down notes** — optional text fields on each training log
- **Daily HRV tracking** — record your morning HRV score (0-100) on the dashboard
```

- [ ] **Step 3: Update the "Logging a Practice" section**

After step 5 ("Press `Ctrl+S` when done adding sets"), update:

```markdown
5. Press `Ctrl+S` when done adding sets
6. Optionally enter warm-up and cool-down notes (e.g., "5 min jump rope"), press Enter to skip
7. Type an optional note (e.g., "Felt strong today") or press Enter to skip
8. Press Enter to save
```

- [ ] **Step 4: Add "HRV Tracking" section**

Add after the "Daily Quote" section:

```markdown
### HRV Tracking

Record your morning Heart Rate Variability score on the Dashboard.

- Press `v` to enter your HRV score (0-100)
- Type the number and press Enter to save
- Today's HRV is displayed inline on the Dashboard
- HRV data is included in JSON export/import for long-term analysis
```

- [ ] **Step 5: Update the Dashboard keyboard reference table**

Add the `v` key row:

```markdown
| `v` | Record/edit today's HRV |
```

- [ ] **Step 6: Update the Log Entry keyboard reference table**

The existing table is still accurate — Tab and Enter work the same in the new warm-up/cool-down phase. No changes needed.

- [ ] **Step 7: Sync Cargo.lock**

Run: `cargo check`

- [ ] **Step 8: Run all tests**

Run: `cargo test 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock README.md
git commit -m "chore: bump version to 0.4.0, update README with warm-up/cool-down/HRV docs"
```

---

### Task 10: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`

Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1 | tail -20`

Expected: No warnings.

- [ ] **Step 3: Build release binary**

Run: `cargo build --release 2>&1 | tail -5`

Expected: Successful build.

- [ ] **Step 4: Launch and smoke test**

Run: `cargo run`

Verify:
1. Dashboard shows "HRV: -- [v] record"
2. Press `v`, type `72`, press Enter — displays "HRV: 72 [v] edit"
3. Press `l`, select a practice, add a set, press Ctrl+S
4. Warm-up/cool-down screen appears with two empty fields
5. Type a warm-up, Tab to cool-down, type cool-down, Enter
6. Note screen appears, save
7. Press `h`, navigate to the log — detail shows warm-up and cool-down
8. Export (`iron export`), check JSON has `warm_up`, `cool_down`, `daily_metrics`
