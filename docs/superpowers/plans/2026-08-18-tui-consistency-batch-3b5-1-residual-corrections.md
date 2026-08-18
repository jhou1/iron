# Batch 3B-5.1 Residual Corrections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the three residual Batch 3B-5 review findings without changing accepted rendering, navigation topology, dependencies, or public component APIs.

**Architecture:** Keep the existing Goals state machine and `syn` ownership visitor. Narrow status preservation to the Milestones Browse focus transition, add direct-IIFE execution semantics to the visitor, and extend its lexical import resolver with canonical framework module aliases and globs. Correct the two README summaries after code behavior is green.

**Tech Stack:** Rust 2021, Ratatui, Crossterm, `syn` test AST parsing, Cargo test/clippy/fmt/build, Git.

## Global Constraints

- Work only in `/Users/hjw/Projects/iron/.worktrees/goals-responsive-panels` on `codex/goals-responsive-panels`.
- Authorized implementation base is `7457e3c` unless this plan commit advances HEAD before execution.
- Follow strict RED-GREEN-REFACTOR; record the observed RED before changing production or lint implementation.
- Do not add dependencies or modify `Cargo.toml`/`Cargo.lock`.
- Do not change database, model, router, app-shell, shortcut, or navigation-topology behavior.
- Do not change public shared-component APIs.
- Do not regenerate or modify frozen Goals, Practices, Quotes, Trends, or History fixtures.
- A direct immediately invoked closure participates in required ownership counting and forbidden-render inspection; a stored or otherwise uninvoked closure does neither.
- Milestones Browse `Esc` preserves selection, viewport, status, and undo; cancelling milestone add/edit/date/delete-confirmation clears stale status and returns to Milestones Browse.
- If another production file, public API, dependency, or fixture must change, stop for a scope ruling.

---

### Task 1: Scope milestone status preservation to browse focus transitions

**Files:**
- Modify: `src/tui/goals.rs:817-833`
- Test: `src/tui/goals.rs` test module near the existing milestone focus-round-trip tests

**Interfaces:**
- Consumes: `GoalsScreen::handle_key`, `GoalsScreen::handle_milestone`, `Mode`, `MilestoneMode`, `StatusMessage`.
- Produces: unchanged key-handler signatures; corrected mode-specific status lifetime.

- [ ] **Step 1: Add the failing cancellation regression**

Add a focused table-driven test in the existing `goals.rs` test module. It must exercise the real key handler and independently assert both mutation cancellation and browse-focus preservation:

```rust
#[test]
fn milestone_cancel_clears_mutation_status_but_browse_back_preserves_context() {
    let db = Database::open_in_memory().unwrap();
    db.create_goal("Goal").unwrap();

    for milestone_mode in [
        MilestoneMode::AddMilestone,
        MilestoneMode::EditMilestone,
        MilestoneMode::EditMilestoneDate,
        MilestoneMode::ConfirmDeleteMilestone,
    ] {
        let mut screen = GoalsScreen::new(&db).unwrap();
        screen.mode = Mode::Milestones;
        screen.milestone_mode = milestone_mode;
        screen.milestone_status_msg = Some(("stale failure".into(), true));
        screen.milestone_input.set("pending input");

        assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
        assert_eq!(screen.mode, Mode::Milestones);
        assert_eq!(screen.milestone_mode, MilestoneMode::Browse);
        assert!(screen.milestone_status_msg.is_none(), "{milestone_mode:?}");
    }

    let mut screen = GoalsScreen::new(&db).unwrap();
    screen.handle_key(key(KeyCode::Enter), &db);
    screen.milestone_status_msg = Some(("keep browse status".into(), false));
    screen.milestone_panel_state.viewport.set_offset(3);

    assert_eq!(screen.handle_key(key(KeyCode::Esc), &db), Action::None);
    assert_eq!(screen.mode, Mode::Browse);
    assert_eq!(
        screen.milestone_status_msg,
        Some(("keep browse status".into(), false))
    );
    assert_eq!(screen.milestone_panel_state.viewport.offset(), 3);
}
```

If the empty goal has no milestone and an edit-mode branch requires a selected milestone, seed one milestone in the fixture. Do not bypass `handle_key` by invoking private mode handlers directly.

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test tui::goals::tests::milestone_cancel_clears_mutation_status_but_browse_back_preserves_context -- --exact --nocapture
```

Expected: the test fails because `Esc` from at least one mutation mode leaves `Some(("stale failure", true))` instead of `None`. Confirm it fails in both library and binary unit-test targets for the intended assertion, not fixture setup.

- [ ] **Step 3: Make the minimal status-lifetime correction**

Replace the broad `Esc` exemption at the top of `handle_milestone` with a mode-aware condition:

```rust
let preserves_browse_context =
    key.code == KeyCode::Esc && self.milestone_mode == MilestoneMode::Browse;
