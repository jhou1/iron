# TUI Consistency Batch 3B-2 Quotes Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Quotes to the proven screen-frame, scrollable-list, and action-panel presentation components without changing its rendered output or behavior.

**Architecture:** Quotes keeps ownership of modes, database operations, validation, input, undo, shortcuts, lifecycle, and floating-preview policy. The existing presentation components own frame, list, viewport, highlight, status, shortcut, and action rendering; `ScrollableListPanel` gains one presentation-only return value exposing safely clipped outer and inner rectangles, and `ScreenFrame` gains a generic configured renderer for a minimum body height plus optional trailing flex while retaining its default wrapper. Six immutable styled-buffer fixtures bind the accepted pre-refactor output.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Fluent localization, the existing TUI framework, Cargo tests, Clippy, and Ratatui `TestBackend`.

## Global Constraints

- Base implementation commit is `2a530ffc6e9eb47ea8411b5a9880e6e10ca76a3d`; approved design commit is `ae2d742` on `codex/tui-consistency-batch-3b-quotes`.
- Follow [the approved Batch 3B-2 design](../specs/2026-08-12-tui-consistency-batch-3b-quotes-components-design.md).
- Batch 3B-2 migrates Quotes only. Trends, History, Goals, Dashboard, and Log Entry remain unchanged.
- Preserve Quotes output, shortcuts, preview, Detail scrolling, mutation ordering, undo, navigation, and lifecycle behavior. The six `80x24` styled-buffer fixtures may not change during migration.
- Practices fixtures must remain unchanged after the list-panel API extension.
- Quotes retains database access, stable IDs, modes, validation, localized status creation, key handling, shortcut selection, undo policy, domain-row formatting, and floating-preview policy.
- Framework components remain presentation-only. They do not import `Database`, `Quote`, screen modes, navigation intents, `anyhow`, or business errors.
- `ScrollableListPanel` may return only safely clipped presentation geometry. It still mutates only the supplied `ViewportState`.
- No generic floating-popup or composite list/detail component is authorized.
- All rectangle arithmetic is saturating or clamped. Empty, zero-sized, one-cell, narrow, short, and edge-positioned targets must not panic or escape their parent/frame.
- Do not change database schema, localization copy, navigation topology, colors, borders, padding, spacing, or keyboard commands.
- Do not add speculative configuration for later screens.
- Do not reintroduce Quick Log, LLM configuration/runtime, Abbreviations, or the `w` action.
- Every production change follows RED-GREEN-refactor. Record the expected failure before implementing the corresponding behavior.
- Fixture generation exists only long enough to capture the old implementation and is removed before the characterization commit.
- Structural ownership lint must inspect executable source. Comments, ordinary strings, raw strings, and character literals do not count as delegation evidence; Rust lifetime syntax must remain visible to the scanner.
- Paragraph lint remains recursive and keeps its current four-line scan window. Refresh an allowlist line only after proving the old and new lines are the same approved widget.
- Stop for a scope ruling if a fixture would need to change, an existing test conflicts with the fixture, a user-facing behavior change appears necessary, or a tracked file outside this plan is required.
- Use five planned commits: Quotes fixtures; list-panel geometry; approved footer-preservation design/plan correction; Quotes migration plus generic ScreenFrame configuration and structural lint; active documentation. Any review-discovered correctness fix uses a separate follow-up commit with focused RED/GREEN evidence.
- Stop after the Quotes-only manual-acceptance handoff. Do not begin Trends until the user explicitly accepts Quotes.

## File Structure

**Create:**

- `tests/fixtures/tui/quotes/browse.txt` — complete styled buffer for Browse.
- `tests/fixtures/tui/quotes/browse_with_undo.txt` — Browse with optional undo shortcut.
- `tests/fixtures/tui/quotes/detail.txt` — focused long-preview Detail mode.
- `tests/fixtures/tui/quotes/add.txt` — Add input and actions.
- `tests/fixtures/tui/quotes/edit.txt` — Edit input and actions.
- `tests/fixtures/tui/quotes/confirm_delete.txt` — delete confirmation and actions.

