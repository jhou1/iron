# Heatmap Multi-View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the dashboard heatmap from a single GitHub-style grid into a multi-view visualization system with 4 views (Map, Chart, Days, Months), featuring stacked bars by practice and a redesigned color scheme.

**Architecture:** Each view is a separate ratatui Widget in `src/tui/widgets/`. The Dashboard holds a `view_index` and cycles views with Tab. Three new DB queries provide per-practice breakdowns for the bar chart views. The existing heatmap gets a visual refresh (better colors, month gaps, reduced day labels).

**Tech Stack:** Rust, ratatui (Widget trait, Buffer API), rusqlite, chrono

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `src/tui/widgets/heatmap.rs` | Modify | Redesign: better colors, month gaps, show Mon/Wed/Fri only |
| `src/tui/widgets/daily_chart.rs` | Create | Vertical bar chart showing sessions per day, stacked by practice |
| `src/tui/widgets/weekday_chart.rs` | Create | 7 horizontal bars (Mon–Sun), stacked by practice |
| `src/tui/widgets/monthly_chart.rs` | Create | 12 horizontal bars (Jan–Dec), stacked by practice |
| `src/tui/widgets/mod.rs` | Modify | Add new widget modules |
| `src/tui/dashboard.rs` | Modify | Add view_index, Tab cycling, data fields, render dispatch |
| `src/db.rs` | Modify | Add 3 new aggregate queries |
| `locales/en.ftl` | Modify | New i18n keys for view labels |
| `locales/zh-CN.ftl` | Modify | Chinese translations |
| `tests/db_test.rs` | Modify | Tests for new queries |
| `tests/tui_lint_test.rs` | Modify | Update allowlist for new widgets |
| `README.md` | Modify | Document multi-view feature |

---

### Task 1: Redesign Heatmap (Map View) Visual

**Files:**
- Modify: `src/tui/widgets/heatmap.rs`

- [ ] **Step 1: Update color palette**

In `src/tui/widgets/heatmap.rs`, replace the `cell_for_count` method (lines 24–40):

```rust
fn cell_for_count(&self, count: i64) -> (&'static str, Color) {
    if self.no_color {
        match count {
            0 => (" ", Color::Reset),
            1 => ("\u{2591}", Color::Reset),
            2 => ("\u{2592}", Color::Reset),
            _ => ("\u{2588}", Color::Reset),
        }
    } else {
        match count {
            0 => ("\u{2588}", Color::Indexed(236)),  // dark gray
            1 => ("\u{2588}", Color::Indexed(22)),   // dark green
            2 => ("\u{2588}", Color::Indexed(28)),   // medium green
            3 => ("\u{2588}", Color::Indexed(34)),   // bright green
            _ => ("\u{2588}", Color::Indexed(82)),   // vivid green
        }
    }
}
```

- [ ] **Step 2: Show only Mon/Wed/Fri day labels**

Replace the day label rendering loop (lines 79–85). Instead of rendering all 7 labels, only render indices 0 (Mon), 2 (Wed), 4 (Fri):

```rust
for (row, label) in day_labels.iter().enumerate() {
    let y = grid_y + row as u16;
    if y >= area.y + area.height {
        break;
    }
    // Only show Mon, Wed, Fri labels (indices 0, 2, 4)
    let text = if row == 0 || row == 2 || row == 4 {
        label.as_str()
    } else {
        ""
    };
    buf.set_string(area.x, y, text, Style::default().fg(Color::Gray));
}
```

- [ ] **Step 3: Add spacing between months**

In the grid rendering loop (lines 111–145), add a 1-column gap when the month changes. This requires tracking the current month and inserting an extra column offset:

Replace the grid loop with one that tracks month boundaries and adds gaps. The key change: instead of calculating `x` as `label_width + week * cell_width`, use an accumulated `col_offset` that adds 1 each time the month changes.

