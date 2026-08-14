# TUI Consistency Framework — Batch 3B-5 Goals Responsive Panels Design

**Date:** 2026-08-14
**Status:** Approved design; implementation not started

## Context

Batch 3B-1 through Batch 3B-4 migrated Practices, Quotes, Trends, and History to the shared Ratatui presentation components. Each slice was implemented, reviewed, release-gated, and manually accepted before the next slice began.

Batch 3B-5 applies the same incremental process to Goals, but unlike the preceding structural-only slices it includes an explicitly approved user-facing redesign. The current Goals screen renders all goals in one bordered list and places the selected goal's milestones in a floating preview or focused modal. The modal is reachable and scrollable, but it obscures part of the goal list and makes the milestone state feel temporary rather than continuously available.

The approved direction replaces that floating overlay with a persistent master/detail composition:

- Goals remain visible in one pane.
- The selected goal's progress and milestones remain visible in a second pane.
- Wide layouts use side-by-side panes.
- Narrow layouts use stacked panes.
- Focus moves between the two panes without opening or closing a popup.
- Goal progress remains derived from binary milestone completion, not a manually entered percentage.

The redesign continues the original Goals objective: every milestone must be present and reachable, and completing or reopening a milestone must update the goal progress immediately without losing context.

## Goals

- Replace the floating milestone popup with a persistent selected-goal panel.
- Use `ScreenFrame` for centered content, one-pass body/status/shortcut allocation, and active shortcut rendering.
- Use `ScrollableListPanel` for the Goals pane border, overflow, viewport, and multi-line selected-goal highlight.
- Use a backward-compatible pinned-header DetailPanel entry point for the always-visible Milestones pane, including focus styling, a fixed progress line, visual-row scrolling, overflow, and wrapped selection highlighting.
- Show Goals and Milestones side by side when the available frame is wide enough and stacked when it is not.
- Keep both panes visible while browsing and editing whenever a selected goal exists.
- Keep the active goal and milestone visible through stable identities, mutation-driven reorder, resize, and re-entry.
- Preserve every goal and milestone database rule, failure-retention guarantee, undo behavior, and navigation lifecycle contract.
- Make shortcuts accurately reflect the active pane and active edit or confirmation state.
- Keep tiny, zero-sized, narrow, short, and resized rendering bounded and panic-free.
- Add only two fixture-proven, backward-compatible framework extensions: a span-aware ListPanel entry point for multi-line selected items and a pinned-header DetailPanel entry point for fixed progress above scrollable milestones.

## Non-Goals

- No manually editable goal percentage and no fractional milestone progress.
- No database, schema, model, export/import, or persistence-format change.
- No navigation-topology, Router, ScreenStore, lifecycle-protocol, or global-key change.
- No Dashboard or Log Entry migration.
- No modification of Practices, Quotes, Trends, or History presentation behavior.
- No generic `MasterDetail` component.
- No generic popup component; the old Goals popup is removed rather than abstracted.
- No mouse interaction.
- No replacement or reintroduction of Quick Log, LLM configuration, or Abbreviations.
- No speculative shared configuration for Dashboard or Log Entry.

## Chosen Approach

Batch 3B-5 composes the approved screen from existing presentation components and two narrow, backward-compatible sibling entry points: span-aware ListPanel selection and a pinned DetailPanel header.

### Screen Composition

- `ScreenFrame` owns the centered outer area, body allocation, shared status row, and wrapped active shortcuts.
- Goals owns a pure responsive split helper that divides the returned body rectangle into the Goals and Milestones pane rectangles.
- `ScrollableListPanel` owns the Goals pane border, title, overflow, scrolling, and selected-item highlight.
- `render_detail_panel_with_header` owns the Milestones pane border, focus color, pinned progress header, overflow, visual-row scrolling, and selected visual-span highlight.
- Goals owns all domain content, progress formatting, input lines, focus state, active commands, and mutation behavior.

### Rejected Alternatives

