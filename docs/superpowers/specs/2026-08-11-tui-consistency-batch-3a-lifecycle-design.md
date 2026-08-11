# TUI Consistency Framework — Batch 3A Application Lifecycle Design

**Date:** 2026-08-11

**Status:** Approved for implementation planning

**Base:** `5e2cbd46dfa1f5a96d60bbb151bdb0a27a8f6fe5` on `codex/tui-consistency`

## Summary

Batch 1 standardized the shared TUI interaction primitives and migrated the list/detail screens. Batch 2 migrated Log Entry and Dashboard, removed Quick Log and Abbreviations, and completed the visible interaction-consistency rollout.

Batch 3 separates the next structural work into two independently accepted sub-batches:

- Batch 3A standardizes application navigation and screen lifecycle.
- Batch 3B, designed only after Batch 3A is accepted, will evaluate further list, form, popup, and responsive-layout component extraction.

This document defines Batch 3A only. The goal is to remove lifecycle policy from the terminal event loop, preserve approved browsing context across navigation, refresh data atomically, and keep navigation failures on the source screen with a visible error.

## Goals

- Replace the screen-construction and refresh policy embedded in `app.rs` with a typed application shell, screen store, and router.
- Give navigation actions enough typed context to open the correct target without reading incidental state from the application loop.
- Preserve browsing context while reloading current database data when a module is re-entered.
- Clear transient editing and confirmation state when a module is left and later re-entered.
- Prepare a target successfully before changing the current route.
- Keep the source route and both source and target screen state unchanged when target preparation fails.
- Display a navigation or refresh failure in the source screen's reserved status area.
- Preserve the existing navigation topology and all accepted Batch 1 and Batch 2 behavior.

## Non-Goals

- No shared list, form, popup, or responsive-layout component extraction. That work belongs to Batch 3B.
- No new screen, navigation path, navigation stack, browser-style Back behavior, or deep-link system.
- No visible UI redesign or shortcut changes.
- No database schema or data-model change.
- No reintroduction or replacement of Quick Log, LLM configuration, or Abbreviations.
- No general asynchronous runtime or background refresh system.
- No general persistence of unfinished form edits across navigation.

## Existing Problems

The current event loop owns concrete instances of every screen and contains separate `match` statements for rendering, input dispatch, construction, refresh, and transition behavior. Lifecycle semantics differ by target:

- Dashboard refreshes an existing instance.
- Goals, Trends, Practices, and Quotes are reconstructed.
- History uses a state-preserving refresh added to prevent stale edit returns.
- Log Entry is constructed differently for new logs and History edits.
- Trends performs an extra chart refresh after every key.

This arrangement makes lifecycle behavior easy to omit or change accidentally. The stale History return found during Batch 2 final review demonstrated the risk: the data-refresh decision lived in routing code rather than in an explicit target lifecycle contract.

`Action::Navigate(Screen)` also carries too little information. Opening Log Entry from History currently depends on the application loop reaching back into `HistoryScreen` to discover the selected entry. The target request should instead identify the log being edited.

## Architecture

### AppShell

`AppShell` owns:

- the current `ScreenId`;
- a `ScreenStore`;
- the `Router`; and
- the terminal-independent event dispatch needed to process one key and one resulting action.

The outer terminal loop remains responsible for terminal setup, teardown, reading events, and drawing. It delegates rendering and key handling to `AppShell`. It no longer constructs, refreshes, or replaces individual screens.

### Navigation Types

Pure navigation types live in `tui/navigation.rs` and are re-exported through `tui` as needed.

`ScreenId` identifies the existing routes:

- Dashboard
- Goals
- Log Entry
- History
- Trends
- Practices
- Quotes

`NavigationIntent` carries the target and required context. Its semantic variants include:

- opening an ordinary module;
- opening a new Log Entry form; and
- editing a specific persisted `log_id` with History as the return route.

The final Rust representation may use nested enums when that keeps invalid combinations unrepresentable. It must not encode edit identity in an untyped string, optional tuple, or application-side read of the current History selection.

`Action` retains `None` and `Quit` and replaces the underspecified screen-only navigation request with a typed `NavigationIntent`.

### ScreenStore

`ScreenStore` holds the concrete screen types. It provides one centralized boundary for:

- rendering the active screen;
- dispatching a key to the active screen;
- preparing a target screen for entry;
- installing a prepared target after success; and
- showing a navigation error on the active source screen.

The store uses static dispatch over concrete screen types. Batch 3A does not introduce boxed screen trait objects or erase screen-specific types.

