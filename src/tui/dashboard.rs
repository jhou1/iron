use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::widgets::heatmap::Heatmap;
use super::{
    centered_area, render_gauge_line, render_status_line, Action, Screen, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{Goal, LogEntry, Quote};
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
    quotes: Vec<Quote>,
    mode: DashboardMode,
    hrv_today: Option<i32>,
    hrv_input: String,
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
        let stats = db.aggregate_stats(7).unwrap_or(crate::db::AggregateStats {
            sessions: 0,
            total_volume: 0.0,
            total_reps: 0.0,
            total_distance: 0.0,
            total_duration: 0.0,
        });
        let (training_days, consecutive_days) = Self::compute_streak(&heatmap_data);
        Ok(Self {
            heatmap_data,
            recent_entries,
            quote,
            goals,
            quotes,
            mode: DashboardMode::Normal,
            hrv_today,
            hrv_input: String::new(),
            status_msg: None,
            no_color,
            weekly_volume: stats.total_volume,
            training_days,
            consecutive_days,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.recent_entries = db.list_logs_recent(7)?;
        self.quotes = db.list_quotes()?;
        self.quote = super::quotes::pick_random_quote(&self.quotes);
        self.goals = db.list_goals()?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.hrv_today = db.get_daily_hrv(&today)?;
        let stats = db.aggregate_stats(7).unwrap_or(crate::db::AggregateStats {
            sessions: 0,
            total_volume: 0.0,
            total_reps: 0.0,
            total_distance: 0.0,
            total_duration: 0.0,
        });
        let (td, cd) = Self::compute_streak(&self.heatmap_data);
        self.weekly_volume = stats.total_volume;
        self.training_days = td;
        self.consecutive_days = cd;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        // Pane height adapts to content
        let active_goal_count = self.goals.iter().filter(|g| !g.completed).count() as u16;
        let goals_lines: u16 = (active_goal_count * 2 + 1).max(2);

        // Recent pane: 1 title + per-group (1 date header + N entries + 1 blank separator)
        let date_groups = {
            let mut count = 0u16;
            let mut current_date = String::new();
            for entry in &self.recent_entries {
                let dk = entry.log.logged_at.format("%Y-%m-%d").to_string();
                if dk != current_date {
                    if !current_date.is_empty() {
                        count += 1; // blank separator
                    }
                    count += 1; // date header
                    current_date = dk;
                }
                count += 1; // entry line
            }
            count
        };
        let recent_lines = (date_groups + 1).max(2); // +1 for title
        let pane_height = recent_lines.max(goals_lines).max(7);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),           // [0] logo header
                Constraint::Length(1),           // [1] spacer
                Constraint::Length(13),          // [2] heatmap: 9 inner + 2 borders + 2 padding
                Constraint::Length(1),           // [3] spacer
                Constraint::Length(7),           // [4] summary: 3 inner + 2 borders + 2 padding
                Constraint::Length(1),           // [5] spacer
                Constraint::Length(pane_height), // [6] split panes
                Constraint::Length(1),           // [7] status line
                Constraint::Length(4),           // [8] footer (4 lines)
                Constraint::Min(0),              // [9] spacer
            ])
            .split(area);

        // ── Logo header (single line) ──
        let logo_text = tr("dashboard-logo-text");
        let logo_line = Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled("\u{26a1} ", Style::default().fg(Color::Yellow)),
            Span::styled(logo_text, Style::default().fg(Color::Yellow).bold()),
        ]);
        frame.render_widget(Paragraph::new(logo_line), chunks[0]);

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
        let heatmap_inner = heatmap_block.inner(chunks[2]);
        frame.render_widget(heatmap_block, chunks[2]);
        let heatmap = Heatmap::new(&self.heatmap_data, 52, self.no_color);
        frame.render_widget(heatmap, heatmap_inner);

        // ── Training summary ──
        self.render_summary(frame, chunks[4]);

        // ── Split panes ──
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[6]);

        self.render_recent_pane(frame, panes[0]);
        self.render_goals_pane(frame, panes[1]);

        // ── Status line ──
        render_status_line(frame, chunks[7], &self.status_msg);

        // ── Footer ──
        let footer_lines: Vec<Line> = if self.mode == DashboardMode::ConfirmQuit {
            vec![Line::from(vec![
                Span::styled(
                    format!(" {} ", tr("dashboard-quit-confirm")),
                    Style::default().fg(Color::Red),
                ),
                Span::styled("[y]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-yes")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[any]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::DarkGray),
                ),
            ])]
        } else if self.mode == DashboardMode::Normal {
            let log_lbl = tr("footer-group-log");
            let rev_lbl = tr("footer-group-review");
            let man_lbl = tr("footer-group-manage");
            let sys_lbl = tr("footer-group-system");
            let max_lbl = log_lbl.width().max(rev_lbl.width()).max(man_lbl.width()).max(sys_lbl.width());

            let log_pad = " ".repeat(max_lbl.saturating_sub(log_lbl.width()));
            let rev_pad = " ".repeat(max_lbl.saturating_sub(rev_lbl.width()));
            let man_pad = " ".repeat(max_lbl.saturating_sub(man_lbl.width()));
            let sys_pad = " ".repeat(max_lbl.saturating_sub(sys_lbl.width()));

            vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {}{}: ", log_pad, log_lbl),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[l]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!("{} ", tr("key-log")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[w]", Style::default().fg(ACCENT)),
                    Span::styled(tr("key-quick-log"), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!(" {}{}: ", rev_pad, rev_lbl),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[h]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!("{} ", tr("key-history")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[t]", Style::default().fg(ACCENT)),
                    Span::styled(tr("key-trends"), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!(" {}{}: ", man_pad, man_lbl),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[e]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!("{} ", tr("key-practices")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[g]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!("{} ", tr("key-goals")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Q]", Style::default().fg(ACCENT)),
                    Span::styled(tr("key-quotes"), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!(" {}{}: ", sys_pad, sys_lbl),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(tr("key-quit"), Style::default().fg(Color::DarkGray)),
                ]),
            ]
        } else {
            vec![Line::from(vec![
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
            ])]
        };
        frame.render_widget(Paragraph::new(footer_lines), chunks[8]);
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
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let volume_tons = self.weekly_volume / 1000.0;
        let volume_text = tr_args(
            "summary-volume",
            &[("value", FluentValue::from(format!("{:.1}", volume_tons)))],
        );
        let consecutive_text = tr_args(
            "summary-consecutive",
            &[("days", FluentValue::from(self.consecutive_days))],
        );
        let recovery_text = if let Some(hrv) = self.hrv_today {
            tr_args(
                "summary-recovery",
                &[("value", FluentValue::from(hrv as i64))],
            )
        } else {
            tr("summary-recovery-na")
        };
        let frequency_text = tr_args(
            "summary-frequency",
            &[("days", FluentValue::from(self.training_days as i64))],
        );

        let sep = Span::styled("  ", Style::default());
        let mut lines = vec![Line::from(vec![
            Span::styled(volume_text, Style::default().fg(Color::White)),
            sep.clone(),
            Span::styled(consecutive_text, Style::default().fg(Color::White)),
            sep.clone(),
            Span::styled(recovery_text, Style::default().fg(Color::White)),
            sep,
            Span::styled(frequency_text, Style::default().fg(Color::White)),
        ])];

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

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        if self.recent_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                tr("dashboard-no-entries"),
                Style::default().fg(Color::Gray),
            )));
        } else {
            let max_name = self
                .recent_entries
                .iter()
                .map(|e| e.practice_name.width())
                .max()
                .unwrap_or(0);
            let name_col = max_name + 2;
            let max_total_w = self
                .recent_entries
                .iter()
                .map(|e| format!("{:.0}", e.total_metric()).len())
                .max()
                .unwrap_or(0);
            let max_label_w = self
                .recent_entries
                .iter()
                .map(|e| e.metric_label().width())
                .max()
                .unwrap_or(0);

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
                let name_pad = name_col.saturating_sub(entry.practice_name.width());
                let num_pad = max_total_w.saturating_sub(total.len());
                let label_pad = max_label_w.saturating_sub(label.width());
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(&entry.practice_name, Style::default().fg(Color::Gray)),
                    Span::raw(" ".repeat(name_pad + num_pad)),
                    Span::styled(
                        format!("{} {}{}", total, label, " ".repeat(label_pad)),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
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

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let active_goals: Vec<&Goal> = self.goals.iter().filter(|g| !g.completed).collect();

        if active_goals.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                tr("dashboard-press-g"),
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut y = inner.y;
        for goal in &active_goals {
            if y >= inner.y + inner.height {
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
                    goal.title.clone(),
                    Style::default().fg(Color::White).bold(),
                )))
                .wrap(Wrap { trim: false }),
                title_rect,
            );
            y += 1;

            if y >= inner.y + inner.height {
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
            let bar_width = (inner.width as usize).saturating_sub(7);
            let mut gauge_line = render_gauge_line(ratio, 0, 0, bar_width, 0);
            gauge_line.spans.push(Span::styled(
                format!("  {}%", pct),
                Style::default().fg(Color::Gray),
            ));
            frame.render_widget(Paragraph::new(gauge_line), gauge_rect);
            y += 2;
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
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('g') => Action::Navigate(Screen::Goals),
            KeyCode::Char('w') => Action::Navigate(Screen::QuickLog),
            KeyCode::Char('Q') => Action::Navigate(Screen::Quotes),
            KeyCode::Char('v') => {
                self.hrv_input = self.hrv_today.map(|v| v.to_string()).unwrap_or_default();
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
                if let Ok(hrv) = self.hrv_input.parse::<i32>() {
                    if (0..=100).contains(&hrv) {
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        match db.set_daily_hrv(&today, hrv) {
                            Ok(()) => {
                                self.hrv_today = Some(hrv);
                            }
                            Err(e) => {
                                self.status_msg = Some((
                                    tr_args(
                                        "status-save-error",
                                        &[("msg", FluentValue::from(e.to_string()))],
                                    ),
                                    true,
                                ));
                            }
                        }
                        self.mode = DashboardMode::Normal;
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.hrv_input.clear();
                self.mode = DashboardMode::Normal;
                Action::None
            }
            KeyCode::Backspace => {
                self.hrv_input.pop();
                Action::None
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.hrv_input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::widgets::{Block, Borders, Padding};

    #[test]
    fn dashboard_layout_produces_adequate_inner_areas() {
        // Simulate dashboard layout with minimum content
        let area = Rect::new(0, 0, 80, 35);
        let pane_height: u16 = 2;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),           // logo header
                Constraint::Length(1),           // spacer
                Constraint::Length(13),          // heatmap
                Constraint::Length(1),           // spacer
                Constraint::Length(7),           // summary
                Constraint::Length(1),           // spacer
                Constraint::Length(pane_height), // split panes
                Constraint::Length(1),           // status line
                Constraint::Length(4),           // footer
                Constraint::Min(0),              // spacer
            ])
            .split(area);

        let heatmap_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        let heatmap_inner = heatmap_block.inner(chunks[2]);
        assert!(
            heatmap_inner.height >= 9,
            "heatmap inner height must be >= 9 (got {}). Increase heatmap constraint if padding is added.",
            heatmap_inner.height
        );

        let summary_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        let summary_inner = summary_block.inner(chunks[4]);
        assert!(
            summary_inner.height >= 3,
            "summary inner height must be >= 3 (got {}). Increase summary constraint if padding is added.",
            summary_inner.height
        );
    }
}