```rust
let cols_available = area.width.saturating_sub(label_width);
let cell_width: u16 = 2;

let mut col_offset: u16 = 0;
let mut prev_month: Option<u32> = None;
let mut week = 0u16;

while week < self.weeks {
    let week_monday = start_monday + Duration::weeks(week as i64);
    let month = week_monday.month();

    // Add gap between months
    if let Some(pm) = prev_month {
        if pm != month {
            col_offset += 1;
        }
    }
    prev_month = Some(month);

    let x = area.x + label_width + col_offset;
    if x + 1 >= area.x + area.width {
        break;
    }

    // Place month label at first week of each new month
    if last_labeled_month != Some(month) {
        let label = &month_names[(month - 1) as usize];
        let label_len = label.width() as u16;
        let label_x = x.max(last_label_end_x);
        if label_x + label_len <= area.x + area.width {
            buf.set_string(label_x, month_row_y, label, Style::default().fg(Color::Gray));
            last_label_end_x = label_x + label_len + 1;
        }
        last_labeled_month = Some(month);
    }

    for day in 0..7u16 {
        let date = week_monday + Duration::days(day as i64);
        if date > today {
            continue;
        }
        let date_str = date.format("%Y-%m-%d").to_string();
        let count = counts.get(&date_str).copied().unwrap_or(0);
        let (ch, color) = self.cell_for_count(count);

        let y = grid_y + day;
        if y < area.y + area.height && x + 1 < area.x + area.width {
            buf.set_string(x, y, ch, Style::default().fg(color));
        }
    }

    col_offset += cell_width;
    week += 1;
}
```

- [ ] **Step 4: Update legend with 5 levels**

Replace the legend rendering (lines 147–176) to show the new 5-level color palette:

```rust
let legend_y = grid_y + 7;
if legend_y < area.y + area.height {
    let legend = if self.no_color {
        Line::from(vec![
            Span::styled(format!("{} ", crate::i18n::tr("heatmap-less")), Style::default()),
            Span::raw(" "),
            Span::raw("\u{2591}"),
            Span::raw(" "),
            Span::raw("\u{2592}"),
            Span::raw(" "),
            Span::raw("\u{2588}"),
            Span::styled(format!(" {}", crate::i18n::tr("heatmap-more")), Style::default()),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", crate::i18n::tr("heatmap-less")), Style::default().fg(Color::Gray)),
            Span::styled("\u{2588}", Style::default().fg(Color::Indexed(236))),
            Span::raw(" "),
            Span::styled("\u{2588}", Style::default().fg(Color::Indexed(22))),
            Span::raw(" "),
            Span::styled("\u{2588}", Style::default().fg(Color::Indexed(28))),
            Span::raw(" "),
            Span::styled("\u{2588}", Style::default().fg(Color::Indexed(34))),
            Span::raw(" "),
            Span::styled("\u{2588}", Style::default().fg(Color::Indexed(82))),
            Span::styled(format!(" {}", crate::i18n::tr("heatmap-more")), Style::default().fg(Color::Gray)),
        ])
    };
    buf.set_line(area.x + label_width, legend_y, &legend, area.width.saturating_sub(label_width));
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: compiles

- [ ] **Step 6: Commit**

```bash
git add src/tui/widgets/heatmap.rs
git commit -m "feat(heatmap): redesign with 5-level colors, month gaps, reduced day labels"
```

---

### Task 2: New DB Queries for Per-Practice Breakdowns

**Files:**
- Modify: `src/db.rs`
- Modify: `tests/db_test.rs`

The three new views need per-practice session counts grouped by day, weekday, or month.

- [ ] **Step 1: Add `daily_practice_counts` method to db.rs**

Add at the end of the `impl Database` block, before the closing `}`:

```rust
    pub fn daily_practice_counts(&self, days: i64) -> Result<Vec<(String, Vec<(String, i64)>)>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT substr(l.logged_at, 1, 10) AS day, p.name, COUNT(*) AS cnt
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1 AND p.active = 1
             GROUP BY day, p.name
             ORDER BY day, p.name",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })?;

        let mut result: Vec<(String, Vec<(String, i64)>)> = Vec::new();
        for row in rows {
            let (day, name, count) = row?;
            if let Some(last) = result.last_mut() {
                if last.0 == day {
                    last.1.push((name, count));
                    continue;
                }
            }
            result.push((day, vec![(name, count)]));
        }
        Ok(result)
    }

    pub fn weekday_practice_counts(&self, days: i64) -> Result<Vec<(u32, Vec<(String, i64)>)>> {
        let cutoff = Local::now().naive_local() - chrono::Duration::days(days);
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%w', substr(l.logged_at, 1, 10)) AS INTEGER) AS dow,
                    p.name, COUNT(*) AS cnt
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE l.logged_at >= ?1 AND p.active = 1
             GROUP BY dow, p.name
             ORDER BY dow, cnt DESC",
        )?;
        let rows = stmt.query_map(params![cutoff.to_string()], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })?;

        let mut result: Vec<(u32, Vec<(String, i64)>)> = vec![];
        for row in rows {
            let (dow, name, count) = row?;
            if let Some(last) = result.last_mut() {
                if last.0 == dow {
                    last.1.push((name, count));
                    continue;
                }
            }
            result.push((dow, vec![(name, count)]));
        }
        Ok(result)
    }

    pub fn monthly_practice_counts(&self, year: i32) -> Result<Vec<(u32, Vec<(String, i64)>)>> {
        let year_str = format!("{:04}", year);
        let mut stmt = self.conn.prepare(
            "SELECT CAST(substr(l.logged_at, 6, 2) AS INTEGER) AS mon,
                    p.name, COUNT(*) AS cnt
             FROM logs l
             JOIN practices p ON l.practice_id = p.id
             WHERE substr(l.logged_at, 1, 4) = ?1 AND p.active = 1
             GROUP BY mon, p.name
             ORDER BY mon, cnt DESC",
        )?;
        let rows = stmt.query_map(params![year_str], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })?;

        let mut result: Vec<(u32, Vec<(String, i64)>)> = vec![];
        for row in rows {
            let (mon, name, count) = row?;
            if let Some(last) = result.last_mut() {
                if last.0 == mon {
                    last.1.push((name, count));
                    continue;
                }
            }
            result.push((mon, vec![(name, count)]));
        }
        Ok(result)
    }
