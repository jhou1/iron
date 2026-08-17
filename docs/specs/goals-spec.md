# Goals Spec

Track training goals with nested milestones and completion progress.

Goals is the approved presentation redesign in the shared component rollout and remains the reference consumer of the interaction contract in `navigation-spec.md`: visual-row-aware viewports, bounded `TextInput`, focused styling, overflow indicators, reserved status, and failure-preserving mutation transitions. Goal and milestone domain rules remain owned by this screen.

## Layout

The screen contains persistent Goals and Milestones panes, a status line, and a shortcut bar. At body widths of `>=83` cells, the panes are horizontal with a one-cell gap; below 83, they are stacked. The Goals pane uses the span-aware `ScrollableListPanel` sibling so a selected Goal remains revealed and is highlighted across its complete multi-line span when focused. The Milestones pane uses the pinned-header `DetailPanel` sibling: the selected Goal aggregate gauge is fixed at its top while milestone rows scroll independently. The existing ListPanel and DetailPanel APIs remain accepted for their current consumers.

Each goal shows its selection marker, completion state, title, optional completion date, gauge, and progress text. Goals are ordered with incomplete goals first, then completed goals; each group retains its saved position.

## Progress and Completion

- A goal with milestones displays `completed/total · percentage`, for example `2/4 · 50%`.
- Its completion state is derived from its milestones: completing the last pending milestone completes the goal; reopening, adding, restoring, or deleting a milestone recalculates the goal immediately.
- A goal without milestones displays `0%` or `100%` and can be completed manually with `Space`.
- Pressing `Space` on a goal that has milestones does not override derived progress; the status line explains that milestones control completion.
- Startup reconciliation repairs stale goal completion values from milestone data.

## Selected-Goal Panel

The Milestones pane contains the pinned selected-Goal gauge followed by every milestone. Each milestone row shows completion state, title, and completion date when available.

- In constrained terminals, `↑`, `↓`, or `↑↓` in a pane title indicates hidden content.
- The focused pane uses an accent border; its active Goal or milestone is highlighted across wrapped rows.
- Navigation keeps the active milestone and contextual date/delete row visible.
- Long edit fields scroll horizontally so the cursor remains visible.
- Successful add/edit/toggle/date/delete operations keep the panel focused; failed mutations retain the current input and show an error.

## Modes

### Goal List

| Mode | Trigger | Behavior |
|---|---|---|
| Browse | default | Navigate Goals while the selected Goal's Milestones pane remains visible |
| AddGoal | `a` | Enter a new goal title |
| EditGoal | `e` | Rename the selected goal |
| EditGoalDate | `D` | Edit a completed goal's date as `YYYY-MM-DD` |
| ConfirmDeleteGoal | `d` | Confirm goal deletion |
| Milestones Browse | `Enter` | Focus and manage the selected Goal's persistent Milestones pane |
| AddMilestone | `m` | Enter the persistent Milestones pane directly in milestone-add mode |

### Milestone Panel

| Mode | Trigger | Behavior |
|---|---|---|
| Browse | `Enter` from Goal Browse | Navigate milestones in the persistent pane |
| AddMilestone | `a` or `m` from goal list | Enter a new milestone title |
| EditMilestone | `Enter` or `e` | Rename the selected milestone |
| EditMilestoneDate | `D` | Edit a completed milestone's date as `YYYY-MM-DD` |
| ConfirmDeleteMilestone | `d` | Confirm milestone deletion |

## Keybindings

### Goal List

| Key | Action |
|---|---|
| `j/k` or arrows | Navigate goals |
| `Enter` | Focus the selected Goal's persistent Milestones pane |
| `m` | Add a milestone to the selected goal |
| `a` | Add a goal |
| `e` | Rename the selected goal |
| `Space` | Toggle completion only when the goal has no milestones |
| `D` | Edit a completed goal's completion date |
| `d`, then `y` | Delete the selected goal |
| `u` | Undo the last goal deletion |
| `Esc` | Return to Dashboard from Goal Browse |

### Milestone Panel

| Key | Action |
|---|---|
| `j/k` or arrows | Navigate milestones |
| `a` | Add a milestone |
| `Enter` or `e` | Rename the selected milestone |
| `Space` | Toggle milestone completion and refresh goal progress |
| `D` | Edit a completed milestone's completion date |
| `d`, then `y` | Delete the selected milestone |
| `u` | Undo the last milestone deletion |
| `Esc` | Return to Goal Browse |

Text-entry modes use `Enter` to save and `Esc` to cancel. Delete confirmation accepts `y`; any other key cancels. Goal add/edit/date and Goal delete-confirmation cancellation return directly to Goal Browse. Milestone add/edit/date and milestone delete-confirmation cancellation return to Milestones Browse. In browse modes, `Esc` then returns from Milestones Browse to Goal Browse and from Goal Browse to Dashboard.

## Undo and Deletion

Goal and milestone deletion each support one level of undo. Milestone restore preserves its original position, creation timestamp, completion state, and completion date. Deleting a goal cascades to its milestones.

## Re-entry

Opening Goals reloads current goals and milestones before the route changes. The selected goal is restored by stable goal ID when possible; if that goal disappeared, its prior index is clamped to the remaining list. If Milestones Browse was focused, its goal and selected milestone are restored by stable IDs when possible, with clamped index fallbacks when either disappeared. Milestones Browse remains focused while any goal remains, including when its original goal disappeared. The independent Goals-list and Milestones-pane viewport contexts are retained and then bounded by measured rendered content.

All Goal or milestone add/edit/date/delete-confirmation modes return to Browse on cross-page re-entry. Goal and milestone inputs/status messages, and both undo payloads, clear. If preparation fails, navigation stays on the source screen, the cached Goals screen is not replaced, and the source shows the error. This presentation redesign does not change database operations, navigation, atomic preparation, re-entry, mutation-failure retention, or undo contracts.
