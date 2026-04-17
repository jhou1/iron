use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::db::{AggregateStats, Database};
use crate::model::{Goal, LogEntry};
use super::widgets::heatmap::Heatmap;
use super::{Action, Screen};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const HEATMAP_CONTENT_WIDTH: u16 = 3 + 52 * 2; // day labels (3) + 52 weeks × 2 chars

const HEADER_ART: [&str; 6] = [
    r#" ___________             .__       .__                    _____          __  .__      .__  __          "#,
    r#" \__    ___/___________  |__| ____ |__| ____    ____     /  _  \   _____/  |_|__|__  _|__|/  |_ ___.__."#,
    r#"   |    |  \_  __ \__  \ |  |/    \|  |/    \  / ___\   /  /_\  \_/ ___\   __\  \  \/ /  \   __<   |  |"#,
    r#"   |    |   |  | \// __ \|  |   |  \  |   |  \/ /_/  > /    |    \  \___|  | |  |\   /|  ||  |  \___  |"#,
    r#"   |____|   |__|  (____  /__|___|  /__|___|  /\___  /  \____|__  /\___  >__| |__| \_/ |__||__|  / ____|"#,
    r#"                       \/        \/        \//_____/           \/     \/                        \/     "#,
];

pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
    quote: String,
    goals: Vec<Goal>,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let recent_entries = db.list_logs_recent(14)?;
        let stats = db.aggregate_stats(14)?;
        let quote = super::quotes::get_daily_quote();
        let goals = db.list_goals()?;
        Ok(Self {
            heatmap_data,
            recent_entries,
            stats,
            quote,
            goals,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.recent_entries = db.list_logs_recent(14)?;
        self.stats = db.aggregate_stats(14)?;
        self.quote = super::quotes::get_daily_quote();
        self.goals = db.list_goals()?;
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Pane height adapts to content
        let goals_lines: u16 = self.goals.iter()
            .map(|g| 1 + g.milestones.len() as u16)
            .sum::<u16>()
            .max(1);
        let pane_height = (self.recent_entries.len() as u16 + 4)
            .max(goals_lines + 2)
            .max(7);

        // Main vertical layout: title | heatmap | quote | panes | spacer | footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),            // [0] title
                Constraint::Length(7),            // [1] heatmap header ASCII art
                Constraint::Length(10),           // [2] heatmap
                Constraint::Length(1),            // [3] daily quote
                Constraint::Length(pane_height),  // [4] split panes
                Constraint::Min(0),              // [5] spacer
                Constraint::Length(2),            // [6] footer
            ])
            .split(area);

        // ── Title bar ──
        let title = Line::from(vec![
            Span::styled(" iron", Style::default().fg(ACCENT).bold()),
            Span::styled(format!(" v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // ── Heatmap header (ASCII art) ──
        let header_lines: Vec<Line> = HEADER_ART.iter()
            .map(|line| Line::from(Span::styled(*line, Style::default().fg(ACCENT))))
            .collect();
        frame.render_widget(Paragraph::new(header_lines), chunks[1]);

        // ── Heatmap ──
        let heatmap_area = Rect {
            x: chunks[2].x + 1,
            y: chunks[2].y,
            width: chunks[2].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[2].height,
        };
        let heatmap = Heatmap::new(&self.heatmap_data, 52);
        frame.render_widget(heatmap, heatmap_area);

        // ── Daily quote ──
        let quote_area = Rect {
            x: chunks[3].x + 1,
            y: chunks[3].y,
            width: chunks[3].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[3].height,
        };
        let quote_line = Line::from(Span::styled(
            format!("  \"{}\"", &self.quote),
            Style::default().fg(Color::Yellow),
        ));
        frame.render_widget(Paragraph::new(quote_line), quote_area);

        // ── Split panes (match heatmap content width) ──
        let panes_area = Rect {
            x: chunks[4].x + 1,
            y: chunks[4].y,
            width: chunks[4].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[4].height,
        };
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(panes_area);

        self.render_recent_pane(frame, panes[0]);
        self.render_goals_pane(frame, panes[1]);

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
        frame.render_widget(Paragraph::new(footer), chunks[6]);
    }

    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Last 14 Days", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Summary line
        let mut parts: Vec<Span> = Vec::new();
        parts.push(Span::styled(
            format!("{} sessions", self.stats.sessions),
            Style::default().fg(GREEN),
        ));
        if self.stats.total_volume > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} kg", self.stats.total_volume),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_reps > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} reps", self.stats.total_reps),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_distance > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.1} km", self.stats.total_distance),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_duration > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            parts.push(Span::styled(
                format!("{:.0} min", self.stats.total_duration),
                Style::default().fg(GREEN),
            ));
        }
        lines.push(Line::from(parts));

        // Separator
        let sep_width = inner.width as usize;
        lines.push(Line::from(Span::styled(
            "─".repeat(sep_width),
            Style::default().fg(Color::DarkGray),
        )));

        // Entry list
        if self.recent_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "No entries in the last 14 days",
                Style::default().fg(Color::Gray),
            )));
        } else {
            for entry in &self.recent_entries {
                let date = entry.log.logged_at.format("%b %d").to_string();
                let sets_count = entry.sets.len();
                let total = entry.total_metric();
                let label = entry.metric_label();
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", date), Style::default().fg(Color::Gray)),
                    Span::styled(&entry.practice_name, Style::default().fg(GREEN)),
                    Span::styled(
                        format!("  {} sets, {:.0} {}", sets_count, total, label),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled("Goals", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.goals.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                "Press [g] to add goals",
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in &self.goals {
            lines.push(Line::from(Span::styled(
                format!("▸ {}", goal.title),
                Style::default().fg(Color::White).bold(),
            )));
            for ms in &goal.milestones {
                if ms.completed {
                    lines.push(Line::from(Span::styled(
                        format!("  ☑ {}", ms.title),
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  ☐ {}", ms.title),
                        Style::default().fg(Color::White),
                    )));
                }
            }
        }

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
