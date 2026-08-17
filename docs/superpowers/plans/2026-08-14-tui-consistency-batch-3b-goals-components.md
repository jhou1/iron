# TUI Consistency Batch 3B-5 Goals Responsive Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Goals milestone popup with persistent responsive Goals/Milestones panes while migrating duplicated presentation mechanics to backward-compatible shared components.

**Architecture:** `ScreenFrame` owns the centered body/status/shortcut frame. A Goals-local pure helper splits that body into side-by-side panes at widths of 83 cells or more and stacked panes below 83 cells. A new span-aware `ScrollableListPanel` sibling renders multi-line Goal selections, while a new pinned-header `DetailPanel` sibling keeps aggregate progress fixed above independently scrolling milestones; both existing component APIs remain unchanged.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Chrono 0.4, Fluent localization, Unicode Width, the existing TUI framework, Cargo tests, Clippy, `syn`-based ownership lint, and Ratatui `TestBackend` styled-buffer fixtures.

## Global Constraints

- Base and approved design commit is `70dae368c096aff39f0dfc0eef7c46b1d6011d1b` on `main`.
- Follow [the approved Batch 3B-5 design](../specs/2026-08-14-tui-consistency-batch-3b-goals-components-design.md).
- Batch 3B-5 changes Goals presentation and the two explicitly approved backward-compatible framework sibling APIs only. Dashboard and Log Entry remain unshipped.
- Use an isolated worktree created through `superpowers:using-git-worktrees` before implementation.
- Goal progress remains binary milestone aggregation. Do not add a percentage field, schema migration, model field, export field, or manual percentage editor.
- At body width `>= 83`, use a one-cell horizontal gap, Goals width `max(34, floor((width - 1) * 2 / 5))` capped so Milestones retains at least 48 cells, and assign all remaining cells to Milestones.
- At body width `< 83`, subtract a one-row gap only when height is at least two, give Goals `floor(remaining * 2 / 5)` rows, and give every remaining row to Milestones.
- `80x24` must use stacked panes. The normal 107-cell content width must produce 42-cell Goals and 64-cell Milestones panes.
- The selected Goal gauge remains visible in the Goals pane. The selected Goal aggregate gauge is also a pinned Milestones header that never scrolls with milestone rows.
- Goals and Milestones retain separate selection, status, undo, and viewport state. Focus changes or resize must not recreate screen state.
- Rename all production popup/modal terminology to milestone-pane terminology: `Mode::Milestones`, `MilestoneMode`, `milestone_goal_idx`, `milestone_selected`, `milestone_status_msg`, `milestone_last_deleted`, and `milestone_panel_state`; split the shared editor into `goal_input` and `milestone_input`.
- `render_scrollable_list_panel` and `render_detail_panel` keep their existing public signatures and accepted behavior. Practices, Quotes, Trends, and History production files must not change.
- The new framework APIs import no `Goal`, `Milestone`, `Database`, navigation, lifecycle, undo, or business-error type and add no Cargo dependency.
- Use no generic `MasterDetail` or popup component.
- Goals rendering must contain no popup `Clear`, popup placement, local list border, local status renderer, local shortcut renderer, or custom footer `Paragraph` after migration.
- Preserve database operations, stable-ID reorder restoration, one-level Goal and milestone undo, navigation topology, atomic target preparation, cross-page re-entry, and failure retention.
- Delete failures must retain their Goal or milestone confirmation state; successful deletes may leave confirmation even when the subsequent refresh reports an error because the write cannot be retried safely.
- Shortcuts are handler-derived and contain every executable command and no inactive command. Use complete `Shortcut` key-label units so wrapping cannot split an item.
- All rectangle arithmetic is saturating or clipped. Zero-sized, one-cell, narrow, short, offset, maximum-coordinate, and resized rectangles must not escape or panic.
- Existing Practices, Quotes, Trends, and History fixtures are immutable from `70dae36..HEAD`.
- New Goals fixtures are captured only after the approved redesign passes semantic RED/GREEN tests. The temporary update path is removed before the fixture commit, and the fixtures are immutable from that commit onward.
- Every production change follows RED-GREEN-refactor. Record the exact expected failure before implementation.
- Structural ownership lint must inspect executable `GoalsScreen::render` AST. Comments, literals, tests, nested items, and dead helpers do not satisfy ownership.
- Paragraph lint remains recursive with its existing four-line scan rule. A same-widget line-number refresh requires an explicit mechanical scope ruling during execution.
- Stop for user direction if an accepted non-Goals fixture changes, an existing public component API must break, another screen needs production edits, persistence semantics conflict with the approved design, or a tracked path outside this plan becomes necessary.
- Use seven planned implementation commits. Review-discovered corrections use separate focused RED/GREEN commits.
- Stop after the Goals-only binary handoff. Do not begin Dashboard or Log Entry until the user manually accepts Goals.

## File Structure

**Create:**

- `tests/fixtures/tui/goals/wide_goal_focus.txt` — 107×24 side-by-side layout with Goals focus.
- `tests/fixtures/tui/goals/wide_milestones_focus.txt` — 107×24 side-by-side layout with Milestones focus.
- `tests/fixtures/tui/goals/narrow_goal_focus.txt` — 80×24 stacked layout with Goals focus.
- `tests/fixtures/tui/goals/narrow_milestones_focus.txt` — 80×24 stacked layout with Milestones focus.
- `tests/fixtures/tui/goals/goal_edit.txt` — inline Goal edit and active confirm/cancel shortcuts.
- `tests/fixtures/tui/goals/milestone_add.txt` — inline milestone add below the pinned progress header.
- `tests/fixtures/tui/goals/milestone_confirm_delete.txt` — milestone confirmation and active confirmation shortcuts.
- `tests/fixtures/tui/goals/overflow.txt` — independent Goal and milestone overflow with selected items revealed.
- `tests/fixtures/tui/goals/empty_goals.txt` — no Goal plus bounded Milestones empty-selection copy.
- `tests/fixtures/tui/goals/empty_milestones.txt` — selected Goal gauge plus add-first-milestone copy.

