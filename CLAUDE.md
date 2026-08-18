# iron

A terminal UI application for tracking training records, built in Rust.

## Quick Reference

```bash
cargo run              # Launch the TUI (opens ~/.iron/iron.db)
cargo test             # Run all tests
cargo clippy           # Lint check
cargo build --release  # Build release binary at target/release/iron
```

The binary is named `iron`. Subcommands: `iron export [path]`, `iron import <path>`.

## Architecture

Single Rust binary. Three layers: **model** (domain types) → **db** (SQLite via rusqlite) → **tui** (ratatui screens) + **export** (JSON serialization).

```
src/
  main.rs              — clap CLI entry point, dispatches to TUI or export/import
  app.rs               — AppShell route/action ownership plus event loop/setup/teardown
  app/
    router.rs           — atomic prepare/install/route transitions
    screens.rs          — ScreenStore concrete screen ownership and lifecycle preparation
  model.rs             — PracticeType, Practice, Log, Set, SetData, LogEntry
  db.rs                — Database struct wrapping rusqlite, all CRUD + queries
  export.rs            — JSON export/import with duplicate detection
  lib.rs               — re-exports model, db, export for integration tests
  tui/
    mod.rs             — shared TUI exports, layout helpers, and modules
    navigation.rs      — typed screen/module IDs, navigation intents, and actions
    lifecycle.rs       — static re-entry and navigation-status contracts
    framework/         — shared viewport, input, presentation, and test primitives
      screen_frame.rs  — centered body with reserved status and shortcut rows
      list_panel.rs    — bordered scrollable list presentation
      action_panel.rs  — bounded contextual action presentation
    dashboard.rs       — heatmap + summary + recent logs + goals + quote + HRV
    log_entry.rs       — practice picker + set-by-set form + date + RPE + notes
    history.rs         — 14-day scrollable log, edit/delete
    trends.rs          — sparkline chart per practice with time window
    practices.rs       — practice inventory CRUD
    goals.rs           — goals and milestone management
    quotes_screen.rs   — quote inventory CRUD
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

Core tables in `~/.iron/iron.db` include:

- `practices` (id, name UNIQUE, type, active, created_at)
- `training_sessions` (id, practice_id FK, date, warm_up?, cool_down?, rpe?, note?)
- `training_sets` (id, training_session_id FK, set_number, weight?, reps?, distance?, duration?)
- `goals` and `milestones`
- `quotes`
- `daily_metrics` (including HRV)

Nullable fields on `training_sets` — only the relevant ones are filled based on practice type.

Quick Log has been permanently removed with no replacement. Its LLM-only configuration/runtime, abbreviation models and CRUD API, and direct `ureq`/`toml` dependencies must not be reintroduced. `Database::init_schema` intentionally runs `DROP TABLE IF EXISTS abbreviations`; opening an upgraded database permanently deletes abbreviation rows while preserving unrelated application data.

## Navigation

Vim-style navigation includes `j/k` up/down, `h/l` left/right, `/` filter, `Esc` back/cancel (or Dashboard quit), `Enter` confirm, `Ctrl+S` save log, `D` edit date, and `d` delete.

Screen actions use typed `NavigationIntent` values. Ordinary module navigation prepares a freshly loaded replacement before changing the route; approved browse context such as stable-ID selection, filters, viewports, focused detail, and the Trends window survives where applicable. Unfinished edit/confirmation state, prior status, and cross-page undo payloads clear on re-entry. History edit intents carry the selected log's stable database ID, and Log Entry is built from that exact ID rather than an application-side selection lookup.

Navigation preparation is atomic: if target loading fails, neither the current route nor the cached target changes, and the source screen shows the localized error while retaining its in-place state. The navigation topology and shortcuts are unchanged. Batch 3A provides this concrete lifecycle routing. Batch 3B-1 Practices, Batch 3B-2 Quotes, Batch 3B-3 Trends, Batch 3B-4 History, and Batch 3B-5 Goals are shipped. Goals is the approved presentation redesign in this rollout; its persistent responsive Goals/Milestones panes are horizontal at body widths of `>=83` cells and stacked below that threshold.

Dashboard responsive-component extraction and Log Entry form extraction remain unshipped.

All text input fields use emacs-style cursor: `Ctrl+B`/Left back, `Ctrl+F`/Right forward, `Ctrl+A`/Home start, `Ctrl+E`/End end, `Ctrl+K` kill to end of line. Characters insert at cursor, not append.

**Every text input field must support the full emacs keybinding set:** `Ctrl+B`, `Ctrl+F`, `Ctrl+A`, `Ctrl+E`, `Ctrl+K`, `Left`, `Right`, `Home`, `End`, `Backspace`. This is a hard constraint — no text input may be added without these bindings. Use `tui::framework::TextInput` for single-line fields.

**Text input must never overflow its container.** Single-line inputs render with `TextInput::visible_spans()` so long text scrolls horizontally with the Unicode cursor. Never render raw `text[..cursor]` + cursor + `text[cursor..]` spans without width clipping.

Shared TUI mechanics live in `src/tui/framework/`: visual viewports, input state, detail/status/shortcut components, bounded rectangle helpers, and test support. Screens own domain state, database operations, validation, and modes. Selected content must remain visible, hidden visual rows use `↑/↓` indicators, and failed mutations retain their active input/focus while showing a visible error. Log Entry is verified at `80x24`; Dashboard is verified at `80x35`; both tolerate smaller direct-render rectangles without panicking.

Practices and Quotes consume all applicable presentation components: `ScreenFrame`, `ScrollableListPanel`, and `ActionPanel`. Trends consumes `ScreenFrame` and `ScrollableListPanel`; its filter rendering and specialized Sparkline/statistics policy remain Trends-owned. History consumes `ScreenFrame` and `ScrollableListPanel`; it retains filter rendering, dynamic log-column formatting, floating-preview placement, Detail-mode measurement and scrolling, edit navigation, delete/undo policy, and lifecycle behavior through the existing `render_detail_panel` helper. `ScreenFrame` also provides a generic one-pass sections API so a screen can reserve its body sections, status, and shortcuts together. Those components own shared presentation geometry, while each screen continues to own domain modes, database operations, validation, shortcuts, and lifecycle behavior. Quotes additionally owns its floating-preview placement and Detail-mode policy, integrating them through the existing `render_detail_panel` helper.

Goals uses `ScreenFrame`, the span-aware `render_scrollable_list_panel_spanned` sibling, and the pinned-header `render_detail_panel_with_header` sibling: selected Goal spans reveal and highlight across wrapped lines, and milestone rows scroll beneath the selected Goal's pinned derived-progress header. The existing `ScrollableListPanel` and `render_detail_panel` APIs remain accepted for their current consumers.

Log Entry uses shared `TextInput` state for its filter, set fields, warm-up, cool-down, RPE, note, and date editor, plus `ViewportState` for its clamped practice selector. Its shortcuts must be generated from the active filter/field/edit/confirmation mode. Validation and database failures retain the complete form, cursor, focus, editing identity, and return target.

Dashboard navigation is limited to Log Entry (`l`), History (`h`), Trends (`t`), Practices (`e`), Goals (`g`), Quotes (`Q`), HRV (`v`), and quit (`Esc`); there is no `w` action. Its status and multi-row shortcut footer are reserved before content layout. HRV validation or write failure retains `HrvInput` and its text; only a successful write clears the input and returns to normal mode. Quit confirmation and HRV input advertise only their active actions.

**All text content must wrap within its container.** Every `Paragraph` that renders user-entered or variable-length text (notes, names, descriptions, quotes, warm-up/cool-down) must use `.wrap(Wrap { trim: false })`. Text must never overflow the right border of its container. Only fixed-width UI elements (footers, shortcuts bars, single-line labels) may omit `.wrap()`.

## Color Scheme

Most theme roles use ANSI terminal colors. The shared selected-row highlight intentionally uses an RGB background, and the empty gauge fill uses an indexed terminal color:

| Role | Color |
|---|---|
| Accent (titles, shortcuts) | Cyan |
| Active input/selected text | Green |
| Shared selected-row background | `Rgb(204, 163, 0)` with Black foreground |
| Empty gauge fill | `Indexed(240)` |
| Bright highlight | LightGreen |
| Labels, borders | DarkGray |
| Error, delete | Red |
| Notes | Yellow |

## Testing

```bash
cargo test                                          # all tests
cargo test --test model_test                        # 6 model unit tests
cargo test --test db_test                           # database and migration tests
cargo test --test export_test                       # basic export/import tests
cargo test --test export_import_integration_test    # complete round-trip tests
```

Integration tests use file-based SQLite databases via `TestDb` helper (creates temp dir, auto-cleans on drop). All tests use `create_log_at` with explicit timestamps for deterministic assertions.

## Conventions

- Screen pattern: each screen is a struct with `new(db)`, `render(frame)`, `handle_key(key, ...)` returning `Action` (None/Navigate/Quit).
- Color constants defined per-screen as `const ACCENT`, `const GREEN`, etc. using ANSI colors.
- Log Entry and Dashboard mutation failures must remain visible and retain their active editing state; do not clear inputs or change mode before a successful database write.
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

When update CLAUDE.md
- after you added new feature that is different from CLAUDE.md, update CLAUDE.md to reconcile.

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
