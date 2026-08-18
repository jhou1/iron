use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding},
    Frame,
};

use crate::tui::{framework::fit_rect, BORDER_COLOR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct StaticPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

#[allow(dead_code)]
fn static_panel_geometry(frame_area: Rect, requested_area: Rect) -> StaticPanelAreas {
    let area = fit_rect(frame_area, requested_area);
    let geometry = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1));
    let inner = fit_rect(area, geometry.inner(area));
    StaticPanelAreas { area, inner }
}

#[allow(dead_code)]
fn static_panel_block(title: String) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
        ]))
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .border_style(Style::default().fg(BORDER_COLOR))
}

#[allow(dead_code)]
pub fn render_static_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> StaticPanelAreas {
    let areas = static_panel_geometry(frame.area(), area);
    frame.render_widget(static_panel_block(title.into()), areas.area);
    render_content(frame, areas.inner);
    areas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{framework::test_support::render_to_buffer, BORDER_COLOR};
    use ratatui::{
        layout::Rect,
        style::{Color, Modifier},
        widgets::{Paragraph, Wrap},
    };

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    #[test]
    fn static_panel_renders_accepted_border_title_padding_and_content() {
        let mut returned = None;
        let buffer = render_to_buffer(20, 7, |frame| {
            returned = Some(render_static_panel(
                frame,
                frame.area(),
                "Summary",
                |frame, inner| {
                    frame
                        .render_widget(Paragraph::new("Content").wrap(Wrap { trim: false }), inner);
                },
            ));
        });
        assert_eq!(
            returned.unwrap(),
            StaticPanelAreas {
                area: Rect::new(0, 0, 20, 7),
                inner: Rect::new(2, 2, 16, 3),
            }
        );
        assert_eq!(buffer[(0, 0)].symbol(), "┌");
        assert_eq!(buffer[(4, 0)].symbol(), "S");
        assert_eq!(buffer[(0, 3)].fg, BORDER_COLOR);
        assert_eq!(buffer[(4, 0)].fg, Color::White);
        assert!(buffer[(4, 0)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(2, 2)].symbol(), "C");
    }

    #[test]
    fn static_panel_passes_the_returned_safe_inner_to_content() {
        let mut closure_inner = None;
        let mut returned = None;
        render_to_buffer(8, 6, |frame| {
            returned = Some(render_static_panel(
                frame,
                Rect::new(2, 1, 5, 4),
                "Panel",
                |_, inner| {
                    closure_inner = Some(inner);
                },
            ));
        });
        assert_eq!(closure_inner, returned.map(|areas| areas.inner));
    }

    #[test]
    fn static_panel_clips_zero_tiny_edge_and_oversized_rectangles() {
        let frame_area = Rect::new(0, 0, 6, 6);
        for requested in [
            Rect::new(0, 0, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(5, 5, 4, 4),
            Rect::new(u16::MAX, u16::MAX, 0, 0),
        ] {
            let mut returned = None;
            render_to_buffer(6, 6, |frame| {
                returned = Some(render_static_panel(frame, requested, "Panel", |_, _| {}));
            });
            let areas = returned.unwrap();
            assert!(
                rect_is_within(frame_area, areas.area),
                "{requested:?} -> {areas:?}"
            );
            assert!(
                rect_is_within(areas.area, areas.inner),
                "{requested:?} -> {areas:?}"
            );
        }
    }
}
