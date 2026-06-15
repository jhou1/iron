use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

const ACCENT: Color = Color::Cyan;

/// A vertical bar chart widget.
///
/// Takes `Vec<(String, f64)>` where String is a label (e.g. date) and f64 is the value.
/// Renders vertical bars from the bottom up, with green color intensity proportional
/// to each value relative to the max. Overlays a linear regression trendline and
/// compact value labels above each bar.
pub struct Sparkline {
    data: Vec<(String, f64)>,
}

impl Sparkline {
    pub fn new(data: Vec<(String, f64)>) -> Self {
        Self { data }
    }

    /// Returns a green shade based on the ratio (0.0..=1.0).
    /// Uses ANSI colors for terminal theme compatibility.
    fn green_for_ratio(ratio: f64) -> Color {
        if ratio < 0.33 {
            Color::DarkGray
        } else if ratio < 0.66 {
            Color::Green
        } else {
            Color::LightGreen
        }
    }
}

impl Widget for Sparkline {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Minimum area: height >= 3, width >= 4, data not empty
        if area.height < 4 || area.width < 4 || self.data.is_empty() {
            return;
        }

        // Reserve 2 rows for x-axis labels (day + month) at the bottom,
        // and columns on the right for y-axis labels.
        let y_label_width: u16 = 6; // right-side y-axis labels (e.g. "1234.5")
        let chart_width = area.width.saturating_sub(y_label_width + 1); // +1 gap
        let chart_height = area.height.saturating_sub(2); // 2 rows for x-axis labels

        if chart_width < 3 || chart_height < 2 {
            return;
        }

        // Each bar is 2 chars wide + 1 gap = 3 chars per bar slot
        let bar_slot = 3u16; // 2 bar + 1 gap
        let max_bars = (chart_width / bar_slot) as usize;
        if max_bars == 0 {
            return;
        }

        // Take the most recent entries (from right), limited by available width
        let start = if self.data.len() > max_bars {
            self.data.len() - max_bars
        } else {
            0
        };
        let visible = &self.data[start..];

        // Find max value
        let max_val = visible.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);
        let max_val = if max_val == 0.0 { 1.0 } else { max_val };

        // Draw bars (bottom-up)
        let chart_top = area.y;
        let chart_bottom = area.y + chart_height - 1; // last row of chart area

        for (i, (label, value)) in visible.iter().enumerate() {
            let x = area.x + (i as u16) * bar_slot;
            let ratio = *value / max_val;
            let bar_height = ((ratio * (chart_height as f64 - 1.0)).round() as u16)
                .max(if *value > 0.0 { 1 } else { 0 });
            let color = Sparkline::green_for_ratio(ratio);

            // Draw bar cells from bottom up
            for row in 0..bar_height {
                let y = chart_bottom - row;
                if y >= chart_top && x + 1 < area.x + area.width {
                    buf.set_string(x, y, "\u{2588}\u{2588}", Style::default().fg(color));
                }
            }

            // Draw compact value label above the bar
            if *value > 0.0 {
                let num_str = format_compact(*value);
                let num_row = chart_bottom.saturating_sub(bar_height + 1).max(chart_top);
                if x < area.x + area.width {
                    buf.set_string(x, num_row, &num_str, Style::default().fg(Color::Gray));
                }
            }

            // Draw x-axis labels below the chart.
            // Labels can contain "day\nmonth" (e.g., "21\nJan") for month boundaries,
            // or just "day" (e.g., "26") for regular entries.
            let day_row = area.y + chart_height;
            let month_row = day_row + 1;
            let parts: Vec<&str> = label.splitn(2, '\n').collect();
            if day_row < area.y + area.height {
                let day_label: String = parts[0].chars().take(2).collect();
                buf.set_string(x, day_row, &day_label, Style::default().fg(Color::Gray));
            }
            if parts.len() > 1 && month_row < area.y + area.height {
                let month_label: String = parts[1].chars().take(3).collect();
                buf.set_string(x, month_row, &month_label, Style::default().fg(ACCENT));
            }
        }

        // Y-axis labels on the right side
        let y_label_x = area.x + chart_width + 1;
        if y_label_x + y_label_width <= area.x + area.width {
            // Max value at the top
            let max_str = format_compact(max_val);
            buf.set_string(
                y_label_x,
                chart_top,
                &max_str,
                Style::default().fg(Color::Gray),
            );

            // "0" at the bottom
            buf.set_string(
                y_label_x,
                chart_bottom,
                "0",
                Style::default().fg(Color::Gray),
            );
        }
    }
}

/// Format a number compactly for y-axis labels.
fn format_compact(v: f64) -> String {
    if v >= 10000.0 {
        format!("{:.0}k", v / 1000.0)
    } else if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else if v == v.floor() {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}
