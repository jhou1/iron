use crate::config::LlmConfig;
use crate::model::{Abbreviation, ParsedLog, Practice};
use std::fmt;

#[derive(Debug)]
pub enum LlmError {
    NoConfig,
    Network(String),
    Timeout,
    ParseError(String),
    ApiError(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::NoConfig => write!(f, "Configure LLM in ~/.iron/config.toml"),
            LlmError::Network(e) => write!(f, "Network error: {}", e),
            LlmError::Timeout => write!(f, "Request timed out"),
            LlmError::ParseError(e) => write!(f, "Failed to parse LLM response: {}", e),
            LlmError::ApiError(e) => write!(f, "API error: {}", e),
        }
    }
}

pub fn build_system_prompt(practices: &[Practice], abbreviations: &[Abbreviation]) -> String {
    let mut prompt = String::from(
        "You are a training log parser. Convert shorthand training notes into structured JSON.\n\n"
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

    prompt.push_str(r#"
Practice types determine set data format:
- weighted: each set = {"Weighted": {"weight": <float>, "reps": <int>}}
- bodyweight: each set = {"Bodyweight": {"reps": <int>}}
- distance: each set = {"Distance": {"distance": <float>}}  (km)
- endurance: each set = {"Endurance": {"duration": <float>}}  (minutes)

Respond ONLY with a JSON array. Each element:
{
  "practice_name": "<exact name from practice list>",
  "sets": [<set data matching practice type>]
}

Rules:
- Match practice names exactly from the list above
- Use the abbreviation dictionary to resolve shortcuts
- If weight is shared across sets (e.g., "60kg 5/5/5"), apply it to all sets
- Notation like "10/10/10" means separate sets with those rep counts
- If you cannot determine the practice, use the raw text as practice_name
"#);

    prompt
}

pub fn parse_llm_response(
    raw: &str,
    practices: &[Practice],
) -> Result<Vec<ParsedLog>, LlmError> {
    let json_str = extract_json(raw);
    let mut parsed: Vec<ParsedLog> = serde_json::from_str(json_str)
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    for entry in &mut parsed {
        entry.matched = practices
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(&entry.practice_name));
    }

    Ok(parsed)
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
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .new_agent();

    let mut request = agent.post(&url)
        .header("Content-Type", "application/json");

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

    let mut response = request
        .send_json(&body)
        .map_err(|e| {
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
