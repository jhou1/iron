# IronCLI — Training Record Tracker

A beautiful terminal UI application for tracking training (workout) records, built in Rust with a Claude Code-inspired aesthetic.

## Overview

IronCLI (`iron`) is an interactive TUI app that lets you log training practices, view a GitHub-style activity heatmap, browse history, and track progress trends via sparkline charts. Data is stored locally in SQLite. The term "practice" is used instead of "exercise" — every training session is a practice.

## Practice Types

Four built-in types. When adding a new practice to the inventory, the user selects one of these:

| Type | UI Label | Fields per set | Derived metric |
|---|---|---|---|
| **Weighted** | weightxreps | weight (kg), reps | volume = sum of (weight x reps) per set |
| **Bodyweight** | reps | reps | total_reps = sum of reps per set |
| **Distance** | distance | distance (km) | total_distance = sum of distance per set |
| **Endurance** | duration | duration (min) | total_duration = sum of duration per set |

**Units are fixed:** kg, km, minutes. No unit conversion.

## Data Model

Three tables in SQLite (`~/.ironcli/iron.db`):

### `practices`
| Column | Type | Description |
|---|---|---|
| id | INTEGER PK | Auto-increment |
| name | TEXT NOT NULL UNIQUE | Practice name (e.g., "Kettlebell Snatch") |
| practice_type | TEXT NOT NULL | One of: weighted, bodyweight, distance, endurance |
| created_at | TEXT NOT NULL | ISO 8601 timestamp |

### `logs`
| Column | Type | Description |
|---|---|---|
| id | INTEGER PK | Auto-increment |
| practice_id | INTEGER NOT NULL | FK → practices.id |
| logged_at | TEXT NOT NULL | ISO 8601 timestamp |
| note | TEXT | Optional free-text note about the session |

### `sets`
| Column | Type | Description |
|---|---|---|
| id | INTEGER PK | Auto-increment |
| log_id | INTEGER NOT NULL | FK → logs.id |
| set_number | INTEGER NOT NULL | Ordinal position (1, 2, 3...) |
| weight | REAL | kg — nullable, used by weighted type |
| reps | INTEGER | nullable, used by weighted + bodyweight |
| distance | REAL | km — nullable, used by distance type |
| duration | REAL | minutes — nullable, used by endurance type |

Derived metrics (volume, total reps, total distance, total duration) are computed at query time, not stored.

## TUI Structure

### Screen Map

```
Dashboard (home)
  [l] → Log Entry (guided form)
  [h] → History (14-day log)
  [t] → Trends (sparkline per practice)
  [e] → Practices (inventory management)
  [q] → Quit

Log Entry
  Select practice (j/k to navigate, / to filter)
  Enter sets one at a time (weight/reps carry forward)
  [Enter] → add next set
  [Ctrl+S] → save & finish
  [d] → delete last set
  [Esc] → cancel, back to dashboard

History
  Scrollable 14-day log (j/k)
  [Enter] on an entry → edit
  [d] → delete entry
  [Esc] → back

Trends
  Select practice (j/k, / to filter)
  Sparkline bar chart of derived metric over time
  [h/l] → scroll time window
  [Esc] → back

Practices
  List all practices (j/k)
  [a] → add new practice (name + pick type)
  [Enter] → edit name
  [d] → delete practice
  [Esc] → back
```

### Color Scheme

Use ANSI terminal colors instead of hardcoded RGB values so the app adapts to any terminal theme (dark, light, custom backgrounds).

| Role | Color |
|---|---|
| Accent (titles, shortcuts, headers) | Cyan |
| Active/selected items, logged entries | Green |
| Bright highlight (3+ sessions) | LightGreen |
| Primary text | White |
| Labels, dimmed text, borders | DarkGray |
| Error, delete prompts | Red |
| Notes | Yellow |

**Heatmap cells:**

| Sessions | Character | Color |
|---|---|---|
| 0 (empty) | `▪` (small square, U+25AA) | DarkGray |
| 1 | `■` (black square, U+25A0) | Indexed(65) muted teal |
| 2 | `■` | Indexed(71) medium green |
| 3+ | `■` | Indexed(118) bright lime |

### Navigation

Vim-style throughout: `j/k` for up/down, `h/l` for left/right or back/forward, `/` for filtering, `Esc` to go back, `Enter` to confirm.

### Text Input (Emacs-Style Cursor)

All text input fields (goal/milestone names, practice names, log notes, date inputs) support emacs-style cursor navigation:

| Key | Action |
|---|---|
| `Ctrl+B` / `Left` | Move cursor back one character |
| `Ctrl+F` / `Right` | Move cursor forward one character |
| `Ctrl+A` / `Home` | Move cursor to beginning of line |
| `Ctrl+E` / `End` | Move cursor to end of line |
| `Backspace` | Delete character before cursor |

Characters are inserted at the cursor position, not appended at the end. The cursor is shown as a block character (`█`) at the current position. This applies to all screens: dashboard goals, practices, and log entry notes.

### Dashboard Layout

The home screen is organized as a vertical stack of fixed-width sections, all aligned to the heatmap content width (107 chars: 3-char day labels + 52 weeks × 2 chars each):

```
┌─────────────────────────────────────────────┐
│ Title bar ("iron v0.1.1")                   │
│ ASCII art header (6-line "Training Activity"│
│   rendered in Cyan)                         │
│ Heatmap (52-week grid, 7 day rows,          │
│   month labels above, legend below)         │
│ ┌──── Last 14 Days ────┬── Statistics ────┐ │
│ │ Recent log entries    │ Sessions, volume │ │
│ │ (dynamic height)      │ reps, distance,  │ │
│ │                       │ duration         │ │
│ └───────────────────────┴─────────────────┘ │
│ (spacer absorbs excess vertical space)      │
│ Footer (keyboard shortcut hints)            │
└─────────────────────────────────────────────┘
```

