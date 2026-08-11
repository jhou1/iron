use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::{
    centered_area,
    framework::{
        fit_rect, render_detail_panel, render_shortcuts, shortcut_height, visual_spans,
        DetailPanelState, Shortcut, TextInput, ViewportState, VisualSpan,
    },
    highlight_row, render_status_line, Action, ModuleId, NavigationIntent, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, SetData};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Browse,
    Detail,
    ConfirmDelete,
}

pub struct HistoryScreen {
    entries: Vec<LogEntry>,
    filtered_indices: Vec<usize>,
    filter: TextInput,
    filtering: bool,
    selected: usize,
    mode: Mode,
    list_viewport: ViewportState,
    detail_state: DetailPanelState,
    detail_content_height: usize,
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
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            mode: Mode::Browse,
            list_viewport: ViewportState::new(0),
            detail_state: DetailPanelState::new(false),
            detail_content_height: 0,
            status_msg: None,
            last_deleted: None,
        })
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    fn apply_filter(&mut self) {
        let lower = self.filter.value().to_lowercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.practice_name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.list_viewport.set_offset(0);
    }

    fn active_shortcuts(&self) -> Vec<Shortcut> {
        let mut shortcuts = match self.mode {
            Mode::ConfirmDelete => vec![
                Shortcut::new(
                    "y",
                    format!("{} · {}", tr("history-delete-confirm"), tr("key-yes")),
                ),
                Shortcut::new("any", tr("key-cancel")),
            ],
            Mode::Detail => vec![
                Shortcut::new("j/k", tr("key-scroll")),
                Shortcut::new("Esc", tr("key-back")),
            ],
            Mode::Browse => vec![
                Shortcut::new("j/k", tr("key-navigate")),
                Shortcut::new("Enter", tr("key-details")),
                Shortcut::new("/", tr("key-filter")),
                Shortcut::new("e", tr("key-edit")),
                Shortcut::new("d", tr("key-delete")),
            ],
        };
        if self.mode == Mode::Browse && self.last_deleted.is_some() {
            shortcuts.push(Shortcut::new("u", tr("key-undo")));
        }
        if self.mode == Mode::Browse {
            shortcuts.push(Shortcut::new("Esc", tr("key-back")));
        }
        shortcuts
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let shortcuts = self.active_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let max_name_len = self
            .filtered_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| {
                let rpe_len = if e.log.rpe.is_some() { 10 } else { 0 };
                e.practice_name.width() + rpe_len
            })
            .max()
            .unwrap_or(4);
        let name_col_w = (max_name_len + 2) as u16;
        let marker_w: u16 = 2; // "> " or "  "
        let date_col_w: u16 = 11; // "2026-05-12" + 1 padding
        let max_total_w = self
            .filtered_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| format!("{:.0}", e.total_metric()).len())
            .max()
            .unwrap_or(4);
        let max_label_w = self
            .filtered_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.metric_label().width())
            .max()
            .unwrap_or(2);
        let vol_col_w = (max_total_w + 1 + max_label_w + 1) as u16;

        // Vertical split: filter | main content (list + detail) | status | shortcuts
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),             // [0] filter
                Constraint::Min(1), // [1] main content area (list + detail side by side)
                Constraint::Length(1), // [2] status line
                Constraint::Length(shortcut_rows), // [3] shortcuts
            ])
            .split(area);

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
            filter_spans.extend(self.filter.visible_spans(v_chunks[0].width, 2, ACCENT));
        } else if !self.filter.value().is_empty() {
            filter_spans.push(Span::styled(self.filter.value().to_owned(), filter_style));
        }
        frame.render_widget(
            Paragraph::new(Line::from(filter_spans)).wrap(Wrap { trim: false }),
            v_chunks[0],
        );

        let list_area = v_chunks[1];
        let visible_items = (list_area.height as usize).saturating_sub(5);
        self.render_list(
            frame,
            list_area,
            visible_items,
            marker_w,
            name_col_w,
            date_col_w,
            vol_col_w,
            max_total_w,
            max_label_w,
        );

        // ── Floating detail popup ──
        if let Some(entry) = self.selected_entry().cloned() {
            let row_in_list = self.selected.saturating_sub(self.list_viewport.offset()) as u16 + 1; // +1 for header
            let popup_content = self.build_detail_lines(&entry);
            let popup_x = list_area.x + 4.min(list_area.width);
            let popup_w = list_area
                .width
                .saturating_sub(8)
                .max(30)
                .min(list_area.right().saturating_sub(popup_x));
            let popup_inner_width = popup_w.saturating_sub(2).max(1);
            let detail_spans = visual_spans(&popup_content, popup_inner_width);
            self.detail_content_height = detail_spans
                .last()
                .map(|span| span.start.saturating_add(span.height))
                .unwrap_or(0);
            let popup_h = if self.mode == Mode::Detail {
                list_area.height
            } else {
                (self.detail_content_height as u16 + 2)
                    .max(4)
                    .min(list_area.height / 2)
            };

            // Position below selected row; flip above if near bottom
            let popup_y = if self.mode == Mode::Detail {
                list_area.y
            } else {
                let below_y = list_area.y + row_in_list + 2;
                if below_y + popup_h > list_area.y + list_area.height {
                    list_area.y + row_in_list.saturating_sub(popup_h)
                } else {
                    below_y
                }
            };
            let popup_rect = fit_rect(
                list_area,
                ratatui::layout::Rect {
                    x: popup_x,
                    y: popup_y,
                    width: popup_w,
                    height: popup_h,
                },
            );

            let rpe = entry
                .log
                .rpe
                .map(|value| format!(" [RPE: {value}]"))
                .unwrap_or_default();
            let title = format!(
                " {} — {}{} ",
                entry.practice_name,
                entry.log.logged_at.format("%Y-%m-%d"),
                rpe
            );
            self.detail_state.focused = self.mode == Mode::Detail;
            if !self.detail_state.focused {
                self.detail_state.viewport.set_offset(0);
            }
            frame.render_widget(Clear, popup_rect);
            render_detail_panel(
                frame,
                popup_rect,
                title,
                popup_content,
                &mut self.detail_state,
                None,
            );
        }

        // Status + shortcuts
        render_status_line(frame, v_chunks[2], &self.status_msg);
        render_shortcuts(frame, v_chunks[3], &shortcuts);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_list(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        visible: usize,
        marker_w: u16,
        name_col_w: u16,
        date_col_w: u16,
        vol_col_w: u16,
        max_total_w: usize,
        max_label_w: usize,
    ) {
        self.list_viewport.set_height(visible);
        if !self.filtered_indices.is_empty() {
            self.list_viewport.reveal(VisualSpan {
                start: self.selected,
                height: 1,
            });
        }
        self.list_viewport.clamp(self.filtered_indices.len());
        let scroll_offset = self.list_viewport.offset();
        let overflow = self
            .list_viewport
            .overflow(self.filtered_indices.len())
            .suffix();
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    format!("{}{}", tr("history-title"), overflow),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.filtered_indices.is_empty() {
            let no_entries_text = format!("  {}", tr("history-no-entries"));
            let empty_content = Line::from(Span::styled(
                no_entries_text,
                Style::default().fg(Color::Gray),
            ));
            let empty = Paragraph::new(empty_content).wrap(Wrap { trim: false });
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
        for (fi, &entry_idx) in self
            .filtered_indices
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible)
        {
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

            let mut name_spans = vec![Span::styled(entry.practice_name.clone(), style)];
            if let Some(rpe) = entry.log.rpe {
                name_spans.push(Span::styled(
                    format!(" [RPE: {}]", rpe),
                    Style::default().fg(Color::Yellow),
                ));
            }

            rows.push(Row::new(vec![
                Cell::from(Span::styled(marker, style)),
                Cell::from(Span::styled(date, dim)),
                Cell::from(Line::from(name_spans)),
                Cell::from(Span::styled(
                    format!(
                        "{:>w$} {}{}",
                        total,
                        label,
                        " ".repeat(max_label_w.saturating_sub(label.width())),
                        w = max_total_w
                    ),
                    dim,
                )),
            ]));
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

        if !self.filtered_indices.is_empty() && self.selected >= scroll_offset {
            let row = (self.selected - scroll_offset) as u16 + 1; // +1 for header row
            highlight_row(frame, inner, row);
        }
    }

    fn build_detail_lines(&self, entry: &LogEntry) -> Vec<Line<'static>> {
        let mut lines: Vec<Line> = Vec::new();

        let mut total_reps = 0.0;
        for set in &entry.sets {
            let detail = match &set.data {
                SetData::Weighted { weight, reps } => {
                    total_reps += *reps as f64;
                    format!(
                        " {}",
                        tr_args(
                            "history-set-weighted",
                            &[
                                ("number", FluentValue::from(set.set_number as f64)),
                                ("weight", FluentValue::from(*weight)),
                                ("reps", FluentValue::from(*reps as f64)),
                            ]
                        )
                    )
                }
                SetData::Bodyweight { reps } => {
                    total_reps += *reps as f64;
                    format!(
                        " {}",
                        tr_args(
                            "history-set-bodyweight",
                            &[
                                ("number", FluentValue::from(set.set_number as f64)),
                                ("reps", FluentValue::from(*reps as f64)),
                            ]
                        )
                    )
                }
                SetData::Distance { distance } => {
                    format!(
                        " {}",
                        tr_args(
                            "history-set-distance",
                            &[
                                ("number", FluentValue::from(set.set_number as f64)),
                                ("distance", FluentValue::from(*distance)),
                            ]
                        )
                    )
                }
                SetData::Endurance { duration } => {
                    format!(
                        " {}",
                        tr_args(
                            "history-set-endurance",
                            &[
                                ("number", FluentValue::from(set.set_number as f64)),
                                ("duration", FluentValue::from(*duration)),
                            ]
                        )
                    )
                }
            };
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::White),
            )));
        }

        if matches!(
            entry.practice_type,
            crate::model::PracticeType::Weighted | crate::model::PracticeType::Bodyweight
        ) && total_reps > 0.0
        {
            lines.push(Line::from(""));
            let total_vol = entry.total_metric();
            let vol_label = entry.metric_label();
            let reps_label = tr("metric-reps");
            lines.push(Line::from(Span::styled(
                format!(
                    " {}",
                    tr_args(
                        "history-summary",
                        &[
                            ("reps", FluentValue::from(total_reps)),
                            ("reps_label", FluentValue::from(reps_label)),
                            ("vol", FluentValue::from(total_vol)),
                            ("vol_label", FluentValue::from(vol_label)),
                        ]
                    )
                ),
                Style::default().fg(Color::Gray),
            )));
        }

        if let Some(warm_up) = &entry.log.warm_up {
            lines.push(Line::from(Span::styled(
                format!(" {}: {}", tr("log-warmup-label"), warm_up),
                Style::default().fg(Color::Gray),
            )));
        }
        if let Some(cool_down) = &entry.log.cool_down {
            lines.push(Line::from(Span::styled(
                format!(" {}: {}", tr("log-cooldown-label"), cool_down),
                Style::default().fg(Color::Gray),
            )));
        }
        if let Some(note) = &entry.log.note {
            lines.push(Line::from(Span::styled(
                format!(" {}: {}", tr("log-note-label"), note),
                Style::default().fg(Color::Gray),
            )));
        }

        lines
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
            Mode::Detail => self.handle_detail(key),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_filter_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filtering = false;
            }
            _ if self.filter.handle_key(key) => {
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty()
                    && self.selected < self.filtered_indices.len() - 1
                {
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
                self.filter.clear();
                self.apply_filter();
                Action::None
            }
            KeyCode::Enter => {
                if !self.filtered_indices.is_empty() {
                    self.mode = Mode::Detail;
                    self.detail_state.focused = true;
                    self.detail_state.viewport.set_offset(0);
                }
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
                    return Action::Navigate(NavigationIntent::EditHistoryLog {
                        log_id: self.selected_entry().expect("selected entry").log.id,
                    });
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(entry) = self.last_deleted.take() {
                    match db.restore_log(&entry) {
                        Ok(_) => match self.refresh_preserving_context(db) {
                            Ok(()) => {
                                self.status_msg = Some((tr("status-restored"), false));
                            }
                            Err(error) => {
                                self.status_msg = Some((format!("Reload failed: {error}"), true));
                            }
                        },
                        Err(e) => {
                            self.last_deleted = Some(entry);
                            self.status_msg = Some((format!("Restore failed: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                if !self.filter.value().is_empty() {
                    self.filter.clear();
                    self.apply_filter();
                    Action::None
                } else {
                    Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard))
                }
            }
            _ => Action::None,
        }
    }

    fn handle_detail(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let content_height = if self.detail_content_height == 0 {
                    usize::MAX
                } else {
                    self.detail_content_height
                };
                self.detail_state.scroll_by(1, content_height);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_state
                    .scroll_by(-1, self.detail_content_height.max(1));
            }
            KeyCode::Esc | KeyCode::Enter => {
                self.mode = Mode::Browse;
                self.detail_state.focused = false;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        if key.code == KeyCode::Char('y') {
            if let Some(entry) = self.selected_entry() {
                let entry_clone = entry.clone();
                let log_id = entry.log.id;
                match db.delete_log(log_id) {
                    Ok(()) => {
                        self.last_deleted = Some(entry_clone);
                        match self.refresh_preserving_context(db) {
                            Ok(()) => {
                                self.status_msg = Some((tr("status-deleted-undo"), false));
                                self.mode = Mode::Browse;
                            }
                            Err(error) => {
                                self.status_msg = Some((format!("Reload failed: {error}"), true));
                            }
                        }
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Delete failed: {}", e), true));
                    }
                }
            }
        } else {
            self.mode = Mode::Browse;
        }
        Action::None
    }

    pub(crate) fn refresh_preserving_context(&mut self, db: &Database) -> anyhow::Result<()> {
        let selected_id = self.selected_entry().map(|entry| entry.log.id);
        self.entries = db.list_logs_all()?;
        let lower = self.filter.value().to_lowercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.practice_name.to_lowercase().contains(&lower))
            .map(|(index, _)| index)
            .collect();

        self.selected = selected_id
            .and_then(|id| {
                self.filtered_indices
                    .iter()
                    .position(|index| self.entries[*index].log.id == id)
            })
            .unwrap_or_else(|| {
                self.selected
                    .min(self.filtered_indices.len().saturating_sub(1))
            });
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        }
        self.list_viewport.clamp(self.filtered_indices.len());
        Ok(())
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn context_for_test(&self) -> (Option<i64>, Option<String>, String) {
        (
            self.selected_entry().map(|entry| entry.log.id),
            self.selected_entry()
                .and_then(|entry| entry.log.note.clone()),
            self.filter.value().to_owned(),
        )
    }
}

