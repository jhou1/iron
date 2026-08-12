# TUI Consistency Batch 3B-1 Practices Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract reusable screen-frame, scrollable-list, and action-panel presentation components and migrate Practices without changing its rendered output or behavior.

**Architecture:** Practices continues to own domain data, modes, input, validation, database operations, shortcuts, and domain-line formatting. Three statically dispatched Ratatui helpers own only shared layout and rendering mechanics. Full styled-buffer fixtures freeze the five Practices modes before extraction and must pass unchanged after migration.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Fluent localization, the existing TUI framework, Cargo tests, Clippy, and TestBackend.

## Global Constraints

- Base implementation commit is `b2c55912b41fd71f67991a9197a60b93a68a4f6c`; approved design commit is `d823731` on `codex/tui-consistency`.
- Follow [the approved Batch 3B-1 design](../specs/2026-08-12-tui-consistency-batch-3b-practices-components-design.md).
- Batch 3B-1 migrates Practices only. Quotes, Trends, History, Goals, Dashboard, and Log Entry remain unchanged.
- Preserve Practices output and shortcuts as close to byte-for-byte unchanged as Ratatui permits. The five `80x24` styled-buffer fixtures may not change during migration.
- Practices retains ownership of database access, stable IDs, modes, validation, localized status creation, key handling, shortcut selection, and domain formatting.
- Framework components own only layout, borders, generic viewport behavior, generic highlighting, clipping, status rendering, and shortcut rendering.
- Components do not import database types, domain models, navigation intents, or screen modes and do not return `anyhow` or business errors.
- All rectangle arithmetic is saturating or clamped. Empty, zero-sized, narrow, and short rectangles must not panic or escape the parent/frame.
- Failed Practices mutations retain the active mode, input, type selection, selected identity, and visible error exactly as they do before migration.
- Do not change database schema, localization copy, navigation topology, colors, borders, padding, spacing, or keyboard commands.
- Do not add speculative configuration for later Batch 3B screen migrations.
- Do not reintroduce Quick Log, LLM configuration/runtime, Abbreviations, or the `w` action.
- Every production change follows red-green-refactor. Capture the expected failure before implementing the corresponding behavior.
- Use exactly three planned commits: characterization fixtures, framework components, then Practices migration plus active documentation.
- The approved TUI lint change transfers border ownership semantically: `practices.rs` must delegate to `render_scrollable_list_panel`, `framework/list_panel.rs` must own `.borders(Borders::ALL)`, and unmigrated screens retain their direct-border rule.
- Paragraph wrapping lint remains strict. Its source discovery must include nested `src/tui/framework/*.rs` files after the new components are added.
- A line-number allowlist entry may change only after confirming that the old and new lines are the same approved widget.
- Stop for a scope ruling if an existing test and a characterization fixture expose a current defect, if a fixture would need to change, or if a tracked file outside this plan is required.

## File Structure

**Create:**

- `src/tui/framework/screen_frame.rs` — centered content frame, dynamic status/footer reservation, and status/shortcut rendering.
- `src/tui/framework/list_panel.rs` — bordered list shell, fixed header, overflow title, viewport reveal/clamp, visible rows, and selected-row highlighting.
- `src/tui/framework/action_panel.rs` — bounded contextual line rendering and optional selected-line highlighting.
- `tests/fixtures/tui/practices/browse.txt` — complete `80x24` styled buffer for Browse.
- `tests/fixtures/tui/practices/add_name.txt` — complete `80x24` styled buffer for AddName.
- `tests/fixtures/tui/practices/add_type.txt` — complete `80x24` styled buffer for AddType.
- `tests/fixtures/tui/practices/edit_name.txt` — complete `80x24` styled buffer for EditName.
- `tests/fixtures/tui/practices/confirm_delete.txt` — complete `80x24` styled buffer for ConfirmDelete.

**Modify:**

- `src/tui/framework/test_support.rs` — reusable rendered `Buffer` capture and deterministic full-cell serialization.
- `src/tui/framework/mod.rs` — declare and re-export the three presentation components.
- `src/tui/practices.rs` — characterization tests, domain-line builders, and the component-based render path.
- `tests/tui_lint_test.rs` — recursive Paragraph scanning plus the approved delegated-border contract.
- `CLAUDE.md` — shipped framework component ownership and Practices-only Batch 3B-1 status.
- `README.md` — Batch 3B-1 Practices status without claiming other migrations.
- `README_zh.md` — Chinese equivalent of the same shipped status.
- `docs/specs/practices-spec.md` — Practices rendering ownership and unchanged behavioral contract.
- `docs/specs/navigation-spec.md` — clarify that Batch 3B-1 changes presentation ownership only, not navigation.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — mark Practices component extraction complete and later Batch 3B slices unshipped.

**Do not modify unless a verified line move requires it:**

