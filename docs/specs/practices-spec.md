# Practices Spec

Manage the practice inventory — add, rename, toggle visibility, and delete practices.

## Layout (top to bottom)

1. Title + column headers (Name | Type | Status)
2. Practice list (scrollable)
3. Action area (contextual input/confirmation, 0 or 6 lines)
4. Status line
5. Shortcuts bar

## List

- Shows all practices (active and inactive)
- Ordered by name
- Each row: marker (">"), name, type label, status ("active"/"inactive")
- Selected row highlighted in green

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

Creates practice immediately on type selection. Returns to Browse.

## Editing

`Enter` on a selected practice enters EditName mode:
- Pre-filled with current name
- Enter saves rename
- Esc cancels

## Toggle Active/Inactive

`t` toggles the selected practice's active flag. Inactive practices are hidden from all other screens (Log Entry, History, Trends, Dashboard) but remain visible here for reactivation.

## Deleting

`d` enters ConfirmDelete mode:
- Shows practice name in red
- `y` confirms — deletes practice and cascades to all its logs and sets
- `n` cancels

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate list (Browse), navigate type selector (AddType) |
| `a` | Add new practice |
| `Enter` | Edit practice name |
| `t` | Toggle active/inactive |
| `d` | Delete practice (confirm) |
| `y/n` | Confirm/cancel deletion |
| `?` | Help overlay |
| `Esc` | Back to Dashboard (or cancel input modes) |

## Text Input Constraints

Name input supports emacs cursor keybindings and horizontal scroll for long text.
