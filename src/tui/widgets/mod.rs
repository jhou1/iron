pub mod heatmap;
pub mod sparkline;

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
