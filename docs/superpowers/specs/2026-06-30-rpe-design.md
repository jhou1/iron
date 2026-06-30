# RPE Session Tracking Design

## 1. Overview
The goal is to allow users to optionally track their Rate of Perceived Exertion (RPE) on a scale of 1-10 for a full training session. This provides insight into session intensity.

## 2. Architecture & Database
- **Migration:** Add `ALTER TABLE training_sessions ADD COLUMN rpe INTEGER;` to the `migrate` function in `src/db.rs`.
- **Schema:** The `rpe` column will be `INTEGER` (nullable).
- **Queries:** Update the `INSERT`, `UPDATE`, and `SELECT` statements for the `training_sessions` table in `src/db.rs` to map the new `rpe` parameter.

## 3. Models & Export
- **Core Models:** Update `Log` and `LogEntry` in `src/model.rs` to include `pub rpe: Option<u8>`.
- **Serialization:** Update `ExportTrainingSession` in `src/export.rs` to include `rpe: Option<u8>` so that JSON imports and exports preserve this data.

## 4. User Interface
- **Log Entry Screen (`src/tui/log_entry.rs`):**
  - Add an `RPE (1-10)` text input to the form.
  - Form validation: Allow empty input. If populated, validate it parses as an integer between 1 and 10 inclusive.
  - Implement the mandatory emacs-style cursor bindings for this new text input field, consistent with `CLAUDE.md` guidelines.
- **History Screen (`src/tui/history.rs`):**
  - Append an `[RPE: X]` visual badge to the session display line for logs that contain an RPE value.
  - Hide the badge entirely if the `rpe` is `None`.

## 5. Testing
- Update unit tests in `tests/model_test.rs` and `tests/db_test.rs` to account for the new field in mock data.
- Ensure import/export integration tests pass with the new field.
