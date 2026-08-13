# TUI Consistency Framework Design

**Date:** 2026-08-10
**Status:** Batches 1, 2, 3A, 3B-1, 3B-2, and 3B-3 complete; History and later presentation-component slices are not shipped
**Branch:** `codex/tui-consistency`
**Base:** validated Goals milestone-management implementation (`0069c04`)

## Context

The validated Goals redesign established observable interaction guarantees that are not consistently implemented elsewhere: selected content remains reachable, wrapped content is measured as visual rows, focused management is distinct from preview, long inputs keep the cursor visible, mutations preserve context on failure, and shortcuts describe the active mode.

The audit of Dashboard, History, Trends, Practices, Quotes, and Log Entry found repeated local implementations of selection, scrolling, input editing, status lines, and shortcut bars. Those implementations have drifted. Examples include logical-row scrolling for wrapped content, detail popups without their own viewport, raw cursor rendering, result scrolling without a known viewport height, lists without overflow signaling, and edit modes that close after a failed database write.

The project has replaced those repeated interaction mechanisms with a unified TUI framework through incremental delivery batches.

## Goals

1. Provide reusable interaction and rendering primitives for list, detail, input, status, and shortcut behavior.
2. Migrate every applicable screen through independently testable delivery batches.
3. Preserve each module's domain state machine and specialized layout.
4. Make selection, overflow, focus, input, errors, and shortcuts predictable throughout the application.
5. Remove superseded local implementations after their consumers migrate.

## Non-Goals

- The framework will not own database connections, domain models, queries, or screen routing.
- Charts, heatmaps, forms, tables, and popups will not be forced into one generic widget.
- The framework primitives do not own schema changes. Batch 2 separately and intentionally removes the obsolete `abbreviations` table while preserving unrelated data.
- Existing keyboard commands will not be renamed unless current documentation and implementation contradict each other; those cases will be resolved explicitly in the migration batch.
- The visual theme will not be redesigned.

## Architecture

`src/tui/framework/` provides four focused units.

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

## Migration Delivery Status

### Batch 1: Framework Foundation and List/Detail Screens — Complete

Batch 1 built the framework's visual viewport, single-line input, list/detail, status, shortcut, and test primitives. It added characterization tests around the accepted Goals behavior, moved Goals to the framework without changing its observable behavior, then migrated History, Trends, Practices, and Quotes.

- History: filter input becomes bounded; the selected log detail becomes a focusable, scrollable panel; list selection and detail overflow remain visible.
- Quotes: the list remains compact while the selected quote has a complete preview/focused view; long quote input remains visible.
- Trends: practice list, filtering, selection, and footer migrate; chart-specific rendering remains specialized.
- Practices: selection, long names, input, status, footer, and mutation transitions migrate; the type selector remains module-specific content inside a framework panel.

Batch 1 completed with Goals regression parity plus list/detail verification at `80x24`.

### Batch 2: Log Entry, Dashboard, and Permanent Removal — Complete

The [approved Batch 2 design](2026-08-10-tui-consistency-batch-2-design.md) superseded this document's original sequencing and moved Dashboard into Batch 2. The shipped batch:

- migrates Log Entry's filter, practice selector, set fields, warm-up, cool-down, RPE, note, date editor, status, and active-mode shortcuts to shared framework primitives;
- keeps the selected practice visible with overflow indicators and bounds all Log Entry content to actual inner rectangles, including smaller direct-render sizes;
- retains Log Entry form, cursor, focus, editing identity, and return target on validation or database failure;
- migrates Dashboard HRV input, status, shortcuts, and responsive bounded layout, with complete remaining navigation at `80x35` and no stale `w` command;
- retains Dashboard HRV mode and input on validation or write failure and clears them only after a successful write;
- renders Dashboard shortcuts dynamically for Normal, HRV input, and quit-confirmation modes; and
- permanently removes Quick Log without a replacement, along with its LLM configuration/runtime, Abbreviations models and CRUD API, locale/test/dependency interfaces, and `abbreviations` table data.

Database initialization executes `DROP TABLE IF EXISTS abbreviations`. Existing abbreviation rows are irreversibly deleted on first open by the updated application; training logs, practices, goals, quotes, daily metrics, and other unrelated data are preserved.

### Batch 3A: Typed Navigation and Screen Lifecycle — Complete

Batch 3A preserves the existing navigation topology and shortcuts while making transitions typed and transactional:

- screens emit ordinary module, new-log, or stable-ID History edit intents;
- History edit preparation resolves the exact requested log ID instead of consulting a current screen selection;
- Dashboard, Goals, History, Trends, Practices, and Quotes reload current database data on ordinary entry while preserving their approved stable-ID browse context, filters, viewports, focused detail, and Trends window where applicable;
- unfinished edit/date/delete-confirmation modes, old status, and cross-page undo payloads clear on re-entry, while in-place mutation failures continue to retain their active state;
- a concrete `ScreenStore` prepares a complete replacement before a `Router` installs it and commits the route; and
- preparation failure leaves the route and cached target unchanged and reports a localized error on the source screen without resetting it.

