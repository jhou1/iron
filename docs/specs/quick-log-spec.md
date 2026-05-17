# Quick Log Spec

Natural language training entry powered by an LLM. Write training notes in shorthand, parse them into structured logs, review, and save.

## Phases

```
Input -> Parsing (async) -> Preview -> [save]
```

## Layout

Two-pane horizontal split:
- **Left pane:** Multi-line text input
- **Right pane:** Parse preview / results

## Phase: Input

**Left pane:**
- Multi-line text input area
- Cursor visible, line-by-line editing
- Example input:
  ```
  DL 60kg 5/5/5
  Pull-ups 10/8/6
  Run 5km
  ```

**Right pane:**
- Placeholder: "Type notes, then press Ctrl+S to parse"

## Phase: Parsing

- Triggered by `Ctrl+S`
- Shows spinner animation + "Parsing with LLM..."
- Runs asynchronously (non-blocking UI)
- Abbreviations expanded before sending to LLM

## Phase: Preview

**Right pane shows parsed results:**
- Each parsed log as a row
- Practice name colored: green if matched to existing practice, red if unmatched
- Sets displayed inline
- Selected row highlighted

**Actions in preview:**
- `j/k` — navigate results
- `a` — add abbreviation for unmatched practice name
- `Enter` — save all matched logs to database
- `Esc` — discard and return to Dashboard

## LLM Integration

### Configuration

File: `~/.iron/config.toml`

```toml
[llm]
endpoint = "http://localhost:11434/v1"  # Ollama or OpenAI-compatible
api_key = "sk-..."                       # optional for local models
model = "llama3.2:3b"
```

If no config exists, Quick Log shows an error on launch.

### Prompt

Sends to LLM:
- Available practices (name + type)
- Abbreviation dictionary
- User's raw input text

Expected response: JSON array of parsed logs with practice name and sets.

### Error Handling

- LLM failure: status message with error
- Invalid JSON response: status message
- User can retry with `Ctrl+S`

## Abbreviation Expansion

Before sending to LLM, all known abbreviations are substituted in the input text. User can add new abbreviations on-the-fly from the preview phase when a practice name is unmatched.

## Date

- Default: today
- `D` opens date input overlay (YYYY-MM-DD)
- Applies to all parsed logs

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+S` | Parse input with LLM |
| `Enter` | New line (input) / Save all logs (preview) |
| `Up/Down` | Navigate lines (input) / Navigate results (preview) |
| `D` | Change log date |
| `a` | Add abbreviation (preview, unmatched practice) |
| `?` | Help overlay |
| `Esc` | Back to Dashboard |
