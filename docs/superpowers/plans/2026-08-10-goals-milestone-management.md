# Goals and Milestone Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every milestone reachable from the selected goal, keep selection visible during fast edits, display `completed/total · percentage`, and synchronize goal completion with milestone state.

**Architecture:** Keep the existing `GoalsScreen` and floating selected-goal panel, but make the panel a stable preview/focus component driven by visual-row measurements rather than fixed logical-row assumptions. Add private database synchronization helpers so every milestone mutation updates its parent goal, while keeping the shared Dashboard renderer and all non-Goals screens unchanged.

**Tech Stack:** Rust 2021, ratatui 0.29, crossterm 0.28, rusqlite 0.32, chrono 0.4, built-in Rust tests and ratatui `TestBackend`.

## Global Constraints

- Stage one modifies Goals behavior only; do not edit History, Trends, Quotes, Practices, Log Entry, Quick Log, or Dashboard rendering/input code.
- Milestones remain binary complete/incomplete; do not add a numeric progress column or schema field.
- Keep shared `render_gauge_line()` output unchanged; format counts locally in Goals.
- All text inputs must use `visible_input_spans()` and remain inside their container.
- User-entered display text must wrap, and selection/scroll calculations must use visual rows.
- Preserve milestone title, position, completion state, and completion date through delete/undo.
- Increment package patch version from `3.2.0` to `3.2.1`.
- Stage two begins only after explicit user acceptance of this stage.

---

### Task 1: Synchronize Goal Completion in the Database

**Files:**
- Modify: `src/db.rs:223-264, 976-1082`
- Test: `tests/db_test.rs:324-401, 850-923`

**Interfaces:**
- Consumes: existing `goals.completed`, `goals.completed_at`, `milestones.completed`, and `milestones.completed_at` columns.
- Produces: private `Database::sync_goal_completion_from_milestones(&self, goal_id: i64) -> Result<()>` and `Database::reconcile_goal_completion_from_milestones(&self) -> Result<()>` helpers; existing public CRUD signatures remain unchanged.

- [ ] **Step 1: Write failing tests for derived completion**

Add independent integration tests using `Database::open_in_memory()`:

```rust
#[test]
fn completing_last_milestone_completes_goal_and_reopening_reactivates_it() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    let first = db.create_milestone(goal_id, "First").unwrap();
    let second = db.create_milestone(goal_id, "Second").unwrap();

    db.toggle_milestone(first).unwrap();
    assert!(!db.list_goals().unwrap()[0].completed);

    db.toggle_milestone(second).unwrap();
    let completed = db.list_goals().unwrap();
    assert!(completed[0].completed);
    assert!(completed[0].completed_at.is_some());

    db.toggle_milestone(first).unwrap();
    let reopened = db.list_goals().unwrap();
    assert!(!reopened[0].completed);
    assert!(reopened[0].completed_at.is_none());
}

#[test]
fn adding_incomplete_milestone_reopens_completed_goal() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    let first = db.create_milestone(goal_id, "First").unwrap();
    db.toggle_milestone(first).unwrap();
    assert!(db.list_goals().unwrap()[0].completed);

    db.create_milestone(goal_id, "Second").unwrap();
    assert!(!db.list_goals().unwrap()[0].completed);
}

#[test]
fn deleting_incomplete_milestone_completes_goal_when_remaining_are_done() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    let done = db.create_milestone(goal_id, "Done").unwrap();
    let pending = db.create_milestone(goal_id, "Pending").unwrap();
    db.toggle_milestone(done).unwrap();

    db.delete_milestone(pending).unwrap();
    assert!(db.list_goals().unwrap()[0].completed);
}
```

- [ ] **Step 2: Run the new tests and verify RED**

Run:

```bash
cargo test --test db_test completing_last_milestone_completes_goal_and_reopening_reactivates_it
cargo test --test db_test adding_incomplete_milestone_reopens_completed_goal
cargo test --test db_test deleting_incomplete_milestone_completes_goal_when_remaining_are_done
```

