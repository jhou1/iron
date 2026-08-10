# Goals and Milestone Management Redesign

## Purpose

Make the Goals screen reliably show every milestone associated with the selected goal and let the user update milestone completion quickly without losing context.

Delivery is deliberately split into two stages:

1. Redesign and verify the Goals screen only.
2. After the user validates stage one, audit History, Trends, Quotes, Practices, and other applicable screens and align their scrolling, focus, input, status, and shortcut behavior.

Stage two must not begin before explicit user acceptance of stage one.

## Current Problems

The database loads all milestones, but the Goals view loses access to some of them:

- Popup height includes two border rows but omits the two rows consumed by uniform vertical padding.
- `modal_scroll` is rendered but is never adjusted when `modal_selected` changes.
- Browse mode shows a popup that looks interactive, while `j` and `k` still move between goals.
- Wrapped titles consume more visual rows than the layout and selection calculations assume.
- Progress displays only a percentage even though the specification promises completed and total counts.
- Goal completion can disagree with milestone completion.
- README and specification shortcuts do not match the implemented shortcuts.
- Existing tests cover goal-list scrolling but not milestone-panel layout or scrolling.

## Stage One Scope

Stage one changes only the Goals workflow and the documentation and tests that describe it. It may add Goals-specific helpers, but it must not change the rendering, input handling, or presentation rules of History, Trends, Quotes, Practices, Dashboard, or other screens. The existing Dashboard may naturally reflect a goal's newly synchronized completion state through its current data query; no Dashboard code is changed in stage one.

The existing milestone data model remains binary: each milestone is either complete or incomplete. Stage one does not add a separate 0–100% value to individual milestones. Goal progress is the aggregate `completed milestones / total milestones`.

### Selected Goal Panel

Selecting a goal opens a stable milestone panel associated with that goal. The same panel is used for preview and management so that entering management mode does not move, resize, or replace the content unexpectedly.

The panel contains:

1. Goal title.
2. Aggregate progress in the form `2/4 · 50%` plus the existing progress bar.
3. Milestone rows in stored position order.
4. Overflow indicators when the complete list cannot fit in the terminal.

Each milestone row shows:

- Selection marker when the panel has focus.
- Complete or incomplete marker.
- Milestone title.
- Completion date when complete.

The panel grows to the content height when all milestones fit. Its maximum height is the available Goals viewport. If the content exceeds that height, the panel becomes scrollable and shows that more items exist above or below. “Show all milestones” therefore means every milestone is present and reachable; a terminal that is physically too short may use a viewport rather than clipping data silently.

All height and scroll calculations use the panel's actual inner rectangle after borders and padding. Wrapped visual rows, date-edit rows, add rows, and confirmation rows count toward the layout.

### Focus and Keyboard Behavior

In the goal list:

- `j` / `k` or arrow keys select a goal.
- `Enter` focuses the selected goal's milestone panel.
- `m` focuses the panel directly in add-milestone mode.
- `a` adds a goal.
- `e` edits the selected goal title.
- `Space` toggles a goal only when it has no milestones.
- `Esc` returns to Dashboard.

In the milestone panel:

- `j` / `k` or arrow keys select a milestone and keep it visible.
- `Space` toggles the selected milestone immediately and retains selection and panel focus.
- `Enter` or `e` edits the selected milestone title.
- `a` adds a milestone, selects the newly created milestone, and scrolls it into view.
- `D` edits the completion date of a completed milestone.
- `d` asks for confirmation before deleting the selected milestone.
- `u` restores the most recently deleted milestone.
- `Esc` returns focus to the goal list without closing the Goals screen.

The footer always describes the shortcuts for the currently focused level. The focused panel has a distinct border or title style so that goal navigation and milestone navigation cannot be confused.

### Completion Semantics

For a goal with one or more milestones, completion is derived from those milestones:

- Completing the final incomplete milestone marks the goal complete and records the goal completion time.
- Reopening any milestone marks the goal incomplete and clears the goal completion time.
- Adding an incomplete milestone to a completed goal reopens the goal.
- Deleting or restoring a milestone recalculates the parent goal.
- If deleting the final milestone leaves the goal with no milestones, the goal keeps its current completion state and returns to manual completion behavior.
- Direct `Space` toggling on a goal with milestones is disabled and shows a short explanation.