**Modify:**

- `src/tui/framework/list_panel.rs` — span-aware, focus-aware sibling API and focused tests; preserve the existing API.
- `src/tui/framework/components.rs` — pinned-header DetailPanel sibling API and focused tests; preserve the existing API.
- `src/tui/framework/mod.rs` — export the two new sibling APIs and their content type.
- `src/tui/goals.rs` — mutation-retention regressions, responsive geometry, state rename, component composition, active shortcuts, fixture builders, and all Goals behavior tests.
- `locales/en.ftl` — `goals-milestones-title` and `goals-no-selection`.
- `locales/zh-CN.ftl` — exact Chinese equivalents.
- `tests/tui_lint_test.rs` — AST ownership enforcement for the Goals production render path and removal from the direct-border group.
- `CLAUDE.md` — record Goals component ownership and responsive/persistent-pane behavior.
- `README.md` — mark Batch 3B-5 Goals shipped and Dashboard/Log Entry unshipped.
- `README_zh.md` — Chinese shipped-state equivalent.
- `docs/specs/goals-spec.md` — replace popup language with persistent responsive panes and pinned progress.
- `docs/specs/navigation-spec.md` — state that routes are unchanged while Goals focus layering changes locally.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — add the completed Batch 3B-5 slice and the two backward-compatible component APIs.

**Do not modify:**

- `src/tui/practices.rs`, `src/tui/quotes_screen.rs`, `src/tui/trends.rs`, `src/tui/history.rs`, `src/tui/dashboard.rs`, `src/tui/log_entry.rs`, database/model/router/app code, Cargo manifests, and existing fixture files.

---

### Task 1: Bind Layered Focus and Mutation-Failure Retention

**Files:**
- Modify: `src/tui/goals.rs:880-1170`
- Test: `src/tui/goals.rs` test module

**Interfaces:**
- Consumes: current `Mode`, `ModalMode`, `GoalsScreen::handle_key`, `Database::open`, and Rusqlite exclusive-lock behavior.
- Produces: green behavior tests that Task 5 mechanically renames, plus corrected delete-failure retention that the redesigned panes must preserve.

- [ ] **Step 1: Add a deterministic layered-Esc characterization**

Add this focused test before any state rename:

```rust
#[test]
fn goals_escape_layers_are_bound_before_panel_redesign() {
    let db = Database::open_in_memory().unwrap();
    db.create_goal("Goal").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();

    assert_eq!(screen.handle_key(key(KeyCode::Char('m')), &db), Action::None);
    assert_eq!(screen.mode, Mode::Modal);
    assert_eq!(screen.modal_mode, ModalMode::AddMilestone);

    assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
    assert_eq!(screen.mode, Mode::Modal);
    assert_eq!(screen.modal_mode, ModalMode::Browse);

    assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
    assert_eq!(screen.mode, Mode::Browse);

    assert_eq!(
        screen.handle_key(key(KeyCode::Esc), &db),
        Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard))
    );
}
```

- [ ] **Step 2: Add real Goal-delete failure retention RED**

Import `rusqlite::Connection` in the test module and add:

```rust
#[test]
fn failed_goal_delete_retains_confirmation_selection_and_viewport() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked-goals.db");
    let db = Database::open(&path).unwrap();
    let goal_id = db.create_goal("Keep goal").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.list_viewport.set_height(1);
    screen.list_viewport.set_offset(3);
    screen.handle_key(key(KeyCode::Char('d')), &db);
    let lock = Connection::open(&path).unwrap();
    lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

    screen.handle_key(key(KeyCode::Char('y')), &db);

    assert_eq!(screen.mode, Mode::ConfirmDeleteGoal);
    assert_eq!(screen.goals[screen.selected].id, goal_id);
    assert_eq!(screen.list_viewport.offset(), 3);
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));

    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(db.list_goals().unwrap().len(), 1);
}
```

- [ ] **Step 3: Run the Goal-delete test and record RED**

Run:

```bash
cargo test tui::goals::tests::failed_goal_delete_retains_confirmation_selection_and_viewport -- --exact --nocapture
```

Expected: FAIL because the current handler unconditionally returns to `Mode::Browse` after a failed delete.

- [ ] **Step 4: Add real milestone-delete failure retention RED**

Add:

```rust
#[test]
fn failed_milestone_delete_retains_confirmation_identity_and_viewports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked-milestones.db");
    let db = Database::open(&path).unwrap();
    let goal_id = db.create_goal("Keep goal").unwrap();
    let milestone_id = db.create_milestone(goal_id, "Keep milestone").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.handle_key(key(KeyCode::Enter), &db);
    screen.list_viewport.set_offset(2);
    screen.detail_state.viewport.set_offset(4);
    screen.handle_key(key(KeyCode::Char('d')), &db);
    let lock = Connection::open(&path).unwrap();
    lock.execute_batch("BEGIN EXCLUSIVE").unwrap();

    screen.handle_key(key(KeyCode::Char('y')), &db);

    assert_eq!(screen.mode, Mode::Modal);
    assert_eq!(screen.modal_mode, ModalMode::ConfirmDeleteMilestone);
    assert_eq!(screen.goals[screen.modal_goal_idx].id, goal_id);
    assert_eq!(
        screen.goals[screen.modal_goal_idx].milestones[screen.modal_selected].id,
        milestone_id
    );
    assert_eq!(screen.list_viewport.offset(), 2);
    assert_eq!(screen.detail_state.viewport.offset(), 4);
    assert!(screen
        .modal_status_msg
        .as_ref()
        .is_some_and(|(_, error)| *error));

    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        db.list_goals()
            .unwrap()
            .into_iter()
            .find(|goal| goal.id == goal_id)
            .unwrap()
            .milestones
            .len(),
        1
    );
}
```

- [ ] **Step 5: Run the milestone-delete test and record RED**

Run:

```bash
cargo test tui::goals::tests::failed_milestone_delete_retains_confirmation_identity_and_viewports -- --exact --nocapture
```

