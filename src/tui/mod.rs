pub mod dashboard;
pub mod framework;
pub mod goals;
pub mod history;
pub(crate) mod lifecycle;
pub mod log_entry;
pub mod navigation;
pub mod practices;
pub mod quotes;
pub mod quotes_screen;
pub mod trends;
pub mod widgets;

use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    Frame,
};

pub use framework::components::{render_status_line, StatusMessage};
pub use navigation::{Action, ModuleId, NavigationIntent, ScreenId};

const HIGHLIGHT_BG: Color = Color::Rgb(204, 163, 0);
const HIGHLIGHT_FG: Color = Color::Black;
pub const BORDER_COLOR: Color = Color::DarkGray;
pub const GAUGE_FILL: Color = Color::Green;
pub const GAUGE_EMPTY: Color = Color::Indexed(240);
pub const CONTENT_WIDTH: u16 = 3 + 52 * 2;

pub fn centered_area(full: Rect, max_width: u16) -> Rect {
    let width = full.width.min(max_width);
    let x = full.x + (full.width.saturating_sub(width)) / 2;
    Rect {
        x,
        y: full.y,
        width,
        height: full.height,
    }
}

pub fn highlight_row(frame: &mut Frame, area: Rect, row: u16) {
    let y = area.y + row;
    if y >= area.y + area.height {
        return;
    }
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut(Position { x, y }) {
            cell.set_bg(HIGHLIGHT_BG);
            if cell.symbol() != "\u{2588}" {
                cell.set_fg(HIGHLIGHT_FG);
            }
        }
    }
}

pub fn render_gauge_line<'a>(
    ratio: f64,
    _done: usize,
    _total: usize,
    bar_width: usize,
    indent: usize,
) -> Line<'a> {
    let filled = (ratio * bar_width as f64).round() as usize;
    let empty = bar_width - filled;

    Line::from(vec![
        Span::raw(" ".repeat(indent)),
        Span::styled("\u{2588}".repeat(filled), Style::default().fg(GAUGE_FILL)),
        Span::styled("\u{2588}".repeat(empty), Style::default().fg(GAUGE_EMPTY)),
    ])
}
