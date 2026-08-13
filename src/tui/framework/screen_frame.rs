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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenFrameSections {
    pub area: Rect,
    pub body: Vec<Rect>,
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
    let sections = render_screen_frame_sections(
        frame,
        full,
        max_width,
        status,
        shortcuts,
        &[Constraint::Min(0)],
        false,
    );
    ScreenFrameAreas {
        area: sections.area,
        body: sections.body[0],
        status: sections.status,
        shortcuts: sections.shortcuts,
    }
}

pub fn render_screen_frame_sections(
    frame: &mut Frame,
    full: Rect,
    max_width: u16,
    status: &StatusMessage,
    shortcuts: &[Shortcut],
    body_constraints: &[Constraint],
    trailing_flex: bool,
) -> ScreenFrameSections {
    let full = fit_rect(frame.area(), full);
    let area = centered_area(full, max_width);
    let body_len = body_constraints.len();
    let mut constraints = Vec::with_capacity(body_len + 3);
    constraints.extend(body_constraints.iter().copied());
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(shortcut_height(shortcuts, area.width)));
    if trailing_flex {
        constraints.push(Constraint::Min(0));
    }
    let rows = Layout::vertical(constraints).split(area);
    let result = ScreenFrameSections {
        area,
        body: rows[..body_len].to_vec(),
        status: rows[body_len],
        shortcuts: rows[body_len + 1],
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

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

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

    #[test]
    fn screen_frame_sections_match_accepted_browse_layout() {
        let shortcuts = vec![Shortcut::new("j/k", "Navigate")];
        let mut sections = None;
        let output = render_to_string(80, 24, |frame| {
            sections = Some(render_screen_frame_sections(
                frame,
                frame.area(),
                80,
                &Some(("Fixture status".into(), false)),
                &shortcuts,
                &[
                    Constraint::Min(4),
                    Constraint::Length(1),
                    Constraint::Length(0),
                ],
                true,
            ));
        });

        assert_eq!(
            sections.unwrap(),
            ScreenFrameSections {
                area: Rect::new(0, 0, 80, 24),
                body: vec![
                    Rect::new(0, 0, 80, 11),
                    Rect::new(0, 11, 80, 1),
                    Rect::new(0, 12, 80, 0),
                ],
                status: Rect::new(0, 12, 80, 1),
                shortcuts: Rect::new(0, 13, 80, 1),
            }
        );
        assert!(output
            .lines()
            .nth(12)
            .unwrap()
            .starts_with("Fixture status"));
        assert!(output
            .lines()
            .nth(13)
            .unwrap()
            .starts_with("[j/k] Navigate"));
    }

    #[test]
    fn screen_frame_sections_match_accepted_trends_layout() {
        let shortcuts = vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("h/l", "Window"),
            Shortcut::new("/", "Filter"),
            Shortcut::new("Esc", "Back"),
        ];
        let area = Rect::new(0, 0, 80, 24);
        let legacy = Layout::vertical([
            Constraint::Length(1),
            Constraint::Percentage(40),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(shortcut_height(&shortcuts, area.width)),
        ])
        .split(area);
        let mut sections = None;

        render_to_buffer(80, 24, |frame| {
            sections = Some(render_screen_frame_sections(
                frame,
                frame.area(),
                107,
                &Some(("Fixture status".into(), true)),
                &shortcuts,
                &[
                    Constraint::Length(1),
                    Constraint::Percentage(40),
                    Constraint::Min(5),
                ],
                false,
            ));
        });

        let sections = sections.unwrap();
        assert_eq!(sections.area, area);
        assert_eq!(sections.body.as_slice(), &legacy[..3]);
        assert_eq!(sections.status, legacy[3]);
        assert_eq!(sections.shortcuts, legacy[4]);
    }

    #[test]
    fn screen_frame_sections_match_accepted_action_layout() {
        let shortcuts = vec![Shortcut::new("Enter", "Confirm")];
        let mut sections = None;
        render_to_buffer(80, 24, |frame| {
            sections = Some(render_screen_frame_sections(
                frame,
                frame.area(),
                80,
                &None,
                &shortcuts,
                &[
                    Constraint::Min(4),
                    Constraint::Length(1),
                    Constraint::Length(3),
                ],
                true,
            ));
        });

        assert_eq!(
            sections.unwrap(),
            ScreenFrameSections {
                area: Rect::new(0, 0, 80, 24),
                body: vec![
                    Rect::new(0, 0, 80, 9),
                    Rect::new(0, 9, 80, 1),
                    Rect::new(0, 10, 80, 3),
                ],
                status: Rect::new(0, 13, 80, 1),
                shortcuts: Rect::new(0, 14, 80, 1),
            }
        );
    }

    #[test]
    fn screen_frame_sections_empty_and_tiny_targets_stay_inside_frame() {
        let body_constraints = [
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(3),
        ];
        for (width, height, target) in [
            (2, 2, Rect::new(1, 1, 0, 0)),
            (1, 1, Rect::new(0, 0, 1, 1)),
            (12, 4, Rect::new(0, 0, 12, 4)),
        ] {
            for constraints in [&[][..], &body_constraints[..]] {
                let frame_area = Rect::new(0, 0, width, height);
                let mut sections = None;
                render_to_buffer(width, height, |frame| {
                    sections = Some(render_screen_frame_sections(
                        frame,
                        target,
                        80,
                        &None,
                        &[Shortcut::new("Esc", "Back")],
                        constraints,
                        true,
                    ));
                });

                let sections = sections.unwrap();
                assert_eq!(sections.body.len(), constraints.len());
                assert!(rect_is_within(frame_area, sections.area), "{sections:?}");
                assert!(rect_is_within(frame_area, sections.status), "{sections:?}");
                assert!(
                    rect_is_within(frame_area, sections.shortcuts),
                    "{sections:?}"
                );
                for body in &sections.body {
                    assert!(rect_is_within(frame_area, *body), "{sections:?}");
                }
            }
        }
    }
}
