use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::db::Database;
use crate::i18n::tr;
use crate::model::{Goal, Milestone};
use super::{centered_area, highlight_row, render_help_overlay, render_status_line, Action, Screen, StatusMessage, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

fn goal_gauge_ratio(goal: &Goal) -> f64 {
    if goal.milestones.is_empty() {
        return if goal.completed { 1.0 } else { 0.0 };
    }
    let done = goal.milestones.iter().filter(|m| m.completed).count();
    done as f64 / goal.milestones.len() as f64
}

fn goal_gauge_label(goal: &Goal) -> String {
    let done = goal.milestones.iter().filter(|m| m.completed).count();
    format!("{}/{}", done, goal.milestones.len())
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
    mode: Mode,
    input: String,
    cursor: usize,
    status_msg: StatusMessage,
    show_help: bool,
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
            mode: Mode::Browse,
            input: String::new(),
            cursor: 0,
            status_msg: None,
            show_help: false,
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

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // [0] title + header
                Constraint::Min(1),    // [1] goals list
                Constraint::Length(1), // [2] status line
                Constraint::Length(1), // [3] footer
                Constraint::Min(0),   // [4] spacer
            ])
            .split(area);

        // ── Title ──
        let title = Line::from(Span::styled(
            tr("goals-title"),
            Style::default().fg(Color::White).bold(),
        ));
        frame.render_widget(Paragraph::new(vec![title, Line::default()]), chunks[0]);

        // ── Goals list ──
        let list_area = chunks[1];

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
                lines.push(render_gauge_line(goal, GREEN));
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
                lines.push(render_gauge_line(goal, Color::DarkGray));
            } else if is_selected && self.mode == Mode::ConfirmDeleteGoal {
                // Show goal normally first
                lines.extend(render_goal_lines(goal, true));
                // Confirmation line
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
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

        // ── Status line ──
        render_status_line(frame, chunks[2], &self.status_msg);

        // ── Footer ──
        let footer_spans = match self.mode {
            Mode::Browse => {
                let mut spans = vec![
                    Span::styled(" [a]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-add-goal")), Style::default().fg(Color::Gray)),
                    Span::styled("[e]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                    Span::styled("[Space]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-toggle")), Style::default().fg(Color::Gray)),
                    Span::styled("[d]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                ];
                if !self.goals.is_empty() {
                    spans.push(Span::styled("[Enter]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(" Open  ", Style::default().fg(Color::Gray)));
                }
                spans.push(Span::styled("[?]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)));
                spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::Gray)));
                spans
            }
            Mode::Modal => {
                // Footer is rendered inside the modal; main footer is empty
                vec![]
            }
            _ => {
                vec![
                    Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                ]
            }
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[3]);

        // ── Modal overlay ──
        if self.mode == Mode::Modal {
            self.render_modal(frame);
        }

        // ── Help overlay ──
        if self.show_help {
            let bindings = if self.mode == Mode::Modal {
                vec![
                    ("j/k", "Navigate"),
                    ("a", "Add milestone"),
                    ("e", "Edit"),
                    ("Space", "Toggle complete"),
                    ("D", "Edit date"),
                    ("d", "Delete"),
                    ("u", "Undo"),
                    ("?", "Help"),
                    ("Esc", "Close"),
                ]
            } else {
                vec![
                    ("j/k", "Navigate"),
                    ("a", "Add goal"),
                    ("e", "Edit"),
                    ("Space", "Toggle complete"),
                    ("D", "Edit date"),
                    ("d", "Delete"),
                    ("u", "Undo"),
                    ("Enter", "Open milestones"),
                    ("?", "Help"),
                    ("Esc", "Back"),
                ]
            };
            render_help_overlay(frame, area, &bindings);
        }
    }

    fn render_modal(&self, frame: &mut Frame) {
        let area = frame.area();
        let goal = match self.goals.get(self.modal_goal_idx) {
            Some(g) => g,
            None => return,
        };

        let modal_width = (CONTENT_WIDTH - 10).min(area.width.saturating_sub(4));
        let milestone_count = goal.milestones.len();
        // Header (gauge): 1 line, separator: 1 line, milestones or empty msg: max lines, footer: 1 line
        let content_lines = if milestone_count == 0 { 1 } else { milestone_count };
        // Extra lines for input/confirm in add/edit/delete modes
        let extra = match self.modal_mode {
            ModalMode::AddMilestone | ModalMode::EditMilestoneDate | ModalMode::ConfirmDeleteMilestone => 1,
            _ => 0,
        };
        let inner_height = 1 + 1 + content_lines + extra + 1 + 1; // gauge + sep + milestones + extra + status + footer
        let modal_height = (inner_height as u16 + 2) // +2 for border
            .min(area.height * 7 / 10)
            .max(8);
        let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_rect = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        frame.render_widget(Clear, modal_rect);

        let title = format!(" {} ", goal.title);
        let block = Block::default()
            .title(Span::styled(title, Style::default().fg(Color::White).bold()))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(modal_rect);
        frame.render_widget(block, modal_rect);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // [0] gauge
                Constraint::Length(1), // [1] separator
                Constraint::Min(1),   // [2] milestone list
                Constraint::Length(1), // [3] status line
                Constraint::Length(1), // [4] footer
            ])
            .split(inner);

        // ── Gauge ──
        frame.render_widget(
            Paragraph::new(render_gauge_line(goal, GREEN)),
            inner_chunks[0],
        );

        // ── Separator ──
        let sep_width = inner_chunks[1].width as usize;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(sep_width),
                Style::default().fg(Color::DarkGray),
            ))),
            inner_chunks[1],
        );

        // ── Milestone list ──
        let list_area = inner_chunks[2];
        let mut lines: Vec<Line> = Vec::new();

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
                lines.push(Line::from(vec![
                    Span::styled(check, Style::default().fg(check_color)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
            } else if is_sel && self.modal_mode == ModalMode::EditMilestoneDate {
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(&ms.title, Style::default().fg(GREEN)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-date-prompt")), Style::default().fg(ACCENT)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
            } else if is_sel && self.modal_mode == ModalMode::ConfirmDeleteMilestone {
                lines.push(render_milestone_line(ms, true));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                ]));
            } else {
                lines.push(render_milestone_line(ms, is_sel));
            }
        }

        // Add milestone input at end of list
        if self.modal_mode == ModalMode::AddMilestone {
            lines.push(Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                Span::styled("\u{2588}", Style::default().fg(GREEN)),
                Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
            ]));
        }

        let list_height = list_area.height as usize;
        let scroll = self.modal_scroll as u16;

        frame.render_widget(
            Paragraph::new(lines.clone()).scroll((scroll, 0)),
            list_area,
        );

        // Highlight selected milestone
        if !goal.milestones.is_empty() && self.modal_mode != ModalMode::AddMilestone {
            // Each milestone is 1 line, so selected line = selected index
            let sel_line = self.modal_selected;
            let visible_row = sel_line.saturating_sub(self.modal_scroll);
            if visible_row < list_height {
                highlight_row(frame, list_area, visible_row as u16);
            }
        }

        // ── Status line ──
        render_status_line(frame, inner_chunks[3], &self.modal_status_msg);

        // ── Footer ──
        let footer_spans = match self.modal_mode {
            ModalMode::Browse => vec![
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[Space]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-toggle")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[D]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-date")), Style::default().fg(Color::Gray)),
                Span::styled("[u]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-undo")), Style::default().fg(Color::Gray)),
                Span::styled("[?]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-close")), Style::default().fg(Color::Gray)),
            ],
            _ => vec![
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ],
        };
        frame.render_widget(Paragraph::new(Line::from(footer_spans)), inner_chunks[4]);
    }

    // ── Key handling ──

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        if self.mode != Mode::Modal {
            self.status_msg = None;
        }

        if self.show_help {
            self.show_help = false;
            return Action::None;
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
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
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

        if self.show_help {
            self.show_help = false;
            return Action::None;
        }

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
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
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
    if goal.completed {
        let date_str = goal.completed_at
            .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
            .unwrap_or_default();
        let marker = if is_selected { "» " } else { "  " };
        let style = if is_selected { Style::default().fg(GREEN) } else { Style::default().fg(Color::Gray) };
        result.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled("✓ ", Style::default().fg(GREEN)),
            Span::styled(format!("{}{}", goal.title, date_str), style),
        ]));
        result.push(render_gauge_line(goal, Color::DarkGray));
    } else {
        let marker = if is_selected { "» " } else { "  " };
        let style = if is_selected {
            Style::default().fg(GREEN).bold()
        } else {
            Style::default().fg(GREEN)
        };
        result.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled(goal.title.clone(), style),
        ]));
        result.push(render_gauge_line(goal, GREEN));
    }
    result
}

