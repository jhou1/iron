use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::tui::{framework::fit_rect, highlight_row, BORDER_COLOR};

use super::{ViewportState, VisualSpan};

pub struct ListPanelContent<'a> {
    pub header: Option<Line<'a>>,
    pub rows: Vec<Line<'a>>,
    pub empty: Option<Line<'a>>,
    pub selected: Option<usize>,
}

pub fn render_scrollable_list_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> ListPanelContent<'a>,
) {
    let area = fit_rect(frame.area(), area);
    let geometry = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1));
    let inner = fit_rect(area, geometry.inner(area));
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
    let title = title.into();
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled(
                format!("{title}{overflow}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
        ]))
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .border_style(Style::default().fg(BORDER_COLOR));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::{
        test_support::{render_to_buffer, render_to_string},
        ViewportState,
    };
    use ratatui::{layout::Rect, style::Color, text::Line};

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
}
