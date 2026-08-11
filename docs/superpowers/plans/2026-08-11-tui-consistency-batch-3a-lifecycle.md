# TUI Consistency Batch 3A Application Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace lifecycle policy embedded in the terminal loop with typed navigation, cached strongly typed screens, atomic context-preserving re-entry, and source-visible navigation failures.

**Architecture:** Pure navigation types describe valid destinations and Log Entry requests. Each ordinary screen implements a statically dispatched lifecycle contract that returns a fully prepared replacement without mutating the cached screen. `ScreenStore` owns concrete screens, `Router` performs prepare-then-commit transitions, and `AppShell` owns the current route while the terminal loop only reads events and draws.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Fluent localization, existing shared TUI framework, Cargo tests and Clippy.

## Global Constraints

- Base implementation commit is `5e2cbd46dfa1f5a96d60bbb151bdb0a27a8f6fe5`; design commit is `510c570` on `codex/tui-consistency`.
- Follow [the approved Batch 3A design](../specs/2026-08-11-tui-consistency-batch-3a-lifecycle-design.md).
- Preserve the existing navigation topology: Dashboard is the hub; ordinary modules return to Dashboard; new Log Entry returns to Dashboard; History edits return to History.
- Re-entry preserves filter values, stable selected identities, approved viewports, read-only detail focus, and the Trends time window.
- Re-entry clears filter-editing mode, add/edit/rename/date modes, confirmation modes, transient inputs, old statuses, and cross-page undo payloads.
- A target screen is installed and the current route changes only after every fallible target-preparation operation succeeds.
- Preparation failure leaves the source route and cached target unchanged and renders a localized error on the source screen.
- Log Entry remains request-scoped. Validation and database failures keep the active form unchanged.
- Use static dispatch over concrete screen types; do not add boxed screen trait objects or a navigation stack.
- Do not extract list, form, popup, or layout components in Batch 3A.
- Do not change the database schema, visible navigation, shortcuts, or accepted Batch 1/2 behavior.
- Do not reintroduce Quick Log, LLM configuration/runtime, Abbreviations, or the `w` action.
- Every changed behavior follows red-green-refactor and ends at an independently reviewable commit.
- Mechanical `tests/tui_lint_test.rs` line refreshes are permitted only when changed source lines move an existing approved widget site.

## File Structure

**Create:**

- `src/tui/navigation.rs` — pure route, module, navigation-intent, and action types.
- `src/tui/lifecycle.rs` — statically dispatched re-entry and navigation-status contracts.
- `src/app/screens.rs` — concrete `ScreenStore` and prepared-target installation.
- `src/app/router.rs` — prepare-then-commit route transitions.

**Modify:**

- `src/app.rs` — terminal-independent `AppShell` and a terminal loop that delegates screen policy.
- `src/tui/mod.rs` — re-export navigation/lifecycle types and remove superseded `Screen`/`Action` definitions.
- `src/db.rs`, `tests/db_test.rs` — stable log-ID lookup used by edit intents.
- `src/tui/dashboard.rs` — Dashboard re-entry and navigation-error status.
- `src/tui/goals.rs` — Goals re-entry and transient-state reset.
- `src/tui/history.rs` — History edit intent and cross-page re-entry contract.
- `src/tui/trends.rs` — Trends re-entry and screen-owned chart refresh.
- `src/tui/practices.rs` — Practices re-entry contract.
- `src/tui/quotes_screen.rs` — Quotes re-entry contract.
- `src/tui/log_entry.rs` — typed return module and navigation-error status.
- `locales/en.ftl`, `locales/zh-CN.ftl` — localized navigation preparation error.
- `CLAUDE.md`, `README.md`, `README_zh.md`, `docs/specs/navigation-spec.md`, and affected screen specifications — shipped lifecycle behavior.
- `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md` — Batch 3A completion state after implementation.
- `tests/tui_lint_test.rs` — mechanical source-line allowlist refreshes only when required.

---

### Task 1: Add Typed Navigation and Stable Log Lookup

**Files:**
- Create: `src/tui/navigation.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/db.rs`
- Modify: `tests/db_test.rs`
- Modify: `src/app.rs`
- Modify: `src/tui/dashboard.rs`
- Modify: `src/tui/goals.rs`
- Modify: `src/tui/history.rs`
- Modify: `src/tui/log_entry.rs`
- Modify: `src/tui/trends.rs`
- Modify: `src/tui/practices.rs`
- Modify: `src/tui/quotes_screen.rs`

**Interfaces:**
- Consumes: existing screen key handlers and `Database::list_logs_all`.
- Produces: `ModuleId`, `ScreenId`, `NavigationIntent`, `Action`, and `Database::get_log_by_id(log_id) -> Result<Option<LogEntry>>`.

- [ ] **Step 1: Add navigation-type tests that make invalid Log Entry opening unrepresentable**

Create `src/tui/navigation.rs` with the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_intents_have_exact_targets() {
        assert_eq!(
            NavigationIntent::Open(ModuleId::Goals).target(),
            ScreenId::Goals
        );
        assert_eq!(NavigationIntent::NewLog.target(), ScreenId::LogEntry);
        assert_eq!(
            NavigationIntent::EditHistoryLog { log_id: 42 }.target(),
            ScreenId::LogEntry
        );
    }

    #[test]
    fn module_ids_exclude_request_scoped_log_entry() {
        let modules = [
            ModuleId::Dashboard,
            ModuleId::History,
            ModuleId::Trends,
            ModuleId::Practices,
            ModuleId::Goals,
            ModuleId::Quotes,
        ];
        assert!(modules
            .into_iter()
            .all(|module| ScreenId::from(module) != ScreenId::LogEntry));
    }
}
```

- [ ] **Step 2: Run the navigation tests to verify RED**

Run:

```bash
cargo test tui::navigation::tests -- --nocapture
```

Expected: compilation fails because the navigation types do not exist.

- [ ] **Step 3: Implement the pure navigation types**

Add above the tests:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleId {
    Dashboard,
    History,
    Trends,
    Practices,
    Goals,
    Quotes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
    Goals,
    Quotes,
}

impl From<ModuleId> for ScreenId {
    fn from(module: ModuleId) -> Self {
        match module {
            ModuleId::Dashboard => Self::Dashboard,
            ModuleId::History => Self::History,
            ModuleId::Trends => Self::Trends,
            ModuleId::Practices => Self::Practices,
            ModuleId::Goals => Self::Goals,
            ModuleId::Quotes => Self::Quotes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationIntent {
    Open(ModuleId),
    NewLog,
    EditHistoryLog { log_id: i64 },
}

impl NavigationIntent {
    pub fn target(self) -> ScreenId {
        match self {
            Self::Open(module) => module.into(),
            Self::NewLog | Self::EditHistoryLog { .. } => ScreenId::LogEntry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Navigate(NavigationIntent),
    Quit,
}
```

Expose modules from `src/tui/mod.rs`:

```rust
pub mod navigation;
pub use navigation::{Action, ModuleId, NavigationIntent, ScreenId};
```

Remove the old `Screen` and `Action` declarations.

- [ ] **Step 4: Add a stable-log lookup test**

Append to `tests/db_test.rs`:

