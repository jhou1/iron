# History Spec

Browse, inspect, edit, and delete training logs.

## Layout

Two-pane horizontal split:

- **Left pane:** Scrollable log list with filter
- **Right pane:** Detail view of selected log

### Left Pane (top to bottom)
1. Title + filter bar
2. Log list (scrollable)
3. Status line
4. Shortcuts bar

### Right Pane
1. Practice name + date header
2. Set details (scrollable)
3. Notes section (if present)

## Log List

- Data source: all logs, filtered to active practices, ordered by date descending
- Each row shows: date ("YYYY MMM DD"), practice name, total metric with unit label
- Selected row highlighted in green

## Detail Panel

For the currently selected log, shows:

- **Header:** "[Practice Name] on [Date Time]"
- **Sets:** numbered list with type-specific formatting
  - Weighted: "Set 1: 60kg x 5 reps = 300 kg·vol"
  - Bodyweight: "Set 1: 10 reps"
  - Distance: "Set 1: 5 km"
  - Endurance: "Set 1: 30 min"
- **Notes** (if present, in yellow):
  - Warm-up text
  - Cool-down text
  - Session note

## Filtering

- `/` toggles filter mode
- Type to filter log list by practice name (case-insensitive, real-time)
- Emacs cursor keybindings in filter input
- `Esc` or `Enter` exits filter mode

## Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Browse | default | Navigate and view logs |
| ConfirmDelete | `d` | Overlay: "Delete [Practice] log from [Date]? [y]/[n]" |

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate log list |
| `/` | Toggle filter mode |
| `e` | Edit selected log (opens Log Entry in edit mode) |
| `d` | Delete selected log (enter confirm mode) |
| `u` | Undo last deletion |
| `y` | Confirm deletion |
| `n` | Cancel deletion |
| `?` | Help overlay |
| `Esc` | Clear filter (if filtering) or back to Dashboard |

## Undo

- One level of undo: stores the last deleted LogEntry
- `u` restores it via `restore_log(entry)` (recreates log + sets)
- Undo buffer cleared when a new deletion occurs

## Edit Flow

Pressing `e` on a selected log:
1. Opens Log Entry screen with `from_existing(log_entry)`
2. All fields pre-populated
3. On save, returns to History with refreshed list
