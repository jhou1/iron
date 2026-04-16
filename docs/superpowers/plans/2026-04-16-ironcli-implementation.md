# IronCLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a beautiful Rust TUI application for tracking training records with a GitHub-style heatmap, set-by-set logging, history browsing, and sparkline trend charts.

**Architecture:** Single Rust binary using ratatui for the terminal UI, rusqlite (bundled) for SQLite storage, and clap for CLI subcommands. The app has 5 screens (Dashboard, Log Entry, History, Trends, Practices) with vim-style navigation. Data flows through three SQLite tables: practices → logs → sets.

**Tech Stack:** Rust, ratatui, crossterm, rusqlite (bundled), clap (derive), chrono, serde, serde_json

**Spec:** `docs/superpowers/specs/2026-04-16-ironcli-design.md`

---

## File Map

```
Cargo.toml                    — project manifest with all dependencies
src/
  main.rs                     — clap CLI entry point, dispatches to TUI or export/import
  model.rs                    — PracticeType, Practice, Log, Set, SetData structs
  db.rs                       — Database struct: schema init, all CRUD queries
  app.rs                      — App struct: state machine, event loop, screen routing
  export.rs                   — ExportData struct, export/import JSON logic
  tui/
    mod.rs                    — Screen enum, shared rendering helpers
    dashboard.rs              — DashboardScreen: heatmap + today + 14-day stats
    log_entry.rs              — LogEntryScreen: practice picker + set-by-set form
    history.rs                — HistoryScreen: 14-day scrollable log with edit/delete
    trends.rs                 — TrendsScreen: practice picker + sparkline chart
    practices.rs              — PracticesScreen: inventory CRUD list
    widgets/
      mod.rs                  — widget module declarations
      heatmap.rs              — HeatmapWidget: GitHub-style year activity grid
      sparkline.rs            — SparklineWidget: vertical bar chart with labels
tests/
  db_test.rs                  — integration tests for database CRUD
  model_test.rs               — unit tests for derived metrics
  export_test.rs              — round-trip export/import tests
```

---

## Phase 1: Foundation (Sequential)

These tasks must be completed in order. They establish the shared types, database layer, and app scaffold that all screens depend on.

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize the Rust project**

```bash
cd /Users/jhou/Projects/ironcli
cargo init --name ironcli
```

- [ ] **Step 2: Replace Cargo.toml with full dependencies**

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "ironcli"
version = "0.1.0"
edition = "2021"
description = "A beautiful TUI for tracking training records"

