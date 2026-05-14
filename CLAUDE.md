# iron

A terminal UI application for tracking training records, built in Rust.

## Quick Reference

```bash
cargo run              # Launch the TUI (opens ~/.iron/iron.db)
cargo test             # Run all 23 tests
cargo clippy           # Lint check
cargo build --release  # Build release binary at target/release/iron
```

The binary is named `iron`. Subcommands: `iron export [path]`, `iron import <path>`.

## Architecture

Single Rust binary. Three layers: **model** (domain types) → **db** (SQLite via rusqlite) → **tui** (ratatui screens) + **export** (JSON serialization).

```
src/
  main.rs              — clap CLI entry point, dispatches to TUI or export/import
  app.rs               — event loop, screen routing, terminal setup/teardown
  model.rs             — PracticeType, Practice, Log, Set, SetData, LogEntry
  db.rs                — Database struct wrapping rusqlite, all CRUD + queries
  export.rs            — JSON export/import with duplicate detection
  lib.rs               — re-exports model, db, export for integration tests
  tui/
    mod.rs             — Screen enum, Action enum
    dashboard.rs       — heatmap + today's log + 14-day stats
    log_entry.rs       — practice picker + set-by-set form + date + note
    history.rs         — 14-day scrollable log, edit/delete
    trends.rs          — sparkline chart per practice with time window
    practices.rs       — practice inventory CRUD
    widgets/
      heatmap.rs       — GitHub-style contribution heatmap (Widget impl)
      sparkline.rs     — vertical bar chart (Widget impl)
```

## Key Concepts

- **Practice** (not "exercise"): a named training activity with a type. The user is _practicing_, not exercising.
- **Practice types**: Weighted (weightxreps), Bodyweight (reps), Distance (distance), Endurance (duration). Shown as UI labels, stored as lowercase enum strings in SQLite.
- **Log**: a single session of a practice, with a date, optional note, and 1+ sets.
- **Set**: one rep/effort within a log. Each set can have different weight/reps (no flat sets×reps model).
- **Derived metrics**: computed at query time, never stored. Volume = sum(weight×reps), etc.

## Data Model (SQLite)

Three tables in `~/.iron/iron.db`:

- `practices` (id, name UNIQUE, practice_type TEXT, created_at)
- `logs` (id, practice_id FK, logged_at TEXT, note TEXT nullable)
- `sets` (id, log_id FK, set_number, weight?, reps?, distance?, duration?)

Nullable fields on `sets` — only the relevant ones are filled based on practice type.

## Navigation

Vim-style throughout: `j/k` up/down, `h/l` left/right, `/` filter, `Esc` back, `Enter` confirm, `Ctrl+S` save log, `D` edit date, `d` delete, `q` quit.

All text input fields use emacs-style cursor: `Ctrl+B`/Left back, `Ctrl+F`/Right forward, `Ctrl+A`/Home start, `Ctrl+E`/End end, `Ctrl+K` kill to end of line. Characters insert at cursor, not append.

**Every text input field must support the full emacs keybinding set:** `Ctrl+B`, `Ctrl+F`, `Ctrl+A`, `Ctrl+E`, `Ctrl+K`, `Left`, `Right`, `Home`, `End`, `Backspace`. This is a hard constraint — no text input may be added without these bindings. Use the `handle_text_input()` pattern from `tui/practices.rs` or `tui/abbreviations.rs` for single-line fields.

**Text input must never overflow its container.** All text input rendering must use `visible_input_spans()` from `tui/mod.rs` to horizontally scroll long text within the available width. Never render raw `text[..cursor]` + cursor + `text[cursor..]` spans without width clipping.

**All text content must wrap within its container.** Every `Paragraph` that renders user-entered or variable-length text (notes, names, descriptions, quotes, warm-up/cool-down) must use `.wrap(Wrap { trim: false })`. Text must never overflow the right border of its container. Only fixed-width UI elements (footers, shortcuts bars, single-line labels) may omit `.wrap()`.

## Color Scheme

Uses ANSI terminal colors (not hardcoded RGB) for theme compatibility:

| Role | Color |
|---|---|
| Accent (titles, shortcuts) | Cyan |
| Active/selected items | Green |
| Bright highlight | LightGreen |
| Labels, borders | DarkGray |
| Error, delete | Red |
| Notes | Yellow |

## Testing

```bash
cargo test                                          # all tests
cargo test --test model_test                        # 6 model unit tests
cargo test --test db_test                           # 10 database CRUD tests
cargo test --test export_test                       # 2 basic export/import tests
cargo test --test export_import_integration_test    # 5 integration tests
```

Integration tests use file-based SQLite databases via `TestDb` helper (creates temp dir, auto-cleans on drop). All tests use `create_log_at` with explicit timestamps for deterministic assertions.

## Conventions

- Screen pattern: each screen is a struct with `new(db)`, `render(frame)`, `handle_key(key, ...)` returning `Action` (None/Navigate/Quit).
- Color constants defined per-screen as `const ACCENT`, `const GREEN`, etc. using ANSI colors.
- Database errors in TUI handlers are currently swallowed with `let _ =` (known limitation).
- `PracticeType::label()` returns the UI-facing name (weightxreps/reps/distance/duration). `Display` impl returns the storage name (weighted/bodyweight/distance/endurance).
- Everytime a feature is added or modified, update the README.md to provide proper user instruction.
- When a screen's rendering is updated (progress bars, goal display, etc.), update the dashboard if it renders the same data to keep styles consistent.
- Everytime fixing a bug, increment the patch version by 1

## Spec & Plan

- Design spec: `docs/superpowers/specs/2026-04-16-ironcli-design.md`
- Implementation plan: `docs/superpowers/plans/2026-04-16-ironcli-implementation.md`

## Renaming

The project was renamed from `ironcli` to `iron` in May 2026. The data directory moved from `~/.ironcli/` to `~/.iron/`. A one-time migration copies the database automatically — no manual action needed.

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request. There should be no compiling error and warnings.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

