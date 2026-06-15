use chrono::Local;
use iron::llm::{build_system_prompt, parse_llm_response};
use iron::model::{Abbreviation, Practice, PracticeType, SetData};

fn sample_practices() -> Vec<Practice> {
    vec![
        Practice {
            id: 1,
            name: "Deadlift".to_string(),
            practice_type: PracticeType::Weighted,
            created_at: Local::now().naive_local(),
            active: true,
        },
        Practice {
            id: 2,
            name: "Pull-ups".to_string(),
            practice_type: PracticeType::Bodyweight,
            created_at: Local::now().naive_local(),
            active: true,
        },
        Practice {
            id: 3,
            name: "Running".to_string(),
            practice_type: PracticeType::Distance,
            created_at: Local::now().naive_local(),
            active: true,
        },
    ]
}

fn sample_abbreviations() -> Vec<Abbreviation> {
    vec![Abbreviation {
        id: 1,
        short: "DL".to_string(),
        full_name: "Deadlift".to_string(),
    }]
}

#[test]
fn test_build_system_prompt_includes_practices() {
    let prompt = build_system_prompt(&sample_practices(), &sample_abbreviations());
    assert!(prompt.contains("Deadlift | weighted"));
    assert!(prompt.contains("Pull-ups | bodyweight"));
}

#[test]
fn test_build_system_prompt_includes_abbreviations() {
    let prompt = build_system_prompt(&sample_practices(), &sample_abbreviations());
    assert!(prompt.contains("DL = Deadlift"));
}

#[test]
fn test_build_system_prompt_no_abbreviations() {
    let prompt = build_system_prompt(&sample_practices(), &[]);
    assert!(prompt.contains("No abbreviations defined"));
}

#[test]
fn test_parse_flat_weighted_response() {
    let json = r#"[
        {
            "practice_name": "Deadlift",
            "practice_type": "weighted",
            "sets": [
                {"weight": 60.0, "reps": 5},
                {"weight": 60.0, "reps": 5}
            ]
        }
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].practice_name, "Deadlift");
    assert!(results[0].matched);
    assert_eq!(results[0].sets.len(), 2);
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 60.0,
            reps: 5
        }
    );
}

#[test]
fn test_parse_flat_bodyweight_response() {
    let json = r#"[
        {
            "practice_name": "Pull-ups",
            "practice_type": "bodyweight",
            "sets": [{"reps": 10}, {"reps": 8}, {"reps": 6}]
        }
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].matched);
    assert_eq!(results[0].sets.len(), 3);
    assert_eq!(results[0].sets[0], SetData::Bodyweight { reps: 10 });
    assert_eq!(results[0].sets[2], SetData::Bodyweight { reps: 6 });
}

#[test]
fn test_parse_response_with_nulls() {
    let json = r#"[
        {
            "practice_name": "Deadlift",
            "practice_type": "weighted",
            "sets": [
                {"weight": 20.0, "reps": 10, "distance": null, "duration": null}
            ]
        }
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 20.0,
            reps: 10
        }
    );
}

#[test]
fn test_parse_response_type_from_practice_list() {
    let json = r#"[
        {
            "practice_name": "Deadlift",
            "sets": [{"weight": 100.0, "reps": 3}]
        }
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 100.0,
            reps: 3
        }
    );
}

#[test]
fn test_parse_response_in_code_fences() {
    let json = "```json\n[{\"practice_name\": \"Deadlift\", \"practice_type\": \"weighted\", \"sets\": [{\"weight\": 60.0, \"reps\": 5}]}]\n```";
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].matched);
}

#[test]
fn test_parse_response_unmatched_practice() {
    let json = r#"[{"practice_name": "Unknown Exercise", "practice_type": "bodyweight", "sets": [{"reps": 10}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].matched);
}

#[test]
fn test_parse_unmatched_no_type_infers_from_fields() {
    let json = r#"[{"practice_name": "Unknown", "sets": [{"weight": 50.0, "reps": 8}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 50.0,
            reps: 8
        }
    );
}

#[test]
fn test_parse_unmatched_reps_only_infers_bodyweight() {
    let json = r#"[{"practice_name": "Unknown", "sets": [{"reps": 15}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results[0].sets[0], SetData::Bodyweight { reps: 15 });
}

#[test]
fn test_parse_distance_response() {
    let json = r#"[{"practice_name": "Running", "practice_type": "distance", "sets": [{"distance": 5.0}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results[0].sets[0], SetData::Distance { distance: 5.0 });
}

#[test]
fn test_parse_invalid_json() {
    let result = parse_llm_response("not json at all", &sample_practices());
    assert!(result.is_err());
}

#[test]
fn test_parse_multi_practice_response() {
    let json = r#"[
        {"practice_name": "Deadlift", "practice_type": "weighted", "sets": [{"weight": 60.0, "reps": 5}]},
        {"practice_name": "Pull-ups", "practice_type": "bodyweight", "sets": [{"reps": 10}, {"reps": 8}]}
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 60.0,
            reps: 5
        }
    );
    assert_eq!(results[1].sets[0], SetData::Bodyweight { reps: 10 });
}

#[test]
fn test_parse_missing_reps_defaults_to_zero() {
    let json = r#"[{"practice_name": "Deadlift", "practice_type": "weighted", "sets": [{"weight": 20.0}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(
        results[0].sets[0],
        SetData::Weighted {
            weight: 20.0,
            reps: 0
        }
    );
}
