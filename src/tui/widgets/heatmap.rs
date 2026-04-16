use chrono::{Datelike, Duration, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use std::collections::HashMap;

/// A GitHub-style contribution heatmap widget.
pub struct Heatmap<'a> {
    data: &'a [(String, i64)],
    weeks: u16,
}

impl<'a> Heatmap<'a> {
    pub fn new(data: &'a [(String, i64)], weeks: u16) -> Self {
        Self { data, weeks }
    }

    fn color_for_count(count: i64) -> Color {
        match count {
            0 => Color::Rgb(30, 30, 58),
            1 => Color::Rgb(45, 90, 45),
            2 => Color::Rgb(61, 139, 61),
            _ => Color::Rgb(78, 202, 78),
        }
    }
}

impl<'a> Widget for Heatmap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 8 || area.width < 10 {
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

        // Day labels on the left (3 chars wide)
        let day_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
        let label_width: u16 = 3; // "Mo " etc.

        // Render day labels
        for (row, label) in day_labels.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            buf.set_string(area.x, y, label, Style::default().fg(Color::DarkGray));
        }

        // Render the heatmap cells
        let cols_available = area.width.saturating_sub(label_width);
        let num_weeks = (cols_available / 2).min(self.weeks); // each cell is 2 chars wide ("█ ")

        for week in 0..num_weeks {
            for day in 0..7u16 {
                let date = start_monday + Duration::weeks(week as i64) + Duration::days(day as i64);
                if date > today {
                    continue;
                }
                let date_str = date.format("%Y-%m-%d").to_string();
                let count = counts.get(&date_str).copied().unwrap_or(0);
                let color = Heatmap::color_for_count(count);

                let x = area.x + label_width + week * 2;
                let y = area.y + day;
                if y < area.y + area.height && x + 1 < area.x + area.width {
                    buf.set_string(x, y, "\u{2588}", Style::default().fg(color));
                }
            }
        }

        // Render legend row below the 7 day rows
        let legend_y = area.y + 7;
        if legend_y < area.y + area.height {
            let legend = Line::from(vec![
                Span::styled("Less ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{2588}", Style::default().fg(Color::Rgb(30, 30, 58))),
                Span::raw(" "),
                Span::styled("\u{2588}", Style::default().fg(Color::Rgb(45, 90, 45))),
                Span::raw(" "),
                Span::styled("\u{2588}", Style::default().fg(Color::Rgb(61, 139, 61))),
                Span::raw(" "),
                Span::styled("\u{2588}", Style::default().fg(Color::Rgb(78, 202, 78))),
                Span::styled(" More", Style::default().fg(Color::DarkGray)),
            ]);
            buf.set_line(area.x + label_width, legend_y, &legend, area.width.saturating_sub(label_width));
        }
    }
}
