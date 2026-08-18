# Batch 3B-7: Log Entry Presentation Components Design

**Date:** 2026-08-19
**Status:** Approved for implementation planning
**Base:** `f747090` (`main`, after accepted Batch 3B-6 Dashboard integration)

## Context

Log Entry is the final screen whose presentation-component extraction remains unshipped. Its behavior is already accepted: request-scoped construction, stable-ID History edits, shared single-line inputs, clamped practice selection, active-mode shortcuts, transactional navigation, and complete failure retention are all implemented and tested.

The remaining problem is presentation ownership. `src/tui/log_entry.rs` still performs local screen framing, bordered-panel construction, padding and safe-inner calculations, list viewport rendering, selected-row highlighting, status placement, and shortcut placement. That duplicates mechanics now owned by the shared TUI framework in every other applicable screen.

This batch extracts those mechanics without redesigning Log Entry. Existing visual output, layout, keyboard commands, domain state, validation, database behavior, navigation, and lifecycle behavior are binding compatibility requirements.

## Goals

1. Freeze the accepted Log Entry output before changing presentation ownership.
2. Add the smallest presentation-only panel primitive needed to preserve Log Entry's existing panel variants exactly.
3. Migrate the practice selector to shared frame and list components.
4. Migrate the entry form to shared frame and bounded-panel components.
5. Remove superseded Log Entry-local border, padding, safe-inner, list slicing, highlight, status, and shortcut rendering.
6. Preserve all accepted fixtures for Dashboard, Goals, History, Trends, Quotes, and Practices.

## Non-Goals

- No visual redesign or field reordering.
- No changes to shortcuts, focus order, form phases, set editing, date editing, or exit confirmation.
- No database, schema, model, route, navigation-intent, lifecycle, or application-shell changes.
- No multi-line input buffer; Note remains an existing single-line `TextInput` rendered inside the accepted area.
- No form schema, generic form engine, validation framework, or domain-aware framework component.
- No change to `StaticPanel`'s public behavior or Dashboard output.
- No resurrection of Quick Log, LLM configuration/runtime, Abbreviations APIs, or the `abbreviations` table.

## Accepted Architecture

### `bounded_panel.rs`

Add `src/tui/framework/bounded_panel.rs` as a presentation-only primitive.

```rust
pub struct BoundedPanelSpec<'a> {
    pub title: Option<Line<'a>>,
    pub border_style: Style,
    pub padding: Padding,
}

pub struct BoundedPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_bounded_panel(
    frame: &mut Frame,
    area: Rect,
    spec: BoundedPanelSpec<'_>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> BoundedPanelAreas;
```

The exact Rust lifetime spelling may be adjusted if required by Ratatui, but the public semantics are fixed:

- all four borders are rendered;
- the requested area is clipped to the frame;
- `Block::inner` is clipped back to the rendered outer area;
- caller-provided title content/style, border style, and padding are preserved exactly;
- the content closure receives the exact safe inner rectangle once;
- no domain data, input state, viewport, status, shortcuts, database, or screen mode enters the component.

Zero-sized, one-cell, clipped-edge, oversized, and maximum-coordinate rectangles must not panic or return geometry outside their parent.

### `StaticPanel` compatibility wrapper

`render_static_panel` becomes a wrapper around `render_bounded_panel`. Its current decorated title, `BORDER_COLOR`, uniform padding, returned geometry, public signature, and Dashboard rendering remain unchanged. `StaticPanelAreas` may remain a named wrapper type or a compatibility alias for `BoundedPanelAreas`; callers must not need behavioral changes.

This direction keeps the architecture layered:

```text
BoundedPanel (generic border/title/padding/safe inner)
    -> StaticPanel (decorated Dashboard-style convenience)
    -> Log Entry (exact caller-supplied form panel variants)
```

It avoids expanding `StaticPanel` into a configuration-heavy API and avoids introducing a domain-aware `FormPanel`.

## Log Entry Migration

### Phase 1: Practice selection

The phase uses one `render_screen_frame_sections` call with the accepted centered width and two body constraints:

```text
Length(1)  filter row
Min(0)     practice list
```

`trailing_flex` remains `false`. The frame renders the reserved status and wrapped shortcut areas exactly once.

