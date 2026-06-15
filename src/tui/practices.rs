use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::{
    centered_area, highlight_row, render_status_line, visible_input_spans, Action, Screen,
    StatusMessage, BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{Practice, PracticeType};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

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
    input_cursor: usize,
    type_selected: usize,
    status_msg: StatusMessage,
}

impl PracticesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        Ok(Self {
            practices,
            selected: 0,
            mode: Mode::Browse,
            input: String::new(),
            input_cursor: 0,
            type_selected: 0,
            status_msg: None,
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
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let list_height = (self.practices.len() as u16).max(1);
        let action_height: u16 = match &self.mode {
            Mode::Browse => 0,
            _ => 6,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(list_height + 3), // bordered block: title/header row + list + border
                Constraint::Length(1),               // spacer
                Constraint::Length(action_height),   // input/action area
                Constraint::Length(1),               // status message
                Constraint::Length(1),               // shortcuts
                Constraint::Min(0),                  // spacer
            ])
            .split(area);

        // ── Bordered practice list ──
        let max_name_len = self
            .practices
            .iter()
            .map(|p| p.name.width())
            .max()
            .unwrap_or(0);
        let col_width = max_name_len + 4;

        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let header_padding = col_width.saturating_sub(name_header.width());
        let type_col_width = self
            .practices
            .iter()
            .map(|p| p.practice_type.label().width())
            .max()
            .unwrap_or(0)
            .max(type_header.width())
            + 2;

        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("practices-title"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let hdr_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let header_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(&name_header, hdr_style),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&type_header, hdr_style),
        ]);

        let mut all_lines = vec![header_line];

        if self.practices.is_empty() {
            all_lines.push(Line::from(Span::styled(
                tr("practices-no-items"),
                Style::default().fg(Color::Gray),
            )));
        } else {
            for (i, p) in self.practices.iter().enumerate() {
                let marker = if i == self.selected { "> " } else { "  " };
                let (name_style, type_color) = if i == self.selected {
                    if p.active {
                        (
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                            Color::Gray,
                        )
                    } else {
                        (
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                            Color::DarkGray,
                        )
                    }
                } else if p.active {
                    (Style::default().fg(Color::White), Color::Gray)
                } else {
                    (Style::default().fg(Color::DarkGray), Color::DarkGray)
                };
                let padding = col_width.saturating_sub(p.name.width());
                let type_label = p.practice_type.label();
                let type_padding = type_col_width.saturating_sub(type_label.width());
                let toggle = if p.active {
                    vec![
                        Span::styled("\u{25B0}", Style::default().fg(GREEN)),
                        Span::styled("\u{25B1}", Style::default().fg(Color::DarkGray)),
                    ]
                } else {
                    vec![
                        Span::styled("\u{25B1}", Style::default().fg(Color::DarkGray)),
                        Span::styled("\u{25B0}", Style::default().fg(Color::DarkGray)),
                    ]
                };
                let mut spans = vec![
                    Span::styled(marker, name_style),
                    Span::styled(&p.name, name_style),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(type_label, Style::default().fg(type_color)),
                    Span::raw(" ".repeat(type_padding)),
                ];
                spans.extend(toggle);
                all_lines.push(Line::from(spans));
            }
        }
        frame.render_widget(Paragraph::new(all_lines), inner);

        if !self.practices.is_empty() {
            highlight_row(frame, inner, self.selected as u16 + 1); // +1 for header row
        }

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
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(
                    &self.input,
                    self.input_cursor,
                    area.width,
                    3,
                    GREEN,
                ));
                vec![
                    Line::from(Span::styled(
                        tr("practices-new-name"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddType => {
                let mut lines: Vec<Line> = vec![Line::from(Span::styled(
                    tr("practices-select-type"),
                    Style::default().fg(Color::White),
                ))];
                for (i, pt) in PracticeType::ALL.iter().enumerate() {
                    let marker = if i == self.type_selected { "> " } else { "  " };
                    let style = if i == self.type_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
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
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(visible_input_spans(
                    &self.input,
                    self.input_cursor,
                    area.width,
                    3,
                    GREEN,
                ));
                vec![
                    Line::from(Span::styled(
                        tr("practices-rename"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
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
                        tr_args(
                            "practices-delete-confirm",
                            &[("name", FluentValue::from(name.to_string()))],
                        ),
                        Style::default().fg(RED),
                    )),
                    Line::from(Span::styled(
                        tr("practices-delete-cascade-warning"),
                        Style::default().fg(RED),
                    )),
                    Line::from(""),
                    Line::from(""),
                ]
            }
        };
        frame.render_widget(Paragraph::new(action_lines), chunks[2]);

        if self.mode == Mode::AddType {
            highlight_row(frame, chunks[2], (self.type_selected + 1) as u16);
        }

        // ── Status line ──
        render_status_line(frame, chunks[3], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = match &self.mode {
            Mode::Browse => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-navigate")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-add")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-edit")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Space]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-toggle")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-delete")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-back")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Mode::AddName | Mode::EditName => Line::from(vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-confirm")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Mode::AddType => Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-select")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-confirm")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Mode::ConfirmDelete => Line::from(vec![
                Span::styled(" [y]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-yes")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[n]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-no")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        };
        frame.render_widget(Paragraph::new(vec![shortcuts]), chunks[4]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::AddName => self.handle_add_name(key),
            Mode::AddType => self.handle_add_type(key, db),
            Mode::EditName => self.handle_edit_name(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.input_cursor > 0 {
                    let prev = self.input[..self.input_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    let prev = self.input[..self.input_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_cursor = prev;
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.input_cursor < self.input.len() {
                    let next = self.input[self.input_cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.input_cursor + i)
                        .unwrap_or(self.input.len());
                    self.input_cursor = next;
                }
                true
            }
            KeyCode::Right => {
                if self.input_cursor < self.input.len() {
                    let next = self.input[self.input_cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.input_cursor + i)
                        .unwrap_or(self.input.len());
                    self.input_cursor = next;
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_cursor = 0;
                true
            }
            KeyCode::Home => {
                self.input_cursor = 0;
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_cursor = self.input.len();
                true
            }
            KeyCode::End => {
                self.input_cursor = self.input.len();
                true
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    let prev = self.input[..self.input_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
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
                self.input_cursor = 0;
                self.type_selected = 0;
                self.mode = Mode::AddName;
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(p) = self.practices.get(self.selected) {
                    self.input = p.name.clone();
                    self.input_cursor = self.input.len();
                    self.mode = Mode::EditName;
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(p) = self.practices.get(self.selected) {
                    match db.set_practice_active(p.id, !p.active) {
                        Ok(()) => self.refresh(db),
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
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
            _ => {
                self.handle_text_input(key);
                Action::None
            }
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
                match db.create_practice(self.input.trim(), pt) {
                    Ok(_) => {
                        self.refresh(db);
                        self.mode = Mode::Browse;
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Error: {}", e), true));
                        self.mode = Mode::Browse;
                    }
                }
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
                        match db.rename_practice(p.id, trimmed) {
                            Ok(()) => self.refresh(db),
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    }
                }
                self.input.clear();
                self.input_cursor = 0;
                self.mode = Mode::Browse;
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.input_cursor = 0;
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
                if let Some(p) = self.practices.get(self.selected) {
                    match db.delete_practice(p.id) {
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
