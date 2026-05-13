# Quick Log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Quick Log" screen that lets users type training notes in shorthand, have an LLM parse them via OpenAI-compatible API, preview the structured result, and save multiple log entries at once.

**Architecture:** New config system (`config.toml`), LLM client (`ureq` + background thread + `mpsc`), abbreviation dictionary (SQLite + CRUD screen), QuickLog screen (multi-line input + side-by-side preview). Event loop changes from blocking `event::read()` to `event::poll(100ms)` to support background result checking.

**Tech Stack:** Rust, ratatui, ureq (HTTP), toml (config), rusqlite, std::thread + mpsc

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/config.rs` (create) | Load `~/.iron/config.toml`, expose `Config` and `LlmConfig` structs |
| `src/llm.rs` (create) | OpenAI-compatible chat completions client, prompt construction, JSON response parsing |
| `src/tui/quick_log.rs` (create) | QuickLog screen: multi-line text input, background LLM dispatch, preview pane, save flow |
| `src/tui/abbreviations.rs` (create) | Abbreviation dictionary CRUD screen (list/add/edit/delete) |
| `src/model.rs` (modify) | Add `Abbreviation` and `ParsedLog` structs |
| `src/db.rs` (modify) | Add `abbreviations` table + CRUD methods |
| `src/tui/mod.rs` (modify) | Add `QuickLog`, `Abbreviations` to `Screen` enum |
| `src/app.rs` (modify) | Add screen routing, poll-based event loop |
| `src/tui/dashboard.rs` (modify) | Add `[w]` keybinding |
| `src/main.rs` (modify) | Add `mod config; mod llm;` |
| `src/lib.rs` (modify) | Add `pub mod config; pub mod llm;` |
| `Cargo.toml` (modify) | Add `ureq` (with `json` feature), `toml` dependencies |
| `locales/en.ftl` (modify) | Add i18n keys for QuickLog and Abbreviations screens |
| `locales/zh-CN.ftl` (modify) | Add Chinese i18n keys |

---

### Task 1: Add Dependencies and Module Declarations

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add ureq and toml to Cargo.toml**

In `Cargo.toml`, add to the `[dependencies]` section:

```toml
ureq = { version = "3", features = ["json"] }
toml = "0.8"
```

- [ ] **Step 2: Add module declarations to main.rs**

In `src/main.rs`, after line `mod i18n;` (line 8), add:

```rust
mod config;
mod llm;
```

- [ ] **Step 3: Add module declarations to lib.rs**

In `src/lib.rs`, after `pub mod i18n;` (line 3), add:

```rust
pub mod config;
pub mod llm;
```

- [ ] **Step 4: Create placeholder files**

Create `src/config.rs` and `src/llm.rs` as empty files so the project compiles:

```rust
// src/config.rs
```

```rust
// src/llm.rs
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors (warnings about unused modules are fine)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/lib.rs src/config.rs src/llm.rs
git commit -m "feat(quick-log): add ureq, toml deps and config/llm module stubs"
```

---

### Task 2: Config System

**Files:**
- Create: `src/config.rs`
- Test: `tests/config_test.rs`

- [ ] **Step 1: Write failing tests for config loading**

Create `tests/config_test.rs`:

```rust
use std::fs;
use tempfile::TempDir;

