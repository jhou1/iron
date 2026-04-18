use chrono::NaiveDateTime;
use std::path::PathBuf;

use ironcli::db::Database;
use ironcli::export::{export_to_json, import_from_json};
use ironcli::model::{LogEntry, PracticeType, SetData};

// ── Test Database with setup/teardown ────────────────────────────────────

struct TestDb {
    db: Database,
    _dir: tempfile::TempDir, // dropped (cleaned up) when TestDb drops
}

impl TestDb {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).expect("failed to open test db");
        Self { db, _dir: dir }
    }

    fn export_path(&self) -> PathBuf {
        self._dir.path().join("export.json")
    }
}

// ── Integrity assertion helper ───────────────────────────────────────────

fn assert_log_integrity(
    entry: &LogEntry,
    expected_name: &str,
    expected_type: PracticeType,
    expected_time: &NaiveDateTime,
    expected_note: Option<&str>,
    expected_sets: &[SetData],
) {
    assert_eq!(
        entry.practice_name, expected_name,
        "practice name mismatch for log at {}",
        entry.log.logged_at
    );
    assert_eq!(
        entry.practice_type, expected_type,
        "practice type mismatch for '{}'",
        expected_name
    );
    assert_eq!(
        entry.log.logged_at, *expected_time,
        "logged_at mismatch for '{}'",
        expected_name
    );
    assert_eq!(
        entry.log.note.as_deref(),
        expected_note,
        "note mismatch for '{}' at {}",
        expected_name,
        expected_time
    );
    assert_eq!(
        entry.sets.len(),
        expected_sets.len(),
        "set count mismatch for '{}' at {}",
        expected_name,
        expected_time
    );
    for (i, set) in entry.sets.iter().enumerate() {
        assert_eq!(
            set.set_number,
            (i as i32) + 1,
            "set_number mismatch at index {} for '{}'",
            i,
            expected_name
        );
        assert_eq!(
            set.data, expected_sets[i],
            "set data mismatch at set #{} for '{}'",
            i + 1,
            expected_name
        );
    }
}

fn find_log_by_practice<'a>(entries: &'a [LogEntry], name: &str) -> &'a LogEntry {
    entries
        .iter()
        .find(|e| e.practice_name == name)
        .unwrap_or_else(|| panic!("log for practice '{}' not found", name))
}

fn find_log_by_time<'a>(entries: &'a [LogEntry], time: &NaiveDateTime) -> &'a LogEntry {
    entries
        .iter()
        .find(|e| e.log.logged_at == *time)
        .unwrap_or_else(|| panic!("log at {} not found", time))
}

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .unwrap_or_else(|_| panic!("failed to parse datetime: {}", s))
}

// ── Tests ────────────────────────────────────────────────────────────────