```

- [ ] **Step 2: Write tests**

Add to `tests/db_test.rs`:

```rust
#[test]
fn test_daily_practice_counts() {
    let (_dir, db) = test_db();
    let p = db.create_practice("Deadlift", PracticeType::Weighted).unwrap();
    let now = Local::now().naive_local();
    db.create_log_at(p.id, &now, &[SetData::Weighted { weight: 100.0, reps: 5 }], None, None, None).unwrap();
    db.create_log_at(p.id, &now, &[SetData::Weighted { weight: 100.0, reps: 5 }], None, None, None).unwrap();

    let counts = db.daily_practice_counts(7).unwrap();
    assert!(!counts.is_empty());
    let today = now.format("%Y-%m-%d").to_string();
    let today_entry = counts.iter().find(|(d, _)| d == &today).unwrap();
    assert_eq!(today_entry.1.len(), 1);
    assert_eq!(today_entry.1[0].0, "Deadlift");
    assert_eq!(today_entry.1[0].1, 2);
}

#[test]
fn test_weekday_practice_counts() {
    let (_dir, db) = test_db();
    let p = db.create_practice("Squats", PracticeType::Weighted).unwrap();
    let now = Local::now().naive_local();
    db.create_log_at(p.id, &now, &[SetData::Weighted { weight: 80.0, reps: 5 }], None, None, None).unwrap();

    let counts = db.weekday_practice_counts(7).unwrap();
    assert!(!counts.is_empty());
    let today_dow = Local::now().date_naive().weekday().num_days_from_sunday();
    let entry = counts.iter().find(|(dow, _)| *dow == today_dow).unwrap();
    assert_eq!(entry.1[0].0, "Squats");
}

