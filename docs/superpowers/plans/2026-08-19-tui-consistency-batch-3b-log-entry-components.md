# TUI Consistency Batch 3B-7 Log Entry Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Log Entry practice selection and form-panel presentation to shared TUI components while preserving the accepted `80x24` styled output, request-scoped state machine, persistence, navigation, and failure-retention behavior.

**Architecture:** A new presentation-only `BoundedPanel` owns clipped border/title/padding/safe-inner geometry; the existing `StaticPanel` becomes its unchanged Dashboard-style wrapper. Log Entry uses one `ScreenFrameSections` per phase, one `ScrollableListPanel` for practice selection, and four `BoundedPanel` calls for the entry form while retaining every domain line, input, mode, validation, database operation, and navigation action.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Chrono 0.4, Rusqlite 0.32, Fluent localization, Unicode Width, Syn 2 ownership linting, existing TUI framework/test support, Cargo tests, Clippy, and styled `TestBackend` buffer fixtures.

**Spec:** `docs/superpowers/specs/2026-08-19-tui-consistency-batch-3b-log-entry-components-design.md`

## Global Constraints

- This is an exact presentation-ownership migration, not a Log Entry redesign.
- Preserve both phases, all field order, title text/glyphs, colors, padding, content order, focus order, navigation, shortcuts, validation, database writes, edit identity, return target, and failure-retention behavior.
- At `80x24`, all eight Log Entry styled buffers must be identical before and after migration.
- Smaller rectangles are geometry/semantic tests, not golden layouts; `0x0`, `1x1`, `24x10`, and resize sequences must remain bounded and panic-free.
- `BoundedPanel` owns no domain, screen, input, viewport, status, shortcut, database, or navigation state.
- `StaticPanel` keeps its public signature, decorated title, uniform padding, `BORDER_COLOR`, returned geometry, and Dashboard output.
- Existing fixtures under `tests/fixtures/tui/{dashboard,goals,practices,quotes,trends,history}` are immutable.
- Add no dependency, form schema, form engine, layout DSL, multi-line input, or domain-aware framework component.
- Do not modify application routing, screen storage, lifecycle, database, schema, model, locale keys, or other screen production code.
- Quick Log, LLM configuration/runtime, Abbreviations APIs, and the `abbreviations` table remain permanently removed.
- Stop for user direction if an accepted fixture changes, another screen requires a production edit, an existing public component contract must break, or the pre-migration Log Entry fixtures cannot remain exact.
- Use TDD for every production change: observe a focused RED, make the smallest GREEN change, then refactor.
- Execute this plan in an isolated worktree created through `superpowers:using-git-worktrees`.

## File Map

- Create `src/tui/framework/bounded_panel.rs` — generic border/title/padding/safe-inner presentation only.
- Create `tests/fixtures/tui/log_entry/{select_browse,select_filter,weighted_form,set_edit,date_edit,note_focus,confirm_exit,existing_edit}.txt` — accepted `80x24` styled buffers.
- Modify `src/tui/framework/static_panel.rs` — delegate unchanged StaticPanel behavior to BoundedPanel.
- Modify `src/tui/framework/mod.rs` — declare BoundedPanel in Task 2 and re-export its consumed API in Task 4.
- Modify `src/tui/log_entry.rs` — fixture helpers, selector migration, entry-form migration, and real-frame geometry tests.
- Modify `tests/tui_lint_test.rs` — selector/full ownership guards and only proven identical-widget Paragraph allowlist shifts.
- Modify `CLAUDE.md`, `README.md`, `README_zh.md`, `docs/specs/log-entry-spec.md`, `docs/specs/navigation-spec.md`, and `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — shipped ownership and unchanged behavior.

---

### Task 1: Freeze Eight Accepted Log Entry Styled Buffers

**Files:**
- Modify: `src/tui/log_entry.rs`
- Modify if and only if existing Log Entry Paragraph sites move: `tests/tui_lint_test.rs`
- Create: `tests/fixtures/tui/log_entry/select_browse.txt`
- Create: `tests/fixtures/tui/log_entry/select_filter.txt`
- Create: `tests/fixtures/tui/log_entry/weighted_form.txt`
- Create: `tests/fixtures/tui/log_entry/set_edit.txt`
- Create: `tests/fixtures/tui/log_entry/date_edit.txt`
- Create: `tests/fixtures/tui/log_entry/note_focus.txt`
- Create: `tests/fixtures/tui/log_entry/confirm_exit.txt`
- Create: `tests/fixtures/tui/log_entry/existing_edit.txt`

**Interfaces:**
- Consumes: current private `LogEntryScreen` state and `framework::test_support::render_to_snapshot`.
- Produces: `golden_log_entry_screen(name)`, compile-time fixture loading, `log_entry_states_match_golden_buffers`, eight immutable `80x24` fixtures, and recorded `LOG_ENTRY_FIXTURE_BASE`.

- [ ] **Step 1: Add deterministic model and screen builders**

In the existing `#[cfg(test)]` module, import `chrono::NaiveDate`, `crate::model::Practice`, and `render_to_snapshot`. Add:

