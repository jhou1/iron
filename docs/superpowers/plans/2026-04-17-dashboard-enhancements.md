# Dashboard Enhancements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add goals/milestones tracking, daily motivational quotes, and merge statistics into the Last 14 Days pane on the IronCLI dashboard.

**Architecture:** Three independent features layered onto the existing dashboard. Goals use two new SQLite tables with full CRUD. Quotes are a pure function with no persistence. The stats merge is a rendering-only change. All changes are confined to model.rs, db.rs, dashboard.rs, export.rs, and a new quotes.rs module.

**Tech Stack:** Rust, ratatui, rusqlite, chrono, serde/serde_json

---

### File Map

| File | Action | Responsibility |
|---|---|---|
| `src/model.rs` | Modify | Add `Goal` and `Milestone` structs |
| `src/db.rs` | Modify | Add goals/milestones tables + CRUD methods |
| `src/tui/quotes.rs` | Create | Built-in quote list, file override, daily selection |
| `src/tui/mod.rs` | Modify | Add `pub mod quotes;` |
| `src/tui/dashboard.rs` | Modify | Merge stats, add quote row, add goals pane + editing mode |
| `src/app.rs` | Modify | Pass `&Database` to `dashboard.handle_key()` |
| `src/export.rs` | Modify | Add goals to export/import, bump version to 2 |
| `tests/db_test.rs` | Modify | Add goals/milestones CRUD tests |
| `tests/quotes_test.rs` | Create | Test quote selection and file override |
| `tests/export_import_integration_test.rs` | Modify | Add goals round-trip test |
| `README.md` | Modify | Document goals keybindings and quote file |

---

### Task 1: Add Goal and Milestone model types

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Add Goal and Milestone structs to model.rs**

Add at the end of `src/model.rs` (after the `LogEntry` impl block, line 131):

```rust
#[derive(Debug, Clone)]
pub struct Goal {
    pub id: i64,
    pub title: String,
    pub position: i32,
    pub created_at: NaiveDateTime,
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone)]
pub struct Milestone {
    pub id: i64,
    pub goal_id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i32,
    pub created_at: NaiveDateTime,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1`
