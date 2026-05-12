# Execution Plan: Fill TUI Design Pattern Gaps

## Patterns to Fix

| # | Pattern | Fix |
|---|---------|-----|
| 3 | Layered Discoverability | Add `?` help overlay per screen |
| 5 | Keybinding Consistency | Add `filter_cursor` + Emacs bindings to Trends filter |
| 7 | Color Degradation | `NO_COLOR` env var support; heatmap character fallback |
| 9 | Undo | Per-screen undo for deletes; `u` to restore |
| 10 | Destructive Tiers | Quotes confirmation; strengthen practices warning |
| 12 | Inline Error Display | Flash message system; replace 26 `let _ =` |
| 13 | Responsive Layouts | Min terminal size check (80×24) |

## Task 1: Shared Infrastructure — `src/tui/mod.rs`

- Add `pub type StatusMessage = Option<(String, bool)>;`
- Add `pub fn render_status_line(frame, area, status)` — green for success, red for error
- Add `pub fn render_help_overlay(frame, area, bindings: &[(&str, &str)])` — centered box with key→description

## Task 2: DB Restore Functions — `src/db.rs`

- `restore_log(&self, entry: &LogEntry) -> Result<i64>` — wraps `create_log_at`
- `restore_quote(&self, quote: &Quote) -> Result<Quote>` — insert with text, new position
- `restore_goal(&self, goal: &Goal) -> Result<i64>` — create goal + all milestones
- `restore_milestone(&self, goal_id, milestone: &Milestone) -> Result<i64>`

## Task 3: i18n — `locales/en.ftl`, `locales/zh-CN.ftl`

New keys: `status-deleted-undo`, `status-restored`, `status-save-error`, `status-delete-error`, `help-title`, `quotes-delete-confirm`, `practices-delete-cascade-warning`, `terminal-too-small`, `key-undo`, `key-help`

## Task 4: Per-Screen Updates

Each screen gets: `status_msg: StatusMessage`, `show_help: bool`, clear status on keypress, render status line, render help overlay on `?`, replace `let _ =`.

- **4a dashboard.rs**: `QuotesConfirmDelete` mode, `last_deleted_quote: Option<Quote>`, undo with `u`
- **4b history.rs**: `last_deleted: Option<LogEntry>`, undo with `u`, update footer hints
- **4c practices.rs**: Stronger cascade warning mentioning `[t]` toggle, no undo
- **4d goals.rs**: `last_deleted: Option<GoalUndoData>` enum, undo with `u`, standardize confirmation
- **4e log_entry.rs**: On save error stay on screen + show error, on success navigate
- **4f trends.rs**: Add `filter_cursor: usize`, full Emacs bindings, cursor-aware rendering

## Task 5: Min Terminal Size — `src/app.rs`

Check `frame.area()` in `terminal.draw()`. If < 80×24, render centered resize message.

## Task 6: NO_COLOR — `src/app.rs`, `src/tui/widgets/heatmap.rs`

Check `NO_COLOR` env var. Pass to `DashboardScreen` → `Heatmap`. Use `░▒▓█` characters when set.

## Task 7: Tests — `tests/db_test.rs`

- `test_restore_log`, `test_restore_quote`, `test_restore_goal_with_milestones`, `test_restore_milestone`

## Execution Waves

1. **Wave 1** (sequential): Tasks 1 + 2 + 3
2. **Wave 2** (parallel): Tasks 4a–4f
3. **Wave 3** (parallel): Tasks 5 + 6 + 7
4. **Final**: `cargo test` + `cargo clippy`
