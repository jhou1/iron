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
}

impl<'a> Heatmap<'a> {
    pub fn new(data: &'a [(String, i64)], weeks: u16) -> Self {
        Self { data, weeks }
    }

    fn cell_for_count(count: i64) -> (&'static str, Color) {
        match count {
            0 => ("\u{25AA}", Color::DarkGray),                  // ▪ small square
            1 => ("\u{25A0}", Color::Indexed(65)),               // ■ muted teal
            2 => ("\u{25A0}", Color::Indexed(71)),               // ■ medium green
            _ => ("\u{25A0}", Color::Indexed(118)),              // ■ bright lime
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

        // Day labels on the left (3 chars wide), shifted down 1 row for month labels
        let day_labels = [
            crate::i18n::tr("heatmap-mon"),
            crate::i18n::tr("heatmap-tue"),
            crate::i18n::tr("heatmap-wed"),
            crate::i18n::tr("heatmap-thu"),
            crate::i18n::tr("heatmap-fri"),
            crate::i18n::tr("heatmap-sat"),
            crate::i18n::tr("heatmap-sun"),
        ];
        let label_width: u16 = 3;
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
        let cols_available = area.width.saturating_sub(label_width);
        let cell_width: u16 = 2; // "■ " — square + 1 space
        let num_weeks = (cols_available / cell_width).min(self.weeks);

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

        // Track which months to label: place label at the first week column of each month
        let mut last_labeled_month: Option<u32> = None;

        for week in 0..num_weeks {
            // Determine the month for this week's Monday
            let week_monday = start_monday + Duration::weeks(week as i64);
            let month = week_monday.month();

            // Place month label at the first week column of each new month
            if last_labeled_month != Some(month) {
                let x = area.x + label_width + week * cell_width;
                let label = &month_names[(month - 1) as usize];
                let label_len = label.width() as u16;
                if x + label_len <= area.x + area.width {
                    buf.set_string(x, month_row_y, label, Style::default().fg(Color::Gray));
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
                let (ch, color) = Heatmap::cell_for_count(count);

                let x = area.x + label_width + week * cell_width;
                let y = grid_y + day;
                if y < area.y + area.height && x + 1 < area.x + area.width {
                    buf.set_string(x, y, ch, Style::default().fg(color));
                }
            }
        }

        // Render legend row below the 7 day rows
        let legend_y = grid_y + 7;
        if legend_y < area.y + area.height {
            let legend = Line::from(vec![
                Span::styled(format!("{} ", crate::i18n::tr("heatmap-less")), Style::default().fg(Color::Gray)),
                Span::styled("\u{25AA}", Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled("\u{25A0}", Style::default().fg(Color::Indexed(65))),
                Span::raw(" "),
                Span::styled("\u{25A0}", Style::default().fg(Color::Indexed(71))),
                Span::raw(" "),
                Span::styled("\u{25A0}", Style::default().fg(Color::Indexed(118))),
                Span::styled(format!(" {}", crate::i18n::tr("heatmap-more")), Style::default().fg(Color::Gray)),
            ]);
            buf.set_line(area.x + label_width, legend_y, &legend, area.width.saturating_sub(label_width));
        }
    }
}
