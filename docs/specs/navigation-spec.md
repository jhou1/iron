# Navigation & Input Spec

Global navigation patterns, keybinding conventions, and shared TUI interaction requirements.

## Screen Flow

```text
                    Dashboard
              /    /   |   \    \    \
      LogEntry History Trends Practices Goals Quotes
                   \
                    LogEntry (edit existing log)
```

Dashboard reaches Log Entry (`l`), History (`h`), Trends (`t`), Practices (`e`), Goals (`g`), and Quotes (`Q`). There is no Quick Log screen, route, or shortcut and no `w` action. Feature-specific screens return to Dashboard via `Esc`; Log Entry opened from History returns to History after a successful edit.

## Global Keys

| Key | Action | Scope |
|---|---|---|
| `Esc` | Back, cancel, or enter the current screen's explicit exit flow | Screen-owned |
| `Ctrl+C` | Force quit | Application event loop |

## List Navigation and Overflow

List screens explicitly choose clamp or wrap edge behavior.

| Screen/list | Edge behavior |
|---|---|
| Log Entry practice selector | Clamp |
| Goals, History, Quotes, Practices | Clamp |
| Trends | Wrap |

`j/k` navigates lists, with arrow-key support where implemented by the screen. The selected item or selected visual span remains visible. When visual rows exist outside a viewport, the panel title adds `↑`, `↓`, or `↑↓`. History and Quotes use `Enter` to focus the complete selected detail and `Esc` to return focus to the list.

## Text Filter Pattern

Log Entry, History, and Trends filter practice names with a shared `TextInput` value and cursor.

1. `/` enters filter mode and reveals the cursor.
2. Typing applies a case-insensitive substring filter in real time.
3. `Enter` or `Esc` exits filter mode.
4. The filter footer advertises only the actions active in filter mode.

## Single-Line Text Input

Every single-line input uses `tui::framework::TextInput` and supports:

| Key | Action |
|---|---|
| `Ctrl+B` or `Left` | Move cursor back one character |
| `Ctrl+F` or `Right` | Move cursor forward one character |
| `Ctrl+A` or `Home` | Move cursor to start of line |
| `Ctrl+E` or `End` | Move cursor to end of line |
| `Ctrl+K` | Delete from cursor to end of line |
| `Backspace` | Delete character before cursor |
| Character keys | Insert at cursor position |

`TextInput::visible_spans` measures display width, clips to the real inner rectangle, and horizontally scrolls long Unicode text to keep the cursor visible. This contract is single-line only; Iron does not provide a multi-line input buffer.

Variable-length display content wraps or truncates inside its container. Shortcut bars wrap only at complete key-label boundaries and reserve every required row, so active commands do not overwrite content.

## Mode, Confirmation, and Mutation Rules

Screens own their modes and expose shortcuts only for commands active in the current mode. Examples include filter confirmation, Log Entry set/date/exit states, Dashboard HRV input, and Dashboard quit confirmation.

Destructive actions use an explicit confirmation before mutation. Modes and inputs clear only after the required database mutation and reload succeed. Validation and mutation failures keep the relevant value, Unicode cursor, focus, selection or return target and render the error in a reserved status row.

## Framework Ownership and Supported Layouts

Reusable mechanics live in `src/tui/framework/`: visual-row measurement and viewport state, shared single-line input, bounded rectangle helpers, status and shortcut components, and TestBackend support. Screens continue to own domain data, database calls, validation, and mode transitions.

- Batch 1 list/detail screens and Log Entry are verified at `80x24`.
- Dashboard's complete content and remaining navigation are verified at `80x35`.
- Log Entry and Dashboard direct renderers tolerate smaller rectangles without panicking or drawing derived inner regions outside their parents.

## Removed Interfaces

Quick Log was removed permanently without a replacement. Its LLM configuration/runtime, Abbreviations models and CRUD API, locale and dependency interfaces, and application route no longer exist. Database initialization drops the obsolete `abbreviations` table while preserving unrelated data.

## Color Convention

| Role | Color |
|---|---|
| Accent (titles, shortcut keys) | Cyan |
| Active/selected items | Green |
| Bright highlight | LightGreen |
| Labels, borders, disabled | DarkGray |
| Errors, delete actions | Red |
| Notes, secondary text | Yellow |

Iron primarily uses ANSI terminal colors. The shared selected-row highlight intentionally uses `Rgb(204, 163, 0)` with a Black foreground, and empty gauge cells use `Indexed(240)`.