[[bin]]
name = "iron"
path = "src/main.rs"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
rusqlite = { version = "0.32", features = ["bundled"] }
clap = { version = "4", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write the clap CLI entry point in main.rs**

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod app;
mod db;
mod export;
mod model;
mod tui;

#[derive(Parser)]
#[command(name = "iron", version, about = "Track your training")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export all data to JSON
    Export {
        /// Output file path (defaults to ~/.ironcli/iron-export-YYYY-MM-DD.json)
        path: Option<PathBuf>,
    },
    /// Import data from JSON
    Import {
        /// Input file path
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Export { path }) => {
            let db = db::Database::open_default()?;
            export::export_to_json(&db, path)?;
            println!("Export complete.");
        }
        Some(Commands::Import { path }) => {
            let db = db::Database::open_default()?;
            let count = export::import_from_json(&db, &path)?;
            println!("Imported {} logs.", count);
        }
        None => {
            app::run()?;
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Add anyhow dependency**

```bash
cd /Users/jhou/Projects/ironcli && cargo add anyhow
```

- [ ] **Step 5: Create stub modules so it compiles**

Create `src/model.rs`:
```rust
// Domain types — implemented in Task 2
```

Create `src/db.rs`:
```rust
// Database layer — implemented in Task 3
pub struct Database;

impl Database {
    pub fn open_default() -> anyhow::Result<Self> {
        todo!()
    }
}
```

Create `src/app.rs`:
```rust
// App state and event loop — implemented in Task 4
pub fn run() -> anyhow::Result<()> {
    todo!()
}
```

Create `src/export.rs`:
```rust
// Export/import logic — implemented in Task 11
use crate::db::Database;
use std::path::{Path, PathBuf};

pub fn export_to_json(_db: &Database, _path: Option<PathBuf>) -> anyhow::Result<()> {
    todo!()
}

pub fn import_from_json(_db: &Database, _path: &Path) -> anyhow::Result<usize> {
    todo!()
}
```

Create `src/tui/mod.rs`:
```rust
// TUI screens — implemented in Tasks 6-10
pub mod widgets;
```

Create `src/tui/widgets/mod.rs`:
```rust
// Custom widgets — implemented in Tasks 6, 9
```

- [ ] **Step 6: Update .gitignore**

```
/target
.superpowers/
```

- [ ] **Step 7: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

Expected: compiles with no errors (stubs use `todo!()`)

- [ ] **Step 8: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git init
git add Cargo.toml Cargo.lock src/ .gitignore docs/
git commit -m "feat: project scaffold with clap CLI, stub modules"
```

---

### Task 2: Domain Model

**Files:**
- Modify: `src/model.rs`
- Create: `tests/model_test.rs`

- [ ] **Step 1: Write tests for PracticeType and derived metrics**

Create `tests/model_test.rs`:

```rust
use ironcli::model::{PracticeType, SetData};

#[test]
fn practice_type_from_str() {
    assert_eq!("weighted".parse::<PracticeType>().unwrap(), PracticeType::Weighted);
    assert_eq!("bodyweight".parse::<PracticeType>().unwrap(), PracticeType::Bodyweight);
    assert_eq!("distance".parse::<PracticeType>().unwrap(), PracticeType::Distance);
    assert_eq!("endurance".parse::<PracticeType>().unwrap(), PracticeType::Endurance);
    assert!("invalid".parse::<PracticeType>().is_err());
}

#[test]
fn practice_type_display() {
    assert_eq!(PracticeType::Weighted.to_string(), "weighted");
    assert_eq!(PracticeType::Bodyweight.to_string(), "bodyweight");
    assert_eq!(PracticeType::Distance.to_string(), "distance");
    assert_eq!(PracticeType::Endurance.to_string(), "endurance");
}

#[test]
fn set_data_metric_weighted() {
    let set = SetData::Weighted { weight: 24.0, reps: 10 };
    assert_eq!(set.metric_value(), 240.0);
    assert_eq!(set.metric_label(), "kg vol");
}

#[test]
fn set_data_metric_bodyweight() {
    let set = SetData::Bodyweight { reps: 20 };
    assert_eq!(set.metric_value(), 20.0);
    assert_eq!(set.metric_label(), "reps");
}

#[test]
fn set_data_metric_distance() {
    let set = SetData::Distance { distance: 5.0 };
    assert_eq!(set.metric_value(), 5.0);
    assert_eq!(set.metric_label(), "km");
}

#[test]
fn set_data_metric_endurance() {
    let set = SetData::Endurance { duration: 30.0 };
    assert_eq!(set.metric_value(), 30.0);
    assert_eq!(set.metric_label(), "min");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test model_test
```

Expected: FAIL — `model` module has no types yet.

- [ ] **Step 3: Implement model.rs**

Replace `src/model.rs` with:

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PracticeType {
    Weighted,
    Bodyweight,
    Distance,
    Endurance,
}

impl PracticeType {
    pub const ALL: [PracticeType; 4] = [
        PracticeType::Weighted,
        PracticeType::Bodyweight,
        PracticeType::Distance,
        PracticeType::Endurance,
    ];
}

impl fmt::Display for PracticeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PracticeType::Weighted => write!(f, "weighted"),
            PracticeType::Bodyweight => write!(f, "bodyweight"),
            PracticeType::Distance => write!(f, "distance"),
            PracticeType::Endurance => write!(f, "endurance"),
        }
    }
}

impl FromStr for PracticeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "weighted" => Ok(PracticeType::Weighted),
            "bodyweight" => Ok(PracticeType::Bodyweight),
            "distance" => Ok(PracticeType::Distance),
            "endurance" => Ok(PracticeType::Endurance),
            other => Err(format!("unknown practice type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Practice {
    pub id: i64,
    pub name: String,
    pub practice_type: PracticeType,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: i64,
    pub practice_id: i64,
    pub logged_at: NaiveDateTime,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Set {
    pub id: i64,
    pub log_id: i64,
    pub set_number: i32,
    pub data: SetData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetData {
    Weighted { weight: f64, reps: i32 },
    Bodyweight { reps: i32 },
    Distance { distance: f64 },
    Endurance { duration: f64 },
}

impl SetData {
    /// Returns the derived metric value for this set.
    /// Weighted: weight * reps (volume). Bodyweight: reps. Distance: km. Endurance: min.
    pub fn metric_value(&self) -> f64 {
        match self {
            SetData::Weighted { weight, reps } => weight * (*reps as f64),
            SetData::Bodyweight { reps } => *reps as f64,
            SetData::Distance { distance } => *distance,
            SetData::Endurance { duration } => *duration,
        }
    }

    /// Returns the unit label for this set's derived metric.
    pub fn metric_label(&self) -> &'static str {
        match self {
            SetData::Weighted { .. } => "kg vol",
            SetData::Bodyweight { .. } => "reps",
            SetData::Distance { .. } => "km",
            SetData::Endurance { .. } => "min",
        }
    }
}

/// A log with its associated practice name, type, and sets — used for display.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub log: Log,
    pub practice_name: String,
    pub practice_type: PracticeType,
    pub sets: Vec<Set>,
}

impl LogEntry {
    /// Total derived metric across all sets in this log.
    pub fn total_metric(&self) -> f64 {
        self.sets.iter().map(|s| s.data.metric_value()).sum()
    }

    /// The metric label (same for all sets since they share a practice type).
    pub fn metric_label(&self) -> &'static str {
        self.sets
            .first()
            .map(|s| s.data.metric_label())
            .unwrap_or("—")
    }
}
```

- [ ] **Step 4: Make model.rs public from lib.rs**

Create `src/lib.rs`:

```rust
pub mod model;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test model_test
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/model.rs src/lib.rs tests/model_test.rs
git commit -m "feat: domain model with PracticeType, Practice, Log, Set, SetData"
```

---

### Task 3: Database Layer

**Files:**
- Modify: `src/db.rs`
- Create: `tests/db_test.rs`

- [ ] **Step 1: Write database integration tests**

Create `tests/db_test.rs`:

```rust
use ironcli::model::{PracticeType, SetData};

// We test through the public Database API.
// Each test gets a fresh in-memory database.
mod helpers {
    use ironcli::db::Database;

    pub fn test_db() -> Database {
        Database::open_in_memory().expect("failed to open test db")
    }
}

#[test]
fn create_and_list_practices() {
    let db = helpers::test_db();
    db.create_practice("Kettlebell Snatch", PracticeType::Weighted).unwrap();
    db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();

    let practices = db.list_practices().unwrap();
    assert_eq!(practices.len(), 2);
    assert_eq!(practices[0].name, "Kettlebell Snatch");
    assert_eq!(practices[0].practice_type, PracticeType::Weighted);
    assert_eq!(practices[1].name, "Push Up");
}

#[test]
fn create_practice_duplicate_name_fails() {
    let db = helpers::test_db();
    db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    let result = db.create_practice("Push Up", PracticeType::Bodyweight);
    assert!(result.is_err());
}

#[test]
fn rename_practice() {
    let db = helpers::test_db();
    let id = db.create_practice("Pushup", PracticeType::Bodyweight).unwrap();
    db.rename_practice(id, "Push Up").unwrap();
    let practices = db.list_practices().unwrap();
    assert_eq!(practices[0].name, "Push Up");
}

#[test]
fn delete_practice() {
    let db = helpers::test_db();
    let id = db.create_practice("Test", PracticeType::Bodyweight).unwrap();
    db.delete_practice(id).unwrap();
    assert_eq!(db.list_practices().unwrap().len(), 0);
}

#[test]
fn create_log_with_sets() {
    let db = helpers::test_db();
    let practice_id = db.create_practice("KB Snatch", PracticeType::Weighted).unwrap();

    let sets = vec![
        SetData::Weighted { weight: 24.0, reps: 10 },
        SetData::Weighted { weight: 24.0, reps: 9 },
        SetData::Weighted { weight: 28.0, reps: 8 },
    ];
    let log_id = db.create_log(practice_id, &sets, Some("Felt strong")).unwrap();

    let entries = db.list_logs_recent(14).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].log.id, log_id);
    assert_eq!(entries[0].practice_name, "KB Snatch");
    assert_eq!(entries[0].sets.len(), 3);
    assert_eq!(entries[0].log.note.as_deref(), Some("Felt strong"));
    assert_eq!(entries[0].sets[0].set_number, 1);
    assert_eq!(entries[0].sets[2].set_number, 3);
}

#[test]
fn update_log_sets_and_note() {
    let db = helpers::test_db();
    let pid = db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    let sets = vec![SetData::Bodyweight { reps: 20 }];
    let log_id = db.create_log(pid, &sets, None).unwrap();

    let new_sets = vec![
        SetData::Bodyweight { reps: 20 },
        SetData::Bodyweight { reps: 18 },
    ];
    db.update_log(log_id, &new_sets, Some("Added a set")).unwrap();

    let entries = db.list_logs_recent(14).unwrap();
    assert_eq!(entries[0].sets.len(), 2);
    assert_eq!(entries[0].log.note.as_deref(), Some("Added a set"));
}

#[test]
fn delete_log() {
    let db = helpers::test_db();
    let pid = db.create_practice("Running", PracticeType::Distance).unwrap();
    let sets = vec![SetData::Distance { distance: 5.0 }];
    let log_id = db.create_log(pid, &sets, None).unwrap();

    db.delete_log(log_id).unwrap();
    assert_eq!(db.list_logs_recent(14).unwrap().len(), 0);
}

#[test]
fn heatmap_data() {
    let db = helpers::test_db();
    let pid = db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    let sets = vec![SetData::Bodyweight { reps: 10 }];
    db.create_log(pid, &sets, None).unwrap();
    db.create_log(pid, &sets, None).unwrap();

    let data = db.heatmap_counts(365).unwrap();
    // Today should have 2 entries
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let count = data.iter().find(|(d, _)| d == &today).map(|(_, c)| *c).unwrap_or(0);
    assert_eq!(count, 2);
}

#[test]
fn logs_for_practice_trend() {
    let db = helpers::test_db();
    let pid = db.create_practice("KB Snatch", PracticeType::Weighted).unwrap();

    let sets1 = vec![SetData::Weighted { weight: 24.0, reps: 10 }];
    let sets2 = vec![SetData::Weighted { weight: 24.0, reps: 10 }, SetData::Weighted { weight: 24.0, reps: 9 }];
    db.create_log(pid, &sets1, None).unwrap();
    db.create_log(pid, &sets2, None).unwrap();

    let entries = db.list_logs_for_practice(pid, 90).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn fourteen_day_stats() {
    let db = helpers::test_db();
    let p1 = db.create_practice("KB Snatch", PracticeType::Weighted).unwrap();
    let p2 = db.create_practice("Running", PracticeType::Distance).unwrap();

    db.create_log(p1, &[SetData::Weighted { weight: 24.0, reps: 10 }], None).unwrap();
    db.create_log(p2, &[SetData::Distance { distance: 5.0 }], None).unwrap();

    let stats = db.aggregate_stats(14).unwrap();
    assert_eq!(stats.sessions, 2);
    assert_eq!(stats.total_volume, 240.0);
    assert_eq!(stats.total_distance, 5.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test db_test
```

Expected: FAIL — `Database` has no methods yet.

- [ ] **Step 3: Implement db.rs**

Replace `src/db.rs` with:

```rust
use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::model::*;

pub struct Database {
    conn: Connection,
}

/// Aggregated stats for a time window.
pub struct AggregateStats {
    pub sessions: i64,
    pub total_volume: f64,
    pub total_reps: f64,
    pub total_distance: f64,
    pub total_duration: f64,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        let dir = dirs::home_dir()
            .context("cannot find home directory")?
            .join(".ironcli");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("iron.db");
        Self::open(&path)
    }

    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS practices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                practice_type TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                practice_id INTEGER NOT NULL REFERENCES practices(id),
                logged_at TEXT NOT NULL,
                note TEXT
            );
            CREATE TABLE IF NOT EXISTS sets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                log_id INTEGER NOT NULL REFERENCES logs(id) ON DELETE CASCADE,
                set_number INTEGER NOT NULL,
                weight REAL,
                reps INTEGER,
                distance REAL,
                duration REAL
            );
            PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    }

    // --- Practices CRUD ---

    pub fn create_practice(&self, name: &str, practice_type: PracticeType) -> Result<i64> {
        let now = Local::now().naive_local();
        self.conn.execute(
            "INSERT INTO practices (name, practice_type, created_at) VALUES (?1, ?2, ?3)",
            params![name, practice_type.to_string(), now.to_string()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_practices(&self) -> Result<Vec<Practice>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, practice_type, created_at FROM practices ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let pt_str: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            Ok(Practice {
                id: row.get(0)?,
                name: row.get(1)?,
                practice_type: pt_str.parse().unwrap_or(PracticeType::Bodyweight),
                created_at: NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap_or_default(),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn rename_practice(&self, id: i64, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE practices SET name = ?1 WHERE id = ?2",
            params![new_name, id],
        )?;
        Ok(())
    }

    pub fn delete_practice(&self, id: i64) -> Result<()> {
        // Delete associated logs and sets first
        let log_ids: Vec<i64> = {
            let mut stmt = self.conn.prepare("SELECT id FROM logs WHERE practice_id = ?1")?;
            stmt.query_map(params![id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for log_id in log_ids {
            self.conn.execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        }
        self.conn.execute("DELETE FROM logs WHERE practice_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM practices WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Logs CRUD ---

    pub fn create_log(
        &self,
        practice_id: i64,
        sets: &[SetData],
        note: Option<&str>,
    ) -> Result<i64> {
        let now = Local::now().naive_local();
        self.conn.execute(
            "INSERT INTO logs (practice_id, logged_at, note) VALUES (?1, ?2, ?3)",
            params![practice_id, now.to_string(), note],
        )?;
        let log_id = self.conn.last_insert_rowid();
        self.insert_sets(log_id, sets)?;
        Ok(log_id)
    }

    pub fn update_log(
        &self,
        log_id: i64,
        sets: &[SetData],
        note: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE logs SET note = ?1 WHERE id = ?2",
            params![note, log_id],
        )?;
        self.conn.execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        self.insert_sets(log_id, sets)?;
        Ok(())
    }

    pub fn delete_log(&self, log_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        self.conn.execute("DELETE FROM logs WHERE id = ?1", params![log_id])?;
        Ok(())
    }

    fn insert_sets(&self, log_id: i64, sets: &[SetData]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO sets (log_id, set_number, weight, reps, distance, duration)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (i, set) in sets.iter().enumerate() {
            let (weight, reps, distance, duration) = match set {
                SetData::Weighted { weight, reps } => {
                    (Some(*weight), Some(*reps), None, None)
                }
                SetData::Bodyweight { reps } => (None, Some(*reps), None, None),
                SetData::Distance { distance } => (None, None, Some(*distance), None),
                SetData::Endurance { duration } => (None, None, None, Some(*duration)),
            };
            stmt.execute(params![
                log_id,
                (i + 1) as i32,
                weight,
                reps,
                distance,
                duration
            ])?;
        }
        Ok(())
    }

    // --- Query helpers ---

    pub fn list_logs_recent(&self, days: i64) -> Result<Vec<LogEntry>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON p.id = l.practice_id
             WHERE l.logged_at >= ?1
             ORDER BY l.logged_at DESC",
        )?;
        let logs: Vec<(Log, String, PracticeType)> = stmt
            .query_map(params![cutoff.to_string()], |row| {
                let pt_str: String = row.get(5)?;
                Ok((
                    Log {
                        id: row.get(0)?,
                        practice_id: row.get(1)?,
                        logged_at: {
                            let s: String = row.get(2)?;
                            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                                .unwrap_or_default()
                        },
                        note: row.get(3)?,
                    },
                    row.get::<_, String>(4)?,
                    pt_str.parse().unwrap_or(PracticeType::Bodyweight),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut entries = Vec::new();
        for (log, practice_name, practice_type) in logs {
            let sets = self.get_sets_for_log(log.id, &practice_type)?;
            entries.push(LogEntry {
                log,
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    pub fn list_logs_for_practice(
        &self,
        practice_id: i64,
        days: i64,
    ) -> Result<Vec<LogEntry>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON p.id = l.practice_id
             WHERE l.practice_id = ?1 AND l.logged_at >= ?2
             ORDER BY l.logged_at ASC",
        )?;
        let logs: Vec<(Log, String, PracticeType)> = stmt
            .query_map(params![practice_id, cutoff.to_string()], |row| {
                let pt_str: String = row.get(5)?;
                Ok((
                    Log {
                        id: row.get(0)?,
                        practice_id: row.get(1)?,
                        logged_at: {
                            let s: String = row.get(2)?;
                            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                                .unwrap_or_default()
                        },
                        note: row.get(3)?,
                    },
                    row.get::<_, String>(4)?,
                    pt_str.parse().unwrap_or(PracticeType::Bodyweight),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut entries = Vec::new();
        for (log, practice_name, practice_type) in logs {
            let sets = self.get_sets_for_log(log.id, &practice_type)?;
            entries.push(LogEntry {
                log,
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    fn get_sets_for_log(&self, log_id: i64, practice_type: &PracticeType) -> Result<Vec<Set>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, log_id, set_number, weight, reps, distance, duration
             FROM sets WHERE log_id = ?1 ORDER BY set_number",
        )?;
        let sets = stmt
            .query_map(params![log_id], |row| {
                let data = match practice_type {
                    PracticeType::Weighted => SetData::Weighted {
                        weight: row.get::<_, f64>(3).unwrap_or(0.0),
                        reps: row.get::<_, i32>(4).unwrap_or(0),
                    },
                    PracticeType::Bodyweight => SetData::Bodyweight {
                        reps: row.get::<_, i32>(4).unwrap_or(0),
                    },
                    PracticeType::Distance => SetData::Distance {
                        distance: row.get::<_, f64>(5).unwrap_or(0.0),
                    },
                    PracticeType::Endurance => SetData::Endurance {
                        duration: row.get::<_, f64>(6).unwrap_or(0.0),
                    },
                };
                Ok(Set {
                    id: row.get(0)?,
                    log_id: row.get(1)?,
                    set_number: row.get(2)?,
                    data,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sets)
    }

    pub fn heatmap_counts(&self, days: i64) -> Result<Vec<(String, i64)>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT DATE(logged_at) as day, COUNT(*) as cnt
             FROM logs
             WHERE logged_at >= ?1
             GROUP BY day
             ORDER BY day",
        )?;
        let rows = stmt
            .query_map(params![cutoff.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn aggregate_stats(&self, days: i64) -> Result<AggregateStats> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let sessions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM logs WHERE logged_at >= ?1",
            params![cutoff.to_string()],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT p.practice_type, s.weight, s.reps, s.distance, s.duration
             FROM sets s
             JOIN logs l ON l.id = s.log_id
             JOIN practices p ON p.id = l.practice_id
             WHERE l.logged_at >= ?1",
        )?;

        let mut total_volume = 0.0;
        let mut total_reps = 0.0;
        let mut total_distance = 0.0;
        let mut total_duration = 0.0;

        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f64>>(1)?,
                row.get::<_, Option<i32>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
            ))
        })?;

        for row in rows {
            let (pt, weight, reps, distance, duration) = row?;
            match pt.as_str() {
                "weighted" => {
                    total_volume += weight.unwrap_or(0.0) * reps.unwrap_or(0) as f64;
                }
                "bodyweight" => {
                    total_reps += reps.unwrap_or(0) as f64;
                }
                "distance" => {
                    total_distance += distance.unwrap_or(0.0);
                }
                "endurance" => {
                    total_duration += duration.unwrap_or(0.0);
                }
                _ => {}
            }
        }

        Ok(AggregateStats {
            sessions,
            total_volume,
            total_reps,
            total_distance,
            total_duration,
        })
    }

    /// Returns all practices and logs with sets for export.
    pub fn export_all(&self) -> Result<(Vec<Practice>, Vec<LogEntry>)> {
        let practices = self.list_practices()?;
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON p.id = l.practice_id
             ORDER BY l.logged_at ASC",
        )?;
        let logs: Vec<(Log, String, PracticeType)> = stmt
            .query_map([], |row| {
                let pt_str: String = row.get(5)?;
                Ok((
                    Log {
                        id: row.get(0)?,
                        practice_id: row.get(1)?,
                        logged_at: {
                            let s: String = row.get(2)?;
                            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                                .unwrap_or_default()
                        },
                        note: row.get(3)?,
                    },
                    row.get::<_, String>(4)?,
                    pt_str.parse().unwrap_or(PracticeType::Bodyweight),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut entries = Vec::new();
        for (log, practice_name, practice_type) in logs {
            let sets = self.get_sets_for_log(log.id, &practice_type)?;
            entries.push(LogEntry {
                log,
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok((practices, entries))
    }

    /// Checks if a log with the same practice name and timestamp already exists.
    pub fn log_exists(&self, practice_name: &str, logged_at: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM logs l
             JOIN practices p ON p.id = l.practice_id
             WHERE p.name = ?1 AND l.logged_at = ?2",
            params![practice_name, logged_at],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
```

- [ ] **Step 4: Add db module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod db;
pub mod model;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test db_test
```

Expected: all 9 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/db.rs src/lib.rs tests/db_test.rs
git commit -m "feat: database layer with full CRUD, heatmap counts, aggregate stats"
```

---

### Task 4: App Scaffold & Event Loop

**Files:**
- Modify: `src/app.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Implement the Screen enum and shared types in tui/mod.rs**

Replace `src/tui/mod.rs` with:

```rust
pub mod dashboard;
pub mod history;
pub mod log_entry;
pub mod practices;
pub mod trends;
pub mod widgets;

/// Represents which screen the app is currently showing.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
}

/// Actions that a screen can request the app to perform.
pub enum Action {
    None,
    Navigate(Screen),
    Quit,
}
```

- [ ] **Step 2: Create stub screen modules**

Create `src/tui/dashboard.rs`:
```rust
use ratatui::Frame;
use crate::db::Database;
use super::Action;

pub struct DashboardScreen;

impl DashboardScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn refresh(&mut self, _db: &Database) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> Action {
        Action::None
    }
}
```

Create `src/tui/log_entry.rs`:
```rust
use ratatui::Frame;
use crate::db::Database;
use super::Action;

pub struct LogEntryScreen;

impl LogEntryScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
```

Create `src/tui/history.rs`:
```rust
use ratatui::Frame;
use crate::db::Database;
use super::Action;

pub struct HistoryScreen;

impl HistoryScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
```

Create `src/tui/trends.rs`:
```rust
use ratatui::Frame;
use crate::db::Database;
use super::Action;

pub struct TrendsScreen;

impl TrendsScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> Action {
        Action::None
    }
}
```

Create `src/tui/practices.rs`:
```rust
use ratatui::Frame;
use crate::db::Database;
use super::Action;

