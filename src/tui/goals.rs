use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};

use super::{
    framework::{
        combined_span, fit_rect, render_detail_panel_with_header, render_screen_frame,
        render_scrollable_list_panel_spanned, visual_spans, DetailPanelState, Shortcut,
        SpannedListPanelContent, TextInput, ViewportState, VisualSpan,
    },
    lifecycle::{NavigationStatus, Reenter},
    render_gauge_line, Action, ModuleId, NavigationIntent, StatusMessage, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::tr;
use crate::model::{Goal, Milestone};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const WIDE_GOALS_PANES_MIN_WIDTH: u16 = 83;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GoalsPaneAreas {
    goals: Rect,
    milestones: Rect,
}

fn goals_pane_areas(body: Rect) -> GoalsPaneAreas {
    if body.width >= WIDE_GOALS_PANES_MIN_WIDTH {
        let available = body.width.saturating_sub(1);
        let candidate = ((available as u32 * 2) / 5) as u16;
        let goals_width = candidate.max(34).min(available.saturating_sub(48));
        let milestones_width = available.saturating_sub(goals_width);
        let goals = fit_rect(body, Rect::new(body.x, body.y, goals_width, body.height));
        let milestones = fit_rect(
            body,
            Rect::new(
                body.x.saturating_add(goals_width).saturating_add(1),
                body.y,
                milestones_width,
                body.height,
            ),
        );
        GoalsPaneAreas { goals, milestones }
    } else {
        let gap = u16::from(body.height >= 2);
        let remaining = body.height.saturating_sub(gap);
        let goals_height = ((remaining as u32 * 2) / 5) as u16;
        let milestones_height = remaining.saturating_sub(goals_height);
        let goals = fit_rect(body, Rect::new(body.x, body.y, body.width, goals_height));
        let milestones = fit_rect(
            body,
            Rect::new(
                body.x,
                body.y.saturating_add(goals_height).saturating_add(gap),
                body.width,
                milestones_height,
            ),
        );
        GoalsPaneAreas { goals, milestones }
    }
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
    Milestones,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MilestoneMode {
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

struct MilestoneContent {
    header: Vec<Line<'static>>,
    lines: Vec<Line<'static>>,
    selected: Option<VisualSpan>,
}

#[derive(Clone)]
struct GoalPanelRenderState {
    goals: Vec<Goal>,
    selected: usize,
    mode: Mode,
    goal_input: TextInput,
}

impl GoalPanelRenderState {
    fn capture(screen: &GoalsScreen) -> Self {
        Self {
            goals: screen.goals.clone(),
            selected: screen.selected,
            mode: screen.mode,
            goal_input: screen.goal_input.clone(),
        }
    }

    fn build(self, inner_width: u16) -> SpannedListPanelContent {
        let mut lines = Vec::new();
        let mut selected_line = None;
        let mut selected_line_count = 0;

        if self.mode == Mode::AddGoal {
            selected_line = Some(0);
            selected_line_count = 1;
            let prefix = "⏳ ";
            let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
            spans.extend(self.goal_input.visible_spans(
                inner_width,
                Line::from(prefix).width() as u16,
                GREEN,
            ));
            lines.push(Line::from(spans));
        }

        for (index, goal) in self.goals.iter().enumerate() {
            let is_selected = index == self.selected;
            if is_selected && self.mode != Mode::AddGoal {
                selected_line = Some(lines.len());
            }

            if is_selected && self.mode == Mode::EditGoal {
                let prefix = "⏳ ";
                let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
                spans.extend(self.goal_input.visible_spans(
                    inner_width,
                    Line::from(prefix).width() as u16,
                    GREEN,
                ));
                lines.push(Line::from(spans));
                lines.push(goal_gauge(goal));
                selected_line_count = 2;
                lines.push(Line::default());
            } else if is_selected && self.mode == Mode::EditGoalDate {
                lines.push(Line::from(vec![
                    Span::styled("» ", Style::default().fg(GREEN)),
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(goal.title.clone(), Style::default().fg(GREEN)),
                ]));
                let prefix = format!("  {} ", tr("dashboard-date-prompt"));
                let prefix_width = Line::from(prefix.as_str()).width() as u16;
                let mut spans = vec![Span::styled(prefix, Style::default().fg(ACCENT))];
                spans.extend(
                    self.goal_input
                        .visible_spans(inner_width, prefix_width, GREEN),
                );
                lines.push(Line::from(spans));
                lines.push(goal_gauge(goal));
                selected_line_count = 3;
                lines.push(Line::default());
            } else if is_selected && self.mode == Mode::ConfirmDeleteGoal {
                let mut goal_lines = render_goal_lines(goal, true);
                goal_lines.pop();
                lines.extend(goal_lines);
                selected_line_count = 3;
                lines.push(delete_confirmation_line());
                lines.push(Line::default());
            } else {
                lines.extend(render_goal_lines(goal, is_selected));
                if is_selected {
                    selected_line_count = 2;
                }
            }
        }

        let spans = visual_spans(&lines, inner_width);
        let selected =
            selected_line.and_then(|line| combined_span(&spans, line, selected_line_count));
        SpannedListPanelContent {
            header: None,
            lines,
            empty: self.goals.is_empty().then(|| {
                Line::from(Span::styled(
                    tr("dashboard-press-a-goal"),
                    Style::default().fg(Color::Gray),
                ))
            }),
            selected,
        }
    }
}

pub struct GoalsScreen {
    goals: Vec<Goal>,
    selected: usize,
    list_viewport: ViewportState,
    mode: Mode,
    goal_input: TextInput,
    status_msg: StatusMessage,
    last_deleted: Option<GoalUndoData>,
    // Milestone state
    milestone_goal_idx: usize,
    milestone_selected: usize,
    milestone_input: TextInput,
    milestone_panel_state: DetailPanelState,
    milestone_mode: MilestoneMode,
    milestone_status_msg: StatusMessage,
    milestone_last_deleted: Option<GoalUndoData>,
}

impl GoalsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let goals = db.list_goals()?;
        Ok(Self {
            goals,
            selected: 0,
            list_viewport: ViewportState::new(0),
            mode: Mode::Browse,
            goal_input: TextInput::new(),
            status_msg: None,
            last_deleted: None,
            milestone_goal_idx: 0,
            milestone_selected: 0,
            milestone_input: TextInput::new(),
            milestone_panel_state: DetailPanelState::new(false),
            milestone_mode: MilestoneMode::Browse,
            milestone_status_msg: None,
            milestone_last_deleted: None,
        })
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        let selected_goal_id = self.goals.get(self.selected).map(|goal| goal.id);
        let milestone_goal_id = (self.mode == Mode::Milestones)
            .then(|| self.goals.get(self.milestone_goal_idx).map(|goal| goal.id))
            .flatten();
        let milestone_id = (self.mode == Mode::Milestones)
            .then(|| {
                self.goals
                    .get(self.milestone_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.milestone_selected))
                    .map(|milestone| milestone.id)
            })
            .flatten();
        self.goals = db.list_goals()?;
        if let Some(goal_id) = selected_goal_id {
            if let Some(index) = self.goals.iter().position(|goal| goal.id == goal_id) {
                self.selected = index;
            }
        }
        if let Some(goal_id) = milestone_goal_id {
            if let Some(index) = self.goals.iter().position(|goal| goal.id == goal_id) {
                self.milestone_goal_idx = index;
            }
        }
        if let Some(milestone_id) = milestone_id {
            if let Some(index) = self.goals.get(self.milestone_goal_idx).and_then(|goal| {
                goal.milestones
                    .iter()
                    .position(|milestone| milestone.id == milestone_id)
            }) {
                self.milestone_selected = index;
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

    fn active_shortcuts(&self) -> Vec<Shortcut> {
        match self.mode {
            Mode::Browse => {
                let goal = self.goals.get(self.selected);
                let mut shortcuts = Vec::new();
                if goal.is_some() {
                    shortcuts.push(Shortcut::new("j/k", tr("key-navigate")));
                }
                shortcuts.push(Shortcut::new("a", tr("key-add-goal")));
                if let Some(goal) = goal {
                    shortcuts.extend([
                        Shortcut::new("Enter", tr("key-milestone")),
                        Shortcut::new("m", tr("key-add")),
                        Shortcut::new("e", tr("key-edit")),
                        Shortcut::new("Space", tr("key-toggle")),
                        Shortcut::new("d", tr("key-delete")),
                    ]);
                    if goal.completed {
                        shortcuts.push(Shortcut::new("D", tr("key-date")));
                    }
                }
                if self.last_deleted.is_some() {
                    shortcuts.push(Shortcut::new("u", tr("key-undo")));
                }
                shortcuts.push(Shortcut::new("Esc", tr("key-back")));
                shortcuts
            }
            Mode::Milestones if self.milestone_mode == MilestoneMode::Browse => {
                let goal = self.goals.get(self.milestone_goal_idx);
                let milestone = goal.and_then(|goal| goal.milestones.get(self.milestone_selected));
                let mut shortcuts = Vec::new();
                if milestone.is_some() {
                    shortcuts.push(Shortcut::new("j/k", tr("key-navigate")));
                }
                if goal.is_some() {
                    shortcuts.push(Shortcut::new("a", tr("key-add")));
                }
                if let Some(milestone) = milestone {
                    shortcuts.extend([
                        Shortcut::new("Enter/e", tr("key-edit")),
                        Shortcut::new("Space", tr("key-toggle")),
                        Shortcut::new("d", tr("key-delete")),
                    ]);
                    if milestone.completed {
                        shortcuts.push(Shortcut::new("D", tr("key-date")));
                    }
                }
                if self.milestone_last_deleted.is_some() {
                    shortcuts.push(Shortcut::new("u", tr("key-undo")));
                }
                shortcuts.push(Shortcut::new("Esc", tr("key-back")));
                shortcuts
            }
            Mode::Milestones if self.milestone_mode == MilestoneMode::ConfirmDeleteMilestone => {
                vec![
                    Shortcut::new("y", tr("key-yes")),
                    Shortcut::new("any", tr("key-cancel")),
                ]
            }
            Mode::Milestones => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
            Mode::ConfirmDeleteGoal => vec![
                Shortcut::new("y", tr("key-yes")),
                Shortcut::new("any", tr("key-cancel")),
            ],
            Mode::AddGoal | Mode::EditGoal | Mode::EditGoalDate => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let shortcuts = self.active_shortcuts();
        let active_status = if self.mode == Mode::Milestones {
            &self.milestone_status_msg
        } else {
            &self.status_msg
        };
        let screen = render_screen_frame(
            frame,
            frame.area(),
            CONTENT_WIDTH,
            active_status,
            &shortcuts,
        );
        let panes = goals_pane_areas(screen.body);
        let goals_focused = self.mode != Mode::Milestones;
        let visible_goal_idx = if goals_focused {
            self.selected
        } else {
            self.milestone_goal_idx
        };
        let goal_panel = GoalPanelRenderState::capture(self);
        let milestone_inner_width = panes.milestones.width.saturating_sub(2);
        let milestone_content =
            self.build_milestone_content(visible_goal_idx, milestone_inner_width);
        let milestone_title = self
            .goals
            .get(visible_goal_idx)
            .map(|goal| goal.title.clone())
            .unwrap_or_else(|| tr("goals-milestones-title"));

        render_scrollable_list_panel_spanned(
            frame,
            panes.goals,
            tr("goals-title"),
            goals_focused,
            &mut self.list_viewport,
            |inner| goal_panel.build(inner.width),
        );

        self.milestone_panel_state.focused = !goals_focused;
        render_detail_panel_with_header(
            frame,
            panes.milestones,
            milestone_title,
            milestone_content.header,
            milestone_content.lines,
            &mut self.milestone_panel_state,
            milestone_content.selected,
        );
    }

    fn build_milestone_content(&self, goal_index: usize, inner_width: u16) -> MilestoneContent {
        let Some(goal) = self.goals.get(goal_index) else {
            return MilestoneContent {
                header: Vec::new(),
                lines: vec![Line::from(Span::styled(
                    tr("goals-no-selection"),
                    Style::default().fg(Color::DarkGray),
                ))],
                selected: None,
            };
        };
        let mut lines = Vec::new();
        let mut selected_line = None;
        let mut selected_line_count = 1;

        if goal.milestones.is_empty() && self.milestone_mode == MilestoneMode::Browse {
            lines.push(Line::from(Span::styled(
                tr("goals-no-milestones"),
                Style::default().fg(Color::DarkGray),
            )));
        }

        for (index, milestone) in goal.milestones.iter().enumerate() {
            let is_selected = self.mode == Mode::Milestones && index == self.milestone_selected;
            if is_selected && self.milestone_mode != MilestoneMode::AddMilestone {
                selected_line = Some(lines.len());
            }
            if is_selected && self.milestone_mode == MilestoneMode::EditMilestone {
                let marker = if milestone.completed { "✓ " } else { "⏳ " };
                let marker_color = if milestone.completed {
                    GREEN
                } else {
                    Color::Yellow
                };
                let mut spans = vec![Span::styled(
                    marker.to_string(),
                    Style::default().fg(marker_color),
                )];
                spans.extend(self.milestone_input.visible_spans(
                    inner_width,
                    Line::from(marker).width() as u16,
                    GREEN,
                ));
                lines.push(Line::from(spans));
            } else if is_selected && self.milestone_mode == MilestoneMode::EditMilestoneDate {
                selected_line_count = 2;
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(milestone.title.clone(), Style::default().fg(GREEN)),
                ]));
                let prefix = format!("  {} ", tr("dashboard-date-prompt"));
                let prefix_width = Line::from(prefix.as_str()).width() as u16;
                let mut spans = vec![Span::styled(prefix, Style::default().fg(ACCENT))];
                spans.extend(
                    self.milestone_input
                        .visible_spans(inner_width, prefix_width, GREEN),
                );
                lines.push(Line::from(spans));
            } else if is_selected && self.milestone_mode == MilestoneMode::ConfirmDeleteMilestone {
                selected_line_count = 2;
                lines.push(render_milestone_line(milestone, true));
                lines.push(delete_confirmation_line());
            } else {
                lines.push(render_milestone_line(milestone, is_selected));
            }
        }

        if self.milestone_mode == MilestoneMode::AddMilestone {
            selected_line = Some(lines.len());
            let prefix = "⏳ ";
            let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Yellow))];
            spans.extend(self.milestone_input.visible_spans(
                inner_width,
                Line::from(prefix).width() as u16,
                GREEN,
            ));
            lines.push(Line::from(spans));
        }

        let spans = visual_spans(&lines, inner_width);
        let selected =
            selected_line.and_then(|line| combined_span(&spans, line, selected_line_count));
        MilestoneContent {
            header: vec![goal_gauge(goal)],
            lines,
            selected,
        }
    }

    // ── Key handling ──

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        if self.mode != Mode::Milestones {
            self.status_msg = None;
        }

        match self.mode {
            Mode::Milestones => self.handle_milestone(key, db),
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
                    self.focus_milestones_for_selected_goal();
                    self.milestone_mode = MilestoneMode::Browse;
                }
                Action::None
            }
            KeyCode::Char('m') => {
                if !self.goals.is_empty() {
                    self.focus_milestones_for_selected_goal();
                    self.milestone_mode = MilestoneMode::AddMilestone;
                    self.milestone_input.clear();
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.goal_input.clear();
                self.mode = Mode::AddGoal;
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.selected) {
                    self.goal_input.set(goal.title.clone());
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
                        self.goal_input.clear();
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
            KeyCode::Esc => Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard)),
            _ => Action::None,
        }
    }

    fn focus_milestones_for_selected_goal(&mut self) {
        let previous_goal_id = self.goals.get(self.milestone_goal_idx).map(|goal| goal.id);
        let next_goal_id = self.goals.get(self.selected).map(|goal| goal.id);
        let previous_milestone_id = self
            .goals
            .get(self.milestone_goal_idx)
            .and_then(|goal| goal.milestones.get(self.milestone_selected))
            .map(|milestone| milestone.id);
        let previous_index = self.milestone_selected;

        self.mode = Mode::Milestones;
        self.milestone_goal_idx = self.selected;
        if previous_goal_id != next_goal_id {
            let milestones = &self.goals[self.milestone_goal_idx].milestones;
            self.milestone_selected = previous_milestone_id
                .and_then(|id| milestones.iter().position(|milestone| milestone.id == id))
                .unwrap_or_else(|| previous_index.min(milestones.len().saturating_sub(1)));
            if milestones.is_empty() {
                self.milestone_selected = 0;
            }
        }
    }

    fn handle_add_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trimmed().to_string();
                if !title.is_empty() {
                    match db.create_goal(&title) {
                        Ok(_) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.goal_input.clear();
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
                self.goal_input.clear();
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.goal_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trimmed().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.goals.get(self.selected).map(|goal| goal.id) {
                        match db.update_goal(goal_id, &title) {
                            Ok(()) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.goal_input.clear();
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
                self.goal_input.clear();
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.goal_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_edit_goal_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date =
                    match chrono::NaiveDate::parse_from_str(self.goal_input.value(), "%Y-%m-%d") {
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
                                self.goal_input.clear();
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
                self.goal_input.clear();
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.goal_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_confirm_delete_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                let mut delete_succeeded = false;
                if let Some(goal) = self.goals.get(self.selected).cloned() {
                    match db.delete_goal(goal.id) {
                        Ok(()) => {
                            delete_succeeded = true;
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
                if delete_succeeded {
                    self.mode = Mode::Browse;
                }
                Action::None
            }
            _ => {
                self.mode = Mode::Browse;
                Action::None
            }
        }
    }

    // ── Milestone key handling ──

    fn handle_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        if key.code != KeyCode::Esc {
            self.milestone_status_msg = None;
        }

        match self.milestone_mode {
            MilestoneMode::Browse => self.handle_milestone_browse(key, db),
            MilestoneMode::AddMilestone => self.handle_milestone_add_milestone(key, db),
            MilestoneMode::EditMilestone => self.handle_milestone_edit_milestone(key, db),
            MilestoneMode::EditMilestoneDate => self.handle_milestone_edit_milestone_date(key, db),
            MilestoneMode::ConfirmDeleteMilestone => self.handle_milestone_confirm_delete(key, db),
        }
    }

    fn milestone_count(&self) -> usize {
        self.goals
            .get(self.milestone_goal_idx)
            .map(|g| g.milestones.len())
            .unwrap_or(0)
    }

    fn handle_milestone_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        let ms_count = self.milestone_count();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if ms_count > 0 && self.milestone_selected < ms_count - 1 {
                    self.milestone_selected += 1;
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.milestone_selected > 0 {
                    self.milestone_selected -= 1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.milestone_input.clear();
                self.milestone_mode = MilestoneMode::AddMilestone;
                Action::None
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(goal) = self.goals.get(self.milestone_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.milestone_selected) {
                        self.milestone_input.set(ms.title.clone());
                        self.milestone_mode = MilestoneMode::EditMilestone;
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                let milestone_id = self
                    .goals
                    .get(self.milestone_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.milestone_selected))
                    .map(|milestone| milestone.id);
                if let Some(milestone_id) = milestone_id {
                    match db.toggle_milestone(milestone_id) {
                        Ok(()) => {
                            if let Err(e) = self.reload_goals(db) {
                                self.milestone_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                        Err(e) => {
                            self.milestone_status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('D') => {
                if let Some(goal) = self.goals.get(self.milestone_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.milestone_selected) {
                        if ms.completed {
                            self.milestone_input.clear();
                            self.milestone_mode = MilestoneMode::EditMilestoneDate;
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if ms_count > 0 {
                    self.milestone_mode = MilestoneMode::ConfirmDeleteMilestone;
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(undo_data) = self.milestone_last_deleted.take() {
                    let result = match &undo_data {
                        GoalUndoData::Goal(_) => Ok(()), // cannot occur in milestone undo state
                        GoalUndoData::Milestone { goal_id, milestone } => {
                            db.restore_milestone(*goal_id, milestone).map(|_| ())
                        }
                    };
                    match result {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.milestone_status_msg = Some((tr("status-restored"), false));
                            }
                            Err(e) => {
                                self.milestone_status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.milestone_last_deleted = Some(undo_data);
                            self.milestone_status_msg =
                                Some((format!("Restore failed: {}", e), true));
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

    fn handle_milestone_add_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.milestone_input.trimmed().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) =
                        self.goals.get(self.milestone_goal_idx).map(|goal| goal.id)
                    {
                        match db.create_milestone(goal_id, &title) {
                            Ok(milestone_id) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.milestone_selected = self
                                        .goals
                                        .get(self.milestone_goal_idx)
                                        .and_then(|goal| {
                                            goal.milestones
                                                .iter()
                                                .position(|milestone| milestone.id == milestone_id)
                                        })
                                        .unwrap_or_else(|| {
                                            self.milestone_count().saturating_sub(1)
                                        });
                                    self.milestone_input.clear();
                                    self.milestone_mode = MilestoneMode::Browse;
                                }
                                Err(e) => {
                                    self.milestone_status_msg =
                                        Some((format!("Error: {}", e), true));
                                }
                            },
                            Err(e) => {
                                self.milestone_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.milestone_input.clear();
                self.milestone_mode = MilestoneMode::Browse;
                Action::None
            }
            _ => {
                self.milestone_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_milestone_edit_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.milestone_input.trimmed().to_string();
                if !title.is_empty() {
                    let milestone_id = self
                        .goals
                        .get(self.milestone_goal_idx)
                        .and_then(|goal| goal.milestones.get(self.milestone_selected))
                        .map(|milestone| milestone.id);
                    if let Some(milestone_id) = milestone_id {
                        match db.update_milestone(milestone_id, &title) {
                            Ok(()) => match self.reload_goals(db) {
                                Ok(()) => {
                                    self.milestone_input.clear();
                                    self.milestone_mode = MilestoneMode::Browse;
                                }
                                Err(e) => {
                                    self.milestone_status_msg =
                                        Some((format!("Error: {}", e), true));
                                }
                            },
                            Err(e) => {
                                self.milestone_status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.milestone_input.clear();
                self.milestone_mode = MilestoneMode::Browse;
                Action::None
            }
            _ => {
                self.milestone_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_milestone_edit_milestone_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let date = match chrono::NaiveDate::parse_from_str(
                    self.milestone_input.value(),
                    "%Y-%m-%d",
                ) {
                    Ok(date) => date,
                    Err(_) => {
                        self.milestone_status_msg = Some((tr("goals-invalid-date"), true));
                        return Action::None;
                    }
                };
                let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                let milestone_id = self
                    .goals
                    .get(self.milestone_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.milestone_selected))
                    .map(|milestone| milestone.id);
                if let Some(milestone_id) = milestone_id {
                    match db.set_milestone_completed_at(milestone_id, &completed_at) {
                        Ok(()) => match self.reload_goals(db) {
                            Ok(()) => {
                                self.milestone_input.clear();
                                self.milestone_mode = MilestoneMode::Browse;
                            }
                            Err(e) => {
                                self.milestone_status_msg = Some((format!("Error: {}", e), true));
                            }
                        },
                        Err(e) => {
                            self.milestone_status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.milestone_input.clear();
                self.milestone_mode = MilestoneMode::Browse;
                Action::None
            }
            _ => {
                self.milestone_input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_milestone_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                let mut delete_succeeded = false;
                if let Some(goal) = self.goals.get(self.milestone_goal_idx) {
                    if let Some(ms) = goal.milestones.get(self.milestone_selected).cloned() {
                        let goal_id = goal.id;
                        match db.delete_milestone(ms.id) {
                            Ok(()) => {
                                delete_succeeded = true;
                                self.milestone_last_deleted = Some(GoalUndoData::Milestone {
                                    goal_id,
                                    milestone: ms,
                                });
                                match self.reload_goals(db) {
                                    Ok(()) => {
                                        self.milestone_status_msg =
                                            Some((tr("status-deleted-undo"), false));
                                    }
                                    Err(e) => {
                                        self.milestone_status_msg =
                                            Some((format!("Error: {}", e), true));
                                    }
                                }
                            }
                            Err(e) => {
                                self.milestone_status_msg =
                                    Some((format!("Delete failed: {}", e), true));
                            }
                        }
                        // Adjust selected index after deletion
                        let new_count = self.milestone_count();
                        if new_count == 0 {
                            self.milestone_selected = 0;
                        } else if self.milestone_selected >= new_count {
                            self.milestone_selected = new_count - 1;
                        }
                    }
                }
                if delete_succeeded {
                    self.milestone_mode = MilestoneMode::Browse;
                }
                Action::None
            }
            _ => {
                self.milestone_mode = MilestoneMode::Browse;
                Action::None
            }
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn titles_for_test(&self) -> Vec<String> {
        self.goals.iter().map(|goal| goal.title.clone()).collect()
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn context_for_test(&self) -> (Option<i64>, usize, bool) {
        (
            self.goals.get(self.selected).map(|goal| goal.id),
            self.list_viewport.offset(),
            self.mode == Mode::Milestones,
        )
    }
}

impl Reenter for GoalsScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        let selected_goal_id = self.goals.get(self.selected).map(|goal| goal.id);
        let milestone_goal_id = (self.mode == Mode::Milestones)
            .then(|| self.goals.get(self.milestone_goal_idx).map(|goal| goal.id))
            .flatten();
        let selected_milestone_id = (self.mode == Mode::Milestones)
            .then(|| {
                self.goals
                    .get(self.milestone_goal_idx)
                    .and_then(|goal| goal.milestones.get(self.milestone_selected))
                    .map(|milestone| milestone.id)
            })
            .flatten();
        let mut next = GoalsScreen::new(db)?;

        next.selected = selected_goal_id
            .and_then(|id| next.goals.iter().position(|goal| goal.id == id))
            .unwrap_or_else(|| self.selected.min(next.goals.len().saturating_sub(1)));
        if next.goals.is_empty() {
            next.selected = 0;
        }

        next.milestone_goal_idx = milestone_goal_id
            .and_then(|id| next.goals.iter().position(|goal| goal.id == id))
            .unwrap_or_else(|| {
                self.milestone_goal_idx
                    .min(next.goals.len().saturating_sub(1))
            });
        if next.goals.is_empty() {
            next.milestone_goal_idx = 0;
        }
        let milestone_count = next
            .goals
            .get(next.milestone_goal_idx)
            .map(|goal| goal.milestones.len())
            .unwrap_or(0);
        next.milestone_selected = selected_milestone_id
            .and_then(|id| {
                next.goals.get(next.milestone_goal_idx).and_then(|goal| {
                    goal.milestones
                        .iter()
                        .position(|milestone| milestone.id == id)
                })
            })
            .unwrap_or_else(|| {
                self.milestone_selected
                    .min(milestone_count.saturating_sub(1))
            });
        if milestone_count == 0 {
            next.milestone_selected = 0;
        }

        next.list_viewport = self.list_viewport;
        next.milestone_panel_state.viewport = self.milestone_panel_state.viewport;

        next.mode = if self.mode == Mode::Milestones && !next.goals.is_empty() {
            Mode::Milestones
        } else {
            Mode::Browse
        };
        next.milestone_panel_state.focused = next.mode == Mode::Milestones;
        next.milestone_mode = MilestoneMode::Browse;
        next.goal_input.clear();
        next.milestone_input.clear();
        next.status_msg = None;
        next.milestone_status_msg = None;
        next.last_deleted = None;
        next.milestone_last_deleted = None;

        Ok(next)
    }
}

impl NavigationStatus for GoalsScreen {
    fn set_navigation_error(&mut self, message: String) {
        if self.mode == Mode::Milestones {
            self.milestone_status_msg = Some((message, true));
        } else {
            self.status_msg = Some((message, true));
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

fn delete_confirmation_line() -> Line<'static> {
    Line::from(vec![
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
    ])
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
    use rusqlite::Connection;

    use crate::tui::framework::test_support::render_to_snapshot;
    use crate::tui::framework::ViewportState;
    use crate::tui::lifecycle::{NavigationStatus, Reenter};
    use crate::tui::BORDER_COLOR;

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

    fn goals_db_with_milestones(count: usize, completed: usize) -> Database {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal("Goal").unwrap();
        for number in 1..=count {
            let milestone_id = db
                .create_milestone(goal_id, &format!("Milestone {number:02}"))
                .unwrap();
            if number <= completed {
                db.toggle_milestone(milestone_id).unwrap();
            }
        }
        db
    }

    fn fixed_goals_time() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 14)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap()
    }

    fn fixed_milestone(
        id: i64,
        goal_id: i64,
        position: i32,
        title: impl Into<String>,
        completed: bool,
    ) -> Milestone {
        Milestone {
            id,
            goal_id,
            title: title.into(),
            completed,
            position,
            created_at: fixed_goals_time(),
            completed_at: completed.then_some(fixed_goals_time()),
        }
    }

    fn fixed_goal(
        id: i64,
        position: i32,
        title: impl Into<String>,
        completed: bool,
        milestones: Vec<Milestone>,
    ) -> Goal {
        Goal {
            id,
            title: title.into(),
            completed,
            position,
            created_at: fixed_goals_time(),
            completed_at: completed.then_some(fixed_goals_time()),
            milestones,
        }
    }

    fn base_golden_goals() -> Vec<Goal> {
        vec![
            fixed_goal(
                101,
                0,
                "Ship responsive Goals panels",
                false,
                vec![
                    fixed_milestone(1_001, 101, 0, "Bind persistent pane focus", true),
                    fixed_milestone(1_002, 101, 1, "Keep progress pinned", false),
                    fixed_milestone(1_003, 101, 2, "Preserve stable identities", true),
                    fixed_milestone(1_004, 101, 3, "Freeze styled buffers", false),
                ],
            ),
            fixed_goal(102, 1, "Complete migration", true, Vec::new()),
            fixed_goal(
                103,
                2,
                "长期目标🚀：让训练计划在窄屏和宽屏之间保持清晰、稳定、可操作".repeat(2),
                false,
                Vec::new(),
            ),
        ]
    }

    fn goals_fixture_size(name: &str) -> (u16, u16) {
        match name {
            "wide_goal_focus"
            | "wide_milestones_focus"
            | "goal_edit"
            | "milestone_add"
            | "milestone_confirm_delete" => (107, 24),
            "narrow_goal_focus"
            | "narrow_milestones_focus"
            | "overflow"
            | "empty_goals"
            | "empty_milestones" => (80, 24),
            other => panic!("unknown Goals fixture size: {other}"),
        }
    }

    fn golden_goals_screen(name: &str) -> GoalsScreen {
        std::env::set_var("LC_ALL", "C");
        crate::i18n::init();

        let goals = base_golden_goals();
        let mut screen = GoalsScreen {
            goals,
            selected: 0,
            list_viewport: ViewportState::new(0),
            mode: Mode::Browse,
            goal_input: TextInput::from("inactive goal input".to_owned()),
            status_msg: Some(("Goals fixture status".into(), false)),
            last_deleted: Some(GoalUndoData::Goal(fixed_goal(
                9_001,
                99,
                "Deleted goal",
                false,
                Vec::new(),
            ))),
            milestone_goal_idx: 0,
            milestone_selected: 1,
            milestone_input: TextInput::from("inactive milestone input".to_owned()),
            milestone_panel_state: DetailPanelState::new(false),
            milestone_mode: MilestoneMode::Browse,
            milestone_status_msg: Some(("Milestones fixture status".into(), false)),
            milestone_last_deleted: Some(GoalUndoData::Milestone {
                goal_id: 101,
                milestone: fixed_milestone(9_002, 101, 99, "Deleted milestone", false),
            }),
        };

        match name {
            "wide_goal_focus" | "narrow_goal_focus" => {}
            "wide_milestones_focus" | "narrow_milestones_focus" => {
                screen.mode = Mode::Milestones;
                screen.milestone_panel_state.focused = true;
            }
            "goal_edit" => {
                screen.selected = 2;
                screen.mode = Mode::EditGoal;
                screen.goal_input = TextInput::from("编辑后的长期目标🚀".repeat(4));
                screen.list_viewport.set_offset(4);
            }
            "milestone_add" => {
                screen.mode = Mode::Milestones;
                screen.milestone_mode = MilestoneMode::AddMilestone;
                screen.milestone_input =
                    TextInput::from("新增里程碑：验证固定标题与独立滚动".repeat(3));
                screen.milestone_panel_state.focused = true;
                screen.milestone_panel_state.viewport.set_offset(1);
            }
            "milestone_confirm_delete" => {
                screen.mode = Mode::Milestones;
                screen.milestone_selected = 2;
                screen.milestone_mode = MilestoneMode::ConfirmDeleteMilestone;
                screen.milestone_panel_state.focused = true;
                screen.milestone_panel_state.viewport.set_offset(1);
                screen.milestone_status_msg = Some(("Delete selected milestone?".into(), true));
            }
            "overflow" => {
                screen.goals = (0..20)
                    .map(|index| {
                        let goal_id = 2_000 + index;
                        let milestones = if index == 19 {
                            (0..20)
                                .map(|milestone_index| {
                                    fixed_milestone(
                                        30_000 + milestone_index,
                                        goal_id,
                                        milestone_index as i32,
                                        format!("Overflow milestone {:02}", milestone_index + 1),
                                        milestone_index % 3 == 0,
                                    )
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };
                        fixed_goal(
                            goal_id,
                            index as i32,
                            format!("Overflow goal {:02}", index + 1),
                            false,
                            milestones,
                        )
                    })
                    .collect();
                screen.selected = 19;
                screen.milestone_goal_idx = 19;
                screen.milestone_selected = 19;
                screen.mode = Mode::Milestones;
                screen.list_viewport.set_offset(54);
                screen.milestone_panel_state.focused = true;
                screen.milestone_panel_state.viewport.set_offset(15);
            }
            "empty_goals" => {
                screen.goals.clear();
                screen.selected = 0;
                screen.milestone_goal_idx = 0;
                screen.milestone_selected = 0;
                screen.last_deleted = None;
                screen.milestone_last_deleted = None;
            }
            "empty_milestones" => {
                screen.goals = vec![fixed_goal(
                    501,
                    0,
                    "Goal awaiting its first milestone",
                    false,
                    Vec::new(),
                )];
                screen.mode = Mode::Milestones;
                screen.milestone_goal_idx = 0;
                screen.milestone_selected = 0;
                screen.milestone_panel_state.focused = true;
                screen.milestone_last_deleted = None;
            }
            other => panic!("unknown Goals golden state: {other}"),
        }

        screen
    }

    fn goals_golden(name: &str) -> &'static str {
        match name {
            "wide_goal_focus" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/wide_goal_focus.txt"
            )),
            "wide_milestones_focus" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/wide_milestones_focus.txt"
            )),
            "narrow_goal_focus" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/narrow_goal_focus.txt"
            )),
            "narrow_milestones_focus" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/narrow_milestones_focus.txt"
            )),
            "goal_edit" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/goal_edit.txt"
            )),
            "milestone_add" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/milestone_add.txt"
            )),
            "milestone_confirm_delete" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/milestone_confirm_delete.txt"
            )),
            "overflow" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/overflow.txt"
            )),
            "empty_goals" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/empty_goals.txt"
            )),
            "empty_milestones" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/goals/empty_milestones.txt"
            )),
            other => panic!("unknown Goals golden fixture: {other}"),
        }
    }

    #[test]
    fn goals_states_match_golden_buffers() {
        for name in [
            "wide_goal_focus",
            "wide_milestones_focus",
            "narrow_goal_focus",
            "narrow_milestones_focus",
            "goal_edit",
            "milestone_add",
            "milestone_confirm_delete",
            "overflow",
            "empty_goals",
            "empty_milestones",
        ] {
            let (width, height) = goals_fixture_size(name);
            let mut screen = golden_goals_screen(name);
            let actual = render_to_snapshot(width, height, |frame| screen.render(frame));
            let expected = goals_golden(name);
            assert_eq!(actual, expected, "Goals {name} buffer changed");
        }
    }

    fn rect_contains(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    #[test]
    fn goals_wide_render_keeps_both_panes_visible_without_obscuring_goals() {
        let db = goals_db_with_milestones(6, 2);
        let mut screen = GoalsScreen::new(&db).unwrap();
        let output = render_screen(&mut screen, 107, 24);

        assert!(output.contains(&tr("goals-title")), "{output}");
        assert!(output.contains("Milestone 06"), "{output}");
        assert!(output.contains("2/6"), "{output}");
    }

    #[test]
    fn goals_narrow_render_stacks_both_panes() {
        let db = goals_db_with_milestones(4, 0);
        let mut screen = GoalsScreen::new(&db).unwrap();
        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains("0/4 · 0%"), "{output}");
        assert!(output.matches('┌').count() >= 2, "{output}");
    }

    #[test]
    fn milestone_progress_header_remains_visible_after_scrolling() {
        let db = goals_db_with_milestones(20, 0);
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        for _ in 1..20 {
            screen.handle_key(key(KeyCode::Down), &db);
            render_screen(&mut screen, 80, 24);
        }
        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains("Milestone 20"), "{output}");
        assert!(output.contains("0/20 · 0%"), "{output}");
    }

    #[test]
    fn goals_focus_changes_only_active_panel_style() {
        let db = goals_db_with_milestones(2, 0);
        let mut screen = GoalsScreen::new(&db).unwrap();
        let goals_focused =
            crate::tui::framework::test_support::render_to_buffer(107, 24, |frame| {
                screen.render(frame)
            });

        assert_eq!(goals_focused[(0, 1)].fg, Color::Cyan);
        assert_eq!(goals_focused[(43, 1)].fg, BORDER_COLOR);
        assert_eq!(goals_focused[(2, 2)].bg, Color::Rgb(204, 163, 0));
        assert_ne!(goals_focused[(44, 2)].bg, Color::Rgb(204, 163, 0));

        screen.handle_key(key(KeyCode::Enter), &db);
        let milestones_focused =
            crate::tui::framework::test_support::render_to_buffer(107, 24, |frame| {
                screen.render(frame)
            });
        assert_eq!(milestones_focused[(0, 1)].fg, BORDER_COLOR);
        assert_eq!(milestones_focused[(43, 1)].fg, Color::Cyan);
        assert_ne!(milestones_focused[(2, 2)].bg, Color::Rgb(204, 163, 0));
        assert_eq!(milestones_focused[(44, 2)].bg, Color::Rgb(204, 163, 0));
    }

    #[test]
    fn goals_narrow_focus_switch_keeps_both_panes() {
        let db = goals_db_with_milestones(4, 0);
        let mut screen = GoalsScreen::new(&db).unwrap();
        let before = render_screen(&mut screen, 80, 24);
        let goals_focused =
            crate::tui::framework::test_support::render_to_buffer(80, 24, |frame| {
                screen.render(frame)
            });
        let panes = goals_pane_areas(Rect::new(0, 0, 80, 21));
        assert_eq!(
            goals_focused[(panes.goals.x, panes.goals.y + 1)].fg,
            Color::Cyan
        );
        assert_eq!(
            goals_focused[(panes.milestones.x, panes.milestones.y + 1)].fg,
            BORDER_COLOR
        );

        screen.handle_key(key(KeyCode::Enter), &db);
        let after = render_screen(&mut screen, 80, 24);
        let milestones_focused =
            crate::tui::framework::test_support::render_to_buffer(80, 24, |frame| {
                screen.render(frame)
            });
        assert!(before.contains(&tr("goals-title")), "{before}");
        assert!(before.contains("Goal"), "{before}");
        assert!(after.contains(&tr("goals-title")), "{after}");
        assert!(after.contains("Goal"), "{after}");
        assert_eq!(
            milestones_focused[(panes.goals.x, panes.goals.y + 1)].fg,
            BORDER_COLOR
        );
        assert_eq!(
            milestones_focused[(panes.milestones.x, panes.milestones.y + 1)].fg,
            Color::Cyan
        );
    }

    #[test]
    fn goals_wrapped_selection_is_fully_highlighted() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal(&"目标🚀".repeat(30)).unwrap();
        db.create_milestone(goal_id, &"里程碑🚀".repeat(30))
            .unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();

        let goals_focused =
            crate::tui::framework::test_support::render_to_buffer(107, 24, |frame| {
                screen.render(frame)
            });
        for y in 2..=7 {
            assert_eq!(goals_focused[(2, y)].bg, Color::Rgb(204, 163, 0));
        }
        assert!(!(1..22).any(|y| goals_focused[(44, y)].bg == Color::Rgb(204, 163, 0)));

        screen.handle_key(key(KeyCode::Enter), &db);
        let milestones_focused =
            crate::tui::framework::test_support::render_to_buffer(107, 24, |frame| {
                screen.render(frame)
            });
        assert!(!(1..22).any(|y| milestones_focused[(2, y)].bg == Color::Rgb(204, 163, 0)));
        for y in 2..=4 {
            assert_eq!(milestones_focused[(44, y)].bg, Color::Rgb(204, 163, 0));
        }
    }

    #[test]
    fn goals_resize_preserves_identity_input_and_viewports() {
        let db = Database::open_in_memory().unwrap();
        for number in 1..=12 {
            db.create_goal(&format!("Goal {number:02}")).unwrap();
        }
        let active_goal_id = GoalsScreen::new(&db).unwrap().goals[1].id;
        let mut milestone_ids = Vec::new();
        for number in 1..=24 {
            milestone_ids.push(
                db.create_milestone(active_goal_id, &format!("Milestone {number:02}"))
                    .unwrap(),
            );
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == active_goal_id)
            .unwrap();
        screen.mode = Mode::Milestones;
        screen.milestone_goal_idx = screen.selected;
        screen.milestone_selected = 5;
        screen.milestone_mode = MilestoneMode::EditMilestone;
        screen.goal_input.set("GOAL-SENTINEL");
        screen.milestone_input.set("里程碑-edit-".repeat(20));
        let goal_cursor = screen.goal_input.cursor();
        let milestone_cursor = screen.milestone_input.cursor();
        screen.last_deleted = Some(GoalUndoData::Goal(screen.goals[0].clone()));
        screen.milestone_last_deleted = Some(GoalUndoData::Milestone {
            goal_id: active_goal_id,
            milestone: screen.goals[screen.milestone_goal_idx].milestones[0].clone(),
        });
        screen.list_viewport.set_offset(2);
        screen.milestone_panel_state.viewport.set_offset(2);

        for (width, height) in [(107, 24), (80, 24), (107, 24)] {
            render_screen(&mut screen, width, height);
            assert_eq!(screen.goals[screen.selected].id, active_goal_id);
            assert_eq!(
                screen.goals[screen.milestone_goal_idx].milestones[screen.milestone_selected].id,
                milestone_ids[5]
            );
            assert_eq!(screen.mode, Mode::Milestones);
            assert_eq!(screen.milestone_mode, MilestoneMode::EditMilestone);
            assert_eq!(screen.goal_input.value(), "GOAL-SENTINEL");
            assert_eq!(screen.goal_input.cursor(), goal_cursor);
            assert_eq!(screen.milestone_input.value(), "里程碑-edit-".repeat(20));
            assert_eq!(screen.milestone_input.cursor(), milestone_cursor);
            assert!(screen.last_deleted.is_some());
            assert!(screen.milestone_last_deleted.is_some());
            assert_eq!(screen.list_viewport.offset(), 2);
            assert_eq!(screen.milestone_panel_state.viewport.offset(), 2);
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ShortcutPane {
        Goals,
        Milestones,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ShortcutSelection {
        Empty,
        Incomplete,
        Complete,
    }

    #[derive(Debug, Clone, Copy)]
    struct BrowseShortcutCase {
        pane: ShortcutPane,
        selection: ShortcutSelection,
        undo: bool,
    }

    #[derive(Debug, Clone, Copy)]
    enum ShortcutCase {
        Browse(BrowseShortcutCase),
        GoalMode(Mode),
        MilestoneMode(MilestoneMode),
    }

    fn shortcut_case(case: ShortcutCase) -> (Database, GoalsScreen) {
        let db = Database::open_in_memory().unwrap();
        let goals_empty = matches!(
            case,
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Goals,
                selection: ShortcutSelection::Empty,
                ..
            })
        );
        let goal_id = if goals_empty {
            None
        } else {
            db.create_goal("Goal A").unwrap();
            let goal_id = db.create_goal("Goal B").unwrap();
            db.create_goal("Goal C").unwrap();
            Some(goal_id)
        };
        let milestones_nonempty = matches!(
            case,
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Milestones,
                selection: ShortcutSelection::Incomplete | ShortcutSelection::Complete,
                ..
            }) | ShortcutCase::MilestoneMode(_)
        );
        if milestones_nonempty {
            let goal_id = goal_id.unwrap();
            db.create_milestone(goal_id, "First").unwrap();
            db.create_milestone(goal_id, "Selected").unwrap();
            db.create_milestone(goal_id, "Third").unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        match case {
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Goals,
                selection,
                undo,
            }) => {
                if selection != ShortcutSelection::Empty {
                    screen.selected = 1;
                    screen.goals[screen.selected].completed =
                        selection == ShortcutSelection::Complete;
                }
                if undo {
                    screen.last_deleted = Some(GoalUndoData::Goal(Goal {
                        id: -10,
                        title: "Deleted goal".into(),
                        completed: false,
                        position: 99,
                        created_at: chrono::NaiveDateTime::default(),
                        completed_at: None,
                        milestones: Vec::new(),
                    }));
                }
            }
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Milestones,
                selection,
                undo,
            }) => {
                screen.mode = Mode::Milestones;
                screen.milestone_goal_idx = screen
                    .goals
                    .iter()
                    .position(|goal| goal.id == goal_id.unwrap())
                    .unwrap();
                if selection != ShortcutSelection::Empty {
                    screen.milestone_selected = 1;
                    screen.goals[screen.milestone_goal_idx].milestones[screen.milestone_selected]
                        .completed = selection == ShortcutSelection::Complete;
                }
                if undo {
                    screen.milestone_last_deleted = Some(GoalUndoData::Milestone {
                        goal_id: goal_id.unwrap(),
                        milestone: Milestone {
                            id: -20,
                            goal_id: goal_id.unwrap(),
                            title: "Deleted milestone".into(),
                            completed: false,
                            position: 99,
                            created_at: chrono::NaiveDateTime::default(),
                            completed_at: None,
                        },
                    });
                }
            }
            ShortcutCase::GoalMode(mode) => {
                screen.mode = mode;
                screen.selected = 1;
                match mode {
                    Mode::AddGoal | Mode::EditGoal => screen.goal_input.set("Changed"),
                    Mode::EditGoalDate => {
                        screen.goals[screen.selected].completed = true;
                        screen.goal_input.set("2026-08-17");
                    }
                    _ => {}
                }
            }
            ShortcutCase::MilestoneMode(mode) => {
                screen.milestone_goal_idx = screen
                    .goals
                    .iter()
                    .position(|goal| goal.id == goal_id.unwrap())
                    .unwrap();
                screen.milestone_selected = 1;
                screen.mode = Mode::Milestones;
                screen.milestone_mode = mode;
                match mode {
                    MilestoneMode::AddMilestone | MilestoneMode::EditMilestone => {
                        screen.milestone_input.set("Changed")
                    }
                    MilestoneMode::EditMilestoneDate => {
                        screen.goals[screen.milestone_goal_idx].milestones
                            [screen.milestone_selected]
                            .completed = true;
                        screen.milestone_input.set("2026-08-17")
                    }
                    _ => {}
                }
            }
        }
        (db, screen)
    }

    fn shortcut_keys(key_name: &str) -> Vec<KeyCode> {
        match key_name {
            "j/k" => vec![KeyCode::Char('j'), KeyCode::Char('k')],
            "a" => vec![KeyCode::Char('a')],
            "Enter" => vec![KeyCode::Enter],
            "Enter/e" => vec![KeyCode::Enter, KeyCode::Char('e')],
            "m" => vec![KeyCode::Char('m')],
            "e" => vec![KeyCode::Char('e')],
            "Space" => vec![KeyCode::Char(' ')],
            "d" => vec![KeyCode::Char('d')],
            "D" => vec![KeyCode::Char('D')],
            "u" => vec![KeyCode::Char('u')],
            "Esc" => vec![KeyCode::Esc],
            "y" => vec![KeyCode::Char('y')],
            "any" => vec![KeyCode::Char('n')],
            other => panic!("unmapped advertised shortcut {other}"),
        }
    }

    fn screen_witness(screen: &GoalsScreen, db: &Database) -> String {
        format!(
            "{:?}|{:?}|{}|{}|{}|{}|{:?}|{:?}|{}|{}|{:?}",
            screen.mode,
            screen.milestone_mode,
            screen.selected,
            screen.milestone_selected,
            screen.goal_input.value(),
            screen.milestone_input.value(),
            screen.status_msg,
            screen.milestone_status_msg,
            screen.last_deleted.is_some(),
            screen.milestone_last_deleted.is_some(),
            db.list_goals().unwrap()
        )
    }

    fn expected_shortcuts(case: ShortcutCase) -> Vec<Shortcut> {
        match case {
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Goals,
                selection,
                undo,
            }) => {
                let mut expected = Vec::new();
                if selection != ShortcutSelection::Empty {
                    expected.push(Shortcut::new("j/k", tr("key-navigate")));
                }
                expected.push(Shortcut::new("a", tr("key-add-goal")));
                if selection != ShortcutSelection::Empty {
                    expected.extend([
                        Shortcut::new("Enter", tr("key-milestone")),
                        Shortcut::new("m", tr("key-add")),
                        Shortcut::new("e", tr("key-edit")),
                        Shortcut::new("Space", tr("key-toggle")),
                        Shortcut::new("d", tr("key-delete")),
                    ]);
                    if selection == ShortcutSelection::Complete {
                        expected.push(Shortcut::new("D", tr("key-date")));
                    }
                }
                if undo {
                    expected.push(Shortcut::new("u", tr("key-undo")));
                }
                expected.push(Shortcut::new("Esc", tr("key-back")));
                expected
            }
            ShortcutCase::Browse(BrowseShortcutCase {
                pane: ShortcutPane::Milestones,
                selection,
                undo,
            }) => {
                let mut expected = Vec::new();
                if selection != ShortcutSelection::Empty {
                    expected.push(Shortcut::new("j/k", tr("key-navigate")));
                }
                expected.push(Shortcut::new("a", tr("key-add")));
                if selection != ShortcutSelection::Empty {
                    expected.extend([
                        Shortcut::new("Enter/e", tr("key-edit")),
                        Shortcut::new("Space", tr("key-toggle")),
                        Shortcut::new("d", tr("key-delete")),
                    ]);
                    if selection == ShortcutSelection::Complete {
                        expected.push(Shortcut::new("D", tr("key-date")));
                    }
                }
                if undo {
                    expected.push(Shortcut::new("u", tr("key-undo")));
                }
                expected.push(Shortcut::new("Esc", tr("key-back")));
                expected
            }
            ShortcutCase::GoalMode(Mode::ConfirmDeleteGoal)
            | ShortcutCase::MilestoneMode(MilestoneMode::ConfirmDeleteMilestone) => vec![
                Shortcut::new("y", tr("key-yes")),
                Shortcut::new("any", tr("key-cancel")),
            ],
            ShortcutCase::GoalMode(_) | ShortcutCase::MilestoneMode(_) => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
        }
    }

    #[test]
    fn goals_shortcuts_match_active_handlers() {
        let mut cases = Vec::new();
        for pane in [ShortcutPane::Goals, ShortcutPane::Milestones] {
            for selection in [
                ShortcutSelection::Empty,
                ShortcutSelection::Incomplete,
                ShortcutSelection::Complete,
            ] {
                for undo in [false, true] {
                    cases.push(ShortcutCase::Browse(BrowseShortcutCase {
                        pane,
                        selection,
                        undo,
                    }));
                }
            }
        }
        cases.extend([
            ShortcutCase::GoalMode(Mode::AddGoal),
            ShortcutCase::GoalMode(Mode::EditGoal),
            ShortcutCase::GoalMode(Mode::EditGoalDate),
            ShortcutCase::GoalMode(Mode::ConfirmDeleteGoal),
            ShortcutCase::MilestoneMode(MilestoneMode::AddMilestone),
            ShortcutCase::MilestoneMode(MilestoneMode::EditMilestone),
            ShortcutCase::MilestoneMode(MilestoneMode::EditMilestoneDate),
            ShortcutCase::MilestoneMode(MilestoneMode::ConfirmDeleteMilestone),
        ]);

        for case in cases {
            let expected = expected_shortcuts(case);
            let (_, screen) = shortcut_case(case);
            assert_eq!(screen.active_shortcuts(), expected, "case: {case:?}");
            for shortcut in expected {
                for code in shortcut_keys(&shortcut.key) {
                    let (db, mut screen) = shortcut_case(case);
                    let before = screen_witness(&screen, &db);
                    let action = screen.handle_key(key(code), &db);
                    let after = screen_witness(&screen, &db);
                    assert!(
                        before != after || action != Action::None,
                        "case {case:?}: advertised shortcut {shortcut:?} constituent {code:?} did not dispatch an active branch"
                    );
                }
            }
        }
    }

    #[test]
    fn goals_all_modes_render_safely_in_small_rectangles() {
        let db = goals_db_with_milestones(1, 0);
        let sizes = [(0, 0), (1, 1), (12, 4), (24, 10), (40, 16), (80, 24)];
        let states = [
            (Mode::Browse, MilestoneMode::Browse),
            (Mode::AddGoal, MilestoneMode::Browse),
            (Mode::EditGoal, MilestoneMode::Browse),
            (Mode::EditGoalDate, MilestoneMode::Browse),
            (Mode::ConfirmDeleteGoal, MilestoneMode::Browse),
            (Mode::Milestones, MilestoneMode::Browse),
            (Mode::Milestones, MilestoneMode::AddMilestone),
            (Mode::Milestones, MilestoneMode::EditMilestone),
            (Mode::Milestones, MilestoneMode::EditMilestoneDate),
            (Mode::Milestones, MilestoneMode::ConfirmDeleteMilestone),
        ];
        for (mode, milestone_mode) in states {
            for (width, height) in sizes {
                let mut screen = GoalsScreen::new(&db).unwrap();
                screen.mode = mode;
                screen.milestone_mode = milestone_mode;
                let mut body = None;
                crate::tui::framework::test_support::render_to_buffer(width, height, |frame| {
                    let shortcuts = screen.active_shortcuts();
                    let active_status = if screen.mode == Mode::Milestones {
                        &screen.milestone_status_msg
                    } else {
                        &screen.status_msg
                    };
                    body = Some(
                        render_screen_frame(
                            frame,
                            frame.area(),
                            CONTENT_WIDTH,
                            active_status,
                            &shortcuts,
                        )
                        .body,
                    );
                });
                let _ =
                    crate::tui::framework::test_support::render_to_buffer(width, height, |frame| {
                        screen.render(frame)
                    });
                let body = body.unwrap();
                let panes = goals_pane_areas(body);
                assert!(
                    rect_contains(body, panes.goals),
                    "{mode:?}/{milestone_mode:?} {width}x{height}: {body:?} -> {panes:?}"
                );
                assert!(
                    rect_contains(body, panes.milestones),
                    "{mode:?}/{milestone_mode:?} {width}x{height}: {body:?} -> {panes:?}"
                );
            }
        }
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
            goal_input: TextInput::new(),
            status_msg: None,
            last_deleted: None,
            milestone_goal_idx: 0,
            milestone_selected: 0,
            milestone_input: TextInput::new(),
            milestone_panel_state: DetailPanelState::new(false),
            milestone_mode: MilestoneMode::Browse,
            milestone_status_msg: None,
            milestone_last_deleted: None,
        }
    }

    #[test]
    fn goals_panes_use_exact_wide_boundary_geometry() {
        assert_eq!(
            goals_pane_areas(Rect::new(0, 0, 83, 20)),
            GoalsPaneAreas {
                goals: Rect::new(0, 0, 34, 20),
                milestones: Rect::new(35, 0, 48, 20),
            }
        );
        assert_eq!(
            goals_pane_areas(Rect::new(0, 0, 107, 20)),
            GoalsPaneAreas {
                goals: Rect::new(0, 0, 42, 20),
                milestones: Rect::new(43, 0, 64, 20),
            }
        );
    }

    #[test]
    fn goals_panes_stack_below_the_wide_threshold() {
        assert_eq!(
            goals_pane_areas(Rect::new(3, 4, 80, 21)),
            GoalsPaneAreas {
                goals: Rect::new(3, 4, 80, 8),
                milestones: Rect::new(3, 13, 80, 12),
            }
        );
    }

    #[test]
    fn goals_panes_remain_contained_and_separated_for_degenerate_rectangles() {
        fn contains(parent: Rect, child: Rect) -> bool {
            let parent_right = parent.x as u32 + parent.width as u32;
            let parent_bottom = parent.y as u32 + parent.height as u32;
            let child_right = child.x as u32 + child.width as u32;
            let child_bottom = child.y as u32 + child.height as u32;
            child.x >= parent.x
                && child.y >= parent.y
                && child_right <= parent_right
                && child_bottom <= parent_bottom
        }

        let bodies = [
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 40, 16),
            Rect::new(3, 4, 80, 21),
            Rect::new(u16::MAX - 40, u16::MAX - 30, 40, 16),
            Rect::new(u16::MAX - 10, u16::MAX - 10, 83, 20),
            Rect::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX),
        ];

        for body in bodies {
            let panes = goals_pane_areas(body);
            assert!(contains(body, panes.goals), "{body:?} -> {panes:?}");
            assert!(contains(body, panes.milestones), "{body:?} -> {panes:?}");

            if body.width >= WIDE_GOALS_PANES_MIN_WIDTH {
                let goals_right = panes.goals.x as u32 + panes.goals.width as u32;
                assert!(
                    goals_right <= panes.milestones.x as u32,
                    "horizontal panes overlap: {body:?} -> {panes:?}"
                );
                if body.x.checked_add(body.width).is_some() {
                    assert_eq!(
                        goals_right + 1,
                        panes.milestones.x as u32,
                        "horizontal panes should keep a one-cell gap: {body:?} -> {panes:?}"
                    );
                }
            } else if body.height >= 2 {
                let goals_bottom = panes.goals.y as u32 + panes.goals.height as u32;
                assert!(
                    goals_bottom <= panes.milestones.y as u32,
                    "vertical panes overlap: {body:?} -> {panes:?}"
                );
                if body.y.checked_add(body.height).is_some() {
                    assert_eq!(
                        goals_bottom + 1,
                        panes.milestones.y as u32,
                        "vertical panes should keep a one-cell gap: {body:?} -> {panes:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn goals_reentry_preserves_stable_browse_identity_and_clears_transients() {
        let db = Database::open_in_memory().unwrap();
        let first = db.create_goal("First").unwrap();
        let second = db.create_goal("Second").unwrap();
        let earlier = db.create_milestone(second, "Earlier").unwrap();
        let milestone = db.create_milestone(second, "Target").unwrap();
        db.create_milestone(second, "Later").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == second)
            .unwrap();
        screen.mode = Mode::Milestones;
        screen.milestone_goal_idx = screen.selected;
        screen.milestone_selected = screen.goals[screen.milestone_goal_idx]
            .milestones
            .iter()
            .position(|item| item.id == milestone)
            .unwrap();
        screen.milestone_mode = MilestoneMode::EditMilestone;
        screen.milestone_input.set("discard me");
        screen.status_msg = Some(("old".into(), false));
        screen.last_deleted = Some(GoalUndoData::Goal(screen.goals[0].clone()));
        screen.milestone_status_msg = Some(("old milestone".into(), false));
        screen.milestone_last_deleted = Some(GoalUndoData::Milestone {
            goal_id: second,
            milestone: screen.goals[screen.milestone_goal_idx].milestones[0].clone(),
        });
        screen.list_viewport = ViewportState::new(1);
        screen.list_viewport.set_offset(1);
        screen.milestone_panel_state.viewport = ViewportState::new(1);
        screen.milestone_panel_state.viewport.set_offset(1);

        db.create_goal("New").unwrap();
        db.delete_milestone(earlier).unwrap();
        db.update_goal(first, "First changed").unwrap();
        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.goals[next.milestone_goal_idx].id, second);
        assert_eq!(
            next.goals[next.milestone_goal_idx].milestones[next.milestone_selected].id,
            milestone
        );
        assert_eq!(next.mode, Mode::Milestones);
        assert_eq!(next.milestone_mode, MilestoneMode::Browse);
        assert!(next.milestone_panel_state.focused);
        assert!(next.goal_input.value().is_empty());
        assert!(next.milestone_input.value().is_empty());
        assert!(next.status_msg.is_none());
        assert!(next.milestone_status_msg.is_none());
        assert!(next.last_deleted.is_none());
        assert!(next.milestone_last_deleted.is_none());
        assert_eq!(next.list_viewport.offset(), 1);
        assert_eq!(next.milestone_panel_state.viewport.offset(), 1);
        assert_eq!(
            next.goals
                .iter()
                .find(|goal| goal.id == first)
                .unwrap()
                .title,
            "First changed"
        );
    }

    #[test]
    fn goals_reentry_preserves_wrapped_content_viewports_until_measured_render() {
        let db = Database::open_in_memory().unwrap();
        let goal_id = db.create_goal(&"Long goal ".repeat(30)).unwrap();
        db.create_milestone(goal_id, &"Long milestone ".repeat(30))
            .unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.mode = Mode::Milestones;
        render_screen(&mut screen, 24, 10);
        screen.list_viewport.set_offset(4);
        screen.milestone_panel_state.viewport.set_offset(3);

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.list_viewport.offset(), 4);
        assert_eq!(next.milestone_panel_state.viewport.offset(), 3);
    }

    #[test]
    fn goals_reentry_leaves_add_mode_and_restores_selected_goal_identity() {
        let db = Database::open_in_memory().unwrap();
        let first = db.create_goal("First").unwrap();
        let second = db.create_goal("Second").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == first)
            .unwrap();
        screen.mode = Mode::AddGoal;
        screen.goal_input.set("discard me");

        db.toggle_goal(second).unwrap();
        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.mode, Mode::Browse);
        assert_eq!(next.goals[next.selected].id, first);
        assert!(next.goal_input.value().is_empty());
        assert!(next.milestone_input.value().is_empty());
    }

    #[test]
    fn goals_reentry_clamps_indices_and_viewports_after_selected_goal_is_deleted() {
        let db = Database::open_in_memory().unwrap();
        let deleted = db.create_goal("Deleted").unwrap();
        db.create_goal("Remaining").unwrap();
        db.create_milestone(deleted, "One").unwrap();
        db.create_milestone(deleted, "Two").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == deleted)
            .unwrap();
        screen.mode = Mode::Milestones;
        screen.milestone_goal_idx = screen.selected;
        screen.milestone_selected = 1;
        screen.list_viewport = ViewportState::new(1);
        screen.list_viewport.set_offset(5);
        screen.milestone_panel_state.viewport = ViewportState::new(1);
        screen.milestone_panel_state.viewport.set_offset(2);

        db.delete_goal(deleted).unwrap();
        let mut next = screen.reenter(&db).unwrap();

        assert_eq!(next.selected, 0);
        assert_eq!(next.milestone_goal_idx, 0);
        assert_eq!(next.milestone_selected, 0);
        assert_eq!(next.mode, Mode::Milestones);
        assert_eq!(next.list_viewport.offset(), 5);
        assert_eq!(next.milestone_panel_state.viewport.offset(), 2);

        render_screen(&mut next, 80, 24);

        assert_eq!(next.list_viewport.offset(), 0);
        assert_eq!(next.milestone_panel_state.viewport.offset(), 0);
    }

    #[test]
    fn goals_navigation_error_uses_the_active_status_surface() {
        let mut screen = make_screen(1);

        screen.set_navigation_error("browse error".into());
        assert_eq!(screen.status_msg, Some(("browse error".into(), true)));
        assert!(screen.milestone_status_msg.is_none());

        screen.mode = Mode::Milestones;
        screen.set_navigation_error("milestone error".into());
        assert_eq!(
            screen.milestone_status_msg,
            Some(("milestone error".into(), true))
        );
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
    fn milestone_pane_stays_inside_small_terminal() {
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
        assert!(screen.milestone_panel_state.viewport.offset() > 0);
        assert!(output.contains("Milestone 20"), "{output}");

        for _ in 1..20 {
            screen.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), &db);
            render_screen(&mut screen, 80, 24);
        }

        assert_eq!(screen.milestone_panel_state.viewport.offset(), 0);
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

        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::AddMilestone);
    }

    #[test]
    fn goals_escape_layers_are_bound_before_panel_redesign() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();

        assert_eq!(
            screen.handle_key(key(KeyCode::Char('m')), &db),
            Action::None
        );
        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::AddMilestone);

        assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::Browse);

        assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
        assert_eq!(screen.mode, Mode::Browse);

        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), &db),
            Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard))
        );
    }

    #[test]
    fn milestone_focus_round_trip_preserves_same_goal_context() {
        let db = goals_db_with_milestones(20, 0);
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        screen.milestone_selected = 12;
        screen.milestone_panel_state.viewport.set_offset(9);
        screen.milestone_status_msg = Some(("keep milestone status".into(), false));
        let goal_id = screen.goals[screen.milestone_goal_idx].id;
        let milestone_id = screen.goals[screen.milestone_goal_idx].milestones[12].id;
        screen.milestone_last_deleted = Some(GoalUndoData::Milestone {
            goal_id,
            milestone: screen.goals[screen.milestone_goal_idx].milestones[0].clone(),
        });

        screen.handle_key(key(KeyCode::Esc), &db);
        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::Browse);
        assert_eq!(screen.goals[screen.milestone_goal_idx].id, goal_id);
        assert_eq!(
            screen.goals[screen.milestone_goal_idx].milestones[screen.milestone_selected].id,
            milestone_id
        );
        assert_eq!(screen.milestone_panel_state.viewport.offset(), 9);
        assert_eq!(
            screen.milestone_status_msg,
            Some(("keep milestone status".into(), false))
        );
        assert!(screen.milestone_last_deleted.is_some());
    }

    #[test]
    fn switching_goal_focus_clamps_milestone_selection_without_clearing_context() {
        let db = Database::open_in_memory().unwrap();
        let first_goal_id = db.create_goal("First goal").unwrap();
        for number in 1..=4 {
            db.create_milestone(first_goal_id, &format!("First milestone {number}"))
                .unwrap();
        }
        let second_goal_id = db.create_goal("Second goal").unwrap();
        db.create_milestone(second_goal_id, "Only milestone")
            .unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == first_goal_id)
            .unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        screen.milestone_selected = 3;
        screen.milestone_panel_state.viewport.set_offset(7);
        screen.milestone_status_msg = Some(("keep cross-goal status".into(), false));
        screen.milestone_last_deleted = Some(GoalUndoData::Milestone {
            goal_id: first_goal_id,
            milestone: screen.goals[screen.milestone_goal_idx].milestones[0].clone(),
        });

        screen.handle_key(key(KeyCode::Esc), &db);
        screen.selected = screen
            .goals
            .iter()
            .position(|goal| goal.id == second_goal_id)
            .unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.goals[screen.milestone_goal_idx].id, second_goal_id);
        assert_eq!(screen.milestone_selected, 0);
        assert_eq!(screen.milestone_panel_state.viewport.offset(), 7);
        assert_eq!(
            screen.milestone_status_msg,
            Some(("keep cross-goal status".into(), false))
        );
        assert!(screen.milestone_last_deleted.is_some());
    }

    #[test]
    fn add_goal_editor_is_revealed_from_a_scrolled_list() {
        let db = Database::open_in_memory().unwrap();
        for number in 1..=20 {
            db.create_goal(&format!("Goal {number:02}")).unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen.goals.len() - 1;
        render_screen(&mut screen, 80, 24);
        assert!(screen.list_viewport.offset() > 0);

        screen.handle_key(key(KeyCode::Char('a')), &db);
        screen.goal_input.set("VISIBLE-ADD-GOAL-EDITOR");
        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains("VISIBLE-ADD-GOAL-EDITOR"), "{output}");
        assert_eq!(screen.list_viewport.offset(), 0);
    }

    #[test]
    fn goal_delete_confirmation_is_revealed_at_the_bottom_of_a_long_list() {
        let db = Database::open_in_memory().unwrap();
        for number in 1..=20 {
            db.create_goal(&format!("Goal {number:02}")).unwrap();
        }
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.selected = screen.goals.len() - 1;
        render_screen(&mut screen, 80, 24);
        screen.handle_key(key(KeyCode::Char('d')), &db);

        let output = render_screen(&mut screen, 80, 24);

        assert!(output.contains(&tr("dashboard-delete-confirm")), "{output}");
    }

    #[test]
    fn failed_goal_delete_retains_confirmation_selection_and_viewport() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locked-goals.db");
        let db = Database::open(&path).unwrap();
        let goal_id = db.create_goal("Keep goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(3);
        screen.handle_key(key(KeyCode::Char('d')), &db);
        let lock = Connection::open(&path).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

        screen.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(screen.mode, Mode::ConfirmDeleteGoal);
        assert_eq!(screen.goals[screen.selected].id, goal_id);
        assert_eq!(screen.list_viewport.offset(), 3);
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));

        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(db.list_goals().unwrap().len(), 1);
    }

    #[test]
    fn failed_milestone_delete_retains_confirmation_identity_and_viewports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locked-milestones.db");
        let db = Database::open(&path).unwrap();
        let goal_id = db.create_goal("Keep goal").unwrap();
        let milestone_id = db.create_milestone(goal_id, "Keep milestone").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        screen.list_viewport.set_offset(2);
        screen.milestone_panel_state.viewport.set_offset(4);
        screen.handle_key(key(KeyCode::Char('d')), &db);
        let lock = Connection::open(&path).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

        screen.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::ConfirmDeleteMilestone);
        assert_eq!(screen.goals[screen.milestone_goal_idx].id, goal_id);
        assert_eq!(
            screen.goals[screen.milestone_goal_idx].milestones[screen.milestone_selected].id,
            milestone_id
        );
        assert_eq!(screen.list_viewport.offset(), 2);
        assert_eq!(screen.milestone_panel_state.viewport.offset(), 4);
        assert!(screen
            .milestone_status_msg
            .as_ref()
            .is_some_and(|(_, error)| *error));

        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(
            db.list_goals()
                .unwrap()
                .into_iter()
                .find(|goal| goal.id == goal_id)
                .unwrap()
                .milestones
                .len(),
            1
        );
    }

    #[test]
    fn adding_milestone_selects_new_row_and_keeps_panel_focused() {
        let db = Database::open_in_memory().unwrap();
        db.create_goal("Goal").unwrap();
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);
        screen.milestone_input.set("New milestone");

        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::Browse);
        assert_eq!(screen.milestone_selected, 0);
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
        screen.milestone_input.set("X".repeat(200));

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
        screen.milestone_input.set("not-a-date");

        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

        assert_eq!(screen.milestone_mode, MilestoneMode::EditMilestoneDate);
        assert_eq!(screen.milestone_input.value(), "not-a-date");
        assert!(screen
            .milestone_status_msg
            .as_ref()
            .is_some_and(|(_, error)| *error));
    }

    #[test]
    fn milestone_error_remains_visible_when_panel_fills_viewport() {
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
        screen.milestone_input.set("not-a-date");
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

        assert_eq!(screen.goals[screen.milestone_goal_idx].id, target_id);
        assert!(screen.goals[screen.milestone_goal_idx].completed);
        assert_eq!(screen.mode, Mode::Milestones);
    }
}
