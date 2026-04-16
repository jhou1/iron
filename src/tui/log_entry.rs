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

const PURPLE: Color = Color::Rgb(124, 124, 245);
const GREEN: Color = Color::Rgb(78, 202, 78);

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
    editing_log_id: Option<i64>,
}

impl LogEntryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
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
            editing_log_id: None,
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
            note,
            editing_log_id: Some(log_entry.log.id),
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(2), // filter bar
                Constraint::Min(1),   // list
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Title
        let title = Line::from(Span::styled(
            " Select Practice",
            Style::default().fg(PURPLE).bold(),
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
            Style::default().fg(PURPLE)
        } else {
            Style::default().fg(Color::DarkGray)
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
            Span::styled(" [j/k]", Style::default().fg(PURPLE)),
            Span::styled(" Navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(PURPLE)),
            Span::styled(" Filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(PURPLE)),
            Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(PURPLE)),
            Span::styled(" Back", Style::default().fg(Color::DarkGray)),
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
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // spacer
                Constraint::Min(1),   // committed sets + input
                Constraint::Length(1), // running total
                Constraint::Length(2), // footer
            ])
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled(
                format!(" {} ", practice.name),
                Style::default().fg(PURPLE).bold(),
            ),
            Span::styled(
                format!("({})", practice.practice_type.label()),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

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
                    Span::styled("Weight (kg): ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{}", self.field1, weight_cursor),
                        Style::default().fg(if self.active_field == 0 { PURPLE } else { Color::White }),
                    ),
                    Span::styled("  Reps: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{}", self.field2, reps_cursor),
                        Style::default().fg(if self.active_field == 1 { PURPLE } else { Color::White }),
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
                    Span::styled("Reps: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(PURPLE),
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
                    Span::styled("Distance (km): ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(PURPLE),
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
                    Span::styled("Duration (min): ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{}", self.field1, cursor),
                        Style::default().fg(PURPLE),
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
        let total_line = Line::from(Span::styled(
            format!("  Sets: {}  Total: {:.1} {}", self.sets.len(), total, label),
            Style::default().fg(Color::White),
        ));
        frame.render_widget(Paragraph::new(total_line), chunks[3]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(PURPLE)),
            Span::styled(" Add set  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Ctrl+S]", Style::default().fg(PURPLE)),
            Span::styled(" Save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[d]", Style::default().fg(PURPLE)),
            Span::styled(" Delete last  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(PURPLE)),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[4]);
    }

    fn handle_enter_sets(&mut self, key: KeyEvent) -> Action {
        let practice = self.chosen_practice.as_ref().unwrap().clone();
        let is_weighted = practice.practice_type == PracticeType::Weighted;
        let has_two_fields = is_weighted;

        // Ctrl+S to save (move to note phase)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            if !self.sets.is_empty() {
                self.phase = Phase::EnterNote;
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Tab => {
                if has_two_fields {
                    self.active_field = if self.active_field == 0 { 1 } else { 0 };
                }
                Action::None
            }
            KeyCode::Enter => {
                if has_two_fields && self.active_field == 0 {
                    // Switch to the second field
                    self.active_field = 1;
                } else {
                    // Commit the set
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
                // Only delete last set when all input fields are empty
                let fields_empty = self.field1.is_empty()
                    && (self.field2.is_empty() || !has_two_fields);
                if fields_empty && !self.sets.is_empty() {
                    self.sets.pop();
                } else if self.active_field == 0 {
                    // Otherwise treat 'd' as digit input? No, 'd' is not a digit.
                    // 'd' is not a digit or '.', so ignore it as input.
                } else {
                    // active_field == 1, also not a valid input char
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
                Constraint::Min(0),   // spacer
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Title
        let title = Line::from(Span::styled(
            format!(" Log {} \u{2014} Add Note", practice.name),
            Style::default().fg(PURPLE).bold(),
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
                Style::default().fg(Color::DarkGray),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let note_text = format!("{}\u{2588}", self.note);
        let note_paragraph = Paragraph::new(Line::from(Span::styled(
            note_text,
            Style::default().fg(Color::White),
        )))
        .block(note_block);
        frame.render_widget(note_paragraph, chunks[4]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(PURPLE)),
            Span::styled(" Save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(PURPLE)),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[6]);
    }

    fn handle_enter_note(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Enter => {
                let practice = self.chosen_practice.as_ref().unwrap();
                let note = if self.note.is_empty() {
                    None
                } else {
                    Some(self.note.as_str())
                };
                if let Some(log_id) = self.editing_log_id {
                    let _ = db.update_log(log_id, &self.sets, note);
                } else {
                    let _ = db.create_log(practice.id, &self.sets, note);
                }
                Action::Navigate(Screen::Dashboard)
            }
            KeyCode::Backspace => {
                self.note.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.note.push(c);
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
