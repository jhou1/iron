use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::model::{Abbreviation, Goal, Log, LogEntry, Milestone, Practice, PracticeType, Quote, Set, SetData};

/// Aggregate statistics over a time period.
#[allow(dead_code)]
pub struct AggregateStats {
    pub sessions: i64,
    pub total_volume: f64,
    pub total_reps: f64,
    pub total_distance: f64,
    pub total_duration: f64,
}


pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens the default database at `~/.iron/iron.db`, creating it if needed.
    /// Automatically migrates data from `~/.ironcli/iron.db` on first use.
    pub fn open_default() -> Result<Self> {
        let home = dirs::home_dir().context("could not determine home directory")?;
        let new_dir = home.join(".iron");
        let old_dir = home.join(".ironcli");
        let new_path = new_dir.join("iron.db");
        let old_path = old_dir.join("iron.db");

        // One-time migration: copy old database to new location if it exists
        if !new_path.exists() && old_path.exists() {
            std::fs::create_dir_all(&new_dir)?;
            std::fs::copy(&old_path, &new_path)
                .with_context(|| format!("failed to migrate database from {} to {}", old_path.display(), new_path.display()))?;
            eprintln!("Migrated database from {} to {}", old_path.display(), new_path.display());
        }

        std::fs::create_dir_all(&new_dir)?;
        Self::open(&new_path)
    }

    /// Opens a database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Opens an in-memory database (for tests).
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS practices (
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

            CREATE TABLE IF NOT EXISTS goals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS milestones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                goal_id INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS quotes (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                text     TEXT NOT NULL,
                position INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL UNIQUE,
                hrv INTEGER
            );

            CREATE TABLE IF NOT EXISTS abbreviations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                short TEXT NOT NULL UNIQUE COLLATE NOCASE,
                full_name TEXT NOT NULL
            );",
        )?;

        // Migrations for existing databases
        let _ = self.conn.execute("ALTER TABLE goals ADD COLUMN completed INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE goals ADD COLUMN completed_at TEXT", []);
        let _ = self.conn.execute("ALTER TABLE milestones ADD COLUMN completed_at TEXT", []);
        let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN warm_up TEXT", []);
        let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN cool_down TEXT", []);
        let _ = self.conn.execute("ALTER TABLE practices ADD COLUMN active INTEGER NOT NULL DEFAULT 1", []);

        Ok(())
    }

    // ── Practice CRUD ──────────────────────────────────────────────────

    pub fn create_practice(&self, name: &str, practice_type: PracticeType) -> Result<Practice> {
        let now = Local::now().naive_local();
        self.conn.execute(
            "INSERT INTO practices (name, practice_type, created_at) VALUES (?1, ?2, ?3)",
            params![name, practice_type.to_string(), now.to_string()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Practice {
            id,
            name: name.to_string(),
            practice_type,
            created_at: now,
            active: true,
        })
    }

    pub fn list_practices(&self) -> Result<Vec<Practice>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, practice_type, created_at, active FROM practices ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let pt_str: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            let active: i32 = row.get(4)?;
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, pt_str, created_str, active))
        })?;

        let mut practices = Vec::new();
        for row in rows {
            let (id, name, pt_str, created_str, active) = row?;
            let practice_type: PracticeType = pt_str
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;
            let created_at = NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                .context("failed to parse created_at")?;
            practices.push(Practice {
                id,
                name,
                practice_type,
                created_at,
                active: active != 0,
            });
        }
        Ok(practices)
    }

    pub fn list_active_practices(&self) -> Result<Vec<Practice>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, practice_type, created_at, active FROM practices WHERE active = 1 ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let pt_str: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, pt_str, created_str))
        })?;

        let mut practices = Vec::new();
        for row in rows {
            let (id, name, pt_str, created_str) = row?;
            let practice_type: PracticeType = pt_str
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;
            let created_at = NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                .context("failed to parse created_at")?;
            practices.push(Practice {
                id,
                name,
                practice_type,
                created_at,
                active: true,
            });
        }
        Ok(practices)
    }

    pub fn set_practice_active(&self, id: i64, active: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE practices SET active = ?1 WHERE id = ?2",
            params![active as i32, id],
        )?;
        Ok(())
    }

    pub fn rename_practice(&self, id: i64, new_name: &str) -> Result<()> {
        self.conn
            .execute("UPDATE practices SET name = ?1 WHERE id = ?2", params![new_name, id])?;
        Ok(())
    }

    pub fn delete_practice(&self, id: i64) -> Result<()> {
        // Cascade: delete associated sets and logs first
        let mut stmt = self.conn.prepare("SELECT id FROM logs WHERE practice_id = ?1")?;
        let log_ids: Vec<i64> = stmt
            .query_map(params![id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        for log_id in log_ids {
            self.conn.execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        }
        self.conn.execute("DELETE FROM logs WHERE practice_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM practices WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Log CRUD ───────────────────────────────────────────────────────

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

    /// Creates a log with a specific timestamp (used by import).
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
            self.conn
                .execute("UPDATE logs SET note = ?1, warm_up = ?2, cool_down = ?3 WHERE id = ?4", params![note, warm_up, cool_down, log_id])?;
        }
        // Delete old sets and insert new ones
        self.conn
            .execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        self.insert_sets(log_id, sets)?;
        Ok(())
    }

    pub fn delete_log(&self, log_id: i64) -> Result<()> {
        // Delete sets first (cascade should handle it, but be explicit)
        self.conn
            .execute("DELETE FROM sets WHERE log_id = ?1", params![log_id])?;
        self.conn
            .execute("DELETE FROM logs WHERE id = ?1", params![log_id])?;
        Ok(())
    }

    pub fn restore_log(&self, entry: &LogEntry) -> Result<i64> {
        let sets: Vec<SetData> = entry.sets.iter().map(|s| s.data.clone()).collect();
        self.create_log_at(
            entry.log.practice_id,
            &entry.log.logged_at,
            &sets,
            entry.log.note.as_deref(),
            entry.log.warm_up.as_deref(),
            entry.log.cool_down.as_deref(),
        )
    }

    fn insert_sets(&self, log_id: i64, sets: &[SetData]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO sets (log_id, set_number, weight, reps, distance, duration)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (i, set_data) in sets.iter().enumerate() {
            let (weight, reps, distance, duration) = match set_data {
                SetData::Weighted { weight, reps } => (Some(*weight), Some(*reps), None, None),
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

    // ── Query helpers ──────────────────────────────────────────────────

    pub fn list_logs_all(&self) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, l.warm_up, l.cool_down, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE p.active = 1
             ORDER BY l.logged_at DESC",
        )?;
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

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, practice_id, logged_at_str, note, warm_up, cool_down, practice_name, pt_str) = row?;
            let logged_at =
                NaiveDateTime::parse_from_str(&logged_at_str, "%Y-%m-%d %H:%M:%S%.f")
                    .context("failed to parse logged_at")?;
            let practice_type: PracticeType =
                pt_str.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let sets = self.load_sets(log_id, &practice_type)?;
            entries.push(LogEntry {
                log: Log {
                    id: log_id,
                    practice_id,
                    logged_at,
                    note,
                    warm_up,
                    cool_down,
                },
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    pub fn list_logs_recent(&self, days: i64) -> Result<Vec<LogEntry>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, l.warm_up, l.cool_down, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1 AND p.active = 1
             ORDER BY l.logged_at DESC",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
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

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, practice_id, logged_at_str, note, warm_up, cool_down, practice_name, pt_str) = row?;
            let logged_at =
                NaiveDateTime::parse_from_str(&logged_at_str, "%Y-%m-%d %H:%M:%S%.f")
                    .context("failed to parse logged_at")?;
            let practice_type: PracticeType =
                pt_str.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let sets = self.load_sets(log_id, &practice_type)?;
            entries.push(LogEntry {
                log: Log {
                    id: log_id,
                    practice_id,
                    logged_at,
                    note,
                    warm_up,
                    cool_down,
                },
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    pub fn list_logs_for_practice(&self, practice_id: i64, days: i64) -> Result<Vec<LogEntry>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, l.warm_up, l.cool_down, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.practice_id = ?1 AND l.logged_at >= ?2
             ORDER BY l.logged_at DESC",
        )?;
        let rows = stmt.query_map(params![practice_id, cutoff.to_string()], |row| {
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

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, pid, logged_at_str, note, warm_up, cool_down, practice_name, pt_str) = row?;
            let logged_at =
                NaiveDateTime::parse_from_str(&logged_at_str, "%Y-%m-%d %H:%M:%S%.f")
                    .context("failed to parse logged_at")?;
            let practice_type: PracticeType =
                pt_str.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let sets = self.load_sets(log_id, &practice_type)?;
            entries.push(LogEntry {
                log: Log {
                    id: log_id,
                    practice_id: pid,
                    logged_at,
                    note,
                    warm_up,
                    cool_down,
                },
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    /// Returns `(date_string, count)` pairs for the heatmap.
    pub fn heatmap_counts(&self, days: i64) -> Result<Vec<(String, i64)>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT substr(l.logged_at, 1, 10) AS day, COUNT(*) AS cnt
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1 AND p.active = 1
             GROUP BY day
             ORDER BY day",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        let mut counts = Vec::new();
        for row in rows {
            counts.push(row?);
        }
        Ok(counts)
    }

    /// Returns aggregate statistics over the last `days` days.
    #[allow(dead_code)]
    pub fn aggregate_stats(&self, days: i64) -> Result<AggregateStats> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let sessions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM logs l JOIN practices p ON l.practice_id = p.id WHERE l.logged_at >= ?1 AND p.active = 1",
            params![cutoff.to_string()],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT s.weight, s.reps, s.distance, s.duration
             FROM sets s
             JOIN logs l ON s.log_id = l.id
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1 AND p.active = 1",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?,
                row.get::<_, Option<i32>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?,
            ))
        })?;

        let mut total_volume = 0.0;
        let mut total_reps = 0.0;
        let mut total_distance = 0.0;
        let mut total_duration = 0.0;

        for row in rows {
            let (weight, reps, distance, duration) = row?;
            if let (Some(w), Some(r)) = (weight, reps) {
                total_volume += w * r as f64;
                total_reps += r as f64;
            } else if let Some(r) = reps {
                // Bodyweight: no weight column, just reps
                total_reps += r as f64;
            }
            if let Some(d) = distance {
                total_distance += d;
            }
            if let Some(d) = duration {
                total_duration += d;
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

    // ── Export helpers ──────────────────────────────────────────────────

    /// Exports all log entries (all time).
    pub fn export_all(&self) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, l.warm_up, l.cool_down, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             ORDER BY l.logged_at DESC",
        )?;
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

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, practice_id, logged_at_str, note, warm_up, cool_down, practice_name, pt_str) = row?;
            let logged_at =
                NaiveDateTime::parse_from_str(&logged_at_str, "%Y-%m-%d %H:%M:%S%.f")
                    .context("failed to parse logged_at")?;
            let practice_type: PracticeType =
                pt_str.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            let sets = self.load_sets(log_id, &practice_type)?;
            entries.push(LogEntry {
                log: Log {
                    id: log_id,
                    practice_id,
                    logged_at,
                    note,
                    warm_up,
                    cool_down,
                },
                practice_name,
                practice_type,
                sets,
            });
        }
        Ok(entries)
    }

    /// Check if a log already exists for a practice at a specific time (for import dedup).
    pub fn log_exists(&self, practice_name: &str, logged_at: &NaiveDateTime) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE p.name = ?1 AND l.logged_at = ?2",
            params![practice_name, logged_at.to_string()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn load_sets(&self, log_id: i64, practice_type: &PracticeType) -> Result<Vec<Set>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, log_id, set_number, weight, reps, distance, duration
             FROM sets
             WHERE log_id = ?1
             ORDER BY set_number",
        )?;
        let rows = stmt.query_map(params![log_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<i32>>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, Option<f64>>(6)?,
            ))
        })?;

        let mut sets = Vec::new();
        for row in rows {
            let (id, lid, set_number, weight, reps, distance, duration) = row?;
            let data = match practice_type {
                PracticeType::Weighted => SetData::Weighted {
                    weight: weight.unwrap_or(0.0),
                    reps: reps.unwrap_or(0),
                },
                PracticeType::Bodyweight => SetData::Bodyweight {
                    reps: reps.unwrap_or(0),
                },
                PracticeType::Distance => SetData::Distance {
                    distance: distance.unwrap_or(0.0),
                },
                PracticeType::Endurance => SetData::Endurance {
                    duration: duration.unwrap_or(0.0),
                },
            };
            sets.push(Set {
                id,
                log_id: lid,
                set_number,
                data,
            });
        }
        Ok(sets)
    }

    // ── Goal CRUD ─────────────────────────────────────────────────────

    pub fn create_goal(&self, title: &str) -> Result<i64> {
        let now = Local::now().naive_local();
        self.conn.execute("UPDATE goals SET position = position + 1", [])?;
        self.conn.execute(
            "INSERT INTO goals (title, position, created_at) VALUES (?1, 0, ?2)",
            params![title, now.format("%Y-%m-%d %H:%M:%S%.f").to_string()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, completed, position, created_at, completed_at FROM goals ORDER BY completed, position"
        )?;
        let goals: Vec<Goal> = stmt.query_map([], |row| {
            let created_str: String = row.get(4)?;
            let completed_at_str: Option<String> = row.get(5)?;
            Ok(Goal {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get::<_, i32>(2)? != 0,
                position: row.get(3)?,
                created_at: NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap_or_default(),
                completed_at: completed_at_str
                    .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f").ok()),
                milestones: Vec::new(),
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        let mut result = goals;
        for goal in &mut result {
            let mut ms_stmt = self.conn.prepare(
                "SELECT id, goal_id, title, completed, position, created_at, completed_at \
                 FROM milestones WHERE goal_id = ?1 ORDER BY position"
            )?;
            goal.milestones = ms_stmt.query_map(params![goal.id], |row| {
                let created_str: String = row.get(5)?;
                let completed_at_str: Option<String> = row.get(6)?;
                Ok(Milestone {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    title: row.get(2)?,
                    completed: row.get::<_, i32>(3)? != 0,
                    position: row.get(4)?,
                    created_at: NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%d %H:%M:%S%.f")
                        .unwrap_or_default(),
                    completed_at: completed_at_str
                        .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f").ok()),
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

    pub fn restore_goal(&self, goal: &Goal) -> Result<i64> {
        let goal_id = self.create_goal(&goal.title)?;
        // Restore completion state if it was completed
        if goal.completed {
            self.toggle_goal(goal_id)?;
        }
        // Restore milestones
        for ms in &goal.milestones {
            let ms_id = self.create_milestone(goal_id, &ms.title)?;
            if ms.completed {
                self.toggle_milestone(ms_id)?;
            }
        }
        Ok(goal_id)
    }

    pub fn toggle_goal(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END, \
             completed_at = CASE WHEN completed = 0 THEN ?1 ELSE NULL END \
             WHERE id = ?2",
            params![Local::now().naive_local().format("%Y-%m-%d %H:%M:%S%.f").to_string(), id],
        )?;
        Ok(())
    }

    pub fn set_goal_completed_at(&self, id: i64, completed_at: &NaiveDateTime) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET completed_at = ?1 WHERE id = ?2",
            params![completed_at.format("%Y-%m-%d %H:%M:%S%.f").to_string(), id],
        )?;
        Ok(())
    }

    pub fn set_milestone_completed_at(&self, id: i64, completed_at: &NaiveDateTime) -> Result<()> {
        self.conn.execute(
            "UPDATE milestones SET completed_at = ?1 WHERE id = ?2",
            params![completed_at.format("%Y-%m-%d %H:%M:%S%.f").to_string(), id],
        )?;
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
            "UPDATE milestones SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END, \
             completed_at = CASE WHEN completed = 0 THEN ?1 ELSE NULL END \
             WHERE id = ?2",
            params![Local::now().naive_local().format("%Y-%m-%d %H:%M:%S%.f").to_string(), id],
        )?;
        Ok(())
    }

    pub fn delete_milestone(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM milestones WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn restore_milestone(&self, goal_id: i64, milestone: &Milestone) -> Result<i64> {
        let ms_id = self.create_milestone(goal_id, &milestone.title)?;
        if milestone.completed {
            self.toggle_milestone(ms_id)?;
        }
        Ok(ms_id)
    }

    // ── Quote CRUD ────────────────────────────────────────────────────

    pub fn create_quote(&self, text: &str) -> Result<Quote> {
        let position: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM quotes",
            [],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO quotes (text, position) VALUES (?1, ?2)",
            params![text, position],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Quote { id, text: text.to_string(), position })
    }

    pub fn list_quotes(&self) -> Result<Vec<Quote>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, position FROM quotes ORDER BY position",
        )?;
        let quotes = stmt
            .query_map([], |row| {
                Ok(Quote {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    position: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(quotes)
    }

    pub fn update_quote(&self, id: i64, text: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE quotes SET text = ?1 WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    pub fn delete_quote(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM quotes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn restore_quote(&self, quote: &Quote) -> Result<Quote> {
        self.create_quote(&quote.text)
    }

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

    // ── Abbreviation CRUD ─────────────────────────────────────────

    pub fn create_abbreviation(&self, short: &str, full_name: &str) -> Result<Abbreviation> {
        self.conn.execute(
            "INSERT INTO abbreviations (short, full_name) VALUES (?1, ?2)",
            params![short, full_name],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Abbreviation {
            id,
            short: short.to_string(),
            full_name: full_name.to_string(),
        })
    }

    pub fn list_abbreviations(&self) -> Result<Vec<Abbreviation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, short, full_name FROM abbreviations ORDER BY short")?;
        let rows = stmt.query_map([], |row| {
            Ok(Abbreviation {
                id: row.get(0)?,
                short: row.get(1)?,
                full_name: row.get(2)?,
            })
        })?;
        let mut abbrs = Vec::new();
        for row in rows {
            abbrs.push(row?);
        }
        Ok(abbrs)
    }

    pub fn update_abbreviation(&self, id: i64, short: &str, full_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE abbreviations SET short = ?1, full_name = ?2 WHERE id = ?3",
            params![short, full_name, id],
        )?;
        Ok(())
    }

    pub fn delete_abbreviation(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM abbreviations WHERE id = ?1", params![id])?;
        Ok(())
    }

}
