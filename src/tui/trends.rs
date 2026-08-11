use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::widgets::sparkline::Sparkline;
use super::{
    centered_area,
    framework::{
        move_selection, render_shortcuts, shortcut_height, truncate_to_width, EdgeBehavior,
        Shortcut, TextInput, ViewportState, VisualSpan,
    },
    highlight_row, render_status_line, Action, ModuleId, NavigationIntent, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, Practice};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

pub struct TrendsScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter: TextInput,
    filtering: bool,
    selected: usize,
    list_viewport: ViewportState,
    chosen_practice: Option<Practice>,
    days_window: i64,
    entries: Vec<LogEntry>,
    needs_refresh: bool,
    status_msg: StatusMessage,
}

impl TrendsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let filtered_indices: Vec<usize> = (0..practices.len()).collect();
        let chosen = filtered_indices.first().map(|&i| practices[i].clone());
        Ok(Self {
            practices,
            filtered_indices,
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            list_viewport: ViewportState::new(0),
            chosen_practice: chosen,
            days_window: 90,
            entries: Vec::new(),
            needs_refresh: true,
            status_msg: None,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let shortcuts = vec![
            Shortcut::new("j/k", tr("key-navigate")),
            Shortcut::new("h/l", tr("key-window")),
            Shortcut::new("/", tr("key-filter")),
            Shortcut::new("Esc", tr("key-back")),
        ];
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),             // filter bar
                Constraint::Percentage(40),        // practice list
                Constraint::Min(5),                // sparkline chart
                Constraint::Length(1),             // status
                Constraint::Length(shortcut_rows), // footer
            ])
            .split(area);

        self.render_practice_list(frame, chunks[0], chunks[1]);
        self.render_sparkline_panel(frame, chunks[2]);

        render_status_line(frame, chunks[3], &self.status_msg);
        render_shortcuts(frame, chunks[4], &shortcuts);
    }

    fn render_practice_list(
        &mut self,
        frame: &mut Frame,
        filter_area: ratatui::layout::Rect,
        list_area: ratatui::layout::Rect,
    ) {
        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");

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

        // Bordered list
        let visible_rows = list_area.height.saturating_sub(5) as usize;
        self.list_viewport.set_height(visible_rows);
        if !self.filtered_indices.is_empty() {
            self.list_viewport.reveal(VisualSpan {
                start: self.selected,
                height: 1,
            });
        }
        self.list_viewport.clamp(self.filtered_indices.len());
        let overflow = self
            .list_viewport
            .overflow(self.filtered_indices.len())
            .suffix();
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    format!("{}{}", tr("trends-title"), overflow),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(list_area);
        frame.render_widget(block, list_area);

        let max_name_width = inner.width.saturating_sub(type_header.width() as u16 + 5) as usize;
        let desired_name_width = self
            .practices
            .iter()
            .map(|practice| practice.name.width())
            .max()
            .unwrap_or(0)
            .max(name_header.width());
        let name_width = desired_name_width.min(max_name_width);
        let col_width = name_width.saturating_add(2);
        let header_padding = col_width.saturating_sub(name_header.width());

        let hdr_style = Style::default().fg(Color::White).bold();
        let mut all_lines = vec![Line::from(vec![
            Span::raw("  "),
            Span::styled(&name_header, hdr_style),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&type_header, hdr_style),
        ])];

        let scroll = self.list_viewport.offset();
        let end = (scroll + visible_rows).min(self.filtered_indices.len());

        for i in scroll..end {
            let idx = self.filtered_indices[i];
            let practice = &self.practices[idx];
            let marker = if i == self.selected { "> " } else { "  " };
            let name_style = if i == self.selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let name = truncate_to_width(&practice.name, name_width);
            let padding = col_width.saturating_sub(name.width());
            all_lines.push(Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(name, name_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(
                    practice.practice_type.label(),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        frame.render_widget(Paragraph::new(all_lines).wrap(Wrap { trim: false }), inner);

        if !self.filtered_indices.is_empty() && self.selected >= scroll && self.selected < end {
            highlight_row(frame, inner, (self.selected - scroll) as u16 + 1);
        }
    }

    fn render_sparkline_panel(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let practice = match self.chosen_practice.as_ref() {
            Some(p) => p,
            None => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::uniform(1))
                    .border_style(Style::default().fg(BORDER_COLOR));
                frame.render_widget(block, area);
                return;
            }
        };

        let metric_label = self
            .entries
            .first()
            .map(|e| e.metric_label())
            .unwrap_or_else(|| "\u{2014}".to_string());
        let block_title = format!(
            " {} ({}) [{}] ",
            practice.name,
            metric_label,
            practice.practice_type.label()
        );
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(&block_title, Style::default().fg(ACCENT).bold()),
                Span::styled("──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Inner layout: subtitle | chart | stats
        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        // Subtitle
        let subtitle = Line::from(Span::styled(
            format!(
                " {}",
                tr_args(
                    "trends-last-days",
                    &[("days", FluentValue::from(self.days_window as f64))]
                )
            ),
            Style::default().fg(Color::Gray),
        ));
        frame.render_widget(
            Paragraph::new(subtitle).wrap(Wrap { trim: false }),
            inner_chunks[0],
        );

        // Chart
        if self.entries.is_empty() {
            let msg = Line::from(Span::styled(
                format!("  {}", tr("trends-no-data")),
                Style::default().fg(Color::Gray),
            ));
            frame.render_widget(
                Paragraph::new(msg).wrap(Wrap { trim: false }),
                inner_chunks[1],
            );
        } else {
            let chart_data: Vec<(String, f64)> = self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let day = e.log.logged_at.format("%d").to_string();
                    let label = if i == 0
                        || e.log.logged_at.month() != self.entries[i - 1].log.logged_at.month()
                    {
                        let month = e.log.logged_at.format("%b").to_string();
                        format!("{}\n{}", day, month)
                    } else {
                        day
                    };
                    (label, e.total_metric())
                })
                .collect();
            let sparkline = Sparkline::new(chart_data);
            frame.render_widget(sparkline, inner_chunks[1]);
        }

        // Stats
        if !self.entries.is_empty() {
            let (avg, peak, trend_pct) = self.stats();
            let trend_color = if trend_pct >= 0.0 { GREEN } else { RED };
            let trend_sign = if trend_pct >= 0.0 { "+" } else { "" };

            let stats_line = Line::from(vec![
                Span::styled(
                    format!(
                        "  {}",
                        tr_args(
                            "trends-avg",
                            &[("value", FluentValue::from(format!("{:.1}", avg)))]
                        )
                    ),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    tr_args(
                        "trends-peak",
                        &[("value", FluentValue::from(format!("{:.1}", peak)))],
                    ),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    tr_args(
                        "trends-trend",
                        &[
                            ("sign", FluentValue::from(trend_sign.to_string())),
                            ("value", FluentValue::from(format!("{:.1}", trend_pct))),
                        ],
                    ),
                    Style::default().fg(trend_color),
                ),
            ]);
            frame.render_widget(
                Paragraph::new(stats_line).wrap(Wrap { trim: false }),
                inner_chunks[2],
            );
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        if self.filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.filtering = false;
                }
                _ if self.filter.handle_key(key) => {
                    self.apply_filter();
                    self.load_chart_with_status(db);
                }
                _ => {}
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard)),
            KeyCode::Char('/') => {
                self.filtering = true;
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = move_selection(
                        self.selected,
                        self.filtered_indices.len(),
                        1,
                        EdgeBehavior::Wrap,
                    );
                    self.sync_chosen();
                    self.load_chart_with_status(db);
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.filtered_indices.is_empty() {
                    self.selected = move_selection(
                        self.selected,
                        self.filtered_indices.len(),
                        -1,
                        EdgeBehavior::Wrap,
                    );
                    self.sync_chosen();
                    self.load_chart_with_status(db);
                }
                Action::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.days_window > 30 {
                    self.days_window -= 30;
                    self.needs_refresh = true;
                    self.load_chart_with_status(db);
                }
                Action::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.days_window < 365 {
                    self.days_window += 30;
                    if self.days_window > 365 {
                        self.days_window = 365;
                    }
                    self.needs_refresh = true;
                    self.load_chart_with_status(db);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn sync_chosen(&mut self) {
        self.chosen_practice = self
            .filtered_indices
            .get(self.selected)
            .map(|&idx| self.practices[idx].clone());
        self.needs_refresh = true;
    }

    fn apply_filter(&mut self) {
        let lower = self.filter.value().to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.list_viewport.set_offset(0);
        self.sync_chosen();
    }

    fn load_chart(&mut self, db: &Database) -> anyhow::Result<()> {
        self.entries = match &self.chosen_practice {
            Some(practice) => {
                let mut entries = db.list_logs_for_practice(practice.id, self.days_window)?;
                entries.sort_by_key(|entry| entry.log.logged_at);
                entries
            }
            None => Vec::new(),
        };
        self.needs_refresh = false;
        Ok(())
    }

    fn load_chart_with_status(&mut self, db: &Database) {
        if let Err(error) = self.load_chart(db) {
            self.status_msg = Some((format!("Error: {error}"), true));
        }
    }

    fn stats(&self) -> (f64, f64, f64) {
        let values: Vec<f64> = self.entries.iter().map(|e| e.total_metric()).collect();
        if values.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let sum: f64 = values.iter().sum();
        let len = values.len() as f64;
        let avg = sum / len;
        let peak = values.iter().cloned().fold(0.0f64, f64::max);

        let mid = values.len() / 2;
        let trend_pct = if mid > 0 {
            let first_sum: f64 = values[..mid].iter().sum();
            let first_avg = first_sum / mid as f64;
            let second_sum: f64 = values[mid..].iter().sum();
            let second_avg = second_sum / (values.len() - mid) as f64;
            if first_avg > 0.0 {
                ((second_avg - first_avg) / first_avg) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        (avg, peak, trend_pct)
    }
}

impl Reenter for TrendsScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        let selected_id = self.chosen_practice.as_ref().map(|practice| practice.id);
        let mut next = Self::new(db)?;

        next.filter = TextInput::from(self.filter.value().to_owned());
        next.days_window = self.days_window;
        next.apply_filter();
        next.selected = selected_id
            .and_then(|id| {
                next.filtered_indices
                    .iter()
                    .position(|index| next.practices[*index].id == id)
            })
            .unwrap_or_else(|| {
                self.selected
                    .min(next.filtered_indices.len().saturating_sub(1))
            });
        if next.filtered_indices.is_empty() {
            next.selected = 0;
        }
        next.sync_chosen();

        next.list_viewport = self.list_viewport;
        next.list_viewport.clamp(next.filtered_indices.len());
        next.filtering = false;
        next.status_msg = None;
        next.load_chart(db)?;

        Ok(next)
    }
}

impl NavigationStatus for TrendsScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PracticeType, SetData};
    use crate::tui::framework::test_support::render_to_string;
    use crate::tui::lifecycle::{NavigationStatus, Reenter};
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render_trends(screen: &mut TrendsScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    fn make_screen(count: usize, list_height: usize) -> TrendsScreen {
        let practices: Vec<Practice> = (0..count)
            .map(|i| Practice {
                id: i as i64 + 1,
                name: format!("Practice {}", i + 1),
                practice_type: PracticeType::Weighted,
                created_at: chrono::NaiveDateTime::default(),
                active: true,
            })
            .collect();
        let filtered_indices = (0..count).collect();
        TrendsScreen {
            practices,
            filtered_indices,
            filter: TextInput::new(),
            filtering: false,
            selected: 0,
            list_viewport: ViewportState::new(list_height),
            chosen_practice: None,
            days_window: 90,
            entries: Vec::new(),
            needs_refresh: false,
            status_msg: None,
        }
    }

    fn trends_db_with_press_and_pull_logs() -> Database {
        let db = Database::open_in_memory().unwrap();
        let press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let pull = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        for practice in [&press, &pull] {
            db.create_log(
                practice.id,
                &[SetData::Bodyweight { reps: 10 }],
                Some("trend fixture"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        }
        db
    }

    fn trends_db_with_improving_press_logs() -> Database {
        let db = Database::open_in_memory().unwrap();
        let press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let now = chrono::Local::now().naive_local();
        for (days_ago, reps) in [(4, 10), (3, 20), (2, 30), (1, 40)] {
            let logged_at = now - chrono::Duration::days(days_ago);
            db.create_log_at(
                press.id,
                &logged_at,
                &[SetData::Bodyweight { reps }],
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }
        db
    }

    fn reveal_selection(screen: &mut TrendsScreen) {
        screen.list_viewport.reveal(VisualSpan {
            start: screen.selected,
            height: 1,
        });
        screen.list_viewport.clamp(screen.filtered_indices.len());
    }

    #[test]
    fn trends_reentry_preserves_filter_practice_window_and_loads_chart() {
        let db = trends_db_with_press_and_pull_logs();
        let mut screen = TrendsScreen::new(&db).unwrap();
        screen.filter.set("press");
        screen.filtering = true;
        screen.apply_filter();
        screen.days_window = 30;
        let selected_id = screen.chosen_practice.as_ref().unwrap().id;
        screen.status_msg = Some(("old".into(), true));

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.filter.value(), "press");
        assert!(!next.filtering);
        assert_eq!(next.chosen_practice.as_ref().unwrap().id, selected_id);
        assert_eq!(next.days_window, 30);
        assert!(!next.needs_refresh);
        assert!(!next.entries.is_empty());
        assert!(next.status_msg.is_none());
    }

    #[test]
    fn multi_log_chart_keeps_improving_trend_chronological() {
        let db = trends_db_with_improving_press_logs();
        let mut screen = TrendsScreen::new(&db).unwrap();

        screen.load_chart(&db).unwrap();

        let metrics = screen
            .entries
            .iter()
            .map(LogEntry::total_metric)
            .collect::<Vec<_>>();
        let (_, _, trend_pct) = screen.stats();
        assert_eq!(metrics, vec![10.0, 20.0, 30.0, 40.0]);
        assert!(trend_pct > 0.0, "expected improvement, got {trend_pct}");
    }

    #[test]
    fn trends_reentry_selects_valid_filtered_practice_when_previous_is_inactive() {
        let db = trends_db_with_press_and_pull_logs();
        let mut screen = TrendsScreen::new(&db).unwrap();
        screen.filter.set("p");
        screen.apply_filter();
        let selected_id = screen.chosen_practice.as_ref().unwrap().id;
        db.set_practice_active(selected_id, false).unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.filter.value(), "p");
        assert_eq!(next.filtered_indices.len(), 1);
        assert_ne!(next.chosen_practice.as_ref().unwrap().id, selected_id);
        assert_eq!(next.chosen_practice.as_ref().unwrap().name, "Pull-up");
        assert!(!next.entries.is_empty());
    }

    #[test]
    fn trends_reentry_failure_leaves_cached_screen_unchanged() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            press.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("cached trend"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut screen = TrendsScreen::new(&db).unwrap();
        screen.filter.set("press");
        screen.filtering = true;
        screen.apply_filter();
        screen.days_window = 30;
        screen.load_chart(&db).unwrap();
        screen.status_msg = Some(("cached status".into(), false));
        let selected_id = screen.chosen_practice.as_ref().unwrap().id;
        let entries_len = screen.entries.len();

        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();

        assert!(screen.reenter(&db).is_err());
        assert_eq!(screen.filter.value(), "press");
        assert!(screen.filtering);
        assert_eq!(screen.chosen_practice.as_ref().unwrap().id, selected_id);
        assert_eq!(screen.days_window, 30);
        assert_eq!(screen.entries.len(), entries_len);
        assert_eq!(screen.status_msg, Some(("cached status".into(), false)));
    }

    #[test]
    fn failed_in_place_chart_load_keeps_context_and_cached_entries() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            press.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("cached trend"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut screen = TrendsScreen::new(&db).unwrap();
        screen.load_chart(&db).unwrap();
        let selected_id = screen.chosen_practice.as_ref().unwrap().id;
        let entry_ids = screen
            .entries
            .iter()
            .map(|entry| entry.log.id)
            .collect::<Vec<_>>();

        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();

        let action = screen.handle_key(key(KeyCode::Right), &db);

        assert_eq!(action, Action::None);
        assert_eq!(screen.chosen_practice.as_ref().unwrap().id, selected_id);
        assert_eq!(screen.days_window, 120);
        assert_eq!(
            screen
                .entries
                .iter()
                .map(|entry| entry.log.id)
                .collect::<Vec<_>>(),
            entry_ids
        );
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn trends_navigation_error_uses_status_line() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = TrendsScreen::new(&db).unwrap();

        screen.set_navigation_error("navigation failed".into());

        assert_eq!(screen.status_msg, Some(("navigation failed".into(), true)));
    }

    #[test]
    fn filtering_to_no_practice_clears_selection_and_chart() {
        let db = trends_db_with_press_and_pull_logs();
        let mut screen = TrendsScreen::new(&db).unwrap();
        screen.load_chart(&db).unwrap();
        assert!(!screen.entries.is_empty());
        screen.filtering = true;

        for character in "missing".chars() {
            screen.handle_key(key(KeyCode::Char(character)), &db);
        }

        assert!(screen.filtered_indices.is_empty());
        assert!(screen.chosen_practice.is_none());
        assert!(screen.entries.is_empty());
        assert!(!screen.needs_refresh);
    }

    #[test]
    fn scroll_follows_selection_down() {
        let mut s = make_screen(20, 5);
        // Select last visible item — no scroll needed
        s.selected = 4;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 0);

        // One past the viewport
        s.selected = 5;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 1);
    }

    #[test]
    fn scroll_follows_selection_up() {
        let mut s = make_screen(20, 5);
        s.selected = 10;
        s.list_viewport.set_offset(10);
        // Move up above scroll
        s.selected = 8;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 8);
    }

    #[test]
    fn scroll_stays_at_zero_when_all_fit() {
        let mut s = make_screen(5, 10);
        s.selected = 4;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 0);
    }

    #[test]
    fn scroll_wraps_to_top() {
        let mut s = make_screen(20, 5);
        s.selected = 19;
        s.list_viewport.set_offset(15);
        // Wrap to top
        s.selected = 0;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 0);
    }

    #[test]
    fn scroll_last_item_visible() {
        let mut s = make_screen(20, 5);
        s.selected = 19;
        reveal_selection(&mut s);
        assert_eq!(s.list_viewport.offset(), 15);
    }

    #[test]
    fn trends_long_filter_and_last_practice_remain_visible() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = make_screen(30, 5);
        screen.filtering = true;
        screen.filter = TextInput::from("X".repeat(100));
        let filter_output = render_trends(&mut screen, 80, 24);
        assert!(filter_output.contains('_'));
        assert!(filter_output.matches('X').count() < 80);

        screen.filtering = false;
        for _ in 1..30 {
            screen.handle_key(key(KeyCode::Down), &db);
        }
        let list_output = render_trends(&mut screen, 80, 24);
        assert!(list_output.contains("Practice 30"), "{list_output}");
    }
}
