# TUI Consistency Batch 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the reusable TUI viewport, single-line input, detail-panel, status, shortcut, and test primitives, then migrate Goals, History, Quotes, Trends, and Practices without changing their domain responsibilities.

**Architecture:** Add a domain-independent `tui::framework` layer that receives Ratatui lines, rectangles, key events, and interaction state. Screens retain their data, modes, validation, database calls, and screen routing; they consume the framework primitives and preserve selected entities by stable database ID after reloads.

**Tech Stack:** Rust 2021, Ratatui 0.29 TestBackend, Crossterm 0.28, Unicode Width 0.2, Rusqlite 0.32, Fluent localization.

## Global Constraints

- Implement on `codex/tui-consistency`, based on validated Goals commit `0069c04`.
- Follow `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`.
- Do not add dependencies or change the SQLite schema.
- Framework code must not import `Database`, `Goal`, `LogEntry`, `Practice`, `Quote`, or screen routing types.
- Preserve accepted Goals behavior and shortcut meanings.
- Verify Batch 1 interactive screens at `80x24`; smaller rectangles must not panic.
- Do not modify Log Entry, Quick Log, or Dashboard implementation in Batch 1.
- Use tests before implementation and commit each task independently.

## File Structure

- Create `src/tui/framework/mod.rs`: public re-exports for framework primitives.
- Create `src/tui/framework/viewport.rs`: visual spans, overflow, navigation, and scroll state.
- Create `src/tui/framework/input.rs`: `TextInput` state, key handling, and bounded rendering spans.
- Create `src/tui/framework/components.rs`: detail panel, selected line rendering, status bar, and shortcut bar.
- Create `src/tui/framework/test_support.rs`: TestBackend rendering helpers under `cfg(test)`.
- Modify `src/tui/mod.rs`: register and re-export framework compatibility names.
- Modify `src/tui/goals.rs`: consume framework primitives while preserving behavior.
- Modify `src/tui/history.rs`: bounded filter and focusable detail viewport.
- Modify `src/tui/quotes_screen.rs`: compact list plus complete preview/focused quote.
- Modify `src/tui/trends.rs`: framework input, list viewport, status, and shortcuts.
- Modify `src/tui/practices.rs`: framework input/list/status/shortcuts and failure-preserving modes.
- Modify `locales/en.ftl` and `locales/zh-CN.ftl`: matching focus/overflow/status text only where existing keys do not express the behavior.
- Modify `docs/specs/{navigation,goals,history,trends,practices}.md`: Batch 1 behavior.
- Create `docs/specs/quotes-spec.md`: Quotes behavior, which currently has no standalone module spec.
- Modify `README.md` and `README_zh.md`: exact Batch 1 shortcuts.

---

### Task 1: Visual Viewport and Navigation Primitives

**Files:**
- Create: `src/tui/framework/mod.rs`
- Create: `src/tui/framework/viewport.rs`
- Modify: `src/tui/mod.rs`

**Interfaces:**
- Consumes: `ratatui::text::Line` and Unicode display widths.
- Produces: `VisualSpan`, `Overflow`, `EdgeBehavior`, `ViewportState`, `visual_spans`, `combined_span`, and `move_selection`.

- [ ] **Step 1: Write failing viewport tests**

Add these tests to `src/tui/framework/viewport.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    #[test]
    fn visual_spans_measure_ascii_cjk_and_empty_lines() {
        let lines = vec![Line::from("123456789"), Line::from("中文测试"), Line::from("")];
        assert_eq!(
            visual_spans(&lines, 4),
            vec![
                VisualSpan { start: 0, height: 3 },
                VisualSpan { start: 3, height: 2 },
                VisualSpan { start: 5, height: 1 },
            ]
        );
    }

    #[test]
    fn viewport_reveals_selected_span_and_reports_overflow() {
        let mut viewport = ViewportState::new(4);
        viewport.reveal(VisualSpan { start: 7, height: 2 });
        assert_eq!(viewport.offset(), 5);
        assert_eq!(viewport.overflow(12), Overflow { above: true, below: true });
        viewport.reveal(VisualSpan { start: 0, height: 1 });
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn navigation_supports_clamp_and_wrap_explicitly() {
        assert_eq!(move_selection(2, 3, 1, EdgeBehavior::Clamp), 2);
        assert_eq!(move_selection(2, 3, 1, EdgeBehavior::Wrap), 0);
        assert_eq!(move_selection(0, 3, -1, EdgeBehavior::Clamp), 0);
        assert_eq!(move_selection(0, 3, -1, EdgeBehavior::Wrap), 2);
    }
}
```

