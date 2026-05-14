# iron

A beautiful terminal UI for tracking your training records. Log practices set-by-set, visualize consistency with a GitHub-style heatmap, and track progress with sparkline charts.

## Features

- **GitHub-style heatmap** showing your training consistency over the year
- **Set-by-set logging** — all fields on one screen: sets, warm-up/cool-down, notes
- **Sparkline trend charts** per practice with avg, peak, and trend percentage
- **Full history** with table-aligned columns and inline set details
- **4 practice types**: Weight x Reps, Reps Only, Distance (km), Duration (min)
- **Vim-style navigation** throughout (j/k, h/l, /, Esc, Enter)
- **Goals and milestones** with inline progress bars showing percentage
- **Daily motivational quotes** on the Dashboard with a dedicated management screen
- **Warm-up & cool-down notes** — optional text fields on each training log
- **Daily HRV tracking** — record your morning HRV score (0-100) on the dashboard
- **Quick Log** — natural language entry powered by LLM
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

You'll land on the **Dashboard** — the home screen showing your training heatmap, recent activity grouped by date, active goals with progress bars, and a daily quote.

### Dashboard Layout

The Dashboard displays:

1. **Header** — `⚡ Iron Training Tracker`
2. **Heatmap** — GitHub-style contribution grid in a bordered panel
3. **Quote** — daily motivational quote (rotates automatically)
4. **HRV** — today's Heart Rate Variability score
5. **Recent** — last 7 days of training, grouped by date
6. **Goals** — active goals with progress bars showing completion percentage

### First Time Setup

1. Press `e` to open the **Practices** screen
2. Press `a` to add a new practice
3. Type a name (e.g., "Kettlebell Snatch") and press Enter
4. Select the practice type with `j/k` and press Enter:
   - **Weight x Reps** — for weighted exercises (tracks kg and reps)
   - **Reps Only** — for bodyweight exercises (tracks reps only)
   - **Distance** — for running, cycling, etc. (tracks km)
   - **Duration** — for holds, planks, etc. (tracks minutes)
5. Press `Esc` to return to the Dashboard

### Logging a Practice

1. Press `l` to open the **Log Entry** screen
2. Type to filter practices, or use `j/k` to navigate, then press Enter
3. All fields are on one screen — sets, warm-up/cool-down, and notes:
   - **Sets section**: enter sets one at a time (weight carries forward for weighted practices)
   - **Warm-up / Cool-down**: optional text fields
   - **Note**: free-text area with automatic line wrapping
4. Use `Tab` to move between sections (weight → reps → warm-up → cool-down → note)
5. Press `D` to change the log date (defaults to today, format: YYYY-MM-DD)
6. Press `Ctrl+S` to save from any section

Example — logging kettlebell snatches:

```
Set 1: 24kg x 10      (type 24, Tab, 10, Enter)
Set 2: 24kg x 9       (Enter to keep 24kg, 9, Enter)
Set 3: 28kg x 8       (type 28, Tab, 8, Enter)
Tab to warm-up → Tab to note → Ctrl+S to save
```

### Viewing History

Press `h` to see your training history. The screen is split into two panels:

- **Left**: table of all logs with aligned Date, Practice, and Volume columns (dates in YYYY-MM-DD format)
- **Right**: detail panel showing sets, totals, warm-up/cool-down, and notes for the selected log

Navigation:
- `j/k` — navigate logs
- `/` — filter by practice name
- `e` — edit a log (re-opens the log entry screen with existing data)
- `d` — delete a log (with confirmation)
- `u` — undo last deletion
- `Esc` — back to Dashboard

### Viewing Trends

Press `t` to see **progress charts**. Select a practice to view a sparkline bar chart showing your derived metric over time.

- `h/l` — expand/shrink time window (30-day increments, range 30-365 days)
- Stats shown: average, peak, and trend percentage (compares first half to second half)
- `/` — switch to a different practice
- `Esc` — back to Dashboard

### Managing Practices

Press `e` to manage your practice inventory. Each practice shows its name, type, and a toggle switch for active/inactive status.

- `a` — add a new practice
- `Enter` — rename a practice
- `t` — toggle a practice between active and inactive (shown as ▰▱ / ▱▰)
- `d` — delete a practice (removes all its logs and sets)
- `Esc` — back to Dashboard

