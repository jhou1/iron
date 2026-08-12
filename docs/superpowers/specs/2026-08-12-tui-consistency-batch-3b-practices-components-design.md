# TUI Consistency Framework — Batch 3B-1 Practices Components Design

**Date:** 2026-08-12

**Status:** Approved for implementation planning

**Base:** `b2c55912b41fd71f67991a9197a60b93a68a4f6c` on `codex/tui-consistency`

## Summary

Batch 3A centralized application navigation and screen lifecycle without changing the accepted screen behavior. Batch 3B begins the next structural layer: extracting repeated presentation mechanics into reusable Ratatui components while keeping each screen's database access, domain modes, validation, key handling, and shortcut policy local to that screen.

Batch 3B is intentionally incremental. This document defines Batch 3B-1 only: introduce three presentation components and migrate Practices as the first vertical slice. The terminal output and shortcut behavior must remain as close to byte-for-byte unchanged as Ratatui permits. Any intentional visual redesign belongs to a later, separately approved batch.

After implementation, Practices is delivered alone for manual acceptance. No other module is migrated until that acceptance is complete.

## Goals

- Freeze the current Practices rendering for all five modes with exact terminal-buffer fixtures.
- Extract shared screen-frame, scrollable-list, and action-panel presentation mechanics into the TUI framework.
- Migrate Practices to those components without changing its visible output, shortcuts, business behavior, persistence behavior, lifecycle behavior, or failure-retention semantics.
- Make every new component safe for empty, zero-sized, narrow, and short rectangles.
- Establish component contracts that later slices can reuse without prematurely designing around every screen.
- Remove the Practices-local layout, viewport rendering, overflow-title, and selected-row rendering code that the new components replace.

## Non-Goals

- No visual redesign, copy change, color change, border change, spacing change, shortcut change, or navigation change.
- No migration of Quotes, Trends, History, Goals, Dashboard, or Log Entry in Batch 3B-1.
- No database, data-model, schema, localization, routing, or lifecycle change.
- No extraction of business modes, key handling, shortcut selection, validation, error creation, or mutation logic from Practices.
- No popup state-machine abstraction, form framework, responsive-dashboard system, or general screen trait-object architecture.
- No coexistence of both the old and new Practices rendering implementations after migration.
- No reintroduction or replacement of Quick Log, LLM configuration, or Abbreviations.

## Chosen Approach

The project will use a single-screen vertical slice, starting with Practices.

This is preferred over extracting a broad component library before migration because Practices contains a representative but bounded combination of list browsing, add/edit actions, type selection, delete confirmation, status messages, dynamic shortcuts, and viewport behavior. It is large enough to validate the seams and small enough to compare exhaustively.

The rejected alternatives are:

- **Framework-first extraction across all screens:** too much surface area before any component contract is proven against a complete screen.
- **Mechanical multi-screen migration:** lowers short-term repetition but makes regressions, component overreach, and output drift harder to localize.

## Compatibility Contract

Batch 3B-1 is a structural refactor, not a behavior change.

At an `80x24` terminal, the complete rendered buffer must match the pre-refactor fixture for each Practices mode:

- Browse
- AddName
- AddType
- EditName
- ConfirmDelete

The fixtures include borders, titles, overflow indicators, headers, row spacing, colors and modifiers, selected-row highlighting, action text, input cursor presentation, status placement, shortcut text, and unused cells. Comparison must use the rendered Ratatui buffer rather than a normalized or trimmed string.

Existing tests for small rectangles, long practice names, failure retention, shortcuts, and lifecycle re-entry remain binding. If an existing test and a new golden fixture expose a current defect, implementation stops for a scope decision rather than silently changing output.

## Ownership Boundary

The components own presentation mechanics only. Practices continues to own all domain and interaction policy.

### Practices Owns

- `Practice` values and stable practice identities.
- `Mode`, including Browse, AddName, AddType, EditName, and ConfirmDelete.
- Text input state and type-selection state.
- Database reads and mutations.
- Validation, localized success/error creation, and failure retention.
- Key handling and the active shortcut set for each mode.
- Navigation actions and lifecycle re-entry behavior.
- Formatting domain-specific list columns, rows, action copy, input lines, and type lines.
- Deciding which row, action, or type is selected.

### Framework Components Own

- Rectangle subdivision that depends only on presentation measurements.
- Borders, padding, titles, headers, and generic row rendering.
- Shortcut wrapping measurement and placement.
- Status and shortcut rendering through the existing shared helpers.
- Generic viewport reveal, clamp, and visible-row calculation.
- Generic selected-row style application.
- Safe clipping and no-op rendering for insufficient space.

