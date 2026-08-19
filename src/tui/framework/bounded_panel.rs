use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Padding},
    Frame,
};

use super::fit_rect;

pub struct BoundedPanelSpec<'a> {
    pub title: Option<Line<'a>>,
    pub border_style: Style,
    pub padding: Padding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_bounded_panel<'a>(
    frame: &mut Frame,
    requested: Rect,
    spec: BoundedPanelSpec<'a>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> BoundedPanelAreas {
    let area = fit_rect(frame.area(), requested);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .padding(spec.padding)
        .border_style(spec.border_style);
    if let Some(title) = spec.title {
        block = block.title(title);
    }
    let inner = fit_rect(area, block.inner(area));
    frame.render_widget(block, area);
    render_content(frame, inner);
    BoundedPanelAreas { area, inner }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::render_to_buffer;
    use ratatui::{
        layout::Rect,
        prelude::Stylize,
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Padding, Paragraph, Wrap},
    };

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    #[test]
    fn bounded_panel_preserves_title_style_padding_border_and_content() {
        let mut returned = None;
        let buffer = render_to_buffer(20, 7, |frame| {
            returned = Some(render_bounded_panel(
                frame,
                frame.area(),
                BoundedPanelSpec {
                    title: Some(Line::from(Span::styled(
                        " Exact title ",
                        Style::default().fg(Color::Cyan).bold(),
                    ))),
                    border_style: Style::default().fg(Color::Yellow),
                    padding: Padding::uniform(1),
                },
                |frame, inner| {
                    frame.render_widget(Paragraph::new("Body").wrap(Wrap { trim: false }), inner)
                },
            ));
        });
        assert_eq!(
            returned.unwrap(),
            BoundedPanelAreas {
                area: Rect::new(0, 0, 20, 7),
                inner: Rect::new(2, 2, 16, 3),
            }
        );
        assert_eq!(buffer[(0, 3)].fg, Color::Yellow);
        assert_eq!(buffer[(1, 0)].symbol(), " ");
        assert_eq!(buffer[(2, 0)].symbol(), "E");
        assert_eq!(buffer[(2, 0)].fg, Color::Cyan);
        assert_eq!(buffer[(2, 2)].symbol(), "B");
    }

    #[test]
    fn bounded_panel_supports_no_title_and_horizontal_padding() {
        let mut closure_inner = None;
        let returned = render_to_buffer(12, 5, |frame| {
            let areas = render_bounded_panel(
                frame,
                frame.area(),
                BoundedPanelSpec {
                    title: None,
                    border_style: Style::default().fg(Color::DarkGray),
                    padding: Padding::horizontal(1),
                },
                |_, inner| closure_inner = Some(inner),
            );
            assert_eq!(areas.inner, Rect::new(2, 1, 8, 3));
        });
        assert_eq!(closure_inner, Some(Rect::new(2, 1, 8, 3)));
        assert_eq!(returned[(0, 0)].symbol(), "┌");
    }

    #[test]
    fn bounded_panel_clips_zero_tiny_edge_and_maximum_rectangles() {
        let frame_area = Rect::new(0, 0, 6, 6);
        for requested in [
            Rect::new(0, 0, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(5, 5, 4, 4),
            Rect::new(u16::MAX, u16::MAX, 0, 0),
        ] {
            let mut closure_inner = None;
            let mut returned = None;
            render_to_buffer(6, 6, |frame| {
                returned = Some(render_bounded_panel(
                    frame,
                    requested,
                    BoundedPanelSpec {
                        title: None,
                        border_style: Style::default(),
                        padding: Padding::uniform(1),
                    },
                    |_, inner| closure_inner = Some(inner),
                ));
            });
            let areas = returned.unwrap();
            assert_eq!(closure_inner, Some(areas.inner));
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
