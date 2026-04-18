# Warm-up, Cool-down & HRV Design Spec

## Overview

Add three new data capture fields to IronCLI:

1. **Warm-up** (text) — per-log, optional, short single-line description of warm-up routine
2. **Cool-down** (text) — per-log, optional, short single-line description of cool-down routine
3. **HRV score** (integer, 0-100) — daily metric, independent of any practice log, entered from the dashboard

These fields support long-term training analysis. Warm-up and cool-down help track routines over time. HRV is a morning readiness metric used to assess recovery and detect overtraining.

## Data Model

### `logs` table changes

Two new nullable columns added via migration:

```sql
ALTER TABLE logs ADD COLUMN warm_up TEXT;
ALTER TABLE logs ADD COLUMN cool_down TEXT;
```

The `Log` model struct gains:
- `warm_up: Option<String>`
- `cool_down: Option<String>`

### New `daily_metrics` table

```sql
CREATE TABLE IF NOT EXISTS daily_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,   -- YYYY-MM-DD, one row per day
    hrv INTEGER                  -- HRV score, 0-100
);
```

New model struct:
```rust
pub struct DailyMetrics {
    pub id: i64,
    pub date: String,
    pub hrv: Option<i32>,
}
```

HRV is once-per-day, enforced by the `UNIQUE` constraint on `date`.

## Log Entry Flow

### Current flow

`SelectPractice` -> `EnterSets` -> `EnterNote`

### Updated flow

`SelectPractice` -> `EnterSets` -> `EnterWarmUpCoolDown` -> `EnterNote`

The new `EnterWarmUpCoolDown` phase displays two single-line text inputs:

```
 Warm-up:   5 min jump rope, stretches|
 Cool-down: |

 [Tab] switch field  [Enter] next  [Ctrl+S] save  [Esc] cancel
```

Behavior:
- Both fields are optional; pressing Enter with empty fields advances to the note phase
- Emacs-style cursor navigation (Ctrl+B/F/A/E, Left/Right/Home/End) consistent with the note field
- Tab switches between the two fields
- Ctrl+S skips ahead to save (same shortcut as in the sets phase)
- When editing an existing log, pre-fills from saved values

## Dashboard HRV Display & Input

### Display

A small inline row on the dashboard, positioned between the quote box and the split panes:

```
 HRV: 72  [v] edit
```

When no HRV is recorded for today:

```
 HRV: --  [v] record
```

### Input

Pressing `v` in normal dashboard mode enters HRV edit mode (inline, similar to date editing in the log entry screen):

```
 HRV: 72|  (0-100, Enter to save, Esc to cancel)
```

- Accepts digits only
- Validates 0-100 on Enter; invalid values are rejected (field stays open)
- Saves via upsert to `daily_metrics` table for today's date
- Only edits today's value; historical HRV is accessible through export

A new `DashboardMode::HrvInput` variant handles this state.

## Database Layer

### Schema migration

In `init_schema`, following the existing migration pattern:

```rust
let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN warm_up TEXT", []);
let _ = self.conn.execute("ALTER TABLE logs ADD COLUMN cool_down TEXT", []);
```

The `daily_metrics` table creation goes in the main `CREATE TABLE IF NOT EXISTS` batch.

### Updated methods

- `create_log` — add `warm_up: Option<&str>`, `cool_down: Option<&str>` parameters
- `create_log_at` — add `warm_up: Option<&str>`, `cool_down: Option<&str>` parameters
- `update_log` — add `warm_up: Option<&str>`, `cool_down: Option<&str>` parameters
- All `list_logs_*` queries (`list_logs_all`, `list_logs_recent`, `list_logs_for_practice`, `export_all`) — select `warm_up` and `cool_down` columns, populate `Log` struct

### New methods

- `get_daily_hrv(date: &str) -> Result<Option<i32>>` — fetch HRV for a given date
- `set_daily_hrv(date: &str, hrv: i32) -> Result<()>` — upsert via `INSERT OR REPLACE`
- `list_daily_metrics() -> Result<Vec<DailyMetrics>>` — fetch all daily metrics (for export)

## Export/Import

### Export changes

`ExportLog` gains two new optional fields:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub warm_up: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub cool_down: Option<String>,
```

New struct:
```rust
pub struct ExportDailyMetrics {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv: Option<i32>,
}
```

`ExportData` gains:
```rust
#[serde(default)]
pub daily_metrics: Vec<ExportDailyMetrics>,
```

Export version stays at 2 — all new fields are additive and optional, so older exports import cleanly.

### Import changes

- Read `warm_up` and `cool_down` from log entries if present (serde default handles missing fields)
- Import `daily_metrics` array if present; upsert by date to avoid duplicates

## I18n

New translation keys for EN and ZH-CN:

- `log-warmup-label` — "Warm-up" / "热身"
- `log-cooldown-label` — "Cool-down" / "放松"
- `dashboard-hrv-label` — "HRV" / "HRV"
- `dashboard-hrv-edit-hint` — "[v] edit" / "[v] 编辑"
- `dashboard-hrv-record-hint` — "[v] record" / "[v] 记录"
- `dashboard-hrv-input-hint` — "(0-100, Enter to save, Esc to cancel)" / "(0-100, Enter 保存, Esc 取消)"

## History Screen

Warm-up and cool-down are displayed in the detail pane (below sets, above note) when present:

```
    Warm-up: 5 min jump rope, stretches
    Cool-down: static stretches, foam roll
    Note: felt good
```

## Out of Scope

- No HRV trendline/sparkline on the Trends screen
- No historical HRV browsing in the UI (use export for analysis)
- No warm-up/cool-down display on the dashboard recent entries (visible in history detail only)
