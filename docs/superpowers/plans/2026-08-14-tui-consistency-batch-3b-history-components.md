# TUI Consistency Batch 3B-4 History Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate History to the proven screen-frame and scrollable-list presentation components without changing its accepted rendered output or behavior.

**Architecture:** History keeps ownership of filtering, dynamic log-column formatting, floating and focused detail presentation, database mutations, edit navigation, shortcuts, failure retention, and lifecycle re-entry. `ScreenFrame` owns the one-pass filter/list/status/shortcut layout, while `ScrollableListPanel` owns the bordered list, fixed header, logical-row viewport, visible slicing, overflow, empty state, and selected highlight. Seven immutable styled-buffer fixtures bind the pre-refactor `Table` output while History converts those cells to equivalent owned `Line` values.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Chrono 0.4, Fluent localization, Unicode Width, the existing TUI framework, Cargo tests, Clippy, and Ratatui `TestBackend`.

## Global Constraints

- Base product implementation commit is `aea0c5bbe6332e4e0ecf2083ceaf96643fac83a5`; approved design commit is `e5764978b2f45a908c0d837f552e2affd0ffb684` on `main`.
- Follow [the approved Batch 3B-4 design](../specs/2026-08-14-tui-consistency-batch-3b-history-components-design.md).
- Batch 3B-4 migrates History only. Goals, Dashboard, Log Entry, and later presentation slices remain unchanged.
- Preserve History output, copy, colors, spacing, shortcuts, bounded list selection, filtering, Detail focus/scrolling, edit navigation, delete/undo behavior, database behavior, failure retention, and lifecycle behavior except for the approved correction that defers detail-viewport clamping until the real popup width is rendered. The seven `80x24` styled-buffer fixtures may not change during migration.
- Practices, Quotes, and Trends fixtures must remain unchanged.
- History retains `LogEntry` data, filtered indices, stable log IDs, filter state, domain formatting, popup policy, modes, database access, undo payload, status creation, key handling, shortcut selection, navigation, and re-entry.
- Framework components remain presentation-only. They do not import `Database`, `LogEntry`, History modes, filter state, navigation intents, undo state, `anyhow`, or business errors.
- Use `ScreenFrame` body constraints `[Length(1), Min(1)]` in that order, with trailing flex disabled.
- Use `ScrollableListPanel` as the sole owner of History list border, title/overflow, fixed header, viewport reveal/clamp, visible-row slicing, empty state, and selected-row highlight.
- Convert the current Ratatui `Table` to already-formatted owned `Line` values. Preserve every characterized marker, column width, inter-column gap, padding cell, alignment, RPE style, dim style, and highlight cell.
- Keep floating/focused detail History-owned and integrated through `render_detail_panel`. Do not add a generic Table, popup, list-detail, or action component. Do not use `ActionPanel`.
- Derive popup placement only from the clipped `ListPanelAreas.area` returned by `ScrollableListPanel`. Clip before `Clear` and detail rendering.
- Detail height and viewport validity are measured from the actual clipped popup inner width during rendering. Do not introduce logical-line height guesses during refresh or re-entry.
- All changed rectangle arithmetic is saturating or clipped. Empty, zero-sized, one-cell, narrow, short, resized, and edge-positioned targets must not panic or escape their parent/frame.
- Do not change database schema, localization copy, Router, ScreenStore, navigation topology, edit intent, lifecycle trait, colors, borders, padding, or keyboard commands.
- Do not add speculative configuration for later screens.
- Do not reintroduce Quick Log, LLM configuration/runtime, Abbreviations, or the `w` action.
- Every production change follows RED-GREEN-refactor. Record the expected failure before implementing the corresponding behavior.
- Fixture generation exists only long enough to capture the old implementation and is removed before the characterization commit.
- Structural ownership lint must inspect executable source. Comments, ordinary strings, raw strings, and character literals do not count as component usage; Rust lifetime syntax must remain visible to the scanner.
- Paragraph lint remains recursive and keeps its four-line scan window. Refresh an allowlist line only after proving the old and new lines are the same approved widget.
- Stop for a scope ruling if a fixture would need to change, an existing test conflicts with a fixture, the existing component API cannot reproduce the Table cells, a user-facing behavior change appears necessary, or a tracked file outside this plan is required.
- Use four planned commits: immutable History fixtures; History geometry/failure contracts; History migration plus structural lint; active documentation. Any review-discovered correctness fix uses a separate follow-up commit with focused RED/GREEN evidence.
- Stop after the History-only manual-acceptance handoff. Do not begin the next screen until the user explicitly accepts History.

## File Structure

**Create:**

- `tests/fixtures/tui/history/browse.txt` — selected log, dynamic columns, and floating preview.
- `tests/fixtures/tui/history/browse_with_undo.txt` — optional undo command and resulting footer layout.
- `tests/fixtures/tui/history/detail.txt` — focused full-height detail with nonzero visual-row offset.
- `tests/fixtures/tui/history/filtering_long_input.txt` — active long Unicode filter and cursor.
- `tests/fixtures/tui/history/no_filter_match.txt` — empty filtered list with no stale detail.
- `tests/fixtures/tui/history/list_overflow.txt` — final selected log, revealed viewport, and upward overflow suffix.
- `tests/fixtures/tui/history/confirm_delete.txt` — confirmation mode and its active shortcuts.

**Modify:**