- **Keep an enhanced floating popup:** this would continue to obscure the list and would not satisfy the approved persistent-context design.
- **Create a full-screen milestone page:** this provides space but removes the simultaneous goal context the user selected.
- **Create a generic `MasterDetail` component now:** Goals alone does not prove a stable abstraction suitable for Dashboard or Log Entry.
- **Compress every Goal into one line:** this avoids a framework extension but removes the approved per-goal gauge from the Goals pane.
- **Keep a Goals-local bordered list:** this preserves duplicated presentation ownership and defeats the consistency objective.
- **Change existing ListPanel or DetailPanel APIs for every screen:** the required capabilities can be added as sibling entry points without forcing mechanically risky changes onto accepted screens.

## Responsive Geometry

The frame continues to use the project `CONTENT_WIDTH` maximum. `ScreenFrame` receives one `Constraint::Min(0)` body section, plus the shared status and active shortcut rows.

Goals then derives the two pane rectangles from that safe body rectangle.

### Wide Layout

When the body width is at least 83 cells, the layout is horizontal:

- minimum Goals pane width: 34 cells;
- gap: 1 cell;
- minimum Milestones pane width: 48 cells;
- after subtracting the one-cell gap, the Goals width is `max(34, floor(remaining * 2 / 5))`, capped so Milestones retains at least 48 cells;
- Milestones receives every remaining cell;
- both rectangles are clipped to the body rectangle.

The width calculation is explicit rather than relying on two competing `Constraint::Min` values. At the 83-cell boundary, the panes receive exactly 34 and 48 cells. At the normal 107-cell content width, they receive 42 and 64 cells.

### Narrow Layout

When the body width is below 83 cells, the layout is vertical:

- a one-cell gap is subtracted when the body has at least two rows;
- Goals receives `floor(remaining * 2 / 5)` rows;
- Milestones receives every remaining row;
- a one-row gap is used only when it fits;
- Milestones receives the extra row when division is uneven;
- saturating arithmetic and clipping keep both rectangles inside the body.

At `80x24`, the accepted layout is stacked. Both panes remain independently scrollable. On extremely short rectangles, visible content may collapse, but neither pane may escape its parent or panic.

### Resize Contract

Changing between horizontal and vertical layout does not recreate screen state. Goal selection, milestone selection, pane focus, text input, status, undo payloads, and both viewport offsets remain intact. Each component remeasures its actual current width and height, then reveals and clamps the active selection.

## Pane Presentation

### Goals Pane

Each Goal retains:

- selected marker;
- complete or pending marker;
- title;
- optional completion date;
- gauge;
- `completed/total · percentage` for goals with milestones;
- `0%` or `100%` for goals without milestones.

A Goal is a multi-line visual item. The complete title/gauge span is highlighted when selected, not only its first line. Long titles wrap within the actual pane width, and selection reveal uses the resulting visual span.

Goal add, edit, completion-date edit, and delete confirmation remain inline in the Goals pane. Long inputs continue using `TextInput::visible_spans` so the cursor stays visible.

If there are no goals, the pane shows the existing add-goal guidance. The Milestones pane simultaneously shows a bounded empty-selection message.

### Milestones Pane

The pane title contains the selected Goal title plus `↑`, `↓`, or `↑↓` when milestone content exists outside the viewport.

The pinned header displays the selected Goal gauge and progress text. It remains visible while the milestone body scrolls. All milestone rows follow in the independently scrollable body. Each milestone retains:

- selected marker while the pane is focused;
- complete or pending marker;
- title;
- optional completion date.

Long milestone titles wrap. The selected milestone's complete visual span is highlighted and revealed. Add, title edit, date edit, and delete confirmation appear inline in this pane, with horizontally scrolling inputs where applicable.

When the selected Goal has no milestones, the pane still shows its gauge and a direct add-milestone hint. The pane remains focusable so `a` can create the first milestone.

### Focus Styling