```rust
fn log_entry_fixture_time() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 19)
        .unwrap()
        .and_hms_opt(6, 30, 0)
        .unwrap()
}

fn fixture_practice(id: i64, name: &str, practice_type: PracticeType) -> Practice {
    Practice {
        id,
        name: name.to_owned(),
        practice_type,
        created_at: log_entry_fixture_time(),
        active: true,
    }
}

fn base_log_entry_fixture() -> LogEntryScreen {
    std::env::set_var("LC_ALL", "C");
    crate::i18n::init();
    let practices = vec![
        fixture_practice(101, "Barbell Back Squat", PracticeType::Weighted),
        fixture_practice(102, "Strict Pull-up", PracticeType::Bodyweight),
        fixture_practice(103, "Road Running", PracticeType::Distance),
        fixture_practice(104, "Row Ergometer", PracticeType::Endurance),
    ];
    LogEntryScreen {
        filtered_indices: (0..practices.len()).collect(),
        practices,
        filter: TextInput::new(),
        filtering: false,
        selected: 2,
        list_viewport: ViewportState::new(0),
        phase: Phase::SelectPractice,
        chosen_practice: None,
        sets: Vec::new(),
        field1: TextInput::new(),
        field2: TextInput::new(),
        active_field: 0,
        note: TextInput::new(),
        warm_up: TextInput::new(),
        cool_down: TextInput::new(),
        rpe_input: TextInput::new(),
        focus: FocusSection::Sets,
        selected_set: None,
        editing_set: None,
        editing_log_id: None,
        log_date: "2026-08-19".into(),
        editing_date: false,
        date_input: TextInput::new(),
        confirming_exit: false,
        return_to: ModuleId::Dashboard,
        status_msg: Some(("Log Entry fixture status".into(), false)),
    }
}

fn configure_weighted_form(screen: &mut LogEntryScreen) {
    screen.phase = Phase::EnterLog;
    screen.chosen_practice = Some(screen.practices[0].clone());
    screen.sets = vec![
        SetData::Weighted { weight: 120.0, reps: 5 },
        SetData::Weighted { weight: 125.0, reps: 3 },
    ];
    screen.field1 = TextInput::from("127.5");
    screen.field2 = TextInput::from("2");
    screen.warm_up = TextInput::from("Empty bar, 60kg, 90kg");
    screen.cool_down = TextInput::from("Five minutes easy cycle");
    screen.rpe_input = TextInput::from("8");
    screen.note = TextInput::from("Controlled fixture session");
    screen.focus = FocusSection::Sets;
    screen.selected_set = Some(1);
}

fn golden_log_entry_screen(name: &str) -> LogEntryScreen {
    let mut screen = base_log_entry_fixture();
    match name {
        "select_browse" => {
            screen.list_viewport.set_offset(1);
        }
        "select_filter" => {
            screen.filtering = true;
            screen.filter = TextInput::from("Road");
            screen.filtered_indices = vec![2];
            screen.selected = 0;
            screen.status_msg = Some(("Filter fixture status".into(), false));
        }
        "weighted_form" => configure_weighted_form(&mut screen),
        "set_edit" => {
            configure_weighted_form(&mut screen);
            screen.editing_set = Some(1);
            screen.selected_set = Some(1);
            screen.field1 = TextInput::from("130");
            screen.field2 = TextInput::from("4");
            screen.active_field = 1;
        }
        "date_edit" => {
            configure_weighted_form(&mut screen);
            screen.editing_date = true;
            screen.date_input = TextInput::from("2026-08-20");
        }
        "note_focus" => {
            configure_weighted_form(&mut screen);
            screen.focus = FocusSection::Note;
            screen.note = TextInput::from("长期训练记录 mixed with a deliberately long suffix ".repeat(4));
        }
        "confirm_exit" => {
            configure_weighted_form(&mut screen);
            screen.confirming_exit = true;
            screen.status_msg = None;
        }
        "existing_edit" => {
            configure_weighted_form(&mut screen);
            screen.editing_log_id = Some(701);
            screen.return_to = ModuleId::History;
            screen.focus = FocusSection::WarmUp;
            screen.status_msg = Some(("Existing log fixture".into(), false));
        }
        other => panic!("unknown Log Entry golden state: {other}"),
    }
    screen
}
```

- [ ] **Step 2: Add compile-time fixture loading and observe empty-fixture RED**

Create the eight empty files and add:

```rust
fn log_entry_golden(name: &str) -> &'static str {
    match name {
        "select_browse" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/select_browse.txt")),
        "select_filter" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/select_filter.txt")),
        "weighted_form" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/weighted_form.txt")),
        "set_edit" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/set_edit.txt")),
        "date_edit" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/date_edit.txt")),
        "note_focus" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/note_focus.txt")),
        "confirm_exit" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/confirm_exit.txt")),
        "existing_edit" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/log_entry/existing_edit.txt")),
        other => panic!("unknown Log Entry golden fixture: {other}"),
    }
}

const LOG_ENTRY_GOLDEN_STATES: [&str; 8] = [
    "select_browse",
    "select_filter",
    "weighted_form",
    "set_edit",
    "date_edit",
    "note_focus",
    "confirm_exit",
    "existing_edit",
];

#[test]
fn log_entry_states_match_golden_buffers() {
    for name in LOG_ENTRY_GOLDEN_STATES {
        let mut screen = golden_log_entry_screen(name);
        let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
        assert_eq!(actual, log_entry_golden(name), "Log Entry {name} changed");
    }
}
```

Run:

```bash
cargo test tui::log_entry::tests::log_entry_states_match_golden_buffers -- --exact --nocapture
```

Expected: the test compiles and fails because every expected fixture is empty.

- [ ] **Step 3: Capture once and remove the writer**

Temporarily add:

```rust
#[test]
#[ignore]
fn capture_log_entry_golden_buffers_once() {
    for name in LOG_ENTRY_GOLDEN_STATES {
        let mut screen = golden_log_entry_screen(name);
        let snapshot = render_to_snapshot(80, 24, |frame| screen.render(frame));
        std::fs::write(
            format!("tests/fixtures/tui/log_entry/{name}.txt"),
            snapshot,
        )
        .unwrap();
    }
}
```

Run:

```bash
cargo test tui::log_entry::tests::capture_log_entry_golden_buffers_once -- --ignored --exact --nocapture
```

Expected: PASS and each fixture has `1921` lines: one area header plus `80 * 24` cells. Remove the entire temporary test immediately. Retain no environment-controlled or ignored updater.

- [ ] **Step 4: Verify characterization and commit**

Run:

```bash
cargo test tui::log_entry::tests::log_entry_states_match_golden_buffers -- --exact --nocapture
cargo test tui::log_entry::tests -- --nocapture
rg -n "capture_log_entry_golden_buffers_once|UPDATE_GOLDEN|std::fs::write" src/tui/log_entry.rs
wc -l tests/fixtures/tui/log_entry/*.txt
cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
git diff --check
```

Expected: both Log Entry targets pass; the updater scan exits `1`; every fixture has `1921` lines; full tests pass. If fixture-helper insertion shifts an existing Log Entry Paragraph allowlist site, map the old and new line to the identical widget and update only that literal. Stop on any semantic lint failure.

Commit:

```bash
git add src/tui/log_entry.rs tests/fixtures/tui/log_entry tests/tui_lint_test.rs
git commit -m "test(log-entry): freeze accepted presentation"
```

Record the commit as `LOG_ENTRY_FIXTURE_BASE`. Every later task runs:

```bash
git diff --exit-code "$LOG_ENTRY_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/log_entry
```

---

### Task 2: Add BoundedPanel and Preserve StaticPanel

**Files:**
- Create: `src/tui/framework/bounded_panel.rs`
- Modify: `src/tui/framework/static_panel.rs`
- Modify: `src/tui/framework/mod.rs`

**Interfaces:**
- Consumes: `fit_rect`, Ratatui `Frame`, `Rect`, `Style`, `Line`, `Block`, `Borders`, and `Padding`.
- Produces:

```rust
pub struct BoundedPanelSpec<'a> {
    pub title: Option<Line<'a>>,
    pub border_style: Style,
    pub padding: Padding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_bounded_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    spec: BoundedPanelSpec<'a>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> BoundedPanelAreas;
```

`static_panel.rs` imports this API through `super::bounded_panel`; the general re-export is intentionally delayed until Task 4 consumes it, so Task 2 needs no temporary dead-code or unused-import suppression.

- [ ] **Step 1: Write failing BoundedPanel tests**

Create `bounded_panel.rs` with the API calls inside a test module. Add:

```rust
#[test]
fn bounded_panel_preserves_title_style_padding_border_and_content() {
    let mut returned = None;
    let buffer = render_to_buffer(20, 7, |frame| {
        returned = Some(render_bounded_panel(
            frame,
            frame.area(),
            BoundedPanelSpec {
                title: Some(Line::from(Span::styled(
                    " Exact title ",
                    Style::default().fg(Color::Cyan).bold(),
                ))),
                border_style: Style::default().fg(Color::Yellow),
                padding: Padding::uniform(1),
            },
            |frame, inner| frame.render_widget(Paragraph::new("Body"), inner),
        ));
    });
    assert_eq!(returned.unwrap(), BoundedPanelAreas {
        area: Rect::new(0, 0, 20, 7),
        inner: Rect::new(2, 2, 16, 3),
    });
    assert_eq!(buffer[(0, 3)].fg, Color::Yellow);
    assert_eq!(buffer[(1, 0)].symbol(), " ");
    assert_eq!(buffer[(2, 0)].symbol(), "E");
    assert_eq!(buffer[(2, 0)].fg, Color::Cyan);
    assert_eq!(buffer[(2, 2)].symbol(), "B");
}

#[test]
fn bounded_panel_supports_no_title_and_horizontal_padding() {
    let mut closure_inner = None;
    let returned = render_to_buffer(12, 5, |frame| {
        let areas = render_bounded_panel(
            frame,
            frame.area(),
            BoundedPanelSpec {
                title: None,
                border_style: Style::default().fg(Color::DarkGray),
                padding: Padding::horizontal(1),
            },
            |_, inner| closure_inner = Some(inner),
        );
        assert_eq!(areas.inner, Rect::new(2, 1, 8, 3));
    });
    assert_eq!(closure_inner, Some(Rect::new(2, 1, 8, 3)));
    assert_eq!(returned[(0, 0)].symbol(), "┌");
}

#[test]
fn bounded_panel_clips_zero_tiny_edge_and_maximum_rectangles() {
    let frame_area = Rect::new(0, 0, 6, 6);
    for requested in [
        Rect::new(0, 0, 0, 0),
        Rect::new(5, 5, 1, 1),
        Rect::new(5, 5, 4, 4),
        Rect::new(u16::MAX, u16::MAX, 0, 0),
    ] {
        let mut closure_inner = None;
        let mut returned = None;
        render_to_buffer(6, 6, |frame| {
            returned = Some(render_bounded_panel(
                frame,
                requested,
                BoundedPanelSpec {
                    title: None,
                    border_style: Style::default(),
                    padding: Padding::uniform(1),
                },
                |_, inner| closure_inner = Some(inner),
            ));
        });
        let areas = returned.unwrap();
        assert_eq!(closure_inner, Some(areas.inner));
        assert!(rect_is_within(frame_area, areas.area), "{requested:?} -> {areas:?}");
        assert!(rect_is_within(areas.area, areas.inner), "{requested:?} -> {areas:?}");
    }
}
```

Use the existing `rect_is_within` expression from other framework geometry tests.

- [ ] **Step 2: Run focused RED**

Declare `pub mod bounded_panel;` in `framework/mod.rs`, then run:

```bash
cargo test tui::framework::bounded_panel::tests -- --nocapture
```

Expected: compilation fails because `BoundedPanelSpec`, `BoundedPanelAreas`, and `render_bounded_panel` are missing.

- [ ] **Step 3: Implement the minimal BoundedPanel**

Add:

```rust
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Padding},
    Frame,
};

use super::fit_rect;

pub struct BoundedPanelSpec<'a> {
    pub title: Option<Line<'a>>,
    pub border_style: Style,
    pub padding: Padding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_bounded_panel<'a>(
    frame: &mut Frame,
    requested: Rect,
    spec: BoundedPanelSpec<'a>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> BoundedPanelAreas {
    let area = fit_rect(frame.area(), requested);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .padding(spec.padding)
        .border_style(spec.border_style);
    if let Some(title) = spec.title {
        block = block.title(title);
    }
    let inner = fit_rect(area, block.inner(area));
    frame.render_widget(block, area);
    render_content(frame, inner);
    BoundedPanelAreas { area, inner }
}
```

