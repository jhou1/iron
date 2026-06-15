use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_full_config() {
    let toml_str = r#"
[llm]
endpoint = "http://localhost:11434/v1"
api_key = "test-key"
model = "llama3"
"#;
    let config: iron::config::Config = toml::from_str(toml_str).unwrap();
    let llm = config.llm.unwrap();
    assert_eq!(llm.endpoint, "http://localhost:11434/v1");
    assert_eq!(llm.api_key, Some("test-key".to_string()));
    assert_eq!(llm.model, "llama3");
}

#[test]
fn test_parse_config_no_llm_section() {
    let toml_str = "";
    let config: iron::config::Config = toml::from_str(toml_str).unwrap();
    assert!(config.llm.is_none());
}

#[test]
fn test_parse_config_empty_api_key() {
    let toml_str = r#"
[llm]
endpoint = "https://api.openai.com/v1"
api_key = ""
model = "gpt-4o-mini"
"#;
    let config: iron::config::Config = toml::from_str(toml_str).unwrap();
    let llm = config.llm.unwrap();
    assert_eq!(llm.api_key, Some("".to_string()));
}

#[test]
fn test_parse_config_no_api_key() {
    let toml_str = r#"
[llm]
endpoint = "http://localhost:11434/v1"
model = "llama3"
"#;
    let config: iron::config::Config = toml::from_str(toml_str).unwrap();
    let llm = config.llm.unwrap();
    assert!(llm.api_key.is_none());
}

#[test]
fn test_load_from_missing_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nonexistent.toml");
    let config = iron::config::Config::load_from(&path);
    assert!(config.llm.is_none());
}

#[test]
fn test_load_from_valid_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    fs::write(
        &path,
        r#"
[llm]
endpoint = "http://localhost:1234/v1"
model = "test-model"
"#,
    )
    .unwrap();
    let config = iron::config::Config::load_from(&path);
    let llm = config.llm.unwrap();
    assert_eq!(llm.endpoint, "http://localhost:1234/v1");
    assert_eq!(llm.model, "test-model");
}
