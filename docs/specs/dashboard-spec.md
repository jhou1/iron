# Dashboard Spec

Dashboard is the home screen. It presents the 52-week training heatmap, weekly summary and streak information, daily quote, today's HRV, recent entries, active goals, status, and navigation.

Quick Log is permanently absent and has no replacement on Dashboard. The former `w` command is not handled or rendered.

## Responsive Layout

Dashboard is a bounded vertical stack:

1. logo/header
2. 52-week heatmap
3. summary panel containing weekly volume, streak, HRV, training frequency, and daily quote
4. split Recent/Goals region
5. reserved status row
6. active-mode shortcut footer

The status and complete shortcut rows are reserved before content panels receive the remaining height. Layout is recalculated from the current frame on every render. `80x35` is the primary acceptance size; smaller direct-render rectangles, including `40x16`, do not panic. Every block-derived inner rectangle is clamped to its parent.

Long practice names, metric suffixes, goal titles, percentages, quotes, and localized labels are truncated or wrapped within their real panel width. Recent and Goals stop rendering at their inner bottom edge.

## Heatmap and Summary

- The heatmap renders up to 52 weeks of daily session counts from `heatmap_counts(365)` using the existing intensity scale.
- The summary shows weekly volume, current streak, today's HRV (or unavailable state), and seven-day training frequency.
- One randomly selected user quote is wrapped inside the Summary panel; an empty quote collection produces no quote line.

## Recent Entries and Active Goals

- Recent shows logs from the last seven days, grouped by date, most recent first.
- Each recent row budgets the practice name and total metric against the pane's actual width.
- Goals shows incomplete goals only.
- Goal titles are bounded; each goal renders a progress gauge with a percentage suffix.

## Modes and Shortcuts

| Mode | Entry | Active commands |
|---|---|---|
| Normal | default | `l`, `h`, `t`, `e`, `g`, `Q`, `v`, `Esc` |
| HrvInput | `v` | `Enter`, `Esc` |
| ConfirmQuit | `Esc` | `y`, any other key to cancel |

Normal-mode navigation is:

| Key | Action |
|---|---|
| `l` | Log Entry |
| `h` | History |
| `t` | Trends |
| `e` | Practices |
| `g` | Goals |
| `Q` | Quotes |
| `v` | Record or edit today's HRV |
| `Esc` | Enter quit confirmation |

The footer is rendered from the current mode with shared `Shortcut` components. It wraps by complete key-label units when needed. HRV input and quit confirmation do not advertise inactive navigation commands.

## HRV Input and Failure Retention

Pressing `v` enters `HrvInput` and preloads today's value when present. The shared single-line `TextInput` keeps its Unicode-safe cursor visible inside the complete summary row.

- `Enter` accepts only an integer in the inclusive range `0..=100`.
- Parse or range validation failure keeps `HrvInput`, its text, and an error in the reserved status row.
- Database write failure keeps `HrvInput`, its text, and the previously displayed/database value while showing the write error.
- Only a successful `set_daily_hrv` updates the displayed value, clears the input, and returns to Normal mode.
- `Esc` cancels, clears the editor, and returns to Normal mode.

## Data Refresh

When navigation returns to Dashboard, it reloads heatmap data, recent entries, quotes, goals, HRV, weekly volume, training frequency, and streak information.

## Removal Contract

Dashboard exposes no Quick Log navigation or LLM-backed logging path. The supporting LLM configuration/runtime and Abbreviations API/data have been permanently removed from the application; opening an existing database drops the `abbreviations` table without altering unrelated training data.
