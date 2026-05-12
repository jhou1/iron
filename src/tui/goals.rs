use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
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
    AddMilestone,
    EditItem,
    EditDate,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GoalItem {
    Goal(i64),
    Milestone(i64),
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
        })
    }

    fn goal_items(&self) -> Vec<GoalItem> {
        let mut items = Vec::new();
        for goal in &self.goals {
            items.push(GoalItem::Goal(goal.id));
            for ms in &goal.milestones {
                items.push(GoalItem::Milestone(ms.id));
            }
        }
        items
    }

    fn selected_goal_item(&self) -> Option<GoalItem> {
        let items = self.goal_items();
        items.get(self.selected).copied()
    }

    fn parent_goal_id(&self) -> Option<i64> {
        match self.selected_goal_item()? {
            GoalItem::Goal(id) => Some(id),
            GoalItem::Milestone(ms_id) => {
                self.goals.iter()
                    .find(|g| g.milestones.iter().any(|m| m.id == ms_id))
                    .map(|g| g.id)
            }
        }
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        self.goals = db.list_goals()?;
        Ok(())
    }

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

    fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        let extra = match self.mode {
            Mode::ConfirmDelete | Mode::EditDate | Mode::AddMilestone => 1,
            _ => 0,
        };

        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected + extra >= self.scroll + visible_height {
            self.scroll = (self.selected + extra) - visible_height + 1;
        }
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
        let visible_height = list_area.height as usize;

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

        let mut idx = 0;
        for goal in &self.goals {
            let is_selected = idx == self.selected;

            if is_selected {
                sel_line_idx = Some(lines.len());
            }

            if is_selected && self.mode == Mode::EditItem {
                lines.push(Line::from(vec![
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
            } else if is_selected && self.mode == Mode::EditDate {
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(&goal.title, Style::default().fg(GREEN)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-date-prompt")), Style::default().fg(ACCENT)),
                    Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                    Span::styled("\u{2588}", Style::default().fg(GREEN)),
                    Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                ]));
            } else if goal.completed {
                let date_str = goal.completed_at
                    .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
                    .unwrap_or_default();
                let marker = if is_selected { "» " } else { "  " };
                let style = if is_selected { Style::default().fg(GREEN) } else { Style::default().fg(Color::Gray) };
                lines.push(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled("✓ ", Style::default().fg(GREEN)),
                    Span::styled(format!("{}{}", goal.title, date_str), style),
                ]));
                lines.push(render_gauge_line(goal, Color::DarkGray));
            } else {
                let marker = if is_selected { "» " } else { "  " };
                let style = if is_selected {
                    Style::default().fg(GREEN).bold()
                } else {
                    Style::default().fg(GREEN)
                };
                lines.push(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled(&goal.title, style),
                ]));
                lines.push(render_gauge_line(goal, GREEN));
            }

            if is_selected && self.mode == Mode::ConfirmDelete {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                ]));
            }
            idx += 1;

            for ms in &goal.milestones {
                let is_ms_selected = idx == self.selected;

                if is_ms_selected {
                    sel_line_idx = Some(lines.len());
                }
                if is_ms_selected && self.mode == Mode::EditItem {
                    let check = if ms.completed { "✓ " } else { "⏳ " };
                    let check_color = if ms.completed { GREEN } else { Color::Yellow };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", check), Style::default().fg(check_color)),
                        Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                        Span::styled("\u{2588}", Style::default().fg(GREEN)),
                        Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                    ]));
                } else if is_ms_selected && self.mode == Mode::EditDate {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("✓ ", Style::default().fg(GREEN)),
                        Span::styled(&ms.title, Style::default().fg(GREEN)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {} ", tr("dashboard-date-prompt")), Style::default().fg(ACCENT)),
                        Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                        Span::styled("\u{2588}", Style::default().fg(GREEN)),
                        Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                    ]));
                } else if ms.completed {
                    let date_str = ms.completed_at
                        .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
                        .unwrap_or_default();
                    let marker = if is_ms_selected { "» " } else { "  " };
                    let style = if is_ms_selected { Style::default().fg(GREEN) } else { Style::default().fg(Color::Gray) };
                    lines.push(Line::from(vec![
                        Span::styled(marker, style),
                        Span::raw("  "),
                        Span::styled("✓ ", Style::default().fg(GREEN)),
                        Span::styled(format!("{}{}", ms.title, date_str), style),
                    ]));
                } else {
                    let marker = if is_ms_selected { "» " } else { "  " };
                    let style = if is_ms_selected {
                        Style::default().fg(GREEN).bold()
                    } else {
                        Style::default().fg(GREEN)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(marker, style),
                        Span::styled("  ⏳ ", Style::default().fg(Color::Yellow)),
                        Span::styled(&ms.title, style),
                    ]));
                }
                if is_ms_selected && self.mode == Mode::ConfirmDelete {
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {} ", tr("dashboard-delete-confirm")), Style::default().fg(Color::Red)),
                        Span::styled("[y]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                        Span::styled("[any]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                    ]));
                }
                idx += 1;
            }

            // Show milestone input right after the selected goal's milestones
            if self.mode == Mode::AddMilestone {
                if let Some(parent_id) = self.parent_goal_id() {
                    if parent_id == goal.id {
                        lines.push(Line::from(vec![
                            Span::styled("  ⏳ ", Style::default().fg(Color::Yellow)),
                            Span::styled(&self.input[..self.cursor], Style::default().fg(GREEN)),
                            Span::styled("\u{2588}", Style::default().fg(GREEN)),
                            Span::styled(&self.input[self.cursor..], Style::default().fg(GREEN)),
                        ]));
                    }
                }
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
            let sel_w = lines[idx].width() as u16;
            let sel_rows = if w > 0 && sel_w > 0 { sel_w.div_ceil(w) } else { 1 };
            (visual_row, sel_rows)
        });

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll as u16, 0)),
            list_area,
        );

        if let Some((visual_row, sel_rows)) = sel_visual {
            let scroll = self.scroll as u16;
            for r in 0..sel_rows {
                let abs_row = visual_row + r;
                if abs_row >= scroll && abs_row < scroll + list_area.height {
                    highlight_row(frame, list_area, abs_row - scroll);
                }
            }
        }

        // ── Status line ──
        render_status_line(frame, chunks[2], &self.status_msg);

        // ── Footer ──
        let footer_spans = if self.mode == Mode::Browse {
            vec![
                Span::styled(" [a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add-goal")), Style::default().fg(Color::Gray)),
                Span::styled("[m]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-milestone")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[Space]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-toggle")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[D]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-date")), Style::default().fg(Color::Gray)),
                Span::styled("[?]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::Gray)),
            ]
        } else {
            vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ]
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[3]);

        // Ignore chunks[4] (spacer)
        let _ = visible_height;

        // ── Help overlay ──
        if self.show_help {
            let bindings = &[
                ("j/k", "Navigate"),
                ("a", "Add goal"),
                ("m", "Add milestone"),
                ("e", "Edit"),
                ("Space", "Toggle complete"),
                ("D", "Edit date"),
                ("d", "Delete"),
                ("u", "Undo"),
                ("?", "Help"),
                ("Esc", "Back"),
            ];
            render_help_overlay(frame, area, bindings);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;

        if self.show_help {
            self.show_help = false;
            return Action::None;
        }

        match self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::AddGoal => self.handle_add_goal(key, db),
            Mode::AddMilestone => self.handle_add_milestone(key, db),
            Mode::EditItem => self.handle_edit_item(key, db),
            Mode::EditDate => self.handle_edit_date(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        let items = self.goal_items();
        let item_count = items.len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if item_count > 0 && self.selected < item_count - 1 {
                    self.selected += 1;
                    self.adjust_scroll(20); // approximate; will be corrected on render
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.adjust_scroll(20);
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.cursor = 0;
                self.mode = Mode::AddGoal;
                Action::None
            }
            KeyCode::Char('m') => {
                if self.parent_goal_id().is_some() {
                    self.input.clear();
                    self.cursor = 0;
                    self.mode = Mode::AddMilestone;
                    self.adjust_scroll(20);
                }
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(item) = self.selected_goal_item() {
                    let current_title = match item {
                        GoalItem::Goal(id) => {
                            self.goals.iter().find(|g| g.id == id).map(|g| g.title.clone())
                        }
                        GoalItem::Milestone(id) => {
                            self.goals.iter()
                                .flat_map(|g| &g.milestones)
                                .find(|m| m.id == id)
                                .map(|m| m.title.clone())
                        }
                    };
                    if let Some(title) = current_title {
                        self.input = title;
                        self.cursor = self.input.len();
                        self.mode = Mode::EditItem;
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                match self.selected_goal_item() {
                    Some(GoalItem::Goal(id)) => {
                        if let Err(e) = db.toggle_goal(id) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
                    }
                    Some(GoalItem::Milestone(id)) => {
                        if let Err(e) = db.toggle_milestone(id) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                        let _ = self.reload_goals(db);
                    }
                    None => {}
                }
                Action::None
            }
            KeyCode::Char('D') => {
                if let Some(item) = self.selected_goal_item() {
                    let is_completed = match item {
                        GoalItem::Goal(id) => self.goals.iter().find(|g| g.id == id).map(|g| g.completed).unwrap_or(false),
                        GoalItem::Milestone(id) => self.goals.iter().flat_map(|g| &g.milestones).find(|m| m.id == id).map(|m| m.completed).unwrap_or(false),
                    };
                    if is_completed {
                        self.input.clear();
                        self.cursor = 0;
                        self.mode = Mode::EditDate;
                        self.adjust_scroll(20);
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if self.selected_goal_item().is_some() {
                    self.mode = Mode::ConfirmDelete;
                    self.adjust_scroll(20);
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

    fn handle_add_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.parent_goal_id() {
                        if let Err(e) = db.create_milestone(goal_id, &title) {
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

    fn handle_edit_item(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.input.trim().to_string();
                if !title.is_empty() {
                    if let Some(item) = self.selected_goal_item() {
                        let result = match item {
                            GoalItem::Goal(id) => db.update_goal(id, &title),
                            GoalItem::Milestone(id) => db.update_milestone(id, &title),
                        };
                        if let Err(e) = result {
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

    fn handle_edit_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.input, "%Y-%m-%d") {
                    let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                    if let Some(item) = self.selected_goal_item() {
                        let result = match item {
                            GoalItem::Goal(id) => db.set_goal_completed_at(id, &completed_at),
                            GoalItem::Milestone(id) => db.set_milestone_completed_at(id, &completed_at),
                        };
                        if let Err(e) = result {
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

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(item) = self.selected_goal_item() {
                    match item {
                        GoalItem::Goal(id) => {
                            if let Some(goal) = self.goals.iter().find(|g| g.id == id).cloned() {
                                match db.delete_goal(id) {
                                    Ok(()) => {
                                        self.last_deleted = Some(GoalUndoData::Goal(goal));
                                        self.status_msg = Some((tr("status-deleted-undo"), false));
                                    }
                                    Err(e) => {
                                        self.status_msg = Some((format!("Delete failed: {}", e), true));
                                    }
                                }
                            }
                        }
                        GoalItem::Milestone(id) => {
                            if let Some((goal_id, milestone)) = self.find_milestone(id) {
                                match db.delete_milestone(id) {
                                    Ok(()) => {
                                        self.last_deleted = Some(GoalUndoData::Milestone { goal_id, milestone });
                                        self.status_msg = Some((tr("status-deleted-undo"), false));
                                    }
                                    Err(e) => {
                                        self.status_msg = Some((format!("Delete failed: {}", e), true));
                                    }
                                }
                            }
                        }
                    }
                    let _ = self.reload_goals(db);
                    let items = self.goal_items();
                    if self.selected >= items.len() && !items.is_empty() {
                        self.selected = items.len() - 1;
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
