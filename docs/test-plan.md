# Test Plan: TUI Design Pattern Gaps

## Automated Tests

### New DB Tests (`tests/db_test.rs`)

**test_restore_log**
1. Create a practice (Weighted)
2. Create a log with 3 sets, a note, warm-up, and cool-down via `create_log_at`
3. Get the log entry via `list_logs_all`
4. Delete the log via `delete_log`
5. Verify log is gone (list_logs_all returns empty)
6. Restore via `restore_log(&entry)`
7. List logs again — verify 1 entry exists
8. Assert: practice_id, logged_at, note, warm_up, cool_down match original
9. Assert: all 3 sets match original data (weight, reps)

**test_restore_quote**
1. Create a quote via `create_quote("test text")`
2. Delete via `delete_quote(id)`
3. Verify gone (list_quotes is empty)
4. Restore via `restore_quote(&quote)`
5. List quotes — verify 1 exists with same text

**test_restore_goal_with_milestones**
1. Create a goal via `create_goal("Test Goal")`
2. Add 2 milestones via `create_milestone`
3. Toggle one milestone completed via `toggle_milestone`
4. Get full goal via `list_goals`
5. Delete the goal via `delete_goal`
6. Verify gone (list_goals returns empty)
7. Restore via `restore_goal(&goal)`
8. List goals — verify 1 goal with 2 milestones
9. Assert: titles match, completion states match

**test_restore_milestone**
1. Create a goal + 2 milestones
2. Delete one milestone via `delete_milestone`
3. Verify goal has 1 milestone
4. Restore via `restore_milestone(goal_id, &milestone)`
5. Verify goal has 2 milestones again

### Existing Tests (must still pass)
- 6 model tests
- 10 DB CRUD tests
- 2 export tests
- 5 export/import integration tests
- 1 quote selection test
- i18n validation tests

## Manual Verification Checklist

### Pattern 12: Flash Messages
- [ ] Create a practice → no error message shown (success implied by UI update)
- [ ] Delete a log in History → green "Deleted. Press [u] to undo" message appears
- [ ] Press any key → message clears
- [ ] Force a DB error (e.g., delete practice then try to log against it) → red error message appears

### Pattern 10: Quotes Confirmation
- [ ] Open quotes manager (Q on Dashboard)
- [ ] Press `d` on a quote → red confirmation text appears ("Delete this quote? [y] Yes [any] Cancel")
- [ ] Press `n` or `Esc` → confirmation dismissed, quote still exists
- [ ] Press `d` then `y` → quote deleted, status message shown

### Pattern 9: Undo
- [ ] History: delete a log → press `u` → log restored, "Restored" message shown
- [ ] History: delete a log → navigate away → come back → `u` does nothing (undo lost)
- [ ] Dashboard quotes: delete → `u` → quote restored
- [ ] Goals: delete a goal with milestones → `u` → goal + milestones restored
- [ ] Goals: delete a milestone → `u` → milestone restored

### Pattern 3: Help Overlay
- [ ] Press `?` on Dashboard → help overlay shows all keybindings
- [ ] Press `?` or `Esc` → overlay dismissed
- [ ] Press `?` on History, Trends, Practices, Goals, LogEntry → each shows screen-specific bindings

### Pattern 5: Trends Filter
- [ ] Open Trends → press `/` → type filter text
- [ ] Ctrl+A moves cursor to start
- [ ] Ctrl+E moves cursor to end
- [ ] Ctrl+B / Left moves cursor left
- [ ] Ctrl+F / Right moves cursor right
- [ ] Ctrl+K kills to end of line
- [ ] Backspace at cursor position (not just pop from end)
- [ ] Characters insert at cursor position

### Pattern 13: Min Terminal Size
- [ ] Resize terminal below 80×24 → "Please resize" message shown
- [ ] Resize back above 80×24 → normal UI restored immediately

### Pattern 7: NO_COLOR
- [ ] Run with `NO_COLOR=1 cargo run` → heatmap uses ░▒▓█ characters
- [ ] Run normally → heatmap uses colored cells as before

## CI Verification
```bash
cargo test          # All tests pass (existing + new)
cargo clippy        # No warnings
cargo build         # Compiles clean
```
