# Batch 3B-5.1 Residual Corrections Design

## Context

Batch 3B-5 shipped the responsive Goals and Milestones presentation and then closed its release-blocking final-review findings. The single scoped re-review of that final-fix wave found no Critical or Important defects, but it identified three concrete residual Minors:

1. cancelling a milestone mutation can leave a stale status visible;
2. the Goals ownership AST lint does not model immediately invoked closures or canonical framework module aliases/glob imports correctly;
3. the English and Chinese READMEs omit Goal delete-confirmation cancellation from the documented focus branch.

Batch 3B-5.1 closes only these residuals. It does not redesign Goals again.

## Goals

- Preserve persistent Milestones browse context across focus changes.
- Clear stale mutation status when cancelling milestone add, edit, date-edit, or delete-confirmation modes.
- Treat immediately invoked closures as executable ownership code while continuing to ignore stored, uninvoked closures.
- Resolve canonical shared-framework module aliases and glob imports with lexical scoping.
- Keep wrong-origin calls and shadowed names from satisfying ownership.
- Correct the English and Chinese README descriptions of Goal delete-confirmation cancellation.

## Non-goals

- No database, model, router, application-shell, shortcut, or navigation-topology changes.
- No public component API changes.
- No dependency changes.
- No full call-graph or general Rust control-flow analyzer.
- No source-text fallback lint.
- No fixture regeneration or accepted visual-output changes.
- No changes to Practices, Quotes, Trends, History, Dashboard, or Log Entry behavior.

## Architecture

### Milestone cancellation ownership

Persistent state and transient mutation state have different lifetimes.

`Milestones Browse -> Esc` is a focus transition. It returns to Goal Browse while retaining milestone selection, viewport, status, and undo so that `Enter` on the same Goal restores the same pane context.

Cancellation inside `AddMilestone`, `EditMilestone`, `EditMilestoneDate`, or `ConfirmDeleteMilestone` is a mutation-state transition. It returns to Milestones Browse and clears the stale mutation status. Text-entry modes also clear their input exactly as they do today. Successful and failed database mutation paths remain unchanged.

The implementation will express the distinction in the mode-specific cancel paths instead of a broad pre-dispatch exception in `handle_milestone`.

### Execution-aware ownership visitor

The existing `syn` visitor remains the ownership lint engine. It will be extended, not replaced.

An immediately invoked closure is executable code. Calls inside it:

- may satisfy the exactly-once shared-component ownership requirements;
- are inspected for forbidden direct borders, `Clear`, footers, and raw `Paragraph` ownership.

A closure that is created, assigned, passed, or returned without being invoked at that expression remains non-executable for this lint. Its body neither satisfies required ownership nor reports forbidden rendering.

The visitor will recognize the direct IIFE shapes representable in the current AST, including a parenthesized closure used as the callee of `ExprCall`. This is deliberately not a general data-flow analysis: invoking a closure through a previously assigned variable is outside this batch.

### Lexical framework import resolution

The existing lexical scope stack remains authoritative. It will additionally model:

- canonical module aliases, such as `use super::framework as shared; shared::render_screen_frame(...)`;
- canonical framework glob imports, such as `use super::framework::*; render_screen_frame(...)`;
- nested shadowing and wrong-origin imports without leaking bindings across scopes.

Explicit local bindings and nearer lexical imports take precedence over outer module aliases or glob availability. An unrelated glob never proves canonical origin. Canonical component calls must still resolve to `super::framework` or `crate|iron::tui::framework`.

### Documentation

Both READMEs will state the same four-stage cancellation contract already recorded in the authoritative Goals specification:

- Goal add/edit/date/delete-confirmation cancellation returns to Goal Browse;
- milestone add/edit/date/delete-confirmation cancellation returns to Milestones Browse;
- Milestones Browse `Esc` returns to Goal Browse;
- Goal Browse `Esc` returns to Dashboard.

No route or shortcut change will be claimed.

## Testing strategy

All production and lint changes follow strict RED-GREEN-REFACTOR.

### Goals behavior regression

A focused Goals test will seed a stale milestone error/status in each mutation-cancel family, cancel the mode through the real key handler, and assert that the screen returns to Milestones Browse with no stale status. A companion assertion will prove that `Esc` from Milestones Browse still preserves status and persistent context.

The break this test catches is broad status preservation being applied to mutation cancellation instead of only to browse-focus transitions.

### AST lint regressions

Synthetic, parsed Rust fixtures will independently prove:

- a direct IIFE can satisfy each required shared-component call;
- forbidden rendering inside a direct IIFE is rejected;
- an uninvoked closure cannot satisfy required ownership and does not create a forbidden-render false positive;
- a canonical framework module alias resolves required calls;
- a canonical framework glob resolves required calls;
- a nested shadow, wrong-origin alias, or unrelated glob cannot satisfy required ownership;
- existing reachable-helper, UFCS, `Clear`, cfg, and recursive Paragraph protections remain green.

Expected counts and violations will be literal and hand-derived. Tests will exercise the real parser and visitor without mocks.

### Documentation and release gates

Human-facing prose will be audited for English/Chinese agreement; no source-text change-detector test will be added.

Final verification includes:

- `cargo fmt -- --check`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo test --all-targets --all-features`;
- `cargo build --all-features`;
- focused Goals, TUI lint, i18n, and removed-feature suites;
- unchanged frozen Goals and non-Goals fixtures;
- clean range diff and worktree status;
- an executable `target/debug/iron` binary.

## Files and boundaries

Expected tracked changes are limited to:

- `src/tui/goals.rs`;
- `tests/tui_lint_test.rs`;
- `README.md`;
- `README_zh.md`;
- this design document;
- the subsequent implementation plan.

If implementation requires another production file, public API, dependency, or fixture change, work stops for a scope ruling.

## Acceptance criteria

- Milestones Browse focus round trips retain selection, viewport, status, and undo.
- Cancelling any milestone mutation mode removes stale status and returns to Milestones Browse.
- Direct IIFEs participate in ownership and forbidden-render analysis.
- Uninvoked closures remain excluded.
- Canonical framework module aliases and globs work under lexical scope.
- Wrong-origin and shadowed calls remain rejected.
- Both READMEs explicitly include Goal delete-confirmation cancellation.
- All release gates pass with no fixture changes and no new dependency.