**Modify:**

- `src/tui/quotes_screen.rs` — fixtures, domain-line builders, preview geometry helper, component-based render path, and focused regressions.
- `src/tui/framework/list_panel.rs` — `ListPanelAreas` return contract and geometry tests.
- `src/tui/framework/screen_frame.rs` — generic `ScreenFrameLayout` and configured renderer with a backward-compatible default wrapper.
- `src/tui/framework/mod.rs` — re-export list geometry and configured ScreenFrame interfaces.
- `tests/tui_lint_test.rs` — transfer Quotes bordered-list ownership to executable `ScrollableListPanel` delegation.
- `CLAUDE.md` — mark Practices and Quotes as the shipped component consumers.
- `README.md` — describe Batch 3B-2 Quotes as shipped without claiming later slices.
- `README_zh.md` — Chinese equivalent of the shipped status.
- `docs/specs/quotes-spec.md` — describe component ownership and unchanged preview/business behavior.
- `docs/specs/navigation-spec.md` — state that Batch 3B-2 changes presentation ownership only.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — mark Quotes extraction complete and later slices unshipped.

**Do not modify unless a verified source-line move requires a mechanical mapping refresh:**

- Paragraph allowlist literals in `tests/tui_lint_test.rs`.

---

### Task 1: Characterize All Quotes Presentation States

**Files:**
- Modify: `src/tui/quotes_screen.rs:521-694`
- Create: `tests/fixtures/tui/quotes/browse.txt`
- Create: `tests/fixtures/tui/quotes/browse_with_undo.txt`
- Create: `tests/fixtures/tui/quotes/detail.txt`
- Create: `tests/fixtures/tui/quotes/add.txt`
- Create: `tests/fixtures/tui/quotes/edit.txt`
- Create: `tests/fixtures/tui/quotes/confirm_delete.txt`

**Interfaces:**
- Consumes: existing `render_to_snapshot(width, height, render)` from `src/tui/framework/test_support.rs`, current `QuotesScreen::render`, private test access to `Mode`, `input`, `editing_id`, `last_deleted`, `detail_state`, and `status_msg`.
- Produces: six compile-time fixtures and `quotes_modes_match_golden_buffers`, which Task 3 must pass without fixture changes.

- [ ] **Step 1: Create deterministic Quotes fixture state**

In the Quotes test module, import `render_to_snapshot` alongside `render_to_string`, then add:

```rust
fn golden_quotes_screen(name: &str) -> QuotesScreen {
    std::env::set_var("LC_ALL", "C");
    crate::i18n::init();

    let db = Database::open_in_memory().unwrap();
    db.create_quote("First compact quote").unwrap();
    let long = db
        .create_quote(
            &"A deliberately long quote with Unicode 训练 and enough words to wrap. "
                .repeat(8),
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
```

Keep all six states in one test so the locale mutation and fixture capture are serialized within each test process.

- [ ] **Step 2: Add the temporary fixture assertion**

Create each fixture file with one newline, then add:

```rust
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
        let path = format!(
            "{}/tests/fixtures/tui/quotes/{name}.txt",
            env!("CARGO_MANIFEST_DIR")
        );
        if std::env::var_os("UPDATE_QUOTES_GOLDENS").is_some() {
            std::fs::write(&path, &actual).expect("Quotes fixture should be written");
        }
        let expected = std::fs::read_to_string(&path).expect("Quotes fixture should exist");
        assert_eq!(actual, expected, "Quotes {name} buffer changed");
    }
}
```

- [ ] **Step 3: Run the fixture test and record RED**

Run:

```bash
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
```

Expected: FAIL on `browse` because the one-line fixture does not equal the current `80x24` styled buffer. Record the failing assertion before generating fixtures.

- [ ] **Step 4: Capture the old implementation exactly once**

Run:

```bash
UPDATE_QUOTES_GOLDENS=1 cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
wc -l tests/fixtures/tui/quotes/*.txt
```