pub struct PracticesScreen;

impl PracticesScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
```

- [ ] **Step 3: Implement app.rs with event loop and screen routing**

Replace `src/app.rs` with:

```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

use crate::db::Database;
use crate::tui::{
    dashboard::DashboardScreen,
    history::HistoryScreen,
    log_entry::LogEntryScreen,
    practices::PracticesScreen,
    trends::TrendsScreen,
    Action, Screen,
};

pub fn run() -> Result<()> {
    let db = Database::open_default()?;

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run_app(&mut terminal, &db);

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, db: &Database) -> Result<()> {
    let mut current_screen = Screen::Dashboard;
    let mut dashboard = DashboardScreen::new(db)?;
    let mut log_entry = LogEntryScreen::new(db)?;
    let mut history = HistoryScreen::new(db)?;
    let mut trends = TrendsScreen::new(db)?;
    let mut practices = PracticesScreen::new(db)?;

    loop {
        terminal.draw(|frame| match current_screen {
            Screen::Dashboard => dashboard.render(frame),
            Screen::LogEntry => log_entry.render(frame),
            Screen::History => history.render(frame),
            Screen::Trends => trends.render(frame),
            Screen::Practices => practices.render(frame),
        })?;

        if let Event::Key(key) = event::read()? {
            // Global quit: Ctrl+C always exits
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }

            let action = match current_screen {
                Screen::Dashboard => dashboard.handle_key(key),
                Screen::LogEntry => log_entry.handle_key(key, db),
                Screen::History => history.handle_key(key, db),
                Screen::Trends => trends.handle_key(key),
                Screen::Practices => practices.handle_key(key, db),
            };

            match action {
                Action::Quit => break,
                Action::Navigate(screen) => {
                    // Refresh the target screen's data before showing it
                    match &screen {
                        Screen::Dashboard => dashboard.refresh(db)?,
                        Screen::LogEntry => log_entry = LogEntryScreen::new(db)?,
                        Screen::History => history = HistoryScreen::new(db)?,
                        Screen::Trends => trends = TrendsScreen::new(db)?,
                        Screen::Practices => practices = PracticesScreen::new(db)?,
                    }
                    current_screen = screen;
                }
                Action::None => {}
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

Expected: compiles with warnings about unused variables (the stub screens don't use their params yet).

- [ ] **Step 5: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/app.rs src/tui/
git commit -m "feat: app event loop with screen routing and stub screens"
```

---

## Phase 2: Parallel Components

These tasks are independent and can be developed by separate subagents simultaneously. Each task builds one screen or component against the interfaces established in Phase 1. Every task should be preceded by reading the spec at `docs/superpowers/specs/2026-04-16-ironcli-design.md` for full context.

---

### Task 5: Heatmap Widget

**Files:**
- Create: `src/tui/widgets/heatmap.rs`
- Modify: `src/tui/widgets/mod.rs`

- [ ] **Step 1: Add heatmap module to widgets/mod.rs**

Update `src/tui/widgets/mod.rs`:

```rust
pub mod heatmap;
pub mod sparkline;
```

Create stub `src/tui/widgets/sparkline.rs`:
```rust
// Implemented in Task 9
```

- [ ] **Step 2: Implement heatmap widget**

Create `src/tui/widgets/heatmap.rs`:

```rust
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use std::collections::HashMap;

/// GitHub-style contribution heatmap widget.
pub struct Heatmap {
    /// Map of "YYYY-MM-DD" → count of sessions that day.
    counts: HashMap<String, i64>,
    /// Number of weeks to display.
    weeks: u16,
}

impl Heatmap {
    pub fn new(data: &[(String, i64)], weeks: u16) -> Self {
        let counts: HashMap<String, i64> = data.iter().cloned().collect();
        Self { counts, weeks }
    }

    fn color_for_count(count: i64) -> Color {
        match count {
            0 => Color::Rgb(30, 30, 58),       // empty — dark slate
            1 => Color::Rgb(45, 90, 45),        // light green
            2 => Color::Rgb(61, 139, 61),       // medium green
            _ => Color::Rgb(78, 202, 78),       // bright green
        }
    }
}

impl Widget for Heatmap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 9 || area.width < 10 {
            return;
        }

        let today = Local::now().date_naive();
        let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let label_width: u16 = 4;

        // Render day-of-week labels
        for (i, label) in day_labels.iter().enumerate() {
            let y = area.y + i as u16;
            if y < area.y + area.height - 1 {
                buf.set_string(area.x, y, label, Style::default().fg(Color::DarkGray));
            }
        }

        // Calculate start date: go back `weeks` weeks from today, align to Monday
        let today_weekday_offset = today.weekday().num_days_from_monday();
        let start = today
            - Duration::days(today_weekday_offset as i64)
            - Duration::weeks((self.weeks - 1) as i64);

        // Render cells
        let cell_width: u16 = 2; // "█ " per cell
        let max_cols = ((area.width - label_width) / cell_width).min(self.weeks);

        for week in 0..max_cols {
            for day in 0..7u16 {
                let date = start + Duration::days((week * 7 + day) as i64);
                if date > today {
                    continue;
                }
                let date_str = date.format("%Y-%m-%d").to_string();
                let count = self.counts.get(&date_str).copied().unwrap_or(0);
                let color = Self::color_for_count(count);

                let x = area.x + label_width + week * cell_width;
                let y = area.y + day;

                if y < area.y + area.height - 1 && x + 1 < area.x + area.width {
                    buf.set_string(x, y, "█", Style::default().fg(color));
                }
            }
        }

        // Legend row
        let legend_y = area.y + 7;
        if legend_y < area.y + area.height {
            let legend_x = area.x + label_width;
            buf.set_string(legend_x, legend_y, "Less ", Style::default().fg(Color::DarkGray));
            let mut x = legend_x + 5;
            for count in [0, 1, 2, 3] {
                let color = Self::color_for_count(count);
                buf.set_string(x, legend_y, "█ ", Style::default().fg(color));
                x += 2;
            }
            buf.set_string(x, legend_y, "More", Style::default().fg(Color::DarkGray));
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 4: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/widgets/
git commit -m "feat: GitHub-style heatmap widget with green intensity levels"
```

---

### Task 6: Dashboard Screen

**Files:**
- Modify: `src/tui/dashboard.rs`

Depends on: Task 5 (heatmap widget)

- [ ] **Step 1: Implement dashboard screen**

Replace `src/tui/dashboard.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::{AggregateStats, Database};
use crate::model::LogEntry;
use super::widgets::heatmap::Heatmap;
use super::Action;
use super::Screen;

pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    today_entries: Vec<LogEntry>,
    stats: AggregateStats,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let all_recent = db.list_logs_recent(0)?; // today only — 0 means "today"
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_entries: Vec<LogEntry> = db
            .list_logs_recent(1)?
            .into_iter()
            .filter(|e| e.log.logged_at.format("%Y-%m-%d").to_string() == today)
            .collect();
        let stats = db.aggregate_stats(14)?;

        Ok(Self {
            heatmap_data,
            today_entries,
            stats,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.today_entries = db
            .list_logs_recent(1)?
            .into_iter()
            .filter(|e| e.log.logged_at.format("%Y-%m-%d").to_string() == today)
            .collect();
        self.stats = db.aggregate_stats(14)?;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // title
                Constraint::Length(9),  // heatmap
                Constraint::Min(6),    // today + stats
                Constraint::Length(2), // shortcuts
            ])
            .split(size);

        // Title
        let title = Line::from(vec![
            Span::styled("iron", Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD)),
            Span::styled(" v0.1.0", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Heatmap
        let heatmap_title = Line::from(Span::styled(
            "Training Activity",
            Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(heatmap_title), Rect { height: 1, ..chunks[1] });

        let heatmap_area = Rect {
            y: chunks[1].y + 1,
            height: chunks[1].height.saturating_sub(1),
            ..chunks[1]
        };
        let weeks = (heatmap_area.width.saturating_sub(4)) / 2;
        let heatmap = Heatmap::new(&self.heatmap_data, weeks.min(52));
        frame.render_widget(heatmap, heatmap_area);

        // Bottom panes: Today (left) + 14-day stats (right)
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Today's training
        let today_str = chrono::Local::now().format("%b %d, %Y").to_string();
        let mut today_lines: Vec<Line> = Vec::new();
        if self.today_entries.is_empty() {
            today_lines.push(Line::from(Span::styled(
                "  No training logged yet",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for entry in &self.today_entries {
                let summary = format!(
                    "  {}  {} sets  {:.0} {}",
                    entry.practice_name,
                    entry.sets.len(),
                    entry.total_metric(),
                    entry.metric_label(),
                );
                today_lines.push(Line::from(Span::styled(
                    summary,
                    Style::default().fg(Color::Rgb(78, 202, 78)),
                )));
            }
        }
        let today_block = Block::default()
            .title(format!(" Today — {} ", today_str))
            .title_style(Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(today_lines).block(today_block), bottom[0]);

        // 14-day stats
        let stats_lines = vec![
            Line::from(vec![
                Span::styled("  Sessions:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", self.stats.sessions), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Volume:    ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.0}kg", self.stats.total_volume), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Reps:      ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.0}", self.stats.total_reps), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Distance:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.1}km", self.stats.total_distance), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Duration:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.0}min", self.stats.total_duration), Style::default().fg(Color::White)),
            ]),
        ];
        let stats_block = Block::default()
            .title(" Last 14 Days ")
            .title_style(Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(stats_lines).block(stats_block), bottom[1]);

        // Keyboard shortcuts
        let shortcuts = Line::from(vec![
            Span::styled(" [l]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Log  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[h]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" History  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[t]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Trends  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[e]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Practices  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[q]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 3: Run the app to visually verify the dashboard**

```bash
cd /Users/jhou/Projects/ironcli && cargo run
```

Expected: the app launches showing the dashboard with an empty heatmap, empty today pane, zeroed stats, and keyboard shortcuts at the bottom. Press `q` to quit.

- [ ] **Step 4: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/dashboard.rs
git commit -m "feat: dashboard screen with heatmap, today's log, and 14-day stats"
```

---

### Task 7: Log Entry Screen

**Files:**
- Modify: `src/tui/log_entry.rs`

- [ ] **Step 1: Implement log entry screen with practice picker and set-by-set form**

Replace `src/tui/log_entry.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::Database;
use crate::model::*;
use super::{Action, Screen};

#[derive(Debug, PartialEq)]
enum Phase {
    SelectPractice,
    EnterSets,
    EnterNote,
}

pub struct LogEntryScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filtering: bool,
    selected: usize,
    phase: Phase,
    // Set entry state
    chosen_practice: Option<Practice>,
    sets: Vec<SetData>,
    // Current field inputs (as strings for editing)
    field1: String, // weight or distance or duration or reps
    field2: String, // reps (for weighted only)
    active_field: usize, // 0 = field1, 1 = field2
    // Note
    note: String,
}

impl LogEntryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::SelectPractice,
            chosen_practice: None,
            sets: Vec::new(),
            field1: String::new(),
            field2: String::new(),
            active_field: 0,
            note: String::new(),
        })
    }

    fn apply_filter(&mut self) {
        let query = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    fn field_labels(&self) -> (&str, Option<&str>) {
        match self.chosen_practice.as_ref().map(|p| p.practice_type) {
            Some(PracticeType::Weighted) => ("Weight (kg)", Some("Reps")),
            Some(PracticeType::Bodyweight) => ("Reps", None),
            Some(PracticeType::Distance) => ("Distance (km)", None),
            Some(PracticeType::Endurance) => ("Duration (min)", None),
            None => ("", None),
        }
    }

    fn has_second_field(&self) -> bool {
        self.chosen_practice
            .as_ref()
            .map(|p| p.practice_type == PracticeType::Weighted)
            .unwrap_or(false)
    }

    fn last_field1_value(&self) -> Option<String> {
        self.sets.last().and_then(|s| match s {
            SetData::Weighted { weight, .. } => Some(format!("{}", weight)),
            _ => None,
        })
    }

    fn commit_set(&mut self) -> bool {
        let pt = match &self.chosen_practice {
            Some(p) => p.practice_type,
            None => return false,
        };

        // Use last weight if field1 is empty (carry-forward for weighted)
        let f1 = if self.field1.is_empty() {
            match self.last_field1_value() {
                Some(v) => v,
                None => return false,
            }
        } else {
            self.field1.clone()
        };

        let set = match pt {
            PracticeType::Weighted => {
                let weight: f64 = match f1.parse() {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                let reps: i32 = match self.field2.parse() {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                SetData::Weighted { weight, reps }
            }
            PracticeType::Bodyweight => {
                let reps: i32 = match f1.parse() {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                SetData::Bodyweight { reps }
            }
            PracticeType::Distance => {
                let distance: f64 = match f1.parse() {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                SetData::Distance { distance }
            }
            PracticeType::Endurance => {
                let duration: f64 = match f1.parse() {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                SetData::Endurance { duration }
            }
        };

        self.sets.push(set);
        // Keep field1 for carry-forward (weighted), clear field2
        if pt != PracticeType::Weighted {
            self.field1.clear();
        }
        self.field2.clear();
        self.active_field = if pt == PracticeType::Weighted { 1 } else { 0 };
        true
    }

    fn running_total(&self) -> f64 {
        self.sets.iter().map(|s| s.metric_value()).sum()
    }

    pub fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        match self.phase {
            Phase::SelectPractice => self.render_practice_picker(frame, size),
            Phase::EnterSets => self.render_set_entry(frame, size),
            Phase::EnterNote => self.render_note_entry(frame, size),
        }
    }

    fn render_practice_picker(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Length(2),  // filter
                Constraint::Min(4),    // list
                Constraint::Length(2), // shortcuts
            ])
            .split(area);

        let title = Line::from(Span::styled(
            "Log Entry — Select Practice",
            Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Filter input
        let filter_line = if self.filtering {
            Line::from(vec![
                Span::styled("  / ", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(&self.filter_text, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        } else {
            Line::from(Span::styled(
                "  Press / to filter",
                Style::default().fg(Color::DarkGray),
            ))
        };
        frame.render_widget(Paragraph::new(filter_line), chunks[1]);

        // Practice list
        let mut lines: Vec<Line> = Vec::new();
        for (i, &idx) in self.filtered_indices.iter().enumerate() {
            let practice = &self.practices[idx];
            let prefix = if i == self.selected { " > " } else { "   " };
            let style = if i == self.selected {
                Style::default().fg(Color::Rgb(78, 202, 78)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let type_str = format!(" ({})", practice.practice_type);
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&practice.name, style),
                Span::styled(type_str, Style::default().fg(Color::DarkGray)),
            ]));
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "   No practices found. Add one in [e] Practices first.",
                Style::default().fg(Color::DarkGray),
            )));
        }
        frame.render_widget(Paragraph::new(lines), chunks[2]);

        let shortcuts = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    fn render_set_entry(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let practice = self.chosen_practice.as_ref().unwrap();
        let (label1, label2) = self.field_labels();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Min(4),    // sets + current input
                Constraint::Length(3), // running total
                Constraint::Length(2), // shortcuts
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled("Log Entry — ", Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD)),
            Span::styled(&practice.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" ({})", practice.practice_type),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Logged sets + current input
        let mut lines: Vec<Line> = Vec::new();
        for (i, set) in self.sets.iter().enumerate() {
            let desc = match set {
                SetData::Weighted { weight, reps } => format!("  #{}   {}kg  x {}", i + 1, weight, reps),
                SetData::Bodyweight { reps } => format!("  #{}   {} reps", i + 1, reps),
                SetData::Distance { distance } => format!("  #{}   {}km", i + 1, distance),
                SetData::Endurance { duration } => format!("  #{}   {}min", i + 1, duration),
            };
            lines.push(Line::from(Span::styled(desc, Style::default().fg(Color::Rgb(78, 202, 78)))));
        }

        // Current input line
        let set_num = self.sets.len() + 1;
        let carry_hint = self.last_field1_value().map(|v| format!(" ({})", v)).unwrap_or_default();

        let f1_style = if self.active_field == 0 {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let f1_display = if self.field1.is_empty() && self.active_field == 0 {
            format!("█{}", carry_hint)
        } else if self.active_field == 0 {
            format!("{}█", self.field1)
        } else {
            self.field1.clone()
        };

        if let Some(l2) = label2 {
            let f2_style = if self.active_field == 1 {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let f2_display = if self.active_field == 1 {
                format!("{}█", self.field2)
            } else {
                self.field2.clone()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  #{}   ", set_num), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}: ", label1), Style::default().fg(Color::DarkGray)),
                Span::styled(f1_display, f1_style),
                Span::styled(format!("  {}: ", l2), Style::default().fg(Color::DarkGray)),
                Span::styled(f2_display, f2_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  #{}   ", set_num), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}: ", label1), Style::default().fg(Color::DarkGray)),
                Span::styled(f1_display, f1_style),
            ]));
        }
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Running total
        let total = self.running_total();
        let metric_label = self.sets.first().map(|s| s.metric_label()).unwrap_or("—");
        let total_lines = vec![
            Line::from(Span::raw("")),
            Line::from(vec![
                Span::styled("  Sets: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}  ", self.sets.len()), Style::default().fg(Color::White)),
                Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.0} {}", total, metric_label), Style::default().fg(Color::White)),
            ]),
        ];
        frame.render_widget(Paragraph::new(total_lines), chunks[2]);

        let shortcuts = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Next set  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Next field  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Ctrl+S]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[d]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Del last  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    fn render_note_entry(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let practice = self.chosen_practice.as_ref().unwrap();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Length(4),  // summary
                Constraint::Length(3),  // note input
                Constraint::Min(0),
                Constraint::Length(2), // shortcuts
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled("Log Entry — ", Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD)),
            Span::styled(&practice.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let total = self.running_total();
        let metric_label = self.sets.first().map(|s| s.metric_label()).unwrap_or("—");
        let summary = vec![
            Line::from(vec![
                Span::styled(format!("  {} sets", self.sets.len()), Style::default().fg(Color::Rgb(78, 202, 78))),
                Span::styled(format!("  |  {:.0} {}", total, metric_label), Style::default().fg(Color::Rgb(78, 202, 78))),
            ]),
            Line::raw(""),
        ];
        frame.render_widget(Paragraph::new(summary), chunks[1]);

        let note_block = Block::default()
            .title(" Note (optional) ")
            .title_style(Style::default().fg(Color::Rgb(124, 124, 245)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let note_text = format!("{}█", self.note);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(note_text, Style::default().fg(Color::White))))
                .block(note_block),
            chunks[2],
        );

        let shortcuts = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[4]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.phase {
            Phase::SelectPractice => self.handle_practice_picker(key),
            Phase::EnterSets => self.handle_set_entry(key, db),
            Phase::EnterNote => self.handle_note_entry(key, db),
        }
    }

    fn handle_practice_picker(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                    self.filter_text.clear();
                    self.apply_filter();
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = (self.selected + 1).min(self.filtered_indices.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Char('/') => {
                self.filtering = true;
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    self.chosen_practice = Some(self.practices[idx].clone());
                    self.phase = Phase::EnterSets;
                    self.sets.clear();
                    self.field1.clear();
                    self.field2.clear();
                    self.active_field = 0;
                }
            }
            KeyCode::Esc => {
                return Action::Navigate(Screen::Dashboard);
            }
            _ => {}
        }
        Action::None
    }

    fn handle_set_entry(&mut self, key: KeyEvent, _db: &Database) -> Action {
        // Ctrl+S: move to note phase
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.sets.is_empty() {
                self.phase = Phase::EnterNote;
                self.note.clear();
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Tab => {
                if self.has_second_field() {
                    self.active_field = 1 - self.active_field;
                }
            }
            KeyCode::Enter => {
                // If on field1 and there's a field2, move to field2
                if self.active_field == 0 && self.has_second_field() {
                    self.active_field = 1;
                } else {
                    self.commit_set();
                }
            }
            KeyCode::Char('d') => {
                // Only delete if current input is empty (avoid capturing 'd' digit)
                if self.field1.is_empty() && self.field2.is_empty() {
                    self.sets.pop();
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                if self.active_field == 0 {
                    self.field1.push(c);
                } else {
                    self.field2.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.active_field == 0 {
                    self.field1.pop();
                } else {
                    self.field2.pop();
                }
            }
            KeyCode::Esc => {
                return Action::Navigate(Screen::Dashboard);
            }
            _ => {}
        }
        Action::None
    }

    fn handle_note_entry(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                // Save the log
                if let Some(practice) = &self.chosen_practice {
                    let note = if self.note.is_empty() {
                        None
                    } else {
                        Some(self.note.as_str())
                    };
                    let _ = db.create_log(practice.id, &self.sets, note);
                }
                return Action::Navigate(Screen::Dashboard);
            }
            KeyCode::Esc => {
                return Action::Navigate(Screen::Dashboard);
            }
            KeyCode::Backspace => {
                self.note.pop();
            }
            KeyCode::Char(c) => {
                self.note.push(c);
            }
            _ => {}
        }
        Action::None
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 3: Test the log entry flow manually**

```bash
cd /Users/jhou/Projects/ironcli && cargo run
```

First press `e` to add a practice, then press `l` to log an entry. Verify: practice picker shows, selecting enters set mode, Enter adds sets, Ctrl+S moves to note, Enter saves.

- [ ] **Step 4: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/log_entry.rs
git commit -m "feat: log entry screen with practice picker, set-by-set input, and note"
```

---

### Task 8: History Screen

**Files:**
- Modify: `src/tui/history.rs`

- [ ] **Step 1: Implement history screen with 14-day log, edit, and delete**

Replace `src/tui/history.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::model::LogEntry;
use super::{Action, Screen};

enum Mode {
    Browse,
    ConfirmDelete,
}

pub struct HistoryScreen {
    entries: Vec<LogEntry>,
    selected: usize,
    scroll_offset: usize,
    mode: Mode,
}

impl HistoryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let entries = db.list_logs_recent(14)?;
        Ok(Self {
            entries,
            selected: 0,
            scroll_offset: 0,
            mode: Mode::Browse,
        })
    }

    fn visible_rows(&self, height: u16) -> usize {
        // Reserve 4 lines for title, detail, shortcuts
        height.saturating_sub(4) as usize
    }

    pub fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Min(4),    // log list
                Constraint::Length(4), // detail for selected
                Constraint::Length(2), // shortcuts
            ])
            .split(size);

        let title = Line::from(Span::styled(
            "History — Last 14 Days",
            Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Log list
        let visible = self.visible_rows(chunks[1].height);
        let mut lines: Vec<Line> = Vec::new();

        if self.entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No entries in the last 14 days.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, entry) in self.entries.iter().enumerate().skip(self.scroll_offset).take(visible) {
                let is_selected = i == self.selected;
                let prefix = if is_selected { " > " } else { "   " };
                let date = entry.log.logged_at.format("%b %d %H:%M").to_string();
                let summary = format!(
                    "{}  {}  {} sets  {:.0} {}",
                    date,
                    entry.practice_name,
                    entry.sets.len(),
                    entry.total_metric(),
                    entry.metric_label(),
                );

                let style = if is_selected {
                    Style::default().fg(Color::Rgb(78, 202, 78)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(summary, style),
                ]));
            }
        }
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Detail pane for selected entry
        let detail_lines = if let Some(entry) = self.entries.get(self.selected) {
            let mut dlines: Vec<Line> = Vec::new();
            for set in &entry.sets {
                let desc = match &set.data {
                    crate::model::SetData::Weighted { weight, reps } => format!("    #{}  {}kg x {}", set.set_number, weight, reps),
                    crate::model::SetData::Bodyweight { reps } => format!("    #{}  {} reps", set.set_number, reps),
                    crate::model::SetData::Distance { distance } => format!("    #{}  {}km", set.set_number, distance),
                    crate::model::SetData::Endurance { duration } => format!("    #{}  {}min", set.set_number, duration),
                };
                dlines.push(Line::from(Span::styled(desc, Style::default().fg(Color::DarkGray))));
            }
            if let Some(note) = &entry.log.note {
                dlines.push(Line::from(Span::styled(
                    format!("    Note: {}", note),
                    Style::default().fg(Color::Rgb(200, 200, 100)),
                )));
            }
            dlines
        } else {
            vec![]
        };
        frame.render_widget(Paragraph::new(detail_lines), chunks[2]);

        // Shortcuts / confirm delete
        let shortcuts = match self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[d]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Delete  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Back", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::ConfirmDelete => Line::from(vec![
                Span::styled(" Delete this log? ", Style::default().fg(Color::Rgb(232, 84, 84))),
                Span::styled("[y]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Yes  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[n]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" No", Style::default().fg(Color::DarkGray)),
            ]),
        };
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            Mode::ConfirmDelete => match key.code {
                KeyCode::Char('y') => {
                    if let Some(entry) = self.entries.get(self.selected) {
                        let _ = db.delete_log(entry.log.id);
                        self.entries = db.list_logs_recent(14).unwrap_or_default();
                        if self.selected >= self.entries.len() && self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    self.mode = Mode::Browse;
                    Action::None
                }
                _ => {
                    self.mode = Mode::Browse;
                    Action::None
                }
            },
            Mode::Browse => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.entries.is_empty() {
                        self.selected = (self.selected + 1).min(self.entries.len() - 1);
                    }
                    Action::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    Action::None
                }
                KeyCode::Char('d') => {
                    if !self.entries.is_empty() {
                        self.mode = Mode::ConfirmDelete;
                    }
                    Action::None
                }
                KeyCode::Esc => Action::Navigate(Screen::Dashboard),
                _ => Action::None,
            },
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 3: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/history.rs
git commit -m "feat: history screen with 14-day log, set details, and delete confirmation"
```

---

### Task 9: Sparkline Widget + Trends Screen

**Files:**
- Modify: `src/tui/widgets/sparkline.rs`
- Modify: `src/tui/trends.rs`

- [ ] **Step 1: Implement sparkline bar chart widget**

Replace `src/tui/widgets/sparkline.rs` with:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Vertical bar chart widget showing values over time.
pub struct SparklineChart {
    data: Vec<(String, f64)>, // (label, value)
    max_value: f64,
}

impl SparklineChart {
    pub fn new(data: Vec<(String, f64)>) -> Self {
        let max_value = data.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
        Self { data, max_value }
    }
}

impl Widget for SparklineChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 4 || self.data.is_empty() {
            return;
        }

        let chart_height = area.height.saturating_sub(2); // reserve 1 for labels, 1 for x-axis
        let bar_width: u16 = 2;
        let gap: u16 = 1;
        let total_per_bar = bar_width + gap;
        let max_bars = ((area.width) / total_per_bar) as usize;
        let visible_data: Vec<&(String, f64)> = self.data.iter().rev().take(max_bars).collect::<Vec<_>>().into_iter().rev().collect();

        let max = if self.max_value > 0.0 { self.max_value } else { 1.0 };

        for (i, (label, value)) in visible_data.iter().enumerate() {
            let bar_height = ((value / max) * chart_height as f64).round() as u16;
            let x = area.x + (i as u16) * total_per_bar;

            // Color gradient based on relative value
            let intensity = (value / max * 255.0).min(255.0) as u8;
            let color = Color::Rgb(
                45_u8.saturating_add(intensity / 8),
                90_u8.saturating_add(intensity / 2),
                45_u8.saturating_add(intensity / 8),
            );

            // Draw bars from bottom up
            for row in 0..bar_height {
                let y = area.y + chart_height - 1 - row;
                if x + 1 < area.x + area.width {
                    buf.set_string(x, y, "██", Style::default().fg(color));
                }
            }

            // X-axis label (show abbreviated)
            let label_y = area.y + chart_height;
            let short_label: String = label.chars().take(2).collect();
            if x + 1 < area.x + area.width {
                buf.set_string(x, label_y, &short_label, Style::default().fg(Color::DarkGray));
            }
        }

        // Y-axis labels (top and bottom)
        let label_y_top = area.y;
        let label_y_bot = area.y + chart_height - 1;
        let max_label = format!("{:.0}", max);
        let right_x = area.x + area.width.saturating_sub(max_label.len() as u16);
        buf.set_string(right_x, label_y_top, &max_label, Style::default().fg(Color::DarkGray));
        buf.set_string(right_x, label_y_bot, "0", Style::default().fg(Color::DarkGray));
    }
}
```

