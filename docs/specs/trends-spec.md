# Trends Spec

View training progress over time with sparkline charts and summary statistics.

## Phases

```
SelectPractice -> ViewChart
```

## Phase 1: SelectPractice

**Purpose:** Choose which practice to view trends for.

**Display:**
- Title: "Select practice to view trends"
- Filter bar
- List of active practices

**Behavior:**
- `j/k` — navigate
- `/` — filter by name
- `Enter` — select, advance to ViewChart
- `Esc` — back to Dashboard

## Phase 2: ViewChart

**Display (top to bottom):**
1. Title: "[Practice Name] — [N] days"
2. Sparkline chart (fills available space)
3. Stats summary: average, peak, trend percentage
4. Status line
5. Footer

### Sparkline Chart

- Vertical bars, one per log entry
- Most recent on the right
- Bar height proportional to metric value (normalized to max)
- Color gradient based on value ratio (low=gray, mid=green, high=bright green)
- X-axis: date labels
- Y-axis: numeric value labels on right side
- Linear trendline overlay

### Stats

- **Average:** sum(metric) / count(logs) over the time window
- **Peak:** max(metric) over the time window
- **Trend:** percentage change comparing average of first half vs second half of the window. Positive = improving, negative = declining.

### Time Window

- Default: 90 days
- Range: 30 to 365 days
- `h` — shrink by 30 days (min 30)
- `l` — expand by 30 days (max 365)
- Chart redraws on window change

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate practice list (Phase 1) |
| `/` | Filter practices (Phase 1) or switch practice (Phase 2) |
| `h` | Shrink time window -30 days |
| `l` | Expand time window +30 days |
| `?` | Help overlay |
| `Esc` | Back to Dashboard |

## Data

- Query: `list_logs_for_practice(practice_id, days_window)`
- Sorted chronologically (oldest first) for chart rendering
- Each data point: (date label, total metric value)