impl Reenter for HistoryScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        let selected_id = self.selected_entry().map(|entry| entry.log.id);
        let mut next = Self::new(db)?;

        next.filter = TextInput::from(self.filter.value().to_owned());
        next.apply_filter();
        next.selected = selected_id
            .and_then(|id| {
                next.filtered_indices
                    .iter()
                    .position(|index| next.entries[*index].log.id == id)
            })
            .unwrap_or_else(|| {
                self.selected
                    .min(next.filtered_indices.len().saturating_sub(1))
            });
        if next.filtered_indices.is_empty() {
            next.selected = 0;
        }

        next.list_viewport = self.list_viewport;
        next.list_viewport.clamp(next.filtered_indices.len());
        next.detail_state.viewport = self.detail_state.viewport;
        next.detail_content_height = self.detail_content_height;
        next.detail_state.viewport.clamp(next.detail_content_height);

        next.mode = if self.mode == Mode::Detail && next.selected_entry().is_some() {
            Mode::Detail
        } else {
            Mode::Browse
        };
        next.detail_state.focused = next.mode == Mode::Detail;
        next.filtering = false;
        next.status_msg = None;
        next.last_deleted = None;

        Ok(next)
    }
}

impl NavigationStatus for HistoryScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::lifecycle::Reenter;
    use crate::{model::PracticeType, tui::framework::test_support::render_to_string};
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render_history(screen: &mut HistoryScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    fn history_db_with_long_entry(set_count: usize) -> Database {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let sets = (0..set_count)
            .map(|_| SetData::Bodyweight { reps: 10 })
            .collect::<Vec<_>>();
        let logged_at = chrono::NaiveDate::from_ymd_opt(2026, 8, 10)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let warm_up = "Warm-up text ".repeat(30);
        let cool_down = "Cool-down text ".repeat(30);
        db.create_log_at(
            practice.id,
            &logged_at,
            &sets,
            Some("Final note text"),
            Some(&warm_up),
            Some(&cool_down),
            Some(8),
        )
        .unwrap();
        db
    }

    fn history_db_with_multiple_filtered_entries() -> Database {
        let db = Database::open_in_memory().unwrap();
        let bench = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let leg = db
            .create_practice("Leg Press", PracticeType::Bodyweight)
            .unwrap();
        for (practice, note) in [(&bench, "bench"), (&leg, "leg")] {
            db.create_log(
                practice.id,
                &[SetData::Bodyweight { reps: 10 }],
                Some(note),
                None,
                None,
                Some(7),
            )
            .unwrap();
        }
        db
    }

    fn enter_press_filter_and_select_second(screen: &mut HistoryScreen, db: &Database) {
        screen.handle_key(key(KeyCode::Char('/')), db);
        for character in "press".chars() {
            screen.handle_key(key(KeyCode::Char(character)), db);
        }
        screen.handle_key(key(KeyCode::Enter), db);
        screen.handle_key(key(KeyCode::Char('j')), db);
    }

    #[test]
    fn history_reentry_preserves_browsing_context_and_clears_transients() {
        let db = history_db_with_multiple_filtered_entries();
        let mut screen = HistoryScreen::new(&db).unwrap();
        enter_press_filter_and_select_second(&mut screen, &db);
        let selected_id = screen.selected_entry().unwrap().log.id;
        screen.mode = Mode::Detail;
        screen.detail_state.focused = true;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(1);
        screen.detail_state.viewport.set_height(2);
        screen.detail_state.viewport.set_offset(2);
        screen.detail_content_height = 5;
        screen.status_msg = Some(("old status".into(), false));
        screen.last_deleted = Some(screen.selected_entry().unwrap().clone());

        db.update_log(
            selected_id,
            &[SetData::Bodyweight { reps: 15 }],
            Some("fresh note"),
            None,
            None,
            None,
            Some(8),
        )
        .unwrap();
        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.selected_entry().unwrap().log.id, selected_id);
        assert_eq!(
            next.selected_entry().unwrap().log.note.as_deref(),
            Some("fresh note")
        );
        assert_eq!(next.filter.value(), "press");
        assert!(!next.filtering);
        assert_eq!(next.mode, Mode::Detail);
        assert!(next.detail_state.focused);
        assert_eq!(next.list_viewport.offset(), 1);
        assert_eq!(next.detail_state.viewport.offset(), 2);
        assert_eq!(next.detail_content_height, 5);
        assert!(next.status_msg.is_none());
        assert!(next.last_deleted.is_none());
    }

    #[test]
    fn history_reentry_clamps_selection_after_selected_logs_are_deleted() {
        let db = history_db_with_multiple_filtered_entries();
        let mut screen = HistoryScreen::new(&db).unwrap();
        enter_press_filter_and_select_second(&mut screen, &db);
        let selected_id = screen.selected_entry().unwrap().log.id;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(1);
        screen.detail_state.viewport.set_height(2);
        screen.detail_state.viewport.set_offset(4);
        screen.detail_content_height = 5;

        db.delete_log(selected_id).unwrap();
        let next = screen.reenter(&db).unwrap();

        assert!(!next.filtered_indices.is_empty());
        assert!(next.selected < next.filtered_indices.len());
        assert!(next.selected_entry().is_some());
        assert_eq!(next.list_viewport.offset(), 0);
        assert_eq!(next.detail_state.viewport.offset(), 3);

        let remaining_id = next.selected_entry().unwrap().log.id;
        db.delete_log(remaining_id).unwrap();
        let empty = next.reenter(&db).unwrap();

        assert!(empty.filtered_indices.is_empty());
        assert_eq!(empty.selected, 0);
        assert!(empty.selected_entry().is_none());
    }

    #[test]
    fn history_detail_focus_scrolls_to_last_content() {
        let db = history_db_with_long_entry(20);
        let mut screen = HistoryScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        for _ in 0..30 {
            screen.handle_key(key(KeyCode::Down), &db);
        }

        let output = render_history(&mut screen, 80, 24);

        assert_eq!(screen.mode, Mode::Detail);
        assert!(output.contains("Final note text"), "{output}");
        assert!(output.contains('↑'), "{output}");
    }

    #[test]
    fn history_long_filter_keeps_cursor_visible() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = HistoryScreen::new(&db).unwrap();
        screen.filtering = true;
        screen.filter = TextInput::from("练习".repeat(60));

        let output = render_history(&mut screen, 80, 24);

        assert!(output.contains('_'), "{output}");
        assert!(output.matches("练习").count() < 30, "{output}");
    }

    #[test]
    fn history_popup_stays_inside_small_terminal() {
        let db = history_db_with_long_entry(1);
        let mut screen = HistoryScreen::new(&db).unwrap();

        let output = render_history(&mut screen, 24, 10);

        assert!(!output.is_empty());
    }

    #[test]
    fn history_reserves_wrapped_shortcuts_after_delete() {
        let db = history_db_with_long_entry(1);
        let mut screen = HistoryScreen::new(&db).unwrap();
        screen.last_deleted = screen.entries.first().cloned();

        let output = render_history(&mut screen, 80, 24);

        assert!(output.contains("[u] Undo"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }

    #[test]
    fn refresh_preserves_history_context_while_reloading_selected_entry() {
        let db = Database::open_in_memory().unwrap();
        let bench_press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let leg_press = db
            .create_practice("Leg Press", PracticeType::Bodyweight)
            .unwrap();
        let pull_up = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let sets = [SetData::Bodyweight { reps: 10 }];
        let logged_at = |day| {
            chrono::NaiveDate::from_ymd_opt(2026, 8, day)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap()
        };
        db.create_log_at(
            bench_press.id,
            &logged_at(12),
            &sets,
            Some("newest press"),
            None,
            None,
            None,
        )
        .unwrap();
        let target_id = db
            .create_log_at(
                leg_press.id,
                &logged_at(11),
                &sets,
                Some("original target"),
                None,
                None,
                None,
            )
            .unwrap();
        db.create_log_at(
            pull_up.id,
            &logged_at(10),
            &sets,
            Some("outside filter"),
            None,
            None,
            None,
        )
        .unwrap();

        let mut screen = HistoryScreen::new(&db).unwrap();
        screen.filter = TextInput::from("press");
        screen.filtering = true;
        screen.apply_filter();
        screen.selected = 1;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(1);
        screen.mode = Mode::Detail;
        screen.detail_state.focused = true;
        screen.detail_state.viewport.set_offset(3);
        screen.detail_content_height = 8;
        screen.status_msg = Some(("preserve status".to_string(), false));
        screen.last_deleted = screen
            .entries
            .iter()
            .find(|entry| entry.log.id != target_id)
            .cloned();
        let deleted_id = screen.last_deleted.as_ref().unwrap().log.id;

        db.update_log(
            target_id,
            &[SetData::Bodyweight { reps: 12 }],
            Some("updated target"),
            None,
            None,
            None,
            Some(8),
        )
        .unwrap();
        screen.refresh_preserving_context(&db).unwrap();

        assert_eq!(screen.filter.value(), "press");
        assert!(screen.filtering);
        assert_eq!(screen.selected, 1);
        assert_eq!(screen.selected_entry().unwrap().log.id, target_id);
        assert_eq!(
            screen.selected_entry().unwrap().log.note.as_deref(),
            Some("updated target")
        );
        assert_eq!(screen.list_viewport.offset(), 1);
        assert_eq!(screen.mode, Mode::Detail);
        assert!(screen.detail_state.focused);
        assert_eq!(screen.detail_state.viewport.offset(), 3);
        assert_eq!(screen.detail_content_height, 8);
        assert_eq!(
            screen.status_msg,
            Some(("preserve status".to_string(), false))
        );
        assert_eq!(screen.last_deleted.as_ref().unwrap().log.id, deleted_id);
    }
}
