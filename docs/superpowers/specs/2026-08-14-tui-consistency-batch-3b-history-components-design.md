# TUI Consistency Framework — Batch 3B-4 History Components Design

**Date:** 2026-08-14
**Status:** Approved design; implementation not started

## Context

Batch 3B has migrated Practices, Quotes, and Trends to the shared presentation components. Each screen was delivered as an isolated vertical slice, verified against immutable styled-buffer fixtures, and manually accepted before the next screen began.

Batch 3B-4 applies the same incremental process to History. History currently owns a filter row, a bordered and scrollable `Table`, a floating Browse preview, a focused Detail viewport, status and shortcut rows, deletion confirmation, one-level undo, edit navigation, and lifecycle re-entry. The migration replaces only duplicated presentation mechanics with the shipped `ScreenFrame` and `ScrollableListPanel` components.

This is a structural refactor. History's accepted output, data behavior, interaction, failure retention, and lifecycle behavior remain binding. Goals, Dashboard, Log Entry, and later presentation slices do not begin during this batch.

## Goals

- Freeze the current History output across its important visible states with exact styled-buffer fixtures.
- Reuse `ScreenFrame` for centered content, body allocation, status, and shortcuts.
- Reuse `ScrollableListPanel` for the list border, title, overflow suffix, viewport, visible-row slicing, empty state, and selected-row highlight.
- Translate the current `Table` header and rows into owned `Line` values that reproduce the accepted cells exactly.
- Preserve History-owned filtering, dynamic columns, domain formatting, floating preview, focused Detail mode, edit navigation, delete/undo behavior, database operations, and lifecycle re-entry.
- Continue using the existing `render_detail_panel` helper without introducing a generic popup component.
- Keep normal, narrow, short, tiny, zero-sized, and resized rendering bounded and panic-free.
- Stop for an explicit scope decision if the existing shared component API cannot reproduce the accepted output.

## Non-Goals

- No visual redesign, copy change, color change, spacing change, shortcut change, or navigation change.
- No migration of Goals, Dashboard, Log Entry, or another screen.
- No database, schema, model, localization, router, navigation-intent, lifecycle-protocol, or edit-flow change.
- No extraction of History filtering, log formatting, popup placement, detail content, delete/undo policy, or modes into shared components.
- No generic table, column, popup, list-detail, or History-specific framework API.
- No use of `ActionPanel`; History has no persistent add/edit form on this screen.
- No parallel old and new History presentation paths after migration.
- No fixture regeneration to hide a migration difference.
- No reintroduction or replacement of Quick Log, LLM configuration, or Abbreviations.

## Chosen Approach

Batch 3B-4 uses the existing `ScreenFrame` and `ScrollableListPanel` APIs without a speculative shared-component extension.

- `ScreenFrame` owns the outer centered layout and reserves the filter, list, status, and active shortcut rows in one pass.
- `ScrollableListPanel` owns the bordered list presentation and logical-row viewport.
- History converts its current four-column `Table` presentation into exactly equivalent header and data `Line` values using the actual list-panel inner width.
- History continues to own the floating Browse preview and focused Detail popup, positioned from the safe list geometry returned by `ScrollableListPanel`.

History calls `render_screen_frame_sections(...)` with these body constraints, in order:

```rust
[
    Constraint::Length(1),
    Constraint::Min(1),
]
```

Trailing flex remains disabled. This reproduces the current `filter | list | status | shortcuts` topology without local footer measurement or rendering.

Rejected alternatives:

- **Add Table support to `ScrollableListPanel`:** History alone does not justify a second row-rendering abstraction, and the existing component contract already accepts styled `Line` values.
- **Keep the local `Table` inside the shared list border:** this would leave visible-row slicing and highlight ownership split between History and the component.
- **Create a shared floating-detail component:** Quotes and History have similar-looking overlays but do not yet prove one stable placement, sizing, focus, or content policy.
- **Adopt `ActionPanel`:** History's confirmation is an interaction mode and status/shortcut change, not a bounded form panel.
- **Change fixtures to accommodate component output:** the migration is required to preserve the accepted UI, not redefine it.

If exact fixture parity cannot be achieved through equivalent `Line` formatting and the existing component contracts, implementation pauses for user approval rather than changing fixtures or automatically extending the framework.

## Compatibility Contract

Batch 3B-4 is not a user-facing behavior change.

At an `80x24` terminal, the complete Ratatui buffer must match the pre-refactor fixture for these states:

1. **Browse** — multiple logs, selected row, four-column header, dynamic metrics, and floating preview.
2. **BrowseWithUndo** — the optional `[u] Undo` shortcut and its resulting footer layout.
3. **Detail** — focused full-height detail panel with wrapped content and a nonzero visual-row viewport.
4. **FilteringLongInput** — active long filter, horizontally bounded input, visible cursor, and filtered rows.
5. **NoFilterMatch** — existing empty-list copy, no selection, and no stale detail popup.
6. **ListOverflow** — enough logs to scroll, the selected row revealed, and the correct overflow suffix.
7. **ConfirmDelete** — unchanged list/preview presentation with confirmation-specific active shortcuts and status behavior.

Each fixture compares every cell's symbol, foreground color, background color, modifiers, skip flag, and coordinates. Fixtures are captured from the pre-refactor implementation, committed before migration, and unavailable to the normal test path for regeneration.

The generated `Line` header and rows must be cell-identical to the current Ratatui `Table`, including marker width, column widths, inter-column gaps, padding, alignment, RPE styling, and selected-row styling.

Existing behavior tests remain binding. If characterization exposes an existing defect, the implementation records a focused RED and requests a scope ruling instead of silently encoding a changed behavior into the fixtures.

For impossible tiny geometry, safety takes precedence over off-screen parity: clipped content may be omitted, but every derived rectangle must remain within its parent and rendering must not panic.

## Ownership Boundary

### History Owns

- `LogEntry` values, filtered indices, stable log identities, and selection fallback policy.
- Filter text, filter-input mode, filtering behavior, prompt, cursor, and input presentation.
- Dynamic marker, date, practice/RPE, and metric column calculations.
- Date, practice, RPE, total, unit, set, summary, warm-up, cool-down, and note formatting.
- `Mode`: Browse, Detail, and ConfirmDelete.
- Floating-preview presence, size, placement, below/above flip policy, and focused Detail expansion.
- Detail title, content lines, visual-row measurement, focus, viewport, and overflow behavior.
- Delete confirmation, one-item undo payload, database mutations, refreshes, and retry behavior.
- Key handling, active shortcuts, edit navigation intent, status creation, and lifecycle re-entry.

### Framework Components Own

- Centering and clipping the active History screen area.
- One-pass allocation of filter, list, status, and shortcut rectangles.
- Measuring wrapped shortcut height and rendering status and shortcuts.
- List border, padding, title, overflow suffix, fixed header, logical-row viewport, selected-row reveal/clamp, visible-row slicing, empty state, and selected-row highlight.
- Returning safely clipped list outer and inner rectangles.

The framework does not import `LogEntry`, History modes, database state, filter state, navigation intents, undo state, or detail state.

## Render Flow

Each History render follows this sequence:

1. History derives the unchanged active shortcut list from its current mode and undo availability.
2. History calls `render_screen_frame_sections(...)` with the filter and list constraints. `ScreenFrame` clips and centers the target, measures shortcut wrapping, renders status and shortcuts, and returns both body rectangles.
3. History renders its existing filter line and cursor presentation in the returned filter rectangle.
4. History computes the existing dynamic column widths from the filtered logs.
5. History calls `render_scrollable_list_panel(...)` with the returned list rectangle and its existing `ViewportState`.
6. Inside the list content closure, History uses the safe inner width to build one owned header `Line` and one owned data `Line` per filtered log. These lines preserve the old `Table` marker, date, practice/RPE, metric alignment, gaps, and styles exactly.
7. `ScrollableListPanel` renders the border/title/overflow, clamps and reveals the selected logical row, renders the fixed header plus visible rows or the existing empty copy, applies the selected-row highlight, and returns safe list geometry.
8. When a selected log exists, History derives the Browse preview or focused Detail popup only from the returned safe list outer rectangle, clips it, clears it, and calls `render_detail_panel`.

After migration, History contains no local list block, overflow-title construction, visible-row slicing, viewport reveal/clamp, selected-row highlight, status renderer, shortcut renderer, shortcut-height calculation, or four-row outer layout.

## Table-to-Line Equivalence

The `Table` removal is a representation change only.

- The marker column retains its current selected `>` and unselected blank presentation.
- The date column retains the `YYYY-MM-DD` value and current dim/selected styles.
- The practice column retains the practice name, optional yellow ` [RPE: n]` suffix, dynamic width, and truncation behavior produced by the accepted buffer.
- The metric column retains right-aligned totals, unit labels, label padding, dynamic width, and selected/dim styles.
- The header retains the existing localized Date, Practice, and Volume labels and bold white style.
- The current logical ordering and one-row-per-log viewport semantics do not change.

Formatting uses display width rather than byte length wherever the existing UI does, so Unicode practice names remain aligned and bounded. The implementation must prove equivalence through fixtures rather than relying only on string snapshots.

## Detail Popup Contract

The detail overlay remains History-owned and continues to use `render_detail_panel`.