The components do not import database types, Practices domain types, navigation intents, screen modes, or validation errors.

## Component Architecture

The final Rust API may use structs, parameter objects, or free functions where that improves lifetimes and call-site clarity. Regardless of syntax, the following responsibilities are fixed.

### ScreenFrame

`ScreenFrame` receives:

- the available `Rect`;
- the active, already-selected shortcut list;
- the optional current status message; and
- the existing horizontal content-width constraint used by Practices.

It:

- computes the actual wrapped shortcut height at the available width;
- centers or bounds the existing content region exactly as Practices does today;
- divides the content region into body, status, and shortcuts rectangles;
- renders the status and shortcut rows using existing framework semantics; and
- returns or exposes the body rectangle for screen-specific subdivision.

`ScreenFrame` does not decide which shortcuts are active, alter shortcut text, reserve speculative rows, or know the screen mode.

### ScrollableListPanel

`ScrollableListPanel` receives:

- a target `Rect`;
- an already-formatted title;
- an optional already-formatted header;
- already-formatted logical rows;
- an optional selected logical-row index;
- a mutable generic `ViewportState`; and
- presentation styles required to reproduce the current Practices output.

It owns:

- the bordered and padded list region;
- the overflow suffix in the title;
- the fixed header row;
- available content-height measurement;
- selected-row reveal;
- viewport offset clamp;
- visible-row selection; and
- selected-row highlighting.

Practices remains responsible for formatting names and type columns. The component must not infer column widths from `Practice` or any other domain value.

The component mutates only the supplied `ViewportState`. It does not mutate selected identity, selected index, screen mode, or domain data.

### ActionPanel

`ActionPanel` receives:

- a target `Rect`;
- already-formatted action, prompt, input, confirmation, or type lines;
- an optional selected line index; and
- presentation styles required to reproduce the current Practices output.

It owns:

- bounded line rendering inside the action region; and
- generic selected-line highlighting, including the current AddType selection appearance.

It does not own text editing, cursor movement, practice-type meaning, deletion confirmation policy, or mode transitions.

## Render and Data Flow

The Practices render path follows this sequence:

1. Practices derives its active shortcuts from its current mode and existing key-handling policy.
2. Practices formats the domain-specific list title, header, rows, action lines, input lines, type lines, selected indices, and status.
3. `ScreenFrame` measures the actual shortcut wrapping and divides the available area into body, status, and shortcut regions.
4. Practices divides the body into the current list, spacer, and mode-dependent action regions using the existing heights and constraints.
5. `ScrollableListPanel` renders the list using the actual list width and mutates only the generic list viewport.
6. `ActionPanel` renders the already-formatted action content and optional selected action row.
7. `ScreenFrame` renders the status and shortcuts.

There is no second source of truth for layout or state. After migration, Practices no longer contains a parallel local implementation of the mechanics delegated to the components.

## Geometry and Safety

All geometry uses saturating arithmetic, clamping, and the existing `fit_rect` semantics where appropriate.

For a zero-sized or insufficient rectangle, a component may render only the safe subset that fits or render nothing. It must never:

- construct a child rectangle outside its parent;
- assume a border has a non-empty inner rectangle;
- index a Ratatui buffer outside the frame;
- subtract dimensions without saturation;
- require a selected row to exist; or
- panic because a header, status, shortcut, or action region is absent.

Viewport offsets are clamped to the content height that can actually be rendered. Selection reveal uses the logical row count and current visible height. The header remains fixed and does not consume a logical data-row index.

## Error and Mutation Semantics

The new components are rendering utilities and have no business-error channel. They do not return database or domain errors and do not depend on `anyhow`.

Practices retains its existing mutation guarantees:

- validation or database failure keeps the current mode active;
- entered text remains available for correction or retry;
- type selection remains unchanged;
- selected practice identity remains unchanged where the existing contract requires it; and
- the localized error remains visible in the reserved status area.

Component extraction must not reorder a database mutation, clear transient input, or change when a status is set.

## Testing Strategy

### Commit 1: Characterization Fixtures

Before production extraction, add golden fixtures for all five Practices modes at `80x24` under:

```text
tests/fixtures/tui/practices/
```

The tests render the existing implementation and compare the entire styled buffer. Fixtures must be deterministic: database rows, dates, selection, input cursor, status, locale, and terminal size are explicitly controlled.