A goal with no milestones retains manual completion through `Space`.

Existing goals with milestones are reconciled to this rule when the stage-one database behavior is introduced. When reconciliation completes an existing goal, its completion time is the latest available milestone completion time, or the current time when no milestone completion time is available. The reconciliation changes only `goals.completed` and `goals.completed_at`; it does not modify milestone titles, order, or completion dates.

Deleting and undoing a milestone preserves its title, original position, completion state, and completion date. Undo must not silently append the milestone at a new position or replace its original completion date with the undo time.

### Progress Display

The Goals screen displays both count and percentage, for example `2/4 · 50%`. A goal without milestones displays manual state as `0%` or `100%` without a misleading `0/0` count.

Stage one must not change the shared gauge output used by Dashboard or other screens. The count and percentage formatting remains local to Goals until stage two determines whether a shared presentation is appropriate.

### Text Input and Wrapping

Goal and milestone inputs use the existing `visible_input_spans()` helper so the cursor remains visible and text never overflows its container. User-entered titles continue to wrap in display mode. Scroll and highlight positions are based on visual row measurements rather than a fixed number of rows per item.

### Error Handling

Create, update, toggle, delete, restore, date, and reload errors are shown in the relevant Goals status area. On a failed mutation, the current focus and input remain available so the user can retry or cancel. The UI must not show a successful state based only on an in-memory assumption when persistence failed.

## Stage One Testing

Add regression coverage for:

- Four milestones at the minimum supported terminal size: all four are visible when space permits.
- More milestones than fit: moving down and up keeps the selected milestone visible.
- The progress summary displays completed count, total count, and percentage.
- Adding a milestone selects it and makes the add result visible.
- Add, date-edit, and delete-confirmation rows participate in scroll calculations.
- Long goal and milestone titles wrap without breaking highlight or scroll positions.
- Goal completion synchronizes when milestones are toggled, added, deleted, and restored.
- Goals with no milestones retain manual completion.
- Persistence failures do not discard edit state.

Run the full test suite after the focused tests. Because this includes a bug fix, increment the package patch version according to the repository convention.

## Stage One Acceptance Checklist

The user should be able to verify the following manually:

1. Select a goal with four milestones and see all four at the current terminal size.
2. Enter the milestone panel and navigate to every milestone with `j` / `k` or arrow keys.
3. Toggle any milestone with `Space` and see the row plus `completed/total · percentage` update immediately.
4. Edit a milestone, add a milestone, edit a completion date, delete a milestone, and undo the deletion without losing panel focus.
5. Resize to a smaller supported terminal and confirm hidden items are indicated and scroll into view.
6. Complete the final milestone and confirm the goal becomes complete; reopen one and confirm the goal becomes active again.
7. Confirm the footer and README shortcuts match actual behavior.

Stage one is complete only after automated verification passes and the user has performed this manual validation.

## Stage Two Consistency Rollout

Stage two begins only after explicit user acceptance of stage one. It starts with an audit rather than automatically copying Goals code into every screen.

The audit covers History, Trends, Quotes, Practices, Log Entry, Quick Log, Dashboard, and any other interactive TUI view. For each applicable screen, compare:

- Selection always remains visible while navigating.
- Borders and padding are included through actual inner rectangles.
- Wrapped content uses visual-row-aware layout where necessary.
- Inputs use horizontal cursor scrolling.
- Preview and management focus are visually distinct.
- Footers describe the active context and match README/specifications.
- Mutations preserve user state on failure and report errors.
- Overflow is indicated rather than silently clipping content.

Only behaviors that apply to a screen should be unified. Stage two may extract shared helpers after the Goals behavior has been validated, but it must avoid forcing list, popup, chart, and form screens into one unsuitable abstraction.

Stage two receives its own implementation plan and acceptance checklist based on the audit findings.

## Out of Scope

- Per-milestone numeric progress other than complete/incomplete.
- Milestone weights or weighted goal progress.
- Drag-and-drop or keyboard reordering.
- New due-date or reminder fields.
- Stage-two changes before stage-one user acceptance.
