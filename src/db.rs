use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::model::{Log, LogEntry, Practice, PracticeType, Set, SetData};

/// Aggregate statistics over a time period.
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
    /// Opens the default database at `~/.ironcli/iron.db`, creating it if needed.
    pub fn open_default() -> Result<Self> {
        let dir = dirs::home_dir()
            .context("could not determine home directory")?
            .join(".ironcli");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("iron.db");
        Self::open(&path)
    }

    /// Opens a database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Opens an in-memory database (for tests).
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
            );",
        )?;
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
        })
    }

    pub fn list_practices(&self) -> Result<Vec<Practice>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, practice_type, created_at FROM practices ORDER BY name")?;
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
            });
        }
        Ok(practices)
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

    /// Creates a log with a specific timestamp (used by import).
    pub fn create_log_at(
        &self,
        practice_id: i64,
        logged_at: &NaiveDateTime,
        sets: &[SetData],
        note: Option<&str>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO logs (practice_id, logged_at, note) VALUES (?1, ?2, ?3)",
            params![practice_id, logged_at.to_string(), note],
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
    ) -> Result<()> {
        if let Some(dt) = logged_at {
            self.conn.execute(
                "UPDATE logs SET note = ?1, logged_at = ?2 WHERE id = ?3",
                params![note, dt.to_string(), log_id],
            )?;
        } else {
            self.conn
                .execute("UPDATE logs SET note = ?1 WHERE id = ?2", params![note, log_id])?;
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

    pub fn list_logs_recent(&self, days: i64) -> Result<Vec<LogEntry>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1
             ORDER BY l.logged_at DESC",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, practice_id, logged_at_str, note, practice_name, pt_str) = row?;
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
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
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
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, pid, logged_at_str, note, practice_name, pt_str) = row?;
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
            "SELECT substr(logged_at, 1, 10) AS day, COUNT(*) AS cnt
             FROM logs
             WHERE logged_at >= ?1
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
    pub fn aggregate_stats(&self, days: i64) -> Result<AggregateStats> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let sessions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM logs WHERE logged_at >= ?1",
            params![cutoff.to_string()],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT s.weight, s.reps, s.distance, s.duration
             FROM sets s
             JOIN logs l ON s.log_id = l.id
             WHERE l.logged_at >= ?1",
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
            "SELECT l.id, l.practice_id, l.logged_at, l.note, p.name, p.practice_type
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
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (log_id, practice_id, logged_at_str, note, practice_name, pt_str) = row?;
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
}