Expected: FAIL because the current handler unconditionally returns to `ModalMode::Browse` after a failed delete.

- [ ] **Step 6: Make confirmation exit conditional on a successful write**

In each delete-confirmation handler, initialize `delete_succeeded` to `false`. In the existing `Ok(())` arm, set it to `true` before the current undo assignment, reload, status, and index-clamp logic. In the `Err(error)` arm, leave it false and retain the current error status assignment. After the match, set `self.mode = Mode::Browse` only when Goal deletion succeeded; set `self.modal_mode = ModalMode::Browse` only when milestone deletion succeeded. A successful write exits confirmation even if its subsequent reload fails because repeating the write is unsafe. Non-`y` cancellation still exits confirmation immediately.

- [ ] **Step 7: Run focused and complete Goals tests**

```bash
cargo fmt --all
cargo test tui::goals::tests::failed_goal_delete_retains_confirmation_selection_and_viewport -- --exact --nocapture
cargo test tui::goals::tests::failed_milestone_delete_retains_confirmation_identity_and_viewports -- --exact --nocapture
cargo test tui::goals::tests::goals_escape_layers_are_bound_before_panel_redesign -- --exact --nocapture
cargo test tui::goals::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all tests pass in library and binary targets; strict Clippy reports no warnings.

- [ ] **Step 8: Commit the bound behavior**

```bash
git add src/tui/goals.rs
git commit -m "fix(goals): retain failed mutation context"
```

---

### Task 2: Add the Span-Aware ListPanel Sibling API

**Files:**
- Modify: `src/tui/framework/list_panel.rs`
- Modify: `src/tui/framework/mod.rs`

**Interfaces:**
- Consumes: existing `ListPanelAreas`, `ViewportState`, `VisualSpan`, `visual_spans`, `render_scrollable_lines`, `fit_rect`, `highlight_row`, and Ratatui `Line`.
- Produces:

```rust
pub struct SpannedListPanelContent {
    pub header: Option<Line<'static>>,
    pub lines: Vec<Line<'static>>,
    pub empty: Option<Line<'static>>,
    pub selected: Option<VisualSpan>,
}

pub fn render_scrollable_list_panel_spanned(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    focused: bool,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> SpannedListPanelContent,
) -> ListPanelAreas;
```

The existing `render_scrollable_list_panel` and `ListPanelContent` interfaces remain byte-for-byte source compatible.

- [ ] **Step 1: Add compile-time RED tests for the new interface**

In `list_panel.rs` tests, add calls to `render_scrollable_list_panel_spanned` using `SpannedListPanelContent`. Include a wrapped selection whose `VisualSpan` is computed with the closure's real `inner.width`:

```rust
#[test]
fn spanned_list_panel_reveals_and_highlights_the_complete_wrapped_selection() {
    let mut viewport = ViewportState::new(0);
    let buffer = render_to_buffer(24, 8, |frame| {
        render_scrollable_list_panel_spanned(
            frame,
            frame.area(),
            "Goals",
            true,
            &mut viewport,
            |inner| {
                let lines = vec![
                    Line::from("First"),
                    Line::from("Second"),
                    Line::from("Selected goal title that wraps"),
                    Line::from("████ 50%"),
                    Line::from("Last"),
                ];
                let spans = visual_spans(&lines, inner.width);
                SpannedListPanelContent {
                    header: None,
                    lines,
                    empty: None,
                    selected: combined_span(&spans, 2, 2),
                }
            },
        );
    });

    assert!(viewport.offset() > 0);
    assert!((0..8).any(|y| {
        (0..24).any(|x| buffer[(x, y)].bg == Color::Rgb(204, 163, 0))
    }));
}
```

Add a second test with `focused = false` that asserts the viewport still reveals the selected span but no cell receives the selected background. Add a third loop over zero-sized, one-cell, edge, and normal target rectangles and assert returned outer/inner areas remain contained.

- [ ] **Step 2: Run the new tests and record RED**

```bash
cargo test tui::framework::list_panel::tests::spanned_list_panel -- --nocapture
```

Expected: compilation fails because `SpannedListPanelContent` and `render_scrollable_list_panel_spanned` do not exist.

- [ ] **Step 3: Add the new content type and export**

Add the exact type above to `list_panel.rs`. Export both names in `framework/mod.rs`:

```rust
pub use list_panel::{
    render_scrollable_list_panel, render_scrollable_list_panel_spanned, ListPanelAreas,
    ListPanelContent, SpannedListPanelContent,
};
```

- [ ] **Step 4: Implement visual-row measurement without changing the old API**

The new function must:

1. clip `area` to `frame.area()`;
2. derive and clip the bordered/padded inner rectangle using the same helpers as the old API;
3. reserve one fixed row when `header.is_some()`;
4. compute `visual_spans(&content.lines, body.width)` and its visual content height;
5. set viewport height to the body height, reveal `content.selected`, clamp to visual content height, and calculate overflow;
6. render the same title decoration and border, using `Color::Cyan` only when `focused` and `BORDER_COLOR` otherwise;
7. render the fixed header at the top when present;
8. render empty copy when `lines` is empty;
9. otherwise render wrapped lines scrolled by the visual-row viewport;
10. pass the selected span for highlighting only when `focused`, while still revealing it when unfocused.

Keep border/geometry construction in private helpers used by both public functions. Do not convert the old row-index path to visual-row semantics.

- [ ] **Step 5: Verify GREEN and accepted-screen compatibility**

```bash
cargo fmt --all
cargo test tui::framework::list_panel::tests -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all new component tests and all accepted-screen fixtures pass unchanged.

- [ ] **Step 6: Commit the sibling API**

```bash
git add src/tui/framework/list_panel.rs src/tui/framework/mod.rs
git commit -m "feat(tui): add span-aware list panel"
```

---

### Task 3: Add the Pinned-Header DetailPanel Sibling API

**Files:**
- Modify: `src/tui/framework/components.rs`
- Modify: `src/tui/framework/mod.rs`

