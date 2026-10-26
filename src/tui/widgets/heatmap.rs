use chrono::{Datelike, Duration, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

/// A GitHub-style contribution heatmap widget.
pub struct Heatmap<'a> {
    data: &'a [(String, i64)],
    weeks: u16,
    no_color: bool,
    today: chrono::NaiveDate,
}

impl<'a> Heatmap<'a> {
    pub fn new(data: &'a [(String, i64)], weeks: u16, no_color: bool) -> Self {
        Self {
            data,
            weeks,
            no_color,
            today: Local::now().date_naive(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_today(
        data: &'a [(String, i64)],
        weeks: u16,
        no_color: bool,
        today: chrono::NaiveDate,
    ) -> Self {
        Self {
            data,
            weeks,
            no_color,
            today,
        }
    }

    fn cell_symbol(count: i64) -> &'static str {
        if count > 0 {
            "✔"
        } else {
            "·"
        }
    }

    fn cell_color(&self, count: i64) -> Color {
        if self.no_color {
            if count > 0 {
                Color::White
            } else {
                Color::DarkGray
            }
        } else if count > 0 {
            Color::Green
        } else {
            Color::Indexed(240) // visible gray on dark terminals
        }
    }
}

impl<'a> Widget for Heatmap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 9 || area.width < 10 {
            return;
        }

        // Build lookup map from date string -> count
        let counts: HashMap<String, i64> = self.data.iter().cloned().collect();

        let today = self.today;

        // Find the Monday of the current week. Each column is one week (Mon..Sun).
        let today_weekday = today.weekday().num_days_from_monday(); // Mon=0..Sun=6
        let current_week_monday = today - Duration::days(today_weekday as i64);

        // Day labels on the left, shifted down 1 row for month labels
        let day_labels = [
            crate::i18n::tr("heatmap-mon"),
            crate::i18n::tr("heatmap-tue"),
            crate::i18n::tr("heatmap-wed"),
            crate::i18n::tr("heatmap-thu"),
            crate::i18n::tr("heatmap-fri"),
            crate::i18n::tr("heatmap-sat"),
            crate::i18n::tr("heatmap-sun"),
        ];
        let label_width = day_labels.iter().map(|l| l.width()).max().unwrap_or(2) as u16 + 1;
        let month_row_y = area.y;
        let grid_y = area.y + 1; // grid starts 1 row below for month labels

        // Render day labels
        for (row, label) in day_labels.iter().enumerate() {
            let y = grid_y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            buf.set_string(area.x, y, label, Style::default().fg(Color::Gray));
        }

        // Render the heatmap cells
        let cell_width: u16 = 2; // 1 day marker + 1 space

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

        let mut last_labeled_month: Option<u32> = None;
        let mut last_label_end_x: u16 = 0;
        let cols_available = area.width.saturating_sub(label_width);
        let num_weeks = (cols_available / cell_width).min(self.weeks);
        let start_monday = if num_weeks == 0 {
            current_week_monday
        } else {
            current_week_monday - Duration::weeks((num_weeks as i64) - 1)
        };