#[test]
fn export_delete_reimport_restores_all_data() {
    let t = TestDb::new();
    let db = &t.db;
    let export_path = t.export_path();

    // Setup: 4 practices, one per type, each with a log
    let kb = db.create_practice("KB Snatch", PracticeType::Weighted).unwrap();
    let pu = db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    let run = db.create_practice("Running", PracticeType::Distance).unwrap();
    let plank = db.create_practice("Plank", PracticeType::Endurance).unwrap();

    let t1 = dt("2025-01-10 08:00:00");
    let t2 = dt("2025-01-11 09:30:00");
    let t3 = dt("2025-01-12 07:15:00");
    let t4 = dt("2025-01-13 18:00:00");

    let kb_sets = vec![
        SetData::Weighted { weight: 24.0, reps: 10 },
        SetData::Weighted { weight: 24.0, reps: 9 },
        SetData::Weighted { weight: 28.0, reps: 8 },
    ];
    let pu_sets = vec![
        SetData::Bodyweight { reps: 20 },
        SetData::Bodyweight { reps: 18 },
    ];
    let run_sets = vec![SetData::Distance { distance: 5.0 }];
    let plank_sets = vec![SetData::Endurance { duration: 2.5 }];

    db.create_log_at(kb.id, &t1, &kb_sets, Some("Felt strong"), None, None).unwrap();
    db.create_log_at(pu.id, &t2, &pu_sets, None, None, None).unwrap();
    db.create_log_at(run.id, &t3, &run_sets, Some("Morning run"), None, None).unwrap();
    db.create_log_at(plank.id, &t4, &plank_sets, None, None, None).unwrap();

    // Pre-export integrity check
    let entries = db.export_all().unwrap();
    assert_eq!(entries.len(), 4);
    assert_log_integrity(find_log_by_practice(&entries, "KB Snatch"), "KB Snatch", PracticeType::Weighted, &t1, Some("Felt strong"), &kb_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Push Up"), "Push Up", PracticeType::Bodyweight, &t2, None, &pu_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Running"), "Running", PracticeType::Distance, &t3, Some("Morning run"), &run_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Plank"), "Plank", PracticeType::Endurance, &t4, None, &plank_sets);

    // Export
    export_to_json(db, Some(export_path.clone())).unwrap();
    assert!(export_path.exists());

    // Delete all practices (cascades to logs/sets)
    let practices = db.list_practices().unwrap();
    for p in &practices {
        db.delete_practice(p.id).unwrap();
    }
    assert_eq!(db.list_practices().unwrap().len(), 0);
    assert_eq!(db.export_all().unwrap().len(), 0);

    // Re-import
    let imported = import_from_json(db, &export_path).unwrap();
    assert_eq!(imported, 4);

    // Post-import integrity check
    let entries = db.export_all().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(db.list_practices().unwrap().len(), 4);
    assert_log_integrity(find_log_by_practice(&entries, "KB Snatch"), "KB Snatch", PracticeType::Weighted, &t1, Some("Felt strong"), &kb_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Push Up"), "Push Up", PracticeType::Bodyweight, &t2, None, &pu_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Running"), "Running", PracticeType::Distance, &t3, Some("Morning run"), &run_sets);
    assert_log_integrity(find_log_by_practice(&entries, "Plank"), "Plank", PracticeType::Endurance, &t4, None, &plank_sets);
}

#[test]
fn partial_delete_reimport_merges_correctly() {
    let t = TestDb::new();
    let db = &t.db;
    let export_path = t.export_path();

    // Setup: 2 practices, 3 logs
    let squat = db.create_practice("Squat", PracticeType::Weighted).unwrap();
    let run = db.create_practice("Running", PracticeType::Distance).unwrap();

    let t1 = dt("2025-02-01 10:00:00");
    let t2 = dt("2025-02-02 10:00:00");
    let t3 = dt("2025-02-03 10:00:00");

    let squat_sets1 = vec![SetData::Weighted { weight: 100.0, reps: 5 }];
    let squat_sets2 = vec![SetData::Weighted { weight: 105.0, reps: 5 }];
    let run_sets = vec![SetData::Distance { distance: 3.0 }];

    let log1_id = db.create_log_at(squat.id, &t1, &squat_sets1, Some("Heavy day"), None, None).unwrap();
    db.create_log_at(squat.id, &t2, &squat_sets2, None, None, None).unwrap();
    db.create_log_at(run.id, &t3, &run_sets, None, None, None).unwrap();

    // Export all 3 logs
    export_to_json(db, Some(export_path.clone())).unwrap();

    // Delete only the first squat log
    db.delete_log(log1_id).unwrap();
    let remaining = db.export_all().unwrap();
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().all(|e| e.log.logged_at != t1), "deleted log should be gone");

    // Practice should still exist
    assert_eq!(db.list_practices().unwrap().len(), 2);

    // Re-import — should restore only the deleted log
    let imported = import_from_json(db, &export_path).unwrap();
    assert_eq!(imported, 1);

    // Verify all 3 logs restored, no duplicate practices
    let entries = db.export_all().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(db.list_practices().unwrap().len(), 2);

    // Verify the restored log has correct data
    let restored = find_log_by_time(&entries, &t1);
    assert_log_integrity(restored, "Squat", PracticeType::Weighted, &t1, Some("Heavy day"), &squat_sets1);

    // Verify existing logs are untouched
    let log2 = find_log_by_time(&entries, &t2);
    assert_log_integrity(log2, "Squat", PracticeType::Weighted, &t2, None, &squat_sets2);
    let log3 = find_log_by_time(&entries, &t3);
    assert_log_integrity(log3, "Running", PracticeType::Distance, &t3, None, &run_sets);
}

