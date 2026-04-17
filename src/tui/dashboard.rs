use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::db::{AggregateStats, Database};
use crate::model::{Goal, LogEntry, Quote};
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardMode {
    Normal,
    Goals,
    AddGoal,
    AddMilestone,
    EditItem,
    EditDate,
    ConfirmDelete,
    QuotesManage,
    QuotesEdit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GoalItem {
    Goal(i64),
    Milestone(i64),
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
    goal_selected: usize,
    goal_input: String,
    goal_cursor: usize,
    goal_scroll: usize,
}

impl DashboardScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let heatmap_data = db.heatmap_counts(365)?;
        let recent_entries = db.list_logs_recent(14)?;
        let stats = db.aggregate_stats(14)?;
        let quotes = db.list_quotes()?;
        let quote = super::quotes::pick_daily_quote(&quotes);
        let goals = db.list_goals()?;
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
            goal_selected: 0,
            goal_input: String::new(),
            goal_cursor: 0,
            goal_scroll: 0,
        })
    }

    pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        self.heatmap_data = db.heatmap_counts(365)?;
        self.recent_entries = db.list_logs_recent(14)?;
        self.stats = db.aggregate_stats(14)?;
        self.quotes = db.list_quotes()?;
        self.quote = super::quotes::pick_daily_quote(&self.quotes);
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

        // Calculate quote box height: content lines + 2 for borders
        let quote_box_width = area.width.saturating_sub(4).min(HEATMAP_CONTENT_WIDTH).saturating_sub(2) as usize;
        let (quote_text, quote_style) = if self.quote.is_empty() {
            (
                "No quotes yet — press Q to add one".to_string(),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            (
                format!("\"{}\"", &self.quote),
                Style::default().fg(Color::Yellow),
            )
        };
        let quote_lines = if quote_box_width > 0 {
            (quote_text.chars().count() + quote_box_width - 1) / quote_box_width
        } else {
            1
        } as u16;
        let quote_height = quote_lines + 2;

        // Main vertical layout: title | heatmap | quote | panes | footer | spacer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),            // [0] title
                Constraint::Length(7),            // [1] heatmap header ASCII art
                Constraint::Length(10),           // [2] heatmap
                Constraint::Length(quote_height), // [3] daily quote box
                Constraint::Length(pane_height),  // [4] split panes
                Constraint::Length(1),            // [5] footer
                Constraint::Min(0),              // [6] spacer absorbs excess at bottom
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
        let footer_spans = if self.mode == DashboardMode::Normal {
            vec![
                Span::styled(" [l]", Style::default().fg(ACCENT)),
                Span::styled(" Log  ", Style::default().fg(Color::Gray)),
                Span::styled("[h]", Style::default().fg(ACCENT)),
                Span::styled(" History  ", Style::default().fg(Color::Gray)),
                Span::styled("[t]", Style::default().fg(ACCENT)),
                Span::styled(" Trends  ", Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(" Practices  ", Style::default().fg(Color::Gray)),
                Span::styled("[g]", Style::default().fg(ACCENT)),
                Span::styled(" Goals  ", Style::default().fg(Color::Gray)),
                Span::styled("[q]", Style::default().fg(ACCENT)),
                Span::styled(" Quit", Style::default().fg(Color::Gray)),
            ]
        } else if self.mode == DashboardMode::Goals {
            vec![
                Span::styled(" [a]", Style::default().fg(ACCENT)),
                Span::styled(" Add goal  ", Style::default().fg(Color::Gray)),
                Span::styled("[m]", Style::default().fg(ACCENT)),
                Span::styled(" Milestone  ", Style::default().fg(Color::Gray)),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(" Edit  ", Style::default().fg(Color::Gray)),
                Span::styled("[Space]", Style::default().fg(ACCENT)),
                Span::styled(" Toggle  ", Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(" Delete  ", Style::default().fg(Color::Gray)),
                Span::styled("[D]", Style::default().fg(ACCENT)),
                Span::styled(" Date  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" Back", Style::default().fg(Color::Gray)),
            ]
        } else {
            vec![
                Span::styled(" [Enter]", Style::default().fg(ACCENT)),
                Span::styled(" Confirm  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" Cancel", Style::default().fg(Color::Gray)),
            ]
        };
        let footer = Line::from(footer_spans);
        frame.render_widget(Paragraph::new(footer), chunks[5]);

        // ── Quotes modal overlay ──
        if matches!(self.mode, DashboardMode::QuotesManage | DashboardMode::QuotesEdit) {
            self.render_quotes_modal(frame);
        }
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

    fn goal_items(&self) -> Vec<GoalItem> {
        let mut items = Vec::new();
        for goal in &self.goals {
            items.push(GoalItem::Goal(goal.id));
            for ms in &goal.milestones {
                items.push(GoalItem::Milestone(ms.id));
            }
        }
        items
    }

    fn selected_goal_item(&self) -> Option<GoalItem> {
        let items = self.goal_items();
        items.get(self.goal_selected).copied()
    }

    fn parent_goal_id(&self) -> Option<i64> {
        match self.selected_goal_item()? {
            GoalItem::Goal(id) => Some(id),
            GoalItem::Milestone(ms_id) => {
                self.goals.iter()
                    .find(|g| g.milestones.iter().any(|m| m.id == ms_id))
                    .map(|g| g.id)
            }
        }
    }

    fn reload_goals(&mut self, db: &Database) -> anyhow::Result<()> {
        self.goals = db.list_goals()?;
        Ok(())
    }

    fn reload_quotes(&mut self, db: &Database) -> anyhow::Result<()> {
        self.quotes = db.list_quotes()?;
        self.quote = super::quotes::pick_daily_quote(&self.quotes);
        Ok(())
    }

    fn adjust_goal_scroll(&mut self) {
        let goals_lines = self.goals.iter()
            .map(|g| 1 + g.milestones.len())
            .sum::<usize>()
            .max(1);
        let recent_lines = self.recent_entries.len() + 4;
        let pane_height = recent_lines.max(goals_lines + 2).max(7);
        let visible_height = pane_height.saturating_sub(2);

        if visible_height == 0 {
            return;
        }

        // Extra lines rendered after the selected item (inline prompts)
        let extra = match self.mode {
            DashboardMode::ConfirmDelete => 1,
            DashboardMode::EditDate => 1,
            DashboardMode::AddMilestone => 1,
            _ => 0,
        };

        if self.goal_selected < self.goal_scroll {
            self.goal_scroll = self.goal_selected;
        } else if self.goal_selected + extra >= self.goal_scroll + visible_height {
            self.goal_scroll = (self.goal_selected + extra) - visible_height + 1;
        }
    }

    fn render_goals_pane(&self, frame: &mut Frame, area: Rect) {
        let border_color = match self.mode {
            DashboardMode::Normal | DashboardMode::QuotesManage | DashboardMode::QuotesEdit => Color::DarkGray,
            _ => ACCENT,
        };

        let block = Block::default()
            .title(Span::styled("Goals", Style::default().fg(Color::White).bold()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.goals.is_empty() && self.mode == DashboardMode::Normal {
            let hint = Paragraph::new(Line::from(Span::styled(
                "Press [g] to add goals",
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(hint, inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let in_goals_mode = matches!(
            self.mode,
            DashboardMode::Goals
                | DashboardMode::AddGoal
                | DashboardMode::AddMilestone
                | DashboardMode::EditItem
                | DashboardMode::EditDate
                | DashboardMode::ConfirmDelete
        );

        // AddGoal input at top so it's always visible
        if self.mode == DashboardMode::AddGoal {
            lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(GREEN).bold()),
                Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                Span::styled("█", Style::default().fg(GREEN)),
                Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
            ]));
        }

        let mut idx = 0;
        for goal in &self.goals {
            let is_selected = in_goals_mode && idx == self.goal_selected;
            let style = if is_selected {
                Style::default().fg(GREEN).bold()
            } else {
                Style::default().fg(Color::White).bold()
            };

            if is_selected && self.mode == DashboardMode::EditItem {
                lines.push(Line::from(vec![
                    Span::styled("▸ ", style),
                    Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                    Span::styled("█", Style::default().fg(GREEN)),
                    Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
                ]));
            } else if is_selected && self.mode == DashboardMode::EditDate {
                lines.push(Line::from(Span::styled(
                    format!("☑ {}", goal.title),
                    Style::default().fg(GREEN),
                )));
                lines.push(Line::from(vec![
                    Span::styled("  Date (YYYY-MM-DD): ", Style::default().fg(ACCENT)),
                    Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                    Span::styled("█", Style::default().fg(GREEN)),
                    Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
                ]));
            } else if goal.completed {
                let date_str = goal.completed_at
                    .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
                    .unwrap_or_default();
                lines.push(Line::from(Span::styled(
                    format!("☑ {}{}", goal.title, date_str),
                    if is_selected { Style::default().fg(GREEN) } else { Style::default().fg(Color::DarkGray) },
                )));
            } else {
                let marker = if is_selected { "> " } else { "▸ " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", marker, goal.title),
                    style,
                )));
            }
            if is_selected && self.mode == DashboardMode::ConfirmDelete {
                lines.push(Line::from(Span::styled(
                    "  Delete? (y/n)",
                    Style::default().fg(Color::Red),
                )));
            }
            idx += 1;

            for ms in &goal.milestones {
                let is_ms_selected = in_goals_mode && idx == self.goal_selected;

                if is_ms_selected && self.mode == DashboardMode::EditItem {
                    let check = if ms.completed { "☑ " } else { "☐ " };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", check), Style::default().fg(GREEN)),
                        Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                        Span::styled("█", Style::default().fg(GREEN)),
                        Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
                    ]));
                } else if is_ms_selected && self.mode == DashboardMode::EditDate {
                    lines.push(Line::from(Span::styled(
                        format!("  ☑ {}", ms.title),
                        Style::default().fg(GREEN),
                    )));
                    lines.push(Line::from(vec![
                        Span::styled("    Date (YYYY-MM-DD): ", Style::default().fg(ACCENT)),
                        Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                        Span::styled("█", Style::default().fg(GREEN)),
                        Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
                    ]));
                } else if ms.completed {
                    let date_str = ms.completed_at
                        .map(|dt| format!(" ({})", dt.format("%Y-%m-%d")))
                        .unwrap_or_default();
                    let style = if is_ms_selected {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let marker = if is_ms_selected { "> " } else { "  " };
                    lines.push(Line::from(Span::styled(
                        format!("{}☑ {}{}", marker, ms.title, date_str),
                        style,
                    )));
                } else {
                    let style = if is_ms_selected {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let marker = if is_ms_selected { "> " } else { "  " };
                    lines.push(Line::from(Span::styled(
                        format!("{}☐ {}", marker, ms.title),
                        style,
                    )));
                }
                if is_ms_selected && self.mode == DashboardMode::ConfirmDelete {
                    lines.push(Line::from(Span::styled(
                        "    Delete? (y/n)",
                        Style::default().fg(Color::Red),
                    )));
                }
                idx += 1;
            }

            // Show milestone input right after the selected goal's milestones
            if self.mode == DashboardMode::AddMilestone {
                if let Some(parent_id) = self.parent_goal_id() {
                    if parent_id == goal.id {
                        lines.push(Line::from(vec![
                            Span::styled("  ☐ ", Style::default().fg(GREEN)),
                            Span::styled(&self.goal_input[..self.goal_cursor], Style::default().fg(GREEN)),
                            Span::styled("█", Style::default().fg(GREEN)),
                            Span::styled(&self.goal_input[self.goal_cursor..], Style::default().fg(GREEN)),
                        ]));
                    }
                }
            }
        }


        if self.goals.is_empty() && self.mode == DashboardMode::Goals {
            lines.push(Line::from(Span::styled(
                "Press [a] to add a goal",
                Style::default().fg(Color::Gray),
            )));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.goal_scroll as u16, 0)),
            inner,
        );
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

        let title = format!(" Quotes ({}) ", self.quotes.len());
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
                "No quotes — press [a] to add one",
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
                Span::styled(" add  ", Style::default().fg(Color::Gray)),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(" edit  ", Style::default().fg(Color::Gray)),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(" delete  ", Style::default().fg(Color::Gray)),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(" close", Style::default().fg(Color::Gray)),
            ]);
            frame.render_widget(Paragraph::new(shortcuts), inner_chunks[1]);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        match self.mode {
            DashboardMode::Normal => self.handle_normal(key),
            DashboardMode::Goals => self.handle_goals(key, db),
            DashboardMode::AddGoal => self.handle_add_goal(key, db),
            DashboardMode::AddMilestone => self.handle_add_milestone(key, db),
            DashboardMode::EditItem => self.handle_edit_item(key, db),
            DashboardMode::EditDate => self.handle_edit_date(key, db),
            DashboardMode::ConfirmDelete => self.handle_confirm_delete(key, db),
            DashboardMode::QuotesManage => Action::None, // handlers added in next task
            DashboardMode::QuotesEdit => Action::None,   // handlers added in next task
        }
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.goal_cursor > 0 {
                    let prev = self.goal_input[..self.goal_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.goal_cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if self.goal_cursor > 0 {
                    let prev = self.goal_input[..self.goal_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.goal_cursor = prev;
                }
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.goal_cursor < self.goal_input.len() {
                    let next = self.goal_input[self.goal_cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.goal_cursor + i)
                        .unwrap_or(self.goal_input.len());
                    self.goal_cursor = next;
                }
                true
            }
            KeyCode::Right => {
                if self.goal_cursor < self.goal_input.len() {
                    let next = self.goal_input[self.goal_cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.goal_cursor + i)
                        .unwrap_or(self.goal_input.len());
                    self.goal_cursor = next;
                }
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.goal_cursor = 0;
                true
            }
            KeyCode::Home => {
                self.goal_cursor = 0;
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.goal_cursor = self.goal_input.len();
                true
            }
            KeyCode::End => {
                self.goal_cursor = self.goal_input.len();
                true
            }
            KeyCode::Backspace => {
                if self.goal_cursor > 0 {
                    let prev = self.goal_input[..self.goal_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.goal_input.remove(prev);
                    self.goal_cursor = prev;
                }
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.goal_input.insert(self.goal_cursor, c);
                self.goal_cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('l') => Action::Navigate(Screen::LogEntry),
            KeyCode::Char('h') => Action::Navigate(Screen::History),
            KeyCode::Char('t') => Action::Navigate(Screen::Trends),
            KeyCode::Char('e') => Action::Navigate(Screen::Practices),
            KeyCode::Char('g') => {
                self.mode = DashboardMode::Goals;
                self.goal_selected = 0;
                self.goal_scroll = 0;
                Action::None
            }
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        }
    }

    fn handle_goals(&mut self, key: KeyEvent, db: &Database) -> Action {
        let items = self.goal_items();
        let item_count = items.len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if item_count > 0 && self.goal_selected < item_count - 1 {
                    self.goal_selected += 1;
                    self.adjust_goal_scroll();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.goal_selected > 0 {
                    self.goal_selected -= 1;
                    self.adjust_goal_scroll();
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::AddGoal;
                Action::None
            }
            KeyCode::Char('m') => {
                if self.parent_goal_id().is_some() {
                    self.goal_input.clear();
                    self.goal_cursor = 0;
                    self.mode = DashboardMode::AddMilestone;
                    self.adjust_goal_scroll();
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_goal_item() {
                    let current_title = match item {
                        GoalItem::Goal(id) => {
                            self.goals.iter().find(|g| g.id == id).map(|g| g.title.clone())
                        }
                        GoalItem::Milestone(id) => {
                            self.goals.iter()
                                .flat_map(|g| &g.milestones)
                                .find(|m| m.id == id)
                                .map(|m| m.title.clone())
                        }
                    };
                    if let Some(title) = current_title {
                        self.goal_input = title;
                        self.goal_cursor = self.goal_input.len();
                        self.mode = DashboardMode::EditItem;
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                match self.selected_goal_item() {
                    Some(GoalItem::Goal(id)) => {
                        let _ = db.toggle_goal(id);
                        let _ = self.reload_goals(db);
                    }
                    Some(GoalItem::Milestone(id)) => {
                        let _ = db.toggle_milestone(id);
                        let _ = self.reload_goals(db);
                    }
                    None => {}
                }
                Action::None
            }
            KeyCode::Char('D') => {
                if let Some(item) = self.selected_goal_item() {
                    let is_completed = match item {
                        GoalItem::Goal(id) => self.goals.iter().find(|g| g.id == id).map(|g| g.completed).unwrap_or(false),
                        GoalItem::Milestone(id) => self.goals.iter().flat_map(|g| &g.milestones).find(|m| m.id == id).map(|m| m.completed).unwrap_or(false),
                    };
                    if is_completed {
                        self.goal_input.clear();
                        self.goal_cursor = 0;
                        self.mode = DashboardMode::EditDate;
                        self.adjust_goal_scroll();
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if self.selected_goal_item().is_some() {
                    self.mode = DashboardMode::ConfirmDelete;
                    self.adjust_goal_scroll();
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

    fn handle_add_goal(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    let _ = db.create_goal(&title);
                    let _ = self.reload_goals(db);
                }
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.goal_selected = 0;
                self.goal_scroll = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_add_milestone(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    if let Some(goal_id) = self.parent_goal_id() {
                        let _ = db.create_milestone(goal_id, &title);
                        let _ = self.reload_goals(db);
                    }
                }
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_edit_item(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let title = self.goal_input.trim().to_string();
                if !title.is_empty() {
                    if let Some(item) = self.selected_goal_item() {
                        match item {
                            GoalItem::Goal(id) => { let _ = db.update_goal(id, &title); }
                            GoalItem::Milestone(id) => { let _ = db.update_milestone(id, &title); }
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_edit_date(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.goal_input, "%Y-%m-%d") {
                    let completed_at = date.and_hms_opt(0, 0, 0).unwrap();
                    if let Some(item) = self.selected_goal_item() {
                        match item {
                            GoalItem::Goal(id) => { let _ = db.set_goal_completed_at(id, &completed_at); }
                            GoalItem::Milestone(id) => { let _ = db.set_milestone_completed_at(id, &completed_at); }
                        }
                        let _ = self.reload_goals(db);
                    }
                }
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            KeyCode::Esc => {
                self.goal_input.clear();
                self.goal_cursor = 0;
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.handle_text_input(key);
                Action::None
            }
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(item) = self.selected_goal_item() {
                    match item {
                        GoalItem::Goal(id) => { let _ = db.delete_goal(id); }
                        GoalItem::Milestone(id) => { let _ = db.delete_milestone(id); }
                    }
                    let _ = self.reload_goals(db);
                    let items = self.goal_items();
                    if self.goal_selected >= items.len() && !items.is_empty() {
                        self.goal_selected = items.len() - 1;
                    }
                }
                self.mode = DashboardMode::Goals;
                Action::None
            }
            _ => {
                self.mode = DashboardMode::Goals;
                Action::None
            }
        }
    }
}