#[test]
fn test_monthly_practice_counts() {
    let (_dir, db) = test_db();
    let p = db.create_practice("Running", PracticeType::Distance).unwrap();
    let now = Local::now().naive_local();
    db.create_log_at(p.id, &now, &[SetData::Distance { distance: 5.0 }], None, None, None).unwrap();

    let year = Local::now().date_naive().year();
    let counts = db.monthly_practice_counts(year).unwrap();
    assert!(!counts.is_empty());
    let this_month = Local::now().date_naive().month();
    let entry = counts.iter().find(|(m, _)| *m == this_month).unwrap();
    assert_eq!(entry.1[0].0, "Running");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test db_test`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/db.rs tests/db_test.rs
git commit -m "feat(heatmap): add daily/weekday/monthly per-practice count queries"
```

---

### Task 3: Shared Color Palette for Stacked Bars

**Files:**
- Modify: `src/tui/widgets/mod.rs`

A shared color palette that assigns consistent colors to practices across all stacked bar views.

- [ ] **Step 1: Add color palette constant and helper**

In `src/tui/widgets/mod.rs`, add after the module declarations:

```rust
use ratatui::style::Color;

pub const PRACTICE_COLORS: [Color; 10] = [
    Color::Green,
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightBlue,
];

pub fn practice_color(index: usize) -> Color {
    PRACTICE_COLORS[index % PRACTICE_COLORS.len()]
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`

- [ ] **Step 3: Commit**

```bash
git add src/tui/widgets/mod.rs
git commit -m "feat(heatmap): add shared practice color palette for stacked bars"
```

---

### Task 4: Weekday Chart Widget (Days View)

**Files:**
- Create: `src/tui/widgets/weekday_chart.rs`
- Modify: `src/tui/widgets/mod.rs`

7 horizontal bars (Mon–Sun), each stacked by practice with a legend.

- [ ] **Step 1: Add module declaration**

In `src/tui/widgets/mod.rs`, add:

```rust
pub mod weekday_chart;
```

- [ ] **Step 2: Implement the widget**

Create `src/tui/widgets/weekday_chart.rs`:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::practice_color;

pub struct WeekdayChart<'a> {
    data: &'a [(u32, Vec<(String, i64)>)],
    no_color: bool,
}

impl<'a> WeekdayChart<'a> {
    pub fn new(data: &'a [(u32, Vec<(String, i64)>)], no_color: bool) -> Self {
        Self { data, no_color }
    }
}