- `src/tui/history.rs` — deterministic fixture builders, pure popup geometry, failure regressions, Table-to-Line formatter, and component-based render path.
- `src/tui/framework/screen_frame.rs` — test-only proof that the shared sections renderer preserves all accepted History footer allocations.
- `tests/tui_lint_test.rs` — transfer History from direct list-border ownership to executable `ScreenFrame` and `ScrollableListPanel` delegation.
- `CLAUDE.md` — mark History as a shipped frame/list consumer while recording its retained filter/detail/domain ownership.
- `README.md` — mark Batch 3B-4 History as shipped without claiming later slices.
- `README_zh.md` — Chinese equivalent of the shipped status.
- `docs/specs/history-spec.md` — document shared frame/list ownership and unchanged History behavior.
- `docs/specs/navigation-spec.md` — state that Batch 3B-4 changes presentation composition only.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — add the completed History slice and leave later work unshipped.

**Do not modify unless a verified same-widget source-line move requires a mechanical mapping refresh:**

- Paragraph allowlist literals in `tests/tui_lint_test.rs`.

---

### Task 1: Characterize All History Presentation States

**Files:**
- Modify: `src/tui/history.rs:758-980`
- Create: `tests/fixtures/tui/history/browse.txt`
- Create: `tests/fixtures/tui/history/browse_with_undo.txt`
- Create: `tests/fixtures/tui/history/detail.txt`
- Create: `tests/fixtures/tui/history/filtering_long_input.txt`
- Create: `tests/fixtures/tui/history/no_filter_match.txt`
- Create: `tests/fixtures/tui/history/list_overflow.txt`
- Create: `tests/fixtures/tui/history/confirm_delete.txt`

**Interfaces:**
- Consumes: existing `render_to_snapshot(width, height, render)`, current `HistoryScreen::render`, private test access to History state, `Log`, `LogEntry`, `Set`, `SetData`, and `PracticeType`.
- Produces: seven compile-time styled-buffer fixtures and `history_states_match_golden_buffers`, which Task 3 must pass without fixture changes.

- [ ] **Step 1: Add deterministic History model builders**

Extend the History test imports:

```rust
use crate::model::{Log, PracticeType, Set, SetData};
use crate::tui::framework::test_support::{
    render_to_buffer, render_to_snapshot, render_to_string,
};
```

Add fixed helpers inside the test module:

```rust
fn fixed_time(month: u32, day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, month, day)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap()
}

fn fixed_history_entry(
    id: i64,
    practice_name: impl Into<String>,
    practice_type: PracticeType,
    logged_at: chrono::NaiveDateTime,
    data: Vec<SetData>,
    rpe: Option<u8>,
    note: Option<String>,
) -> LogEntry {
    let practice_name = practice_name.into();
    LogEntry {
        log: Log {
            id,
            practice_id: id + 10_000,
            logged_at,
            note,
            warm_up: Some("Band warm-up with Unicode 训练 and controlled breathing".into()),
            cool_down: Some("Easy cooldown and mobility".into()),
            rpe,
        },
        practice_name,
        practice_type,
        sets: data
            .into_iter()
            .enumerate()
            .map(|(index, data)| Set {
                id: id * 100 + index as i64,
                log_id: id,
                set_number: index as i32 + 1,
                data,
            })
            .collect(),
        rpe,
    }
}

fn base_history_entries() -> Vec<LogEntry> {
    vec![
        fixed_history_entry(
            301,
            "Bench Press",
            PracticeType::Weighted,
            fixed_time(8, 13),
            vec![
                SetData::Weighted { weight: 60.0, reps: 5 },
                SetData::Weighted { weight: 67.5, reps: 5 },
                SetData::Weighted { weight: 72.5, reps: 3 },
            ],
            Some(8),
            Some("A deliberately long fixture note that wraps across visual rows. ".repeat(5)),
        ),
        fixed_history_entry(
            302,
            "Pull-up",
            PracticeType::Bodyweight,
            fixed_time(8, 12),
            vec![SetData::Bodyweight { reps: 12 }, SetData::Bodyweight { reps: 10 }],
            Some(7),
            Some("Controlled tempo".into()),
        ),
        fixed_history_entry(
            303,
            "Trail Run",
            PracticeType::Distance,
            fixed_time(8, 11),
            vec![SetData::Distance { distance: 5.5 }],
            None,
            None,
        ),
    ]
}
```

- [ ] **Step 2: Build all seven reachable presentation states**

Add:

```rust
fn golden_history_screen(name: &str) -> HistoryScreen {
    std::env::set_var("LC_ALL", "C");
    crate::i18n::init();

    let entries = base_history_entries();
    let mut screen = HistoryScreen {
        filtered_indices: (0..entries.len()).collect(),
        entries,
        filter: TextInput::new(),
        filtering: false,
        selected: 0,
        mode: Mode::Browse,
        list_viewport: ViewportState::new(0),
        detail_state: DetailPanelState::new(false),
        detail_content_height: 0,
        status_msg: Some(("Fixture status".into(), false)),
        last_deleted: None,
    };

    match name {
        "browse" => {}
        "browse_with_undo" => {
            screen.last_deleted = Some(fixed_history_entry(
                9_001,
                "Deleted Log",
                PracticeType::Bodyweight,
                fixed_time(8, 1),
                vec![SetData::Bodyweight { reps: 9 }],
                Some(6),
                Some("Undo fixture".into()),
            ));
        }
        "detail" => {
            screen.mode = Mode::Detail;
            screen.detail_state.focused = true;
            screen.detail_state.viewport.set_offset(2);
        }
        "filtering_long_input" => {
            let long = "训练".repeat(50);
            screen.entries[0].practice_name = long.clone();
            screen.filter = TextInput::from(long);
            screen.filtering = true;
            screen.filtered_indices = vec![0];
        }
        "no_filter_match" => {
            screen.filter = TextInput::from("missing practice".to_owned());
            screen.filtering = true;
            screen.filtered_indices.clear();
            screen.selected = 0;
        }
        "list_overflow" => {
            screen.entries = (0..30)
                .map(|index| {
                    fixed_history_entry(
                        1_000 + index,
                        format!("Practice {:02}", index + 1),
                        PracticeType::Bodyweight,
                        fixed_time(7, 1) + chrono::Duration::days(index),
                        vec![SetData::Bodyweight { reps: index as i32 + 1 }],
                        None,
                        None,
                    )
                })
                .collect();
            screen.filtered_indices = (0..screen.entries.len()).collect();
            screen.selected = screen.filtered_indices.len() - 1;
            screen.list_viewport.set_offset(25);
        }
        "confirm_delete" => {
            screen.selected = 1;
            screen.mode = Mode::ConfirmDelete;
        }
        other => panic!("unknown History golden state: {other}"),
    }

    screen
}
```