- [ ] **Step 2: Run the viewport tests and verify RED**

Run:

```bash
cargo test tui::framework::viewport::tests -- --nocapture
```

Expected: compilation fails because `tui::framework` and its types do not exist.

- [ ] **Step 3: Implement the viewport interfaces**

Create `src/tui/framework/viewport.rs` with these public types and signatures:

```rust
use ratatui::text::Line;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualSpan {
    pub start: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overflow {
    pub above: bool,
    pub below: bool,
}

impl Overflow {
    pub fn suffix(self) -> &'static str {
        match (self.above, self.below) {
            (true, true) => " ↑↓",
            (true, false) => " ↑",
            (false, true) => " ↓",
            (false, false) => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeBehavior { Clamp, Wrap }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ViewportState {
    offset: usize,
    height: usize,
}

impl ViewportState {
    pub fn new(height: usize) -> Self;
    pub fn offset(&self) -> usize;
    pub fn set_height(&mut self, height: usize);
    pub fn set_offset(&mut self, offset: usize);
    pub fn reveal(&mut self, span: VisualSpan);
    pub fn clamp(&mut self, content_height: usize);
    pub fn overflow(&self, content_height: usize) -> Overflow;
}

pub fn visual_spans(lines: &[Line<'_>], width: u16) -> Vec<VisualSpan>;
pub fn combined_span(spans: &[VisualSpan], start: usize, count: usize) -> Option<VisualSpan>;
pub fn move_selection(current: usize, len: usize, delta: isize, edge: EdgeBehavior) -> usize;
```

Use `Line::width().div_ceil(width.max(1) as usize)`, with empty lines consuming one row. `ViewportState::reveal` must reveal an entire span when it fits and at least its start when it is taller than the viewport.

Create `src/tui/framework/mod.rs` and add `pub mod framework;` to `src/tui/mod.rs`:

```rust
pub mod components;
pub mod input;
#[cfg(test)]
pub mod test_support;
pub mod viewport;

pub use viewport::{
    combined_span, move_selection, visual_spans, EdgeBehavior, Overflow, ViewportState,
    VisualSpan,
};
```

- [ ] **Step 4: Run viewport tests and the existing Goals layout tests**

Run:

```bash
cargo test tui::framework::viewport::tests -- --nocapture
cargo test tui::goals::tests -- --nocapture
```

Expected: all viewport tests and the existing Goals tests pass.

- [ ] **Step 5: Commit the viewport primitives**

```bash
git add src/tui/mod.rs src/tui/framework/mod.rs src/tui/framework/viewport.rs
git commit -m "feat(tui): add shared viewport primitives"
```

---

### Task 2: Reusable Unicode Single-Line Input

**Files:**
- Create: `src/tui/framework/input.rs`
- Modify: `src/tui/framework/mod.rs`
- Modify: `src/tui/mod.rs`

**Interfaces:**
- Consumes: `crossterm::event::KeyEvent`, terminal width, prefix width, and color.
- Produces: `TextInput` with domain-independent editing and `visible_spans`.

- [ ] **Step 1: Write failing `TextInput` tests**

Add to `src/tui/framework/input.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent { KeyEvent::new(code, KeyModifiers::NONE) }

    #[test]
    fn unicode_editing_moves_and_deletes_on_char_boundaries() {
        let mut input = TextInput::from("目标A");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value(), "目A");
        assert_eq!(input.cursor(), "目".len());
    }

    #[test]
    fn emacs_navigation_and_kill_match_project_contract() {
        let mut input = TextInput::from("abcdef");
        input.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        input.handle_key(key(KeyCode::Right));
        input.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(input.value(), "a");
    }

    #[test]
    fn visible_spans_keep_long_cursor_inside_width() {
        let input = TextInput::from("X".repeat(100));
        let rendered = input.visible_spans(12, 2, ratatui::style::Color::Green);
        let text: String = rendered
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        let line = ratatui::text::Line::from(rendered);
        assert!(line.width() <= 10);
        assert!(text.contains('_'));
    }
}
```