Expected: each test fails because milestone mutations currently leave the parent goal unchanged.

- [ ] **Step 3: Implement parent synchronization after every milestone mutation**

Add a private helper that leaves zero-milestone goals unchanged and otherwise derives completion:

```rust
fn sync_goal_completion_from_milestones(&self, goal_id: i64) -> Result<()> {
    let (total, done, latest): (i64, i64, Option<String>) = self.conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN completed = 1 THEN 1 ELSE 0 END), 0), MAX(completed_at) \
         FROM milestones WHERE goal_id = ?1",
        params![goal_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if total == 0 {
        return Ok(());
    }
    if done == total {
        let completed_at = latest.unwrap_or_else(|| {
            Local::now()
                .naive_local()
                .format("%Y-%m-%d %H:%M:%S%.f")
                .to_string()
        });
        self.conn.execute(
            "UPDATE goals SET completed = 1, completed_at = ?1 WHERE id = ?2",
            params![completed_at, goal_id],
        )?;
    } else {
        self.conn.execute(
            "UPDATE goals SET completed = 0, completed_at = NULL WHERE id = ?1",
            params![goal_id],
        )?;
    }
    Ok(())
}
```

Query the parent `goal_id` before toggle/delete/date mutations and call the helper after `create_milestone`, `toggle_milestone`, `delete_milestone`, `restore_milestone`, and `set_milestone_completed_at` succeed.

- [ ] **Step 4: Run the focused database tests and verify GREEN**

Run:

```bash
cargo test --test db_test milestone
cargo test --test db_test toggle_goal_completion
```

Expected: all milestone tests and the manual zero-milestone goal toggle test pass.

- [ ] **Step 5: Write failing tests for reconciliation and undo fidelity**

Add a file-backed test that corrupts a previously consistent parent goal using a direct `rusqlite::Connection`, reopens it with `Database::open()`, and expects startup reconciliation. Extend `test_restore_milestone` with literal position and completion-date assertions:

```rust
assert_eq!(restored.position, milestone.position);
assert_eq!(restored.completed, milestone.completed);
assert_eq!(restored.completed_at, milestone.completed_at);
```

Use a completed milestone with a fixed date of `2026-03-15 00:00:00` so the expectation does not depend on the clock.

- [ ] **Step 6: Run the reconciliation and restore tests and verify RED**

Run:

```bash
cargo test --test db_test reconciles_existing_goal_completion_on_open
cargo test --test db_test test_restore_milestone
```

Expected: reconciliation remains inconsistent, and restored position/date differ from the original.

- [ ] **Step 7: Implement reconciliation and exact milestone restoration**

Implement `reconcile_goal_completion_from_milestones()` by selecting goal IDs with at least one milestone and calling the synchronization helper after schema migrations. Replace `restore_milestone()`'s create/toggle sequence with an explicit insert using the original `position`, `created_at`, `completed`, and `completed_at`, then synchronize its goal.

- [ ] **Step 8: Run all database and export/import tests**

Run:

```bash
cargo test --test db_test
cargo test --test export_import_integration_test goals_and_milestones_survive_export_import
```

Expected: PASS. Export/import may reconcile inconsistent legacy goal flags according to the new derived rule.

- [ ] **Step 9: Commit the database behavior**

```bash
git add src/db.rs tests/db_test.rs
git commit -m "fix(goals): derive completion from milestones"
```

---

### Task 2: Add Visual-Row Layout and Progress Helpers

**Files:**
- Modify: `src/tui/goals.rs:10-120, 321-435, 575-650, 1184-1285`

**Interfaces:**
- Consumes: ratatui `Line::width()` and the actual popup inner width/height.
- Produces: private `VisualSpan { start: usize, height: usize }`, `visual_spans(&[Line], width: u16) -> Vec<VisualSpan>`, `keep_span_visible(scroll: usize, viewport: usize, span: VisualSpan) -> usize`, and `goal_progress_text(&Goal) -> String` helpers.

- [ ] **Step 1: Write failing pure layout and progress tests**

Add unit tests in `src/tui/goals.rs`:

```rust
#[test]
fn progress_text_shows_completed_total_and_percentage() {
    let goal = make_goal_with_milestones(&[true, true, false, false]);
    assert_eq!(goal_progress_text(&goal), "2/4 · 50%");
}

#[test]
fn keep_span_visible_scrolls_down_and_back_up() {
    let selected = VisualSpan { start: 5, height: 2 };
    assert_eq!(keep_span_visible(0, 4, selected), 3);
    let first = VisualSpan { start: 0, height: 1 };
    assert_eq!(keep_span_visible(3, 4, first), 0);
}

#[test]
fn visual_spans_count_wrapped_rows() {
    let lines = vec![Line::from("123456789"), Line::from("x")];
    assert_eq!(
        visual_spans(&lines, 4),
        vec![
            VisualSpan { start: 0, height: 3 },
            VisualSpan { start: 3, height: 1 },
        ]
    );
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test tui::goals::tests::progress_text_shows_completed_total_and_percentage
cargo test tui::goals::tests::keep_span_visible_scrolls_down_and_back_up
cargo test tui::goals::tests::visual_spans_count_wrapped_rows
```

Expected: compile failure because the helper interfaces do not exist.

- [ ] **Step 3: Implement the minimal pure helpers**

Derive each line's height as `max(1, ceil(line.width / width))`, accumulate starts, and clamp scrolling so the entire selected span is visible when it fits. Return `0%` or `100%` for zero-milestone manual goals and `done/total · percent%` otherwise.

- [ ] **Step 4: Update the Goals gauge locally**

Keep `render_gauge_line()` unchanged and replace the percent-only suffix in `goal_gauge()` with:

```rust
line.spans.push(Span::styled(
    format!("  {}", goal_progress_text(goal)),
    Style::default().fg(Color::Gray),
));
```

- [ ] **Step 5: Run the focused tests and verify GREEN**

Run:

```bash
cargo test tui::goals::tests
```

Expected: all existing main-list scroll tests and the new helper tests pass.

- [ ] **Step 6: Commit the layout primitives**

```bash
git add src/tui/goals.rs
git commit -m "refactor(goals): measure wrapped visual rows"
```

---

### Task 3: Render a Complete, Scrollable Milestone Panel

**Files:**
- Modify: `src/tui/goals.rs:211-573`
- Test: `src/tui/goals.rs:1197-end`

**Interfaces:**
- Consumes: Task 2 visual spans and `goal_progress_text()`.
- Produces: a private `ModalContent { lines: Vec<Line<'static>>, selected_line: Option<usize> }` returned by `build_modal_content(&self, goal: &Goal) -> ModalContent`; panel rendering updates `modal_scroll` from actual inner dimensions.

- [ ] **Step 1: Write a failing minimum-terminal render regression**

Use a real in-memory database and ratatui `TestBackend`:

```rust
#[test]
fn preview_shows_four_milestones_at_minimum_terminal_size() {
    let db = Database::open_in_memory().unwrap();
    let goal_id = db.create_goal("Goal").unwrap();
    for title in ["01", "02", "03", "04"] {
        db.create_milestone(goal_id, title).unwrap();
    }
    let mut screen = GoalsScreen::new(&db).unwrap();
    let output = render_screen(&mut screen, 80, 24);

    for title in ["01", "02", "03", "04"] {
        assert!(output.contains(title), "missing milestone {title}\n{output}");
    }
}
```

`render_screen()` creates `TestBackend::new(width, height)`, draws `screen.render(frame)`, and joins every buffer cell symbol row by row. This test catches the exact border-plus-padding regression shown in the screenshot.

- [ ] **Step 2: Run the render test and verify RED**

Run:

```bash
cargo test tui::goals::tests::preview_shows_four_milestones_at_minimum_terminal_size -- --nocapture
```

Expected: FAIL because `03` and `04` are clipped.

- [ ] **Step 3: Replace logical-line popup sizing with measured inner sizing**

