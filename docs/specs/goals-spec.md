# Goals Spec

Track training goals with nested milestones and completion progress.

Goals is the reference consumer of the shared interaction contract in `navigation-spec.md`: visual-row-aware viewports, bounded `TextInput`, focused detail styling, overflow indicators, reserved status, and failure-preserving mutation transitions. Goal and milestone domain rules remain owned by this screen.

## Layout

The screen contains a bordered, scrollable goal list, a contextual selected-goal panel, a status line, and a shortcut bar. The selected goal's panel is shown as a read-only preview whenever it has milestones. `Enter` focuses that panel for management.

Each goal shows its selection marker, completion state, title, optional completion date, gauge, and progress text. Goals are ordered with incomplete goals first, then completed goals; each group retains its saved position.

## Progress and Completion

- A goal with milestones displays `completed/total · percentage`, for example `2/4 · 50%`.
- Its completion state is derived from its milestones: completing the last pending milestone completes the goal; reopening, adding, restoring, or deleting a milestone recalculates the goal immediately.
- A goal without milestones displays `0%` or `100%` and can be completed manually with `Space`.
- Pressing `Space` on a goal that has milestones does not override derived progress; the status line explains that milestones control completion.
- Startup reconciliation repairs stale goal completion values from milestone data.

## Selected-Goal Panel

The panel contains the goal gauge followed by every milestone. Each milestone row shows completion state, title, and completion date when available.

- The preview grows to show all content when space permits.
- In constrained terminals, `↑`, `↓`, or `↑↓` in the panel title indicates hidden content.
- Focused panels use an accent border and highlight the active milestone, including wrapped rows.
- Navigation keeps the active milestone and contextual date/delete row visible.
- Long edit fields scroll horizontally so the cursor remains visible.
- Successful add/edit/toggle/date/delete operations keep the panel focused; failed mutations retain the current input and show an error.

## Modes

### Goal List

| Mode | Trigger | Behavior |
|---|---|---|
| Browse | default | Navigate goals and preview the selected goal |
| AddGoal | `a` | Enter a new goal title |
| EditGoal | `e` | Rename the selected goal |
| EditGoalDate | `D` | Edit a completed goal's date as `YYYY-MM-DD` |
| ConfirmDeleteGoal | `d` | Confirm goal deletion |
| Milestone panel | `Enter` | Focus and manage the selected goal's milestones |
| AddMilestone | `m` | Focus the panel directly in milestone-add mode |

### Milestone Panel

| Mode | Trigger | Behavior |
|---|---|---|
| Browse | `Enter` from goal list | Navigate milestones |
| AddMilestone | `a` or `m` from goal list | Enter a new milestone title |
| EditMilestone | `Enter` or `e` | Rename the selected milestone |
| EditMilestoneDate | `D` | Edit a completed milestone's date as `YYYY-MM-DD` |
| ConfirmDeleteMilestone | `d` | Confirm milestone deletion |

## Keybindings

### Goal List

| Key | Action |
|---|---|
| `j/k` or arrows | Navigate goals |
| `Enter` | Focus the selected goal's milestone panel |
| `m` | Add a milestone to the selected goal |
| `a` | Add a goal |
| `e` | Rename the selected goal |
| `Space` | Toggle completion only when the goal has no milestones |
| `D` | Edit a completed goal's completion date |
| `d`, then `y` | Delete the selected goal |
| `u` | Undo the last goal deletion |
| `Esc` | Return to Dashboard |

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
| `Esc` | Return to the goal list |

Text-entry modes use `Enter` to save and `Esc` to cancel. Delete confirmation accepts `y`; any other key cancels.

## Undo and Deletion

Goal and milestone deletion each support one level of undo. Milestone restore preserves its original position, creation timestamp, completion state, and completion date. Deleting a goal cascades to its milestones.