Inactive practices are hidden from the log entry picker, history, trends, and dashboard statistics. They remain visible on the Practices screen and can be reactivated at any time by pressing `t` again.

### Goals

Press `g` on the Dashboard to open the Goals screen. Goals help you set training targets and track milestones toward them.

Each goal displays a progress bar with inline percentage (e.g., `▰▰▰▰▰▰25%▱▱▱▱▱▱  1/4`) based on milestone completion.

- `a` — add a new goal
- `m` — add a milestone to the selected goal
- `Enter` — edit a goal or milestone title, or open milestone list
- `Space` — toggle completion status (goals and milestones)
- `D` — edit completion date on a completed goal or milestone (format: YYYY-MM-DD)
- `d` — delete a goal or milestone
- `j/k` — navigate between goals and milestones
- `Esc` — return to the Dashboard

### Quotes

Press `Q` (uppercase) on the Dashboard to open the Quotes screen. A motivational quote rotates daily on the Dashboard.

- `j/k` — navigate your quotes list
- `a` — add a new quote
- `e` or `Enter` — edit the selected quote
- `d` — delete a quote
- `u` — undo last deletion
- `Esc` — back to Dashboard

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

Press `a` from the Dashboard to manage your abbreviation dictionary. Abbreviations help the LLM understand your shortcuts (e.g., `DL` = `Deadlift`, `BP` = `Bench Press`). You can also add abbreviations on-the-fly from the Quick Log preview pane by pressing `a`.

## Practice Types

| Type | UI Label | You Enter | Tracked Metric |
|---|---|---|---|
| Weighted | Weight x Reps | weight (kg) + reps per set | Volume (sum of weight x reps) |
| Bodyweight | Reps Only | reps per set | Total reps |
| Distance | Distance | distance (km) per set | Total distance |
| Endurance | Duration | duration (min) per set | Total duration |

Units are fixed: **kg**, **km**, **minutes**.

## Keyboard Reference

### Dashboard

| Key | Action |
|---|---|
| `l` | Log a practice |
| `w` | Quick Log (LLM-powered) |
| `h` | View history |
| `t` | View trends |
| `e` | Manage practices |
| `a` | Manage abbreviations |
| `g` | Goals |
| `Q` | Quotes |
| `v` | Record/edit today's HRV |
| `q` | Quit |

### Log Entry

| Key | Action |
|---|---|
| `Tab` | Next field (weight → reps → warm-up → cool-down → note) |
| `Shift+Tab` | Previous field |
| `Enter` | Add set (in sets section) |
| `Ctrl+S` | Save log (from any section) |
| `D` | Change log date |
| `Backspace` | Delete last set (when fields empty) |
| `Esc` | Cancel |

### History

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `/` | Filter by practice name |
| `e` | Edit log |
| `d` | Delete log |
| `u` | Undo last deletion |
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

### Goals

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

### Quotes

| Key | Action |
|---|---|
| `j/k` | Navigate quotes |
| `a` | Add quote |
| `e` or `Enter` | Edit quote |
| `d` | Delete quote |
| `u` | Undo last deletion |
| `Esc` | Back to Dashboard |

### Quick Log

| Key | Action |
|---|---|
| `Ctrl+S` | Parse notes with LLM |
| `Enter` | New line (input) / Save all logs (preview) |
| `Up/Down` | Navigate lines (input) / Navigate results (preview) |
| `D` | Change log date |
| `a` | Add abbreviation (preview only) |
| `Esc` | Back to Dashboard |

### Abbreviations

| Key | Action |
|---|---|
| `j/k` | Navigate |
| `a` | Add abbreviation |
| `e` or `Enter` | Edit abbreviation |
| `d` | Delete abbreviation |
| `Esc` | Back to Dashboard |

### Text Input (all screens)

| Key | Action |
|---|---|
| `Ctrl+B` or `Left` | Move cursor back |
| `Ctrl+F` or `Right` | Move cursor forward |
| `Ctrl+A` or `Home` | Move to start |
| `Ctrl+E` or `End` | Move to end |
| `Ctrl+K` | Delete to end of line |
| `Backspace` | Delete before cursor |

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
