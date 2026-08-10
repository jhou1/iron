use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use super::{
    centered_area,
    framework::{
        combined_span, fit_rect, render_detail_panel, visual_spans, DetailPanelState, TextInput,
        ViewportState, VisualSpan,
    },
    highlight_row, render_gauge_line, render_status_line, Action, Screen, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::tr;
use crate::model::{Goal, Milestone};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

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
    list_viewport: ViewportState,
    mode: Mode,
    input: TextInput,
    status_msg: StatusMessage,
    last_deleted: Option<GoalUndoData>,
    // Modal state
    modal_goal_idx: usize,
    modal_selected: usize,
    detail_state: DetailPanelState,
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
            list_viewport: ViewportState::new(0),
            mode: Mode::Browse,
            input: TextInput::new(),
            status_msg: None,
            last_deleted: None,
            modal_goal_idx: 0,
            modal_selected: 0,
            detail_state: DetailPanelState::new(false),
            modal_mode: ModalMode::Browse,
            modal_status_msg: None,
            modal_last_deleted: None,
        })
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        let selected_goal_id = self.goals.get(self.selected).map(|goal| goal.id);
        let modal_goal_id = (self.mode == Mode::Modal)
            .then(|| self.goals.get(self.modal_goal_idx).map(|goal| goal.id))
            .flatten();
        self.goals = db.list_goals()?;
        if let Some(goal_id) = selected_goal_id {
            if let Some(index) = self.goals.iter().position(|goal| goal.id == goal_id) {
                self.selected = index;
            }
        }
        if let Some(goal_id) = modal_goal_id {
            if let Some(index) = self.goals.iter().position(|goal| goal.id == goal_id) {
                self.modal_goal_idx = index;
            }
        }
        Ok(())
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
        self.list_viewport.set_height(list_area.height as usize);

        let mut lines: Vec<Line> = Vec::new();
        let mut sel_line_idx: Option<usize> = None;

        // AddGoal input at top
        if self.mode == Mode::AddGoal {
            let prefix = "⏳ ";
            let mut input_spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
            input_spans.extend(self.input.visible_spans(
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
                input_spans.extend(self.input.visible_spans(
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
                input_spans.extend(
                    self.input
                        .visible_spans(list_area.width, prefix_width, GREEN),
                );
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
        let spans = visual_spans(&lines, list_area.width);
        let content_height = spans
            .last()
            .map(|span| span.start.saturating_add(span.height))
            .unwrap_or(0);
        let selected_span = sel_line_idx.and_then(|idx| combined_span(&spans, idx, 2));
        if let Some(selected_span) = selected_span {
            self.list_viewport.reveal(selected_span);
        }
        self.list_viewport.clamp(content_height);
        let sel_visual = sel_line_idx.map(|idx| {
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
                .scroll((self.list_viewport.offset() as u16, 0)),
            list_area,
        );

        if let Some((visual_row, sel_rows)) = sel_visual {
            if self.mode != Mode::AddGoal {
                let scroll = self.list_viewport.offset() as u16;
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
                        let scroll_offset = self.list_viewport.offset() as u16;
                        let row_in_view = visual_row + sel_rows - scroll_offset;
                        let popup_x = list_area.x + 4.min(list_area.width);
                        let popup_w = list_area
                            .width
                            .saturating_sub(8)
                            .max(30)
                            .min(list_area.right().saturating_sub(popup_x));
                        let popup_inner_width = popup_w.saturating_sub(2).max(1);

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
                            .saturating_add(2)
                            .max(5)
                            .min(list_area.height);

                        let below_y = list_area.y + row_in_view + 1;
                        let popup_y = if below_y + popup_h > list_area.y + list_area.height {
                            list_area.y + row_in_view.saturating_sub(popup_h + sel_rows - 1)
                        } else {
                            below_y
                        };
                        let popup_rect = fit_rect(
                            list_area,
                            Rect {
                                x: popup_x,
                                y: popup_y,
                                width: popup_w,
                                height: popup_h,
                            },
                        );

                        let selected_span = content.selected_line.and_then(|line_index| {
                            combined_span(&spans, line_index, content.selected_line_count)
                        });
                        let selected_span = selected_span.map(|span| {
                            if content.selected_line == Some(1) {
                                VisualSpan {
                                    start: 0,
                                    height: span.start + span.height,
                                }
                            } else {
                                span
                            }
                        });

                        self.detail_state.focused = self.mode == Mode::Modal;
                        if !self.detail_state.focused {
                            self.detail_state.viewport.set_offset(0);
                        }
                        frame.render_widget(Clear, popup_rect);
                        render_detail_panel(
                            frame,
                            popup_rect,
                            format!(" {} ", goal.title),
                            content.lines,
                            &mut self.detail_state,
                            (self.mode == Mode::Modal)
                                .then_some(selected_span)
                                .flatten(),
                        );
                    }
                }
            }
        }

        // ── Status line ──
        let active_status = if self.mode == Mode::Modal {
            &self.modal_status_msg
        } else {
            &self.status_msg
        };
        render_status_line(frame, chunks[1], active_status);

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
                input_spans.extend(self.input.visible_spans(
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
                input_spans.extend(self.input.visible_spans(inner_width, prefix_width, GREEN));
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
            input_spans.extend(self.input.visible_spans(
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
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            KeyCode::Enter => {
                if !self.goals.is_empty() {
                    self.mode = Mode::Modal;
                    self.modal_goal_idx = self.selected;
                    self.modal_selected = 0;
                    self.detail_state.viewport.set_offset(0);
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
                    self.detail_state.viewport.set_offset(0);
                    self.modal_mode = ModalMode::AddMilestone;
                    self.modal_status_msg = None;
                    self.modal_last_deleted = None;
                    self.input.clear();
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.mode = Mode::AddGoal;
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.selected) {
                    self.input.set(goal.title.clone());
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
                let title = self.input.trimmed().to_string();
                if !title.is_empty() {
                    match db.create_goal(&title) {
                        Ok(_) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.input.clear();
                                self.selected = 0;
                                self.list_viewport.set_offset(0);
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
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trimmed().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.goals.get(self.selected).map(|goal| goal.id) {
                        match db.update_goal(goal_id, &title) {
                            Ok(()) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.input.clear();
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
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date = match chrono::NaiveDate::parse_from_str(self.input.value(), "%Y-%m-%d") {
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
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
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
                self.modal_mode = ModalMode::AddMilestone;
                Action::None
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.modal_selected) {
                        self.input.set(ms.title.clone());
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
                let title = self.input.trimmed().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.goals.get(self.modal_goal_idx).map(|goal| goal.id) {
                        match db.create_milestone(goal_id, &title) {
                            Ok(_) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.modal_selected =
                                        self.modal_milestone_count().saturating_sub(1);
                                    self.input.clear();
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
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_modal_edit_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trimmed().to_string();
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
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_modal_edit_milestone_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date = match chrono::NaiveDate::parse_from_str(self.input.value(), "%Y-%m-%d") {
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
                self.modal_mode = ModalMode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
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
    use crossterm::event::KeyModifiers;
    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::framework::ViewportState;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

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

    fn make_screen(goal_count: usize) -> GoalsScreen {
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
            list_viewport: ViewportState::new(0),
            mode: Mode::Browse,
            input: TextInput::new(),
            status_msg: None,
            last_deleted: None,
            modal_goal_idx: 0,
            modal_selected: 0,
            detail_state: DetailPanelState::new(false),
            modal_mode: ModalMode::Browse,
            modal_status_msg: None,
            modal_last_deleted: None,
        }
    }

    #[test]
    fn selected_last_goal_remains_visible() {
        let mut screen = make_screen(30);
        screen.selected = 29;

        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains("Goal 30"), "{output}");
    }

    #[test]
    fn progress_text_shows_completed_total_and_percentage() {
        let goal = make_goal_with_milestones(&[true, true, false, false]);
        assert_eq!(goal_progress_text(&goal), "2/4 · 50%");
    }

    #[test]
    fn milestone_popup_stays_inside_small_terminal() {
        let mut screen = make_screen(1);
        screen.goals[0].milestones = make_goal_with_milestones(&[false]).milestones;

        let output = render_screen(&mut screen, 24, 10);

        assert!(!output.is_empty());
    }

    #[test]
    fn viewport_reveals_span_down_and_back_up() {
        let mut viewport = ViewportState::new(4);
        let selected = VisualSpan {
            start: 5,
            height: 2,
        };
        viewport.reveal(selected);
        assert_eq!(viewport.offset(), 3);
        let first = VisualSpan {
            start: 0,
            height: 1,
        };
        viewport.reveal(first);
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn framework_counts_wrapped_visual_rows() {
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
        assert!(screen.detail_state.viewport.offset() > 0);
        assert!(output.contains("Milestone 20"), "{output}");

        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), &db);
            render_screen(&mut screen, 80, 24);
        }

        assert_eq!(screen.detail_state.viewport.offset(), 0);
    }

    #[test]
    fn goals_framework_migration_preserves_preview_and_focus_contract() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        for number in 1..=20 {
            db.create_milestone(goal_id, &format!("Milestone {number:02}"))
                .unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();

        let preview = render_screen(&mut screen, 80, 24);
        assert!(preview.contains("Goal ↓"), "{preview}");

        screen.handle_key(key(KeyCode::Enter), &db);
        for _ in 1..20 {
            screen.handle_key(key(KeyCode::Down), &db);
        }
        let focused = render_screen(&mut screen, 80, 24);
        assert!(focused.contains("Milestone 20"), "{focused}");
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
        screen.input.set("New milestone");

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
        screen.input.set("X".repeat(200));

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
        screen.input.set("not-a-date");

        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        assert_eq!(screen.modal_mode, ModalMode::EditMilestoneDate);
        assert_eq!(screen.input.value(), "not-a-date");
        assert!(screen
            .modal_status_msg
            .as_ref()
            .is_some_and(|(_, error)| *error));
    }

    #[test]
    fn modal_error_remains_visible_when_panel_fills_viewport() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        let mut milestone_ids = Vec::new();
        for number in 1..=20 {
            milestone_ids.push(
                db.create_milestone(goal_id, &format!("Milestone {number:02}"))
                    .unwrap(),
            );
        }
        db.toggle_milestone(*milestone_ids.last().unwrap()).unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);
        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), &db);
        }
        screen.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE), &db);
        screen.input.set("not-a-date");
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains(&tr("goals-invalid-date")), "{output}");
    }

    #[test]
    fn completing_goal_in_panel_preserves_goal_identity_after_reorder() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Other goal").unwrap();
        let target_id = db.create_goal("Target goal").unwrap();
        db.create_milestone(target_id, "Only milestone").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        let target_index = screen
            .goals
            .iter()
            .position(|goal| goal.id == target_id)
            .unwrap();
        screen.selected = target_index;
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &db);

        assert_eq!(screen.goals[screen.modal_goal_idx].id, target_id);
        assert!(screen.goals[screen.modal_goal_idx].completed);
        assert_eq!(screen.mode, Mode::Modal);
    }
}
