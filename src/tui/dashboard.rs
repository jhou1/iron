use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::widgets::heatmap::Heatmap;
use super::{
    centered_area,
    framework::{
        fit_rect, render_shortcuts, shortcut_height, truncate_to_width, Shortcut, TextInput,
    },
    render_gauge_line, render_status_line, Action, ModuleId, NavigationIntent, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
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

fn dashboard_block_inner(block: &Block<'_>, parent: Rect) -> Rect {
    fit_rect(parent, block.inner(parent))
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
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        if area.width == 0 || area.height == 0 {
            return;
        }

        let shortcuts = self.active_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),             // [0] logo header
                Constraint::Length(1),             // [1] spacer
                Constraint::Length(13),            // [2] heatmap preferred height
                Constraint::Length(1),             // [3] spacer
                Constraint::Length(7),             // [4] summary preferred height
                Constraint::Length(1),             // [5] spacer
                Constraint::Min(1),                // [6] split panes use remaining height
                Constraint::Length(1),             // [7] reserved status row
                Constraint::Length(shortcut_rows), // [8] complete dynamic footer
            ])
            .split(area);

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
        let heatmap_block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("dashboard-heatmap-title"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        if chunks[2].width > 0 && chunks[2].height > 0 {
            let heatmap_inner = dashboard_block_inner(&heatmap_block, chunks[2]);
            frame.render_widget(heatmap_block, chunks[2]);
            if heatmap_inner.width > 0 && heatmap_inner.height > 0 {
                let heatmap = Heatmap::new(&self.heatmap_data, 52, self.no_color);
                frame.render_widget(heatmap, heatmap_inner);
            }
        }

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

        // ── Status line ──
        if chunks[7].width > 0 && chunks[7].height > 0 {
            let status = if self.mode == DashboardMode::ConfirmQuit {
                Some((tr("dashboard-quit-confirm"), true))
            } else {
                self.status_msg.clone()
            };
            render_status_line(frame, chunks[7], &status);
        }

        // ── Footer ──
        if chunks[8].width > 0 && chunks[8].height > 0 {
            render_shortcuts(frame, chunks[8], &shortcuts);
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
        if area.width == 0 || area.height == 0 {
            return;
        }

        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("summary-title"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = dashboard_block_inner(&block, area);
        frame.render_widget(block, area);
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
    }

    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("dashboard-recent-title"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));

        let inner = dashboard_block_inner(&block, area);
        frame.render_widget(block, area);
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
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("dashboard-goals"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));

        let inner = dashboard_block_inner(&block, area);
        frame.render_widget(block, area);
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
                milestones.iter().filter(|m| m.completed).count() as f64 / milestones.len() as f64
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
    use crate::model::{PracticeType, SetData};
    use crate::tui::framework::test_support::render_to_string;
    use crate::tui::lifecycle::Reenter;
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
    fn dashboard_block_inners_remain_within_tiny_parents() {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));

        for parent in [Rect::new(0, 2, 40, 1), Rect::new(7, 9, 1, 1)] {
            let inner = dashboard_block_inner(&block, parent);
            assert!(
                inner.x >= parent.x
                    && inner.y >= parent.y
                    && inner.right() <= parent.right()
                    && inner.bottom() <= parent.bottom(),
                "inner {inner:?} escaped parent {parent:?}"
            );
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