if !preserves_browse_context {
    self.milestone_status_msg = None;
}
```

Do not change mutation success/failure branches, undo payloads, selection reconciliation, or input clearing.

- [ ] **Step 4: Verify GREEN and surrounding Goals behavior**

Run:

```bash
cargo test tui::goals::tests::milestone_cancel_clears_mutation_status_but_browse_back_preserves_context -- --exact --nocapture
cargo test tui::goals::tests -- --nocapture
```

Expected: the focused test passes in both targets; the full Goals suite remains green. Deliberately mutate the condition to preserve every `Esc` and confirm the focused regression fails, then restore the correct implementation.

- [ ] **Step 5: Commit Task 1**

```bash
git add src/tui/goals.rs
git commit -m "fix(goals): clear cancelled milestone status"
```

---

### Task 2: Model executable closures and canonical framework imports in the ownership lint

**Files:**
- Modify: `tests/tui_lint_test.rs:246-490`
- Modify: `tests/tui_lint_test.rs:742-1010`
- Test: `tests/tui_lint_test.rs` Goals ownership synthetic-test section

**Interfaces:**
- Consumes: `goals_ownership_violations(&str) -> Vec<String>`, `GoalsRenderVisitor`, `GoalsCallResolution`, `syn::Visit`.
- Produces: execution-aware direct-IIFE traversal and lexically resolved canonical framework module aliases/globs; no production API.

- [ ] **Step 1: Add failing direct-IIFE tests**

Add two synthetic tests using literal Rust source. The first must place all three required calls inside a direct IIFE and expect no violations; the second must keep valid required calls and place forbidden rendering inside a direct IIFE:

```rust
#[test]
fn goals_ownership_executes_immediately_invoked_closures() {
    let source = r#"
        use super::framework::{
            render_detail_panel_with_header, render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                (|| {
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_detail_panel_with_header();
                })();
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_checks_forbidden_rendering_inside_an_iife() {
    let source = r#"
        use super::framework::{
            render_detail_panel_with_header, render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                (|| { Paragraph::new("forbidden"); })();
            }
        }
    "#;

    let violations = goals_ownership_violations(source);
    assert!(violations.iter().any(|item| item.contains("Paragraph::new")));
}
```

Keep `goals_ownership_does_not_accept_delegation_inside_an_uninvoked_closure` unchanged as the negative control.

- [ ] **Step 2: Add failing module-alias and glob tests**

Add a table-driven positive test with the exact two canonical forms:

```rust
#[test]
fn goals_ownership_accepts_canonical_framework_module_aliases_and_globs() {
    let cases = [
        r#"
            use super::framework as shared;
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    shared::render_screen_frame();
                    shared::render_scrollable_list_panel_spanned();
                    shared::render_detail_panel_with_header();
                }
            }
        "#,
        r#"
            use super::framework::*;
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_detail_panel_with_header();
                }
            }
        "#,
    ];

    for source in cases {
        assert!(goals_ownership_violations(source).is_empty());
    }
}
```

Add negative controls showing that `use domain::framework as shared`, an inner `let shared = ...` shadow, and `use domain::*` cannot satisfy any required call.

- [ ] **Step 3: Run the new tests and verify RED**

Run each new exact test with:

```bash
cargo test --test tui_lint_test goals_ownership_executes_immediately_invoked_closures -- --exact --nocapture
cargo test --test tui_lint_test goals_ownership_checks_forbidden_rendering_inside_an_iife -- --exact --nocapture
cargo test --test tui_lint_test goals_ownership_accepts_canonical_framework_module_aliases_and_globs -- --exact --nocapture
```

Expected: IIFE ownership reports required calls `found 0`; forbidden IIFE rendering is missed; module alias and glob cases report required calls `found 0`. Negative controls must fail only if a wrong origin is incorrectly accepted.

- [ ] **Step 4: Introduce a lexical call-scope representation**

Replace the bare call-scope map with a small internal test-only scope type:

```rust
#[derive(Default)]
struct GoalsCallScope {
    bindings: HashMap<String, GoalsCallResolution>,
    canonical_framework_glob: bool,
}