```rust
#[test]
fn get_log_by_id_returns_exact_entry_or_none() {
    let db = Database::open_in_memory().unwrap();
    let practice = db
        .create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    let first = db
        .create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("first"),
            None,
            None,
            Some(7),
        )
        .unwrap();
    let second = db
        .create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 12 }],
            Some("second"),
            None,
            None,
            Some(8),
        )
        .unwrap();

    assert_eq!(
        db.get_log_by_id(second).unwrap().unwrap().log.note.as_deref(),
        Some("second")
    );
    assert_eq!(db.get_log_by_id(first).unwrap().unwrap().log.id, first);
    assert!(db.get_log_by_id(i64::MAX).unwrap().is_none());
}
```

- [ ] **Step 5: Run the database test to verify RED**

Run:

```bash
cargo test --test db_test get_log_by_id_returns_exact_entry_or_none -- --exact --nocapture
```

Expected: compilation fails because `get_log_by_id` does not exist.

- [ ] **Step 6: Implement stable-log lookup without a schema change**

Add to `impl Database` in `src/db.rs`:

```rust
pub fn get_log_by_id(&self, log_id: i64) -> Result<Option<LogEntry>> {
    Ok(self
        .list_logs_all()?
        .into_iter()
        .find(|entry| entry.log.id == log_id))
}
```

This deliberately reuses the canonical log materialization query. Batch 3A does not add a second row-to-domain mapping or alter schema.

- [ ] **Step 7: Convert every navigation producer and the temporary app consumer**

Use these exact mappings:

```rust
// Dashboard
KeyCode::Char('l') => Action::Navigate(NavigationIntent::NewLog),
KeyCode::Char('h') => Action::Navigate(NavigationIntent::Open(ModuleId::History)),
KeyCode::Char('t') => Action::Navigate(NavigationIntent::Open(ModuleId::Trends)),
KeyCode::Char('e') => Action::Navigate(NavigationIntent::Open(ModuleId::Practices)),
KeyCode::Char('g') => Action::Navigate(NavigationIntent::Open(ModuleId::Goals)),
KeyCode::Char('Q') => Action::Navigate(NavigationIntent::Open(ModuleId::Quotes)),

// Ordinary module Esc
Action::Navigate(NavigationIntent::Open(ModuleId::Dashboard))

// History edit
Action::Navigate(NavigationIntent::EditHistoryLog {
    log_id: self.selected_entry().expect("selected entry").log.id,
})
```

Change `LogEntryScreen::return_to` from `Screen` to `ModuleId`. New forms use `ModuleId::Dashboard`; edit forms use `ModuleId::History`; successful save and confirmed cancel return `NavigationIntent::Open(self.return_to)`.

Temporarily keep transition policy in `app.rs`, but consume the typed intent:

```rust
match intent {
    NavigationIntent::Open(module) => match module {
        ModuleId::Dashboard => dashboard.refresh(db)?,
        ModuleId::Goals => goals_screen = GoalsScreen::new(db)?,
        ModuleId::History => history.refresh_preserving_context(db)?,
        ModuleId::Trends => trends = TrendsScreen::new(db)?,
        ModuleId::Practices => practices = PracticesScreen::new(db)?,
        ModuleId::Quotes => quotes_screen = QuotesScreen::new(db)?,
    },
    NavigationIntent::NewLog => {
        log_entry = LogEntryScreen::new(db)?;
    }
    NavigationIntent::EditHistoryLog { log_id } => {
        let entry = db
            .get_log_by_id(log_id)?
            .ok_or_else(|| anyhow::anyhow!("log {log_id} no longer exists"))?;
        log_entry = LogEntryScreen::from_existing(db, &entry)?;
    }
}
current_screen = intent.target();
```

Set `current_screen = intent.target()` only after the target operation succeeds.

- [ ] **Step 8: Verify typed navigation and existing workflows**

Run:

```bash
cargo test tui::navigation::tests -- --nocapture
cargo test --test db_test get_log_by_id_returns_exact_entry_or_none -- --exact --nocapture
cargo test app::tests -- --nocapture
cargo test tui::log_entry::tests -- --nocapture
cargo test tui::history::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass with no warnings. The History handler must emit the selected stable log ID; `app.rs` must not call `history.selected_entry()` to construct an edit.

- [ ] **Step 9: Commit typed navigation**

```bash
git add src/tui/navigation.rs src/tui/mod.rs src/db.rs tests/db_test.rs src/app.rs \
  src/tui/dashboard.rs src/tui/goals.rs src/tui/history.rs src/tui/log_entry.rs \
  src/tui/trends.rs src/tui/practices.rs src/tui/quotes_screen.rs
git commit -m "refactor(navigation): add typed screen intents"
```

---

### Task 2: Add Lifecycle Contracts and Migrate Dashboard and History

**Files:**
- Create: `src/tui/lifecycle.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/dashboard.rs`
- Modify: `src/tui/history.rs`

**Interfaces:**
- Consumes: `Database`, concrete screen status fields, shared viewport/input state.
- Produces: `Reenter::reenter(&self, db) -> Result<Self>` and `NavigationStatus::set_navigation_error(message)` implementations for Dashboard and History.

- [ ] **Step 1: Define the static lifecycle contracts**

Create `src/tui/lifecycle.rs`:

```rust
use crate::db::Database;

pub(crate) trait Reenter: Sized {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self>;
}

pub(crate) trait NavigationStatus {
    fn set_navigation_error(&mut self, message: String);
}
```

Expose it internally from `src/tui/mod.rs`:

```rust
pub(crate) mod lifecycle;
```

- [ ] **Step 2: Add Dashboard re-entry tests**

Inside `src/tui/dashboard.rs` tests, add:

```rust
#[test]
fn dashboard_reentry_loads_fresh_data_and_clears_transient_state() {
    let db = Database::open_in_memory().unwrap();
    let mut screen = DashboardScreen::new(&db, true).unwrap();
    screen.mode = DashboardMode::HrvInput;
    screen.hrv_input.set("88");
    screen.status_msg = Some(("old status".into(), true));
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.set_daily_hrv(&today, 64).unwrap();

    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.mode, DashboardMode::Normal);
    assert_eq!(next.hrv_today, Some(64));
    assert!(next.hrv_input.value().is_empty());
    assert!(next.status_msg.is_none());
    assert!(next.no_color);
}

#[test]
fn failed_dashboard_reentry_does_not_mutate_cached_screen() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(temp.path()).unwrap();
    let mut screen = DashboardScreen::new(&db, false).unwrap();
    screen.mode = DashboardMode::HrvInput;
    screen.hrv_input.set("77");
    rusqlite::Connection::open(temp.path())
        .unwrap()
        .execute("DROP TABLE quotes", [])
        .unwrap();

    assert!(screen.reenter(&db).is_err());
    assert_eq!(screen.mode, DashboardMode::HrvInput);
    assert_eq!(screen.hrv_input.value(), "77");
}
```

- [ ] **Step 3: Run Dashboard tests to verify RED**

Run:

```bash
cargo test tui::dashboard::tests::dashboard_reentry -- --nocapture
cargo test tui::dashboard::tests::failed_dashboard_reentry -- --nocapture
```

Expected: compilation fails because `Reenter` is not implemented.

- [ ] **Step 4: Implement atomic Dashboard re-entry**

Import the traits and implement:

```rust
impl Reenter for DashboardScreen {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self> {
        Self::new(db, self.no_color)
    }
}

