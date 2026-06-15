use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use std::sync::mpsc::{self, Receiver};

use super::{
    centered_area, render_status_line, visible_input_spans, Action, Screen, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::config::LlmConfig;
use crate::db::Database;
use crate::i18n::tr;
use crate::llm::{self, LlmError};
use crate::model::{Abbreviation, ParsedLog, Practice, SetData};

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;
const YELLOW: Color = Color::Yellow;

const SPINNER: &[char] = &[
    '\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280F}',
];

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Input,
    Parsing,
    Preview,
    BrowseAbbreviations, // Show list of abbreviations
    AddAbbrShort,
    AddAbbrFull,
    EditAbbrShort,
    EditAbbrFull,
    ConfirmDeleteAbbr, // Confirm deletion of abbreviation
}

pub struct QuickLogScreen {
    input_lines: Vec<String>,
    current_line: usize,
    cursor_pos: usize,
    llm_config: Option<LlmConfig>,
    practices: Vec<Practice>,
    abbreviations: Vec<Abbreviation>,
    parsed_results: Vec<ParsedLog>,
    result_receiver: Option<Receiver<Result<Vec<ParsedLog>, LlmError>>>,
    selected_result: usize,
    scroll_offset: usize,
    phase: Phase,
    status_msg: StatusMessage,
    log_date: String,
    spinner_frame: usize,
    // Abbreviation modal fields
    abbr_short_input: String,
    abbr_short_cursor: usize,
    abbr_full_input: String,
    abbr_full_cursor: usize,
    abbr_editing_id: Option<i64>,
    abbr_selected: usize,  // Selected index in abbreviations list
    pre_abbr_phase: Phase, // Phase to return to when canceling abbreviation modal
}

impl QuickLogScreen {
    pub fn new(db: &Database, config: &Option<LlmConfig>) -> anyhow::Result<Self> {
        let practices = db.list_active_practices()?;
        let abbreviations = db.list_abbreviations()?;
        let log_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(Self {
            input_lines: vec![String::new()],
            current_line: 0,
            cursor_pos: 0,
            llm_config: config.clone(),
            practices,
            abbreviations,
            parsed_results: Vec::new(),
            result_receiver: None,
            selected_result: 0,
            scroll_offset: 0,
            phase: Phase::Input,
            status_msg: None,
            log_date,
            spinner_frame: 0,
            abbr_short_input: String::new(),
            abbr_short_cursor: 0,
            abbr_full_input: String::new(),
            abbr_full_cursor: 0,
            abbr_editing_id: None,
            abbr_selected: 0,
            pre_abbr_phase: Phase::Input,
        })
    }

    fn refresh_abbreviations(&mut self, db: &Database) {
        if let Ok(abbrs) = db.list_abbreviations() {
            self.abbreviations = abbrs;
        }
    }

