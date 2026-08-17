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
    render_detail_panel_core(
        frame,
        area,
        DetailPanelContent {
            title: title.into(),
            header: Vec::new(),
            lines,
            selected,
        },
        state,
        true,
    );
}

#[allow(dead_code)]
pub fn render_detail_panel_with_header(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    header: Vec<Line<'static>>,
    lines: Vec<Line<'static>>,
    state: &mut DetailPanelState,
    selected: Option<VisualSpan>,
) {
    let highlight_selection = state.focused;
    render_detail_panel_core(
        frame,
        area,
        DetailPanelContent {
            title: title.into(),
            header,
            lines,
            selected,
        },
        state,
        highlight_selection,
    );
}

struct DetailPanelContent {
    title: String,
    header: Vec<Line<'static>>,
    lines: Vec<Line<'static>>,
    selected: Option<VisualSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DetailPanelAreas {
    area: Rect,
    inner: Rect,
    header: Rect,
    body: Rect,
}

fn detail_panel_geometry(
    frame_area: Rect,
    requested_area: Rect,
    header: &[Line<'static>],
) -> DetailPanelAreas {
    let area = fit_rect(frame_area, requested_area);
    let base_block = Block::default().borders(Borders::ALL);
    let inner = fit_rect(area, base_block.inner(area));
    let header_height = visual_spans(header, inner.width)
        .last()
        .map(|span| span.start.saturating_add(span.height))
        .unwrap_or(0)
        .min(inner.height as usize) as u16;
    let header = fit_rect(
        inner,
        Rect::new(inner.x, inner.y, inner.width, header_height),
    );
    let body = fit_rect(
        inner,
        Rect::new(
            inner.x,
            inner.y.saturating_add(header.height),
            inner.width,
            inner.height.saturating_sub(header.height),
        ),
    );

    DetailPanelAreas {
        area,
        inner,
        header,
        body,
    }
}

fn detail_panel_block(
    title: &str,
    suffix: &str,
    focused: bool,
    panel_width: u16,
) -> Block<'static> {
    let decorated_title = if suffix.is_empty() {
        format!("{} ", title.trim_end())
    } else {
        let title_capacity = (panel_width as usize)
            .saturating_sub(2)
            .saturating_sub(suffix.width())
            .saturating_sub(1);
        let visible_title = truncate_to_width(title.trim_end(), title_capacity);
        format!("{visible_title}{suffix} ")
    };
    let accent = if focused { Color::Cyan } else { BORDER_COLOR };

    Block::default()
        .borders(Borders::ALL)
        .title(decorated_title)
        .title_style(Style::default().fg(accent))
        .border_style(Style::default().fg(accent))
}

fn render_detail_panel_core(
    frame: &mut Frame,
    requested_area: Rect,
    content: DetailPanelContent,
    state: &mut DetailPanelState,
    highlight_selection: bool,
) {
    let areas = detail_panel_geometry(frame.area(), requested_area, &content.header);
    let spans = visual_spans(&content.lines, areas.body.width);
    let content_height = spans
        .last()
        .map(|span| span.start.saturating_add(span.height))
        .unwrap_or(0);

    state.viewport.set_height(areas.body.height as usize);
    if let Some(selected) = content.selected {
        state.viewport.reveal(selected);
    }
    state.viewport.clamp(content_height);

    let suffix = state.viewport.overflow(content_height).suffix();
    frame.render_widget(
        detail_panel_block(&content.title, suffix, state.focused, areas.area.width),
        areas.area,
    );
    frame.render_widget(
        Paragraph::new(content.header).wrap(Wrap { trim: false }),
        areas.header,
    );
    render_scrollable_lines(
        frame,
        areas.body,
        content.lines,
        &mut state.viewport,
        highlight_selection.then_some(content.selected).flatten(),
    );
}

pub fn render_status_line(frame: &mut Frame, area: Rect, status: &StatusMessage) {
    if let Some((message, is_error)) = status {
        let color = if *is_error { Color::Red } else { Color::Green };
        let line = Line::from(Span::styled(message.clone(), Style::default().fg(color)));
        frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);
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
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};
    use ratatui::{
        layout::{Constraint, Layout, Rect},
        style::Color,
        text::Line,
    };

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

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
    fn pinned_detail_header_stays_visible_while_body_scrolls() {
        let mut state = DetailPanelState::new(true);
        state.viewport.set_offset(12);
        let output = render_to_string(40, 8, |frame| {
            render_detail_panel_with_header(
                frame,
                frame.area(),
                " Goal ",
                vec![Line::from("████████  10/20 · 50%")],
                (1..=20)
                    .map(|number| Line::from(format!("Milestone {number:02}")))
                    .collect(),
                &mut state,
                Some(VisualSpan {
                    start: 19,
                    height: 1,
                }),
            );
        });

        assert!(output.contains("10/20 · 50%"), "{output}");
        assert!(output.contains("Milestone 20"), "{output}");
        assert!(output.contains("Goal ↑"), "{output}");
    }

