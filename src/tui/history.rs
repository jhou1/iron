use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, SetData};
use super::{highlight_row, Action, Screen};
use fluent_bundle::FluentValue;

const GREEN: Color = Color::Green;
const ACCENT: Color = Color::Cyan;
const NOTE_COLOR: Color = Color::Yellow;
const CONTENT_WIDTH: u16 = 3 + 52 * 2;

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
        let full = frame.area();
        let area = Rect {
            x: full.x + 1,
            y: full.y,
            width: full.width.saturating_sub(2).min(CONTENT_WIDTH),
            height: full.height,
        };

        // Vertical layout: title | list | detail | shortcuts | spacer
        let list_height = (self.entries.len() as u16).max(1);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),           // title
                Constraint::Length(list_height), // scrollable list
                Constraint::Length(6),           // detail pane
                Constraint::Length(1),           // shortcuts
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        // ── Title ──
        let title_text = tr("history-title");
        let title = Line::from(vec![
            Span::styled(
                &title_text,
                Style::default().fg(Color::White).bold(),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // ── List pane ──
        let list_height = chunks[1].height as usize;
        self.adjust_scroll(list_height);
        self.render_list(frame, chunks[1], list_height);

        // ── Detail pane ──
        self.render_detail(frame, chunks[2]);

        // ── Shortcuts ──
        let shortcuts = if self.mode == Mode::ConfirmDelete {
            let delete_confirm_text = format!(" {} ", tr("history-delete-confirm"));
            let yes_text = format!(" {}  ", tr("key-yes"));
            let cancel_text = format!(" {}", tr("key-cancel"));
            Line::from(vec![
                Span::styled(delete_confirm_text, Style::default().fg(Color::Red)),
                Span::styled("[y]", Style::default().fg(ACCENT)),
                Span::styled(yes_text, Style::default().fg(Color::Gray)),
                Span::styled("[any]", Style::default().fg(ACCENT)),
                Span::styled(cancel_text, Style::default().fg(Color::Gray)),
            ])
        } else {
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
        frame.render_widget(Paragraph::new(shortcuts), chunks[3]);
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

    fn render_list(&self, frame: &mut Frame, area: ratatui::layout::Rect, visible: usize) {
        if self.entries.is_empty() {
            let no_entries_text = format!("  {}", tr("history-no-entries"));
            let empty = Paragraph::new(Line::from(Span::styled(
                no_entries_text,
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(empty, area);
            return;
        }

        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible)
            .map(|(i, entry)| {
                let marker = if i == self.selected { " > " } else { "   " };
                let date = entry.log.logged_at.format("%b %d %H:%M").to_string();
                let sets_count = entry.sets.len();
                let total = entry.total_metric();
                let label = entry.metric_label();
                let info = tr_args("history-entry", &[
                    ("date", FluentValue::from(date.clone())),
                    ("name", FluentValue::from(entry.practice_name.clone())),
                    ("sets", FluentValue::from(sets_count as f64)),
                    ("total", FluentValue::from(format!("{:.0}", total))),
                    ("label", FluentValue::from(label.clone())),
                ]);
                let text = format!("{}{}", marker, info);
                if i == self.selected {
                    Line::from(Span::styled(text, Style::default().fg(GREEN).bold()))
                } else {
                    Line::from(Span::styled(text, Style::default().fg(Color::White)))
                }
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), area);

        if !self.entries.is_empty() && self.selected >= self.scroll_offset {
            let row = (self.selected - self.scroll_offset) as u16;
            highlight_row(frame, area, row);
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let Some(entry) = self.selected_entry() else {
            return;
        };

        let mut lines: Vec<Line> = Vec::new();

        for set in &entry.sets {
            let detail = match &set.data {
                SetData::Weighted { weight, reps } => {
                    format!("    {}", tr_args("history-set-weighted", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("weight", FluentValue::from(*weight)),
                        ("reps", FluentValue::from(*reps as f64)),
                    ]))
                }
                SetData::Bodyweight { reps } => {
                    format!("    {}", tr_args("history-set-bodyweight", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("reps", FluentValue::from(*reps as f64)),
                    ]))
                }
                SetData::Distance { distance } => {
                    format!("    {}", tr_args("history-set-distance", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("distance", FluentValue::from(*distance)),
                    ]))
                }
                SetData::Endurance { duration } => {
                    format!("    {}", tr_args("history-set-endurance", &[
                        ("number", FluentValue::from(set.set_number as f64)),
                        ("duration", FluentValue::from(*duration as f64)),
                    ]))
                }
            };
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(warm_up) = &entry.log.warm_up {
            lines.push(Line::from(Span::styled(
                format!("    {}", tr_args("history-warmup", &[
                    ("text", FluentValue::from(warm_up.clone())),
                ])),
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(cool_down) = &entry.log.cool_down {
            lines.push(Line::from(Span::styled(
                format!("    {}", tr_args("history-cooldown", &[
                    ("text", FluentValue::from(cool_down.clone())),
                ])),
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(note) = &entry.log.note {
            lines.push(Line::from(Span::styled(
                format!("    {}", tr_args("history-note", &[
                    ("note", FluentValue::from(note.clone())),
                ])),
                Style::default().fg(NOTE_COLOR),
            )));
        }

        frame.render_widget(Paragraph::new(lines), area);
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