**Interfaces:**
- Consumes: existing `DetailPanelState`, `render_detail_panel`, `render_scrollable_lines`, `visual_spans`, `fit_rect`, `ViewportState`, and `VisualSpan`.
- Produces:

```rust
pub fn render_detail_panel_with_header(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    header: Vec<Line<'static>>,
    lines: Vec<Line<'static>>,
    state: &mut DetailPanelState,
    selected: Option<VisualSpan>,
);
```

The existing `render_detail_panel` signature and output remain unchanged.

- [ ] **Step 1: Add pinned-header RED tests**

Add tests that render a fixed progress header and 20 milestone body rows:

```rust
#[test]
fn pinned_detail_header_stays_visible_while_body_scrolls() {
    let mut state = DetailPanelState::new(true);
    state.viewport.set_offset(12);
    let output = render_to_string(40, 8, |frame| {
        render_detail_panel_with_header(
            frame,
            frame.area(),
            " Goal ",
            vec![Line::from("████████  10/20 · 50%")],
            (1..=20)
                .map(|number| Line::from(format!("Milestone {number:02}")))
                .collect(),
            &mut state,
            Some(VisualSpan { start: 19, height: 1 }),
        );
    });

    assert!(output.contains("10/20 · 50%"), "{output}");
    assert!(output.contains("Milestone 20"), "{output}");
    assert!(output.contains("Goal ↑"), "{output}");
}
```

Add tests for a wrapping two-line header, an empty body, an unfocused border with no selected-row background, and zero/one-cell/edge rectangles. The unfocused case must still reveal the selected span. For containment, capture the header/body rectangles through a private pure geometry helper and assert both remain inside the panel inner rectangle.

- [ ] **Step 2: Run focused tests and record RED**

```bash
cargo test tui::framework::components::tests::pinned_detail -- --nocapture
```

Expected: compilation fails because `render_detail_panel_with_header` does not exist.

- [ ] **Step 3: Implement a shared safe DetailPanel core**

Add the exact public function above. The implementation must:

1. clip `area` to `frame.area()`;
2. derive the bordered inner rectangle and clip it back to the safe panel area;
3. measure header visual height at `inner.width` and cap it to `inner.height`;
4. split the inner rectangle into pinned header and body rectangles with saturating arithmetic;
5. set `state.viewport.height` from body height only;
6. compute body content height and overflow independently of the header;
7. reveal/clamp the body-relative selected span;
8. decorate the title with body overflow;
9. render the border, wrapped pinned header, and independently scrolled body;
10. highlight only body rows in the selected span while `state.focused`, but reveal the span regardless of focus.

Refactor border/title/body rendering into a private helper shared by `render_detail_panel` and `render_detail_panel_with_header`. Preserve the old function's no-header geometry exactly at normal sizes.

- [ ] **Step 4: Export and verify GREEN**

Add `render_detail_panel_with_header` to the existing `components` re-export list in `framework/mod.rs`, then run:

```bash
cargo fmt --all
cargo test tui::framework::components::tests -- --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo test tui::framework:: -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: the pinned-header tests pass, Quotes and History fixtures are unchanged, and strict Clippy is clean.

- [ ] **Step 5: Commit the pinned header**

```bash
git add src/tui/framework/components.rs src/tui/framework/mod.rs
git commit -m "feat(tui): add pinned detail header"
```

---

### Task 4: Bind Responsive Goals Pane Geometry

**Files:**
- Modify: `src/tui/goals.rs:20-45`
- Test: `src/tui/goals.rs` test module

**Interfaces:**
- Consumes: Ratatui `Rect` and existing `fit_rect`.
- Produces:

```rust
const WIDE_GOALS_PANES_MIN_WIDTH: u16 = 83;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GoalsPaneAreas {
    goals: Rect,
    milestones: Rect,
}

fn goals_pane_areas(body: Rect) -> GoalsPaneAreas;
```

- [ ] **Step 1: Add exact geometry RED tests**

Add tests for the approved boundary:

```rust
#[test]
fn goals_panes_use_exact_wide_boundary_geometry() {
    assert_eq!(
        goals_pane_areas(Rect::new(0, 0, 83, 20)),
        GoalsPaneAreas {
            goals: Rect::new(0, 0, 34, 20),
            milestones: Rect::new(35, 0, 48, 20),
        }
    );
    assert_eq!(
        goals_pane_areas(Rect::new(0, 0, 107, 20)),
        GoalsPaneAreas {
            goals: Rect::new(0, 0, 42, 20),
            milestones: Rect::new(43, 0, 64, 20),
        }
    );
}

#[test]
fn goals_panes_stack_below_the_wide_threshold() {
    assert_eq!(
        goals_pane_areas(Rect::new(3, 4, 80, 21)),
        GoalsPaneAreas {
            goals: Rect::new(3, 4, 80, 8),
            milestones: Rect::new(3, 13, 80, 12),
        }
    );
}
```

Add a containment loop over `Rect::new(0,0,0,0)`, one-cell rectangles, `40x16`, offset rectangles, and rectangles near `u16::MAX`. Assert both returned panes remain inside `body` and never overlap the one-cell gap.

- [ ] **Step 2: Run focused geometry tests and record RED**

```bash
cargo test tui::goals::tests::goals_panes -- --nocapture
```

Expected: compilation fails because the constant, struct, and helper do not exist.

- [ ] **Step 3: Implement the pure helper exactly**

Use `u32` only for ratio multiplication, saturating coordinate arithmetic, and `fit_rect` on both results. The wide branch subtracts one cell, computes the two-fifths candidate, applies the 34/48 minima, and assigns the remainder. The narrow branch uses a gap only when `height >= 2`, computes `remaining * 2 / 5`, and assigns the remainder to Milestones.

The helper must not inspect screen state, terminal globals, Goals, milestones, or a `Frame`.

- [ ] **Step 4: Verify geometry and current Goals behavior**

```bash
cargo fmt --all
cargo test tui::goals::tests::goals_panes -- --nocapture
cargo test tui::goals::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: exact geometry and all pre-redesign behavior tests pass.