- [ ] **Step 2: Run input tests and verify RED**

```bash
cargo test tui::framework::input::tests -- --nocapture
```

Expected: compilation fails because `TextInput` does not exist.

- [ ] **Step 3: Implement `TextInput`**

Implement this API in `src/tui/framework/input.rs`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self;
    pub fn from(value: impl Into<String>) -> Self;
    pub fn value(&self) -> &str;
    pub fn cursor(&self) -> usize;
    pub fn is_empty_trimmed(&self) -> bool;
    pub fn trimmed(&self) -> &str;
    pub fn set(&mut self, value: impl Into<String>);
    pub fn clear(&mut self);
    pub fn handle_key(&mut self, key: KeyEvent) -> bool;
    pub fn visible_spans(
        &self,
        max_width: u16,
        prefix_width: u16,
        color: Color,
    ) -> Vec<Span<'static>>;
}
```

`handle_key` returns `true` only for handled editing keys. `visible_spans` owns its strings so screens can assemble `'static` modal lines. Move the current width-aware logic from `visible_input_spans` into this method, preserve `_` as the cursor, and keep a compatibility wrapper in `src/tui/mod.rs` until all three batches complete:

```rust
pub fn visible_input_spans<'a>(
    text: &'a str,
    cursor: usize,
    max_width: u16,
    prefix_width: u16,
    color: Color,
) -> Vec<Span<'a>>;
```

Do not change that compatibility function's current implementation in Batch 1; non-migrated Batch 2/3 screens still call it.

Re-export `TextInput` from `framework/mod.rs`.

- [ ] **Step 4: Run input, TUI lint, and full unit tests**

```bash
cargo test tui::framework::input::tests -- --nocapture
cargo test --test tui_lint_test
cargo test --lib
```

Expected: all pass and compatibility consumers still compile.

- [ ] **Step 5: Commit the input primitive**

```bash
git add src/tui/framework/input.rs src/tui/framework/mod.rs src/tui/mod.rs
git commit -m "feat(tui): add shared text input state"
```

---

### Task 3: Shared Detail, Status, Shortcut, and Test Components

**Files:**
- Create: `src/tui/framework/components.rs`
- Create: `src/tui/framework/test_support.rs`
- Modify: `src/tui/framework/mod.rs`
- Modify: `src/tui/mod.rs`
- Modify: `tests/tui_lint_test.rs`

**Interfaces:**
- Consumes: formatted `Line<'static>` content, actual `Rect`, `ViewportState`, and shortcut descriptors.
- Produces: `DetailPanelState`, `Shortcut`, `render_detail_panel`, `render_scrollable_lines`, `render_status_line`, `render_shortcuts`, and `render_to_string` for tests.

- [ ] **Step 1: Write failing component tests**

Add to `components.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::render_to_string;
    use ratatui::layout::{Constraint, Layout};

    #[test]
    fn focused_detail_panel_shows_accent_and_down_overflow() {
        let mut state = DetailPanelState::new(true);
        let output = render_to_string(40, 8, |frame| {
            let area = frame.area();
            render_detail_panel(
                frame,
                area,
                " Detail ",
                (1..=20).map(|n| Line::from(format!("Item {n}"))).collect(),
                &mut state,
                None,
            );
        });
        assert!(output.contains("Detail ↓"), "{output}");
        assert!(output.contains("Item 1"), "{output}");
    }

    #[test]
    fn status_and_shortcuts_render_in_reserved_rows() {
        let output = render_to_string(50, 3, |frame| {
            let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(2)])
                .split(frame.area());
            render_status_line(frame, rows[0], &Some(("Failed".into(), true)));
            render_shortcuts(
                frame,
                rows[1],
                &[Shortcut::new("Enter", "Open"), Shortcut::new("Esc", "Back")],
            );
        });
        assert!(output.contains("Failed"));
        assert!(output.contains("[Enter] Open"));
        assert!(output.contains("[Esc] Back"));
    }
}
```

