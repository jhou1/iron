use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Clear,
    Frame,
};

use super::lifecycle::{NavigationStatus, Reenter};
use super::{
    framework::{
        fit_rect, render_action_panel, render_detail_panel, render_screen_frame_sections,
        render_scrollable_list_panel, truncate_to_width, visual_spans, DetailPanelState,
        ListPanelAreas, ListPanelContent, ScreenFrameSections, Shortcut, TextInput, ViewportState,
    },
    Action, ModuleId, NavigationIntent, StatusMessage, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::tr;
use crate::model::Quote;

const ACCENT: Color = Color::Cyan;
const GREEN: Color = Color::Green;

fn quote_list_content(
    quotes: &[Quote],
    selected: usize,
    inner: ratatui::layout::Rect,
) -> ListPanelContent<'static> {
    let rows = quotes
        .iter()
        .enumerate()
        .map(|(index, quote)| {
            let selected = index == selected;
            let marker = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let max_text = inner.width.saturating_sub(3) as usize;
            let text = truncate_to_width(&quote.text, max_text);
            Line::from(Span::styled(format!("{marker}{text}"), style))
        })
        .collect();

    ListPanelContent {
        header: None,
        rows,
        empty: Some(Line::from(Span::styled(
            tr("dashboard-no-quotes-modal"),
            Style::default().fg(Color::Gray),
        ))),
        selected: (!quotes.is_empty()).then_some(selected),
    }
}

fn quote_preview_rect(
    inner: ratatui::layout::Rect,
    focused: bool,
    content_height: usize,
) -> ratatui::layout::Rect {
    let x = inner.x.saturating_add(4.min(inner.width));
    let width = quote_preview_width(inner);
    let height = if focused {
        inner.height
    } else {
        u16::try_from(content_height)
            .unwrap_or(u16::MAX)
            .saturating_add(2)
            .max(4)
            .min(inner.height / 2)
    };
    fit_rect(
        inner,
        ratatui::layout::Rect {
            x,
            y: if focused {
                inner.y
            } else {
                inner.y.saturating_add(inner.height.saturating_sub(height))
            },
            width,
            height,
        },
    )
}

