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
        if area.height < 4 || area.width < 10 { return; }

        // Collect unique practice names
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

        let chart_height = area.height.saturating_sub(2) as usize; // 1 x-axis + 1 legend
        let bar_slot = 3u16; // 2 bar chars + 1 gap
        let max_bars = (area.width / bar_slot) as usize;

        // Take most recent entries that fit
        let entries: Vec<&(String, Vec<(String, i64)>)> = self.data.iter()
            .rev().take(max_bars).collect::<Vec<_>>().into_iter().rev().collect();

        let max_total: i64 = entries.iter()
            .map(|(_, practices)| practices.iter().map(|(_, c)| c).sum::<i64>())
            .max().unwrap_or(1).max(1);

        // Render vertical bars
        for (col, (date, practices)) in entries.iter().enumerate() {
            let x = area.x + (col as u16) * bar_slot;
            if x + 1 >= area.x + area.width { break; }

            let total: i64 = practices.iter().map(|(_, c)| c).sum();
            let bar_height = if max_total > 0 {
                ((total as f64 / max_total as f64) * chart_height as f64).ceil() as usize
            } else { 0 };

            // Build stacked segments
            let mut segments: Vec<(usize, Color)> = Vec::new();
            let mut remaining = bar_height;
            for (name, count) in practices.iter() {
                let seg_h = if total > 0 {
                    ((*count as f64 / total as f64) * bar_height as f64).round() as usize
                } else { 0 };
                let seg_h = seg_h.min(remaining);
                let color_idx = practice_names.iter().position(|n| n == name).unwrap_or(0);
                let color = if self.no_color { Color::White } else { practice_color(color_idx) };
                if seg_h > 0 {
                    segments.push((seg_h, color));
                    remaining -= seg_h;
                }
            }

            // Render from bottom up
            let base_y = area.y + chart_height as u16 - 1;
            let mut filled = 0usize;
            for (seg_h, color) in &segments {
                for i in 0..*seg_h {
                    let y = base_y.saturating_sub((filled + i) as u16);
                    if y >= area.y {
                        buf.set_string(x, y, "\u{2588}\u{2588}", Style::default().fg(*color));
                    }
                }
                filled += seg_h;
            }

            // X-axis label (day number from date string)
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
                if x + 2 + name.len() as u16 >= area.x + area.width { break; }
                buf.set_string(x, legend_y, "\u{25A0}", Style::default().fg(color));
                x += 2;
                buf.set_string(x, legend_y, name, Style::default().fg(Color::Gray));
                x += name.len() as u16 + 2;
            }
        }
    }
}
