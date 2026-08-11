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
    centered_area,
    framework::{
        fit_rect, move_selection, render_shortcuts, shortcut_height, truncate_to_width,
        EdgeBehavior, Shortcut, TextInput, ViewportState, VisualSpan,
    },
    highlight_row, render_status_line, Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH,
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
    filter: TextInput,
    filtering: bool,
    selected: usize,
    list_viewport: ViewportState,
    phase: Phase,
    chosen_practice: Option<Practice>,
    sets: Vec<SetData>,
    field1: TextInput,
    field2: TextInput,
    active_field: usize,
    note: TextInput,
    warm_up: TextInput,
    cool_down: TextInput,
    rpe_input: TextInput,
    focus: FocusSection,
    selected_set: Option<usize>,
    editing_set: Option<usize>,
    editing_log_id: Option<i64>,
    log_date: String,
    editing_date: bool,
    date_input: TextInput,
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
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            list_viewport: ViewportState::new(0),
            phase: Phase::SelectPractice,
            chosen_practice: None,
            sets: Vec::new(),
            field1: TextInput::new(),
            field2: TextInput::new(),
            active_field: 0,
            note: TextInput::new(),
            warm_up: TextInput::new(),
            cool_down: TextInput::new(),
            rpe_input: TextInput::new(),
            focus: FocusSection::Sets,
            selected_set: None,
            editing_set: None,
            editing_log_id: None,
            log_date: today,
            editing_date: false,
            date_input: TextInput::new(),
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
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            list_viewport: ViewportState::new(0),
            phase: Phase::EnterLog,
            chosen_practice: practice,
            sets,
            field1: TextInput::new(),
            field2: TextInput::new(),
            active_field: 0,
            note: TextInput::from(note),
            warm_up: TextInput::from(warm_up),
            cool_down: TextInput::from(cool_down),
            rpe_input: TextInput::from(rpe_input),
            focus: FocusSection::Sets,
            selected_set: None,
            editing_set: None,
            editing_log_id: Some(log_entry.log.id),
            log_date,
            editing_date: false,
            date_input: TextInput::new(),
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

    fn active_select_shortcuts(&self) -> Vec<Shortcut> {
        if self.filtering {
            vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ]
        } else {
            vec![
                Shortcut::new("j/k", tr("key-navigate")),
                Shortcut::new("/", tr("key-filter")),
                Shortcut::new("Enter", tr("key-select")),
                Shortcut::new("Esc", tr("key-back")),
            ]
        }
    }

    fn render_select_practice(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let shortcuts = self.active_select_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(shortcut_rows),
            ])
            .split(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(outer[0]);

        // Filter bar
        let filter_style = if self.filtering {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(Color::Gray)
        };
        let mut filter_spans = if self.filtering || !self.filter.value().is_empty() {
            vec![Span::styled(" /", filter_style)]
        } else {
            vec![Span::styled(
                format!(" {}", tr("log-press-filter")),
                filter_style,
            )]
        };
        if self.filtering {
            filter_spans.extend(self.filter.visible_spans(chunks[0].width, 2, ACCENT));
        } else if !self.filter.value().is_empty() {
            filter_spans.push(Span::styled(
                truncate_to_width(
                    self.filter.value(),
                    chunks[0].width.saturating_sub(2) as usize,
                ),
                filter_style,
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(filter_spans)).wrap(Wrap { trim: false }),
            chunks[0],
        );

        // Bordered list
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = fit_rect(chunks[1], block.inner(chunks[1]));
        let visible_rows = inner.height.saturating_sub(1) as usize; // -1 for header
        self.list_viewport.set_height(visible_rows);
        if !self.filtered_indices.is_empty() {
            self.list_viewport.reveal(VisualSpan {
                start: self.selected,
                height: 1,
            });
        }
        self.list_viewport.clamp(self.filtered_indices.len());
        let overflow = self
            .list_viewport
            .overflow(self.filtered_indices.len())
            .suffix();
        let block = block.title(Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled(
                format!("{}{}", tr("log-select-practice"), overflow),
                Style::default().fg(Color::White).bold(),
            ),
            Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
        ]));
        frame.render_widget(block, chunks[1]);

        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let desired_type_width = self
            .practices
            .iter()
            .map(|practice| practice.practice_type.label().width())
            .max()
            .unwrap_or(0)
            .max(type_header.width());
        let type_col_width = desired_type_width.min((inner.width as usize) / 3);
        let name_col_width = (inner.width as usize)
            .saturating_sub(2)
            .saturating_sub(2)
            .saturating_sub(type_col_width);
        let name_header = truncate_to_width(&name_header, name_col_width);
        let type_header = truncate_to_width(&type_header, type_col_width);
        let header_padding = name_col_width
            .saturating_sub(name_header.width())
            .saturating_add(2);

        let mut all_lines = vec![Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&name_header, Style::default().fg(Color::DarkGray)),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&type_header, Style::default().fg(Color::DarkGray)),
        ])];

        let scroll = self.list_viewport.offset();
        let end = scroll
            .saturating_add(visible_rows)
            .min(self.filtered_indices.len());

        for i in scroll..end {
            let idx = self.filtered_indices[i];
            let practice = &self.practices[idx];
            let marker = if i == self.selected { "> " } else { "  " };
            let name_style = if i == self.selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let name = truncate_to_width(&practice.name, name_col_width);
            let padding = name_col_width
                .saturating_sub(name.width())
                .saturating_add(2);
            let type_label = truncate_to_width(&practice.practice_type.label(), type_col_width);
            all_lines.push(Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(name, name_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(type_label, Style::default().fg(Color::Gray)),
            ]));
        }
        frame.render_widget(Paragraph::new(all_lines).wrap(Wrap { trim: false }), inner);

        if !self.filtered_indices.is_empty() && self.selected >= scroll && self.selected < end {
            highlight_row(
                frame,
                inner,
                ((self.selected - scroll) as u16).saturating_add(1),
            );
            // +1 for header
        }

        render_status_line(frame, outer[1], &self.status_msg);

        render_shortcuts(frame, outer[2], &shortcuts);
    }

    fn handle_select_practice(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.filtering = false;
                }
                _ if self.filter.handle_key(key) => {
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
                    self.selected = move_selection(
                        self.selected,
                        self.filtered_indices.len(),
                        1,
                        EdgeBehavior::Clamp,
                    );
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.filtered_indices.is_empty() {
                    self.selected = move_selection(
                        self.selected,
                        self.filtered_indices.len(),
                        -1,
                        EdgeBehavior::Clamp,
                    );
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    self.chosen_practice = Some(self.practices[idx].clone());
                    self.phase = Phase::EnterLog;
                    self.sets.clear();
                    self.field1.clear();
                    self.field2.clear();
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

    fn apply_filter(&mut self) {
        let lower = self.filter.value().to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.list_viewport.set_offset(0);
    }

    // ── Unified Log Entry Screen ─────────────────────────────────────

    fn active_enter_shortcuts(&self) -> Vec<Shortcut> {
        if self.confirming_exit {
            return vec![
                Shortcut::new("y/Enter", tr("key-yes")),
                Shortcut::new("n/Esc", tr("key-cancel")),
            ];
        }
        if self.editing_date {
            return vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ];
        }

        let mut shortcuts = vec![Shortcut::new("Tab", tr("key-next"))];
        if self.focus == FocusSection::Sets {
            let enter_label = if self.editing_set.is_some() {
                tr("key-confirm")
            } else if self.active_field == 0
                && self
                    .chosen_practice
                    .as_ref()
                    .is_some_and(|practice| practice.practice_type == PracticeType::Weighted)
            {
                tr("key-next")
            } else {
                tr("key-add-set")
            };
            shortcuts.push(Shortcut::new("Enter", enter_label));
            if self.editing_set.is_none() && !self.sets.is_empty() {
                shortcuts.push(Shortcut::new("j/k", tr("key-navigate")));
            }
            if self.selected_set.is_some() && self.editing_set.is_none() {
                shortcuts.push(Shortcut::new("d", tr("key-delete")));
                shortcuts.push(Shortcut::new("e", tr("key-edit")));
            }
        }
        if !self.sets.is_empty() && self.editing_set.is_none() {
            shortcuts.push(Shortcut::new("Ctrl+S", tr("key-save")));
        }
        if self.focus == FocusSection::Sets {
            shortcuts.push(Shortcut::new("D", tr("key-date")));
        }
        shortcuts.push(Shortcut::new("Esc", tr("key-cancel")));
        shortcuts
    }

    fn render_enter_log(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let practice = self.chosen_practice.as_ref().unwrap();
        let shortcuts = self.active_enter_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let sets_content_lines = u16::try_from(self.sets.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2)
            .max(2); // committed sets + input + total

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(shortcut_rows),
            ])
            .split(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // [0] date section (border + padding)
                Constraint::Length(1), // [1] spacer
                Constraint::Length(sets_content_lines.saturating_add(4)), // [2] sets section
                Constraint::Length(7), // [3] warm-up/cool-down section (border + padding)
                Constraint::Min(0),    // [4] note section (border, grows)
            ])
            .split(outer[0]);

        // ── Date section ──
        let date_border_color = BORDER_COLOR;
        let date_title = truncate_to_width(
            &format!(" {} ({}) ", practice.name, practice.practice_type.label()),
            chunks[0].width.saturating_sub(2) as usize,
        );
        let date_block = Block::default()
            .title(Span::styled(date_title, Style::default().fg(ACCENT).bold()))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(date_border_color));
        let date_inner = fit_rect(chunks[0], date_block.inner(chunks[0]));
        frame.render_widget(date_block, chunks[0]);

        let date_line = if self.editing_date {
            let label = truncate_to_width(
                &format!("{} ", tr("log-date-label")),
                date_inner.width as usize,
            );
            let prefix_width = label.width() as u16;
            let mut spans = vec![Span::styled(label, Style::default().fg(Color::Gray))];
            spans.extend(
                self.date_input
                    .visible_spans(date_inner.width, prefix_width, ACCENT),
            );
            Line::from(spans)
        } else {
            let label = truncate_to_width(
                &format!("{} ", tr("log-date-label")),
                date_inner.width as usize,
            );
            let date_width = date_inner.width.saturating_sub(label.width() as u16) as usize;
            let date = truncate_to_width(&self.log_date, date_width);
            let hint = if self.focus == FocusSection::Sets && !self.confirming_exit {
                let hint_width = date_inner
                    .width
                    .saturating_sub(label.width() as u16)
                    .saturating_sub(date.width() as u16) as usize;
                truncate_to_width(&format!("  {}", tr("log-date-change-hint")), hint_width)
            } else {
                String::new()
            };
            Line::from(vec![
                Span::styled(label, Style::default().fg(Color::Gray)),
                Span::styled(date, Style::default().fg(Color::White)),
                Span::styled(hint, Style::default().fg(Color::Gray)),
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
        let sets_inner = fit_rect(chunks[2], sets_block.inner(chunks[2]));
        frame.render_widget(sets_block, chunks[2]);

        let mut sets_lines: Vec<Line> = Vec::new();
        for (i, set) in self.sets.iter().enumerate() {
            let text = truncate_to_width(
                &tr_args(
                    "log-set-line",
                    &[
                        ("number", FluentValue::from((i + 1) as f64)),
                        ("data", FluentValue::from(format_set_data(set))),
                    ],
                ),
                sets_inner.width.saturating_sub(2) as usize,
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
                    let weight_label = format!("{} ", tr("log-weight-label"));
                    let reps_label = format!("  {} ", tr("log-reps-label"));
                    let fixed_width = input_label
                        .width()
                        .saturating_add(weight_label.width())
                        .saturating_add(reps_label.width())
                        .saturating_add(1); // active cursor
                    let values_width = (sets_inner.width as usize).saturating_sub(fixed_width);
                    let mut spans = vec![
                        Span::styled(input_label.clone(), Style::default().fg(Color::White)),
                        Span::styled(weight_label.clone(), Style::default().fg(Color::Gray)),
                    ];
                    if self.active_field == 0 {
                        let inactive_reps_width = self.field2.value().width().min(values_width / 2);
                        let active_weight_width = values_width.saturating_sub(inactive_reps_width);
                        let prefix_width = input_label.width().saturating_add(weight_label.width());
                        let active_max_width = prefix_width
                            .saturating_add(active_weight_width)
                            .saturating_add(1)
                            .min(sets_inner.width as usize);
                        spans.extend(self.field1.visible_spans(
                            active_max_width as u16,
                            prefix_width as u16,
                            ACCENT,
                        ));
                        spans.push(Span::styled(reps_label, Style::default().fg(Color::Gray)));
                        spans.push(Span::styled(
                            truncate_to_width(self.field2.value(), inactive_reps_width),
                            Style::default().fg(Color::White),
                        ));
                    } else {
                        let inactive_weight_width =
                            self.field1.value().width().min(values_width / 2);
                        let field1 = truncate_to_width(self.field1.value(), inactive_weight_width);
                        spans.push(Span::styled(field1, Style::default().fg(Color::White)));
                        spans.push(Span::styled(
                            reps_label.clone(),
                            Style::default().fg(Color::Gray),
                        ));
                        let prefix_width = (input_label.width()
                            + weight_label.width()
                            + spans[2].width()
                            + reps_label.width()) as u16;
                        spans.extend(self.field2.visible_spans(
                            sets_inner.width,
                            prefix_width,
                            ACCENT,
                        ));
                    }
                    sets_lines.push(Line::from(spans));
                }
                PracticeType::Bodyweight => {
                    let field_label = format!("{} ", tr("log-reps-label"));
                    let prefix_width = (input_label.width() + field_label.width()) as u16;
                    let mut spans = vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(field_label, Style::default().fg(Color::Gray)),
                    ];
                    spans.extend(
                        self.field1
                            .visible_spans(sets_inner.width, prefix_width, ACCENT),
                    );
                    sets_lines.push(Line::from(spans));
                }
                PracticeType::Distance => {
                    let field_label = format!("{} ", tr("log-distance-label"));
                    let prefix_width = (input_label.width() + field_label.width()) as u16;
                    let mut spans = vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(field_label, Style::default().fg(Color::Gray)),
                    ];
                    spans.extend(
                        self.field1
                            .visible_spans(sets_inner.width, prefix_width, ACCENT),
                    );
                    sets_lines.push(Line::from(spans));
                }
                PracticeType::Endurance => {
                    let field_label = format!("{} ", tr("log-duration-label"));
                    let prefix_width = (input_label.width() + field_label.width()) as u16;
                    let mut spans = vec![
                        Span::styled(input_label, Style::default().fg(Color::White)),
                        Span::styled(field_label, Style::default().fg(Color::Gray)),
                    ];
                    spans.extend(
                        self.field1
                            .visible_spans(sets_inner.width, prefix_width, ACCENT),
                    );
                    sets_lines.push(Line::from(spans));
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
            truncate_to_width(&total_text, sets_inner.width as usize),
            Style::default().fg(Color::DarkGray),
        )));

        frame.render_widget(Paragraph::new(sets_lines), sets_inner);

        // ── Warm-up / Cool-down section ──
        let wucd_border_color = BORDER_COLOR;
        let wucd_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(wucd_border_color));
        let wucd_inner = fit_rect(chunks[3], wucd_block.inner(chunks[3]));
        frame.render_widget(wucd_block, chunks[3]);

        let wu_active = self.focus == FocusSection::WarmUp;
        let wu_color = if wu_active { ACCENT } else { Color::White };
        let wu_label = format!("{}: ", tr("log-warmup-label"));
        let wu_prefix_w = wu_label.width() as u16;
        let mut wu_spans = vec![Span::styled(&wu_label, Style::default().fg(Color::Gray))];
        if wu_active {
            wu_spans.extend(
                self.warm_up
                    .visible_spans(wucd_inner.width, wu_prefix_w, ACCENT),
            );
        } else {
            wu_spans.push(Span::styled(
                truncate_to_width(
                    self.warm_up.value(),
                    wucd_inner.width.saturating_sub(wu_prefix_w) as usize,
                ),
                Style::default().fg(wu_color),
            ));
        }

        let cd_active = self.focus == FocusSection::CoolDown;
        let cd_color = if cd_active { ACCENT } else { Color::White };
        let cd_label = format!("{}: ", tr("log-cooldown-label"));
        let cd_prefix_w = cd_label.width() as u16;
        let mut cd_spans = vec![Span::styled(&cd_label, Style::default().fg(Color::Gray))];
        if cd_active {
            cd_spans.extend(
                self.cool_down
                    .visible_spans(wucd_inner.width, cd_prefix_w, ACCENT),
            );
        } else {
            cd_spans.push(Span::styled(
                truncate_to_width(
                    self.cool_down.value(),
                    wucd_inner.width.saturating_sub(cd_prefix_w) as usize,
                ),
                Style::default().fg(cd_color),
            ));
        }

        let rpe_active = self.focus == FocusSection::Rpe;
        let rpe_color = if rpe_active { ACCENT } else { Color::White };
        let rpe_label = "RPE (1-10): ";
        let rpe_prefix_w = rpe_label.width() as u16;
        let mut rpe_spans = vec![Span::styled(rpe_label, Style::default().fg(Color::Gray))];
        if rpe_active {
            rpe_spans.extend(
                self.rpe_input
                    .visible_spans(wucd_inner.width, rpe_prefix_w, ACCENT),
            );
        } else {
            rpe_spans.push(Span::styled(
                self.rpe_input.value(),
                Style::default().fg(rpe_color),
            ));
        }

        let wucd_lines = vec![
            Line::from(wu_spans),
            Line::from(cd_spans),
            Line::from(rpe_spans),
        ];
        frame.render_widget(
            Paragraph::new(wucd_lines).wrap(Wrap { trim: false }),
            wucd_inner,
        );

        // ── Note section ──
        let note_border_color = BORDER_COLOR;
        let note_block = Block::default()
            .title(Span::styled(
                format!(" {} ", tr("log-note-optional")),
                Style::default().fg(Color::Gray),
            ))
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .border_style(Style::default().fg(note_border_color));
        let note_inner = fit_rect(chunks[4], note_block.inner(chunks[4]));
        frame.render_widget(note_block, chunks[4]);

        if self.focus == FocusSection::Note {
            let note_line = Line::from(self.note.visible_spans(note_inner.width, 0, ACCENT));
            frame.render_widget(
                Paragraph::new(note_line).wrap(Wrap { trim: false }),
                note_inner,
            );
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    self.note.value(),
                    Style::default().fg(Color::White),
                ))
                .wrap(Wrap { trim: false }),
                note_inner,
            );
        }

        // ── Status line ──
        if self.confirming_exit {
            render_status_line(frame, outer[1], &Some((tr("log-cancel-confirm"), true)));
        } else {
            render_status_line(frame, outer[1], &self.status_msg);
        }

        // ── Footer ──
        render_shortcuts(frame, outer[2], &shortcuts);
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
            if !self.sets.is_empty() && self.editing_set.is_none() {
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
            self.date_input.set(self.log_date.clone());
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
                    self.rpe_input.set(self.rpe_input.value().to_owned());
                }
                FocusSection::Rpe => {
                    self.focus = FocusSection::Note;
                    self.note.set(self.note.value().to_owned());
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
                        self.note.set(self.note.value().to_owned());
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
                    self.rpe_input.set(self.rpe_input.value().to_owned());
                }
            }
        }
    }

    fn cancel_edit(&mut self) {
        self.editing_set = None;
        self.field1.clear();
        self.field2.clear();
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
                let active_input_empty = if has_two_fields && self.active_field == 1 {
                    self.field2.value().is_empty()
                } else {
                    self.field1.value().is_empty()
                };
                let both_fields_empty = self.field1.value().is_empty()
                    && (self.field2.value().is_empty() || !has_two_fields);
                if active_input_empty && !self.sets.is_empty() {
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
                } else {
                    self.active_set_input_mut(has_two_fields)
                        .handle_key_if(key, |character| {
                            character.is_ascii_digit() || character == '.'
                        });
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
                            self.field1
                                .set(format!("{:.1}", weight).trim_end_matches(".0"));
                            self.field2.set(reps.to_string());
                        }
                        SetData::Bodyweight { reps } => {
                            self.field1.set(reps.to_string());
                        }
                        SetData::Distance { distance } => {
                            self.field1
                                .set(format!("{:.1}", distance).trim_end_matches(".0"));
                        }
                        SetData::Endurance { duration } => {
                            self.field1
                                .set(format!("{:.1}", duration).trim_end_matches(".0"));
                        }
                    }
                    self.active_field = 0;
                }
                Action::None
            }
            _ => {
                self.active_set_input_mut(has_two_fields)
                    .handle_key_if(key, |character| {
                        character.is_ascii_digit() || character == '.'
                    });
                Action::None
            }
        }
    }

    fn active_set_input_mut(&mut self, has_two_fields: bool) -> &mut TextInput {
        if has_two_fields && self.active_field == 1 {
            &mut self.field2
        } else {
            &mut self.field1
        }
    }

    fn handle_text_field_input(&mut self, key: KeyEvent, target: TextFieldTarget) -> Action {
        match target {
            TextFieldTarget::WarmUp => {
                self.warm_up.handle_key(key);
            }
            TextFieldTarget::CoolDown => {
                self.cool_down.handle_key(key);
            }
            TextFieldTarget::Rpe => {
                let accepts_more = self.rpe_input.value().len() < 2;
                self.rpe_input
                    .handle_key_if(key, |character| accepts_more && character.is_ascii_digit());
            }
            TextFieldTarget::Note => {
                self.note.handle_key(key);
            }
        }
        Action::None
    }

    fn handle_date_edit(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                match NaiveDate::parse_from_str(self.date_input.value(), "%Y-%m-%d") {
                    Ok(_) => {
                        self.log_date = self.date_input.value().to_owned();
                        self.date_input.clear();
                        self.editing_date = false;
                    }
                    Err(_) => {
                        self.status_msg = Some((tr("log-invalid-date"), true));
                    }
                }
            }
            KeyCode::Esc => {
                self.editing_date = false;
            }
            _ => {
                self.date_input.handle_key_if(key, |character| {
                    character.is_ascii_digit() || character == '-'
                });
            }
        }
        Action::None
    }

    fn commit_set(&mut self) {
        let practice = self.chosen_practice.as_ref().unwrap();
        let set_data = match practice.practice_type {
            PracticeType::Weighted => {
                let weight = if self.field1.value().is_empty() {
                    self.sets
                        .last()
                        .and_then(|s| match s {
                            SetData::Weighted { weight, .. } => Some(*weight),
                            _ => None,
                        })
                        .unwrap_or(0.0)
                } else {
                    self.field1.value().parse::<f64>().unwrap_or(0.0)
                };
                let reps = self.field2.value().parse::<i32>().unwrap_or(0);
                if reps == 0 {
                    return;
                }
                Some(SetData::Weighted { weight, reps })
            }
            PracticeType::Bodyweight => {
                let reps = self.field1.value().parse::<i32>().unwrap_or(0);
                if reps == 0 {
                    return;
                }
                Some(SetData::Bodyweight { reps })
            }
            PracticeType::Distance => {
                let distance = self.field1.value().parse::<f64>().unwrap_or(0.0);
                if distance == 0.0 {
                    return;
                }
                Some(SetData::Distance { distance })
            }
            PracticeType::Endurance => {
                let duration = self.field1.value().parse::<f64>().unwrap_or(0.0);
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
                self.active_field = 1;
            } else {
                self.field1.clear();
                self.active_field = 0;
            }
        }
    }

    fn save_log(&mut self, db: &Database) -> Action {
        let practice = self.chosen_practice.as_ref().unwrap();

        let rpe = if self.rpe_input.value().is_empty() {
            None
        } else {
            match self.rpe_input.value().parse::<u8>() {
                Ok(val) if (1..=10).contains(&val) => Some(val),
                _ => {
                    self.status_msg =
                        Some(("RPE must be a number between 1 and 10".to_string(), true));
                    return Action::None;
                }
            }
        };

        let note = if self.note.value().is_empty() {
            None
        } else {
            Some(self.note.value())
        };
        let warm_up = if self.warm_up.value().is_empty() {
            None
        } else {
            Some(self.warm_up.value())
        };
        let cool_down = if self.cool_down.value().is_empty() {
            None
        } else {
            Some(self.cool_down.value())
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
            db.create_log_at(
                practice.id,
                &datetime,
                &self.sets,
                note,
                warm_up,
                cool_down,
                rpe,
            )
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
    use crate::tui::framework::test_support::render_to_string;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use rusqlite::Connection;

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

    fn render_log_entry(screen: &mut LogEntryScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    #[test]
    fn long_filter_and_last_practice_remain_visible() {
        let db = Database::open_in_memory().unwrap();
        for number in 1..=40 {
            db.create_practice(&format!("Practice {number:02}"), PracticeType::Weighted)
                .unwrap();
        }
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.filtering = true;
        screen.filter = TextInput::from("练习".repeat(60));
        let filtered = render_log_entry(&mut screen, 80, 24);
        assert!(filtered.contains('_'), "{filtered}");

        screen.filter.clear();
        screen.apply_filter();
        screen.filtering = false;
        for _ in 1..40 {
            screen.handle_key(key(KeyCode::Down), &db);
        }
        let last = render_log_entry(&mut screen, 80, 24);
        assert!(last.contains("Practice 40"), "{last}");
        assert!(last.contains('↑'), "{last}");
    }

    #[test]
    fn every_long_log_input_keeps_its_cursor_visible() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Deadlift", PracticeType::Weighted)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        let long = "练习".repeat(80);
        for focus in [
            FocusSection::Sets,
            FocusSection::WarmUp,
            FocusSection::CoolDown,
            FocusSection::Rpe,
            FocusSection::Note,
        ] {
            screen.focus = focus;
            match focus {
                FocusSection::Sets => screen.field1.set(&long),
                FocusSection::WarmUp => screen.warm_up.set(&long),
                FocusSection::CoolDown => screen.cool_down.set(&long),
                FocusSection::Rpe => screen.rpe_input.set("99"),
                FocusSection::Note => screen.note.set(&long),
            }
            let output = render_log_entry(&mut screen, 80, 24);
            assert!(output.contains('_'), "focus={focus:?}\n{output}");
        }
    }

    #[test]
    fn invalid_date_keeps_date_editor_and_input() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Deadlift", PracticeType::Weighted)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.editing_date = true;
        screen.date_input = TextInput::from("2026-99-99");

        screen.handle_key(key(KeyCode::Enter), &db);

        assert!(screen.editing_date);
        assert_eq!(screen.date_input.value(), "2026-99-99");
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn invalid_rpe_retains_complete_form_and_focus() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.sets.push(SetData::Bodyweight { reps: 10 });
        screen.note.set("keep me");
        screen.rpe_input.set("99");
        screen.focus = FocusSection::Rpe;

        let action = screen.save_log(&db);

        assert!(matches!(action, Action::None));
        assert_eq!(screen.sets, vec![SetData::Bodyweight { reps: 10 }]);
        assert_eq!(screen.note.value(), "keep me");
        assert_eq!(screen.rpe_input.value(), "99");
        assert_eq!(screen.focus, FocusSection::Rpe);
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn log_entry_renders_small_rectangle_without_panicking() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        let output = render_log_entry(&mut screen, 24, 10);
        assert!(!output.is_empty());
    }

    #[test]
    fn log_entry_shortcuts_include_every_active_command_at_supported_width() {
        let db = Database::open_in_memory().unwrap();
        db.create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        let output = render_log_entry(&mut screen, 80, 24);
        for command in ["[j/k]", "[/]", "[Enter]", "[Esc]"] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
    }

    #[test]
    fn database_write_failure_retains_edit_context_and_complete_form() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locked.db");
        let db = Database::open(&path).unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 8 }],
            Some("original"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let entry = db.list_logs_all().unwrap().remove(0);
        let mut screen = LogEntryScreen::from_existing(&db, &entry).unwrap();
        screen.sets = vec![SetData::Bodyweight { reps: 10 }];
        screen.note.set("keep me");
        screen.warm_up.set("bands");
        screen.cool_down.set("stretch");
        screen.rpe_input.set("8");
        screen.focus = FocusSection::Note;

        let lock = Connection::open(&path).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

        let action = screen.save_log(&db);

        assert!(matches!(action, Action::None));
        assert_eq!(screen.note.value(), "keep me");
        assert_eq!(screen.warm_up.value(), "bands");
        assert_eq!(screen.cool_down.value(), "stretch");
        assert_eq!(screen.rpe_input.value(), "8");
        assert_eq!(screen.sets, vec![SetData::Bodyweight { reps: 10 }]);
        assert_eq!(screen.editing_log_id, Some(entry.log.id));
        assert_eq!(screen.return_to, Screen::History);
        assert_eq!(screen.focus, FocusSection::Note);
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));

        lock.execute_batch("ROLLBACK").unwrap();
    }

    #[test]
    fn selected_set_shortcuts_include_every_active_command_at_supported_width() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.sets.push(SetData::Bodyweight { reps: 10 });
        screen.selected_set = Some(0);

        let output = render_log_entry(&mut screen, 80, 24);

        for command in [
            "[Tab]", "[Enter]", "[j/k]", "[d]", "[e]", "[Ctrl+S]", "[D]", "[Esc]",
        ] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
    }

    #[test]
    fn set_edit_shortcuts_replace_add_and_navigation_with_confirm_cancel() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.sets.push(SetData::Bodyweight { reps: 10 });
        screen.selected_set = Some(0);
        screen.editing_set = Some(0);
        screen.field1.set("12");

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(
            output.contains(&format!("[Enter] {}", tr("key-confirm"))),
            "{output}"
        );
        assert!(output.contains("[Esc]"), "{output}");
        for inactive in ["[j/k]", "[d]", "[e]", "[Ctrl+S]"] {
            assert!(
                !output.contains(inactive),
                "unexpected {inactive}\n{output}"
            );
        }
        assert!(!output.contains(&tr("key-add-set")), "{output}");
    }

    #[test]
    fn ctrl_s_during_set_edit_preserves_pending_state_without_writing() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Deadlift", PracticeType::Weighted)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Weighted {
                weight: 50.0,
                reps: 10,
            }],
            Some("original note"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let entry = db.list_logs_all().unwrap().remove(0);
        let mut screen = LogEntryScreen::from_existing(&db, &entry).unwrap();
        screen.selected_set = Some(0);
        screen.editing_set = Some(0);
        screen.field1.set("55");
        screen.field2.set("12");
        screen.active_field = 1;
        screen.note.set("pending note");
        screen.focus = FocusSection::Sets;

        let action = screen.handle_key(ctrl_key('s'), &db);
        let persisted = db.list_logs_all().unwrap().remove(0);

        assert_eq!(persisted.log.note.as_deref(), Some("original note"));
        assert_eq!(
            persisted.sets[0].data,
            SetData::Weighted {
                weight: 50.0,
                reps: 10,
            }
        );
        assert!(matches!(action, Action::None));
        assert_eq!(screen.phase, Phase::EnterLog);
        assert_eq!(screen.selected_set, Some(0));
        assert_eq!(screen.editing_set, Some(0));
        assert_eq!(screen.editing_log_id, Some(entry.log.id));
        assert_eq!(screen.field1.value(), "55");
        assert_eq!(screen.field2.value(), "12");
        assert_eq!(screen.active_field, 1);
        assert_eq!(screen.note.value(), "pending note");
        assert_eq!(screen.focus, FocusSection::Sets);
        assert_eq!(screen.return_to, Screen::History);
    }

    #[test]
    fn date_edit_shortcuts_show_only_confirm_and_cancel() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.editing_date = true;
        screen.date_input.set("2026-08-10");

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(output.contains("[Enter]"), "{output}");
        assert!(output.contains("[Esc]"), "{output}");
        for inactive in ["[Tab]", "[Ctrl+S]", "[D]"] {
            assert!(
                !output.contains(inactive),
                "unexpected {inactive}\n{output}"
            );
        }
    }

    #[test]
    fn exit_confirmation_shortcuts_show_only_confirm_and_cancel() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.confirming_exit = true;

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(output.contains(&tr("log-cancel-confirm")), "{output}");
        assert!(output.contains("[y/Enter]"), "{output}");
        assert!(output.contains("[n/Esc]"), "{output}");
        for inactive in ["[Tab]", "[Ctrl+S]", "[D]"] {
            assert!(
                !output.contains(inactive),
                "unexpected {inactive}\n{output}"
            );
        }
    }

    #[test]
    fn non_sets_focus_hides_inactive_date_hint() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.focus = FocusSection::Note;

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(!output.contains("[D]"), "{output}");
    }

    #[test]
    fn empty_log_hides_inactive_save_shortcut() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(!output.contains("[Ctrl+S]"), "{output}");
    }

    #[test]
    fn weighted_set_enter_shortcut_matches_immediate_action() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Deadlift", PracticeType::Weighted)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);

        let weight_output = render_log_entry(&mut screen, 80, 24);
        assert!(
            weight_output.contains(&format!("[Enter] {}", tr("key-next"))),
            "{weight_output}"
        );
        assert!(
            !weight_output.contains(&tr("key-add-set")),
            "{weight_output}"
        );

        screen.active_field = 1;
        let reps_output = render_log_entry(&mut screen, 80, 24);
        assert!(
            reps_output.contains(&format!("[Enter] {}", tr("key-add-set"))),
            "{reps_output}"
        );
    }

    #[test]
    fn active_weight_budgets_space_for_bounded_inactive_reps() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Deadlift", PracticeType::Weighted)
            .unwrap();
        let mut screen = LogEntryScreen::new(&db).unwrap();
        screen.phase = Phase::EnterLog;
        screen.chosen_practice = Some(practice);
        screen.active_field = 0;
        screen.field1.set("1234567890".repeat(10));
        screen.field2.set("9876543210".repeat(10));

        let output = render_log_entry(&mut screen, 80, 24);

        assert!(output.contains('_'), "{output}");
        assert!(
            output.contains(&format!("{} 987", tr("log-reps-label"))),
            "{output}"
        );
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
        assert_eq!(screen.field1.value(), "50");
        assert_eq!(screen.field1.cursor(), 2);

        // Tab to reps field
        screen.handle_key(key(KeyCode::Tab), &db);
        assert_eq!(screen.active_field, 1);

        // Type reps
        screen.handle_key(char_key('1'), &db);
        screen.handle_key(char_key('0'), &db);
        assert_eq!(screen.field2.value(), "10");
        assert_eq!(screen.field2.cursor(), 2);

        // Left moves cursor back
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2.cursor(), 1);
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2.cursor(), 0);
        screen.handle_key(key(KeyCode::Left), &db);
        assert_eq!(screen.field2.cursor(), 0); // clamped

        // Right moves cursor forward
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2.cursor(), 1);
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2.cursor(), 2);
        screen.handle_key(key(KeyCode::Right), &db);
        assert_eq!(screen.field2.cursor(), 2); // clamped

        // Ctrl+A to start
        screen.handle_key(ctrl_key('a'), &db);
        assert_eq!(screen.field2.cursor(), 0);

        // Ctrl+E to end
        screen.handle_key(ctrl_key('e'), &db);
        assert_eq!(screen.field2.cursor(), 2);
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
        assert_eq!(screen.field1.value(), "50");
        assert_eq!(screen.field2.value(), "10");
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
        assert!(screen.field1.value().is_empty());
    }
}
