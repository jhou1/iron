# Goals Spec

Track training goals with nested milestones and completion progress.

## Layout (top to bottom)

1. Title
2. Goal list (scrollable)
3. Action area (contextual)
4. Status line
5. Shortcuts bar

## Goal List

Each row shows:
- Marker: ">" if selected
- Checkbox: "☑" (completed) or "☐" (incomplete)
- Title
- Progress gauge: `[=====     ] 3/5` (completed/total milestones)
- Completion date (if completed): "(Apr 15)"

Goals ordered: incomplete first (by position), then completed (by position).

## Progress Gauge

- If goal has milestones: completed_milestones / total_milestones
- If no milestones: 0% when incomplete, 100% when completed

## Modes (Main)

| Mode | Trigger | Behavior |
|------|---------|----------|
| Browse | default | Navigate goal list |
| AddGoal | `a` | Text input for new goal title |
| EditGoal | `Enter` | Text input to rename goal |
| EditGoalDate | `D` | Date input (YYYY-MM-DD) for completion date |
| ConfirmDeleteGoal | `d` | Deletion confirmation |
| Modal | `Enter`/`m` | Milestone sub-screen for selected goal |

## Milestone Modal

Overlay showing milestones for a single goal.

**Layout:**
- Goal name as centered header
- Milestone list (scrollable)
- Action area
- Shortcuts

**Milestone row:** checkbox, title, completion date (if completed)

### Modal Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Browse | default | Navigate milestones |
| AddMilestone | `a` | Text input for new milestone |
| EditMilestone | `Enter` | Text input to rename |
| EditMilestoneDate | `D` | Date input for completion date |
| ConfirmDeleteMilestone | `d` | Deletion confirmation |

## Keybindings (Main)

| Key | Action |
|-----|--------|
| `j/k` | Navigate goals |
| `a` | Add new goal |
| `Enter` | Edit goal title or open milestones |
| `m` | Add milestone to selected goal |
| `Space` | Toggle goal completion |
| `D` | Edit completion date |
| `d` | Delete goal (confirm) |
| `y/n` | Confirm/cancel deletion |
| `?` | Help overlay |
| `Esc` | Back to Dashboard |

## Keybindings (Modal)

| Key | Action |
|-----|--------|
| `j/k` | Navigate milestones |
| `a` | Add milestone |
| `Enter` | Edit milestone title |
| `Space` | Toggle milestone completion |
| `D` | Edit milestone completion date |
| `d` | Delete milestone (confirm) |
| `Esc` | Close modal, back to goal list |

## Undo

One level of undo for goal or milestone deletions. Stores the deleted entity and restores it on `u`.

## Cascade Deletion

Deleting a goal deletes all its milestones.

## Completion Toggle

- `Space` toggles `completed` flag
- Sets `completed_at` to now when completing, clears it when uncompleting
- Goals: toggling does not affect milestone states
- Milestones: toggling updates the parent goal's progress gauge