Expected: the test passes and every fixture reports `1921` lines: one area header plus `80 * 24` cell records.

- [ ] **Step 5: Remove the runtime update and file-read paths**

Add a compile-time fixture selector:

```rust
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
```

Replace the temporary environment/write/read block with:

```rust
let expected = quotes_golden(name);
assert_eq!(actual, expected, "Quotes {name} buffer changed");
```

Verify the production and test source no longer contains the update marker:

```bash
rg -n "UPDATE_QUOTES_GOLDENS|std::fs::write|read_to_string" src/tui/quotes_screen.rs
```

Expected: no matches.

- [ ] **Step 6: Run characterization and existing Quotes suites**

```bash
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo fmt -- --check
git diff --check
```

Expected: six fixtures pass in both library and binary targets, all pre-existing Quotes tests pass, lint passes, and formatting/diff checks exit zero.

- [ ] **Step 7: Commit immutable characterization**

```bash
git add src/tui/quotes_screen.rs tests/fixtures/tui/quotes
git commit -m "test(quotes): capture mode rendering"
```

The commit must contain no framework or production render change.

---

### Task 2: Return Safe Geometry from ScrollableListPanel

**Files:**
- Modify: `src/tui/framework/list_panel.rs:11-62`
- Modify: `src/tui/framework/mod.rs:16`

**Interfaces:**
- Consumes: existing `fit_rect`, `Rect`, `ViewportState`, and `render_scrollable_list_panel` behavior.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_scrollable_list_panel<'a>(...) -> ListPanelAreas
```

Task 3 uses `ListPanelAreas::inner` for Quotes preview placement. Existing Practices calls may ignore the return value.

- [ ] **Step 1: Add a failing geometry-return regression**

In `list_panel.rs` tests, add:

```rust
fn rect_is_within(parent: Rect, child: Rect) -> bool {
    child.x >= parent.x
        && child.y >= parent.y
        && child.right() <= parent.right()
        && child.bottom() <= parent.bottom()
}

#[test]
fn list_panel_returns_clipped_outer_and_inner_geometry() {
    let frame_area = Rect::new(0, 0, 6, 6);
    for target in [
        Rect::new(0, 0, 6, 6),
        Rect::new(5, 5, 0, 0),
        Rect::new(5, 5, 1, 1),
        Rect::new(1, 1, 2, 3),
        Rect::new(5, 5, 4, 4),
    ] {
        let mut viewport = ViewportState::new(0);
        let mut returned = None;
        let mut closure_inner = None;
        render_to_buffer(6, 6, |frame| {
            let areas = render_scrollable_list_panel(
                frame,
                target,
                "Items",
                &mut viewport,
                |inner| {
                    closure_inner = Some(inner);
                    ListPanelContent {
                        header: None,
                        rows: Vec::new(),
                        empty: None,
                        selected: None,
                    }
                },
            );
            returned = Some(areas);
        });
        let areas = returned.unwrap();
        assert_eq!(closure_inner, Some(areas.inner));
        assert!(rect_is_within(frame_area, areas.area), "{target:?} -> {areas:?}");
        assert!(rect_is_within(areas.area, areas.inner), "{target:?} -> {areas:?}");
    }
}
```

- [ ] **Step 2: Run the focused test and record RED**

```bash
cargo test tui::framework::list_panel::tests::list_panel_returns_clipped_outer_and_inner_geometry -- --exact --nocapture
```

Expected: compilation fails because the renderer still returns `()` and there is no `ListPanelAreas` contract.

- [ ] **Step 3: Add the minimal return type**

In `list_panel.rs`, add the public type immediately after `ListPanelContent`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}
```

Change the function return type to `-> ListPanelAreas`. Keep all existing rendering statements unchanged and finish the function with:

```rust
    ListPanelAreas { area, inner }
```

Do not add mode, style, popup, or domain parameters.

- [ ] **Step 4: Export the return type**

Change `src/tui/framework/mod.rs` from:

```rust
pub use list_panel::{render_scrollable_list_panel, ListPanelContent};
```