Use one test containing all states so locale initialization and fixture capture remain serialized inside each test process.

- [ ] **Step 3: Add the temporary fixture update path**

Create each fixture with one newline through `apply_patch`, then add:

```rust
#[test]
fn history_states_match_golden_buffers() {
    for name in [
        "browse",
        "browse_with_undo",
        "detail",
        "filtering_long_input",
        "no_filter_match",
        "list_overflow",
        "confirm_delete",
    ] {
        let mut screen = golden_history_screen(name);
        let actual = render_to_snapshot(80, 24, |frame| screen.render(frame));
        let path = format!(
            "{}/tests/fixtures/tui/history/{name}.txt",
            env!("CARGO_MANIFEST_DIR")
        );
        if std::env::var_os("UPDATE_HISTORY_GOLDENS").is_some() {
            std::fs::write(&path, &actual).expect("History fixture should be written");
        }
        let expected = std::fs::read_to_string(&path).expect("History fixture should exist");
        assert_eq!(actual, expected, "History {name} buffer changed");
    }
}
```

- [ ] **Step 4: Run the fixture test and record RED**

Run:

```bash
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
```

Expected: FAIL on `browse` because the one-line fixture does not equal the current `80x24` styled buffer. Record that assertion before generating fixtures.

- [ ] **Step 5: Capture the old renderer exactly once**

Run:

```bash
UPDATE_HISTORY_GOLDENS=1 cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
wc -l tests/fixtures/tui/history/*.txt
```

Expected: the test passes and every fixture reports `1921` lines: one area header plus `80 * 24` cell records.

- [ ] **Step 6: Remove every runtime fixture-update path**

Add the compile-time selector:

```rust
fn history_golden(name: &str) -> &'static str {
    match name {
        "browse" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/browse.txt")),
        "browse_with_undo" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/browse_with_undo.txt")),
        "detail" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/detail.txt")),
        "filtering_long_input" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/filtering_long_input.txt")),
        "no_filter_match" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/no_filter_match.txt")),
        "list_overflow" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/list_overflow.txt")),
        "confirm_delete" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tui/history/confirm_delete.txt")),
        other => panic!("unknown History golden fixture: {other}"),
    }
}
```

Replace the environment/write/read block with:

```rust
let expected = history_golden(name);
assert_eq!(actual, expected, "History {name} buffer changed");
```

Verify the update path is gone:

```bash
rg -n "UPDATE_HISTORY_GOLDENS|std::fs::write|read_to_string" src/tui/history.rs
```

Expected: no matches.

- [ ] **Step 7: Run characterization and baseline suites**

```bash
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo test tui::history::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo fmt -- --check
git diff --check
```

Expected: all fixtures pass in both library and binary targets, existing History tests pass, lint passes, and formatting/diff checks exit zero.

- [ ] **Step 8: Commit immutable characterization**

```bash
git add src/tui/history.rs tests/fixtures/tui/history
git commit -m "test(history): capture presentation states"
```

The commit must contain no framework or production render change.

---

### Task 2: Bind History Geometry and Failure Retention

**Files:**
- Modify: `src/tui/framework/screen_frame.rs:84-280`
- Modify: `src/tui/history.rs:85-260,758-1040`

**Interfaces:**
- Consumes: `render_screen_frame_sections`, `ScreenFrameSections`, `ListPanelAreas.area`, `fit_rect`, current inline History popup policy, and Task 1's deterministic History screens.
- Produces: `history_detail_rect(list_area: Rect, row_in_list: u16, content_height: usize, focused: bool) -> Rect`, frame-allocation regressions, popup containment regressions, width-aware re-entry clamping, mutation-failure regressions, and tiny/resized render coverage used by Task 3.

- [ ] **Step 1: Add the exact History frame-allocation regression**

In `screen_frame.rs` tests, add:

```rust
#[test]
fn screen_frame_sections_match_accepted_history_layouts() {
    let area = Rect::new(0, 0, 80, 24);
    let shortcut_sets = [
        vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("Enter", "Details"),
            Shortcut::new("/", "Filter"),
            Shortcut::new("e", "Edit"),
            Shortcut::new("d", "Delete"),
            Shortcut::new("Esc", "Back"),
        ],
        vec![
            Shortcut::new("j/k", "Navigate"),
            Shortcut::new("Enter", "Details"),
            Shortcut::new("/", "Filter"),
            Shortcut::new("e", "Edit"),
            Shortcut::new("d", "Delete"),
            Shortcut::new("u", "Undo"),
            Shortcut::new("Esc", "Back"),
        ],
        vec![Shortcut::new("y", "Delete · Yes"), Shortcut::new("any", "Cancel")],
    ];

    for shortcuts in shortcut_sets {
        let legacy = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
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
                &Some(("Fixture status".into(), false)),
                &shortcuts,
                &[Constraint::Length(1), Constraint::Min(1)],
                false,
            ));
        });

        let sections = sections.unwrap();
        assert_eq!(sections.area, area);
        assert_eq!(sections.body.as_slice(), &legacy[..2]);
        assert_eq!(sections.status, legacy[2]);
        assert_eq!(sections.shortcuts, legacy[3]);
    }
}
```