- [ ] **Step 2: Implement trends screen**

Replace `src/tui/trends.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::model::{LogEntry, Practice};
use super::widgets::sparkline::SparklineChart;
use super::{Action, Screen};

enum Phase {
    SelectPractice,
    ViewChart,
}

pub struct TrendsScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filtering: bool,
    selected: usize,
    phase: Phase,
    // Chart data
    chosen_practice: Option<Practice>,
    entries: Vec<LogEntry>,
    days_window: i64,
}

impl TrendsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::SelectPractice,
            chosen_practice: None,
            entries: Vec::new(),
            days_window: 30,
        })
    }

    fn apply_filter(&mut self) {
        let query = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    fn load_chart_data(&mut self, db: &Database) {
        if let Some(practice) = &self.chosen_practice {
            self.entries = db
                .list_logs_for_practice(practice.id, self.days_window)
                .unwrap_or_default();
        }
    }

    fn chart_data(&self) -> Vec<(String, f64)> {
        self.entries
            .iter()
            .map(|e| {
                let label = e.log.logged_at.format("%d").to_string();
                let value = e.total_metric();
                (label, value)
            })
            .collect()
    }

    fn stats(&self) -> (f64, f64, f64) {
        if self.entries.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let values: Vec<f64> = self.entries.iter().map(|e| e.total_metric()).collect();
        let avg = values.iter().sum::<f64>() / values.len() as f64;
        let peak = values.iter().cloned().fold(0.0_f64, f64::max);

        // Trend: compare average of last half to first half
        let mid = values.len() / 2;
        let trend = if mid > 0 && values.len() > 1 {
            let first_avg = values[..mid].iter().sum::<f64>() / mid as f64;
            let second_avg = values[mid..].iter().sum::<f64>() / (values.len() - mid) as f64;
            if first_avg > 0.0 {
                ((second_avg - first_avg) / first_avg) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        (avg, peak, trend)
    }

    pub fn render(&self, frame: &mut Frame) {
        match self.phase {
            Phase::SelectPractice => self.render_practice_picker(frame),
            Phase::ViewChart => self.render_chart(frame),
        }
    }

    fn render_practice_picker(&self, frame: &mut Frame) {
        let size = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(4),
                Constraint::Length(2),
            ])
            .split(size);

        let title = Line::from(Span::styled(
            "Trends — Select Practice",
            Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let filter_line = if self.filtering {
            Line::from(vec![
                Span::styled("  / ", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(&self.filter_text, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        } else {
            Line::from(Span::styled("  Press / to filter", Style::default().fg(Color::DarkGray)))
        };
        frame.render_widget(Paragraph::new(filter_line), chunks[1]);

        let mut lines: Vec<Line> = Vec::new();
        for (i, &idx) in self.filtered_indices.iter().enumerate() {
            let practice = &self.practices[idx];
            let prefix = if i == self.selected { " > " } else { "   " };
            let style = if i == self.selected {
                Style::default().fg(Color::Rgb(78, 202, 78)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&practice.name, style),
                Span::styled(
                    format!(" ({})", practice.practice_type),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        frame.render_widget(Paragraph::new(lines), chunks[2]);

        let shortcuts = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Back", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    fn render_chart(&self, frame: &mut Frame) {
        let size = frame.area();
        let practice = self.chosen_practice.as_ref().unwrap();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Length(1),  // subtitle
                Constraint::Min(8),    // chart
                Constraint::Length(3), // stats
                Constraint::Length(2), // shortcuts
            ])
            .split(size);

        let metric_label = self
            .entries
            .first()
            .map(|e| e.metric_label())
            .unwrap_or("—");

        let title = Line::from(vec![
            Span::styled(&practice.name, Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" — {} ({})", metric_label, practice.practice_type),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let subtitle = Line::from(Span::styled(
            format!("Last {} days", self.days_window),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(subtitle), chunks[1]);

        // Sparkline chart
        let data = self.chart_data();
        if data.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  No data for this period.",
                    Style::default().fg(Color::DarkGray),
                ))),
                chunks[2],
            );
        } else {
            let chart = SparklineChart::new(data);
            frame.render_widget(chart, chunks[2]);
        }

        // Stats
        let (avg, peak, trend) = self.stats();
        let trend_color = if trend >= 0.0 { Color::Rgb(78, 202, 78) } else { Color::Rgb(232, 84, 84) };
        let trend_sign = if trend >= 0.0 { "+" } else { "" };
        let stats_line = Line::from(vec![
            Span::styled("  Avg: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.0}  ", avg), Style::default().fg(Color::White)),
            Span::styled("Peak: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.0}  ", peak), Style::default().fg(Color::White)),
            Span::styled("Trend: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}{:.0}%", trend_sign, trend), Style::default().fg(trend_color)),
        ]);
        frame.render_widget(Paragraph::new(stats_line), chunks[3]);

        let shortcuts = Line::from(vec![
            Span::styled(" [h/l]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Time window  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Change practice  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
            Span::styled(" Back", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(shortcuts), chunks[4]);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.phase {
            Phase::SelectPractice => self.handle_practice_picker(key),
            Phase::ViewChart => self.handle_chart(key),
        }
    }

    fn handle_practice_picker(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                    self.filter_text.clear();
                    self.apply_filter();
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = (self.selected + 1).min(self.filtered_indices.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Char('/') => {
                self.filtering = true;
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    self.chosen_practice = Some(self.practices[idx].clone());
                    self.phase = Phase::ViewChart;
                    // Need db reference — store practice and load on next render
                }
            }
            KeyCode::Esc => return Action::Navigate(Screen::Dashboard),
            _ => {}
        }
        Action::None
    }

    fn handle_chart(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                // Expand time window
                self.days_window = (self.days_window + 30).min(365);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                // Shrink time window
                self.days_window = (self.days_window - 30).max(30);
            }
            KeyCode::Char('/') => {
                self.phase = Phase::SelectPractice;
            }
            KeyCode::Esc => return Action::Navigate(Screen::Dashboard),
            _ => {}
        }
        Action::None
    }

    /// Called by app.rs after handle_key to refresh chart data when db is available.
    pub fn refresh_chart(&mut self, db: &Database) {
        self.load_chart_data(db);
    }
}
```