to:

```rust
pub use list_panel::{render_scrollable_list_panel, ListPanelAreas, ListPanelContent};
```

- [ ] **Step 5: Verify component geometry and unchanged rendering**

```bash
cargo test tui::framework::list_panel::tests -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::practices::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
git diff --check
```

Expected: all commands exit zero; both returned rectangles stay bound; Practices output is unchanged.

- [ ] **Step 6: Audit the API boundary**

```bash
rg -n "Database|Quote|Mode|NavigationIntent|anyhow|popup|detail" \
  src/tui/framework/list_panel.rs
git diff -- src/tui/framework/list_panel.rs src/tui/framework/mod.rs
```

Expected: the first command has no production matches; the diff contains only geometry return type/tests/export, not rendering-policy changes.

- [ ] **Step 7: Commit the geometry contract**

```bash
git add src/tui/framework/list_panel.rs src/tui/framework/mod.rs
git commit -m "refactor(tui): expose list panel geometry"
```

---

### Task 3: Migrate Quotes to Shared Presentation Components

**Files:**
- Modify: `src/tui/quotes_screen.rs:1-309,521-694`
- Modify: `src/tui/framework/screen_frame.rs`
- Modify: `src/tui/framework/mod.rs`
- Modify: `tests/tui_lint_test.rs:242-287`

**Interfaces:**
- Consumes: `render_screen_frame_with_layout`, `ScreenFrameLayout`, `render_scrollable_list_panel -> ListPanelAreas`, `ListPanelContent`, `render_action_panel`, existing `render_detail_panel`, `fit_rect`, `TextInput`, `DetailPanelState`, and `ViewportState`.
- Produces: a backward-compatible configured ScreenFrame path; one Quotes render path with `quote_list_content`, `quote_preview_rect`, and `action_content`; structural lint requiring executable Quotes delegation.

**Approved scope correction:** The immutable Browse fixture exposed 917 styled-cell differences with the original planned three-row frame. A Quotes-local trailing-flex diagnostic reduced this to 144 footer-cell differences; a bounded-sub-rectangle diagnostic achieved parity but was rejected because it retained legacy footer math in Quotes. The approved implementation adds generic shared ownership with exactly `ScreenFrameLayout { min_body_height: u16, trailing_flex: bool }`. Quotes must not retain or derive the old six-row layout.

- [ ] **Step 0: Commit the approved footer-preservation correction**

Update this plan and the approved design before product code, then commit only those records:

```bash
git add docs/superpowers/specs/2026-08-12-tui-consistency-batch-3b-quotes-components-design.md \
  docs/superpowers/plans/2026-08-13-tui-consistency-batch-3b-quotes-components.md
git commit -m "docs: preserve Quotes footer placement"
```

- [ ] **Step 0a: Extend ScreenFrame through RED/GREEN**

Add `ScreenFrameLayout` with exactly `min_body_height` and `trailing_flex`, plus `render_screen_frame_with_layout(...)`. When trailing flex is enabled, allocate body/status/shortcuts/trailing flex in that order; otherwise preserve the current three-row layout. Keep `render_screen_frame(...)` as the default wrapper. Tests must prove default layout/output remains unchanged, configured `80x24` placement puts status at row 12 and shortcuts at row 13 for the accepted Browse geometry, and zero/tiny configured targets stay inside the frame.

- [ ] **Step 1: Transfer Quotes border ownership in lint and record RED**

Change the direct-border list to exclude `quotes_screen.rs`:

```rust
let required_screens = ["goals.rs", "trends.rs", "history.rs", "log_entry.rs"];
```

Replace the Practices-only delegation assertion with:

```rust
for (filename, screen_name) in [
    ("practices.rs", "Practices"),
    ("quotes_screen.rs", "Quotes"),
] {
    let source = std::fs::read_to_string(tui_dir.join(filename)).unwrap();
    assert!(
        source_code_contains(&source, "render_scrollable_list_panel("),
        "{screen_name} must delegate its bordered list to ScrollableListPanel"
    );
}
```