- [ ] **Step 4: Make StaticPanel a compatibility wrapper**

In `static_panel.rs`, remove its private geometry/block builders and import `render_bounded_panel`, `BoundedPanelAreas`, and `BoundedPanelSpec`. Preserve the public signature with:

```rust
pub type StaticPanelAreas = BoundedPanelAreas;

pub fn render_static_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> StaticPanelAreas {
    render_bounded_panel(
        frame,
        area,
        BoundedPanelSpec {
            title: Some(Line::from(vec![
                Span::styled("── ", Style::default().fg(BORDER_COLOR)),
                Span::styled(
                    title.into(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
            ])),
            border_style: Style::default().fg(BORDER_COLOR),
            padding: Padding::uniform(1),
        },
        render_content,
    )
}
```

Do not re-export BoundedPanel yet. Do not add any lint suppression.

- [ ] **Step 5: Run component, compatibility, and fixture GREEN**

Run:

```bash
cargo test tui::framework::bounded_panel::tests -- --nocapture
cargo test tui::framework::static_panel::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::dashboard::tests::dashboard_states_match_golden_buffers -- --exact --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
git diff --check
git diff --exit-code "$LOG_ENTRY_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/log_entry
git diff --exit-code 6b70eaf613a534db3828649e2115711bf4ea70d7..HEAD -- tests/fixtures/tui/dashboard
```

Expected: every command exits `0`; StaticPanel and all Dashboard output remain exact; no warning suppression is required.

Commit:

```bash
git add src/tui/framework/bounded_panel.rs src/tui/framework/static_panel.rs src/tui/framework/mod.rs
git commit -m "feat(tui): add bounded panel component"
```

---

### Task 3: Migrate the Practice Selector

**Files:**
- Modify: `src/tui/log_entry.rs`
- Modify: `tests/tui_lint_test.rs`

**Interfaces:**
- Consumes: `render_screen_frame_sections`, `render_scrollable_list_panel`, `ListPanelContent`, existing `ViewportState`, and immutable Log Entry fixtures.
- Produces: selector-only ownership enforcement, one shared frame/list call in `render_select_practice`, and unchanged selector output/behavior.

- [ ] **Step 1: Add selector ownership helpers and RED test**

Add test helpers beside existing Dashboard ownership helpers:

```rust
fn method_region(source: &str, start: &str, end: &str) -> String {
    let start_index = source.find(start).unwrap_or_else(|| panic!("missing {start}"));
    let tail = &source[start_index..];
    let end_index = tail.find(end).unwrap_or_else(|| panic!("missing {end}"));
    source_code(&tail[..end_index])
}

fn log_entry_selector_ownership_violations(source: &str) -> Vec<String> {
    let selector = method_region(
        source,
        "fn render_select_practice",
        "fn handle_select_practice",
    );
    let mut violations = Vec::new();
    for (needle, expected, label) in [
        ("render_screen_frame_sections(", 1, "ScreenFrameSections"),
        ("render_scrollable_list_panel(", 1, "ScrollableListPanel"),
    ] {
        let actual = selector.match_indices(needle).count();
        if actual != expected {
            violations.push(format!(
                "Log Entry selector must call {label} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    for forbidden in [
        ".borders(Borders::ALL)",
        ".inner(",
        "render_status_line(",
        "render_shortcuts(",
        "highlight_row(",
        ".skip(",
        ".take(",
    ] {
        if selector.contains(forbidden) {
            violations.push(format!("Log Entry selector still owns {forbidden}"));
        }
    }
    violations
}

#[test]
fn log_entry_selector_delegates_shared_presentation() {
    let source = std::fs::read_to_string("src/tui/log_entry.rs").unwrap();
    let violations = log_entry_selector_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn log_entry_selector_ownership_ignores_comments_strings_and_tests() {
    let source = r###"
        fn render_select_practice(&mut self) {
            // render_screen_frame_sections(); render_scrollable_list_panel();
            let _text = r#"render_scrollable_list_panel("#;
        }
        fn handle_select_practice(&mut self) {}
        #[cfg(test)]
        fn decoy() {
            render_screen_frame_sections();
            render_scrollable_list_panel();
        }
    "###;
    let violations = log_entry_selector_ownership_violations(source);
    assert!(violations.iter().any(|item| item.contains("ScreenFrameSections")));
    assert!(violations.iter().any(|item| item.contains("ScrollableListPanel")));
}
```

Run:

```bash
cargo test --test tui_lint_test log_entry_selector_delegates_shared_presentation -- --exact --nocapture
cargo test --test tui_lint_test log_entry_selector_ownership_ignores_comments_strings_and_tests -- --exact --nocapture
```

Expected: production ownership fails with zero shared calls and local ownership; the decoy control passes.

- [ ] **Step 2: Replace local selector framing**

Replace `centered_area`, the two local layouts, and direct status/footer rendering with:

```rust
let shortcuts = self.active_select_shortcuts();
let sections = render_screen_frame_sections(
    frame,
    frame.area(),
    CONTENT_WIDTH,
    &self.status_msg,
    &shortcuts,
    &[Constraint::Length(1), Constraint::Min(0)],
    false,
);
let filter_area = sections.body[0];
let list_area = sections.body[1];
```

Render the filter exactly as before in `filter_area`. Remove the local `shortcut_height`, `render_status_line`, and `render_shortcuts` calls from this method.

- [ ] **Step 3: Replace local list geometry/slicing/highlight**

Keep disjoint borrows before the component call:

```rust
let practices = &self.practices;
let filtered_indices = &self.filtered_indices;
let selected = self.selected;
render_scrollable_list_panel(
    frame,
    list_area,
    tr("log-select-practice"),
    &mut self.list_viewport,
    |inner| {
        let name_header = tr("practices-col-name");
        let type_header = tr("practices-col-type");
        let desired_type_width = practices
            .iter()
            .map(|practice| practice.practice_type.label().width())
            .max()
            .unwrap_or(0)
            .max(type_header.width());
        let type_col_width = desired_type_width.min((inner.width as usize) / 3);
        let name_col_width = (inner.width as usize)
            .saturating_sub(2)
            .saturating_sub(2)
            .saturating_sub(type_col_width);
        let name_header = truncate_to_width(&name_header, name_col_width);
        let type_header = truncate_to_width(&type_header, type_col_width);
        let header_padding = name_col_width
            .saturating_sub(name_header.width())
            .saturating_add(2);
        let header = Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(name_header, Style::default().fg(Color::DarkGray)),
            Span::raw(" ".repeat(header_padding)),
            Span::styled(type_header, Style::default().fg(Color::DarkGray)),
        ]);
        let rows = filtered_indices
            .iter()
            .enumerate()
            .map(|(row_index, index)| {
                let practice = &practices[*index];
                let marker = if row_index == selected { "> " } else { "  " };
                let name_style = if row_index == selected {
                    Style::default().fg(Color::White).bold()
                } else {
                    Style::default().fg(Color::White)
                };
                let name = truncate_to_width(&practice.name, name_col_width);
                let padding = name_col_width
                    .saturating_sub(name.width())
                    .saturating_add(2);
                Line::from(vec![
                    Span::styled(marker, name_style),
                    Span::styled(name, name_style),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(
                        truncate_to_width(&practice.practice_type.label(), type_col_width),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            })
            .collect();
        ListPanelContent {
            header: Some(header),
            rows,
            empty: None,
            selected: (!filtered_indices.is_empty()).then_some(selected),
        }
    },
);
```

The caller retains the accepted `> ` marker and bold selected text; ListPanel additionally supplies the existing selected-row background. This preserves both textual and cell-style output.

- [ ] **Step 4: Run selector GREEN and geometry regression**

Run:

```bash
cargo test tui::log_entry::tests::log_entry_states_match_golden_buffers -- --exact --nocapture
cargo test tui::log_entry::tests::long_filter_and_last_practice_remain_visible -- --exact --nocapture
cargo test --test tui_lint_test log_entry_selector_delegates_shared_presentation -- --exact --nocapture
cargo test --test tui_lint_test log_entry_selector_ownership_ignores_comments_strings_and_tests -- --exact --nocapture
cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture
```

Expected: fixtures and selector behavior pass in both targets; ownership and Paragraph lint pass. Update only proven identical-widget Log Entry allowlist line numbers.

- [ ] **Step 5: Run task gates and commit**

Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::log_entry::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --check
git diff --exit-code "$LOG_ENTRY_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/log_entry
```

Expected: every gate exits `0`; no fixture changes.

Commit:

```bash
git add src/tui/log_entry.rs tests/tui_lint_test.rs
git commit -m "refactor(log-entry): migrate practice selector presentation"
```

---

### Task 4: Migrate the Entry Form and Enforce Full Ownership

**Files:**
- Modify: `src/tui/framework/mod.rs`
- Modify: `src/tui/log_entry.rs`
- Modify: `tests/tui_lint_test.rs`

**Interfaces:**
- Consumes: `render_screen_frame_sections`, `render_bounded_panel`, `BoundedPanelSpec`, the existing form state machine, and immutable fixtures.
- Produces: two total shared-frame calls, one list-panel call, four bounded-panel calls, no local bordered-panel/status/footer ownership, robust executable ownership tests, and unchanged form behavior.

- [ ] **Step 1: Add the consumed BoundedPanel re-export**

In `framework/mod.rs`, re-export only the names consumed by Log Entry:

```rust
pub use bounded_panel::{render_bounded_panel, BoundedPanelSpec};
```

`BoundedPanelAreas` remains available as `framework::bounded_panel::BoundedPanelAreas`. Do not add `allow(dead_code)` or any unused-import allowance.

- [ ] **Step 2: Add full ownership RED with real Rust decoys**

Add a Syn-based `log_entry_ownership_violations` rather than extending the accepted Goals analyzer. It must:

1. parse the file;
2. find the single production inherent `impl LogEntryScreen`;
3. start at `render` and visit reachable `self.render_select_practice` and `self.render_enter_log` methods;
4. count only calls whose unqualified name is imported from `super::framework`;
5. reject local/root bindings or wrong-module imports that shadow required component names;
6. ignore nested items, comments, strings, and `#[cfg(test)]` methods/modules;
7. count `.borders`, `Block::bordered`, `Block::default`, direct status/shortcut calls, `highlight_row`, and selector `.skip/.take` as forbidden ownership.

Use these fixed counters:

```rust
#[derive(Default)]
struct LogEntryRenderOwnership {
    screen_frame_calls: usize,
    list_panel_calls: usize,
    bounded_panel_calls: usize,
    borders_method_calls: usize,
    block_constructor_calls: usize,
    status_line_calls: usize,
    shortcuts_calls: usize,
    highlight_calls: usize,
    selector_skip_calls: usize,
    selector_take_calls: usize,
    called_methods: BTreeSet<String>,
    wrong_origin_required_calls: BTreeSet<String>,
}

impl LogEntryRenderOwnership {
    fn merge(&mut self, other: Self) {
        self.screen_frame_calls += other.screen_frame_calls;
        self.list_panel_calls += other.list_panel_calls;
        self.bounded_panel_calls += other.bounded_panel_calls;
        self.borders_method_calls += other.borders_method_calls;
        self.block_constructor_calls += other.block_constructor_calls;
        self.status_line_calls += other.status_line_calls;
        self.shortcuts_calls += other.shortcuts_calls;
        self.highlight_calls += other.highlight_calls;
        self.selector_skip_calls += other.selector_skip_calls;
        self.selector_take_calls += other.selector_take_calls;
        self.called_methods.extend(other.called_methods);
        self.wrong_origin_required_calls
            .extend(other.wrong_origin_required_calls);
    }
}
```

