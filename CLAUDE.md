# IronCLI

A terminal UI application for tracking training records, built in Rust.

## Quick Reference

```bash
cargo run              # Launch the TUI (opens ~/.ironcli/iron.db)
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

Three tables in `~/.ironcli/iron.db`:

- `practices` (id, name UNIQUE, practice_type TEXT, created_at)
- `logs` (id, practice_id FK, logged_at TEXT, note TEXT nullable)
- `sets` (id, log_id FK, set_number, weight?, reps?, distance?, duration?)

Nullable fields on `sets` — only the relevant ones are filled based on practice type.

## Navigation

Vim-style throughout: `j/k` up/down, `h/l` left/right, `/` filter, `Esc` back, `Enter` confirm, `Ctrl+S` save log, `D` edit date, `d` delete, `q` quit.

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
cargo test                                          # all 23 tests
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

## Spec & Plan

- Design spec: `docs/superpowers/specs/2026-04-16-ironcli-design.md`
- Implementation plan: `docs/superpowers/plans/2026-04-16-ironcli-implementation.md`