        for week in 0..num_weeks {
            let week_monday = start_monday + Duration::weeks(week as i64);
            let month = week_monday.month();

            let x = area.x + label_width + week * cell_width;
            if x + 1 >= area.x + area.width {
                break;
            }

            if last_labeled_month != Some(month) {
                let label = &month_names[(month - 1) as usize];
                let label_len = label.width() as u16;
                let label_x = x.max(last_label_end_x);
                if label_x + label_len <= area.x + area.width {
                    buf.set_string(
                        label_x,
                        month_row_y,
                        label,
                        Style::default().fg(Color::Gray),
                    );
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
                let color = self.cell_color(count);
                let y = grid_y + day;
                if y < area.y + area.height && x < area.x + area.width {
                    buf.set_string(x, y, Self::cell_symbol(count), Style::default().fg(color));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    const IDLE_MARKER: &str = "·";
    const TRAINED_MARKER: &str = "✔";

    #[test]
    fn heatmap_renders_when_area_height_is_9() {
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::with_today(&data, 52, false, "2026-05-16".parse().unwrap());
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 9));
        heatmap.render(Rect::new(0, 0, 30, 9), &mut buf);
        // At least one cell should use the active-day Unicode marker.
        let has_content = buf
            .content
            .iter()
            .any(|cell| cell.symbol() == TRAINED_MARKER);
        assert!(
            has_content,
            "heatmap should render active-day Unicode markers when area.height >= 9"
        );
    }

    #[test]
    fn heatmap_renders_nothing_when_area_height_is_8() {
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::new(&data, 52, false);
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
        heatmap.render(Rect::new(0, 0, 30, 8), &mut buf);
        let has_content = buf
            .content
            .iter()
            .any(|cell| cell.symbol() == TRAINED_MARKER);
        assert!(
            !has_content,
            "heatmap should render nothing when area.height < 9"
        );
    }

    #[test]
    fn heatmap_marks_training_and_non_training_days_in_unicode() {
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::with_today(&data, 52, false, "2026-05-16".parse().unwrap());
        let mut buf = Buffer::empty(Rect::new(0, 0, 110, 9));
        heatmap.render(Rect::new(0, 0, 110, 9), &mut buf);

        let mut marker_columns = Vec::new();
        for y in 1..=7_u16 {
            for x in 0..buf.area.width {
                if let Some(symbol) = symbol_at(&buf, x, y) {
                    if symbol == TRAINED_MARKER || symbol == IDLE_MARKER {
                        marker_columns.push(x);
                    }
                }
            }
        }
        let rightmost_x = marker_columns
            .into_iter()
            .max()
            .expect("heatmap should render day markers");

        assert_eq!(
            symbol_at(&buf, rightmost_x, 6),
            Some(TRAINED_MARKER),
            "the active Saturday should use ✔"
        );
        assert_eq!(
            symbol_at(&buf, rightmost_x, 5),
            Some(IDLE_MARKER),
            "the inactive Friday should use ·"
        );
        assert!(
            row_symbols(&buf, 8).trim().is_empty(),
            "heatmap should not render an explanatory legend row"
        );
        assert!(
            !buf.content.iter().any(|cell| cell.symbol() == "\u{25CF}"),
            "heatmap should not retain the old circle marker"
        );
    }

    #[test]
    fn heatmap_keeps_recent_training_days_visible_in_narrow_terminals() {
        let data = vec![("2026-08-17".to_string(), 1), ("2026-08-18".to_string(), 3)];
        let heatmap = Heatmap::with_today(&data, 52, false, "2026-08-18".parse().unwrap());
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 9));
        heatmap.render(Rect::new(0, 0, 80, 9), &mut buf);

        let mut marker_columns = Vec::new();
        for y in 1..=7_u16 {
            for x in 0..buf.area.width {
                if let Some(symbol) = symbol_at(&buf, x, y) {
                    if symbol == TRAINED_MARKER || symbol == IDLE_MARKER {
                        marker_columns.push(x);
                    }
                }
            }
        }
        let current_week_x = marker_columns
            .into_iter()
            .max()
            .expect("heatmap should render day markers");

        assert_eq!(
            symbol_at(&buf, current_week_x, 1),
            Some(TRAINED_MARKER),
            "Monday's training should remain visible in an 80-column terminal"
        );
        assert_eq!(
            symbol_at(&buf, current_week_x, 2),
            Some(TRAINED_MARKER),
            "Tuesday's training should remain visible in an 80-column terminal"
        );
    }

    #[test]
    fn heatmap_renders_with_padded_block_inner_area() {
        use ratatui::widgets::{Block, Borders, Padding};
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::with_today(&data, 52, false, "2026-05-16".parse().unwrap());
        // A Block with Borders::ALL + Padding::uniform(1) on a 13-row area produces inner height 9
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        let area = Rect::new(0, 0, 30, 13);
        let inner = block.inner(area);
        assert_eq!(inner.height, 9, "inner height should be exactly 9");
        let mut buf = Buffer::empty(area);
        block.render(area, &mut buf);
        heatmap.render(inner, &mut buf);
        let has_content = buf
            .content
            .iter()
            .any(|cell| cell.symbol() == TRAINED_MARKER);
        assert!(
            has_content,
            "heatmap should render inside a padded 13-row block"
        );
    }

    fn symbol_at(buf: &Buffer, x: u16, y: u16) -> Option<&str> {
        buf.content
            .get(y as usize * buf.area.width as usize + x as usize)
            .map(|cell| cell.symbol())
    }

    fn row_symbols(buf: &Buffer, y: u16) -> String {
        buf.content[y as usize * buf.area.width as usize..]
            .iter()
            .take(buf.area.width as usize)
            .map(|cell| cell.symbol().to_string())
            .collect()
    }
}