Use a `syn::visit::Visit` implementation whose call handling follows this exact decision table:

```rust
match called_name.as_str() {
    "render_screen_frame_sections" if canonical_framework_import => {
        ownership.screen_frame_calls += 1;
    }
    "render_scrollable_list_panel" if canonical_framework_import => {
        ownership.list_panel_calls += 1;
    }
    "render_bounded_panel" if canonical_framework_import => {
        ownership.bounded_panel_calls += 1;
    }
    "render_screen_frame_sections"
    | "render_scrollable_list_panel"
    | "render_bounded_panel" => {
        ownership.wrong_origin_required_calls.insert(called_name);
    }
    "render_status_line" => ownership.status_line_calls += 1,
    "render_shortcuts" => ownership.shortcuts_calls += 1,
    "highlight_row" => ownership.highlight_calls += 1,
    _ => {}
}
```

For `self.method(...)`, insert the method name into `called_methods`. Drive reachable-method analysis with this worklist:

```rust
let methods = item_impl
    .items
    .iter()
    .filter_map(|item| match item {
        syn::ImplItem::Fn(method) if !is_cfg_test(&method.attrs) => {
            Some((method.sig.ident.to_string(), method))
        }
        _ => None,
    })
    .collect::<HashMap<_, _>>();
let mut pending = vec!["render".to_owned()];
let mut visited = HashSet::new();
while let Some(method_name) = pending.pop() {
    if !visited.insert(method_name.clone()) {
        continue;
    }
    let Some(method) = methods.get(&method_name) else {
        continue;
    };
    let mut visitor = LogEntryRenderVisitor::new(&canonical_framework_imports);
    visitor.visit_block(&method.block);
    pending.extend(
        visitor
            .ownership
            .called_methods
            .iter()
            .filter(|called| {
                methods.contains_key(called.as_str()) && !visited.contains(called.as_str())
            })
            .cloned(),
    );
    ownership.merge(visitor.ownership);
}
```

`visit_item` must not descend into nested declarations. The canonical-import collector recursively expands `UseTree::Path` and `UseTree::Group`, records unrenamed names whose full path is `super::framework::<required-name>`, and records rename/wrong-module imports as wrong-origin. A local/root function, const, static, parameter, or `let` pattern with a required name removes that name from the current canonical set and records wrong-origin use if called. This makes dead helpers unable to satisfy or violate production ownership and makes `use wrong::module::render_bounded_panel` fail closed.

Add:

```rust
#[test]
fn log_entry_delegates_all_presentation_to_framework() {
    let source = std::fs::read_to_string("src/tui/log_entry.rs").unwrap();
    let violations = log_entry_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn log_entry_ownership_rejects_wrong_origin_dead_and_test_decoys() {
    let source = r#"
        use wrong::framework::render_bounded_panel;
        use super::framework::{render_screen_frame_sections, render_scrollable_list_panel};
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel();
                Block::default().borders(Borders::ALL);
            }
            fn dead_helper(&self) {
                render_bounded_panel(); render_bounded_panel(); render_bounded_panel();
            }
            #[cfg(test)]
            fn test_decoy(&self) {
                render_bounded_panel(); render_bounded_panel(); render_bounded_panel();
            }
        }
    "#;
    let violations = log_entry_ownership_violations(source);
    assert!(violations.iter().any(|item| item.contains("wrong-origin")));
    assert!(violations.iter().any(|item| item.contains("BoundedPanel") && item.contains("found 0")));
    assert!(violations.iter().any(|item| item.contains("local Block")));
}

#[test]
fn log_entry_ownership_accepts_only_the_exact_reachable_component_path() {
    let source = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
            }
        }
    "#;
    assert!(log_entry_ownership_violations(source).is_empty());
}
```

Run:

```bash
cargo test --test tui_lint_test log_entry_delegates_all_presentation_to_framework -- --exact --nocapture
cargo test --test tui_lint_test log_entry_ownership_rejects_wrong_origin_dead_and_test_decoys -- --exact --nocapture
cargo test --test tui_lint_test log_entry_ownership_accepts_only_the_exact_reachable_component_path -- --exact --nocapture
```

Expected: production fails because only one frame and zero bounded panels exist and local Blocks remain; both synthetic controls pass.

- [ ] **Step 3: Replace entry-form frame/status/footer ownership**

Calculate the existing dynamic Sets height first. Replace centered area, `outer`, `chunks`, and direct status/footer rendering with:

```rust
let shortcuts = self.active_enter_shortcuts();
let status = if self.confirming_exit && self.status_msg.is_none() {
    Some((tr("log-cancel-confirm"), true))
} else {
    self.status_msg.clone()
};
let sections = render_screen_frame_sections(
    frame,
    frame.area(),
    CONTENT_WIDTH,
    &status,
    &shortcuts,
    &[
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(sets_content_lines.saturating_add(4)),
        Constraint::Length(7),
        Constraint::Min(0),
    ],
    false,
);
let chunks = &sections.body;
```

Delete the bottom direct `render_status_line` and `render_shortcuts` branches. Preserve navigation-error precedence exactly through `status`.

- [ ] **Step 4: Replace all four local Blocks with BoundedPanel**

Import `render_bounded_panel` and `BoundedPanelSpec`. Replace each `Block::default`, `block.inner`, `fit_rect`, and block render with one wrapper while keeping its current content body inside the closure.

Date:

```rust
let date_title = truncate_to_width(
    &format!(" {} ({}) ", practice.name, practice.practice_type.label()),
    chunks[0].width.saturating_sub(2) as usize,
);
render_bounded_panel(
    frame,
    chunks[0],
    BoundedPanelSpec {
        title: Some(Line::from(Span::styled(
            date_title,
            Style::default().fg(ACCENT).bold(),
        ))),
        border_style: Style::default().fg(BORDER_COLOR),
        padding: Padding::uniform(1),
    },
    |frame, date_inner| {
        self.render_date_content(frame, date_inner);
    },
);
```

