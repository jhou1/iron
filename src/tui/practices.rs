use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::model::{Practice, PracticeType};
use super::{Action, Screen};

const PURPLE: Color = Color::Rgb(124, 124, 245);
const GREEN: Color = Color::Rgb(78, 202, 78);
const RED: Color = Color::Rgb(232, 84, 84);

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    AddName,
    AddType,
    EditName,
    ConfirmDelete,
}

pub struct PracticesScreen {
    practices: Vec<Practice>,
    selected: usize,
    mode: Mode,
    input: String,
    type_selected: usize,
}

impl PracticesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        Ok(Self {
            practices,
            selected: 0,
            mode: Mode::Browse,
            input: String::new(),
            type_selected: 0,
        })
    }

    fn refresh(&mut self, db: &Database) {
        if let Ok(practices) = db.list_practices() {
            self.practices = practices;
            if self.selected >= self.practices.len() && !self.practices.is_empty() {
                self.selected = self.practices.len() - 1;
            }
            if self.practices.is_empty() {
                self.selected = 0;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // title
                Constraint::Min(4),   // practice list
                Constraint::Length(6), // input/action area
                Constraint::Length(2), // shortcuts
            ])
            .split(area);

        // ── Title ──
        let title = Line::from(vec![
            Span::styled(" Practices", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(vec![title, Line::from("")]), chunks[0]);

        // ── Practice list ──
        let list_lines: Vec<Line> = if self.practices.is_empty() {
            vec![Line::from(Span::styled(
                "  No practices yet. Press 'a' to add one.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            self.practices
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let marker = if i == self.selected { "> " } else { "  " };
                    let name_style = if i == self.selected {
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(vec![
                        Span::styled(marker, name_style),
                        Span::styled(&p.name, name_style),
                        Span::styled(
                            format!(" ({})", p.practice_type.label()),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])
                })
                .collect()
        };
        frame.render_widget(Paragraph::new(list_lines), chunks[1]);

        // ── Input/action area ──
        let action_lines = match &self.mode {
            Mode::Browse => {
                vec![
                    Line::from(""),
                    Line::from(""),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddName => {
                vec![
                    Line::from(Span::styled(
                        " New practice name:",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!(" > {}_", self.input),
                        Style::default().fg(GREEN),
                    )),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddType => {
                let mut lines: Vec<Line> = vec![Line::from(Span::styled(
                    " Select type:",
                    Style::default().fg(Color::White),
                ))];
                for (i, pt) in PracticeType::ALL.iter().enumerate() {
                    let marker = if i == self.type_selected { "> " } else { "  " };
                    let style = if i == self.type_selected {
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    lines.push(Line::from(Span::styled(
                        format!(" {}{}", marker, pt.label()),
                        style,
                    )));
                }
                // 1 header + 4 types = 5 lines, fits in the 6-line action area
                lines
            }
            Mode::EditName => {
                vec![
                    Line::from(Span::styled(
                        " Rename practice:",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!(" > {}_", self.input),
                        Style::default().fg(GREEN),
                    )),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::ConfirmDelete => {
                let name = self
                    .practices
                    .get(self.selected)
                    .map(|p| p.name.as_str())
                    .unwrap_or("?");
                vec![
                    Line::from(Span::styled(
                        format!(" Delete {}?", name),
                        Style::default().fg(RED),
                    )),
                    Line::from(Span::styled(
                        " This removes all its logs.",
                        Style::default().fg(RED),
                    )),
                    Line::from(""),
                    Line::from(""),
                ]
            }
        };
        frame.render_widget(Paragraph::new(action_lines), chunks[2]);

        // ── Shortcuts ──
        let shortcuts = match &self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(PURPLE)),
                Span::styled(" Navigate  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[a]", Style::default().fg(PURPLE)),
                Span::styled(" Add  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter]", Style::default().fg(PURPLE)),
                Span::styled(" Edit  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[d]", Style::default().fg(PURPLE)),
                Span::styled(" Delete  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(PURPLE)),
                Span::styled(" Back", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::AddName | Mode::EditName => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(PURPLE)),
                Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(PURPLE)),
                Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::AddType => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(PURPLE)),
                Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter]", Style::default().fg(PURPLE)),
                Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Esc]", Style::default().fg(PURPLE)),
                Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
            ]),
            Mode::ConfirmDelete => Line::from(vec![
                Span::styled(" [y]", Style::default().fg(PURPLE)),
                Span::styled(" Yes  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[n]", Style::default().fg(PURPLE)),
                Span::styled(" No", Style::default().fg(Color::DarkGray)),
            ]),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match &self.mode {
            Mode::Browse => self.handle_browse(key),
            Mode::AddName => self.handle_add_name(key),
            Mode::AddType => self.handle_add_type(key, db),
            Mode::EditName => self.handle_edit_name(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.practices.is_empty() {
                    self.selected = (self.selected + 1) % self.practices.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.practices.is_empty() {
                    self.selected = self
                        .selected
                        .checked_sub(1)
                        .unwrap_or(self.practices.len() - 1);
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.type_selected = 0;
                self.mode = Mode::AddName;
                Action::None
            }
            KeyCode::Enter => {
                if let Some(p) = self.practices.get(self.selected) {
                    self.input = p.name.clone();
                    self.mode = Mode::EditName;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.practices.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            _ => Action::None,
        }
    }

    fn handle_add_name(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    self.mode = Mode::AddType;
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Backspace => {
                self.input.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_add_type(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.type_selected = (self.type_selected + 1) % PracticeType::ALL.len();
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.type_selected = self
                    .type_selected
                    .checked_sub(1)
                    .unwrap_or(PracticeType::ALL.len() - 1);
                Action::None
            }
            KeyCode::Enter => {
                let pt = PracticeType::ALL[self.type_selected];
                let _ = db.create_practice(self.input.trim(), pt);
                self.refresh(db);
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_edit_name(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(p) = self.practices.get(self.selected) {
                    let trimmed = self.input.trim();
                    if !trimmed.is_empty() {
                        let _ = db.rename_practice(p.id, trimmed);
                        self.refresh(db);
                    }
                }
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Backspace => {
                self.input.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(p) = self.practices.get(self.selected) {
                    let _ = db.delete_practice(p.id);
                    self.refresh(db);
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
