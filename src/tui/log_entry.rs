use chrono::{Local, NaiveDate, NaiveTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, Practice, PracticeType, SetData};
use super::{centered_area, highlight_row, render_status_line, visible_input_spans, Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    SelectPractice,
    EnterLog,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusSection {
    Sets,
    WarmUp,
    CoolDown,
    Note,
}

pub struct LogEntryScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filter_cursor: usize,
    filtering: bool,
    selected: usize,
    phase: Phase,
    chosen_practice: Option<Practice>,
    sets: Vec<SetData>,
    field1: String,
    field1_cursor: usize,
    field2: String,
    field2_cursor: usize,
    active_field: usize,
    note: String,
    note_cursor: usize,
    warm_up: String,
    warm_up_cursor: usize,
    cool_down: String,
    cool_down_cursor: usize,
    focus: FocusSection,
    editing_log_id: Option<i64>,
    log_date: String,
    editing_date: bool,
    date_input: String,
    date_input_cursor: usize,
    return_to: Screen,
    status_msg: StatusMessage,
}

impl LogEntryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        let today = Local::now().format("%Y-%m-%d").to_string();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            phase: Phase::SelectPractice,
            chosen_practice: None,
            sets: Vec::new(),
            field1: String::new(),
            field1_cursor: 0,
            field2: String::new(),
            field2_cursor: 0,
            active_field: 0,
            note: String::new(),
            note_cursor: 0,
            warm_up: String::new(),
            warm_up_cursor: 0,
            cool_down: String::new(),
            cool_down_cursor: 0,
            focus: FocusSection::Sets,
            editing_log_id: None,
            log_date: today,
            editing_date: false,
            date_input: String::new(),
            date_input_cursor: 0,
            return_to: Screen::Dashboard,
            status_msg: None,
        })
    }

    pub fn from_existing(db: &Database, log_entry: &LogEntry) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        let practice = practices
            .iter()
            .find(|p| p.id == log_entry.log.practice_id)
            .cloned();
        let sets: Vec<SetData> = log_entry.sets.iter().map(|s| s.data.clone()).collect();
        let note = log_entry.log.note.clone().unwrap_or_default();
        let warm_up = log_entry.log.warm_up.clone().unwrap_or_default();
        let cool_down = log_entry.log.cool_down.clone().unwrap_or_default();

        let log_date = log_entry.log.logged_at.format("%Y-%m-%d").to_string();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            phase: Phase::EnterLog,
            chosen_practice: practice,
            sets,
            field1: String::new(),
            field1_cursor: 0,
            field2: String::new(),
            field2_cursor: 0,
            active_field: 0,
            note_cursor: note.len(),
            note,
            warm_up_cursor: warm_up.len(),
            warm_up,
            cool_down_cursor: cool_down.len(),
            cool_down,
            focus: FocusSection::Sets,
            editing_log_id: Some(log_entry.log.id),
            log_date,
            editing_date: false,
            date_input: String::new(),
            date_input_cursor: 0,
            return_to: Screen::History,
            status_msg: None,
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        match self.phase {
            Phase::SelectPractice => self.render_select_practice(frame),
            Phase::EnterLog => self.render_enter_log(frame),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match self.phase {
            Phase::SelectPractice => self.handle_select_practice(key),
            Phase::EnterLog => self.handle_enter_log(key, db),
        }
    }

    // ── Phase 1: SelectPractice ───────────────────────────────────────

    fn render_select_practice(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let list_height = (self.filtered_indices.len() as u16).max(1);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),           // title
                Constraint::Length(2),           // filter bar + column header
                Constraint::Length(list_height), // list
                Constraint::Length(1),           // status line
                Constraint::Length(1),           // footer
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        let title = Line::from(Span::styled(
            format!(" {}", tr("log-select-practice")),
            Style::default().fg(ACCENT).bold(),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let max_name_len = self.practices.iter()
            .map(|p| p.name.width())
            .max()
            .unwrap_or(0);
        let col_width = max_name_len + 4;

        let filter_display = if self.filtering {
            let (before, after) = self.filter_text.split_at(self.filter_cursor);
            format!(" /{}{}{}", before, "\u{2588}", after)
        } else if !self.filter_text.is_empty() {
            format!(" /{}", self.filter_text)
        } else {
            format!(" {}", tr("log-press-filter"))
        };
        let filter_style = if self.filtering {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(Color::Gray)
        };
        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let header_padding = col_width.saturating_sub(name_header.width());
        let filter_lines = vec![
            Line::from(Span::styled(filter_display, filter_style)),
            Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(&name_header, Style::default().fg(Color::DarkGray)),
                Span::raw(" ".repeat(header_padding)),
                Span::styled(&type_header, Style::default().fg(Color::DarkGray)),
            ]),
        ];
        frame.render_widget(Paragraph::new(filter_lines), chunks[1]);

        let lines: Vec<Line> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let practice = &self.practices[idx];
                let marker = if i == self.selected { "> " } else { "  " };
                let name_style = if i == self.selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::White)
                };
                let padding = col_width.saturating_sub(practice.name.width());
                Line::from(vec![
                    Span::styled(marker, name_style),
                    Span::styled(&practice.name, name_style),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(
                        practice.practice_type.label(),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            })
            .collect();
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[2]);

        if !self.filtered_indices.is_empty() {
            highlight_row(frame, chunks[2], self.selected as u16);
        }

        render_status_line(frame, chunks[3], &self.status_msg);

        let footer = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-filter")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-select")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[4]);
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
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    }
                }
                KeyCode::Left => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    }
                }
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices().nth(1).map(|(i, _)| self.filter_cursor + i)
                            .unwrap_or(self.filter_text.len());
                    }
                }
                KeyCode::Right => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices().nth(1).map(|(i, _)| self.filter_cursor + i)
                            .unwrap_or(self.filter_text.len());
                    }
                }
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_cursor = 0;
                }
                KeyCode::Home => {
                    self.filter_cursor = 0;
                }
                KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_cursor = self.filter_text.len();
                }
                KeyCode::End => {
                    self.filter_cursor = self.filter_text.len();
                }
                KeyCode::Backspace => {
                    if self.filter_cursor > 0 {
                        let prev = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                        self.filter_text.remove(prev);
                        self.filter_cursor = prev;
                        self.apply_filter();
                    }
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_text.truncate(self.filter_cursor);
                }
                KeyCode::Char(c) => {
                    self.filter_text.insert(self.filter_cursor, c);
                    self.filter_cursor += c.len_utf8();
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
                    self.phase = Phase::EnterLog;
                    self.sets.clear();
                    self.field1.clear();
                    self.field1_cursor = 0;
                    self.field2.clear();
                    self.field2_cursor = 0;
                    self.active_field = 0;
                    self.focus = FocusSection::Sets;
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

    // ── Unified Log Entry Screen ─────────────────────────────────────

    fn render_enter_log(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let practice = self.chosen_practice.as_ref().unwrap();

        let sets_content_lines = (self.sets.len() as u16 + 2).max(2); // committed sets + input + total

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                    // [0] date section (border)
                Constraint::Length(1),                    // [1] spacer
                Constraint::Length(sets_content_lines + 2), // [2] sets section (border)
                Constraint::Length(4),                    // [3] warm-up/cool-down section (border)
                Constraint::Min(3),                      // [4] note section (border, grows)
                Constraint::Length(1),                    // [5] status line
                Constraint::Length(1),                    // [6] footer
            ])
            .split(area);

        // ── Date section ──
        let date_border_color = BORDER_COLOR;
        let date_title = format!(" {} ({}) ", practice.name, practice.practice_type.label());
        let date_block = Block::default()
            .title(Span::styled(date_title, Style::default().fg(ACCENT).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(date_border_color));
        let date_inner = date_block.inner(chunks[0]);
        frame.render_widget(date_block, chunks[0]);

        let date_line = if self.editing_date {
            let (before, after) = self.date_input.split_at(self.date_input_cursor);
            Line::from(vec![
                Span::styled(format!("{} ", tr("log-date-label")), Style::default().fg(Color::Gray)),
                Span::styled(before, Style::default().fg(ACCENT)),
                Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                Span::styled(after, Style::default().fg(ACCENT)),
                Span::styled(format!("  {}", tr("log-date-edit-hint")), Style::default().fg(Color::Gray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("{} ", tr("log-date-label")), Style::default().fg(Color::Gray)),
                Span::styled(&self.log_date, Style::default().fg(Color::White)),
                Span::styled(format!("  {}", tr("log-date-change-hint")), Style::default().fg(Color::Gray)),
            ])
        };
        frame.render_widget(Paragraph::new(date_line), date_inner);

        // ── Sets section ──
        let sets_border_color = BORDER_COLOR;
        let sets_block = Block::default()
            .title(Span::styled(" Sets ", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(sets_border_color));
        let sets_inner = sets_block.inner(chunks[2]);
        frame.render_widget(sets_block, chunks[2]);

        let mut sets_lines: Vec<Line> = Vec::new();
        for (i, set) in self.sets.iter().enumerate() {
            let text = tr_args("log-set-line", &[
                ("number", FluentValue::from((i + 1) as f64)),
                ("data", FluentValue::from(format_set_data(set))),
            ]);
            sets_lines.push(Line::from(Span::styled(text, Style::default().fg(Color::White))));
        }

        // Current input fields
        if self.focus == FocusSection::Sets {
            let set_num = self.sets.len() + 1;
            match practice.practice_type {
                PracticeType::Weighted => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    let (f2_before, f2_after) = self.field2.split_at(self.field2_cursor);
                    let f1_style = if self.active_field == 0 { ACCENT } else { Color::White };
                    let f2_style = if self.active_field == 1 { ACCENT } else { Color::White };
                    sets_lines.push(Line::from(vec![
                        Span::styled(format!("Set {}: ", set_num), Style::default().fg(Color::White)),
                        Span::styled(format!("{} ", tr("log-weight-label")), Style::default().fg(Color::Gray)),
                        Span::styled(f1_before, Style::default().fg(f1_style)),
                        if self.active_field == 0 { Span::styled("\u{2588}", Style::default().fg(ACCENT)) } else { Span::raw("") },
                        Span::styled(f1_after, Style::default().fg(f1_style)),
                        Span::styled(format!("  {} ", tr("log-reps-label")), Style::default().fg(Color::Gray)),
                        Span::styled(f2_before, Style::default().fg(f2_style)),
                        if self.active_field == 1 { Span::styled("\u{2588}", Style::default().fg(ACCENT)) } else { Span::raw("") },
                        Span::styled(f2_after, Style::default().fg(f2_style)),
                    ]));
                }
                PracticeType::Bodyweight => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(format!("Set {}: ", set_num), Style::default().fg(Color::White)),
                        Span::styled(format!("{} ", tr("log-reps-label")), Style::default().fg(Color::Gray)),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
                PracticeType::Distance => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(format!("Set {}: ", set_num), Style::default().fg(Color::White)),
                        Span::styled(format!("{} ", tr("log-distance-label")), Style::default().fg(Color::Gray)),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
                PracticeType::Endurance => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(format!("Set {}: ", set_num), Style::default().fg(Color::White)),
                        Span::styled(format!("{} ", tr("log-duration-label")), Style::default().fg(Color::Gray)),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
            }
        }

        // Running total
        let total: f64 = self.sets.iter().map(|s| s.metric_value()).sum();
        let label = self.sets.first()
            .map(|s| s.metric_label())
            .unwrap_or(metric_label_for_type(&practice.practice_type));
        let total_reps: i32 = self.sets.iter().map(|s| match s {
            SetData::Weighted { reps, .. } => *reps,
            _ => 0,
        }).sum();
        let total_formatted = format!("{:.1}", total);
        let total_text = if total_reps > 0 {
            tr_args("log-sets-total-reps", &[
                ("sets", FluentValue::from(self.sets.len() as f64)),
                ("total", FluentValue::from(total_formatted)),
                ("label", FluentValue::from(label.clone())),
                ("reps", FluentValue::from(total_reps as f64)),
            ])
        } else {
            tr_args("log-sets-total", &[
                ("sets", FluentValue::from(self.sets.len() as f64)),
                ("total", FluentValue::from(total_formatted)),
                ("label", FluentValue::from(label.clone())),
            ])
        };
        sets_lines.push(Line::from(Span::styled(total_text, Style::default().fg(Color::DarkGray))));

        frame.render_widget(Paragraph::new(sets_lines), sets_inner);

        // ── Warm-up / Cool-down section ──
        let wucd_border_color = BORDER_COLOR;
        let wucd_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(wucd_border_color));
        let wucd_inner = wucd_block.inner(chunks[3]);
        frame.render_widget(wucd_block, chunks[3]);

        let wu_active = self.focus == FocusSection::WarmUp;
        let wu_color = if wu_active { ACCENT } else { Color::White };
        let wu_label = format!("{}: ", tr("log-warmup-label"));
        let wu_prefix_w = wu_label.width() as u16;
        let mut wu_spans = vec![Span::styled(&wu_label, Style::default().fg(Color::Gray))];
        if wu_active {
            wu_spans.extend(visible_input_spans(&self.warm_up, self.warm_up_cursor, wucd_inner.width, wu_prefix_w, wu_color));
        } else {
            wu_spans.push(Span::styled(&self.warm_up, Style::default().fg(wu_color)));
        }

        let cd_active = self.focus == FocusSection::CoolDown;
        let cd_color = if cd_active { ACCENT } else { Color::White };
        let cd_label = format!("{}: ", tr("log-cooldown-label"));
        let cd_prefix_w = cd_label.width() as u16;
        let mut cd_spans = vec![Span::styled(&cd_label, Style::default().fg(Color::Gray))];
        if cd_active {
            cd_spans.extend(visible_input_spans(&self.cool_down, self.cool_down_cursor, wucd_inner.width, cd_prefix_w, cd_color));
        } else {
            cd_spans.push(Span::styled(&self.cool_down, Style::default().fg(cd_color)));
        }

        let wucd_lines = vec![
            Line::from(wu_spans),
            Line::from(cd_spans),
        ];
        frame.render_widget(Paragraph::new(wucd_lines), wucd_inner);

        // ── Note section ──
        let note_border_color = BORDER_COLOR;
        let note_block = Block::default()
            .title(Span::styled(
                format!(" {} ", tr("log-note-optional")),
                Style::default().fg(Color::Gray),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(note_border_color));
        let note_inner = note_block.inner(chunks[4]);
        frame.render_widget(note_block, chunks[4]);

        if self.focus == FocusSection::Note {
            let before = &self.note[..self.note_cursor];
            let after = &self.note[self.note_cursor..];
            let note_line = Line::from(vec![
                Span::styled(before, Style::default().fg(Color::White)),
                Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                Span::styled(after, Style::default().fg(Color::White)),
            ]);
            frame.render_widget(Paragraph::new(note_line).wrap(Wrap { trim: false }), note_inner);
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(&self.note, Style::default().fg(Color::White)))
                    .wrap(Wrap { trim: false }),
                note_inner,
            );
        }

        // ── Status line ──
        render_status_line(frame, chunks[5], &self.status_msg);

        // ── Footer ──
        let footer = Line::from(vec![
            Span::styled(" [Tab]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-next")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-add-set")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-save")), Style::default().fg(Color::DarkGray)),
            Span::styled("[D]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-date")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[6]);
    }

    fn handle_enter_log(&mut self, key: KeyEvent, db: &Database) -> Action {
        if self.editing_date {
            return self.handle_date_edit(key);
        }

        let practice = self.chosen_practice.as_ref().unwrap().clone();
        let is_weighted = practice.practice_type == PracticeType::Weighted;

        // Ctrl+S — save from any section
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            if !self.sets.is_empty() {
                return self.save_log(db);
            }
            return Action::None;
        }

        // D — edit date from sets section
        if key.code == KeyCode::Char('D') && !key.modifiers.contains(KeyModifiers::CONTROL)
            && self.focus == FocusSection::Sets
        {
            self.editing_date = true;
            self.date_input = self.log_date.clone();
            self.date_input_cursor = self.date_input.len();
            return Action::None;
        }

        // Esc — cancel from any section
        if key.code == KeyCode::Esc {
            return Action::Navigate(self.return_to.clone());
        }

        // Tab / Shift+Tab — cycle focus
        if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
            let forward = key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT);
            self.advance_focus(forward, is_weighted);
            return Action::None;
        }

        // Section-specific key handling
        match self.focus {
            FocusSection::Sets => self.handle_sets_input(key, is_weighted),
            FocusSection::WarmUp => self.handle_text_field_input(key, TextFieldTarget::WarmUp),
            FocusSection::CoolDown => self.handle_text_field_input(key, TextFieldTarget::CoolDown),
            FocusSection::Note => self.handle_text_field_input(key, TextFieldTarget::Note),
        }
    }

    fn advance_focus(&mut self, forward: bool, is_weighted: bool) {
        if forward {
            match self.focus {
                FocusSection::Sets => {
                    if is_weighted && self.active_field == 0 {
                        self.active_field = 1;
                    } else {
                        self.focus = FocusSection::WarmUp;
                    }
                }
                FocusSection::WarmUp => {
                    self.focus = FocusSection::CoolDown;
                }
                FocusSection::CoolDown => {
                    self.focus = FocusSection::Note;
                    self.note_cursor = self.note.len();
                }
                FocusSection::Note => {
                    self.focus = FocusSection::Sets;
                    self.active_field = 0;
                }
            }
        } else {
            match self.focus {
                FocusSection::Sets => {
                    if is_weighted && self.active_field == 1 {
                        self.active_field = 0;
                    } else {
                        self.focus = FocusSection::Note;
                        self.note_cursor = self.note.len();
                    }
                }
                FocusSection::WarmUp => {
                    self.focus = FocusSection::Sets;
                    self.active_field = if is_weighted { 1 } else { 0 };
                }
                FocusSection::CoolDown => {
                    self.focus = FocusSection::WarmUp;
                }
                FocusSection::Note => {
                    self.focus = FocusSection::CoolDown;
                }
            }
        }
    }

    fn handle_sets_input(&mut self, key: KeyEvent, is_weighted: bool) -> Action {
        let has_two_fields = is_weighted;

        // Ctrl+K – delete from cursor to end
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k') {
            if has_two_fields && self.active_field == 0 {
                self.field1.truncate(self.field1_cursor);
            } else if has_two_fields && self.active_field == 1 {
                self.field2.truncate(self.field2_cursor);
            } else {
                self.field1.truncate(self.field1_cursor);
            }
            return Action::None;
        }

        let both_fields_empty = self.field1.is_empty()
            && (self.field2.is_empty() || !has_two_fields);

        let (text, cursor) = if has_two_fields {
            if self.active_field == 0 {
                (&mut self.field1, &mut self.field1_cursor)
            } else {
                (&mut self.field2, &mut self.field2_cursor)
            }
        } else {
            (&mut self.field1, &mut self.field1_cursor)
        };

        // Emacs cursor nav
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b')
            || key.code == KeyCode::Left
        {
            if *cursor > 0 {
                *cursor = text[..*cursor].char_indices()
                    .next_back().map(|(i, _)| i).unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..].char_indices().nth(1)
                    .map(|(i, _)| *cursor + i).unwrap_or(text.len());
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a')
            || key.code == KeyCode::Home
        {
            *cursor = 0;
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e')
            || key.code == KeyCode::End
        {
            *cursor = text.len();
            return Action::None;
        }

        match key.code {
            KeyCode::Enter => {
                if has_two_fields && self.active_field == 0 {
                    self.active_field = 1;
                } else {
                    self.commit_set();
                }
                Action::None
            }
            KeyCode::Backspace => {
                if text.is_empty() && both_fields_empty && !self.sets.is_empty() {
                    self.sets.pop();
                } else if *cursor > 0 {
                    let prev = text[..*cursor].char_indices().next_back()
                        .map(|(i, _)| i).unwrap_or(0);
                    text.remove(prev);
                    *cursor = prev;
                }
                Action::None
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                text.insert(*cursor, c);
                *cursor += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_text_field_input(&mut self, key: KeyEvent, target: TextFieldTarget) -> Action {
        let (text, cursor) = match target {
            TextFieldTarget::WarmUp => (&mut self.warm_up, &mut self.warm_up_cursor),
            TextFieldTarget::CoolDown => (&mut self.cool_down, &mut self.cool_down_cursor),
            TextFieldTarget::Note => (&mut self.note, &mut self.note_cursor),
        };

        // Ctrl+K
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k') {
            text.truncate(*cursor);
            return Action::None;
        }

        // Emacs cursor nav
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b')
            || key.code == KeyCode::Left
        {
            if *cursor > 0 {
                *cursor = text[..*cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..].char_indices().nth(1).map(|(i, _)| *cursor + i).unwrap_or(text.len());
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a')
            || key.code == KeyCode::Home
        {
            *cursor = 0;
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e')
            || key.code == KeyCode::End
        {
            *cursor = text.len();
            return Action::None;
        }

        match key.code {
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let prev = text[..*cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    text.remove(prev);
                    *cursor = prev;
                }
                Action::None
            }
            KeyCode::Char(c) => {
                text.insert(*cursor, c);
                *cursor += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_date_edit(&mut self, key: KeyEvent) -> Action {
        let (text, cursor) = (&mut self.date_input, &mut self.date_input_cursor);

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b')
            || key.code == KeyCode::Left
        {
            if *cursor > 0 {
                *cursor = text[..*cursor].char_indices().next_back()
                    .map(|(i, _)| i).unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..].char_indices().nth(1)
                    .map(|(i, _)| *cursor + i).unwrap_or(text.len());
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a')
            || key.code == KeyCode::Home
        {
            *cursor = 0;
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e')
            || key.code == KeyCode::End
        {
            *cursor = text.len();
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k') {
            text.truncate(*cursor);
            return Action::None;
        }

        match key.code {
            KeyCode::Enter => {
                if NaiveDate::parse_from_str(&self.date_input, "%Y-%m-%d").is_ok() {
                    self.log_date = self.date_input.clone();
                }
                self.editing_date = false;
            }
            KeyCode::Esc => {
                self.editing_date = false;
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let prev = text[..*cursor].char_indices().next_back()
                        .map(|(i, _)| i).unwrap_or(0);
                    text.remove(prev);
                    *cursor = prev;
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                text.insert(*cursor, c);
                *cursor += c.len_utf8();
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
            if practice.practice_type == PracticeType::Weighted {
                self.field2.clear();
                self.field2_cursor = 0;
                self.active_field = 1;
            } else {
                self.field1.clear();
                self.field1_cursor = 0;
                self.active_field = 0;
            }
        }
    }

    fn save_log(&mut self, db: &Database) -> Action {
        let practice = self.chosen_practice.as_ref().unwrap();
        let note = if self.note.is_empty() { None } else { Some(self.note.as_str()) };
        let warm_up = if self.warm_up.is_empty() { None } else { Some(self.warm_up.as_str()) };
        let cool_down = if self.cool_down.is_empty() { None } else { Some(self.cool_down.as_str()) };
        let date = NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let datetime = date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        let result = if let Some(log_id) = self.editing_log_id {
            db.update_log(log_id, &self.sets, note, Some(&datetime), warm_up, cool_down)
        } else {
            db.create_log_at(practice.id, &datetime, &self.sets, note, warm_up, cool_down).map(|_| ())
        };
        match result {
            Ok(_) => Action::Navigate(self.return_to.clone()),
            Err(e) => {
                self.status_msg = Some((format!("Error: {}", e), true));
                Action::None
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TextFieldTarget {
    WarmUp,
    CoolDown,
    Note,
}

fn format_set_data(set: &SetData) -> String {
    use crate::i18n::tr_args;
    use fluent_bundle::FluentValue;
    match set {
        SetData::Weighted { weight, reps } => tr_args("set-weighted", &[
            ("weight", FluentValue::from(*weight)),
            ("reps", FluentValue::from(*reps as f64)),
        ]),
        SetData::Bodyweight { reps } => tr_args("set-bodyweight", &[
            ("reps", FluentValue::from(*reps as f64)),
        ]),
        SetData::Distance { distance } => tr_args("set-distance", &[
            ("distance", FluentValue::from(*distance)),
        ]),
        SetData::Endurance { duration } => tr_args("set-endurance", &[
            ("duration", FluentValue::from(*duration)),
        ]),
    }
}

fn metric_label_for_type(pt: &PracticeType) -> String {
    use crate::i18n::tr;
    match pt {
        PracticeType::Weighted => tr("metric-kg-vol"),
        PracticeType::Bodyweight => tr("metric-reps"),
        PracticeType::Distance => tr("metric-km"),
        PracticeType::Endurance => tr("metric-min"),
    }
}