- [ ] **Step 2: Run the focused frame test and record GREEN characterization**

```bash
cargo test tui::framework::screen_frame::tests::screen_frame_sections_match_accepted_history_layouts -- --exact --nocapture
```

Expected: PASS in both library and binary targets because the generic sections API already supports the accepted constraints. This is characterization, not a production RED.

- [ ] **Step 3: Write popup tests against the missing pure helper**

In History tests, import `Rect`, add `rect_is_within`, and add:

```rust
fn rect_is_within(parent: Rect, child: Rect) -> bool {
    child.x >= parent.x
        && child.y >= parent.y
        && child.right() <= parent.right()
        && child.bottom() <= parent.bottom()
}

#[test]
fn history_detail_rect_preserves_below_flip_and_focused_geometry() {
    let list = Rect::new(0, 1, 80, 20);
    assert_eq!(history_detail_rect(list, 1, 5, false), Rect::new(4, 4, 72, 7));
    assert_eq!(history_detail_rect(list, 17, 5, false), Rect::new(4, 11, 72, 7));
    assert_eq!(history_detail_rect(list, 8, 5, true), Rect::new(4, 1, 72, 20));
}

#[test]
fn history_detail_rect_never_escapes_tiny_or_edge_parents() {
    for parent in [
        Rect::new(0, 0, 0, 0),
        Rect::new(5, 5, 1, 1),
        Rect::new(2, 3, 12, 4),
        Rect::new(u16::MAX - 2, u16::MAX - 2, 8, 8),
    ] {
        for focused in [false, true] {
            for content_height in [0, 5, usize::MAX] {
                let popup = history_detail_rect(parent, u16::MAX, content_height, focused);
                assert!(rect_is_within(parent, popup), "{parent:?} -> {popup:?}");
            }
        }
    }
}
```

- [ ] **Step 4: Run popup tests and record RED**

```bash
cargo test tui::history::tests::history_detail_rect -- --nocapture
```

Expected: compilation FAIL because `history_detail_rect` does not exist. Record the missing-function error.

- [ ] **Step 5: Extract the current popup calculation with saturating arithmetic**

Add above `impl HistoryScreen`:

```rust
fn history_detail_rect(
    list_area: ratatui::layout::Rect,
    row_in_list: u16,
    content_height: usize,
    focused: bool,
) -> ratatui::layout::Rect {
    let popup_x = list_area.x.saturating_add(4.min(list_area.width));
    let popup_w = list_area
        .width
        .saturating_sub(8)
        .max(30)
        .min(list_area.right().saturating_sub(popup_x));
    let popup_h = if focused {
        list_area.height
    } else {
        (content_height.min(u16::MAX as usize) as u16)
            .saturating_add(2)
            .max(4)
            .min(list_area.height / 2)
    };
    let popup_y = if focused {
        list_area.y
    } else {
        let below_y = list_area
            .y
            .saturating_add(row_in_list)
            .saturating_add(2);
        if below_y.saturating_add(popup_h) > list_area.bottom() {
            list_area
                .y
                .saturating_add(row_in_list.saturating_sub(popup_h))
        } else {
            below_y
        }
    };

    fit_rect(
        list_area,
        ratatui::layout::Rect::new(popup_x, popup_y, popup_w, popup_h),
    )
}
```

Replace the inline rectangle construction in `HistoryScreen::render` with a call to this helper, using `self.mode == Mode::Detail`. Do not change title, content measurement, `Clear`, or `render_detail_panel` in this task.

- [ ] **Step 6: Add width-aware re-entry, mutation-failure, and tiny/resized coverage**

Add:

```rust
#[test]
fn history_reentry_defers_detail_clamp_until_actual_width_render() {
    let db = history_db_with_long_entry(20);
    let mut screen = HistoryScreen::new(&db).unwrap();
    screen.mode = Mode::Detail;
    screen.detail_state.focused = true;
    screen.detail_state.viewport.set_height(2);
    screen.detail_state.viewport.set_offset(usize::MAX);
    screen.detail_content_height = 1;

    let mut next = screen.reenter(&db).unwrap();

    assert_eq!(next.detail_state.viewport.offset(), usize::MAX);
    let _ = render_history(&mut next, 80, 24);
    assert_ne!(next.detail_state.viewport.offset(), usize::MAX);
}

#[test]
fn history_delete_failure_retains_confirmation_and_context() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(file.path()).unwrap();
    let practice = db.create_practice("Press", PracticeType::Bodyweight).unwrap();
    db.create_log(practice.id, &[SetData::Bodyweight { reps: 10 }], Some("keep"), None, None, Some(7)).unwrap();
    let mut screen = HistoryScreen::new(&db).unwrap();
    let selected_id = screen.selected_entry().unwrap().log.id;
    screen.handle_key(key(KeyCode::Char('d')), &db);
    rusqlite::Connection::open(file.path()).unwrap().execute("DROP TABLE training_sessions", []).unwrap();

    screen.handle_key(key(KeyCode::Char('y')), &db);

    assert_eq!(screen.mode, Mode::ConfirmDelete);
    assert_eq!(screen.selected_entry().unwrap().log.id, selected_id);
    assert!(screen.last_deleted.is_none());
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
}

#[test]
fn history_restore_failure_restores_undo_payload_and_context() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(file.path()).unwrap();
    let practice = db.create_practice("Press", PracticeType::Bodyweight).unwrap();
    db.create_log(practice.id, &[SetData::Bodyweight { reps: 10 }], Some("current"), None, None, Some(7)).unwrap();
    let mut screen = HistoryScreen::new(&db).unwrap();
    let selected_id = screen.selected_entry().unwrap().log.id;
    let deleted = fixed_history_entry(9_002, "Retry", PracticeType::Bodyweight, fixed_time(8, 1), vec![SetData::Bodyweight { reps: 9 }], Some(6), Some("retry".into()));
    screen.last_deleted = Some(deleted.clone());
    rusqlite::Connection::open(file.path()).unwrap().execute("DROP TABLE training_sessions", []).unwrap();

    screen.handle_key(key(KeyCode::Char('u')), &db);

    assert_eq!(screen.mode, Mode::Browse);
    assert_eq!(screen.selected_entry().unwrap().log.id, selected_id);
    assert_eq!(screen.last_deleted.as_ref().unwrap().log.id, deleted.log.id);
    assert!(screen.status_msg.as_ref().is_some_and(|(_, error)| *error));
}

#[test]
fn history_states_render_in_tiny_and_resized_terminals_without_panicking() {
    for name in ["browse", "browse_with_undo", "detail", "filtering_long_input", "no_filter_match", "list_overflow", "confirm_delete"] {
        let mut screen = golden_history_screen(name);
        for (width, height) in [(1, 1), (12, 4), (24, 10), (40, 6), (80, 24), (20, 5)] {
            let _ = render_to_snapshot(width, height, |frame| screen.render(frame));
        }
        let _ = render_to_buffer(80, 24, |frame| screen.render(frame));
        if !screen.filtered_indices.is_empty() {
            assert!(screen.list_viewport.offset() <= screen.selected);
        } else {
            assert_eq!(screen.list_viewport.offset(), 0);
        }
    }
}
```

