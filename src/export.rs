use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::model::{PracticeType, SetData};

// ── Export data structures ─────────────────────────────────────────────

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

// ── Export ──────────────────────────────────────────────────────────────

pub fn export_to_json(db: &Database, path: Option<PathBuf>) -> Result<()> {
    let practices = db.list_practices()?;
    let log_entries = db.export_all()?;

    let export_practices: Vec<ExportPractice> = practices
        .iter()
        .map(|p| ExportPractice {
            id: p.id,
            name: p.name.clone(),
            practice_type: p.practice_type,
            created_at: p.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();

    let export_logs: Vec<ExportLog> = log_entries
        .iter()
        .map(|entry| {
            let sets = entry
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
                logged_at: entry.log.logged_at.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
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

    let json = serde_json::to_string_pretty(&data)?;

    let out_path = match path {
        Some(p) => p,
        None => {
            let dir = dirs::home_dir()
                .context("could not determine home directory")?
                .join(".ironcli");
            std::fs::create_dir_all(&dir)?;
            let date = Local::now().format("%Y-%m-%d").to_string();
            dir.join(format!("iron-export-{}.json", date))
        }
    };

    std::fs::write(&out_path, json)?;
    eprintln!("Exported to {}", out_path.display());
    Ok(())
}

// ── Import ─────────────────────────────────────────────────────────────

pub fn import_from_json(db: &Database, path: &Path) -> Result<usize> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let data: ExportData =
        serde_json::from_str(&json).context("failed to parse export JSON")?;

    // Build a map of practice name -> id, creating missing practices.
    let mut practice_map: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();

    // Load existing practices first
    for p in db.list_practices()? {
        practice_map.insert(p.name.clone(), p.id);
    }

    // Ensure every practice from the export exists
    for ep in &data.practices {
        if !practice_map.contains_key(&ep.name) {
            let created = db.create_practice(&ep.name, ep.practice_type)?;
            practice_map.insert(created.name.clone(), created.id);
        }
    }

    // Also ensure practices referenced by logs exist (in case the export
    // has logs referencing practices not in the practices array).
    for log in &data.logs {
        if !practice_map.contains_key(&log.practice) {
            // Infer practice type from the first set
            let pt = infer_practice_type_from_sets(&log.sets);
            let created = db.create_practice(&log.practice, pt)?;
            practice_map.insert(created.name.clone(), created.id);
        }
    }

    let mut imported = 0;

    for log in &data.logs {
        let logged_at = NaiveDateTime::parse_from_str(&log.logged_at, "%Y-%m-%d %H:%M:%S%.f")
            .with_context(|| format!("failed to parse logged_at: {}", log.logged_at))?;

        // Skip duplicates
        if db.log_exists(&log.practice, &logged_at)? {
            continue;
        }

        let practice_id = *practice_map
            .get(&log.practice)
            .context("practice not found in map (unexpected)")?;

        let sets: Vec<SetData> = log
            .sets
            .iter()
            .map(reconstruct_set_data)
            .collect();

        db.create_log_at(
            practice_id,
            &logged_at,
            &sets,
            log.note.as_deref(),
        )?;

        imported += 1;
    }

    Ok(imported)
}

/// Reconstruct SetData from flat export fields.
fn reconstruct_set_data(s: &ExportSet) -> SetData {
    if let (Some(weight), Some(reps)) = (s.weight, s.reps) {
        SetData::Weighted { weight, reps }
    } else if let Some(reps) = s.reps {
        SetData::Bodyweight { reps }
    } else if let Some(distance) = s.distance {
        SetData::Distance { distance }
    } else if let Some(duration) = s.duration {
        SetData::Endurance { duration }
    } else {
        // Fallback: bodyweight with 0 reps
        SetData::Bodyweight { reps: 0 }
    }
}

/// Infer practice type from a set of export sets.
fn infer_practice_type_from_sets(sets: &[ExportSet]) -> PracticeType {
    if let Some(s) = sets.first() {
        if s.weight.is_some() && s.reps.is_some() {
            PracticeType::Weighted
        } else if s.reps.is_some() {
            PracticeType::Bodyweight
        } else if s.distance.is_some() {
            PracticeType::Distance
        } else if s.duration.is_some() {
            PracticeType::Endurance
        } else {
            PracticeType::Bodyweight
        }
    } else {
        PracticeType::Bodyweight
    }
}
