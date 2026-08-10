use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use super::{
    centered_area, highlight_row, render_gauge_line, render_status_line, visible_input_spans,
    Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::tr;
use crate::model::{Goal, Milestone};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisualSpan {
    start: usize,
    height: usize,
}

fn visual_spans(lines: &[Line<'_>], width: u16) -> Vec<VisualSpan> {
    let width = usize::from(width.max(1));
    let mut start = 0;
    lines
        .iter()
        .map(|line| {
            let line_width = line.width();
            let height = if line_width == 0 {
                1
            } else {
                line_width.div_ceil(width)
            };
            let span = VisualSpan { start, height };
            start += height;
            span
        })
        .collect()
}

fn keep_span_visible(scroll: usize, viewport: usize, span: VisualSpan) -> usize {
    if viewport == 0 || span.start < scroll {
        return span.start;
    }

    let span_bottom = span.start + span.height;
    if span_bottom > scroll + viewport {
        span_bottom.saturating_sub(viewport)
    } else {
        scroll
    }
}

fn owned_visible_input_spans(
    text: &str,
    cursor: usize,
    max_width: u16,
    prefix_width: u16,
    color: Color,
) -> Vec<Span<'static>> {
    visible_input_spans(text, cursor, max_width, prefix_width, color)
        .into_iter()
        .map(|span| Span::styled(span.content.into_owned(), span.style))
        .collect()
}

fn goal_gauge_ratio(goal: &Goal) -> f64 {
    if goal.milestones.is_empty() {
        return if goal.completed { 1.0 } else { 0.0 };
    }
    let done = goal.milestones.iter().filter(|m| m.completed).count();
    done as f64 / goal.milestones.len() as f64
}

fn goal_progress_text(goal: &Goal) -> String {
    let ratio = goal_gauge_ratio(goal);
    let pct = (ratio * 100.0).round() as u32;
    if goal.milestones.is_empty() {
        format!("{}%", pct)
    } else {
        let done = goal.milestones.iter().filter(|m| m.completed).count();
        format!("{}/{} · {}%", done, goal.milestones.len(), pct)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Browse,
    AddGoal,
    EditGoal,
    EditGoalDate,
    ConfirmDeleteGoal,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ModalMode {
    Browse,
    AddMilestone,
    EditMilestone,
    EditMilestoneDate,
    ConfirmDeleteMilestone,
}

enum GoalUndoData {
    Goal(Goal),
    Milestone { goal_id: i64, milestone: Milestone },
}

struct ModalContent {
    lines: Vec<Line<'static>>,
    selected_line: Option<usize>,
    selected_line_count: usize,
}

pub struct GoalsScreen {
    goals: Vec<Goal>,
    selected: usize,
    scroll: usize,
    list_height: u16,
    mode: Mode,
    input: String,
    cursor: usize,
    status_msg: StatusMessage,
    last_deleted: Option<GoalUndoData>,
    // Modal state
    modal_goal_idx: usize,
    modal_selected: usize,
    modal_scroll: usize,
    modal_mode: ModalMode,
    modal_status_msg: StatusMessage,
    modal_last_deleted: Option<GoalUndoData>,
}

impl GoalsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let goals = db.list_goals()?;
        Ok(Self {
            goals,
            selected: 0,
            scroll: 0,
            list_height: 0,
            mode: Mode::Browse,
            input: String::new(),
            cursor: 0,
            status_msg: None,
            last_deleted: None,
            modal_goal_idx: 0,
            modal_selected: 0,
            modal_scroll: 0,
            modal_mode: ModalMode::Browse,
            modal_status_msg: None,
            modal_last_deleted: None,
        })
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        self.goals = db.list_goals()?;
        Ok(())
    }

    fn adjust_scroll(&mut self) {
        let lines_per_goal: usize = 3; // title + gauge + spacer
        let sel_top = self.selected * lines_per_goal;
        let sel_bottom = sel_top + 2; // title + gauge (don't need spacer visible)
        if sel_top < self.scroll {
            self.scroll = sel_top;
        } else if sel_bottom > self.scroll + self.list_height as usize {
            self.scroll = sel_bottom.saturating_sub(self.list_height as usize);
        }
    }

    #[allow(dead_code)]
    fn find_milestone(&self, id: i64) -> Option<(i64, Milestone)> {
        for goal in &self.goals {
            for ms in &goal.milestones {
                if ms.id == id {
                    return Some((goal.id, ms.clone()));
                }
            }
        }
        None
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.cursor > 0 {
                    let prev = self.input[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    let prev = self.input[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.cursor < self.input.len() {
                    let next = self.input[self.cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.input.len());
                    self.cursor = next;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    let next = self.input[self.cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.input.len());
                    self.cursor = next;
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor = 0;
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor = self.input.len();
                true
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = self.input[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input.remove(prev);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.truncate(self.cursor);
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    // ── Rendering ──

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // [0] goals list (bordered)
                Constraint::Length(1), // [1] status line
                Constraint::Length(1), // [2] footer
                Constraint::Length(0), // [3] reserved
            ])
            .split(area);

        // ── Goals list (bordered) ──
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(tr("goals-title"), Style::default().fg(Color::White).bold()),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let list_area = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);
        self.list_height = list_area.height;

        let mut lines: Vec<Line> = Vec::new();
        let mut sel_line_idx: Option<usize> = None;

        // AddGoal input at top
        if self.mode == Mode::AddGoal {
            let prefix = "⏳ ";
            let mut input_spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
            input_spans.extend(visible_input_spans(
                &self.input,
                self.cursor,
                list_area.width,
                Line::from(prefix).width() as u16,
                GREEN,
            ));
            lines.push(Line::from(input_spans));
        }

        if self.goals.is_empty() && self.mode == Mode::Browse {
            lines.push(Line::from(Span::styled(
                tr("dashboard-press-a-goal"),
                Style::default().fg(Color::Gray),
            )));
        }

        for (idx, goal) in self.goals.iter().enumerate() {
            let is_selected = idx == self.selected;

            if is_selected {
                sel_line_idx = Some(lines.len());
            }

            if is_selected && self.mode == Mode::EditGoal {
                // Replace title line with edit input
                let prefix = "⏳ ";
                let mut input_spans =
                    vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
                input_spans.extend(visible_input_spans(
                    &self.input,
                    self.cursor,
                    list_area.width,
                    Line::from(prefix).width() as u16,
                    GREEN,
                ));
                lines.push(Line::from(input_spans));
                // Still show gauge below
                lines.push(goal_gauge(goal));
                lines.push(Line::default());
            } else if is_selected && self.mode == Mode::EditGoalDate {
                // Show title
                lines.push(Line::from(vec![
                    Span::styled("» ", Style::default().fg(GREEN)),
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(&goal.title, Style::default().fg(GREEN)),
                ]));
                // Date input
                let prefix = format!("  {} ", tr("dashboard-date-prompt"));
                let prefix_width = Line::from(prefix.as_str()).width() as u16;
                let mut input_spans = vec![Span::styled(prefix, Style::default().fg(ACCENT))];
                input_spans.extend(visible_input_spans(
                    &self.input,
                    self.cursor,
                    list_area.width,
                    prefix_width,
                    GREEN,
                ));
                lines.push(Line::from(input_spans));
                // Gauge
                lines.push(goal_gauge(goal));
                lines.push(Line::default());
            } else if is_selected && self.mode == Mode::ConfirmDeleteGoal {
                // Show goal normally first
                lines.extend(render_goal_lines(goal, true));
                // Confirmation line
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", tr("dashboard-delete-confirm")),
                        Style::default().fg(Color::Red),
                    ),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-yes")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}", tr("key-cancel")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                lines.extend(render_goal_lines(goal, is_selected));
            }
        }

        // Compute selected line visual position for highlighting
        let sel_visual = sel_line_idx.map(|idx| {
            let spans = visual_spans(&lines, list_area.width);
            let title = spans[idx];
            let gauge_height = spans.get(idx + 1).map(|span| span.height).unwrap_or(0);
            (
                title.start as u16,
                (title.height + gauge_height).max(1) as u16,
            )
        });

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll as u16, 0)),
            list_area,
        );

        if let Some((visual_row, sel_rows)) = sel_visual {
            if self.mode != Mode::AddGoal {
                let scroll = self.scroll as u16;
                for r in 0..sel_rows {
                    let abs_row = visual_row + r;
                    if abs_row >= scroll && abs_row < scroll + list_area.height {
                        highlight_row(frame, list_area, abs_row - scroll);
                    }
                }
            }
        }

        // ── Floating detail popup (read-only in Browse, interactive in Modal) ──
        if matches!(self.mode, Mode::Browse | Mode::Modal) {
            let goal_idx = if self.mode == Mode::Modal {
                self.modal_goal_idx
            } else {
                self.selected
            };
            if let Some(goal) = self.goals.get(goal_idx).cloned() {
                let show_popup = self.mode == Mode::Modal || !goal.milestones.is_empty();
                if show_popup {
                    if let Some((visual_row, sel_rows)) = sel_visual {
                        let scroll_offset = self.scroll as u16;
                        let row_in_view = visual_row + sel_rows - scroll_offset;
                        let popup_w = list_area.width.saturating_sub(8).max(30);
                        let popup_inner_width = popup_w.saturating_sub(4).max(1);

                        let content = if self.mode == Mode::Modal {
                            self.build_modal_content(&goal, popup_inner_width)
                        } else {
                            let mut lines_out: Vec<Line> = Vec::new();
                            lines_out.push(goal_gauge(&goal));
                            for ms in &goal.milestones {
                                lines_out.push(render_milestone_line(ms, false));
                            }
                            ModalContent {
                                lines: lines_out,
                                selected_line: None,
                                selected_line_count: 0,
                            }
                        };

                        let spans = visual_spans(&content.lines, popup_inner_width);
                        let popup_content_height = spans
                            .last()
                            .map(|span| span.start + span.height)
                            .unwrap_or(1) as u16;
                        let popup_h = popup_content_height
                            .saturating_add(5) // borders + padding + one action row
                            .min(list_area.height)
                            .max(5);
                        let popup_x = list_area.x + 4;

                        let below_y = list_area.y + row_in_view + 1;
                        let popup_y = if below_y + popup_h > list_area.y + list_area.height {
                            list_area.y + row_in_view.saturating_sub(popup_h + sel_rows - 1)
                        } else {
                            below_y
                        };
                        let popup_rect = Rect {
                            x: popup_x,
                            y: popup_y,
                            width: popup_w,
                            height: popup_h,
                        };

                        let viewport_height = popup_h.saturating_sub(4) as usize;
                        let max_scroll =
                            (popup_content_height as usize).saturating_sub(viewport_height);
                        let mut modal_scroll = if self.mode == Mode::Modal {
                            self.modal_scroll.min(max_scroll)
                        } else {
                            0
                        };
                        let selected_span = content.selected_line.and_then(|line_index| {
                            let first = spans.get(line_index).copied()?;
                            let last_index = line_index
                                .saturating_add(content.selected_line_count.saturating_sub(1));
                            let last = spans.get(last_index).copied().unwrap_or(first);
                            Some(VisualSpan {
                                start: first.start,
                                height: last.start + last.height - first.start,
                            })
                        });
                        if let Some(span) = selected_span {
                            let scroll_span = if content.selected_line == Some(1) {
                                VisualSpan {
                                    start: 0,
                                    height: span.start + span.height,
                                }
                            } else {
                                span
                            };
                            modal_scroll =
                                keep_span_visible(modal_scroll, viewport_height, scroll_span)
                                    .min(max_scroll);
                            self.modal_scroll = modal_scroll;
                        }

                        let above = modal_scroll > 0;
                        let below = modal_scroll < max_scroll;
                        let overflow = match (above, below) {
                            (true, true) => " ↑↓",
                            (true, false) => " ↑",
                            (false, true) => " ↓",
                            (false, false) => "",
                        };
                        let title = format!(" {}{} ", goal.title, overflow);
                        let border_color = if self.mode == Mode::Modal {
                            ACCENT
                        } else {
                            BORDER_COLOR
                        };
                        let block = Block::default()
                            .title(Span::styled(
                                title,
                                Style::default().fg(Color::White).bold(),
                            ))
                            .borders(Borders::ALL)
                            .padding(Padding::uniform(1))
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(border_color));

                        let inner = block.inner(popup_rect);
                        frame.render_widget(Clear, popup_rect);
                        frame.render_widget(block, popup_rect);

                        frame.render_widget(
                            Paragraph::new(content.lines.clone())
                                .scroll((modal_scroll as u16, 0))
                                .wrap(Wrap { trim: false }),
                            inner,
                        );

                        if self.mode == Mode::Modal {
                            if let Some(span) = selected_span {
                                for row in 0..span.height {
                                    let visual_row = span.start + row;
                                    if visual_row >= modal_scroll
                                        && visual_row < modal_scroll + viewport_height
                                    {
                                        highlight_row(
                                            frame,
                                            inner,
                                            (visual_row - modal_scroll) as u16,
                                        );
                                    }
                                }
                            }
                        }

                        // Render modal status inside popup if in modal mode
                        if self.mode == Mode::Modal {
                            if let Some((msg, is_error)) = &self.modal_status_msg {
                                let color = if *is_error { Color::Red } else { Color::Green };
                                let status_y = popup_rect.y + popup_rect.height;
                                if status_y < list_area.y + list_area.height {
                                    let status_rect = Rect {
                                        x: popup_x,
                                        y: status_y,
                                        width: popup_w,
                                        height: 1,
                                    };
                                    let status_line = Line::from(Span::styled(
                                        msg.as_str(),
                                        Style::default().fg(color),
                                    ));
                                    frame.render_widget(
                                        Paragraph::new(status_line).wrap(Wrap { trim: false }),
                                        status_rect,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Status line ──
        render_status_line(frame, chunks[1], &self.status_msg);

        // ── Footer ──
        let footer_spans = match self.mode {
            Mode::Browse => {
                let mut spans = vec![
                    Span::styled(" [a]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-add-goal")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[m]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-milestone")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[e]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-edit")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Space]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-toggle")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[d]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-delete")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ];
                if !self.goals.is_empty() {
                    spans.push(Span::styled("[Enter]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(
                        format!(" {}  ", tr("key-milestone")),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(
                    format!(" {}", tr("key-back")),
                    Style::default().fg(Color::DarkGray),
                ));
                spans
            }
            Mode::Modal => {
                let modal_footer: Vec<Span> = match self.modal_mode {
                    ModalMode::Browse => vec![
                        Span::styled(" [a]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-add")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[e]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-edit")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[Space]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-toggle")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[d]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-delete")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[D]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-date")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[Esc]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}", tr("key-close")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ],
                    _ => vec![
                        Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}  ", tr("key-confirm")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("[Esc]", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}", tr("key-cancel")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ],
                };
                modal_footer
            }
            _ => {
                vec![
                    Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-confirm")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}", tr("key-cancel")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]
            }
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer).wrap(Wrap { trim: false }), chunks[2]);
    }

    fn build_modal_content(&self, goal: &Goal, inner_width: u16) -> ModalContent {
        let mut lines: Vec<Line> = Vec::new();
        let mut selected_line = None;
        let mut selected_line_count = 1;
        lines.push(goal_gauge(goal));

        if goal.milestones.is_empty() && self.modal_mode == ModalMode::Browse {
            lines.push(Line::from(Span::styled(
                tr("goals-no-milestones"),
                Style::default().fg(Color::DarkGray),
            )));
        }

        for (i, ms) in goal.milestones.iter().enumerate() {
            let is_sel = i == self.modal_selected;
            if is_sel && self.modal_mode != ModalMode::AddMilestone {
                selected_line = Some(lines.len());
            }

            if is_sel && self.modal_mode == ModalMode::EditMilestone {
                let check = if ms.completed { "✓ " } else { "⏳ " };
                let check_color = if ms.completed { GREEN } else { Color::Yellow };
                let mut input_spans = vec![Span::styled(
                    check.to_string(),
                    Style::default().fg(check_color),
                )];
                input_spans.extend(owned_visible_input_spans(
                    &self.input,
                    self.cursor,
                    inner_width,
                    Line::from(check).width() as u16,
                    GREEN,
                ));
                lines.push(Line::from(input_spans));
            } else if is_sel && self.modal_mode == ModalMode::EditMilestoneDate {
                selected_line_count = 2;
                lines.push(Line::from(vec![
                    Span::styled("✓ ".to_string(), Style::default().fg(GREEN)),
                    Span::styled(ms.title.clone(), Style::default().fg(GREEN)),
                ]));
                let prefix = format!("  {} ", tr("dashboard-date-prompt"));
                let prefix_width = Line::from(prefix.as_str()).width() as u16;
                let mut input_spans = vec![Span::styled(prefix, Style::default().fg(ACCENT))];
                input_spans.extend(owned_visible_input_spans(
                    &self.input,
                    self.cursor,
                    inner_width,
                    prefix_width,
                    GREEN,
                ));
                lines.push(Line::from(input_spans));
            } else if is_sel && self.modal_mode == ModalMode::ConfirmDeleteMilestone {
                selected_line_count = 2;
                lines.push(render_milestone_line(ms, true));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", tr("dashboard-delete-confirm")),
                        Style::default().fg(Color::Red),
                    ),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-yes")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}", tr("key-cancel")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                lines.push(render_milestone_line(ms, is_sel));
            }
        }

        if self.modal_mode == ModalMode::AddMilestone {
            selected_line = Some(lines.len());
            let prefix = "⏳ ";
            let mut input_spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
            input_spans.extend(owned_visible_input_spans(
                &self.input,
                self.cursor,
                inner_width,
                Line::from(prefix).width() as u16,
                GREEN,
            ));
            lines.push(Line::from(input_spans));
        }

        ModalContent {
            lines,
            selected_line,
            selected_line_count,
        }
    }

    // ── Key handling ──

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        if self.mode != Mode::Modal {
            self.status_msg = None;
        }

        match self.mode {
            Mode::Modal => self.handle_modal(key, db),
            Mode::Browse => self.handle_browse(key, db),
            Mode::AddGoal => self.handle_add_goal(key, db),
            Mode::EditGoal => self.handle_edit_goal(key, db),
            Mode::EditGoalDate => self.handle_edit_goal_date(key, db),
            Mode::ConfirmDeleteGoal => self.handle_confirm_delete_goal(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        let goal_count = self.goals.len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if goal_count > 0 && self.selected < goal_count - 1 {
                    self.selected += 1;
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Enter => {
                if !self.goals.is_empty() {
                    self.mode = Mode::Modal;
                    self.modal_goal_idx = self.selected;
                    self.modal_selected = 0;
                    self.modal_scroll = 0;
                    self.modal_mode = ModalMode::Browse;
                    self.modal_status_msg = None;
                    self.modal_last_deleted = None;
                }
                Action::None
            }
            KeyCode::Char('m') => {
                if !self.goals.is_empty() {
                    self.mode = Mode::Modal;
                    self.modal_goal_idx = self.selected;
                    self.modal_selected = 0;
                    self.modal_scroll = 0;
                    self.modal_mode = ModalMode::AddMilestone;
                    self.modal_status_msg = None;
                    self.modal_last_deleted = None;
                    self.input.clear();
                    self.cursor = 0;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::AddGoal;
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.selected) {
                    self.input = goal.title.clone();
                    self.cursor = self.input.len();
                    self.mode = Mode::EditGoal;
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(goal) = self.goals.get(self.selected) {
                    if goal.milestones.is_empty() {
                        if let Err(e) = db.toggle_goal(goal.id) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        } else if let Err(e) = self.reload_goals(db) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    } else {
                        self.status_msg = Some((tr("goals-progress-from-milestones"), false));
                    }
                }
                Action::None
            }
            KeyCode::Char('D') => {
                if let Some(goal) = self.goals.get(self.selected) {
                    if goal.completed {
                        self.input.clear();
                        self.cursor = 0;
                        self.mode = Mode::EditGoalDate;
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.goals.is_empty() {
                    self.mode = Mode::ConfirmDeleteGoal;
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(undo_data) = self.last_deleted.take() {
                    let result = match &undo_data {
                        GoalUndoData::Goal(goal) => db.restore_goal(goal).map(|_| ()),
                        GoalUndoData::Milestone { goal_id, milestone } => {
                            db.restore_milestone(*goal_id, milestone).map(|_| ())
                        }
                    };
                    match result {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.status_msg = Some((tr("status-restored"), false));
                            }
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.last_deleted = Some(undo_data);
                            self.status_msg = Some((format!("Restore failed: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            _ => Action::None,
        }
    }

    fn handle_add_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    match db.create_goal(&title) {
                        Ok(_) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.input.clear();
                                self.cursor = 0;
                                self.selected = 0;
                                self.scroll = 0;
                                self.mode = Mode::Browse;
                            }
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.goals.get(self.selected).map(|goal| goal.id) {
                        match db.update_goal(goal_id, &title) {
                            Ok(()) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.input.clear();
                                    self.cursor = 0;
                                    self.mode = Mode::Browse;
                                }
                                Err(e) => {
                                    self.status_msg = Some((format!("Error: {}", e), true));
                                }
                            },
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date = match chrono::NaiveDate::parse_from_str(&self.input, "%Y-%m-%d") {
                    Ok(date) => date,
                    Err(_) => {
                        self.status_msg = Some((tr("goals-invalid-date"), true));
                        return Action::None;
                    }
                };
                let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                if let Some(goal_id) = self.goals.get(self.selected).map(|goal| goal.id) {
                    match db.set_goal_completed_at(goal_id, &completed_at) {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.input.clear();
                                self.cursor = 0;
                                self.mode = Mode::Browse;
                            }
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_confirm_delete_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(goal) = self.goals.get(self.selected).cloned() {
                    match db.delete_goal(goal.id) {
                        Ok(()) => {
                            self.last_deleted = Some(GoalUndoData::Goal(goal));
                            match self.reload_goals(db) {
                                Ok(()) => {
                                    self.status_msg = Some((tr("status-deleted-undo"), false));
                                }
                                Err(e) => {
                                    self.status_msg = Some((format!("Error: {}", e), true));
                                }
                            }
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Delete failed: {}", e), true));
                        }
                    }
                    if self.selected >= self.goals.len() && !self.goals.is_empty() {
                        self.selected = self.goals.len() - 1;
                    }
                }
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.mode = Mode::Browse;
                Action::None
            }
        }
    }

    // ── Modal key handling ──

    fn handle_modal(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.modal_status_msg = None;

        match self.modal_mode {
            ModalMode::Browse => self.handle_modal_browse(key, db),
            ModalMode::AddMilestone => self.handle_modal_add_milestone(key, db),
            ModalMode::EditMilestone => self.handle_modal_edit_milestone(key, db),
            ModalMode::EditMilestoneDate => self.handle_modal_edit_milestone_date(key, db),
            ModalMode::ConfirmDeleteMilestone => self.handle_modal_confirm_delete(key, db),
        }
    }

    fn modal_milestone_count(&self) -> usize {
        self.goals
            .get(self.modal_goal_idx)
            .map(|g| g.milestones.len())
            .unwrap_or(0)
    }

    fn handle_modal_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        let ms_count = self.modal_milestone_count();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if ms_count > 0 && self.modal_selected < ms_count - 1 {
                    self.modal_selected += 1;
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.modal_selected > 0 {
                    self.modal_selected -= 1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::AddMilestone;
                Action::None
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.modal_selected) {
                        self.input = ms.title.clone();
                        self.cursor = self.input.len();
                        self.modal_mode = ModalMode::EditMilestone;
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                let milestone_id = self
                    .goals
                    .get(self.modal_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.modal_selected))
                    .map(|milestone| milestone.id);
                if let Some(milestone_id) = milestone_id {
                    match db.toggle_milestone(milestone_id) {
                        Ok(()) => {
                            if let Err(e) = self.reload_goals(db) {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                        Err(e) => {
                            self.modal_status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('D') => {
                if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.modal_selected) {
                        if ms.completed {
                            self.input.clear();
                            self.cursor = 0;
                            self.modal_mode = ModalMode::EditMilestoneDate;
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if ms_count > 0 {
                    self.modal_mode = ModalMode::ConfirmDeleteMilestone;
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(undo_data) = self.modal_last_deleted.take() {
                    let result = match &undo_data {
                        GoalUndoData::Goal(_) => Ok(()), // shouldn't happen in modal
                        GoalUndoData::Milestone { goal_id, milestone } => {
                            db.restore_milestone(*goal_id, milestone).map(|_| ())
                        }
                    };
                    match result {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.modal_status_msg = Some((tr("status-restored"), false));
                            }
                            Err(e) => {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.modal_last_deleted = Some(undo_data);
                            self.modal_status_msg = Some((format!("Restore failed: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_modal_add_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.goals.get(self.modal_goal_idx).map(|goal| goal.id) {
                        match db.create_milestone(goal_id, &title) {
                            Ok(_) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.modal_selected =
                                        self.modal_milestone_count().saturating_sub(1);
                                    self.input.clear();
                                    self.cursor = 0;
                                    self.modal_mode = ModalMode::Browse;
                                }
                                Err(e) => {
                                    self.modal_status_msg = Some((format!("Error: {}", e), true));
                                }
                            },
                            Err(e) => {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_modal_edit_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    let milestone_id = self
                        .goals
                        .get(self.modal_goal_idx)
                        .and_then(|goal| goal.milestones.get(self.modal_selected))
                        .map(|milestone| milestone.id);
                    if let Some(milestone_id) = milestone_id {
                        match db.update_milestone(milestone_id, &title) {
                            Ok(()) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.input.clear();
                                    self.cursor = 0;
                                    self.modal_mode = ModalMode::Browse;
                                }
                                Err(e) => {
                                    self.modal_status_msg = Some((format!("Error: {}", e), true));
                                }
                            },
                            Err(e) => {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_modal_edit_milestone_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date = match chrono::NaiveDate::parse_from_str(&self.input, "%Y-%m-%d") {
                    Ok(date) => date,
                    Err(_) => {
                        self.modal_status_msg = Some((tr("goals-invalid-date"), true));
                        return Action::None;
                    }
                };
                let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                let milestone_id = self
                    .goals
                    .get(self.modal_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.modal_selected))
                    .map(|milestone| milestone.id);
                if let Some(milestone_id) = milestone_id {
                    match db.set_milestone_completed_at(milestone_id, &completed_at) {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.input.clear();
                                self.cursor = 0;
                                self.modal_mode = ModalMode::Browse;
                            }
                            Err(e) => {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.modal_status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_modal_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.modal_selected).cloned() {
                        let goal_id = goal.id;
                        match db.delete_milestone(ms.id) {
                            Ok(()) => {
                                self.modal_last_deleted = Some(GoalUndoData::Milestone {
                                    goal_id,
                                    milestone: ms,
                                });
                                match self.reload_goals(db) {
                                    Ok(()) => {
                                        self.modal_status_msg =
                                            Some((tr("status-deleted-undo"), false));
                                    }
                                    Err(e) => {
                                        self.modal_status_msg =
                                            Some((format!("Error: {}", e), true));
                                    }
                                }
                            }
                            Err(e) => {
                                self.modal_status_msg =
                                    Some((format!("Delete failed: {}", e), true));
                            }
                        }
                        // Adjust selected index after deletion
                        let new_count = self.modal_milestone_count();
                        if new_count == 0 {
                            self.modal_selected = 0;
                        } else if self.modal_selected >= new_count {
                            self.modal_selected = new_count - 1;
                        }
                    }
                }
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
        }
    }
}

fn render_goal_lines(goal: &Goal, is_selected: bool) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let marker = if is_selected { "» " } else { "  " };
    let style = if is_selected {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    if goal.completed {
        let date_str = goal
            .completed_at
            .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
            .unwrap_or_default();
        result.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled("✓ ", Style::default().fg(GREEN)),
            Span::styled(format!("{}{}", goal.title, date_str), style),
        ]));
    } else {
        result.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled(goal.title.clone(), style),
        ]));
    }
    result.push(goal_gauge(goal));
    result.push(Line::default());
    result
}

fn render_milestone_line(ms: &Milestone, is_selected: bool) -> Line<'static> {
    let marker = if is_selected { "» " } else { "  " };
    let style = if is_selected {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    if ms.completed {
        let date_str = ms
            .completed_at
            .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(marker, style),
            Span::styled("✓ ", Style::default().fg(GREEN)),
            Span::styled(format!("{}{}", ms.title, date_str), style),
        ])
    } else {
        Line::from(vec![
            Span::styled(marker, style),
            Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled(ms.title.clone(), style),
        ])
    }
}

fn goal_gauge(goal: &Goal) -> Line<'static> {
    let ratio = goal_gauge_ratio(goal);
    let done = goal.milestones.iter().filter(|m| m.completed).count();
    let total = goal.milestones.len();
    let mut line = render_gauge_line(ratio, done, total, 16, 4);
    line.spans.push(Span::styled(
        format!("  {}", goal_progress_text(goal)),
        Style::default().fg(Color::Gray),
    ));
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_screen(screen: &mut GoalsScreen, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.render(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut output = String::new();
        for y in 0..height {
            for x in 0..width {
                output.push_str(buffer[(x, y)].symbol());
            }
            output.push('\n');
        }
        output
    }

    fn make_goal_with_milestones(states: &[bool]) -> Goal {
        Goal {
            id: 1,
            title: "Goal".to_string(),
            completed: false,
            position: 0,
            created_at: chrono::NaiveDateTime::default(),
            completed_at: None,
            milestones: states
                .iter()
                .enumerate()
                .map(|(index, completed)| Milestone {
                    id: index as i64 + 1,
                    goal_id: 1,
                    title: format!("Milestone {}", index + 1),
                    completed: *completed,
                    position: index as i32 + 1,
                    created_at: chrono::NaiveDateTime::default(),
                    completed_at: completed.then(chrono::NaiveDateTime::default),
                })
                .collect(),
        }
    }

    fn make_screen(goal_count: usize, list_height: u16) -> GoalsScreen {
        let goals: Vec<Goal> = (0..goal_count)
            .map(|i| Goal {
                id: i as i64 + 1,
                title: format!("Goal {}", i + 1),
                completed: false,
                position: i as i32,
                created_at: chrono::NaiveDateTime::default(),
                completed_at: None,
                milestones: vec![],
            })
            .collect();
        GoalsScreen {
            goals,
            selected: 0,
            scroll: 0,
            list_height,
            mode: Mode::Browse,
            input: String::new(),
            cursor: 0,
            status_msg: None,
            last_deleted: None,
            modal_goal_idx: 0,
            modal_selected: 0,
            modal_scroll: 0,
            modal_mode: ModalMode::Browse,
            modal_status_msg: None,
            modal_last_deleted: None,
        }
    }

    #[test]
    fn scroll_follows_selection_down() {
        // 10 goals, each 3 lines; viewport fits 9 lines (3 goals)
        let mut s = make_screen(10, 9);
        // Move down to goal 3 (index 2) — still visible, no scroll
        s.selected = 2;
        s.adjust_scroll();
        assert_eq!(s.scroll, 0);

        // Move down to goal 4 (index 3) — bottom at line 11, viewport 9
        s.selected = 3;
        s.adjust_scroll();
        assert!(
            s.scroll > 0,
            "scroll should advance when selected goes below viewport"
        );
        // sel_bottom = 3*3 + 2 = 11, scroll = 11 - 9 = 2
        assert_eq!(s.scroll, 2);
    }

    #[test]
    fn scroll_follows_selection_up() {
        let mut s = make_screen(10, 9);
        s.selected = 5;
        s.scroll = 12; // scrolled past goal 5
        s.adjust_scroll();
        // sel_top = 5*3 = 15, which is >= scroll=12, sel_bottom = 17 > 12+9=21? no. 17 < 21.
        // Actually sel_top=15 >= scroll=12 and sel_bottom=17 <= 12+9=21, so no change needed.
        // Let me pick a case where scroll is too far:
        s.selected = 1;
        s.scroll = 10;
        s.adjust_scroll();
        // sel_top = 3, scroll should snap to 3
        assert_eq!(s.scroll, 3);
    }

    #[test]
    fn scroll_stays_at_zero_when_all_fit() {
        // 3 goals, 9-line viewport — all fit
        let mut s = make_screen(3, 9);
        s.selected = 2;
        s.adjust_scroll();
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn scroll_last_goal_visible() {
        let mut s = make_screen(10, 9);
        s.selected = 9;
        s.adjust_scroll();
        // sel_bottom = 9*3 + 2 = 29, scroll = 29 - 9 = 20
        assert_eq!(s.scroll, 20);
    }

    #[test]
    fn progress_text_shows_completed_total_and_percentage() {
        let goal = make_goal_with_milestones(&[true, true, false, false]);
        assert_eq!(goal_progress_text(&goal), "2/4 · 50%");
    }

    #[test]
    fn keep_span_visible_scrolls_down_and_back_up() {
        let selected = VisualSpan {
            start: 5,
            height: 2,
        };
        assert_eq!(keep_span_visible(0, 4, selected), 3);
        let first = VisualSpan {
            start: 0,
            height: 1,
        };
        assert_eq!(keep_span_visible(3, 4, first), 0);
    }

    #[test]
    fn visual_spans_count_wrapped_rows() {
        let lines = vec![Line::from("123456789"), Line::from("x")];
        assert_eq!(
            visual_spans(&lines, 4),
            vec![
                VisualSpan {
                    start: 0,
                    height: 3,
                },
                VisualSpan {
                    start: 3,
                    height: 1,
                },
            ]
        );
    }

    #[test]
    fn preview_shows_four_milestones_at_minimum_terminal_size() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        for title in ["01", "02", "03", "04"] {
            db.create_milestone(goal_id, title).unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        let output = render_screen(&mut screen, 80, 24);

        for title in ["01", "02", "03", "04"] {
            assert!(
                output.contains(title),
                "missing milestone {title}\n{output}"
            );
        }
    }

    #[test]
    fn focused_panel_keeps_selected_milestone_visible() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        for number in 1..=20 {
            db.create_milestone(goal_id, &format!("Milestone {number:02}"))
                .unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), &db);
            render_screen(&mut screen, 80, 24);
        }

        let output = render_screen(&mut screen, 80, 24);
        assert!(screen.modal_scroll > 0);
        assert!(output.contains("Milestone 20"), "{output}");

        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), &db);
            render_screen(&mut screen, 80, 24);
        }

        assert_eq!(screen.modal_scroll, 0);
    }

    #[test]
    fn m_opens_selected_goal_directly_in_add_milestone_mode() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();

        screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);

        assert_eq!(screen.mode, Mode::Modal);
        assert_eq!(screen.modal_mode, ModalMode::AddMilestone);
    }

    #[test]
    fn adding_milestone_selects_new_row_and_keeps_panel_focused() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);
        screen.input = "New milestone".to_string();
        screen.cursor = screen.input.len();

        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        assert_eq!(screen.mode, Mode::Modal);
        assert_eq!(screen.modal_mode, ModalMode::Browse);
        assert_eq!(screen.modal_selected, 0);
        assert_eq!(screen.goals[0].milestones[0].title, "New milestone");
    }

    #[test]
    fn space_on_goal_with_milestones_explains_derived_progress() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        db.create_milestone(goal_id, "Milestone").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();

        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &db);

        assert!(!screen.goals[0].completed);
        assert!(screen.status_msg.is_some());
        assert!(!screen.status_msg.as_ref().unwrap().1);
    }

    #[test]
    fn long_milestone_input_scrolls_horizontally_inside_panel() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);
        screen.input = "X".repeat(200);
        screen.cursor = screen.input.len();

        let output = render_screen(&mut screen, 80, 24);

        assert!(output.matches('X').count() < 80, "{output}");
        assert!(
            output.contains('_'),
            "cursor should remain visible\n{output}"
        );
    }

    #[test]
    fn delete_confirmation_for_last_milestone_stays_visible() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        for number in 1..=20 {
            db.create_milestone(goal_id, &format!("Milestone {number:02}"))
                .unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);
        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), &db);
        }
        screen.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &db);

        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains(&tr("dashboard-delete-confirm")), "{output}");
    }

    #[test]
    fn invalid_milestone_date_keeps_input_open_with_error() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        let milestone_id = db.create_milestone(goal_id, "Milestone").unwrap();
        db.toggle_milestone(milestone_id).unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);
        screen.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE), &db);
        screen.input = "not-a-date".to_string();
        screen.cursor = screen.input.len();

        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        assert_eq!(screen.modal_mode, ModalMode::EditMilestoneDate);
        assert_eq!(screen.input, "not-a-date");
        assert!(screen
            .modal_status_msg
            .as_ref()
            .is_some_and(|(_, error)| *error));
    }
}