- [ ] **Step 2: Run component tests and verify RED**

```bash
cargo test tui::framework::components::tests -- --nocapture
```

Expected: compilation fails because the component and test-support APIs do not exist.

- [ ] **Step 3: Implement the component APIs**

Use these exact public shapes:

```rust
pub type StatusMessage = Option<(String, bool)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub key: String,
    pub label: String,
}

impl Shortcut {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetailPanelState {
    pub focused: bool,
    pub viewport: ViewportState,
}

impl DetailPanelState {
    pub fn new(focused: bool) -> Self;
    pub fn scroll_by(&mut self, delta: isize, content_height: usize);
}

pub fn render_scrollable_lines(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line<'static>>,
    viewport: &mut ViewportState,
    selected: Option<VisualSpan>,
);

pub fn render_detail_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    lines: Vec<Line<'static>>,
    state: &mut DetailPanelState,
    selected: Option<VisualSpan>,
);

pub fn render_status_line(frame: &mut Frame, area: Rect, status: &StatusMessage);
pub fn render_shortcuts(frame: &mut Frame, area: Rect, shortcuts: &[Shortcut]);
pub fn truncate_to_width(text: &str, width: usize) -> String;
```

`render_detail_panel` must calculate its block inner area first, set viewport height from that inner rectangle, reveal the selected span, clamp offset, add `Overflow::suffix()` to the title, and use cyan only when focused. `render_scrollable_lines` must highlight all visible rows in the selected span.

Create `test_support.rs`:

```rust
pub fn render_to_string(
    width: u16,
    height: u16,
    mut render: impl FnMut(&mut ratatui::Frame<'_>),
) -> String;
```

Implement it with `TestBackend`, `Terminal::draw`, and buffer row concatenation. Re-export the framework status aliases from `src/tui/mod.rs` so non-migrated screens keep compiling. Update TUI lint allowlist entries only when a fixed one-line UI element intentionally omits wrapping; prefer `.wrap(Wrap { trim: false })` for variable content.

- [ ] **Step 4: Run framework and lint tests**

```bash
cargo test tui::framework -- --nocapture
cargo test --test tui_lint_test
```

Expected: all pass.

- [ ] **Step 5: Commit shared components**

```bash
git add src/tui/framework src/tui/mod.rs tests/tui_lint_test.rs
git commit -m "feat(tui): add shared interaction components"
```

---

### Task 4: Migrate Goals as the Framework Reference Consumer

**Files:**
- Modify: `src/tui/goals.rs`
- Modify: `src/tui/framework/components.rs` only if a missing framework behavior is proven by a failing Goals characterization test.

**Interfaces:**
- Consumes: all Batch 1 framework APIs from Tasks 1-3.
- Produces: unchanged Goals behavior with no private duplicate viewport or input implementation.

- [ ] **Step 1: Add Goals characterization tests before migration**

Keep every existing Goals test and add:

```rust
#[test]
fn goals_framework_migration_preserves_preview_and_focus_contract() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    for number in 1..=20 {
        db.create_milestone(goal_id, &format!("Milestone {number:02}"))
            .unwrap();
    }
    let mut screen = GoalsScreen::new(&db).unwrap();
    let preview = render_screen(&mut screen, 80, 24);
    assert!(preview.contains("Goal ↓"), "{preview}");
    screen.handle_key(key(KeyCode::Enter), &db);
    for _ in 1..20 { screen.handle_key(key(KeyCode::Down), &db); }
    let focused = render_screen(&mut screen, 80, 24);
    assert!(focused.contains("Milestone 20"), "{focused}");
}
```

Use the test module's existing `key`/`render_screen` helpers or add the one-line `key` helper if absent.

- [ ] **Step 2: Run the characterization test before migration**

```bash
cargo test tui::goals::tests::goals_framework_migration_preserves_preview_and_focus_contract -- --nocapture
```

