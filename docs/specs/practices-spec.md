# Practices Spec

Manage the practice inventory — add, rename, toggle visibility, and delete practices.

## Layout (top to bottom)

1. Title + column headers (Name | Type | Status)
2. Practice list (scrollable)
3. Action area (contextual input/confirmation, 0 or 6 lines)
4. Status line
5. Shortcuts bar

## Presentation Ownership

Batch 3B-1 composes Practices from three presentation-only components. `ScreenFrame` reserves the status and shortcut rows. `ScrollableListPanel` owns the list border, overflow markers, viewport reveal and clamp, and selected-row highlight. `ActionPanel` owns bounded rendering and highlighting for contextual action rows.

Practices still formats every domain-specific line and makes every state decision. It owns practice columns and status glyphs, action copy, modes, selection decisions, shortcuts, validation, database mutations, and lifecycle behavior.

## List

- Shows all practices (active and inactive)
- Ordered by name
- Each row: marker (">"), name, type label, status ("active"/"inactive")
- Selected row highlighted in green
- Uses the actual bordered inner height at every terminal size; selection clamps at the first and last item and remains visible
- The title shows `↑`, `↓`, or `↑↓` when rows exist outside the viewport
- Long name and type labels are truncated by terminal display width so the status toggle remains inside the panel

## Modes

| Mode | Trigger | Display |
|------|---------|---------|
| Browse | default | List navigation |
| AddName | `a` | Text input: "Name:" |
| AddType | Enter in AddName | Type selector (4 options) |
| EditName | `Enter` on practice | Text input pre-filled with current name |
| ConfirmDelete | `d` | "Delete [name]? [y] Yes [n] No" |

## Adding a Practice

Two-step flow:
1. **AddName:** Type practice name, Enter to confirm
2. **AddType:** Select from weighted/bodyweight/distance/endurance using j/k + Enter

Creates the practice on type confirmation. The screen returns to Browse and clears input only after creation and list refresh both succeed. A failure keeps AddType and the entered name available.

## Editing

`Enter` on a selected practice enters EditName mode:
- Pre-filled with current name
- Enter saves rename
- Esc cancels
- Empty or duplicate names and database/refresh errors keep EditName, the text, and the visible error status

## Toggle Active/Inactive

`Space` toggles the selected practice's active flag. Inactive practices are hidden from all other screens (Log Entry, History, Trends, Dashboard) but remain visible here for reactivation.

## Deleting

`d` enters ConfirmDelete mode:
- Shows practice name in red
- `y` confirms — deletes practice and cascades to all its logs and sets
- `n` cancels
- A failed delete keeps ConfirmDelete open and shows the error in the reserved status row

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate list (Browse), navigate type selector (AddType) |
| `a` | Add new practice |
| `e` | Edit practice name |
| `Space` | Toggle active/inactive |
| `d` | Delete practice (confirm) |
| `y/n` | Confirm/cancel deletion |
| `?` | Help overlay |
| `Esc` | Back to Dashboard (or cancel input modes) |

## Text Input Constraints

Name input uses the shared Unicode-safe Emacs keybindings and horizontal scrolling. It never overflows the action panel. Every mode renders only its active shortcuts through the shared shortcut component.

## Re-entry

Opening Practices reloads all current practices before the route changes. It restores the selected practice by stable ID when possible and preserves a bounded list viewport. Cross-page re-entry always resumes Browse and clears unfinished add/name/type/edit/delete-confirmation state, input, and old status.

This reset does not apply to an in-place mutation failure: failed add, rename, or delete operations retain their active mode and input as described above. If cross-page preparation fails, navigation stays on the source screen, the cached Practices screen is unchanged, and the source shows the error.