The two mutation tests should already pass; they bind existing behavior before the renderer changes. The new re-entry test must be RED on the current eager clamp, and the geometry tests must turn GREEN only after the pure helper exists.

- [ ] **Step 7: Record re-entry RED, then defer detail clamping to render**

Run:

```bash
cargo test tui::history::tests::history_reentry_defers_detail_clamp_until_actual_width_render -- --exact --nocapture
```

Expected: FAIL because `HistoryScreen::reenter` clamps the copied offset against stale `detail_content_height` before the destination width is known.

Remove only this eager line from `Reenter`:

```rust
next.detail_state.viewport.clamp(next.detail_content_height);
```

Keep copying `detail_state.viewport` and `detail_content_height`. `render_detail_panel` already measures `visual_spans` using the actual popup inner width and clamps the viewport during the next render.

Update the existing selected-deletion re-entry assertion from the eager-clamped value to the preserved value:

```rust
assert_eq!(next.detail_state.viewport.offset(), 4);
```

Rerun the focused test. Expected: PASS before render with the original offset, then PASS after render with a finite width-measured offset.

- [ ] **Step 8: Run Task 2 gates**

```bash
cargo test tui::framework::screen_frame::tests::screen_frame_sections_match_accepted_history_layouts -- --exact --nocapture
cargo test tui::history::tests::history_detail_rect -- --nocapture
cargo test tui::history::tests::history_reentry_defers_detail_clamp_until_actual_width_render -- --exact --nocapture
cargo test tui::history::tests::history_delete_failure_retains_confirmation_and_context -- --exact --nocapture
cargo test tui::history::tests::history_restore_failure_restores_undo_payload_and_context -- --exact --nocapture
cargo test tui::history::tests::history_states_render_in_tiny_and_resized_terminals_without_panicking -- --exact --nocapture
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
cargo fmt -- --check
git diff --check
```

Expected: all focused tests and fixtures pass in library/binary targets; formatting and diff checks exit zero.

- [ ] **Step 9: Commit the geometry and failure contracts**

```bash
git add src/tui/framework/screen_frame.rs src/tui/history.rs
git commit -m "fix(history): bind width-aware detail geometry"
```

---

### Task 3: Migrate History to ScreenFrame and ScrollableListPanel

**Files:**
- Modify: `src/tui/history.rs:1-405,758-1100`
- Modify: `tests/tui_lint_test.rs:308-350`
- Test: all seven `tests/fixtures/tui/history/*.txt` remain byte-identical

**Interfaces:**
- Consumes: `render_screen_frame_sections`, `ScreenFrameSections`, `render_scrollable_list_panel`, `ListPanelContent<'static>`, `ListPanelAreas`, `truncate_to_width`, `ViewportState`, Task 2's `history_detail_rect`, and Task 1's immutable fixtures.
- Produces: `HistoryColumns`, `history_list_content(entries, filtered_indices, selected, inner_width, columns) -> ListPanelContent<'static>`, one component-based History render path, and executable structural ownership enforcement.

- [ ] **Step 1: Add a structural lint that makes the old renderer RED**

Change the direct-border group:

```rust
let required_screens = ["goals.rs", "log_entry.rs"];
```

Change the delegated loop to require both presentation calls:

```rust
for (filename, screen_name) in [
    ("practices.rs", "Practices"),
    ("quotes_screen.rs", "Quotes"),
    ("trends.rs", "Trends"),
    ("history.rs", "History"),
] {
    let source = std::fs::read_to_string(tui_dir.join(filename)).unwrap();
    assert!(
        source_code_contains(&source, "render_scrollable_list_panel("),
        "{screen_name} must delegate its bordered list to ScrollableListPanel"
    );
    if filename == "history.rs" {
        assert!(
            source_code_contains(&source, "render_screen_frame_sections("),
            "History must delegate frame/footer geometry to ScreenFrame"
        );
    }
}
```

- [ ] **Step 2: Run the ownership lint and record RED**

```bash
cargo test --test tui_lint_test screens_must_have_bordered_blocks -- --exact --nocapture
```

Expected: FAIL only because History does not yet call `render_scrollable_list_panel` and `render_screen_frame_sections`.

- [ ] **Step 3: Write the row-equivalence test and record RED**

Using `golden_history_screen`, add a test against the not-yet-defined `HistoryColumns`, `HistoryScreen::columns`, and `history_list_content` interfaces:

```rust
#[test]
fn history_line_rows_preserve_unicode_rpe_and_metric_cells() {
    let screen = golden_history_screen("filtering_long_input");
    let columns = screen.columns();
    let content = history_list_content(
        &screen.entries,
        &screen.filtered_indices,
        screen.selected,
        76,
        columns,
    );

    assert!(content
        .header
        .as_ref()
        .unwrap()
        .to_string()
        .contains(&tr("history-col-date")));
    assert_eq!(content.rows.len(), 1);
    let row = content.rows[0].clone();
    assert!(row.to_string().contains("训练"));
    assert!(row.to_string().contains("[RPE: 8]"));
    assert!(row.width() <= 76);
}
```

Run:

```bash
cargo test tui::history::tests::history_line_rows_preserve_unicode_rpe_and_metric_cells -- --exact --nocapture
```

Expected: compilation FAIL because `HistoryColumns`, `columns`, or `history_list_content` does not exist. Record the missing-interface error.

- [ ] **Step 4: Add explicit History column and styled-cell helpers**

In `history.rs`, import `truncate_to_width`, `ListPanelContent`, `render_screen_frame_sections`, and `render_scrollable_list_panel`. Remove `Cell`, `Row`, and `Table` only after the new path is complete.

Add:

```rust
#[derive(Debug, Clone, Copy)]
struct HistoryColumns {
    marker: usize,
    date: usize,
    name: usize,
    metric: usize,
    max_total: usize,
    max_label: usize,
}

impl HistoryScreen {
    fn columns(&self) -> HistoryColumns {
        let max_name = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| entry.practice_name.width() + usize::from(entry.log.rpe.is_some()) * 10)
            .max()
            .unwrap_or(4);
        let max_total = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| format!("{:.0}", entry.total_metric()).len())
            .max()
            .unwrap_or(4);
        let max_label = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .map(|entry| entry.metric_label().width())
            .max()
            .unwrap_or(2);
        HistoryColumns {
            marker: 2,
            date: 11,
            name: max_name + 2,
            metric: max_total + 1 + max_label + 1,
            max_total,
            max_label,
        }
    }
}

fn append_cell(
    output: &mut Vec<Span<'static>>,
    spans: Vec<Span<'static>>,
    requested_width: usize,
    remaining: &mut usize,
    gap_after: bool,
) {
    let width = requested_width.min(*remaining);
    let mut unused = width;
    for span in spans {
        if unused == 0 {
            break;
        }
        let text = truncate_to_width(span.content.as_ref(), unused);
        let used = text.width();
        if !text.is_empty() {
            output.push(Span::styled(text, span.style));
        }
        unused = unused.saturating_sub(used);
    }
    if unused > 0 {
        output.push(Span::raw(" ".repeat(unused)));
    }
    *remaining = remaining.saturating_sub(width);
    if gap_after && *remaining > 0 {
        output.push(Span::raw(" "));
        *remaining -= 1;
    }
}
```

Add a free formatter that does not borrow `HistoryScreen` while the viewport is mutably borrowed:

```rust
fn history_list_content(
    entries: &[LogEntry],
    filtered_indices: &[usize],
    selected: usize,
    inner_width: u16,
    columns: HistoryColumns,
) -> ListPanelContent<'static> {
    if filtered_indices.is_empty() {
        return ListPanelContent {
            header: None,
            rows: Vec::new(),
            empty: Some(Line::from(Span::styled(
                format!("  {}", tr("history-no-entries")),
                Style::default().fg(Color::Gray),
            ))),
            selected: None,
        };
    }

    let build_line = |cells: Vec<Vec<Span<'static>>>| {
        let widths = [columns.marker, columns.date, columns.name, columns.metric];
        let mut remaining = inner_width as usize;
        let mut output = Vec::new();
        let cell_count = cells.len();
        for (index, (cell, width)) in cells.into_iter().zip(widths).enumerate() {
            append_cell(&mut output, cell, width, &mut remaining, index + 1 < cell_count);
        }
        Line::from(output)
    };

    let header_style = Style::default().fg(Color::White).bold();
    let header = build_line(vec![
        vec![],
        vec![Span::styled(tr("history-col-date"), header_style)],
        vec![Span::styled(tr("history-col-practice"), header_style)],
        vec![Span::styled(tr("history-col-volume"), header_style)],
    ]);

    let rows = filtered_indices
        .iter()
        .enumerate()
        .filter_map(|(filtered_index, entry_index)| {
            let entry = entries.get(*entry_index)?;
            let selected_row = filtered_index == selected;
            let primary = if selected_row {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let dim = if selected_row {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            let total = format!("{:.0}", entry.total_metric());
            let label = entry.metric_label();
            let mut name = vec![Span::styled(entry.practice_name.clone(), primary)];
            if let Some(rpe) = entry.log.rpe {
                name.push(Span::styled(
                    format!(" [RPE: {rpe}]"),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Some(build_line(vec![
                vec![Span::styled(if selected_row { ">" } else { " " }, primary)],
                vec![Span::styled(entry.log.logged_at.format("%Y-%m-%d").to_string(), dim)],
                name,
                vec![Span::styled(
                    format!(
                        "{:>width$} {}{}",
                        total,
                        label,
                        " ".repeat(columns.max_label.saturating_sub(label.width())),
                        width = columns.max_total,
                    ),
                    dim,
                )],
            ]))
        })
        .collect();

    ListPanelContent {
        header: Some(header),
        rows,
        empty: None,
        selected: Some(selected),
    }
}
```

This is the exact intended algorithm. If the first migration run shows any styled-cell difference, inspect the old `Table` cell geometry and correct this formatter only. Do not edit a fixture or extend the shared component.

- [ ] **Step 5: Run the row formatter test and record GREEN**