- the Paragraph allowlist literals in `tests/tui_lint_test.rs`; the semantic lint implementation is planned, but unrelated allowlist mappings are not.

---

### Task 1: Characterize All Practices Modes with Full Styled Buffers

**Files:**
- Modify: `src/tui/framework/test_support.rs`
- Modify: `src/tui/practices.rs:583-756`
- Create: `tests/fixtures/tui/practices/browse.txt`
- Create: `tests/fixtures/tui/practices/add_name.txt`
- Create: `tests/fixtures/tui/practices/add_type.txt`
- Create: `tests/fixtures/tui/practices/edit_name.txt`
- Create: `tests/fixtures/tui/practices/confirm_delete.txt`

**Interfaces:**
- Consumes: Ratatui `TestBackend`, current `PracticesScreen::render`, and private Practices test access to `Mode`, `input`, `type_selected`, `selected`, and `status_msg`.
- Produces: `render_to_buffer`, `buffer_snapshot`, and `render_to_snapshot` test helpers plus five immutable baseline fixtures used by Task 3.

- [ ] **Step 1: Add exact rendered-buffer helpers**

Refactor `src/tui/framework/test_support.rs` so the existing text helper is preserved and full style state is serializable:

```rust
use std::fmt::Write;

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

pub fn render_to_buffer(
    width: u16,
    height: u16,
    mut render: impl FnMut(&mut ratatui::Frame<'_>),
) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| render(frame))
        .expect("test frame should render");
    terminal.backend().buffer().clone()
}

pub fn buffer_snapshot(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut output = format!(
        "area\tx={}\ty={}\twidth={}\theight={}\n",
        area.x, area.y, area.width, area.height
    );
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let cell = &buffer[(x, y)];
            writeln!(
                output,
                "{x},{y}\t{:?}\t{:?}\t{:?}\t{:?}\t{}",
                cell.symbol(), cell.fg, cell.bg, cell.modifier, cell.skip
            )
            .expect("writing to a String should succeed");
        }
    }
    output
}

pub fn render_to_snapshot(
    width: u16,
    height: u16,
    render: impl FnMut(&mut ratatui::Frame<'_>),
) -> String {
    buffer_snapshot(&render_to_buffer(width, height, render))
}
```

Reimplement `render_to_string` by calling `render_to_buffer` and keep its current row trimming and newline behavior exactly. Run:

```bash
cargo test tui::framework::components::tests -- --nocapture
```

Expected: all existing framework component tests pass; this helper refactor changes no rendered output.

- [ ] **Step 2: Add deterministic five-mode fixture construction**

In the Practices test module, import `render_to_snapshot` and add a helper that gives every mode the same deterministic database rows, selected inactive row, status, and locale:

```rust
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
```

Keep all five cases in one test so locale setup and fixture generation are serialized within one test function.

- [ ] **Step 3: Write the fixture assertion and verify RED**

Create the five fixture files with one newline each, then add this temporary characterization test:

```rust
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
        let path = format!(
            "{}/tests/fixtures/tui/practices/{name}.txt",
            env!("CARGO_MANIFEST_DIR")
        );
        if std::env::var_os("UPDATE_PRACTICES_GOLDENS").is_some() {
            std::fs::write(&path, &actual).expect("golden fixture should be written");
        }
        let expected = std::fs::read_to_string(&path).expect("golden fixture should exist");
        assert_eq!(actual, expected, "Practices {name} buffer changed");
    }
}
```

Run:

```bash
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
```

Expected: FAIL on `browse` because the empty fixture does not equal the current `80x24` rendered buffer. Record the failing assertion before generating fixtures.

- [ ] **Step 4: Capture the accepted pre-refactor buffers**

Run the same current implementation with the temporary update switch:

```bash
UPDATE_PRACTICES_GOLDENS=1 cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
```

Expected: PASS and all five fixture files contain one header plus exactly `80 * 24` cell records. Check:

```bash
wc -l tests/fixtures/tui/practices/*.txt
```

Expected: each file reports `1921` lines.

- [ ] **Step 5: Remove the update path and make fixtures immutable**

Replace runtime writing/reading with compile-time fixture selection:

```rust
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
```

The final test computes `actual` exactly as before and uses:

```rust
assert_eq!(
    actual,
    practices_golden(name),
    "Practices {name} buffer changed"
);
```

Verify that `UPDATE_PRACTICES_GOLDENS` and `std::fs::write` no longer occur:

```bash
rg -n "UPDATE_PRACTICES_GOLDENS|std::fs::write" src/tui/practices.rs
```

Expected: no matches.

- [ ] **Step 6: Run the characterization gates**

```bash
cargo test tui::practices::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
git diff --check
```

Expected: every command passes. Inspect the fixtures and confirm they contain non-default foreground/background/modifier values, proving this is not a symbol-only snapshot.