impl NavigationStatus for DashboardScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}
```

Refactor `DashboardScreen::new` so every required read, including `aggregate_stats(7)`, uses `?` before constructing `Self`; do not mutate an existing screen field-by-field. Keep `refresh` compiling through Task 5 as an atomic compatibility wrapper:

```rust
pub fn refresh(&mut self, db: &Database) -> anyhow::Result<()> {
    *self = self.reenter(db)?;
    Ok(())
}
```

Task 6 removes this wrapper when `ScreenStore::prepare` becomes the only ordinary re-entry path.

- [ ] **Step 5: Add History re-entry contract tests**

Extend the existing History context fixture with a cross-page test:

```rust
#[test]
fn history_reentry_preserves_browsing_context_and_clears_transients() {
    let db = history_db_with_multiple_filtered_entries();
    let mut screen = HistoryScreen::new(&db).unwrap();
    enter_press_filter_and_select_second(&mut screen, &db);
    let selected_id = screen.selected_entry().unwrap().log.id;
    screen.mode = Mode::Detail;
    screen.detail_state.focused = true;
    screen.list_viewport.set_offset(1);
    screen.detail_state.viewport.set_offset(2);
    screen.status_msg = Some(("old status".into(), false));
    screen.last_deleted = Some(screen.selected_entry().unwrap().clone());

    db.update_log(
        selected_id,
        &[SetData::Bodyweight { reps: 15 }],
        Some("fresh note"),
        None,
        None,
        None,
        Some(8),
    )
    .unwrap();
    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.selected_entry().unwrap().log.id, selected_id);
    assert_eq!(next.selected_entry().unwrap().log.note.as_deref(), Some("fresh note"));
    assert_eq!(next.filter.value(), "press");
    assert!(!next.filtering);
    assert_eq!(next.mode, Mode::Detail);
    assert!(next.detail_state.focused);
    assert!(next.status_msg.is_none());
    assert!(next.last_deleted.is_none());
}
```

Define the two test helpers in the same test module:

```rust
fn history_db_with_multiple_filtered_entries() -> Database {
    let db = Database::open_in_memory().unwrap();
    let bench = db
        .create_practice("Bench Press", PracticeType::Bodyweight)
        .unwrap();
    let leg = db
        .create_practice("Leg Press", PracticeType::Bodyweight)
        .unwrap();
    for (practice, note) in [(&bench, "bench"), (&leg, "leg")] {
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some(note),
            None,
            None,
            Some(7),
        )
        .unwrap();
    }
    db
}

fn enter_press_filter_and_select_second(screen: &mut HistoryScreen, db: &Database) {
    screen.handle_key(key(KeyCode::Char('/')), db);
    for character in "press".chars() {
        screen.handle_key(key(KeyCode::Char(character)), db);
    }
    screen.handle_key(key(KeyCode::Enter), db);
    screen.handle_key(key(KeyCode::Char('j')), db);
}
```

Add a second test that deletes the selected log before `reenter` and asserts selection is valid or zero when empty.

- [ ] **Step 6: Run History tests to verify RED**

Run:

```bash
cargo test tui::history::tests::history_reentry -- --nocapture
```

Expected: compilation fails because History does not implement `Reenter`.

- [ ] **Step 7: Implement History cross-page re-entry separately from in-place reload**

Implement `Reenter` by building `let mut next = Self::new(db)?;`, transferring filter text, rebuilding `filtered_indices`, selecting by stable log ID, copying and clamping list/detail viewports, and preserving `Mode::Detail` only when the old mode was Detail. Always set:

```rust
next.filtering = false;
next.status_msg = None;
next.last_deleted = None;
```

Keep `refresh_preserving_context` for in-place delete/undo reload semantics, where the undo payload remains valid because the screen was not left.

Implement navigation error status:

```rust
impl NavigationStatus for HistoryScreen {
    fn set_navigation_error(&mut self, message: String) {
        self.status_msg = Some((message, true));
    }
}
```

- [ ] **Step 8: Verify both lifecycle implementations**

Run:

```bash
cargo test tui::dashboard::tests -- --nocapture
cargo test tui::history::tests -- --nocapture
cargo test app::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass. Existing in-place History delete/undo behavior remains unchanged.

- [ ] **Step 9: Commit Dashboard and History lifecycle**

```bash
git add src/tui/lifecycle.rs src/tui/mod.rs src/tui/dashboard.rs src/tui/history.rs
git commit -m "refactor(lifecycle): add dashboard and history reentry"
```

---

### Task 3: Implement Goals Re-entry by Stable Goal and Milestone Identity

**Files:**
- Modify: `src/tui/goals.rs`

**Interfaces:**
- Consumes: `Reenter`, `NavigationStatus`, `Goal.id`, `Milestone.id`.
- Produces: atomic Goals re-entry that preserves browsing selection but clears mutations, confirmations, statuses, and undo data.

- [ ] **Step 1: Add Goals re-entry tests**

Inside `src/tui/goals.rs` tests add:

```rust
#[test]
fn goals_reentry_preserves_stable_browse_identity_and_clears_transients() {
    let db = Database::open_in_memory().unwrap();
    let first = db.create_goal("First").unwrap();
    let second = db.create_goal("Second").unwrap();
    let milestone = db.create_milestone(second, "Target").unwrap();
    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.selected = screen.goals.iter().position(|goal| goal.id == second).unwrap();
    screen.mode = Mode::Modal;
    screen.modal_goal_idx = screen.selected;
    screen.modal_selected = 0;
    screen.modal_mode = ModalMode::EditMilestone;
    screen.input.set("discard me");
    screen.status_msg = Some(("old".into(), false));
    screen.last_deleted = Some(GoalUndoData::Goal(screen.goals[0].clone()));

    db.update_goal_title(first, "First changed").unwrap();
    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.goals[next.modal_goal_idx].id, second);
    assert_eq!(next.goals[next.modal_goal_idx].milestones[next.modal_selected].id, milestone);
    assert_eq!(next.mode, Mode::Modal);
    assert_eq!(next.modal_mode, ModalMode::Browse);
    assert!(next.input.value().is_empty());
    assert!(next.status_msg.is_none());
    assert!(next.modal_status_msg.is_none());
    assert!(next.last_deleted.is_none());
    assert!(next.modal_last_deleted.is_none());
    assert_eq!(next.goals.iter().find(|goal| goal.id == first).unwrap().title, "First changed");
}
```

Add a second test starting in `Mode::AddGoal` and assert re-entry returns `Mode::Browse` while restoring the old stable selected goal if it still exists. Add a third test deleting the selected goal before re-entry and assert indices/viewports clamp.

- [ ] **Step 2: Run Goals tests to verify RED**

Run:

```bash
cargo test tui::goals::tests::goals_reentry -- --nocapture
```

Expected: compilation fails because Goals does not implement `Reenter`.

- [ ] **Step 3: Implement Goals lifecycle**

Implement `Reenter` inside the Goals module so it can inspect private modes:

1. Capture selected goal ID.
2. Capture modal goal ID and selected milestone ID when the old screen is in any modal mode.
3. Construct `let mut next = GoalsScreen::new(db)?;`.
4. Restore stable IDs and copy/clamp approved viewports.
5. Preserve modal browsing when the old screen was modal, but force `ModalMode::Browse`.
6. Force non-modal add/edit/date/delete states to `Mode::Browse`.
7. Clear both inputs, both statuses, and both undo payloads.

Implement error placement:

```rust
impl NavigationStatus for GoalsScreen {
    fn set_navigation_error(&mut self, message: String) {
        if self.mode == Mode::Modal {
            self.modal_status_msg = Some((message, true));
        } else {
            self.status_msg = Some((message, true));
        }
    }
}
```

- [ ] **Step 4: Verify Goals behavior and geometry**

Run:

```bash
cargo test tui::goals::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: Goals tests and TUI lint pass; no warning or geometry behavior changes.

- [ ] **Step 5: Commit Goals lifecycle**

```bash
git add src/tui/goals.rs tests/tui_lint_test.rs
git commit -m "refactor(goals): add context-preserving reentry"
```

Only stage `tests/tui_lint_test.rs` if a mechanical line refresh was required.

---

### Task 4: Move Trends Refresh into Its Screen Lifecycle

**Files:**
- Modify: `src/tui/trends.rs`
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `Reenter`, `NavigationStatus`, selected `Practice.id`, `days_window`.
- Produces: `TrendsScreen::handle_key(&mut self, key, db) -> Action` and an atomic re-entry with chart data already loaded.

- [ ] **Step 1: Add Trends lifecycle tests**

Inside `src/tui/trends.rs` tests add:

```rust
#[test]
fn trends_reentry_preserves_filter_practice_window_and_loads_chart() {
    let db = trends_db_with_press_and_pull_logs();
    let mut screen = TrendsScreen::new(&db).unwrap();
    screen.filter.set("press");
    screen.filtering = true;
    screen.apply_filter();
    screen.days_window = 30;
    let selected_id = screen.chosen_practice.as_ref().unwrap().id;
    screen.status_msg = Some(("old".into(), true));

    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.filter.value(), "press");
    assert!(!next.filtering);
    assert_eq!(next.chosen_practice.as_ref().unwrap().id, selected_id);
    assert_eq!(next.days_window, 30);
    assert!(!next.needs_refresh);
    assert!(!next.entries.is_empty());
    assert!(next.status_msg.is_none());
}
```

Define the fixture in the same test module:

```rust
fn trends_db_with_press_and_pull_logs() -> Database {
    let db = Database::open_in_memory().unwrap();
    let press = db
        .create_practice("Bench Press", PracticeType::Bodyweight)
        .unwrap();
    let pull = db
        .create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    for practice in [&press, &pull] {
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("trend fixture"),
            None,
            None,
            Some(7),
        )
        .unwrap();
    }
    db
}
```

Add a missing-practice test that deactivates the selected practice before re-entry and asserts the next valid filtered selection is used. Add a failure test that removes the logs table before re-entry and asserts `reenter` returns `Err` while the cached screen fields remain unchanged.

- [ ] **Step 2: Run Trends tests to verify RED**

Run:

```bash
cargo test tui::trends::tests::trends_reentry -- --nocapture
```

Expected: compilation fails because Trends does not implement `Reenter`.

- [ ] **Step 3: Make chart loading explicit and fallible**

Replace the event-loop side effect with a screen-owned method:

```rust
fn load_chart(&mut self, db: &Database) -> anyhow::Result<()> {
    self.entries = match &self.chosen_practice {
        Some(practice) => db.list_logs_for_practice(practice.id, self.days_window)?,
        None => Vec::new(),
    };
    self.needs_refresh = false;
    Ok(())
}
```

Change the public handler signature:

```rust
pub fn handle_key(&mut self, key: KeyEvent, db: &Database) -> Action
```

After a key changes practice/filter/window, call `load_chart`. On an in-place chart-load failure, keep the current selection/window and set the existing error status. Remove `trends.refresh_chart(db)` from `app.rs`.

- [ ] **Step 4: Implement Trends re-entry**

Construct a new screen, copy filter text and `days_window`, rebuild filtered indices, restore selected practice by ID, copy/clamp list viewport, set `filtering = false`, clear status, and call `next.load_chart(db)?` before returning.

Implement `NavigationStatus` by assigning an error status.

- [ ] **Step 5: Verify Trends and app dispatch**

Run:

```bash
cargo test tui::trends::tests -- --nocapture
cargo test app::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass. `src/app.rs` contains no Trends-specific post-key refresh call.

- [ ] **Step 6: Commit Trends lifecycle**

```bash
git add src/tui/trends.rs src/app.rs tests/tui_lint_test.rs
git commit -m "refactor(trends): own chart lifecycle"
```

Only stage the lint file if mechanically refreshed.

---

### Task 5: Add Practices and Quotes Re-entry Contracts

**Files:**
- Modify: `src/tui/practices.rs`
- Modify: `src/tui/quotes_screen.rs`

**Interfaces:**
- Consumes: `Reenter`, `NavigationStatus`, stable practice/quote IDs.
- Produces: two ordinary cached screens that preserve browse context and clear mutation state.

- [ ] **Step 1: Add Practices re-entry tests**

Add inside Practices tests:

```rust
#[test]
fn practices_reentry_preserves_selection_and_clears_edit_state() {
    let db = Database::open_in_memory().unwrap();
    let first = db.create_practice("First", PracticeType::Bodyweight).unwrap();
    let second = db.create_practice("Second", PracticeType::Weighted).unwrap();
    let mut screen = PracticesScreen::new(&db).unwrap();
    screen.selected = screen.practices.iter().position(|p| p.id == second.id).unwrap();
    screen.mode = Mode::EditName;
    screen.input.set("discard me");
    screen.status_msg = Some(("old".into(), true));
    db.rename_practice(first.id, "First changed").unwrap();

    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.practices[next.selected].id, second.id);
    assert_eq!(next.mode, Mode::Browse);
    assert!(next.input.value().is_empty());
    assert!(next.status_msg.is_none());
    assert_eq!(next.practices.iter().find(|p| p.id == first.id).unwrap().name, "First changed");
}
```

Add a selected-practice deletion test that asserts safe index/viewport clamping.

- [ ] **Step 2: Add Quotes re-entry tests**

Add inside Quotes tests:

```rust
#[test]
fn quotes_reentry_preserves_detail_context_and_clears_mutations() {
    let db = Database::open_in_memory().unwrap();
    let first = db.create_quote("First quote").unwrap();
    let second = db.create_quote("Second quote").unwrap();
    let mut screen = QuotesScreen::new(&db).unwrap();
    screen.selected = screen
        .quotes
        .iter()
        .position(|quote| quote.id == second.id)
        .unwrap();
    screen.mode = Mode::Detail;
    screen.detail_state.focused = true;
    screen.detail_state.viewport.set_offset(2);
    screen.input.set("discard me");
    screen.status_msg = Some(("old".into(), true));
    screen.last_deleted = Some(screen.quotes[0].clone());
    db.update_quote(first.id, "First changed").unwrap();

    let next = screen.reenter(&db).unwrap();

    assert_eq!(next.quotes[next.selected].id, second.id);
    assert_eq!(next.mode, Mode::Detail);
    assert!(next.detail_state.focused);
    assert!(next.input.value().is_empty());
    assert!(next.editing_id.is_none());
    assert!(next.status_msg.is_none());
    assert!(next.last_deleted.is_none());
    assert_eq!(
        next.quotes.iter().find(|q| q.id == first.id).unwrap().text,
        "First changed"
    );
}
```