Generalize the component message to:

```rust
"ScrollableListPanel must own the delegated bordered-list contract"
```

Run:

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
```

Expected: FAIL because the old Quotes renderer does not call `render_scrollable_list_panel`.

- [ ] **Step 2: Add a failing preview-containment test**

In Quotes tests, add:

```rust
fn rect_is_within(parent: ratatui::layout::Rect, child: ratatui::layout::Rect) -> bool {
    child.x >= parent.x
        && child.y >= parent.y
        && child.right() <= parent.right()
        && child.bottom() <= parent.bottom()
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
```

Run:

```bash
cargo test tui::quotes_screen::tests::quote_preview_rect_stays_inside_list_inner -- --exact --nocapture
```

Expected: compilation fails because `quote_preview_rect` is not defined.

- [ ] **Step 3: Add domain-row and preview geometry helpers**

Update imports to remove local list/frame rendering types and include the shared components. Retain `Clear`, `Line`, `Span`, styles, layout constraints, and preview helpers.

Add outside the `impl`:

```rust
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
    let x = inner.x + 4.min(inner.width);
    let width = quote_preview_width(inner);
    let height = if focused {
        inner.height
    } else {
        (content_height as u16 + 2).max(4).min(inner.height / 2)
    };
    fit_rect(
        inner,
        ratatui::layout::Rect {
            x,
            y: if focused {
                inner.y
            } else {
                inner.y + inner.height.saturating_sub(height)
            },
            width,
            height,
        },
    )
}