- The focused pane uses the existing accent border treatment.
- The non-focused pane remains visible with the normal border color.
- Only the focused pane renders an active selected-row highlight.
- Status is rendered once below both panes.
- Shortcuts are rendered once and include only commands executable in the current focus and mode.

The redesign contains no `Clear` overlay and no locally positioned popup rectangle.

## Interaction Model

### Goals Focus

- `j/k` and arrow keys move through Goals using the current bounded navigation policy.
- Moving the selection immediately refreshes the Milestones pane from the selected in-memory Goal.
- `Enter` moves focus to the Milestones pane.
- `m` moves focus to the Milestones pane and immediately starts AddMilestone.
- `a`, `e`, `D`, `d`, `Space`, and `u` retain their current goal-level meanings and validity rules.
- `Esc` returns to Dashboard.

### Milestones Focus

- `j/k` and arrow keys move through milestones and reveal the selected wrapped span.
- `Space` immediately toggles the selected milestone and refreshes aggregate Goal progress.
- `a`, `e`, `D`, `d`, and `u` retain their current milestone-level meanings.
- In Milestones Browse, `Esc` returns focus to Goals without leaving the screen.
- In a milestone edit or confirmation state, `Esc` cancels that state and returns to Milestones Browse.
- A subsequent Goals-focus `Esc` returns to Dashboard.

Text-entry modes continue to use `Enter` to confirm and `Esc` to cancel. Delete confirmation retains its accepted confirmation semantics. No new global shortcuts are introduced.

## Progress and Ordering Contract

- Each milestone remains binary: complete or incomplete.
- Goal progress with milestones is always `completed milestones / total milestones`.
- Completing the last incomplete milestone completes the Goal.
- Reopening, adding, deleting, or restoring a milestone recalculates Goal completion immediately.
- A Goal without milestones retains manual completion through Goal-focus `Space`.
- Goal ordering remains incomplete first and completed second, preserving stored order inside each group.

After any operation that can reorder Goals, the active Goal is restored by stable Goal ID. When milestone focus remains valid, the active milestone is restored by stable milestone ID. If an identity disappeared, the prior index is clamped to a valid fallback.

## State Model

The old popup terminology is removed because the second pane is persistent.

- `Mode::Modal` becomes `Mode::Milestones`.
- `ModalMode` becomes `MilestoneMode`.
- `modal_goal_idx`, `modal_selected`, `modal_status_msg`, and `modal_last_deleted` become milestone-owned fields.
- Goal and milestone inputs, statuses, undo payloads, selections, and viewport state remain independently owned.
- `DetailPanelState::focused` reflects pane focus rather than popup modality.

This state clarification is local to Goals. It does not alter application-level navigation types or lifecycle traits.

## Span-Aware ListPanel Extension

The accepted Goals pane requires multi-line selected-item reveal and highlight. The current `render_scrollable_list_panel` API accepts a selected row index and assumes every item occupies one visual row.

Batch 3B-5 adds a backward-compatible sibling entry point in `framework/list_panel.rs` named `render_scrollable_list_panel_spanned`.

The new entry point:

- accepts flattened styled `Line` content and an optional selected `VisualSpan`;
- accepts whether the pane is focused, using the existing accent border only when focused;
- measures content using the actual safe inner width;
- reveals and clamps the viewport by visual rows;
- highlights every visible row belonging to the selected span only while focused, while still revealing the selected span when unfocused;
- owns the same border, padding, title, overflow, empty-state, and safe-area policy as the existing ListPanel;
- imports no Goal, Milestone, database, navigation, or screen type.

The existing `render_scrollable_list_panel` signature and behavior remain intact. Practices, Quotes, Trends, and History do not require production changes. Both public entry points share private geometry and rendering primitives so border and containment policy cannot diverge.

No external crate is added for this extension; it uses the existing `VisualSpan`, `ViewportState`, `visual_spans`, and Ratatui types.

If implementation shows that the new entry point cannot share the existing component policy without changing accepted output in a migrated screen, work pauses for a user scope ruling rather than modifying another screen or its fixture.

## Pinned-Header DetailPanel Extension