Explicit matches over `ScreenId` remain inside the store and router. Removing all matches is not a goal; centralizing their ownership and lifecycle semantics is.

### Router

The router consumes a `NavigationIntent`, the source `ScreenId`, the `ScreenStore`, and the database. It performs target preparation before committing the new route.

The router is the only layer that decides:

- whether a target uses a re-entry refresh, a new Log Entry form, or a persisted-log edit form;
- which return route a Log Entry form receives;
- when a prepared target replaces its prior cached instance; and
- when the current route changes.

The router does not render widgets, read terminal events, perform domain mutations, or own database queries that belong to a screen.

## Screen Lifecycle Contract

### Ordinary Module Re-entry

Dashboard, Goals, History, Trends, Practices, and Quotes are cached ordinary modules. Re-entering one must produce a new, fully loaded screen instance derived from:

1. current database data; and
2. the previous instance's approved browsing context.

The lifecycle operation has value semantics similar to:

```rust
fn reenter(&self, db: &Database) -> anyhow::Result<Self>;
```

The shared lifecycle contract may be expressed as a trait implemented by the concrete screen types, but `ScreenStore` calls it with static dispatch. The trait is not used as a trait object.

Each implementation must load the database-backed state before replacing the cached instance. If construction or loading fails, the existing target instance remains untouched.

### Preserved Browsing Context

When applicable to the screen, re-entry preserves:

- filter text;
- selected stable domain identity, such as practice, log, goal, or quote ID;
- selected list position as a safe fallback when the selected identity disappeared;
- list and detail viewport offsets, clamped to the refreshed content;
- read-only detail focus;
- Trends time window and selected practice identity; and
- other read-only navigation state explicitly covered by a screen contract test.

The refreshed database value wins over any cached value. Stable identity, not the previous vector index alone, is the primary selection key.

### Cleared Transient State

Re-entry clears:

- active filter-editing mode while retaining the filter value;
- add, edit, rename, and date-input modes;
- delete and quit confirmation modes;
- unsaved transient field values;
- old success and error status messages; and
- undo payloads that would otherwise cross a page departure and operate on stale assumptions.

These rules apply when a module is left and later entered again. They do not change in-place failure behavior: a validation or database mutation failure on the current screen continues to preserve its active form, focus, cursor, identity, and undo payload according to the existing screen contract.

### Log Entry Lifecycle

Log Entry is request-scoped rather than an ordinary cached re-entry target.

- Dashboard creates a new empty form.
- History emits an edit intent containing the selected persisted `log_id`.
- The router loads that ID and constructs an edit form with History as its return route.
- Cancel discards the request-scoped form and returns to its configured route.
- Successful save returns to its configured route.
- Validation and database failures keep the current form in place and do not invoke router preparation.

The application loop does not read `HistoryScreen::selected_entry` to prepare an edit.

## Navigation Topology

Batch 3A preserves the existing user-visible topology:

- Dashboard is the application hub.
- Goals, History, Trends, Practices, and Quotes return to Dashboard through `Esc`.
- New Log Entry returns to Dashboard.
- A Log Entry edit opened from History returns to History.
- No general navigation stack or new cross-module link is introduced.

## Transactional Transition Flow

A transition follows this sequence:

1. The active screen handles a key and returns an `Action`.
2. `AppShell` forwards a navigation action to the router.
3. The router prepares the target without changing `current_screen`.
4. An ordinary module returns a newly loaded instance with allowed browsing context transferred from its cached instance. Log Entry returns a newly constructed request-scoped form.
5. After successful preparation, `ScreenStore` installs the prepared target.
6. Only then does `AppShell` commit the target `ScreenId`.

No fallible operation occurs after committing the route.

## Failure Semantics

If target preparation fails:

- `current_screen` remains the source route;
- the source screen's browsing and transient state remain unchanged;
- the cached target instance remains unchanged;
- no partially loaded target is rendered; and
- the source screen displays a localized navigation or refresh error in its reserved status row.

Every ordinary source screen therefore provides a narrow way for `ScreenStore` to set a navigation error. The store owns the type-specific dispatch; individual screens continue to own their status storage and rendering.

If the application cannot display a source error because initialization failed before any source exists, startup returns the error through the existing terminal setup path.

## Per-Screen Requirements

### Dashboard

- Reload heatmap, summary, recent logs, active goals, quote, HRV, streak, frequency, and volume atomically.
- Clear HRV input and quit confirmation modes on re-entry.
- Preserve the configured color mode.

### Goals

- Reload goals and milestones.
- Restore the selected stable goal and milestone identities where they still exist.
- Clamp both list and focused-panel viewports.
- Exit add/edit/delete confirmation modes and clear transient inputs and statuses.