- [ ] **Step 7: Commit the characterization baseline**

```bash
git add src/tui/framework/test_support.rs src/tui/practices.rs tests/fixtures/tui/practices
git commit -m "test(practices): capture mode rendering"
```

---

### Task 2: Add ScreenFrame, ScrollableListPanel, and ActionPanel

**Files:**
- Create: `src/tui/framework/screen_frame.rs`
- Create: `src/tui/framework/list_panel.rs`
- Create: `src/tui/framework/action_panel.rs`
- Modify: `src/tui/framework/mod.rs`

**Interfaces:**
- Consumes: `centered_area`, `fit_rect`, `render_status_line`, `render_shortcuts`, `shortcut_height`, `highlight_row`, `Shortcut`, `StatusMessage`, `ViewportState`, and `VisualSpan`.
- Produces:
  - `ScreenFrameAreas { area, body, status, shortcuts }`
  - `render_screen_frame(frame, full, max_width, status, shortcuts) -> ScreenFrameAreas`
  - `ListPanelContent { header, rows, empty, selected }`
  - `render_scrollable_list_panel(frame, area, title, viewport, build_content)`
  - `render_action_panel(frame, area, lines, selected)`

- [ ] **Step 1: Write failing ScreenFrame tests**

Create `src/tui/framework/screen_frame.rs` with tests that refer to the not-yet-defined API:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};

    #[test]
    fn screen_frame_reserves_wrapped_shortcuts_and_status() {
        let shortcuts = vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("Esc", "Back"),
        ];
        let mut areas = None;
        let output = render_to_string(20, 6, |frame| {
            let area = frame.area();
            areas = Some(render_screen_frame(
                frame,
                area,
                20,
                &Some(("Saved".into(), false)),
                &shortcuts,
            ));
        });

        assert_eq!(
            areas.unwrap(),
            ScreenFrameAreas {
                area: Rect::new(0, 0, 20, 6),
                body: Rect::new(0, 0, 20, 3),
                status: Rect::new(0, 3, 20, 1),
                shortcuts: Rect::new(0, 4, 20, 2),
            }
        );
        assert!(output.contains("Saved"), "{output}");
        assert!(output.contains("[j/k] Navigate"), "{output}");
        assert!(output.contains("[Esc] Back"), "{output}");
    }

    #[test]
    fn screen_frame_zero_target_stays_inside_frame() {
        let mut areas = None;
        render_to_buffer(2, 2, |frame| {
            areas = Some(render_screen_frame(
                frame,
                Rect::new(1, 1, 0, 0),
                107,
                &None,
                &[],
            ));
        });
        assert_eq!(areas.unwrap().area, Rect::new(1, 1, 0, 0));
    }
}
```

Declare `pub mod screen_frame;` temporarily in `framework/mod.rs`, then run:

```bash
cargo test tui::framework::screen_frame::tests -- --nocapture
```

Expected: compilation fails because `ScreenFrameAreas` and `render_screen_frame` do not exist.

- [ ] **Step 2: Implement ScreenFrame minimally**

Implement this public boundary in `screen_frame.rs`:

```rust
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::tui::{centered_area, framework::fit_rect};

