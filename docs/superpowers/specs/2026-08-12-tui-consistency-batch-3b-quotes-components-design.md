# TUI Consistency Framework — Batch 3B-2 Quotes Components Design

**Date:** 2026-08-12  
**Status:** Approved design; implementation in progress (footer-preservation correction approved 2026-08-13)

## Context

Batch 3B-1 introduced three presentation-only Ratatui components—`ScreenFrame`, `ScrollableListPanel`, and `ActionPanel`—and migrated Practices without changing its accepted terminal output or interaction behavior. Practices has passed automated and manual acceptance.

Batch 3B-2 is the next single-screen vertical slice. It migrates Quotes to the proven components while preserving the existing compact list, floating preview, focused Detail mode, add/edit input, delete confirmation, undo behavior, shortcuts, lifecycle, and persistence behavior.

This remains an incremental structural refactor. Trends does not begin until Quotes has passed its own automated and manual acceptance.

## Goals

- Freeze the current Quotes rendering across every visible mode and the optional undo state with exact styled-buffer fixtures.
- Reuse `ScreenFrame`, `ScrollableListPanel`, and `ActionPanel` for the presentation mechanics already proven by Practices.
- Extend the list-panel API only enough to return its safely clipped outer and inner rectangles for Quotes preview placement.
- Extend `ScreenFrame` with a generic, presentation-only layout configuration proven necessary by the immutable Quotes fixtures, while keeping its existing renderer as a backward-compatible default wrapper.
- Migrate Quotes without changing visible output, shortcuts, navigation, database behavior, lifecycle behavior, failure retention, detail scrolling, or undo behavior.
- Keep floating-preview sizing, focus, content measurement, and scrolling owned by Quotes and the existing `render_detail_panel` helper.
- Remove Quotes-local duplication of frame layout, list borders, overflow-title composition, viewport reveal/clamp, visible-row slicing, selected-row highlighting, status rendering, shortcut rendering, and action-panel rendering.
- Keep every affected component safe for empty, zero-sized, one-cell, narrow, and short rectangles.

## Non-Goals

- No visual redesign, copy change, color change, border change, spacing change, shortcut change, or navigation change.
- No migration of Trends, History, Goals, Dashboard, or Log Entry.
- No database, data-model, schema, localization, router, navigation-intent, or lifecycle change.
- No extraction of Quotes modes, database operations, validation, key handling, shortcut selection, undo policy, or detail-focus state.
- No new generic floating-popup or composite list-detail screen component.
- No speculative component options for later screens.
- No coexistence of old and new Quotes presentation implementations after migration.
- No reintroduction or replacement of Quick Log, LLM configuration, or Abbreviations.

## Chosen Approach

Batch 3B-2 uses the existing three components and makes two demonstrated API extensions: `ScrollableListPanel` returns safely clipped geometry after it renders, and `ScreenFrame` accepts an optional generic body/trailing-flex layout policy through a new configured renderer.

Quotes needs the list panel's inner rectangle because its preview is positioned inside that padded region. Recomputing the block geometry in Quotes would preserve duplicate presentation logic, while introducing a new popup framework would over-design for screens that have not yet been migrated. Returning generic geometry keeps the component boundary narrow and lets Quotes retain its unique preview policy.

The second extension was approved after migration RED proved that the original three-row `ScreenFrame` layout moved the Quotes footer and expanded its list. The exact migration produced 917 styled-cell differences in Browse. Adding a trailing flexible row only inside Quotes restored list/preview placement but left 144 footer-cell differences because the default frame still anchored status and shortcuts at the terminal bottom. The approved solution keeps that policy in the shared frame component: `ScreenFrameLayout` exposes only `min_body_height: u16` and `trailing_flex: bool`, and `render_screen_frame_with_layout` owns the four-row allocation. Quotes must not retain or derive its old six-row footer layout locally.

Rejected alternatives:

