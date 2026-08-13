# TUI Consistency Batch 3B-3 Trends Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Trends to the proven screen-frame and scrollable-list presentation components without changing its rendered output or behavior.

**Architecture:** Trends keeps ownership of filtering, practice-row formatting, chart data, Sparkline rendering, statistics, time-window controls, shortcuts, navigation, lifecycle, and failure retention. `ScreenFrame` owns the one-pass filter/list/chart/status/shortcut layout, while `ScrollableListPanel` owns the practice-list border, overflow, viewport, visible rows, and selected highlight. Six immutable styled-buffer fixtures bind the accepted pre-refactor output.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Chrono 0.4, Fluent localization, the existing TUI framework, Cargo tests, Clippy, and Ratatui `TestBackend`.

## Global Constraints

- Base implementation commit is `d9dac9f42b154cb02f5588728ed5546762595096`; approved design commit is `6aea597` on `main`.
- Follow [the approved Batch 3B-3 design](../specs/2026-08-13-tui-consistency-batch-3b-trends-components-design.md).
- Batch 3B-3 migrates Trends only. History, Goals, Dashboard, Log Entry, and later slices remain unchanged.
- Preserve Trends output, shortcuts, filtering, selection wrapping, chronological chart direction, statistics, 30–365 day window, navigation, lifecycle, and failure behavior. The six `80x24` styled-buffer fixtures may not change during migration.
- Practices and Quotes fixtures must remain unchanged.
- Trends retains database access, stable practice IDs, filter state, domain-row formatting, chart entries, Sparkline/statistics policy, status creation, key handling, shortcut selection, navigation, and lifecycle behavior.
- Framework components remain presentation-only. They do not import `Database`, `Practice`, `LogEntry`, filter state, chart state, navigation intents, `anyhow`, or business errors.
- Use `ScreenFrame` body constraints `[Length(1), Percentage(40), Min(5)]` in that order, with trailing flex disabled.
- Do not extract a shared filter, chart, statistics, or composite list-chart component. Do not use `ActionPanel` in Trends.
- All changed rectangle arithmetic is saturating or clamped. Empty, zero-sized, one-cell, narrow, short, and edge-positioned targets must not panic or escape their parent/frame.
- Do not change database schema, localization copy, navigation topology, colors, borders, padding, spacing, or keyboard commands.
- Do not add speculative configuration for later screens.
- Do not reintroduce Quick Log, LLM configuration/runtime, Abbreviations, or the `w` action.
- Every production change follows RED-GREEN-refactor. Record the expected failure before implementing the corresponding behavior.
- Fixture generation exists only long enough to capture the old implementation and is removed before the characterization commit.
- Structural ownership lint must inspect executable source. Comments, ordinary strings, raw strings, and character literals do not count as delegation evidence; Rust lifetime syntax must remain visible to the scanner.
- Paragraph lint remains recursive and keeps its current four-line scan window. Refresh an allowlist line only after proving the old and new lines are the same approved widget.
- Stop for a scope ruling if a fixture would need to change, an existing test conflicts with a fixture, a user-facing behavior change appears necessary, or a tracked file outside this plan is required.
- Use four planned commits: immutable Trends fixtures; accepted Trends frame contract; Trends migration plus structural lint; active documentation. Any review-discovered correctness fix uses a separate follow-up commit with focused RED/GREEN evidence.
- Stop after the Trends-only manual-acceptance handoff. Do not begin History until the user explicitly accepts Trends.

## File Structure

**Create:**

- `tests/fixtures/tui/trends/browse_with_chart.txt` — selected practice with populated chronological chart and statistics.
- `tests/fixtures/tui/trends/filtering_long_input.txt` — active long filter with bounded visible cursor.
- `tests/fixtures/tui/trends/no_filter_match.txt` — empty filtered list and cleared chart.
- `tests/fixtures/tui/trends/list_overflow.txt` — last selected practice and upward overflow marker.
- `tests/fixtures/tui/trends/no_log_data.txt` — selected practice with the existing no-data chart state.
- `tests/fixtures/tui/trends/cached_chart_error.txt` — retained chart plus visible error status.

