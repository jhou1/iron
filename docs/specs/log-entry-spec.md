# Log Entry Spec

Multi-phase form for recording a training session. Guides the user through practice selection, set-by-set data entry, optional warm-up/cool-down notes, and a session note.

## Phases

```
SelectPractice -> EnterSets -> EnterWarmUpCoolDown -> EnterNote -> [save]
```

Each phase has its own layout and keybindings. `Esc` moves backward through phases (or exits the screen from phase 1).

## Phase 1: SelectPractice

**Purpose:** Choose which practice to log.

**Display:**
- Title: "Log practice"
- Filter bar with text input
- Table: Name | Type columns
- Only active practices shown

**Behavior:**
- `j/k` — navigate practice list
- `/` — toggle filter mode (type to filter by name, case-insensitive)
- `Enter` — select practice, advance to Phase 2
- `Esc` — exit to Dashboard

**Filtering:** Standard text filter with emacs cursor keybindings. Filters practice list in real-time as user types.

## Phase 2: EnterSets

**Purpose:** Add sets one at a time with type-specific fields.

**Display:**
- Title: "Log sets for [Practice Name]"
- Current log date (YYYY-MM-DD) with edit hint
- Table of entered sets: Set# | value(s)
- Active input field(s) for next set

**Input fields by practice type:**

| Type | Fields | Flow |
|------|--------|------|
| Weighted | weight (kg), reps | Tab between fields, Enter adds set |
| Bodyweight | reps | Enter adds set |
| Distance | distance (km) | Enter adds set |
| Endurance | duration (min) | Enter adds set |

**Weight carry-forward:** For weighted practices, the weight from the previous set pre-fills the next set's weight field. User can accept (Enter) or overtype.

**Keys:**
- `Tab` — switch between weight/reps fields (weighted only)
- `Enter` — confirm field value / add set
- `Ctrl+S` — finish sets, advance to Phase 3
- `D` — edit log date (opens date input overlay, format YYYY-MM-DD)
- `d` — delete last set (only when input fields are empty)
- `Esc` — cancel, return to origin screen

## Phase 3: EnterWarmUpCoolDown

**Purpose:** Optional warm-up and cool-down notes.

**Display:**
- Two text input fields, vertically stacked
- Active field highlighted
- Labels: "Warm-up:" and "Cool-down:"

**Behavior:**
- `j/k` or `Tab` — switch between fields
- `Enter` — confirm field, advance (warm-up -> cool-down -> Phase 4)
- Emacs cursor keybindings in both fields
- `Esc` — back to Phase 2

## Phase 4: EnterNote

**Purpose:** Optional free-text session note.

**Display:**
- Single text input field
- Label: "Note:"

**Behavior:**
- `Enter` — save log to database
- `Esc` — back to Phase 3

## Saving

On save:
- **New log:** `create_log_at(practice_id, logged_at, sets, note, warm_up, cool_down)`
- **Editing existing:** `update_log(log_id, sets, note, logged_at, warm_up, cool_down)` — deletes old sets, inserts new ones

Returns to Dashboard (new log) or History (editing).

## Editing Mode

Constructed with `from_existing(log_entry)`:
- Skips Phase 1 (practice already known)
- Pre-fills all fields from existing log data
- Starts in Phase 2
- On save, updates instead of creating
- Returns to History screen

## Text Input Constraints

All text fields must support:
- Emacs cursor: Ctrl+B/F/A/E/K, Left, Right, Home, End, Backspace
- Horizontal scroll for long text (never overflow container)
- Characters insert at cursor position