### History

- Reuse and generalize the current stable-log refresh behavior.
- Preserve filter value, stable selected log, list/detail viewport, and read-only detail focus.
- Exit filter editing and delete confirmation; clear status and undo payload on re-entry.
- The in-place delete/undo reload path may preserve its current local undo behavior because the screen was not left.

### Trends

- Reload the active-practice list and chart data together.
- Preserve selected practice by ID, filter value, list viewport, and selected time window.
- Exit filter editing and clear status.
- Remove the event-loop-specific chart refresh side effect; chart refresh belongs to Trends handling or lifecycle.

### Practices

- Reload all practices.
- Preserve selected practice identity and viewport.
- Exit add, rename, and type-selection modes; clear transient input and status.

### Quotes

- Reload quotes.
- Preserve selected quote identity, list/detail viewport, and read-only detail focus.
- Exit add/edit/delete confirmation modes and clear transient input, status, and undo payload.

### Log Entry

- Use typed new/edit intents.
- Load edit state by persisted log ID.
- Preserve existing validation, database-failure, cursor, focus, set-edit, and return-route behavior while the form remains active.

## Testing Strategy

### Navigation Type Tests

- New-log and edit-log intents carry only valid target context.
- Edit intent contains a persisted log ID and an explicit return route.
- Screen-only navigation cannot accidentally represent an edit request.

### Lifecycle Contract Tests

Each ordinary screen receives focused tests for the state it owns:

- changed database data is visible after re-entry;
- selection restores by stable ID rather than stale vector index;
- missing identities clamp to a valid fallback;
- filter values and approved viewports remain;
- transient editing and confirmation modes clear;
- old statuses and cross-page undo payloads clear; and
- failed preparation leaves the cached instance unchanged.

### Router Tests

Router tests run without a real terminal and cover:

- target success commits the prepared instance and route;
- target failure preserves source route, source context, and cached target;
- target failure sets a source-screen error;
- ordinary re-entry uses the lifecycle contract;
- Dashboard opens a new Log Entry form;
- History emits an edit intent with the stable log ID;
- edit construction failure does not leave History; and
- successful and cancelled Log Entry returns refresh History without losing its approved browsing context.

### Workflow Tests

- Dashboard to each module, back to Dashboard, and re-entry after database data changes.
- History to Log Entry edit, successful return, and a second edit of the same stable log ID.
- History to Log Entry edit and cancel return.
- Re-enter a filtered list after its selected item was changed or removed.
- Refresh failure while navigating from at least two different source screen types.

### Release Gates

Batch 3A must keep all existing layout, small-rectangle, failure-retention, removal, localization, and TUI lint regressions green and pass:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
git diff --check
```

The Quick Log/LLM/Abbreviations removal audit and duplicate input/scroll-state audit remain release gates.

## Documentation

Implementation updates:

- `CLAUDE.md` architecture and lifecycle rules;
- `docs/specs/navigation-spec.md` typed intent, re-entry, and failure behavior;
- screen specifications where their lifecycle contract becomes explicit; and
- the broader framework design to mark Batch 3A complete after implementation acceptance.

Documentation must describe shipped behavior only. Batch 3B remains a future design topic until separately approved.

## Delivery Sequence

1. Add typed navigation types while preserving current behavior.
2. Add terminal-independent `AppShell`, `ScreenStore`, and Router characterization tests.
3. Move render and key dispatch behind `ScreenStore`.
4. Implement ordinary-screen lifecycle contracts one screen at a time, with focused review after each group.
5. Convert Log Entry new/edit routing to typed intents.
6. Make navigation transitions transactional and expose source-screen refresh errors.
7. Remove superseded construction and refresh policy from the terminal loop.
8. Update active documentation and run final release gates and whole-branch review.
9. Deliver Batch 3A for manual acceptance.

No Batch 3B implementation begins before explicit Batch 3A acceptance.

## Manual Acceptance

The user verifies:

1. Dashboard navigation targets remain unchanged.
2. Leaving and re-entering each ordinary module shows current database data while preserving filter, selected item, and viewport where applicable.
3. Re-entry clears unfinished add/edit/rename/delete-confirmation state and old status messages.
4. History edit save and cancel both return to the same filtered/selected log context; a second edit opens the refreshed log.
5. A refresh failure leaves the user on the source screen with a visible error.
6. New and edited Log Entry forms retain their existing success, cancel, validation, and database-failure behavior.
7. No new navigation path, shortcut, Quick Log, LLM, or Abbreviations interface appears.