use super::{
    render_shortcuts, render_status_line, shortcut_height, Shortcut, StatusMessage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenFrameAreas {
    pub area: Rect,
    pub body: Rect,
    pub status: Rect,
    pub shortcuts: Rect,
}

pub fn render_screen_frame(
    frame: &mut Frame,
    full: Rect,
    max_width: u16,
    status: &StatusMessage,
    shortcuts: &[Shortcut],
) -> ScreenFrameAreas {
    let full = fit_rect(frame.area(), full);
    let area = centered_area(full, max_width);
    let rows = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(shortcut_height(shortcuts, area.width)),
    ])
    .split(area);
    let result = ScreenFrameAreas {
        area,
        body: rows[0],
        status: rows[1],
        shortcuts: rows[2],
    };
    render_status_line(frame, result.status, status);
    render_shortcuts(frame, result.shortcuts, shortcuts);
    result
}
```

Run the focused tests again. Expected: both pass.

- [ ] **Step 3: Write failing ScrollableListPanel tests**

Create `src/tui/framework/list_panel.rs`, declare it in `framework/mod.rs`, and add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};
    use ratatui::style::Color;

    #[test]
    fn list_panel_keeps_header_fixed_and_reveals_selected_row() {
        let mut viewport = ViewportState::new(0);
        let output = render_to_string(24, 8, |frame| {
            let area = frame.area();
            render_scrollable_list_panel(
                frame,
                area,
                "Items",
                &mut viewport,
                |_| ListPanelContent {
                    header: Some(Line::from("  Name")),
                    rows: (1..=6)
                        .map(|number| Line::from(format!("  Item {number}")))
                        .collect(),
                    empty: Some(Line::from("No items")),
                    selected: Some(5),
                },
            );
        });

        assert_eq!(viewport.offset(), 3);
        assert!(output.contains("Items ↑"), "{output}");
        assert!(output.contains("Name"), "{output}");
        assert!(output.contains("Item 4"), "{output}");
        assert!(output.contains("Item 6"), "{output}");
        assert!(!output.contains("Item 1"), "{output}");
    }

    #[test]
    fn list_panel_highlights_only_the_visible_selected_data_row() {
        let mut viewport = ViewportState::new(0);
        let buffer = render_to_buffer(24, 8, |frame| {
            let area = frame.area();
            render_scrollable_list_panel(
                frame,
                area,
                "Items",
                &mut viewport,
                |_| ListPanelContent {
                    header: Some(Line::from("Header")),
                    rows: (1..=6)
                        .map(|number| Line::from(format!("Item {number}")))
                        .collect(),
                    empty: None,
                    selected: Some(5),
                },
            );
        });

        assert_eq!(buffer[(2, 5)].bg, Color::Rgb(204, 163, 0));
        assert_ne!(buffer[(2, 2)].bg, Color::Rgb(204, 163, 0));
    }

    #[test]
    fn list_panel_handles_empty_and_zero_sized_targets() {
        let mut viewport = ViewportState::new(9);
        render_to_buffer(2, 2, |frame| {
            render_scrollable_list_panel(
                frame,
                Rect::new(1, 1, 0, 0),
                "Empty",
                &mut viewport,
                |_| ListPanelContent {
                    header: Some(Line::from("Header")),
                    rows: Vec::new(),
                    empty: Some(Line::from("No items")),
                    selected: None,
                },
            );
        });
        assert_eq!(viewport.offset(), 0);
    }
}
```

Run:

```bash
cargo test tui::framework::list_panel::tests -- --nocapture
```

Expected: compilation fails because `ListPanelContent` and `render_scrollable_list_panel` do not exist.

- [ ] **Step 4: Implement ScrollableListPanel minimally**

Implement the following ownership boundary. The content closure receives the real bordered inner rectangle so the screen can format its domain columns at the actual width:

```rust
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::tui::{framework::fit_rect, highlight_row, BORDER_COLOR};

use super::{ViewportState, VisualSpan};

pub struct ListPanelContent<'a> {
    pub header: Option<Line<'a>>,
    pub rows: Vec<Line<'a>>,
    pub empty: Option<Line<'a>>,
    pub selected: Option<usize>,
}

pub fn render_scrollable_list_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> ListPanelContent<'a>,
) {
    let area = fit_rect(frame.area(), area);
    let geometry = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1));
    let inner = geometry.inner(area);
    let content = build_content(inner);
    let header_height = usize::from(content.header.is_some());
    let visible_height = (inner.height as usize).saturating_sub(header_height);

    viewport.set_height(visible_height);
    if let Some(selected) = content.selected.filter(|index| *index < content.rows.len()) {
        viewport.reveal(VisualSpan {
            start: selected,
            height: 1,
        });
    }
    viewport.clamp(content.rows.len());
    let overflow = viewport.overflow(content.rows.len()).suffix();
    let title = title.into();
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled(
                format!("{title}{overflow}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
        ]))
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .border_style(Style::default().fg(BORDER_COLOR));
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    if let Some(header) = content.header {
        lines.push(header);
    }
    if content.rows.is_empty() {
        if let Some(empty) = content.empty {
            lines.push(empty);
        }
    } else {
        lines.extend(
            content
                .rows
                .iter()
                .skip(viewport.offset())
                .take(visible_height)
                .cloned(),
        );
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if let Some(selected) = content.selected.filter(|index| {
        *index >= viewport.offset()
            && *index < viewport.offset().saturating_add(visible_height)
            && *index < content.rows.len()
    }) {
        let row = header_height.saturating_add(selected - viewport.offset());
        highlight_row(frame, inner, row.min(u16::MAX as usize) as u16);
    }
}
```

If the exact Ratatui `Block::inner` behavior at a zero rectangle differs from the assumed result, preserve the interface and add an early empty-area branch that sets viewport height to zero, clamps to the built row count, and returns before rendering. Do not enlarge the rectangle.

Run the list-panel tests. Expected: all pass.

- [ ] **Step 5: Write failing ActionPanel tests**

