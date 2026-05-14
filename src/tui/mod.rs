pub mod abbreviations;
pub mod dashboard;
pub mod goals;
pub mod history;
pub mod log_entry;
pub mod practices;
pub mod quick_log;
pub mod quotes;
pub mod quotes_screen;
pub mod trends;
pub mod widgets;

use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const HIGHLIGHT_BG: Color = Color::Rgb(204, 163, 0);
const HIGHLIGHT_FG: Color = Color::Black;
pub const BORDER_COLOR: Color = Color::DarkGray;
pub const GAUGE_FILL: Color = Color::Green;
pub const GAUGE_EMPTY: Color = Color::Indexed(240);
pub const CONTENT_WIDTH: u16 = 3 + 52 * 2;

pub fn centered_area(full: Rect, max_width: u16) -> Rect {
    let width = full.width.min(max_width);
    let x = full.x + (full.width.saturating_sub(width)) / 2;
    Rect { x, y: full.y, width, height: full.height }
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

pub fn visible_input_spans<'a>(
    text: &'a str,
    cursor: usize,
    max_width: u16,
    prefix_width: u16,
    color: Color,
) -> Vec<Span<'a>> {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    let avail = (max_width.saturating_sub(prefix_width).saturating_sub(1)) as usize; // 1 for cursor char
    let before = &text[..cursor];
    let after = &text[cursor..];
    let before_w = before.width();
    if before_w + after.width() <= avail {
        return vec![
            Span::styled(before, Style::default().fg(color)),
            Span::styled("_", Style::default().fg(color)),
            Span::styled(after, Style::default().fg(color)),
        ];
    }
    let scroll = before_w.saturating_sub(avail);
    let mut visible_before = before;
    let mut skipped = 0;
    for (i, ch) in before.char_indices() {
        skipped += ch.width().unwrap_or(0);
        if skipped >= scroll {
            visible_before = &before[i + ch.len_utf8()..];
            break;
        }
    }
    let remaining = avail.saturating_sub(visible_before.width());
    let mut end = 0;
    let mut used = 0;
    for (i, ch) in after.char_indices() {
        let w = ch.width().unwrap_or(0);
        if used + w > remaining {
            break;
        }
        used += w;
        end = i + ch.len_utf8();
    }
    let visible_after = &after[..end];
    vec![
        Span::styled(visible_before, Style::default().fg(color)),
        Span::styled("_", Style::default().fg(color)),
        Span::styled(visible_after, Style::default().fg(color)),
    ]
}

pub type StatusMessage = Option<(String, bool)>; // (message, is_error)

pub fn render_status_line(frame: &mut Frame, area: Rect, status: &StatusMessage) {
    if let Some((msg, is_error)) = status {
        let color = if *is_error { Color::Red } else { Color::Green };
        let line = Line::from(Span::styled(msg.as_str(), Style::default().fg(color)));
        frame.render_widget(Paragraph::new(line), area);
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
    QuickLog,
    Abbreviations,
    Quotes,
}

pub enum Action {
    None,
    Navigate(Screen),
    Quit,
}

pub fn render_gauge_line<'a>(ratio: f64, _done: usize, _total: usize, bar_width: usize, indent: usize) -> Line<'a> {
    let filled = (ratio * bar_width as f64).round() as usize;
    let empty = bar_width - filled;

    Line::from(vec![
        Span::raw(" ".repeat(indent)),
        Span::styled("\u{2588}".repeat(filled), Style::default().fg(GAUGE_FILL)),
        Span::styled("\u{2588}".repeat(empty), Style::default().fg(GAUGE_EMPTY)),
    ])
}
