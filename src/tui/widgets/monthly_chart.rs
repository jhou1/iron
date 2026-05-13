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
        if area.height < 9 || area.width < 20 { return; }

        let month_names = [
            crate::i18n::tr("heatmap-jan"), crate::i18n::tr("heatmap-feb"),
            crate::i18n::tr("heatmap-mar"), crate::i18n::tr("heatmap-apr"),
            crate::i18n::tr("heatmap-may"), crate::i18n::tr("heatmap-jun"),
            crate::i18n::tr("heatmap-jul"), crate::i18n::tr("heatmap-aug"),
            crate::i18n::tr("heatmap-sep"), crate::i18n::tr("heatmap-oct"),
            crate::i18n::tr("heatmap-nov"), crate::i18n::tr("heatmap-dec"),
        ];

        let label_width: u16 = 5;
        let bar_area_width = area.width.saturating_sub(label_width) as usize;
        let max_rows = (area.height.saturating_sub(1)) as usize; // reserve 1 for legend

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

        let max_total: i64 = self.data.iter()
            .map(|(_, practices)| practices.iter().map(|(_, c)| c).sum::<i64>())
            .max().unwrap_or(1).max(1);

        // Render each month bar (1..=12)
        for (row, month) in (1u32..=12).enumerate() {
            if row >= max_rows { break; }
            let y = area.y + row as u16;

            let label = &month_names[(month - 1) as usize];
            buf.set_string(area.x, y, format!("{:>4} ", label), Style::default().fg(Color::Gray));

            let practices = self.data.iter()
                .find(|(m, _)| *m == month)
                .map(|(_, p)| p.as_slice())
                .unwrap_or(&[]);

            let total: i64 = practices.iter().map(|(_, c)| c).sum();
            let bar_width = ((total as f64 / max_total as f64) * bar_area_width as f64) as usize;

            let mut x_offset = 0usize;
            for (name, count) in practices {
                let seg_width = if total > 0 {
                    ((*count as f64 / total as f64) * bar_width as f64).round() as usize
                } else { 0 };
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
        let legend_y = area.y + max_rows.min(12) as u16;
        if legend_y < area.y + area.height {
            let mut x = area.x + label_width;
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