- [ ] **Step 3: Update app.rs to call refresh_chart on trends**

In `src/app.rs`, update the trends key handling in `run_app`. After calling `trends.handle_key(key)`, add a refresh call. Find the match arm for `Screen::Trends` and change it:

```rust
Screen::Trends => {
    let action = trends.handle_key(key);
    trends.refresh_chart(db);
    action
}
```

Replace the simpler version:
```rust
Screen::Trends => trends.handle_key(key),
```
with the version above that includes the `refresh_chart` call.

Also update the `Navigate` handler for `Screen::Trends`:
```rust
Screen::Trends => {
    trends = TrendsScreen::new(db)?;
    // If re-entering, data will load on first interaction
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 5: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/widgets/sparkline.rs src/tui/trends.rs src/app.rs
git commit -m "feat: sparkline widget and trends screen with time-window navigation"
```

---

### Task 10: Practices Screen

**Files:**
- Modify: `src/tui/practices.rs`

- [ ] **Step 1: Implement practices inventory screen**

Replace `src/tui/practices.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::Database;
use crate::model::{Practice, PracticeType};
use super::{Action, Screen};

enum Mode {
    Browse,
    AddName,
    AddType,
    EditName,
    ConfirmDelete,
}

pub struct PracticesScreen {
    practices: Vec<Practice>,
    selected: usize,
    mode: Mode,
    input: String,
    type_selected: usize,
}

impl PracticesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        Ok(Self {
            practices,
            selected: 0,
            mode: Mode::Browse,
            input: String::new(),
            type_selected: 0,
        })
    }

    fn refresh(&mut self, db: &Database) {
        self.practices = db.list_practices().unwrap_or_default();
        if self.selected >= self.practices.len() && self.selected > 0 {
            self.selected = self.practices.len() - 1;
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title
                Constraint::Min(4),    // list
                Constraint::Length(4), // input/action area
                Constraint::Length(2), // shortcuts
            ])
            .split(size);

        let title = Line::from(Span::styled(
            "Practices",
            Style::default().fg(Color::Rgb(124, 124, 245)).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Practice list
        let mut lines: Vec<Line> = Vec::new();
        if self.practices.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No practices yet. Press [a] to add one.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, p) in self.practices.iter().enumerate() {
                let is_selected = i == self.selected;
                let prefix = if is_selected { " > " } else { "   " };
                let style = if is_selected {
                    Style::default().fg(Color::Rgb(78, 202, 78)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(&p.name, style),
                    Span::styled(
                        format!("  ({})", p.practice_type),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Input/action area
        match &self.mode {
            Mode::AddName | Mode::EditName => {
                let label = if matches!(self.mode, Mode::AddName) {
                    " New Practice Name "
                } else {
                    " Edit Practice Name "
                };
                let block = Block::default()
                    .title(label)
                    .title_style(Style::default().fg(Color::Rgb(124, 124, 245)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray));
                let text = format!("{}█", self.input);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(text, Style::default().fg(Color::White))))
                        .block(block),
                    chunks[2],
                );
            }
            Mode::AddType => {
                let mut type_lines: Vec<Line> = vec![Line::from(Span::styled(
                    " Select Type:",
                    Style::default().fg(Color::Rgb(124, 124, 245)),
                ))];
                for (i, pt) in PracticeType::ALL.iter().enumerate() {
                    let prefix = if i == self.type_selected { " > " } else { "   " };
                    let style = if i == self.type_selected {
                        Style::default().fg(Color::Rgb(78, 202, 78)).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    type_lines.push(Line::from(Span::styled(format!("{}{}", prefix, pt), style)));
                }
                frame.render_widget(Paragraph::new(type_lines), chunks[2]);
            }
            Mode::ConfirmDelete => {
                let name = self
                    .practices
                    .get(self.selected)
                    .map(|p| p.name.as_str())
                    .unwrap_or("?");
                let msg = Line::from(vec![
                    Span::styled(
                        format!(" Delete \"{}\"? This removes all its logs. ", name),
                        Style::default().fg(Color::Rgb(232, 84, 84)),
                    ),
                    Span::styled("[y] Yes  [n] No", Style::default().fg(Color::DarkGray)),
                ]);
                frame.render_widget(Paragraph::new(msg), chunks[2]);
            }
            Mode::Browse => {}
        }

        // Shortcuts
        let shortcuts = match self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[a]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Add  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Edit  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[d]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Delete  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Back", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::AddName | Mode::EditName => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::AddType => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(124, 124, 245))),
                Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::ConfirmDelete => Line::from(Span::raw("")),
        };
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            Mode::Browse => self.handle_browse(key),
            Mode::AddName => self.handle_add_name(key),
            Mode::AddType => self.handle_add_type(key, db),
            Mode::EditName => self.handle_edit_name(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.practices.is_empty() {
                    self.selected = (self.selected + 1).min(self.practices.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Char('a') => {
                self.mode = Mode::AddName;
                self.input.clear();
            }
            KeyCode::Enter => {
                if let Some(p) = self.practices.get(self.selected) {
                    self.input = p.name.clone();
                    self.mode = Mode::EditName;
                }
            }
            KeyCode::Char('d') => {
                if !self.practices.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
            }
            KeyCode::Esc => return Action::Navigate(Screen::Dashboard),
            _ => {}
        }
        Action::None
    }

    fn handle_add_name(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    self.mode = Mode::AddType;
                    self.type_selected = 0;
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            _ => {}
        }
        Action::None
    }

    fn handle_add_type(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.type_selected = (self.type_selected + 1).min(PracticeType::ALL.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.type_selected = self.type_selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                let pt = PracticeType::ALL[self.type_selected];
                let _ = db.create_practice(self.input.trim(), pt);
                self.refresh(db);
                self.mode = Mode::Browse;
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_edit_name(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(p) = self.practices.get(self.selected) {
                    if !self.input.trim().is_empty() {
                        let _ = db.rename_practice(p.id, self.input.trim());
                        self.refresh(db);
                    }
                }
                self.mode = Mode::Browse;
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            _ => {}
        }
        Action::None
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(p) = self.practices.get(self.selected) {
                    let _ = db.delete_practice(p.id);
                    self.refresh(db);
                }
                self.mode = Mode::Browse;
            }
            _ => {
                self.mode = Mode::Browse;
            }
        }
        Action::None
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 3: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/practices.rs
git commit -m "feat: practices screen with add, edit, delete, and type selection"
```

