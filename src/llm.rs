use crate::config::LlmConfig;
use crate::model::{Abbreviation, ParsedLog, Practice, PracticeType, RawParsedLog, SetData};
use std::fmt;

#[derive(Debug)]
pub enum LlmError {
    Network(String),
    Timeout,
    ParseError(String),
    ApiError(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Network(e) => write!(f, "Network error: {}", e),
            LlmError::Timeout => write!(f, "Request timed out"),
            LlmError::ParseError(e) => write!(f, "Failed to parse LLM response: {}", e),
            LlmError::ApiError(e) => write!(f, "API error: {}", e),
        }
    }
}

pub fn build_system_prompt(practices: &[Practice], abbreviations: &[Abbreviation]) -> String {
    let mut prompt = String::from(
        "You are a training log parser. Convert shorthand training notes into structured JSON.\n\n",
    );

    prompt.push_str("Available practices (name | type):\n");
    for p in practices {
        prompt.push_str(&format!("- {} | {}\n", p.name, p.practice_type));
    }

    prompt.push('\n');
    if abbreviations.is_empty() {
        prompt.push_str("No abbreviations defined.\n");
    } else {
        prompt.push_str("Abbreviation dictionary:\n");
        for a in abbreviations {
            prompt.push_str(&format!("- {} = {}\n", a.short, a.full_name));
        }
    }

    prompt.push_str(
        r#"
Respond ONLY with a JSON array. Each element:
{
  "practice_name": "<exact name from practice list>",
  "practice_type": "<weighted|bodyweight|distance|endurance>",
  "sets": [<set data>]
}

Set fields by practice type:
- weighted: {"weight": <number>, "reps": <number>}
- bodyweight: {"reps": <number>}
- distance: {"distance": <number>}  (km)
- endurance: {"duration": <number>}  (minutes)

Rules:
- Match practice names exactly from the list above
- Use the abbreviation dictionary to resolve shortcuts
- If weight is shared across sets (e.g., "60kg 5/5/5"), apply it to all sets
- Notation like "10/10/10" means separate sets with those rep counts
- If you cannot determine the practice, use the raw text as practice_name
"#,
    );

    prompt
}

pub fn parse_llm_response(raw: &str, practices: &[Practice]) -> Result<Vec<ParsedLog>, LlmError> {
    let json_str = extract_json(raw);
    let raw_logs: Vec<RawParsedLog> =
        serde_json::from_str(json_str).map_err(|e| LlmError::ParseError(e.to_string()))?;

    let mut results = Vec::new();
    for entry in raw_logs {
        let matched_practice = practices
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&entry.practice_name));

        let practice_type = resolve_practice_type(&entry.practice_type, matched_practice);

        let sets: Vec<SetData> = entry
            .sets
            .iter()
            .filter_map(|raw_set| match practice_type {
                Some(PracticeType::Weighted) => Some(SetData::Weighted {
                    weight: raw_set.weight.unwrap_or(0.0),
                    reps: raw_set.reps.unwrap_or(0),
                }),
                Some(PracticeType::Bodyweight) => Some(SetData::Bodyweight {
                    reps: raw_set.reps.unwrap_or(0),
                }),
                Some(PracticeType::Distance) => Some(SetData::Distance {
                    distance: raw_set.distance.unwrap_or(0.0),
                }),
                Some(PracticeType::Endurance) => Some(SetData::Endurance {
                    duration: raw_set.duration.unwrap_or(0.0),
                }),
                None => match (
                    raw_set.weight,
                    raw_set.reps,
                    raw_set.distance,
                    raw_set.duration,
                ) {
                    (Some(weight), Some(reps), _, _) => Some(SetData::Weighted { weight, reps }),
                    (None, Some(reps), _, _) => Some(SetData::Bodyweight { reps }),
                    (_, _, Some(distance), _) => Some(SetData::Distance { distance }),
                    (_, _, _, Some(duration)) => Some(SetData::Endurance { duration }),
                    _ => None,
                },
            })
            .collect();

        results.push(ParsedLog {
            practice_name: entry.practice_name,
            sets,
            matched: matched_practice.is_some(),
        });
    }

    Ok(results)
}

fn resolve_practice_type(
    raw_type: &Option<String>,
    matched_practice: Option<&Practice>,
) -> Option<PracticeType> {
    if let Some(pt_str) = raw_type {
        if let Ok(pt) = pt_str.parse::<PracticeType>() {
            return Some(pt);
        }
    }
    matched_practice.map(|p| p.practice_type)
}

fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_fence[content_start..];
        if let Some(end) = content.find("```") {
            return content[..end].trim();
        }
    }
    trimmed
}

pub fn call_llm(
    config: &LlmConfig,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, LlmError> {
    let url = format!("{}/chat/completions", config.endpoint.trim_end_matches('/'));

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(config.timeout_secs())))
        .build()
        .new_agent();

    let mut request = agent.post(&url).header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            request = request.header("Authorization", &format!("Bearer {}", key));
        }
    }

    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_message}
        ],
        "temperature": 0.0
    });

    let mut response = request.send_json(&body).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("timed out") || msg.contains("timeout") {
            LlmError::Timeout
        } else {
            LlmError::Network(msg)
        }
    })?;

    let response_body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    response_body["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| LlmError::ApiError("No content in response".to_string()))
}