- [ ] **Step 3: Run both screen suites to verify RED**

Run:

```bash
cargo test tui::practices::tests::practices_reentry -- --nocapture
cargo test tui::quotes_screen::tests::quotes_reentry -- --nocapture
```

Expected: compilation fails because neither screen implements `Reenter`.

- [ ] **Step 4: Implement both lifecycle contracts**

For each screen:

1. Capture the stable selected ID and approved viewport/detail state.
2. Construct a new screen with `Self::new(db)?`.
3. Restore the stable selected identity or clamp the previous position.
4. Copy and clamp viewports.
5. Preserve Quotes `Mode::Detail` and detail focus only when previously in Detail.
6. Force all Practices mutation modes and all Quotes add/edit/delete modes to Browse.
7. Clear inputs, editing IDs, statuses, and Quotes undo payload.

Implement `NavigationStatus` for each by assigning its root status message.

- [ ] **Step 5: Verify mutation and geometry regressions**

Run:

```bash
cargo test tui::practices::tests -- --nocapture
cargo test tui::quotes_screen::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass, including existing failed-mutation retention tests.

- [ ] **Step 6: Commit Practices and Quotes lifecycle**

```bash
git add src/tui/practices.rs src/tui/quotes_screen.rs tests/tui_lint_test.rs
git commit -m "refactor(lifecycle): add practices and quotes reentry"
```

Only stage the lint file if mechanically refreshed.

---

### Task 6: Add the Concrete ScreenStore and Prepared Targets

**Files:**
- Create: `src/app/screens.rs`
- Modify: `src/app.rs`
- Modify: `src/tui/log_entry.rs`

**Interfaces:**
- Consumes: typed navigation, every ordinary screen's `Reenter`, `NavigationStatus`, and `Database::get_log_by_id`.
- Produces: `ScreenStore`, `PreparedScreen`, `prepare`, `install`, centralized render/key dispatch, and Log Entry navigation error status.

- [ ] **Step 1: Add ScreenStore preparation tests**

Create `src/app/screens.rs` with tests that expect these APIs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PracticeType, SetData};

    #[test]
    fn edit_intent_prepares_exact_log_id_without_history_selection_access() {
        let db = Database::open_in_memory().unwrap();
        let practice = db.create_practice("Pull-up", PracticeType::Bodyweight).unwrap();
        let log_id = db
            .create_log(
                practice.id,
                &[SetData::Bodyweight { reps: 10 }],
                Some("target"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        let store = ScreenStore::new(&db, false).unwrap();

        let prepared = store
            .prepare(NavigationIntent::EditHistoryLog { log_id }, &db)
            .unwrap();

        assert_eq!(prepared.target(), ScreenId::LogEntry);
        assert_eq!(prepared.log_entry_for_test().identity_for_test().0, Some(log_id));
    }

    #[test]
    fn missing_edit_target_does_not_replace_cached_log_entry() {
        let db = Database::open_in_memory().unwrap();
        let mut store = ScreenStore::new(&db, false).unwrap();
        let before = store.log_entry_for_test().identity_for_test();

        assert!(store
            .prepare(
                NavigationIntent::EditHistoryLog { log_id: i64::MAX },
                &db,
            )
            .is_err());
        assert_eq!(store.log_entry_for_test().identity_for_test(), before);
    }
}
```

Test-only accessors return simple IDs/phase values rather than exposing mutable screen fields.

- [ ] **Step 2: Run ScreenStore tests to verify RED**

Run:

```bash
cargo test app::screens::tests -- --nocapture
```

Expected: compilation fails because `ScreenStore` does not exist.

- [ ] **Step 3: Implement concrete storage and prepared targets**

Use this shape:

```rust
pub(crate) struct ScreenStore {
    dashboard: DashboardScreen,
    goals: GoalsScreen,
    log_entry: LogEntryScreen,
    history: HistoryScreen,
    trends: TrendsScreen,
    practices: PracticesScreen,
    quotes: QuotesScreen,
}

pub(crate) enum PreparedScreen {
    Dashboard(DashboardScreen),
    Goals(GoalsScreen),
    LogEntry(LogEntryScreen),
    History(HistoryScreen),
    Trends(TrendsScreen),
    Practices(PracticesScreen),
    Quotes(QuotesScreen),
}
```

Implement:

```rust
impl PreparedScreen {
    pub(crate) fn target(&self) -> ScreenId;
}

impl ScreenStore {
    pub(crate) fn new(db: &Database, no_color: bool) -> anyhow::Result<Self>;
    pub(crate) fn render(&mut self, current: ScreenId, frame: &mut Frame);
    pub(crate) fn handle_key(
        &mut self,
        current: ScreenId,
        key: KeyEvent,
        db: &Database,
    ) -> Action;
    pub(crate) fn prepare(
        &self,
        intent: NavigationIntent,
        db: &Database,
    ) -> anyhow::Result<PreparedScreen>;
    pub(crate) fn install(&mut self, prepared: PreparedScreen);
    pub(crate) fn set_navigation_error(&mut self, source: ScreenId, message: String);
}
```

`prepare(Open(module))` calls the concrete cached target's `reenter`. `prepare(NewLog)` constructs a new Log Entry. `prepare(EditHistoryLog { log_id })` loads the exact ID and constructs an edit form. `install` is infallible.

Add these exact test-only accessors:

```rust
// src/tui/log_entry.rs
#[cfg(test)]
pub(crate) fn identity_for_test(&self) -> (Option<i64>, ModuleId) {
    (self.editing_log_id, self.return_to)
}

#[cfg(test)]
pub(crate) fn form_value_for_test(&self) -> (&str, &str) {
    (self.note.value(), self.rpe_input.value())
}

// src/app/screens.rs
#[cfg(test)]
impl PreparedScreen {
    pub(crate) fn log_entry_for_test(&self) -> &LogEntryScreen {
        match self {
            Self::LogEntry(screen) => screen,
            _ => panic!("prepared target is not Log Entry"),
        }
    }
}

#[cfg(test)]
impl ScreenStore {
    pub(crate) fn log_entry_for_test(&self) -> &LogEntryScreen {
        &self.log_entry
    }

    pub(crate) fn practices_identity_for_test(&self) -> Vec<i64> {
        self.practices.ids_for_test()
    }

    pub(crate) fn goals_titles_for_test(&self) -> Vec<String> {
        self.goals.titles_for_test()
    }

    pub(crate) fn goals_context_for_test(&self) -> (Option<i64>, usize, bool) {
        self.goals.context_for_test()
    }

    pub(crate) fn log_entry_context_for_test(
        &self,
    ) -> ((Option<i64>, ModuleId), (String, String)) {
        let identity = self.log_entry.identity_for_test();
        let (note, rpe) = self.log_entry.form_value_for_test();
        (identity, (note.to_owned(), rpe.to_owned()))
    }

    pub(crate) fn history_context_for_test(&self) -> (Option<i64>, Option<String>, String) {
        self.history.context_for_test()
    }
}
```