---

### Task 11: Export/Import

**Files:**
- Modify: `src/export.rs`
- Create: `tests/export_test.rs`

- [ ] **Step 1: Write export/import round-trip test**

Create `tests/export_test.rs`:

```rust
use ironcli::db::Database;
use ironcli::export::{export_to_json, import_from_json};
use ironcli::model::{PracticeType, SetData};
use std::path::PathBuf;

#[test]
fn round_trip_export_import() {
    // Create source db with data
    let src = Database::open_in_memory().unwrap();
    let p1 = src.create_practice("KB Snatch", PracticeType::Weighted).unwrap();
    let p2 = src.create_practice("Running", PracticeType::Distance).unwrap();
    src.create_log(
        p1,
        &[
            SetData::Weighted { weight: 24.0, reps: 10 },
            SetData::Weighted { weight: 24.0, reps: 9 },
        ],
        Some("Felt strong"),
    ).unwrap();
    src.create_log(
        p2,
        &[SetData::Distance { distance: 5.0 }],
        None,
    ).unwrap();

    // Export to temp file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    export_to_json(&src, Some(path.clone())).unwrap();

    // Import into fresh db
    let dst = Database::open_in_memory().unwrap();
    let count = import_from_json(&dst, &path).unwrap();
    assert_eq!(count, 2);

    // Verify data
    let practices = dst.list_practices().unwrap();
    assert_eq!(practices.len(), 2);

    let logs = dst.list_logs_recent(365).unwrap();
    assert_eq!(logs.len(), 2);

    // Find KB Snatch log
    let kb = logs.iter().find(|l| l.practice_name == "KB Snatch").unwrap();
    assert_eq!(kb.sets.len(), 2);
    assert_eq!(kb.log.note.as_deref(), Some("Felt strong"));
}

#[test]
fn import_skips_duplicates() {
    let db = Database::open_in_memory().unwrap();
    let pid = db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    db.create_log(pid, &[SetData::Bodyweight { reps: 20 }], None).unwrap();

    // Export
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    export_to_json(&db, Some(path.clone())).unwrap();

    // Import into same db — should skip the duplicate
    let count = import_from_json(&db, &path).unwrap();
    assert_eq!(count, 0);
    assert_eq!(db.list_logs_recent(365).unwrap().len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test export_test
```

