use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::{framework::fit_rect, highlight_row};

pub fn render_action_panel(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line<'_>>,
    selected: Option<usize>,
) {
    let area = fit_rect(frame.area(), area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    if let Some(selected) = selected.filter(|row| *row < area.height as usize) {
        highlight_row(frame, area, selected.min(u16::MAX as usize) as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};
    use ratatui::{layout::Rect, style::Color, text::Line};

    #[test]
    fn action_panel_renders_lines_and_selected_highlight() {
        let lines = vec![
            Line::from("Select type"),
            Line::from("  Weighted"),
            Line::from("  Bodyweight"),
        ];
        let buffer = render_to_buffer(24, 3, |frame| {
            let area = frame.area();
            render_action_panel(frame, area, lines.clone(), Some(2));
        });
        assert_eq!(buffer[(0, 2)].bg, Color::Rgb(204, 163, 0));
        assert_ne!(buffer[(0, 1)].bg, Color::Rgb(204, 163, 0));

        let output = render_to_string(24, 3, |frame| {
            let area = frame.area();
            render_action_panel(frame, area, lines.clone(), Some(2));
        });
        assert!(output.contains("Select type"), "{output}");
        assert!(output.contains("Bodyweight"), "{output}");
    }

    #[test]
    fn action_panel_zero_target_does_not_panic() {
        render_to_buffer(2, 2, |frame| {
            render_action_panel(
                frame,
                Rect::new(1, 1, 0, 0),
                vec![Line::from("hidden")],
                Some(0),
            );
        });
    }
}