enum GoalsCallResolution {
    Framework(Vec<String>),
    FrameworkModule(Vec<String>),
    Shadowed,
}
```

Add an exact module-origin predicate:

```rust
fn canonical_goals_framework_module(path: &[String]) -> bool {
    matches!(path, [root, framework] if root == "super" && framework == "framework")
        || matches!(path, [root, tui, framework]
            if matches!(root.as_str(), "crate" | "iron")
                && tui == "tui" && framework == "framework")
}
```

Update `collect_goals_call_imports` so component imports remain `Framework`, canonical module renames become `FrameworkModule`, and a glob sets `canonical_framework_glob` only when its accumulated prefix is a canonical framework module. Wrong-origin imports become `Shadowed` and remain lexically scoped.

- [ ] **Step 5: Resolve one- and two-segment calls through lexical scopes**

Update `canonical_goals_framework_call` to accept `&[GoalsCallScope]` and apply these precedence rules from inner to outer scope:

```rust
// one segment: explicit binding/shadow wins; otherwise a canonical glob
// can provide only one of the three known component names.
// two segments: the first segment must resolve to FrameworkModule, then
// appending the second segment must produce a canonical component path.
// longer paths continue through canonical_goals_framework_path directly.
```

A nearer `Shadowed` binding must return `None` immediately. An unrelated glob must neither prove origin nor hide a valid explicit outer import.

- [ ] **Step 6: Traverse only direct immediately invoked closures**

Add a helper that unwraps group/paren expressions and returns a closure only when it is the direct callee:

```rust
fn direct_invoked_closure(expression: &syn::Expr) -> Option<&syn::ExprClosure> {
    match expression {
        syn::Expr::Closure(closure) => Some(closure),
        syn::Expr::Group(group) => direct_invoked_closure(&group.expr),
        syn::Expr::Paren(paren) => direct_invoked_closure(&paren.expr),
        _ => None,
    }
}
```

In `visit_expr_call`, if `direct_invoked_closure(&call.func)` succeeds, push fresh lexical scopes, shadow closure input patterns, visit the closure body exactly once, pop the scopes, then visit call arguments. Keep `visit_expr_closure` empty so stored/uninvoked closures remain excluded. Do not attempt variable-based closure data flow.

- [ ] **Step 7: Verify GREEN, precision, and mutation sensitivity**

Run:

```bash
cargo test --test tui_lint_test goals_ownership_ -- --nocapture
cargo test --test tui_lint_test paragraph_widgets_must_wrap_user_content -- --exact --nocapture
cargo test --test tui_lint_test -- --nocapture
```

Expected: all Goals ownership cases, the recursive Paragraph lint, and the complete lint target pass. Deliberately disable direct-IIFE traversal and confirm the IIFE test fails; deliberately disable module-alias/glob resolution and confirm the corresponding table rows fail; restore the implementation.

- [ ] **Step 8: Commit Task 2**

```bash
git add tests/tui_lint_test.rs
git commit -m "test(tui): complete Goals ownership reachability"
```

---

### Task 3: Correct README cancellation copy and run release gates

**Files:**
- Modify: `README.md:169`
- Modify: `README_zh.md:169`
- Verify only: all accepted fixture directories and release gates

**Interfaces:**
- Consumes: the authoritative cancellation branches in `docs/specs/goals-spec.md`.
- Produces: English and Chinese summaries that match runtime behavior; no runtime interface.

- [ ] **Step 1: Correct the English and Chinese summaries**

Use these exact semantic corrections while preserving surrounding prose:

```markdown
`Esc` cancels Goal add/edit/date/delete-confirmation modes directly to Goal Browse, while milestone add/edit/date/delete-confirmation modes cancel to Milestones Browse.
```

```markdown
`Esc` 会把目标添加、编辑、日期编辑和删除确认直接取消回目标浏览；里程碑添加、编辑、日期编辑和删除确认则取消回里程碑浏览。
```

Do not claim any route or shortcut change. Do not add a source-text test for human prose.

- [ ] **Step 2: Audit documentation agreement**

Read the relevant Goals paragraphs in both READMEs, `docs/specs/goals-spec.md`, `docs/specs/navigation-spec.md`, and the active framework design. Confirm the four branches agree:

1. Goal mutation cancel -> Goal Browse.
2. Milestone mutation cancel -> Milestones Browse.
3. Milestones Browse `Esc` -> Goal Browse.
4. Goal Browse `Esc` -> Dashboard.

- [ ] **Step 3: Commit Task 3**

```bash
git add README.md README_zh.md
git commit -m "docs(tui): complete Goals cancellation branches"
```

- [ ] **Step 4: Run fresh focused and full release gates**

Run from the isolated worktree after all code and docs changes:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test tui::goals::tests -- --nocapture
cargo test --test tui_lint_test -- --nocapture
cargo test --test i18n_test -- --nocapture
cargo test --test removed_quick_log_test -- --nocapture
cargo test --all-targets --all-features
cargo build --all-features
git diff --check 70dae36..HEAD
```

Expected: every command exits 0 with zero Clippy warnings.

- [ ] **Step 5: Verify fixture and scope immutability**

Confirm that no fixture file is changed:

```bash
git diff --name-only 6f30a12..HEAD -- tests/fixtures/goals
git diff --name-only 70dae36..HEAD -- tests/fixtures
git diff --name-only 7457e3c..HEAD
git status --short --untracked-files=all
test -x target/debug/iron
```

Expected: both fixture commands print nothing; the range from `7457e3c` contains only this implementation plan plus `src/tui/goals.rs`, `tests/tui_lint_test.rs`, `README.md`, and `README_zh.md`; status is clean; the binary check exits 0.

- [ ] **Step 6: Record final evidence**

Write the plan-owned SDD report with exact RED/GREEN outputs, commit hashes, test counts, fixture audits, scope audit, concerns, and binary path. Do not commit the ignored SDD report.