```bash
cargo test tui::history::tests::history_line_rows_preserve_unicode_rpe_and_metric_cells -- --exact --nocapture
```

Expected: PASS in both library and binary targets.

- [ ] **Step 6: Replace the outer layout with ScreenFrame**

Replace centered-area and local footer allocation/rendering with:

```rust
let shortcuts = self.active_shortcuts();
let sections = render_screen_frame_sections(
    frame,
    frame.area(),
    CONTENT_WIDTH,
    &self.status_msg,
    &shortcuts,
    &[Constraint::Length(1), Constraint::Min(1)],
    false,
);
let filter_area = sections.body[0];
let list_area = sections.body[1];
```

Render the unchanged filter `Paragraph` into `filter_area`. Delete the local `centered_area`, `shortcut_height`, `render_status_line`, and `render_shortcuts` usage from History.

- [ ] **Step 7: Replace `render_list` and `Table` with ScrollableListPanel**

Use the `columns()` method and formatter introduced in Step 4, then render the list once:

```rust
let columns = self.columns();
let entries = &self.entries;
let filtered_indices = &self.filtered_indices;
let selected = self.selected;
let list_areas = render_scrollable_list_panel(
    frame,
    list_area,
    tr("history-title"),
    &mut self.list_viewport,
    |inner| history_list_content(entries, filtered_indices, selected, inner.width, columns),
);
```

Delete `render_list`, its local `Block`, `Table`, `Row`, `Cell`, manual visible count, overflow suffix, viewport reveal/clamp, empty `Paragraph`, and `highlight_row` call.

- [ ] **Step 8: Reconnect detail rendering to safe returned list geometry**

After `render_scrollable_list_panel`, compute `row_in_list` from the now-clamped shared viewport, measure the detail content at the candidate popup width, and call:

```rust
let popup_rect = history_detail_rect(
    list_areas.area,
    row_in_list,
    self.detail_content_height,
    self.mode == Mode::Detail,
);
```

The measurement order must use the actual clipped popup width. If needed, calculate the width-only candidate first with `history_detail_rect(..., 0, focused)`, measure `visual_spans` with `candidate.width.saturating_sub(2).max(1)`, then calculate the final height-dependent rectangle. Preserve `Clear`, title, focus, Browse offset reset, and `render_detail_panel` exactly.

Do not use `list_areas.inner` for placement: the accepted History popup is positioned from the old outer list rectangle. Do not reconstruct the list block locally.

- [ ] **Step 9: Run the migration fixture first and stop on any difference**

```bash
cargo test tui::history::tests::history_states_match_golden_buffers -- --exact --nocapture
```

Expected: all seven states PASS in both library and binary targets with fixtures untouched.

If any cell differs, stop normal implementation work. Diagnose marker/column/gap/padding/style or popup geometry against the old fixture. Correct production formatting only. Do not invoke an update variable, edit a fixture, or add Table-specific behavior to the shared component without a user ruling.

- [ ] **Step 10: Verify geometry, behavior, ownership, and previous consumers**

```bash
cargo test tui::history::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
git diff --exit-code -- tests/fixtures/tui/history tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends
```

Expected: every command exits zero, strict Clippy reports no warnings, and no fixture has changed since its characterization commit.

If the Paragraph lint fails only because a pre-existing approved widget moved, prove old/new line identity with `git show` before mechanically updating that one allowlist literal. Do not alter lint semantics.

- [ ] **Step 11: Audit the migrated ownership boundary**

Run:

```bash
rg -n "Table::new|Row::new|Cell::from|\.borders\(Borders::ALL\)|render_status_line\(|render_shortcuts\(|shortcut_height\(|highlight_row\(" src/tui/history.rs
rg -n "render_screen_frame_sections\(|render_scrollable_list_panel\(|render_detail_panel\(" src/tui/history.rs
```

Expected: the first command has no production matches; the second shows one executable frame call, one executable list-panel call, and the existing detail-panel call. Test-only matches must be examined and justified rather than hidden.

- [ ] **Step 12: Commit the History migration**

```bash
git add src/tui/history.rs tests/tui_lint_test.rs
git commit -m "refactor(history): adopt shared presentation components"
```

The fixture directory must not be staged in this commit.

---

### Task 4: Document and Release-Gate Batch 3B-4

**Files:**
- Modify: `CLAUDE.md:78-95`
- Modify: `README.md:49-52,97-110`
- Modify: `README_zh.md:49-52,97-110`
- Modify: `docs/specs/history-spec.md:1-80`
- Modify: `docs/specs/navigation-spec.md:15-52`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md:1-85,184-210`

**Interfaces:**
- Consumes: Task 3's verified component boundary and the approved design.
- Produces: accurate active documentation, final release evidence, a History-only binary for manual acceptance, and no claim that a later screen has shipped.

- [ ] **Step 1: Update contributor and README shipped-state language**

In `CLAUDE.md`, replace the shipped-state sentence with:

```markdown
Batch 3A provides this concrete lifecycle routing. Batch 3B-1 Practices, Batch 3B-2 Quotes, Batch 3B-3 Trends, and Batch 3B-4 History presentation migrations are shipped; later presentation slices are not shipped.
```

Extend the component paragraph with:

```markdown
History consumes `ScreenFrame` and `ScrollableListPanel`; it retains filter rendering, dynamic log-column formatting, floating-preview placement, Detail-mode measurement and scrolling, edit navigation, delete/undo policy, and lifecycle behavior through the existing `render_detail_panel` helper.
```

Apply the same shipped-state correction to both READMEs. In the History section, state that the shared frame owns filter/list/status/shortcut allocation, the shared list panel owns border/overflow/viewport/header/highlight, and History still owns log formatting, filtering, detail, editing, delete/undo, and re-entry. Do not describe a new key, color, or layout.

- [ ] **Step 2: Update active specifications**

Add this ownership paragraph under `docs/specs/history-spec.md` Layout:

```markdown
`ScreenFrame` reserves the filter, list, status, and wrapped shortcut rows in one composition pass. `ScrollableListPanel` owns the History list border, fixed header, overflow suffix, logical-row viewport, visible slicing, empty state, and selected-row highlight. History supplies already-formatted date/practice/RPE/metric lines and retains filtering, popup placement, Detail focus, editing, delete/undo, database, and re-entry policy. The floating and focused detail remains integrated through `render_detail_panel` rather than a generic popup component.
```

Append to the Batch presentation sentence in `docs/specs/navigation-spec.md`:

```markdown
Batch 3B-4 changes only History presentation composition and defers detail-viewport clamping until the actual popup width is known; it does not change routes, stable-ID edit intents, re-entry selection/filter/transient-state policy, delete/undo behavior, or shortcuts.
```

In the framework design:

- change the status to include `3B-4` complete and later slices unshipped;
- add History to the shipped `ScreenFrame` and `ScrollableListPanel` consumers;
- add `### Batch 3B-4: History Presentation Components — Complete` describing the seven fixtures, Table-to-Line equivalence, retained popup/domain behavior, geometry safety, and unchanged navigation/lifecycle;
- replace the later-work sentence so only Goals popups, Dashboard responsive layout, Log Entry forms, and later slices remain unshipped.

