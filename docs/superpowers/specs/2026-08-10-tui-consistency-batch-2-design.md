# TUI Consistency Framework — Batch 2 Design

**Date:** 2026-08-10
**Status:** Approved for implementation planning
**Branch:** `codex/tui-consistency`

## Objective

Batch 2 completes the shared-interaction migration for Log Entry and Dashboard and permanently removes Quick Log and every feature that exists only to support it.

This specification supersedes the Batch 2 and Dashboard sequencing in `2026-08-10-tui-consistency-framework-design.md`. Quick Log is not migrated. Dashboard moves into Batch 2. No other screen is added to this batch.

## Scope

### Included

- Migrate Log Entry to the Batch 1 framework primitives.
- Migrate Dashboard input, status, shortcut, and responsive-layout behavior to the shared framework.
- Remove Quick Log from navigation, application routing, source modules, tests, documentation, localization, and dependencies.
- Remove the LLM client and LLM-only configuration loader.
- Remove abbreviation models, database APIs, tests, documentation, localization, and schema.
- Permanently drop the `abbreviations` table from existing databases when they are opened.
- Update English and Chinese user documentation and the authoritative TUI specifications.

### Excluded

- Further behavior changes to Goals, History, Quotes, Trends, or Practices beyond regression fixes required by shared framework changes.
- A replacement natural-language logging workflow.
- Export, backup, or recovery of abbreviation rows before deletion.
- Changes to the training log, practice, goal, quote, HRV, import, or export data models.
- A new general-purpose multi-line text buffer.

## Destructive Removal Contract

Quick Log removal is deliberate and irreversible.

On every database initialization, Iron executes:

```sql
DROP TABLE IF EXISTS abbreviations;
```

Consequences:

- Existing abbreviation rows are permanently deleted the first time the updated application opens the database.
- New databases never create the table.
- No abbreviation model or CRUD API remains.
- Iron does not export or preserve abbreviation rows before dropping the table.
- Removing the feature does not alter any training log, practice, goal, quote, or daily metric row.

The schema cleanup must run inside `Database::init_schema` before initialization returns. Failure to drop the table fails database opening instead of silently leaving a partially removed feature.

## Quick Log Removal Architecture

### Application and TUI

- Delete `src/tui/quick_log.rs`.
- Remove `pub mod quick_log` and `Screen::QuickLog` from `src/tui/mod.rs`.
- Remove Quick Log construction, rendering, key handling, background-result polling, and navigation reset logic from `src/app.rs`.
- Remove the Dashboard `w` action and every rendered Quick Log shortcut.
- Restore the application event loop to blocking `event::read()` because no background LLM result remains to poll.

### LLM and Configuration

- Delete `src/llm.rs` and `src/config.rs`.
- Remove their module declarations and public exports from `src/main.rs` and `src/lib.rs`.
- Delete `tests/llm_test.rs` and `tests/config_test.rs`.
- Remove `ureq` and `toml` from direct dependencies.
- Retain `dirs`, because database location discovery still uses it.
- Retain `serde` and `serde_json`, because import/export still uses them.

### Models and Database

- Remove `Abbreviation`, `ParsedLog`, `RawParsedLog`, and `RawSet` from `src/model.rs` when no non-Quick-Log consumer remains.
- Remove abbreviation imports and CRUD methods from `src/db.rs`.
- Remove abbreviation database tests.
- Remove `CREATE TABLE IF NOT EXISTS abbreviations` from the current schema.
- Add the explicit destructive `DROP TABLE IF EXISTS abbreviations` migration.

### Documentation and Localization

- Remove Quick Log and Abbreviations specifications and the historical implementation plan/design documents dedicated to those features.
- Remove Quick Log and Abbreviations sections and shortcut entries from both READMEs.
- Remove Quick Log and Abbreviations from Dashboard, navigation, data-model, and framework specifications.
- Remove Quick Log and Abbreviations localization keys from both locale files.
- Remove Quick Log from TUI lint screen inventories.
- Historical documents that describe broader work may retain a clearly marked historical reference only when deleting the complete document would erase unrelated accepted design context. Active specifications and user documentation must contain no live Quick Log behavior.

## Log Entry Design

### Preserved Workflow

Log Entry keeps its existing two top-level phases:

1. Select an active practice.
2. Enter or edit sets, warm-up, cool-down, RPE, note, and date.

New logs return to Dashboard after a successful save. Logs opened from History return to History after a successful update. Practice-type-specific fields, weight carry-forward, set editing, date editing, exit confirmation, and validation rules remain domain-owned by `LogEntryScreen`.

### Shared State Migration

Every current `String` plus byte-cursor pair becomes a framework `TextInput`:

- practice filter
- first and second set fields
- warm-up
- cool-down
- RPE
- note
- date editor

The practice list replaces local `scroll` and `list_height` calculations with `ViewportState`. Navigation uses the explicitly selected edge behavior and reveals the selected practice using `VisualSpan`. Column widths and user-supplied practice names remain bounded to the real inner rectangle.

### Rendering and Focus

- Every input uses `TextInput::visible_spans`, preserving a visible Unicode-safe cursor.
- The active input or section has a visible accent treatment.
- The practice list title reports `↑`, `↓`, or `↑↓` when items exist outside the viewport.
- Status occupies a reserved row and never replaces content.
- Shortcuts use `Shortcut`, `shortcut_height`, and `render_shortcuts`, so every active command remains represented at the supported width.
- The screen renderer must not panic for rectangles smaller than the application minimum.

### Mutation and Error Semantics

