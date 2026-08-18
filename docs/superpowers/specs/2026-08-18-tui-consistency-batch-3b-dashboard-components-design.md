# TUI Consistency Framework — Batch 3B-6 Dashboard Components Design

**Date:** 2026-08-18  
**Status:** Approved for implementation planning  
**Scope:** Dashboard presentation composition only

## Context

Batch 3A centralized typed navigation and transactional screen lifecycle. Batch 3B then migrated Practices, Quotes, Trends, History, and Goals to shared presentation components. Dashboard already has responsive bounds, mode-accurate shortcuts, shared text input, and failure-retention behavior, but it still owns repeated outer-frame and bordered-panel mechanics.

Batch 3B-6 completes the Dashboard presentation slice without redesigning it. The accepted `80x35` output, navigation topology, shortcuts, data queries, HRV workflow, and lifecycle behavior remain unchanged. Log Entry form extraction remains a later, separately approved slice.

## Goals

- Move Dashboard outer-frame composition to the shipped `ScreenFrameSections` API.
- Introduce one small, presentation-only static-panel sibling for repeated title, border, padding, and safe-inner geometry.
- Preserve the accepted Dashboard layout and styled output at `80x35`.
- Preserve all Dashboard domain formatting, database behavior, modes, shortcuts, and failure semantics.
- Keep every previously accepted screen and fixture unchanged.
- Keep the new component independent of Dashboard domain models and application state.

## Non-Goals

- No visual redesign, new Dashboard section, new breakpoint, or content reordering.
- No changes to navigation keys, module topology, HRV rules, database queries, re-entry, or target-preparation semantics.
- No scrolling, selection, or viewport state for Recent or Goals.
- No migration of Log Entry.
- No production changes to Practices, Quotes, Trends, History, or Goals.
- No breaking change to an existing framework API.
- No new dependency and no general-purpose layout DSL.

## Accepted Behavior Baseline

At `80x35`, three styled-buffer fixtures define the exact accepted output:

1. **Normal** — populated Dashboard with a long quote, long practice name, long metric suffix, long active-goal title, and progress gauge.
2. **HRV input** — the summary contains the active editor, a long value where useful for cursor visibility, reserved status, and only the mode's confirm/cancel shortcuts.
3. **Quit confirmation** — the confirmation message occupies the status row and only the confirmation/cancel shortcuts are present.

The fixtures are captured from the pre-migration implementation and become immutable after their characterization commit. Migration must preserve every symbol and style cell in these fixtures.

Smaller targets, including `80x24`, `40x16`, zero-sized rectangles, one-cell rectangles, and clipped edge rectangles, are semantic and geometry tests rather than golden layouts. They must not panic or render outside the requested frame. Where the target has enough rows, the reserved status and complete current-mode shortcut units remain visible; very small targets degrade by clipping safely rather than by inventing a separate layout.

## Architecture

### Screen Frame Ownership

Dashboard computes its current shortcuts and status before laying out content, then calls `render_screen_frame_sections` once. The body constraints preserve the existing order and preferred sizes:

1. logo: `Length(1)`
2. spacer: `Length(1)`
3. heatmap: `Length(13)`
4. spacer: `Length(1)`
5. summary: `Length(7)`
6. spacer: `Length(1)`
7. Recent/Goals region: `Min(1)`

The frame appends one reserved status row and the measured number of wrapped shortcut rows. Trailing flex remains disabled. The Recent/Goals body region keeps its current horizontal `50% / 50%` split and gains no new breakpoint.

### Static Panel Sibling

The framework gains one narrow sibling API:

```rust
pub fn render_static_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> StaticPanelAreas
```

It receives:

- the Ratatui frame;
- a requested rectangle;
- a title;
- a one-shot content-rendering closure that receives the frame and safe inner rectangle.

`StaticPanelAreas` contains exactly the clipped outer `area` and padded `inner` rectangle. The function returns these rectangles after rendering so integration and geometry tests can verify the same bounds supplied to the closure.

The component owns only:

- clamping the requested area to the frame;
- the existing decorated title form;
- `Borders::ALL`;
- uniform padding;
- the existing `BORDER_COLOR` panel border/title style; and
- calculating a safe inner rectangle and invoking the closure with it.

It has no database dependency, Dashboard type, mode, input, selection, viewport, shortcut, or status knowledge. It is not a list component and does not synthesize overflow behavior. Zero, tiny, and clipped rectangles are safe. Existing framework APIs keep their current signatures and semantics.

### Dashboard Ownership

Dashboard continues to own:

- the logo line;
- Heatmap construction and data;
- weekly volume, streak, HRV, frequency, and quote formatting;
- HRV editor placement and cursor visibility;
- Recent grouping, date headers, name/metric width budgeting, and truncation;
- active-goal filtering, goal title truncation, milestone progress calculation, gauge rendering, and percentage suffixes;
- current mode, active shortcut selection, status selection, and key handling; and
- all database, lifecycle, navigation, and failure behavior.

The migration removes Dashboard's private safe-inner helper and repeated bordered `Block` construction. It does not move domain formatting into the framework.

## Rendering and Data Flow

For every render:

1. Dashboard derives the current-mode shortcuts.
2. Dashboard derives the visible status: quit confirmation in `ConfirmQuit`, otherwise its stored status.
3. `ScreenFrameSections` centers the accepted content width, allocates the seven body sections, and reserves status and shortcut rows.
4. Dashboard renders the logo into the first body section.
5. Four static-panel calls render Heatmap, Summary, Recent, and Goals using their returned safe inner rectangles.
6. Dashboard's existing renderers format and clip content within those inner rectangles.

No state is created by the frame or static panel. Re-rendering after resize recalculates all rectangles from the current target.

## Modes, Errors, and Lifecycle

The mode contract is unchanged:

- Normal: `l`, `h`, `t`, `e`, `g`, `Q`, `v`, `Esc`
- HRV input: `Enter`, `Esc`
- Quit confirmation: `y`, any other key to cancel

The static panel is infallible presentation code. It cannot change domain state or replace an error. Existing rules remain authoritative:

- invalid HRV remains in HRV input with its text and validation error;
- a failed HRV write preserves the input, prior displayed/database value, and error status;
- successful HRV save updates the displayed value, clears input, and returns to Normal;
- ordinary Dashboard re-entry reloads all data and clears transient state;
- failed incoming preparation leaves the source route and cached Dashboard unchanged; and
- failed outgoing preparation leaves Dashboard active and displays the localized error without resetting it.

## Testing Strategy

### Characterization

- Add the three pre-migration `80x35` styled-buffer fixtures.
- Remove the temporary fixture-update path before committing the fixtures.
- Treat those Dashboard fixtures as immutable from their characterization commit onward.

### Framework Tests

- exact title, border, padding, and accent cells;
- returned outer and inner rectangles remain inside the frame and requested target;
- the content closure receives exactly the returned safe inner rectangle;
- zero-sized, one-cell, clipped, and maximum-coordinate rectangles do not panic; and
- the component contains no Dashboard/domain dependency.

### Dashboard Tests

- all three golden states match in both library and binary targets;
- existing navigation, shortcut, HRV, re-entry, mutation-failure, and navigation-failure tests continue to pass;
- long quote, practice, metric, goal, percentage, and localized text remain bounded;
- `80x24`, `40x16`, zero-sized, one-cell, and edge rectangles remain geometry-safe; and
- resizing recomputes bounded panel geometry without stale layout state.

### Ownership and Regression Guard

A lightweight lint verifies that Dashboard production rendering delegates once to the shared frame and four times to the static panel, and no longer constructs its bordered panels locally. It uses the existing lexical/source infrastructure and does not extend the Goals-specific semantic resolver or add a dependency.

All accepted Practices, Quotes, Trends, History, and Goals fixtures must remain byte-for-byte unchanged. Any required production change to those screens, any breaking framework change, or any accepted fixture drift is a scope conflict that requires explicit user direction.

### Release Gates

The final batch runs:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
```

It also runs focused Dashboard, framework, TUI lint, localization, removed-feature, fixture-immutability, diff-whitespace, and executable-binary checks.

## Documentation and Delivery

Active documentation will mark Batch 3B-6 Dashboard presentation extraction as shipped while keeping Log Entry form extraction explicitly unshipped. Dashboard's existing user-facing spec remains behaviorally unchanged except for presentation-ownership wording.

The work is delivered as an isolated Dashboard-only binary for manual verification. Manual acceptance covers the three `80x35` modes, long content, smaller terminals, HRV success/cancel/failure, navigation, and regression checks for other screens. Implementation stops after that handoff; Log Entry does not begin until the user explicitly accepts Dashboard.

## Acceptance Criteria

Batch 3B-6 is ready for manual acceptance when:

- the three Dashboard fixtures are identical before and after migration;
- `ScreenFrameSections` and the static-panel sibling own the approved presentation mechanics;
- Dashboard retains all domain rendering and behavior;
- small and pathological rectangles are geometry-safe;
- existing public component APIs and accepted non-Dashboard fixtures are unchanged;
- active documentation accurately marks Dashboard shipped and Log Entry unshipped;
- all release gates pass with zero warnings; and
- the isolated binary is available for user verification.