Expected: pass against the validated implementation. This is the before-migration baseline.

- [ ] **Step 3: Replace private Goals primitives**

- Remove private `VisualSpan`, `visual_spans`, `keep_span_visible`, and `owned_visible_input_spans`.
- Import the corresponding framework APIs.
- Replace `input: String` plus `cursor: usize` with `input: TextInput`.
- Replace `modal_scroll: usize` with `detail_state: DetailPanelState` or its contained `ViewportState`.
- Render the milestone panel through `render_detail_panel` while keeping current popup placement and goal-specific content construction.
- Continue preserving the modal goal by ID in `reload_goals`.
- Keep all domain-specific modes and database operations inside `goals.rs`.

The key transition pattern must remain:

```rust
match db.update_milestone(milestone_id, input.trimmed()) {
    Ok(()) => match self.reload_goals(db) {
        Ok(()) => {
            self.input.clear();
            self.modal_mode = ModalMode::Browse;
        }
        Err(error) => self.modal_status_msg = Some((format!("Error: {error}"), true)),
    },
    Err(error) => self.modal_status_msg = Some((format!("Error: {error}"), true)),
}
```

- [ ] **Step 4: Run all Goals and framework tests**

```bash
cargo test tui::goals::tests -- --nocapture
cargo test tui::framework -- --nocapture
```

Expected: all tests, including the pre-migration characterization test, pass.

- [ ] **Step 5: Confirm duplicate code is gone and commit**

```bash
rg -n "fn visual_spans|fn keep_span_visible|fn owned_visible_input_spans" src/tui/goals.rs
```

Expected: no matches.

```bash
git add src/tui/goals.rs src/tui/framework/components.rs
git commit -m "refactor(goals): consume shared TUI framework"
```

---

### Task 5: Migrate History List and Focusable Detail Panel

**Files:**
- Modify: `src/tui/history.rs`
- Modify: `docs/specs/history-spec.md`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: `TextInput`, `ViewportState`, `DetailPanelState`, shared status and shortcuts.
- Produces: History Browse/Detail/ConfirmDelete modes with stable list selection and complete detail access.

- [ ] **Step 1: Write failing History rendering and focus tests**

Add a TestBackend helper via `framework::test_support::render_to_string` and tests that create a log with at least 20 sets plus long warm-up, cool-down, and note text:

```rust
fn history_db_with_long_entry(set_count: usize) -> Database {
    let db = Database::open_in_memory().unwrap();
    let practice = db
        .create_practice("Pull-up", crate::model::PracticeType::Bodyweight)
        .unwrap();
    let sets = (0..set_count)
        .map(|_| SetData::Bodyweight { reps: 10 })
        .collect::<Vec<_>>();
    let logged_at = chrono::NaiveDate::from_ymd_opt(2026, 8, 10)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let warm_up = "Warm-up text ".repeat(30);
    let cool_down = "Cool-down text ".repeat(30);
    db.create_log_at(
        practice.id,
        &logged_at,
        &sets,
        Some("Final note text"),
        Some(&warm_up),
        Some(&cool_down),
        Some(8),
    )
    .unwrap();
    db
}

#[test]
fn history_detail_focus_scrolls_to_last_content() {
    let db = history_db_with_long_entry(20);
    let mut screen = HistoryScreen::new(&db).unwrap();
    screen.handle_key(key(KeyCode::Enter), &db);
    for _ in 0..30 { screen.handle_key(key(KeyCode::Down), &db); }
    let output = render_history(&mut screen, 80, 24);
    assert_eq!(screen.mode, Mode::Detail);
    assert!(output.contains("Final note text"), "{output}");
    assert!(output.contains('↑'), "{output}");
}

#[test]
fn history_long_filter_keeps_cursor_visible() {
    let db = Database::open_in_memory().unwrap();
    let mut screen = HistoryScreen::new(&db).unwrap();
    screen.filtering = true;
    screen.filter = TextInput::from("练习".repeat(60));
    let output = render_history(&mut screen, 80, 24);
    assert!(output.contains('_'), "{output}");
    assert!(output.matches("练习").count() < 30, "{output}");
}
```

