use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Constraint,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::{
    framework::{
        fit_rect, render_detail_panel, render_screen_frame_sections, render_scrollable_list_panel,
        visual_spans, DetailPanelState, ListPanelAreas, ListPanelContent, ScreenFrameSections,
        Shortcut, TextInput, ViewportState,
    },
    Action, ModuleId, NavigationIntent, StatusMessage, CONTENT_WIDTH,
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

#[derive(Debug, Clone, Copy)]
struct HistoryColumns {
    marker: usize,
    date: usize,
    name: usize,
    metric: usize,
    max_total: usize,
    max_label: usize,
}

fn clip_history_cell_span(text: &str, width: usize) -> String {
    let mut used = 0;
    UnicodeSegmentation::graphemes(text, true)
        .take_while(|grapheme| {
            let grapheme_width = UnicodeWidthStr::width(*grapheme);
            if used + grapheme_width > width {
                false
            } else {
                used += grapheme_width;
                true
            }
        })
        .collect()
}

fn append_cell(
    output: &mut Vec<Span<'static>>,
    spans: Vec<Span<'static>>,
    requested_width: usize,
    remaining: &mut usize,
    gap_after: bool,
) {
    let width = requested_width.min(*remaining);
    let mut unused = width;
    for span in spans {
        if unused == 0 {
            break;
        }
        let span_width = span.content.width();
        let text = clip_history_cell_span(span.content.as_ref(), unused);
        let rendered_width = text.width();
        if !text.is_empty() {
            output.push(Span::styled(text, span.style));
        }
        let consumed = span_width.min(unused);
        if rendered_width < consumed {
            output.push(Span::raw(" ".repeat(consumed - rendered_width)));
        }
        unused = unused.saturating_sub(consumed);
    }
    if unused > 0 {
        output.push(Span::raw(" ".repeat(unused)));
    }
    *remaining = remaining.saturating_sub(width);
    if gap_after && *remaining > 0 {
        output.push(Span::raw(" "));
        *remaining -= 1;
    }
}

fn history_list_content(
    entries: &[LogEntry],
    filtered_indices: &[usize],
    selected: usize,
    inner_width: u16,
    columns: HistoryColumns,
) -> ListPanelContent<'static> {
    if filtered_indices.is_empty() {
        return ListPanelContent {
            header: None,
            rows: Vec::new(),
            empty: Some(Line::from(Span::styled(
                format!("  {}", tr("history-no-entries")),
                Style::default().fg(Color::Gray),
            ))),
            selected: None,
        };
    }

    let mut widths = [columns.marker, columns.date, columns.name, columns.metric];
    let fixed_width = columns
        .marker
        .saturating_add(columns.date)
        .saturating_add(columns.metric)
        .saturating_add(widths.len().saturating_sub(1));
    widths[2] = columns
        .name
        .min((inner_width as usize).saturating_sub(fixed_width));

    let build_line = |cells: Vec<Vec<Span<'static>>>| {
        let mut remaining = inner_width as usize;
        let mut output = Vec::new();
        let cell_count = cells.len();
        for (index, (cell, width)) in cells.into_iter().zip(widths).enumerate() {
            append_cell(
                &mut output,
                cell,
                width,
                &mut remaining,
                index + 1 < cell_count,
            );
        }
        Line::from(output)
    };

    let header_style = Style::default().fg(Color::White).bold();
    let header = build_line(vec![
        vec![],
        vec![Span::styled(tr("history-col-date"), header_style)],
        vec![Span::styled(tr("history-col-practice"), header_style)],
        vec![Span::styled(tr("history-col-volume"), header_style)],
    ]);

    let rows = filtered_indices
        .iter()
        .enumerate()
        .filter_map(|(filtered_index, entry_index)| {
            let entry = entries.get(*entry_index)?;
            let selected_row = filtered_index == selected;
            let primary = if selected_row {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let dim = if selected_row {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            let total = format!("{:.0}", entry.total_metric());
            let label = entry.metric_label();
            let rpe = entry.log.rpe.map(|rpe| format!(" [RPE: {rpe}]"));
            let mut name = vec![Span::styled(entry.practice_name.clone(), primary)];
            if let Some(rpe) = rpe {
                name.push(Span::styled(rpe, Style::default().fg(Color::Yellow)));
            }
            Some(build_line(vec![
                vec![Span::styled(if selected_row { ">" } else { " " }, primary)],
                vec![Span::styled(
                    entry.log.logged_at.format("%Y-%m-%d").to_string(),
                    dim,
                )],
                name,
                vec![Span::styled(
                    format!(
                        "{:>width$} {}{}",
                        total,
                        label,
                        " ".repeat(columns.max_label.saturating_sub(label.width())),
                        width = columns.max_total,
                    ),
                    dim,
                )],
            ]))
        })
        .collect();

    ListPanelContent {
        header: Some(header),
        rows,
        empty: None,
        selected: Some(selected),
    }
}

fn history_detail_rect(
    list_area: ratatui::layout::Rect,
    row_in_list: u16,
    content_height: usize,
    focused: bool,
) -> ratatui::layout::Rect {
    let popup_x = list_area.x.saturating_add(4.min(list_area.width));
    let popup_w = list_area
        .width
        .saturating_sub(8)
        .max(30)
        .min(list_area.right().saturating_sub(popup_x));
    let popup_h = if focused {
        list_area.height
    } else {
        (content_height.min(u16::MAX as usize) as u16)
            .saturating_add(2)
            .max(4)
            .min(list_area.height / 2)
    };
    let popup_y = if focused {
        list_area.y
    } else {
        let below_y = list_area.y.saturating_add(row_in_list).saturating_add(2);
        if below_y.saturating_add(popup_h) > list_area.bottom() {
            list_area
                .y
                .saturating_add(row_in_list.saturating_sub(popup_h))
        } else {
            below_y
        }
    };

    fit_rect(
        list_area,
        ratatui::layout::Rect::new(popup_x, popup_y, popup_w, popup_h),
    )
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

    fn columns(&self) -> HistoryColumns {
        let max_name = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| entry.practice_name.width() + usize::from(entry.log.rpe.is_some()) * 10)
            .max()
            .unwrap_or(4);
        let max_total = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| format!("{:.0}", entry.total_metric()).len())
            .max()
            .unwrap_or(4);
        let max_label = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| entry.metric_label().width())
            .max()
            .unwrap_or(2);
        HistoryColumns {
            marker: 2,
            date: 11,
            name: max_name + 2,
            metric: max_total + 1 + max_label + 1,
            max_total,
            max_label,
        }
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
        let shortcuts = self.active_shortcuts();
        let sections: ScreenFrameSections = render_screen_frame_sections(
            frame,
            frame.area(),
            CONTENT_WIDTH,
            &self.status_msg,
            &shortcuts,
            &[Constraint::Length(1), Constraint::Min(1)],
            false,
        );
        let filter_area = sections.body[0];
        let list_area = sections.body[1];

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
            filter_spans.extend(self.filter.visible_spans(filter_area.width, 2, ACCENT));
        } else if !self.filter.value().is_empty() {
            filter_spans.push(Span::styled(self.filter.value().to_owned(), filter_style));
        }
        frame.render_widget(
            Paragraph::new(Line::from(filter_spans)).wrap(Wrap { trim: false }),
            filter_area,
        );

        let columns = self.columns();
        let entries = &self.entries;
        let filtered_indices = &self.filtered_indices;
        let selected = self.selected;
        let list_areas: ListPanelAreas = render_scrollable_list_panel(
            frame,
            list_area,
            tr("history-title"),
            &mut self.list_viewport,
            |inner| history_list_content(entries, filtered_indices, selected, inner.width, columns),
        );

        // ── Floating detail popup ──
        if let Some(entry) = self.selected_entry().cloned() {
            let row_in_list = self.selected.saturating_sub(self.list_viewport.offset()) as u16 + 1; // +1 for header
            let popup_content = self.build_detail_lines(&entry);
            let popup_inner_width =
                history_detail_rect(list_areas.area, row_in_list, 0, self.mode == Mode::Detail)
                    .width
                    .saturating_sub(2)
                    .max(1);
            let detail_spans = visual_spans(&popup_content, popup_inner_width);
            self.detail_content_height = detail_spans
                .last()
                .map(|span| span.start.saturating_add(span.height))
                .unwrap_or(0);
            let popup_rect = history_detail_rect(
                list_areas.area,
                row_in_list,
                self.detail_content_height,
                self.mode == Mode::Detail,
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
    use crate::model::{Log, PracticeType, Set, SetData};
    use crate::tui::framework::test_support::{
        render_to_buffer, render_to_snapshot, render_to_string,
    };
    use crate::tui::lifecycle::Reenter;
    use crossterm::event::KeyModifiers;
    use ratatui::{
        layout::Rect,
        widgets::{Cell, Row, Table},
    };

    fn fixed_time(month: u32, day: u32) -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, month, day)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap()
    }

    fn fixed_history_entry(
        id: i64,
        practice_name: impl Into<String>,
        practice_type: PracticeType,
        logged_at: chrono::NaiveDateTime,
        data: Vec<SetData>,
        rpe: Option<u8>,
        note: Option<String>,
    ) -> LogEntry {
        let practice_name = practice_name.into();
        LogEntry {
            log: Log {
                id,
                practice_id: id + 10_000,
                logged_at,
                note,
                warm_up: Some("Band warm-up with Unicode 训练 and controlled breathing".into()),
                cool_down: Some("Easy cooldown and mobility".into()),
                rpe,
            },
            practice_name,
            practice_type,
            sets: data
                .into_iter()
                .enumerate()
                .map(|(index, data)| Set {
                    id: id * 100 + index as i64,
                    log_id: id,
                    set_number: index as i32 + 1,
                    data,
                })
                .collect(),
            rpe,
        }
    }

    fn base_history_entries() -> Vec<LogEntry> {
        vec![
            fixed_history_entry(
                301,
                "Bench Press",
                PracticeType::Weighted,
                fixed_time(8, 13),
                vec![
                    SetData::Weighted {
                        weight: 60.0,
                        reps: 5,
                    },
                    SetData::Weighted {
                        weight: 67.5,
                        reps: 5,
                    },
                    SetData::Weighted {
                        weight: 72.5,
                        reps: 3,
                    },
                ],
                Some(8),
                Some("A deliberately long fixture note that wraps across visual rows. ".repeat(5)),
            ),
            fixed_history_entry(
                302,
                "Pull-up",
                PracticeType::Bodyweight,
                fixed_time(8, 12),
                vec![
                    SetData::Bodyweight { reps: 12 },
                    SetData::Bodyweight { reps: 10 },
                ],
                Some(7),
                Some("Controlled tempo".into()),
            ),
            fixed_history_entry(
                303,
                "Trail Run",
                PracticeType::Distance,
                fixed_time(8, 11),
                vec![SetData::Distance { distance: 5.5 }],
                None,
                None,
            ),
        ]
    }

    fn golden_history_screen(name: &str) -> HistoryScreen {
        std::env::set_var("LC_ALL", "C");
        crate::i18n::init();

        let entries = base_history_entries();
        let mut screen = HistoryScreen {
            filtered_indices: (0..entries.len()).collect(),
            entries,
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            mode: Mode::Browse,
            list_viewport: ViewportState::new(0),
            detail_state: DetailPanelState::new(false),
            detail_content_height: 0,
            status_msg: Some(("Fixture status".into(), false)),
            last_deleted: None,
        };

        match name {
            "browse" => {}
            "browse_with_undo" => {
                screen.last_deleted = Some(fixed_history_entry(
                    9_001,
                    "Deleted Log",
                    PracticeType::Bodyweight,
                    fixed_time(8, 1),
                    vec![SetData::Bodyweight { reps: 9 }],
                    Some(6),
                    Some("Undo fixture".into()),
                ));
            }
            "detail" => {
                screen.mode = Mode::Detail;
                screen.detail_state.focused = true;
                screen.detail_state.viewport.set_offset(2);
            }
            "filtering_long_input" => {
                let long = "训练".repeat(50);
                screen.entries[0].practice_name = long.clone();
                screen.filter = TextInput::from(long);
                screen.filtering = true;
                screen.filtered_indices = vec![0];
            }
            "no_filter_match" => {
                screen.filter = TextInput::from("missing practice".to_owned());
                screen.filtering = true;
                screen.filtered_indices.clear();
                screen.selected = 0;
            }
            "list_overflow" => {
                screen.entries = (0..30)
                    .map(|index| {
                        fixed_history_entry(
                            1_000 + index,
                            format!("Practice {:02}", index + 1),
                            PracticeType::Bodyweight,
                            fixed_time(7, 1) + chrono::Duration::days(index),
                            vec![SetData::Bodyweight {
                                reps: index as i32 + 1,
                            }],
                            None,
                            None,
                        )
                    })
                    .collect();
                screen.filtered_indices = (0..screen.entries.len()).collect();
                screen.selected = screen.filtered_indices.len() - 1;
                screen.list_viewport.set_offset(25);
            }
            "confirm_delete" => {
                screen.selected = 1;
                screen.mode = Mode::ConfirmDelete;
            }
            other => panic!("unknown History golden state: {other}"),
        }

        screen
    }

    fn history_golden(name: &str) -> &'static str {
        match name {
            "browse" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/browse.txt"
            )),
            "browse_with_undo" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/browse_with_undo.txt"
            )),
            "detail" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/detail.txt"
            )),
            "filtering_long_input" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/filtering_long_input.txt"
            )),
            "no_filter_match" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/no_filter_match.txt"
            )),
            "list_overflow" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/list_overflow.txt"
            )),
            "confirm_delete" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/history/confirm_delete.txt"
            )),
            other => panic!("unknown History golden fixture: {other}"),
        }
    }

    fn legacy_history_table_buffer(
        entries: &[LogEntry],
        filtered_indices: &[usize],
        selected: usize,
        inner_width: u16,
        columns: HistoryColumns,
    ) -> ratatui::buffer::Buffer {
        let header_style = Style::default().fg(Color::White).bold();
        let header = Row::new(vec![
            Cell::from(""),
            Cell::from(Span::styled(tr("history-col-date"), header_style)),
            Cell::from(Span::styled(tr("history-col-practice"), header_style)),
            Cell::from(Span::styled(tr("history-col-volume"), header_style)),
        ]);
        let rows = filtered_indices
            .iter()
            .enumerate()
            .filter_map(|(filtered_index, entry_index)| {
                let entry = entries.get(*entry_index)?;
                let selected_row = filtered_index == selected;
                let primary = if selected_row {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::White)
                };
                let dim = if selected_row {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let mut name = vec![Span::styled(entry.practice_name.clone(), primary)];
                if let Some(rpe) = entry.log.rpe {
                    name.push(Span::styled(
                        format!(" [RPE: {rpe}]"),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                let total = format!("{:.0}", entry.total_metric());
                let label = entry.metric_label();
                Some(Row::new(vec![
                    Cell::from(Span::styled(if selected_row { ">" } else { " " }, primary)),
                    Cell::from(Span::styled(
                        entry.log.logged_at.format("%Y-%m-%d").to_string(),
                        dim,
                    )),
                    Cell::from(Line::from(name)),
                    Cell::from(Span::styled(
                        format!(
                            "{:>width$} {}{}",
                            total,
                            label,
                            " ".repeat(columns.max_label.saturating_sub(label.width())),
                            width = columns.max_total,
                        ),
                        dim,
                    )),
                ]))
            })
            .collect::<Vec<_>>();
        let height = rows.len().saturating_add(1).min(u16::MAX as usize) as u16;
        let table = Table::new(
            rows,
            [
                Constraint::Length(columns.marker.min(u16::MAX as usize) as u16),
                Constraint::Length(columns.date.min(u16::MAX as usize) as u16),
                Constraint::Length(columns.name.min(u16::MAX as usize) as u16),
                Constraint::Length(columns.metric.min(u16::MAX as usize) as u16),
            ],
        )
        .header(header);

        render_to_buffer(inner_width, height, |frame| {
            frame.render_widget(table.clone(), frame.area());
        })
    }

    fn line_history_table_buffer(
        entries: &[LogEntry],
        filtered_indices: &[usize],
        selected: usize,
        inner_width: u16,
        columns: HistoryColumns,
    ) -> ratatui::buffer::Buffer {
        let content =
            history_list_content(entries, filtered_indices, selected, inner_width, columns);
        let mut lines = content.header.into_iter().collect::<Vec<_>>();
        lines.extend(content.rows);
        let height = lines.len().min(u16::MAX as usize) as u16;
        render_to_buffer(inner_width, height, |frame| {
            frame.render_widget(
                Paragraph::new(lines.clone()).wrap(Wrap { trim: false }),
                frame.area(),
            );
        })
    }

    fn assert_history_row_matches_legacy_table(
        entries: &[LogEntry],
        filtered_indices: &[usize],
        selected: usize,
        compared_row: usize,
        inner_width: u16,
        columns: HistoryColumns,
    ) {
        let legacy =
            legacy_history_table_buffer(entries, filtered_indices, selected, inner_width, columns);
        let line =
            line_history_table_buffer(entries, filtered_indices, selected, inner_width, columns);
        let y = compared_row.saturating_add(1).min(u16::MAX as usize) as u16;
        for x in 0..inner_width {
            assert_eq!(
                line[(x, y)],
                legacy[(x, y)],
                "styled cell mismatch at ({x}, {y}) for inner width {inner_width}"
            );
        }
    }

    fn rect_is_within(parent: Rect, child: Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    #[test]
    fn history_detail_rect_preserves_below_flip_and_focused_geometry() {
        let list = Rect::new(0, 1, 80, 20);
        assert_eq!(
            history_detail_rect(list, 1, 5, false),
            Rect::new(4, 4, 72, 7)
        );
        assert_eq!(
            history_detail_rect(list, 17, 5, false),
            Rect::new(4, 11, 72, 7)
        );
        assert_eq!(
            history_detail_rect(list, 8, 5, true),
            Rect::new(4, 1, 72, 20)
        );
    }

    #[test]
    fn history_detail_rect_never_escapes_tiny_or_edge_parents() {
        for parent in [
            Rect::new(0, 0, 0, 0),
            Rect::new(5, 5, 1, 1),
            Rect::new(2, 3, 12, 4),
            Rect::new(u16::MAX - 2, u16::MAX - 2, 8, 8),
        ] {
            for focused in [false, true] {
                for content_height in [0, 5, usize::MAX] {
                    let popup = history_detail_rect(parent, u16::MAX, content_height, focused);
                    assert!(rect_is_within(parent, popup), "{parent:?} -> {popup:?}");
                }
            }
        }
    }

    #[test]
    fn history_states_match_golden_buffers() {
        for name in [
            "browse",
            "browse_with_undo",
            "detail",
            "filtering_long_input",
            "no_filter_match",
            "list_overflow",
            "confirm_delete",
        ] {
            let mut screen = golden_history_screen(name);
            let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
            let expected = history_golden(name);
            assert_eq!(actual, expected, "History {name} buffer changed");
        }
    }

    #[test]
    fn history_line_rows_show_rpe_when_the_legacy_name_cell_has_room() {
        let screen = golden_history_screen("browse");
        let columns = screen.columns();
        let content = history_list_content(
            &screen.entries,
            &screen.filtered_indices,
            screen.selected,
            76,
            columns,
        );

        let row = content.rows[0].clone();
        assert!(row.to_string().contains("Bench Press"));
        assert!(row.to_string().contains("[RPE: 8]"));
        assert!(row.to_string().contains(&screen.entries[0].metric_label()));
        assert!(row.width() <= 76);
    }

    #[test]
    fn history_line_rows_let_a_long_unicode_name_hide_the_rpe_suffix() {
        let screen = golden_history_screen("filtering_long_input");
        let columns = screen.columns();
        let content = history_list_content(
            &screen.entries,
            &screen.filtered_indices,
            screen.selected,
            76,
            columns,
        );

        assert!(content
            .header
            .as_ref()
            .unwrap()
            .to_string()
            .contains(&tr("history-col-date")));
        assert_eq!(content.rows.len(), 1);
        let row = content.rows[0].clone();
        assert!(row.to_string().contains("训练"));
        assert!(!row.to_string().contains("[RPE: 8]"));
        assert!(row.to_string().contains(&screen.entries[0].metric_label()));
        assert!(row.width() <= 76);
    }

    #[test]
    fn history_long_unselected_names_match_legacy_table_clipping() {
        let selected = 8;
        let compared_row = 9;
        let entries = (0..10)
            .map(|index| {
                let name = if index == compared_row {
                    "Long ASCII practice 训练组合 ".repeat(8)
                } else {
                    format!("Practice {index:02}")
                };
                fixed_history_entry(
                    4_000 + index as i64,
                    name,
                    PracticeType::Bodyweight,
                    fixed_time(8, 1) + chrono::Duration::days(index as i64),
                    vec![SetData::Bodyweight { reps: 10 }],
                    Some(9),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let filtered_indices = (0..entries.len()).collect::<Vec<_>>();
        let screen = HistoryScreen {
            filtered_indices: filtered_indices.clone(),
            entries: entries.clone(),
            filter: TextInput::new(),
            filtering: false,
            selected,
            mode: Mode::Browse,
            list_viewport: ViewportState::new(0),
            detail_state: DetailPanelState::new(false),
            detail_content_height: 0,
            status_msg: None,
            last_deleted: None,
        };
        let columns = screen.columns();
        let compared_y = compared_row as u16 + 1;
        let popup = history_detail_rect(
            Rect::new(0, 0, 76, entries.len() as u16 + 1),
            selected as u16 + 1,
            4,
            false,
        );
        assert!(
            popup.bottom() <= compared_y,
            "popup {popup:?} must flip above compared row {compared_y}"
        );

        for inner_width in [76, 34] {
            assert_history_row_matches_legacy_table(
                &entries,
                &filtered_indices,
                selected,
                compared_row,
                inner_width,
                columns,
            );
        }
    }

    fn assert_grapheme_cut_matches_legacy_table(practice_name: &str, name_cell_width: usize) {
        let entries = vec![fixed_history_entry(
            5_000,
            practice_name,
            PracticeType::Bodyweight,
            fixed_time(8, 1),
            vec![SetData::Bodyweight { reps: 10 }],
            None,
            None,
        )];
        let filtered_indices = vec![0];
        let screen = HistoryScreen {
            filtered_indices: filtered_indices.clone(),
            entries: entries.clone(),
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            mode: Mode::Browse,
            list_viewport: ViewportState::new(0),
            detail_state: DetailPanelState::new(false),
            detail_content_height: 0,
            status_msg: None,
            last_deleted: None,
        };
        let mut columns = screen.columns();
        columns.name = name_cell_width;
        let fixed_width = columns
            .marker
            .saturating_add(columns.date)
            .saturating_add(columns.metric)
            .saturating_add(3);
        let inner_width = fixed_width
            .saturating_add(name_cell_width)
            .min(u16::MAX as usize) as u16;

        assert_history_row_matches_legacy_table(
            &entries,
            &filtered_indices,
            0,
            0,
            inner_width,
            columns,
        );
    }

    #[test]
    fn history_flag_cut_matches_legacy_table_grapheme_clipping() {
        assert_grapheme_cut_matches_legacy_table("A🇨🇳Z", 2);
    }

    #[test]
    fn history_combining_cut_matches_legacy_table_grapheme_clipping() {
        assert_grapheme_cut_matches_legacy_table("e\u{301}Z", 1);
    }

    #[test]
    fn history_zwj_cut_matches_legacy_table_grapheme_clipping() {
        assert_grapheme_cut_matches_legacy_table("👩‍💻Z", 2);
    }

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
        assert_eq!(next.detail_state.viewport.offset(), 4);

        let remaining_id = next.selected_entry().unwrap().log.id;
        db.delete_log(remaining_id).unwrap();
        let empty = next.reenter(&db).unwrap();

        assert!(empty.filtered_indices.is_empty());
        assert_eq!(empty.selected, 0);
        assert!(empty.selected_entry().is_none());
    }

    #[test]
    fn history_reentry_defers_detail_clamp_until_actual_width_render() {
        let db = history_db_with_long_entry(20);
        let mut screen = HistoryScreen::new(&db).unwrap();
        screen.mode = Mode::Detail;
        screen.detail_state.focused = true;
        screen.detail_state.viewport.set_height(2);
        screen.detail_state.viewport.set_offset(usize::MAX);
        screen.detail_content_height = 1;

        let mut next = screen.reenter(&db).unwrap();

        assert_eq!(next.detail_state.viewport.offset(), usize::MAX);
        let _ = render_history(&mut next, 80, 24);
        assert_ne!(next.detail_state.viewport.offset(), usize::MAX);
    }

    #[test]
    fn history_delete_failure_retains_confirmation_and_context() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        let practice = db
            .create_practice("Press", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("keep"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut screen = HistoryScreen::new(&db).unwrap();
        let selected_id = screen.selected_entry().unwrap().log.id;
        screen.handle_key(key(KeyCode::Char('d')), &db);
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();

        screen.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(screen.mode, Mode::ConfirmDelete);
        assert_eq!(screen.selected_entry().unwrap().log.id, selected_id);
        assert!(screen.last_deleted.is_none());
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn history_restore_failure_restores_undo_payload_and_context() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        let practice = db
            .create_practice("Press", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("current"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut screen = HistoryScreen::new(&db).unwrap();
        let selected_id = screen.selected_entry().unwrap().log.id;
        let deleted = fixed_history_entry(
            9_002,
            "Retry",
            PracticeType::Bodyweight,
            fixed_time(8, 1),
            vec![SetData::Bodyweight { reps: 9 }],
            Some(6),
            Some("retry".into()),
        );
        screen.last_deleted = Some(deleted.clone());
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();

        screen.handle_key(key(KeyCode::Char('u')), &db);

        assert_eq!(screen.mode, Mode::Browse);
        assert_eq!(screen.selected_entry().unwrap().log.id, selected_id);
        assert_eq!(screen.last_deleted.as_ref().unwrap().log.id, deleted.log.id);
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn history_states_render_in_tiny_and_resized_terminals_without_panicking() {
        for name in [
            "browse",
            "browse_with_undo",
            "detail",
            "filtering_long_input",
            "no_filter_match",
            "list_overflow",
            "confirm_delete",
        ] {
            let mut screen = golden_history_screen(name);
            for (width, height) in [(1, 1), (12, 4), (24, 10), (40, 6), (80, 24), (20, 5)] {
                let _ = render_to_snapshot(width, height, |frame| screen.render(frame));
            }
            let _ = render_to_buffer(80, 24, |frame| screen.render(frame));
            if !screen.filtered_indices.is_empty() {
                assert!(screen.list_viewport.offset() <= screen.selected);
            } else {
                assert_eq!(screen.list_viewport.offset(), 0);
            }
        }
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