- **Add a generic floating-detail component now:** this could be useful to a later screen, but Quotes alone does not establish a complete shared popup contract.
- **Create a composite list/detail/action screen component:** this would couple reusable presentation code to Quotes modes, focus, and layout policy.
- **Recompute list inner geometry locally:** this would leave two sources of truth for border padding and tiny-rectangle clipping.
- **Retain or derive the old six-row layout in Quotes:** a bounded-sub-rectangle diagnostic proved fixture parity was possible, but it duplicated footer allocation and was rejected in favor of generic shared ownership.

## Compatibility Contract

Batch 3B-2 is a structural refactor, not a user-facing behavior change.

At an `80x24` terminal, the complete rendered Ratatui buffer must match its pre-refactor fixture for these states:

1. **Browse** — multiple quotes, selected row, compact truncation, and floating preview.
2. **BrowseWithUndo** — optional `[u] Undo` shortcut and its resulting shortcut layout.
3. **Detail** — long quote, focused detail border, visual-row scrolling, and overflow marker.
4. **Add** — active input, cursor presentation, confirm/cancel actions, status, and shortcuts.
5. **Edit** — existing quote content, editing cursor, confirm/cancel actions, status, and shortcuts.
6. **ConfirmDelete** — confirmation copy, action hints, status, and active shortcuts.

Fixtures compare every cell's symbol, foreground color, background color, modifiers, and skip flag. They are captured from the old implementation, committed before migration, and cannot be regenerated by the normal test path.

Existing behavior tests remain binding, including long preview scrolling, empty input retention, small-terminal rendering, dynamic undo shortcuts, lifecycle re-entry, mutation-state reset, and database failure retention. If a characterization fixture exposes a current defect, implementation stops for a scope decision instead of silently accepting a visual change.

For zero-sized or insufficient rectangles, safety takes precedence over reproducing impossible off-screen geometry: rendering may safely omit clipped content, but it must not panic or access outside the frame.

## Ownership Boundary

### Quotes Owns

- `Quote` values and stable quote identities.
- `Mode`: Browse, Detail, Add, Edit, and ConfirmDelete.
- Selected quote index and identity restoration policy.
- Text input, editing identity, delete confirmation, and one-item undo payload.
- Database reads, writes, refreshes, deletes, and restores.
- Validation, localized status/error creation, and retry behavior.
- Key handling and the active shortcut set for each mode.
- Navigation actions and lifecycle re-entry behavior.
- Domain-specific list-row formatting and display-width truncation.
- Floating-preview presence, size, placement policy, focus, content measurement, and scroll state.
- Formatting the Add, Edit, and ConfirmDelete action lines.

### Framework Components Own

- Centering and bounding the screen content region.
- Measuring active shortcut wrapping and reserving status/shortcut rows.
- Rendering status and shortcuts through existing shared helpers.
- List border, padding, title, overflow suffix, logical-row viewport, visible rows, and selected-row highlight.
- Returning safely clipped list outer and inner rectangles.
- Bounded rendering of already-formatted action lines.

The framework components do not import database types, `Quote`, Quotes modes, navigation intents, validation errors, undo state, or detail state.

## Component API Extension

`render_scrollable_list_panel` will return a small presentation-only value, conceptually:

```rust
pub struct ListPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}
```

The exact Rust syntax may vary to improve naming or ergonomics, but the contract is fixed:

- `area` is the target clipped to the frame.
- `inner` is the padded block interior clipped to `area`.
- Both rectangles remain inside the frame and their parent for normal, zero-sized, one-cell, and edge-positioned targets.
- Returning geometry does not expose viewport internals or permit the component to mutate screen state beyond the supplied `ViewportState`.
- Existing Practices rendering consumes the component exactly as before and must remain fixture-identical.

The failing Quotes characterization proved one additional presentation configuration necessary. `ScreenFrameLayout` has exactly these concepts:

```rust
pub struct ScreenFrameLayout {
    pub min_body_height: u16,
    pub trailing_flex: bool,
}
```

`render_screen_frame_with_layout(...)` uses `[Constraint::Min(min_body_height), status, shortcuts, Constraint::Min(0)]` when `trailing_flex` is true and preserves the current three-row layout otherwise. Existing `render_screen_frame(...)` remains the default wrapper, so Practices and other existing callers retain their output. Quotes supplies `min_body_height = 5 + action_height` and `trailing_flex = true`; it then subdivides only the returned body into list, spacer, and action areas. Neither the layout type nor the configured renderer imports Quotes or business state.

