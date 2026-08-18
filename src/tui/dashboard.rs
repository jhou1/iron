use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::widgets::heatmap::Heatmap;
use super::{
    framework::{
        render_screen_frame_sections, render_static_panel, truncate_to_width, Shortcut, TextInput,
    },
    render_gauge_line, Action, ModuleId, NavigationIntent, StatusMessage, BORDER_COLOR,
    CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{Goal, LogEntry};
use fluent_bundle::FluentValue;

const ACCENT: Color = Color::Cyan;

#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardMode {
    Normal,
    ConfirmQuit,
    HrvInput,
}

pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    quote: String,
    goals: Vec<Goal>,
    mode: DashboardMode,
    hrv_today: Option<i32>,
    hrv_input: TextInput,
    status_msg: StatusMessage,
    no_color: bool,
    weekly_volume: f64,
    training_days: usize,
    consecutive_days: i64,
}

impl DashboardScreen {
    pub fn new(db: &Database, no_color: bool) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let recent_entries = db.list_logs_recent(7)?;
        let quotes = db.list_quotes()?;
        let quote = super::quotes::pick_random_quote(&quotes);
        let goals = db.list_goals()?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let hrv_today = db.get_daily_hrv(&today)?;
        let stats = db.aggregate_stats(7)?;
        let (training_days, consecutive_days) = Self::compute_streak(&heatmap_data);
        Ok(Self {
            heatmap_data,
            recent_entries,
            quote,
            goals,
            mode: DashboardMode::Normal,
            hrv_today,
            hrv_input: TextInput::new(),
            status_msg: None,
            no_color,
            weekly_volume: stats.total_volume,
            training_days,
            consecutive_days,
        })
    }

    fn active_shortcuts(&self) -> Vec<Shortcut> {
        match self.mode {
            DashboardMode::Normal => vec![
                Shortcut::new("l", tr("key-log")),
                Shortcut::new("h", tr("key-history")),
                Shortcut::new("t", tr("key-trends")),
                Shortcut::new("e", tr("key-practices")),
                Shortcut::new("g", tr("key-goals")),
                Shortcut::new("Q", tr("key-quotes")),
                Shortcut::new("v", tr("key-hrv")),
                Shortcut::new("Esc", tr("key-quit")),
            ],
            DashboardMode::ConfirmQuit => vec![
                Shortcut::new("y", tr("key-yes")),
                Shortcut::new("any", tr("key-cancel")),
            ],
            DashboardMode::HrvInput => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        self.render_on(frame, chrono::Local::now().date_naive());
    }

    fn render_on(&self, frame: &mut Frame, today: chrono::NaiveDate) {
        let shortcuts = self.active_shortcuts();
        let status = if self.mode == DashboardMode::ConfirmQuit {
            Some((tr("dashboard-quit-confirm"), true))
        } else {
            self.status_msg.clone()
        };
        let sections = render_screen_frame_sections(
            frame,
            frame.area(),
            CONTENT_WIDTH,
            &status,
            &shortcuts,
            &[
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(13),
                Constraint::Length(1),
                Constraint::Length(7),
                Constraint::Length(1),
                Constraint::Min(1),
            ],
            false,
        );
        let chunks = &sections.body;

        // ── Logo header (single line) ──
        let logo_text = tr("dashboard-logo-text");
        let logo_line = Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled("\u{26a1} ", Style::default().fg(Color::Yellow)),
            Span::styled(logo_text, Style::default().fg(Color::Yellow).bold()),
        ]);
        if chunks[0].width > 0 && chunks[0].height > 0 {
            frame.render_widget(Paragraph::new(logo_line), chunks[0]);
        }

        // ── Heatmap ──
        render_static_panel(
            frame,
            chunks[2],
            tr("dashboard-heatmap-title"),
            |frame, inner| {
                if inner.width > 0 && inner.height > 0 {
                    frame.render_widget(
                        Heatmap::with_today(&self.heatmap_data, 52, self.no_color, today),
                        inner,
                    );
                }
            },
        );

        // ── Training summary ──
        self.render_summary(frame, chunks[4]);

        // ── Split panes ──
        if chunks[6].width > 0 && chunks[6].height > 0 {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[6]);

            self.render_recent_pane(frame, panes[0]);
            self.render_goals_pane(frame, panes[1]);
        }
    }

    fn compute_streak(heatmap_data: &[(String, i64)]) -> (usize, i64) {
        let today = chrono::Local::now().date_naive();
        let cutoff_7 = (today - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let training_days = heatmap_data
            .iter()
            .filter(|(d, _)| d.as_str() > cutoff_7.as_str())
            .count();

        let dates: std::collections::HashSet<&str> =
            heatmap_data.iter().map(|(d, _)| d.as_str()).collect();
        let mut consecutive = 0i64;
        let mut check = today;
        while dates.contains(check.format("%Y-%m-%d").to_string().as_str()) {
            consecutive += 1;
            check -= chrono::Duration::days(1);
        }
        (training_days, consecutive)
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        render_static_panel(frame, area, tr("summary-title"), |frame, inner| {
            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let volume_tons = self.weekly_volume / 1000.0;
            let volume_text = tr_args(
                "summary-volume",
                &[("value", FluentValue::from(format!("{:.1}", volume_tons)))],
            );
            let consecutive_text = tr_args(
                "summary-consecutive",
                &[("days", FluentValue::from(self.consecutive_days))],
            );
            let frequency_text = tr_args(
                "summary-frequency",
                &[("days", FluentValue::from(self.training_days as i64))],
            );
            let recovery_spans = if self.mode == DashboardMode::HrvInput {
                let prefix = format!("{}: ", tr("dashboard-hrv-label"));
                let non_input_width = volume_text
                    .width()
                    .saturating_add(consecutive_text.width())
                    .saturating_add(prefix.width())
                    .saturating_add(frequency_text.width())
                    .saturating_add("  ".width().saturating_mul(3));
                let non_input_width = u16::try_from(non_input_width).unwrap_or(u16::MAX);
                let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::White))];
                spans.extend(
                    self.hrv_input
                        .visible_spans(inner.width, non_input_width, ACCENT),
                );
                spans
            } else {
                let recovery_text = if let Some(hrv) = self.hrv_today {
                    tr_args(
                        "summary-recovery",
                        &[("value", FluentValue::from(hrv as i64))],
                    )
                } else {
                    tr("summary-recovery-na")
                };
                vec![Span::styled(
                    recovery_text,
                    Style::default().fg(Color::White),
                )]
            };
            let sep = Span::styled("  ", Style::default());
            let mut summary_spans = vec![
                Span::styled(volume_text, Style::default().fg(Color::White)),
                sep.clone(),
                Span::styled(consecutive_text, Style::default().fg(Color::White)),
                sep.clone(),
            ];
            summary_spans.extend(recovery_spans);
            summary_spans.extend([
                sep,
                Span::styled(frequency_text, Style::default().fg(Color::White)),
            ]);
            let mut lines = vec![Line::from(summary_spans)];

            if !self.quote.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("> {}", self.quote),
                    Style::default().fg(Color::Yellow).italic(),
                )));
            }

            frame.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .alignment(ratatui::layout::Alignment::Center),
                inner,
            );
        });
    }

    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        render_static_panel(frame, area, tr("dashboard-recent-title"), |frame, inner| {
            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let mut lines: Vec<Line> = Vec::new();

            if self.recent_entries.is_empty() {
                lines.push(Line::from(Span::styled(
                    tr("dashboard-no-entries"),
                    Style::default().fg(Color::Gray),
                )));
            } else {
                let mut current_date = String::new();
                for entry in &self.recent_entries {
                    let date_key = entry.log.logged_at.format("%Y-%m-%d").to_string();
                    if date_key != current_date {
                        if !current_date.is_empty() {
                            lines.push(Line::from(""));
                        }
                        lines.push(Line::from(Span::styled(
                            date_key.clone(),
                            Style::default().fg(Color::White).bold(),
                        )));
                        current_date = date_key;
                    }
                    let total = format!("{:.0}", entry.total_metric());
                    let label = entry.metric_label();
                    let row_width = inner.width as usize;
                    let indent = truncate_to_width("    ", row_width);
                    let indent_width = indent.width();
                    let suffix_budget = row_width.saturating_sub(indent_width);
                    let suffix = truncate_to_width(&format!("{total} {label}"), suffix_budget);
                    let suffix_width = suffix.width();
                    let gap_width = usize::from(suffix_width > 0 && suffix_width < suffix_budget);
                    let name_width = row_width
                        .saturating_sub(indent_width)
                        .saturating_sub(suffix_width)
                        .saturating_sub(gap_width);
                    let practice_name = truncate_to_width(&entry.practice_name, name_width);
                    let gap = if practice_name.is_empty() || suffix.is_empty() {
                        ""
                    } else {
                        " "
                    };
                    lines.push(Line::from(vec![
                        Span::raw(indent),
                        Span::styled(practice_name, Style::default().fg(Color::Gray)),
                        Span::raw(gap),
                        Span::styled(suffix, Style::default().fg(Color::Gray)),
                    ]));
                }
            }

            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        });
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        render_static_panel(frame, area, tr("dashboard-goals"), |frame, inner| {
            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let active_goals: Vec<&Goal> = self.goals.iter().filter(|g| !g.completed).collect();

            if active_goals.is_empty() && self.mode == DashboardMode::Normal {
                let hint = Paragraph::new(Line::from(Span::styled(
                    tr("dashboard-press-g"),
                    Style::default().fg(Color::Gray),
                )));
                frame.render_widget(hint, inner);
                return;
            }

            let mut y = inner.y;
            for goal in &active_goals {
                if y >= inner.bottom() {
                    break;
                }
                let title_rect = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        truncate_to_width(&goal.title, inner.width as usize),
                        Style::default().fg(Color::White).bold(),
                    )))
                    .wrap(Wrap { trim: false }),
                    title_rect,
                );
                y = y.saturating_add(1);

                if y >= inner.bottom() {
                    break;
                }
                let milestones = &goal.milestones;
                let ratio = if milestones.is_empty() {
                    if goal.completed {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    milestones.iter().filter(|m| m.completed).count() as f64
                        / milestones.len() as f64
                };
                let pct = (ratio * 100.0).round() as u32;
                let gauge_rect = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                let suffix = truncate_to_width(&format!("  {pct}%"), inner.width as usize);
                let bar_width = (inner.width as usize).saturating_sub(suffix.width());
                let mut gauge_line = render_gauge_line(ratio, 0, 0, bar_width, 0);
                gauge_line
                    .spans
                    .push(Span::styled(suffix, Style::default().fg(Color::Gray)));
                frame.render_widget(Paragraph::new(gauge_line), gauge_rect);
                y = y.saturating_add(2);
            }
        });
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match self.mode {
            DashboardMode::Normal => self.handle_normal(key),
            DashboardMode::ConfirmQuit => {
                if key.code == KeyCode::Char('y') {
                    Action::Quit
                } else {
                    self.mode = DashboardMode::Normal;
                    Action::None
                }
            }
            DashboardMode::HrvInput => self.handle_hrv_input(key, db),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(NavigationIntent::NewLog),
            KeyCode::Char('h') => Action::Navigate(NavigationIntent::Open(ModuleId::History)),
            KeyCode::Char('t') => Action::Navigate(NavigationIntent::Open(ModuleId::Trends)),
            KeyCode::Char('e') => Action::Navigate(NavigationIntent::Open(ModuleId::Practices)),
            KeyCode::Char('g') => Action::Navigate(NavigationIntent::Open(ModuleId::Goals)),
            KeyCode::Char('Q') => Action::Navigate(NavigationIntent::Open(ModuleId::Quotes)),
            KeyCode::Char('v') => {
                self.hrv_input
                    .set(self.hrv_today.map(|v| v.to_string()).unwrap_or_default());
                self.mode = DashboardMode::HrvInput;
                Action::None
            }
            KeyCode::Esc => {
                self.mode = DashboardMode::ConfirmQuit;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_hrv_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let parsed = self.hrv_input.value().parse::<i32>();
                match parsed {
                    Ok(hrv) if (0..=100).contains(&hrv) => {
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        match db.set_daily_hrv(&today, hrv) {
                            Ok(()) => {
                                self.hrv_today = Some(hrv);
                                self.hrv_input.clear();
                                self.mode = DashboardMode::Normal;
                            }
                            Err(error) => {
                                self.status_msg = Some((
                                    tr_args(
                                        "status-save-error",
                                        &[("msg", FluentValue::from(error.to_string()))],
                                    ),
                                    true,
                                ));
                            }
                        }
                    }
                    _ => self.status_msg = Some((tr("dashboard-hrv-invalid"), true)),
                }
                Action::None
            }
            KeyCode::Esc => {
                self.hrv_input.clear();
                self.mode = DashboardMode::Normal;
                Action::None
            }
            _ if self
                .hrv_input
                .handle_key_if(key, |character| character.is_ascii_digit()) =>
            {
                Action::None
            }
            _ => Action::None,
        }
    }
}

