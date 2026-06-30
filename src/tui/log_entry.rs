use chrono::{Local, NaiveDate, NaiveTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::{
    centered_area, highlight_row, render_status_line, visible_input_spans, Action, Screen,
    StatusMessage, BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, Practice, PracticeType, SetData};
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
    Rpe,
    Note,
}

pub struct LogEntryScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filter_cursor: usize,
    filtering: bool,
    selected: usize,
    scroll: usize,
    list_height: usize,
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
    rpe_input: String,
    rpe_cursor: usize,
    focus: FocusSection,
    selected_set: Option<usize>,
    editing_set: Option<usize>,
    editing_log_id: Option<i64>,
    log_date: String,
    editing_date: bool,
    date_input: String,
    date_input_cursor: usize,
    confirming_exit: bool,
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
            scroll: 0,
            list_height: 0,
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
            rpe_input: String::new(),
            rpe_cursor: 0,
            focus: FocusSection::Sets,
            selected_set: None,
            editing_set: None,
            editing_log_id: None,
            log_date: today,
            editing_date: false,
            date_input: String::new(),
            date_input_cursor: 0,
            confirming_exit: false,
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
        let rpe_input = log_entry.log.rpe.map(|r| r.to_string()).unwrap_or_default();

        let log_date = log_entry.log.logged_at.format("%Y-%m-%d").to_string();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            scroll: 0,
            list_height: 0,
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
            rpe_cursor: rpe_input.len(),
            rpe_input,
            focus: FocusSection::Sets,
            selected_set: None,
            editing_set: None,
            editing_log_id: Some(log_entry.log.id),
            log_date,
            editing_date: false,
            date_input: String::new(),
            date_input_cursor: 0,
            confirming_exit: false,
            return_to: Screen::History,
            status_msg: None,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
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

    fn render_select_practice(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // filter bar
                Constraint::Min(1),    // bordered list
                Constraint::Length(1), // status line
                Constraint::Length(1), // footer
                Constraint::Min(0),    // spacer
            ])
            .split(area);

        let max_name_len = self
            .practices
            .iter()
            .map(|p| p.name.width())
            .max()
            .unwrap_or(0);
        let col_width = max_name_len + 4;

        // Filter bar
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
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(filter_display, filter_style))),
            chunks[0],
        );

        // Bordered list
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("log-select-practice"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let header_padding = col_width.saturating_sub(name_header.width());

        let mut all_lines = vec![Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&name_header, Style::default().fg(Color::DarkGray)),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&type_header, Style::default().fg(Color::DarkGray)),
        ])];

        let visible_rows = inner.height.saturating_sub(1) as usize; // -1 for header
        self.list_height = visible_rows;
        let end = (self.scroll + visible_rows).min(self.filtered_indices.len());

        for i in self.scroll..end {
            let idx = self.filtered_indices[i];
            let practice = &self.practices[idx];
            let marker = if i == self.selected { "> " } else { "  " };
            let name_style = if i == self.selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let padding = col_width.saturating_sub(practice.name.width());
            all_lines.push(Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(&practice.name, name_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(
                    practice.practice_type.label(),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        frame.render_widget(Paragraph::new(all_lines), inner);

        if !self.filtered_indices.is_empty() && self.selected >= self.scroll && self.selected < end {
            highlight_row(frame, inner, (self.selected - self.scroll) as u16 + 1); // +1 for header
        }

        render_status_line(frame, chunks[2], &self.status_msg);

        let footer = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-navigate")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-filter")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[Enter]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-select")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}", tr("key-back")),
                Style::default().fg(Color::DarkGray),
            ),
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
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
                KeyCode::Left => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| self.filter_cursor + i)
                            .unwrap_or(self.filter_text.len());
                    }
                }
                KeyCode::Right => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| self.filter_cursor + i)
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
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
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
                    self.adjust_scroll();
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
                    self.adjust_scroll();
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
                    self.selected_set = None;
                    self.editing_set = None;
                    self.focus = FocusSection::Sets;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + self.list_height && self.list_height > 0 {
            self.scroll = self.selected + 1 - self.list_height;
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
        self.scroll = 0;
    }

    // ── Unified Log Entry Screen ─────────────────────────────────────

    fn render_enter_log(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let practice = self.chosen_practice.as_ref().unwrap();

        let sets_content_lines = (self.sets.len() as u16 + 2).max(2); // committed sets + input + total

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // [0] date section (border + padding)
                Constraint::Length(1), // [1] spacer
                Constraint::Length(sets_content_lines + 4), // [2] sets section (border + padding)
                Constraint::Length(7), // [3] warm-up/cool-down section (border + padding)
                Constraint::Min(3),    // [4] note section (border, grows)
                Constraint::Length(1), // [5] status line
                Constraint::Length(1), // [6] footer
            ])
            .split(area);

        // ── Date section ──
        let date_border_color = BORDER_COLOR;
        let date_title = format!(" {} ({}) ", practice.name, practice.practice_type.label());
        let date_block = Block::default()
            .title(Span::styled(date_title, Style::default().fg(ACCENT).bold()))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(date_border_color));
        let date_inner = date_block.inner(chunks[0]);
        frame.render_widget(date_block, chunks[0]);

        let date_line = if self.editing_date {
            let (before, after) = self.date_input.split_at(self.date_input_cursor);
            Line::from(vec![
                Span::styled(
                    format!("{} ", tr("log-date-label")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(before, Style::default().fg(ACCENT)),
                Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                Span::styled(after, Style::default().fg(ACCENT)),
                Span::styled(
                    format!("  {}", tr("log-date-edit-hint")),
                    Style::default().fg(Color::Gray),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    format!("{} ", tr("log-date-label")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(&self.log_date, Style::default().fg(Color::White)),
                Span::styled(
                    format!("  {}", tr("log-date-change-hint")),
                    Style::default().fg(Color::Gray),
                ),
            ])
        };
        frame.render_widget(Paragraph::new(date_line), date_inner);

        // ── Sets section ──
        let sets_border_color = BORDER_COLOR;
        let sets_block = Block::default()
            .title(Span::styled(
                " Sets ",
                Style::default().fg(Color::White).bold(),
            ))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(sets_border_color));
        let sets_inner = sets_block.inner(chunks[2]);
        frame.render_widget(sets_block, chunks[2]);

        let mut sets_lines: Vec<Line> = Vec::new();
        for (i, set) in self.sets.iter().enumerate() {
            let text = tr_args(
                "log-set-line",
                &[
                    ("number", FluentValue::from((i + 1) as f64)),
                    ("data", FluentValue::from(format_set_data(set))),
                ],
            );
            let is_selected = self.selected_set == Some(i) && self.editing_set != Some(i);
            let is_editing = self.editing_set == Some(i);
            let style = if is_editing {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if is_selected || is_editing {
                "> "
            } else {
                "  "
            };
            sets_lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(text, style),
            ]));
        }

        // Current input fields
        if self.focus == FocusSection::Sets {
            let input_label = if let Some(idx) = self.editing_set {
                format!("Edit Set {}: ", idx + 1)
            } else {
                format!("Set {}: ", self.sets.len() + 1)
            };
            match practice.practice_type {
                PracticeType::Weighted => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    let (f2_before, f2_after) = self.field2.split_at(self.field2_cursor);
                    let f1_style = if self.active_field == 0 {
                        ACCENT
                    } else {
                        Color::White
                    };
                    let f2_style = if self.active_field == 1 {
                        ACCENT
                    } else {
                        Color::White
                    };
                    sets_lines.push(Line::from(vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{} ", tr("log-weight-label")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(f1_before, Style::default().fg(f1_style)),
                        if self.active_field == 0 {
                            Span::styled("\u{2588}", Style::default().fg(ACCENT))
                        } else {
                            Span::raw("")
                        },
                        Span::styled(f1_after, Style::default().fg(f1_style)),
                        Span::styled(
                            format!("  {} ", tr("log-reps-label")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(f2_before, Style::default().fg(f2_style)),
                        if self.active_field == 1 {
                            Span::styled("\u{2588}", Style::default().fg(ACCENT))
                        } else {
                            Span::raw("")
                        },
                        Span::styled(f2_after, Style::default().fg(f2_style)),
                    ]));
                }
                PracticeType::Bodyweight => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{} ", tr("log-reps-label")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
                PracticeType::Distance => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{} ", tr("log-distance-label")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
                PracticeType::Endurance => {
                    let (f1_before, f1_after) = self.field1.split_at(self.field1_cursor);
                    sets_lines.push(Line::from(vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{} ", tr("log-duration-label")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(f1_before, Style::default().fg(ACCENT)),
                        Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                        Span::styled(f1_after, Style::default().fg(ACCENT)),
                    ]));
                }
            }
        }

        // Running total
        let total: f64 = self.sets.iter().map(|s| s.metric_value()).sum();
        let label = self
            .sets
            .first()
            .map(|s| s.metric_label())
            .unwrap_or(metric_label_for_type(&practice.practice_type));
        let total_reps: i32 = self
            .sets
            .iter()
            .map(|s| match s {
                SetData::Weighted { reps, .. } => *reps,
                _ => 0,
            })
            .sum();
        let total_formatted = format!("{:.1}", total);
        let total_text = if total_reps > 0 {
            tr_args(
                "log-sets-total-reps",
                &[
                    ("sets", FluentValue::from(self.sets.len() as f64)),
                    ("total", FluentValue::from(total_formatted)),
                    ("label", FluentValue::from(label.clone())),
                    ("reps", FluentValue::from(total_reps as f64)),
                ],
            )
        } else {
            tr_args(
                "log-sets-total",
                &[
                    ("sets", FluentValue::from(self.sets.len() as f64)),
                    ("total", FluentValue::from(total_formatted)),
                    ("label", FluentValue::from(label.clone())),
                ],
            )
        };
        sets_lines.push(Line::from(Span::styled(
            total_text,
            Style::default().fg(Color::DarkGray),
        )));

        frame.render_widget(Paragraph::new(sets_lines), sets_inner);

        // ── Warm-up / Cool-down section ──
        let wucd_border_color = BORDER_COLOR;
        let wucd_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(wucd_border_color));
        let wucd_inner = wucd_block.inner(chunks[3]);
        frame.render_widget(wucd_block, chunks[3]);

        let wu_active = self.focus == FocusSection::WarmUp;
        let wu_color = if wu_active { ACCENT } else { Color::White };
        let wu_label = format!("{}: ", tr("log-warmup-label"));
        let wu_prefix_w = wu_label.width() as u16;
        let mut wu_spans = vec![Span::styled(&wu_label, Style::default().fg(Color::Gray))];
        if wu_active {
            wu_spans.extend(visible_input_spans(
                &self.warm_up,
                self.warm_up_cursor,
                wucd_inner.width,
                wu_prefix_w,
                wu_color,
            ));
        } else {
            wu_spans.push(Span::styled(&self.warm_up, Style::default().fg(wu_color)));
        }

        let cd_active = self.focus == FocusSection::CoolDown;
        let cd_color = if cd_active { ACCENT } else { Color::White };
        let cd_label = format!("{}: ", tr("log-cooldown-label"));
        let cd_prefix_w = cd_label.width() as u16;
        let mut cd_spans = vec![Span::styled(&cd_label, Style::default().fg(Color::Gray))];
        if cd_active {
            cd_spans.extend(visible_input_spans(
                &self.cool_down,
                self.cool_down_cursor,
                wucd_inner.width,
                cd_prefix_w,
                cd_color,
            ));
        } else {
            cd_spans.push(Span::styled(&self.cool_down, Style::default().fg(cd_color)));
        }

        let rpe_active = self.focus == FocusSection::Rpe;
        let rpe_color = if rpe_active { ACCENT } else { Color::White };
        let rpe_label = "RPE (1-10): ";
        let rpe_prefix_w = rpe_label.width() as u16;
        let mut rpe_spans = vec![Span::styled(rpe_label, Style::default().fg(Color::Gray))];
        if rpe_active {
            rpe_spans.extend(visible_input_spans(
                &self.rpe_input,
                self.rpe_cursor,
                wucd_inner.width,
                rpe_prefix_w,
                rpe_color,
            ));
        } else {
            rpe_spans.push(Span::styled(&self.rpe_input, Style::default().fg(rpe_color)));
        }

        let wucd_lines = vec![Line::from(wu_spans), Line::from(cd_spans), Line::from(rpe_spans)];
        frame.render_widget(Paragraph::new(wucd_lines), wucd_inner);

        // ── Note section ──
        let note_border_color = BORDER_COLOR;
        let note_block = Block::default()
            .title(Span::styled(
                format!(" {} ", tr("log-note-optional")),
                Style::default().fg(Color::Gray),
            ))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
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
            frame.render_widget(
                Paragraph::new(note_line).wrap(Wrap { trim: false }),
                note_inner,
            );
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
        let footer_line = if self.confirming_exit {
            Line::from(vec![
                Span::styled(
                    format!(" {} ", tr("log-cancel-confirm")),
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
        } else {
            let mut spans = vec![
                Span::styled(" [Tab]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-next")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-add-set")),
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            if self.focus == FocusSection::Sets && !self.sets.is_empty() {
                spans.push(Span::styled("[j/k]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(
                    format!(" {}  ", tr("key-navigate")),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled("[d]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(
                    format!(" {}  ", tr("key-delete")),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled("[e]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(
                    format!(" {}  ", tr("key-edit")),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            spans.extend(vec![
                Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-save")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[D]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-date")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            Line::from(spans)
        };
        frame.render_widget(Paragraph::new(footer_line), chunks[6]);
    }

    fn handle_enter_log(&mut self, key: KeyEvent, db: &Database) -> Action {
        if self.confirming_exit {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    return Action::Navigate(self.return_to.clone());
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirming_exit = false;
                    return Action::None;
                }
                _ => return Action::None,
            }
        }

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

        // When editing a set, restrict navigation keys within Sets section
        if self.editing_set.is_some() && self.focus == FocusSection::Sets {
            if key.code == KeyCode::Esc {
                self.cancel_edit();
                return Action::None;
            }
            if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
                self.cancel_edit();
                // fall through to normal Tab handling
            } else if key.code == KeyCode::Char('D')
                && !key.modifiers.contains(KeyModifiers::CONTROL)
            {
                self.cancel_edit();
                // fall through to normal D handling
            } else {
                return self.handle_sets_input(key, is_weighted);
            }
        }

        // D — edit date from sets section
        if key.code == KeyCode::Char('D')
            && !key.modifiers.contains(KeyModifiers::CONTROL)
            && self.focus == FocusSection::Sets
        {
            self.editing_date = true;
            self.date_input = self.log_date.clone();
            self.date_input_cursor = self.date_input.len();
            return Action::None;
        }

        // Esc — cancel from any section
        if key.code == KeyCode::Esc {
            self.confirming_exit = true;
            return Action::None;
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
            FocusSection::Rpe => self.handle_text_field_input(key, TextFieldTarget::Rpe),
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
                    self.focus = FocusSection::Rpe;
                    self.rpe_cursor = self.rpe_input.len();
                }
                FocusSection::Rpe => {
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
                FocusSection::Rpe => {
                    self.focus = FocusSection::CoolDown;
                }
                FocusSection::Note => {
                    self.focus = FocusSection::Rpe;
                    self.rpe_cursor = self.rpe_input.len();
                }
            }
        }
    }

    fn cancel_edit(&mut self) {
        self.editing_set = None;
        self.field1.clear();
        self.field1_cursor = 0;
        self.field2.clear();
        self.field2_cursor = 0;
        self.active_field = 0;
    }

    fn handle_sets_input(&mut self, key: KeyEvent, is_weighted: bool) -> Action {
        let has_two_fields = is_weighted;
        let n = self.sets.len();

        // Set navigation with j/k (only when not editing a set)
        if self.editing_set.is_none() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if n > 0 {
                        self.selected_set = if let Some(i) = self.selected_set {
                            if i + 1 < n {
                                Some(i + 1)
                            } else {
                                None
                            }
                        } else {
                            Some(0)
                        };
                    }
                    return Action::None;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if n > 0 {
                        self.selected_set = if let Some(i) = self.selected_set {
                            if i > 0 {
                                Some(i - 1)
                            } else {
                                None
                            }
                        } else {
                            Some(n - 1)
                        };
                    }
                    return Action::None;
                }
                _ => {}
            }
        }

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

        let both_fields_empty =
            self.field1.is_empty() && (self.field2.is_empty() || !has_two_fields);

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
                *cursor = text[..*cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor + i)
                    .unwrap_or(text.len());
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
                if has_two_fields && self.active_field == 0 && self.editing_set.is_none() {
                    self.active_field = 1;
                } else {
                    self.commit_set();
                }
                Action::None
            }
            KeyCode::Backspace => {
                if text.is_empty() && !self.sets.is_empty() {
                    if let Some(idx) = self.selected_set {
                        // Selected set + empty active field = delete it
                        self.sets.remove(idx);
                        self.selected_set = if self.sets.is_empty() {
                            None
                        } else if idx >= self.sets.len() {
                            Some(self.sets.len() - 1)
                        } else {
                            Some(idx)
                        };
                    } else if both_fields_empty {
                        self.sets.pop();
                    }
                } else if *cursor > 0 {
                    let prev = text[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    text.remove(prev);
                    *cursor = prev;
                }
                Action::None
            }
            KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(idx) = self.selected_set {
                    self.sets.remove(idx);
                    self.selected_set = if self.sets.is_empty() {
                        None
                    } else if idx >= self.sets.len() {
                        Some(self.sets.len() - 1)
                    } else {
                        Some(idx)
                    };
                }
                Action::None
            }
            KeyCode::Char('e') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(idx) = self.selected_set {
                    self.editing_set = Some(idx);
                    let set = &self.sets[idx];
                    match set {
                        SetData::Weighted { weight, reps } => {
                            self.field1 =
                                format!("{:.1}", weight).trim_end_matches(".0").to_string();
                            if self.field1.ends_with(".0") {
                                self.field1 = format!("{:.0}", weight);
                            }
                            self.field2 = reps.to_string();
                        }
                        SetData::Bodyweight { reps } => {
                            self.field1 = reps.to_string();
                        }
                        SetData::Distance { distance } => {
                            self.field1 = format!("{:.1}", distance)
                                .trim_end_matches(".0")
                                .to_string();
                            if self.field1.ends_with(".0") {
                                self.field1 = format!("{:.0}", distance);
                            }
                        }
                        SetData::Endurance { duration } => {
                            self.field1 = format!("{:.1}", duration)
                                .trim_end_matches(".0")
                                .to_string();
                            if self.field1.ends_with(".0") {
                                self.field1 = format!("{:.0}", duration);
                            }
                        }
                    }
                    self.field1_cursor = self.field1.len();
                    self.field2_cursor = self.field2.len();
                    self.active_field = 0;
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
            TextFieldTarget::Rpe => (&mut self.rpe_input, &mut self.rpe_cursor),
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
                *cursor = text[..*cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor + i)
                    .unwrap_or(text.len());
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
                    let prev = text[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    text.remove(prev);
                    *cursor = prev;
                }
                Action::None
            }
            KeyCode::Char(c) => {
                if matches!(target, TextFieldTarget::Rpe) {
                    if c.is_ascii_digit() && text.len() < 2 {
                        text.insert(*cursor, c);
                        *cursor += c.len_utf8();
                    }
                } else {
                    text.insert(*cursor, c);
                    *cursor += c.len_utf8();
                }
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
                *cursor = text[..*cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
            || key.code == KeyCode::Right
        {
            if *cursor < text.len() {
                *cursor = text[*cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor + i)
                    .unwrap_or(text.len());
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
                    let prev = text[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
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
            if let Some(idx) = self.editing_set {
                self.sets[idx] = data;
                self.editing_set = None;
            } else {
                self.sets.push(data);
            }
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
        
        let rpe = if self.rpe_input.is_empty() {
            None
        } else {
            match self.rpe_input.parse::<u8>() {
                Ok(val) if (1..=10).contains(&val) => Some(val),
                _ => {
                    self.status_msg = Some(("RPE must be a number between 1 and 10".to_string(), true));
                    return Action::None;
                }
            }
        };

        let note = if self.note.is_empty() {
            None
        } else {
            Some(self.note.as_str())
        };
        let warm_up = if self.warm_up.is_empty() {
            None
        } else {
            Some(self.warm_up.as_str())
        };
        let cool_down = if self.cool_down.is_empty() {
            None
        } else {
            Some(self.cool_down.as_str())
        };
        let date = NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let datetime = date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        let result = if let Some(log_id) = self.editing_log_id {
            db.update_log(
                log_id,
                &self.sets,
                note,
                Some(&datetime),
                warm_up,
                cool_down,
                rpe,
            )
        } else {
            db.create_log_at(practice.id, &datetime, &self.sets, note, warm_up, cool_down, rpe)
                .map(|_| ())
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
    Rpe,
    Note,
}

fn format_set_data(set: &SetData) -> String {
    use crate::i18n::tr_args;
    use fluent_bundle::FluentValue;
    match set {
        SetData::Weighted { weight, reps } => tr_args(
            "set-weighted",
            &[
                ("weight", FluentValue::from(*weight)),
                ("reps", FluentValue::from(*reps as f64)),
            ],
        ),
        SetData::Bodyweight { reps } => tr_args(
            "set-bodyweight",
            &[("reps", FluentValue::from(*reps as f64))],
        ),
        SetData::Distance { distance } => tr_args(
            "set-distance",
            &[("distance", FluentValue::from(*distance))],
        ),
        SetData::Endurance { duration } => tr_args(
            "set-endurance",
            &[("duration", FluentValue::from(*duration))],
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn test_db() -> (Database, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Database::open(dir.path().join("test.db").as_ref()).expect("open db");
        (db, dir)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    #[test]
    fn sets_cursor_movement_works() {
        let (db, _dir) = test_db();
        db.create_practice("Bench", PracticeType::Weighted).unwrap();

        let mut screen = LogEntryScreen::new(&db).unwrap();
        // Select practice -> EnterLog phase
        screen.handle_key(key(KeyCode::Enter), &db);

        // Type weight
        screen.handle_key(char_key('5'), &db);
        screen.handle_key(char_key('0'), &db);
        assert_eq!(screen.field1, "50");
        assert_eq!(screen.field1_cursor, 2);

        // Tab to reps field
        screen.handle_key(key(KeyCode::Tab), &db);
        assert_eq!(screen.active_field, 1);

        // Type reps
        screen.handle_key(char_key('1'), &db);
        screen.handle_key(char_key('0'), &db);
        assert_eq!(screen.field2, "10");
        assert_eq!(screen.field2_cursor, 2);

        // Left moves cursor back
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2_cursor, 1);
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2_cursor, 0);
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2_cursor, 0); // clamped

        // Right moves cursor forward
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2_cursor, 1);
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2_cursor, 2);
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2_cursor, 2); // clamped

        // Ctrl+A to start
        screen.handle_key(ctrl_key('a'), &db);
        assert_eq!(screen.field2_cursor, 0);

        // Ctrl+E to end
        screen.handle_key(ctrl_key('e'), &db);
        assert_eq!(screen.field2_cursor, 2);
    }

    #[test]
    fn sets_navigation_and_delete_works() {
        let (db, _dir) = test_db();
        db.create_practice("Bench", PracticeType::Weighted).unwrap();

        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);

        // Add first set: weight then reps
        screen.handle_key(char_key('5'), &db);
        screen.handle_key(char_key('0'), &db);
        screen.handle_key(key(KeyCode::Tab), &db);
        screen.handle_key(char_key('1'), &db);
        screen.handle_key(char_key('0'), &db);
        screen.handle_key(key(KeyCode::Enter), &db);
        // Add 2 more sets (field1 retains weight, active_field stays at 1)
        for _ in 0..2 {
            screen.handle_key(char_key('1'), &db);
            screen.handle_key(char_key('0'), &db);
            screen.handle_key(key(KeyCode::Enter), &db);
        }
        assert_eq!(screen.sets.len(), 3);
        assert!(screen.selected_set.is_none());

        // k selects last set (index 2)
        screen.handle_key(char_key('k'), &db);
        assert_eq!(screen.selected_set, Some(2));

        // k moves up to set 1 (index 1)
        screen.handle_key(char_key('k'), &db);
        assert_eq!(screen.selected_set, Some(1));

        // k moves up to set 0
        screen.handle_key(char_key('k'), &db);
        assert_eq!(screen.selected_set, Some(0));

        // k from first set goes back to input line
        screen.handle_key(char_key('k'), &db);
        assert!(screen.selected_set.is_none());

        // j from input goes to first set
        screen.handle_key(char_key('j'), &db);
        assert_eq!(screen.selected_set, Some(0));

        // j moves down through sets
        screen.handle_key(char_key('j'), &db);
        assert_eq!(screen.selected_set, Some(1));
        screen.handle_key(char_key('j'), &db);
        assert_eq!(screen.selected_set, Some(2));

        // j from last set goes to input line
        screen.handle_key(char_key('j'), &db);
        assert!(screen.selected_set.is_none());

        // Select set 1 and delete it
        screen.handle_key(char_key('k'), &db); // to set 2
        screen.handle_key(char_key('k'), &db); // to set 1
        screen.handle_key(char_key('d'), &db);
        assert_eq!(screen.sets.len(), 2);
        // After delete, should still point to valid set
        assert_eq!(screen.selected_set, Some(1));

        // Backspace with empty fields and set selected deletes selected set
        screen.handle_key(char_key('k'), &db); // to set 0
        assert_eq!(screen.selected_set, Some(0));
        screen.handle_key(key(KeyCode::Backspace), &db);
        assert_eq!(screen.sets.len(), 1);
        assert_eq!(screen.selected_set, Some(0));
    }

    #[test]
    fn sets_edit_works() {
        let (db, _dir) = test_db();
        db.create_practice("Bench", PracticeType::Weighted).unwrap();

        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);

        // Add a set: 50kg x 10 reps
        screen.handle_key(char_key('5'), &db);
        screen.handle_key(char_key('0'), &db);
        screen.handle_key(key(KeyCode::Tab), &db);
        screen.handle_key(char_key('1'), &db);
        screen.handle_key(char_key('0'), &db);
        screen.handle_key(key(KeyCode::Enter), &db);
        assert_eq!(screen.sets.len(), 1);
        assert!(matches!(
            screen.sets[0],
            SetData::Weighted {
                weight: 50.0,
                reps: 10
            }
        ));

        // Select the set and edit it
        screen.handle_key(char_key('k'), &db);
        assert_eq!(screen.selected_set, Some(0));
        screen.handle_key(char_key('e'), &db);
        assert_eq!(screen.editing_set, Some(0));
        assert_eq!(screen.field1, "50");
        assert_eq!(screen.field2, "10");
        assert_eq!(screen.active_field, 0);

        // Change weight to 55 (go to start, clear, then type)
        screen.handle_key(ctrl_key('a'), &db);
        screen.handle_key(ctrl_key('k'), &db);
        screen.handle_key(char_key('5'), &db);
        screen.handle_key(char_key('5'), &db);
        screen.handle_key(key(KeyCode::Enter), &db); // move to reps
        assert_eq!(screen.active_field, 1);
        screen.handle_key(key(KeyCode::Enter), &db); // save edit
        assert!(screen.editing_set.is_none());
        assert_eq!(screen.sets.len(), 1);
        assert!(matches!(
            screen.sets[0],
            SetData::Weighted {
                weight: 55.0,
                reps: 10
            }
        ));

        // selected_set stays at 0 after edit; press e again then cancel with Esc
        assert_eq!(screen.selected_set, Some(0));
        screen.handle_key(char_key('e'), &db);
        assert_eq!(screen.editing_set, Some(0));
        screen.handle_key(key(KeyCode::Esc), &db);
        assert!(screen.editing_set.is_none());
        assert!(screen.field1.is_empty());
    }
}