Build the block first, include four chrome rows (two border plus two padding) in the outer-height request, reserve one transient action row so preview and focused geometry remain stable, and cap the panel to the actual available Goals viewport. Use `block.inner(popup_rect)` as the sole viewport height for scrolling.

- [ ] **Step 4: Track selected visual span and adjust `modal_scroll`**

Return the selected logical line from `build_modal_content()`, map it through `visual_spans()`, and call `keep_span_visible()` during focused rendering. Render `↑`, `↓`, or `↑↓` in the panel title when content exists outside the viewport. Use the selected visual span for row highlighting instead of `modal_selected + 1`.

- [ ] **Step 5: Write and run failing navigation-scroll tests**

Create eight milestones, enter panel focus with `Enter`, send `j` repeatedly through `handle_key()`, render after every key, and assert the selected title remains present. Then navigate back to the first item and assert `modal_scroll == 0`.

Run:

```bash
cargo test tui::goals::tests::focused_panel_keeps_selected_milestone_visible
```

Expected before the scroll adjustment: FAIL because `modal_scroll` stays zero.

- [ ] **Step 6: Run all Goals rendering and scroll tests**

Run:

```bash
cargo test tui::goals::tests
```

Expected: PASS, including four-item preview, wrapped rows, and eight-item navigation.

- [ ] **Step 7: Commit the panel rendering**

```bash
git add src/tui/goals.rs
git commit -m "fix(goals): keep all milestones reachable"
```

---

### Task 4: Align Focus, Shortcuts, Input, and Mutation Behavior

**Files:**
- Modify: `src/tui/goals.rs:10-207, 465-1125`
- Modify: `locales/en.ftl:57-61`
- Modify: `locales/zh-CN.ftl:57-61`
- Test: `src/tui/goals.rs:1197-end`
- Test: `tests/i18n_test.rs`

**Interfaces:**
- Consumes: existing `visible_input_spans()`, `StatusMessage`, database CRUD, and Tasks 1-3 panel state.
- Produces: documented main/panel key behavior with stable selection after mutations and a translated message for derived goal completion.

- [ ] **Step 1: Write failing keyboard behavior tests**

Add tests using a real in-memory database:

```rust
#[test]
fn m_opens_selected_goal_directly_in_add_milestone_mode() {
    let db = Database::open_in_memory().unwrap();
    db.create_goal("Goal").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();

    screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);

    assert_eq!(screen.mode, Mode::Modal);
    assert_eq!(screen.modal_mode, ModalMode::AddMilestone);
}

#[test]
fn adding_milestone_selects_new_row_and_keeps_panel_focused() {
    let db = Database::open_in_memory().unwrap();
    db.create_goal("Goal").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE), &db);
    screen.input = "New milestone".to_string();
    screen.cursor = screen.input.len();

    screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &db);

    assert_eq!(screen.mode, Mode::Modal);
    assert_eq!(screen.modal_mode, ModalMode::Browse);
    assert_eq!(screen.modal_selected, 0);
    assert_eq!(screen.goals[0].milestones[0].title, "New milestone");
}
```

Also test that `Space` on a goal with milestones does not call manual toggle and leaves a non-error status message.

- [ ] **Step 2: Run the keyboard tests and verify RED**

Run:

```bash
cargo test tui::goals::tests::m_opens_selected_goal_directly_in_add_milestone_mode
cargo test tui::goals::tests::adding_milestone_selects_new_row_and_keeps_panel_focused
cargo test tui::goals::tests::space_on_goal_with_milestones_explains_derived_progress
```

Expected: `m` is unhandled, new rows are not selected, and manual goal toggling is still allowed.

- [ ] **Step 3: Implement focus and fast-action keys**

In goal browse mode, map `Enter` to focused panel browse and `m` to focused panel add mode. In panel browse, accept both `Enter` and `e` for title editing. After add, reload, select the last position-ordered milestone, and retain `Mode::Modal`. After toggle/edit/date/delete/undo, retain the nearest valid selected milestone and allow the next render to bring it into view.

- [ ] **Step 4: Disable manual completion for goals with milestones**