// We test the parsing logic directly since Config::load() reads from ~/.iron/
// Instead, test the deserialization.

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
    fs::write(&path, r#"
[llm]
endpoint = "http://localhost:1234/v1"
model = "test-model"
"#).unwrap();
    let config = iron::config::Config::load_from(&path);
    let llm = config.llm.unwrap();
    assert_eq!(llm.endpoint, "http://localhost:1234/v1");
    assert_eq!(llm.model, "test-model");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test config_test`
Expected: compilation errors — `Config` and `LlmConfig` not defined yet

- [ ] **Step 3: Implement config.rs**

Write `src/config.rs`:

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub llm: Option<LlmConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl Config {
    pub fn load() -> Self {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return Self::default(),
        };
        let path = home.join(".iron").join("config.toml");
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: all 6 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "feat(quick-log): add config system for ~/.iron/config.toml"
```

---

### Task 3: Abbreviation Model and Database

**Files:**
- Modify: `src/model.rs` (add `Abbreviation` struct)
- Modify: `src/db.rs` (add table + CRUD)
- Test: `tests/db_test.rs` (add abbreviation tests)

- [ ] **Step 1: Add Abbreviation struct to model.rs**

In `src/model.rs`, after the `DailyMetrics` struct (after line 175), add:

```rust
#[derive(Debug, Clone)]
pub struct Abbreviation {
    pub id: i64,
    pub short: String,
    pub full_name: String,
}
```

- [ ] **Step 2: Add abbreviations table to db.rs init_schema**

In `src/db.rs`, inside `init_schema()`, after the `daily_metrics` table creation (before the closing `";` on line 117), add:

```sql

            CREATE TABLE IF NOT EXISTS abbreviations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                short TEXT NOT NULL UNIQUE COLLATE NOCASE,
                full_name TEXT NOT NULL
            );
```

Also add the `Abbreviation` import in line 6:

```rust
use crate::model::{Abbreviation, Goal, Log, LogEntry, Milestone, Practice, PracticeType, Quote, Set, SetData};
```

- [ ] **Step 3: Add CRUD methods to db.rs**

At the end of the `impl Database` block in `src/db.rs`, before the closing `}`, add:

```rust
    // ── Abbreviation CRUD ─────────────────────────────────────────

    pub fn create_abbreviation(&self, short: &str, full_name: &str) -> Result<Abbreviation> {
        self.conn.execute(
            "INSERT INTO abbreviations (short, full_name) VALUES (?1, ?2)",
            params![short, full_name],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Abbreviation {
            id,
            short: short.to_string(),
            full_name: full_name.to_string(),
        })
    }

    pub fn list_abbreviations(&self) -> Result<Vec<Abbreviation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, short, full_name FROM abbreviations ORDER BY short")?;
        let rows = stmt.query_map([], |row| {
            Ok(Abbreviation {
                id: row.get(0)?,
                short: row.get(1)?,
                full_name: row.get(2)?,
            })
        })?;
        let mut abbrs = Vec::new();
        for row in rows {
            abbrs.push(row?);
        }
        Ok(abbrs)
    }

    pub fn update_abbreviation(&self, id: i64, short: &str, full_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE abbreviations SET short = ?1, full_name = ?2 WHERE id = ?3",
            params![short, full_name, id],
        )?;
        Ok(())
    }

    pub fn delete_abbreviation(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM abbreviations WHERE id = ?1", params![id])?;
        Ok(())
    }
```

- [ ] **Step 4: Write tests for abbreviation CRUD**

Add to `tests/db_test.rs`:

```rust
#[test]
fn test_abbreviation_crud() {
    let (_dir, db) = test_db();

    // Create
    let abbr = db.create_abbreviation("DL", "Deadlift").unwrap();
    assert_eq!(abbr.short, "DL");
    assert_eq!(abbr.full_name, "Deadlift");

    // List
    let abbrs = db.list_abbreviations().unwrap();
    assert_eq!(abbrs.len(), 1);
    assert_eq!(abbrs[0].short, "DL");

    // Update
    db.update_abbreviation(abbr.id, "DL", "Dead Lift").unwrap();
    let abbrs = db.list_abbreviations().unwrap();
    assert_eq!(abbrs[0].full_name, "Dead Lift");

    // Delete
    db.delete_abbreviation(abbr.id).unwrap();
    let abbrs = db.list_abbreviations().unwrap();
    assert!(abbrs.is_empty());
}

#[test]
fn test_abbreviation_unique_constraint() {
    let (_dir, db) = test_db();
    db.create_abbreviation("DL", "Deadlift").unwrap();
    let result = db.create_abbreviation("DL", "Something else");
    assert!(result.is_err());
}

#[test]
fn test_abbreviation_case_insensitive() {
    let (_dir, db) = test_db();
    db.create_abbreviation("DL", "Deadlift").unwrap();
    let result = db.create_abbreviation("dl", "Something else");
    assert!(result.is_err());
}

#[test]
fn test_abbreviation_list_ordered() {
    let (_dir, db) = test_db();
    db.create_abbreviation("KB SW", "Kettlebell Swing").unwrap();
    db.create_abbreviation("BP", "Bench Press").unwrap();
    db.create_abbreviation("DL", "Deadlift").unwrap();
    let abbrs = db.list_abbreviations().unwrap();
    assert_eq!(abbrs[0].short, "BP");
    assert_eq!(abbrs[1].short, "DL");
    assert_eq!(abbrs[2].short, "KB SW");
}
```

Note: `test_db()` is the existing helper in `tests/db_test.rs` that creates a temp database. Check that it returns `(TempDir, Database)` or `(TestDb, Database)` and match the pattern used by existing tests.

- [ ] **Step 5: Run tests**

Run: `cargo test --test db_test`
Expected: all tests pass (existing + 4 new)

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/db.rs tests/db_test.rs
git commit -m "feat(quick-log): add abbreviation model and database CRUD"
```

---

### Task 4: ParsedLog Model

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Add ParsedLog struct**

In `src/model.rs`, after the `Abbreviation` struct, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLog {
    pub practice_name: String,
    pub sets: Vec<SetData>,
    #[serde(default)]
    pub matched: bool,
}
```

Note: `matched` has `#[serde(default)]` because the LLM won't return it — it's computed after deserialization by validating against the practice list.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add src/model.rs
git commit -m "feat(quick-log): add ParsedLog struct for LLM response parsing"
```

---

### Task 5: LLM Client

**Files:**
- Create: `src/llm.rs`
- Test: `tests/llm_test.rs`

- [ ] **Step 1: Write tests for prompt construction and response parsing**

Create `tests/llm_test.rs`:

```rust
use iron::llm::{build_system_prompt, parse_llm_response};
use iron::model::{Abbreviation, Practice, PracticeType, SetData};
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test llm_test`
Expected: compilation errors — functions not defined yet

- [ ] **Step 3: Implement llm.rs**

Write `src/llm.rs`:

```rust
use crate::config::LlmConfig;
use crate::model::{Abbreviation, ParsedLog, Practice, SetData};
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test llm_test`
Expected: all 7 tests pass

- [ ] **Step 5: Run cargo clippy**

Run: `cargo clippy`
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add src/llm.rs tests/llm_test.rs
git commit -m "feat(quick-log): add LLM client with prompt construction and response parsing"
```

---

### Task 6: Screen Enum and Module Declarations

**Files:**
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Add module declarations**

In `src/tui/mod.rs`, after `pub mod trends;` (line 7), add:

```rust
pub mod abbreviations;
pub mod quick_log;
```

- [ ] **Step 2: Add screen variants to Screen enum**

In `src/tui/mod.rs`, in the `Screen` enum (lines 92-100), add two new variants after `Goals`:

```rust
    QuickLog,
    Abbreviations,
```

- [ ] **Step 3: Create placeholder files**

Create `src/tui/quick_log.rs` and `src/tui/abbreviations.rs` with minimal compilable content:

`src/tui/abbreviations.rs`:
```rust
use crossterm::event::KeyEvent;
use ratatui::Frame;
use crate::db::Database;
use super::{Action, Screen};

pub struct AbbreviationsScreen;

impl AbbreviationsScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
```

`src/tui/quick_log.rs`:
```rust
use crossterm::event::KeyEvent;
use ratatui::Frame;
use crate::config::LlmConfig;
use crate::db::Database;
use super::{Action, Screen};

pub struct QuickLogScreen;

impl QuickLogScreen {
    pub fn new(_db: &Database, _config: &Option<LlmConfig>) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: KeyEvent, _db: &Database) -> Action {
        Action::None
    }

    pub fn check_background_result(&mut self) {}
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles (warnings about unused imports/fields are fine)

- [ ] **Step 5: Commit**

```bash
git add src/tui/mod.rs src/tui/quick_log.rs src/tui/abbreviations.rs
git commit -m "feat(quick-log): add QuickLog and Abbreviations screen stubs"
```

---

### Task 7: App.rs Integration (Event Loop + Screen Routing)

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add imports**

In `src/app.rs`, add to the imports section (lines 1-22):

After `use std::io::stdout;` (line 10), add:
```rust
use std::time::Duration;
```

In the `use crate::tui::` block (lines 14-22), add:
```rust
    abbreviations::AbbreviationsScreen,
    quick_log::QuickLogScreen,
```

After `use crate::db::Database;` (line 12), add:
```rust
use crate::config::Config;
```

- [ ] **Step 2: Load config and initialize screens in run_app**

In `run_app()`, after `let mut practices = PracticesScreen::new(db)?;` (line 46), add:

```rust
    let config = Config::load();
    let mut quick_log = QuickLogScreen::new(db, &config.llm)?;
    let mut abbreviations = AbbreviationsScreen::new(db)?;
```

- [ ] **Step 3: Add render match arms**

In the `terminal.draw()` closure (lines 63-70), add after the `Screen::Practices` arm:

```rust
                Screen::QuickLog => quick_log.render(frame),
                Screen::Abbreviations => abbreviations.render(frame),
```

- [ ] **Step 4: Change event loop from blocking to poll-based**

Replace line 73:
```rust
        if let Event::Key(key) = event::read()? {
```

With:
```rust
        if !event::poll(Duration::from_millis(100))? {
            if let Screen::QuickLog = current_screen {
                quick_log.check_background_result();
            }
            continue;
        }
        if let Event::Key(key) = event::read()? {
```

This means: poll for 100ms, if no event, check background result and loop. If there is an event, process it as before.

Also add after the key handling block (after the `match action` block, before the closing `}` of the `if let Event::Key` block), add:

```rust
            if let Screen::QuickLog = current_screen {
                quick_log.check_background_result();
            }
```

- [ ] **Step 5: Add handle_key match arms**

In the key handling section (lines 78-89), add after `Screen::Practices`:

```rust
                Screen::QuickLog => quick_log.handle_key(key, db),
                Screen::Abbreviations => abbreviations.handle_key(key, db),
```

- [ ] **Step 6: Add navigation routing**

In the `Action::Navigate(screen)` match arm (lines 93-116), add after the `Screen::Practices` arm:

```rust
                        Screen::QuickLog => {
                            quick_log = QuickLogScreen::new(db, &config.llm)?;
                        }
                        Screen::Abbreviations => {
                            abbreviations = AbbreviationsScreen::new(db)?;
                        }
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 8: Commit**

```bash
git add src/app.rs
git commit -m "feat(quick-log): integrate QuickLog and Abbreviations into app event loop"
```

---

### Task 8: Dashboard Keybinding

**Files:**
- Modify: `src/tui/dashboard.rs`

- [ ] **Step 1: Add [w] keybinding in handle_normal()**

In `src/tui/dashboard.rs`, in `handle_normal()` method, add after the `KeyCode::Char('g')` arm (line 751):

```rust
            KeyCode::Char('w') => Action::Navigate(Screen::QuickLog),
```

- [ ] **Step 2: Add [w] to footer hints**

Find the footer rendering section for `DashboardMode::Normal` (around line 230-260). Add a new span pair for the `[w]` key. The exact location depends on where the footer spans are built — add it near the other navigation keys:

```rust
                Span::styled("[w]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-quick-log")), Style::default().fg(Color::Gray)),
```

- [ ] **Step 3: Add i18n key**

In `locales/en.ftl`, in the keys section (near the existing `key-*` entries), add:

```
key-quick-log = Quick Log
```

In `locales/zh-CN.ftl`, add:

```
key-quick-log = 快速记录
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/tui/dashboard.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(quick-log): add [w] keybinding to dashboard for Quick Log"
```

---

### Task 9: Abbreviations Screen (Full Implementation)

**Files:**
- Modify: `src/tui/abbreviations.rs`

- [ ] **Step 1: Implement the full Abbreviations screen**

Replace `src/tui/abbreviations.rs` with the full implementation. This follows the `PracticesScreen` pattern at `src/tui/practices.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::tr;
use crate::model::Abbreviation;
use super::{centered_area, highlight_row, render_help_overlay, render_status_line, Action, Screen, StatusMessage, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    AddShort,
    AddFull,
    EditShort,
    EditFull,
    ConfirmDelete,
}

pub struct AbbreviationsScreen {
    abbreviations: Vec<Abbreviation>,
    selected: usize,
    mode: Mode,
    short_input: String,
    short_cursor: usize,
    full_input: String,
    full_cursor: usize,
    editing_id: Option<i64>,
    status_msg: StatusMessage,
    show_help: bool,
}

impl AbbreviationsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let abbreviations = db.list_abbreviations()?;
        Ok(Self {
            abbreviations,
            selected: 0,
            mode: Mode::Browse,
            short_input: String::new(),
            short_cursor: 0,
            full_input: String::new(),
            full_cursor: 0,
            editing_id: None,
            status_msg: None,
            show_help: false,
        })
    }

    fn refresh(&mut self, db: &Database) {
        if let Ok(abbrs) = db.list_abbreviations() {
            self.abbreviations = abbrs;
            if self.selected >= self.abbreviations.len() && !self.abbreviations.is_empty() {
                self.selected = self.abbreviations.len() - 1;
            }
            if self.abbreviations.is_empty() {
                self.selected = 0;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let list_height = (self.abbreviations.len() as u16).max(1);
        let action_height: u16 = match &self.mode {
            Mode::Browse => 0,
            _ => 4,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),             // title + header
                Constraint::Length(list_height),   // list
                Constraint::Length(action_height), // input area
                Constraint::Length(1),             // status
                Constraint::Length(1),             // shortcuts
                Constraint::Min(0),                // spacer
            ])
            .split(area);

        // ── Title + header ──
        let max_short = self.abbreviations.iter()
            .map(|a| a.short.width())
            .max()
            .unwrap_or(5)
            .max(10);
        let short_col = max_short + 4;

        let title_lines = vec![
            Line::from(Span::styled(
                tr("abbreviations-title"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(tr("abbreviations-col-short"), Style::default().fg(Color::DarkGray)),
                Span::raw(" ".repeat(short_col.saturating_sub(tr("abbreviations-col-short").width()))),
                Span::styled(tr("abbreviations-col-full"), Style::default().fg(Color::DarkGray)),
            ]),
        ];
        frame.render_widget(Paragraph::new(title_lines), chunks[0]);

        // ── List ──
        let list_lines: Vec<Line> = if self.abbreviations.is_empty() {
            vec![Line::from(Span::styled(
                tr("abbreviations-no-items"),
                Style::default().fg(Color::Gray),
            ))]
        } else {
            self.abbreviations.iter().enumerate().map(|(i, a)| {
                let marker = if i == self.selected { "> " } else { "  " };
                let style = if i == self.selected {
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let padding = short_col.saturating_sub(a.short.width());
                Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(&a.short, style),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(&a.full_name, Style::default().fg(Color::Gray)),
                ])
            }).collect()
        };
        frame.render_widget(Paragraph::new(list_lines), chunks[1]);

        if !self.abbreviations.is_empty() {
            highlight_row(frame, chunks[1], self.selected as u16);
        }

        // ── Input area ──
        let action_lines = match &self.mode {
            Mode::Browse => vec![],
            Mode::AddShort | Mode::EditShort => {
                let label = if self.mode == Mode::AddShort {
                    tr("abbreviations-enter-short")
                } else {
                    tr("abbreviations-edit-short")
                };
                vec![
                    Line::from(Span::styled(label, Style::default().fg(Color::White))),
                    Line::from(vec![
                        Span::styled(" > ", Style::default().fg(GREEN)),
                        Span::styled(&self.short_input[..self.short_cursor], Style::default().fg(GREEN)),
                        Span::styled("_", Style::default().fg(GREEN)),
                        Span::styled(&self.short_input[self.short_cursor..], Style::default().fg(GREEN)),
                    ]),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddFull | Mode::EditFull => {
                let label = if self.mode == Mode::AddFull {
                    tr("abbreviations-enter-full")
                } else {
                    tr("abbreviations-edit-full")
                };
                vec![
                    Line::from(Span::styled(
                        format!("{}: {}", tr("abbreviations-col-short"), self.short_input),
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(label, Style::default().fg(Color::White))),
                    Line::from(vec![
                        Span::styled(" > ", Style::default().fg(GREEN)),
                        Span::styled(&self.full_input[..self.full_cursor], Style::default().fg(GREEN)),
                        Span::styled("_", Style::default().fg(GREEN)),
                        Span::styled(&self.full_input[self.full_cursor..], Style::default().fg(GREEN)),
                    ]),
                    Line::from(""),
                ]
            }
            Mode::ConfirmDelete => {
                let name = self.abbreviations.get(self.selected)
                    .map(|a| a.short.as_str())
                    .unwrap_or("?");
                vec![
                    Line::from(Span::styled(
                        format!("Delete \"{}\"? (y/n)", name),
                        Style::default().fg(RED),
                    )),
                    Line::from(""),
                    Line::from(""),
                    Line::from(""),
                ]
            }
        };
        if !action_lines.is_empty() {
            frame.render_widget(Paragraph::new(action_lines), chunks[2]);
        }

        // ── Status ──
        render_status_line(frame, chunks[3], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = match &self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::Gray)),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[?]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::Gray)),
            ]),
            Mode::AddShort | Mode::AddFull | Mode::EditShort | Mode::EditFull => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ]),
            Mode::ConfirmDelete => Line::from(vec![
                Span::styled(" [y]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                Span::styled("[n]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-no")), Style::default().fg(Color::Gray)),
            ]),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[4]);

        // ── Help overlay ──
        if self.show_help {
            let bindings = &[
                ("j/k", "Navigate"),
                ("a", "Add"),
                ("e", "Edit"),
                ("d", "Delete"),
                ("?", "Help"),
                ("Esc", "Back"),
            ];
            render_help_overlay(frame, area, bindings);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::AddShort => self.handle_add_short(key),
            Mode::AddFull => self.handle_add_full(key, db),
            Mode::EditShort => self.handle_edit_short(key),
            Mode::EditFull => self.handle_edit_full(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_text_input(input: &mut String, cursor: &mut usize, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor > 0 {
                    *cursor = input[..*cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                true
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    *cursor = input[..*cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor < input.len() {
                    *cursor = input[*cursor..].char_indices().nth(1).map(|(i, _)| *cursor + i).unwrap_or(input.len());
                }
                true
            }
            KeyCode::Right => {
                if *cursor < input.len() {
                    *cursor = input[*cursor..].char_indices().nth(1).map(|(i, _)| *cursor + i).unwrap_or(input.len());
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => { *cursor = 0; true }
            KeyCode::Home => { *cursor = 0; true }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => { *cursor = input.len(); true }
            KeyCode::End => { *cursor = input.len(); true }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let prev = input[..*cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    input.remove(prev);
                    *cursor = prev;
                }
                true
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.truncate(*cursor);
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, _db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.abbreviations.is_empty() {
                    self.selected = (self.selected + 1) % self.abbreviations.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.abbreviations.is_empty() {
                    self.selected = self.selected.checked_sub(1).unwrap_or(self.abbreviations.len() - 1);
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.short_input.clear();
                self.short_cursor = 0;
                self.full_input.clear();
                self.full_cursor = 0;
                self.mode = Mode::AddShort;
                Action::None
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(a) = self.abbreviations.get(self.selected) {
                    self.short_input = a.short.clone();
                    self.short_cursor = self.short_input.len();
                    self.full_input = a.full_name.clone();
                    self.full_cursor = self.full_input.len();
                    self.editing_id = Some(a.id);
                    self.mode = Mode::EditShort;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.abbreviations.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::QuickLog),
            _ => Action::None,
        }
    }

    fn handle_add_short(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.short_input.trim().is_empty() {
                    self.mode = Mode::AddFull;
                }
                Action::None
            }
            KeyCode::Esc => { self.mode = Mode::Browse; Action::None }
            _ => { Self::handle_text_input(&mut self.short_input, &mut self.short_cursor, key); Action::None }
        }
    }

    fn handle_add_full(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.full_input.trim().is_empty() {
                    match db.create_abbreviation(self.short_input.trim(), self.full_input.trim()) {
                        Ok(_) => self.refresh(db),
                        Err(e) => { self.status_msg = Some((format!("Error: {}", e), true)); }
                    }
                    self.mode = Mode::Browse;
                }
                Action::None
            }
            KeyCode::Esc => { self.mode = Mode::Browse; Action::None }
            _ => { Self::handle_text_input(&mut self.full_input, &mut self.full_cursor, key); Action::None }
        }
    }

    fn handle_edit_short(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.short_input.trim().is_empty() {
                    self.mode = Mode::EditFull;
                }
                Action::None
            }
            KeyCode::Esc => { self.mode = Mode::Browse; Action::None }
            _ => { Self::handle_text_input(&mut self.short_input, &mut self.short_cursor, key); Action::None }
        }
    }

    fn handle_edit_full(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(id) = self.editing_id {
                    if !self.full_input.trim().is_empty() {
                        match db.update_abbreviation(id, self.short_input.trim(), self.full_input.trim()) {
                            Ok(()) => self.refresh(db),
                            Err(e) => { self.status_msg = Some((format!("Error: {}", e), true)); }
                        }
                    }
                }
                self.editing_id = None;
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => { self.editing_id = None; self.mode = Mode::Browse; Action::None }
            _ => { Self::handle_text_input(&mut self.full_input, &mut self.full_cursor, key); Action::None }
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(a) = self.abbreviations.get(self.selected) {
                    match db.delete_abbreviation(a.id) {
                        Ok(()) => self.refresh(db),
                        Err(e) => { self.status_msg = Some((format!("Delete failed: {}", e), true)); }
                    }
                }
                self.mode = Mode::Browse;
                Action::None
            }
            _ => { self.mode = Mode::Browse; Action::None }
        }
    }
}
```

- [ ] **Step 2: Add i18n keys**

In `locales/en.ftl`, add:

```
# ── Abbreviations ──
abbreviations-title = Abbreviations
abbreviations-col-short = Short
abbreviations-col-full = Full Name
abbreviations-no-items = No abbreviations — press [a] to add one
abbreviations-enter-short = Enter abbreviation (e.g., DL):
abbreviations-enter-full = Enter full name (e.g., Deadlift):
abbreviations-edit-short = Edit abbreviation:
abbreviations-edit-full = Edit full name:
```

In `locales/zh-CN.ftl`, add the corresponding Chinese translations.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add src/tui/abbreviations.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(quick-log): implement Abbreviations CRUD screen"
```

---

### Task 10: QuickLog Screen (Full Implementation)

**Files:**
- Modify: `src/tui/quick_log.rs`

This is the largest task. The QuickLog screen has three phases: Input (multi-line text editing), Parsing (spinner + background thread), and Preview (structured result display + confirm/save).

- [ ] **Step 1: Implement the full QuickLog screen**

Replace `src/tui/quick_log.rs` with the full implementation:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::sync::mpsc::{self, Receiver};

use crate::config::LlmConfig;
use crate::db::Database;
use crate::i18n::tr;
use crate::llm::{self, LlmError};
use crate::model::{Abbreviation, ParsedLog, Practice, SetData};
use super::{centered_area, render_help_overlay, render_status_line, Action, Screen, StatusMessage, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;
const YELLOW: Color = Color::Yellow;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Input,
    Parsing,
    Preview,
}

pub struct QuickLogScreen {
    // Text input
    input_lines: Vec<String>,
    current_line: usize,
    cursor_pos: usize,

    // Config
    llm_config: Option<LlmConfig>,

    // Cached data for LLM context
    practices: Vec<Practice>,
    abbreviations: Vec<Abbreviation>,

    // LLM results
    parsed_results: Vec<ParsedLog>,
    result_receiver: Option<Receiver<Result<Vec<ParsedLog>, LlmError>>>,

    // Preview state
    selected_result: usize,
    scroll_offset: usize,

    // Screen state
    phase: Phase,
    status_msg: StatusMessage,
    show_help: bool,
    log_date: String,
    spinner_frame: usize,
}

impl QuickLogScreen {
    pub fn new(db: &Database, config: &Option<LlmConfig>) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let abbreviations = db.list_abbreviations()?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(Self {
            input_lines: vec![String::new()],
            current_line: 0,
            cursor_pos: 0,
            llm_config: config.clone(),
            practices,
            abbreviations,
            parsed_results: Vec::new(),
            result_receiver: None,
            selected_result: 0,
            scroll_offset: 0,
            phase: Phase::Input,
            status_msg: None,
            show_help: false,
            log_date: today,
            spinner_frame: 0,
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // title
                Constraint::Min(6),    // main content (side-by-side)
                Constraint::Length(1),  // date line
                Constraint::Length(1),  // status
                Constraint::Length(1),  // shortcuts
            ])
            .split(area);

        // ── Title ──
        let title = Line::from(Span::styled(
            tr("quicklog-title"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // ── Side-by-side content ──
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[1]);

        self.render_input_pane(frame, content_chunks[0]);
        self.render_preview_pane(frame, content_chunks[1]);

        // ── Date line ──
        let date_line = Line::from(vec![
            Span::styled(format!("  {}: ", tr("quicklog-date")), Style::default().fg(Color::Gray)),
            Span::styled(&self.log_date, Style::default().fg(ACCENT)),
        ]);
        frame.render_widget(Paragraph::new(date_line), chunks[2]);

        // ── Status ──
        render_status_line(frame, chunks[3], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = match &self.phase {
            Phase::Input => Line::from(vec![
                Span::styled(" [Ctrl+S]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("quicklog-key-parse")), Style::default().fg(Color::Gray)),
                Span::styled("[?]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::Gray)),
            ]),
            Phase::Parsing => Line::from(vec![
                Span::styled(" [Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ]),
            Phase::Preview => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("quicklog-key-save")), Style::default().fg(Color::Gray)),
                Span::styled("[j/k]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("quicklog-key-remove")), Style::default().fg(Color::Gray)),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("quicklog-key-abbr")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("quicklog-key-edit")), Style::default().fg(Color::Gray)),
            ]),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[4]);

        // ── Help overlay ──
        if self.show_help {
            let bindings = match &self.phase {
                Phase::Input => vec![
                    ("Ctrl+S", "Parse with LLM"),
                    ("Enter", "New line"),
                    ("↑/↓", "Move between lines"),
                    ("Esc", "Back to Dashboard"),
                    ("?", "Help"),
                ],
                Phase::Parsing => vec![
                    ("Esc", "Cancel"),
                ],
                Phase::Preview => vec![
                    ("Enter", "Save all"),
                    ("j/k", "Navigate entries"),
                    ("d", "Remove entry"),
                    ("a", "Abbreviations"),
                    ("Esc", "Back to edit"),
                ],
            };
            let binding_refs: Vec<(&str, &str)> = bindings.iter().map(|(a, b)| (*a, *b)).collect();
            render_help_overlay(frame, area, &binding_refs);
        }
    }

    fn render_input_pane(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-input-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = self.input_lines.iter().enumerate().map(|(i, line)| {
            if self.phase == Phase::Input && i == self.current_line {
                let before = &line[..self.cursor_pos];
                let after = &line[self.cursor_pos..];
                Line::from(vec![
                    Span::styled(before, Style::default().fg(Color::White)),
                    Span::styled("▏", Style::default().fg(GREEN)),
                    Span::styled(after, Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Gray)))
            }
        }).collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-preview-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match &self.phase {
            Phase::Input => {
                if self.llm_config.is_none() {
                    let msg = Line::from(Span::styled(
                        tr("quicklog-no-config"),
                        Style::default().fg(YELLOW),
                    ));
                    frame.render_widget(Paragraph::new(msg).wrap(Wrap { trim: false }), inner);
                }
            }
            Phase::Parsing => {
                let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner = spinners[self.spinner_frame % spinners.len()];
                let msg = Line::from(vec![
                    Span::styled(spinner, Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("quicklog-parsing")), Style::default().fg(Color::Gray)),
                ]);
                frame.render_widget(Paragraph::new(msg), inner);
            }
            Phase::Preview => {
                let mut lines: Vec<Line> = Vec::new();
                for (i, entry) in self.parsed_results.iter().enumerate() {
                    let is_selected = i == self.selected_result;
                    let marker = if entry.matched { "✓" } else { "✗" };
                    let marker_color = if entry.matched { GREEN } else { RED };
                    let name_style = if !entry.matched {
                        Style::default().fg(YELLOW)
                    } else if is_selected {
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(format!(" {} ", marker), Style::default().fg(marker_color)),
                        Span::styled(&entry.practice_name, name_style),
                    ]));

                    for (j, set) in entry.sets.iter().enumerate() {
                        let set_text = match set {
                            SetData::Weighted { weight, reps } => format!("   Set {}: {}kg × {}", j + 1, weight, reps),
                            SetData::Bodyweight { reps } => format!("   Set {}: {} reps", j + 1, reps),
                            SetData::Distance { distance } => format!("   Set {}: {} km", j + 1, distance),
                            SetData::Endurance { duration } => format!("   Set {}: {} min", j + 1, duration),
                        };
                        lines.push(Line::from(Span::styled(
                            set_text,
                            Style::default().fg(Color::Gray),
                        )));
                    }

                    if i < self.parsed_results.len() - 1 {
                        lines.push(Line::from(""));
                    }
                }

                if lines.is_empty() {
                    lines.push(Line::from(Span::styled(
                        tr("quicklog-no-results"),
                        Style::default().fg(Color::Gray),
                    )));
                }

                let has_unmatched = self.parsed_results.iter().any(|p| !p.matched);
                if has_unmatched {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        tr("quicklog-unmatched-warning"),
                        Style::default().fg(YELLOW),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), inner);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.phase {
            Phase::Input => self.handle_input(key, db),
            Phase::Parsing => self.handle_parsing(key),
            Phase::Preview => self.handle_preview(key, db),
        }
    }

    fn handle_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.start_parsing(db);
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Enter => {
                let rest = self.input_lines[self.current_line][self.cursor_pos..].to_string();
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                self.current_line += 1;
                self.input_lines.insert(self.current_line, rest);
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Up => {
                if self.current_line > 0 {
                    self.current_line -= 1;
                    self.cursor_pos = self.cursor_pos.min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Down => {
                if self.current_line < self.input_lines.len() - 1 {
                    self.current_line += 1;
                    self.cursor_pos = self.cursor_pos.min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let line = &mut self.input_lines[self.current_line];
                    let prev = line[..self.cursor_pos]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    line.remove(prev);
                    self.cursor_pos = prev;
                } else if self.current_line > 0 {
                    let removed = self.input_lines.remove(self.current_line);
                    self.current_line -= 1;
                    self.cursor_pos = self.input_lines[self.current_line].len();
                    self.input_lines[self.current_line].push_str(&removed);
                }
                Action::None
            }
            KeyCode::Left | KeyCode::Char('b') if key.code == KeyCode::Left || key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.cursor_pos > 0 {
                    let line = &self.input_lines[self.current_line];
                    self.cursor_pos = line[..self.cursor_pos]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                Action::None
            }
            KeyCode::Right | KeyCode::Char('f') if key.code == KeyCode::Right || key.modifiers.contains(KeyModifiers::CONTROL) => {
                let line = &self.input_lines[self.current_line];
                if self.cursor_pos < line.len() {
                    self.cursor_pos = line[self.cursor_pos..]
                        .char_indices().nth(1).map(|(i, _)| self.cursor_pos + i).unwrap_or(line.len());
                }
                Action::None
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::End => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                Action::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_parsing(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            self.result_receiver = None;
            self.phase = Phase::Input;
        }
        Action::None
    }

    fn handle_preview(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = (self.selected_result + 1) % self.parsed_results.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = self.selected_result
                        .checked_sub(1).unwrap_or(self.parsed_results.len() - 1);
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.parsed_results.is_empty() {
                    self.parsed_results.remove(self.selected_result);
                    if self.selected_result >= self.parsed_results.len() && !self.parsed_results.is_empty() {
                        self.selected_result = self.parsed_results.len() - 1;
                    }
                }
                Action::None
            }
            KeyCode::Char('a') => {
                Action::Navigate(Screen::Abbreviations)
            }
            KeyCode::Enter | KeyCode::Char('s') if key.code == KeyCode::Enter || key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_logs(db)
            }
            KeyCode::Esc => {
                self.phase = Phase::Input;
                self.parsed_results.clear();
                self.selected_result = 0;
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn start_parsing(&mut self, db: &Database) {
        let config = match &self.llm_config {
            Some(c) => c.clone(),
            None => {
                self.status_msg = Some((tr("quicklog-no-config"), true));
                return;
            }
        };

        let raw_text: String = self.input_lines.join("\n");
        if raw_text.trim().is_empty() {
            return;
        }

        // Refresh practices and abbreviations before sending
        if let Ok(p) = db.list_active_practices() { self.practices = p; }
        if let Ok(a) = db.list_abbreviations() { self.abbreviations = a; }

        let practices = self.practices.clone();
        let abbreviations = self.abbreviations.clone();

        let (tx, rx) = mpsc::channel();
        self.result_receiver = Some(rx);
        self.phase = Phase::Parsing;
        self.spinner_frame = 0;

        std::thread::spawn(move || {
            let system_prompt = llm::build_system_prompt(&practices, &abbreviations);
            let result = llm::call_llm(&config, &system_prompt, &raw_text)
                .and_then(|response| llm::parse_llm_response(&response, &practices));
            let _ = tx.send(result);
        });
    }

    pub fn check_background_result(&mut self) {
        if self.phase == Phase::Parsing {
            self.spinner_frame += 1;
        }

        let receiver = match &self.result_receiver {
            Some(r) => r,
            None => return,
        };

        match receiver.try_recv() {
            Ok(Ok(results)) => {
                self.parsed_results = results;
                self.selected_result = 0;
                self.phase = Phase::Preview;
                self.result_receiver = None;
            }
            Ok(Err(e)) => {
                self.status_msg = Some((e.to_string(), true));
                self.phase = Phase::Input;
                self.result_receiver = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status_msg = Some(("LLM thread crashed".to_string(), true));
                self.phase = Phase::Input;
                self.result_receiver = None;
            }
        }
    }

    fn save_logs(&mut self, db: &Database) -> Action {
        let has_unmatched = self.parsed_results.iter().any(|p| !p.matched);
        if has_unmatched {
            self.status_msg = Some((tr("quicklog-unmatched-error"), true));
            return Action::None;
        }

        if self.parsed_results.is_empty() {
            return Action::None;
        }

        let raw_text = self.input_lines.join("\n");
        let date = match chrono::NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d") {
            Ok(d) => d.and_hms_opt(12, 0, 0).unwrap(),
            Err(_) => {
                self.status_msg = Some(("Invalid date".to_string(), true));
                return Action::None;
            }
        };

        for entry in &self.parsed_results {
            let practice = match self.practices.iter().find(|p| p.name.eq_ignore_ascii_case(&entry.practice_name)) {
                Some(p) => p,
                None => continue,
            };
            let note = Some(raw_text.as_str());
            if let Err(e) = db.create_log_at(practice.id, &date, &entry.sets, note, None, None) {
                self.status_msg = Some((format!("Save error: {}", e), true));
                return Action::None;
            }
        }

        Action::Navigate(Screen::Dashboard)
    }
}
```

- [ ] **Step 2: Add i18n keys**

In `locales/en.ftl`, add:

```
# ── Quick Log ──
quicklog-title = Quick Log
quicklog-input-title = Input
quicklog-preview-title = Preview
quicklog-date = Date
quicklog-no-config = Configure LLM in ~/.iron/config.toml
quicklog-parsing = Parsing with LLM...
quicklog-no-results = No results
quicklog-unmatched-warning = ✗ marks unknown practices — create them first or fix the text
quicklog-unmatched-error = Cannot save: some practice names don't match
quicklog-key-parse = Parse
quicklog-key-save = Save all
quicklog-key-remove = Remove
quicklog-key-abbr = Abbreviations
quicklog-key-edit = Back to edit
```

In `locales/zh-CN.ftl`, add the corresponding Chinese translations.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: no errors (fix any warnings)

- [ ] **Step 6: Commit**

```bash
git add src/tui/quick_log.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(quick-log): implement QuickLog screen with LLM parsing and preview"
```

---

### Task 11: Manual Testing and Polish

**Files:** None new — this is verification and bugfixing.

- [ ] **Step 1: Create a test config file**

Create `~/.iron/config.toml` with your LLM endpoint:

```toml
[llm]
endpoint = "http://localhost:11434/v1"
model = "llama3"
```

Or for OpenAI:
```toml
[llm]
endpoint = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o-mini"
```

- [ ] **Step 2: Test the full flow**

Run: `cargo run`

1. Press `[w]` from Dashboard → verify QuickLog screen renders
2. Type multi-line training notes (e.g., "DL 60kg 5/5/5")
3. Press `Ctrl+S` → verify spinner appears, then preview shows parsed result
4. Press `Enter` to save → verify logs appear in History
5. Check that the note field contains the full raw text

- [ ] **Step 3: Test error cases**

1. Remove `config.toml` → press `[w]`, verify "Configure LLM" message
2. Set bad endpoint → press `Ctrl+S`, verify error message
3. Type an unknown practice → verify yellow highlight, save blocked
4. Press `Esc` during parsing → verify returns to input
5. Press `[a]` in preview → verify Abbreviations screen opens
6. Add an abbreviation → press `Esc` → verify returns to QuickLog

- [ ] **Step 4: Fix any issues found**

Fix bugs discovered during manual testing.

- [ ] **Step 5: Final verification**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings

- [ ] **Step 6: Commit any fixes**

```bash
git add -A
git commit -m "fix(quick-log): polish and bugfixes from manual testing"
```

---

### Task 12: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add Quick Log documentation**

Add a section to `README.md` documenting:
- The Quick Log screen and how to access it (`[w]` from Dashboard)
- How to configure `~/.iron/config.toml` for LLM
- The shorthand syntax examples
- The abbreviation dictionary feature
- The side-by-side preview and save flow

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add Quick Log feature documentation to README"
```
