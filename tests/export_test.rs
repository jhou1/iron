use ironcli::db::Database;
use ironcli::export::{export_to_json, import_from_json};
use ironcli::model::{PracticeType, SetData};

#[test]
fn round_trip_export_import() {
    let db1 = Database::open_in_memory().unwrap();

    // Create practices of different types
    let snatch = db1
        .create_practice("Kettlebell Snatch", PracticeType::Weighted)
        .unwrap();
    let pushups = db1
        .create_practice("Push-ups", PracticeType::Bodyweight)
        .unwrap();
    let run = db1
        .create_practice("5K Run", PracticeType::Distance)
        .unwrap();
    let plank = db1
        .create_practice("Plank Hold", PracticeType::Endurance)
        .unwrap();

    // Create logs with sets
    db1.create_log(
        snatch.id,
        &[
            SetData::Weighted {
                weight: 24.0,
                reps: 10,
            },
            SetData::Weighted {
                weight: 24.0,
                reps: 9,
            },
        ],
        Some("Felt great"),
    )
    .unwrap();

    db1.create_log(pushups.id, &[SetData::Bodyweight { reps: 20 }], None)
        .unwrap();

    db1.create_log(
        run.id,
        &[SetData::Distance { distance: 5.2 }],
        Some("Morning run"),
    )
    .unwrap();

    db1.create_log(
        plank.id,
        &[SetData::Endurance { duration: 2.5 }],
        None,
    )
    .unwrap();

    // Export to temp file
    let tmp_dir = tempfile::tempdir().unwrap();
    let export_path = tmp_dir.path().join("export.json");
    export_to_json(&db1, Some(export_path.clone())).unwrap();

    assert!(export_path.exists());

    // Import into a fresh database
    let db2 = Database::open_in_memory().unwrap();
    let imported = import_from_json(&db2, &export_path).unwrap();
    assert_eq!(imported, 4);

    // Verify practices were created
    let practices = db2.list_practices().unwrap();
    assert_eq!(practices.len(), 4);

    let practice_names: Vec<&str> = practices.iter().map(|p| p.name.as_str()).collect();
    assert!(practice_names.contains(&"Kettlebell Snatch"));
    assert!(practice_names.contains(&"Push-ups"));
    assert!(practice_names.contains(&"5K Run"));
    assert!(practice_names.contains(&"Plank Hold"));

    // Verify logs were imported
    let logs = db2.export_all().unwrap();
    assert_eq!(logs.len(), 4);

    // Find the snatch log and verify sets
    let snatch_log = logs
        .iter()
        .find(|l| l.practice_name == "Kettlebell Snatch")
        .unwrap();
    assert_eq!(snatch_log.sets.len(), 2);
    assert_eq!(
        snatch_log.sets[0].data,
        SetData::Weighted {
            weight: 24.0,
            reps: 10
        }
    );
    assert_eq!(
        snatch_log.sets[1].data,
        SetData::Weighted {
            weight: 24.0,
            reps: 9
        }
    );
    assert_eq!(snatch_log.log.note, Some("Felt great".to_string()));

    // Verify bodyweight log
    let pushup_log = logs
        .iter()
        .find(|l| l.practice_name == "Push-ups")
        .unwrap();
    assert_eq!(pushup_log.sets.len(), 1);
    assert_eq!(pushup_log.sets[0].data, SetData::Bodyweight { reps: 20 });
    assert_eq!(pushup_log.log.note, None);

    // Verify distance log
    let run_log = logs.iter().find(|l| l.practice_name == "5K Run").unwrap();
    assert_eq!(run_log.sets.len(), 1);
    assert_eq!(run_log.sets[0].data, SetData::Distance { distance: 5.2 });

    // Verify endurance log
    let plank_log = logs
        .iter()
        .find(|l| l.practice_name == "Plank Hold")
        .unwrap();
    assert_eq!(plank_log.sets.len(), 1);
    assert_eq!(
        plank_log.sets[0].data,
        SetData::Endurance { duration: 2.5 }
    );
}

#[test]
fn import_skips_duplicates() {
    let db = Database::open_in_memory().unwrap();

    let practice = db
        .create_practice("Squat", PracticeType::Weighted)
        .unwrap();
    db.create_log(
        practice.id,
        &[SetData::Weighted {
            weight: 100.0,
            reps: 5,
        }],
        Some("Heavy day"),
    )
    .unwrap();

    // Export
    let tmp_dir = tempfile::tempdir().unwrap();
    let export_path = tmp_dir.path().join("export.json");
    export_to_json(&db, Some(export_path.clone())).unwrap();

    // Import into the same database -- should skip all duplicates
    let imported = import_from_json(&db, &export_path).unwrap();
    assert_eq!(imported, 0);

    // Verify no duplicates were created
    let logs = db.export_all().unwrap();
    assert_eq!(logs.len(), 1);
}