## Render and Data Flow

Each Quotes render follows this sequence:

1. Quotes derives the active shortcuts from its current mode and undo state.
2. Quotes calls the configured `ScreenFrame` renderer with `min_body_height = 5 + action_height` and trailing flex enabled. The component clips and centers the available area, measures shortcut wrapping, allocates the generic body/status/shortcuts/trailing-flex rows, renders status/shortcuts, and returns the body rectangle.
3. Quotes divides the body into the current list, spacer, and mode-dependent action regions using the existing constraints and heights.
4. Quotes formats all logical list rows using the list panel's actual inner width.
5. `ScrollableListPanel` renders the border, overflow title, visible rows, viewport, and selected highlight, then returns its safe geometry.
6. In Browse or Detail, Quotes positions the existing floating preview within the returned list `inner` rectangle and calls `render_detail_panel`.
7. For Add, Edit, or ConfirmDelete, Quotes formats the existing three action lines and passes them to `ActionPanel`.

There is one production render path. After migration, Quotes does not retain its previous parallel border, viewport, highlight, status, shortcut, or action `Paragraph` implementation.

## Preview and Detail Contract

The floating preview is intentionally not absorbed into a new component.

- Browse shows the selected quote's complete wrapped text in the existing bounded lower preview.
- Detail expands the preview to the existing focused area and enables visual-row scrolling.
- `Enter` enters Detail only when a quote exists.
- `j/k` and arrow keys scroll Detail by visual row.
- `Esc` or `Enter` returns from Detail to Browse.
- Unfocused Browse preview resets its detail viewport to the top.
- The existing accent border, title, overflow markers, clear operation, and content wrapping remain unchanged.
- Popup rectangles are derived only from the safe list inner rectangle and clipped before `Clear` or detail rendering.

## Modes and Interaction Contract

The migration does not change mode transitions:

- **Browse:** navigate, open Detail, add, edit, request deletion, undo, or return to Dashboard.
- **Detail:** scroll the complete preview or return to Browse.
- **Add/Edit:** edit through the existing bounded `TextInput`; Enter attempts persistence and Esc cancels.
- **ConfirmDelete:** `y` confirms; any other key cancels.

The shortcut footer continues to describe only the active mode. Browse adds `[u] Undo` only when an undo payload exists and always includes `[Esc] Back`.

## Failure and State Retention

Failure semantics remain unchanged:

- Empty Add/Edit input retains mode, text-input state, and cursor and shows an error.
- Create/update failure retains Add/Edit and its retryable input state.
- A successful mutation followed by refresh failure retains the current mode and input state and shows the error according to the existing contract; this structural slice does not change post-write refresh or retry semantics.
- Delete failure retains ConfirmDelete.
- Restore failure restores the undo payload so the user may retry.
- Successful delete stores exactly one quote for undo and returns to Browse after refresh.
- Successful restore refreshes the list and reports success.
- Re-entry reloads current data, restores the stable selection where possible, preserves bounded browse/detail viewports, keeps Detail only when a quote remains, and clears transient mutation state, old status, and cross-page undo.
- Cross-page preparation failure remains atomic and is handled by the existing application shell.

## Geometry and Safety

All new or changed geometry uses saturating arithmetic and `fit_rect`-equivalent clipping.

The implementation must not:

- construct a list child outside the target or frame;
- use an unclipped `Block::inner` result;
- compute preview dimensions from a rectangle not returned by the list component;
- call `Clear` or `render_detail_panel` with an escaping rectangle;
- assume a minimum terminal width or height;
- underflow when borders, padding, spacing, status, shortcuts, or actions consume all available space.

On tiny rectangles, the safe subset may be empty. The viewport still clamps to valid logical content, and later expansion must reveal the selected item normally.

## Test Strategy

### Characterization Before Migration

- Add the six immutable `80x24` styled-buffer fixtures.
- Prove the test is initially RED with empty fixture content.
- Capture fixtures only from the pre-refactor implementation.
- Remove or disable the update path before committing fixtures.