Add matching locale keys:

```text
goals-progress-from-milestones = Goal completion is controlled by milestones
```

```text
goals-progress-from-milestones = 目标完成状态由里程碑决定
```

On main-list `Space`, show this status and do not call `toggle_goal()` when milestones exist. Preserve manual toggle for empty goals.

- [ ] **Step 5: Replace raw input spans with clipped input spans**

Import `visible_input_spans` and build every goal/milestone add, edit, and date input with the real container width and prefix width. Do not slice and render the entire before/after input directly.

- [ ] **Step 6: Preserve state on persistence failures**

Change each Goals mutation handler so `reload_goals()` errors are reported instead of discarded. When create/update/date persistence fails, keep the current input mode and text; only clear input and return to browse after both mutation and reload succeed.

- [ ] **Step 7: Run Goals and i18n tests**

Run:

```bash
cargo test tui::goals::tests
cargo test --test i18n_test
cargo test --test tui_lint_test
```

Expected: PASS with no stale TUI lint allowlist lines.

- [ ] **Step 8: Commit the interaction behavior**

```bash
git add src/tui/goals.rs locales/en.ftl locales/zh-CN.ftl tests/i18n_test.rs tests/tui_lint_test.rs
git commit -m "feat(goals): streamline milestone management"
```

---

### Task 5: Update Goals Documentation, Version, and Full Verification

**Files:**
- Modify: `docs/specs/goals-spec.md:5-102`
- Modify: `README.md:126-139`
- Modify: `README_zh.md:126-139`
- Modify: `Cargo.toml:1-4`
- Modify: `Cargo.lock` package version entry for `iron`
- Modify: `docs/superpowers/specs/2026-08-10-goals-milestone-management-design.md` only if implementation exposed a factual correction; do not expand scope.

**Interfaces:**
- Consumes: implemented stage-one behavior and verified shortcut map.
- Produces: user-facing verification instructions and package version `3.2.1`.

- [ ] **Step 1: Update the Goals specification**

Document the selected-goal panel, `done/total · percentage`, overflow indicators, focus distinction, `m` direct add, `Enter`/`e` edit inside the panel, derived completion for goals with milestones, and manual completion for goals without milestones. Remove contradictory statements that `Enter` edits and opens at the same level.

- [ ] **Step 2: Update both README shortcut tables**

Separate goal-list and milestone-panel shortcuts so `j/k`, `Enter`, `m`, `Space`, `a`, `e`, `D`, `d`, `u`, and `Esc` match the implementation exactly in both languages.

- [ ] **Step 3: Increment the patch version**

Change `Cargo.toml` package version to `3.2.1`, then run:

```bash
cargo check
```

This updates the root package version in `Cargo.lock` without changing dependency versions.

- [ ] **Step 4: Run formatting and static checks**

Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: clean output with zero warnings. Remove the existing unused `Datelike` import in `tests/db_test.rs` if it still triggers under `-D warnings`.

- [ ] **Step 5: Run the full automated suite**

Run:

```bash
cargo test
```

Expected: every unit, integration, export/import, i18n, and TUI lint test passes.

- [ ] **Step 6: Review the stage-one diff boundary**

Run:

```bash
git diff --stat HEAD~4
git diff --name-only HEAD~4
git status --short
```

Expected modified implementation scope: `src/db.rs`, `src/tui/goals.rs`, Goals tests, locale files, Goals spec, READMEs, `Cargo.toml`, and `Cargo.lock`. No other TUI module implementation file is modified.

- [ ] **Step 7: Commit documentation and version**

```bash
git add Cargo.toml Cargo.lock README.md README_zh.md docs/specs/goals-spec.md docs/superpowers/specs/2026-08-10-goals-milestone-management-design.md tests/db_test.rs
git commit -m "docs(goals): document milestone management workflow"
```

- [ ] **Step 8: Prepare the manual validation handoff**

Report the binary path or `cargo run` command and the seven checks from the design's Stage One Acceptance Checklist. Explicitly state that stage two remains blocked until the user reports stage-one acceptance.