The terminal loop now delegates screen behavior to a terminal-independent application shell and contains no concrete screen construction or refresh policy.

### Batch 3B-1: Practices Presentation Components — Complete

Batch 3B-1 adds presentation-only `ScreenFrame`, `ScrollableListPanel`, and `ActionPanel` components and migrates only Practices to them. The frame reserves status and shortcut rows; the list panel owns border, overflow, viewport reveal and clamp, and selected-row highlight; and the action panel owns bounded contextual rendering. Practices retains all domain line formatting, modes, database operations, validation, shortcuts, and lifecycle behavior. Five immutable styled-buffer fixtures preserve exact output across Browse, AddName, AddType, EditName, and ConfirmDelete.

### Batch 3B-2: Quotes Presentation Components — Complete

Batch 3B-2 extends `ScreenFrame` with a generic one-pass sections API and migrates Quotes to `ScreenFrame`, `ScrollableListPanel`, and `ActionPanel`. The frame reserves the list, spacer, optional action area, status, and wrapped shortcut rows in one composition pass. The list panel returns safe outer and inner `ListPanelAreas` geometry; Quotes uses only that returned inner rectangle to measure and position its floating preview. The action panel bounds Add, Edit, and ConfirmDelete content.

Quotes retains quote row formatting, domain modes, database operations, input state, mutation ordering, undo, shortcuts, lifecycle behavior, floating-preview placement, and Detail focus/scroll policy. Its preview remains Quotes-owned integration through the existing `render_detail_panel` helper rather than a new generic popup abstraction. Six immutable styled-buffer fixtures preserve exact output across Browse, Browse with Undo, Detail, Add, Edit, and ConfirmDelete; the five Practices fixtures also remain unchanged.

### Batch 3B-3: Trends Presentation Components — Complete

Batch 3B-3 migrates Trends to `ScreenFrame` and `ScrollableListPanel`. The frame receives exactly the accepted filter, list, and chart body constraints with trailing flex disabled, reserving status and shortcut rows without changing their geometry. The list panel is the sole owner of the Trends list border, title and overflow markers, viewport reveal and clamp, and selected-row highlight.

Trends retains its filter rendering, row formatting, selection and wrap policy, database query, time-window controls, error and re-entry behavior, and its specialized Sparkline/chart and statistics policy. Six immutable styled-buffer fixtures preserve exact output for a populated chart, long filter input, no filter match, list overflow, no log data, and a cached chart query error; all five Practices and six Quotes fixtures also remain unchanged.

History, Goals popups, Dashboard responsive layout, and Log Entry forms remain unshipped as later presentation-component extraction slices. Batch 3B-3 changes no other screen and does not establish that the Trends component subset is suitable for those later slices.

## Testing Strategy

### Framework Tests

- visual spans for ASCII, CJK, empty, and wrapped lines
- clamp and wrap navigation
- selection moving below and above the viewport
- overflow state at first, middle, and last positions
- Unicode cursor movement and deletion
- horizontal single-line input scrolling
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
- new databases contain no `abbreviations` table
- existing databases drop `abbreviations` while preserving unrelated rows

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
- Each batch is delivered for manual acceptance before any later batch begins.
- Old and new implementations may coexist only while a batch is in progress. Once a screen passes its batch acceptance, its superseded local implementation is removed.
- Commit boundaries allow an individual screen migration to be reverted without discarding the framework or previously accepted screens.
- The final release version is `3.3.0`.

## Manual Acceptance Summary

### Batch 1

Navigate long Goals, History, Trends, Practices, and Quotes collections; verify every item and detail is reachable, focus is visible, overflow is signaled, long filters/edits retain the cursor, and mutation errors retain context.

### Batch 2

Verify Log Entry selection overflow, bounded Unicode cursors, active-mode shortcuts, and failure retention at `80x24`. Verify Dashboard Heatmap, Summary, Recent, Goals, quote, HRV, status, and mode-specific shortcuts at `80x35` and during resizing. Confirm `w`, Quick Log, LLM configuration/runtime, and Abbreviations interfaces are absent, and confirm opening an existing database removes only the obsolete `abbreviations` table.

### Batch 3A

Verify every ordinary module reloads external database changes while retaining its approved stable selection, filter, viewport, detail focus, and Trends window. Confirm unfinished cross-page mutation state, prior status, and undo payloads clear; History save/cancel returns to the same filtered selected log and a second edit uses refreshed data; and a forced preparation failure stays on the source screen with a visible error and unchanged cached target. Confirm all existing routes, shortcuts, Batch 1/2 layouts, and permanent removal guarantees remain unchanged. Do not begin Batch 3B until Batch 3A receives explicit manual acceptance.