impl<'a> Widget for WeekdayChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 9 || area.width < 20 {
            return;
        }

        let day_names = [
            crate::i18n::tr("heatmap-sun"),
            crate::i18n::tr("heatmap-mon"),
            crate::i18n::tr("heatmap-tue"),
            crate::i18n::tr("heatmap-wed"),
            crate::i18n::tr("heatmap-thu"),
            crate::i18n::tr("heatmap-fri"),
            crate::i18n::tr("heatmap-sat"),
        ];

        let label_width: u16 = 5;
        let bar_area_width = area.width.saturating_sub(label_width) as usize;

        // Collect all unique practice names (ordered by total frequency)
        let mut practice_totals: Vec<(String, i64)> = Vec::new();
        for (_, practices) in self.data {
            for (name, count) in practices {
                if let Some(entry) = practice_totals.iter_mut().find(|(n, _)| n == name) {
                    entry.1 += count;
                } else {
                    practice_totals.push((name.clone(), *count));
                }
            }
        }
        practice_totals.sort_by(|a, b| b.1.cmp(&a.1));
        let practice_names: Vec<String> = practice_totals.iter().map(|(n, _)| n.clone()).collect();

        // Find max total across all days for scaling
        let max_total: i64 = self.data.iter()
            .map(|(_, practices)| practices.iter().map(|(_, c)| c).sum::<i64>())
            .max()
            .unwrap_or(1)
            .max(1);

        // Render each weekday bar (Sun=0..Sat=6, display Mon first)
        let display_order = [1u32, 2, 3, 4, 5, 6, 0]; // Mon..Sun
        for (row, &dow) in display_order.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height - 1 {
                break;
            }

            // Day label
            let label = &day_names[dow as usize];
            buf.set_string(area.x, y, format!("{:>4} ", label), Style::default().fg(Color::Gray));

            // Get practices for this day
            let practices = self.data.iter()
                .find(|(d, _)| *d == dow)
                .map(|(_, p)| p.as_slice())
                .unwrap_or(&[]);

            let total: i64 = practices.iter().map(|(_, c)| c).sum();
            let bar_width = if max_total > 0 {
                ((total as f64 / max_total as f64) * bar_area_width as f64) as usize
            } else {
                0
            };

            // Render stacked segments
            let mut x_offset = 0usize;
            for (name, count) in practices {
                let seg_width = if total > 0 {
                    ((*count as f64 / total as f64) * bar_width as f64).round() as usize
                } else {
                    0
                };
                let color_idx = practice_names.iter().position(|n| n == name).unwrap_or(0);
                let color = if self.no_color { Color::White } else { practice_color(color_idx) };

                for i in 0..seg_width {
                    let x = area.x + label_width + (x_offset + i) as u16;
                    if x < area.x + area.width {
                        buf.set_string(x, y, "\u{2588}", Style::default().fg(color));
                    }
                }
                x_offset += seg_width;
            }
        }

        // Legend row
        let legend_y = area.y + 7;
        if legend_y < area.y + area.height {
            let mut x = area.x + label_width;
            for (i, name) in practice_names.iter().take(6).enumerate() {
                let color = if self.no_color { Color::White } else { practice_color(i) };
                if x + 2 + name.len() as u16 >= area.x + area.width {
                    break;
                }
                buf.set_string(x, legend_y, "\u{25A0}", Style::default().fg(color));
                x += 2;
                buf.set_string(x, legend_y, name, Style::default().fg(Color::Gray));
                x += name.len() as u16 + 2;
            }
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/tui/widgets/weekday_chart.rs src/tui/widgets/mod.rs
git commit -m "feat(heatmap): add weekday stacked bar chart widget (Days view)"
```

---

### Task 5: Monthly Chart Widget (Months View)

**Files:**
- Create: `src/tui/widgets/monthly_chart.rs`
- Modify: `src/tui/widgets/mod.rs`

12 horizontal bars (Jan–Dec), stacked by practice. Very similar pattern to weekday_chart but with 12 rows and month names.

- [ ] **Step 1: Add module declaration**

In `src/tui/widgets/mod.rs`, add:

```rust
pub mod monthly_chart;
```

- [ ] **Step 2: Implement the widget**

Create `src/tui/widgets/monthly_chart.rs`:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::practice_color;

pub struct MonthlyChart<'a> {
    data: &'a [(u32, Vec<(String, i64)>)],
    no_color: bool,
}

impl<'a> MonthlyChart<'a> {
    pub fn new(data: &'a [(u32, Vec<(String, i64)>)], no_color: bool) -> Self {
        Self { data, no_color }
    }
}

impl<'a> Widget for MonthlyChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 9 || area.width < 20 {
            return;
        }

        let month_names = [
            crate::i18n::tr("heatmap-jan"),
            crate::i18n::tr("heatmap-feb"),
            crate::i18n::tr("heatmap-mar"),
            crate::i18n::tr("heatmap-apr"),
            crate::i18n::tr("heatmap-may"),
            crate::i18n::tr("heatmap-jun"),
            crate::i18n::tr("heatmap-jul"),
            crate::i18n::tr("heatmap-aug"),
            crate::i18n::tr("heatmap-sep"),
            crate::i18n::tr("heatmap-oct"),
            crate::i18n::tr("heatmap-nov"),
            crate::i18n::tr("heatmap-dec"),
        ];

        let label_width: u16 = 5;
        let bar_area_width = area.width.saturating_sub(label_width) as usize;
        let max_rows = (area.height - 1) as usize; // reserve 1 for legend

        // Collect all unique practice names
        let mut practice_totals: Vec<(String, i64)> = Vec::new();
        for (_, practices) in self.data {
            for (name, count) in practices {
                if let Some(entry) = practice_totals.iter_mut().find(|(n, _)| n == name) {
                    entry.1 += count;
                } else {
                    practice_totals.push((name.clone(), *count));
                }
            }
        }
        practice_totals.sort_by(|a, b| b.1.cmp(&a.1));
        let practice_names: Vec<String> = practice_totals.iter().map(|(n, _)| n.clone()).collect();

        let max_total: i64 = self.data.iter()
            .map(|(_, practices)| practices.iter().map(|(_, c)| c).sum::<i64>())
            .max()
            .unwrap_or(1)
            .max(1);

        // Render each month bar (1..=12)
        for (row, month) in (1u32..=12).enumerate() {
            if row >= max_rows {
                break;
            }
            let y = area.y + row as u16;

            let label = &month_names[(month - 1) as usize];
            buf.set_string(area.x, y, format!("{:>4} ", label), Style::default().fg(Color::Gray));

            let practices = self.data.iter()
                .find(|(m, _)| *m == month)
                .map(|(_, p)| p.as_slice())
                .unwrap_or(&[]);

            let total: i64 = practices.iter().map(|(_, c)| c).sum();
            let bar_width = if max_total > 0 {
                ((total as f64 / max_total as f64) * bar_area_width as f64) as usize
            } else {
                0
            };

            let mut x_offset = 0usize;
            for (name, count) in practices {
                let seg_width = if total > 0 {
                    ((*count as f64 / total as f64) * bar_width as f64).round() as usize
                } else {
                    0
                };
                let color_idx = practice_names.iter().position(|n| n == name).unwrap_or(0);
                let color = if self.no_color { Color::White } else { practice_color(color_idx) };

                for i in 0..seg_width {
                    let x = area.x + label_width + (x_offset + i) as u16;
                    if x < area.x + area.width {
                        buf.set_string(x, y, "\u{2588}", Style::default().fg(color));
                    }
                }
                x_offset += seg_width;
            }
        }

        // Legend row (same as weekday chart)
        let legend_y = area.y + max_rows.min(12) as u16;
        if legend_y < area.y + area.height {
            let mut x = area.x + label_width;
            for (i, name) in practice_names.iter().take(6).enumerate() {
                let color = if self.no_color { Color::White } else { practice_color(i) };
                if x + 2 + name.len() as u16 >= area.x + area.width {
                    break;
                }
                buf.set_string(x, legend_y, "\u{25A0}", Style::default().fg(color));
                x += 2;
                buf.set_string(x, legend_y, name, Style::default().fg(Color::Gray));
                x += name.len() as u16 + 2;
            }
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/tui/widgets/monthly_chart.rs src/tui/widgets/mod.rs
git commit -m "feat(heatmap): add monthly stacked bar chart widget (Months view)"
```

---

### Task 6: Daily Chart Widget (Chart View)

**Files:**
- Create: `src/tui/widgets/daily_chart.rs`
- Modify: `src/tui/widgets/mod.rs`

Vertical bars showing sessions per day (like sparkline but stacked by practice).

- [ ] **Step 1: Add module declaration**

In `src/tui/widgets/mod.rs`, add:

```rust
pub mod daily_chart;
```

- [ ] **Step 2: Implement the widget**

Create `src/tui/widgets/daily_chart.rs`:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::practice_color;

pub struct DailyChart<'a> {
    data: &'a [(String, Vec<(String, i64)>)],
    no_color: bool,
}