- [ ] **Step 2: Run History tests and verify RED**

```bash
cargo test tui::history::tests::history_detail_focus_scrolls_to_last_content -- --nocapture
cargo test tui::history::tests::history_long_filter_keeps_cursor_visible -- --nocapture
```

Expected: fail because History has no Detail mode or `TextInput` filter.

- [ ] **Step 3: Implement History migration**

- Change mode to `Browse | Detail | ConfirmDelete`.
- Replace `filter_text`/`filter_cursor` with `filter: TextInput`.
- Add `list_viewport: ViewportState` and `detail_state: DetailPanelState`.
- In Browse, `Enter` changes to Detail and resets detail offset; `e` retains edit navigation.
- In Detail, `j/k` and arrows call `detail_state.scroll_by(±1, content_height)`; `Esc` returns to Browse.
- Use the selected entry ID before list reload and restore its filtered position afterward when present.
- Compute detail panel height from wrapped visual rows and actual inner width. Preview uses standard border; focused detail uses accent.
- Replace `if let Ok(entries) = db.list_logs_all()` reload branches with an explicit `match` that updates entries on `Ok` and sets an error status on `Err`.
- Use shared status and shortcut renderers for every mode.

Render filter input as:

```rust
let mut spans = vec![Span::styled(" /", filter_style)];
if self.filtering {
    spans.extend(self.filter.visible_spans(filter_area.width, 2, ACCENT));
} else {
    spans.push(Span::styled(self.filter.value().to_owned(), filter_style));
}
```

- [ ] **Step 4: Run History, framework, i18n, and lint tests**

```bash
cargo test tui::history::tests -- --nocapture
cargo test tui::framework -- --nocapture
cargo test --test i18n_test --test tui_lint_test
```

Expected: all pass.

- [ ] **Step 5: Update History spec and commit**

Document `Enter` detail focus, detail `j/k`, `Esc` focus exit, overflow indicators, and bounded filter input in `docs/specs/history-spec.md`.

```bash
git add src/tui/history.rs docs/specs/history-spec.md locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(history): add focusable scrollable details"
```

---

### Task 6: Migrate Quotes List, Preview, and Editing

**Files:**
- Modify: `src/tui/quotes_screen.rs`
- Create: `docs/specs/quotes-spec.md`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: framework list/detail, `TextInput`, status, and shortcuts.
- Produces: compact Quote list with complete preview/focus and failure-preserving Add/Edit.

- [ ] **Step 1: Write failing Quotes tests**

```rust
#[test]
fn selected_long_quote_has_complete_focusable_preview() {
    let db = Database::open_in_memory().unwrap();
    let text = "Long quote ".repeat(80);
    db.create_quote(&text).unwrap();
    let mut screen = QuotesScreen::new(&db).unwrap();
    screen.handle_key(key(KeyCode::Enter), &db);
    for _ in 0..20 { screen.handle_key(key(KeyCode::Down), &db); }
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
```

- [ ] **Step 2: Run Quotes tests and verify RED**

```bash
cargo test tui::quotes_screen::tests -- --nocapture
```

Expected: fail because Quotes has no Detail mode and empty edits currently exit.

- [ ] **Step 3: Implement Quotes migration**

- Change modes to `Browse | Detail | Add | Edit | ConfirmDelete`.
- In Browse, `Enter` focuses the complete quote preview; `e` edits.
- Use `truncate_to_width` only for compact list rows; never truncate the preview text.
- Add `detail_state: DetailPanelState` and scroll it with `j/k` or arrows in Detail.
- Replace raw input fields with `TextInput` and calculate prefix width with `Line::width()`.
- Change `refresh` to return `anyhow::Result<()>`, preserve selected quote by ID, and show refresh errors.
- Only clear Add/Edit input and return to Browse after create/update plus refresh succeeds.
- Preserve `last_deleted` when restore fails; refresh only after successful delete/restore.
- Render active shortcuts through the shared shortcut component.

Use localized validation text for empty quote input in both FTL files.

- [ ] **Step 4: Run Quotes, i18n, and lint tests**

