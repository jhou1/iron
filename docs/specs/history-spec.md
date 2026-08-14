# History Spec

Browse, inspect, edit, and delete training logs.

## Layout

The History screen keeps the log list as the primary surface and opens the selected log in a floating detail panel. The panel is a read-only preview during Browse and becomes a focused, full-height viewport after `Enter`.

`ScreenFrame` reserves the filter, list, status, and wrapped shortcut rows in one composition pass. `ScrollableListPanel` owns the History list border, fixed header, overflow suffix, logical-row viewport, visible slicing, empty state, and selected-row highlight. History supplies already-formatted date/practice/RPE/metric lines and retains filtering, popup placement, Detail focus, editing, delete/undo, database, and re-entry policy. The floating and focused detail remains integrated through `render_detail_panel` rather than a generic popup component.

### Screen rows (top to bottom)
1. Title + filter bar
2. Log list (scrollable)
3. Status line
4. Shortcuts bar

The list and detail titles append `↑`, `↓`, or `↑↓` when content exists beyond the current viewport.

## Log List

- Data source: all logs, filtered to active practices, ordered by date descending
- Each row shows: date ("YYYY MMM DD"), practice name, total metric with unit label
- Selected row highlighted in green

## Detail Panel

For the currently selected log, the preview and focused detail show:

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

Press `Enter` to focus the detail panel. In focused detail, `j/k` or the arrow keys scroll by visual row so wrapped warm-up, cool-down, and note text remain reachable. `Esc` returns focus to the list without changing its selection.

## Filtering

- `/` toggles filter mode
- Type to filter log list by practice name (case-insensitive, real-time)
- Emacs cursor keybindings in filter input
- Long filter text scrolls horizontally and keeps the cursor visible inside the filter row
- `Esc` or `Enter` exits filter mode

## Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Browse | default | Navigate and view logs |
| Detail | `Enter` | Focus and scroll the complete selected log detail |
| ConfirmDelete | `d` | Overlay: "Delete [Practice] log from [Date]? [y]/[n]" |

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate log list |
| `Enter` | Focus selected log detail |
| `j/k`, arrows | Scroll while detail is focused |
| `/` | Toggle filter mode |
| `e` | Edit selected log (opens Log Entry in edit mode) |
| `d` | Delete selected log (enter confirm mode) |
| `u` | Undo last deletion |
| `y` | Confirm deletion |
| `n` | Cancel deletion |
| `?` | Help overlay |
| `Esc` | Exit filter/detail focus, clear an active filter, or return to Dashboard |

## Undo

- One level of undo: stores the last deleted LogEntry
- `u` restores it via `restore_log(entry)` (recreates log + sets)
- Undo buffer cleared when a new deletion occurs

## Edit Flow

Pressing `e` on a selected log:
1. Emits an edit intent containing the selected log's stable database ID
2. Resolves that exact ID and opens a pre-populated Log Entry form
3. Keeps the edit identity and History return target inside the form
4. On save, returns to History with refreshed data and the same filtered selected log when it still exists
5. On cancel, returns to History with the same filtered selected log; a later edit reloads the latest saved data

## Re-entry

Opening History reloads current logs before the route changes. It preserves filter text, restores the selected log by stable ID when possible, retains bounded list/detail viewports, and keeps Detail focus only when a selected log remains. Filter-input mode, delete confirmation, old status, and the cross-page undo payload clear.

This cross-page lifecycle is distinct from History's in-place delete/undo refresh, which retains the operation's local context. If target preparation fails, navigation stays on the source screen, the cached History screen is unchanged, and the source shows the error.