- The popup's only placement parent is the safe list outer rectangle returned by `ScrollableListPanel`.
- Horizontal placement preserves the current four-cell inset on each side where space permits.
- Browse height remains content-driven, with the existing minimum and half-list maximum.
- Detail mode remains full list height.
- Browse placement starts below the selected visible row and flips above when the lower space is insufficient.
- Every candidate rectangle is clipped with `fit_rect` before `Clear` or detail rendering.
- The title retains practice name, log date, and optional RPE.
- Content retains the existing set lines, summary, warm-up, cool-down, and note formatting.
- Browse keeps the detail viewport at the top; Detail preserves and scrolls its visual-row viewport.

`visual_spans` uses the actual clipped popup inner width. History records the resulting visual content height and clamps/reveals only after width-aware measurement during rendering. It must not guess wrapped height from logical line count during refresh or re-entry.

A small private History geometry helper may isolate and test popup positioning. It remains presentation-only, accepts already safe rectangles and measurements, returns a clipped rectangle, and is not promoted to the shared framework in this slice.

## Interaction Contract

The migration does not change interaction:

- Browse `j/k` and arrow keys move with the current bounded, non-wrapping selection policy.
- `Enter` focuses Detail only when a selected log exists.
- Detail `j/k` and arrow keys scroll by visual row; `Esc` returns to Browse.
- `/` enters live practice-name filtering and clears the previous filter according to current behavior.
- Filter editing uses the existing bounded `TextInput`; Enter or Esc leaves filter-input mode.
- `e` emits an edit intent containing the selected log's stable database ID.
- `d` enters ConfirmDelete only when a selected log exists.
- ConfirmDelete `y` attempts deletion; every other key cancels according to the current handler.
- `u` restores the one retained deleted log when available.
- Browse `Esc` first clears a nonempty filter and only then returns to Dashboard.
- The shortcut footer continues to advertise only the commands active in the current mode.

## Failure and Mutation Retention

Existing failure semantics remain binding:

- Delete failure retains ConfirmDelete, selection, list context, and a visible error so the operation can be retried or cancelled.
- Restore failure puts the removed undo payload back into `last_deleted` and shows an error, so `u` can be retried.
- A successful delete or restore followed by refresh failure keeps the last usable in-memory History state and reports the reload failure according to the existing contract; this structural slice does not redesign post-write reconciliation.
- Filtering, browsing, and Detail state are not replaced by partially loaded data after an in-place database failure.
- A cross-screen preparation failure remains atomic: History stays active and intact when it is the source, or the current source remains active and the cached History screen remains unchanged when History is the target.
- Navigation errors remain visible through the existing application-shell status mechanism.

No Router, ScreenStore, Log Entry, or database changes are authorized by this design.

## Lifecycle and Re-entry Contract

History continues to own its existing `Reenter` behavior:

- Reload current logs before route commit.
- Preserve filter text and reapply it to the refreshed values.
- Restore the selected log by stable ID when possible.
- If that ID no longer exists, fall back to a valid bounded index, or zero when no filtered logs remain.
- Preserve the list and detail viewport offsets across reload, then let actual-width rendering clamp them safely.
- Keep Detail focus only when a selected log remains.
- Clear filter-input mode, ConfirmDelete, old status, and the cross-page undo payload.
- Keep in-place delete/undo refresh behavior distinct from cross-page re-entry so operation-local context remains available.

The migration does not alter the lifecycle trait, preparation/installation order, or route commit boundary.

## Geometry and Safety

All changed geometry uses rectangles returned by `ScreenFrame` and `ScrollableListPanel`, saturating arithmetic, and `fit_rect`-equivalent clipping.

The implementation must not:

- call raw `Block::inner` for the migrated list;
- render a list child outside the component-owned rectangle;
- calculate popup position from an unclipped or locally reconstructed list rectangle;
- call `Clear` or `render_detail_panel` with a rectangle outside the safe list parent;
- underflow when borders, padding, header, status, shortcuts, or popup margins consume the available size;
- assume a minimum terminal width or height;
- let Unicode column calculations or filter cursor rendering exceed their rectangles;
- panic when any filter, list, status, shortcut, or popup rectangle is zero-sized or one cell.

On later expansion or resize, the selected logical row must again be revealed and Detail scrolling must use the newly measured visual height.

## Test Strategy

### Characterization Before Migration

- Add the seven immutable `80x24` styled-buffer fixtures.
- Prove the golden test is initially RED with empty fixture content.
- Capture fixtures only from the old History renderer.
- Remove the fixture-update path before committing characterization.
- Verify each fixture contains the expected 80×24 coordinates and complete style metadata.

### Focused Framework and History Tests

