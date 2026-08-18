# TUI Consistency Batch 3B-6 Dashboard Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Dashboard outer-frame and bordered-panel presentation to shared components while preserving the accepted `80x35` styled output and all existing behavior.

**Architecture:** `ScreenFrameSections` owns the centered outer layout, reserved status row, and wrapped shortcut rows. A new presentation-only `render_static_panel` sibling owns clipped border/title/padding geometry, while Dashboard continues to own every domain line, Heatmap, HRV mode, database operation, navigation action, and failure policy.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Chrono 0.4, Rusqlite 0.32, Fluent localization, Unicode Width, existing TUI framework/test support, Cargo tests, Clippy, and styled `TestBackend` buffer fixtures.

**Spec:** `docs/superpowers/specs/2026-08-18-tui-consistency-batch-3b-dashboard-components-design.md`

## Global Constraints

- Batch 3B-6 changes Dashboard presentation composition only; Log Entry remains unshipped.
- Preserve Dashboard layout, title glyphs, colors, padding, content order, navigation, shortcuts, database queries, HRV workflow, re-entry, and navigation-failure semantics.
- At `80x35`, Normal, HRV input, and quit-confirmation styled buffers must be identical before and after migration.
- Smaller rectangles are geometry/semantic tests, not golden layouts; they must remain bounded and panic-free.
- Existing framework APIs keep their signatures and semantics. Add only the exact sibling API in Task 2.
- Do not add list selection, scrolling, or viewport state to Recent or Goals.
- Do not modify Practices, Quotes, Trends, History, Goals, or Log Entry production code.
- Existing fixtures under `tests/fixtures/tui/{practices,quotes,trends,history,goals}` are immutable.
- Add no dependency and no general-purpose layout DSL.
- Stop for user direction if an accepted non-Dashboard fixture changes, another screen needs a production edit, an existing public component contract must break, or pre-migration Dashboard fixtures cannot remain exact.
- Use TDD for every production change: observe focused RED, make the smallest GREEN change, then refactor.
- Execute this plan in an isolated worktree created through `superpowers:using-git-worktrees`.

## File Map

- Create `src/tui/framework/static_panel.rs` — static bordered-panel geometry and renderer only.
- Create `tests/fixtures/tui/dashboard/{normal,hrv_input,confirm_quit}.txt` — accepted `80x35` buffers.
- Modify `src/tui/framework/mod.rs` — declare and re-export the sibling API.
- Modify `src/tui/widgets/heatmap.rs` — backward-compatible explicit-date constructor for deterministic tests.
- Modify `src/tui/dashboard.rs` — deterministic test seam, fixtures, shared component migration, and geometry tests.
- Modify `tests/tui_lint_test.rs` — lightweight Dashboard ownership guard and proven mechanical allowlist shifts only.
- Modify `CLAUDE.md`, `README.md`, `README_zh.md`, `docs/specs/dashboard-spec.md`, `docs/specs/navigation-spec.md`, and `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — shipped ownership and unchanged behavior.

---

### Task 1: Freeze Deterministic Dashboard Styled Buffers

**Files:**
- Modify: `src/tui/widgets/heatmap.rs`
- Modify: `src/tui/dashboard.rs`
- Modify if and only if existing Dashboard Paragraph sites move: `tests/tui_lint_test.rs`
- Create: `tests/fixtures/tui/dashboard/normal.txt`
- Create: `tests/fixtures/tui/dashboard/hrv_input.txt`
- Create: `tests/fixtures/tui/dashboard/confirm_quit.txt`

**Interfaces:**
- Consumes: existing `Heatmap::new(data, weeks, no_color)` and `framework::test_support::render_to_snapshot`.
- Produces: backward-compatible `Heatmap::with_today(data, weeks, no_color, today)`, private `DashboardScreen::render_on(frame, today)`, one golden test, and three immutable fixtures.

- [ ] **Step 1: Add deterministic fixture builders and empty expected files**

Import `chrono::{NaiveDate, NaiveDateTime}`, all model types used below, and `render_to_snapshot` in the Dashboard test module. Add:

```rust
fn dashboard_fixture_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 18).unwrap()
}

