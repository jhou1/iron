use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::Abbreviation;
use super::{centered_area, highlight_row, render_help_overlay, render_status_line, visible_input_spans, Action, Screen, StatusMessage, CONTENT_WIDTH};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    AddShort,
    AddFull,
    EditShort,
    EditFull,
    ConfirmDelete,
}

pub struct AbbreviationsScreen {
    abbreviations: Vec<Abbreviation>,
    selected: usize,
    mode: Mode,
    short_input: String,
    short_cursor: usize,
    full_input: String,
    full_cursor: usize,
    editing_id: Option<i64>,
    status_msg: StatusMessage,
    show_help: bool,
}

impl AbbreviationsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let abbreviations = db.list_abbreviations()?;
        Ok(Self {
            abbreviations,
            selected: 0,
            mode: Mode::Browse,
            short_input: String::new(),
            short_cursor: 0,
            full_input: String::new(),
            full_cursor: 0,
            editing_id: None,
            status_msg: None,
            show_help: false,
        })
    }

    fn refresh(&mut self, db: &Database) {
        if let Ok(abbreviations) = db.list_abbreviations() {
            self.abbreviations = abbreviations;
            if self.selected >= self.abbreviations.len() && !self.abbreviations.is_empty() {
                self.selected = self.abbreviations.len() - 1;
            }
            if self.abbreviations.is_empty() {
                self.selected = 0;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let list_height = (self.abbreviations.len() as u16).max(1);
        let action_height: u16 = match &self.mode {
            Mode::Browse => 0,
            _ => 4,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),             // title + header
                Constraint::Length(list_height),   // abbreviation list
                Constraint::Length(action_height), // input/action area
                Constraint::Length(1),             // status message
                Constraint::Length(1),             // shortcuts
                Constraint::Min(0),                // spacer
            ])
            .split(area);

        // ── Title + header ──
        let max_short_len = self.abbreviations.iter()
            .map(|a| a.short.width())
            .max()
            .unwrap_or(0);
        let short_header = tr("abbreviations-col-short");
        let col_width = max_short_len.max(short_header.width()) + 4;

        let full_header = tr("abbreviations-col-full");
        let header_padding = col_width.saturating_sub(short_header.width());
        let title_lines = vec![
            Line::from(Span::styled(
                tr("abbreviations-title"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(&short_header, Style::default().fg(Color::DarkGray)),
                Span::raw(" ".repeat(header_padding)),
                Span::styled(&full_header, Style::default().fg(Color::DarkGray)),
            ]),
        ];
        frame.render_widget(Paragraph::new(title_lines), chunks[0]);

        // ── Abbreviation list ──
        let list_lines: Vec<Line> = if self.abbreviations.is_empty() {
            vec![Line::from(Span::styled(
                tr("abbreviations-no-items"),
                Style::default().fg(Color::Gray),
            ))]
        } else {
            self.abbreviations
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let marker = if i == self.selected { "> " } else { "  " };
                    let style = if i == self.selected {
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let padding = col_width.saturating_sub(a.short.width());
                    Line::from(vec![
                        Span::styled(marker, style),
                        Span::styled(&a.short, style),
                        Span::raw(" ".repeat(padding)),
                        Span::styled(&a.full_name, style),
                    ])
                })
                .collect()
        };
        frame.render_widget(Paragraph::new(list_lines), chunks[1]);

        if !self.abbreviations.is_empty() {
            highlight_row(frame, chunks[1], self.selected as u16);
        }

        // ── Input/action area ──
        let action_lines = match &self.mode {
            Mode::Browse => {
                vec![
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddShort => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(&self.short_input, self.short_cursor, area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("abbreviations-enter-short"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                ]
            }
            Mode::AddFull => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(&self.full_input, self.full_cursor, area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("abbreviations-enter-full"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                ]
            }
            Mode::EditShort => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(&self.short_input, self.short_cursor, area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("abbreviations-edit-short"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                ]
            }
            Mode::EditFull => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(&self.full_input, self.full_cursor, area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("abbreviations-edit-full"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                ]
            }
            Mode::ConfirmDelete => {
                let short = self
                    .abbreviations
                    .get(self.selected)
                    .map(|a| a.short.as_str())
                    .unwrap_or("?");
                vec![
                    Line::from(Span::styled(
                        tr_args("abbreviations-delete-confirm", &[("short", FluentValue::from(short.to_string()))]),
                        Style::default().fg(RED),
                    )),
                    Line::from(""),
                ]
            }
        };
        frame.render_widget(Paragraph::new(action_lines), chunks[2]);

        // ── Status line ──
        render_status_line(frame, chunks[3], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = match &self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::Gray)),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[?]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-help")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::Gray)),
            ]),
            Mode::AddShort | Mode::AddFull | Mode::EditShort | Mode::EditFull => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ]),
            Mode::ConfirmDelete => Line::from(vec![
                Span::styled(" [y]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                Span::styled("[n]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-no")), Style::default().fg(Color::Gray)),
            ]),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[4]);

        // ── Help overlay ──
        if self.show_help {
            let bindings = &[
                ("j/k", "Navigate"),
                ("a", "Add"),
                ("e", "Edit"),
                ("d", "Delete"),
                ("?", "Help"),
                ("Esc", "Back"),
            ];
            render_help_overlay(frame, area, bindings);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::AddShort => self.handle_add_short(key),
            Mode::AddFull => self.handle_add_full(key, db),
            Mode::EditShort => self.handle_edit_short(key),
            Mode::EditFull => self.handle_edit_full(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_text_input(input: &mut String, cursor: &mut usize, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    *cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    *cursor = prev;
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor < input.len() {
                    let next = input[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(input.len());
                    *cursor = next;
                }
                true
            }
            KeyCode::Right => {
                if *cursor < input.len() {
                    let next = input[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(input.len());
                    *cursor = next;
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *cursor = 0;
                true
            }
            KeyCode::Home => {
                *cursor = 0;
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *cursor = input.len();
                true
            }
            KeyCode::End => {
                *cursor = input.len();
                true
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    input.remove(prev);
                    *cursor = prev;
                }
                true
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.truncate(*cursor);
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, _db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.abbreviations.is_empty() {
                    self.selected = (self.selected + 1) % self.abbreviations.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.abbreviations.is_empty() {
                    self.selected = self
                        .selected
                        .checked_sub(1)
                        .unwrap_or(self.abbreviations.len() - 1);
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.short_input.clear();
                self.short_cursor = 0;
                self.full_input.clear();
                self.full_cursor = 0;
                self.mode = Mode::AddShort;
                Action::None
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(a) = self.abbreviations.get(self.selected) {
                    self.editing_id = Some(a.id);
                    self.short_input = a.short.clone();
                    self.short_cursor = self.short_input.len();
                    self.full_input = a.full_name.clone();
                    self.full_cursor = self.full_input.len();
                    self.mode = Mode::EditShort;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.abbreviations.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::QuickLog),
            _ => Action::None,
        }
    }

    fn handle_add_short(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.short_input.trim().is_empty() {
                    self.mode = Mode::AddFull;
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                Self::handle_text_input(&mut self.short_input, &mut self.short_cursor, key);
                Action::None
            }
        }
    }

    fn handle_add_full(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.full_input.trim().is_empty() {
                    match db.create_abbreviation(self.short_input.trim(), self.full_input.trim()) {
                        Ok(_) => {
                            self.refresh(db);
                            self.mode = Mode::Browse;
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                            self.mode = Mode::Browse;
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                Self::handle_text_input(&mut self.full_input, &mut self.full_cursor, key);
                Action::None
            }
        }
    }

    fn handle_edit_short(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.short_input.trim().is_empty() {
                    self.mode = Mode::EditFull;
                }
                Action::None
            }
            KeyCode::Esc => {
                self.editing_id = None;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                Self::handle_text_input(&mut self.short_input, &mut self.short_cursor, key);
                Action::None
            }
        }
    }

    fn handle_edit_full(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(id) = self.editing_id {
                    let short_trimmed = self.short_input.trim();
                    let full_trimmed = self.full_input.trim();
                    if !short_trimmed.is_empty() && !full_trimmed.is_empty() {
                        match db.update_abbreviation(id, short_trimmed, full_trimmed) {
                            Ok(()) => self.refresh(db),
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                self.editing_id = None;
                self.short_input.clear();
                self.short_cursor = 0;
                self.full_input.clear();
                self.full_cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => {
                self.editing_id = None;
                self.short_input.clear();
                self.short_cursor = 0;
                self.full_input.clear();
                self.full_cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                Self::handle_text_input(&mut self.full_input, &mut self.full_cursor, key);
                Action::None
            }
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(a) = self.abbreviations.get(self.selected) {
                    match db.delete_abbreviation(a.id) {
                        Ok(()) => self.refresh(db),
                        Err(e) => {
                            self.status_msg = Some((format!("Delete failed: {}", e), true));
                        }
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