The practice list uses one `render_scrollable_list_panel` call. Log Entry continues to own:

- the source practices and filtered-index mapping;
- filter text and filtering mode;
- selected filtered index and clamp policy;
- practice-name and practice-type column budgeting;
- formatted header and row `Line` values;
- empty selection behavior and navigation actions.

The list panel owns:

- border, decorated title, and padding;
- viewport height, reveal, clamp, and overflow suffix;
- visible-row slicing;
- selected-row highlight;
- clipped outer and inner geometry.

No new selection or viewport state is introduced.

### Phase 2: Entry form

The phase uses one `render_screen_frame_sections` call with the current five body constraints and `trailing_flex = false`:

```text
Length(5)                              date panel
Length(1)                              spacer
Length(sets_content_lines + 4)         Sets panel
Length(7)                              warm-up/cool-down/RPE panel
Min(0)                                 Note panel
```

The current saturating conversion and height calculation remain Log Entry-owned.

Four `render_bounded_panel` calls preserve the exact existing panel variants:

| Panel | Title | Padding | Border |
|---|---|---|---|
| Date | practice name/type, accent bold | uniform 1 | `BORDER_COLOR` |
| Sets | `Sets`, white bold | uniform 1 | `BORDER_COLOR` |
| Warm-up/Cool-down/RPE | none | uniform 1 | `BORDER_COLOR` |
| Note | optional-note label, gray | horizontal 1 | `BORDER_COLOR` |

Log Entry continues to format and render all content inside the returned inner rectangles:

- date label/value/editor and change hint;
- committed set rows, selection/edit marker, practice-type-specific active fields, and running total;
- warm-up, cool-down, and RPE inputs;
- Note input and inactive value;
- cursor visibility, truncation, colors, and focus styles.

The framework does not calculate totals, interpret practice types, validate fields, or mutate form state.

### Status and shortcuts

Both phases calculate their existing active shortcut list before invoking the frame. Entry-form status is:

- the navigation or validation status when one exists;
- otherwise the exit-confirmation prompt while confirmation is active;
- otherwise empty.

This preserves the accepted rule that a failed return navigation error remains visible even when exit confirmation state is retained. Status and shortcuts render once through `ScreenFrameSections`; Log Entry no longer calls the shared status or shortcut renderers directly.

## State, Data, and Error Flow

The migration changes no state transition.

```text
request-scoped constructor
    -> SelectPractice or pre-populated EnterLog state
    -> handler mutates only in-place form state
    -> renderer formats domain content
    -> shared components allocate and render presentation geometry
    -> save validates fields
    -> create_log_at or update_log
    -> typed navigation to Dashboard or History
```

Existing guarantees remain binding:

- invalid date retains the date editor and exact text;
- invalid RPE retains the complete form and focus;
- create/update failure retains every set, input, cursor, focus, editing ID, and return target;
- failed return preparation leaves Log Entry active, preserves the persisted log identity, and prevents duplicate insertion on retry;
- navigation errors take visual precedence over a retained exit-confirmation prompt;
- a successful new save returns to Dashboard;
- a successful edit or confirmed cancellation returns to History;
- target preparation remains atomic and application routing is unchanged.

Shared presentation functions are infallible and must never clear input, change mode, perform database work, or emit navigation actions.

## Characterization Fixtures

Before production migration, commit eight immutable styled-buffer fixtures rendered at `80x24`:

1. populated practice browse;
2. active practice filtering;
3. weighted entry form with committed Sets;
4. active Set edit;
5. active date edit;
6. long Note with Note focus;
7. exit confirmation;
8. pre-populated existing-log edit.

Fixtures use deterministic test data and dates without adding a production clock seam. A temporary fixture writer is permitted only in the characterization step and must be removed before that commit. Later tasks compile fixtures with `include_str!`; migration must not update them.

Existing behavior tests remain authoritative for other practice types, input cursor budgeting, shortcut availability, database failures, set navigation/edit/delete, saving, and navigation retention.

## Ownership Enforcement

After migration, a source ownership test scans only executable production code and requires exactly:

- two `render_screen_frame_sections` calls, one per phase;
- one `render_scrollable_list_panel` call;
- four `render_bounded_panel` calls.

It rejects remaining Log Entry-local:

- `.borders(Borders::ALL)` construction;
- `Block::inner` use;
- direct `render_status_line` and `render_shortcuts` calls;
- local visible-row slicing or selected-row highlighting for the practice list.

Comments, ordinary/raw strings, unreachable test decoys, and `#[cfg(test)]` code cannot satisfy ownership. Synthetic positive and negative controls prove the guard detects missing, duplicate, wrong-origin, and non-production evidence without rejecting the approved production path.

This lint is scoped to Log Entry and does not rewrite the already accepted Goals ownership analyzer or other screen guards.

## Geometry and Responsive Contract

- Exact styled output at `80x24` is unchanged.
- Direct rendering at `24x10`, `1x1`, and zero-sized areas does not panic.
- All derived outer and inner rectangles remain inside the frame and their parent panels.
- Resize sequences recompute complete output rather than retaining stale geometry.
- Long filter and form inputs retain a visible cursor through `TextInput::visible_spans`.
- Dynamic Sets height remains unchanged; this batch does not introduce form scrolling or redesign short-terminal content priority.
- Shortcut rows remain complete key-label units and never overwrite form content.

## Delivery Sequence

The work is delivered as five independently reviewed tasks on an isolated branch/worktree:

1. **Characterization:** add eight immutable Log Entry fixtures and deterministic test helpers only.
2. **BoundedPanel:** add the generic primitive, migrate `StaticPanel` internally, and prove Dashboard/non-Dashboard fixture parity.
3. **Practice selector:** migrate only `SelectPractice` to shared frame/list ownership.
4. **Entry form:** migrate `EnterLog` to shared frame/four bounded panels, add ownership enforcement, and remove obsolete local presentation code.
5. **Release:** update active documentation, run all focused/full gates and fixture audits, perform independent whole-slice review, and hand off a local binary for manual acceptance.

Each task records a stable base commit. Critical and Important review findings are fixed before advancing. The batch stops after manual handoff; merge or push requires explicit user direction.

## Verification Gates

Every implementation task runs its focused tests plus applicable fixture audits. The final batch must pass:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check <batch-base>..HEAD
test -x target/debug/iron
```

Required focused coverage includes:

- all eight Log Entry fixtures in both library and binary targets;
- all Log Entry behavior tests;
- BoundedPanel, StaticPanel, ScreenFrame, ListPanel, and framework suites;
- TUI ownership/Paragraph lint;
- i18n parity and permanent-removal guard;
- unchanged Dashboard, Goals, Practices, Quotes, Trends, and History fixtures;
- absence of any fixture updater after characterization.

## Documentation

On completion, update:

- `CLAUDE.md`;
- `README.md` and `README_zh.md`;
- `docs/specs/log-entry-spec.md`;
- `docs/specs/navigation-spec.md`;
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`.

The documentation will mark Batch 3B-7 complete, describe Log Entry's shared presentation ownership, state that behavior and visuals are unchanged, and remove the final “Log Entry extraction unshipped” language. It will not claim a new form engine or multi-line input.

## Manual Acceptance

Manual verification covers:

- practice browsing, filtering, selection, and back navigation;
- all four practice types;
- set add/edit/delete and selection navigation;
- date edit success/cancel/invalid input;
- long warm-up, cool-down, RPE, and Note inputs;
- save to Dashboard and edit/cancel return to History;
- validation, database-write, and target-preparation failure retention;
- exit confirmation and navigation-error precedence;
- `80x24`, `24x10`, and live resize behavior;
- regression navigation through Dashboard, History, Goals, Trends, Practices, and Quotes;
- continued absence of Quick Log, LLM, and Abbreviations surfaces.

## Completion Criteria

The batch is complete only when:

- all eight pre-migration fixtures remain byte-identical;
- `BoundedPanel` owns generic border/title/padding/safe-inner presentation;
- `StaticPanel` and Dashboard remain behaviorally and visually unchanged;
- Log Entry uses two shared frames, one shared practice list, and four bounded panels;
- Log Entry has no remaining local bordered-panel or footer/status ownership;
- domain state, validation, persistence, routing, and lifecycle behavior remain unchanged;
- all geometry, ownership, formatting, lint, full-test, build, fixture, and binary gates pass;
- independent review has no unresolved Critical or Important finding;
- the user receives the local binary for manual acceptance before integration.