- [ ] **Step 3: Run documentation accuracy and removed-surface audits**

```bash
rg -n "History and later slices are not shipped|History work does not start|Batch 3B-4.*not shipped" README.md README_zh.md CLAUDE.md docs/specs docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
rg -n -i "quick[-_ ]?log|llm|ureq|abbreviations?|abbreviation[_ -](title|prompt|list|create|update|delete)" src tests locales Cargo.toml
```

Expected: the first command has no stale active claim. The second has no live removed-feature match except the existing narrow migration compatibility references permitted by `removed_quick_log_test`.

- [ ] **Step 4: Run all focused gates**

```bash
cargo test tui::history::tests -- --nocapture
cargo test tui::framework:: -- --nocapture
cargo test app:: -- --nocapture
cargo test tui::practices::tests::practices_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::quotes_screen::tests::quotes_modes_match_golden_buffers -- --exact --nocapture
cargo test tui::trends::tests::trends_states_match_golden_buffers -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: every focused suite passes with zero failures.

- [ ] **Step 5: Run fresh release gates**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check aea0c5bbe6332e4e0ecf2083ceaf96643fac83a5..HEAD
git diff --exit-code -- tests/fixtures/tui/history tests/fixtures/tui/practices tests/fixtures/tui/quotes tests/fixtures/tui/trends
```

Expected: formatting, strict Clippy, all tests, build, range whitespace check, and fixture immutability all exit zero. Record fresh per-target test counts from the full test output; do not reuse an earlier count.

- [ ] **Step 6: Run exact ownership and scope audits**

```bash
rg -n "Table::new|Row::new|Cell::from|\.borders\(Borders::ALL\)|render_status_line\(|render_shortcuts\(|shortcut_height\(|highlight_row\(" src/tui/history.rs
rg -n "render_screen_frame_sections\(|render_scrollable_list_panel\(|render_detail_panel\(" src/tui/history.rs
git diff --name-only aea0c5bbe6332e4e0ecf2083ceaf96643fac83a5..HEAD
git status --short --untracked-files=all
```

Expected:

- no old History list/frame presentation owner remains in production;
- History has executable frame/list/detail integration;
- changed tracked paths are only those declared in this plan plus the plan/design documents already on `main`;
- no unexpected tracked or untracked file is present.

- [ ] **Step 7: Commit active documentation**

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/history-spec.md docs/specs/navigation-spec.md docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md
git commit -m "docs(tui): document History component migration"
```

- [ ] **Step 8: Perform a fresh post-commit verification and prepare manual handoff**

```bash
git diff --check aea0c5bbe6332e4e0ecf2083ceaf96643fac83a5..HEAD
git status --short --untracked-files=all
test -x target/debug/iron
```

Expected: range diff check exits zero, status is blank, and `target/debug/iron` exists.

Before handoff, independently review the full `aea0c5b..HEAD` range against the approved design. Report Critical, Important, and Minor findings with exact file/line evidence. Fix any Critical or Important finding through a separate focused RED/GREEN commit and rerun all release gates. Then stop and provide the History-only binary for the user's manual acceptance. Do not begin the next module.

## Completion Checklist

- [ ] Seven pre-refactor History fixtures are immutable and pass unchanged in library and binary targets.
- [ ] Practices, Quotes, and Trends fixtures remain unchanged.
- [ ] `ScreenFrame` owns History filter/list/status/shortcut allocation.
- [ ] `ScrollableListPanel` owns History border/header/overflow/viewport/slicing/empty/highlight presentation.
- [ ] Table-to-Line formatting is cell-identical across all characterized states.
- [ ] Floating and focused detail remains History-owned, width-aware, clipped, and fixture-identical.
- [ ] Filtering, edit navigation, delete/undo, failure retention, re-entry selection/filter/transient-state semantics, and shortcuts remain unchanged; only the approved width-aware detail clamp is refined.
- [ ] Normal, narrow, short, tiny, zero-sized, edge, and resized rendering remains bounded and panic-free.
- [ ] Structural lint rejects local History border ownership and non-executable delegation evidence.
- [ ] Formatting, strict Clippy, full tests, build, removal guard, range diff check, and scope audits pass.
- [ ] Documentation marks History shipped and later presentation slices unshipped.
- [ ] Worktree is clean and the History-only binary is ready for manual acceptance.
- [ ] Implementation pauses for explicit user acceptance before another module begins.