The approved Milestones pane keeps aggregate progress visible while milestone rows scroll. The current `render_detail_panel` treats every line as one scrollable body and therefore cannot satisfy that requirement.

Batch 3B-5 adds a backward-compatible sibling entry point in `framework/components.rs` named `render_detail_panel_with_header`.

The new entry point:

- accepts pinned header lines separately from scrollable body lines;
- reserves the measured header height inside the safe bordered inner rectangle;
- assigns the remaining height to the existing visual-row viewport;
- interprets the selected `VisualSpan` relative to the scrollable body only;
- keeps title overflow based on hidden body content;
- applies the same focused/unfocused border and selected-span styling as `render_detail_panel`;
- handles a header or body that receives zero available rows without escaping or panicking;
- imports no Goal, Milestone, database, navigation, or screen type.

The existing `render_detail_panel` signature and behavior remain intact. Quotes and History require no production changes. Both public entry points share private border, title, viewport, overflow, and selected-span rendering primitives.

No external crate is added for this extension. If the pinned header would change an accepted Quotes or History fixture, implementation pauses for a scope ruling rather than modifying that fixture.

## Render Flow

Each render follows this sequence:

1. Goals derives active shortcuts from the current pane focus and edit/confirmation mode.
2. Goals selects the active Goal or milestone status message.
3. Goals calls `ScreenFrame` once to reserve body, status, and wrapped shortcut rows.
4. A pure Goals geometry helper splits the safe body rectangle horizontally or vertically.
5. Goals builds flattened Goal lines and the selected Goal `VisualSpan` using the actual Goals-pane inner width.
6. Goals calls the span-aware ListPanel once for the Goals pane.
7. Goals builds the selected Goal gauge as pinned header content, then builds milestone lines, inline mutation content, and the selected milestone span using the actual Milestones-pane inner width.
8. Goals calls `render_detail_panel_with_header` once for the Milestones pane.
9. The components independently reveal and clamp their viewports using current measured geometry.

After migration, Goals contains no local outer status/footer layout, local list block, local list overflow title, local selected-row painting, popup placement, popup height calculation, popup `Clear`, or custom footer `Paragraph`.

## Mutation and Failure Retention

Existing persistence semantics remain binding:

- Database mutation happens before any success-only state transition.
- Failed Goal create, edit, completion-date change, toggle, delete, or restore retains the current Goal focus, input or confirmation mode, selection, viewport, and retryable undo payload.
- Failed milestone create, edit, completion-date change, toggle, delete, or restore retains Milestones focus, input or confirmation mode, selected Goal/milestone identity, both viewports, and retryable undo payload.
- A failed mutation never displays an in-memory success state that was not persisted.
- A successful mutation refreshes current data and restores stable identities before adjusting focus or selection.
- The relevant error is shown on the one shared status row without replacing pane content.

If a database write succeeds but its immediate refresh fails, the implementation must preserve the last usable screen state and report the refresh failure according to the current Goals contract. This presentation slice does not broaden transaction semantics or change database APIs.

## Lifecycle and Navigation

Batch 3A behavior remains unchanged:

- Incoming Goals navigation prepares fresh goals and milestones before route commit.
- Selected Goal is restored by stable ID when possible.
- Milestones focus, its Goal, and selected milestone are restored by stable IDs when possible.
- Missing identities use bounded index fallbacks.
- Both viewport offsets are retained and then bounded by actual-width rendering.
- Cross-page re-entry clears unfinished goal/milestone mutations, stale statuses, shared input, and both undo payloads.
- A preparation failure leaves the source route and cached Goals target unchanged and reports the error on the source screen.

No Router, ScreenStore, navigation intent, Dashboard route, or key topology changes are authorized.

## Geometry and Safety

All rectangles derive from the current frame, `ScreenFrame`, the pure responsive split helper, component-owned inner geometry, saturating arithmetic, and `fit_rect`-equivalent clipping.

The implementation must not:

- call `Clear` from Goals rendering;
- render either pane outside the `ScreenFrame` body;
- reconstruct a component inner rectangle with unchecked `Block::inner`;
- underflow when the body cannot fit the preferred gap or proportions;
- assume `80x24` is always available;
- let wrapped Goal or milestone content escape its pane;
- preserve an out-of-range viewport or selection after mutation, re-entry, or resize;
- panic on zero-width, zero-height, one-cell, short, or narrow rectangles.

When size later expands, the current stable selection must remain active and be revealed again.

## Test Strategy

### Behavior Characterization Before Redesign

Before replacing the popup, add focused state-transition coverage for every existing Goal and milestone mode, shortcut, successful mutation, failure-retention path, undo path, stable-ID reorder, re-entry reset, and layered `Esc` behavior that must survive the redesign.

The old floating-buffer output is not an immutable compatibility fixture because the user explicitly approved a new layout. Existing behavior and domain tests remain binding.

### RED/GREEN Presentation Tests

Add failing tests before implementation for:

- horizontal Goals/Milestones rectangles at the 83-cell boundary and normal 107-cell width;
- stacked rectangles at `80x24` and immediately below the threshold;
- containment at `40x16`, tiny edge rectangles, one-cell rectangles, and zero-sized rectangles;
- both panes visible in Goal focus and Milestones focus;
- focused and unfocused border styling;
- independent Goal and milestone viewport reveal/clamp;
- multi-line selected Goal highlight through the span-aware ListPanel;
- wrapped selected milestone highlight and overflow suffix;
- no popup `Clear` or obscured Goal-list cells;
- active-mode shortcut completeness and absence of inactive commands;
- resize between horizontal and vertical layouts without state loss;
- four milestones visible when the measured pane permits and larger collections reachable through scrolling;
- immediate `Space` completion feedback and aggregate progress update;
- all Goal and milestone inline edit and confirmation modes;
- mutation-failure retention for both panes;
- stable-ID selection after Goal reorder, milestone deletion, refresh, and re-entry.

### New Accepted Fixtures

Once the semantic RED/GREEN tests match the approved design, capture new immutable styled-buffer fixtures for the redesigned Goals screen. The fixture set must cover at least:

- wide Goal focus;
- wide Milestones focus;
- narrow stacked Goal focus;
- narrow stacked Milestones focus;
- Goal add/edit or confirmation presentation;
- milestone add/edit/date or confirmation presentation;
- list and milestone overflow;
- empty Goals and empty Milestones states.

Fixture capture is a one-time explicit characterization step. The update path is removed before the fixture commit. Later implementation and review commands compare the committed fixture range to `HEAD`; fixtures may not be regenerated to hide a regression.

### Shared-Component Regression

- Prove the old single-row ListPanel API remains cell-identical for Practices, Quotes, Trends, and History fixtures.
- Prove the new span-aware API reveals and highlights wrapped multi-line selections, handles empty content, and remains bounded in tiny rectangles.
- Prove the span-aware Goals pane retains selection visibility without active-row highlighting when focus moves to Milestones.
- Prove the existing DetailPanel API remains cell-identical for Quotes and History fixtures.
- Prove the pinned-header API keeps the header fixed while the body scrolls, measures body overflow independently, preserves selected-span highlighting, and remains bounded in tiny rectangles.
- Keep all existing Paragraph, direct-border, delegated-ownership, and removed-feature lint behavior intact.

### Structural Ownership Lint

The production `GoalsScreen::render` path must be checked structurally, not by comments or string matching. It must require exactly:

- one `ScreenFrame` delegation;
- one span-aware Goals ListPanel delegation;
- one pinned-header Milestones DetailPanel delegation.

It must reject local Goals list borders, local status/shortcut rendering, popup `Clear`, and dead/test/helper decoys used as ownership evidence. Existing ownership requirements for every other screen remain unchanged.

### Release Gates

