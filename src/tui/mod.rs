pub mod abbreviations;
pub mod dashboard;
pub mod goals;
pub mod history;
pub mod log_entry;
pub mod practices;
pub mod quick_log;
pub mod quotes;
pub mod trends;
pub mod widgets;

use ratatui::{
    layout::{Alignment, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const HIGHLIGHT_BG: Color = Color::DarkGray;
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

pub fn render_help_overlay(frame: &mut Frame, area: Rect, bindings: &[(&str, &str)]) {
    if bindings.is_empty() {
        return;
    }

    let max_line_width = bindings
        .iter()
        .map(|(key, desc)| key.len() + desc.len() + 2) // 2 for "  " separator
        .max()
        .unwrap_or(0);
    let box_width = (max_line_width + 6) as u16; // padding
    let box_height = (bindings.len() + 3) as u16; // borders + title

    let box_width = box_width.min(area.width);
    let box_height = box_height.min(area.height);

    let x = area.x + (area.width.saturating_sub(box_width)) / 2;
    let y = area.y + (area.height.saturating_sub(box_height)) / 2;
    let overlay_area = Rect::new(x, y, box_width, box_height);

    frame.render_widget(Clear, overlay_area);

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("[{key}]"), Style::default().fg(Color::Cyan)),
                Span::raw(format!("  {desc}")),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, overlay_area);
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
}

pub enum Action {
    None,
    Navigate(Screen),
    Quit,
}
