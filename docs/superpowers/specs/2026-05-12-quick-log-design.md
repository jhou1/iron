# Quick Log: LLM-Powered Free-Text Training Input — Design Spec

## Context

Currently, logging a training session in iron requires navigating through a structured multi-phase flow: select practice, enter sets one by one, add warm-up/cool-down, add note. This is thorough but slow when the user already has shorthand notes (e.g., from a notebook or mental model like "DL 60kg 5/5/5"). This feature adds a "Quick Log" screen where the user types free-form shorthand text, sends it to an LLM via OpenAI-compatible API, reviews the parsed result, and saves — creating multiple log entries from a single text input.

## Decisions

| Decision | Choice |
|----------|--------|
| LLM API protocol | OpenAI-compatible chat completions (works with Claude proxy, GPT, Ollama, LM Studio, llama.cpp) |
| HTTP client | `ureq` (sync, minimal deps) + `std::thread::spawn` background thread |
| Config storage | `~/.iron/config.toml` via `toml` crate |
| Abbreviation storage | SQLite table, TUI-managed CRUD screen |
| Entry point | New dedicated screen ("Quick Log"), keybinding `[w]` from Dashboard |
| Multi-log support | Yes — multi-line input, each line = separate log entry |
| Preview UX | Side-by-side: left = raw text, right = structured preview |
| Raw text preservation | All created logs get the full raw text as their `note` field |
| Unknown practice handling | Block save until all practice names resolve to existing practices |
| Abbreviation → LLM integration | Send dictionary + practice list as prompt context; LLM resolves |

## Architecture

### New Files

- **`src/config.rs`** — Config loading from `~/.iron/config.toml`
- **`src/llm.rs`** — OpenAI-compatible LLM client, prompt construction, response parsing
- **`src/tui/quick_log.rs`** — QuickLog screen (multi-line input, background LLM call, preview, confirm)
- **`src/tui/abbreviations.rs`** — Abbreviation dictionary CRUD screen

### Modified Files

- **`Cargo.toml`** — add `ureq = "3"`, `toml = "0.8"`
- **`src/model.rs`** — add `Abbreviation`, `ParsedLog` structs
- **`src/db.rs`** — add `abbreviations` table schema + CRUD methods
- **`src/tui/mod.rs`** — add `QuickLog`, `Abbreviations` to `Screen` enum; add `pub mod quick_log; pub mod abbreviations;`
- **`src/app.rs`** — add QuickLog/Abbreviations screen init + routing; change `event::read()` to `event::poll(100ms)` + `event::read()`
- **`src/tui/dashboard.rs`** — add `[w]` keybinding in `handle_normal()`, update footer hints
- **`src/main.rs`** — add `mod config; mod llm;`
- **`src/lib.rs`** — add `pub mod config; pub mod llm;`
- **Locale files** — new i18n keys for QuickLog and Abbreviations screens

### New Dependencies

| Crate | Version | Purpose | Transitive deps |
|-------|---------|---------|-----------------|
| `ureq` | 3 | Sync HTTP client for OpenAI API | ~15 |
| `toml` | 0.8 | Config file parsing | ~5 |

## Config System

```toml
# ~/.iron/config.toml
[llm]
endpoint = "http://localhost:11434/v1"
api_key = ""
model = "llama3"
```

- `Config::load()` reads `~/.iron/config.toml`, returns default `Config` if file missing
- Loaded once in `app::run()`, passed to QuickLog screen

## Abbreviation Dictionary

### Schema

```sql
CREATE TABLE IF NOT EXISTS abbreviations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    short TEXT NOT NULL UNIQUE COLLATE NOCASE,
    full_name TEXT NOT NULL
);
```

### DB Methods

- `create_abbreviation(&self, short: &str, full_name: &str) -> Result<Abbreviation>`
- `list_abbreviations(&self) -> Result<Vec<Abbreviation>>`
- `update_abbreviation(&self, id: i64, short: &str, full_name: &str) -> Result<()>`
- `delete_abbreviation(&self, id: i64) -> Result<()>`

### Abbreviations Screen

Follows the Practices screen pattern: list + add/edit/delete modes.

## LLM Client

System prompt includes: active practice list (name + type), abbreviation dictionary, JSON schema for response, parsing rules. User message is the raw multi-line text. Response is validated against practice list.

## QuickLog Screen

### Phases

```
Input  →  Parsing  →  Preview  →  (Save)
                         ↓
                      Input (on Esc, edit text and retry)
```

### Layout

```
┌─ Quick Log ─────────────────────────────────────────────────────┐
│  ┌─ Input ──────────────────┐  ┌─ Preview ─────────────────┐   │
│  │ DL 60kg 5/5/5            │  │ ✓ Deadlift (weighted)     │   │
│  │ KB SW 16kg 10/10/10      │  │   Set 1: 60kg × 5         │   │
│  │ Pull-ups 10/8/6          │  │   ...                     │   │
│  └──────────────────────────┘  └────────────────────────────┘   │
│  Date: 2026-05-12  │  Ctrl+S: parse  │  Enter: save  │  ?: help│
└─────────────────────────────────────────────────────────────────┘
```

### Background Thread

`std::thread::spawn` + `mpsc::channel`. Event loop uses `event::poll(100ms)` to check for results without blocking.

### Save Flow

All created logs get the full raw text as note. Block save if any practice name is unresolved.