**Modify:**

- `src/tui/trends.rs` — deterministic fixtures, component-based render path, domain list-content builder, and tiny-terminal regressions.
- `src/tui/framework/screen_frame.rs` — test-only proof that the generic sections renderer preserves the accepted Trends allocation.
- `tests/tui_lint_test.rs` — transfer Trends list-border ownership from the direct-border group to executable `ScrollableListPanel` delegation.
- `CLAUDE.md` — mark Trends as a shipped frame/list consumer while recording its specialized filter/chart ownership.
- `README.md` — mark Batch 3B-3 Trends as shipped without claiming History or later slices.
- `README_zh.md` — Chinese equivalent of the shipped status.
- `docs/specs/trends-spec.md` — document shared frame/list ownership and unchanged chart behavior.
- `docs/specs/navigation-spec.md` — state that Batch 3B-3 changes presentation composition only.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — add the completed Trends slice and leave later work unshipped.

**Do not modify unless a verified source-line move requires a mechanical mapping refresh:**

- Paragraph allowlist literals in `tests/tui_lint_test.rs`.

---

### Task 1: Characterize All Trends Presentation States

**Files:**
- Modify: `src/tui/trends.rs:528-862`
- Create: `tests/fixtures/tui/trends/browse_with_chart.txt`
- Create: `tests/fixtures/tui/trends/filtering_long_input.txt`
- Create: `tests/fixtures/tui/trends/no_filter_match.txt`
- Create: `tests/fixtures/tui/trends/list_overflow.txt`
- Create: `tests/fixtures/tui/trends/no_log_data.txt`
- Create: `tests/fixtures/tui/trends/cached_chart_error.txt`

**Interfaces:**
- Consumes: existing `render_to_snapshot(width, height, render)` from `src/tui/framework/test_support.rs`, current `TrendsScreen::render`, private test access to Trends state, and model types `Log`, `LogEntry`, `Practice`, `PracticeType`, `Set`, and `SetData`.
- Produces: six compile-time styled-buffer fixtures and `trends_states_match_golden_buffers`, which Task 3 must pass without fixture changes.

- [ ] **Step 1: Add deterministic Trends fixture builders**

In the Trends test module, replace the current model/test-support imports with:

```rust
use crate::model::{Log, PracticeType, Set, SetData};
use crate::tui::framework::test_support::{render_to_snapshot, render_to_string};
```

Add fixed helpers that never use the current date or an external database:

```rust
fn fixed_time(month: u32, day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, month, day)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap()
}

fn fixed_practice(id: i64, name: impl Into<String>) -> Practice {
    Practice {
        id,
        name: name.into(),
        practice_type: PracticeType::Bodyweight,
        created_at: fixed_time(1, 1),
        active: true,
    }
}

fn fixed_entry(id: i64, practice: &Practice, month: u32, day: u32, reps: i32) -> LogEntry {
    LogEntry {
        log: Log {
            id,
            practice_id: practice.id,
            logged_at: fixed_time(month, day),
            note: None,
            warm_up: None,
            cool_down: None,
            rpe: None,
        },
        practice_name: practice.name.clone(),
        practice_type: practice.practice_type,
        sets: vec![Set {
            id: id * 10,
            log_id: id,
            set_number: 1,
            data: SetData::Bodyweight { reps },
        }],
        rpe: None,
    }
}

fn fixed_chart(practice: &Practice) -> Vec<LogEntry> {
    vec![
        fixed_entry(101, practice, 4, 28, 10),
        fixed_entry(102, practice, 5, 2, 20),
        fixed_entry(103, practice, 5, 9, 30),
        fixed_entry(104, practice, 5, 16, 40),
    ]
}
```

These dates deliberately cross a month boundary so the fixture binds both day and month labels. Values increase so the fixture binds positive trend color and text.

- [ ] **Step 2: Build all six reachable presentation states**

Add:

```rust
fn golden_trends_screen(name: &str) -> TrendsScreen {
    std::env::set_var("LC_ALL", "C");
    crate::i18n::init();

    let practices = vec![
        fixed_practice(1, "Bench Press"),
        fixed_practice(2, "Pull-up"),
        fixed_practice(3, "Ring Dip"),
    ];
    let chosen = practices[0].clone();
    let mut screen = TrendsScreen {
        practices,
        filtered_indices: vec![0, 1, 2],
        filter: TextInput::new(),
        filtering: false,
        selected: 0,
        list_viewport: ViewportState::new(0),
        chosen_practice: Some(chosen.clone()),
        days_window: 90,
        entries: fixed_chart(&chosen),
        needs_refresh: false,
        status_msg: None,
    };

    match name {
        "browse_with_chart" => {}
        "filtering_long_input" => {
            let long = "训练".repeat(50);
            screen.practices[0].name = long.clone();
            screen.filter = TextInput::from(long);
            screen.filtering = true;
            screen.filtered_indices = vec![0];
            screen.chosen_practice = Some(screen.practices[0].clone());
            screen.entries = fixed_chart(&screen.practices[0]);
        }
        "no_filter_match" => {
            screen.filter = TextInput::from("missing practice".to_owned());
            screen.filtering = true;
            screen.filtered_indices.clear();
            screen.selected = 0;
            screen.chosen_practice = None;
            screen.entries.clear();
        }
        "list_overflow" => {
            screen.practices = (1..=30)
                .map(|id| fixed_practice(id, format!("Practice {id:02}")))
                .collect();
            screen.filtered_indices = (0..screen.practices.len()).collect();
            screen.selected = screen.practices.len() - 1;
            screen.list_viewport.set_offset(25);
            screen.chosen_practice = Some(screen.practices[screen.selected].clone());
            screen.entries = fixed_chart(screen.chosen_practice.as_ref().unwrap());
        }
        "no_log_data" => screen.entries.clear(),
        "cached_chart_error" => {
            screen.status_msg = Some(("Fixture chart query failed".into(), true));
        }
        other => panic!("unknown Trends golden state: {other}"),
    }

    screen
}
```

Keep all six states in one test so locale initialization and fixture capture are serialized within each test process.

- [ ] **Step 3: Add the temporary fixture update path**

Create each fixture with one newline, then add:

```rust
#[test]
fn trends_states_match_golden_buffers() {
    for name in [
        "browse_with_chart",
        "filtering_long_input",
        "no_filter_match",
        "list_overflow",
        "no_log_data",
        "cached_chart_error",
    ] {
        let mut screen = golden_trends_screen(name);
        let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
        let path = format!(
            "{}/tests/fixtures/tui/trends/{name}.txt",
            env!("CARGO_MANIFEST_DIR")
        );
        if std::env::var_os("UPDATE_TRENDS_GOLDENS").is_some() {
            std::fs::write(&path, &actual).expect("Trends fixture should be written");
        }
        let expected = std::fs::read_to_string(&path).expect("Trends fixture should exist");
        assert_eq!(actual, expected, "Trends {name} buffer changed");
    }
}
```

- [ ] **Step 4: Run the fixture test and record RED**

Run:

```bash
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
```

Expected: FAIL on `browse_with_chart` because the one-line fixture does not equal the current `80x24` styled buffer. Record the failing assertion before generating fixtures.

- [ ] **Step 5: Capture the old renderer exactly once**

Run:

```bash
UPDATE_TRENDS_GOLDENS=1 cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
wc -l tests/fixtures/tui/trends/*.txt
```

Expected: the test passes and every fixture reports `1921` lines: one area header plus `80 * 24` cell records.

- [ ] **Step 6: Remove every runtime fixture-update path**

Add the compile-time selector:

```rust
fn trends_golden(name: &str) -> &'static str {
    match name {
        "browse_with_chart" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/browse_with_chart.txt"
        )),
        "filtering_long_input" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/filtering_long_input.txt"
        )),
        "no_filter_match" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/no_filter_match.txt"
        )),
        "list_overflow" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/list_overflow.txt"
        )),
        "no_log_data" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/no_log_data.txt"
        )),
        "cached_chart_error" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tui/trends/cached_chart_error.txt"
        )),
        other => panic!("unknown Trends golden fixture: {other}"),
    }
}
```

Replace the environment/write/read block with:

```rust
let expected = trends_golden(name);
assert_eq!(actual, expected, "Trends {name} buffer changed");
```

Verify the source no longer contains an update or runtime fixture path:

```bash
rg -n "UPDATE_TRENDS_GOLDENS|std::fs::write|read_to_string" src/tui/trends.rs
```

Expected: no matches.

- [ ] **Step 7: Run characterization and baseline suites**

```bash
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo fmt -- --check
git diff --check
```

Expected: all fixtures pass in both library and binary targets, all existing Trends tests pass, lint passes, and formatting/diff checks exit zero.

- [ ] **Step 8: Commit immutable characterization**

```bash
git add src/tui/trends.rs tests/fixtures/tui/trends
git commit -m "test(trends): capture presentation states"
```

The commit must contain no framework or production render change.

---

### Task 2: Bind ScreenFrame to the Accepted Trends Layout

**Files:**
- Modify: `src/tui/framework/screen_frame.rs:84-257`
- Modify: `src/tui/trends.rs:844-862`

**Interfaces:**
- Consumes: existing `render_screen_frame_sections`, `ScreenFrameSections`, `render_to_buffer`, and Task 1's deterministic Trends screen states.
- Produces: a generic frame-allocation regression proving the exact legacy Trends constraints remain equivalent, plus baseline tiny-terminal safety that Task 3 must retain.

- [ ] **Step 1: Add the exact legacy-layout equivalence test**

In `screen_frame.rs` tests, add:

```rust
#[test]
fn screen_frame_sections_match_accepted_trends_layout() {
    let shortcuts = vec![
        Shortcut::new("j/k", "Navigate"),
        Shortcut::new("h/l", "Window"),
        Shortcut::new("/", "Filter"),
        Shortcut::new("Esc", "Back"),
    ];
    let area = Rect::new(0, 0, 80, 24);
    let legacy = Layout::vertical([
        Constraint::Length(1),
        Constraint::Percentage(40),
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Length(shortcut_height(&shortcuts, area.width)),
    ])
    .split(area);
    let mut sections = None;

    render_to_buffer(80, 24, |frame| {
        sections = Some(render_screen_frame_sections(
            frame,
            frame.area(),
            107,
            &Some(("Fixture status".into(), true)),
            &shortcuts,
            &[
                Constraint::Length(1),
                Constraint::Percentage(40),
                Constraint::Min(5),
            ],
            false,
        ));
    });

    let sections = sections.unwrap();
    assert_eq!(sections.area, area);
    assert_eq!(sections.body.as_slice(), &legacy[..3]);
    assert_eq!(sections.status, legacy[3]);
    assert_eq!(sections.shortcuts, legacy[4]);
}
```

This is a characterization test for the already-shipped generic API. It must pass without production changes.

- [ ] **Step 2: Run the frame characterization**

```bash
cargo test tui::framework::screen_frame::tests::screen_frame_sections_match_accepted_trends_layout -- --exact --nocapture
```

Expected: PASS in both targets. A failure means the approved component API cannot preserve the current Trends allocation; stop for a scope ruling instead of changing the fixture or constraints.

- [ ] **Step 3: Add baseline tiny and resized Trends rendering coverage**

In the Trends tests, add:

```rust
#[test]
fn trends_states_render_in_small_and_resized_terminals_without_panicking() {
    for name in [
        "browse_with_chart",
        "filtering_long_input",
        "no_filter_match",
        "list_overflow",
        "no_log_data",
        "cached_chart_error",
    ] {
        let mut screen = golden_trends_screen(name);
        for (width, height) in [(1, 1), (12, 4), (24, 10), (40, 6), (80, 24), (20, 5)] {
            let _ = render_to_snapshot(width, height, |frame| screen.render(frame));
        }
        let resized = render_to_string(80, 24, |frame| screen.render(frame));
        if !screen.filtered_indices.is_empty() {
            let selected_name = &screen.practices[screen.filtered_indices[screen.selected]].name;
            let visible_prefix = truncate_to_width(selected_name, 8);
            assert!(
                resized.contains(&visible_prefix),
                "selected practice should be visible after resize: {name}\n{resized}"
            );
        }
    }
}
```

The eight-column prefix is intentionally narrower than the characterized `80x24` Name column while still binding selection reveal to the selected domain row; do not weaken this to a no-panic-only test.

- [ ] **Step 4: Run the geometry and fixture suites**

```bash
cargo test tui::framework::screen_frame::tests -- --nocapture
cargo test tui::trends::tests::trends_states_render_in_small_and_resized_terminals_without_panicking -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo fmt -- --check
git diff --check
```

Expected: all commands pass and all fixtures remain exact. If a baseline tiny target exposes an existing escaping chart rectangle, preserve the failing test and stop for a focused correctness ruling rather than mixing an unplanned chart change into the component migration.

- [ ] **Step 5: Commit the accepted frame and geometry contract**

```bash
git add src/tui/framework/screen_frame.rs src/tui/trends.rs
git commit -m "test(tui): bind Trends frame geometry"
```

This commit contains tests only.

---

### Task 3: Migrate Trends to Shared Presentation Components

**Files:**
- Modify: `src/tui/trends.rs:1-342`
- Modify: `tests/tui_lint_test.rs:307-350`

**Interfaces:**
- Consumes:

```rust
pub fn render_screen_frame_sections(
    frame: &mut Frame,
    full: Rect,
    max_width: u16,
    status: &StatusMessage,
    shortcuts: &[Shortcut],
    body_constraints: &[Constraint],
    trailing_flex: bool,
) -> ScreenFrameSections;

pub fn render_scrollable_list_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    title: impl Into<String>,
    viewport: &mut ViewportState,
    build_content: impl FnOnce(Rect) -> ListPanelContent<'a>,
) -> ListPanelAreas;
```

- Produces: one Trends render path that delegates frame/list mechanics while retaining Trends-owned filter and chart presentation; structural lint requires executable Trends delegation.

- [ ] **Step 1: Transfer Trends list ownership in the structural lint**

Change the direct-border group to:

```rust
let required_screens = ["goals.rs", "history.rs", "log_entry.rs"];
```

Change the delegated group to:

```rust
for (filename, screen_name) in [
    ("practices.rs", "Practices"),
    ("quotes_screen.rs", "Quotes"),
    ("trends.rs", "Trends"),
] {
```

Do not alter `source_code_contains`, its string/comment/lifetime tests, or the Paragraph lint.

