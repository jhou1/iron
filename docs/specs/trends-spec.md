# Trends Spec

View training progress over time with sparkline charts and summary statistics.

## Layout

Trends shows a bounded active-practice list above a specialized sparkline chart. Changing the selected practice immediately changes the chart; there is no separate phase transition.

The practice list uses the shared viewport. Selection wraps at both ends, remains visible, and the title shows `↑`, `↓`, or `↑↓` when more rows exist. Long practice names are truncated by terminal display width so the type column remains inside its panel.

The `/` filter uses the shared bounded input with Unicode-safe Emacs cursor movement. Long filter text scrolls horizontally to keep the cursor visible.

## Chart panel

**Display (top to bottom):**
1. Title: "[Practice Name] — [N] days"
2. Sparkline chart (fills available space)
3. Stats summary: average, peak, trend percentage
4. Visible error status line
5. Mode-accurate shortcut row

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
| `j/k` | Navigate the practice list (wraps at both ends) |
| `/` | Filter practices by name |
| `h` | Shrink time window -30 days |
| `l` | Expand time window +30 days |
| `?` | Help overlay |
| `Esc` | Back to Dashboard |

## Data

- Query: `list_logs_for_practice(practice_id, days_window)`
- Sorted chronologically (oldest first) for chart rendering
- Each data point: (date label, total metric value)
- Query failures retain the last successful chart and appear in the reserved status row
