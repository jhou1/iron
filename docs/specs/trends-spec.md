# Trends Spec

View training progress over time with sparkline charts and summary statistics.

## Layout

Trends shows a bounded active-practice list above a specialized sparkline chart. Changing the selected practice immediately changes the chart; there is no separate phase transition.

Batch 3B-3 changes only this presentation composition. `ScreenFrame` owns the centered frame and reserves the filter, practice-list, chart, status, and shortcut rows in one pass. The status line and mode-accurate shortcut rows are global frame rows below the body, not part of the chart panel. `ScrollableListPanel` is the sole owner of the practice list's border, title and overflow markers, viewport reveal and clamp, and selected-row highlight. Trends continues to own filter rendering, practice row formatting, selection and filter state, and the specialized Sparkline/statistics panel.

The practice list uses the shared panel viewport. Selection wraps at both ends, remains visible, and the title shows `↑`, `↓`, or `↑↓` when more rows exist. Long practice names are truncated by terminal display width so the type column remains inside its panel.

The `/` filter uses the shared bounded input with Unicode-safe Emacs cursor movement. Long filter text scrolls horizontally to keep the cursor visible.

Six immutable styled-buffer fixtures preserve exact visual compatibility for a populated chart, long filter input, no filter match, list overflow, no log data, and a cached chart with a query error. The migration does not change filtering, selection wrapping, chart chronology or rendering policy, statistics formulas, time-window controls, no-data behavior, error retention, or re-entry behavior.

## Chart panel

**Display (top to bottom):**
1. Practice/metric/type title and time-window subtitle
2. Sparkline chart (fills available space)
3. Stats summary: average, peak, trend percentage

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

## Re-entry

Opening Trends reloads the active-practice list and chart before the route changes. Filter text, the selected practice's stable ID, the bounded list viewport, and the 30–365 day window are restored; filter-input mode and old status clear. The refreshed chart remains chronological.

An in-place chart query failure keeps the selected practice, changed window, and last successful chart while showing an error. A cross-page preparation failure is atomic: navigation stays on the source screen, the cached Trends target is unchanged, and the source shows the error.