- [ ] **Step 2: Run the ownership lint and record RED**

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
```

Expected: FAIL with `Trends must delegate its bordered list to ScrollableListPanel`.

- [ ] **Step 3: Replace the outer layout with ScreenFrame sections**

Update the framework imports in `trends.rs` to consume:

```rust
framework::{
    move_selection, render_screen_frame_sections, render_scrollable_list_panel,
    truncate_to_width, EdgeBehavior, ListPanelContent, Shortcut, TextInput,
    ViewportState,
},
```

Remove production imports of `centered_area`, `highlight_row`, `render_shortcuts`, `render_status_line`, `shortcut_height`, and `VisualSpan`. Keep `Block`, `Borders`, `Padding`, `Layout`, and `Direction` because the specialized chart still owns them. Import `Rect` with the existing Ratatui layout imports.

Replace `render` with:

```rust
pub fn render(&mut self, frame: &mut Frame) {
    let shortcuts = vec![
        Shortcut::new("j/k", tr("key-navigate")),
        Shortcut::new("h/l", tr("key-window")),
        Shortcut::new("/", tr("key-filter")),
        Shortcut::new("Esc", tr("key-back")),
    ];
    let screen = render_screen_frame_sections(
        frame,
        frame.area(),
        CONTENT_WIDTH,
        &self.status_msg,
        &shortcuts,
        &[
            Constraint::Length(1),
            Constraint::Percentage(40),
            Constraint::Min(5),
        ],
        false,
    );

    self.render_filter(frame, screen.body[0]);
    self.render_practice_list(frame, screen.body[1]);
    self.render_sparkline_panel(frame, screen.body[2]);
}
```

Do not derive status or shortcuts rectangles locally.

- [ ] **Step 4: Keep filter presentation Trends-owned**

Extract the filter part of the old `render_practice_list` into:

```rust
fn render_filter(&self, frame: &mut Frame, area: Rect) {
    let filter_style = if self.filtering {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(Color::Gray)
    };
    let mut spans = if self.filtering || !self.filter.value().is_empty() {
        vec![Span::styled(" /", filter_style)]
    } else {
        vec![Span::styled(
            format!(" {}", tr("log-press-filter")),
            filter_style,
        )]
    };
    if self.filtering {
        spans.extend(self.filter.visible_spans(area.width, 2, ACCENT));
    } else if !self.filter.value().is_empty() {
        spans.push(Span::styled(self.filter.value().to_owned(), filter_style));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }),
        area,
    );
}
```

This is the old filter rendering with only the parameter name changed.

- [ ] **Step 5: Build all logical list rows from the component inner width**

Add a presentation-only associated helper:

```rust
fn practice_list_content(
    practices: &[Practice],
    filtered_indices: &[usize],
    selected: usize,
    inner: Rect,
) -> ListPanelContent<'static> {
    let name_header = tr("practices-col-name");
    let type_header = tr("practices-col-type");
    let max_name_width = inner.width.saturating_sub(type_header.width() as u16 + 5) as usize;
    let desired_name_width = practices
        .iter()
        .map(|practice| practice.name.width())
        .max()
        .unwrap_or(0)
        .max(name_header.width());
    let name_width = desired_name_width.min(max_name_width);
    let col_width = name_width.saturating_add(2);
    let header_padding = col_width.saturating_sub(name_header.width());
    let header_style = Style::default().fg(Color::White).bold();
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(name_header, header_style),
        Span::raw(" ".repeat(header_padding)),
        Span::styled(type_header, header_style),
    ]);
    let rows = filtered_indices
        .iter()
        .enumerate()
        .map(|(row, index)| {
            let practice = &practices[*index];
            let selected_style = if row == selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let name = truncate_to_width(&practice.name, name_width);
            let padding = col_width.saturating_sub(name.width());
            Line::from(vec![
                Span::styled(if row == selected { "> " } else { "  " }, selected_style),
                Span::styled(name, selected_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(practice.practice_type.label(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    ListPanelContent {
        header: Some(header),
        rows,
        empty: None,
        selected: (!filtered_indices.is_empty()).then_some(selected),
    }
}
```

All strings are owned, so returning `ListPanelContent<'static>` does not leak a screen borrow. Preserve the current empty-list output: the table header remains and no new empty-state copy is introduced.

- [ ] **Step 6: Delegate the practice list to ScrollableListPanel**

Replace the local list block, viewport, slice, Paragraph, and highlight code with:

```rust
fn render_practice_list(&mut self, frame: &mut Frame, area: Rect) {
    let practices = &self.practices;
    let filtered_indices = &self.filtered_indices;
    let selected = self.selected;
    render_scrollable_list_panel(
        frame,
        area,
        tr("trends-title"),
        &mut self.list_viewport,
        |inner| Self::practice_list_content(practices, filtered_indices, selected, inner),
    );
}
```

The component now owns border/title/overflow, viewport height/reveal/clamp, visible-row slicing, and selected-row highlight. Do not add an `ActionPanel` call or change `render_sparkline_panel`.

- [ ] **Step 7: Restore the test-only VisualSpan import**

The existing `reveal_selection` helper still directly characterizes viewport behavior. Import its type inside the test module:

```rust
use crate::tui::framework::{test_support::{render_to_snapshot, render_to_string}, VisualSpan};
```

Do not keep a production-only `VisualSpan` import just for tests.

- [ ] **Step 8: Run lint and fixture GREEN**

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_render_in_small_and_resized_terminals_without_panicking -- --exact --nocapture
git diff --exit-code 1463268..HEAD -- tests/fixtures/tui/trends
```

Expected: ownership lint passes, all six fixtures pass in library and binary targets, all small/resized cases pass, and fixtures have not changed since their characterization commit. Any fixture difference is a stop condition; do not regenerate it.

- [ ] **Step 9: Run screen, framework, and compatibility regressions**

```bash
cargo test tui::trends::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
git diff --check
```

Expected: all commands exit zero, Clippy has no warnings, and the Practices and Quotes fixtures remain exact.

- [ ] **Step 10: Audit duplicate Trends presentation mechanics**

```bash
sed -n '1,/^#\[cfg(test)\]/p' src/tui/trends.rs | \
  rg -n "centered_area|render_status_line|render_shortcuts|shortcut_height|highlight_row|list_viewport\.(set_height|reveal|overflow)"
rg -n "render_screen_frame_sections|render_scrollable_list_panel|render_sparkline_panel|Block::default|Borders::ALL" src/tui/trends.rs
rg -n "\.borders\(Borders::ALL\)|viewport\.(reveal|clamp|overflow)|highlight_row" src/tui/framework/list_panel.rs
```

Expected:

- the first command has no production matches;
- the second finds both shared delegates and the intentionally Trends-owned chart block/borders; and
- the third finds centralized list presentation behavior.

`list_viewport.clamp` remains permitted in `Reenter`, and direct border rendering remains permitted only inside the specialized chart panel.

- [ ] **Step 11: Commit the Trends migration and lint transfer**

```bash
git add src/tui/trends.rs tests/tui_lint_test.rs
git commit -m "refactor(trends): adopt shared presentation components"
```

Do not include fixtures, docs, locales, database, router, framework production code, or another screen in this commit.

---

### Task 4: Document, Verify, Review, and Hand Off Trends-Only

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/trends-spec.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`

**Interfaces:**
- Consumes: completed Trends migration, immutable Trends/Practices/Quotes fixtures, and existing release tests.
- Produces: accurate shipped-state documentation, a fully verified branch, and a Trends-only manual-acceptance binary.

- [ ] **Step 1: Update active documentation precisely**

Make these semantic changes and no broader historical rewrites:

- `CLAUDE.md`: state that Practices and Quotes use all applicable presentation components, while Trends uses `ScreenFrame` and `ScrollableListPanel` and retains its filter and specialized Sparkline/statistics policy.
- `README.md`: change the rollout statement to “Batch 3B-1 Practices, Batch 3B-2 Quotes, and Batch 3B-3 Trends presentation migrations are shipped; History and later slices are not shipped.”
- `README_zh.md`: add the exact Chinese equivalent without implying History or another screen migrated.
- `docs/specs/trends-spec.md`: document frame/list ownership, immutable visual compatibility, and unchanged filtering/chart/time-window/error/re-entry behavior.
- `docs/specs/navigation-spec.md`: state Batch 3B-3 changes only Trends presentation composition and does not change routes, intents, re-entry, or shortcuts.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`: add a completed Batch 3B-3 Trends section, identify the filter and chart as Trends-owned, and leave History, Goals popups, Dashboard responsive layout, and Log Entry forms unshipped.

Do not edit locale files or user-facing Trends copy.

- [ ] **Step 2: Run documentation and removal audits**

```bash
rg -n "Trends and later slices are not shipped|Trends 及后续切片尚未交付|Batch 3B-2 changes only Quotes" \
  CLAUDE.md README.md README_zh.md docs/specs docs/superpowers/specs
rg -n "Batch 3B-3|Trends|History.*not shipped|History.*尚未交付" \
  CLAUDE.md README.md README_zh.md docs/specs/trends-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: stale active rollout claims are gone; the new claims identify only Practices, Quotes, and Trends as shipped component consumers; History remains unshipped; the removed-feature guard passes.

- [ ] **Step 3: Run fresh final release gates**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::framework:: -- --nocapture
cargo test tui::trends::tests -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --exit-code 1463268..HEAD -- tests/fixtures/tui/trends
git diff --check 6aea597..HEAD
git diff --check
```

Expected: every command exits zero; Clippy emits no warnings; all six Trends, five Practices, and six Quotes fixtures remain unchanged; the full suite and build pass.

- [ ] **Step 4: Perform scoped pre-commit self-review**

```bash
git diff --stat
git diff -- src/tui/trends.rs src/tui/framework/screen_frame.rs tests/tui_lint_test.rs
git diff -- CLAUDE.md README.md README_zh.md docs/specs/trends-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git status --short --untracked-files=all
```

Confirm:

- no fixture changed after Task 1;
- no Trends handler, database query, lifecycle, intent, shortcut, localization copy, chart algorithm, or statistics formula changed;
- `ScreenFrame` receives exactly the three accepted body constraints with trailing flex disabled;
- `ScrollableListPanel` is the only owner of Trends list border/title/overflow/viewport/highlight mechanics;
- the filter and Sparkline/statistics panel remain Trends-owned;
- shared components import no Trends or database types;
- every variable-length Paragraph remains wrapped;
- lint requires executable Practices, Quotes, and Trends delegation while retaining direct-border requirements for unmigrated screens; and
- docs identify History and later slices as unshipped.

- [ ] **Step 5: Commit active documentation**

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/trends-spec.md \
  docs/specs/navigation-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document Trends component migration"
```

- [ ] **Step 6: Request a final whole-slice read-only review**

Review `6aea597..HEAD` against the approved design and this plan. Require explicit findings by Critical, Important, and Minor severity for:

- completeness, determinism, and immutability of all six Trends fixtures;
- exact Trends output parity and unchanged Practices/Quotes fixtures;
- frame/list containment on normal and undersized targets;
- component/domain ownership boundaries;
- filter cursor visibility, selection wrapping, overflow, and resize reveal;
- chart chronological direction, month labels, statistics, and no-data behavior;
- in-place query failure retention and re-entry context;
- recursive Paragraph lint and executable delegated-border lint accuracy;
- absence of duplicate Trends frame/list mechanics;
- four-commit planned scope and documentation accuracy; and
- confirmation that History and all other unplanned screens are unchanged.

Resolve every valid Critical or Important finding with a focused failing regression first. Keep review fixes in separate commits and rerun every release gate in Step 3. Resolve Minor findings that are safe and in scope; otherwise record an explicit user-approved deferral before handoff.

- [ ] **Step 7: Confirm final handoff state**

```bash
git status --short --untracked-files=all
git log --oneline 6aea597..HEAD
test -x target/debug/iron
```

Expected: tracked status is clean, commits match the approved slice plus any separately reviewed fixes, and the binary exists.

- [ ] **Step 8: Provide Trends-only manual acceptance**

Give the user the built binary and ask them to verify:

1. Practice selection with `j/k` and arrows, wrap behavior, scrolling, truncation, title overflow markers, and selected highlight.
2. `/` filtering, long Unicode input, visible cursor, no-match behavior, and Enter/Esc leaving filter mode.
3. Populated Sparkline direction, most-recent value on the right, date/month labels, colors, average, peak, and positive/negative trend display.
4. A selected practice with no logs and filtering to no selected practice.
5. `h/l` and arrows across the 30–365 day range.
6. Retained selected practice/window/chart plus visible status after a simulated query failure.
7. Normal, narrow, short, tiny, resized, and re-expanded terminals.
8. Dashboard-to-Trends navigation, return, external-data reload, stable-ID context restoration, and atomic navigation failure behavior.

Stop after this handoff. Do not design or implement History until the user explicitly accepts Trends.
