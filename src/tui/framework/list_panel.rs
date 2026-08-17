use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::tui::{framework::fit_rect, highlight_row, BORDER_COLOR};

use super::{render_scrollable_lines, visual_spans, ViewportState, VisualSpan};

pub struct ListPanelContent<'a> {
    pub header: Option<Line<'a>>,
    pub rows: Vec<Line<'a>>,
    pub empty: Option<Line<'a>>,
    pub selected: Option<usize>,
}

#[allow(dead_code)]
pub struct SpannedListPanelContent {
    pub header: Option<Line<'static>>,
    pub lines: Vec<Line<'static>>,
    pub empty: Option<Line<'static>>,
    pub selected: Option<VisualSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

fn list_panel_geometry(frame_area: Rect, requested_area: Rect) -> (Rect, Rect) {
    let area = fit_rect(frame_area, requested_area);
    let geometry = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1));
    let inner = fit_rect(area, geometry.inner(area));
    (area, inner)
}

fn list_panel_block(title: String, overflow: &str, accent: Color) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled("── ", Style::default().fg(accent)),
            Span::styled(
                format!("{title}{overflow}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ──", Style::default().fg(accent)),
        ]))
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .border_style(Style::default().fg(accent))
}

pub fn render_scrollable_list_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> ListPanelContent<'a>,
) -> ListPanelAreas {
    let (area, inner) = list_panel_geometry(frame.area(), area);
    let content = build_content(inner);
    let header_height = usize::from(content.header.is_some());
    let visible_height = (inner.height as usize).saturating_sub(header_height);

    viewport.set_height(visible_height);
    if let Some(selected) = content.selected.filter(|index| *index < content.rows.len()) {
        viewport.reveal(VisualSpan {
            start: selected,
            height: 1,
        });
    }
    viewport.clamp(content.rows.len());
    let overflow = viewport.overflow(content.rows.len()).suffix();
    let block = list_panel_block(title.into(), overflow, BORDER_COLOR);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    if let Some(header) = content.header {
        lines.push(header);
    }
    if content.rows.is_empty() {
        if let Some(empty) = content.empty {
            lines.push(empty);
        }
    } else {
        lines.extend(
            content
                .rows
                .iter()
                .skip(viewport.offset())
                .take(visible_height)
                .cloned(),
        );
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if let Some(selected) = content.selected.filter(|index| {
        *index >= viewport.offset()
            && *index < viewport.offset().saturating_add(visible_height)
            && *index < content.rows.len()
    }) {
        let row = header_height.saturating_add(selected - viewport.offset());
        highlight_row(frame, inner, row.min(u16::MAX as usize) as u16);
    }

    ListPanelAreas { area, inner }
}

#[allow(dead_code)]
pub fn render_scrollable_list_panel_spanned(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    focused: bool,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> SpannedListPanelContent,
) -> ListPanelAreas {
    let (area, inner) = list_panel_geometry(frame.area(), area);
    let content = build_content(inner);
    let header_height = u16::from(content.header.is_some());
    let header_area = fit_rect(
        inner,
        Rect::new(inner.x, inner.y, inner.width, header_height),
    );
    let body = fit_rect(
        inner,
        Rect::new(
            inner.x,
            inner.y.saturating_add(header_height),
            inner.width,
            inner.height.saturating_sub(header_height),
        ),
    );
    let spans = visual_spans(&content.lines, body.width);
    let content_height = spans
        .last()
        .map(|span| span.start.saturating_add(span.height))
        .unwrap_or(0);

    viewport.set_height(body.height as usize);
    if let Some(selected) = content.selected {
        viewport.reveal(selected);
    }
    viewport.clamp(content_height);
    let overflow = viewport.overflow(content_height).suffix();
    let accent = if focused { Color::Cyan } else { BORDER_COLOR };
    frame.render_widget(list_panel_block(title.into(), overflow, accent), area);

    if let Some(header) = content.header {
        frame.render_widget(
            Paragraph::new(header).wrap(Wrap { trim: false }),
            header_area,
        );
    }
    if content.lines.is_empty() {
        if let Some(empty) = content.empty {
            frame.render_widget(Paragraph::new(empty).wrap(Wrap { trim: false }), body);
        }
    } else {
        render_scrollable_lines(
            frame,
            body,
            content.lines,
            viewport,
            if focused { content.selected } else { None },
        );
    }

    ListPanelAreas { area, inner }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::{
        combined_span,
        test_support::{render_to_buffer, render_to_string},
        visual_spans, ViewportState,
    };
    use ratatui::{layout::Rect, style::Color, text::Line};

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    #[test]
    fn list_panel_returns_clipped_outer_and_inner_geometry() {
        let frame_area = Rect::new(0, 0, 6, 6);
        for target in [
            Rect::new(0, 0, 6, 6),
            Rect::new(5, 5, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(1, 1, 2, 3),
            Rect::new(5, 5, 4, 4),
        ] {
            let mut viewport = ViewportState::new(0);
            let mut returned = None;
            let mut closure_inner = None;
            render_to_buffer(6, 6, |frame| {
                let areas =
                    render_scrollable_list_panel(frame, target, "Items", &mut viewport, |inner| {
                        closure_inner = Some(inner);
                        ListPanelContent {
                            header: None,
                            rows: Vec::new(),
                            empty: None,
                            selected: None,
                        }
                    });
                returned = Some(areas);
            });
            let areas = returned.unwrap();
            assert_eq!(closure_inner, Some(areas.inner));
            assert!(
                rect_is_within(frame_area, areas.area),
                "{target:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.area, areas.inner),
                "{target:?} -> {areas:?}"
            );
        }
    }

    #[test]
    fn list_panel_renders_all_border_edges_with_border_style() {
        let mut viewport = ViewportState::new(0);
        let buffer = render_to_buffer(12, 6, |frame| {
            let area = frame.area();
            render_scrollable_list_panel(frame, area, "Items", &mut viewport, |_| {
                ListPanelContent {
                    header: None,
                    rows: vec![Line::from("Item")],
                    empty: None,
                    selected: None,
                }
            });
        });

        for ((x, y), symbol) in [
            ((0, 0), "┌"),
            ((11, 0), "┐"),
            ((0, 5), "└"),
            ((11, 5), "┘"),
            ((0, 3), "│"),
            ((11, 3), "│"),
            ((5, 5), "─"),
        ] {
            assert_eq!(buffer[(x, y)].symbol(), symbol, "border cell ({x}, {y})");
            assert_eq!(buffer[(x, y)].fg, BORDER_COLOR, "border style ({x}, {y})");
        }
    }

    #[test]
    fn list_panel_keeps_header_fixed_and_reveals_selected_row() {
        let mut viewport = ViewportState::new(0);
        let output = render_to_string(24, 8, |frame| {
            let area = frame.area();
            render_scrollable_list_panel(frame, area, "Items", &mut viewport, |_| {
                ListPanelContent {
                    header: Some(Line::from("  Name")),
                    rows: (1..=6)
                        .map(|number| Line::from(format!("  Item {number}")))
                        .collect(),
                    empty: Some(Line::from("No items")),
                    selected: Some(5),
                }
            });
        });

        assert_eq!(viewport.offset(), 3);
        assert!(output.contains("Items ↑"), "{output}");
        assert!(output.contains("Name"), "{output}");
        assert!(output.contains("Item 4"), "{output}");
        assert!(output.contains("Item 6"), "{output}");
        assert!(!output.contains("Item 1"), "{output}");
    }

    #[test]
    fn list_panel_highlights_only_the_visible_selected_data_row() {
        let mut viewport = ViewportState::new(0);
        let buffer = render_to_buffer(24, 8, |frame| {
            let area = frame.area();
            render_scrollable_list_panel(frame, area, "Items", &mut viewport, |_| {
                ListPanelContent {
                    header: Some(Line::from("Header")),
                    rows: (1..=6)
                        .map(|number| Line::from(format!("Item {number}")))
                        .collect(),
                    empty: None,
                    selected: Some(5),
                }
            });
        });

        assert_eq!(buffer[(2, 5)].bg, Color::Rgb(204, 163, 0));
        assert_ne!(buffer[(2, 2)].bg, Color::Rgb(204, 163, 0));
    }

    #[test]
    fn list_panel_handles_empty_and_zero_sized_targets() {
        let mut viewport = ViewportState::new(9);
        render_to_buffer(2, 2, |frame| {
            render_scrollable_list_panel(
                frame,
                Rect::new(1, 1, 0, 0),
                "Empty",
                &mut viewport,
                |_| ListPanelContent {
                    header: Some(Line::from("Header")),
                    rows: Vec::new(),
                    empty: Some(Line::from("No items")),
                    selected: None,
                },
            );
        });
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn list_panel_content_area_stays_inside_tiny_edge_targets() {
        let mut viewport = ViewportState::new(0);
        let mut zero_inner = None;
        let mut one_cell_inner = None;
        render_to_buffer(6, 6, |frame| {
            render_scrollable_list_panel(
                frame,
                Rect::new(5, 5, 0, 0),
                "Empty",
                &mut viewport,
                |inner| {
                    zero_inner = Some(inner);
                    ListPanelContent {
                        header: None,
                        rows: Vec::new(),
                        empty: None,
                        selected: None,
                    }
                },
            );
            render_scrollable_list_panel(
                frame,
                Rect::new(5, 5, 1, 1),
                "Empty",
                &mut viewport,
                |inner| {
                    one_cell_inner = Some(inner);
                    ListPanelContent {
                        header: None,
                        rows: Vec::new(),
                        empty: None,
                        selected: None,
                    }
                },
            );
        });

        assert_eq!(zero_inner, Some(Rect::new(5, 5, 0, 0)));
        assert_eq!(one_cell_inner, Some(Rect::new(6, 6, 0, 0)));
    }

    #[test]
    fn spanned_list_panel_reveals_and_highlights_the_complete_wrapped_selection() {
        let mut viewport = ViewportState::new(0);
        let buffer = render_to_buffer(24, 8, |frame| {
            render_scrollable_list_panel_spanned(
                frame,
                frame.area(),
                "Goals",
                true,
                &mut viewport,
                |inner| {
                    let lines = vec![
                        Line::from("First"),
                        Line::from("Second"),
                        Line::from("Selected goal title that wraps"),
                        Line::from("████ 50%"),
                        Line::from("Last"),
                    ];
                    let spans = visual_spans(&lines, inner.width);
                    SpannedListPanelContent {
                        header: None,
                        lines,
                        empty: None,
                        selected: combined_span(&spans, 2, 2),
                    }
                },
            );
        });

        assert_eq!(viewport.offset(), 1);
        for y in 3..=5 {
            assert_eq!(buffer[(2, y)].bg, Color::Rgb(204, 163, 0));
        }
        assert_eq!(buffer[(0, 3)].fg, Color::Cyan);
        assert!((0..8).any(|y| { (0..24).any(|x| buffer[(x, y)].bg == Color::Rgb(204, 163, 0)) }));
    }

    #[test]
    fn spanned_list_panel_reveals_selection_without_highlighting_when_unfocused() {
        let mut viewport = ViewportState::new(0);
        let buffer = render_to_buffer(24, 8, |frame| {
            render_scrollable_list_panel_spanned(
                frame,
                frame.area(),
                "Goals",
                false,
                &mut viewport,
                |inner| {
                    let lines = vec![
                        Line::from("First"),
                        Line::from("Second"),
                        Line::from("Selected goal title that wraps"),
                        Line::from("████ 50%"),
                        Line::from("Last"),
                    ];
                    let spans = visual_spans(&lines, inner.width);
                    SpannedListPanelContent {
                        header: None,
                        lines,
                        empty: None,
                        selected: combined_span(&spans, 2, 2),
                    }
                },
            );
        });

        assert_eq!(viewport.offset(), 1);
        assert_eq!(buffer[(0, 3)].fg, BORDER_COLOR);
        assert!(!(0..8).any(|y| { (0..24).any(|x| buffer[(x, y)].bg == Color::Rgb(204, 163, 0)) }));
    }

    #[test]
    fn spanned_list_panel_clips_returned_geometry_for_tiny_and_edge_targets() {
        let frame_area = Rect::new(0, 0, 6, 6);
        for target in [
            Rect::new(0, 0, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(5, 5, 4, 4),
            Rect::new(6, 6, 1, 1),
            Rect::new(1, 1, 4, 3),
        ] {
            let mut viewport = ViewportState::new(0);
            let mut returned = None;
            render_to_buffer(6, 6, |frame| {
                returned = Some(render_scrollable_list_panel_spanned(
                    frame,
                    target,
                    "Items",
                    false,
                    &mut viewport,
                    |_| SpannedListPanelContent {
                        header: None,
                        lines: Vec::new(),
                        empty: None,
                        selected: None,
                    },
                ));
            });
            let areas = returned.expect("renderer returns panel areas");
            assert!(
                rect_is_within(frame_area, areas.area),
                "{target:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.area, areas.inner),
                "{target:?} -> {areas:?}"
            );
        }
    }
}
