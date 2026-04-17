use chrono::{Local, NaiveDate, NaiveTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::Database;
use crate::model::{LogEntry, Practice, PracticeType, SetData};
use super::{Action, Screen};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    SelectPractice,
    EnterSets,
    EnterNote,
}

pub struct LogEntryScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filtering: bool,
    selected: usize,
    phase: Phase,
    chosen_practice: Option<Practice>,
    sets: Vec<SetData>,
    field1: String,
    field2: String,
    active_field: usize,
    note: String,
    note_cursor: usize,    // byte offset into note string
    editing_log_id: Option<i64>,
    log_date: String,      // YYYY-MM-DD, defaults to today
    date_confirmed: bool,   // false = cursor on date line, true = entering sets
    editing_date: bool,     // true when in date-edit mode
    date_input: String,     // buffer for typing a new date
    return_to: Screen,     // screen to return to on Esc or save
}

impl LogEntryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        let today = Local::now().format("%Y-%m-%d").to_string();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::SelectPractice,
            chosen_practice: None,
            sets: Vec::new(),
            field1: String::new(),
            field2: String::new(),
            active_field: 0,
            note: String::new(),
            note_cursor: 0,
            editing_log_id: None,
            log_date: today,
            date_confirmed: false,
            editing_date: false,
            date_input: String::new(),
            return_to: Screen::Dashboard,
        })
    }

    pub fn from_existing(db: &Database, log_entry: &LogEntry) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        let practice = practices
            .iter()
            .find(|p| p.id == log_entry.log.practice_id)
            .cloned();
        let sets: Vec<SetData> = log_entry.sets.iter().map(|s| s.data.clone()).collect();
        let note = log_entry.log.note.clone().unwrap_or_default();

        let log_date = log_entry.log.logged_at.format("%Y-%m-%d").to_string();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::EnterSets,
            chosen_practice: practice,
            sets,
            field1: String::new(),
            field2: String::new(),
            active_field: 0,
            note_cursor: note.len(),
            note,
            editing_log_id: Some(log_entry.log.id),
            log_date,
            date_confirmed: true,
            editing_date: false,
            date_input: String::new(),
            return_to: Screen::History,
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        match self.phase {
            Phase::SelectPractice => self.render_select_practice(frame),
            Phase::EnterSets => self.render_enter_sets(frame),
            Phase::EnterNote => self.render_enter_note(frame),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.phase {
            Phase::SelectPractice => self.handle_select_practice(key),
            Phase::EnterSets => self.handle_enter_sets(key),
            Phase::EnterNote => self.handle_enter_note(key, db),
        }
    }

    // ── Phase 1: SelectPractice ───────────────────────────────────────

    fn render_select_practice(&self, frame: &mut Frame) {
        let area = frame.area();

        let list_height = (self.filtered_indices.len() as u16).max(1);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),           // title
                Constraint::Length(2),           // filter bar
                Constraint::Length(list_height), // list
                Constraint::Length(1),           // footer
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        // Title
        let title = Line::from(Span::styled(
            " Select Practice",
            Style::default().fg(ACCENT).bold(),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Filter bar
        let filter_display = if self.filtering {
            format!(" /{}█", self.filter_text)
        } else if !self.filter_text.is_empty() {
            format!(" /{}", self.filter_text)
        } else {
            String::from(" Press / to filter")
        };
        let filter_style = if self.filtering {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(Color::Gray)
        };
        let filter_line = Line::from(Span::styled(filter_display, filter_style));
        frame.render_widget(Paragraph::new(filter_line), chunks[1]);

        // Practice list
        let lines: Vec<Line> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let practice = &self.practices[idx];
                let prefix = if i == self.selected { "> " } else { "  " };
                let text = format!(
                    "{}{} ({})",
                    prefix, practice.name, practice.practice_type.label()
                );
                let style = if i == self.selected {
                    Style::default().fg(GREEN)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(text, style))
            })
            .collect();
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[2]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(ACCENT)),
            Span::styled(" Navigate  ", Style::default().fg(Color::Gray)),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(" Filter  ", Style::default().fg(Color::Gray)),
            Span::styled("[Enter]", Style::default().fg(ACCENT)),
            Span::styled(" Select  ", Style::default().fg(Color::Gray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Back", Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[3]);
    }

    fn handle_select_practice(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(self.return_to.clone()),
            KeyCode::Char('/') => {
                self.filtering = true;
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = (self.selected + 1) % self.filtered_indices.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.filtered_indices.is_empty() {
                    self.selected = if self.selected == 0 {
                        self.filtered_indices.len() - 1
                    } else {
                        self.selected - 1
                    };
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    self.chosen_practice = Some(self.practices[idx].clone());
                    self.phase = Phase::EnterSets;
                    self.sets.clear();
                    self.field1.clear();
                    self.field2.clear();
                    self.active_field = 0;
                    self.date_confirmed = false;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn apply_filter(&mut self) {
        let lower = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    // ── Phase 2: EnterSets ────────────────────────────────────────────

    fn render_enter_sets(&self, frame: &mut Frame) {
        let area = frame.area();
        let practice = self.chosen_practice.as_ref().unwrap();

        let sets_height = (self.sets.len() as u16 + 3).max(3); // sets + input fields
        let note_height: u16 = if self.note.is_empty() { 0 } else { 1 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),           // title
                Constraint::Length(1),           // date line
                Constraint::Length(sets_height), // committed sets + input
                Constraint::Length(1),           // running total
                Constraint::Length(note_height), // note (if any)
                Constraint::Length(1),           // footer
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled(
                format!(" {} ", practice.name),
                Style::default().fg(ACCENT).bold(),
            ),
            Span::styled(
                format!("({})", practice.practice_type.label()),
                Style::default().fg(Color::Gray),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Date line
        let date_line = if self.editing_date {
            Line::from(vec![
                Span::styled("  Date: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}\u{2588}", self.date_input),
                    Style::default().fg(ACCENT),
                ),
                Span::styled(
                    "  (YYYY-MM-DD, Enter to confirm)",
                    Style::default().fg(Color::Gray),
                ),
            ])
        } else if !self.date_confirmed {
            Line::from(vec![
                Span::styled("> Date: ", Style::default().fg(GREEN).bold()),
                Span::styled(&self.log_date, Style::default().fg(GREEN).bold()),
                Span::styled("  [Enter] confirm  [D] edit", Style::default().fg(Color::Gray)),
            ])
        } else {
            Line::from(vec![
                Span::styled("  Date: ", Style::default().fg(Color::Gray)),
                Span::styled(&self.log_date, Style::default().fg(Color::White)),
                Span::styled("  [D] to change", Style::default().fg(Color::Gray)),
            ])
        };
        frame.render_widget(Paragraph::new(date_line), chunks[1]);

        // Committed sets + current input
        let mut lines: Vec<Line> = Vec::new();

        // Show committed sets
        for (i, set) in self.sets.iter().enumerate() {
            let text = format!("  Set {}: {}", i + 1, format_set_data(set));
            lines.push(Line::from(Span::styled(text, Style::default().fg(GREEN))));
        }

        // Current input fields
        let set_num = self.sets.len() + 1;
        match practice.practice_type {
            PracticeType::Weighted => {
                let weight_cursor = if self.active_field == 0 { "\u{2588}" } else { "" };
                let reps_cursor = if self.active_field == 1 { "\u{2588}" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Set {}: ", set_num),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("Weight (kg): ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}{}", self.field1, weight_cursor),
                        Style::default().fg(if self.active_field == 0 { ACCENT } else { Color::White }),
                    ),
                    Span::styled("  Reps: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}{}", self.field2, reps_cursor),
                        Style::default().fg(if self.active_field == 1 { ACCENT } else { Color::White }),
                    ),
                ]));
            }
            PracticeType::Bodyweight => {
                let cursor = "\u{2588}";
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Set {}: ", set_num),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("Reps: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(ACCENT),
                    ),
                ]));
            }
            PracticeType::Distance => {
                let cursor = "\u{2588}";
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Set {}: ", set_num),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("Distance (km): ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(ACCENT),
                    ),
                ]));
            }
            PracticeType::Endurance => {
                let cursor = "\u{2588}";
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Set {}: ", set_num),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("Duration (min): ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(ACCENT),
                    ),
                ]));
            }
        }
        frame.render_widget(Paragraph::new(lines), chunks[2]);

        // Running total
        let total: f64 = self.sets.iter().map(|s| s.metric_value()).sum();
        let label = self
            .sets
            .first()
            .map(|s| s.metric_label())
            .unwrap_or(metric_label_for_type(&practice.practice_type));
        let total_reps: i32 = self.sets.iter().map(|s| match s {
            SetData::Weighted { reps, .. } => *reps,
            _ => 0,
        }).sum();
        let total_text = if total_reps > 0 {
            format!("  Sets: {}  Total: {:.1} {}  Reps: {}", self.sets.len(), total, label, total_reps)
        } else {
            format!("  Sets: {}  Total: {:.1} {}", self.sets.len(), total, label)
        };
        let total_line = Line::from(Span::styled(
            total_text,
            Style::default().fg(Color::White),
        ));
        frame.render_widget(Paragraph::new(total_line), chunks[3]);

        // Note (if present)
        if !self.note.is_empty() {
            let note_line = Line::from(vec![
                Span::styled("  Note: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&self.note, Style::default().fg(Color::Yellow)),
            ]);
            frame.render_widget(Paragraph::new(note_line), chunks[4]);
        }

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(ACCENT)),
            Span::styled(" Add set  ", Style::default().fg(Color::Gray)),
            Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
            Span::styled(" Save  ", Style::default().fg(Color::Gray)),
            Span::styled("[D]", Style::default().fg(ACCENT)),
            Span::styled(" Date  ", Style::default().fg(Color::Gray)),
            Span::styled("[d]", Style::default().fg(ACCENT)),
            Span::styled(" Del last  ", Style::default().fg(Color::Gray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Cancel", Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[5]);
    }

    fn handle_enter_sets(&mut self, key: KeyEvent) -> Action {
        // Date editing sub-mode
        if self.editing_date {
            return self.handle_date_edit(key);
        }

        // Date confirmation step — cursor is on the date line
        if !self.date_confirmed {
            return match key.code {
                KeyCode::Enter => {
                    self.date_confirmed = true;
                    Action::None
                }
                KeyCode::Char('D') | KeyCode::Char('d') => {
                    self.editing_date = true;
                    self.date_input = self.log_date.clone();
                    Action::None
                }
                KeyCode::Esc => Action::Navigate(self.return_to.clone()),
                _ => Action::None,
            };
        }

        let practice = self.chosen_practice.as_ref().unwrap().clone();
        let is_weighted = practice.practice_type == PracticeType::Weighted;
        let has_two_fields = is_weighted;

        // Ctrl+S to save (move to note phase)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            if !self.sets.is_empty() {
                self.phase = Phase::EnterNote;
                self.note_cursor = self.note.len();
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(self.return_to.clone()),
            KeyCode::Char('D') => {
                self.editing_date = true;
                self.date_input = self.log_date.clone();
                Action::None
            }
            KeyCode::Tab => {
                if has_two_fields {
                    self.active_field = if self.active_field == 0 { 1 } else { 0 };
                }
                Action::None
            }
            KeyCode::Enter => {
                if has_two_fields && self.active_field == 0 {
                    self.active_field = 1;
                } else {
                    self.commit_set();
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.active_field == 0 {
                    self.field1.pop();
                } else {
                    self.field2.pop();
                }
                Action::None
            }
            KeyCode::Char('d') => {
                let fields_empty = self.field1.is_empty()
                    && (self.field2.is_empty() || !has_two_fields);
                if fields_empty && !self.sets.is_empty() {
                    self.sets.pop();
                }
                Action::None
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                if self.active_field == 0 {
                    self.field1.push(c);
                } else {
                    self.field2.push(c);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_date_edit(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                // Validate the date
                if NaiveDate::parse_from_str(&self.date_input, "%Y-%m-%d").is_ok() {
                    self.log_date = self.date_input.clone();
                    self.date_confirmed = true;
                }
                self.editing_date = false;
            }
            KeyCode::Esc => {
                self.editing_date = false;
            }
            KeyCode::Backspace => {
                self.date_input.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                self.date_input.push(c);
            }
            _ => {}
        }
        Action::None
    }

    fn commit_set(&mut self) {
        let practice = self.chosen_practice.as_ref().unwrap();
        let set_data = match practice.practice_type {
            PracticeType::Weighted => {
                let weight = if self.field1.is_empty() {
                    // Carry forward from last set
                    self.sets
                        .last()
                        .and_then(|s| match s {
                            SetData::Weighted { weight, .. } => Some(*weight),
                            _ => None,
                        })
                        .unwrap_or(0.0)
                } else {
                    self.field1.parse::<f64>().unwrap_or(0.0)
                };
                let reps = self.field2.parse::<i32>().unwrap_or(0);
                if reps == 0 {
                    return;
                }
                Some(SetData::Weighted { weight, reps })
            }
            PracticeType::Bodyweight => {
                let reps = self.field1.parse::<i32>().unwrap_or(0);
                if reps == 0 {
                    return;
                }
                Some(SetData::Bodyweight { reps })
            }
            PracticeType::Distance => {
                let distance = self.field1.parse::<f64>().unwrap_or(0.0);
                if distance == 0.0 {
                    return;
                }
                Some(SetData::Distance { distance })
            }
            PracticeType::Endurance => {
                let duration = self.field1.parse::<f64>().unwrap_or(0.0);
                if duration == 0.0 {
                    return;
                }
                Some(SetData::Endurance { duration })
            }
        };

        if let Some(data) = set_data {
            self.sets.push(data);

            // For weighted: keep field1 (weight carries forward), clear field2 (reps)
            // For others: clear field1
            if practice.practice_type == PracticeType::Weighted {
                // Keep field1 (weight), clear field2 (reps), set active to field2
                self.field2.clear();
                self.active_field = 1;
            } else {
                self.field1.clear();
                self.active_field = 0;
            }
        }
    }

    // ── Phase 3: EnterNote ────────────────────────────────────────────

    fn render_enter_note(&self, frame: &mut Frame) {
        let area = frame.area();
        let practice = self.chosen_practice.as_ref().unwrap();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // spacer
                Constraint::Length(3), // summary
                Constraint::Length(1), // spacer
                Constraint::Length(3), // note input
                Constraint::Length(1), // footer
                Constraint::Min(0),   // spacer absorbs excess
            ])
            .split(area);

        // Title
        let title = Line::from(Span::styled(
            format!(" Log {} \u{2014} Add Note", practice.name),
            Style::default().fg(ACCENT).bold(),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Summary
        let total: f64 = self.sets.iter().map(|s| s.metric_value()).sum();
        let label = self
            .sets
            .first()
            .map(|s| s.metric_label())
            .unwrap_or("units");
        let summary_lines = vec![
            Line::from(vec![
                Span::styled("  Date: ", Style::default().fg(Color::Gray)),
                Span::styled(&self.log_date, Style::default().fg(Color::White)),
            ]),
            Line::from(Span::styled(
                format!("  {} sets logged", self.sets.len()),
                Style::default().fg(GREEN),
            )),
            Line::from(Span::styled(
                format!("  Total: {:.1} {}", total, label),
                Style::default().fg(Color::White),
            )),
        ];
        frame.render_widget(Paragraph::new(summary_lines), chunks[2]);

        // Note input
        let note_block = Block::default()
            .title(Span::styled(
                "Note (optional)",
                Style::default().fg(Color::Gray),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let (before, after) = self.note.split_at(self.note_cursor);
        let note_line = Line::from(vec![
            Span::styled(before.to_string(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(ACCENT)),
            Span::styled(after.to_string(), Style::default().fg(Color::White)),
        ]);
        let note_paragraph = Paragraph::new(note_line)
        .block(note_block);
        frame.render_widget(note_paragraph, chunks[4]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(ACCENT)),
            Span::styled(" Save  ", Style::default().fg(Color::Gray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Cancel", Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[5]);
    }

    fn handle_enter_note(&mut self, key: KeyEvent, db: &Database) -> Action {
        // Ctrl+B / Left: move cursor back
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b')
            || key.code == KeyCode::Left
        {
            if self.note_cursor > 0 {
                // Move back one char (find previous char boundary)
                self.note_cursor = self.note[..self.note_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            return Action::None;
        }
        // Ctrl+F / Right: move cursor forward
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if self.note_cursor < self.note.len() {
                // Move forward one char
                self.note_cursor = self.note[self.note_cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| self.note_cursor + i)
                    .unwrap_or(self.note.len());
            }
            return Action::None;
        }
        // Ctrl+A / Home: move to start
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a')
            || key.code == KeyCode::Home
        {
            self.note_cursor = 0;
            return Action::None;
        }
        // Ctrl+E / End: move to end
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e')
            || key.code == KeyCode::End
        {
            self.note_cursor = self.note.len();
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(self.return_to.clone()),
            KeyCode::Enter => {
                let practice = self.chosen_practice.as_ref().unwrap();
                let note = if self.note.is_empty() {
                    None
                } else {
                    Some(self.note.as_str())
                };
                let date = NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d")
                    .unwrap_or_else(|_| Local::now().date_naive());
                let datetime = date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
                if let Some(log_id) = self.editing_log_id {
                    let _ = db.update_log(log_id, &self.sets, note, Some(&datetime));
                } else {
                    let _ = db.create_log_at(practice.id, &datetime, &self.sets, note);
                }
                Action::Navigate(self.return_to.clone())
            }
            KeyCode::Backspace => {
                if self.note_cursor > 0 {
                    let prev = self.note[..self.note_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.note.remove(prev);
                    self.note_cursor = prev;
                }
                Action::None
            }
            KeyCode::Char(c) => {
                self.note.insert(self.note_cursor, c);
                self.note_cursor += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }
}

fn format_set_data(set: &SetData) -> String {
    match set {
        SetData::Weighted { weight, reps } => format!("{:.1} kg x {} reps", weight, reps),
        SetData::Bodyweight { reps } => format!("{} reps", reps),
        SetData::Distance { distance } => format!("{:.2} km", distance),
        SetData::Endurance { duration } => format!("{:.1} min", duration),
    }
}

fn metric_label_for_type(pt: &PracticeType) -> &'static str {
    match pt {
        PracticeType::Weighted => "kg vol",
        PracticeType::Bodyweight => "reps",
        PracticeType::Distance => "km",
        PracticeType::Endurance => "min",
    }
}
