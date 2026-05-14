use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::db::Database;
use crate::i18n::tr;
use crate::model::{Goal, Milestone};
use super::{centered_area, highlight_row, render_gauge_line, render_status_line, Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

fn goal_gauge_ratio(goal: &Goal) -> f64 {
    if goal.milestones.is_empty() {
        return if goal.completed { 1.0 } else { 0.0 };
    }
    let done = goal.milestones.iter().filter(|m| m.completed).count();
    done as f64 / goal.milestones.len() as f64
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
                Constraint::Min(0),   // [3] spacer
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
            .border_style(Style::default().fg(BORDER_COLOR));
        let list_area = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);
        self.list_height = list_area.height;

        let mut lines: Vec<Line> = Vec::new();
        let mut sel_line_idx: Option<usize> = None;

        // AddGoal input at top
        if self.mode == Mode::AddGoal {
            lines.push(Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                Span::styled("\u{2588}", Style::default().fg(GREEN)),
                Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
            ]));
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
                lines.push(Line::from(vec![
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
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
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-date-prompt")), Style::default().fg(ACCENT)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
                // Gauge
                lines.push(goal_gauge(goal));
                lines.push(Line::default());
            } else if is_selected && self.mode == Mode::ConfirmDeleteGoal {
                // Show goal normally first
                lines.extend(render_goal_lines(goal, true));
                // Confirmation line
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                ]));
            } else {
                lines.extend(render_goal_lines(goal, is_selected));
            }
        }

        // Compute selected line visual position for highlighting
        let sel_visual = sel_line_idx.map(|idx| {
            let w = list_area.width;
            let mut visual_row = 0u16;
            for (i, line) in lines.iter().enumerate() {
                if i == idx {
                    break;
                }
                let lw = line.width() as u16;
                visual_row += if w > 0 && lw > 0 { lw.div_ceil(w) } else { 1 };
            }
            // Highlight both title and gauge lines (2 lines for the selected goal)
            let sel_rows = if idx + 1 < lines.len() { 2u16 } else { 1u16 };
            (visual_row, sel_rows)
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
            let goal_idx = if self.mode == Mode::Modal { self.modal_goal_idx } else { self.selected };
            if let Some(goal) = self.goals.get(goal_idx) {
                let show_popup = self.mode == Mode::Modal || !goal.milestones.is_empty();
                if show_popup {
                    if let Some((visual_row, sel_rows)) = sel_visual {
                        let scroll_offset = self.scroll as u16;
                        let row_in_view = visual_row + sel_rows - scroll_offset;

                        let popup_lines = if self.mode == Mode::Modal {
                            self.build_modal_lines(goal)
                        } else {
                            let mut lines_out: Vec<Line> = Vec::new();
                            lines_out.push(goal_gauge(goal));
                            for ms in &goal.milestones {
                                lines_out.push(render_milestone_line(ms, false));
                            }
                            lines_out
                        };

                        let popup_h = (popup_lines.len() as u16 + 2).min(list_area.height * 2 / 3).max(4);
                        let popup_w = list_area.width.saturating_sub(8).max(30);
                        let popup_x = list_area.x + 4;

                        let below_y = list_area.y + row_in_view + 1;
                        let popup_y = if below_y + popup_h > list_area.y + list_area.height {
                            list_area.y + row_in_view.saturating_sub(popup_h + sel_rows - 1)
                        } else {
                            below_y
                        };
                        let popup_rect = Rect { x: popup_x, y: popup_y, width: popup_w, height: popup_h };

                        let title = format!(" {} ", goal.title);
                        let block = Block::default()
                            .title(Span::styled(title, Style::default().fg(Color::White).bold()))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(BORDER_COLOR));

                        let inner = block.inner(popup_rect);
                        frame.render_widget(Clear, popup_rect);
                        frame.render_widget(block, popup_rect);

                        let modal_scroll = if self.mode == Mode::Modal { self.modal_scroll as u16 } else { 0 };
                        frame.render_widget(
                            Paragraph::new(popup_lines.clone()).scroll((modal_scroll, 0)).wrap(Wrap { trim: false }),
                            inner,
                        );

                        // Highlight selected milestone in modal mode
                        if self.mode == Mode::Modal && !goal.milestones.is_empty() && self.modal_mode != ModalMode::AddMilestone {
                            let sel_line = (self.modal_selected + 1) as u16; // +1 for gauge line
                            if sel_line >= modal_scroll && sel_line < modal_scroll + inner.height {
                                highlight_row(frame, inner, sel_line - modal_scroll);
                            }
                        }

                        // Render modal status inside popup if in modal mode
                        if self.mode == Mode::Modal {
                            if let Some((msg, is_error)) = &self.modal_status_msg {
                                let color = if *is_error { Color::Red } else { Color::Green };
                                let status_y = popup_rect.y + popup_rect.height;
                                if status_y < list_area.y + list_area.height {
                                    let status_rect = Rect { x: popup_x, y: status_y, width: popup_w, height: 1 };
                                    frame.render_widget(Paragraph::new(Line::from(Span::styled(msg.as_str(), Style::default().fg(color)))), status_rect);
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
                    Span::styled(format!(" {}  ", tr("key-add-goal")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[e]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[Space]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-toggle")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[d]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::DarkGray)),
                ];
                if !self.goals.is_empty() {
                    spans.push(Span::styled("[Enter]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(format!(" {}  ", tr("key-milestone")), Style::default().fg(Color::DarkGray)));
                }
                spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::DarkGray)));
                spans
            }
            Mode::Modal => {
                let modal_footer: Vec<Span> = match self.modal_mode {
                    ModalMode::Browse => vec![
                        Span::styled(" [a]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[e]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[Space]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-toggle")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[d]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[D]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-date")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[Esc]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}", tr("key-close")), Style::default().fg(Color::DarkGray)),
                    ],
                    _ => vec![
                        Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::DarkGray)),
                        Span::styled("[Esc]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                    ],
                };
                modal_footer
            }
            _ => {
                vec![
                    Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                ]
            }
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[2]);

    }

    fn build_modal_lines(&self, goal: &Goal) -> Vec<Line<'static>> {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(goal_gauge(goal));

        if goal.milestones.is_empty() && self.modal_mode == ModalMode::Browse {
            lines.push(Line::from(Span::styled(
                tr("goals-no-milestones"),
                Style::default().fg(Color::DarkGray),
            )));
        }

        for (i, ms) in goal.milestones.iter().enumerate() {
            let is_sel = i == self.modal_selected;

            if is_sel && self.modal_mode == ModalMode::EditMilestone {
                let check = if ms.completed { "✓ " } else { "⏳ " };
                let check_color = if ms.completed { GREEN } else { Color::Yellow };
                let before = self.input[..self.cursor].to_string();
                let after = self.input[self.cursor..].to_string();
                lines.push(Line::from(vec![
                    Span::styled(check.to_string(), Style::default().fg(check_color)),
                    Span::styled(before, Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(after, Style::default().fg(GREEN)),
                ]));
            } else if is_sel && self.modal_mode == ModalMode::EditMilestoneDate {
                lines.push(Line::from(vec![
                    Span::styled("✓ ".to_string(), Style::default().fg(GREEN)),
                    Span::styled(ms.title.clone(), Style::default().fg(GREEN)),
                ]));
                let before = self.input[..self.cursor].to_string();
                let after = self.input[self.cursor..].to_string();
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-date-prompt")), Style::default().fg(ACCENT)),
                    Span::styled(before, Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(after, Style::default().fg(GREEN)),
                ]));
            } else if is_sel && self.modal_mode == ModalMode::ConfirmDeleteMilestone {
                lines.push(render_milestone_line(ms, true));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                ]));
            } else {
                lines.push(render_milestone_line(ms, is_sel));
            }
        }

        if self.modal_mode == ModalMode::AddMilestone {
            let before = self.input[..self.cursor].to_string();
            let after = self.input[self.cursor..].to_string();
            lines.push(Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(before, Style::default().fg(GREEN)),
                Span::styled("\u{2588}", Style::default().fg(GREEN)),
                Span::styled(after, Style::default().fg(GREEN)),
            ]));
        }

        lines
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
                    if let Err(e) = db.toggle_goal(goal.id) {
                        self.status_msg = Some((format!("Error: {}", e), true));
                    }
                    let _ = self.reload_goals(db);
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
                        GoalUndoData::Milestone { goal_id, milestone } => db.restore_milestone(*goal_id, milestone).map(|_| ()),
                    };
                    match result {
                        Ok(()) => {
                            let _ = self.reload_goals(db);
                            self.status_msg = Some((tr("status-restored"), false));
                        }
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
                    if let Err(e) = db.create_goal(&title) {
                        self.status_msg = Some((format!("Error: {}", e), true));
                    }
                    let _ = self.reload_goals(db);
                }
                self.input.clear();
                self.cursor = 0;
                self.selected = 0;
                self.scroll = 0;
                self.mode = Mode::Browse;
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
                    if let Some(goal) = self.goals.get(self.selected) {
                        if let Err(e) = db.update_goal(goal.id, &title) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::Browse;
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
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.input, "%Y-%m-%d") {
                    let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                    if let Some(goal) = self.goals.get(self.selected) {
                        if let Err(e) = db.set_goal_completed_at(goal.id, &completed_at) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::Browse;
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
                            self.status_msg = Some((tr("status-deleted-undo"), false));
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Delete failed: {}", e), true));
                        }
                    }
                    let _ = self.reload_goals(db);
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
            KeyCode::Char('e') => {
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
                if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.modal_selected) {
                        if let Err(e) = db.toggle_milestone(ms.id) {
                            self.modal_status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
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
                        GoalUndoData::Milestone { goal_id, milestone } => db.restore_milestone(*goal_id, milestone).map(|_| ()),
                    };
                    match result {
                        Ok(()) => {
                            let _ = self.reload_goals(db);
                            self.modal_status_msg = Some((tr("status-restored"), false));
                        }
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
                    if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                        if let Err(e) = db.create_milestone(goal.id, &title) {
                            self.modal_status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
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
                    if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                        if let Some(ms) = goal.milestones.get(self.modal_selected) {
                            if let Err(e) = db.update_milestone(ms.id, &title) {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                            let _ = self.reload_goals(db);
                        }
                    }
                }
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
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
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.input, "%Y-%m-%d") {
                    let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                    if let Some(goal) = self.goals.get(self.modal_goal_idx) {
                        if let Some(ms) = goal.milestones.get(self.modal_selected) {
                            if let Err(e) = db.set_milestone_completed_at(ms.id, &completed_at) {
                                self.modal_status_msg = Some((format!("Error: {}", e), true));
                            }
                            let _ = self.reload_goals(db);
                        }
                    }
                }
                self.input.clear();
                self.cursor = 0;
                self.modal_mode = ModalMode::Browse;
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
                                self.modal_status_msg = Some((tr("status-deleted-undo"), false));
                            }
                            Err(e) => {
                                self.modal_status_msg = Some((format!("Delete failed: {}", e), true));
                            }
                        }
                        let _ = self.reload_goals(db);
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
        let date_str = goal.completed_at
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
        let date_str = ms.completed_at
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
    let pct = (ratio * 100.0).round() as u32;
    line.spans.push(Span::styled(format!("  {}%", pct), Style::default().fg(Color::Gray)));
    line
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(s.scroll > 0, "scroll should advance when selected goes below viewport");
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
}