Create `src/tui/framework/action_panel.rs`, declare it, and add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::framework::test_support::{render_to_buffer, render_to_string};
    use ratatui::{style::Color, text::Line};

    #[test]
    fn action_panel_renders_lines_and_selected_highlight() {
        let lines = vec![
            Line::from("Select type"),
            Line::from("  Weighted"),
            Line::from("  Bodyweight"),
        ];
        let buffer = render_to_buffer(24, 3, |frame| {
            let area = frame.area();
            render_action_panel(frame, area, lines.clone(), Some(2));
        });
        assert_eq!(buffer[(0, 2)].bg, Color::Rgb(204, 163, 0));
        assert_ne!(buffer[(0, 1)].bg, Color::Rgb(204, 163, 0));

        let output = render_to_string(24, 3, |frame| {
            let area = frame.area();
            render_action_panel(frame, area, lines.clone(), Some(2));
        });
        assert!(output.contains("Select type"), "{output}");
        assert!(output.contains("Bodyweight"), "{output}");
    }

    #[test]
    fn action_panel_zero_target_does_not_panic() {
        render_to_buffer(2, 2, |frame| {
            render_action_panel(
                frame,
                Rect::new(1, 1, 0, 0),
                vec![Line::from("hidden")],
                Some(0),
            );
        });
    }
}
```

Run:

```bash
cargo test tui::framework::action_panel::tests -- --nocapture
```

Expected: compilation fails because `render_action_panel` does not exist.

- [ ] **Step 6: Implement ActionPanel minimally**

```rust
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::{framework::fit_rect, highlight_row};

pub fn render_action_panel(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line<'_>>,
    selected: Option<usize>,
) {
    let area = fit_rect(frame.area(), area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    if let Some(selected) = selected.filter(|row| *row < area.height as usize) {
        highlight_row(frame, area, selected.min(u16::MAX as usize) as u16);
    }
}
```

Run the action-panel tests. Expected: both pass.

- [ ] **Step 7: Export only the proved interfaces**

Update `src/tui/framework/mod.rs`:

```rust
pub mod action_panel;
pub mod components;
pub mod input;
pub mod list_panel;
pub mod screen_frame;
#[cfg(test)]
pub mod test_support;
pub mod viewport;

pub use action_panel::render_action_panel;
pub use list_panel::{render_scrollable_list_panel, ListPanelContent};
pub use screen_frame::{render_screen_frame, ScreenFrameAreas};
```

Keep existing exports intact. Do not add component traits, builders, generic error types, or screen-specific options.

- [ ] **Step 8: Run component and regression gates**

```bash
cargo test tui::framework::screen_frame::tests -- --nocapture
cargo test tui::framework::list_panel::tests -- --nocapture
cargo test tui::framework::action_panel::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
git diff --check
```

Expected: all component tests pass and the pre-refactor Practices golden test remains unchanged and green. If Clippy reports the new public exports as unused before Task 3, use the same narrow documented `#[allow(unused_imports)]` pattern already present in `framework/mod.rs`; do not add dummy production consumers.

- [ ] **Step 9: Commit the framework components**

```bash
git add src/tui/framework/action_panel.rs src/tui/framework/list_panel.rs \
  src/tui/framework/screen_frame.rs src/tui/framework/mod.rs
git commit -m "refactor(tui): add shared screen presentation components"
```

---

### Task 3: Migrate Practices, Transfer Lint Ownership, and Document Batch 3B-1

**Files:**
- Modify: `src/tui/practices.rs:1-348,583-756`
- Modify: `tests/tui_lint_test.rs`
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/practices-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: the three Task 2 render functions and the five immutable Task 1 fixtures.
- Produces: one component-based Practices render path, recursive wrapping lint coverage, delegated border ownership, accurate shipped documentation, and a Practices-only manual-acceptance handoff.

- [ ] **Step 1: Change the border lint contract first and verify RED**

In `screens_must_have_bordered_blocks`, remove `practices.rs` from `required_screens`. Add explicit delegated checks:

```rust
let practices_source = std::fs::read_to_string(tui_dir.join("practices.rs")).unwrap();
assert!(
    practices_source.contains("render_scrollable_list_panel("),
    "Practices must delegate its bordered list to ScrollableListPanel"
);

let list_panel_source =
    std::fs::read_to_string(tui_dir.join("framework/list_panel.rs")).unwrap();
assert!(
    list_panel_source.contains(".borders(Borders::ALL)"),
    "ScrollableListPanel must own the Practices bordered-list contract"
);
```

Run:

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
```

Expected: FAIL because `practices.rs` does not yet call `render_scrollable_list_panel`.

- [ ] **Step 2: Make Paragraph lint recurse into framework files**

Add a focused source collector above the tests:

```rust
fn rust_sources(root: &Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .unwrap_or_else(|_| panic!("failed to read {}", directory.display()))
        {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}
```

Extract the existing violation scan into this helper so production lint and the recursion regression execute the same logic:

```rust
fn paragraph_violations(root: &Path, allowlist: &HashSet<&str>) -> Vec<String> {
    let mut violations = Vec::new();
    for path in rust_sources(root) {
        let relative = path.strip_prefix(root).unwrap().to_string_lossy();
        let source = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        for (line_num_0, line) in lines.iter().enumerate() {
            if !line.trim().contains("Paragraph::new(") {
                continue;
            }
            let end = (line_num_0 + 4).min(lines.len());
            if lines[line_num_0..end].join(" ").contains(".wrap(") {
                continue;
            }
            let line_num = line_num_0 + 1;
            let key = format!("{}:{}", relative, line_num);
            if !allowlist.contains(key.as_str()) {
                violations.push(format!(
                    "  {}:{} — Paragraph::new() without .wrap(). Add .wrap(Wrap {{ trim: false }}) or allowlist with reason.",
                    relative, line_num
                ));
            }
        }
    }
    violations
}
```

Replace the current one-level scan in `paragraph_widgets_must_wrap_user_content` with:

```rust
let violations = paragraph_violations(tui_dir, &allowlist);
```

Existing top-level allowlist strings remain unchanged because their relative paths are still `dashboard.rs` and `log_entry.rs`.

Add a self-test proving recursion, using a temporary nested source tree and a small extracted `paragraph_violations(root, allowlist)` helper rather than duplicating the scanner:

```rust
#[test]
fn paragraph_lint_scans_nested_framework_sources() {
    let temp = tempfile::tempdir().unwrap();
    let nested = temp.path().join("framework");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        nested.join("bad_panel.rs"),
        "fn render() { Paragraph::new(variable_content); }\n",
    )
    .unwrap();

    let violations = paragraph_violations(temp.path(), &HashSet::new());

    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("framework/bad_panel.rs:1")),
        "{violations:?}"
    );
}
```

Run:

```bash
cargo test --test tui_lint_test paragraph_lint_scans_nested_framework_sources -- --exact --nocapture
cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture
```

Expected: both pass; every new component Paragraph already uses `.wrap(Wrap { trim: false })`.

- [ ] **Step 3: Extract domain-only practice list formatting**

In `src/tui/practices.rs`, remove the render-only imports `Block`, `Borders`, `Padding`, and `Paragraph`. Keep `Wrap` only if a remaining non-component Paragraph needs it; otherwise remove it too. Import:

```rust
use ratatui::layout::Rect;

