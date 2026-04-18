use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::model::{PracticeType, SetData};
use crate::i18n::tr_args;
use fluent_bundle::FluentValue;

// ── Export data structures ─────────────────────────────────────────────

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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub warm_up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cool_down: Option<String>,
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

#[derive(Serialize, Deserialize)]
pub struct ExportGoal {
    pub title: String,
    pub position: i32,
    #[serde(default)]
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub milestones: Vec<ExportMilestone>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportMilestone {
    pub title: String,
    pub completed: bool,
    pub position: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportQuote {
    pub text: String,
    pub position: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ExportDailyMetrics {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv: Option<i32>,
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
                warm_up: entry.log.warm_up.clone(),
                cool_down: entry.log.cool_down.clone(),
                sets,
            }
        })
        .collect();

    let goals = db.list_goals()?;
    let export_goals: Vec<ExportGoal> = goals
        .iter()
        .map(|g| ExportGoal {
            title: g.title.clone(),
            position: g.position,
            completed: g.completed,
            completed_at: g.completed_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.f").to_string()),
            milestones: g.milestones
                .iter()
                .map(|m| ExportMilestone {
                    title: m.title.clone(),
                    completed: m.completed,
                    position: m.position,
                    completed_at: m.completed_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.f").to_string()),
                })
                .collect(),
        })
        .collect();

    let quotes = db.list_quotes()?;
    let export_quotes: Vec<ExportQuote> = quotes
        .iter()
        .map(|q| ExportQuote {
            text: q.text.clone(),
            position: q.position,
        })
        .collect();

    let daily_metrics_list = db.list_daily_metrics()?;
    let export_daily_metrics: Vec<ExportDailyMetrics> = daily_metrics_list
        .iter()
        .map(|m| ExportDailyMetrics {
            date: m.date.clone(),
            hrv: m.hrv,
        })
        .collect();

    let data = ExportData {
        version: 2,
        exported_at: Local::now().to_rfc3339(),
        practices: export_practices,
        logs: export_logs,
        goals: export_goals,
        quotes: export_quotes,
        daily_metrics: export_daily_metrics,
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
    eprintln!("{}", tr_args("cli-exported-to", &[
        ("path", FluentValue::from(out_path.display().to_string())),
    ]));
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
            log.warm_up.as_deref(),
            log.cool_down.as_deref(),
        )?;

        imported += 1;
    }

    if !data.goals.is_empty() {
        let existing_goals = db.list_goals()?;
        for g in &existing_goals {
            db.delete_goal(g.id)?;
        }

        for eg in &data.goals {
            let goal_id = db.create_goal(&eg.title)?;
            if eg.completed {
                db.toggle_goal(goal_id)?;
                if let Some(ref ca) = eg.completed_at {
                    if let Ok(dt) = NaiveDateTime::parse_from_str(ca, "%Y-%m-%d %H:%M:%S%.f") {
                        db.set_goal_completed_at(goal_id, &dt)?;
                    }
                }
            }
            for em in &eg.milestones {
                let ms_id = db.create_milestone(goal_id, &em.title)?;
                if em.completed {
                    db.toggle_milestone(ms_id)?;
                    if let Some(ref ca) = em.completed_at {
                        if let Ok(dt) = NaiveDateTime::parse_from_str(ca, "%Y-%m-%d %H:%M:%S%.f") {
                            db.set_milestone_completed_at(ms_id, &dt)?;
                        }
                    }
                }
            }
        }
    }

    if !data.quotes.is_empty() {
        let existing_quotes = db.list_quotes()?;
        for q in &existing_quotes {
            db.delete_quote(q.id)?;
        }

        for eq in &data.quotes {
            db.create_quote(&eq.text)?;
        }
    }

    for dm in &data.daily_metrics {
        if let Some(hrv) = dm.hrv {
            let _ = db.set_daily_hrv(&dm.date, hrv);
        }
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