```bash
cargo test tui::quotes_screen::tests -- --nocapture
cargo test --test i18n_test --test tui_lint_test
```

Expected: all pass.

- [ ] **Step 5: Add Quotes spec and commit**

The new spec must document compact list rows, complete preview, `Enter` focus, `e` edit, focused scrolling, Add/Edit failure retention, delete/undo, and shortcuts.

```bash
git add src/tui/quotes_screen.rs docs/specs/quotes-spec.md locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(quotes): add complete focusable previews"
```

---

### Task 7: Migrate Trends Selection, Filter, Status, and Shortcuts

**Files:**
- Modify: `src/tui/trends.rs`
- Modify: `docs/specs/trends-spec.md`

**Interfaces:**
- Consumes: `TextInput`, `ViewportState`, `move_selection`, shared status and shortcuts.
- Produces: bounded Trends selection/filter behavior while preserving specialized chart rendering.

- [ ] **Step 1: Write failing Trends tests**

```rust
#[test]
fn trends_long_filter_and_last_practice_remain_visible() {
    let mut screen = make_screen(30, 5);
    screen.filtering = true;
    screen.filter = TextInput::from("X".repeat(100));
    let filter_output = render_trends(&mut screen, 80, 24);
    assert!(filter_output.contains('_'));
    assert!(filter_output.matches('X').count() < 80);
    screen.filtering = false;
    for _ in 1..30 { screen.handle_key(key(KeyCode::Down)); }
    let list_output = render_trends(&mut screen, 80, 24);
    assert!(list_output.contains("Practice 30"), "{list_output}");
}
```

- [ ] **Step 2: Run the Trends test and verify RED**

```bash
cargo test tui::trends::tests::trends_long_filter_and_last_practice_remain_visible -- --nocapture
```

Expected: fail because Trends still renders the filter with raw cursor spans.

- [ ] **Step 3: Implement Trends migration**

- Replace `filter_text`/`filter_cursor` with `filter: TextInput`.
- Replace local `adjust_scroll` with `ViewportState::reveal(VisualSpan { start: selected, height: 1 })` after setting the real list height.
- Keep wrap navigation by calling `move_selection(self.selected, self.filtered_indices.len(), delta, EdgeBehavior::Wrap)` with `delta` set to `1` for down and `-1` for up.
- Bound practice name columns with `truncate_to_width` instead of deriving a column wider than the panel.
- Add `status_msg: StatusMessage` and a reserved status row.
- Change `refresh_chart` to display database errors rather than silently leaving stale entries.
- Render footer and status with shared components; preserve chart widget and statistics code.

- [ ] **Step 4: Run Trends, framework, and lint tests**

```bash
cargo test tui::trends::tests -- --nocapture
cargo test tui::framework -- --nocapture
cargo test --test tui_lint_test
```

Expected: all pass.

- [ ] **Step 5: Update Trends spec and commit**

```bash
git add src/tui/trends.rs docs/specs/trends-spec.md
git commit -m "refactor(trends): use shared TUI interactions"
```

---

### Task 8: Migrate Practices List, Forms, and Mutation Transitions

**Files:**
- Modify: `src/tui/practices.rs`
- Modify: `docs/specs/practices-spec.md`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: framework input, viewport, status, shortcuts, and truncation.
- Produces: bounded Practices UI whose failed mutations retain mode and input.

- [ ] **Step 1: Write failing Practices tests**

```rust
fn practices_db(count: usize) -> Database {
    let db = Database::open_in_memory().unwrap();
    for number in 1..=count {
        db.create_practice(
            &format!("Practice {number:02}"),
            PracticeType::Weighted,
        )
        .unwrap();
    }
    db
}

#[test]
fn duplicate_rename_retains_edit_mode_and_input() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice("Existing", PracticeType::Weighted).unwrap();
    db.create_practice("Target", PracticeType::Bodyweight).unwrap();
    let mut screen = PracticesScreen::new(&db).unwrap();
    screen.selected = screen.practices.iter().position(|p| p.name == "Target").unwrap();
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
    for _ in 1..30 { screen.handle_key(key(KeyCode::Down), &db); }
    let output = render_practices(&mut screen, 80, 24);
    assert!(output.contains("Practice 30"), "{output}");
}
```

