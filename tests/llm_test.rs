use iron::llm::{build_system_prompt, parse_llm_response};
use iron::model::{Abbreviation, Practice, PracticeType};
use chrono::Local;

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
    ]
}

fn sample_abbreviations() -> Vec<Abbreviation> {
    vec![
        Abbreviation { id: 1, short: "DL".to_string(), full_name: "Deadlift".to_string() },
    ]
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
fn test_parse_valid_response() {
    let json = r#"[
        {
            "practice_name": "Deadlift",
            "sets": [
                {"Weighted": {"weight": 60.0, "reps": 5}},
                {"Weighted": {"weight": 60.0, "reps": 5}}
            ]
        }
    ]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].practice_name, "Deadlift");
    assert!(results[0].matched);
    assert_eq!(results[0].sets.len(), 2);
}

#[test]
fn test_parse_response_in_code_fences() {
    let json = "```json\n[\n{\"practice_name\": \"Deadlift\", \"sets\": [{\"Weighted\": {\"weight\": 60.0, \"reps\": 5}}]}\n]\n```";
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].matched);
}

#[test]
fn test_parse_response_unmatched_practice() {
    let json = r#"[{"practice_name": "Unknown Exercise", "sets": [{"Bodyweight": {"reps": 10}}]}]"#;
    let results = parse_llm_response(json, &sample_practices()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].matched);
}

#[test]
fn test_parse_invalid_json() {
    let result = parse_llm_response("not json at all", &sample_practices());
    assert!(result.is_err());
}
