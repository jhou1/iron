use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::{AggregateStats, Database};
use crate::model::LogEntry;
use super::widgets::heatmap::Heatmap;
use super::{Action, Screen};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    today_entries: Vec<LogEntry>,
    stats: AggregateStats,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let today_entries = Self::load_today_entries(db)?;
        let stats = db.aggregate_stats(14)?;
        Ok(Self {
            heatmap_data,
            today_entries,
            stats,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.today_entries = Self::load_today_entries(db)?;
        self.stats = db.aggregate_stats(14)?;
        Ok(())
    }

    fn load_today_entries(db: &Database) -> anyhow::Result<Vec<LogEntry>> {
        let today_str = Local::now().format("%Y-%m-%d").to_string();
        let recent = db.list_logs_recent(1)?;
        let filtered: Vec<LogEntry> = recent
            .into_iter()
            .filter(|e| e.log.logged_at.format("%Y-%m-%d").to_string() == today_str)
            .collect();
        Ok(filtered)
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Main vertical layout: title | heatmap | panes | footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // heatmap header
                Constraint::Length(9), // heatmap (7 day rows + 1 legend + 1 padding)
                Constraint::Min(4),   // split panes
                Constraint::Length(2), // footer
            ])
            .split(area);

        // ── Title bar ──
        let title = Line::from(vec![
            Span::styled(" iron", Style::default().fg(ACCENT).bold()),
            Span::styled(" v0.1.0", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // ── Heatmap header ──
        let heatmap_header = Line::from(vec![
            Span::styled(" Training Activity", Style::default().fg(Color::White).bold()),
        ]);
        frame.render_widget(Paragraph::new(heatmap_header), chunks[1]);

        // ── Heatmap ──
        let heatmap_area = Rect {
            x: chunks[2].x + 1, // indent 1
            y: chunks[2].y,
            width: chunks[2].width.saturating_sub(2),
            height: chunks[2].height,
        };
        let heatmap = Heatmap::new(&self.heatmap_data, 52);
        frame.render_widget(heatmap, heatmap_area);

        // ── Split panes ──
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);

        self.render_today_pane(frame, panes[0]);
        self.render_stats_pane(frame, panes[1]);

        // ── Footer ──
        let footer = Line::from(vec![
            Span::styled(" [l]", Style::default().fg(ACCENT)),
            Span::styled(" Log  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[h]", Style::default().fg(ACCENT)),
            Span::styled(" History  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[t]", Style::default().fg(ACCENT)),
            Span::styled(" Trends  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[e]", Style::default().fg(ACCENT)),
            Span::styled(" Practices  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[q]", Style::default().fg(ACCENT)),
            Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[4]);
    }

    fn render_today_pane(&self, frame: &mut Frame, area: Rect) {
        let today_label = Local::now().format("Today \u{2014} %a %b %d, %Y").to_string();
        let block = Block::default()
            .title(Span::styled(today_label, Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.today_entries.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No entries yet today",
                Style::default().fg(Color::DarkGray),
            )));
            frame.render_widget(empty, inner);
        } else {
            let lines: Vec<Line> = self
                .today_entries
                .iter()
                .map(|entry| {
                    let sets_count = entry.sets.len();
                    let total = entry.total_metric();
                    let label = entry.metric_label();
                    Line::from(vec![
                        Span::styled(&entry.practice_name, Style::default().fg(GREEN)),
                        Span::styled(
                            format!("  {} sets, {:.0} {}", sets_count, total, label),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), inner);
        }
    }

    fn render_stats_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Last 14 Days", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let sessions = format!("{}", self.stats.sessions);
        let volume = format!("{:.0} kg", self.stats.total_volume);
        let reps = format!("{:.0}", self.stats.total_reps);
        let distance = format!("{:.1} km", self.stats.total_distance);
        let duration = format!("{:.0} min", self.stats.total_duration);
        let lines = vec![
            stat_line("Sessions", &sessions),
            stat_line("Volume", &volume),
            stat_line("Reps", &reps),
            stat_line("Distance", &distance),
            stat_line("Duration", &duration),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        }
    }
}

fn stat_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<12}", label), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(GREEN)),
    ])
}