The characterization commit establishes the accepted baseline. It must pass before framework code is added.

### Commit 2: Component Tests

Add and test:

- `src/tui/framework/screen_frame.rs`
- `src/tui/framework/list_panel.rs`
- `src/tui/framework/action_panel.rs`

Required component coverage includes:

- dynamic shortcut wrapping reserves the correct number of rows;
- body, status, and shortcut rectangles do not overlap or escape the parent;
- zero-sized, narrow, and short parents do not panic;
- list header remains fixed while data rows scroll;
- selected rows are revealed and viewport offsets are clamped;
- empty lists and absent selections render safely;
- overflow titles reproduce the existing suffix behavior;
- selected action rows receive only the requested highlight; and
- oversized lines are clipped or wrapped exactly according to the current Practices contract.

Tests should inspect both computed rectangles/state and rendered buffers where appearance matters.

### Commit 3: Practices Migration and Documentation

Migrate Practices, remove the replaced local mechanics, and rerun the golden tests without fixture changes. A fixture change is a behavior change and requires a separate user ruling.

Retain and run:

- the full Practices test suite;
- Practices lifecycle and failure-retention tests;
- TUI lint tests;
- strict Clippy across all targets and features;
- formatting checks;
- all-target, all-feature build and test gates; and
- `git diff --check`.

If `tests/tui_lint_test.rs` uses line-number allowlists and a widget moves mechanically, only the mapping for that exact widget may be updated. The old and new source site must be verified before changing the allowlist.

## Planned Files

Expected new production files:

- `src/tui/framework/screen_frame.rs`
- `src/tui/framework/list_panel.rs`
- `src/tui/framework/action_panel.rs`

Expected modified production files:

- `src/tui/framework/mod.rs`
- `src/tui/practices.rs`

Expected test assets or test files:

- `tests/fixtures/tui/practices/*.txt`
- component and Practices tests colocated with their modules or in the existing TUI test structure
- `tests/tui_lint_test.rs` only if a verified mechanical allowlist refresh is necessary

Expected documentation updates after implementation:

- `CLAUDE.md`
- the TUI framework design
- the Practices specification
- navigation or README documentation only if needed to state shipped-versus-unshipped Batch 3B scope accurately

This list is a scope forecast rather than permission to change unrelated files. Any required tracked file outside the implementation plan triggers a scope review.

## Delivery and Rollback

Batch 3B-1 uses three independently reviewable commits:

1. characterize Practices rendering;
2. add the three framework components and their tests; and
3. migrate Practices and update documentation.

Each commit must pass its applicable gates and remain independently reversible. The component commit may exist before Practices consumes the components, but after the migration commit there is only one Practices rendering path.

The delivery ends after Practices is ready for manual verification. Later screen migrations do not begin automatically.

## Manual Acceptance

Manual verification covers:

- Browse mode, including list selection and scrolling;
- AddName input and cursor behavior;
- AddType selection and highlighting;
- EditName input and retained identity;
- ConfirmDelete copy and confirmation shortcuts;
- long practice names and overflow indication;
- empty-list behavior;
- database or validation failure retention; and
- resizing through normal, narrow, short, and tiny terminal dimensions.

The accepted result should look and behave the same as the pre-refactor Practices screen. The refactor is successful when the implementation is simpler and reusable without asking the user to relearn the interface.

## Later Batch 3B Slices

After Practices acceptance, the recommended migration order is:

1. Quotes
2. Trends
3. History

Goals popup extraction, Dashboard responsive-layout extraction, and Log Entry form extraction remain separate later slices because their interaction and geometry contracts are materially different. They are explicitly not shipped or authorized by this design.

Before each later slice, the team reviews the proven component API against that screen and extends it only for demonstrated requirements. Batch 3B-1 must not add speculative configuration solely for those future migrations.

## Acceptance Criteria

Batch 3B-1 is ready for manual acceptance when:

- all five pre-refactor Practices golden fixtures pass unchanged after migration;
- the three component suites cover their geometry, viewport, and rendering contracts;
- Practices contains no duplicate local implementation of the extracted mechanics;
- Practices business, lifecycle, shortcuts, status, and failure behavior remain unchanged;
- tiny rectangles cannot cause out-of-bounds rendering or panic;
- formatting, strict Clippy, build, full tests, TUI lint, and diff checks pass;
- documentation states that Practices alone has migrated in Batch 3B-1; and
- the tracked worktree is clean at the handoff point.