- [ ] **Step 5: Commit responsive geometry**

```bash
git add src/tui/goals.rs
git commit -m "feat(goals): bind responsive pane geometry"
```

---

### Task 5: Replace the Popup with Persistent Component Panes

**Files:**
- Modify: `src/tui/goals.rs`
- Modify: `locales/en.ftl:59-64`
- Modify: `locales/zh-CN.ftl:59-64`

**Interfaces:**
- Consumes: `render_screen_frame`, `render_scrollable_list_panel_spanned`, `SpannedListPanelContent`, `render_detail_panel_with_header`, `Shortcut`, `GoalsPaneAreas`, `TextInput`, `VisualSpan`, and the Task 1 behavior contracts.
- Produces: persistent responsive panes, renamed milestone state, `GoalsScreen::active_shortcuts`, and component-only production rendering.

- [ ] **Step 1: Add presentation RED tests before replacing render code**

Add this exact deterministic fixture helper, then add the presentation tests using `render_to_buffer` and `render_screen`:

```rust
fn goals_db_with_milestones(count: usize, completed: usize) -> Database {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    for number in 1..=count {
        let milestone_id = db
            .create_milestone(goal_id, &format!("Milestone {number:02}"))
            .unwrap();
        if number <= completed {
            db.toggle_milestone(milestone_id).unwrap();
        }
    }
    db
}
```

```rust
#[test]
fn goals_wide_render_keeps_both_panes_visible_without_obscuring_goals() {
    let db = goals_db_with_milestones(6, 2);
    let mut screen = GoalsScreen::new(&db).unwrap();
    let output = render_screen(&mut screen, 107, 24);

    assert!(output.contains(&tr("goals-title")), "{output}");
    assert!(output.contains("Milestone 06"), "{output}");
    assert!(output.contains("2/6"), "{output}");
}

#[test]
fn goals_narrow_render_stacks_both_panes() {
    let db = goals_db_with_milestones(4, 0);
    let mut screen = GoalsScreen::new(&db).unwrap();
    let output = render_screen(&mut screen, 80, 24);

    assert!(output.contains("0/4 · 0%"), "{output}");
    assert!(output.matches('┌').count() >= 2, "{output}");
}

#[test]
fn milestone_progress_header_remains_visible_after_scrolling() {
    let db = goals_db_with_milestones(20, 0);
    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.handle_key(key(KeyCode::Enter), &db);
    for _ in 1..20 {
        screen.handle_key(key(KeyCode::Down), &db);
        render_screen(&mut screen, 80, 24);
    }
    let output = render_screen(&mut screen, 80, 24);

    assert!(output.contains("Milestone 20"), "{output}");
    assert!(output.contains("0/20 · 0%"), "{output}");
}
```

Add these named focused tests:

- `goals_focus_changes_only_active_panel_style` asserts the focused border is Cyan, the inactive border remains `BORDER_COLOR`, and only the focused selected span receives the selected background.
- `goals_narrow_focus_switch_keeps_both_panes` renders `80x24` before and after `Enter`, asserts both titles remain present, and asserts only focus styling changes.
- `goals_resize_preserves_identity_input_and_viewports` uses enough Goals/milestones that its chosen offsets remain valid at both sizes, renders `107→80→107` while `milestone_input` contains a long edit and `goal_input` contains a distinct sentinel, and asserts Goal ID, milestone ID, mode, both input values/cursors without cross-pane copying, undo payload presence, and both viewport offsets survive each resize.
- `goals_wrapped_selection_is_fully_highlighted` uses a long Unicode Goal and milestone and asserts every visible row in each selected `VisualSpan` is highlighted only in its focused pane.
- `goals_shortcuts_match_active_handlers` is table-driven over every `Mode`/`MilestoneMode`, empty/non-empty selection, completed/incomplete state, and present/absent undo payload; it asserts the complete ordered `(key, label)` units and confirms every advertised key dispatches a non-inactive handler branch.
- `goals_all_modes_render_safely_in_small_rectangles` loops every mode over `(0,0)`, `(1,1)`, `(12,4)`, `(24,10)`, `(40,16)`, and `(80,24)`, asserting render does not panic and every pane returned by `goals_pane_areas` remains contained.

- [ ] **Step 2: Run the new presentation tests and record RED**

```bash
cargo test tui::goals::tests::goals_wide_render_keeps_both_panes_visible_without_obscuring_goals -- --exact --nocapture
cargo test tui::goals::tests::goals_narrow_render_stacks_both_panes -- --exact --nocapture
cargo test tui::goals::tests::milestone_progress_header_remains_visible_after_scrolling -- --exact --nocapture
cargo test tui::goals::tests::goals_focus_changes_only_active_panel_style -- --exact --nocapture
cargo test tui::goals::tests::goals_narrow_focus_switch_keeps_both_panes -- --exact --nocapture
cargo test tui::goals::tests::goals_resize_preserves_identity_input_and_viewports -- --exact --nocapture
cargo test tui::goals::tests::goals_wrapped_selection_is_fully_highlighted -- --exact --nocapture
cargo test tui::goals::tests::goals_shortcuts_match_active_handlers -- --exact --nocapture
cargo test tui::goals::tests::goals_all_modes_render_safely_in_small_rectangles -- --exact --nocapture
```

Expected: the old popup renderer fails the persistent-pane geometry and pinned-header assertions.

- [ ] **Step 3: Add exact localization keys**

Add to `locales/en.ftl`:

```ftl
goals-milestones-title = Milestones
goals-no-selection = Select or add a goal to manage milestones
```

Add to `locales/zh-CN.ftl`:

```ftl
goals-milestones-title = 里程碑
goals-no-selection = 请选择或添加目标以管理里程碑
```

- [ ] **Step 4: Rename persistent milestone state mechanically**

Use these exact production names:

```rust
enum Mode {
    Browse,
    AddGoal,
    EditGoal,
    EditGoalDate,
    ConfirmDeleteGoal,
    Milestones,
}

enum MilestoneMode {
    Browse,
    AddMilestone,
    EditMilestone,
    EditMilestoneDate,
    ConfirmDeleteMilestone,
}
```

Rename fields and handlers consistently:

```text
modal_goal_idx       -> milestone_goal_idx
modal_selected       -> milestone_selected
detail_state         -> milestone_panel_state
modal_mode           -> milestone_mode
modal_status_msg     -> milestone_status_msg
modal_last_deleted   -> milestone_last_deleted
ModalContent         -> MilestoneContent
handle_modal*        -> handle_milestone*
modal_milestone_count -> milestone_count
input (Goal handlers) -> goal_input
input (milestone handlers) -> milestone_input
```

Replace the single shared `input` field with `goal_input: TextInput` and `milestone_input: TextInput`; route every Goal edit/add/date handler and Goal render builder to the former, and every milestone edit/add/date handler and Milestones render builder to the latter. Focus changes never copy one input into the other. Re-entry clears both. Update Task 1 tests and every existing Goals test mechanically. No compatibility alias remains in production.

- [ ] **Step 5: Implement handler-derived active shortcuts**

Add `fn active_shortcuts(&self) -> Vec<Shortcut>`.

For Goal Browse:

- include `j/k Navigate` when a Goal exists;
- always include `a Add goal`;
- include `Enter Milestone`, `m Add`, `e Edit`, `Space Toggle`, and `d Delete` when a Goal exists;
- include `D Date` only when the selected Goal is complete;
- include `u Undo` only when `last_deleted.is_some()`;
- always include `Esc Back`.

For Milestones Browse:

- include `j/k Navigate` only when a milestone exists;
- always include `a Add` while a Goal exists;
- include `Enter/e Edit`, `Space Toggle`, and `d Delete` only when a milestone exists;
- include `D Date` only when the selected milestone is complete;
- include `u Undo` only when `milestone_last_deleted.is_some()`;
- always include `Esc Back`.

Every add/edit/date mode returns exactly `Enter Confirm` and `Esc Cancel`. Each delete confirmation returns exactly `y Yes` and `any Cancel`.

- [ ] **Step 6: Build span-aware Goal content**

Replace local Paragraph/list painting with a pure builder that receives the actual safe inner width and returns `SpannedListPanelContent`. Before mutably borrowing `list_viewport`, capture the clone-only render inputs in an exact local `GoalPanelRenderState { goals, selected, mode, goal_input }`; its `build(inner_width)` method flattens each Goal's title, gauge, and separator lines. Track the selected Goal's starting logical line and selected-line count, then compute its selected `VisualSpan` with `visual_spans` and `combined_span`.

The selected span includes title and gauge but not the trailing separator. Add/edit/date/delete inline lines retain their current text, styles, and bounded `TextInput` cursor behavior.

- [ ] **Step 7: Build pinned Milestones content**

Make `MilestoneContent` contain:

```rust
struct MilestoneContent {
    header: Vec<Line<'static>>,
    lines: Vec<Line<'static>>,
    selected: Option<VisualSpan>,
}
```

For a selected Goal, `header` is exactly `vec![goal_gauge(goal)]`. The body contains all milestone or inline mutation lines, without a duplicate gauge. Because the shared DetailPanel owns one border cell on each side and no horizontal padding, compute `milestone_inner_width = panes.milestones.width.saturating_sub(2)` and use that exact width for `visual_spans`; do not construct a local `Block` merely to recover its inner rectangle. For no selected Goal, use an empty header and `tr("goals-no-selection")`. For a Goal without milestones in Browse, retain `tr("goals-no-milestones")`.

- [ ] **Step 8: Replace `GoalsScreen::render` with component composition**

The production path must follow this shape:

```rust
pub fn render(&mut self, frame: &mut Frame) {
    let shortcuts = self.active_shortcuts();
    let active_status = if self.mode == Mode::Milestones {
        &self.milestone_status_msg
    } else {
        &self.status_msg
    };
    let screen = render_screen_frame(
        frame,
        frame.area(),
        CONTENT_WIDTH,
        active_status,
        &shortcuts,
    );
    let panes = goals_pane_areas(screen.body);
    let goals_focused = self.mode != Mode::Milestones;
    let visible_goal_idx = if goals_focused {
        self.selected
    } else {
        self.milestone_goal_idx
    };
    let goal_panel = GoalPanelRenderState::capture(self);
    let milestone_inner_width = panes.milestones.width.saturating_sub(2);
    let milestone_content = self.build_milestone_content(visible_goal_idx, milestone_inner_width);
    let milestone_title = self
        .goals
        .get(visible_goal_idx)
        .map(|goal| goal.title.clone())
        .unwrap_or_else(|| tr("goals-milestones-title"));

    render_scrollable_list_panel_spanned(
        frame,
        panes.goals,
        tr("goals-title"),
        goals_focused,
        &mut self.list_viewport,
        |inner| goal_panel.build(inner.width),
    );

    self.milestone_panel_state.focused = !goals_focused;
    render_detail_panel_with_header(
        frame,
        panes.milestones,
        milestone_title,
        milestone_content.header,
        milestone_content.lines,
        &mut self.milestone_panel_state,
        milestone_content.selected,
    );
}
```

Use split field borrows or cloned selected Goal data so the ListPanel closure does not borrow all of `self` while its viewport is mutably borrowed. Remove `centered_area`, local `Layout`, `Block`, `Borders`, `Padding`, `Clear`, `Paragraph`, `highlight_row`, `render_status_line`, and popup geometry from production imports and render code.

- [ ] **Step 9: Preserve handlers, re-entry, status routing, and stable identities**

Update renamed fields without changing database calls. Goal selection drives the visible Milestones preview in Goal focus; `Enter` and `m` set `milestone_goal_idx = selected`. `Esc` follows the approved edit → Milestones Browse → Goal Browse → Dashboard layering.

