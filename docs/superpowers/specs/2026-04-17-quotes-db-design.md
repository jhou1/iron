# Quotes DB — Design Spec

**Date:** 2026-04-17
**Status:** Approved

## Overview

Move the daily motivational quote from a flat text file (`~/.ironcli/quotes.txt`) into SQLite so the database is the single artifact users need to back up. Add a dashboard overlay modal for full CRUD management.

## Data Layer

### Schema

```sql
CREATE TABLE IF NOT EXISTS quotes (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    text     TEXT NOT NULL,
    position INTEGER NOT NULL
);
```

`position` records insertion order and is used for stable list display.

### Model

Add `Quote` struct to `model.rs`:

```rust
pub struct Quote {
    pub id: i64,
    pub text: String,
    pub position: i32,
}
```

### DB Methods (`db.rs`)

| Method | Signature | Behaviour |
|---|---|---|
| `create_quote` | `(text: &str) -> Result<Quote>` | Appends at end (position = max+1) |
| `list_quotes` | `() -> Result<Vec<Quote>>` | Ordered by position ASC |
| `update_quote` | `(id: i64, text: &str) -> Result<()>` | Updates text only |
| `delete_quote` | `(id: i64) -> Result<()>` | Deletes by id |

### Migration

No migration of old data. The `quotes.txt` file path logic and `load_quotes_file` / `builtin_quotes` functions in `quotes.rs` are deleted entirely. `get_daily_quote` is replaced by a pure function `pick_daily_quote(quotes: &[Quote]) -> String` that selects by `day_of_year % quotes.len()`, returning an empty string when the slice is empty.

## UI — Dashboard Modal

### Trigger

Press `Q` from `DashboardMode::Normal` on the dashboard.

### New `DashboardMode` Variants

```rust
QuotesManage,  // browsing the list
QuotesEdit,    // text input active (add or edit)
```

`QuotesEdit` carries a flag `quotes_editing_id: Option<i64>` on `DashboardScreen` — `None` = adding new, `Some(id)` = editing existing.

### `DashboardScreen` New Fields

```rust
quotes: Vec<Quote>,
quotes_selected: usize,
quotes_input: String,
quotes_cursor: usize,
quotes_editing_id: Option<i64>,
```

### Modal Layout

Centered floating overlay rendered on top of the dashboard. Fixed width (same content width as heatmap). Height: min(quote count + 4, terminal height - 4).

```
╭─ Quotes (3) ──────────────────────────────────────────╮
│ > The only bad workout is the one that didn't happen.  │
│   Discipline is choosing between what you want now...  │
│   The pain you feel today will be the strength...      │
│                                                        │
│  a add  e edit  d delete  Esc close                   │
╰────────────────────────────────────────────────────────╯
```

In `QuotesEdit` mode, a text input field replaces the shortcuts bar:

```
╭─ Quotes (3) ──────────────────────────────────────────╮
│ > The only bad workout is the one that didn't happen.  │
│   ...                                                  │
│                                                        │
│ > [edit text here█                                   ] │
│  Enter save  Esc cancel                               │
╰────────────────────────────────────────────────────────╯
```

### Key Bindings (inside modal)

| Key | Mode | Action |
|---|---|---|
| `j` / `k` | QuotesManage | Move selection |
| `a` | QuotesManage | Open QuotesEdit (add new) |
| `e` / `Enter` | QuotesManage | Open QuotesEdit (edit selected) |
| `d` | QuotesManage | Delete selected, refresh |
| `Esc` | QuotesManage | Return to Normal mode |
| Emacs cursor keys | QuotesEdit | Navigate within input |
| `Enter` | QuotesEdit | Save (create or update), refresh |
| `Esc` | QuotesEdit | Cancel, return to QuotesManage |

### Empty State

When the `quotes` table is empty, the dashboard quote box displays:

```
"No quotes yet — press Q to add one"
```

in `DarkGray` style (no yellow, no quote marks from the pick function).

## Deleted Code

- `builtin_quotes()` function
- `load_quotes_file()` function
- `get_daily_quote()` function (replaced by `pick_daily_quote`)
- All `quotes.txt` path logic

## Error Handling

DB errors in modal key handlers follow existing convention: swallowed with `let _ =`.

## Tests

No new tests required beyond what already exists — the `pick_daily_quote` function is pure and trivial. DB CRUD for quotes follows the same pattern as goals and is covered implicitly by the existing `TestDb` infrastructure if tests are added later.
