# TUI Consistency Framework Design

**Date:** 2026-08-10
**Status:** Superseded for Batch 2 by [the Batch 2 design](2026-08-10-tui-consistency-batch-2-design.md)
**Branch:** `codex/tui-consistency`
**Base:** validated Goals milestone-management implementation (`0069c04`)

## Context

The validated Goals redesign established observable interaction guarantees that are not consistently implemented elsewhere: selected content remains reachable, wrapped content is measured as visual rows, focused management is distinct from preview, long inputs keep the cursor visible, mutations preserve context on failure, and shortcuts describe the active mode.

The audit of Dashboard, History, Trends, Practices, Quotes, and Log Entry found repeated local implementations of selection, scrolling, input editing, status lines, and shortcut bars. Those implementations have drifted. Examples include logical-row scrolling for wrapped content, detail popups without their own viewport, raw cursor rendering, result scrolling without a known viewport height, lists without overflow signaling, and edit modes that close after a failed database write.

The project will replace those repeated interaction mechanisms with a unified TUI framework. Migration will be incremental rather than a big-bang rewrite.

## Goals

1. Provide reusable interaction and rendering primitives for list, detail, input, status, and shortcut behavior.
2. Migrate every applicable screen in three independently testable batches.
3. Preserve each module's domain state machine and specialized layout.
4. Make selection, overflow, focus, input, errors, and shortcuts predictable throughout the application.
5. Remove superseded local implementations after their consumers migrate.

## Non-Goals

- The framework will not own database connections, domain models, queries, or screen routing.
- Charts, heatmaps, forms, tables, and popups will not be forced into one generic widget.
- The database schema and on-disk format will not change.
- Existing keyboard commands will not be renamed unless current documentation and implementation contradict each other; those cases will be resolved explicitly in the migration batch.
- The visual theme will not be redesigned.

## Architecture

Create `src/tui/framework/` with four focused units.

### `viewport.rs`

Owns visual-row geometry and scroll state:

- `VisualSpan { start: usize, height: usize }`
- `visual_spans(lines: &[Line], width: u16) -> Vec<VisualSpan>`
- `keep_span_visible(offset, viewport_height, span) -> usize`
- `Overflow { above: bool, below: bool }`
- selection viewport state that supports explicit clamp or wrap navigation policy

All calculations use the actual inner rectangle after borders and padding. Wrapped lines, multi-line list items, and contextual action rows participate in the selected span.

### `input.rs`

Owns reusable text editing state and cursor visibility:

- `TextInput` for single-line Unicode text, cursor movement, insertion, deletion, and horizontal viewport rendering
- full Emacs bindings required by `CLAUDE.md`: `Ctrl+B/F/A/E/K`, arrows, Home, End, and Backspace
- rendering built on width-aware Unicode calculations rather than byte or character counts

The module exposes event-handling and render-state primitives. It does not validate domain-specific values such as dates, weights, or HRV.

### `components.rs`

Provides composable Ratatui presentation components:

- selectable list rendering driven by caller-provided lines and viewport state
- preview/focused detail panels with accent focus, selected-row highlighting, and `↑`, `↓`, or `↑↓` overflow indicators
- status bar rendering with consistent success and error colors
- shortcut bar rendering from active-mode shortcut descriptors
- bordered panel construction using the real inner rectangle

Components receive already-formatted domain content and return no domain data.

### `test_support.rs`

Provides reusable TestBackend helpers for rendering screens to text and asserting:

- content is inside its border
- selected spans remain visible
- overflow markers match scroll state
- cursor and error status remain visible
- minimum-size layouts do not panic

This module is compiled for tests only.

## Ownership and Data Flow

Screens continue to own domain data, modes, validation, and database operations. The framework owns interaction state and presentation mechanics.

```text
domain data
    -> screen formats list/detail/input content
    -> framework calculates visual spans and viewport
    -> framework renders components
    -> screen receives input event and performs domain action
    -> successful mutation reloads data and preserves stable identity
```

Stable database IDs, not array positions, preserve selection across reloads and sorting changes.

## Uniform Behavior Contract

### Selection and Scrolling

- The active item or active contextual row is always visible.
- Wrapped content is measured in visual terminal rows.
- Hidden content is indicated with `↑`, `↓`, or `↑↓`.
- Each list declares whether navigation clamps at the ends or wraps; the footer must reflect the implemented behavior.
- Reloading data preserves the selected entity by stable ID when it still exists.