fn render_milestone_line(ms: &Milestone, is_selected: bool) -> Line<'static> {
    if ms.completed {
        let date_str = ms.completed_at
            .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
            .unwrap_or_default();
        let marker = if is_selected { "» " } else { "  " };
        let style = if is_selected { Style::default().fg(GREEN) } else { Style::default().fg(Color::Gray) };
        Line::from(vec![
            Span::styled(marker, style),
            Span::styled("✓ ", Style::default().fg(GREEN)),
            Span::styled(format!("{}{}", ms.title, date_str), style),
        ])
    } else {
        let marker = if is_selected { "» " } else { "  " };
        let style = if is_selected {
            Style::default().fg(GREEN).bold()
        } else {
            Style::default().fg(GREEN)
        };
        Line::from(vec![
            Span::styled(marker, style),
            Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled(ms.title.clone(), style),
        ])
    }
}

fn render_gauge_line(goal: &Goal, fill_color: Color) -> Line<'static> {
    const BAR_WIDTH: usize = 16;
    let ratio = goal_gauge_ratio(goal);
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH - filled;
    let label = goal_gauge_label(goal);
    Line::from(vec![
        Span::raw("    "),
        Span::styled("█".repeat(filled), Style::default().fg(fill_color)),
        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("  {}", label), Style::default().fg(Color::Gray)),
    ])
}
