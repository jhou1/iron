# iron

A beautiful terminal UI for tracking your training records. Log practices set-by-set, visualize consistency with a GitHub-style heatmap, and track progress with sparkline charts.

## Features

- **Multi-view heatmap** showing your training consistency over the year (GitHub-style grid, daily chart, weekday bars, monthly bars)
- **Set-by-set logging** — each set can have different weight/reps, with weight carry-forward
- **Sparkline trend charts** per practice with avg, peak, and trend percentage
- **14-day history** with inline set details and session notes
- **4 practice types**: weighted (weightxreps), bodyweight (reps), distance (km), endurance (min)
- **Vim-style navigation** throughout (j/k, h/l, /, Esc, Enter)
- **Goals and milestones** — set training targets and track progress
- **Daily motivational quotes** on the Dashboard — add and manage quotes in the database
- **Warm-up & cool-down notes** — optional text fields on each training log
- **Daily HRV tracking** — record your morning HRV score (0-100) on the dashboard
- **JSON export/import** for backup and data portability
- Data stored locally in SQLite — single file, zero cloud dependencies

## Installation

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (1.70+)

### Build from source

```bash
git clone <repo-url>
cd iron
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

## Language / i18n

iron supports multiple languages with automatic detection from environment variables.

**Supported languages:**
- English (default)
- Chinese (Simplified)

The app automatically detects your language from the `LANG` or `LC_ALL` environment variables. All UI text and CLI messages are translated.

To run in a specific language:

```bash
# Run in English
iron

# Run in Chinese (Simplified)
LANG=zh_CN.UTF-8 iron
```

## Usage

Launch the app:

```bash
iron
```

You'll land on the **Dashboard** — the home screen showing your training heatmap and today's stats.

### Heatmap Views

The Dashboard displays a multi-view heatmap at the top. Press `Tab` to cycle through four visualization modes:

1. **Map** (default) — GitHub-style contribution grid showing the last year of training activity
   - Each cell represents one day
   - 5-level green gradient based on total volume/reps/distance/duration
   - Empty cells indicate rest days

2. **Chart** — Daily vertical bars for the last 90 days
   - Each bar shows total volume for that day
   - Stacked by practice (different colors per practice)
   - Legend below shows practice names with their colors

3. **Days** — Weekday horizontal bars (Monday–Sunday)
   - Shows average volume per weekday across all history
   - Stacked by practice
   - Helps identify training patterns (e.g., "I usually train hardest on Thursdays")

4. **Months** — Monthly horizontal bars (January–December)
   - Shows total volume per month across all years
   - Stacked by practice
   - Useful for tracking seasonal training cycles

All views use consistent practice colors from a 10-color palette. The legend at the bottom identifies each practice.

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
6. Optionally enter warm-up and cool-down notes (e.g., "5 min jump rope"), press Enter to skip
7. Type an optional note (e.g., "Felt strong today") or press Enter to skip
8. Press Enter to save

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

Press `e` to manage your practice inventory. Each practice shows its name, type, and active/inactive status.

- `a` — add a new practice
- `Enter` — rename a practice
- `t` — toggle a practice between active and inactive
- `d` — delete a practice (removes all its logs and sets)
- `Esc` — back to Dashboard

Inactive practices are hidden from the log entry picker, history, trends, and dashboard statistics. They remain visible on the Practices screen and can be reactivated at any time by pressing `t` again.

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

A motivational quote rotates daily on the Dashboard. The quote changes each day automatically based on the day of year.

Press `Q` (uppercase) on the Dashboard to manage your quote library.

Inside the Quotes manager:
- `j/k` — navigate your quotes list
- `a` — add a new quote
- `e` or `Enter` — edit the selected quote
- `d` — delete a quote
- `Esc` — close the manager and return to Dashboard

When no quotes exist, the Dashboard shows "No quotes yet — press Q to add one" in gray text. Quotes are stored in the SQLite database alongside all your training data.

### HRV Tracking

Record your morning Heart Rate Variability score on the Dashboard.

- Press `v` to enter your HRV score (0-100)
- Type the number and press Enter to save
- Today's HRV is displayed inline on the Dashboard
- HRV data is included in JSON export/import for long-term analysis

### Quick Log (LLM-powered)

Press `w` on the Dashboard to open Quick Log — write training notes in natural shorthand and let an LLM parse them into structured logs.

**Example shorthand:**

```
DL 60kg 5/5/5
Pull-ups 10/8/6
Run 5km
```

**Workflow:**

1. Type your training notes in the left pane (one practice per line or multi-line blocks)
2. Press `Ctrl+S` to send to the LLM for parsing
3. Review parsed results in the right pane (matched practices show in green, unmatched in red)
4. Press `a` to add abbreviations for shortcuts (e.g., `DL` → `Deadlift`)
5. Press `D` to change the log date (defaults to today)
6. Press `Enter` to save all logs to the database
7. Press `Esc` to return to Dashboard

**Configuration:**

Create `~/.iron/config.toml` with your LLM settings:

```toml
# For local Ollama
[llm]
endpoint = "http://localhost:11434/v1"
model = "llama3.2:3b"

# For OpenAI
[llm]
endpoint = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o-mini"
```

The endpoint must implement the OpenAI Chat Completions API format (`/chat/completions`). The `api_key` is optional (not needed for Ollama).

**Abbreviations:**

Press `[a]` from the Dashboard to manage your abbreviation dictionary. Abbreviations help the LLM understand your shortcuts (e.g., `DL` = `Deadlift`, `BP` = `Bench Press`). You can also add abbreviations on-the-fly from the Quick Log preview pane by pressing `a`.

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
| `Tab` | Cycle heatmap view (Map/Chart/Days/Months) |
| `l` | Log a practice |
| `w` | Quick Log (LLM-powered) |
| `h` | View history |
| `t` | View trends |
| `e` | Manage practices |
| `a` | Manage abbreviations |
| `g` | Goals mode |
| `Q` | Manage quotes |
| `v` | Record/edit today's HRV |
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
| `/` | Filter by practice name |
| `e` | Edit log |
| `d` | Delete log |
| `Esc` | Clear filter / Back |

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
| `t` | Toggle active/inactive |
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

### Quotes Manager

| Key | Action |
|---|---|
| `j/k` | Navigate quotes |
| `a` | Add quote |
| `e` or `Enter` | Edit quote |
| `d` | Delete quote |
| `Esc` | Close manager |

### Quick Log

| Key | Action |
|---|---|
| `Ctrl+S` | Parse notes with LLM |
| `Enter` | New line (input) / Save all logs (preview) |
| `Up/Down` | Navigate lines (input) / Navigate results (preview) |
| `D` | Change log date |
| `a` | Add abbreviation (preview only) |
| `?` | Toggle help overlay |
| `Esc` | Back to Dashboard |

### Abbreviations

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `a` | Add abbreviation |
| `e` or `Enter` | Edit abbreviation |
| `d` | Delete abbreviation |
| `Esc` | Back to Dashboard |

## Data Backup

### Export

```bash
iron export                     # exports to ~/.iron/iron-export-YYYY-MM-DD.json
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
~/.iron/iron.db
```

To back up manually, just copy this file. To reset, delete it — a fresh database will be created on next launch.

The data directory was previously `~/.ironcli/`. If you have an existing database there, iron automatically migrates it to `~/.iron/` the first time you launch the new version. No manual action needed.