Add these test-only concrete-screen accessors:

```rust
// src/tui/practices.rs
#[cfg(test)]
pub(crate) fn ids_for_test(&self) -> Vec<i64> {
    self.practices.iter().map(|practice| practice.id).collect()
}

// src/tui/goals.rs
#[cfg(test)]
pub(crate) fn titles_for_test(&self) -> Vec<String> {
    self.goals.iter().map(|goal| goal.title.clone()).collect()
}

#[cfg(test)]
pub(crate) fn context_for_test(&self) -> (Option<i64>, usize, bool) {
    (
        self.goals.get(self.selected).map(|goal| goal.id),
        self.list_viewport.offset(),
        self.mode == Mode::Modal,
    )
}

// src/tui/history.rs
#[cfg(test)]
pub(crate) fn context_for_test(&self) -> (Option<i64>, Option<String>, String) {
    (
        self.selected_entry().map(|entry| entry.log.id),
        self.selected_entry().and_then(|entry| entry.log.note.clone()),
        self.filter.value().to_owned(),
    )
}
```

Implement `NavigationStatus` for `LogEntryScreen` by setting its existing `status_msg`; this must not clear or replace any form field.

- [ ] **Step 4: Move render and key dispatch behind ScreenStore**

Declare in `src/app.rs`:

```rust
mod screens;
use screens::ScreenStore;
```

Replace seven local screen variables with one `ScreenStore`. Replace the render and key-handler matches with `screens.render(current_screen, frame)` and `screens.handle_key(current_screen, key, db)`.

Keep the transition sequence temporarily in `app.rs`:

```rust
let prepared = screens.prepare(intent, db)?;
let target = prepared.target();
screens.install(prepared);
current_screen = target;
```

Delete `DashboardScreen::refresh`; no application consumer remains. Keep History's `refresh_preserving_context` because delete/undo still use it internally.

- [ ] **Step 5: Verify store dispatch and prepare/install atomicity**

Run:

```bash
cargo test app::screens::tests -- --nocapture
cargo test app::tests -- --nocapture
cargo test tui:: -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass. `src/app.rs` no longer imports concrete screen types or matches on current screen for rendering, key handling, construction, or refresh.

- [ ] **Step 6: Commit ScreenStore**

```bash
git add src/app.rs src/app/screens.rs src/tui/log_entry.rs
git commit -m "refactor(app): centralize concrete screen storage"
```

---

### Task 7: Add Router and Terminal-Independent AppShell

**Files:**
- Create: `src/app/router.rs`
- Modify: `src/app.rs`
- Modify: `src/app/screens.rs`

**Interfaces:**
- Consumes: `ScreenStore::prepare/install`, `NavigationIntent`, `ScreenId`.
- Produces: `Router::transition`, `AppShell`, `ShellControl`, and a terminal loop free of lifecycle policy.

- [ ] **Step 1: Add Router success and commit-order tests**

Create `src/app/router.rs` tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_transition_installs_target_before_route_commit() {
        let db = Database::open_in_memory().unwrap();
        let mut store = ScreenStore::new(&db, false).unwrap();
        db.create_goal("fresh goal").unwrap();
        let mut current = ScreenId::Dashboard;

        Router::default()
            .transition(
                &mut current,
                &mut store,
                NavigationIntent::Open(ModuleId::Goals),
                &db,
            )
            .unwrap();

        assert_eq!(current, ScreenId::Goals);
        assert!(store
            .goals_titles_for_test()
            .contains(&"fresh goal".to_string()));
    }

    #[test]
    fn failed_transition_does_not_commit_route_or_target() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let mut store = ScreenStore::new(&db, false).unwrap();
        let before = store.practices_identity_for_test();
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE practices", [])
            .unwrap();
        let mut current = ScreenId::Dashboard;

        assert!(Router::default()
            .transition(
                &mut current,
                &mut store,
                NavigationIntent::Open(ModuleId::Practices),
                &db,
            )
            .is_err());

        assert_eq!(current, ScreenId::Dashboard);
        assert_eq!(store.practices_identity_for_test(), before);
    }
}
```

- [ ] **Step 2: Run Router tests to verify RED**

Run:

```bash
cargo test app::router::tests -- --nocapture
```

Expected: compilation fails because `Router` does not exist.

- [ ] **Step 3: Implement prepare-then-commit Router**

Create:

```rust
#[derive(Debug, Default)]
pub(crate) struct Router;

impl Router {
    pub(crate) fn transition(
        &self,
        current: &mut ScreenId,
        screens: &mut ScreenStore,
        intent: NavigationIntent,
        db: &Database,
    ) -> anyhow::Result<()> {
        let prepared = screens.prepare(intent, db)?;
        let target = prepared.target();
        screens.install(prepared);
        *current = target;
        Ok(())
    }
}
```

There must be no fallible call after `screens.install`.

- [ ] **Step 4: Add AppShell behavior tests**

In `src/app.rs` tests, replace direct screen wiring with:

```rust
#[test]
fn app_shell_routes_dashboard_to_goals_and_back() {
    let db = Database::open_in_memory().unwrap();
    let mut shell = AppShell::new(&db, false).unwrap();

    assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
    assert_eq!(
        shell.handle_key(key(KeyCode::Char('g')), &db).unwrap(),
        ShellControl::Continue
    );
    assert_eq!(shell.current_for_test(), ScreenId::Goals);
    assert_eq!(
        shell.handle_key(key(KeyCode::Esc), &db).unwrap(),
        ShellControl::Continue
    );
    assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
}
```

Replace the existing direct-wiring History tests with these shell helpers and workflows:

```rust
fn lifecycle_shell_fixture() -> (Database, AppShell, i64) {
    let db = Database::open_in_memory().unwrap();
    let bench = db
        .create_practice("Bench Press", PracticeType::Bodyweight)
        .unwrap();
    let leg = db
        .create_practice("Leg Press", PracticeType::Bodyweight)
        .unwrap();
    let pull = db
        .create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    let sets = [SetData::Bodyweight { reps: 10 }];
    let logged_at = |day| {
        NaiveDate::from_ymd_opt(2026, 8, day)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    };
    db.create_log_at(
        bench.id,
        &logged_at(12),
        &sets,
        Some("newest press"),
        None,
        None,
        Some(7),
    )
    .unwrap();
    let target_id = db
        .create_log_at(
            leg.id,
            &logged_at(11),
            &sets,
            Some("original target"),
            None,
            None,
            Some(7),
        )
        .unwrap();
    db.create_log_at(
        pull.id,
        &logged_at(10),
        &sets,
        Some("outside filter"),
        None,
        None,
        Some(7),
    )
    .unwrap();
    let shell = AppShell::new(&db, false).unwrap();
    (db, shell, target_id)
}

fn open_filtered_second_history_log(shell: &mut AppShell, db: &Database) {
    assert_eq!(shell.handle_key(key(KeyCode::Char('h')), db).unwrap(), ShellControl::Continue);
    assert_eq!(shell.handle_key(key(KeyCode::Char('/')), db).unwrap(), ShellControl::Continue);
    for character in "press".chars() {
        assert_eq!(
            shell.handle_key(key(KeyCode::Char(character)), db).unwrap(),
            ShellControl::Continue
        );
    }
    shell.handle_key(key(KeyCode::Enter), db).unwrap();
    shell.handle_key(key(KeyCode::Char('j')), db).unwrap();
    shell.handle_key(key(KeyCode::Char('e')), db).unwrap();
}

#[test]
fn shell_history_save_returns_to_same_refreshed_log() {
    let (db, mut shell, target_id) = lifecycle_shell_fixture();
    open_filtered_second_history_log(&mut shell, &db);
    assert_eq!(shell.current_for_test(), ScreenId::LogEntry);
    assert_eq!(
        shell.screens.log_entry_for_test().identity_for_test().0,
        Some(target_id)
    );
    for _ in 0..4 {
        shell.handle_key(key(KeyCode::Tab), &db).unwrap();
    }
    shell.handle_key(ctrl_key('a'), &db).unwrap();
    shell.handle_key(ctrl_key('k'), &db).unwrap();
    for character in "updated target".chars() {
        shell.handle_key(key(KeyCode::Char(character)), &db).unwrap();
    }
    shell.handle_key(ctrl_key('s'), &db).unwrap();

    assert_eq!(shell.current_for_test(), ScreenId::History);
    assert_eq!(
        shell.screens.history_context_for_test(),
        (Some(target_id), Some("updated target".into()), "press".into())
    );
    shell.handle_key(key(KeyCode::Char('e')), &db).unwrap();
    assert_eq!(
        shell.screens.log_entry_for_test().identity_for_test().0,
        Some(target_id)
    );
}

#[test]
fn shell_history_cancel_returns_to_same_unmodified_log() {
    let (db, mut shell, target_id) = lifecycle_shell_fixture();
    open_filtered_second_history_log(&mut shell, &db);
    shell.handle_key(key(KeyCode::Esc), &db).unwrap();
    shell.handle_key(key(KeyCode::Char('y')), &db).unwrap();

    assert_eq!(shell.current_for_test(), ScreenId::History);
    assert_eq!(
        shell.screens.history_context_for_test(),
        (Some(target_id), Some("original target".into()), "press".into())
    );
}
```

- [ ] **Step 5: Implement AppShell and simplify the terminal loop**

Use:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellControl {
    Continue,
    Quit,
}

struct AppShell {
    current: ScreenId,
    screens: ScreenStore,
    router: Router,
}
```

Implement `new`, `render`, and these exact Task 7 action methods:

```rust
fn handle_key(&mut self, key: KeyEvent, db: &Database) -> anyhow::Result<ShellControl>

fn apply_action(&mut self, action: Action, db: &Database) -> anyhow::Result<ShellControl>
```

`handle_key` delegates the screen-produced action to `apply_action`. Task 7 propagates transition errors. Task 8 changes both return types to `ShellControl` and converts transition errors into source-screen status without route changes.

The terminal loop retains only:

- terminal-size fallback rendering;
- `shell.render(frame)`;
- key press filtering and global Ctrl+C; and
- `shell.handle_key(key, db)` with Quit/Continue handling.

- [ ] **Step 6: Verify AppShell success workflows**

Run:

```bash
cargo test app::tests -- --nocapture
cargo test app::router::tests -- --nocapture
cargo test app::screens::tests -- --nocapture
cargo test tui::history::tests -- --nocapture
cargo test tui::log_entry::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: success routing and History workflows pass. `run_app` contains no concrete screen lifecycle logic.

- [ ] **Step 7: Commit Router and AppShell**

```bash
git add src/app.rs src/app/router.rs src/app/screens.rs
git commit -m "refactor(app): route through transactional shell"
```

---

### Task 8: Keep Navigation Failures on the Source Screen

**Files:**
- Modify: `src/app.rs`
- Modify: `src/app/router.rs`
- Modify: `src/app/screens.rs`
- Modify: `locales/en.ftl`
- Modify: `locales/zh-CN.ftl`

**Interfaces:**
- Consumes: `NavigationStatus`, Router errors, source `ScreenId`.
- Produces: source-visible localized failure handling with unchanged route and cached target.

- [ ] **Step 1: Add exact localization keys**

Add:

```ftl
# locales/en.ftl
navigation-refresh-error = Unable to open screen: { $msg }

# locales/zh-CN.ftl
navigation-refresh-error = 无法打开页面：{ $msg }
```

- [ ] **Step 2: Add two-source failure workflow tests**

In `src/app.rs` tests add a Dashboard-source failure:

```rust
#[test]
fn dashboard_navigation_failure_stays_put_and_preserves_cached_target() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(temp.path()).unwrap();
    let mut shell = AppShell::new(&db, false).unwrap();
    let target_before = shell.screens.practices_identity_for_test();
    rusqlite::Connection::open(temp.path())
        .unwrap()
        .execute("DROP TABLE practices", [])
        .unwrap();

    assert_eq!(shell.handle_key(key(KeyCode::Char('e')), &db), ShellControl::Continue);
    assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
    assert_eq!(shell.screens.practices_identity_for_test(), target_before);
    assert!(render_shell(&mut shell, 80, 35).contains("Unable to open screen"));
}
```

Add the rendering helper in the same test module:

```rust
fn render_shell(shell: &mut AppShell, width: u16, height: u16) -> String {
    crate::tui::framework::test_support::render_to_string(width, height, |frame| {
        shell.render(frame)
    })
}
```

Add an exact Goals-source failure:

```rust
#[test]
fn goals_navigation_failure_preserves_source_browse_context() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(temp.path()).unwrap();
    db.create_goal("Keep selected").unwrap();
    let mut shell = AppShell::new(&db, false).unwrap();
    assert_eq!(shell.handle_key(key(KeyCode::Char('g')), &db), ShellControl::Continue);
    let source_before = shell.screens.goals_context_for_test();
    rusqlite::Connection::open(temp.path())
        .unwrap()
        .execute("DROP TABLE daily_metrics", [])
        .unwrap();

    assert_eq!(shell.handle_key(key(KeyCode::Esc), &db), ShellControl::Continue);

    assert_eq!(shell.current_for_test(), ScreenId::Goals);
    assert_eq!(shell.screens.goals_context_for_test(), source_before);
    assert!(render_shell(&mut shell, 80, 24).contains("Unable to open screen"));
}
```

Add an exact request-scoped Log Entry-source failure. This calls the same `apply_action` path as a save/cancel return without requiring a test hook between the successful database mutation and its immediate navigation action:

```rust
#[test]
fn log_entry_return_failure_preserves_request_scoped_form() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let db = Database::open(temp.path()).unwrap();
    let practice = db
        .create_practice("Pull-up", PracticeType::Bodyweight)
        .unwrap();
    db.create_log(
        practice.id,
        &[SetData::Bodyweight { reps: 10 }],
        Some("keep form"),
        None,
        None,
        Some(7),
    )
    .unwrap();
    let mut shell = AppShell::new(&db, false).unwrap();
    assert_eq!(shell.handle_key(key(KeyCode::Char('h')), &db), ShellControl::Continue);
    assert_eq!(shell.handle_key(key(KeyCode::Char('e')), &db), ShellControl::Continue);
    let form_before = shell.screens.log_entry_context_for_test();
    rusqlite::Connection::open(temp.path())
        .unwrap()
        .execute("DROP TABLE training_sessions", [])
        .unwrap();

    assert_eq!(
        shell.apply_action(
            Action::Navigate(NavigationIntent::Open(ModuleId::History)),
            &db,
        ),
        ShellControl::Continue
    );

    assert_eq!(shell.current_for_test(), ScreenId::LogEntry);
    assert_eq!(shell.screens.log_entry_context_for_test(), form_before);
    assert!(render_shell(&mut shell, 80, 24).contains("Unable to open screen"));
}
```