- Validation errors keep the active field, cursor, focus section, and entered data.
- Database create/update failure keeps the complete form and return target.
- Mode, form, and editing identity clear only after the database mutation succeeds.
- Exit confirmation remains explicit when unsaved state exists.
- Status text is shown in the reserved status row.

## Dashboard Design

### Preserved Content

Dashboard continues to show:

- logo/header
- 52-week training heatmap
- weekly summary and streak information
- recent training entries
- active goals and milestone-derived progress
- daily quote
- current HRV value and HRV editor
- navigation and quit confirmation

Quick Log and Abbreviations are removed without replacement. The remaining navigation targets are Log Entry, History, Trends, Practices, Goals, and Quotes.

### Shared Interaction Migration

- Replace `hrv_input: String` with `TextInput`.
- Render HRV editing with a bounded visible cursor.
- Keep invalid numeric or range input in `HrvInput` with an error status.
- Clear HRV input and return to normal mode only after `set_daily_hrv` succeeds.
- Render status through the shared reserved status component.
- Replace the hand-built fixed four-line footer with shared complete-command shortcut layout.
- Render quit confirmation through active-mode shortcuts/status without advertising inactive commands.

### Responsive Layout

Dashboard keeps a vertical content stack and a split Recent/Goals region, but all computed heights are clamped to the actual frame.

- `80x35` is the primary Dashboard acceptance size.
- Smaller direct-render test rectangles must not panic.
- Resizing recomputes inner widths and heights on every render.
- Recent and Goals content must remain inside their bordered panels.
- Long practice names, goal titles, quotes, and localized labels wrap or truncate inside their real inner width.
- Status and shortcut rows reserve their required height before content panels receive the remaining space.

## Data and Control Flow

### Application Startup

1. `Database::open_default` opens the SQLite database.
2. `Database::init_schema` creates current tables and drops `abbreviations` if it exists.
3. The application constructs Dashboard, Goals, Log Entry, History, Trends, Practices, and Quotes screens.
4. The event loop blocks on keyboard input and routes actions among the remaining screens.

### Log Entry Save

1. The screen validates active inputs and converts them to domain values.
2. New entries call the existing create-log database path; existing entries call the existing update-log path.
3. Failure retains form state and displays an error.
4. Success navigates to the correct return screen, whose data is refreshed by application routing.

### Dashboard HRV Save

1. `v` enters `HrvInput` and preloads the existing value if present.
2. `TextInput` handles editing.
3. `Enter` validates an integer in the inclusive range `0..=100`.
4. Validation or database failure retains `HrvInput` and its contents.
5. Success updates the displayed HRV value, clears input, and returns to normal mode.

## Testing Strategy

All behavior changes follow red-green-refactor.

### Removal Tests

- A new in-memory database contains no `abbreviations` table.
- Opening a file database that contains abbreviation rows drops the table and preserves unrelated rows.
- Repository searches find no live Quick Log screen, route, shortcut, LLM config, abbreviation model, or abbreviation CRUD references.
- Dependency metadata contains no direct `ureq` or `toml` dependency.
- Locale files retain matching key sets after deletion.

### Log Entry Tests

- A long practice list reaches and displays the last item.
- A long Unicode filter keeps its cursor visible.
- Long set, warm-up, cool-down, RPE, note, and date fields stay inside their assigned rectangles.
- Edit mode preserves the original log identity and return-to-History behavior.
- Invalid date/RPE/set input retains field focus and contents.
- Database mutation failure retains the complete form.
- Tiny direct-render rectangles do not panic.
- Active shortcuts remain complete at `80x24`.

### Dashboard Tests

- Dashboard renders at `80x35` without overflow.
- Smaller direct-render rectangles do not panic.
- The footer contains every remaining active navigation command and no Quick Log command.
- Long goals, recent-entry names, quotes, and localized labels stay within panel bounds.
- Invalid HRV values retain input mode and text.
- Simulated database failure retains HRV input mode and reports an error.
- Quit confirmation exposes only confirmation/cancel actions.

### Release Gates

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

The worktree must be clean after commits. No superseded local input, cursor, list-scroll, status, or shortcut implementation remains in Log Entry or Dashboard.

## Commit and Review Boundaries

Implementation is divided into independently reviewable commits:

1. Destructive abbreviation schema migration and backend model/API removal.
2. Quick Log application/module/dependency removal.
3. Quick Log/Abbreviations documentation and localization removal.
4. Log Entry shared-framework migration.
5. Dashboard shared-framework migration and navigation cleanup.
6. Final documentation, repository-reference audit, and verification fixes.

Each boundary must pass its targeted tests. The complete branch must pass all release gates and a read-only code review before manual acceptance.

## Manual Acceptance

### Log Entry

- Navigate the first and last practice in a long list.
- Enter long Unicode values in every field and confirm the cursor remains visible.
- Add, edit, and delete sets for every practice type.
- Edit warm-up, cool-down, RPE, note, and date.
- Confirm validation failures retain the exact field and value.
- Create a new log and edit an existing History log.

### Dashboard

- Verify Heatmap, Summary, Recent, Goals, Quote, HRV, status, and shortcuts at `80x35`.
- Resize the terminal and confirm panels remain bounded.
- Verify `w` and all Quick Log/Abbreviation labels are absent.
- Enter valid and invalid HRV values.
- Verify quit confirmation and every remaining navigation shortcut.

### Removal

- Confirm Quick Log cannot be reached from Dashboard or routing.
- Confirm existing databases open successfully and no longer contain the `abbreviations` table.
- Confirm training logs and all unrelated user data remain intact.
