# iron

English | [中文版](README_zh.md)

A terminal UI for tracking training. Log practices set-by-set, review history, and visualize consistency. All data stays local in a single SQLite file.

![](docs/iron.jpg)

## Philosophy

- **Terminal-native**: keyboard-driven, fast, distraction-free
- **Local-first**: your data lives in `~/.iron/iron.db`, nowhere else
- **Minimal**: no accounts, no sync, no bloat — just training records

## Features

- **Set-by-set logging** with weight carry-forward, warm-up/cool-down, notes
- **Edit or delete committed sets** without re-entering the whole log
- **History** with filter, inline detail popup, edit, delete, undo
- **Trends** — sparkline charts per practice with time-window control
- **GitHub-style heatmap** on the Dashboard
- **Goals & milestones** with progress bars
- **Daily quotes** on the Dashboard, managed via Quotes screen
- **HRV tracking** — record a 0–100 score each morning
- **Quick Log** — natural-language entry via LLM (optional)
- **4 practice types**: Weight x Reps, Reps Only, Distance (km), Duration (min)
- **JSON export/import** for backup
- **Vim navigation** (j/k, h/l, /, Enter, Esc) + Emacs text input (Ctrl+A/E/B/F/K)
- **i18n**: English and Chinese (Simplified)

## Installation

Requires Rust 1.70+.

```bash
git clone https://github.com/jhou1/iron.git
cargo build --release
cp target/release/iron ~/.local/bin/
```

## Usage

```bash
iron              # English
LANG=zh_CN.UTF-8 iron  # Chinese
```

### Dashboard

The home screen shows the heatmap, training summary, recent activity, active goals, and a daily quote.

| Key   | Action                  |
|-------|-------------------------|
| `l`   | Log a practice          |
| `w`   | Quick Log (LLM-powered) |
| `h`   | History                 |
| `t`   | Trends                  |
| `e`   | Practices               |
| `g`   | Goals                   |
| `Q`   | Quotes                  |
| `v`   | Record today's HRV      |
| `Esc` | Quit                    |

### First Time Setup

1. Press `e` to open **Practices**
2. Press `a` to add a practice, name it, press Enter
3. Select the type with `j/k` and press Enter:
   - **Weight x Reps** — tracks kg and reps
   - **Reps Only** — bodyweight reps
   - **Distance** — km
   - **Duration** — minutes
4. Press `Esc` to return to Dashboard

### Logging

Press `l` to open the Log Entry screen.

1. Filter or navigate with `j/k`, press Enter to select a practice
2. Enter sets:
   - Type weight, Tab to reps, type reps, Enter to commit
   - For weighted practices, weight carries forward; just type reps and Enter
3. `Tab` cycles sections: sets → warm-up → cool-down → note
4. `j/k` to navigate committed sets; `e` to edit, `d` to delete
5. `D` to change the log date
6. `Ctrl+S` to save from any section
7. `Esc` to cancel

### History

Press `h`. Left panel shows all logs; right panel shows selected log details.

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `/` | Filter by practice name |
| `e` | Edit log |
| `d` | Delete log |
| `u` | Undo last deletion |
| `Esc` | Back |

### Trends

Press `t`. Select a practice to view its sparkline chart.

| Key | Action |
|---|---|
| `j/k` | Navigate practice list |
| `h/l` | Adjust time window (±30 days, 30–365) |
| `/` | Filter practice list |
| `Esc` | Back |

### Practices

Press `e` to manage your practice inventory.

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `a` | Add practice |
| `Enter` | Rename |
| `Space` | Toggle active/inactive |
| `d` | Delete |
| `Esc` | Back |

### Goals

Press `g`.

| Key | Action |
|---|---|
| `j/k` | Navigate goals / milestones |
| `a` | Add goal |
| `m` | Add milestone |
| `Enter` | Edit title |
| `Space` | Toggle completion |
| `D` | Edit completion date |
| `d` | Delete |
| `Esc` | Back |

### Quick Log (LLM)

Press `w`. Type training notes in natural shorthand (e.g., `DL 60kg 5/5/5`), press `Ctrl+S` to parse. Review results, press Enter to save. Requires LLM config in `~/.iron/config.toml`.

## Data

Stored in `~/.iron/iron.db` (SQLite). Back up by copying the file, or use JSON export:

```bash
iron export backup.json
iron import backup.json    # skips duplicates
```

The app auto-migrates data from the old `~/.ironcli/` directory on first launch.

## Keyboard Reference

### Text Input (all fields)

| Key | Action |
|---|---|
| `Ctrl+B` / Left | Cursor back |
| `Ctrl+F` / Right | Cursor forward |
| `Ctrl+A` / Home | Start of line |
| `Ctrl+E` / End | End of line |
| `Ctrl+K` | Delete to end |
| `Backspace` | Delete before cursor |