    #[test]
    fn pinned_detail_header_uses_its_wrapped_height_before_scrolling_the_body() {
        let mut state = DetailPanelState::new(true);
        let output = render_to_string(12, 8, |frame| {
            render_detail_panel_with_header(
                frame,
                frame.area(),
                " Goal ",
                vec![Line::from("123456789012345678")],
                (1..=8)
                    .map(|number| Line::from(format!("Body {number}")))
                    .collect(),
                &mut state,
                Some(VisualSpan {
                    start: 7,
                    height: 1,
                }),
            );
        });

        assert!(output.contains("1234567890"), "{output}");
        assert!(output.contains("12345678"), "{output}");
        assert!(output.contains("Body 8"), "{output}");
        assert!(output.contains("Goal ↑"), "{output}");
    }

    #[test]
    fn pinned_detail_header_keeps_empty_body_out_of_title_overflow() {
        let mut state = DetailPanelState::new(false);
        state.viewport.set_offset(12);
        let output = render_to_string(24, 6, |frame| {
            render_detail_panel_with_header(
                frame,
                frame.area(),
                " Goal ",
                vec![Line::from("No milestones yet")],
                Vec::new(),
                &mut state,
                None,
            );
        });

        assert!(output.contains("No milestones yet"), "{output}");
        assert!(!output.contains("Goal ↑"), "{output}");
        assert!(!output.contains("Goal ↓"), "{output}");
        assert_eq!(state.viewport.offset(), 0);
    }

    #[test]
    fn unfocused_pinned_detail_reveals_selected_body_without_highlighting_it() {
        let mut state = DetailPanelState::new(false);
        let buffer = render_to_buffer(20, 6, |frame| {
            render_detail_panel_with_header(
                frame,
                frame.area(),
                " Goal ",
                vec![Line::from("Progress")],
                (1..=8)
                    .map(|number| Line::from(format!("Body {number:02}")))
                    .collect(),
                &mut state,
                Some(VisualSpan {
                    start: 7,
                    height: 1,
                }),
            );
        });

        assert_eq!(state.viewport.offset(), 5);
        assert_eq!(buffer[(1, 4)].symbol(), "B");
        assert_ne!(buffer[(1, 4)].bg, Color::Rgb(204, 163, 0));
    }

    #[test]
    fn pinned_detail_keeps_overflow_suffix_visible_with_a_long_title() {
        let mut state = DetailPanelState::new(false);
        let output = render_to_string(24, 8, |frame| {
            render_detail_panel_with_header(
                frame,
                frame.area(),
                "A selected goal title that is much wider than the panel",
                vec![Line::from("0/20 · 0%")],
                (1..=20)
                    .map(|number| Line::from(format!("Milestone {number:02}")))
                    .collect(),
                &mut state,
                Some(VisualSpan {
                    start: 0,
                    height: 1,
                }),
            );
        });

        assert!(output.lines().next().unwrap().contains('↓'), "{output}");
    }

    #[test]
    fn pinned_detail_geometry_clips_header_and_body_inside_the_safe_inner_rectangle() {
        let frame_area = Rect::new(0, 0, 6, 6);
        let header = vec![Line::from("Header wraps")];

        for requested_area in [
            Rect::new(0, 0, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(5, 5, 4, 4),
            Rect::new(u16::MAX, u16::MAX, 0, 0),
        ] {
            let areas = detail_panel_geometry(frame_area, requested_area, &header);
            assert!(
                rect_is_within(frame_area, areas.area),
                "{requested_area:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.area, areas.inner),
                "{requested_area:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.inner, areas.header),
                "{requested_area:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.inner, areas.body),
                "{requested_area:?} -> {areas:?}"
            );
        }
    }

    #[test]
    fn pinned_detail_renders_zero_one_and_max_edge_rectangles_without_panic() {
        let mut state = DetailPanelState::new(true);
        render_to_buffer(6, 6, |frame| {
            for area in [
                Rect::new(0, 0, 0, 0),
                Rect::new(5, 5, 1, 1),
                Rect::new(5, 5, 4, 4),
                Rect::new(u16::MAX, u16::MAX, 0, 0),
            ] {
                render_detail_panel_with_header(
                    frame,
                    area,
                    " Goal ",
                    vec![Line::from("Header")],
                    vec![Line::from("Body")],
                    &mut state,
                    Some(VisualSpan {
                        start: 0,
                        height: 1,
                    }),
                );
            }
        });
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