Expected: compiles with no errors (structs are unused for now, but no warnings because we'll use them in the next task)

- [ ] **Step 3: Commit**

```bash
git add src/model.rs
git commit -m "feat: add Goal and Milestone model types"
```

---

### Task 2: Add goals/milestones database schema and CRUD

**Files:**
- Modify: `src/db.rs`
- Modify: `tests/db_test.rs`

- [ ] **Step 1: Write the failing tests for goals CRUD**

Add to the end of `tests/db_test.rs`:

```rust
use ironcli::model::Goal;

#[test]
fn create_and_list_goals() {
    let db = Database::open_in_memory().unwrap();
    let id1 = db.create_goal("Master KB Sport").unwrap();
    let id2 = db.create_goal("Run a marathon").unwrap();

    let goals = db.list_goals().unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals[0].id, id1);
    assert_eq!(goals[0].title, "Master KB Sport");
    assert_eq!(goals[0].position, 1);
    assert_eq!(goals[1].id, id2);
    assert_eq!(goals[1].title, "Run a marathon");
    assert_eq!(goals[1].position, 2);
}

#[test]
fn update_goal_title() {
    let db = Database::open_in_memory().unwrap();
    let id = db.create_goal("Old title").unwrap();
    db.update_goal(id, "New title").unwrap();

    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].title, "New title");
}

#[test]
fn delete_goal_cascades_milestones() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("My Goal").unwrap();
    db.create_milestone(goal_id, "Step 1").unwrap();
    db.create_milestone(goal_id, "Step 2").unwrap();

    db.delete_goal(goal_id).unwrap();

    let goals = db.list_goals().unwrap();
    assert!(goals.is_empty());
}

#[test]
fn create_and_list_milestones() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("My Goal").unwrap();
    let m1 = db.create_milestone(goal_id, "First milestone").unwrap();
    let m2 = db.create_milestone(goal_id, "Second milestone").unwrap();

    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones.len(), 2);
    assert_eq!(goals[0].milestones[0].id, m1);
    assert_eq!(goals[0].milestones[0].title, "First milestone");
    assert_eq!(goals[0].milestones[0].completed, false);
    assert_eq!(goals[0].milestones[0].position, 1);
    assert_eq!(goals[0].milestones[1].id, m2);
    assert_eq!(goals[0].milestones[1].title, "Second milestone");
    assert_eq!(goals[0].milestones[1].position, 2);
}

#[test]
fn toggle_milestone_completion() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    let m_id = db.create_milestone(goal_id, "Task").unwrap();

    // Initially not completed
    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones[0].completed, false);

    // Toggle on
    db.toggle_milestone(m_id).unwrap();
    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones[0].completed, true);

    // Toggle off
    db.toggle_milestone(m_id).unwrap();
    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones[0].completed, false);
}

#[test]
fn update_milestone_title() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    let m_id = db.create_milestone(goal_id, "Old").unwrap();

    db.update_milestone(m_id, "New").unwrap();

    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones[0].title, "New");
}

#[test]
fn delete_milestone() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    db.create_milestone(goal_id, "Keep").unwrap();
    let m2 = db.create_milestone(goal_id, "Remove").unwrap();

    db.delete_milestone(m2).unwrap();

    let goals = db.list_goals().unwrap();
    assert_eq!(goals[0].milestones.len(), 1);
    assert_eq!(goals[0].milestones[0].title, "Keep");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test db_test 2>&1`
Expected: FAIL — methods `create_goal`, `list_goals`, etc. do not exist on `Database`

- [ ] **Step 3: Add goals and milestones tables to init_schema**

In `src/db.rs`, inside `init_schema()` (line 48-77), add the two new CREATE TABLE statements after the `sets` table, before the closing `";`:

```sql
CREATE TABLE IF NOT EXISTS goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS milestones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_id INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
```

- [ ] **Step 4: Add the Goal import to db.rs**

At the top of `src/db.rs`, update the use statement on line 5:

```rust
use crate::model::{Goal, Log, LogEntry, Milestone, Practice, PracticeType, Set, SetData};
```

- [ ] **Step 5: Add goals CRUD methods to db.rs**

Add a new section at the end of the `impl Database` block (before the closing `}`), after the existing `log_exists` method:

```rust
    // ── Goal CRUD ─────────────────────────────────────────────────────

    pub fn create_goal(&self, title: &str) -> Result<i64> {
        let now = Local::now().naive_local();
        let position: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM goals",
            [],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO goals (title, position, created_at) VALUES (?1, ?2, ?3)",
            params![title, position, now.format("%Y-%m-%d %H:%M:%S%.f").to_string()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, position, created_at FROM goals ORDER BY position"
        )?;
        let goals: Vec<Goal> = stmt.query_map([], |row| {
            let created_str: String = row.get(3)?;
            Ok(Goal {
                id: row.get(0)?,
                title: row.get(1)?,
                position: row.get(2)?,
                created_at: NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap_or_default(),
                milestones: Vec::new(),
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        let mut result = goals;
        for goal in &mut result {
            let mut ms_stmt = self.conn.prepare(
                "SELECT id, goal_id, title, completed, position, created_at \
                 FROM milestones WHERE goal_id = ?1 ORDER BY position"
            )?;
            goal.milestones = ms_stmt.query_map(params![goal.id], |row| {
                let created_str: String = row.get(5)?;
                Ok(Milestone {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    title: row.get(2)?,
                    completed: row.get::<_, i32>(3)? != 0,
                    position: row.get(4)?,
                    created_at: NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                        .unwrap_or_default(),
                })
            })?.collect::<std::result::Result<Vec<_>, _>>()?;
        }

        Ok(result)
    }

    pub fn update_goal(&self, id: i64, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn delete_goal(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM milestones WHERE goal_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM goals WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Milestone CRUD ────────────────────────────────────────────────

    pub fn create_milestone(&self, goal_id: i64, title: &str) -> Result<i64> {
        let now = Local::now().naive_local();
        let position: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM milestones WHERE goal_id = ?1",
            params![goal_id],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO milestones (goal_id, title, completed, position, created_at) \
             VALUES (?1, ?2, 0, ?3, ?4)",
            params![goal_id, title, position, now.format("%Y-%m-%d %H:%M:%S%.f").to_string()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_milestone(&self, id: i64, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE milestones SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn toggle_milestone(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE milestones SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END \
             WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete_milestone(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM milestones WHERE id = ?1", params![id])?;
        Ok(())
    }
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test db_test 2>&1`
Expected: All tests pass (17 total: 10 existing + 7 new)

- [ ] **Step 7: Commit**

```bash
git add src/model.rs src/db.rs tests/db_test.rs
git commit -m "feat: add goals and milestones database schema and CRUD"
```

---

### Task 3: Add daily quotes module

**Files:**
- Create: `src/tui/quotes.rs`
- Modify: `src/tui/mod.rs`
- Create: `tests/quotes_test.rs`

- [ ] **Step 1: Write the failing tests**

Create `tests/quotes_test.rs`:

```rust
use std::io::Write;

// We'll test the quote module once it exists.
// For now, test the selection logic and file override.

#[test]
fn builtin_quotes_not_empty() {
    let quotes = ironcli_quotes::builtin_quotes();
    assert!(!quotes.is_empty());
    assert!(quotes.len() >= 20);
}

#[test]
fn daily_quote_deterministic() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q1 = ironcli_quotes::select_quote(&quotes, 0);
    let q2 = ironcli_quotes::select_quote(&quotes, 0);
    assert_eq!(q1, q2);
}

#[test]
fn daily_quote_rotates() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q0 = ironcli_quotes::select_quote(&quotes, 0);
    let q1 = ironcli_quotes::select_quote(&quotes, 1);
    assert_ne!(q0, q1);
}

#[test]
fn daily_quote_wraps_around() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q0 = ironcli_quotes::select_quote(&quotes, 0);
    let q3 = ironcli_quotes::select_quote(&quotes, 3);
    assert_eq!(q0, q3);
}

#[test]
fn load_quotes_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quotes.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "Custom quote 1").unwrap();
    writeln!(f, "").unwrap(); // empty line should be skipped
    writeln!(f, "Custom quote 2").unwrap();

    let quotes = ironcli_quotes::load_quotes_file(&path);
    assert_eq!(quotes, Some(vec![
        "Custom quote 1".to_string(),
        "Custom quote 2".to_string(),
    ]));
}

#[test]
fn load_quotes_from_missing_file() {
    let path = std::path::Path::new("/tmp/nonexistent_ironcli_quotes.txt");
    let quotes = ironcli_quotes::load_quotes_file(path);
    assert_eq!(quotes, None);
}

// Module alias for test ergonomics
mod ironcli_quotes {
    use std::path::Path;

    pub fn builtin_quotes() -> Vec<String> {
        ironcli::tui::quotes::builtin_quotes()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn select_quote(quotes: &[String], day_of_year: u32) -> String {
        ironcli::tui::quotes::select_quote(
            &quotes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            day_of_year,
        ).to_string()
    }

    pub fn load_quotes_file(path: &Path) -> Option<Vec<String>> {
        ironcli::tui::quotes::load_quotes_file(path)
    }
}
```

Note: the tests reference `ironcli::tui::quotes`, so we need to re-export the tui module.

- [ ] **Step 2: Add `pub mod tui;` to lib.rs**

Update `src/lib.rs` to:

```rust
pub mod db;
pub mod export;
pub mod model;
pub mod tui;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test quotes_test 2>&1`
Expected: FAIL — module `quotes` does not exist

- [ ] **Step 4: Create the quotes module**

Create `src/tui/quotes.rs`:

```rust
use std::path::Path;

pub fn builtin_quotes() -> &'static [&'static str] {
    &[
        "The only bad workout is the one that didn't happen.",
        "Discipline is choosing between what you want now and what you want most.",
        "The pain you feel today will be the strength you feel tomorrow.",
        "Don't count the days, make the days count.",
        "Success isn't always about greatness. It's about consistency.",
        "The body achieves what the mind believes.",
        "Fall seven times, stand up eight.",
        "You don't have to be extreme, just consistent.",
        "Strength does not come from the body. It comes from the will.",
        "The hard days are what make you stronger.",
        "What seems impossible today will one day become your warm-up.",
        "Your body can stand almost anything. It's your mind that you have to convince.",
        "The clock is ticking. Are you becoming the person you want to be?",
        "No one is you, and that is your superpower.",
        "You are only one workout away from a good mood.",
        "It never gets easier. You just get stronger.",
        "Motivation is what gets you started. Habit is what keeps you going.",
        "The secret of getting ahead is getting started.",
        "Strive for progress, not perfection.",
        "A year from now, you'll wish you had started today.",
        "The difference between try and triumph is a little umph.",
        "Sweat is fat crying.",
        "Be stronger than your strongest excuse.",
        "If it doesn't challenge you, it doesn't change you.",
        "The only limit is the one you set yourself.",
        "Champions are made when nobody is watching.",
        "Push yourself because no one else is going to do it for you.",
        "Great things never come from comfort zones.",
        "Wake up with determination. Go to bed with satisfaction.",
        "Today I will do what others won't, so tomorrow I can do what others can't.",
    ]
}

pub fn select_quote<'a>(quotes: &'a [&'a str], day_of_year: u32) -> &'a str {
    if quotes.is_empty() {
        return "";
    }
    quotes[(day_of_year as usize) % quotes.len()]
}

pub fn load_quotes_file(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

pub fn get_daily_quote() -> String {
    let day_of_year = chrono::Local::now().ordinal();

    let quotes_path = dirs::home_dir()
        .map(|h| h.join(".ironcli").join("quotes.txt"));

    if let Some(ref path) = quotes_path {
        if let Some(custom) = load_quotes_file(path) {
            let refs: Vec<&str> = custom.iter().map(|s| s.as_str()).collect();
            return select_quote(&refs, day_of_year).to_string();
        }
    }

    let builtins = builtin_quotes();
    select_quote(builtins, day_of_year).to_string()
}
```

- [ ] **Step 5: Add `pub mod quotes;` to tui/mod.rs**

In `src/tui/mod.rs`, add after the existing module declarations:

```rust
pub mod quotes;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test quotes_test 2>&1`
Expected: All 6 tests pass

- [ ] **Step 7: Commit**

```bash
git add src/tui/quotes.rs src/tui/mod.rs src/lib.rs tests/quotes_test.rs
git commit -m "feat: add daily rotating quotes module with file override"
```

---

### Task 4: Merge statistics into Last 14 Days pane

**Files:**
- Modify: `src/tui/dashboard.rs`

- [ ] **Step 1: Replace render_stats_pane with merged summary in render_recent_pane**

In `src/tui/dashboard.rs`, replace the `render_recent_pane` method (lines 126-162) with:

```rust
    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Last 14 Days", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Summary line: "N sessions · Xkg · Y reps [· Z km] [· W min]"
        let mut parts: Vec<Span> = Vec::new();
        parts.push(Span::styled(
            format!("{} sessions", self.stats.sessions),
            Style::default().fg(GREEN),
        ));
        if self.stats.total_volume > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} kg", self.stats.total_volume),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_reps > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} reps", self.stats.total_reps),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_distance > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.1} km", self.stats.total_distance),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_duration > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} min", self.stats.total_duration),
                Style::default().fg(GREEN),
            ));
        }
        lines.push(Line::from(parts));

        // Separator
        let sep_width = inner.width as usize;
        lines.push(Line::from(Span::styled(
            "─".repeat(sep_width),
            Style::default().fg(Color::DarkGray),
        )));

        // Entry list
        if self.recent_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "No entries in the last 14 days",
                Style::default().fg(Color::Gray),
            )));
        } else {
            for entry in &self.recent_entries {
                let date = entry.log.logged_at.format("%b %d").to_string();
                let sets_count = entry.sets.len();
                let total = entry.total_metric();
                let label = entry.metric_label();
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", date), Style::default().fg(Color::Gray)),
                    Span::styled(&entry.practice_name, Style::default().fg(GREEN)),
                    Span::styled(
                        format!("  {} sets, {:.0} {}", sets_count, total, label),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
```

- [ ] **Step 2: Remove render_stats_pane method**

Delete the entire `render_stats_pane` method (lines 164-186) and the `stat_line` helper function (lines 200-205) from `src/tui/dashboard.rs`.

- [ ] **Step 3: Update render() to remove stats pane split**

In the `render` method, the panes split currently uses two 50% columns. For now, keep the split but render only the recent pane in the left column. The right column will be used for goals in the next task. Replace the panes rendering section:

Replace the call `self.render_stats_pane(frame, panes[1]);` (line 108) with nothing — just remove the line. We'll add the goals pane here in Task 6.

- [ ] **Step 4: Update pane height calculation**

The pane height now needs to account for the 2 extra lines (summary + separator). Update line 57:

```rust
let pane_height = (self.recent_entries.len() as u16 + 4).max(7); // +4: summary, separator, 2 borders
```

- [ ] **Step 5: Verify it compiles and renders**

Run: `cargo build 2>&1`
Expected: compiles with no errors

Run: `cargo run` and verify the dashboard shows the merged stats summary line at the top of the Last 14 Days pane.

- [ ] **Step 6: Commit**

```bash
git add src/tui/dashboard.rs
git commit -m "feat: merge statistics into Last 14 Days pane"
```

---

### Task 5: Add daily quote to dashboard layout

**Files:**
- Modify: `src/tui/dashboard.rs`

- [ ] **Step 1: Add quote to DashboardScreen state**

Add a `quote` field to the `DashboardScreen` struct:

```rust
pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
    quote: String,
}
```

- [ ] **Step 2: Initialize quote in new() and refresh()**

In `new()`, add after loading stats:

```rust
let quote = super::quotes::get_daily_quote();
```

And include it in the `Self { ... }` block:

```rust
Ok(Self {
    heatmap_data,
    recent_entries,
    stats,
    quote,
})
```

In `refresh()`, add:

```rust
self.quote = super::quotes::get_daily_quote();
```

- [ ] **Step 3: Add quote row to the vertical layout**

In the `render` method, update the layout constraints to insert a quote row between the heatmap and panes. Change the constraints array:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),            // title
        Constraint::Length(7),            // heatmap header ASCII art
        Constraint::Length(10),           // heatmap
        Constraint::Length(1),            // daily quote
        Constraint::Length(pane_height),  // split panes
        Constraint::Min(0),              // spacer
        Constraint::Length(2),            // footer
    ])
    .split(area);
