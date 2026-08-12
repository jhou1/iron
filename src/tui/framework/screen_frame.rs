use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::tui::{centered_area, framework::fit_rect};

use super::{render_shortcuts, render_status_line, shortcut_height, Shortcut, StatusMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenFrameAreas {
    pub area: Rect,
    pub body: Rect,
    pub status: Rect,
    pub shortcuts: Rect,
}

pub fn render_screen_frame(
    frame: &mut Frame,
    full: Rect,
    max_width: u16,
    status: &StatusMessage,
    shortcuts: &[Shortcut],
) -> ScreenFrameAreas {
    let full = fit_rect(frame.area(), full);
    let area = centered_area(full, max_width);
    let rows = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(shortcut_height(shortcuts, area.width)),
    ])
    .split(area);
    let result = ScreenFrameAreas {
        area,
        body: rows[0],
        status: rows[1],
        shortcuts: rows[2],
    };
    render_status_line(frame, result.status, status);
    render_shortcuts(frame, result.shortcuts, shortcuts);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};
    use crate::tui::framework::Shortcut;
    use ratatui::layout::Rect;

    #[test]
    fn screen_frame_reserves_wrapped_shortcuts_and_status() {
        let shortcuts = vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("Esc", "Back"),
        ];
        let mut areas = None;
        let output = render_to_string(20, 6, |frame| {
            let area = frame.area();
            areas = Some(render_screen_frame(
                frame,
                area,
                20,
                &Some(("Saved".into(), false)),
                &shortcuts,
            ));
        });

        assert_eq!(
            areas.unwrap(),
            ScreenFrameAreas {
                area: Rect::new(0, 0, 20, 6),
                body: Rect::new(0, 0, 20, 3),
                status: Rect::new(0, 3, 20, 1),
                shortcuts: Rect::new(0, 4, 20, 2),
            }
        );
        assert!(output.contains("Saved"), "{output}");
        assert!(output.contains("[j/k] Navigate"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }

    #[test]
    fn screen_frame_zero_target_stays_inside_frame() {
        let mut areas = None;
        render_to_buffer(2, 2, |frame| {
            areas = Some(render_screen_frame(
                frame,
                Rect::new(1, 1, 0, 0),
                107,
                &None,
                &[],
            ));
        });
        assert_eq!(areas.unwrap().area, Rect::new(1, 1, 0, 0));
    }
}