fn dashboard_fixture_time(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, day)
        .unwrap()
        .and_hms_opt(6, 30, 0)
        .unwrap()
}

fn golden_dashboard_screen(name: &str) -> DashboardScreen {
    std::env::set_var("LC_ALL", "C");
    crate::i18n::init();
    let logged_at = dashboard_fixture_time(18);
    let goal_time = dashboard_fixture_time(1);
    let mut screen = DashboardScreen {
        heatmap_data: vec![("2026-08-17".into(), 1), ("2026-08-18".into(), 3)],
        recent_entries: vec![LogEntry {
            log: Log {
                id: 701,
                practice_id: 301,
                logged_at,
                note: Some("Fixture note".into()),
                warm_up: None,
                cool_down: None,
                rpe: Some(8),
            },
            practice_name: "A deliberately long weighted practice name that must stay inside Recent".into(),
            practice_type: PracticeType::Weighted,
            sets: vec![Set {
                id: 801,
                log_id: 701,
                set_number: 1,
                data: SetData::Weighted { weight: 1234.0, reps: 5678 },
            }],
            rpe: Some(8),
        }],
        quote: "A deliberately long quote must wrap only inside Summary while the footer remains complete.".into(),
        goals: vec![Goal {
            id: 901,
            title: "A deliberately long active goal title that must stay inside Goals".into(),
            completed: false,
            position: 0,
            created_at: goal_time,
            completed_at: None,
            milestones: vec![
                Milestone {
                    id: 902,
                    goal_id: 901,
                    title: "Done".into(),
                    completed: true,
                    position: 0,
                    created_at: goal_time,
                    completed_at: Some(goal_time),
                },
                Milestone {
                    id: 903,
                    goal_id: 901,
                    title: "Pending".into(),
                    completed: false,
                    position: 1,
                    created_at: goal_time,
                    completed_at: None,
                },
            ],
        }],
        mode: DashboardMode::Normal,
        hrv_today: Some(64),
        hrv_input: TextInput::new(),
        status_msg: Some(("Dashboard fixture status".into(), false)),
        no_color: false,
        weekly_volume: 12_345.0,
        training_days: 4,
        consecutive_days: 3,
    };
    match name {
        "normal" => {}
        "hrv_input" => {
            screen.mode = DashboardMode::HrvInput;
            screen.hrv_input = TextInput::from("9".repeat(100));
            screen.status_msg = Some(("HRV fixture status".into(), true));
        }
        "confirm_quit" => {
            screen.mode = DashboardMode::ConfirmQuit;
            screen.status_msg = Some(("Hidden by confirmation".into(), true));
        }
        other => panic!("unknown Dashboard golden state: {other}"),
    }
    screen
}
```

Create the three fixture files empty and add compile-time loading plus comparison:

```rust
fn dashboard_golden(name: &str) -> &'static str {
    match name {
        "normal" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/dashboard/normal.txt")),
        "hrv_input" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/dashboard/hrv_input.txt")),
        "confirm_quit" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/dashboard/confirm_quit.txt")),
        other => panic!("unknown Dashboard golden fixture: {other}"),
    }
}

#[test]
fn dashboard_states_match_golden_buffers() {
    for name in ["normal", "hrv_input", "confirm_quit"] {
        let screen = golden_dashboard_screen(name);
        let actual = render_to_snapshot(80, 35, |frame| {
            screen.render_on(frame, dashboard_fixture_today())
        });
        assert_eq!(actual, dashboard_golden(name), "Dashboard {name} buffer changed");
    }
}
```

- [ ] **Step 2: Run the golden test and record RED**

Run:

```bash
cargo test tui::dashboard::tests::dashboard_states_match_golden_buffers -- --exact --nocapture
```

Expected: compilation fails because `DashboardScreen::render_on` is missing. After Step 3 adds the seam, it fails because each expected fixture is empty.

- [ ] **Step 3: Add the minimal explicit-date seam**

In `Heatmap`, store `today: chrono::NaiveDate`. Keep `Heatmap::new` and add:

```rust
pub fn new(data: &'a [(String, i64)], weeks: u16, no_color: bool) -> Self {
    Self::with_today(data, weeks, no_color, Local::now().date_naive())
}