- **Title bar:** App name and version.
- **ASCII art header:** A 6-line stylized "Training Activity" banner rendered in Cyan.
- **Heatmap:** GitHub-style contribution grid spanning 52 weeks. Rows = days of the week (Mon–Sun). Month labels above. Legend row below ("Less ▪ ■ ■ ■ More").
- **Bottom-left pane:** "Last 14 Days" — lists all log entries from the past 14 days with date, practice name, set count, and derived metric. Pane height adapts dynamically to the number of entries (minimum 7 rows for the stats pane).
- **Bottom-right pane:** "Statistics" — aggregated 14-day stats: sessions, volume (kg), reps, distance (km), duration (min).
- **Width alignment:** The heatmap and both bottom panes share the same maximum content width and left margin (1-char indent), so their edges are visually aligned regardless of terminal width.
- **Footer:** Keyboard shortcut hints pinned to the bottom.

### Log Entry Flow

1. Select a practice from the inventory using a filterable list (j/k, / to search).
2. Enter sets one at a time. The log date is shown at the top, defaulting to today. Press `D` (shift+d) to edit the date in YYYY-MM-DD format. Each set prompts for the fields relevant to the practice type:
   - Weighted: weight (kg), reps
   - Bodyweight: reps
   - Distance: distance (km)
   - Endurance: duration (min)
3. After entering a set, press Enter to add the next one. Weight (for weighted type) carries forward from the previous set — just press Enter to keep it, or type a new value.
4. A running total is displayed live (sets count + derived metric).
5. Press Ctrl+S to finish adding sets. A note prompt appears — type a free-text note (e.g., "I feel great in this session") or press Enter to skip.
6. Press Enter to save. Press Esc to cancel.

When editing an existing log from History, the date is pre-populated from the original log and can be changed with `D`.

### Trend View

Sparkline bar chart showing the derived metric (volume, total reps, total distance, or total duration) per log over time for a selected practice. The x-axis uses two rows: the top row shows day-of-month (e.g., "05", "18") for every bar, and the bottom row shows the month abbreviation (e.g., "Jan", "Feb") in cyan only at month boundaries. Summary stats below: average, peak, and trend percentage. Navigate time window with h/l.

## CRUD Operations

Full create, read, update, delete on entries:

- **Create:** Log Entry flow (guided form with set-by-set input)
- **Read:** Dashboard (today + 14-day stats), History (scrollable log), Trends (sparkline)
- **Update:** Select a log in History, press Enter to re-open the set-by-set editor. Existing sets are shown and can be modified, deleted, or appended to.
- **Delete:** Select a log in History, press d to delete the entire log and its sets (with confirmation prompt).

Practice inventory also supports full CRUD via the Practices screen.

## Export / Import

### `iron export [path]`

Dumps all data to JSON. Defaults to `~/.ironcli/iron-export-YYYY-MM-DD.json`.

```json
{
  "version": 1,
  "exported_at": "2026-04-16T10:30:00Z",
  "practices": [
    { "id": 1, "name": "Kettlebell Snatch", "type": "weighted", "created_at": "2026-04-01" }
  ],
  "logs": [
    {
      "id": 1,
      "practice": "Kettlebell Snatch",
      "logged_at": "2026-04-16T09:30:00",
      "note": "I feel great in this session",
      "sets": [
        { "set_number": 1, "weight": 24, "reps": 10 },
        { "set_number": 2, "weight": 24, "reps": 9 },
        { "set_number": 3, "weight": 28, "reps": 11 }
      ]
    }
  ]
}
```

- Uses practice names instead of IDs for human readability
- `version` field for future schema migrations

### `iron import <path>`

Reads a JSON file and merges into the database. Skips duplicates based on practice name + timestamp.

## Technology Stack

| Concern | Crate |
|---|---|
| TUI framework | `ratatui` + `crossterm` |
| SQLite | `rusqlite` (with `bundled` feature) |
| Serialization | `serde` + `serde_json` |
| Date/time | `chrono` |
| CLI entry point | `clap` |

## Module Structure

```
src/
  main.rs          — CLI entry point (clap), launches TUI or runs export/import
  app.rs           — app state, screen routing, event loop
  db.rs            — SQLite schema, queries, migrations
  model.rs         — Practice, Log, Set, PracticeType structs
  tui/
    dashboard.rs   — ASCII art header + heatmap + last-14-days + stats
    log_entry.rs   — guided form with set-by-set input
    history.rs     — 14-day scrollable log
    trends.rs      — sparkline chart per practice
    practices.rs   — inventory CRUD
    widgets/
      heatmap.rs   — GitHub-style heatmap widget
      sparkline.rs — bar chart widget
  export.rs        — JSON export/import logic
```

## Distribution

- `cargo install ironcli`
- Prebuilt binaries for macOS/Linux
- Single binary, zero runtime dependencies (SQLite bundled)

## Future Considerations (Not in Scope)

These were mentioned during brainstorming as potential future additions but are explicitly out of scope for the initial build:

- Quick entry mode: parse `"kb snatch 24kg 10x10"` as a one-liner
- Template-based logging: pick from recent/favorite practices and repeat
- Calendar view: monthly calendar with training data
- Cloud sync