Add `fn render_date_content(&self, frame: &mut Frame, date_inner: Rect)`. Its body is the current contiguous source beginning `let date_line = if self.editing_date` and ending with `frame.render_widget(Paragraph::new(date_line), date_inner);`, moved verbatim. No label, hint budgeting, color, or input call changes.

Sets:

```rust
render_bounded_panel(
    frame,
    chunks[2],
    BoundedPanelSpec {
        title: Some(Line::from(Span::styled(
            " Sets ",
            Style::default().fg(Color::White).bold(),
        ))),
        border_style: Style::default().fg(BORDER_COLOR),
        padding: Padding::uniform(1),
    },
    |frame, sets_inner| {
        self.render_sets_content(frame, sets_inner, practice);
    },
);
```

Add `fn render_sets_content(&self, frame: &mut Frame, sets_inner: Rect, practice: &Practice)`. Its body is the current contiguous source from `let mut sets_lines: Vec<Line> = Vec::new();` through `frame.render_widget(Paragraph::new(sets_lines), sets_inner);`, moved verbatim. The wrapper only replaces the old Sets block construction and safe-inner calculation.

Warm-up/Cool-down/RPE:

```rust
render_bounded_panel(
    frame,
    chunks[3],
    BoundedPanelSpec {
        title: None,
        border_style: Style::default().fg(BORDER_COLOR),
        padding: Padding::uniform(1),
    },
    |frame, wucd_inner| {
        self.render_wucd_content(frame, wucd_inner);
    },
);
```

Add `fn render_wucd_content(&self, frame: &mut Frame, wucd_inner: Rect)`. Its body is the current contiguous source from `let wu_active` through the wrapped `wucd_lines` render, moved verbatim. The wrapper only replaces the old untitled block construction and safe-inner calculation.

Note:

```rust
render_bounded_panel(
    frame,
    chunks[4],
    BoundedPanelSpec {
        title: Some(Line::from(Span::styled(
            format!(" {} ", tr("log-note-optional")),
            Style::default().fg(Color::Gray),
        ))),
        border_style: Style::default().fg(BORDER_COLOR),
        padding: Padding::horizontal(1),
    },
    |frame, note_inner| {
        self.render_note_content(frame, note_inner);
    },
);
```

Add `fn render_note_content(&self, frame: &mut Frame, note_inner: Rect)` with the current complete focused/inactive Note branch moved verbatim. Change `render_enter_log` to `fn render_enter_log(&self, frame: &mut Frame)`; all form rendering is read-only, while `render_select_practice` remains mutable for viewport updates. Import `Rect` beside `Constraint` for the four helper signatures. These helpers keep domain formatting in Log Entry and give the reachable ownership visitor explicit call edges. Do not introduce a form model. Remove production imports of `Block`, `Borders`, `centered_area`, `fit_rect`, `highlight_row`, `render_status_line`, `render_shortcuts`, `shortcut_height`, and `VisualSpan` only after `rg` confirms no remaining production use. Keep `Padding` because it configures BoundedPanel.

- [ ] **Step 5: Add real-frame tiny/resize coverage**

Retain the existing `log_entry_renders_small_rectangle_without_panicking` and add:

```rust
#[test]
fn every_log_entry_fixture_renders_one_tiny_and_resized_frames() {
    for name in LOG_ENTRY_GOLDEN_STATES {
        for (width, height) in [(1, 1), (24, 10), (80, 24), (107, 35)] {
            let mut screen = golden_log_entry_screen(name);
            let snapshot = render_to_snapshot(width, height, |frame| screen.render(frame));
            assert_eq!(
                snapshot.lines().count(),
                width as usize * height as usize + 1,
                "state={name} size={width}x{height}",
            );
        }
    }
}
```

Zero-area safety is covered through `Rect::new(0, 0, 0, 0)` in BoundedPanel, ScreenFrame, and ListPanel component tests. Do not add a production area parameter solely for tests.

- [ ] **Step 6: Run full ownership/fixture GREEN and resolve only proven lint movement**

Run:

```bash
cargo test tui::log_entry::tests::log_entry_states_match_golden_buffers -- --exact --nocapture
cargo test tui::log_entry::tests -- --nocapture
cargo test --test tui_lint_test log_entry_delegates_all_presentation_to_framework -- --exact --nocapture
cargo test --test tui_lint_test log_entry_ownership_rejects_wrong_origin_dead_and_test_decoys -- --exact --nocapture
cargo test --test tui_lint_test log_entry_ownership_accepts_only_the_exact_reachable_component_path -- --exact --nocapture
cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture
```

Expected: all pass in applicable targets and all eight fixtures remain exact. If Paragraph lines moved, map each old site to the identical variable/static widget and update only those literals. New variable content must use `.wrap(Wrap { trim: false })`; stop on semantic or out-of-scope failure.

