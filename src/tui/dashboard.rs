use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::db::{AggregateStats, Database};
use crate::i18n::{tr, tr_args};
use crate::model::{Goal, LogEntry, Quote};
use fluent_bundle::FluentValue;
use super::widgets::heatmap::Heatmap;
use super::{highlight_row, Action, Screen};

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardMode {
    Normal,
    QuotesManage,
    QuotesEdit,
    HrvInput,
}

pub struct DashboardScreen {
    heatmap_data: Vec<(String, i64)>,
    recent_entries: Vec<LogEntry>,
    stats: AggregateStats,
    quote: String,
    goals: Vec<Goal>,
    quotes: Vec<Quote>,
    quotes_selected: usize,
    quotes_input: String,
    quotes_cursor: usize,
    quotes_editing_id: Option<i64>,
    mode: DashboardMode,
    hrv_today: Option<i32>,
    hrv_input: String,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let recent_entries = db.list_logs_recent(7)?;
        let stats = db.aggregate_stats(7)?;
        let quotes = db.list_quotes()?;
        let quote = super::quotes::pick_daily_quote(&quotes);
        let goals = db.list_goals()?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let hrv_today = db.get_daily_hrv(&today)?;
        Ok(Self {
            heatmap_data,
            recent_entries,
            stats,
            quote,
            goals,
            quotes,
            quotes_selected: 0,
            quotes_input: String::new(),
            quotes_cursor: 0,
            quotes_editing_id: None,
            mode: DashboardMode::Normal,
            hrv_today,
            hrv_input: String::new(),
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.recent_entries = db.list_logs_recent(7)?;
        self.stats = db.aggregate_stats(7)?;
        self.quotes = db.list_quotes()?;
        self.quote = super::quotes::pick_daily_quote(&self.quotes);
        self.goals = db.list_goals()?;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.hrv_today = db.get_daily_hrv(&today)?;
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

        // Calculate quote box height: content lines + 2 for borders
        let quote_box_width = area.width.saturating_sub(4).min(HEATMAP_CONTENT_WIDTH).saturating_sub(2) as usize;
        let (quote_text, quote_style) = if self.quote.is_empty() {
            (
                tr("dashboard-no-quotes"),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            (
                format!("\"{}\"", &self.quote),
                Style::default().fg(Color::Yellow),
            )
        };
        let quote_lines = if quote_box_width > 0 {
            quote_text.chars().count().div_ceil(quote_box_width)
        } else {
            1
        } as u16;
        let quote_height = quote_lines + 2;

        // Main vertical layout: title | heatmap | quote | HRV | panes | footer | spacer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),            // [0] title
                Constraint::Length(7),            // [1] heatmap header ASCII art
                Constraint::Length(10),           // [2] heatmap
                Constraint::Length(quote_height), // [3] daily quote box
                Constraint::Length(1),            // [4] HRV row
                Constraint::Length(pane_height),  // [5] split panes
                Constraint::Length(1),            // [6] footer
                Constraint::Min(0),              // [7] spacer absorbs excess at bottom
            ])
            .split(area);

        // ── Title bar ──
        let title = Line::from(vec![
            Span::styled(format!(" {}", tr("dashboard-title")), Style::default().fg(ACCENT).bold()),
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

        // ── Daily quote (centered, rounded border) ──
        let quote_area = Rect {
            x: chunks[3].x + 1,
            y: chunks[3].y,
            width: chunks[3].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[3].height,
        };
        let quote_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let quote_paragraph = Paragraph::new(Line::from(Span::styled(
            quote_text.clone(),
            quote_style,
        )))
        .block(quote_block)
        .wrap(Wrap { trim: false })
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(quote_paragraph, quote_area);

        // ── HRV row ──
        let hrv_area = Rect {
            x: chunks[4].x + 1,
            y: chunks[4].y,
            width: chunks[4].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[4].height,
        };
        let hrv_line = if self.mode == DashboardMode::HrvInput {
            Line::from(vec![
                Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
                Span::styled(&self.hrv_input, Style::default().fg(ACCENT)),
                Span::styled("\u{2588}", Style::default().fg(ACCENT)),
                Span::styled(format!("  {}", tr("dashboard-hrv-input-hint")), Style::default().fg(Color::Gray)),
            ])
        } else if let Some(hrv) = self.hrv_today {
            Line::from(vec![
                Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", hrv), Style::default().fg(GREEN)),
                Span::styled(format!("  {}", tr("dashboard-hrv-edit-hint")), Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!(" {}: ", tr("dashboard-hrv-label")), Style::default().fg(Color::Gray)),
                Span::styled("--", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("  {}", tr("dashboard-hrv-record-hint")), Style::default().fg(Color::DarkGray)),
            ])
        };
        frame.render_widget(Paragraph::new(hrv_line), hrv_area);

        // ── Split panes (match heatmap content width) ──
        let panes_area = Rect {
            x: chunks[5].x + 1,
            y: chunks[5].y,
            width: chunks[5].width.saturating_sub(2).min(HEATMAP_CONTENT_WIDTH),
            height: chunks[5].height,
        };
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(panes_area);

        self.render_recent_pane(frame, panes[0]);
        self.render_goals_pane(frame, panes[1]);

        // ── Footer ──
        let footer_spans = if self.mode == DashboardMode::Normal {
            vec![
                Span::styled(" [l]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-log")), Style::default().fg(Color::Gray)),
                Span::styled("[h]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-history")), Style::default().fg(Color::Gray)),
                Span::styled("[t]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-trends")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-practices")), Style::default().fg(Color::Gray)),
                Span::styled("[g]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-goals")), Style::default().fg(Color::Gray)),
                Span::styled("[Q]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-quotes")), Style::default().fg(Color::Gray)),
                Span::styled("[v]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-hrv")), Style::default().fg(Color::Gray)),
                Span::styled("[q]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-quit")), Style::default().fg(Color::Gray)),
            ]
        } else if self.mode == DashboardMode::QuotesManage {
            vec![
                Span::styled(" [a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-close")), Style::default().fg(Color::Gray)),
            ]
        } else {
            vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-confirm")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-cancel")), Style::default().fg(Color::Gray)),
            ]
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[6]);

        // ── Quotes modal overlay ──
        if matches!(self.mode, DashboardMode::QuotesManage | DashboardMode::QuotesEdit) {
            self.render_quotes_modal(frame);
        }
    }

    fn render_recent_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(tr("dashboard-last-7-days"), Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Summary line
        let mut parts: Vec<Span> = Vec::new();
        parts.push(Span::styled(
            tr_args("dashboard-sessions", &[("count", FluentValue::from(self.stats.sessions))]),
            Style::default().fg(GREEN),
        ));
        if self.stats.total_volume > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            let formatted = format!("{:.0}", self.stats.total_volume);
            parts.push(Span::styled(
                tr_args("dashboard-total-volume", &[("value", FluentValue::from(formatted))]),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_reps > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            let formatted = format!("{:.0}", self.stats.total_reps);
            parts.push(Span::styled(
                tr_args("dashboard-total-reps", &[("value", FluentValue::from(formatted))]),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_distance > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            let formatted = format!("{:.1}", self.stats.total_distance);
            parts.push(Span::styled(
                tr_args("dashboard-total-distance", &[("value", FluentValue::from(formatted))]),
                Style::default().fg(GREEN),
            ));
        }
        if self.stats.total_duration > 0.0 {
            parts.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            let formatted = format!("{:.0}", self.stats.total_duration);
            parts.push(Span::styled(
                tr_args("dashboard-total-duration", &[("value", FluentValue::from(formatted))]),
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
                tr("dashboard-no-entries"),
                Style::default().fg(Color::Gray),
            )));
        } else {
            for entry in &self.recent_entries {
                let date = entry.log.logged_at.format("%b %d").to_string();
                let sets_count = entry.sets.len();
                let total = format!("{:.0}", entry.total_metric());
                let label = entry.metric_label();
                let sets_text = tr_args("dashboard-sets-metric", &[
                    ("sets", FluentValue::from(sets_count)),
                    ("total", FluentValue::from(total)),
                    ("label", FluentValue::from(label)),
                ]);
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", date), Style::default().fg(Color::Gray)),
                    Span::styled(&entry.practice_name, Style::default().fg(GREEN)),
                    Span::styled(
                        format!("  {}", sets_text),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn reload_quotes(&mut self, db: &Database) -> anyhow::Result<()> {
        self.quotes = db.list_quotes()?;
        self.quote = super::quotes::pick_daily_quote(&self.quotes);
        Ok(())
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(tr("dashboard-goals"), Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show only active (uncompleted) goals, read-only
        let active_goals: Vec<&Goal> = self.goals.iter().filter(|g| !g.completed).collect();

        if active_goals.is_empty() {
            let hint = Paragraph::new(Line::from(Span::styled(
                tr("dashboard-press-g"),
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in &active_goals {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("\u{231b} ", Style::default().fg(Color::Yellow)),
                Span::styled(&goal.title, Style::default().fg(GREEN)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_quotes_modal(&self, frame: &mut Frame) {
        let area = frame.area();

        let modal_width = area.width.saturating_sub(4).min(HEATMAP_CONTENT_WIDTH);
        let list_height = (self.quotes.len() as u16).max(1);
        let modal_height = (list_height + 4).min(area.height.saturating_sub(4)).max(6);
        let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_rect = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        frame.render_widget(Clear, modal_rect);

        let title = format!(" {} ", tr_args("dashboard-quotes-count", &[("count", FluentValue::from(self.quotes.len()))]));
        let block = Block::default()
            .title(Span::styled(title, Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT));

        let inner = block.inner(modal_rect);
        frame.render_widget(block, modal_rect);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let mut lines: Vec<Line> = Vec::new();
        if self.quotes.is_empty() {
            lines.push(Line::from(Span::styled(
                tr("dashboard-no-quotes-modal"),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, q) in self.quotes.iter().enumerate() {
                let is_sel = i == self.quotes_selected;
                let prefix = if is_sel { "> " } else { "  " };
                let style = if is_sel {
                    Style::default().fg(GREEN).bold()
                } else {
                    Style::default().fg(Color::White)
                };
                let max_text = inner_chunks[0].width.saturating_sub(2) as usize;
                let display = if q.text.chars().count() > max_text {
                    let truncated: String = q.text.chars().take(max_text.saturating_sub(1)).collect();
                    format!("{}{}…", prefix, truncated)
                } else {
                    format!("{}{}", prefix, q.text)
                };
                lines.push(Line::from(Span::styled(display, style)));
            }
        }

        let list_area_height = inner_chunks[0].height as usize;
        let scroll = self.quotes_selected
            .saturating_sub(list_area_height.saturating_sub(1)) as u16;

        frame.render_widget(
            Paragraph::new(lines).scroll((scroll, 0)),
            inner_chunks[0],
        );

        if !self.quotes.is_empty() {
            let visible_row = self.quotes_selected.saturating_sub(scroll as usize);
            highlight_row(frame, inner_chunks[0], visible_row as u16);
        }

        if self.mode == DashboardMode::QuotesEdit {
            let input_line = Line::from(vec![
                Span::styled("> ", Style::default().fg(ACCENT)),
                Span::styled(&self.quotes_input[..self.quotes_cursor], Style::default().fg(GREEN)),
                Span::styled("█", Style::default().fg(GREEN)),
                Span::styled(&self.quotes_input[self.quotes_cursor..], Style::default().fg(GREEN)),
            ]);
            frame.render_widget(Paragraph::new(input_line), inner_chunks[1]);
        } else {
            let shortcuts = Line::from(vec![
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-add")), Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-edit")), Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}  ", tr("key-delete")), Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(format!(" {}", tr("key-close")), Style::default().fg(Color::Gray)),
            ]);
            frame.render_widget(Paragraph::new(shortcuts), inner_chunks[1]);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            DashboardMode::Normal => self.handle_normal(key),
            DashboardMode::QuotesManage => self.handle_quotes_manage(key, db),
            DashboardMode::QuotesEdit => self.handle_quotes_edit(key, db),
            DashboardMode::HrvInput => self.handle_hrv_input(key, db),
        }
    }

    fn handle_quotes_text_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.quotes_cursor > 0 {
                    let prev = self.quotes_input[..self.quotes_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.quotes_cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if self.quotes_cursor > 0 {
                    let prev = self.quotes_input[..self.quotes_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.quotes_cursor = prev;
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.quotes_cursor < self.quotes_input.len() {
                    let next = self.quotes_input[self.quotes_cursor..]
                        .char_indices().nth(1)
                        .map(|(i, _)| self.quotes_cursor + i)
                        .unwrap_or(self.quotes_input.len());
                    self.quotes_cursor = next;
                }
                true
            }
            KeyCode::Right => {
                if self.quotes_cursor < self.quotes_input.len() {
                    let next = self.quotes_input[self.quotes_cursor..]
                        .char_indices().nth(1)
                        .map(|(i, _)| self.quotes_cursor + i)
                        .unwrap_or(self.quotes_input.len());
                    self.quotes_cursor = next;
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quotes_cursor = 0;
                true
            }
            KeyCode::Home => {
                self.quotes_cursor = 0;
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quotes_cursor = self.quotes_input.len();
                true
            }
            KeyCode::End => {
                self.quotes_cursor = self.quotes_input.len();
                true
            }
            KeyCode::Backspace => {
                if self.quotes_cursor > 0 {
                    let prev = self.quotes_input[..self.quotes_cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.quotes_input.remove(prev);
                    self.quotes_cursor = prev;
                }
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quotes_input.insert(self.quotes_cursor, c);
                self.quotes_cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_quotes_manage(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.quotes.is_empty() && self.quotes_selected < self.quotes.len() - 1 {
                    self.quotes_selected += 1;
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.quotes_selected > 0 {
                    self.quotes_selected -= 1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.quotes_input.clear();
                self.quotes_cursor = 0;
                self.quotes_editing_id = None;
                self.mode = DashboardMode::QuotesEdit;
                Action::None
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(q) = self.quotes.get(self.quotes_selected) {
                    self.quotes_input = q.text.clone();
                    self.quotes_cursor = self.quotes_input.len();
                    self.quotes_editing_id = Some(q.id);
                    self.mode = DashboardMode::QuotesEdit;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if let Some(q) = self.quotes.get(self.quotes_selected) {
                    let id = q.id;
                    let _ = db.delete_quote(id);
                    let _ = self.reload_quotes(db);
                    if self.quotes.is_empty() {
                        self.quotes_selected = 0;
                    } else if self.quotes_selected >= self.quotes.len() {
                        self.quotes_selected = self.quotes.len() - 1;
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = DashboardMode::Normal;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_quotes_edit(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let text = self.quotes_input.trim().to_string();
                if !text.is_empty() {
                    if let Some(id) = self.quotes_editing_id {
                        let _ = db.update_quote(id, &text);
                    } else {
                        let _ = db.create_quote(&text);
                    }
                    let _ = self.reload_quotes(db);
                }
                self.quotes_input.clear();
                self.quotes_cursor = 0;
                self.quotes_editing_id = None;
                self.mode = DashboardMode::QuotesManage;
                Action::None
            }
            KeyCode::Esc => {
                self.quotes_input.clear();
                self.quotes_cursor = 0;
                self.quotes_editing_id = None;
                self.mode = DashboardMode::QuotesManage;
                Action::None
            }
            _ => {
                self.handle_quotes_text_input(key);
                Action::None
            }
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('g') => Action::Navigate(Screen::Goals),
            KeyCode::Char('Q') => {
                self.quotes_selected = 0;
                self.mode = DashboardMode::QuotesManage;
                Action::None
            }
            KeyCode::Char('v') => {
                self.hrv_input = self.hrv_today.map(|v| v.to_string()).unwrap_or_default();
                self.mode = DashboardMode::HrvInput;
                Action::None
            }
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        }
    }

    fn handle_hrv_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Ok(hrv) = self.hrv_input.parse::<i32>() {
                    if (0..=100).contains(&hrv) {
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        let _ = db.set_daily_hrv(&today, hrv);
                        self.hrv_today = Some(hrv);
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