pub(crate) fn with_today(
    data: &'a [(String, i64)],
    weeks: u16,
    no_color: bool,
    today: chrono::NaiveDate,
) -> Self {
    Self { data, weeks, no_color, today }
}
```

Replace the widget's local `Local::now().date_naive()` with `self.today`. Split Dashboard rendering without changing its body:

```rust
pub fn render(&self, frame: &mut Frame) {
    self.render_on(frame, chrono::Local::now().date_naive());
}

fn render_on(&self, frame: &mut Frame, today: chrono::NaiveDate) {
    // Existing render body; only construct Heatmap through:
    let heatmap = Heatmap::with_today(&self.heatmap_data, 52, self.no_color, today);
}
```

Run the focused test again. Expected: it compiles and fails only on empty expected fixtures.

- [ ] **Step 4: Capture fixtures with a temporary one-shot test**

Temporarily add:

```rust
#[test]
#[ignore]
fn capture_dashboard_golden_buffers_once() {
    for name in ["normal", "hrv_input", "confirm_quit"] {
        let screen = golden_dashboard_screen(name);
        let snapshot = render_to_snapshot(80, 35, |frame| {
            screen.render_on(frame, dashboard_fixture_today())
        });
        std::fs::write(format!("tests/fixtures/tui/dashboard/{name}.txt"), snapshot).unwrap();
    }
}
```

Run:

```bash
cargo test tui::dashboard::tests::capture_dashboard_golden_buffers_once -- --ignored --exact --nocapture
```

Expected: PASS and every fixture contains `2801` lines: one area header plus `80 * 35` cells. Remove the entire temporary capture test immediately; retain no environment-controlled updater or writer.

- [ ] **Step 5: Verify fixture GREEN and commit**

Run:

```bash
cargo test tui::dashboard::tests::dashboard_states_match_golden_buffers -- --exact --nocapture
cargo test tui::widgets::heatmap::tests -- --nocapture
rg -n "capture_dashboard_golden_buffers_once|UPDATE_GOLDEN|std::fs::write" src/tui/dashboard.rs
wc -l tests/fixtures/tui/dashboard/*.txt
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code "$(git log -1 --format=%H --diff-filter=A -- tests/fixtures/tui/goals)"..HEAD -- tests/fixtures/tui/goals
```

Expected: tests/gates pass; the `rg` command exits `1`; each fixture has `2801` lines; no accepted fixture changes.

Run `cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture`. If the deterministic render seam shifted an existing Dashboard allowlist site, map the old and new lines to the identical widget and update only that literal. Then run `cargo test --all-targets --all-features`; both commands must pass before the characterization commit.

Commit:

```bash
git add src/tui/widgets/heatmap.rs src/tui/dashboard.rs tests/fixtures/tui/dashboard tests/tui_lint_test.rs
git commit -m "test(dashboard): freeze accepted presentation"
```

Record the commit hash as `DASHBOARD_FIXTURE_BASE`. Later tasks must run `git diff --exit-code "$DASHBOARD_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/dashboard`.

---

### Task 2: Add the Static Panel Sibling

**Files:**
- Create: `src/tui/framework/static_panel.rs`
- Modify: `src/tui/framework/mod.rs`

**Interfaces:**
- Consumes: `fit_rect`, `BORDER_COLOR`, Ratatui `Frame`, `Rect`, `Block`, `Borders`, `Padding`, `Line`, `Span`, `Style`, `Color`, and `Modifier`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticPanelAreas {
    pub area: Rect,
    pub inner: Rect,
}

pub fn render_static_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> StaticPanelAreas;
```

- [ ] **Step 1: Write failing component tests**

Create `static_panel.rs` with test imports and calls to the missing API. Add:

```rust
#[test]
fn static_panel_renders_accepted_border_title_padding_and_content() {
    let mut returned = None;
    let buffer = render_to_buffer(20, 7, |frame| {
        returned = Some(render_static_panel(frame, frame.area(), "Summary", |frame, inner| {
            frame.render_widget(
                Paragraph::new("Content").wrap(Wrap { trim: false }),
                inner,
            );
        }));
    });
    assert_eq!(returned.unwrap(), StaticPanelAreas {
        area: Rect::new(0, 0, 20, 7),
        inner: Rect::new(2, 2, 16, 3),
    });
    assert_eq!(buffer[(0, 0)].symbol(), "┌");
    assert_eq!(buffer[(4, 0)].symbol(), "S");
    assert_eq!(buffer[(0, 3)].fg, BORDER_COLOR);
    assert_eq!(buffer[(4, 0)].fg, Color::White);
    assert!(buffer[(4, 0)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(2, 2)].symbol(), "C");
}

#[test]
fn static_panel_passes_the_returned_safe_inner_to_content() {
    let mut closure_inner = None;
    let mut returned = None;
    render_to_buffer(8, 6, |frame| {
        returned = Some(render_static_panel(frame, Rect::new(2, 1, 5, 4), "Panel", |_, inner| {
            closure_inner = Some(inner);
        }));
    });
    assert_eq!(closure_inner, returned.map(|areas| areas.inner));
}

#[test]
fn static_panel_clips_zero_tiny_edge_and_oversized_rectangles() {
    let frame_area = Rect::new(0, 0, 6, 6);
    for requested in [
        Rect::new(0, 0, 0, 0),
        Rect::new(5, 5, 1, 1),
        Rect::new(5, 5, 4, 4),
        Rect::new(u16::MAX, u16::MAX, 0, 0),
    ] {
        let mut returned = None;
        render_to_buffer(6, 6, |frame| {
            returned = Some(render_static_panel(frame, requested, "Panel", |_, _| {}));
        });
        let areas = returned.unwrap();
        assert!(rect_is_within(frame_area, areas.area), "{requested:?} -> {areas:?}");
        assert!(rect_is_within(areas.area, areas.inner), "{requested:?} -> {areas:?}");
    }
}
```

Define `rect_is_within` exactly as existing framework geometry tests do.

- [ ] **Step 2: Run focused RED**

Run `cargo test tui::framework::static_panel::tests -- --nocapture`.

Expected: compilation fails because `StaticPanelAreas` and `render_static_panel` are missing.

- [ ] **Step 3: Implement the minimal component and export it**

```rust
fn static_panel_geometry(frame_area: Rect, requested_area: Rect) -> StaticPanelAreas {
    let area = fit_rect(frame_area, requested_area);
    let geometry = Block::default().borders(Borders::ALL).padding(Padding::uniform(1));
    let inner = fit_rect(area, geometry.inner(area));
    StaticPanelAreas { area, inner }
}

fn static_panel_block(title: String) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled("── ", Style::default().fg(BORDER_COLOR)),
            Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(" ──", Style::default().fg(BORDER_COLOR)),
        ]))
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .border_style(Style::default().fg(BORDER_COLOR))
}

pub fn render_static_panel(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    render_content: impl FnOnce(&mut Frame, Rect),
) -> StaticPanelAreas {
    let areas = static_panel_geometry(frame.area(), area);
    frame.render_widget(static_panel_block(title.into()), areas.area);
    render_content(frame, areas.inner);
    areas
}
```

In `framework/mod.rs`, add:

```rust
pub mod static_panel;
#[allow(unused_imports)]
pub use static_panel::{render_static_panel, StaticPanelAreas};
```

- [ ] **Step 4: Run framework GREEN and commit**

Run:

```bash
cargo test tui::framework::static_panel::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo fmt -- --check
git diff --check
git diff --exit-code "$DASHBOARD_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/dashboard
```

Expected: all tests/gates pass and Dashboard fixtures are unchanged. Keep the narrow re-export allowance because Dashboard consumes the function without naming `StaticPanelAreas`; do not broaden it to the module or file.

Commit:

```bash
git add src/tui/framework/static_panel.rs src/tui/framework/mod.rs
git commit -m "feat(tui): add static panel component"
```

---

### Task 3: Migrate Dashboard and Transfer Presentation Ownership

**Files:**
- Modify: `src/tui/dashboard.rs`
- Modify: `tests/tui_lint_test.rs`

**Interfaces:**
- Consumes: `render_screen_frame_sections`, `render_static_panel`, seven approved body constraints, and immutable Dashboard fixtures.
- Produces: one shared-frame call, four static-panel calls, no local bordered panel, unchanged domain content, and a lightweight ownership guard.

- [ ] **Step 1: Write the failing ownership guard**

Add beside `source_code_contains`:

```rust
fn source_code_count(source: &str, needle: &str) -> usize {
    source_code(source).match_indices(needle).count()
}

fn dashboard_production_source(source: &str) -> String {
    let code = source_code(source);
    code.split("#[cfg(test)]").next().unwrap_or(&code).to_owned()
}

fn dashboard_ownership_violations(source: &str) -> Vec<String> {
    let production = dashboard_production_source(source);
    let mut violations = Vec::new();
    for (needle, expected, label) in [
        ("render_screen_frame_sections(", 1, "ScreenFrameSections"),
        ("render_static_panel(", 4, "StaticPanel"),
    ] {
        let actual = source_code_count(&production, needle);
        if actual != expected {
            violations.push(format!(
                "Dashboard must delegate to {label} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    for forbidden in [".borders(Borders::ALL)", "dashboard_block_inner("] {
        if source_code_contains(&production, forbidden) {
            violations.push(format!("Dashboard still owns {forbidden}"));
        }
    }
    violations
}

#[test]
fn dashboard_delegates_static_presentation_to_framework() {
    let source = std::fs::read_to_string("src/tui/dashboard.rs").unwrap();
    let violations = dashboard_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}
```

Add a synthetic control whose comments, strings, and `#[cfg(test)]` calls cannot satisfy ownership:

```rust
#[test]
fn dashboard_ownership_ignores_nonproduction_decoys() {
    let source = r#"
        fn render() {
            // render_screen_frame_sections(); render_static_panel();
            let _text = "render_static_panel(";
        }
        #[cfg(test)]
        mod tests {
            fn decoy() {
                render_screen_frame_sections();
                render_static_panel(); render_static_panel();
                render_static_panel(); render_static_panel();
                Block::default().borders(Borders::ALL);
            }
        }
    "#;
    let violations = dashboard_ownership_violations(source);
    assert!(violations.iter().any(|item| item.contains("ScreenFrameSections")));
    assert!(violations.iter().any(|item| item.contains("StaticPanel")));
    assert!(!violations.iter().any(|item| item.contains("Borders::ALL")));
}
```

- [ ] **Step 2: Run ownership RED**

Run:

```bash
cargo test --test tui_lint_test dashboard_delegates_static_presentation_to_framework -- --exact --nocapture
cargo test --test tui_lint_test dashboard_ownership_ignores_nonproduction_decoys -- --exact --nocapture
```

Expected: production ownership fails with zero shared calls and local border ownership; the synthetic control passes.

- [ ] **Step 3: Migrate the outer frame exactly**

Replace the outer centered layout and explicit status/footer render with:

```rust
let shortcuts = self.active_shortcuts();
let status = if self.mode == DashboardMode::ConfirmQuit {
    Some((tr("dashboard-quit-confirm"), true))
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
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(13),
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Min(1),
    ],
    false,
);
let chunks = &sections.body;
```

Keep logo at `chunks[0]`, Heatmap at `chunks[2]`, Summary at `chunks[4]`, and the `50% / 50%` split at `chunks[6]`. Remove Dashboard's direct status/footer calls.

- [ ] **Step 4: Migrate all four bordered panels**

Import `render_static_panel`. Replace Heatmap's local block with:

```rust
render_static_panel(frame, chunks[2], tr("dashboard-heatmap-title"), |frame, inner| {
    if inner.width > 0 && inner.height > 0 {
        frame.render_widget(
            Heatmap::with_today(&self.heatmap_data, 52, self.no_color, today),
            inner,
        );
    }
});
```

Convert `render_summary`, `render_recent_pane`, and `render_goals_pane` so each calls `render_static_panel` once and keeps its existing content inside the closure. Use titles `summary-title`, `dashboard-recent-title`, and `dashboard-goals`. Delete `dashboard_block_inner`; remove production imports of `Block`, `Borders`, and `Padding`.

Do not use `ScrollableListPanel`, add state, or move summary, HRV editor, quote, date grouping, metric budgeting, goal filtering, gauge calculation, or truncation into the framework.

- [ ] **Step 5: Run fixture and ownership GREEN**

Run:

```bash
cargo test tui::dashboard::tests::dashboard_states_match_golden_buffers -- --exact --nocapture
cargo test tui::dashboard::tests -- --nocapture
cargo test --test tui_lint_test dashboard_delegates_static_presentation_to_framework -- --exact --nocapture
cargo test --test tui_lint_test dashboard_ownership_ignores_nonproduction_decoys -- --exact --nocapture
```

Expected: all pass in library and binary targets; all three fixtures remain exact.

- [ ] **Step 6: Replace the obsolete private-inner test with real-frame boundary tests**

Delete `dashboard_block_inners_remain_within_tiny_parents`; Task 2 now owns zero-sized, clipped-edge, and maximum-coordinate panel geometry. Keep Dashboard's existing public `render(frame)` contract and fixed-date `render_on(frame, today)` seam—do not add an area parameter solely for tests. Add:

```rust
#[test]
fn dashboard_renders_one_cell_small_and_resized_frames_without_panicking() {
    let screen = golden_dashboard_screen("normal");
    for (width, height) in [(1, 1), (40, 16), (80, 24), (80, 35), (107, 35)] {
        crate::tui::framework::test_support::render_to_buffer(width, height, |frame| {
            screen.render_on(frame, dashboard_fixture_today());
        });
    }
}

#[test]
fn dashboard_recalculates_complete_output_after_resize() {
    let screen = golden_dashboard_screen("normal");
    for (width, height) in [(80, 35), (80, 24), (40, 16), (107, 35)] {
        let output = render_dashboard(&screen, width, height);
        assert_eq!(output.lines().count(), height as usize);
    }
}
```

- [ ] **Step 7: Resolve only proven Paragraph allowlist movement**

Run `cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture`.

If it fails only because the four existing Dashboard sites moved, map every old site to the identical widget and update only those four literals. New variable content must use `.wrap(Wrap { trim: false })`, not a new allowlist entry. Stop if a semantic or out-of-scope failure appears.

- [ ] **Step 8: Run task gates and commit**

Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::dashboard::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --check
git diff --exit-code "$DASHBOARD_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/dashboard
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code "$(git log -1 --format=%H --diff-filter=A -- tests/fixtures/tui/goals)"..HEAD -- tests/fixtures/tui/goals
```

Expected: every command exits `0`, strict Clippy has zero warnings, and fixtures are unchanged.

Commit:

```bash
git add src/tui/dashboard.rs tests/tui_lint_test.rs
git commit -m "refactor(dashboard): adopt shared presentation components"
```

---

### Task 4: Document, Release-Gate, and Hand Off Batch 3B-6

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/dashboard-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed Dashboard migration, final test counts, `DASHBOARD_FIXTURE_BASE`, and approved terminology.
- Produces: accurate docs, clean release-gated branch, and Dashboard-only binary.

- [ ] **Step 1: Update active documentation**

- `CLAUDE.md`: add `static_panel.rs` to the framework map; mark Batch 3B-6 shipped; leave only Log Entry extraction unshipped; state Dashboard consumes `ScreenFrameSections` and `StaticPanel` but owns domain rendering and behavior.
- `README.md` and `README_zh.md`: mark Dashboard extraction shipped, keep Log Entry unshipped, and claim no visual/shortcut change.
- `docs/specs/dashboard-spec.md`: add shared frame/four-panel ownership while retaining every current responsive, content, shortcut, HRV, lifecycle, and removal rule.
- `docs/specs/navigation-spec.md`: state Batch 3B-6 changes presentation ownership only, not routes, intents, preparation, re-entry, or shortcuts.
- framework design: add `### Batch 3B-6: Dashboard Presentation Components — Complete`, describing three immutable fixtures, deterministic Heatmap seam, shared frame/static panel, safe geometry, and retained domain ownership; leave only Log Entry unshipped.

- [ ] **Step 2: Audit documentation claims**

Run:

```bash
rg -n "Dashboard responsive-component extraction.*unshipped|Dashboard 响应式组件抽取.*未交付" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
rg -n "Batch 3B-6|StaticPanel|static_panel|Log Entry.*unshipped|Log Entry.*尚未交付" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
```

Expected: first command exits `1`; second finds shipped Dashboard ownership and unshipped Log Entry statements.

- [ ] **Step 3: Run fresh focused and full gates**

Run:

```bash
cargo test tui::dashboard::tests -- --nocapture
cargo test tui::framework::static_panel::tests -- --nocapture
cargo test tui::framework::screen_frame::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check
test -x target/debug/iron
```

Expected: every command passes, strict Clippy has zero warnings, and fresh per-target test counts are recorded.

- [ ] **Step 4: Prove fixture immutability and commit docs**

Run:

```bash
git diff --exit-code "$DASHBOARD_FIXTURE_BASE"..HEAD -- tests/fixtures/tui/dashboard
git diff --exit-code 70dae36..HEAD -- tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends tests/fixtures/tui/history
git diff --exit-code "$(git log -1 --format=%H --diff-filter=A -- tests/fixtures/tui/goals)"..HEAD -- tests/fixtures/tui/goals
```

Expected: all exit `0`. Then commit:

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/dashboard-spec.md docs/specs/navigation-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Dashboard component ownership"
```

- [ ] **Step 5: Perform an independent whole-slice review**

Review from the design commit's parent through `HEAD`. Report Critical, Important, and Minor findings with exact evidence. Verify fixture capture precedes migration; no updater remains; production Heatmap uses local today; the exact seven constraints and `trailing_flex = false` are used; one frame/four panels render; status/footer render once; geometry is safe; other screens/fixtures are unchanged; lint ignores non-production decoys; and docs leave Log Entry unshipped.

Fix every Critical or Important finding through a separate focused RED/GREEN commit, rerun all gates, and re-review the focused fix. Record accepted Minor findings explicitly without expanding scope silently.

- [ ] **Step 6: Hand off the binary and stop**

Run:

```bash
git status --short --untracked-files=all
git log --oneline --decorate -8
test -x target/debug/iron
```

Expected: clean worktree, reviewable commits, executable binary. Ask the user to verify Normal `80x35`, long content, HRV success/cancel/validation/write failure, quit confirmation, resize through `80x24` and `40x16`, all Dashboard routes, fresh data on return, and other-screen regressions.

Stop after the Dashboard-only handoff. Do not merge and do not begin Log Entry until explicit acceptance.

## Completion Checklist

- [ ] Three deterministic pre-migration Dashboard fixtures are committed and immutable.
- [ ] Production Heatmap still uses current local date.
- [ ] `render_static_panel` owns clipped border/title/padding geometry and no domain state.
- [ ] Dashboard uses one shared frame and four static panels.
- [ ] Dashboard owns no local bordered panel and retains domain formatting and behavior.
- [ ] `80x35` output is exact; smaller/pathological rectangles are safe.
- [ ] Existing component APIs and accepted non-Dashboard fixtures are unchanged.
- [ ] Ownership lint rejects local presentation and ignores non-production decoys.
- [ ] Documentation marks Batch 3B-6 shipped and Log Entry unshipped.
- [ ] Formatting, strict Clippy, full tests, build, diff, fixture, and binary gates pass.
- [ ] Independent review has no unresolved Critical or Important finding.
- [ ] User receives a Dashboard-only binary and implementation stops for manual acceptance.
