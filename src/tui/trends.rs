use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{LogEntry, Practice};
use fluent_bundle::FluentValue;
use super::widgets::sparkline::Sparkline;
use super::{centered_area, highlight_row, Action, Screen, BORDER_COLOR, CONTENT_WIDTH};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

pub struct TrendsScreen {
    practices: Vec<Practice>,
    filtered_indices: Vec<usize>,
    filter_text: String,
    filter_cursor: usize,
    filtering: bool,
    selected: usize,
    scroll: usize,
    list_height: usize,
    chosen_practice: Option<Practice>,
    days_window: i64,
    entries: Vec<LogEntry>,
    needs_refresh: bool,
}

impl TrendsScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let filtered_indices: Vec<usize> = (0..practices.len()).collect();
        let chosen = filtered_indices.first().map(|&i| practices[i].clone());
        Ok(Self {
            practices,
            filtered_indices,
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            scroll: 0,
            list_height: 0,
            chosen_practice: chosen,
            days_window: 90,
            entries: Vec::new(),
            needs_refresh: true,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),         // filter bar
                Constraint::Percentage(40),    // practice list
                Constraint::Percentage(60),    // sparkline chart
                Constraint::Length(1),         // footer
            ])
            .split(area);

        self.render_practice_list(frame, chunks[0], chunks[1]);
        self.render_sparkline_panel(frame, chunks[2]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(" [j/k]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-navigate")), Style::default().fg(Color::DarkGray)),
            Span::styled("[h/l]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-window")), Style::default().fg(Color::DarkGray)),
            Span::styled("[/]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}  ", tr("key-filter")), Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(format!(" {}", tr("key-back")), Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(footer), chunks[3]);
    }

    fn render_practice_list(&mut self, frame: &mut Frame, filter_area: ratatui::layout::Rect, list_area: ratatui::layout::Rect) {
        let max_name_len = self.practices.iter()
            .map(|p| p.name.width())
            .max()
            .unwrap_or(0);
        let col_width = max_name_len + 4;

        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let header_padding = col_width.saturating_sub(name_header.width());

        // Filter bar
        let filter_display = if self.filtering {
            let (before, after) = self.filter_text.split_at(self.filter_cursor);
            format!(" /{}{}{}", before, "\u{2588}", after)
        } else if !self.filter_text.is_empty() {
            format!(" /{}", self.filter_text)
        } else {
            format!(" {}", tr("log-press-filter"))
        };
        let filter_style = if self.filtering {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(Color::Gray)
        };
        frame.render_widget(Paragraph::new(Line::from(Span::styled(filter_display, filter_style))), filter_area);

        // Bordered list
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(tr("trends-title"), Style::default().fg(Color::White).bold()),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(list_area);
        frame.render_widget(block, list_area);

        let hdr_style = Style::default().fg(Color::White).bold();
        let mut all_lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(&name_header, hdr_style),
                Span::raw(" ".repeat(header_padding)),
                Span::styled(&type_header, hdr_style),
            ]),
        ];

        let visible_rows = inner.height.saturating_sub(1) as usize; // -1 for header
        self.list_height = visible_rows;
        let end = (self.scroll + visible_rows).min(self.filtered_indices.len());

        for i in self.scroll..end {
            let idx = self.filtered_indices[i];
            let practice = &self.practices[idx];
            let marker = if i == self.selected { "> " } else { "  " };
            let name_style = if i == self.selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let padding = col_width.saturating_sub(practice.name.width());
            all_lines.push(Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(&practice.name, name_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(practice.practice_type.label(), Style::default().fg(Color::Gray)),
            ]));
        }
        frame.render_widget(Paragraph::new(all_lines), inner);

        if !self.filtered_indices.is_empty() && self.selected >= self.scroll && self.selected < end {
            highlight_row(frame, inner, (self.selected - self.scroll) as u16 + 1);
        }
    }

    fn render_sparkline_panel(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let practice = match self.chosen_practice.as_ref() {
            Some(p) => p,
            None => {
                let block = Block::default()
                    .borders(Borders::ALL)
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
        let block_title = format!(" {} ({}) [{}] ", practice.name, metric_label, practice.practice_type.label());
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(&block_title, Style::default().fg(ACCENT).bold()),
                Span::styled("──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
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
            format!(" {}", tr_args("trends-last-days", &[("days", FluentValue::from(self.days_window as f64))])),
            Style::default().fg(Color::Gray),
        ));
        frame.render_widget(Paragraph::new(subtitle), inner_chunks[0]);

        // Chart
        if self.entries.is_empty() {
            let msg = Line::from(Span::styled(
                format!("  {}", tr("trends-no-data")),
                Style::default().fg(Color::Gray),
            ));
            frame.render_widget(Paragraph::new(msg), inner_chunks[1]);
        } else {
            let chart_data: Vec<(String, f64)> = self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let day = e.log.logged_at.format("%d").to_string();
                    let label = if i == 0
                        || e.log.logged_at.month()
                            != self.entries[i - 1].log.logged_at.month()
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
                    format!("  {}", tr_args("trends-avg", &[("value", FluentValue::from(format!("{:.1}", avg)))])),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    tr_args("trends-peak", &[("value", FluentValue::from(format!("{:.1}", peak)))]),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  |  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    tr_args("trends-trend", &[("sign", FluentValue::from(trend_sign.to_string())), ("value", FluentValue::from(format!("{:.1}", trend_pct)))]),
                    Style::default().fg(trend_color),
                ),
            ]);
            frame.render_widget(Paragraph::new(stats_line), inner_chunks[2]);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    }
                }
                KeyCode::Left => {
                    if self.filter_cursor > 0 {
                        self.filter_cursor = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    }
                }
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices().nth(1).map(|(i, _)| self.filter_cursor + i)
                            .unwrap_or(self.filter_text.len());
                    }
                }
                KeyCode::Right => {
                    if self.filter_cursor < self.filter_text.len() {
                        self.filter_cursor = self.filter_text[self.filter_cursor..]
                            .char_indices().nth(1).map(|(i, _)| self.filter_cursor + i)
                            .unwrap_or(self.filter_text.len());
                    }
                }
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_cursor = 0;
                }
                KeyCode::Home => {
                    self.filter_cursor = 0;
                }
                KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_cursor = self.filter_text.len();
                }
                KeyCode::End => {
                    self.filter_cursor = self.filter_text.len();
                }
                KeyCode::Backspace => {
                    if self.filter_cursor > 0 {
                        let prev = self.filter_text[..self.filter_cursor]
                            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                        self.filter_text.remove(prev);
                        self.filter_cursor = prev;
                        self.apply_filter();
                    }
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter_text.truncate(self.filter_cursor);
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.insert(self.filter_cursor, c);
                    self.filter_cursor += c.len_utf8();
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
                self.filter_cursor = self.filter_text.len();
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_indices.is_empty() {
                    self.selected = (self.selected + 1) % self.filtered_indices.len();
                    self.adjust_scroll();
                    self.sync_chosen();
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
                    self.adjust_scroll();
                    self.sync_chosen();
                }
                Action::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.days_window > 30 {
                    self.days_window -= 30;
                    self.needs_refresh = true;
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
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + self.list_height {
            self.scroll = self.selected + 1 - self.list_height;
        }
    }

    fn sync_chosen(&mut self) {
        if let Some(&idx) = self.filtered_indices.get(self.selected) {
            self.chosen_practice = Some(self.practices[idx].clone());
            self.needs_refresh = true;
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
        self.scroll = 0;
        self.sync_chosen();
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
                    entries.sort_by(|a, b| a.log.logged_at.cmp(&b.log.logged_at));
                    self.entries = entries;
                }
                Err(_) => {
                    self.entries.clear();
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PracticeType;

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
            filter_text: String::new(),
            filter_cursor: 0,
            filtering: false,
            selected: 0,
            scroll: 0,
            list_height,
            chosen_practice: None,
            days_window: 90,
            entries: Vec::new(),
            needs_refresh: false,
        }
    }

    #[test]
    fn scroll_follows_selection_down() {
        let mut s = make_screen(20, 5);
        // Select last visible item — no scroll needed
        s.selected = 4;
        s.adjust_scroll();
        assert_eq!(s.scroll, 0);

        // One past the viewport
        s.selected = 5;
        s.adjust_scroll();
        assert_eq!(s.scroll, 1);
    }

    #[test]
    fn scroll_follows_selection_up() {
        let mut s = make_screen(20, 5);
        s.selected = 10;
        s.scroll = 10;
        // Move up above scroll
        s.selected = 8;
        s.adjust_scroll();
        assert_eq!(s.scroll, 8);
    }

    #[test]
    fn scroll_stays_at_zero_when_all_fit() {
        let mut s = make_screen(5, 10);
        s.selected = 4;
        s.adjust_scroll();
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn scroll_wraps_to_top() {
        let mut s = make_screen(20, 5);
        s.selected = 19;
        s.scroll = 15;
        // Wrap to top
        s.selected = 0;
        s.adjust_scroll();
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn scroll_last_item_visible() {
        let mut s = make_screen(20, 5);
        s.selected = 19;
        s.adjust_scroll();
        assert_eq!(s.scroll, 15);
    }
}
