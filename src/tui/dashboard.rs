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
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let recent_entries = db.list_logs_recent(14)?;
        let stats = db.aggregate_stats(14)?;
        Ok(Self {
            heatmap_data,
            recent_entries,
            stats,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.recent_entries = db.list_logs_recent(14)?;
        self.stats = db.aggregate_stats(14)?;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Pane height adapts to content: entries + 2 for borders, min 7 for stats pane
        let pane_height = (self.recent_entries.len() as u16 + 2).max(7);

        // Main vertical layout: title | heatmap | panes | spacer | footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),            // title
                Constraint::Length(2),            // heatmap header (with spacing)
                Constraint::Length(10),           // heatmap (1 month labels + 7 day rows + 1 legend + 1 padding)
                Constraint::Length(pane_height),  // split panes (sized to content)
                Constraint::Min(0),              // spacer absorbs excess
                Constraint::Length(2),            // footer pinned to bottom
            ])
            .split(area);

        // ── Title bar ──
        let title = Line::from(vec![
            Span::styled(" iron", Style::default().fg(ACCENT).bold()),
            Span::styled(" v0.1.0", Style::default().fg(Color::Gray)),
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

        self.render_recent_pane(frame, panes[0]);
        self.render_stats_pane(frame, panes[1]);

        // ── Footer ──
        let footer = Line::from(vec![
            Span::styled(" [l]", Style::default().fg(ACCENT)),
            Span::styled(" Log  ", Style::default().fg(Color::Gray)),
            Span::styled("[h]", Style::default().fg(ACCENT)),
            Span::styled(" History  ", Style::default().fg(Color::Gray)),
            Span::styled("[t]", Style::default().fg(ACCENT)),
            Span::styled(" Trends  ", Style::default().fg(Color::Gray)),
            Span::styled("[e]", Style::default().fg(ACCENT)),
            Span::styled(" Practices  ", Style::default().fg(Color::Gray)),
            Span::styled("[q]", Style::default().fg(ACCENT)),
            Span::styled(" Quit", Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[5]);
    }

    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Last 14 Days", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.recent_entries.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No entries in the last 14 days",
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(empty, inner);
        } else {
            let lines: Vec<Line> = self
                .recent_entries
                .iter()
                .map(|entry| {
                    let date = entry.log.logged_at.format("%b %d").to_string();
                    let sets_count = entry.sets.len();
                    let total = entry.total_metric();
                    let label = entry.metric_label();
                    Line::from(vec![
                        Span::styled(format!("{} ", date), Style::default().fg(Color::Gray)),
                        Span::styled(&entry.practice_name, Style::default().fg(GREEN)),
                        Span::styled(
                            format!("  {} sets, {:.0} {}", sets_count, total, label),
                            Style::default().fg(Color::Gray),
                        ),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), inner);
        }
    }

    fn render_stats_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Statistics", Style::default().fg(Color::White).bold()))
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
        Span::styled(format!("{:<12}", label), Style::default().fg(Color::Gray)),
        Span::styled(value.to_string(), Style::default().fg(GREEN)),
    ])
}
