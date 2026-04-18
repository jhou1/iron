use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::model::{LogEntry, SetData};
use super::{highlight_row, Action, Screen};

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
                Constraint::Length(4),           // detail pane
                Constraint::Length(1),           // shortcuts
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        // ── Title ──
        let title = Line::from(vec![
            Span::styled(
                " History",
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
            Line::from(vec![
                Span::styled(" Delete this entry? ", Style::default().fg(Color::Red)),
                Span::styled("[y]", Style::default().fg(ACCENT)),
                Span::styled(" Yes  ", Style::default().fg(Color::Gray)),
                Span::styled("[any]", Style::default().fg(ACCENT)),
                Span::styled(" Cancel", Style::default().fg(Color::Gray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(" [j/k]", Style::default().fg(ACCENT)),
                Span::styled(" Navigate  ", Style::default().fg(Color::Gray)),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(" Edit  ", Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" Back", Style::default().fg(Color::Gray)),
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
            let empty = Paragraph::new(Line::from(Span::styled(
                "  No entries yet",
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
                let text = format!(
                    "{}{}  {}  {} sets  {:.0} {}",
                    marker, date, entry.practice_name, sets_count, total, label
                );
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
                    format!("    #{}  {}kg x {}", set.set_number, weight, reps)
                }
                SetData::Bodyweight { reps } => {
                    format!("    #{}  {} reps", set.set_number, reps)
                }
                SetData::Distance { distance } => {
                    format!("    #{}  {} km", set.set_number, distance)
                }
                SetData::Endurance { duration } => {
                    format!("    #{}  {} min", set.set_number, duration)
                }
            };
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(note) = &entry.log.note {
            lines.push(Line::from(Span::styled(
                format!("    Note: {}", note),
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
