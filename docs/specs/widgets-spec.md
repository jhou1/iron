# Widgets Spec

Reusable visual components used across screens.

## Heatmap

GitHub-style contribution grid showing training frequency over time.

### Input
- Data: list of (date string, session count) pairs
- Weeks: number of columns (52 for year view)

### Visual

```
       Jan  Feb  Mar  ...  Dec
Mon    ●  ●  ●  ●  ●  ...
Tue    ●  ●  ●  ●  ●  ...
Wed    ●  ●  ●  ●  ●  ...
Thu    ●  ●  ●  ●  ●  ...
Fri    ●  ●  ●  ●  ●  ...
Sat    ●  ●  ●  ●  ●  ...
Sun    ●  ●  ●  ●  ●  ...
```

### Layout
- Left column: day labels (Mon-Sun)
- Top row: month labels (abbreviated)
- Grid: 7 rows x N weeks
- Cell width: 2 characters (circle + space)

### Color Scale (5 levels)

| Sessions | Color |
|----------|-------|
| 0 | Dark gray |
| 1 | Dark green |
| 2 | Medium green |
| 3 | Bright green |
| 4+ | Vivid green |

No-color mode: maps to grayscale equivalents.

### Cell Character
"●" (bullet circle)

---

## Sparkline

Vertical bar chart showing metric values over time.

### Input
- Data: list of (label, value) pairs — label is typically a date string

### Visual

```
        ██
     ██ ██    ██
  ██ ██ ██ ██ ██
  ██ ██ ██ ██ ██
  ─────────────── 
  May  Jun  Jul
```

### Layout
- Bars: one per data point, most recent on right
- Bar slot width: 3 characters (2 bar + 1 gap)
- X-axis: date labels at bottom
- Y-axis: numeric labels on right side
- Minimum render area: 4x4 characters

### Bar Height
Proportional to value, normalized to maximum value in dataset. Minimum 1 character for any value > 0.

### Color Gradient

| Value ratio | Color |
|-------------|-------|
| 0.0 - 0.33 | Dark gray |
| 0.33 - 0.66 | Green |
| 0.66 - 1.0 | Light green |

### Trendline
Linear regression overlay rendered as a visual indicator of overall direction.

### Value Labels
Compact format above bars (e.g., "1.2k", "450", "12.5M").

### Adaptive Data
Shows the most recent N data points that fit within the available width.