impl Reenter for DashboardScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        Self::new(db, self.no_color)
    }
}

impl NavigationStatus for DashboardScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Log, Milestone, PracticeType, Set, SetData};
    use crate::tui::framework::test_support::{
        render_to_buffer, render_to_snapshot, render_to_string,
    };
    use crate::tui::lifecycle::Reenter;
    use chrono::{NaiveDate, NaiveDateTime};
    use crossterm::event::{KeyEventKind, KeyModifiers};
    use rusqlite::Connection;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn render_dashboard(screen: &DashboardScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    fn dashboard_fixture_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 18).unwrap()
    }

    fn dashboard_fixture_time(day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, day)
            .unwrap()
            .and_hms_opt(6, 30, 0)
            .unwrap()
    }

    fn golden_dashboard_screen(name: &str) -> DashboardScreen {
        std::env::set_var("LC_ALL", "C");
        crate::i18n::init();
        let logged_at = dashboard_fixture_time(18);
        let goal_time = dashboard_fixture_time(1);
        let mut screen = DashboardScreen {
            heatmap_data: vec![("2026-08-17".into(), 1), ("2026-08-18".into(), 3)],
            recent_entries: vec![LogEntry {
                log: Log {
                    id: 701,
                    practice_id: 301,
                    logged_at,
                    note: Some("Fixture note".into()),
                    warm_up: None,
                    cool_down: None,
                    rpe: Some(8),
                },
                practice_name: "A deliberately long weighted practice name that must stay inside Recent".into(),
                practice_type: PracticeType::Weighted,
                sets: vec![Set {
                    id: 801,
                    log_id: 701,
                    set_number: 1,
                    data: SetData::Weighted { weight: 1234.0, reps: 5678 },
                }],
                rpe: Some(8),
            }],
            quote: "A deliberately long quote must wrap only inside Summary while the footer remains complete.".into(),
            goals: vec![Goal {
                id: 901,
                title: "A deliberately long active goal title that must stay inside Goals".into(),
                completed: false,
                position: 0,
                created_at: goal_time,
                completed_at: None,
                milestones: vec![
                    Milestone {
                        id: 902,
                        goal_id: 901,
                        title: "Done".into(),
                        completed: true,
                        position: 0,
                        created_at: goal_time,
                        completed_at: Some(goal_time),
                    },
                    Milestone {
                        id: 903,
                        goal_id: 901,
                        title: "Pending".into(),
                        completed: false,
                        position: 1,
                        created_at: goal_time,
                        completed_at: None,
                    },
                ],
            }],
            mode: DashboardMode::Normal,
            hrv_today: Some(64),
            hrv_input: TextInput::new(),
            status_msg: Some(("Dashboard fixture status".into(), false)),
            no_color: false,
            weekly_volume: 12_345.0,
            training_days: 4,
            consecutive_days: 3,
        };
        match name {
            "normal" => {}
            "hrv_input" => {
                screen.mode = DashboardMode::HrvInput;
                screen.hrv_input = TextInput::from("9".repeat(100));
                screen.status_msg = Some(("HRV fixture status".into(), true));
            }
            "confirm_quit" => {
                screen.mode = DashboardMode::ConfirmQuit;
                screen.status_msg = Some(("Hidden by confirmation".into(), true));
            }
            other => panic!("unknown Dashboard golden state: {other}"),
        }
        screen
    }

    fn dashboard_golden(name: &str) -> &'static str {
        match name {
            "normal" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/dashboard/normal.txt"
            )),
            "hrv_input" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/dashboard/hrv_input.txt"
            )),
            "confirm_quit" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/dashboard/confirm_quit.txt"
            )),
            other => panic!("unknown Dashboard golden fixture: {other}"),
        }
    }

    #[test]
    fn dashboard_states_match_golden_buffers() {
        for name in ["normal", "hrv_input", "confirm_quit"] {
            let screen = golden_dashboard_screen(name);
            let actual = render_to_snapshot(80, 35, |frame| {
                screen.render_on(frame, dashboard_fixture_today())
            });
            assert_eq!(
                actual,
                dashboard_golden(name),
                "Dashboard {name} buffer changed"
            );
        }
    }

    #[test]
    fn dashboard_reentry_loads_fresh_data_and_clears_transient_state() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, true).unwrap();
        screen.mode = DashboardMode::HrvInput;
        screen.hrv_input.set("88");
        screen.status_msg = Some(("old status".into(), true));
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        db.set_daily_hrv(&today, 64).unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.mode, DashboardMode::Normal);
        assert_eq!(next.hrv_today, Some(64));
        assert!(next.hrv_input.value().is_empty());
        assert!(next.status_msg.is_none());
        assert!(next.no_color);
    }

    #[test]
    fn failed_dashboard_reentry_does_not_mutate_cached_screen() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.mode = DashboardMode::HrvInput;
        screen.hrv_input.set("77");
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE quotes", [])
            .unwrap();

        assert!(screen.reenter(&db).is_err());
        assert_eq!(screen.mode, DashboardMode::HrvInput);
        assert_eq!(screen.hrv_input.value(), "77");
    }

    #[test]
    fn dashboard_renders_one_cell_small_and_resized_frames_without_panicking() {
        let screen = golden_dashboard_screen("normal");
        for (width, height) in [(1, 1), (40, 16), (80, 24), (80, 35), (107, 35)] {
            render_to_buffer(width, height, |frame| {
                screen.render_on(frame, dashboard_fixture_today());
            });
        }
    }

    #[test]
    fn dashboard_recalculates_complete_output_after_resize() {
        let screen = golden_dashboard_screen("normal");
        for (width, height) in [(80, 35), (80, 24), (40, 16), (107, 35)] {
            let output = render_dashboard(&screen, width, height);
            assert_eq!(output.lines().count(), height as usize);
        }
    }

    #[test]
    fn invalid_hrv_retains_input_mode_text_and_error() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Char('v')), &db);
        screen.hrv_input.set("101");

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, DashboardMode::HrvInput);
        assert_eq!(screen.hrv_input.value(), "101");
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn long_hrv_input_keeps_cursor_visible() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.mode = DashboardMode::HrvInput;
        screen.hrv_input = TextInput::from("9".repeat(100));

        let output = render_dashboard(&screen, 80, 35);

        assert!(output.contains('_'), "{output}");
    }

    #[test]
    fn long_hrv_input_stays_on_summary_row_and_preserves_quote() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.mode = DashboardMode::HrvInput;
        screen.hrv_input = TextInput::from("9".repeat(100));
        screen.quote = "Recovery quote remains visible".to_string();

        let output = render_dashboard(&screen, 80, 35);
        let summary_rows: Vec<&str> = output
            .lines()
            .filter(|line| {
                ["This Week:", "Streak:", "HRV:", "Frequency:"]
                    .iter()
                    .any(|marker| line.contains(marker))
            })
            .collect();

        assert_eq!(summary_rows.len(), 1, "{output}");
        for marker in ["This Week:", "Streak:", "HRV:", "_", "Frequency:"] {
            assert!(
                summary_rows[0].contains(marker),
                "missing {marker}\n{output}"
            );
        }
        assert!(
            output.contains("> Recovery quote remains visible"),
            "{output}"
        );
    }

    #[test]
    fn parse_failure_retains_hrv_input_with_exact_validation_error() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Char('v')), &db);
        screen.hrv_input.set("not-a-number");

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, DashboardMode::HrvInput);
        assert_eq!(screen.hrv_input.value(), "not-a-number");
        assert_eq!(screen.status_msg, Some((tr("dashboard-hrv-invalid"), true)));
    }

    #[test]
    fn database_write_failure_retains_hrv_input_mode_text_and_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locked.db");
        let db = Database::open(&path).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        db.set_daily_hrv(&today, 60).unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Char('v')), &db);
        screen.hrv_input.set("75");
        let lock = Connection::open(&path).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, DashboardMode::HrvInput);
        assert_eq!(screen.hrv_input.value(), "75");
        assert_eq!(screen.hrv_today, Some(60));
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));

        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(db.get_daily_hrv(&today).unwrap(), Some(60));
    }

    #[test]
    fn successful_hrv_save_updates_screen_database_and_exits_input_mode() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Char('v')), &db);
        screen.hrv_input.set("75");

        screen.handle_key(key(KeyCode::Enter), &db);

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(screen.mode, DashboardMode::Normal);
        assert_eq!(screen.hrv_input.value(), "");
        assert_eq!(screen.hrv_today, Some(75));
        assert_eq!(db.get_daily_hrv(&today).unwrap(), Some(75));
    }

    #[test]
    fn dashboard_renders_complete_remaining_navigation_at_80_by_35() {
        let db = Database::open_in_memory().unwrap();
        let screen = DashboardScreen::new(&db, false).unwrap();
        let output = render_dashboard(&screen, 80, 35);

        for command in ["[l]", "[h]", "[t]", "[e]", "[g]", "[Q]", "[v]", "[Esc]"] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
        assert!(!output.contains("[w]"), "{output}");
    }

    #[test]
    fn dashboard_renders_small_rectangle_without_panicking() {
        let db = Database::open_in_memory().unwrap();
        let screen = DashboardScreen::new(&db, false).unwrap();
        let output = render_dashboard(&screen, 40, 16);

        assert!(!output.is_empty());
    }

    #[test]
    fn long_content_preserves_dashboard_sections_and_footer() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice(
                "A very long weighted practice name that must stay inside Recent",
                PracticeType::Weighted,
            )
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Weighted {
                weight: 1234.0,
                reps: 5678,
            }],
            None,
            None,
            None,
            None,
        )
        .unwrap();
        db.create_goal(
            "A very long goal title that must stay inside Goals and preserve the footer",
        )
        .unwrap();
        db.create_quote(
            "A very long quote must wrap only inside the actual Summary panel width while every Dashboard section and footer command remains visible.",
        )
        .unwrap();
        let screen = DashboardScreen::new(&db, false).unwrap();

        let output = render_dashboard(&screen, 80, 35);

        for section in ["Heatmap", "Summary", "Recent", "Goals"] {
            assert!(output.contains(section), "missing {section}\n{output}");
        }
        for command in ["[l]", "[h]", "[t]", "[e]", "[g]", "[Q]", "[v]", "[Esc]"] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
    }

    #[test]
    fn quit_confirmation_advertises_only_real_confirmation_commands() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Esc), &db);

        let output = render_dashboard(&screen, 80, 35);

        for command in ["[y]", "[any]"] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
        for inactive in ["[l]", "[h]", "[t]", "[e]", "[g]", "[Q]", "[v]", "[Esc]"] {
            assert!(
                !output.contains(inactive),
                "unexpected {inactive}\n{output}"
            );
        }
    }

    #[test]
    fn hrv_input_advertises_only_real_confirmation_commands() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = DashboardScreen::new(&db, false).unwrap();
        screen.handle_key(key(KeyCode::Char('v')), &db);

        let output = render_dashboard(&screen, 80, 35);

        for command in ["[Enter]", "[Esc]"] {
            assert!(output.contains(command), "missing {command}\n{output}");
        }
        for inactive in [
            "[l]", "[h]", "[t]", "[e]", "[g]", "[Q]", "[v]", "[y]", "[any]",
        ] {
            assert!(
                !output.contains(inactive),
                "unexpected {inactive}\n{output}"
            );
        }
    }
}