- `cargo fmt -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- focused Goals, framework, lifecycle, app, i18n, TUI lint, and removed-feature suites
- all Practices, Quotes, Trends, and History fixtures unchanged
- new Goals fixtures unchanged after their characterization commit
- `cargo test --all-targets --all-features`
- `cargo build --all-features`
- Batch-range `git diff --check`
- exact scope and duplicate-presentation audits
- clean tracked worktree at handoff

## Documentation

After implementation, active documentation will state that Batch 3B-5 Goals is the first approved presentation redesign in the component rollout, while Practices, Quotes, Trends, and History remain behavior-preserving migrations.

Documentation will describe:

- persistent Goals/Milestones panes;
- responsive side-by-side and stacked layouts;
- focus and layered `Esc` behavior;
- automatic milestone-derived progress;
- span-aware ListPanel ownership;
- unchanged persistence, navigation, re-entry, and failure semantics;
- Dashboard and Log Entry as later unshipped presentation slices.

Historical design records are not rewritten as if this layout shipped earlier.

## Implementation Sequencing

The later implementation plan must keep independently reviewable boundaries:

1. Characterize binding Goal and milestone state transitions and capture explicit pre-redesign failure behavior.
2. Add the backward-compatible span-aware ListPanel API with focused RED/GREEN geometry, wrapping, highlight, and compatibility tests.
3. Add the backward-compatible pinned-header DetailPanel API with focused RED/GREEN header/body/overflow and compatibility tests.
4. Add responsive Goals pane geometry and interaction RED tests.
5. Replace the floating popup with persistent panes, migrate state terminology, and preserve mutation/lifecycle behavior.
6. Capture and freeze the new approved Goals styled-buffer fixtures, remove the update path, and add production ownership lint.
7. Update shipped-state documentation and run complete release gates.
8. Perform a whole-slice review before manual handoff.

Any shared-component incompatibility, accepted-screen fixture change, persistence-semantic conflict, or plan-versus-code mismatch pauses implementation for user approval.

## Manual Acceptance

Batch 3B-5 is handed off as a Goals-only binary. Manual acceptance covers:

- persistent selected-goal progress and all milestone rows;
- side-by-side layout at wide widths and stacked layout at `80x24`;
- Goal and Milestones focus styling and switching;
- Goal browsing with immediate Milestones-pane synchronization;
- milestone scrolling, wrapped selection, overflow markers, and large milestone collections;
- direct `Space` completion updates and correct aggregate progress;
- Goal and milestone add, edit, date, delete confirmation, and undo;
- stable selection after Goal reorder and mutation refresh;
- long Unicode input and cursor visibility;
- mutation-failure retry/cancel behavior;
- cross-page re-entry and atomic navigation failures;
- normal, narrow, short, tiny, zero-sized, and resized terminals;
- accurate active-mode status and wrapped shortcuts; and
- unchanged Dashboard return and all other screen behavior.

No Dashboard or Log Entry presentation work begins until the user explicitly accepts this Goals slice.

## Acceptance Criteria

Batch 3B-5 is ready for manual acceptance when:

- Goals uses one `ScreenFrame`, one span-aware ListPanel, and one pinned-header DetailPanel as the only owners of the delegated presentation mechanics;
- the floating popup and its `Clear`/placement logic are removed;
- wide and narrow layouts follow the approved responsive geometry and remain bounded at all tested sizes;
- all Goals and milestones remain visible or reachable without silently clipped data;
- Goal progress remains milestone-derived and updates immediately after a successful mutation;
- Goal and Milestone focus, edits, confirmations, undo, failure retention, stable identities, viewports, and layered `Esc` behavior match the approved contract;
- the old ListPanel API and every accepted Practices, Quotes, Trends, and History fixture remain unchanged;
- new Goals styled-buffer fixtures freeze the redesigned accepted states;
- structural lint enforces production ownership and rejects popup/local-border regressions;
- all release gates pass with zero strict Clippy warnings;
- documentation accurately marks Goals as shipped and Dashboard/Log Entry as unshipped; and
- the user manually accepts the Goals binary before the next slice begins.
