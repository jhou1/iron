use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use std::sync::mpsc::{self, Receiver};

use crate::config::LlmConfig;
use crate::db::Database;
use crate::i18n::tr;
use crate::llm::{self, LlmError};
use crate::model::{Abbreviation, ParsedLog, Practice, SetData};
use super::{centered_area, render_help_overlay, render_status_line, visible_input_spans, Action, Screen, StatusMessage, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;
const YELLOW: Color = Color::Yellow;

const SPINNER: &[char] = &['\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}'];

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Input,
    Parsing,
    Preview,
}

pub struct QuickLogScreen {
    input_lines: Vec<String>,
    current_line: usize,
    cursor_pos: usize,
    llm_config: Option<LlmConfig>,
    practices: Vec<Practice>,
    abbreviations: Vec<Abbreviation>,
    parsed_results: Vec<ParsedLog>,
    result_receiver: Option<Receiver<Result<Vec<ParsedLog>, LlmError>>>,
    selected_result: usize,
    scroll_offset: usize,
    phase: Phase,
    status_msg: StatusMessage,
    show_help: bool,
    log_date: String,
    spinner_frame: usize,
}

impl QuickLogScreen {
    pub fn new(db: &Database, config: &Option<LlmConfig>) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let abbreviations = db.list_abbreviations()?;
        let log_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(Self {
            input_lines: vec![String::new()],
            current_line: 0,
            cursor_pos: 0,
            llm_config: config.clone(),
            practices,
            abbreviations,
            parsed_results: Vec::new(),
            result_receiver: None,
            selected_result: 0,
            scroll_offset: 0,
            phase: Phase::Input,
            status_msg: None,
            show_help: false,
            log_date,
            spinner_frame: 0,
        })
    }

    pub fn check_background_result(&mut self) {
        if self.phase != Phase::Parsing {
            return;
        }

        self.spinner_frame = (self.spinner_frame + 1) % SPINNER.len();

        if let Some(ref receiver) = self.result_receiver {
            match receiver.try_recv() {
                Ok(Ok(results)) => {
                    self.parsed_results = results;
                    self.selected_result = 0;
                    self.scroll_offset = 0;
                    self.phase = Phase::Preview;
                    self.result_receiver = None;
                }
                Ok(Err(e)) => {
                    self.status_msg = Some((format!("Error: {}", e), true));
                    self.phase = Phase::Input;
                    self.result_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_msg = Some(("Error: LLM thread disconnected".to_string(), true));
                    self.phase = Phase::Input;
                    self.result_receiver = None;
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // title
                Constraint::Min(6),    // two panes
                Constraint::Length(1), // status message
                Constraint::Length(1), // shortcuts
            ])
            .split(area);

        // ── Title ──
        let title = Line::from(Span::styled(
            tr("quicklog-title"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // ── Two panes side by side ──
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        self.render_input_pane(frame, panes[0]);
        self.render_preview_pane(frame, panes[1]);

        // ── Status line ──
        render_status_line(frame, chunks[2], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = self.build_shortcuts();
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[3]);

        // ── Help overlay ──
        if self.show_help {
            let bindings = match self.phase {
                Phase::Input => vec![
                    ("Ctrl+S", "Parse with LLM"),
                    ("Enter", "New line"),
                    ("Up/Down", "Move between lines"),
                    ("?", "Help"),
                    ("Esc", "Back"),
                ],
                Phase::Parsing => vec![
                    ("Esc", "Cancel"),
                ],
                Phase::Preview => vec![
                    ("j/k", "Navigate"),
                    ("d", "Remove entry"),
                    ("Enter/Ctrl+S", "Save all"),
                    ("a", "Abbreviations"),
                    ("Esc", "Back to edit"),
                    ("?", "Help"),
                ],
            };
            let bindings_refs: Vec<(&str, &str)> = bindings.iter().map(|(a, b)| (*a, *b)).collect();
            render_help_overlay(frame, area, &bindings_refs);
        }
    }

    fn render_input_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-input-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text_style = if self.phase == Phase::Input {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut lines: Vec<Line> = Vec::new();
        for (i, line_text) in self.input_lines.iter().enumerate() {
            if self.phase == Phase::Input && i == self.current_line {
                let spans = visible_input_spans(line_text, self.cursor_pos, inner.width, 0, Color::White);
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(Span::styled(line_text.as_str(), text_style)));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "\u{258F}",
                Style::default().fg(GREEN),
            )));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-preview-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.phase {
            Phase::Input => {
                if self.llm_config.is_none() {
                    let msg = Line::from(Span::styled(
                        tr("quicklog-no-config"),
                        Style::default().fg(YELLOW),
                    ));
                    frame.render_widget(Paragraph::new(vec![msg]), inner);
                }
            }
            Phase::Parsing => {
                let spinner_char = SPINNER[self.spinner_frame];
                let msg = Line::from(vec![
                    Span::styled(
                        format!("{} ", spinner_char),
                        Style::default().fg(ACCENT),
                    ),
                    Span::styled(
                        tr("quicklog-parsing"),
                        Style::default().fg(Color::White),
                    ),
                ]);
                frame.render_widget(Paragraph::new(vec![msg]), inner);
            }
            Phase::Preview => {
                self.render_preview_results(frame, inner);
            }
        }
    }

    fn render_preview_results(&self, frame: &mut Frame, area: Rect) {
        if self.parsed_results.is_empty() {
            let msg = Line::from(Span::styled(
                tr("quicklog-no-results"),
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(vec![msg]), area);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let has_unmatched = self.parsed_results.iter().any(|r| !r.matched);

        for (i, result) in self.parsed_results.iter().enumerate() {
            let is_selected = i == self.selected_result;
            let marker = if result.matched { "\u{2713}" } else { "\u{2717}" };
            let marker_color = if result.matched { GREEN } else { RED };
            let name_color = if result.matched {
                if is_selected { GREEN } else { Color::White }
            } else {
                YELLOW
            };
            let name_style = if is_selected {
                Style::default().fg(name_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(name_color)
            };

            let prefix = if is_selected { "> " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(prefix, name_style),
                Span::styled(format!("{} ", marker), Style::default().fg(marker_color)),
                Span::styled(&result.practice_name, name_style),
            ]));

            // Show sets
            for (j, set_data) in result.sets.iter().enumerate() {
                let set_text = format_set(j + 1, set_data);
                let set_style = if is_selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(
                    format!("    {}", set_text),
                    set_style,
                )));
            }
        }

        if has_unmatched {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                tr("quicklog-unmatched-warning"),
                Style::default().fg(YELLOW),
            )));
        }

        // Apply scroll offset
        let visible_height = area.height as usize;
        let display_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .collect();

        frame.render_widget(Paragraph::new(display_lines).wrap(Wrap { trim: false }), area);
    }

    fn build_shortcuts(&self) -> Line<'static> {
        match self.phase {
            Phase::Input => {
                let mut spans = vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        format!("{}: ", tr("quicklog-date")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        self.log_date.clone(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  ", Style::default()),
                    Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("quicklog-key-parse")),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled("[?]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("key-help")),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}", tr("key-back")),
                        Style::default().fg(Color::Gray),
                    ),
                ];
                if self.llm_config.is_none() {
                    spans.clear();
                    spans.push(Span::styled(" ", Style::default()));
                    spans.push(Span::styled("[?]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(
                        format!(" {}  ", tr("key-help")),
                        Style::default().fg(Color::Gray),
                    ));
                    spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(
                        format!(" {}", tr("key-back")),
                        Style::default().fg(Color::Gray),
                    ));
                }
                Line::from(spans)
            }
            Phase::Parsing => Line::from(vec![
                Span::styled(" [Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Phase::Preview => Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{}: ", tr("quicklog-date")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    self.log_date.clone(),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  ", Style::default()),
                Span::styled("[j/k]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-navigate")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-remove")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-save")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-abbr")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("quicklog-key-edit")),
                    Style::default().fg(Color::Gray),
                ),
            ]),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match self.phase {
            Phase::Input => self.handle_input(key),
            Phase::Parsing => self.handle_parsing(key),
            Phase::Preview => self.handle_preview(key, db),
        }
    }

    fn handle_input(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.start_llm_parse();
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                    Action::None
                } else {
                    Action::Navigate(Screen::Dashboard)
                }
            }
            KeyCode::Enter => {
                // Insert new line
                let remainder = self.input_lines[self.current_line][self.cursor_pos..].to_string();
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                self.current_line += 1;
                self.input_lines.insert(self.current_line, remainder);
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Up => {
                if self.current_line > 0 {
                    self.current_line -= 1;
                    self.cursor_pos = self.cursor_pos.min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Down => {
                if self.current_line < self.input_lines.len() - 1 {
                    self.current_line += 1;
                    self.cursor_pos = self.cursor_pos.min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_lines[self.current_line].remove(prev);
                    self.cursor_pos = prev;
                } else if self.current_line > 0 {
                    // Merge with previous line
                    let current_text = self.input_lines.remove(self.current_line);
                    self.current_line -= 1;
                    self.cursor_pos = self.input_lines[self.current_line].len();
                    self.input_lines[self.current_line].push_str(&current_text);
                }
                Action::None
            }
            // Emacs-style cursor navigation
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor_pos = prev;
                }
                Action::None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor_pos = prev;
                }
                Action::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let line = &self.input_lines[self.current_line];
                if self.cursor_pos < line.len() {
                    let next = line[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(line.len());
                    self.cursor_pos = next;
                }
                Action::None
            }
            KeyCode::Right => {
                let line = &self.input_lines[self.current_line];
                if self.cursor_pos < line.len() {
                    let next = line[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(line.len());
                    self.cursor_pos = next;
                }
                Action::None
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::End => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                Action::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_parsing(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            self.result_receiver = None;
            self.phase = Phase::Input;
        }
        Action::None
    }

    fn handle_preview(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = (self.selected_result + 1) % self.parsed_results.len();
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = self
                        .selected_result
                        .checked_sub(1)
                        .unwrap_or(self.parsed_results.len() - 1);
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.parsed_results.is_empty() {
                    self.parsed_results.remove(self.selected_result);
                    if self.selected_result >= self.parsed_results.len()
                        && !self.parsed_results.is_empty()
                    {
                        self.selected_result = self.parsed_results.len() - 1;
                    }
                    if self.parsed_results.is_empty() {
                        self.selected_result = 0;
                    }
                }
                Action::None
            }
            KeyCode::Enter | KeyCode::Char('s')
                if key.code == KeyCode::Enter
                    || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.save_all(db)
            }
            KeyCode::Char('a') => Action::Navigate(Screen::Abbreviations),
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.phase = Phase::Input;
                    self.parsed_results.clear();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn start_llm_parse(&mut self) {
        let config = match &self.llm_config {
            Some(c) => c.clone(),
            None => {
                self.status_msg = Some((tr("quicklog-no-config"), true));
                return;
            }
        };

        let input_text: String = self.input_lines.join("\n");
        let trimmed = input_text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let practices = self.practices.clone();
        let abbreviations = self.abbreviations.clone();

        let (tx, rx) = mpsc::channel();
        self.result_receiver = Some(rx);
        self.phase = Phase::Parsing;
        self.spinner_frame = 0;

        std::thread::spawn(move || {
            let system_prompt = llm::build_system_prompt(&practices, &abbreviations);
            let result = llm::call_llm(&config, &system_prompt, &trimmed)
                .and_then(|raw| llm::parse_llm_response(&raw, &practices));
            let _ = tx.send(result);
        });
    }

    fn save_all(&mut self, db: &Database) -> Action {
        if self.parsed_results.is_empty() {
            return Action::None;
        }

        // Check all matched
        if self.parsed_results.iter().any(|r| !r.matched) {
            self.status_msg = Some((tr("quicklog-unmatched-error"), true));
            return Action::None;
        }

        let raw_text = self.input_lines.join("\n");

        // Parse date
        let date = match chrono::NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d") {
            Ok(d) => d.and_hms_opt(12, 0, 0).unwrap(),
            Err(_) => {
                self.status_msg = Some(("Invalid date".to_string(), true));
                return Action::None;
            }
        };

        for entry in &self.parsed_results {
            // Find practice by name (case-insensitive match)
            let practice = match self.practices.iter().find(|p| {
                p.name.eq_ignore_ascii_case(&entry.practice_name)
            }) {
                Some(p) => p,
                None => {
                    self.status_msg = Some((format!("Practice not found: {}", entry.practice_name), true));
                    return Action::None;
                }
            };

            if let Err(e) = db.create_log_at(
                practice.id,
                &date,
                &entry.sets,
                Some(&raw_text),
                None,
                None,
            ) {
                self.status_msg = Some((format!("Error: {}", e), true));
                return Action::None;
            }
        }

        Action::Navigate(Screen::Dashboard)
    }

    fn adjust_scroll(&mut self) {
        // Rough estimate: each result takes 1 + sets.len() lines
        let mut line_idx = 0;
        for (i, result) in self.parsed_results.iter().enumerate() {
            if i == self.selected_result {
                break;
            }
            line_idx += 1 + result.sets.len();
        }
        if line_idx < self.scroll_offset {
            self.scroll_offset = line_idx;
        }
        // We don't know exact visible height here, so just ensure selected is reachable
    }
}

fn format_set(number: usize, data: &SetData) -> String {
    match data {
        SetData::Weighted { weight, reps } => {
            format!("Set {}: {}kg x {}", number, weight, reps)
        }
        SetData::Bodyweight { reps } => {
            format!("Set {}: {} reps", number, reps)
        }
        SetData::Distance { distance } => {
            format!("Set {}: {} km", number, distance)
        }
        SetData::Endurance { duration } => {
            format!("Set {}: {} min", number, duration)
        }
    }
}
