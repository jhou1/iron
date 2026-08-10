use crate::tui::{highlight_row, BORDER_COLOR};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{visual_spans, ViewportState, VisualSpan};

pub type StatusMessage = Option<(String, bool)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub key: String,
    pub label: String,
}

impl Shortcut {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetailPanelState {
    pub focused: bool,
    pub viewport: ViewportState,
}

impl DetailPanelState {
    pub fn new(focused: bool) -> Self {
        Self {
            focused,
            viewport: ViewportState::default(),
        }
    }

    pub fn scroll_by(&mut self, delta: isize, content_height: usize) {
        self.viewport
            .set_offset(self.viewport.offset().saturating_add_signed(delta));
        self.viewport.clamp(content_height);
    }
}

pub fn render_scrollable_lines(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line<'static>>,
    viewport: &mut ViewportState,
    selected: Option<VisualSpan>,
) {
    viewport.set_height(area.height as usize);
    let spans = visual_spans(&lines, area.width);
    let content_height = spans
        .last()
        .map(|span| span.start.saturating_add(span.height))
        .unwrap_or(0);
    if let Some(selected) = selected {
        viewport.reveal(selected);
    }
    viewport.clamp(content_height);

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((viewport.offset().min(u16::MAX as usize) as u16, 0)),
        area,
    );

    if let Some(selected) = selected {
        let visible_start = selected.start.max(viewport.offset());
        let visible_end = selected
            .start
            .saturating_add(selected.height)
            .min(viewport.offset().saturating_add(area.height as usize));
        for row in visible_start..visible_end {
            highlight_row(frame, area, (row - viewport.offset()) as u16);
        }
    }
}

pub fn render_detail_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    lines: Vec<Line<'static>>,
    state: &mut DetailPanelState,
    selected: Option<VisualSpan>,
) {
    let title = title.into();
    let base_block = Block::default().borders(Borders::ALL);
    let inner = base_block.inner(area);
    state.viewport.set_height(inner.height as usize);

    let spans = visual_spans(&lines, inner.width);
    let content_height = spans
        .last()
        .map(|span| span.start.saturating_add(span.height))
        .unwrap_or(0);
    if let Some(selected) = selected {
        state.viewport.reveal(selected);
    }
    state.viewport.clamp(content_height);

    let suffix = state.viewport.overflow(content_height).suffix();
    let decorated_title = format!("{}{} ", title.trim_end(), suffix);
    let accent = if state.focused {
        Color::Cyan
    } else {
        BORDER_COLOR
    };
    let block = base_block
        .title(decorated_title)
        .title_style(Style::default().fg(accent))
        .border_style(Style::default().fg(accent));
    frame.render_widget(block, area);
    render_scrollable_lines(frame, inner, lines, &mut state.viewport, selected);
}

pub fn render_status_line(frame: &mut Frame, area: Rect, status: &StatusMessage) {
    if let Some((message, is_error)) = status {
        let color = if *is_error { Color::Red } else { Color::Green };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                message.clone(),
                Style::default().fg(color),
            )))
            .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn shortcut_lines(shortcuts: &[Shortcut], width: u16) -> Vec<Line<'static>> {
    let width = width.max(1) as usize;
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut used = 0;

    for shortcut in shortcuts {
        let key = format!("[{}]", shortcut.key);
        let label = format!(" {}", shortcut.label);
        let item_width = key.width() + label.width();
        let separator_width = usize::from(!spans.is_empty()) * 2;
        if !spans.is_empty() && used + separator_width + item_width > width {
            lines.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
            used += 2;
        }
        spans.push(Span::styled(key, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(label));
        used += item_width;
    }

    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

pub fn shortcut_height(shortcuts: &[Shortcut], width: u16) -> u16 {
    Paragraph::new(shortcut_lines(shortcuts, width))
        .wrap(Wrap { trim: false })
        .line_count(width)
        .min(u16::MAX as usize) as u16
}

pub fn render_shortcuts(frame: &mut Frame, area: Rect, shortcuts: &[Shortcut]) {
    frame.render_widget(
        Paragraph::new(shortcut_lines(shortcuts, area.width)).wrap(Wrap { trim: false }),
        area,
    );
}

pub fn truncate_to_width(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }

    let mut used = 0;
    text.chars()
        .take_while(|character| {
            let character_width = character.width().unwrap_or(0);
            if used + character_width > width {
                false
            } else {
                used += character_width;
                true
            }
        })
        .collect()
}

pub fn fit_rect(parent: Rect, preferred: Rect) -> Rect {
    let x = preferred.x.max(parent.x).min(parent.right());
    let y = preferred.y.max(parent.y).min(parent.bottom());
    Rect {
        x,
        y,
        width: preferred.width.min(parent.right().saturating_sub(x)),
        height: preferred.height.min(parent.bottom().saturating_sub(y)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::render_to_string;
    use ratatui::{
        layout::{Constraint, Layout},
        text::Line,
    };

    #[test]
    fn focused_detail_panel_shows_accent_and_down_overflow() {
        let mut state = DetailPanelState::new(true);
        let output = render_to_string(40, 8, |frame| {
            let area = frame.area();
            render_detail_panel(
                frame,
                area,
                " Detail ",
                (1..=20)
                    .map(|number| Line::from(format!("Item {number}")))
                    .collect(),
                &mut state,
                None,
            );
        });
        assert!(output.contains("Detail ↓"), "{output}");
        assert!(output.contains("Item 1"), "{output}");
    }

    #[test]
    fn status_and_shortcuts_render_in_reserved_rows() {
        let output = render_to_string(50, 3, |frame| {
            let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(2)])
                .split(frame.area());
            render_status_line(frame, rows[0], &Some(("Failed".into(), true)));
            render_shortcuts(
                frame,
                rows[1],
                &[Shortcut::new("Enter", "Open"), Shortcut::new("Esc", "Back")],
            );
        });
        assert!(output.contains("Failed"), "{output}");
        assert!(output.contains("[Enter] Open"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }

    #[test]
    fn fit_rect_clamps_every_edge_to_parent() {
        let parent = Rect::new(4, 3, 10, 5);

        assert_eq!(
            fit_rect(parent, Rect::new(8, 6, 30, 20)),
            Rect::new(8, 6, 6, 2)
        );
    }

    #[test]
    fn shortcut_height_reserves_all_wrapped_rows() {
        let shortcuts = vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("Enter", "Details"),
            Shortcut::new("/", "Filter"),
            Shortcut::new("e", "Edit"),
            Shortcut::new("d", "Delete"),
            Shortcut::new("u", "Undo"),
            Shortcut::new("Esc", "Back"),
        ];

        assert_eq!(shortcut_height(&shortcuts, 80), 2);
    }
}