Expected: FAIL — `export` functions are stubs.

- [ ] **Step 3: Implement export.rs**

Replace `src/export.rs` with:

```rust
use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::model::*;

#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub practices: Vec<ExportPractice>,
    pub logs: Vec<ExportLog>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportPractice {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub practice_type: PracticeType,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExportLog {
    pub id: i64,
    pub practice: String,
    pub logged_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub sets: Vec<ExportSet>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportSet {
    pub set_number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

pub fn export_to_json(db: &Database, path: Option<PathBuf>) -> Result<()> {
    let (practices, log_entries) = db.export_all()?;

    let export_practices: Vec<ExportPractice> = practices
        .iter()
        .map(|p| ExportPractice {
            id: p.id,
            name: p.name.clone(),
            practice_type: p.practice_type,
            created_at: p.created_at.to_string(),
        })
        .collect();

    let export_logs: Vec<ExportLog> = log_entries
        .iter()
        .map(|entry| {
            let sets: Vec<ExportSet> = entry
                .sets
                .iter()
                .map(|s| {
                    let (weight, reps, distance, duration) = match &s.data {
                        SetData::Weighted { weight, reps } => {
                            (Some(*weight), Some(*reps), None, None)
                        }
                        SetData::Bodyweight { reps } => (None, Some(*reps), None, None),
                        SetData::Distance { distance } => (None, None, Some(*distance), None),
                        SetData::Endurance { duration } => (None, None, None, Some(*duration)),
                    };
                    ExportSet {
                        set_number: s.set_number,
                        weight,
                        reps,
                        distance,
                        duration,
                    }
                })
                .collect();

            ExportLog {
                id: entry.log.id,
                practice: entry.practice_name.clone(),
                logged_at: entry.log.logged_at.to_string(),
                note: entry.log.note.clone(),
                sets,
            }
        })
        .collect();

    let data = ExportData {
        version: 1,
        exported_at: Local::now().to_rfc3339(),
        practices: export_practices,
        logs: export_logs,
    };

    let output_path = path.unwrap_or_else(|| {
        let date = Local::now().format("%Y-%m-%d").to_string();
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ironcli")
            .join(format!("iron-export-{}.json", date))
    });

    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(&output_path, json)?;
    eprintln!("Exported to {}", output_path.display());
    Ok(())
}

pub fn import_from_json(db: &Database, path: &Path) -> Result<usize> {
    let json = std::fs::read_to_string(path).context("failed to read import file")?;
    let data: ExportData = serde_json::from_str(&json).context("failed to parse JSON")?;

    let mut imported = 0;

    // Ensure all practices exist
    let existing = db.list_practices()?;
    for ep in &data.practices {
        if !existing.iter().any(|p| p.name == ep.name) {
            db.create_practice(&ep.name, ep.practice_type)?;
        }
    }

    // Re-read practices to get IDs
    let practices = db.list_practices()?;

    for log in &data.logs {
        // Check for duplicate
        if db.log_exists(&log.practice, &log.logged_at)? {
            continue;
        }

        let practice = practices
            .iter()
            .find(|p| p.name == log.practice)
            .context(format!("practice not found: {}", log.practice))?;

        let sets: Vec<SetData> = log
            .sets
            .iter()
            .map(|s| {
                if let (Some(w), Some(r)) = (s.weight, s.reps) {
                    SetData::Weighted { weight: w, reps: r }
                } else if let Some(r) = s.reps {
                    SetData::Bodyweight { reps: r }
                } else if let Some(d) = s.distance {
                    SetData::Distance { distance: d }
                } else if let Some(d) = s.duration {
                    SetData::Endurance { duration: d }
                } else {
                    SetData::Bodyweight { reps: 0 }
                }
            })
            .collect();

        let note = log.note.as_deref();
        db.create_log(practice.id, &sets, note)?;
        imported += 1;
    }

    Ok(imported)
}
```

