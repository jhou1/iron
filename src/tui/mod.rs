pub mod dashboard;
pub mod goals;
pub mod history;
pub mod log_entry;
pub mod practices;
pub mod quotes;
pub mod trends;
pub mod widgets;

use ratatui::{
    layout::{Position, Rect},
    style::Color,
    Frame,
};

const HIGHLIGHT_BG: Color = Color::DarkGray;

pub fn highlight_row(frame: &mut Frame, area: Rect, row: u16) {
    let y = area.y + row;
    if y >= area.y + area.height {
        return;
    }
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut(Position { x, y }) {
            cell.set_bg(HIGHLIGHT_BG);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
    Goals,
}

pub enum Action {
    None,
    Navigate(Screen),
    Quit,
}