```

- [ ] **Step 4: Render the quote**

Add the quote rendering after the heatmap section, before the panes. The quote goes in `chunks[3]`:

```rust
// ── Daily quote ──
let quote_area = Rect {
    x: chunks[3].x + 1,
    y: chunks[3].y,
    width: chunks[3].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
    height: chunks[3].height,
};
let quote_line = Line::from(Span::styled(
    format!("  \"{}\"", &self.quote),
    Style::default().fg(Color::Yellow),
));
frame.render_widget(Paragraph::new(quote_line), quote_area);
```

- [ ] **Step 5: Update chunk indices**

Since we added a new chunk, all indices after the quote shift by 1:
- Panes: `chunks[3]` → `chunks[4]`
- Footer: `chunks[5]` → `chunks[6]`

Update the panes area to use `chunks[4]`:

```rust
let panes_area = Rect {
    x: chunks[4].x + 1,
    y: chunks[4].y,
    width: chunks[4].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
    height: chunks[4].height,
};
```

Update the footer to use `chunks[6]`:

```rust
frame.render_widget(Paragraph::new(footer), chunks[6]);
```

- [ ] **Step 6: Verify it compiles and renders**

Run: `cargo build 2>&1`
Expected: compiles

Run: `cargo run` and verify a yellow quote appears between the heatmap and the panes.

- [ ] **Step 7: Commit**

```bash
git add src/tui/dashboard.rs
git commit -m "feat: add daily rotating quote to dashboard"
```

---

### Task 6: Add goals pane (passive display)

**Files:**
- Modify: `src/tui/dashboard.rs`

- [ ] **Step 1: Add goals data to DashboardScreen**

Add to the struct:

```rust
pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
    quote: String,
    goals: Vec<Goal>,
}
```

Add the import at the top of dashboard.rs:

```rust
use crate::model::{Goal, LogEntry};
```

- [ ] **Step 2: Load goals in new() and refresh()**

In `new()`:

```rust
let goals = db.list_goals()?;
```

Include in `Self { ... }`:

```rust
Ok(Self {
    heatmap_data,
    recent_entries,
    stats,
    quote,
    goals,
})
```

In `refresh()`:

```rust
self.goals = db.list_goals()?;
```

- [ ] **Step 3: Add render_goals_pane method**

Add after `render_recent_pane`:

```rust
    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Goals", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.goals.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                "Press [g] to add goals",
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in &self.goals {
            lines.push(Line::from(Span::styled(
                format!("▸ {}", goal.title),
                Style::default().fg(Color::White).bold(),
            )));
            for ms in &goal.milestones {
                if ms.completed {
                    lines.push(Line::from(Span::styled(
                        format!("  ☑ {}", ms.title),
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  ☐ {}", ms.title),
                        Style::default().fg(Color::White),
                    )));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
```

- [ ] **Step 4: Wire up goals pane in render()**

In the `render` method, replace the line where `render_stats_pane` was removed (or add after `render_recent_pane`):

```rust
self.render_recent_pane(frame, panes[0]);
self.render_goals_pane(frame, panes[1]);
```

- [ ] **Step 5: Update pane height to account for goals**

Update the pane height calculation to consider goals content too:

```rust
let goals_lines: u16 = self.goals.iter()
    .map(|g| 1 + g.milestones.len() as u16)
    .sum::<u16>()
    .max(1); // at least 1 for the hint text
let pane_height = (self.recent_entries.len() as u16 + 4)
    .max(goals_lines + 2) // +2 for borders
    .max(7);
```

- [ ] **Step 6: Verify it compiles and renders**

Run: `cargo build 2>&1`
Expected: compiles

Run: `cargo run` and verify the Goals pane shows "Press [g] to add goals" on the right side.

- [ ] **Step 7: Commit**

```bash
git add src/tui/dashboard.rs
git commit -m "feat: add goals pane with passive display on dashboard"
```

---

### Task 7: Add goals editing mode

**Files:**
- Modify: `src/tui/dashboard.rs`
- Modify: `src/app.rs`

This is the largest task. It adds keyboard interaction for creating, editing, navigating, toggling, and deleting goals and milestones.

- [ ] **Step 1: Add DashboardMode enum and state fields**

At the top of `src/tui/dashboard.rs` (after the constants), add:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardMode {
    Normal,
    Goals,
    AddGoal,
    AddMilestone,
    EditItem,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GoalItem {
    Goal(i64),
    Milestone(i64),
}
```

Add to `DashboardScreen`:

```rust
pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
    quote: String,
    goals: Vec<Goal>,
    mode: DashboardMode,
    goal_selected: usize,
    goal_input: String,
}
```

Initialize the new fields in `new()`:

```rust
mode: DashboardMode::Normal,
goal_selected: 0,
goal_input: String::new(),
```

And in `refresh()`, do NOT reset mode/selection (user stays in goals mode after DB operations).

- [ ] **Step 2: Build flat navigation list helper**

Add a helper method to build the flat list of GoalItems for navigation:

```rust
    fn goal_items(&self) -> Vec<GoalItem> {
        let mut items = Vec::new();
        for goal in &self.goals {
            items.push(GoalItem::Goal(goal.id));
            for ms in &goal.milestones {
                items.push(GoalItem::Milestone(ms.id));
            }
        }
        items
    }

    fn selected_goal_item(&self) -> Option<GoalItem> {
        let items = self.goal_items();
        items.get(self.goal_selected).copied()
    }

    fn parent_goal_id(&self) -> Option<i64> {
        match self.selected_goal_item()? {
            GoalItem::Goal(id) => Some(id),
            GoalItem::Milestone(ms_id) => {
                self.goals.iter()
                    .find(|g| g.milestones.iter().any(|m| m.id == ms_id))
                    .map(|g| g.id)
            }
        }
    }
```

- [ ] **Step 3: Update handle_key to accept &Database and route by mode**

Change the `handle_key` signature and body:

```rust
    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            DashboardMode::Normal => self.handle_normal(key),
            DashboardMode::Goals => self.handle_goals(key, db),
            DashboardMode::AddGoal => self.handle_add_goal(key, db),
            DashboardMode::AddMilestone => self.handle_add_milestone(key, db),
            DashboardMode::EditItem => self.handle_edit_item(key, db),
            DashboardMode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('g') => {
                self.mode = DashboardMode::Goals;
                self.goal_selected = 0;
                Action::None
            }
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        }
    }

    fn handle_goals(&mut self, key: KeyEvent, db: &Database) -> Action {
        let items = self.goal_items();
        let item_count = items.len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if item_count > 0 && self.goal_selected < item_count - 1 {
                    self.goal_selected += 1;
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.goal_selected > 0 {
                    self.goal_selected -= 1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.goal_input.clear();
                self.mode = DashboardMode::AddGoal;
                Action::None
            }
            KeyCode::Char('m') => {
                if self.parent_goal_id().is_some() {
                    self.goal_input.clear();
                    self.mode = DashboardMode::AddMilestone;
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_goal_item() {
                    let current_title = match item {
                        GoalItem::Goal(id) => {
                            self.goals.iter().find(|g| g.id == id).map(|g| g.title.clone())
                        }
                        GoalItem::Milestone(id) => {
                            self.goals.iter()
                                .flat_map(|g| &g.milestones)
                                .find(|m| m.id == id)
                                .map(|m| m.title.clone())
                        }
                    };
                    if let Some(title) = current_title {
                        self.goal_input = title;
                        self.mode = DashboardMode::EditItem;
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(GoalItem::Milestone(id)) = self.selected_goal_item() {
                    let _ = db.toggle_milestone(id);
                    let _ = self.reload_goals(db);
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if self.selected_goal_item().is_some() {
                    self.mode = DashboardMode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = DashboardMode::Normal;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_add_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    let _ = db.create_goal(&title);
                    let _ = self.reload_goals(db);
                }
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Backspace => {
                self.goal_input.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.goal_input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_add_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.parent_goal_id() {
                        let _ = db.create_milestone(goal_id, &title);
                        let _ = self.reload_goals(db);
                    }
                }
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Backspace => {
                self.goal_input.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.goal_input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_edit_item(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    if let Some(item) = self.selected_goal_item() {
                        match item {
                            GoalItem::Goal(id) => { let _ = db.update_goal(id, &title); }
                            GoalItem::Milestone(id) => { let _ = db.update_milestone(id, &title); }
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Backspace => {
                self.goal_input.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.goal_input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(item) = self.selected_goal_item() {
                    match item {
                        GoalItem::Goal(id) => { let _ = db.delete_goal(id); }
                        GoalItem::Milestone(id) => { let _ = db.delete_milestone(id); }
                    }
                    let _ = self.reload_goals(db);
                    let items = self.goal_items();
                    if self.goal_selected >= items.len() && !items.is_empty() {
                        self.goal_selected = items.len() - 1;
                    }
                }
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.mode = DashboardMode::Goals;
                Action::None
            }
        }
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        self.goals = db.list_goals()?;
        Ok(())
    }
```

- [ ] **Step 4: Update app.rs to pass db to dashboard.handle_key**

In `src/app.rs`, line 57, change:

```rust
Screen::Dashboard => dashboard.handle_key(key),
```

to:

```rust
Screen::Dashboard => dashboard.handle_key(key, db),
```

- [ ] **Step 5: Add Database import to dashboard.rs**

Add at the top of `src/tui/dashboard.rs`:

```rust
use crate::db::Database;
```

- [ ] **Step 6: Update render_goals_pane to show selection and input**

Replace the `render_goals_pane` method to handle all modes:

```rust
    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        let border_color = if self.mode != DashboardMode::Normal {
            ACCENT
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .title(Span::styled("Goals", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.goals.is_empty() && self.mode == DashboardMode::Normal {
            let hint = Paragraph::new(Line::from(Span::styled(
                "Press [g] to add goals",
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let items = self.goal_items();
        let in_goals_mode = self.mode != DashboardMode::Normal;

        let mut idx = 0;
        for goal in &self.goals {
            let is_selected = in_goals_mode && idx == self.goal_selected;
            let style = if is_selected {
                Style::default().fg(GREEN).bold()
            } else {
                Style::default().fg(Color::White).bold()
            };

            if is_selected && self.mode == DashboardMode::EditItem {
                lines.push(Line::from(vec![
                    Span::styled("▸ ", style),
                    Span::styled(&self.goal_input, Style::default().fg(GREEN)),
                    Span::styled("█", Style::default().fg(GREEN)),
                ]));
            } else {
                let marker = if is_selected { "> " } else { "▸ " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", marker, goal.title),
                    style,
                )));
            }
            idx += 1;

            for ms in &goal.milestones {
                let is_ms_selected = in_goals_mode && idx == self.goal_selected;

                if is_ms_selected && self.mode == DashboardMode::EditItem {
                    let check = if ms.completed { "☑ " } else { "☐ " };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", check), Style::default().fg(GREEN)),
                        Span::styled(&self.goal_input, Style::default().fg(GREEN)),
                        Span::styled("█", Style::default().fg(GREEN)),
                    ]));
                } else if ms.completed {
                    let style = if is_ms_selected {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  ☑ {}", ms.title),
                        style,
                    )));
                } else {
                    let style = if is_ms_selected {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  ☐ {}", ms.title),
                        style,
                    )));
                }
                idx += 1;
            }
        }

        // Input line for add modes
        match self.mode {
            DashboardMode::AddGoal => {
                lines.push(Line::from(vec![
                    Span::styled("▸ ", Style::default().fg(GREEN).bold()),
                    Span::styled(&self.goal_input, Style::default().fg(GREEN)),
                    Span::styled("█", Style::default().fg(GREEN)),
                ]));
            }
            DashboardMode::AddMilestone => {
                lines.push(Line::from(vec![
                    Span::styled("  ☐ ", Style::default().fg(GREEN)),
                    Span::styled(&self.goal_input, Style::default().fg(GREEN)),
                    Span::styled("█", Style::default().fg(GREEN)),
                ]));
            }
            DashboardMode::ConfirmDelete => {
                lines.push(Line::from(Span::styled(
                    "  Delete? (y/n)",
                    Style::default().fg(Color::Red),
                )));
            }
            _ => {}
        }

        if self.goals.is_empty() && self.mode == DashboardMode::Goals {
            lines.push(Line::from(Span::styled(
                "Press [a] to add a goal",
                Style::default().fg(Color::Gray),
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
```

- [ ] **Step 7: Update footer to show mode-specific keybindings**

Replace the footer rendering section in `render()`:

```rust
        // ── Footer ──
        let footer_spans = if self.mode == DashboardMode::Normal {
            vec![
                Span::styled(" [l]", Style::default().fg(ACCENT)),
                Span::styled(" Log  ", Style::default().fg(Color::Gray)),
                Span::styled("[h]", Style::default().fg(ACCENT)),
                Span::styled(" History  ", Style::default().fg(Color::Gray)),
                Span::styled("[t]", Style::default().fg(ACCENT)),
                Span::styled(" Trends  ", Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(" Practices  ", Style::default().fg(Color::Gray)),
                Span::styled("[g]", Style::default().fg(ACCENT)),
                Span::styled(" Goals  ", Style::default().fg(Color::Gray)),
                Span::styled("[q]", Style::default().fg(ACCENT)),
                Span::styled(" Quit", Style::default().fg(Color::Gray)),
            ]
        } else if self.mode == DashboardMode::Goals {
            vec![
                Span::styled(" [a]", Style::default().fg(ACCENT)),
                Span::styled(" Add goal  ", Style::default().fg(Color::Gray)),
                Span::styled("[m]", Style::default().fg(ACCENT)),
                Span::styled(" Milestone  ", Style::default().fg(Color::Gray)),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(" Edit  ", Style::default().fg(Color::Gray)),
                Span::styled("[Space]", Style::default().fg(ACCENT)),
                Span::styled(" Toggle  ", Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" Back", Style::default().fg(Color::Gray)),
            ]
        } else {
            vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(" Confirm  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" Cancel", Style::default().fg(Color::Gray)),
            ]
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[6]);
```

- [ ] **Step 8: Verify it compiles**

Run: `cargo build 2>&1`
Expected: compiles with no errors

- [ ] **Step 9: Manual testing**

Run: `cargo run` and verify:
1. Dashboard shows "Press [g] to add goals" in the right pane
2. Press `g` → Goals mode activates (border turns cyan, footer changes)
3. Press `a` → type a goal name → press Enter → goal appears with `▸` prefix
4. Navigate with `j/k` between goals and milestones
5. Press `m` on a goal → type milestone → Enter → milestone appears with `☐`
6. Press `Space` on a milestone → toggles `☐` ↔ `☑`
7. Press `Enter` on any item → edit its text
8. Press `d` → confirm with `y` → item deleted
9. Press `Esc` → back to normal dashboard mode
10. All existing keybindings (`l`, `h`, `t`, `e`, `q`) still work in normal mode

- [ ] **Step 10: Commit**

```bash
git add src/tui/dashboard.rs src/app.rs
git commit -m "feat: add goals editing mode with full CRUD on dashboard"
```

---

### Task 8: Add goals to export/import

**Files:**
- Modify: `src/export.rs`
- Modify: `tests/export_import_integration_test.rs`

- [ ] **Step 1: Write the failing test**

Add to the end of `tests/export_import_integration_test.rs`:

```rust
#[test]
fn goals_and_milestones_survive_export_import() {
    let source = TestDb::new();
    let db = &source.db;

    // Create goals with milestones
    let g1 = db.create_goal("Master KB Sport").unwrap();
    db.create_milestone(g1, "10-min snatch set").unwrap();
    let m2 = db.create_milestone(g1, "First competition").unwrap();
    db.toggle_milestone(m2).unwrap(); // mark completed

    let g2 = db.create_goal("Run a marathon").unwrap();
    db.create_milestone(g2, "Run 10km under 50min").unwrap();

    // Export
    let export_path = source.dir.path().join("export.json");
    ironcli::export::export_to_json(db, Some(export_path.clone())).unwrap();

    // Import into fresh DB
    let target = TestDb::new();
    ironcli::export::import_from_json(&target.db, &export_path).unwrap();

    // Verify goals
    let goals = target.db.list_goals().unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals[0].title, "Master KB Sport");
    assert_eq!(goals[0].milestones.len(), 2);
    assert_eq!(goals[0].milestones[0].title, "10-min snatch set");
    assert_eq!(goals[0].milestones[0].completed, false);
    assert_eq!(goals[0].milestones[1].title, "First competition");
    assert_eq!(goals[0].milestones[1].completed, true);
    assert_eq!(goals[1].title, "Run a marathon");
    assert_eq!(goals[1].milestones.len(), 1);
    assert_eq!(goals[1].milestones[0].title, "Run 10km under 50min");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test export_import_integration_test goals_and_milestones_survive_export_import 2>&1`
Expected: FAIL — export doesn't include goals yet

- [ ] **Step 3: Add export structs for goals**

Add to `src/export.rs` after the `ExportSet` struct (line 49):

```rust
#[derive(Serialize, Deserialize)]
pub struct ExportGoal {
    pub title: String,
    pub position: i32,
    pub milestones: Vec<ExportMilestone>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportMilestone {
    pub title: String,
    pub completed: bool,
    pub position: i32,
}
```

- [ ] **Step 4: Add goals field to ExportData**

Update the `ExportData` struct to include goals with a default for backward compatibility:

```rust
#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub practices: Vec<ExportPractice>,
    pub logs: Vec<ExportLog>,
    #[serde(default)]
    pub goals: Vec<ExportGoal>,
}
```

- [ ] **Step 5: Update export_to_json to include goals**

In `export_to_json`, after building `export_logs` (before constructing `ExportData`), add:

```rust
    let goals = db.list_goals()?;
    let export_goals: Vec<ExportGoal> = goals
        .iter()
        .map(|g| ExportGoal {
            title: g.title.clone(),
            position: g.position,
            milestones: g.milestones
                .iter()
                .map(|m| ExportMilestone {
                    title: m.title.clone(),
                    completed: m.completed,
                    position: m.position,
                })
                .collect(),
        })
        .collect();
```

Update the `ExportData` construction to version 2 and include goals:

```rust
    let data = ExportData {
        version: 2,
        exported_at: Local::now().to_rfc3339(),
        practices: export_practices,
        logs: export_logs,
        goals: export_goals,
    };
```

- [ ] **Step 6: Update import_from_json to import goals**

At the end of `import_from_json`, before `Ok(imported)`, add:

```rust
    // Import goals (clear existing, then re-create)
    if !data.goals.is_empty() {
        // Delete existing goals first
        let existing_goals = db.list_goals()?;
        for g in &existing_goals {
            db.delete_goal(g.id)?;
        }

        for eg in &data.goals {
            let goal_id = db.create_goal(&eg.title)?;
            for em in &eg.milestones {
                let ms_id = db.create_milestone(goal_id, &em.title)?;
                if em.completed {
                    db.toggle_milestone(ms_id)?;
                }
            }
        }
    }
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --test export_import_integration_test 2>&1`
Expected: All tests pass (6 total: 5 existing + 1 new)

Also run the full test suite:

Run: `cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src/export.rs tests/export_import_integration_test.rs
git commit -m "feat: add goals and milestones to export/import (version 2)"
```

---

### Task 9: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Read the current README**

Read `README.md` to understand its structure.

- [ ] **Step 2: Add goals documentation**

Add a section about goals keybindings and the daily quote feature. Include:
- Goals editing mode (`g` to enter, `Esc` to exit)
- Goal keybindings table (a, m, Enter, Space, d, j/k)
- Daily quote: mention `~/.ironcli/quotes.txt` override

- [ ] **Step 3: Verify README looks correct**

Read the updated README to verify formatting.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add goals and daily quote documentation to README"
```

---

### Task 10: Bump version

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Bump patch version**

Per the project convention (bump patch version for each bug fix / feature), update the version in `Cargo.toml`. Read the current version first and increment the patch number by 1.

- [ ] **Step 2: Build to update Cargo.lock**

Run: `cargo build 2>&1`

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.1.4"
```

---

### Task 11: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1`
Expected: No warnings

- [ ] **Step 3: Manual smoke test**

Run: `cargo run` and verify:
1. Dashboard shows ASCII art header, heatmap, daily quote (yellow), merged stats in left pane, goals pane on right
2. Goals mode works: add/edit/delete goals and milestones, toggle completion
3. All other screens (Log Entry, History, Trends, Practices) still work
4. Export includes goals, import restores them

- [ ] **Step 4: Commit any fixes**

If clippy or testing revealed issues, fix and commit.