#[test]
fn import_skips_duplicates_all_practice_types() {
    let t = TestDb::new();
    let db = &t.db;
    let export_path = t.export_path();

    // Setup: all 4 types
    let kb = db.create_practice("KB Snatch", PracticeType::Weighted).unwrap();
    let pu = db.create_practice("Push Up", PracticeType::Bodyweight).unwrap();
    let run = db.create_practice("Running", PracticeType::Distance).unwrap();
    let plank = db.create_practice("Plank", PracticeType::Endurance).unwrap();

    let t1 = dt("2025-03-01 08:00:00");
    let t2 = dt("2025-03-02 09:00:00");
    let t3 = dt("2025-03-03 10:00:00");
    let t4 = dt("2025-03-04 11:00:00");

    db.create_log_at(kb.id, &t1, &[SetData::Weighted { weight: 24.0, reps: 10 }], None, None, None).unwrap();
    db.create_log_at(pu.id, &t2, &[SetData::Bodyweight { reps: 20 }], Some("Easy"), None, None).unwrap();
    db.create_log_at(run.id, &t3, &[SetData::Distance { distance: 5.0 }], None, None, None).unwrap();
    db.create_log_at(plank.id, &t4, &[SetData::Endurance { duration: 3.0 }], None, None, None).unwrap();

    // Export
    export_to_json(db, Some(export_path.clone())).unwrap();

    // Import into same db — all data still present
    let imported = import_from_json(db, &export_path).unwrap();
    assert_eq!(imported, 0, "all logs are duplicates, none should be imported");

    // Verify no duplicates were created
    assert_eq!(db.export_all().unwrap().len(), 4);
    assert_eq!(db.list_practices().unwrap().len(), 4);
}

#[test]
fn notes_and_timestamps_survive_round_trip() {
    let t = TestDb::new();
    let db = &t.db;
    let export_path = t.export_path();

    let bench = db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let simple_set = vec![SetData::Weighted { weight: 60.0, reps: 8 }];

    let t1 = dt("2025-03-15 06:30:00");
    let t2 = dt("2025-03-15 18:45:30");
    let t3 = dt("2025-03-16 12:00:00");
    let t4 = dt("2025-06-01 23:59:59");

    let note1 = Some("Morning session -- felt strong!");
    let note2: Option<&str> = None;
    let note3 = Some("Line 1\nLine 2");
    let note4 = Some("Unicode: caf\u{00e9} \u{2192} weights");

    db.create_log_at(bench.id, &t1, &simple_set, note1, None, None).unwrap();
    db.create_log_at(bench.id, &t2, &simple_set, note2, None, None).unwrap();
    db.create_log_at(bench.id, &t3, &simple_set, note3, None, None).unwrap();
    db.create_log_at(bench.id, &t4, &simple_set, note4, None, None).unwrap();

    // Export, delete, re-import
    export_to_json(db, Some(export_path.clone())).unwrap();
    db.delete_practice(bench.id).unwrap();
    assert_eq!(db.export_all().unwrap().len(), 0);

    let imported = import_from_json(db, &export_path).unwrap();
    assert_eq!(imported, 4);

    // Verify each note and timestamp
    let entries = db.export_all().unwrap();
    assert_eq!(entries.len(), 4);

    let e1 = find_log_by_time(&entries, &t1);
    assert_eq!(e1.log.note.as_deref(), note1);

    let e2 = find_log_by_time(&entries, &t2);
    assert_eq!(e2.log.note, None, "None note should remain None, not Some(\"\")");

    let e3 = find_log_by_time(&entries, &t3);
    assert_eq!(e3.log.note.as_deref(), note3, "newline in note should survive");

    let e4 = find_log_by_time(&entries, &t4);
    assert_eq!(e4.log.note.as_deref(), note4, "unicode in note should survive");
}