### Component Tests

- Verify returned list `area` and `inner` geometry for normal, zero-sized, one-cell, narrow, short, and frame-edge targets.
- Verify both rectangles stay within their binding parents.
- Preserve existing border, header, overflow, viewport, empty-state, and highlight tests.
- Prove the default `ScreenFrame` wrapper is unchanged, configured `80x24` status/shortcut placement matches the accepted Quotes rows, and configured zero/tiny targets remain clipped inside the frame.
- Re-run Practices golden fixtures unchanged after the API extension.

### Quotes Tests

- Prove all six Quotes fixtures are identical after migration.
- Verify preview rectangles remain inside the returned list interior.
- Retain and extend long-preview, Detail scrolling, empty-list, input retention, delete/restore failure, dynamic shortcuts, re-entry, and small-terminal tests where needed.
- Test normal, narrow, short, and tiny terminal dimensions without panic or out-of-bounds geometry.

### Structural Lint

The recursive TUI lint will require Practices and Quotes to delegate their owned list border to `ScrollableListPanel` using executable source evidence. It will reject comments, ordinary strings, raw strings, and character literals as ownership evidence while preserving Rust lifetime syntax.

Unmigrated screens keep their existing direct-border requirement. The Paragraph wrapping lint keeps its current recursive scan, four-line window, and approved semantic allowlist unless an exact source-line move requires a mechanical refresh.

### Release Gates

- `cargo fmt -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- focused framework, Quotes, Practices golden, lifecycle, i18n, TUI lint, and removed-feature tests
- `cargo test --all-targets --all-features`
- `cargo build --all-features`
- fixture immutability checks
- `git diff --check` over the Batch 3B-2 range
- clean tracked worktree at handoff

## Documentation

After implementation, active documentation will state that Batch 3B-1 Practices and Batch 3B-2 Quotes are shipped, while Trends and later component-extraction slices remain unshipped.

Documentation updates are limited to the files needed to describe the Quotes component boundary, preserved behavior, test contract, and rollout status. Historical design records are not rewritten as if they had shipped earlier.

## Commit Strategy

Implementation uses independently reviewable commits:

1. Capture immutable Quotes styled-buffer fixtures from the old implementation.
2. Extend list-panel safe geometry and its tests without migrating Quotes.
3. Record the approved footer-preservation design/plan correction.
4. Migrate Quotes to the shared components, extend `ScreenFrame` generically, and update structural lint.
5. Update shipped-state documentation and complete final verification.

Any correctness fix discovered during characterization or review is isolated in its own follow-up commit with focused RED/GREEN evidence.

## Manual Acceptance

Batch 3B-2 is handed off as a Quotes-only binary. Manual acceptance covers:

- Browse selection, scrolling, truncation, overflow, and preview;
- Detail focus, visual-row scrolling, overflow, and return behavior;
- Add and Edit input, Unicode/cursor visibility, save, cancel, and failure retention;
- delete confirmation, successful deletion, undo, and retryable failures;
- empty list and re-entry after external data changes;
- normal, narrow, short, tiny, and resized terminals;
- status lines and mode-specific shortcut wrapping; and
- return to Dashboard and atomic re-entry behavior.

Trends work does not start until the user explicitly accepts this Quotes slice.

## Acceptance Criteria

Batch 3B-2 is ready for manual acceptance when:

- all six pre-refactor Quotes fixtures pass unchanged after migration;
- Practices fixtures remain unchanged;
- the list-panel geometry extension is safe and independently tested;
- Quotes contains no duplicate local implementation of the delegated presentation mechanics;
- Quotes preview, business, lifecycle, shortcuts, status, undo, and failure behavior remain unchanged;
- tiny rectangles cannot cause out-of-bounds rendering or panic;
- structural lint accurately distinguishes executable delegation from non-code evidence;
- formatting, strict Clippy, build, full tests, fixture checks, and diff checks pass;
- documentation identifies only Practices and Quotes as shipped Batch 3B component consumers; and
- the tracked worktree is clean at the manual-acceptance handoff.