fn quote_preview_width(inner: ratatui::layout::Rect) -> u16 {
    let x = inner.x + 4.min(inner.width);
    inner
        .width
        .saturating_sub(8)
        .max(30)
        .min(inner.right().saturating_sub(x))
}
```

This preserves the current preview math but binds it to the returned safe `inner` rectangle.

- [ ] **Step 4: Extract existing action formatting without changing copy or style**

Add this private method:

```rust
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
```

Do not change `[any]`, action spacing, colors, or line count.

- [ ] **Step 5: Replace the render path with the three shared components**

Replace `QuotesScreen::render` with this structure:

```rust
pub fn render(&mut self, frame: &mut Frame) {
    let shortcuts = self.active_shortcuts();
    let action_height = match &self.mode {
        Mode::Browse | Mode::Detail => 0,
        Mode::Add | Mode::Edit | Mode::ConfirmDelete => 3,
    };
    let area = frame.area();
    let screen = render_screen_frame_with_layout(
        frame,
        area,
        CONTENT_WIDTH,
        &self.status_msg,
        &shortcuts,
        ScreenFrameLayout {
            min_body_height: 5 + action_height,
            trailing_flex: true,
        },
    );
    let body = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(1),
        Constraint::Length(action_height),
    ])
    .split(screen.body);

    let quotes = &self.quotes;
    let selected = self.selected;
    let list = render_scrollable_list_panel(
        frame,
        body[0],
        tr("dashboard-quotes"),
        &mut self.list_viewport,
        |inner| quote_list_content(quotes, selected, inner),
    );

    if matches!(self.mode, Mode::Browse | Mode::Detail) {
        if let Some(quote) = self.quotes.get(self.selected) {
            let focused = self.mode == Mode::Detail;
            let preview_width = quote_preview_width(list.inner)
                .saturating_sub(2)
                .max(1);
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

    render_action_panel(frame, body[2], self.action_content(screen.area.width), None);
}
```

`quote_preview_width` is the single source of truth for the old popup width. `visual_spans` receives that width minus the detail border, and `quote_preview_rect` receives the resulting measured content height. Do not duplicate or approximate this calculation.

Delete the superseded local frame/list/action render code. Do not touch `handle_key`, mutation ordering, `refresh`, or `Reenter` except for test-only access.

- [ ] **Step 6: Run the structural lint and fixture tests for GREEN**

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
cargo test tui::quotes_screen::tests::quote_preview_rect_stays_inside_list_inner -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
git diff --exit-code HEAD -- tests/fixtures/tui/quotes
```

Expected: lint passes, geometry passes in library and binary targets, all six fixtures are exact, and fixture diff is empty. If any fixture differs, stop and diagnose; do not regenerate it.

- [ ] **Step 7: Add tiny-terminal and retry-state regressions**

Add:

```rust
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
```

Add the two remaining retry regressions:

```rust
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
```

Prove the tests bind the existing retention behavior with these temporary mutations, one at a time:

1. Clear `self.input` in the create/update error arm; the write-retention test must fail.
2. Set `self.mode = Mode::Browse` in the delete error arm; the confirmation test must fail.
3. Omit `self.last_deleted = Some(quote)` in the restore error arm; the undo test must fail.

Revert each mutation immediately, rerun the focused test for GREEN, and leave the real production handlers behaviorally unchanged.

- [ ] **Step 8: Run all screen and framework regressions**

```bash
cargo test tui::quotes_screen::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
git diff --check
```

Expected: all commands exit zero and Clippy emits no warnings.

- [ ] **Step 9: Audit duplicate presentation mechanics**

```bash
rg -n "Block::default|Borders::ALL|Padding::uniform|highlight_row|render_status_line|render_shortcuts|shortcut_height|Paragraph::new|list_viewport\.(reveal|overflow)" \
  src/tui/quotes_screen.rs
rg -n "render_screen_frame|render_scrollable_list_panel|render_action_panel|render_detail_panel" \
  src/tui/quotes_screen.rs
rg -n "\.borders\(Borders::ALL\)|viewport\.(reveal|clamp|overflow)|highlight_row" \
  src/tui/framework/list_panel.rs
```

Expected:

- the first command has no superseded Quotes frame/list/action rendering matches;
- the second finds all three shared components plus the intentionally retained detail helper; and
- the third finds centralized list presentation behavior.

`list_viewport.clamp` remains permitted in Quotes refresh/re-entry lifecycle code; it is not a duplicate render implementation.

- [ ] **Step 10: Commit Quotes migration and lint transfer**

```bash
git add src/tui/quotes_screen.rs src/tui/framework/screen_frame.rs src/tui/framework/mod.rs tests/tui_lint_test.rs
git commit -m "refactor(quotes): adopt shared presentation components"
```

Do not include fixture, documentation, localization, database, router, or other screen changes in this commit.

---

### Task 4: Document, Verify, Review, and Hand Off Quotes-Only

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/quotes-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed Quotes migration and its tests.
- Produces: accurate shipped-state documentation, a fully verified branch, and a Quotes-only manual-acceptance binary.

- [ ] **Step 1: Update active documentation precisely**

Make these semantic changes and no broader historical rewrites:

- `CLAUDE.md`: state that Practices and Quotes consume `ScreenFrame`, `ScrollableListPanel`, and `ActionPanel`; Quotes still owns its floating-preview and Detail policy and uses the existing detail helper.
- `README.md`: change “Batch 3B-1 migrates Practices presentation only” to “Batch 3B-1 Practices and Batch 3B-2 Quotes presentation migrations are shipped; Trends and later slices are not shipped.”
- `README_zh.md`: add the exact Chinese equivalent without implying Trends or other screens migrated.
- `docs/specs/quotes-spec.md`: document the three component ownership boundaries, returned safe list geometry, and unchanged floating-preview/mode/error/undo behavior.
- `docs/specs/navigation-spec.md`: state Batch 3B-2 changes only Quotes presentation composition and does not change routes, intents, re-entry, or shortcuts.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`: add a completed Batch 3B-2 Quotes section and leave Trends, History, Goals popups, Dashboard responsive layout, and Log Entry forms explicitly unshipped.

Do not edit locale files or user-facing Quotes copy.

- [ ] **Step 2: Run documentation and removal audits**

```bash
rg -n "Practices is the only|Batch 3B-1 migrates Practices presentation only|later Batch 3B slices are not shipped" \
  CLAUDE.md README.md README_zh.md docs/specs docs/superpowers/specs
rg -n "Batch 3B-2|Quotes" \
  CLAUDE.md README.md README_zh.md docs/specs/quotes-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: stale active shipped-state claims are gone; new claims identify only Practices and Quotes; removed-feature guard passes.

- [ ] **Step 3: Run final fresh release gates**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::framework:: -- --nocapture
cargo test tui::quotes_screen::tests -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --exit-code HEAD -- tests/fixtures/tui/quotes tests/fixtures/tui/practices
git diff --check ae2d742..HEAD
git diff --check
```

Expected: every command exits zero; Clippy emits no warnings; all six Quotes and five Practices fixtures remain unchanged; the full suite and build pass.

- [ ] **Step 4: Perform scoped pre-commit self-review**

```bash
git diff --stat
git diff -- src/tui/quotes_screen.rs src/tui/framework/list_panel.rs \
  src/tui/framework/mod.rs tests/tui_lint_test.rs
git diff -- CLAUDE.md README.md README_zh.md docs/specs/quotes-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git status --short --untracked-files=all
```

Confirm:

- no fixture changed after Task 1;
- no Quotes handler, mutation ordering, undo, lifecycle, intent, shortcut, copy, or locale changed;
- preview positioning uses only returned list geometry;
- shared components import no Quotes or database types;
- old Quotes frame/list/action rendering is gone;
- `render_detail_panel` remains Quotes-owned integration rather than a new popup abstraction;
- every variable-length Paragraph remains wrapped;
- lint requires executable Practices and Quotes delegation and retains direct-border requirements for unmigrated screens; and
- docs identify Trends and later slices as unshipped.

- [ ] **Step 5: Commit active documentation**

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/quotes-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Quotes component migration"
```

- [ ] **Step 6: Request a final whole-slice read-only review**

Review `ae2d742..HEAD` against the approved design and this plan. Require explicit findings by Critical, Important, and Minor severity for:

- completeness and immutability of all six Quotes fixtures;
- exact output parity and unchanged Practices fixtures;
- `ListPanelAreas` containment on normal and tiny targets;
- component/domain ownership boundaries;
- preview measurement, placement, focus, scrolling, and overflow;
- empty, narrow, short, and tiny rendering;
- mutation failure retention, delete/undo behavior, and re-entry;
- recursive Paragraph lint and executable delegated-border lint accuracy;
- absence of duplicate Quotes frame/list/action mechanics;
- four-commit planned scope and documentation accuracy; and
- confirmation that Trends and all other screens are unchanged.

Resolve every valid Critical or Important finding with a focused failing regression first. Keep review fixes in separate commits and rerun every release gate in Step 3. Resolve Minor findings that are safe and in scope; otherwise record an explicit user-approved deferral before handoff.

- [ ] **Step 7: Confirm the final handoff state**

```bash
git status --short --untracked-files=all
git log --oneline ae2d742..HEAD
test -x target/debug/iron
```

Expected: tracked status is clean, commits match the approved slice plus any separately reviewed fixes, and the binary exists.

- [ ] **Step 8: Provide Quotes-only manual acceptance**

Give the user the built binary and ask them to verify:

1. Browse selection, scrolling, truncation, overflow markers, and complete floating preview.
2. Detail focus, `j/k` and arrows, overflow, and `Esc`/`Enter` return.
3. Add input, Unicode cursor visibility, save, cancel, empty-input error, and retry state.
4. Edit prefill, cursor/editing, save, cancel, and database failure retention.
5. ConfirmDelete copy, `y` confirmation, any-key cancellation, successful deletion, and undo.
6. Empty Quotes, long content, normal/narrow/short/tiny windows, and live resizing.
7. Status lines, mode-specific shortcuts, optional `[u] Undo`, and wrapped footer rows.
8. Dashboard-to-Quotes navigation, return, external-data reload, preserved Detail context, and atomic failure behavior.

Stop after this handoff. Do not design or implement Trends until the user explicitly accepts Quotes.