impl<'a> DailyChart<'a> {
    pub fn new(data: &'a [(String, Vec<(String, i64)>)], no_color: bool) -> Self {
        Self { data, no_color }
    }
}

impl<'a> Widget for DailyChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 4 || area.width < 10 {
            return;
        }

        // Collect all unique practice names
        let mut practice_totals: Vec<(String, i64)> = Vec::new();
        for (_, practices) in self.data {
            for (name, count) in practices {
                if let Some(entry) = practice_totals.iter_mut().find(|(n, _)| n == name) {
                    entry.1 += count;
                } else {
                    practice_totals.push((name.clone(), *count));
                }
            }
        }
        practice_totals.sort_by(|a, b| b.1.cmp(&a.1));
        let practice_names: Vec<String> = practice_totals.iter().map(|(n, _)| n.clone()).collect();

        let chart_height = area.height.saturating_sub(2) as usize; // reserve 1 x-axis + 1 legend
        let bar_slot = 3u16; // 2 bar chars + 1 gap
        let max_bars = (area.width / bar_slot) as usize;
        let entries: Vec<&(String, Vec<(String, i64)>)> = self.data.iter().rev().take(max_bars).collect::<Vec<_>>().into_iter().rev().collect();

        let max_total: i64 = entries.iter()
            .map(|(_, practices)| practices.iter().map(|(_, c)| c).sum::<i64>())
            .max()
            .unwrap_or(1)
            .max(1);

        // Render vertical bars
        for (col, (date, practices)) in entries.iter().enumerate() {
            let x = area.x + (col as u16) * bar_slot;
            if x + 1 >= area.x + area.width {
                break;
            }

            let total: i64 = practices.iter().map(|(_, c)| c).sum();
            let bar_height = if max_total > 0 {
                ((total as f64 / max_total as f64) * chart_height as f64).ceil() as usize
            } else {
                0
            };

            // Build stacked segments from bottom up
            let mut segments: Vec<(usize, Color)> = Vec::new();
            let mut remaining = bar_height;
            for (name, count) in practices.iter() {
                let seg_h = if total > 0 {
                    ((*count as f64 / total as f64) * bar_height as f64).round() as usize
                } else {
                    0
                };
                let seg_h = seg_h.min(remaining);
                let color_idx = practice_names.iter().position(|n| n == name).unwrap_or(0);
                let color = if self.no_color { Color::White } else { practice_color(color_idx) };
                if seg_h > 0 {
                    segments.push((seg_h, color));
                    remaining -= seg_h;
                }
            }

            // Render from bottom
            let base_y = area.y + chart_height as u16 - 1;
            let mut filled = 0usize;
            for (seg_h, color) in &segments {
                for i in 0..*seg_h {
                    let y = base_y - (filled + i) as u16;
                    if y >= area.y {
                        buf.set_string(x, y, "\u{2588}\u{2588}", Style::default().fg(*color));
                    }
                }
                filled += seg_h;
            }

            // X-axis label (day number)
            let label_y = area.y + chart_height as u16;
            if label_y < area.y + area.height {
                let day = if date.len() >= 10 { &date[8..10] } else { "" };
                buf.set_string(x, label_y, day, Style::default().fg(Color::Gray));
            }
        }

        // Legend row
        let legend_y = area.y + chart_height as u16 + 1;
        if legend_y < area.y + area.height {
            let mut x = area.x;
            for (i, name) in practice_names.iter().take(6).enumerate() {
                let color = if self.no_color { Color::White } else { practice_color(i) };
                if x + 2 + name.len() as u16 >= area.x + area.width {
                    break;
                }
                buf.set_string(x, legend_y, "\u{25A0}", Style::default().fg(color));
                x += 2;
                buf.set_string(x, legend_y, name, Style::default().fg(Color::Gray));
                x += name.len() as u16 + 2;
            }
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/tui/widgets/daily_chart.rs src/tui/widgets/mod.rs
git commit -m "feat(heatmap): add daily stacked vertical bar chart widget (Chart view)"
```

---

### Task 7: Dashboard View Cycling Integration

**Files:**
- Modify: `src/tui/dashboard.rs`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

Add Tab key to cycle between Map/Chart/Days/Months views on the dashboard.

- [ ] **Step 1: Add data fields and view state to DashboardScreen**

In `src/tui/dashboard.rs`, add these fields to the struct (after `heatmap_data`):

```rust
    daily_data: Vec<(String, Vec<(String, i64)>)>,
    weekday_data: Vec<(u32, Vec<(String, i64)>)>,
    monthly_data: Vec<(u32, Vec<(String, i64)>)>,
    heatmap_view: usize,  // 0=Map, 1=Chart, 2=Days, 3=Months
```

- [ ] **Step 2: Add imports**

Add to the imports in dashboard.rs:

```rust
use super::widgets::daily_chart::DailyChart;
use super::widgets::weekday_chart::WeekdayChart;
use super::widgets::monthly_chart::MonthlyChart;
```

- [ ] **Step 3: Initialize in new() and refresh()**

In `new()`, after the heatmap_counts line, add:

```rust
let daily_data = db.daily_practice_counts(90)?;
let weekday_data = db.weekday_practice_counts(365)?;
let monthly_data = db.monthly_practice_counts(Local::now().year())?;
```

And include them in the returned struct: `daily_data, weekday_data, monthly_data, heatmap_view: 0,`

In `refresh()`, add after the heatmap_counts line:

```rust
self.daily_data = db.daily_practice_counts(90)?;
self.weekday_data = db.weekday_practice_counts(365)?;
self.monthly_data = db.monthly_practice_counts(Local::now().year())?;
```

- [ ] **Step 4: Replace heatmap rendering with view dispatch**

Replace the heatmap rendering lines (`let heatmap = Heatmap::new(...); frame.render_widget(heatmap, chunks[1]);`) with:

```rust
        // ── Visualization view ──
        let view_names = [
            tr("heatmap-view-map"),
            tr("heatmap-view-chart"),
            tr("heatmap-view-days"),
            tr("heatmap-view-months"),
        ];
        let view_label = format!("[{}]", view_names[self.heatmap_view]);
        let label_area = Rect::new(
            chunks[1].x + chunks[1].width.saturating_sub(view_label.len() as u16 + 1),
            chunks[1].y,
            view_label.len() as u16 + 1,
            1,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&view_label, Style::default().fg(Color::DarkGray)))),
            label_area,
        );

        match self.heatmap_view {
            0 => {
                let heatmap = Heatmap::new(&self.heatmap_data, 52, self.no_color);
                frame.render_widget(heatmap, chunks[1]);
            }
            1 => {
                let chart = DailyChart::new(&self.daily_data, self.no_color);
                frame.render_widget(chart, chunks[1]);
            }
            2 => {
                let chart = WeekdayChart::new(&self.weekday_data, self.no_color);
                frame.render_widget(chart, chunks[1]);
            }
            3 => {
                let chart = MonthlyChart::new(&self.monthly_data, self.no_color);
                frame.render_widget(chart, chunks[1]);
            }
            _ => {}
        }
