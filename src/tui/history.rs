use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, SetData};
use super::{centered_area, highlight_row, render_status_line, Action, Screen, StatusMessage, BORDER_COLOR, CONTENT_WIDTH};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;


#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Browse,
    ConfirmDelete,
}

pub struct HistoryScreen {
    entries: Vec<LogEntry>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filter_cursor: usize,
    filtering: bool,
    selected: usize,
    mode: Mode,
    scroll_offset: usize,
    status_msg: StatusMessage,
    last_deleted: Option<LogEntry>,
}

impl HistoryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let entries = db.list_logs_all()?;
        let filtered_indices = (0..entries.len()).collect();
        Ok(Self {
            entries,
            filtered_indices,
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            mode: Mode::Browse,
            scroll_offset: 0,
            status_msg: None,
            last_deleted: None,
        })
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.filtered_indices.get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    fn apply_filter(&mut self) {
        let lower = self.filter_text.to_lowercase();
        self.filtered_indices = self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.practice_name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let max_name_len = self.filtered_indices.iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.practice_name.width())
            .max()
            .unwrap_or(4);
        let name_col_w = (max_name_len + 2) as u16;
        let marker_w: u16 = 2;     // "> " or "  "
        let date_col_w: u16 = 11;  // "2026-05-12" + 1 padding
        let vol_col_w: u16 = 16;
        let border_w: u16 = 2;
        let list_width = marker_w + date_col_w + name_col_w + vol_col_w + border_w;

        // Vertical split: filter | main content (list + detail) | status | shortcuts
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // [0] filter
                Constraint::Min(1),    // [1] main content area (list + detail side by side)
                Constraint::Length(1), // [2] status line
                Constraint::Length(1), // [3] shortcuts
            ])
            .split(area);

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
        let filter_line = Paragraph::new(Line::from(Span::styled(filter_display, filter_style)));
        frame.render_widget(filter_line, v_chunks[0]);

        // Horizontal split within main content: list (left) | detail (right)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(list_width),
                Constraint::Min(20),
            ])
            .split(v_chunks[1]);

        let list_height = h_chunks[0].height as usize;
        self.adjust_scroll(list_height.saturating_sub(3)); // -2 for border, -1 for header row
        self.render_list(frame, h_chunks[0], list_height.saturating_sub(3), marker_w, name_col_w, date_col_w, vol_col_w);

        // ── Right: detail panel ──
        self.render_detail(frame, h_chunks[1]);

        // Status + shortcuts
        let shortcuts = {
            let navigate_text = format!(" {}  ", tr("key-navigate"));
            let filter_text = format!(" {}  ", tr("key-filter"));
            let edit_text = format!(" {}  ", tr("key-edit"));
            let delete_text = format!(" {}  ", tr("key-delete"));
            let mut spans = vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(navigate_text, Style::default().fg(Color::Gray)),
                Span::styled("[/]", Style::default().fg(ACCENT)),
                Span::styled(filter_text, Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(edit_text, Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(delete_text, Style::default().fg(Color::Gray)),
            ];
            if self.last_deleted.is_some() {
                let undo_text = format!(" {}  ", tr("key-undo"));
                spans.push(Span::styled("[u]", Style::default().fg(ACCENT)));
                spans.push(Span::styled(undo_text, Style::default().fg(Color::Gray)));
            }
            let back_text = format!(" {}", tr("key-back"));
            spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
            spans.push(Span::styled(back_text, Style::default().fg(Color::Gray)));
            Line::from(spans)
        };
        render_status_line(frame, v_chunks[2], &self.status_msg);
        frame.render_widget(Paragraph::new(shortcuts), v_chunks[3]);

    }

    /// Adjusts scroll_offset so the selected item is visible within the given height.
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

    #[allow(clippy::too_many_arguments)]
    fn render_list(&self, frame: &mut Frame, area: ratatui::layout::Rect, visible: usize, marker_w: u16, name_col_w: u16, date_col_w: u16, vol_col_w: u16) {
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(tr("history-title"), Style::default().fg(Color::White).bold()),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.filtered_indices.is_empty() {
            let no_entries_text = format!("  {}", tr("history-no-entries"));
            let empty = Paragraph::new(Line::from(Span::styled(
                no_entries_text,
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(empty, inner);
            return;
        }

        let hdr_style = Style::default().fg(Color::White).bold();
        let header = Row::new(vec![
            Cell::from(""),
            Cell::from(Span::styled(tr("history-col-date"), hdr_style)),
            Cell::from(Span::styled(tr("history-col-practice"), hdr_style)),
            Cell::from(Span::styled(tr("history-col-volume"), hdr_style)),
        ]);

        let mut rows: Vec<Row> = Vec::new();
        for (fi, &entry_idx) in self.filtered_indices.iter().enumerate().skip(self.scroll_offset).take(visible) {
            let entry = &self.entries[entry_idx];
            let date = entry.log.logged_at.format("%Y-%m-%d").to_string();
            let total = format!("{:.0}", entry.total_metric());
            let label = entry.metric_label();

            let style = if fi == self.selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let dim = if fi == self.selected {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let marker = if fi == self.selected { ">" } else { " " };

            rows.push(Row::new(vec![
                Cell::from(Span::styled(marker, style)),
                Cell::from(Span::styled(date, dim)),
                Cell::from(Span::styled(entry.practice_name.clone(), style)),
                Cell::from(Span::styled(format!("{} {}", total, label), dim)),
            ]));

            if fi == self.selected && self.mode == Mode::ConfirmDelete {
                rows.push(Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(Line::from(vec![
                        Span::styled(format!("{} ", tr("history-delete-confirm")), Style::default().fg(Color::Red)),
                        Span::styled("[y]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                        Span::styled("[any]", Style::default().fg(ACCENT)),
                        Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                    ])),
                    Cell::from(""),
                ]));
            }
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(marker_w),
                Constraint::Length(date_col_w),
                Constraint::Length(name_col_w),
                Constraint::Length(vol_col_w),
            ],
        )
        .header(header);

        frame.render_widget(table, inner);

        if !self.filtered_indices.is_empty() && self.selected >= self.scroll_offset {
            let row = (self.selected - self.scroll_offset) as u16 + 1; // +1 for header row
            highlight_row(frame, inner, row);
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let Some(entry) = self.selected_entry() else {
            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                    Span::styled("Detail", Style::default().fg(Color::White).bold()),
                    Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
                ]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_COLOR));
            frame.render_widget(block, area);
            return;
        };

        let title_text = format!(" {} — {} ", entry.practice_name,
            entry.log.logged_at.format("%Y-%m-%d"));
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(title_text, Style::default().fg(Color::White).bold()),
                Span::styled("──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR));

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        let mut total_reps = 0.0;
        for set in &entry.sets {
            let detail = match &set.data {
                SetData::Weighted { weight, reps } => {
                    total_reps += *reps as f64;
                    format!("  {}", tr_args("history-set-weighted", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("weight", FluentValue::from(*weight)),
                        ("reps", FluentValue::from(*reps as f64)),
                    ]))
                }
                SetData::Bodyweight { reps } => {
                    total_reps += *reps as f64;
                    format!("  {}", tr_args("history-set-bodyweight", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("reps", FluentValue::from(*reps as f64)),
                    ]))
                }
                SetData::Distance { distance } => {
                    format!("  {}", tr_args("history-set-distance", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("distance", FluentValue::from(*distance)),
                    ]))
                }
                SetData::Endurance { duration } => {
                    format!("  {}", tr_args("history-set-endurance", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("duration", FluentValue::from(*duration)),
                    ]))
                }
            };
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::White),
            )));
        }

        // Summary line: total reps + volume for weighted training
        if matches!(entry.practice_type, crate::model::PracticeType::Weighted | crate::model::PracticeType::Bodyweight) && total_reps > 0.0 {
            lines.push(Line::from(""));
            let total_vol = entry.total_metric();
            let vol_label = entry.metric_label();
            let reps_label = tr("metric-reps");
            lines.push(Line::from(Span::styled(
                format!("  {}", tr_args("history-summary", &[
                    ("reps", FluentValue::from(total_reps)),
                    ("reps_label", FluentValue::from(reps_label)),
                    ("vol", FluentValue::from(total_vol)),
                    ("vol_label", FluentValue::from(vol_label)),
                ])),
                Style::default().fg(Color::White),
            )));
        }

        if let Some(warm_up) = &entry.log.warm_up {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}:", tr("log-warmup-label")),
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", warm_up),
                Style::default().fg(Color::White),
            )));
        }

        if let Some(cool_down) = &entry.log.cool_down {
            lines.push(Line::from(Span::styled(
                format!("  {}:", tr("log-cooldown-label")),
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", cool_down),
                Style::default().fg(Color::White),
            )));
        }

        if let Some(note) = &entry.log.note {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", tr("log-note-label")),
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", note),
                Style::default().fg(Color::White),
            )));
        }

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match self.mode {
            Mode::Browse => {
                if self.filtering {
                    self.handle_filter_input(key);
                    Action::None
                } else {
                    self.handle_browse(key, db)
                }
            }
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_filter_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
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
                self.apply_filter();
            }
            KeyCode::Char(c) => {
                self.filter_text.insert(self.filter_cursor, c);
                self.filter_cursor += c.len_utf8();
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
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
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter_text.clear();
                self.filter_cursor = 0;
                self.apply_filter();
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.filtered_indices.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Char('e') => {
                if !self.filtered_indices.is_empty() {
                    return Action::Navigate(Screen::LogEntry);
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(entry) = self.last_deleted.take() {
                    match db.restore_log(&entry) {
                        Ok(_) => {
                            if let Ok(entries) = db.list_logs_all() {
                                self.entries = entries;
                            }
                            self.apply_filter();
                            self.status_msg = Some((tr("status-restored"), false));
                        }
                        Err(e) => {
                            self.last_deleted = Some(entry);
                            self.status_msg = Some((format!("Restore failed: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                if !self.filter_text.is_empty() {
                    self.filter_text.clear();
                    self.filter_cursor = 0;
                    self.apply_filter();
                    Action::None
                } else {
                    Action::Navigate(Screen::Dashboard)
                }
            }
            _ => Action::None,
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        if key.code == KeyCode::Char('y') {
            if let Some(entry) = self.selected_entry() {
                let entry_clone = entry.clone();
                let log_id = entry.log.id;
                match db.delete_log(log_id) {
                    Ok(()) => {
                        self.last_deleted = Some(entry_clone);
                        self.status_msg = Some((tr("status-deleted-undo"), false));
                        if let Ok(entries) = db.list_logs_all() {
                            self.entries = entries;
                        }
                        self.apply_filter();
                        if self.selected >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
                            self.selected = self.filtered_indices.len() - 1;
                        }
                        if self.filtered_indices.is_empty() {
                            self.selected = 0;
                        }
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Delete failed: {}", e), true));
                    }
                }
            }
        }
        self.mode = Mode::Browse;
        Action::None
    }
}