#[test]
fn set_ordering_preserved_across_export_import() {
    let t = TestDb::new();
    let db = &t.db;
    let export_path = t.export_path();

    let bench = db.create_practice("Bench Press", PracticeType::Weighted).unwrap();
    let t1 = dt("2025-04-01 10:00:00");

    // 5 sets with intentionally duplicate values at different positions
    let sets = vec![
        SetData::Weighted { weight: 40.0, reps: 12 },  // warmup
        SetData::Weighted { weight: 60.0, reps: 10 },
        SetData::Weighted { weight: 80.0, reps: 8 },   // working set
        SetData::Weighted { weight: 100.0, reps: 5 },  // top set
        SetData::Weighted { weight: 80.0, reps: 8 },   // backoff — same as set 3
    ];

    db.create_log_at(bench.id, &t1, &sets, Some("Pyramid day"), None, None).unwrap();

    // Export, delete, re-import
    export_to_json(db, Some(export_path.clone())).unwrap();
    db.delete_practice(bench.id).unwrap();
    assert_eq!(db.export_all().unwrap().len(), 0);

    let imported = import_from_json(db, &export_path).unwrap();
    assert_eq!(imported, 1);

    // Verify set ordering
    let entries = db.export_all().unwrap();
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry.sets.len(), 5, "all 5 sets should survive including duplicates");
    assert_log_integrity(entry, "Bench Press", PracticeType::Weighted, &t1, Some("Pyramid day"), &sets);

    // Extra explicit check: sets 3 and 5 have identical data but different set_numbers
    assert_eq!(entry.sets[2].set_number, 3);
    assert_eq!(entry.sets[4].set_number, 5);
    assert_eq!(entry.sets[2].data, entry.sets[4].data, "sets 3 and 5 should have identical data");
}

#[test]
fn goals_and_milestones_survive_export_import() {
    let source = TestDb::new();
    let db = &source.db;

    let g1 = db.create_goal("Master KB Sport").unwrap();
    db.create_milestone(g1, "10-min snatch set").unwrap();
    let m2 = db.create_milestone(g1, "First competition").unwrap();
    db.toggle_milestone(m2).unwrap();

    let g2 = db.create_goal("Run a marathon").unwrap();
    db.create_milestone(g2, "Run 10km under 50min").unwrap();

    let export_path = source._dir.path().join("export.json");
    ironcli::export::export_to_json(db, Some(export_path.clone())).unwrap();

    let target = TestDb::new();
    ironcli::export::import_from_json(&target.db, &export_path).unwrap();

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

#[test]
fn quotes_survive_export_import() {
    let source = TestDb::new();
    let db = &source.db;

    db.create_quote("Anyone can cook!").unwrap();
    db.create_quote("Train hard, rest harder.").unwrap();
    db.create_quote("Consistency beats intensity.").unwrap();

    let export_path = source._dir.path().join("export.json");
    export_to_json(db, Some(export_path.clone())).unwrap();

    let target = TestDb::new();
    import_from_json(&target.db, &export_path).unwrap();

    let quotes = target.db.list_quotes().unwrap();
    assert_eq!(quotes.len(), 3);
    assert_eq!(quotes[0].text, "Anyone can cook!");
    assert_eq!(quotes[1].text, "Train hard, rest harder.");
    assert_eq!(quotes[2].text, "Consistency beats intensity.");
}

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
