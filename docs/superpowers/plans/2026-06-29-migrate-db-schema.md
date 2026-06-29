# DB Schema Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `iron.db` schema: rename `logs` table to `training_sessions` (and `logged_at` to `created_at`), and `sets` table to `training_sets` (and `log_id` to `training_session_id`), and update all SQL queries.

**Architecture:** We will use SQLite's `ALTER TABLE` to perform the migration in `db.rs`'s `init_schema` method, and we will update all `CREATE TABLE` and `SELECT/INSERT/UPDATE/DELETE` statements across `db.rs`.

**Tech Stack:** Rust, rusqlite

---

### Task 1: Update `init_schema` in `src/db.rs`

**Files:**
- Modify: `src/db.rs`

- [ ] **Step 1: Add migrations for old tables**

```rust
        // Add migration before new schema definition or check if old tables exist
        let check_logs: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='logs'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if check_logs > 0 {
            let _ = self.conn.execute("ALTER TABLE logs RENAME TO training_sessions", []);
            let _ = self.conn.execute("ALTER TABLE training_sessions RENAME COLUMN logged_at TO created_at", []);
        }

        let check_sets: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sets'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if check_sets > 0 {
            let _ = self.conn.execute("ALTER TABLE sets RENAME TO training_sets", []);
            let _ = self.conn.execute("ALTER TABLE training_sets RENAME COLUMN log_id TO training_session_id", []);
        }
```
*Note: Make sure this happens BEFORE the `CREATE TABLE IF NOT EXISTS` block, or inside the migrations area.*

- [ ] **Step 2: Update `CREATE TABLE` schema**

Change `logs` to `training_sessions` and `logged_at` to `created_at`.
Change `sets` to `training_sets` and `log_id` to `training_session_id`.
Make sure `REFERENCES training_sessions(id)` is used.

### Task 2: Update all SQL queries in `src/db.rs`

**Files:**
- Modify: `src/db.rs`

- [ ] **Step 1: Replace all queries**
Find and replace all instances of `logs` with `training_sessions`.
Find and replace all instances of `logged_at` with `created_at`.
Find and replace all instances of `sets` with `training_sets`.
Find and replace all instances of `log_id` with `training_session_id` in SQL strings.

Example:
```rust
            "INSERT INTO training_sessions (practice_id, created_at, note, warm_up, cool_down) VALUES (?1, ?2, ?3, ?4, ?5)",
```

- [ ] **Step 2: Verify `cargo test --test db_test` fails then passes**
Run: `cargo test --test db_test`
Fix any compilation errors and failing tests.

### Task 3: Ensure all tests pass

**Files:**
- Test: all

- [ ] **Step 1: Run all tests**
Run: `cargo test`
Ensure everything compiles and runs successfully.

