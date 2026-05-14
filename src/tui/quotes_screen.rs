use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::Quote;
use crate::tui::quotes::pick_random_quote;
use fluent_bundle::FluentValue;
use super::{centered_area, highlight_row, render_status_line, visible_input_spans, Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    Add,
    Edit,
    ConfirmDelete,
}

pub struct QuotesScreen {
    quotes: Vec<Quote>,
    selected: usize,
    scroll_offset: usize,
    mode: Mode,
    input: String,
    input_cursor: usize,
    editing_id: Option<i64>,
    status_msg: StatusMessage,
    last_deleted: Option<Quote>,
    weekly_volume: f64,
    training_days: usize,
    consecutive_days: i64,
    hrv: Option<i32>,
    featured_quote: String,
}

impl QuotesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let quotes = db.list_quotes()?;
        let stats = db.aggregate_stats(7).unwrap_or(crate::db::AggregateStats {
            sessions: 0, total_volume: 0.0, total_reps: 0.0, total_distance: 0.0, total_duration: 0.0,
        });
        let training_days = db.heatmap_counts(7).unwrap_or_default().len();

        let heatmap_90 = db.heatmap_counts(90).unwrap_or_default();
        let training_dates: std::collections::HashSet<String> =
            heatmap_90.iter().map(|(d, _)| d.clone()).collect();
        let mut consecutive_days = 0i64;
        let mut check = chrono::Local::now().date_naive();
        while training_dates.contains(&check.format("%Y-%m-%d").to_string()) {
            consecutive_days += 1;
            check -= chrono::Duration::days(1);
        }

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let hrv = db.get_daily_hrv(&today).unwrap_or(None);
        let featured_quote = pick_random_quote(&quotes);

        Ok(Self {
            weekly_volume: stats.total_volume,
            training_days,
            consecutive_days,
            hrv,
            featured_quote,
            quotes,
            selected: 0,
            scroll_offset: 0,
            mode: Mode::Browse,
            input: String::new(),
            input_cursor: 0,
            editing_id: None,
            status_msg: None,
            last_deleted: None,
        })
    }

    fn refresh(&mut self, db: &Database) {
        if let Ok(quotes) = db.list_quotes() {
            self.quotes = quotes;
            if self.selected >= self.quotes.len() && !self.quotes.is_empty() {
                self.selected = self.quotes.len() - 1;
            }
            if self.quotes.is_empty() {
                self.selected = 0;
            }
        }
    }

    fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let action_height: u16 = match &self.mode {
            Mode::Browse => 0,
            _ => 3,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(11),             // summary pane
                Constraint::Length(1),              // spacer
                Constraint::Min(4),                // quote list
                Constraint::Length(action_height),  // input/action area
                Constraint::Length(1),              // status message
                Constraint::Length(1),              // shortcuts
                Constraint::Min(0),                 // spacer
            ])
            .split(area);

        // ── Training summary ──
        self.render_summary(frame, chunks[0]);

        // ── Bordered quote list ──
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(tr("dashboard-quotes"), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(chunks[2]);
        frame.render_widget(block, chunks[2]);

        let visible = inner.height as usize;
        self.adjust_scroll(visible);

        if self.quotes.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                tr("dashboard-no-quotes-modal"),
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
        } else {
            let lines: Vec<Line> = self.quotes.iter().enumerate()
                .skip(self.scroll_offset)
                .take(visible)
                .map(|(i, q)| {
                    let is_sel = i == self.selected;
                    let marker = if is_sel { "> " } else { "  " };
                    let style = if is_sel {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let max_text = inner.width.saturating_sub(3) as usize;
                    let display = if q.text.chars().count() > max_text {
                        let truncated: String = q.text.chars().take(max_text.saturating_sub(1)).collect();
                        format!("{}{}…", marker, truncated)
                    } else {
                        format!("{}{}", marker, q.text)
                    };
                    Line::from(Span::styled(display, style))
                })
                .collect();

            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

            if self.selected >= self.scroll_offset {
                let row = (self.selected - self.scroll_offset) as u16;
                highlight_row(frame, inner, row);
            }
        }

        // ── Action area ──
        if action_height > 0 {
            let action_lines = match &self.mode {
                Mode::Add | Mode::Edit => {
                    let label = if self.mode == Mode::Add { tr("key-add") } else { tr("key-edit") };
                    let mut spans = vec![
                        Span::styled(format!(" {}: ", label), Style::default().fg(Color::Gray)),
                    ];
                    spans.extend(visible_input_spans(&self.input, self.input_cursor, area.width, (label.len() + 4) as u16, GREEN));
                    vec![
                        Line::from(spans),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                            Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::DarkGray)),
                            Span::styled("[Esc]", Style::default().fg(ACCENT)),
                            Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                        ]),
                    ]
                }
                Mode::ConfirmDelete => {
                    vec![
                        Line::from(Span::styled(
                            format!(" {}", tr("quotes-delete-confirm")),
                            Style::default().fg(Color::Red),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(" [y]", Style::default().fg(ACCENT)),
                            Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::DarkGray)),
                            Span::styled("[any]", Style::default().fg(ACCENT)),
                            Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::DarkGray)),
                        ]),
                    ]
                }
                Mode::Browse => vec![],
            };
            frame.render_widget(Paragraph::new(action_lines), chunks[3]);
        }

        // ── Status line ──
        render_status_line(frame, chunks[4], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = match &self.mode {
            Mode::Browse => {
                let mut spans = vec![
                    Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[a]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[e]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::DarkGray)),
                    Span::styled("[d]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::DarkGray)),
                ];
                if self.last_deleted.is_some() {
                    spans.push(Span::styled("[u]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(format!(" {}  ", tr("key-undo")), Style::default().fg(Color::DarkGray)));
                }
                spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::DarkGray)));
                Line::from(spans)
            }
            _ => Line::from(""),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[5]);

    }

    fn render_summary(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(tr("summary-title"), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let volume_tons = self.weekly_volume / 1000.0;
        let volume_text = tr_args("summary-volume", &[("value", FluentValue::from(format!("{:.1}", volume_tons)))]);
        let consecutive_text = tr_args("summary-consecutive", &[("days", FluentValue::from(self.consecutive_days))]);
        let recovery_text = if let Some(hrv) = self.hrv {
            tr_args("summary-recovery", &[("value", FluentValue::from(hrv as i64))])
        } else {
            tr("summary-recovery-na")
        };
        let frequency_text = tr_args("summary-frequency", &[("days", FluentValue::from(self.training_days as i64))]);

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(format!("  {}", volume_text), Style::default().fg(Color::White))),
            Line::from(Span::styled(format!("  {}", consecutive_text), Style::default().fg(Color::White))),
            Line::from(Span::styled(format!("  {}", recovery_text), Style::default().fg(Color::White))),
            Line::from(Span::styled(format!("  {}", frequency_text), Style::default().fg(Color::White))),
        ];

        if !self.featured_quote.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  > {}", self.featured_quote),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            )));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::Add | Mode::Edit => self.handle_input(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.input_cursor > 0 {
                    self.input_cursor = self.input[..self.input_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                true
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor = self.input[..self.input_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor = self.input[self.input_cursor..]
                        .char_indices().nth(1).map(|(i, _)| self.input_cursor + i)
                        .unwrap_or(self.input.len());
                }
                true
            }
            KeyCode::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor = self.input[self.input_cursor..]
                        .char_indices().nth(1).map(|(i, _)| self.input_cursor + i)
                        .unwrap_or(self.input.len());
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_cursor = 0;
                true
            }
            KeyCode::Home => { self.input_cursor = 0; true }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_cursor = self.input.len();
                true
            }
            KeyCode::End => { self.input_cursor = self.input.len(); true }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    let prev = self.input[..self.input_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.input.remove(prev);
                    self.input_cursor = prev;
                }
                true
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.truncate(self.input_cursor);
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.quotes.is_empty() && self.selected < self.quotes.len() - 1 {
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
            KeyCode::Char('a') => {
                self.input.clear();
                self.input_cursor = 0;
                self.editing_id = None;
                self.mode = Mode::Add;
                Action::None
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(q) = self.quotes.get(self.selected) {
                    self.input = q.text.clone();
                    self.input_cursor = self.input.len();
                    self.editing_id = Some(q.id);
                    self.mode = Mode::Edit;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.quotes.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(quote) = self.last_deleted.take() {
                    match db.restore_quote(&quote) {
                        Ok(_) => {
                            self.refresh(db);
                            self.status_msg = Some((tr("status-restored"), false));
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            _ => Action::None,
        }
    }

    fn handle_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let text = self.input.trim().to_string();
                if !text.is_empty() {
                    let result = if let Some(id) = self.editing_id {
                        db.update_quote(id, &text)
                    } else {
                        db.create_quote(&text).map(|_| ())
                    };
                    match result {
                        Ok(()) => self.refresh(db),
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                self.input.clear();
                self.input_cursor = 0;
                self.editing_id = None;
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.input_cursor = 0;
                self.editing_id = None;
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
        if key.code == KeyCode::Char('y') {
            if let Some(q) = self.quotes.get(self.selected) {
                let quote_clone = q.clone();
                let id = q.id;
                match db.delete_quote(id) {
                    Ok(()) => {
                        self.last_deleted = Some(quote_clone);
                        self.status_msg = Some((tr("status-deleted-undo"), false));
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Error: {}", e), true));
                    }
                }
                self.refresh(db);
            }
        }
        self.mode = Mode::Browse;
        Action::None
    }
}