fn quote_preview_width(inner: ratatui::layout::Rect) -> u16 {
    let x = inner.x.saturating_add(4.min(inner.width));
    inner
        .width
        .saturating_sub(8)
        .max(30)
        .min(inner.right().saturating_sub(x))
}

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

    fn action_content(&self, width: u16) -> Vec<Line<'static>> {
        match &self.mode {
            Mode::Add | Mode::Edit => {
                let label = if self.mode == Mode::Add {
                    tr("key-add")
                } else {
                    tr("key-edit")
                };
                let prefix = format!(" {}: ", label);
                let prefix_width = Line::from(prefix.as_str()).width() as u16;
                let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Gray))];
                spans.extend(self.input.visible_spans(width, prefix_width, GREEN));
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
            Mode::ConfirmDelete => vec![
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
            ],
            Mode::Browse | Mode::Detail => Vec::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let shortcuts = self.active_shortcuts();
        let action_height = match &self.mode {
            Mode::Browse | Mode::Detail => 0,
            Mode::Add | Mode::Edit | Mode::ConfirmDelete => 3,
        };
        let area = frame.area();
        let screen: ScreenFrameSections = render_screen_frame_sections(
            frame,
            area,
            CONTENT_WIDTH,
            &self.status_msg,
            &shortcuts,
            &[
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(action_height),
            ],
            true,
        );

        let quotes = &self.quotes;
        let selected = self.selected;
        let list: ListPanelAreas = render_scrollable_list_panel(
            frame,
            screen.body[0],
            tr("dashboard-quotes"),
            &mut self.list_viewport,
            |inner| quote_list_content(quotes, selected, inner),
        );

        if matches!(self.mode, Mode::Browse | Mode::Detail) {
            if let Some(quote) = self.quotes.get(self.selected) {
                let focused = self.mode == Mode::Detail;
                let preview_width = quote_preview_width(list.inner).saturating_sub(2).max(1);
                let lines = vec![Line::from(quote.text.clone())];
                self.detail_content_height = visual_spans(&lines, preview_width)
                    .first()
                    .map(|span| span.height)
                    .unwrap_or(0);
                let popup = quote_preview_rect(list.inner, focused, self.detail_content_height);
                self.detail_state.focused = focused;
                if !focused {
                    self.detail_state.viewport.set_offset(0);
                }
                frame.render_widget(Clear, popup);
                render_detail_panel(
                    frame,
                    popup,
                    format!(" {} ", tr("dashboard-quotes")),
                    lines,
                    &mut self.detail_state,
                    None,
                );
            }
        }

        render_action_panel(
            frame,
            screen.body[2],
            self.action_content(screen.area.width),
            None,
        );
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
    use crate::tui::framework::test_support::{render_to_snapshot, render_to_string};
    use crate::tui::lifecycle::{NavigationStatus, Reenter};
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render_quotes(screen: &mut QuotesScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    fn rect_is_within(parent: ratatui::layout::Rect, child: ratatui::layout::Rect) -> bool {
        child.x >= parent.x
            && child.y >= parent.y
            && child.right() <= parent.right()
            && child.bottom() <= parent.bottom()
    }

    fn golden_quotes_screen(name: &str) -> QuotesScreen {
        std::env::set_var("LC_ALL", "C");
        crate::i18n::init();

        let db = Database::open_in_memory().unwrap();
        db.create_quote("First compact quote").unwrap();
        let long = db
            .create_quote(
                &"A deliberately long quote with Unicode 训练 and enough words to wrap. ".repeat(8),
            )
            .unwrap();
        db.create_quote("Third quote for list scrolling").unwrap();

        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.selected = screen
            .quotes
            .iter()
            .position(|quote| quote.id == long.id)
            .unwrap();
        screen.status_msg = Some(("Fixture status".into(), false));

        match name {
            "browse" => {}
            "browse_with_undo" => {
                screen.last_deleted = Some(Quote {
                    id: 9_001,
                    text: "Deleted quote available for undo".into(),
                    position: 9,
                });
            }
            "detail" => {
                screen.mode = Mode::Detail;
                screen.detail_state.focused = true;
                screen.detail_state.viewport.set_offset(2);
            }
            "add" => {
                screen.mode = Mode::Add;
                screen.input.set("新增 quote");
            }
            "edit" => {
                screen.mode = Mode::Edit;
                screen.input.set("Edited quote text");
                screen.editing_id = Some(long.id);
            }
            "confirm_delete" => screen.mode = Mode::ConfirmDelete,
            other => panic!("unknown Quotes golden state: {other}"),
        }

        screen
    }

    fn quotes_golden(name: &str) -> &'static str {
        match name {
            "browse" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/browse.txt"
            )),
            "browse_with_undo" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/browse_with_undo.txt"
            )),
            "detail" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/detail.txt"
            )),
            "add" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/add.txt"
            )),
            "edit" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/edit.txt"
            )),
            "confirm_delete" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/quotes/confirm_delete.txt"
            )),
            other => panic!("unknown Quotes golden fixture: {other}"),
        }
    }

    #[test]
    fn quotes_modes_match_golden_buffers() {
        for name in [
            "browse",
            "browse_with_undo",
            "detail",
            "add",
            "edit",
            "confirm_delete",
        ] {
            let mut screen = golden_quotes_screen(name);
            let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
            let expected = quotes_golden(name);
            assert_eq!(actual, expected, "Quotes {name} buffer changed");
        }
    }

    #[test]
    fn quote_preview_rect_stays_inside_list_inner() {
        for inner in [
            ratatui::layout::Rect::new(0, 0, 0, 0),
            ratatui::layout::Rect::new(5, 5, 1, 1),
            ratatui::layout::Rect::new(2, 3, 12, 4),
            ratatui::layout::Rect::new(4, 2, 72, 15),
        ] {
            for focused in [false, true] {
                let preview = quote_preview_rect(inner, focused, 40);
                assert!(rect_is_within(inner, preview), "{inner:?} -> {preview:?}");
            }
        }
    }

    #[test]
    fn quote_preview_rect_clamps_extreme_content_height_and_coordinates() {
        let large = ratatui::layout::Rect::new(0, 0, u16::MAX, u16::MAX);
        let expected = ratatui::layout::Rect::new(4, 32_768, 65_527, 32_767);
        let previews: Vec<_> = [u16::MAX as usize - 1, u16::MAX as usize, usize::MAX]
            .into_iter()
            .map(|content_height| {
                std::panic::catch_unwind(|| quote_preview_rect(large, false, content_height)).ok()
            })
            .collect();

        let edge = ratatui::layout::Rect {
            x: u16::MAX - 2,
            y: u16::MAX - 2,
            width: 8,
            height: 8,
        };
        let edge_preview =
            std::panic::catch_unwind(|| quote_preview_rect(edge, false, usize::MAX)).ok();

        assert_eq!(previews, vec![Some(expected); 3]);
        assert!(previews
            .iter()
            .flatten()
            .all(|preview| rect_is_within(large, *preview)));
        assert_eq!(
            edge_preview,
            Some(ratatui::layout::Rect::new(u16::MAX, u16::MAX, 0, 0))
        );
        assert!(rect_is_within(edge, edge_preview.unwrap()));
    }

    #[test]
    fn quotes_all_modes_render_in_tiny_terminals_without_panicking() {
        for name in [
            "browse",
            "browse_with_undo",
            "detail",
            "add",
            "edit",
            "confirm_delete",
        ] {
            for (width, height) in [(1, 1), (12, 4), (24, 10), (40, 6)] {
                let mut screen = golden_quotes_screen(name);
                let _ = render_quotes(&mut screen, width, height);
            }
        }
    }

    #[test]
    fn quote_write_failure_retains_edit_state() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        let quote = db.create_quote("Original").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Char('e')), &db);
        screen.input.set("Retry me");
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute("DROP TABLE quotes", [])
            .unwrap();

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, Mode::Edit);
        assert_eq!(screen.editing_id, Some(quote.id));
        assert_eq!(screen.input.value(), "Retry me");
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn quote_delete_failure_retains_confirmation() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        db.create_quote("Delete me").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        screen.handle_key(key(KeyCode::Char('d')), &db);
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute("DROP TABLE quotes", [])
            .unwrap();

        screen.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(screen.mode, Mode::ConfirmDelete);
        assert!(screen.last_deleted.is_none());
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn quote_restore_failure_restores_undo_payload() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        db.create_quote("Current").unwrap();
        let mut screen = QuotesScreen::new(&db).unwrap();
        let deleted = Quote {
            id: 9_001,
            text: "Retry restore".into(),
            position: 9,
        };
        screen.last_deleted = Some(deleted.clone());
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute("DROP TABLE quotes", [])
            .unwrap();

        screen.handle_key(key(KeyCode::Char('u')), &db);

        assert_eq!(screen.mode, Mode::Browse);
        assert_eq!(screen.last_deleted, Some(deleted));
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
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
    fn quotes_reentry_clears_every_transient_mode_and_field() {
        let db = Database::open_in_memory().unwrap();
        let quote = db.create_quote("Quote").unwrap();

        for mode in [Mode::Add, Mode::Edit, Mode::ConfirmDelete] {
            let mut screen = QuotesScreen::new(&db).unwrap();
            screen.mode = mode.clone();
            screen.input.set("discard me");
            screen.editing_id = Some(quote.id);
            screen.status_msg = Some(("old status".into(), true));
            screen.last_deleted = Some(screen.quotes[0].clone());

            let next = screen.reenter(&db).unwrap();

            assert_eq!(next.quotes[next.selected].id, quote.id, "{mode:?}");
            assert_eq!(next.mode, Mode::Browse, "{mode:?}");
            assert!(next.input.value().is_empty(), "{mode:?}");
            assert!(next.editing_id.is_none(), "{mode:?}");
            assert!(next.status_msg.is_none(), "{mode:?}");
            assert!(next.last_deleted.is_none(), "{mode:?}");
        }
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
