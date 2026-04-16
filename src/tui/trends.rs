use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::db::Database;
use crate::model::{LogEntry, Practice};
use super::widgets::sparkline::Sparkline;
use super::{Action, Screen};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    SelectPractice,
    ViewChart,
}

pub struct TrendsScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filtering: bool,
    selected: usize,
    phase: Phase,
    chosen_practice: Option<Practice>,
    days_window: i64,
    entries: Vec<LogEntry>,
    needs_refresh: bool,
}

impl TrendsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        let filtered_indices = (0..practices.len()).collect();
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filtering: false,
            selected: 0,
            phase: Phase::SelectPractice,
            chosen_practice: None,
            days_window: 90,
            entries: Vec::new(),
            needs_refresh: false,
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        match self.phase {
            Phase::SelectPractice => self.render_select_practice(frame),
            Phase::ViewChart => self.render_view_chart(frame),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.phase {
            Phase::SelectPractice => self.handle_select_practice(key),
            Phase::ViewChart => self.handle_view_chart(key),
        }
    }

    /// Called by app.rs after handle_key to reload chart data from the database.
    pub fn refresh_chart(&mut self, db: &Database) {
        if !self.needs_refresh {
            return;
        }
        self.needs_refresh = false;

        if let Some(ref practice) = self.chosen_practice {
            match db.list_logs_for_practice(practice.id, self.days_window) {
                Ok(mut entries) => {
                    // Sort chronologically (oldest first) for the chart
                    entries.sort_by(|a, b| a.log.logged_at.cmp(&b.log.logged_at));
                    self.entries = entries;
                }
                Err(_) => {
                    self.entries.clear();
                }
            }
        }
    }

    // ── Phase: SelectPractice ──────────────────────────────────────────

    fn render_select_practice(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(2), // filter bar
                Constraint::Min(1),   // list
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Title
        let title = Line::from(Span::styled(
            " Trends \u{2014} Select Practice",
            Style::default().fg(ACCENT).bold(),
        ));
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Filter bar
        let filter_display = if self.filtering {
            format!(" /{}\u{2588}", self.filter_text)
        } else if !self.filter_text.is_empty() {
            format!(" /{}", self.filter_text)
        } else {
            String::from(" Press / to filter")
        };
        let filter_style = if self.filtering {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let filter_line = Line::from(Span::styled(filter_display, filter_style));
        frame.render_widget(Paragraph::new(filter_line), chunks[1]);

        // Practice list
        let lines: Vec<Line> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let practice = &self.practices[idx];
                let prefix = if i == self.selected { "> " } else { "  " };
                let text = format!(
                    "{}{} ({})",
                    prefix, practice.name, practice.practice_type.label()
                );
                let style = if i == self.selected {
                    Style::default().fg(GREEN)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(text, style))
            })
            .collect();
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[2]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(ACCENT)),
            Span::styled(" Navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(" Filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(ACCENT)),
            Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Back", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[3]);
    }

    fn handle_select_practice(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Char('/') => {
                self.filtering = true;
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = (self.selected + 1) % self.filtered_indices.len();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.filtered_indices.is_empty() {
                    self.selected = if self.selected == 0 {
                        self.filtered_indices.len() - 1
                    } else {
                        self.selected - 1
                    };
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    self.chosen_practice = Some(self.practices[idx].clone());
                    self.phase = Phase::ViewChart;
                    self.needs_refresh = true;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn apply_filter(&mut self) {
        let lower = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .practices
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    // ── Phase: ViewChart ───────────────────────────────────────────────

    fn render_view_chart(&self, frame: &mut Frame) {
        let area = frame.area();
        let practice = match self.chosen_practice.as_ref() {
            Some(p) => p,
            None => return,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // subtitle
                Constraint::Length(1), // spacer
                Constraint::Min(4),   // chart
                Constraint::Length(1), // spacer
                Constraint::Length(1), // stats
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Title: practice name + metric label + practice type
        let metric_label = self
            .entries
            .first()
            .map(|e| e.metric_label())
            .unwrap_or("\u{2014}");
        let title = Line::from(vec![
            Span::styled(
                format!(" {} ", practice.name),
                Style::default().fg(ACCENT).bold(),
            ),
            Span::styled(
                format!("({}) ", metric_label),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("[{}]", practice.practice_type.label()),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        // Subtitle
        let subtitle = Line::from(Span::styled(
            format!(" Last {} days", self.days_window),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(subtitle), chunks[1]);

        // Chart
        if self.entries.is_empty() {
            let msg = Line::from(Span::styled(
                "  No data for this period.",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(msg), chunks[3]);
        } else {
            let chart_data: Vec<(String, f64)> = self
                .entries
                .iter()
                .map(|e| {
                    let label = e.log.logged_at.format("%m/%d").to_string();
                    (label, e.total_metric())
                })
                .collect();
            let sparkline = Sparkline::new(chart_data);
            frame.render_widget(sparkline, chunks[3]);
        }

        // Stats
        if !self.entries.is_empty() {
            let (avg, peak, trend_pct) = self.stats();
            let trend_color = if trend_pct >= 0.0 { GREEN } else { RED };
            let trend_sign = if trend_pct >= 0.0 { "+" } else { "" };

            let stats_line = Line::from(vec![
                Span::styled(
                    format!("  Avg: {:.1}", avg),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("Peak: {:.1}", peak),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("Trend: {}{:.1}%", trend_sign, trend_pct),
                    Style::default().fg(trend_color),
                ),
            ]);
            frame.render_widget(Paragraph::new(stats_line), chunks[5]);
        }

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [h/l]", Style::default().fg(ACCENT)),
            Span::styled(" Window  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(" Pick practice  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Dashboard", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[6]);
    }

    fn handle_view_chart(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Char('/') => {
                // Go back to practice picker
                self.phase = Phase::SelectPractice;
                self.filter_text.clear();
                self.filtering = false;
                self.selected = 0;
                let filtered_indices = (0..self.practices.len()).collect();
                self.filtered_indices = filtered_indices;
                Action::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                // Decrease window by 30 days, min 30
                if self.days_window > 30 {
                    self.days_window -= 30;
                    self.needs_refresh = true;
                }
                Action::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                // Increase window by 30 days, max 365
                if self.days_window < 365 {
                    self.days_window += 30;
                    if self.days_window > 365 {
                        self.days_window = 365;
                    }
                    self.needs_refresh = true;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Calculate aggregate stats: (avg, peak, trend_pct).
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
