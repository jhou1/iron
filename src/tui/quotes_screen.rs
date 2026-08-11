use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use super::lifecycle::{NavigationStatus, Reenter};
use super::{
    centered_area,
    framework::{
        fit_rect, render_detail_panel, render_shortcuts, shortcut_height, truncate_to_width,
        visual_spans, DetailPanelState, Shortcut, TextInput, ViewportState, VisualSpan,
    },
    highlight_row, render_status_line, Action, ModuleId, NavigationIntent, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::tr;
use crate::model::Quote;

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    Detail,
    Add,
    Edit,
    ConfirmDelete,
}

pub struct QuotesScreen {
    quotes: Vec<Quote>,
    selected: usize,
    list_viewport: ViewportState,
    detail_state: DetailPanelState,
    detail_content_height: usize,
    mode: Mode,
    input: TextInput,
    editing_id: Option<i64>,
    status_msg: StatusMessage,
    last_deleted: Option<Quote>,
}

impl QuotesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let quotes = db.list_quotes()?;
        Ok(Self {
            quotes,
            selected: 0,
            list_viewport: ViewportState::new(0),
            detail_state: DetailPanelState::new(false),
            detail_content_height: 0,
            mode: Mode::Browse,
            input: TextInput::new(),
            editing_id: None,
            status_msg: None,
            last_deleted: None,
        })
    }

    fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        let selected_id = self.quotes.get(self.selected).map(|quote| quote.id);
        self.quotes = db.list_quotes()?;
        self.selected = selected_id
            .and_then(|id| self.quotes.iter().position(|quote| quote.id == id))
            .unwrap_or_else(|| self.selected.min(self.quotes.len().saturating_sub(1)));
        if self.quotes.is_empty() {
            self.selected = 0;
        }
        self.list_viewport.clamp(self.quotes.len());
        Ok(())
    }

    fn active_shortcuts(&self) -> Vec<Shortcut> {
        let mut shortcuts = match &self.mode {
            Mode::Browse => vec![
                Shortcut::new("j/k", tr("key-navigate")),
                Shortcut::new("Enter", tr("key-details")),
                Shortcut::new("a", tr("key-add")),
                Shortcut::new("e", tr("key-edit")),
                Shortcut::new("d", tr("key-delete")),
            ],
            Mode::Detail => vec![
                Shortcut::new("j/k", tr("key-scroll")),
                Shortcut::new("Esc", tr("key-back")),
            ],
            Mode::Add | Mode::Edit => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
            Mode::ConfirmDelete => vec![
                Shortcut::new("y", tr("key-yes")),
                Shortcut::new("any", tr("key-cancel")),
            ],
        };
        if self.mode == Mode::Browse && self.last_deleted.is_some() {
            shortcuts.push(Shortcut::new("u", tr("key-undo")));
        }
        if self.mode == Mode::Browse {
            shortcuts.push(Shortcut::new("Esc", tr("key-back")));
        }
        shortcuts
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let shortcuts = self.active_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let action_height: u16 = match &self.mode {
            Mode::Browse | Mode::Detail => 0,
            _ => 3,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),                // bordered list
                Constraint::Length(1),             // spacer
                Constraint::Length(action_height), // input/action area
                Constraint::Length(1),             // status message
                Constraint::Length(shortcut_rows), // shortcuts
                Constraint::Min(0),                // spacer
            ])
            .split(area);

        // ── Bordered quote list ──
        let visible = chunks[0].height.saturating_sub(4) as usize;
        self.list_viewport.set_height(visible);
        if !self.quotes.is_empty() {
            self.list_viewport.reveal(VisualSpan {
                start: self.selected,
                height: 1,
            });
        }
        self.list_viewport.clamp(self.quotes.len());
        let overflow = self.list_viewport.overflow(self.quotes.len()).suffix();
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    format!("{}{}", tr("dashboard-quotes"), overflow),
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

        if self.quotes.is_empty() {
            let hint_content = Line::from(Span::styled(
                tr("dashboard-no-quotes-modal"),
                Style::default().fg(Color::Gray),
            ));
            let hint = Paragraph::new(hint_content).wrap(Wrap { trim: false });
            frame.render_widget(hint, inner);
        } else {
            let lines: Vec<Line> = self
                .quotes
                .iter()
                .enumerate()
                .skip(self.list_viewport.offset())
                .take(visible)
                .map(|(i, q)| {
                    let is_sel = i == self.selected;
                    let marker = if is_sel { "> " } else { "  " };
                    let style = if is_sel {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let max_text = inner.width.saturating_sub(3) as usize;
                    let display = truncate_to_width(&q.text, max_text);
                    let display = format!("{marker}{display}");
                    Line::from(Span::styled(display, style))
                })
                .collect();

            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

            if self.selected >= self.list_viewport.offset() {
                let row = (self.selected - self.list_viewport.offset()) as u16;
                highlight_row(frame, inner, row);
            }
        }

        if matches!(self.mode, Mode::Browse | Mode::Detail) {
            if let Some(quote) = self.quotes.get(self.selected) {
                let popup_x = inner.x + 4.min(inner.width);
                let popup_w = inner
                    .width
                    .saturating_sub(8)
                    .max(30)
                    .min(inner.right().saturating_sub(popup_x));
                let popup_inner_width = popup_w.saturating_sub(2).max(1);
                let lines = vec![Line::from(quote.text.clone())];
                let spans = visual_spans(&lines, popup_inner_width);
                self.detail_content_height = spans.first().map(|span| span.height).unwrap_or(0);
                let popup_h = if self.mode == Mode::Detail {
                    inner.height
                } else {
                    (self.detail_content_height as u16 + 2)
                        .max(4)
                        .min(inner.height / 2)
                };
                let popup_rect = fit_rect(
                    inner,
                    ratatui::layout::Rect {
                        x: popup_x,
                        y: if self.mode == Mode::Detail {
                            inner.y
                        } else {
                            inner.y + inner.height.saturating_sub(popup_h)
                        },
                        width: popup_w,
                        height: popup_h,
                    },
                );
                self.detail_state.focused = self.mode == Mode::Detail;
                if !self.detail_state.focused {
                    self.detail_state.viewport.set_offset(0);
                }
                frame.render_widget(Clear, popup_rect);
                render_detail_panel(
                    frame,
                    popup_rect,
                    format!(" {} ", tr("dashboard-quotes")),
                    lines,
                    &mut self.detail_state,
                    None,
                );
            }
        }

        // ── Action area ──
        if action_height > 0 {
            let action_lines = match &self.mode {
                Mode::Add | Mode::Edit => {
                    let label = if self.mode == Mode::Add {
                        tr("key-add")
                    } else {
                        tr("key-edit")
                    };
                    let prefix = format!(" {}: ", label);
                    let prefix_width = Line::from(prefix.as_str()).width() as u16;
                    let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Gray))];
                    spans.extend(self.input.visible_spans(area.width, prefix_width, GREEN));
                    vec![
                        Line::from(spans),
                        Line::from(""),
                        Line::from(vec![
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
                    ]
                }
                Mode::ConfirmDelete => {
                    vec![
                        Line::from(Span::styled(
                            format!(" {}", tr("quotes-delete-confirm")),
                            Style::default().fg(Color::Red),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(" [y]", Style::default().fg(ACCENT)),
                            Span::styled(
                                format!(" {}  ", tr("key-yes")),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled("[any]", Style::default().fg(ACCENT)),
                            Span::styled(
                                format!(" {}", tr("key-cancel")),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]),
                    ]
                }
                Mode::Browse | Mode::Detail => vec![],
            };
            frame.render_widget(
                Paragraph::new(action_lines).wrap(Wrap { trim: false }),
                chunks[2],
            );
        }

        // ── Status line ──
        render_status_line(frame, chunks[3], &self.status_msg);

        // ── Shortcuts ──
        render_shortcuts(frame, chunks[4], &shortcuts);
    }

    pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action {
        self.status_msg = None;
        match &self.mode {
            Mode::Browse => self.handle_browse(key, db),
            Mode::Detail => self.handle_detail(key),
            Mode::Add | Mode::Edit => self.handle_input(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.quotes.is_empty() && self.selected < self.quotes.len() - 1 {
                    self.selected += 1;
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.editing_id = None;
                self.mode = Mode::Add;
                Action::None
            }
            KeyCode::Enter => {
                if !self.quotes.is_empty() {
                    self.mode = Mode::Detail;
                    self.detail_state.focused = true;
                    self.detail_state.viewport.set_offset(0);
                }
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(q) = self.quotes.get(self.selected) {
                    self.input.set(q.text.clone());
                    self.editing_id = Some(q.id);
                    self.mode = Mode::Edit;
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.quotes.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Char('u') => {
                if let Some(quote) = self.last_deleted.take() {
                    match db.restore_quote(&quote) {
                        Ok(_) => match self.refresh(db) {
                            Ok(()) => {
                                self.status_msg = Some((tr("status-restored"), false));
                            }
                            Err(error) => {
                                self.status_msg = Some((format!("Error: {error}"), true));
                            }
                        },
                        Err(e) => {
                            self.last_deleted = Some(quote);
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard)),
            _ => Action::None,
        }
    }

    fn handle_detail(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let content_height = if self.detail_content_height == 0 {
                    usize::MAX
                } else {
                    self.detail_content_height
                };
                self.detail_state.scroll_by(1, content_height);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_state
                    .scroll_by(-1, self.detail_content_height.max(1));
            }
            KeyCode::Esc | KeyCode::Enter => {
                self.mode = Mode::Browse;
                self.detail_state.focused = false;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_input(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let text = self.input.trimmed().to_string();
                if text.is_empty() {
                    self.status_msg = Some((tr("quotes-empty-error"), true));
                    return Action::None;
                }
                let result = if let Some(id) = self.editing_id {
                    db.update_quote(id, &text)
                } else {
                    db.create_quote(&text).map(|_| ())
                };
                match result {
                    Ok(()) => match self.refresh(db) {
                        Ok(()) => {
                            self.input.clear();
                            self.editing_id = None;
                            self.mode = Mode::Browse;
                        }
                        Err(error) => {
                            self.status_msg = Some((format!("Error: {error}"), true));
                        }
                    },
                    Err(error) => {
                        self.status_msg = Some((format!("Error: {error}"), true));
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.editing_id = None;
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, db: &Database) -> Action {
        if key.code == KeyCode::Char('y') {
            if let Some(q) = self.quotes.get(self.selected) {
                let quote_clone = q.clone();
                let id = q.id;
                match db.delete_quote(id) {
                    Ok(()) => {
                        self.last_deleted = Some(quote_clone);
                        match self.refresh(db) {
                            Ok(()) => {
                                self.status_msg = Some((tr("status-deleted-undo"), false));
                                self.mode = Mode::Browse;
                            }
                            Err(error) => {
                                self.status_msg = Some((format!("Error: {error}"), true));
                            }
                        }
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Error: {}", e), true));
                    }
                }
            }
        } else {
            self.mode = Mode::Browse;
        }
        Action::None
    }
}

impl Reenter for QuotesScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        let selected_id = self.quotes.get(self.selected).map(|quote| quote.id);
        let mut next = Self::new(db)?;

        next.selected = selected_id
            .and_then(|id| next.quotes.iter().position(|quote| quote.id == id))
            .unwrap_or_else(|| self.selected.min(next.quotes.len().saturating_sub(1)));
        if next.quotes.is_empty() {
            next.selected = 0;
        }

        next.list_viewport = self.list_viewport;
        next.list_viewport.clamp(next.quotes.len());
        next.detail_state.viewport = self.detail_state.viewport;
        next.detail_content_height = self.detail_content_height;
        next.detail_state.viewport.clamp(next.detail_content_height);

        next.mode = if self.mode == Mode::Detail && !next.quotes.is_empty() {
            Mode::Detail
        } else {
            Mode::Browse
        };
        next.detail_state.focused = next.mode == Mode::Detail;
        next.input.clear();
        next.editing_id = None;
        next.status_msg = None;
        next.last_deleted = None;

        Ok(next)
    }
}

impl NavigationStatus for QuotesScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::render_to_string;
    use crate::tui::lifecycle::{NavigationStatus, Reenter};
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render_quotes(screen: &mut QuotesScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    #[test]
    fn quotes_reentry_preserves_detail_context_and_clears_mutations() {
        let db = Database::open_in_memory().unwrap();
        let first = db.create_quote("First quote").unwrap();
        let second = db.create_quote("Second quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.selected = screen
            .quotes
            .iter()
            .position(|quote| quote.id == second.id)
            .unwrap();
        screen.mode = Mode::Detail;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(1);
        screen.detail_state.focused = true;
        screen.detail_state.viewport.set_height(1);
        screen.detail_state.viewport.set_offset(2);
        screen.detail_content_height = 4;
        screen.input.set("discard me");
        screen.status_msg = Some(("old".into(), true));
        screen.last_deleted = Some(screen.quotes[0].clone());
        db.update_quote(first.id, "First changed").unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.quotes[next.selected].id, second.id);
        assert_eq!(next.list_viewport.offset(), 1);
        assert_eq!(next.mode, Mode::Detail);
        assert!(next.detail_state.focused);
        assert_eq!(next.detail_state.viewport.offset(), 2);
        assert_eq!(next.detail_content_height, 4);
        assert!(next.input.value().is_empty());
        assert!(next.editing_id.is_none());
        assert!(next.status_msg.is_none());
        assert!(next.last_deleted.is_none());
        assert_eq!(
            next.quotes.iter().find(|q| q.id == first.id).unwrap().text,
            "First changed"
        );
    }

    #[test]
    fn quotes_reentry_returns_edit_mode_to_browse_and_clears_editing_id() {
        let db = Database::open_in_memory().unwrap();
        let quote = db.create_quote("Quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.mode = Mode::Edit;
        screen.input.set("discard me");
        screen.editing_id = Some(quote.id);

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.mode, Mode::Browse);
        assert!(next.input.value().is_empty());
        assert!(next.editing_id.is_none());
    }

    #[test]
    fn quotes_reentry_clamps_selection_and_viewports_after_selected_deletion() {
        let db = Database::open_in_memory().unwrap();
        db.create_quote("First quote").unwrap();
        let second = db.create_quote("Second quote").unwrap();
        let third = db.create_quote("Third quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.selected = 2;
        screen.mode = Mode::Detail;
        screen.detail_state.focused = true;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(2);
        screen.detail_state.viewport.set_height(1);
        screen.detail_state.viewport.set_offset(4);
        screen.detail_content_height = 4;
        db.delete_quote(third.id).unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.selected, 1);
        assert_eq!(next.quotes[next.selected].id, second.id);
        assert_eq!(next.list_viewport.offset(), 1);
        assert_eq!(next.detail_state.viewport.offset(), 3);
        assert_eq!(next.mode, Mode::Detail);
        assert!(next.detail_state.focused);
    }

    #[test]
    fn quotes_navigation_error_uses_status_line() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();

        screen.set_navigation_error("navigation failed".into());

        assert_eq!(screen.status_msg, Some(("navigation failed".into(), true)));
    }

    #[test]
    fn selected_long_quote_has_complete_focusable_preview() {
        let db = Database::open_in_memory().unwrap();
        let text = "Long quote ".repeat(80);
        db.create_quote(&text).unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Enter), &db);
        for _ in 0..20 {
            screen.handle_key(key(KeyCode::Down), &db);
        }

        let output = render_quotes(&mut screen, 80, 24);

        assert_eq!(screen.mode, Mode::Detail);
        assert!(output.contains('↑'), "{output}");
    }

    #[test]
    fn empty_quote_edit_retains_input_mode() {
        let db = Database::open_in_memory().unwrap();
        db.create_quote("Quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Char('e')), &db);
        screen.input.clear();

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, Mode::Edit);
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn quote_popup_stays_inside_small_terminal() {
        let db = Database::open_in_memory().unwrap();
        db.create_quote("Quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();

        let output = render_quotes(&mut screen, 24, 10);

        assert!(!output.is_empty());
    }

    #[test]
    fn quotes_reserve_wrapped_shortcuts_after_delete() {
        let db = Database::open_in_memory().unwrap();
        db.create_quote("Quote").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.last_deleted = screen.quotes.first().cloned();

        let output = render_quotes(&mut screen, 80, 24);

        assert!(output.contains("[u] Undo"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }
}