use super::framework::{
    render_action_panel, render_screen_frame, render_scrollable_list_panel, ListPanelContent,
};
```

Add a free helper outside `impl PracticesScreen` that formats all logical rows at the component-provided inner width:

```rust
fn practice_list_content(
    practices: &[Practice],
    selected: usize,
    inner: Rect,
) -> ListPanelContent<'static> {
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
        .saturating_sub(type_col_width)
        .saturating_sub(2);
    let name_header = truncate_to_width(&name_header, name_col_width);
    let type_header = truncate_to_width(&type_header, type_col_width);
    let header_padding = name_col_width.saturating_sub(name_header.width());
    let type_header_padding = type_col_width.saturating_sub(type_header.width());
    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(name_header, header_style),
        Span::raw(" ".repeat(header_padding)),
        Span::styled(type_header, header_style),
        Span::raw(" ".repeat(type_header_padding + 2)),
    ]);

    let rows = practices
        .iter()
        .enumerate()
        .map(|(index, practice)| {
            let marker = if index == selected { "> " } else { "  " };
            let (name_style, type_color) = if index == selected {
                if practice.active {
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
            } else if practice.active {
                (Style::default().fg(Color::White), Color::Gray)
            } else {
                (Style::default().fg(Color::DarkGray), Color::DarkGray)
            };
            let name = truncate_to_width(&practice.name, name_col_width);
            let name_padding = name_col_width.saturating_sub(name.width());
            let type_label =
                truncate_to_width(&practice.practice_type.label(), type_col_width);
            let type_padding = type_col_width.saturating_sub(type_label.width());
            let mut spans = vec![
                Span::styled(marker, name_style),
                Span::styled(name, name_style),
                Span::raw(" ".repeat(name_padding)),
                Span::styled(type_label, Style::default().fg(type_color)),
                Span::raw(" ".repeat(type_padding)),
            ];
            if practice.active {
                spans.push(Span::styled("◰", Style::default().fg(GREEN)));
                spans.push(Span::styled("◱", Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::styled("◱", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled("◰", Style::default().fg(Color::DarkGray)));
            }
            Line::from(spans)
        })
        .collect();

    ListPanelContent {
        header: Some(header),
        rows,
        empty: Some(Line::from(Span::styled(
            tr("practices-no-items"),
            Style::default().fg(Color::Gray),
        ))),
        selected: (!practices.is_empty()).then_some(selected),
    }
}
```

This helper formats domain columns and status glyphs but contains no viewport, border, overflow, clipping, or frame mutation.

- [ ] **Step 4: Extract action-line formatting without moving the state machine**

Add a private method that returns already-styled lines and an optional selected visual row:

```rust
fn action_content(&self, width: u16) -> (Vec<Line<'static>>, Option<usize>) {
    match &self.mode {
        Mode::Browse => (
            vec![Line::from(""), Line::from(""), Line::from(""), Line::from("")],
            None,
        ),
        Mode::AddName => {
            let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
            spans.extend(self.input.visible_spans(width, 3, GREEN));
            (
                vec![
                    Line::from(Span::styled(
                        tr("practices-new-name"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                    Line::from(""),
                    Line::from(""),
                ],
                None,
            )
        }
        Mode::AddType => {
            let mut lines = vec![Line::from(Span::styled(
                tr("practices-select-type"),
                Style::default().fg(Color::White),
            ))];
            for (index, practice_type) in PracticeType::ALL.iter().enumerate() {
                let marker = if index == self.type_selected { "> " } else { "  " };
                let style = if index == self.type_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                lines.push(Line::from(Span::styled(
                    format!(" {marker}{}", practice_type.label()),
                    style,
                )));
            }
            (lines, Some(self.type_selected + 1))
        }
        Mode::EditName => {
            let mut spans = vec![Span::styled(" > ", Style::default().fg(GREEN))];
            spans.extend(self.input.visible_spans(width, 3, GREEN));
            (
                vec![
                    Line::from(Span::styled(
                        tr("practices-rename"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(spans),
                    Line::from(""),
                    Line::from(""),
                ],
                None,
            )
        }
        Mode::ConfirmDelete => {
            let name = self
                .practices
                .get(self.selected)
                .map(|practice| practice.name.as_str())
                .unwrap_or("?");
            (
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
                ],
                None,
            )
        }
    }
}
```

Do not move `handle_key`, mutation functions, mode transitions, status assignment, or `Reenter`.

- [ ] **Step 5: Replace the Practices render path with the three components**

Replace `PracticesScreen::render` with:

```rust
pub fn render(&mut self, frame: &mut Frame) {
    let shortcuts = self.active_shortcuts();
    let area = frame.area();
    let screen = render_screen_frame(
        frame,
        area,
        CONTENT_WIDTH,
        &self.status_msg,
        &shortcuts,
    );
    let action_height = if self.mode == Mode::Browse { 0 } else { 6 };
    let body = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(1),
        Constraint::Length(action_height),
    ])
    .split(screen.body);

    let practices = &self.practices;
    let selected = self.selected;
    render_scrollable_list_panel(
        frame,
        body[0],
        tr("practices-title"),
        &mut self.list_viewport,
        |inner| practice_list_content(practices, selected, inner),
    );

    let (action_lines, selected_action) = self.action_content(screen.area.width);
    render_action_panel(frame, body[2], action_lines, selected_action);
}
```

Delete the superseded local block construction, list viewport reveal/clamp, visible-row slicing, overflow-title construction, `Paragraph` calls, and `highlight_row` calls from Practices. There must be one render path only.

- [ ] **Step 6: Verify exact fixture parity before any further edit**

Run:

```bash
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
```

Expected: PASS with all five fixture files byte-for-byte unchanged. Check:

```bash
git diff --exit-code -- tests/fixtures/tui/practices
```

Expected: no diff. If any fixture changes, stop and identify the exact cell/geometry difference; do not update the fixture.

- [ ] **Step 7: Add tiny-screen and long-name regression coverage**

Add these focused Practices tests:

```rust
#[test]
fn practices_all_modes_render_in_tiny_terminals_without_panicking() {
    for mode in [
        Mode::Browse,
        Mode::AddName,
        Mode::AddType,
        Mode::EditName,
        Mode::ConfirmDelete,
    ] {
        for (width, height) in [(1, 1), (12, 4), (24, 10)] {
            let mut screen = golden_screen(mode.clone());
            let _ = render_practices(&mut screen, width, height);
        }
    }
}

#[test]
fn practices_long_name_keeps_toggle_inside_the_bordered_panel() {
    let db = Database::open_in_memory().unwrap();
    db.create_practice(
        "一个很长的练习名称 mixed with a deliberately long ASCII suffix",
        PracticeType::Endurance,
    )
    .unwrap();
    let mut screen = PracticesScreen::new(&db).unwrap();

    let buffer = crate::tui::framework::test_support::render_to_buffer(40, 12, |frame| {
        screen.render(frame)
    });

    let data_row = (0..40)
        .map(|x| buffer[(x, 3)].symbol())
        .collect::<String>();
    assert_eq!(buffer[(39, 3)].symbol(), "│", "{data_row}");
    assert!(data_row.contains('◰') || data_row.contains('◱'), "{data_row}");
}
```

Run:

```bash
cargo test tui::practices::tests -- --nocapture
```

Expected: all Practices tests pass in both library and binary targets.

- [ ] **Step 8: Complete and verify the lint ownership transfer**

Run:

```bash
cargo test --test tui_lint_test -- --nocapture
```

Expected: recursive Paragraph scan, delegated Practices border, component border, and all unmigrated direct-border checks pass.

Audit the ownership explicitly:

```bash
rg -n "Block::default|Borders::ALL|Padding::uniform|highlight_row|Paragraph::new|list_viewport\.(reveal|clamp|overflow)" src/tui/practices.rs
rg -n "render_screen_frame|render_scrollable_list_panel|render_action_panel" src/tui/practices.rs
rg -n "borders\(Borders::ALL\)|viewport\.(reveal|clamp|overflow)|highlight_row" src/tui/framework/list_panel.rs
```

Expected:

- the first command has no matches from the superseded Practices render mechanics;
- the second finds all three component calls; and
- the third finds the centralized border, viewport, overflow, and highlight behavior.

- [ ] **Step 9: Update active documentation with Practices-only shipped status**

Make these exact semantic updates:

- `CLAUDE.md`: list `screen_frame.rs`, `list_panel.rs`, and `action_panel.rs` under the framework; state that Practices is the only Batch 3B-1 component consumer and still owns domain modes/database/validation/shortcuts.
- `README.md`: replace the blanket “Batch 3B UI component extraction is not included” statement with “Batch 3B-1 migrates Practices presentation only; other screens are unchanged and later slices are not shipped.”
- `README_zh.md`: add the exact Chinese equivalent without implying Quotes or other modules migrated.
- `docs/specs/practices-spec.md`: state that ScreenFrame reserves status/shortcuts, ScrollableListPanel owns border/overflow/viewport/highlight, and ActionPanel owns bounded action rendering; all lines and state decisions remain Practices-owned.
- `docs/specs/navigation-spec.md`: state that Batch 3B-1 does not change topology, intents, lifecycle, or shortcuts.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`: mark Batch 3B-1 Practices extraction complete; list Quotes, Trends, History, Goals popups, Dashboard responsive layout, and Log Entry forms as unshipped later work.

Do not claim that the three components are already suitable for every future screen. Do not change user-facing copy or localization files.

- [ ] **Step 10: Run final formatting, static analysis, test, and build gates**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --check d823731..HEAD
git diff --check
```

Expected: every command exits zero, Clippy emits no warnings, all five fixtures remain unchanged, and the full suite passes.

- [ ] **Step 11: Perform the scoped self-review before commit**

Review the complete diff and verify:

```bash
git diff --stat
git diff -- src/tui/practices.rs src/tui/framework tests/tui_lint_test.rs
git diff -- CLAUDE.md README.md README_zh.md docs/specs/practices-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git status --short --untracked-files=all
```

Confirm all of the following before staging:

- no fixture file changed in Task 3;
- no business handler, mutation ordering, lifecycle contract, navigation intent, locale string, or shortcut set changed;
- new components import no `Database`, `Practice`, `Mode`, `NavigationIntent`, or `anyhow`;
- the old Practices border/viewport/highlight/action Paragraph implementation is gone;
- every new Paragraph is wrapped;
- lint checks real ownership rather than a marker comment; and
- docs distinguish shipped Practices extraction from unshipped later slices.

- [ ] **Step 12: Commit Practices migration and documentation**

```bash
git add src/tui/practices.rs tests/tui_lint_test.rs CLAUDE.md README.md README_zh.md \
  docs/specs/practices-spec.md docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "refactor(practices): adopt shared presentation components"
```

- [ ] **Step 13: Request a final whole-slice read-only review**

Review `d823731..HEAD` against the approved design. Require explicit findings by Critical, Important, and Minor severity for:

- styled-buffer fixture completeness and immutability;
- exact five-mode output parity;
- component/domain ownership boundaries;
- tiny-rectangle containment and panic safety;
- fixed header, overflow, reveal, clamp, and highlight behavior;
- mutation failure retention and lifecycle behavior;
- recursive Paragraph lint and delegated border lint accuracy;
- absence of duplicate Practices render mechanics;
- three-commit scope and documentation accuracy; and
- confirmation that no other screen or removed feature changed.

Resolve every valid Critical or Important finding with a failing regression first, then rerun all gates in Step 10. Keep fixes in separately identified follow-up commits rather than rewriting recorded review history.

- [ ] **Step 14: Prepare the Practices-only manual acceptance handoff**

Provide the built binary and ask the user to verify:

1. Browse selection, list scrolling, inactive styling, and overflow markers.
2. AddName input and cursor behavior.
3. AddType selection and highlighting.
4. EditName prefill, editing, cancellation, and failure retention.
5. ConfirmDelete copy, warning, and `y/n` shortcuts.
6. Long names, empty lists, narrow/short/tiny resizing, and visible status/footer rows.
7. All existing Dashboard-to-Practices navigation and return behavior.

Stop after this handoff. Do not begin Quotes or any later Batch 3B slice until the user explicitly accepts Practices.
