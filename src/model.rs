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

    /// Human-friendly label shown in the UI when selecting a practice type.
    pub fn label(&self) -> &'static str {
        match self {
            PracticeType::Weighted => "weightxreps",
            PracticeType::Bodyweight => "reps",
            PracticeType::Distance => "distance",
            PracticeType::Endurance => "duration",
        }
    }
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
    pub fn metric_value(&self) -> f64 {
        match self {
            SetData::Weighted { weight, reps } => weight * (*reps as f64),
            SetData::Bodyweight { reps } => *reps as f64,
            SetData::Distance { distance } => *distance,
            SetData::Endurance { duration } => *duration,
        }
    }

    pub fn metric_label(&self) -> &'static str {
        match self {
            SetData::Weighted { .. } => "kg vol",
            SetData::Bodyweight { .. } => "reps",
            SetData::Distance { .. } => "km",
            SetData::Endurance { .. } => "min",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub log: Log,
    pub practice_name: String,
    #[allow(dead_code)]
    pub practice_type: PracticeType,
    pub sets: Vec<Set>,
}

impl LogEntry {
    pub fn total_metric(&self) -> f64 {
        self.sets.iter().map(|s| s.data.metric_value()).sum()
    }

    pub fn metric_label(&self) -> &'static str {
        self.sets
            .first()
            .map(|s| s.data.metric_label())
            .unwrap_or("—")
    }
}

#[derive(Debug, Clone)]
pub struct Goal {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i32,
    #[allow(dead_code)]
    pub created_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone)]
pub struct Milestone {
    pub id: i64,
    #[allow(dead_code)]
    pub goal_id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i32,
    #[allow(dead_code)]
    pub created_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}