    pub fn check_background_result(&mut self) {
        if self.phase != Phase::Parsing {
            return;
        }

        self.spinner_frame = (self.spinner_frame + 1) % SPINNER.len();

        if let Some(ref receiver) = self.result_receiver {
            match receiver.try_recv() {
                Ok(Ok(results)) => {
                    self.parsed_results = results;
                    self.selected_result = 0;
                    self.scroll_offset = 0;
                    self.phase = Phase::Preview;
                    self.result_receiver = None;
                }
                Ok(Err(e)) => {
                    self.status_msg = Some((format!("Error: {}", e), true));
                    self.phase = Phase::Input;
                    self.result_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_msg = Some(("Error: LLM thread disconnected".to_string(), true));
                    self.phase = Phase::Input;
                    self.result_receiver = None;
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),    // bordered block with two panes
                Constraint::Length(1), // status message
                Constraint::Length(1), // shortcuts
            ])
            .split(area);

        // ── Bordered block ──
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    tr("quicklog-title"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ]))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        // ── Two panes side by side ──
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(50),
            ])
            .split(inner);

        self.render_input_pane(frame, panes[0]);
        self.render_preview_pane(frame, panes[2]);

        // ── Status line ──
        render_status_line(frame, chunks[1], &self.status_msg);

        // ── Shortcuts ──
        let shortcuts = self.build_shortcuts();
        frame.render_widget(
            Paragraph::new(vec![shortcuts]).wrap(Wrap { trim: false }),
            chunks[2],
        );

        // ── Abbreviation modal (if active) ──
        if self.is_abbr_modal_active() {
            self.render_abbr_modal(frame, area);
        }
    }

    fn is_abbr_modal_active(&self) -> bool {
        matches!(
            self.phase,
            Phase::BrowseAbbreviations
                | Phase::AddAbbrShort
                | Phase::AddAbbrFull
                | Phase::EditAbbrShort
                | Phase::EditAbbrFull
                | Phase::ConfirmDeleteAbbr
        )
    }

    fn render_abbr_modal(&self, frame: &mut Frame, parent_area: Rect) {
        use ratatui::widgets::Clear;

        // Modal dimensions - larger for browse mode
        let modal_width = 60u16;
        let modal_height = if matches!(self.phase, Phase::BrowseAbbreviations) {
            20u16.min(parent_area.height.saturating_sub(4)) // Taller for list
        } else {
            8u16
        };

        // Center the modal
        let modal_area = Rect {
            x: parent_area.x + (parent_area.width.saturating_sub(modal_width)) / 2,
            y: parent_area.y + (parent_area.height.saturating_sub(modal_height)) / 2,
            width: modal_width.min(parent_area.width),
            height: modal_height.min(parent_area.height),
        };

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Handle browse mode separately
        if self.phase == Phase::BrowseAbbreviations {
            self.render_abbr_browse_modal(frame, modal_area);
            return;
        }

        // Handle delete confirmation
        if self.phase == Phase::ConfirmDeleteAbbr {
            self.render_abbr_delete_confirm(frame, modal_area);
            return;
        }

        // Modal content for add/edit modes
        let (title, prompt, input_text, cursor_pos) = match self.phase {
            Phase::AddAbbrShort | Phase::EditAbbrShort => {
                let title = if matches!(self.phase, Phase::AddAbbrShort) {
                    tr("abbreviations-enter-short")
                } else {
                    tr("abbreviations-edit-short")
                };
                (
                    title,
                    tr("abbreviations-enter-short"),
                    &self.abbr_short_input,
                    self.abbr_short_cursor,
                )
            }
            Phase::AddAbbrFull | Phase::EditAbbrFull => {
                let title = if matches!(self.phase, Phase::AddAbbrFull) {
                    tr("abbreviations-enter-full")
                } else {
                    tr("abbreviations-edit-full")
                };
                (
                    title,
                    tr("abbreviations-enter-full"),
                    &self.abbr_full_input,
                    self.abbr_full_cursor,
                )
            }
            _ => return,
        };

        // Modal block
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Content layout
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // prompt
                Constraint::Length(1), // spacer
                Constraint::Length(1), // input
            ])
            .margin(1)
            .split(inner);

        // Prompt
        let prompt_line = Line::from(vec![Span::styled(
            prompt,
            Style::default().fg(Color::White),
        )]);
        frame.render_widget(
            Paragraph::new(prompt_line).wrap(Wrap { trim: false }),
            content[0],
        );

        // Input with cursor
        let mut spans = vec![Span::styled(" > ", Style::default().fg(Color::White))];
        spans.extend(visible_input_spans(
            input_text,
            cursor_pos,
            content[2].width,
            3,
            Color::White,
        ));
        let input_line = Line::from(spans);
        frame.render_widget(
            Paragraph::new(input_line).wrap(Wrap { trim: false }),
            content[2],
        );
    }

    fn render_abbr_browse_modal(&self, frame: &mut Frame, modal_area: Rect) {
        use unicode_width::UnicodeWidthStr;

        // Modal block with title
        let block = Block::default()
            .title(format!(" {} ", tr("abbreviations-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Calculate column widths
        let max_short_len = self
            .abbreviations
            .iter()
            .map(|a| a.short.width())
            .max()
            .unwrap_or(0);
        let short_header = tr("abbreviations-col-short");
        let col_width = max_short_len.max(short_header.width()) + 2;

        // Header
        let header_padding = col_width.saturating_sub(short_header.width());
        let full_header = tr("abbreviations-col-full");

        // Layout: header + list + shortcuts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // title + header
                Constraint::Min(3),    // list
                Constraint::Length(1), // shortcuts
            ])
            .margin(1)
            .split(inner);

        // Header lines
        let header_lines = vec![Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&short_header, Style::default().fg(Color::DarkGray)),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&full_header, Style::default().fg(Color::DarkGray)),
        ])];
        frame.render_widget(
            Paragraph::new(header_lines).wrap(Wrap { trim: false }),
            chunks[0],
        );

        // List of abbreviations
        let list_lines: Vec<Line> = if self.abbreviations.is_empty() {
            vec![Line::from(Span::styled(
                tr("abbreviations-no-items"),
                Style::default().fg(Color::Gray),
            ))]
        } else {
            self.abbreviations
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let marker = if i == self.abbr_selected { "> " } else { "  " };
                    let style = if i == self.abbr_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let padding = col_width.saturating_sub(a.short.width());
                    Line::from(vec![
                        Span::styled(marker, style),
                        Span::styled(&a.short, style),
                        Span::raw(" ".repeat(padding)),
                        Span::styled(&a.full_name, style),
                    ])
                })
                .collect()
        };
        frame.render_widget(
            Paragraph::new(list_lines).wrap(Wrap { trim: false }),
            chunks[1],
        );

        // Shortcuts
        let shortcuts = Line::from(vec![
            Span::styled("[j/k]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-navigate")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[a]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-add")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[e]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-edit")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[d]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}  ", tr("key-delete")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(
                format!(" {}", tr("key-back")),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(vec![shortcuts]).wrap(Wrap { trim: false }),
            chunks[2],
        );
    }

    fn render_abbr_delete_confirm(&self, frame: &mut Frame, modal_area: Rect) {
        use fluent_bundle::FluentValue;
        use ratatui::widgets::Clear;

        // Smaller modal for confirmation
        let confirm_area = Rect {
            x: modal_area.x + 10,
            y: modal_area.y + 5,
            width: 40u16.min(modal_area.width.saturating_sub(20)),
            height: 5u16.min(modal_area.height.saturating_sub(10)),
        };
        frame.render_widget(Clear, confirm_area);

        let short = self
            .abbreviations
            .get(self.abbr_selected)
            .map(|a| a.short.as_str())
            .unwrap_or("?");

        let block = Block::default()
            .title(format!(" {} ", tr("abbreviations-title")))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(RED))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(confirm_area);
        frame.render_widget(block, confirm_area);

        let msg = crate::i18n::tr_args(
            "abbreviations-delete-confirm",
            &[("short", FluentValue::from(short.to_string()))],
        );
        let lines = vec![
            Line::from(vec![Span::styled(msg, Style::default().fg(RED))]),
            Line::from(vec![
                Span::styled("[y]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-yes")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[n]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-no")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .alignment(ratatui::layout::Alignment::Center),
            inner,
        );
    }

    fn render_input_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-input-title")))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text_style = if self.phase == Phase::Input || self.is_abbr_modal_active() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut lines: Vec<Line> = Vec::new();
        for (i, line_text) in self.input_lines.iter().enumerate() {
            if self.phase == Phase::Input && i == self.current_line {
                let before = &line_text[..self.cursor_pos];
                let after = &line_text[self.cursor_pos..];
                lines.push(Line::from(vec![
                    Span::styled(before, Style::default().fg(Color::White)),
                    Span::styled("_", Style::default().fg(GREEN)),
                    Span::styled(after, Style::default().fg(Color::White)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(line_text.as_str(), text_style)));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "\u{258F}",
                Style::default().fg(GREEN),
            )));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", tr("quicklog-preview-title")))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .border_style(Style::default().fg(BORDER_COLOR));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.phase {
            Phase::Input => {
                if self.llm_config.is_none() {
                    let msg = Line::from(Span::styled(
                        tr("quicklog-no-config"),
                        Style::default().fg(YELLOW),
                    ));
                    frame
                        .render_widget(Paragraph::new(vec![msg]).wrap(Wrap { trim: false }), inner);
                }
            }
            Phase::BrowseAbbreviations
            | Phase::AddAbbrShort
            | Phase::AddAbbrFull
            | Phase::EditAbbrShort
            | Phase::EditAbbrFull
            | Phase::ConfirmDeleteAbbr => {
                // Show preview results while modal is open (same as Preview phase)
                self.render_preview_results(frame, inner);
            }
            Phase::Parsing => {
                let spinner_char = SPINNER[self.spinner_frame];
                let msg = Line::from(vec![
                    Span::styled(format!("{} ", spinner_char), Style::default().fg(ACCENT)),
                    Span::styled(tr("quicklog-parsing"), Style::default().fg(Color::White)),
                ]);
                frame.render_widget(Paragraph::new(vec![msg]).wrap(Wrap { trim: false }), inner);
            }
            Phase::Preview => {
                self.render_preview_results(frame, inner);
            }
        }
    }

    fn render_preview_results(&self, frame: &mut Frame, area: Rect) {
        if self.parsed_results.is_empty() {
            let msg = Line::from(Span::styled(
                tr("quicklog-no-results"),
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(vec![msg]).wrap(Wrap { trim: false }), area);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let has_unmatched = self.parsed_results.iter().any(|r| !r.matched);

        for (i, result) in self.parsed_results.iter().enumerate() {
            let is_selected = i == self.selected_result;
            let marker = if result.matched {
                "\u{2713}"
            } else {
                "\u{2717}"
            };
            let marker_color = if result.matched { GREEN } else { RED };
            let name_color = if result.matched { Color::White } else { YELLOW };
            let name_style = if is_selected {
                Style::default().fg(name_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(name_color)
            };

            let prefix = if is_selected { "> " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(prefix, name_style),
                Span::styled(format!("{} ", marker), Style::default().fg(marker_color)),
                Span::styled(&result.practice_name, name_style),
            ]));

            // Show sets
            for (j, set_data) in result.sets.iter().enumerate() {
                let set_text = format_set(j + 1, set_data);
                let set_style = if is_selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(
                    format!("    {}", set_text),
                    set_style,
                )));
            }
        }

        if has_unmatched {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                tr("quicklog-unmatched-warning"),
                Style::default().fg(YELLOW),
            )));
        }

        // Apply scroll offset
        let visible_height = area.height as usize;
        let display_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .collect();

        frame.render_widget(
            Paragraph::new(display_lines).wrap(Wrap { trim: false }),
            area,
        );
    }

    fn build_shortcuts(&self) -> Line<'static> {
        match self.phase {
            Phase::Input => {
                let mut spans = vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        format!("{}: ", tr("quicklog-date")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(self.log_date.clone(), Style::default().fg(Color::White)),
                    Span::styled("  ", Style::default()),
                    Span::styled("[Ctrl+S]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("quicklog-key-parse")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Ctrl+O]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}  ", tr("quicklog-key-abbr")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("[Esc]", Style::default().fg(ACCENT)),
                    Span::styled(
                        format!(" {}", tr("key-back")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ];
                if self.llm_config.is_none() {
                    spans.clear();
                    spans.push(Span::styled(" ", Style::default()));
                    spans.push(Span::styled("[Ctrl+O]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(
                        format!(" {}  ", tr("quicklog-key-abbr")),
                        Style::default().fg(Color::DarkGray),
                    ));
                    spans.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                    spans.push(Span::styled(
                        format!(" {}", tr("key-back")),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                Line::from(spans)
            }
            Phase::Parsing => Line::from(vec![
                Span::styled(" [Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-cancel")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Phase::Preview => Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{}: ", tr("quicklog-date")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(self.log_date.clone(), Style::default().fg(Color::White)),
                Span::styled("  ", Style::default()),
                Span::styled("[j/k]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-navigate")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-remove")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Enter]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-save")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("quicklog-key-abbr")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("quicklog-key-edit")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Phase::BrowseAbbreviations => Line::from(vec![
                Span::styled("[j/k]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-navigate")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[a]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-add")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[e]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-edit")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[d]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-delete")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[Esc]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-back")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Phase::AddAbbrShort
            | Phase::AddAbbrFull
            | Phase::EditAbbrShort
            | Phase::EditAbbrFull => Line::from(vec![
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
            ]),
            Phase::ConfirmDeleteAbbr => Line::from(vec![
                Span::styled("[y]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}  ", tr("key-yes")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("[n]", Style::default().fg(ACCENT)),
                Span::styled(
                    format!(" {}", tr("key-no")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match self.phase {
            Phase::Input => self.handle_input(key),
            Phase::Parsing => self.handle_parsing(key),
            Phase::Preview => self.handle_preview(key, db),
            Phase::BrowseAbbreviations => self.handle_abbr_browse(key, db),
            Phase::AddAbbrShort | Phase::EditAbbrShort => self.handle_abbr_short_input(key),
            Phase::AddAbbrFull | Phase::EditAbbrFull => self.handle_abbr_full_input(key, db),
            Phase::ConfirmDeleteAbbr => self.handle_abbr_delete_confirm(key, db),
        }
    }

    fn handle_input(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.start_llm_parse();
                Action::None
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_abbr_browse();
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::Dashboard),
            KeyCode::Enter => {
                // Insert new line
                let remainder = self.input_lines[self.current_line][self.cursor_pos..].to_string();
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                self.current_line += 1;
                self.input_lines.insert(self.current_line, remainder);
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Up => {
                if self.current_line > 0 {
                    self.current_line -= 1;
                    self.cursor_pos = self
                        .cursor_pos
                        .min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Down => {
                if self.current_line < self.input_lines.len() - 1 {
                    self.current_line += 1;
                    self.cursor_pos = self
                        .cursor_pos
                        .min(self.input_lines[self.current_line].len());
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_lines[self.current_line].remove(prev);
                    self.cursor_pos = prev;
                } else if self.current_line > 0 {
                    // Merge with previous line
                    let current_text = self.input_lines.remove(self.current_line);
                    self.current_line -= 1;
                    self.cursor_pos = self.input_lines[self.current_line].len();
                    self.input_lines[self.current_line].push_str(&current_text);
                }
                Action::None
            }
            // Emacs-style cursor navigation
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor_pos = prev;
                }
                Action::None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    let prev = self.input_lines[self.current_line][..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor_pos = prev;
                }
                Action::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let line = &self.input_lines[self.current_line];
                if self.cursor_pos < line.len() {
                    let next = line[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(line.len());
                    self.cursor_pos = next;
                }
                Action::None
            }
            KeyCode::Right => {
                let line = &self.input_lines[self.current_line];
                if self.cursor_pos < line.len() {
                    let next = line[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(line.len());
                    self.cursor_pos = next;
                }
                Action::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                Action::None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::End => {
                self.cursor_pos = self.input_lines[self.current_line].len();
                Action::None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].truncate(self.cursor_pos);
                Action::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_lines[self.current_line].insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_parsing(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            self.result_receiver = None;
            self.phase = Phase::Input;
        }
        Action::None
    }

    fn handle_preview(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = (self.selected_result + 1) % self.parsed_results.len();
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.parsed_results.is_empty() {
                    self.selected_result = self
                        .selected_result
                        .checked_sub(1)
                        .unwrap_or(self.parsed_results.len() - 1);
                    self.adjust_scroll();
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.parsed_results.is_empty() {
                    self.parsed_results.remove(self.selected_result);
                    if self.selected_result >= self.parsed_results.len()
                        && !self.parsed_results.is_empty()
                    {
                        self.selected_result = self.parsed_results.len() - 1;
                    }
                    if self.parsed_results.is_empty() {
                        self.selected_result = 0;
                    }
                }
                Action::None
            }
            KeyCode::Enter | KeyCode::Char('s')
                if key.code == KeyCode::Enter || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.save_all(db)
            }
            KeyCode::Char('a') => {
                self.open_abbr_browse();
                Action::None
            }
            KeyCode::Esc => {
                self.phase = Phase::Input;
                self.parsed_results.clear();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn start_llm_parse(&mut self) {
        let config = match &self.llm_config {
            Some(c) => c.clone(),
            None => {
                self.status_msg = Some((tr("quicklog-no-config"), true));
                return;
            }
        };

        let input_text: String = self.input_lines.join("\n");
        let trimmed = input_text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let practices = self.practices.clone();
        let abbreviations = self.abbreviations.clone();

        let (tx, rx) = mpsc::channel();
        self.result_receiver = Some(rx);
        self.phase = Phase::Parsing;
        self.spinner_frame = 0;

        std::thread::spawn(move || {
            let system_prompt = llm::build_system_prompt(&practices, &abbreviations);
            let result = llm::call_llm(&config, &system_prompt, &trimmed)
                .and_then(|raw| llm::parse_llm_response(&raw, &practices));
            let _ = tx.send(result);
        });
    }

    fn save_all(&mut self, db: &Database) -> Action {
        if self.parsed_results.is_empty() {
            return Action::None;
        }

        // Check all matched
        if self.parsed_results.iter().any(|r| !r.matched) {
            self.status_msg = Some((tr("quicklog-unmatched-error"), true));
            return Action::None;
        }

        let raw_text = self.input_lines.join("\n");

        // Parse date
        let date = match chrono::NaiveDate::parse_from_str(&self.log_date, "%Y-%m-%d") {
            Ok(d) => d.and_hms_opt(12, 0, 0).unwrap(),
            Err(_) => {
                self.status_msg = Some(("Invalid date".to_string(), true));
                return Action::None;
            }
        };

        for entry in &self.parsed_results {
            // Find practice by name (case-insensitive match)
            let practice = match self
                .practices
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(&entry.practice_name))
            {
                Some(p) => p,
                None => {
                    self.status_msg =
                        Some((format!("Practice not found: {}", entry.practice_name), true));
                    return Action::None;
                }
            };

            if let Err(e) =
                db.create_log_at(practice.id, &date, &entry.sets, Some(&raw_text), None, None)
            {
                self.status_msg = Some((format!("Error: {}", e), true));
                return Action::None;
            }
        }

        Action::Navigate(Screen::Dashboard)
    }

    fn adjust_scroll(&mut self) {
        // Rough estimate: each result takes 1 + sets.len() lines
        let mut line_idx = 0;
        for (i, result) in self.parsed_results.iter().enumerate() {
            if i == self.selected_result {
                break;
            }
            line_idx += 1 + result.sets.len();
        }
        if line_idx < self.scroll_offset {
            self.scroll_offset = line_idx;
        }
        // We don't know exact visible height here, so just ensure selected is reachable
    }

    // ── Abbreviation modal methods ──

    fn start_add_abbreviation(&mut self) {
        self.abbr_short_input.clear();
        self.abbr_short_cursor = 0;
        self.abbr_full_input.clear();
        self.abbr_full_cursor = 0;
        self.abbr_editing_id = None;
        self.phase = Phase::AddAbbrShort;
    }

    fn start_edit_abbreviation(&mut self, id: i64, short: &str, full: &str) {
        self.abbr_short_input = short.to_string();
        self.abbr_short_cursor = short.len();
        self.abbr_full_input = full.to_string();
        self.abbr_full_cursor = full.len();
        self.abbr_editing_id = Some(id);
        self.phase = Phase::EditAbbrShort;
    }

    fn handle_abbr_text_input(input: &mut String, cursor: &mut usize, key: KeyEvent) {
        match key.code {
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    *cursor = prev;
                }
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    *cursor = prev;
                }
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if *cursor < input.len() {
                    let next = input[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(input.len());
                    *cursor = next;
                }
            }
            KeyCode::Right => {
                if *cursor < input.len() {
                    let next = input[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(input.len());
                    *cursor = next;
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *cursor = 0;
            }
            KeyCode::Home => {
                *cursor = 0;
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *cursor = input.len();
            }
            KeyCode::End => {
                *cursor = input.len();
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let prev = input[..*cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    input.remove(prev);
                    *cursor = prev;
                }
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.truncate(*cursor);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            _ => {}
        }
    }

    fn handle_abbr_short_input(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.abbr_short_input.trim().is_empty() {
                    self.phase = if matches!(self.phase, Phase::AddAbbrShort) {
                        Phase::AddAbbrFull
                    } else {
                        Phase::EditAbbrFull
                    };
                }
            }
            KeyCode::Esc => {
                self.phase = self.pre_abbr_phase.clone();
            }
            _ => {
                Self::handle_abbr_text_input(
                    &mut self.abbr_short_input,
                    &mut self.abbr_short_cursor,
                    key,
                );
            }
        }
        Action::None
    }

    fn handle_abbr_full_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let short_trimmed = self.abbr_short_input.trim();
                let full_trimmed = self.abbr_full_input.trim();
                if !short_trimmed.is_empty() && !full_trimmed.is_empty() {
                    if let Some(id) = self.abbr_editing_id {
                        // Update existing
                        if let Err(e) = db.update_abbreviation(id, short_trimmed, full_trimmed) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    } else {
                        // Create new
                        if let Err(e) = db.create_abbreviation(short_trimmed, full_trimmed) {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                    // Refresh abbreviations so LLM can use them
                    self.refresh_abbreviations(db);
                }
                // Reset and return to preview
                self.abbr_short_input.clear();
                self.abbr_short_cursor = 0;
                self.abbr_full_input.clear();
                self.abbr_full_cursor = 0;
                self.abbr_editing_id = None;
                self.phase = Phase::Preview;
            }
            KeyCode::Esc => {
                self.abbr_short_input.clear();
                self.abbr_short_cursor = 0;
                self.abbr_full_input.clear();
                self.abbr_full_cursor = 0;
                self.abbr_editing_id = None;
                self.phase = self.pre_abbr_phase.clone();
            }
            _ => {
                Self::handle_abbr_text_input(
                    &mut self.abbr_full_input,
                    &mut self.abbr_full_cursor,
                    key,
                );
            }
        }
        Action::None
    }

    // ── Abbreviation browse mode handlers ──

    fn open_abbr_browse(&mut self) {
        self.pre_abbr_phase = self.phase.clone();
        self.abbr_selected = 0;
        self.phase = Phase::BrowseAbbreviations;
    }

    fn handle_abbr_browse(&mut self, key: KeyEvent, _db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.abbreviations.is_empty() {
                    self.abbr_selected = (self.abbr_selected + 1) % self.abbreviations.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.abbreviations.is_empty() {
                    self.abbr_selected = self
                        .abbr_selected
                        .checked_sub(1)
                        .unwrap_or(self.abbreviations.len() - 1);
                }
            }
            KeyCode::Char('a') => {
                self.start_add_abbreviation();
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(abbr) = self.abbreviations.get(self.abbr_selected) {
                    let id = abbr.id;
                    let short = abbr.short.clone();
                    let full = abbr.full_name.clone();
                    self.start_edit_abbreviation(id, &short, &full);
                }
            }
            KeyCode::Char('d') => {
                if !self.abbreviations.is_empty() {
                    self.phase = Phase::ConfirmDeleteAbbr;
                }
            }
            KeyCode::Esc => {
                self.phase = self.pre_abbr_phase.clone();
            }
            _ => {}
        }
        Action::None
    }

    fn handle_abbr_delete_confirm(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(abbr) = self.abbreviations.get(self.abbr_selected) {
                    if let Err(e) = db.delete_abbreviation(abbr.id) {
                        self.status_msg = Some((format!("Delete failed: {}", e), true));
                    } else {
                        // Refresh and adjust selection
                        self.refresh_abbreviations(db);
                        if self.abbr_selected >= self.abbreviations.len()
                            && !self.abbreviations.is_empty()
                        {
                            self.abbr_selected = self.abbreviations.len() - 1;
                        }
                    }
                }
                self.phase = Phase::BrowseAbbreviations;
            }
            _ => {
                // Any other key cancels
                self.phase = Phase::BrowseAbbreviations;
            }
        }
        Action::None
    }
}

fn format_set(number: usize, data: &SetData) -> String {
    match data {
        SetData::Weighted { weight, reps } => {
            format!("Set {}: {}kg x {}", number, weight, reps)
        }
        SetData::Bodyweight { reps } => {
            format!("Set {}: {} reps", number, reps)
        }
        SetData::Distance { distance } => {
            format!("Set {}: {} km", number, distance)
        }
        SetData::Endurance { duration } => {
            format!("Set {}: {} min", number, duration)
        }
    }
}
