use ironcli::db::Database;
use ironcli::model::{PracticeType, SetData};

#[test]
fn create_and_list_practices() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    db.create_practice("Pull-ups", PracticeType::Bodyweight).unwrap();

    let practices = db.list_practices().unwrap();
    assert_eq!(practices.len(), 2);
    assert_eq!(practices[0].name, "Bench Press");
    assert_eq!(practices[0].practice_type, PracticeType::Weighted);
    assert_eq!(practices[1].name, "Pull-ups");
    assert_eq!(practices[1].practice_type, PracticeType::Bodyweight);
}

#[test]
fn create_practice_duplicate_name_fails() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Squat", PracticeType::Weighted).unwrap();
    let result = db.create_practice("Squat", PracticeType::Weighted);
    assert!(result.is_err());
}

#[test]
fn rename_practice() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Squats", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let id = practices[0].id;

    db.rename_practice(id, "Back Squat").unwrap();

    let practices = db.list_practices().unwrap();
    assert_eq!(practices[0].name, "Back Squat");
}

#[test]
fn delete_practice() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Deadlift", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let id = practices[0].id;

    db.delete_practice(id).unwrap();

    let practices = db.list_practices().unwrap();
    assert!(practices.is_empty());
}

#[test]
fn create_log_with_sets() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![
        SetData::Weighted { weight: 60.0, reps: 10 },
        SetData::Weighted { weight: 80.0, reps: 8 },
        SetData::Weighted { weight: 100.0, reps: 5 },
    ];

    db.create_log(practice_id, &sets, Some("Felt good")).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].practice_name, "Bench Press");
    assert_eq!(entries[0].sets.len(), 3);
    assert_eq!(entries[0].log.note, Some("Felt good".to_string()));
    assert_eq!(entries[0].sets[0].data, SetData::Weighted { weight: 60.0, reps: 10 });
    assert_eq!(entries[0].sets[1].data, SetData::Weighted { weight: 80.0, reps: 8 });
    assert_eq!(entries[0].sets[2].data, SetData::Weighted { weight: 100.0, reps: 5 });
}

#[test]
fn update_log_sets_and_note() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Squat", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![
        SetData::Weighted { weight: 60.0, reps: 10 },
    ];
    db.create_log(practice_id, &sets, Some("First attempt")).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    let log_id = entries[0].log.id;

    let new_sets = vec![
        SetData::Weighted { weight: 80.0, reps: 8 },
        SetData::Weighted { weight: 100.0, reps: 5 },
    ];
    db.update_log(log_id, &new_sets, Some("Updated")).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    assert_eq!(entries[0].sets.len(), 2);
    assert_eq!(entries[0].log.note, Some("Updated".to_string()));
    assert_eq!(entries[0].sets[0].data, SetData::Weighted { weight: 80.0, reps: 8 });
    assert_eq!(entries[0].sets[1].data, SetData::Weighted { weight: 100.0, reps: 5 });
}

#[test]
fn delete_log() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Run", PracticeType::Distance).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![SetData::Distance { distance: 5.0 }];
    db.create_log(practice_id, &sets, None).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    let log_id = entries[0].log.id;

    db.delete_log(log_id).unwrap();

    let entries = db.list_logs_recent(1).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn heatmap_data() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Push-ups", PracticeType::Bodyweight).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets = vec![SetData::Bodyweight { reps: 20 }];
    db.create_log(practice_id, &sets, None).unwrap();
    db.create_log(practice_id, &sets, None).unwrap();

    let counts = db.heatmap_counts(30).unwrap();
    // Today's date should have count 2
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let today_count = counts.iter().find(|(d, _)| d == &today);
    assert!(today_count.is_some());
    assert_eq!(today_count.unwrap().1, 2);
}

#[test]
fn logs_for_practice_trend() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Deadlift", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let practice_id = practices[0].id;

    let sets1 = vec![SetData::Weighted { weight: 100.0, reps: 5 }];
    let sets2 = vec![SetData::Weighted { weight: 110.0, reps: 5 }];
    db.create_log(practice_id, &sets1, None).unwrap();
    db.create_log(practice_id, &sets2, None).unwrap();

    let entries = db.list_logs_for_practice(practice_id, 30).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn fourteen_day_stats() {
    let db = Database::open_in_memory().unwrap();

    // Create a weighted practice with one log
    db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let practices = db.list_practices().unwrap();
    let bench_id = practices[0].id;

    let weighted_sets = vec![
        SetData::Weighted { weight: 60.0, reps: 10 }, // volume = 600
        SetData::Weighted { weight: 80.0, reps: 5 },  // volume = 400
    ];
    db.create_log(bench_id, &weighted_sets, None).unwrap();

    // Create a distance practice with one log
    db.create_practice("Run", PracticeType::Distance).unwrap();
    let practices = db.list_practices().unwrap();
    let run_id = practices.iter().find(|p| p.name == "Run").unwrap().id;

    let distance_sets = vec![
        SetData::Distance { distance: 5.0 },
    ];
    db.create_log(run_id, &distance_sets, None).unwrap();

    let stats = db.aggregate_stats(14).unwrap();
    assert_eq!(stats.sessions, 2);
    assert!((stats.total_volume - 1000.0).abs() < f64::EPSILON);
    assert!((stats.total_reps - 15.0).abs() < f64::EPSILON);
    assert!((stats.total_distance - 5.0).abs() < f64::EPSILON);
    assert!((stats.total_duration - 0.0).abs() < f64::EPSILON);
}