- [ ] **Step 2: Run Practices tests and verify RED**

```bash
cargo test tui::practices::tests -- --nocapture
```

Expected: duplicate rename returns to Browse and the new test fails to compile until `TextInput` is adopted.

- [ ] **Step 3: Implement Practices migration**

- Replace local input fields and handler with `TextInput`.
- Replace local scroll adjustment with `ViewportState` and clamp navigation.
- Size the list with `Constraint::Min(4)` rather than the number of practices; use the real inner height.
- Truncate long name/type columns to the available inner width.
- Change `refresh` to return `anyhow::Result<()>` and preserve selection by practice ID.
- Keep AddType as module-owned content but render its selection through the shared selected-line renderer.
- In AddType and EditName, change mode and clear input only after mutation plus refresh succeeds.
- Keep Delete confirmation open on database failure and show the shared status line.
- Render every mode's footer from `Shortcut` descriptors.

- [ ] **Step 4: Run Practices, i18n, and framework tests**

```bash
cargo test tui::practices::tests -- --nocapture
cargo test --test i18n_test --test tui_lint_test
cargo test tui::framework -- --nocapture
```

Expected: all pass.

- [ ] **Step 5: Update Practices spec and commit**

```bash
git add src/tui/practices.rs docs/specs/practices-spec.md locales/en.ftl locales/zh-CN.ftl
git commit -m "refactor(practices): use shared TUI interactions"
```

---

### Task 9: Batch 1 Documentation, Regression, and Manual Handoff

**Files:**
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/specs/goals-spec.md` only to reference the shared behavior contract; do not change accepted behavior.
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `CLAUDE.md`
- Modify: `tests/tui_lint_test.rs` only for accurate framework paths or removed stale entries.

**Interfaces:**
- Consumes: completed Batch 1 framework and migrated screens.
- Produces: accurate user/developer documentation and a verified Batch 1 checkpoint.

- [ ] **Step 1: Update navigation and architecture documentation**

Document:

- clamp versus wrap is explicit per screen
- `Enter` focuses History/Quotes detail and `Esc` returns to the list
- all Batch 1 text inputs use `TextInput`
- framework source ownership and the rule that screens retain domain/database state
- Batch 1 minimum verification size `80x24`

Update the English and Chinese README shortcut tables for Goals, History, Quotes, Trends, and Practices. Do not document Batch 2/3 behavior as already implemented.

- [ ] **Step 2: Run formatting and static analysis**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: zero warnings and zero errors.

- [ ] **Step 3: Run full automated verification**

```bash
cargo test
cargo build
```

Expected: every framework, screen, database, integration, i18n, and lint test passes; build exits 0.

- [ ] **Step 4: Verify Batch 1 scope and duplicate removal**

```bash
git diff --name-only 6b86273..HEAD
rg -n "fn handle_text_input|fn adjust_scroll|fn visual_spans|fn keep_span_visible" \
  src/tui/goals.rs src/tui/history.rs src/tui/quotes_screen.rs src/tui/trends.rs src/tui/practices.rs
git status --short
```

Expected:

- no Log Entry, Quick Log, or Dashboard implementation files in the diff
- no superseded local input/scroll helper matches in migrated screens
- clean working tree after the documentation commit

- [ ] **Step 5: Commit Batch 1 documentation**

```bash
git add README.md README_zh.md CLAUDE.md docs/specs tests/tui_lint_test.rs
git commit -m "docs(tui): document Batch 1 interaction framework"
```

- [ ] **Step 6: Prepare manual validation handoff**

Provide the built binary and ask the user to verify:

1. Goals behavior is unchanged.
2. History and Quotes detail focus can reach first and last wrapped content.
3. Trends and Practices can reach the first and last items in long lists.
4. Long filters and edit inputs retain a visible cursor.
5. Focused panels use accent borders and show correct overflow arrows.
6. Failed practice rename retains input and mode.
7. Every footer matches actual commands.

Do not start Batch 2 until the user explicitly accepts Batch 1.