### Preview and Focus

- Read-only preview uses the standard border color.
- Focused management uses the accent border and highlights the active item across all wrapped rows.
- `Enter` focuses a manageable detail panel when the screen has a preview/detail distinction.
- `Esc` first returns from focused detail to its parent list, then returns to the previous screen.

### Text Input

- All single-line inputs use `TextInput` and remain horizontally bounded.
- Empty or invalid values retain focus and display a useful status message.

### Mutations and Errors

- A mode changes and input clears only after the database mutation and required reload succeed.
- Failed edits retain the original input and cursor.
- Failed deletes retain selection; failed undo retains the undo payload.
- Errors appear in a reserved status line that remains visible even when a panel fills its viewport.

### Status and Shortcuts

- Status bars use one shared renderer and fixed success/error semantics.
- Shortcut bars are generated from the active mode and remain consistent with README and specifications.
- Long shortcut bars may use multiple reserved lines, but may not silently overwrite content.

## Three Migration Batches

### Batch 1: Framework Foundation and List/Detail Screens

Build the framework's visual viewport, single-line input, list/detail, status, shortcut, and test primitives. Add characterization tests around the accepted Goals behavior, move Goals to the framework without changing its observable behavior, then migrate History, Trends, Practices, and Quotes.

- History: filter input becomes bounded; the selected log detail becomes a focusable, scrollable panel; list selection and detail overflow remain visible.
- Quotes: the list remains compact while the selected quote has a complete preview/focused view; long quote input remains visible.
- Trends: practice list, filtering, selection, and footer migrate; chart-specific rendering remains specialized.
- Practices: selection, long names, input, status, footer, and mutation transitions migrate; the type selector remains module-specific content inside a framework panel.

Batch 1 acceptance requires Goals regression parity plus list/detail verification at `80x24`.

### Batch 2: Superseded

Batch 2 is superseded by [the Batch 2 design](2026-08-10-tui-consistency-batch-2-design.md), which defines the approved Log Entry and Dashboard migration and removal scope.

### Batch 3: Dashboard and Framework Consolidation

Migrate Dashboard status, shortcuts, HRV input, and bounded content panels. Keep Heatmap and summary calculations specialized.

- Dashboard Goals display `completed/total · percentage` using the same progress formatting as Goals.
- Recent entries, goals, and quote content remain inside their panels and expose overflow when applicable.
- HRV uses the shared input behavior; invalid or failed saves preserve input and show status.
- Remove local input, viewport, status, and footer code made obsolete by completed migrations.
- Update all specifications, English and Chinese README sections, `CLAUDE.md`, and architectural documentation.

Batch 3 acceptance uses `80x35` for Dashboard. Smaller terminals must degrade without panicking or drawing outside their assigned rectangles.

## Testing Strategy

### Framework Tests

- visual spans for ASCII, CJK, empty, and wrapped lines
- clamp and wrap navigation
- selection moving below and above the viewport
- overflow state at first, middle, and last positions
- Unicode cursor movement and deletion
- horizontal and vertical input scrolling
- focused and preview component rendering

### Screen Tests

Before migration, record the behavior that must remain. After migration, use TestBackend at the batch's minimum size to cover:

- first and last item reachability
- wrapped selected content
- long inputs with a visible cursor
- focus entry and exit
- delete, undo, invalid input, and failed-write status behavior
- footer commands for every mode

### Data Tests

- selection identity after database-driven reorder or reload
- no database schema change

Every batch must pass:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

## Delivery and Rollback

- Work continues on `codex/tui-consistency`, based on the validated Goals commit.
- Each framework unit and each migrated screen receives an independently reviewable commit.
- Each batch is delivered for manual acceptance before the next batch begins.
- Old and new implementations may coexist only while a batch is in progress. Once a screen passes its batch acceptance, its superseded local implementation is removed.
- Commit boundaries allow an individual screen migration to be reverted without discarding the framework or previously accepted screens.
- The final release version is `3.3.0`.

## Manual Acceptance Summary

### Batch 1

Navigate long Goals, History, Trends, Practices, and Quotes collections; verify every item and detail is reachable, focus is visible, overflow is signaled, long filters/edits retain the cursor, and mutation errors retain context.

### Batch 3

Verify Dashboard panels, Goals progress, HRV validation, status, and footer behavior at `80x35` and during terminal resizing; confirm every README shortcut matches the application.
