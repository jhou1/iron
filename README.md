# IronCLI

A beautiful terminal UI for tracking your training records. Log practices set-by-set, visualize consistency with a GitHub-style heatmap, and track progress with sparkline charts.

## Features

- GitHub-style **heatmap** showing your training consistency over the year
- **Set-by-set logging** — each set can have different weight/reps, with weight carry-forward
- **Sparkline trend charts** per practice with avg, peak, and trend percentage
- **14-day history** with inline set details and session notes
- **4 practice types**: weighted (weightxreps), bodyweight (reps), distance (km), endurance (min)
- **Vim-style navigation** throughout (j/k, h/l, /, Esc, Enter)
- **Goals and milestones** — set training targets and track progress
- **Daily motivational quote** on the Dashboard, customizable via `~/.ironcli/quotes.txt`
- **JSON export/import** for backup and data portability
- Data stored locally in SQLite — single file, zero cloud dependencies

## Installation

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (1.70+)

### Build from source

```bash
git clone <repo-url>
cd ironcli
cargo build --release
```

The binary is at `target/release/iron`. Copy it to your PATH:

```bash
cp target/release/iron ~/.local/bin/
```

Or install directly via Cargo:

```bash
cargo install --path .
```

## Usage

Launch the app:

```bash
iron
```

You'll land on the **Dashboard** — the home screen showing your training heatmap and today's stats.

### First Time Setup

1. Press `e` to open the **Practices** screen
2. Press `a` to add a new practice
3. Type a name (e.g., "Kettlebell Snatch") and press Enter
4. Select the practice type with `j/k` and press Enter:
   - **weightxreps** — for weighted exercises (tracks kg and reps)
   - **reps** — for bodyweight exercises (tracks reps only)
   - **distance** — for running, cycling, etc. (tracks km)
   - **duration** — for holds, planks, etc. (tracks minutes)
5. Press `Esc` to return to the Dashboard

### Logging a Practice

1. Press `l` to open the **Log Entry** screen
2. Type to filter practices, or use `j/k` to navigate, then press Enter
3. Enter sets one at a time:
   - For weighted: type weight, press Enter or Tab, type reps, press Enter
   - Weight carries forward from the previous set — just press Enter to keep it
   - For other types: type the value and press Enter
4. Press `D` to change the log date (defaults to today, format: YYYY-MM-DD)
5. Press `Ctrl+S` when done adding sets
6. Type an optional note (e.g., "Felt strong today") or press Enter to skip
7. Press Enter to save

Example — logging kettlebell snatches:

```
Set 1: 24kg x 10      (type 24, Enter, 10, Enter)
Set 2: 24kg x 9       (Enter to keep 24kg, 9, Enter)
Set 3: 28kg x 8       (type 28, Enter, 8, Enter)
Ctrl+S → type note → Enter
```

### Viewing History

Press `h` to see your **last 14 days** of training. Navigate with `j/k` to see set details for each log.

- `Enter` — edit a log (re-opens the set editor with existing data)
- `d` — delete a log (with confirmation)
- `Esc` — back to Dashboard

### Viewing Trends

Press `t` to see **progress charts**. Select a practice to view a sparkline bar chart showing your derived metric over time.

- `h/l` — expand/shrink time window (30-day increments, range 30-365 days)
- Stats shown: average, peak, and trend percentage (compares first half to second half)
- `/` — switch to a different practice
- `Esc` — back to Dashboard

### Managing Practices

Press `e` to manage your practice inventory.

- `a` — add a new practice
- `Enter` — rename a practice
- `d` — delete a practice (removes all its logs and sets)
- `Esc` — back to Dashboard

### Goals

Press `g` on the Dashboard to enter Goals editing mode. Goals help you set training targets and track milestones toward them.

- `a` — add a new goal
- `m` — add a milestone to the selected goal
- `Enter` — edit a goal or milestone title
- `Space` — toggle completion status (goals and milestones)
- `D` — edit completion date on a completed goal or milestone (format: YYYY-MM-DD)
- `d` — delete a goal or milestone
- `j/k` — navigate between goals and milestones
- `Esc` — return to the Dashboard

Completed goals and milestones display a checkmark with the completion date inline (e.g., `☑ First competition (Apr 15)`). Goals and milestones are stored in the database and included in JSON export/import.

### Daily Quote

A motivational quote rotates daily on the Dashboard. The quote changes each day automatically.

To customize, create `~/.ironcli/quotes.txt` with one quote per line. Empty lines are ignored. When this file exists, it replaces the built-in quotes entirely.

**Example `~/.ironcli/quotes.txt`:**

```
The only bad workout is the one that didn't happen.
Fall seven times, stand up eight.
You don't have to be extreme, just consistent.
Discipline is choosing between what you want now and what you want most.
```

To go back to the built-in quotes, delete or rename the file.

## Practice Types

| Type | UI Label | You Enter | Tracked Metric |
|---|---|---|---|
| Weighted | weightxreps | weight (kg) + reps per set | Volume (sum of weight x reps) |
| Bodyweight | reps | reps per set | Total reps |
| Distance | distance | distance (km) per set | Total distance |
| Endurance | duration | duration (min) per set | Total duration |

Units are fixed: **kg**, **km**, **minutes**.

## Keyboard Reference

### Dashboard

| Key | Action |
|---|---|
| `l` | Log a practice |
| `h` | View history |
| `t` | View trends |
| `e` | Manage practices |
| `g` | Goals mode |
| `q` | Quit |

### Log Entry

| Key | Action |
|---|---|
| `Enter` | Add set / next field |
| `Tab` | Switch field (weighted only) |
| `Ctrl+S` | Finish sets, enter note |
| `D` | Change log date |
| `d` | Delete last set (when fields empty) |
| `Esc` | Cancel |

### History

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `Enter` | Edit log |
| `d` | Delete log |
| `Esc` | Back |

### Trends

| Key | Action |
|---|---|
| `j/k` | Navigate practice list |
| `h/l` | Adjust time window |
| `/` | Change practice |
| `Esc` | Back |

### Practices

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `a` | Add practice |
| `Enter` | Edit name |
| `d` | Delete practice |
| `Esc` | Back |

### Goals Mode

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `a` | Add goal |
| `m` | Add milestone |
| `Enter` | Edit goal/milestone title |
| `Space` | Toggle completion |
| `D` | Edit completion date |
| `d` | Delete goal/milestone |
| `Esc` | Back to Dashboard |

## Data Backup

### Export

```bash
iron export                     # exports to ~/.ironcli/iron-export-YYYY-MM-DD.json
iron export /path/to/backup.json  # exports to a specific path
```

### Import

```bash
iron import /path/to/backup.json
```

Import is safe — it skips duplicate logs (matched by practice name + timestamp) and creates any missing practices automatically.

## Data Location

All data is stored in a single SQLite file:

```
~/.ironcli/iron.db
```

To back up manually, just copy this file. To reset, delete it — a fresh database will be created on next launch.
