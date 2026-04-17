# Dashboard Enhancements — Goals, Milestones & Daily Quote

Adds three enhancements to the IronCLI dashboard: merged statistics into the Last 14 Days pane, a Goals & Milestones pane replacing the Statistics pane, and a daily rotating motivational quote.

## Dashboard Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Title bar ("iron v0.1.3")                                       │
│ ASCII art header (6-line banner, Cyan)                          │
│ Heatmap (52-week grid, 7 day rows, month labels, legend)        │
│ Daily quote (1 line, Yellow)                                    │
│ ┌── Last 14 Days (merged) ──────────┬──── Goals ──────────────┐ │
│ │ 11 sessions · 11047 kg · 525 reps │ ▸ Master KB Sport       │ │
│ │ ────────────────────────────────── │   ☐ 10-min snatch set  │ │
│ │ Apr 16 Kettlebell Snatch  5s 2260 │   ☑ First competition  │ │
│ │ Apr 16 Kettlebell Snatch  2s  800 │ ▸ Run a half marathon  │ │
│ │ Apr 15 Long Cycle         5s 1000 │   ☐ Run 10km under 50m │ │
│ │ ...                               │                        │ │
│ └───────────────────────────────────┴────────────────────────┘ │
│ Footer (keybindings)                                            │
└─────────────────────────────────────────────────────────────────┘
```

All sections (heatmap, quote, panes) share the same maximum content width (107 chars) and left margin (1-char indent) for visual alignment.

## Feature 1: Merge Statistics into Last 14 Days

The current Statistics pane is removed. Its data becomes a compact one-line summary at the top of the "Last 14 Days" pane.

### Summary line format

```
{sessions} sessions · {volume} kg · {reps} reps
```

Only non-zero metrics are included. If distance or duration stats are non-zero, they are appended:

```
11 sessions · 11047 kg · 525 reps · 5.2 km · 13 min
```

The summary line is styled in Green (matching current stat values). A horizontal separator (DarkGray) sits between the summary and the entry list below.

### Entry list

Unchanged from current behavior — each entry shows date, practice name, set count, and derived metric.

## Feature 2: Goals & Milestones

A new Goals pane replaces the Statistics pane on the right side of the dashboard. Goals are freeform text with hierarchical milestones underneath.

### Data Model

Two new SQLite tables:

```sql
CREATE TABLE goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE milestones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_id INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
```

- `position` tracks display order within its parent (goals are ordered globally, milestones within their goal).
- `completed` is 0 (pending) or 1 (done).
- Deleting a goal cascades to its milestones.

### Display

- Goals shown with `▸` prefix in White, bold.
- Pending milestones indented with `  ☐` prefix in White.
- Completed milestones indented with `  ☑` prefix in DarkGray (dimmed).
- The selected item (when in Goals mode) is highlighted in Green.
- The pane scrolls if content exceeds available height.
- When no goals exist, show "Press [g] to add goals" in Gray.

### Keyboard Interaction

The Goals pane is passive by default. Pressing `g` on the dashboard enters Goals editing mode, which focuses the right pane.

While in Goals mode:

| Key | Action |
|---|---|
| `j` / `k` | Navigate down/up through goals and milestones |
| `a` | Add a new goal (inline text input) |
| `m` | Add a milestone under the selected goal, or under the parent goal if a milestone is selected (inline text input) |
| `Enter` | Edit the selected goal or milestone text (inline text input) |
| `Space` | Toggle milestone completion (☐ ↔ ☑) |
| `d` | Delete selected goal (with all milestones) or single milestone, with confirmation |
| `Esc` | Exit Goals mode, return to passive dashboard |

When in Goals mode, the footer switches to show Goals-specific keybindings instead of the dashboard navigation keys.

### Navigation model

Goals and milestones form a flat navigation list for j/k purposes:

```
▸ Goal 1          ← index 0
  ☐ Milestone A   ← index 1
  ☑ Milestone B   ← index 2
▸ Goal 2          ← index 3
  ☐ Milestone C   ← index 4
```

The selected index determines which item receives actions (edit, delete, toggle, add milestone under).

## Feature 3: Daily Rotating Quote

A single-line motivational quote displayed between the heatmap and the bottom panes.

### Quote source

1. **Built-in list:** ~30 hardcoded training/discipline quotes bundled in the binary.
2. **User override:** If `~/.ironcli/quotes.txt` exists and is non-empty, its contents replace the built-in list entirely. One quote per line. Empty lines are ignored.

### Quote selection

Deterministic based on the current date: `day_of_year % quote_count`. The same quote shows all day and changes the next day.

### Display

- Rendered as a single `Line` widget, styled in Yellow.
- Prefixed with `"  "` (2-space indent) for visual alignment with heatmap content.
- If the quote exceeds the available width, it is truncated (no wrapping).

### Built-in quotes (sample)

```
"The only bad workout is the one that didn't happen."
"Discipline is choosing between what you want now and what you want most."
"The pain you feel today will be the strength you feel tomorrow."
"Don't count the days, make the days count."
"Success isn't always about greatness. It's about consistency."
"The body achieves what the mind believes."
"Fall seven times, stand up eight."
"You don't have to be extreme, just consistent."
"Train insane or remain the same."
"Strength does not come from the body. It comes from the will."
```

The full list of ~30 quotes will be defined at implementation time.

## Database Changes

- Add `goals` and `milestones` tables (schema above).
- New Database methods:
  - `list_goals() -> Vec<Goal>` (with milestones loaded)
  - `create_goal(title) -> i64`
  - `update_goal(id, title)`
  - `delete_goal(id)` (cascades milestones)
  - `create_milestone(goal_id, title) -> i64`
  - `update_milestone(id, title)`
  - `toggle_milestone(id)` (flips completed 0↔1)
  - `delete_milestone(id)`

## New Model Types

```rust
pub struct Goal {
    pub id: i64,
    pub title: String,
    pub position: i32,
    pub created_at: NaiveDateTime,
    pub milestones: Vec<Milestone>,
}

pub struct Milestone {
    pub id: i64,
    pub goal_id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i32,
    pub created_at: NaiveDateTime,
}
```

## Dashboard Keybindings (Updated)

| Key | Action |
|---|---|
| `l` | Navigate to Log Entry screen |
| `h` | Navigate to History screen |
| `t` | Navigate to Trends screen |
| `e` | Navigate to Practices screen |
| `g` | Enter Goals editing mode |
| `q` | Quit |

## Export/Import

Goals and milestones are included in JSON export/import:

```json
{
  "version": 2,
  "goals": [
    {
      "title": "Master KB Sport",
      "position": 1,
      "milestones": [
        { "title": "Complete 10-min snatch set", "completed": false, "position": 1 },
        { "title": "Compete in first competition", "completed": true, "position": 2 }
      ]
    }
  ],
  "practices": [...],
  "logs": [...]
}
```

The export version bumps to 2. Version 1 imports (without goals) remain compatible.

## Scope Exclusions

- No automatic progress computation for goals.
- No personal records or PR tracking.
- No visual polish changes beyond what's described here.
- No changes to History, Trends, Log Entry, or Practices screens.
