# Abbreviations Spec

Manage a dictionary of shorthand mappings used by Quick Log to expand abbreviated practice names.

## Layout (top to bottom)

1. Title
2. Abbreviation list (scrollable)
3. Action area (0 or 4 lines, contextual)
4. Status line
5. Shortcuts bar

## List

Each row shows:
- Marker: ">" if selected
- Short form (bold/green if selected)
- " -> "
- Full name

Ordered alphabetically by short form.

## Modes

| Mode | Trigger | Display |
|------|---------|---------|
| Browse | default | List navigation |
| AddShort | `a` | Text input: "Short form:" |
| AddFull | Enter in AddShort | Text input: "Full name:" |
| EditShort | `e`/`Enter` | Text input pre-filled with current short |
| EditFull | Enter in EditShort | Text input pre-filled with current full name |
| ConfirmDelete | `d` | "Delete [short]? [y] Yes [n] No" |

## Adding

Two-step flow:
1. Type short form, Enter
2. Type full name, Enter

Creates abbreviation immediately. Short form must be unique (case-insensitive).

## Editing

Two-step flow:
1. Edit short form (pre-filled), Enter
2. Edit full name (pre-filled), Enter

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate list |
| `a` | Add new abbreviation |
| `e` or `Enter` | Edit selected abbreviation |
| `d` | Delete (confirm) |
| `y/n` | Confirm/cancel deletion |
| `?` | Help overlay |
| `Esc` | Back to Dashboard (or cancel input) |

## Text Input Constraints

All text fields support emacs cursor keybindings and horizontal scroll.