- [ ] **Step 3: Run failure workflows to verify RED**

Run:

```bash
cargo test app::tests::dashboard_navigation_failure -- --nocapture
cargo test app::tests::goals_navigation_failure -- --nocapture
cargo test app::tests::log_entry_return_failure -- --nocapture
```

Expected: tests fail because AppShell still propagates Router errors or exits.

- [ ] **Step 4: Catch transition failure in AppShell**

Change `handle_key` to return `ShellControl` rather than `Result<ShellControl>` for screen actions. Use:

```rust
Action::Navigate(intent) => {
    let source = self.current;
    if let Err(error) = self
        .router
        .transition(&mut self.current, &mut self.screens, intent, db)
    {
        self.screens.set_navigation_error(
            source,
            tr_args(
                "navigation-refresh-error",
                &[("msg", FluentValue::from(error.to_string()))],
            ),
        );
    }
    ShellControl::Continue
}
```

Do not reset, reinstall, or re-enter the source after failure. The only allowed source mutation is its status message.

- [ ] **Step 5: Verify localization, failure atomicity, and all workflows**

Run:

```bash
cargo test app::tests -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test tui::dashboard::tests -- --nocapture
cargo test tui::goals::tests -- --nocapture
cargo test tui::history::tests -- --nocapture
cargo test tui::trends::tests -- --nocapture
cargo test tui::practices::tests -- --nocapture
cargo test tui::quotes_screen::tests -- --nocapture
cargo test tui::log_entry::tests -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all pass. Router tests still prove failed target preparation does not install or commit.

- [ ] **Step 6: Commit source-visible failure semantics**

```bash
git add src/app.rs src/app/router.rs src/app/screens.rs locales/en.ftl locales/zh-CN.ftl
git commit -m "feat(navigation): retain source on refresh failure"
```

---

### Task 9: Update Active Documentation and Run Final Release Gates

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_zh.md`
- Modify: `docs/specs/navigation-spec.md`
- Modify: `docs/specs/dashboard-spec.md`
- Modify: `docs/specs/goals-spec.md`
- Modify: `docs/specs/history-spec.md`
- Modify: `docs/specs/trends-spec.md`
- Modify: `docs/specs/practices-spec.md`
- Modify: `docs/specs/quotes-spec.md`
- Modify: `docs/specs/log-entry-spec.md`
- Modify: `docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md`
- Modify: `tests/tui_lint_test.rs` only if approved widget lines moved

**Interfaces:**
- Consumes: final Batch 3A runtime behavior.
- Produces: accurate shipped documentation, final gate evidence, and manual acceptance handoff.

- [ ] **Step 1: Document only shipped Batch 3A behavior**

Update active documents to state:

- navigation uses typed intents and stable Log Entry edit IDs;
- ordinary screens refresh current database data while preserving approved browse context;
- unfinished edit/confirmation state, old status, and cross-page undo payload clear on re-entry;
- target preparation is atomic and failure remains on the source screen with an error;
- navigation topology and shortcuts did not change; and
- Batch 3B UI component extraction is not shipped.

Mark Batch 3A complete in the framework design only after all implementation tasks pass review.

- [ ] **Step 2: Run formatting and static analysis**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: zero warnings and zero errors.

- [ ] **Step 3: Run full test and build gates**

```bash
cargo test --all-targets --all-features
cargo build --all-features
cargo test --test i18n_test -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
```

Expected: every test passes and the build exits zero.

- [ ] **Step 4: Run lifecycle and removal audits**

```bash
rg -n "DashboardScreen::new|GoalsScreen::new|HistoryScreen::new|TrendsScreen::new|PracticesScreen::new|QuotesScreen::new|LogEntryScreen::new|LogEntryScreen::from_existing|refresh_preserving_context|refresh_chart" src/app.rs
rg -n "Action::Navigate\(Screen|selected_entry\(\).*LogEntryScreen::from_existing" src
rg -n "filter_cursor|field1_cursor|field2_cursor|warm_up_cursor|cool_down_cursor|rpe_cursor|note_cursor|date_input_cursor|fn adjust_scroll|visible_input_spans" src/tui/log_entry.rs src/tui/dashboard.rs
rg -n -g '!removed_quick_log_test.rs' "QuickLog|quick_log|quicklog-|key-quick-log|create_abbreviation|list_abbreviations|update_abbreviation|delete_abbreviation|crate::llm|crate::config|ureq|toml[[:space:]]*=" src tests locales Cargo.toml
git diff --check 510c570..HEAD
git status --short
```

Expected:

- `src/app.rs` contains no concrete screen construction/refresh policy.
- no old screen-only navigation or application-side History selection read remains.
- duplicate input-state and live removal audits have no matches.
- whole-range diff check passes and the tracked worktree is clean after commit.

- [ ] **Step 5: Commit final documentation**

```bash
git add CLAUDE.md README.md README_zh.md docs/specs/navigation-spec.md \
  docs/specs/dashboard-spec.md docs/specs/goals-spec.md docs/specs/history-spec.md \
  docs/specs/trends-spec.md docs/specs/practices-spec.md docs/specs/quotes-spec.md \
  docs/specs/log-entry-spec.md \
  docs/superpowers/specs/2026-08-10-tui-consistency-framework-design.md \
  tests/tui_lint_test.rs
git commit -m "docs(tui): document Batch 3A lifecycle"
```

Only stage `tests/tui_lint_test.rs` if it contains an approved mechanical refresh.

- [ ] **Step 6: Request final whole-batch read-only review**

The reviewer compares `510c570..HEAD` and reports Critical, Important, and Minor findings. Require explicit verification of:

- typed intents contain every required target context;
- stable-ID selection and approved context survive ordinary re-entry;
- transient state clears exactly at cross-page re-entry, not on in-place failures;
- prepare failure leaves route and cached target unchanged and shows source error;
- no screen construction/refresh policy remains in the terminal loop;
- History edit save/cancel and second-edit workflows remain correct;
- all release gates and old Batch 1/2 regressions pass; and
- no Quick Log/LLM/Abbreviations surface is reintroduced.

Resolve every valid Critical or Important finding with a failing regression before implementation changes, then rerun every release gate.

- [ ] **Step 7: Prepare manual acceptance handoff**

Provide the built binary and ask the user to verify:

1. Dashboard navigation and shortcuts are unchanged.
2. Every ordinary module reloads external database changes on re-entry.
3. Filters, stable selections, viewports, details, and Trends window remain where applicable.
4. Unfinished add/edit/rename/delete-confirmation state and old status clear after leaving and re-entering.
5. History edit save and cancel return to the same filtered selected log; second edit uses refreshed data.
6. Forced target refresh failure remains on the source page with a visible error.
7. New/edit Log Entry form behavior and all Batch 1/2 layouts remain unchanged.

Do not begin Batch 3B until the user explicitly accepts Batch 3A.