- [ ] **Step 7: Run task gates and commit**

Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::framework:: -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --check
git diff --exit-code "$LOG_ENTRY_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/log_entry
git diff --exit-code 6b70eaf613a534db3828649e2115711bf4ea70d7..HEAD -- tests/fixtures/tui/dashboard
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code 6f30a12d8d9d0da14f7e9564aec6b3ddfc7bad67..HEAD -- tests/fixtures/tui/goals
```

Expected: every command exits `0`; strict Clippy has zero warnings; all accepted fixtures are unchanged.

Commit:

```bash
git add src/tui/framework/mod.rs src/tui/log_entry.rs tests/tui_lint_test.rs
git commit -m "refactor(log-entry): adopt shared form presentation"
```

---

### Task 5: Document, Release-Gate, Review, and Hand Off Batch 3B-7

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/log-entry-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed Log Entry migration, `LOG_ENTRY_FIXTURE_BASE`, final test counts, and accepted terminology.
- Produces: accurate shipped documentation, release-gated branch, independent whole-slice verdict, and a local binary for manual acceptance.

- [ ] **Step 1: Update active documentation**

- `CLAUDE.md`: add `bounded_panel.rs` to the framework map; mark Batch 3B-7 shipped; remove the final “Log Entry extraction unshipped” statement; state that Log Entry consumes two frames, one ListPanel, and four BoundedPanels while retaining domain state and behavior.
- `README.md` and `README_zh.md`: mark Log Entry extraction shipped with no visual, shortcut, route, or lifecycle change; retain permanent-removal language.
- `docs/specs/log-entry-spec.md`: add shared presentation ownership, eight immutable fixtures, exact `80x24` parity, and small-rectangle safety without changing any existing behavior contract.
- `docs/specs/navigation-spec.md`: state Batch 3B-7 changes presentation ownership only, not routes, intents, target preparation, request-scoped construction, failure retention, or shortcuts.
- framework design: add `bounded_panel.rs`, add `### Batch 3B-7: Log Entry Presentation Components — Complete`, and remove the remaining unshipped extraction statement. Do not claim a generic form engine or multi-line input.

- [ ] **Step 2: Audit documentation claims**

Run:

```bash
rg -n "Log Entry form extraction remains unshipped|Log Entry.*尚未交付|Log Entry.*未交付" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
rg -n "Batch 3B-7|BoundedPanel|bounded_panel|eight immutable|八个" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
rg -n "Quick Log|LLM|Abbreviations|abbreviations" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
```

Expected: the first command exits `1`; the second finds shipped ownership; the third finds only intentional permanent-removal or migration-history statements.

- [ ] **Step 3: Run fresh release gates**

Run:

```bash
cargo test tui::log_entry::tests -- --nocapture
cargo test tui::framework::bounded_panel::tests -- --nocapture
cargo test tui::framework::static_panel::tests -- --nocapture
cargo test tui::framework::screen_frame::tests -- --nocapture
cargo test tui::framework::list_panel::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check d92d785^..HEAD
test -x target/debug/iron
```

Expected: every command passes, strict Clippy has zero warnings, and fresh per-target counts are recorded. Do not replace the range-aware check with a working-tree-only check.

- [ ] **Step 4: Prove all fixture immutability and commit docs**

Run:

```bash
git diff --exit-code "$LOG_ENTRY_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/log_entry
git diff --exit-code 6b70eaf613a534db3828649e2115711bf4ea70d7..HEAD -- tests/fixtures/tui/dashboard
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code 6f30a12d8d9d0da14f7e9564aec6b3ddfc7bad67..HEAD -- tests/fixtures/tui/goals
rg -n "capture_log_entry_golden_buffers_once|UPDATE_GOLDEN|std::fs::write" src/tui/log_entry.rs
```

Expected: all fixture diffs exit `0`; updater scan exits `1`.

Commit:

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/log-entry-spec.md docs/specs/navigation-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Log Entry component ownership"
```

- [ ] **Step 5: Perform an independent whole-slice review**

Review from `d92d785^` through final `HEAD`. Report Critical, Important, and Minor findings with exact evidence. Verify:

- characterization commit precedes every production migration and no updater remains;
- eight `80x24` fixtures are byte-identical after migration;
- BoundedPanel is presentation-only, safe for pathological geometry, and preserves caller title/style/padding;
- StaticPanel and Dashboard fixtures are unchanged;
- selector uses one frame/list and retains filtering, columns, clamp selection, overflow, and highlight;
- entry form uses one frame/four panels and retains every field, dynamic height, total, focus, status precedence, shortcut, persistence, and navigation behavior;
- source ownership counts only executable reachable canonical calls and rejects wrong-origin/dead/test decoys;
- no local Log Entry border/inner/status/footer/list-slicing ownership remains;
- no dependency, locale, route, database, model, or other-screen production change occurred;
- docs are accurate and no extraction remains described as unshipped.

Fix every Critical or Important finding through one focused RED/GREEN fix wave, rerun all gates, and perform a scoped re-review of that fix. Record accepted Minor findings explicitly without expanding scope silently.

- [ ] **Step 6: Hand off the binary and stop**

Run:

```bash
git status --short --untracked-files=all
git log --oneline --decorate -10
test -x target/debug/iron
```

Expected: clean worktree, reviewable commits, executable binary. Ask the user to verify practice browse/filter/select/back; all four practice types; set add/edit/delete; date edit; long inputs; save/edit/cancel returns; validation/database/navigation failures; exit confirmation; `80x24`, `24x10`, and live resize; all other routes; and continued absence of removed surfaces.

Stop after the Log Entry-only handoff. Do not merge or push until explicit acceptance.

## Completion Checklist

- [ ] Eight deterministic pre-migration Log Entry fixtures are committed and immutable.
- [ ] BoundedPanel owns clipped border/title/padding/safe-inner geometry and no domain state.
- [ ] StaticPanel keeps its API and Dashboard output exactly.
- [ ] Practice selection uses one shared frame and one shared list panel.
- [ ] Entry form uses one shared frame and four bounded panels.
- [ ] Log Entry owns no local bordered panel, safe-inner calculation, status/footer rendering, or practice-list slicing/highlight.
- [ ] Domain state, validation, persistence, stable edit identity, return target, routing, and failure retention remain unchanged.
- [ ] `80x24` output is exact; tiny/pathological rectangles are safe.
- [ ] Ownership lint rejects wrong-origin/dead/test decoys and accepts only the reachable production component path.
- [ ] Existing component APIs and all accepted non-Log-Entry fixtures are unchanged.
- [ ] Documentation marks Batch 3B-7 shipped without claiming a form engine or multi-line input.
- [ ] Formatting, strict Clippy, full tests, build, range diff, fixture, removal, and binary gates pass.
- [ ] Independent review has no unresolved Critical or Important finding.
- [ ] User receives a Log Entry-only binary and implementation stops for manual acceptance.