- [ ] **Step 4: Add export module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod db;
pub mod export;
pub mod model;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /Users/jhou/Projects/ironcli && cargo test --test export_test
```

Expected: all 2 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/export.rs src/lib.rs tests/export_test.rs
git commit -m "feat: JSON export/import with round-trip support and duplicate skipping"
```

---

## Phase 3: Integration & Polish

---

### Task 12: History Edit Flow

**Files:**
- Modify: `src/tui/log_entry.rs`
- Modify: `src/tui/history.rs`
- Modify: `src/app.rs`

The spec requires pressing Enter on a log in History to re-open the set-by-set editor. This requires coordination between History and LogEntry screens.

- [ ] **Step 1: Add edit constructor to LogEntryScreen**

Add a method to `src/tui/log_entry.rs` that pre-populates the screen with an existing log's data:

```rust
impl LogEntryScreen {
    /// Create a LogEntryScreen pre-populated with an existing log for editing.
    pub fn from_existing(db: &Database, log_entry: &crate::model::LogEntry) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        let practice = practices
            .iter()
            .find(|p| p.id == log_entry.log.practice_id)
            .cloned();

        let sets: Vec<SetData> = log_entry.sets.iter().map(|s| s.data.clone()).collect();
        let note = log_entry.log.note.clone().unwrap_or_default();

        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::EnterSets,
            chosen_practice: practice,
            sets,
            field1: String::new(),
            field2: String::new(),
            active_field: 0,
            note,
        })
    }
}
```

- [ ] **Step 2: Add editing_log_id field to LogEntryScreen**

Add a field `editing_log_id: Option<i64>` to `LogEntryScreen`. Set it to `None` in `new()`, set it to `Some(log.id)` in `from_existing()`. In `handle_note_entry`, when saving:

```rust
// In handle_note_entry, replace the create_log call:
if let Some(practice) = &self.chosen_practice {
    let note = if self.note.is_empty() { None } else { Some(self.note.as_str()) };
    if let Some(log_id) = self.editing_log_id {
        let _ = db.update_log(log_id, &self.sets, note);
    } else {
        let _ = db.create_log(practice.id, &self.sets, note);
    }
}
```

- [ ] **Step 3: Add Enter handler to HistoryScreen**

In `src/tui/history.rs`, add a method to get the currently selected entry and handle Enter:

```rust
impl HistoryScreen {
    /// Returns the currently selected log entry, if any.
    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.entries.get(self.selected)
    }
}
```

In `handle_browse`, add:
```rust
KeyCode::Enter => {
    // Signal to app.rs that we want to edit — handled via Action
    // We'll return Navigate to LogEntry; app.rs checks for selected entry
    if !self.entries.is_empty() {
        return Action::Navigate(Screen::LogEntry);
    }
}
```

- [ ] **Step 4: Wire up in app.rs**

In `src/app.rs`, when navigating from History to LogEntry, check if there's a selected entry to edit:

```rust
Action::Navigate(screen) => {
    match &screen {
        Screen::Dashboard => dashboard.refresh(db)?,
        Screen::LogEntry => {
            // Check if navigating from History (edit mode)
            if current_screen == Screen::History {
                if let Some(entry) = history.selected_entry() {
                    log_entry = LogEntryScreen::from_existing(db, entry)?;
                } else {
                    log_entry = LogEntryScreen::new(db)?;
                }
            } else {
                log_entry = LogEntryScreen::new(db)?;
            }
        }
        Screen::History => history = HistoryScreen::new(db)?,
        Screen::Trends => {
            trends = TrendsScreen::new(db)?;
        }
        Screen::Practices => practices = PracticesScreen::new(db)?,
    }
    current_screen = screen;
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /Users/jhou/Projects/ironcli && cargo check
```

- [ ] **Step 6: Manually test edit flow**

```bash
cd /Users/jhou/Projects/ironcli && cargo run
```

1. Log a practice with 2 sets
2. Go to History → select the log → press Enter
3. Verify the LogEntry screen opens with the existing sets pre-populated
4. Add another set → Ctrl+S → update note → Enter
5. Go back to History → verify the log now shows 3 sets and the updated note

- [ ] **Step 7: Commit**

```bash
cd /Users/jhou/Projects/ironcli
git add src/tui/log_entry.rs src/tui/history.rs src/app.rs
git commit -m "feat: edit existing logs from history via Enter key"
```

---

### Task 13: Final Integration & Polish

**Files:**
- Modify: `src/main.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Verify main.rs correctly references all modules**

Read `src/main.rs` and ensure it has the correct module declarations. It should have:

```rust
mod app;
mod db;
mod export;
mod model;
mod tui;
```

The `lib.rs` exports `pub mod db`, `pub mod export`, `pub mod model` for tests, but the binary uses `mod` declarations. Both can coexist — the binary uses its own module tree, tests use the library.

- [ ] **Step 2: Run the full test suite**

```bash
cd /Users/jhou/Projects/ironcli && cargo test
```

Expected: all tests pass (model, db, export).

- [ ] **Step 3: Run cargo clippy for lint checks**

```bash
cd /Users/jhou/Projects/ironcli && cargo clippy -- -W warnings
```

Fix any warnings.

- [ ] **Step 4: Build the release binary**

```bash
cd /Users/jhou/Projects/ironcli && cargo build --release
```

- [ ] **Step 5: End-to-end manual test**

```bash
cd /Users/jhou/Projects/ironcli && cargo run
```

Test the full flow:
1. Press `e` → add "Kettlebell Snatch" (weighted), "Push Up" (bodyweight), "Running" (distance), "One Leg Standing" (endurance)
2. Press `Esc` → back to dashboard
3. Press `l` → select KB Snatch → enter sets: 24/10, 24/9, 28/8 → Ctrl+S → type a note → Enter
4. Press `l` → select Push Up → enter sets: 20, 18, 15 → Ctrl+S → Enter (skip note)
5. Dashboard should show today's entries and heatmap has a green cell for today
6. Press `h` → history shows the 2 logs with set details and note
7. Press `Esc` → `t` → select KB Snatch → sparkline shows 1 bar
8. Press `q` to quit

Then test export/import:
```bash
./target/release/iron export /tmp/test-export.json
cat /tmp/test-export.json
```

Verify the JSON is well-formed and contains the logged data.

- [ ] **Step 6: Commit any fixes from integration testing**

```bash
cd /Users/jhou/Projects/ironcli
git add -A
git commit -m "fix: integration polish from end-to-end testing"
```

---

## Parallelization Guide

Tasks are grouped into phases:

| Phase | Tasks | Strategy |
|---|---|---|
| **Phase 1** (sequential) | Tasks 1 → 2 → 3 → 4 | Must be done in order — each builds on the previous |
| **Phase 2** (parallel) | Tasks 5+6, 7, 8, 9, 10, 11 | Independent screens/components. Can dispatch as parallel subagents. Task 6 depends on Task 5 (heatmap widget) so group them. |
| **Phase 3** (sequential) | Tasks 12 → 13 | Integration — requires all Phase 2 tasks complete |

**Recommended subagent split for Phase 2:**
- Agent A: Task 5 + Task 6 (heatmap widget + dashboard)
- Agent B: Task 7 (log entry screen)
- Agent C: Task 8 (history screen)
- Agent D: Task 9 (sparkline widget + trends screen)
- Agent E: Task 10 (practices screen)
- Agent F: Task 11 (export/import)
