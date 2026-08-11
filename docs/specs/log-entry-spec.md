# Log Entry Spec

The Log Entry screen records or edits a training session through two top-level phases: practice selection and a unified entry form.

## Top-Level Flow

```text
SelectPractice -> EnterLog -> save
```

New logs return to Dashboard after a successful save. History emits `EditHistoryLog { log_id }`; target preparation resolves that exact stable database ID before constructing the edit form. Logs opened this way keep the original log identity and return to History after a successful update or confirmed cancellation.

## Practice Selection

The selector shows active practices in Name and Type columns above a reserved status row and active-mode shortcut footer.

- `j/k` or arrow keys move through matching practices and clamp at the first and last item.
- `/` enters filter mode; the shared single-line input filters names in real time, case-insensitively.
- `Enter` leaves filter mode while filtering, or selects the highlighted practice while navigating.
- `Esc` leaves filter mode while filtering, or returns to Dashboard while navigating.
- The selected practice is always revealed through `ViewportState`.
- The list title adds `↑`, `↓`, or `↑↓` when matching practices exist outside the visible viewport.
- Practice names, type labels, and columns are truncated to the actual inner width.

The footer shows navigation/filter/select/back only in navigation mode and confirm/cancel only in filter mode.

## Unified Entry Form

The form contains the log date, committed and active set fields, warm-up, cool-down, RPE, and note. `Tab` and `Shift+Tab` cycle the active field or section; the active input has a visible cursor and accent treatment.

### Set Fields

| Practice type | Fields | Enter behavior |
|---|---|---|
| Weighted | weight (kg), reps | Weight advances to reps; reps adds or confirms the set |
| Bodyweight | reps | Adds or confirms the set |
| Distance | distance (km) | Adds or confirms the set |
| Endurance | duration (min) | Adds or confirms the set |

For weighted practices, the previous set's weight carries forward. When a committed set is selected, `e` edits it and `d` deletes it. `j/k` navigates committed sets only when set navigation is active.

### Other Fields and Actions

- Warm-up, cool-down, RPE, and note are single-line inputs in the same form.
- `D` opens the single-line `YYYY-MM-DD` date editor while the Sets section is active.
- `Ctrl+S` saves only when at least one set exists and no set edit is active.
- `Esc` opens an explicit exit confirmation; confirmation returns to the form's origin without saving.

The footer is generated from the active state. It changes the `Enter` label between next/add/confirm, shows set navigation and edit/delete only when available, hides save for an empty log or active set edit, hides date outside Sets, and shows only confirm/cancel during date editing or exit confirmation. At `80x24`, every active command is rendered in complete key-label units.

## Text Input and Layout Contract

The practice filter, both set fields, warm-up, cool-down, RPE, note, and date editor use the shared `TextInput` state.

- `Ctrl+B/F/A/E/K`, Left, Right, Home, End, Backspace, and insertion at the cursor are supported.
- `TextInput::visible_spans` horizontally scrolls long Unicode values so the cursor remains visible.
- Inputs and inactive companion values are budgeted against the real inner rectangle.
- Variable practice names and form content remain inside their borders.
- The renderer does not panic for rectangles smaller than the application's `80x24` minimum; direct rendering at `24x10` is covered.

This is single-line input behavior. No multi-line input buffer is provided.

## Saving and Failure Retention

New entries call `create_log_at`; existing entries call `update_log`.

- Invalid date input keeps the date editor and exact value active with an error status.
- Invalid RPE keeps the complete form and current focus with an error status.
- A database create or update failure keeps every set and input, its cursor/focus, the editing log identity, and the Dashboard or History return target.
- Form state clears and navigation occurs only after the database mutation succeeds.
- Status always uses the reserved status row instead of replacing form content.

## Navigation Lifecycle

Log Entry is request-scoped rather than an ordinary cached module re-entry: `NewLog` creates a fresh form, while `EditHistoryLog { log_id }` creates a form from the exact requested record. There is no application-side read of History selection during target preparation.

If preparing Log Entry or its Dashboard/History return target fails, the route and cached target remain unchanged. A failed return leaves the complete current form, cursor, focus, edit identity, and return target in place and shows the navigation error on Log Entry. The existing Dashboard/History routes and shortcuts are unchanged.
