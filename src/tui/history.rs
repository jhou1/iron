use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, SetData};
use super::{centered_area, highlight_row, Action, Screen, CONTENT_WIDTH};
use fluent_bundle::FluentValue;

const GREEN: Color = Color::Green;
const ACCENT: Color = Color::Cyan;
const NOTE_COLOR: Color = Color::Yellow;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Browse,
    ConfirmDelete,
}

pub struct HistoryScreen {
    entries: Vec<LogEntry>,
    selected: usize,
    mode: Mode,
    /// Scroll offset for the list pane so the selected item stays visible.
    scroll_offset: usize,
}

impl HistoryScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let entries = db.list_logs_all()?;
        Ok(Self {
            entries,
            selected: 0,
            mode: Mode::Browse,
            scroll_offset: 0,
        })
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.entries.get(self.selected)
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let max_name_len = self.entries.iter()
            .map(|e| e.practice_name.width())
            .max()
            .unwrap_or(0);
        let name_col = max_name_len + 2;
        let list_width = (3 + 13 + 2 + name_col + 16) as u16;

        // Horizontal split: list (left) | detail panel (right)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(list_width),
                Constraint::Min(20),
            ])
            .split(area);

        // ── Left: title + list + shortcuts ──
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // title + column headers
                Constraint::Min(1),    // scrollable list
                Constraint::Length(1), // shortcuts
            ])
            .split(h_chunks[0]);

        let date_header = tr("history-col-date");
        let practice_header = tr("history-col-practice");
        let volume_header = tr("history-col-volume");
        let header_name_padding = name_col.saturating_sub(practice_header.width());
        let title_lines = vec![
            Line::from(Span::styled(
                tr("history-title"),
                Style::default().fg(Color::White).bold(),
            )),
            Line::from(vec![
                Span::styled("   ", Style::default().fg(Color::DarkGray)),
                Span::styled(&date_header, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(&practice_header, Style::default().fg(Color::DarkGray)),
                Span::raw(" ".repeat(header_name_padding)),
                Span::styled(&volume_header, Style::default().fg(Color::DarkGray)),
            ]),
        ];
        frame.render_widget(Paragraph::new(title_lines), left_chunks[0]);

        let list_height = left_chunks[1].height as usize;
        self.adjust_scroll(list_height);
        self.render_list(frame, left_chunks[1], list_height, name_col);

        let shortcuts = {
            let navigate_text = format!(" {}  ", tr("key-navigate"));
            let edit_text = format!(" {}  ", tr("key-edit"));
            let delete_text = format!(" {}  ", tr("key-delete"));
            let back_text = format!(" {}", tr("key-back"));
            Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(navigate_text, Style::default().fg(Color::Gray)),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(edit_text, Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(delete_text, Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(back_text, Style::default().fg(Color::Gray)),
            ])
        };
        frame.render_widget(Paragraph::new(shortcuts), left_chunks[2]);

        // ── Right: detail panel ──
        self.render_detail(frame, h_chunks[1]);
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

    fn render_list(&self, frame: &mut Frame, area: ratatui::layout::Rect, visible: usize, name_col: usize) {
        if self.entries.is_empty() {
            let no_entries_text = format!("  {}", tr("history-no-entries"));
            let empty = Paragraph::new(Line::from(Span::styled(
                no_entries_text,
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(empty, area);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for (i, entry) in self.entries.iter().enumerate().skip(self.scroll_offset).take(visible) {
            let marker = if i == self.selected { " > " } else { "   " };
            let date = entry.log.logged_at.format("%Y %b %d").to_string();
            let total = format!("{:.0}", entry.total_metric());
            let label = entry.metric_label();
            let name_padding = name_col.saturating_sub(entry.practice_name.width());

            let style = if i == self.selected {
                Style::default().fg(GREEN).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let dim = Style::default().fg(Color::Gray);

            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(date, dim),
                Span::raw("  "),
                Span::styled(&entry.practice_name, style),
                Span::raw(" ".repeat(name_padding)),
                Span::styled(format!("{} {}", total, label), dim),
            ]));

            if i == self.selected && self.mode == Mode::ConfirmDelete {
                let confirm_text = format!("     {} ", tr("history-delete-confirm"));
                lines.push(Line::from(vec![
                    Span::styled(confirm_text, Style::default().fg(Color::Red)),
                    Span::styled("[y]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}  ", tr("key-yes")), Style::default().fg(Color::Gray)),
                    Span::styled("[any]", Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), area);

        if !self.entries.is_empty() && self.selected >= self.scroll_offset {
            let row = (self.selected - self.scroll_offset) as u16;
            highlight_row(frame, area, row);
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let Some(entry) = self.selected_entry() else {
            let block = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray));
            frame.render_widget(block, area);
            return;
        };

        let title = format!(" {} — {} ", entry.practice_name,
            entry.log.logged_at.format("%Y %b %d"));
        let block = Block::default()
            .title(Span::styled(title, Style::default().fg(ACCENT)))
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray));

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        for set in &entry.sets {
            let detail = match &set.data {
                SetData::Weighted { weight, reps } => {
                    format!("  {}", tr_args("history-set-weighted", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("weight", FluentValue::from(*weight)),
                        ("reps", FluentValue::from(*reps as f64)),
                    ]))
                }
                SetData::Bodyweight { reps } => {
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
                        ("duration", FluentValue::from(*duration as f64)),
                    ]))
                }
            };
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::White),
            )));
        }

        if let Some(warm_up) = &entry.log.warm_up {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", tr_args("history-warmup", &[
                    ("text", FluentValue::from(warm_up.clone())),
                ])),
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(cool_down) = &entry.log.cool_down {
            lines.push(Line::from(Span::styled(
                format!("  {}", tr_args("history-cooldown", &[
                    ("text", FluentValue::from(cool_down.clone())),
                ])),
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(note) = &entry.log.note {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", tr_args("history-note", &[
                    ("note", FluentValue::from(note.clone())),
                ])),
                Style::default().fg(NOTE_COLOR),
            )));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, _db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
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
            KeyCode::Char('d') => {
                if !self.entries.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Enter => {
                if !self.entries.is_empty() {
                    return Action::Navigate(Screen::LogEntry);
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            _ => Action::None,
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        if key.code == KeyCode::Char('y') {
            if let Some(entry) = self.entries.get(self.selected) {
                let log_id = entry.log.id;
                let _ = db.delete_log(log_id);
                // Re-fetch entries after deletion
                if let Ok(entries) = db.list_logs_all() {
                    self.entries = entries;
                }
                // Adjust selected index
                if self.selected >= self.entries.len() && !self.entries.is_empty() {
                    self.selected = self.entries.len() - 1;
                }
                if self.entries.is_empty() {
                    self.selected = 0;
                }
            }
        }
        self.mode = Mode::Browse;
        Action::None
    }
}
