# Navigation & Input Spec

Global navigation patterns, keybinding conventions, and text input requirements.

## Screen Flow

```
                    Dashboard
                   /  |  |  \  \  \  \
                  /   |  |   \  \  \   \
           LogEntry  History Trends Practices Goals QuickLog Abbreviations
```

All screens return to Dashboard via `Esc`. History can navigate to LogEntry (edit mode), which returns to History.

## Global Keys

| Key | Action | Scope |
|-----|--------|-------|
| `Esc` | Back to previous screen / cancel current action | All screens |
| `Ctrl+C` | Force quit | Event loop |
| `?` | Toggle help overlay | All screens |

## Vim-Style Navigation

All list-based screens use consistent movement:

| Key | Action |
|-----|--------|
| `j` | Move down / next item |
| `k` | Move up / previous item |
| `h` | Left (time window shrink in Trends) |
| `l` | Right (time window expand in Trends) |
| `/` | Enter filter mode |
| `Enter` | Confirm / select / submit |

## Text Filter Pattern

Used in: Log Entry, History, Trends (practice selection).

**State:** filter active flag, filter text string, cursor position.

**Behavior:**
1. `/` enters filter mode (cursor appears in filter bar)
2. Typing filters the list in real-time (case-insensitive substring match)
3. `Enter` or `Esc` exits filter mode
4. Full emacs cursor keybindings in filter input

## Text Input Requirements

**Every text input field** must support:

| Key | Action |
|-----|--------|
| `Ctrl+B` or `Left` | Move cursor back one character |
| `Ctrl+F` or `Right` | Move cursor forward one character |
| `Ctrl+A` or `Home` | Move cursor to start of line |
| `Ctrl+E` or `End` | Move cursor to end of line |
| `Ctrl+K` | Delete from cursor to end of line |
| `Backspace` | Delete character before cursor |
| Character keys | Insert at cursor position |

**Rendering:** Text input must never overflow its container. Use horizontal scrolling to keep the cursor visible within the available width.

**Wrapping:** All variable-length text display (notes, descriptions, quotes) must wrap within container bounds. Only fixed-width UI elements (footers, shortcut bars) may omit wrapping.

## Confirmation Pattern

Destructive actions (delete) use a two-step confirmation:

1. Press `d` — enters confirm mode, shows "[y] Yes [n] No"
2. Press `y` to confirm or `n`/`Esc` to cancel

## Multi-Phase Input Pattern

Used in: Log Entry (4 phases), Trends (2 phases), Practices (add = 2 phases).

Each phase has its own render and key handler. Phase transitions:
- Forward: `Enter` or specific key (e.g., `Ctrl+S`)
- Backward: `Esc`

## Color Convention

| Role | Color |
|------|-------|
| Accent (titles, shortcut keys) | Cyan |
| Active/selected items | Green |
| Bright highlight | LightGreen |
| Labels, borders, disabled | DarkGray |
| Errors, delete actions | Red |
| Notes, secondary text | Yellow |

Uses ANSI terminal colors only (no hardcoded RGB) for theme compatibility.