```

- [ ] **Step 5: Add Tab keybinding in handle_normal()**

In `handle_normal()`, add before the `_ => Action::None` arm:

```rust
            KeyCode::Tab => {
                self.heatmap_view = (self.heatmap_view + 1) % 4;
                Action::None
            }
```

- [ ] **Step 6: Add Tab to footer**

In the Normal mode footer spans, add:

```rust
                Span::styled("[Tab]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-view")), Style::default().fg(Color::Gray)),
```

- [ ] **Step 7: Add i18n keys**

In `locales/en.ftl`:

```
heatmap-view-map = Map
heatmap-view-chart = Chart
heatmap-view-days = Days
heatmap-view-months = Months
key-view = View
```

In `locales/zh-CN.ftl`:

```
heatmap-view-map = 热力图
heatmap-view-chart = 日图
heatmap-view-days = 周图
heatmap-view-months = 月图
key-view = 切换视图
```

- [ ] **Step 8: Verify compilation and all tests**

Run: `cargo check && cargo test`

- [ ] **Step 9: Commit**

```bash
git add src/tui/dashboard.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(heatmap): add Tab cycling between Map/Chart/Days/Months views"
```

---

### Task 8: Update Lint Test and README

**Files:**
- Modify: `tests/tui_lint_test.rs`
- Modify: `README.md`

- [ ] **Step 1: Update tui_lint_test.rs allowlist**

The new widget files use `buf.set_string()` directly (ratatui Buffer API), not `Paragraph::new()`, so they won't trigger the lint test. But check if any new `Paragraph::new()` calls were added and update the allowlist if needed.

Run: `cargo test --test tui_lint_test`

If it fails, update the allowlist with the new line numbers for any shifted entries in dashboard.rs.

- [ ] **Step 2: Update README**

Add a section documenting:
- Tab key cycles heatmap views on Dashboard
- Map: GitHub-style contribution grid
- Chart: daily vertical bars stacked by practice
- Days: weekday horizontal bars stacked by practice  
- Months: monthly horizontal bars stacked by practice

- [ ] **Step 3: Final verification**

Run: `cargo test && cargo clippy`

- [ ] **Step 4: Commit**

```bash
git add tests/tui_lint_test.rs README.md
git commit -m "docs: add multi-view heatmap documentation, update lint test"
```

---

## Verification

1. `cargo test` — all tests pass (existing + 3 new DB tests)
2. `cargo clippy` — no warnings
3. `cargo run` → Dashboard → verify Map view renders with new colors and month gaps
4. Press `Tab` → verify Chart view shows daily vertical stacked bars
5. Press `Tab` → verify Days view shows weekday horizontal stacked bars
6. Press `Tab` → verify Months view shows monthly horizontal stacked bars
7. Press `Tab` → cycles back to Map
8. Verify legend shows practice names with correct colors in all bar views