- Prove `ScreenFrame` returns the same filter, list, status, and shortcut rectangles as the current History layout.
- Prove all seven History fixtures remain identical after migration in both library and binary targets.
- Verify Table-to-Line output preserves header, column gaps, Unicode alignment, RPE style, metric alignment, dim styles, and selected-row highlight.
- Preserve and extend tests for long filter cursor visibility, no-match filtering, last-row reveal, overflow suffixes, wrapped shortcuts, stable-ID edit navigation, and Detail visual-row scrolling.
- Verify popup placement below and above the selected row, full-height Detail, and containment in normal, edge, narrow, short, tiny, zero-sized, and resized targets.
- Verify delete failure retains ConfirmDelete and context.
- Verify restore failure retains the undo payload and context.
- Verify re-entry preserves stable selection/filter/viewports/Detail where valid and clears transient modes/status/undo.
- Keep Practices, Quotes, and Trends golden fixtures unchanged.

### Structural Lint

The recursive TUI ownership lint will move History from the direct-border group to the delegated group. It must require executable use of both `ScreenFrame` and `ScrollableListPanel`, rejecting comments, ordinary strings, raw strings, and character literals as evidence. Goals and Log Entry retain their existing direct-border requirement.

The recursive Paragraph wrapping lint keeps its current scan, four-line window, and semantic allowlist. A same-widget source-line move may receive a separately reviewed mechanical allowlist refresh; the lint's behavior does not change in this slice.

### Release Gates

- `cargo fmt -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- focused framework, History, Practices golden, Quotes golden, Trends golden, lifecycle, app, i18n, TUI lint, and removed-feature tests
- `cargo test --all-targets --all-features`
- `cargo build --all-features`
- all fixture immutability checks
- Batch-range `git diff --check`
- exact scope and duplicate-presentation audits
- clean tracked worktree at handoff

## Documentation

After implementation, active documentation will state that Practices, Quotes, Trends, and History are shipped shared-component consumers. Goals, Dashboard, Log Entry, and any later presentation migrations remain explicitly unshipped.

Documentation changes are limited to the English and Chinese READMEs, History specification, navigation specification, framework design, and contributor guidance needed to record the component boundary and rollout state. Historical design records are not rewritten as if History shipped earlier.

## Implementation Sequencing

The later implementation plan must keep independently reviewable boundaries:

1. Capture immutable History fixtures from the old renderer.
2. Add only fixture-proven History frame, row-equivalence, or popup-geometry coverage; do not speculate beyond History.
3. Migrate History to `ScreenFrame` and `ScrollableListPanel`, convert Table rows to equivalent Lines, extend structural ownership lint, and preserve all fixtures.
4. Update shipped-state documentation and run the complete release gates.
5. Perform a whole-slice code and specification review before manual handoff.

Any correctness fix found during characterization or review is isolated with focused RED/GREEN evidence. A fixture, Table-equivalence, popup-geometry, or shared-component conflict pauses implementation for user approval.

## Manual Acceptance

Batch 3B-4 is handed off as a History-only binary. Manual acceptance covers:

- log navigation, bounded selection, scrolling, overflow suffixes, column alignment, RPE styling, metric labels, and selected-row highlight;
- live filtering, long Unicode input, cursor visibility, no-match behavior, leaving filter mode, and layered Esc behavior;
- floating Browse preview, below/above placement, focused full-height Detail, wrapped notes, visual-row scrolling, and overflow markers;
- stable-ID edit navigation, save/cancel return behavior, and refreshed History data;
- delete confirmation, failure retry/cancel, successful deletion, one-level undo, and restore failure retry;
- re-entry after logs or practices change;
- normal, narrow, short, tiny, zero-sized, and resized terminals;
- mode-specific status and shortcut wrapping; and
- Dashboard return and atomic navigation-failure behavior.

No later module begins until the user explicitly accepts this History slice.

## Acceptance Criteria

Batch 3B-4 is ready for manual acceptance when:

- all seven pre-refactor History fixtures pass unchanged after migration in library and binary targets;
- Practices, Quotes, and Trends fixtures remain unchanged;
- History uses `ScreenFrame` and `ScrollableListPanel` as the only owners of delegated frame/list presentation mechanics;
- the generated Lines are cell-identical to the removed Table across the characterized states;
- floating and focused detail behavior remains History-owned, width-aware, clipped, and fixture-identical;
- existing filtering, navigation, editing, delete/undo, failure retention, re-entry, shortcuts, and database behavior remain green;
- tiny and undersized geometry stays inside its parents without panic;
- structural lint rejects a return to local History list-border ownership;
- all release gates pass with zero strict Clippy warnings;
- documentation accurately distinguishes shipped History work from later unshipped slices; and
- the user manually accepts History before the next screen begins.
