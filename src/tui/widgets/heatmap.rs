use chrono::{Datelike, Duration, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

/// A GitHub-style contribution heatmap widget.
pub struct Heatmap<'a> {
    data: &'a [(String, i64)],
    weeks: u16,
    no_color: bool,
}

impl<'a> Heatmap<'a> {
    pub fn new(data: &'a [(String, i64)], weeks: u16, no_color: bool) -> Self {
        Self { data, weeks, no_color }
    }

    fn cell_color(&self, count: i64) -> Color {
        if self.no_color {
            match count {
                0 => Color::DarkGray,
                1 => Color::DarkGray,
                2 => Color::Gray,
                _ => Color::White,
            }
        } else {
            match count {
                0 => Color::Indexed(240),  // visible gray on dark terminals
                1 => Color::Indexed(22),   // dark green
                2 => Color::Indexed(28),   // medium green
                3 => Color::Indexed(34),   // bright green
                _ => Color::Indexed(82),   // vivid green
            }
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

        let today = Local::now().date_naive();

        // Calculate the start date: we want `weeks` columns of 7 days each.
        // Each column is one week (Mon..Sun). The rightmost column contains today.
        // Find the Monday of the current week.
        let today_weekday = today.weekday().num_days_from_monday(); // Mon=0..Sun=6
        let current_week_monday = today - Duration::days(today_weekday as i64);
        let start_monday = current_week_monday - Duration::weeks((self.weeks as i64) - 1);

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
        let label_width = day_labels.iter()
            .map(|l| l.width())
            .max()
            .unwrap_or(2) as u16 + 1;
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
        let cell_width: u16 = 2; // 1 circle char + 1 space

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
                let color = self.cell_color(count);
                let y = grid_y + day;
                if y < area.y + area.height && x < area.x + area.width {
                    buf.set_string(x, y, "\u{25CF}", Style::default().fg(color));
                }
            }
        }

        // Render legend row below the 7 day rows
        let legend_y = grid_y + 7;
        if legend_y < area.y + area.height {
            let legend = if self.no_color {
                Line::from(vec![
                    Span::styled(format!("{} ", crate::i18n::tr("heatmap-less")), Style::default()),
                    Span::raw(" "), Span::raw("\u{2591}"), Span::raw(" "),
                    Span::raw("\u{2592}"), Span::raw(" "), Span::raw("\u{2588}"),
                    Span::styled(format!(" {}", crate::i18n::tr("heatmap-more")), Style::default()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("{} ", crate::i18n::tr("heatmap-less")), Style::default().fg(Color::Gray)),
                    Span::styled("\u{25CF}", Style::default().fg(Color::Indexed(240))), Span::raw(" "),
                    Span::styled("\u{25CF}", Style::default().fg(Color::Indexed(22))),  Span::raw(" "),
                    Span::styled("\u{25CF}", Style::default().fg(Color::Indexed(28))),  Span::raw(" "),
                    Span::styled("\u{25CF}", Style::default().fg(Color::Indexed(34))),  Span::raw(" "),
                    Span::styled("\u{25CF}", Style::default().fg(Color::Indexed(82))),
                    Span::styled(format!(" {}", crate::i18n::tr("heatmap-more")), Style::default().fg(Color::Gray)),
                ])
            };
            buf.set_line(area.x + label_width, legend_y, &legend, area.width.saturating_sub(label_width));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn heatmap_renders_when_area_height_is_9() {
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::new(&data, 52, false);
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 9));
        heatmap.render(Rect::new(0, 0, 30, 9), &mut buf);
        // At least one cell should have the circle character
        let has_content = buf.content.iter().any(|cell| cell.symbol() == "\u{25CF}");
        assert!(has_content, "heatmap should render circles when area.height >= 9");
    }

    #[test]
    fn heatmap_renders_nothing_when_area_height_is_8() {
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::new(&data, 52, false);
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
        heatmap.render(Rect::new(0, 0, 30, 8), &mut buf);
        let has_content = buf.content.iter().any(|cell| cell.symbol() == "\u{25CF}");
        assert!(!has_content, "heatmap should render nothing when area.height < 9");
    }

    #[test]
    fn heatmap_renders_with_padded_block_inner_area() {
        use ratatui::widgets::{Block, Borders, Padding};
        let data = vec![("2026-05-16".to_string(), 5i64)];
        let heatmap = Heatmap::new(&data, 52, false);
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
        let has_content = buf.content.iter().any(|cell| cell.symbol() == "\u{25CF}");
        assert!(has_content, "heatmap should render inside a padded 13-row block");
    }
}
