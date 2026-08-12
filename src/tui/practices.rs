use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::lifecycle::{NavigationStatus, Reenter};
use super::{
    centered_area,
    framework::{
        move_selection, render_shortcuts, shortcut_height, truncate_to_width, EdgeBehavior,
        Shortcut, TextInput, ViewportState, VisualSpan,
    },
    highlight_row, render_status_line, Action, ModuleId, NavigationIntent, StatusMessage,
    BORDER_COLOR, CONTENT_WIDTH,
};
use crate::db::Database;
use crate::i18n::{tr, tr_args};
use crate::model::{Practice, PracticeType};
use fluent_bundle::FluentValue;

const GREEN: Color = Color::Green;
const RED: Color = Color::Red;

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    AddName,
    AddType,
    EditName,
    ConfirmDelete,
}

pub struct PracticesScreen {
    practices: Vec<Practice>,
    selected: usize,
    list_viewport: ViewportState,
    mode: Mode,
    input: TextInput,
    type_selected: usize,
    status_msg: StatusMessage,
}

impl PracticesScreen {
    pub fn new(db: &Database) -> anyhow::Result<Self> {
        let practices = db.list_practices()?;
        Ok(Self {
            practices,
            selected: 0,
            list_viewport: ViewportState::new(0),
            mode: Mode::Browse,
            input: TextInput::new(),
            type_selected: 0,
            status_msg: None,
        })
    }

    fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
        let selected_id = self
            .practices
            .get(self.selected)
            .map(|practice| practice.id);
        self.practices = db.list_practices()?;
        self.selected = selected_id
            .and_then(|id| self.practices.iter().position(|practice| practice.id == id))
            .unwrap_or_else(|| self.selected.min(self.practices.len().saturating_sub(1)));
        if self.practices.is_empty() {
            self.selected = 0;
        }
        self.list_viewport.clamp(self.practices.len());
        Ok(())
    }

    fn active_shortcuts(&self) -> Vec<Shortcut> {
        match &self.mode {
            Mode::Browse => vec![
                Shortcut::new("j/k", tr("key-navigate")),
                Shortcut::new("a", tr("key-add")),
                Shortcut::new("e", tr("key-edit")),
                Shortcut::new("Space", tr("key-toggle")),
                Shortcut::new("d", tr("key-delete")),
                Shortcut::new("Esc", tr("key-back")),
            ],
            Mode::AddName | Mode::EditName => vec![
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
            Mode::AddType => vec![
                Shortcut::new("j/k", tr("key-select")),
                Shortcut::new("Enter", tr("key-confirm")),
                Shortcut::new("Esc", tr("key-cancel")),
            ],
            Mode::ConfirmDelete => vec![
                Shortcut::new("y", tr("key-yes")),
                Shortcut::new("n", tr("key-no")),
            ],
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = centered_area(frame.area(), CONTENT_WIDTH);
        let shortcuts = self.active_shortcuts();
        let shortcut_rows = shortcut_height(&shortcuts, area.width);

        let action_height: u16 = match &self.mode {
            Mode::Browse => 0,
            _ => 6,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),                // bordered list
                Constraint::Length(1),             // spacer
                Constraint::Length(action_height), // input/action area
                Constraint::Length(1),             // status message
                Constraint::Length(shortcut_rows), // shortcuts
            ])
            .split(area);

        // ── Bordered practice list ──
        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let visible_height = chunks[0].height.saturating_sub(5) as usize;
        self.list_viewport.set_height(visible_height);
        if !self.practices.is_empty() {
            self.list_viewport.reveal(VisualSpan {
                start: self.selected,
                height: 1,
            });
        }
        self.list_viewport.clamp(self.practices.len());
        let overflow = self.list_viewport.overflow(self.practices.len()).suffix();
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    format!("{}{}", tr("practices-title"), overflow),
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

        let desired_type_width = self
            .practices
            .iter()
            .map(|practice| practice.practice_type.label().width())
            .max()
            .unwrap_or(0)
            .max(type_header.width());
        let type_col_width = desired_type_width.min((inner.width as usize) / 3);
        let name_col_width = (inner.width as usize)
            .saturating_sub(2) // marker
            .saturating_sub(type_col_width)
            .saturating_sub(2); // active toggle
        let name_header = truncate_to_width(&name_header, name_col_width);
        let type_header = truncate_to_width(&type_header, type_col_width);
        let header_padding = name_col_width.saturating_sub(name_header.width());
        let type_header_padding = type_col_width.saturating_sub(type_header.width());

        let hdr_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let header_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(&name_header, hdr_style),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(&type_header, hdr_style),
            Span::raw(" ".repeat(type_header_padding + 2)),
        ]);

        let mut all_lines = vec![header_line];

        if self.practices.is_empty() {
            all_lines.push(Line::from(Span::styled(
                tr("practices-no-items"),
                Style::default().fg(Color::Gray),
            )));
        } else {
            for (i, p) in self
                .practices
                .iter()
                .enumerate()
                .skip(self.list_viewport.offset())
                .take(visible_height)
            {
                let marker = if i == self.selected { "> " } else { "  " };
                let (name_style, type_color) = if i == self.selected {
                    if p.active {
                        (
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                            Color::Gray,
                        )
                    } else {
                        (
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                            Color::DarkGray,
                        )
                    }
                } else if p.active {
                    (Style::default().fg(Color::White), Color::Gray)
                } else {
                    (Style::default().fg(Color::DarkGray), Color::DarkGray)
                };
                let name = truncate_to_width(&p.name, name_col_width);
                let padding = name_col_width.saturating_sub(name.width());
                let type_label = truncate_to_width(&p.practice_type.label(), type_col_width);
                let type_padding = type_col_width.saturating_sub(type_label.width());
                let toggle = if p.active {
                    vec![
                        Span::styled("\u{25B0}", Style::default().fg(GREEN)),
                        Span::styled("\u{25B1}", Style::default().fg(Color::DarkGray)),
                    ]
                } else {
                    vec![
                        Span::styled("\u{25B1}", Style::default().fg(Color::DarkGray)),
                        Span::styled("\u{25B0}", Style::default().fg(Color::DarkGray)),
                    ]
                };
                let mut spans = vec![
                    Span::styled(marker, name_style),
                    Span::styled(name, name_style),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(type_label, Style::default().fg(type_color)),
                    Span::raw(" ".repeat(type_padding)),
                ];
                spans.extend(toggle);
                all_lines.push(Line::from(spans));
            }
        }
        frame.render_widget(Paragraph::new(all_lines).wrap(Wrap { trim: false }), inner);

        if !self.practices.is_empty() && self.selected >= self.list_viewport.offset() {
            let row = (self.selected - self.list_viewport.offset()) as u16 + 1;
            highlight_row(frame, inner, row);
        }

        // ── Input/action area ──
        let action_lines = match &self.mode {
            Mode::Browse => {
                vec![
                    Line::from(""),
                    Line::from(""),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddName => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(self.input.visible_spans(area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("practices-new-name"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::AddType => {
                let mut lines: Vec<Line> = vec![Line::from(Span::styled(
                    tr("practices-select-type"),
                    Style::default().fg(Color::White),
                ))];
                for (i, pt) in PracticeType::ALL.iter().enumerate() {
                    let marker = if i == self.type_selected { "> " } else { "  " };
                    let style = if i == self.type_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    lines.push(Line::from(Span::styled(
                        format!(" {}{}", marker, pt.label()),
                        style,
                    )));
                }
                // 1 header + 4 types = 5 lines, fits in the 6-line action area
                lines
            }
            Mode::EditName => {
                let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
                spans.extend(self.input.visible_spans(area.width, 3, GREEN));
                vec![
                    Line::from(Span::styled(
                        tr("practices-rename"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                    Line::from(""),
                    Line::from(""),
                ]
            }
            Mode::ConfirmDelete => {
                let name = self
                    .practices
                    .get(self.selected)
                    .map(|p| p.name.as_str())
                    .unwrap_or("?");
                vec![
                    Line::from(Span::styled(
                        tr_args(
                            "practices-delete-confirm",
                            &[("name", FluentValue::from(name.to_string()))],
                        ),
                        Style::default().fg(RED),
                    )),
                    Line::from(Span::styled(
                        tr("practices-delete-cascade-warning"),
                        Style::default().fg(RED),
                    )),
                    Line::from(""),
                    Line::from(""),
                ]
            }
        };
        frame.render_widget(
            Paragraph::new(action_lines).wrap(Wrap { trim: false }),
            chunks[2],
        );

        if self.mode == Mode::AddType {
            highlight_row(frame, chunks[2], (self.type_selected + 1) as u16);
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
            Mode::AddName => self.handle_add_name(key),
            Mode::AddType => self.handle_add_type(key, db),
            Mode::EditName => self.handle_edit_name(key, db),
            Mode::ConfirmDelete => self.handle_confirm_delete(key, db),
        }
    }

    fn handle_browse(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.practices.is_empty() {
                    self.selected =
                        move_selection(self.selected, self.practices.len(), 1, EdgeBehavior::Clamp);
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.practices.is_empty() {
                    self.selected = move_selection(
                        self.selected,
                        self.practices.len(),
                        -1,
                        EdgeBehavior::Clamp,
                    );
                }
                Action::None
            }
            KeyCode::Char('a') => {
                self.input.clear();
                self.type_selected = 0;
                self.mode = Mode::AddName;
                Action::None
            }
            KeyCode::Char('e') => {
                if let Some(p) = self.practices.get(self.selected) {
                    self.input.set(p.name.clone());
                    self.mode = Mode::EditName;
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(p) = self.practices.get(self.selected) {
                    match db.set_practice_active(p.id, !p.active) {
                        Ok(()) => {
                            if let Err(error) = self.refresh(db) {
                                self.status_msg = Some((format!("Error: {error}"), true));
                            }
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Error: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('d') => {
                if !self.practices.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard)),
            _ => Action::None,
        }
    }

    fn handle_add_name(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty_trimmed() {
                    self.mode = Mode::AddType;
                } else {
                    self.status_msg = Some((tr("practices-empty-name-error"), true));
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => {
                self.input.handle_key(key);
                Action::None
            }
        }
    }

    fn handle_add_type(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.type_selected = (self.type_selected + 1) % PracticeType::ALL.len();
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.type_selected = self
                    .type_selected
                    .checked_sub(1)
                    .unwrap_or(PracticeType::ALL.len() - 1);
                Action::None
            }
            KeyCode::Enter => {
                let pt = PracticeType::ALL[self.type_selected];
                match db.create_practice(self.input.trimmed(), pt) {
                    Ok(_) => match self.refresh(db) {
                        Ok(()) => {
                            self.input.clear();
                            self.mode = Mode::Browse;
                        }
                        Err(error) => {
                            self.status_msg = Some((format!("Error: {error}"), true));
                        }
                    },
                    Err(e) => {
                        self.status_msg = Some((format!("Error: {}", e), true));
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_edit_name(&mut self, key: KeyEvent, db: &Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(p) = self.practices.get(self.selected) {
                    let trimmed = self.input.trimmed();
                    if !trimmed.is_empty() {
                        match db.rename_practice(p.id, trimmed) {
                            Ok(()) => match self.refresh(db) {
                                Ok(()) => {
                                    self.input.clear();
                                    self.mode = Mode::Browse;
                                }
                                Err(error) => {
                                    self.status_msg = Some((format!("Error: {error}"), true));
                                }
                            },
                            Err(e) => {
                                self.status_msg = Some((format!("Error: {}", e), true));
                            }
                        }
                    } else {
                        self.status_msg = Some((tr("practices-empty-name-error"), true));
                    }
                }
                Action::None
            }
            KeyCode::Esc => {
                self.input.clear();
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
        match key.code {
            KeyCode::Char('y') => {
                if let Some(p) = self.practices.get(self.selected) {
                    match db.delete_practice(p.id) {
                        Ok(()) => match self.refresh(db) {
                            Ok(()) => {
                                self.mode = Mode::Browse;
                            }
                            Err(error) => {
                                self.status_msg = Some((format!("Error: {error}"), true));
                            }
                        },
                        Err(e) => {
                            self.status_msg = Some((format!("Delete failed: {}", e), true));
                        }
                    }
                }
                Action::None
            }
            _ => {
                self.mode = Mode::Browse;
                Action::None
            }
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn ids_for_test(&self) -> Vec<i64> {
        self.practices.iter().map(|practice| practice.id).collect()
    }
}

impl Reenter for PracticesScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        let selected_id = self
            .practices
            .get(self.selected)
            .map(|practice| practice.id);
        let mut next = Self::new(db)?;

        next.selected = selected_id
            .and_then(|id| next.practices.iter().position(|practice| practice.id == id))
            .unwrap_or_else(|| self.selected.min(next.practices.len().saturating_sub(1)));
        if next.practices.is_empty() {
            next.selected = 0;
        }

        next.list_viewport = self.list_viewport;
        next.list_viewport.clamp(next.practices.len());
        next.mode = Mode::Browse;
        next.input.clear();
        next.status_msg = None;

        Ok(next)
    }
}

impl NavigationStatus for PracticesScreen {
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

    fn render_practices(screen: &mut PracticesScreen, width: u16, height: u16) -> String {
        render_to_string(width, height, |frame| screen.render(frame))
    }

    fn practices_db(count: usize) -> Database {
        let db = Database::open_in_memory().unwrap();
        for number in 1..=count {
            db.create_practice(&format!("Practice {number:02}"), PracticeType::Weighted)
                .unwrap();
        }
        db
    }

    fn golden_screen(mode: Mode) -> PracticesScreen {
        std::env::set_var("LC_ALL", "C");
        crate::i18n::init();

        let db = Database::open_in_memory().unwrap();
        db.create_practice("Barbell Squat", PracticeType::Weighted)
            .unwrap();
        let selected = db
            .create_practice("Pull-ups", PracticeType::Bodyweight)
            .unwrap();
        db.create_practice(
            "Zone 2 endurance with a deliberately long practice name",
            PracticeType::Endurance,
        )
        .unwrap();
        db.set_practice_active(selected.id, false).unwrap();

        let mut screen = PracticesScreen::new(&db).unwrap();
        screen.selected = screen
            .practices
            .iter()
            .position(|practice| practice.id == selected.id)
            .unwrap();
        screen.mode = mode.clone();
        screen.status_msg = Some(("Fixture status".into(), false));
        match mode {
            Mode::AddName => screen.input.set("新 practice"),
            Mode::AddType => {
                screen.input.set("Interval Run");
                screen.type_selected = 2;
            }
            Mode::EditName => screen.input.set("Pull-ups revised"),
            Mode::Browse | Mode::ConfirmDelete => {}
        }
        screen
    }

    fn practices_golden(name: &str) -> &'static str {
        match name {
            "browse" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/practices/browse.txt"
            )),
            "add_name" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/practices/add_name.txt"
            )),
            "add_type" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/practices/add_type.txt"
            )),
            "edit_name" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/practices/edit_name.txt"
            )),
            "confirm_delete" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tui/practices/confirm_delete.txt"
            )),
            unexpected => panic!("unknown Practices golden {unexpected}"),
        }
    }

    #[test]
    fn practices_modes_match_golden_buffers() {
        for (name, mode) in [
            ("browse", Mode::Browse),
            ("add_name", Mode::AddName),
            ("add_type", Mode::AddType),
            ("edit_name", Mode::EditName),
            ("confirm_delete", Mode::ConfirmDelete),
        ] {
            let mut screen = golden_screen(mode);
            let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
            assert_eq!(
                actual,
                practices_golden(name),
                "Practices {name} buffer changed"
            );
        }
    }

    #[test]
    fn practices_reentry_preserves_selection_and_clears_edit_state() {
        let db = Database::open_in_memory().unwrap();
        let first = db
            .create_practice("First", PracticeType::Bodyweight)
            .unwrap();
        let second = db
            .create_practice("Second", PracticeType::Weighted)
            .unwrap();
        let mut screen = PracticesScreen::new(&db).unwrap();
        screen.selected = screen
            .practices
            .iter()
            .position(|p| p.id == second.id)
            .unwrap();
        screen.mode = Mode::EditName;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(1);
        screen.input.set("discard me");
        screen.status_msg = Some(("old".into(), true));
        db.rename_practice(first.id, "First changed").unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.practices[next.selected].id, second.id);
        assert_eq!(next.list_viewport.offset(), 1);
        assert_eq!(next.mode, Mode::Browse);
        assert!(next.input.value().is_empty());
        assert!(next.status_msg.is_none());
        assert_eq!(
            next.practices
                .iter()
                .find(|p| p.id == first.id)
                .unwrap()
                .name,
            "First changed"
        );
    }

    #[test]
    fn practices_reentry_clears_every_transient_mode_and_field() {
        let db = Database::open_in_memory().unwrap();
        let selected = db
            .create_practice("Selected", PracticeType::Bodyweight)
            .unwrap();

        for mode in [
            Mode::AddName,
            Mode::AddType,
            Mode::EditName,
            Mode::ConfirmDelete,
        ] {
            let mut screen = PracticesScreen::new(&db).unwrap();
            screen.mode = mode.clone();
            screen.input.set("discard me");
            screen.type_selected = PracticeType::ALL.len() - 1;
            screen.status_msg = Some(("old status".into(), true));

            let next = screen.reenter(&db).unwrap();

            assert_eq!(next.practices[next.selected].id, selected.id, "{mode:?}");
            assert_eq!(next.mode, Mode::Browse, "{mode:?}");
            assert!(next.input.value().is_empty(), "{mode:?}");
            assert_eq!(next.type_selected, 0, "{mode:?}");
            assert!(next.status_msg.is_none(), "{mode:?}");
        }
    }

    #[test]
    fn practices_reentry_clamps_selection_and_viewport_after_selected_deletion() {
        let db = Database::open_in_memory().unwrap();
        db.create_practice("First", PracticeType::Bodyweight)
            .unwrap();
        let second = db
            .create_practice("Second", PracticeType::Weighted)
            .unwrap();
        let third = db
            .create_practice("Third", PracticeType::Endurance)
            .unwrap();
        let mut screen = PracticesScreen::new(&db).unwrap();
        screen.selected = 2;
        screen.list_viewport.set_height(1);
        screen.list_viewport.set_offset(2);
        db.delete_practice(third.id).unwrap();

        let next = screen.reenter(&db).unwrap();

        assert_eq!(next.selected, 1);
        assert_eq!(next.practices[next.selected].id, second.id);
        assert_eq!(next.list_viewport.offset(), 1);
    }

    #[test]
    fn practices_navigation_error_uses_status_line() {
        let db = Database::open_in_memory().unwrap();
        let mut screen = PracticesScreen::new(&db).unwrap();

        screen.set_navigation_error("navigation failed".into());

        assert_eq!(screen.status_msg, Some(("navigation failed".into(), true)));
    }

    #[test]
    fn duplicate_rename_retains_edit_mode_and_input() {
        let db = Database::open_in_memory().unwrap();
        db.create_practice("Existing", PracticeType::Weighted)
            .unwrap();
        db.create_practice("Target", PracticeType::Bodyweight)
            .unwrap();
        let mut screen = PracticesScreen::new(&db).unwrap();
        screen.selected = screen
            .practices
            .iter()
            .position(|practice| practice.name == "Target")
            .unwrap();
        screen.handle_key(key(KeyCode::Char('e')), &db);
        screen.input.set("Existing");

        screen.handle_key(key(KeyCode::Enter), &db);

        assert_eq!(screen.mode, Mode::EditName);
        assert_eq!(screen.input.value(), "Existing");
        assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
    }

    #[test]
    fn practices_last_item_remains_visible_at_minimum_size() {
        let db = practices_db(30);
        let mut screen = PracticesScreen::new(&db).unwrap();
        for _ in 1..30 {
            screen.handle_key(key(KeyCode::Down), &db);
        }

        let output = render_practices(&mut screen, 80, 24);

        assert!(output.contains("Practice 30"), "{output}");
    }

    #[test]
    fn practices_reserve_wrapped_browse_shortcuts() {
        let db = practices_db(1);
        let mut screen = PracticesScreen::new(&db).unwrap();

        let output = render_practices(&mut screen, 80, 24);

        assert!(output.contains("[d] Delete"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }
}
