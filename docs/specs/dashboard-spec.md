# Dashboard Spec

Home screen. Shows training overview at a glance: heatmap, recent activity, goals, daily quote, and HRV.

## Layout

Vertical stack, top to bottom:

1. **Header** — app logo + version, rounded border
2. **Heatmap** — GitHub-style contribution grid (fixed 10 lines)
3. **Quote** — daily motivational quote (variable height, min 2 lines), centered
4. **HRV row** — today's Heart Rate Variability reading (3 lines)
5. **Split pane** — left: recent entries, right: active goals (fills remaining space)
6. **Status line** — success/error messages (1 line)
7. **Footer** — contextual keyboard shortcuts (1 line)

## Heatmap

Displays a year-long (52 weeks x 7 days) grid of training activity.

- Data: `heatmap_counts(365)` — one count per day
- Visual: circles/blocks in a 5-level green intensity scale
  - 0 sessions: gray
  - 1: dark green
  - 2: medium green
  - 3: bright green
  - 4+: vivid green
- Layout: day labels (Mon-Sun) on left, month labels on top
- Tab key cycles through heatmap view modes (map is the primary view)

## Quote Display

- Shows one quote from the user's quote collection, randomly selected
- If no quotes exist: shows placeholder text prompting user to add quotes
- Wrapped text, centered in container

## HRV Display

- Shows today's HRV value if recorded: "HRV: 42"
- If not recorded: "HRV: --" with hint to press `v` to record
- Input: numeric 0-100, validated on entry

## Recent Entries (Left Pane)

- Shows logs from the last 7 days
- Each row: date, practice name, total metric with unit
- Styled list, most recent first

## Active Goals (Right Pane)

- Shows incomplete goals only
- Each goal shows: title + progress gauge
- Gauge: `[=====     ] 3/5` (completed milestones / total)

## Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Normal | default | Navigation shortcuts visible |
| ConfirmQuit | `q` | Shows "Really quit? [y]/[any]" |
| QuotesManage | `Q` | Modal overlay listing all quotes |
| QuotesEdit | `a`/`e` in QuotesManage | Text input for quote text |
| QuotesConfirmDelete | `d` in QuotesManage | Confirm quote deletion |
| HrvInput | `v` | Text input for HRV value |

## Quote Management (Modal)

Triggered by `Q`. Overlay with scrollable list of all quotes.

- `j/k` — navigate
- `a` — add new quote (text input)
- `e` or `Enter` — edit selected quote
- `d` — delete selected quote (with confirmation)
- `Esc` — close modal

## Navigation

| Key | Action |
|-----|--------|
| `l` | Go to Log Entry |
| `w` | Go to Quick Log |
| `h` | Go to History |
| `t` | Go to Trends |
| `e` | Go to Practices |
| `g` | Go to Goals |
| `a` | Go to Abbreviations |
| `Tab` | Cycle heatmap view |
| `Q` | Manage quotes |
| `v` | Record/edit HRV |
| `?` | Help overlay |
| `q` | Quit (with confirmation) |

## Data Refresh

When returning to Dashboard from any screen, reload: heatmap data, recent entries, stats, quotes, goals, HRV.

## Status Messages

- Green text for success (e.g., "Log saved")
- Red text for errors
- Displayed in status line, cleared on next action