On successful milestone operations, restore the active Goal and milestone by stable IDs after `reload_goals`. Preserve `Reenter` selection/focus/viewports and clear `goal_input`, `milestone_input`, and all other renamed transient fields exactly as the current contract requires. `NavigationStatus` writes to `milestone_status_msg` only in `Mode::Milestones`.

- [ ] **Step 10: Verify migration GREEN**

```bash
cargo fmt --all
cargo test tui::goals::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test app:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all Goals/component/i18n/app tests and every accepted-screen fixture pass; strict Clippy is clean.

- [ ] **Step 11: Commit persistent panes**

```bash
git add src/tui/goals.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(goals): add persistent milestone panels"
```

---

### Task 6: Freeze Redesigned Goals States and Enforce Production Ownership

**Files:**
- Modify: `src/tui/goals.rs` test module
- Modify: `tests/tui_lint_test.rs`
- Create: all ten `tests/fixtures/tui/goals/*.txt` files declared above

**Interfaces:**
- Consumes: `render_to_snapshot`, redesigned `GoalsScreen`, `Mode::Milestones`, `MilestoneMode`, and the `syn` visitor infrastructure already used for History.
- Produces: `goals_states_match_golden_buffers`, ten compile-time styled fixtures, and `goals_ownership_violations`.

- [ ] **Step 1: Add deterministic redesigned Goals fixture builders**

Create fixed `Goal` and `Milestone` builders using IDs, positions, and `2026-08-14 08:00:00` timestamps. Build three Goals: one incomplete Goal with four mixed-state milestones, one completed Goal with a date, and one long Unicode Goal. Use `LC_ALL=C` and initialize localization.

Add:

```rust
fn goals_fixture_size(name: &str) -> (u16, u16) {
    match name {
        "wide_goal_focus" | "wide_milestones_focus" | "goal_edit"
        | "milestone_add" | "milestone_confirm_delete" => (107, 24),
        "narrow_goal_focus" | "narrow_milestones_focus" | "overflow"
        | "empty_goals" | "empty_milestones" => (80, 24),
        other => panic!("unknown Goals fixture size: {other}"),
    }
}
```

`golden_goals_screen(name)` must set the exact renamed mode, both pane inputs, selected identities, viewport offsets, status, and undo state for each declared fixture. The overflow state contains at least 20 Goals and 20 milestones and selects the final item in each viewport.

- [ ] **Step 2: Add temporary empty fixtures and update path**

Create each fixture with one newline through `apply_patch`. Add one serialized test loop that renders the mapped dimensions, writes only when `UPDATE_GOALS_GOLDENS` is present, reads the file, and compares exact `buffer_snapshot` output.

- [ ] **Step 3: Run and record fixture RED**

```bash
cargo test tui::goals::tests::goals_states_match_golden_buffers -- --exact --nocapture
```

Expected: FAIL on `wide_goal_focus` because the one-line fixture does not equal the redesigned styled buffer.

- [ ] **Step 4: Capture redesigned fixtures exactly once**

```bash
UPDATE_GOALS_GOLDENS=1 cargo test tui::goals::tests::goals_states_match_golden_buffers -- --exact --nocapture
wc -l tests/fixtures/tui/goals/*.txt
```

Expected: 107×24 fixtures contain `2569` lines and 80×24 fixtures contain `1921` lines.

- [ ] **Step 5: Remove every runtime update path**

Replace file reads with an exhaustive `include_str!` selector named `goals_golden`. Remove all `UPDATE_GOALS_GOLDENS`, `std::fs::write`, runtime fixture paths, and runtime fixture reads.

Run:

```bash
rg -n "UPDATE_GOALS_GOLDENS|std::fs::write|read_to_string" src/tui/goals.rs
cargo test tui::goals::tests::goals_states_match_golden_buffers -- --exact --nocapture
```

Expected: `rg` returns no fixture-update match and the golden test passes in library and binary targets.

- [ ] **Step 6: Add AST-scoped Goals ownership RED tests**

Generalize only the reusable AST helper pieces from History, then add `is_goals_screen_type`, `GoalsRenderOwnership`, `GoalsRenderVisitor`, and `goals_ownership_violations`. Extend the alias/path collector to normalize qualified and aliased uses of `Block`, `Clear`, `Paragraph`, `render_screen_frame`, `render_scrollable_list_panel_spanned`, and `render_detail_panel_with_header` before counting or rejecting them.

The validator must inspect exactly one non-test inherent `GoalsScreen::render`, skip nested item declarations, and require exactly one call to each:

```text
render_screen_frame
render_scrollable_list_panel_spanned
render_detail_panel_with_header
```

It must reject any production-render call/path for:

```text
.borders(...)
Block::bordered
render_status_line
render_shortcuts
Clear
Paragraph::new
```

Add synthetic tests for missing calls hidden by test/dead-helper decoys, duplicate calls, local border ownership, local status/shortcut rendering, `Clear`, `Paragraph::new`, qualified paths, aliases, and one valid component-only control. Only the single production inherent `GoalsScreen::render` body counts: nested functions/items, test modules, and dead helper functions neither satisfy required ownership nor create false positives.

Run:

```bash
cargo test --test tui_lint_test goals_ownership -- --nocapture
```

Expected before validator implementation: FAIL because decoys/local ownership are not rejected or the validator is absent.

- [ ] **Step 7: Integrate the validator into the recursive screen lint**

Remove `goals.rs` from `required_screens`, leaving only `log_entry.rs`. Add Goals to the delegated screen loop and invoke `goals_ownership_violations` exactly as History invokes its AST validator. Preserve every existing History, Practices, Quotes, Trends, Paragraph, and framework-border assertion.

- [ ] **Step 8: Verify fixtures, lint, and full focused regression**

```bash
cargo fmt --all
cargo test tui::goals::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: every suite passes and no accepted fixture changes.

- [ ] **Step 9: Commit fixtures and ownership lint**

```bash
git add src/tui/goals.rs tests/tui_lint_test.rs tests/fixtures/tui/goals
git commit -m "test(goals): freeze responsive panel states"
```

- [ ] **Step 10: Prove fixture immutability from the correct commits**

```bash
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --name-status HEAD^..HEAD -- tests/fixtures/tui/goals
```

Expected: the first command has no output. The second prints exactly ten `A` entries for the intentional initial Goals fixture additions in the characterization commit; record this commit hash for Task 7's range check.

---

### Task 7: Document and Release-Gate Batch 3B-5

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/goals-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed persistent Goals panes, final test counts, Task 6 fixture commit, and approved design terminology.
- Produces: active shipped-state documentation, final release evidence, clean worktree, and a Goals-only binary for manual acceptance.

- [ ] **Step 1: Update active documentation exactly**

Document these facts without claiming Dashboard or Log Entry:

- Batch 3B-5 Goals is shipped and is an approved presentation redesign, unlike the prior output-preserving slices.
- Goals uses a persistent responsive Goals/Milestones composition: horizontal at `>=83` body cells and stacked below 83.
- aggregate progress is pinned while milestone rows scroll;
- Goal progress remains milestone-derived;
- focus and layered `Esc` are Goal edit → Milestones Browse → Goal Browse → Dashboard as applicable;
- the old ListPanel and DetailPanel APIs remain accepted while the sibling APIs add multi-line selection and pinned headers;
- database, navigation, re-entry, atomic failure, and undo contracts remain unchanged;
- Dashboard responsive-component extraction and Log Entry form extraction remain unshipped.

- [ ] **Step 2: Remove stale active claims**

Run:

```bash
rg -n "Goals.*unshipped|Goals.*尚未交付|Goals popups|Goals popup|floating.*milestone|ModalMode|Mode::Modal" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
```

Expected: no stale active statement remains. Historical approved design records outside the active-doc set are not rewritten.

- [ ] **Step 3: Run focused release gates**

```bash
cargo test tui::goals::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test app:: -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
```

Expected: all focused suites pass in every applicable target.

- [ ] **Step 4: Run fresh full release gates**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check 70dae36..HEAD
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code "$(git log -1 --format=%H --diff-filter=A -- tests/fixtures/tui/goals)"..HEAD -- tests/fixtures/tui/goals
```

Expected: formatting, strict Clippy, full tests, build, range whitespace, all accepted fixture ranges, and the new Goals fixture range pass. Record fresh per-target counts from this run.

- [ ] **Step 5: Run exact ownership, removal, and scope audits**

```bash
rg -n "Mode::Modal|ModalMode|modal_|Clear|centered_area|render_status_line|render_shortcuts|shortcut_height|highlight_row|\.borders\(" src/tui/goals.rs
rg -n "render_screen_frame\(|render_scrollable_list_panel_spanned\(|render_detail_panel_with_header\(" src/tui/goals.rs
rg -n "Quick Log|QuickLog|LLM|llm|ureq|Abbreviation|abbreviations" src locales tests Cargo.toml
git diff --name-only 70dae36..HEAD
git status --short --untracked-files=all
```

Expected:

- no production popup/modal/local presentation owner remains in Goals; test-only names must be judged by exact location rather than deleted blindly;
- the production render path contains the three required component delegations;
- removed-feature audit produces only the intentionally allowlisted migration/test compatibility references;
- changed tracked paths are exactly those declared in this plan plus this plan and the approved design already on `main`;
- no unexpected tracked or untracked path exists.

- [ ] **Step 6: Commit active documentation**

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/goals-spec.md docs/specs/navigation-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Goals responsive panels"
```

- [ ] **Step 7: Run fresh post-commit verification**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check 70dae36..HEAD
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code "$(git log -1 --format=%H --diff-filter=A -- tests/fixtures/tui/goals)"..HEAD -- tests/fixtures/tui/goals
git status --short --untracked-files=all
test -x target/debug/iron
```

Expected: all gates pass, status is blank, and `target/debug/iron` exists.

- [ ] **Step 8: Perform the final whole-slice review and manual handoff**

Review the complete `70dae36..HEAD` range against the approved design and this plan. Report Critical, Important, and Minor findings with exact file/line evidence. Resolve every Critical or Important finding through focused RED/GREEN commits, rerun the complete release gates, and perform one final scoped re-review of those fixes.

Then provide the isolated worktree binary for manual verification of wide/stacked layout, persistent progress, scrolling, focus, edits, failure retry, re-entry, resize, and other-screen regressions. Stop. Do not merge and do not begin Dashboard or Log Entry before explicit user acceptance.

## Completion Checklist

- [ ] Goal and milestone delete failures retain their confirmation, identity, viewport, and visible error.
- [ ] Existing ListPanel and DetailPanel public APIs remain unchanged and all accepted-screen fixtures stay immutable.
- [ ] Span-aware ListPanel reveals/highlights complete multi-line Goal spans and suppresses active highlighting when unfocused.
- [ ] Pinned-header DetailPanel keeps aggregate progress fixed while milestones scroll independently.
- [ ] Goals uses exact 83-cell responsive geometry and remains bounded in tiny/resized rectangles.
- [ ] Floating popup, `Clear`, local list border, local status/footer rendering, and modal terminology are removed from production Goals.
- [ ] Goals and Milestones panes preserve independent focus, selections, statuses, undo payloads, inputs, and viewports.
- [ ] Shortcuts match every executable current-mode command and no inactive command.
- [ ] Milestone completion immediately and persistently updates the derived Goal count and percentage.
- [ ] Stable Goal/milestone identities survive reorder, mutation refresh, resize, and re-entry with bounded fallbacks.
- [ ] Ten redesigned Goals fixtures pass in library and binary targets and are immutable after capture.
- [ ] AST lint enforces one frame, one span-aware list, one pinned detail, and zero local popup/border/status/footer ownership in production render.
- [ ] Practices, Quotes, Trends, History, app, i18n, removal, formatting, strict Clippy, full tests, build, and audits pass.
- [ ] Documentation marks Goals shipped and Dashboard/Log Entry unshipped.
- [ ] Worktree is clean and the Goals-only binary is ready for explicit manual acceptance.
