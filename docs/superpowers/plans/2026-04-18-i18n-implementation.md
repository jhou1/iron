# i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add English + Chinese (Simplified) i18n to ironcli using Project Fluent, auto-detecting locale from system environment variables.

**Architecture:** A new `src/i18n.rs` module wraps `fluent-bundle` behind `tr()`/`tr_args()` functions, backed by `.ftl` files embedded at compile time. All TUI screens and CLI output replace inline string literals with these function calls. Locale is detected once at startup from `LANG`/`LC_ALL`.

**Tech Stack:** `fluent-bundle 0.15`, `fluent-syntax 0.11` (for completeness test), `unic-langid 0.9`

---

### Task 1: Add Fluent dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add fluent-bundle and unic-langid to dependencies**

In `Cargo.toml`, add to `[dependencies]`:

```toml
fluent-bundle = "0.15"
unic-langid = "0.9"
```

And add to `[dev-dependencies]`:

```toml
fluent-syntax = "0.11"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add fluent-bundle and unic-langid dependencies for i18n"
```

---

### Task 2: Create English translation file

**Files:**
- Create: `locales/en.ftl`

This file must contain every user-facing string in the app. The keys below are extracted from reading every TUI screen file, `main.rs`, `export.rs`, and `model.rs`. Organize by screen/area.

- [ ] **Step 1: Create `locales/en.ftl`**

```fluent
# ── Common ──
app-name = IronCLI
app-about = Track your training

# ── Dashboard ──
dashboard-title = iron
dashboard-last-14-days = Last 14 Days
dashboard-goals = Goals
dashboard-quotes = Quotes
dashboard-no-quotes = No quotes yet — press Q to add one
dashboard-no-entries = No entries in the last 14 days
dashboard-sessions = { $count } sessions
dashboard-total-volume = { $value } kg
dashboard-total-reps = { $value } reps
dashboard-total-distance = { $value } km
dashboard-total-duration = { $value } min
dashboard-sets-metric = { $sets } sets, { $total } { $label }
dashboard-press-g = Press [g] to add goals
dashboard-press-a-goal = Press [a] to add a goal
dashboard-date-prompt = Date (YYYY-MM-DD): 
dashboard-delete-confirm = Delete? (y/n)
dashboard-quotes-count = Quotes ({ $count }) 
dashboard-no-quotes-modal = No quotes — press [a] to add one

# ── Log Entry ──
log-select-practice = Select Practice
log-press-filter = Press / to filter
log-weight-label = Weight (kg): 
log-reps-label = Reps: 
log-distance-label = Distance (km): 
log-duration-label = Duration (min): 
log-note-label = Note: 
log-date-label = Date: 
log-date-confirm-hint = [Enter] confirm  [D] edit
log-date-change-hint = [D] to change
log-date-edit-hint = (YYYY-MM-DD, Enter to confirm)
log-set-line = Set { $number }: { $data }
log-sets-total = Sets: { $sets }  Total: { $total } { $label }
log-sets-total-reps = Sets: { $sets }  Total: { $total } { $label }  Reps: { $reps }
log-add-note-title = Log { $name } — Add Note
log-sets-logged = { $count } sets logged
log-total-value = Total: { $total } { $label }
log-note-optional = Note (optional)

# ── History ──
history-title = History
history-no-entries = No entries yet
history-entry = { $date }  { $name }  { $sets } sets  { $total } { $label }
history-set-weighted = #{ $number }  { $weight }kg x { $reps }
history-set-bodyweight = #{ $number }  { $reps } reps
history-set-distance = #{ $number }  { $distance } km
history-set-endurance = #{ $number }  { $duration } min
history-note = Note: { $note }
history-delete-confirm = Delete this entry? 

# ── Trends ──
trends-title = Trends — Select Practice
trends-last-days = Last { $days } days
trends-no-data = No data for this period.
trends-avg = Avg: { $value }
trends-peak = Peak: { $value }
trends-trend = Trend: { $sign }{ $value }%

# ── Practices ──
practices-title = Practices
practices-no-items = No practices yet. Press 'a' to add one.
practices-new-name = New practice name:
practices-select-type = Select type:
practices-rename = Rename practice:
practices-delete-confirm = Delete { $name }?
practices-delete-warning = This removes all its logs.

# ── Practice type labels ──
practice-type-weighted = weightxreps
practice-type-bodyweight = reps
practice-type-distance = distance
practice-type-endurance = duration

# ── Metric labels ──
metric-kg-vol = kg vol
metric-reps = reps
metric-km = km
metric-min = min

# ── Set data formatting ──
set-weighted = { $weight } kg x { $reps } reps
set-bodyweight = { $reps } reps
set-distance = { $distance } km
set-endurance = { $duration } min

# ── Keyboard labels ──
key-log = Log
key-history = History
key-trends = Trends
key-practices = Practices
key-goals = Goals
key-quotes = Quotes
key-quit = Quit
key-navigate = Navigate
key-filter = Filter
key-select = Select
key-back = Back
key-add = Add
key-edit = Edit
key-delete = Delete
key-confirm = Confirm
key-cancel = Cancel
key-add-set = Add set
key-save = Save
key-date = Date
key-del-last = Del last
key-add-goal = Add goal
key-milestone = Milestone
key-toggle = Toggle
key-close = Close
key-window = Window
key-pick-practice = Pick practice
key-dashboard = Dashboard
key-yes = Yes
key-no = No

# ── CLI ──
cli-export-about = Export all data to JSON
cli-import-about = Import data from JSON
cli-export-path-help = Output file path (defaults to ~/.ironcli/iron-export-YYYY-MM-DD.json)
cli-import-path-help = Input file path
cli-export-complete = Export complete.
cli-imported = Imported { $count } logs.
cli-exported-to = Exported to { $path }
```

- [ ] **Step 2: Commit**

```bash
git add locales/en.ftl
git commit -m "feat(i18n): add English translation file"
```

---

### Task 3: Create Chinese translation file

**Files:**
- Create: `locales/zh-CN.ftl`

Must mirror every key from `en.ftl` exactly. All units translated (`kg` → `公斤`, etc.).

- [ ] **Step 1: Create `locales/zh-CN.ftl`**

```fluent
# ── 通用 ──
app-name = IronCLI
app-about = 记录你的训练

# ── 仪表盘 ──
dashboard-title = iron
dashboard-last-14-days = 最近 14 天
dashboard-goals = 目标
dashboard-quotes = 语录
dashboard-no-quotes = 还没有语录 - 按 Q 添加
dashboard-no-entries = 最近 14 天没有记录
dashboard-sessions = { $count } 次训练
dashboard-total-volume = { $value } 公斤
dashboard-total-reps = { $value } 次
dashboard-total-distance = { $value } 公里
dashboard-total-duration = { $value } 分钟
dashboard-sets-metric = { $sets } 组, { $total } { $label }
dashboard-press-g = 按 [g] 添加目标
dashboard-press-a-goal = 按 [a] 添加目标
dashboard-date-prompt = 日期 (YYYY-MM-DD)：
dashboard-delete-confirm = 删除？(y/n)
dashboard-quotes-count = 语录 ({ $count }) 
dashboard-no-quotes-modal = 没有语录 - 按 [a] 添加

# ── 记录条目 ──
log-select-practice = 选择练习项目
log-press-filter = 按 / 过滤
log-weight-label = 重量（公斤）：
log-reps-label = 次数：
log-distance-label = 距离（公里）：
log-duration-label = 时长（分钟）：
log-note-label = 备注：
log-date-label = 日期：
log-date-confirm-hint = [Enter] 确认  [D] 编辑
log-date-change-hint = [D] 修改
log-date-edit-hint = （YYYY-MM-DD，按 Enter 确认）
log-set-line = 第 { $number } 组：{ $data }
log-sets-total = 组数：{ $sets }  合计：{ $total } { $label }
log-sets-total-reps = 组数：{ $sets }  合计：{ $total } { $label }  次数：{ $reps }
log-add-note-title = 记录 { $name } — 添加备注
log-sets-logged = 已记录 { $count } 组
log-total-value = 合计：{ $total } { $label }
log-note-optional = 备注（可选）

# ── 历史 ──
history-title = 历史
history-no-entries = 暂无记录
history-entry = { $date }  { $name }  { $sets } 组  { $total } { $label }
history-set-weighted = #{ $number }  { $weight }公斤 x { $reps }
history-set-bodyweight = #{ $number }  { $reps } 次
history-set-distance = #{ $number }  { $distance } 公里
history-set-endurance = #{ $number }  { $duration } 分钟
history-note = 备注：{ $note }
history-delete-confirm = 删除此记录？

# ── 趋势 ──
trends-title = 趋势 — 选择练习项目
trends-last-days = 最近 { $days } 天
trends-no-data = 此时段无数据。
trends-avg = 平均：{ $value }
trends-peak = 峰值：{ $value }
trends-trend = 趋势：{ $sign }{ $value }%

# ── 练习项目 ──
practices-title = 练习项目
practices-no-items = 还没有练习项目。按 'a' 添加。
practices-new-name = 新练习名称：
practices-select-type = 选择类型：
practices-rename = 重命名练习：
practices-delete-confirm = 删除 { $name }？
practices-delete-warning = 这将删除其所有记录。

# ── 练习类型标签 ──
practice-type-weighted = 重量x次数
practice-type-bodyweight = 次数
practice-type-distance = 距离
practice-type-endurance = 耐力

# ── 指标标签 ──
metric-kg-vol = 公斤量
metric-reps = 次
metric-km = 公里
metric-min = 分钟

# ── 组数据格式化 ──
set-weighted = { $weight } 公斤 x { $reps } 次
set-bodyweight = { $reps } 次
set-distance = { $distance } 公里
set-endurance = { $duration } 分钟

# ── 按键标签 ──
key-log = 记录
key-history = 历史
key-trends = 趋势
key-practices = 练习项目
key-goals = 目标
key-quotes = 语录
key-quit = 退出
key-navigate = 导航
key-filter = 过滤
key-select = 选择
key-back = 返回
key-add = 添加
key-edit = 编辑
key-delete = 删除
key-confirm = 确认
key-cancel = 取消
key-add-set = 添加组
key-save = 保存
key-date = 日期
key-del-last = 删除上组
key-add-goal = 添加目标
key-milestone = 里程碑
key-toggle = 切换
key-close = 关闭
key-window = 窗口
key-pick-practice = 选择练习
key-dashboard = 仪表盘
key-yes = 是
key-no = 否

# ── 命令行 ──
cli-export-about = 导出所有数据为 JSON
cli-import-about = 从 JSON 导入数据
cli-export-path-help = 输出文件路径（默认 ~/.ironcli/iron-export-YYYY-MM-DD.json）
cli-import-path-help = 输入文件路径
cli-export-complete = 导出完成。
cli-imported = 已导入 { $count } 条记录。
cli-exported-to = 已导出到 { $path }
```

- [ ] **Step 2: Commit**

```bash
git add locales/zh-CN.ftl
git commit -m "feat(i18n): add Chinese (Simplified) translation file"
```

---

### Task 4: Implement i18n module with tests

**Files:**
- Create: `src/i18n.rs`
- Modify: `src/main.rs` (add `mod i18n;`)
- Modify: `src/lib.rs` (add `pub mod i18n;`)
- Create: `tests/i18n_test.rs`

- [ ] **Step 1: Write tests for i18n module**

Create `tests/i18n_test.rs`:

```rust
use std::collections::HashSet;

#[test]
fn tr_returns_english_by_default() {
    // init defaults to English when LANG is not zh
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("dashboard-goals");
    assert_eq!(result, "Goals");
}

#[test]
fn tr_returns_chinese_when_locale_is_zh() {
    std::env::set_var("LANG", "zh_CN.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("dashboard-goals");
    assert_eq!(result, "目标");
}

#[test]
fn tr_args_interpolates_values() {
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]);
    assert_eq!(result, "5 sessions");
}

#[test]
fn tr_args_interpolates_chinese() {
    std::env::set_var("LANG", "zh_CN.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]);
    assert_eq!(result, "5 次训练");
}

#[test]
fn tr_fallback_for_missing_key() {
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("nonexistent-key");
    assert_eq!(result, "nonexistent-key");
}

#[test]
fn ftl_files_have_matching_keys() {
    let en_src = include_str!("../locales/en.ftl");
    let zh_src = include_str!("../locales/zh-CN.ftl");

    let en_keys = extract_message_keys(en_src);
    let zh_keys = extract_message_keys(zh_src);

    let missing_in_zh: Vec<&String> = en_keys.difference(&zh_keys).collect();
    let extra_in_zh: Vec<&String> = zh_keys.difference(&en_keys).collect();

    assert!(
        missing_in_zh.is_empty(),
        "Keys in en.ftl but missing from zh-CN.ftl: {:?}",
        missing_in_zh
    );
    assert!(
        extra_in_zh.is_empty(),
        "Keys in zh-CN.ftl but not in en.ftl: {:?}",
        extra_in_zh
    );
}

fn extract_message_keys(src: &str) -> HashSet<String> {
    use fluent_syntax::parser;
    use fluent_syntax::ast::Entry;

    let resource = parser::parse(src).expect("Failed to parse .ftl file");
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            Entry::Message(msg) => Some(msg.id.name.to_string()),
            _ => None,
        })
        .collect()
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test i18n_test`
Expected: FAIL — `ironcli::i18n` does not exist yet

- [ ] **Step 3: Implement `src/i18n.rs`**

```rust
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;
use std::cell::RefCell;

const EN_FTL: &str = include_str!("../locales/en.ftl");
const ZH_FTL: &str = include_str!("../locales/zh-CN.ftl");

struct I18nBundle {
    bundle: FluentBundle<FluentResource>,
}

thread_local! {
    static BUNDLE: RefCell<Option<I18nBundle>> = const { RefCell::new(None) };
}

pub fn init() {
    let lang = detect_locale();
    let ftl_src = if lang.starts_with("zh") { ZH_FTL } else { EN_FTL };
    let langid: LanguageIdentifier = if lang.starts_with("zh") {
        "zh-CN".parse().unwrap()
    } else {
        "en".parse().unwrap()
    };

    let resource = FluentResource::try_new(ftl_src.to_string())
        .expect("Failed to parse FTL");
    let mut bundle = FluentBundle::new(vec![langid]);
    bundle.add_resource(resource)
        .expect("Failed to add FTL resource");

    BUNDLE.with(|b| {
        *b.borrow_mut() = Some(I18nBundle { bundle });
    });
}

fn detect_locale() -> String {
    std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".to_string())
}

pub fn tr(key: &str) -> String {
    BUNDLE.with(|b| {
        let borrow = b.borrow();
        let i18n = match borrow.as_ref() {
            Some(i) => i,
            None => return key.to_string(),
        };
        let msg = match i18n.bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };
        let mut errors = vec![];
        i18n.bundle.format_pattern(pattern, None, &mut errors).to_string()
    })
}

pub fn tr_args(key: &str, args: &[(&str, FluentValue)]) -> String {
    BUNDLE.with(|b| {
        let borrow = b.borrow();
        let i18n = match borrow.as_ref() {
            Some(i) => i,
            None => return key.to_string(),
        };
        let msg = match i18n.bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, v.clone());
        }
        let mut errors = vec![];
        i18n.bundle.format_pattern(pattern, Some(&fluent_args), &mut errors).to_string()
    })
}
```

- [ ] **Step 4: Add `mod i18n;` to `src/main.rs` and `pub mod i18n;` to `src/lib.rs`**

In `src/main.rs`, add after the existing `mod tui;` line:

```rust
mod i18n;
```

In `src/lib.rs`, add:

```rust
pub mod i18n;
```

- [ ] **Step 5: Call `i18n::init()` in main**

In `src/main.rs`, add at the very start of `fn main()` before `let cli = Cli::parse();`:

```rust
i18n::init();
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test i18n_test`
Expected: all 6 tests PASS

Note: the `tr` and `tr_args` tests set env vars and call `init()` — since tests run in parallel and share the process, the `thread_local!` ensures each thread gets its own bundle. However, if tests are flaky, run with `-- --test-threads=1`.

- [ ] **Step 7: Run all existing tests to verify nothing broke**

Run: `cargo test`
Expected: all tests PASS (existing 23 + 6 new = 29)

- [ ] **Step 8: Commit**

```bash
git add src/i18n.rs src/main.rs src/lib.rs tests/i18n_test.rs
git commit -m "feat(i18n): implement i18n module with locale detection, tr()/tr_args()"
```

---

### Task 5: i18n the model layer

**Files:**
- Modify: `src/model.rs:24-31` (PracticeType::label)
- Modify: `src/model.rs:101-108` (SetData::metric_label)
- Modify: `src/tui/log_entry.rs:796-812` (format_set_data, metric_label_for_type)

These functions return `&'static str` but now need to return `String` since translations are dynamic. Update their return types and all call sites.

- [ ] **Step 1: Change `PracticeType::label()` to use i18n**

In `src/model.rs`, change `label()`:

```rust
pub fn label(&self) -> String {
    use crate::i18n::tr;
    match self {
        PracticeType::Weighted => tr("practice-type-weighted"),
        PracticeType::Bodyweight => tr("practice-type-bodyweight"),
        PracticeType::Distance => tr("practice-type-distance"),
        PracticeType::Endurance => tr("practice-type-endurance"),
    }
}
```

- [ ] **Step 2: Change `SetData::metric_label()` to use i18n**

In `src/model.rs`, change `metric_label()`:

```rust
pub fn metric_label(&self) -> String {
    use crate::i18n::tr;
    match self {
        SetData::Weighted { .. } => tr("metric-kg-vol"),
        SetData::Bodyweight { .. } => tr("metric-reps"),
        SetData::Distance { .. } => tr("metric-km"),
        SetData::Endurance { .. } => tr("metric-min"),
    }
}
```

- [ ] **Step 3: Change `LogEntry::metric_label()` to return `String`**

In `src/model.rs`, change `metric_label()`:

```rust
pub fn metric_label(&self) -> String {
    self.sets
        .first()
        .map(|s| s.data.metric_label())
        .unwrap_or_else(|| "—".to_string())
}
```

- [ ] **Step 4: Update `format_set_data` and `metric_label_for_type` in `log_entry.rs`**

In `src/tui/log_entry.rs`, update the two free functions at the bottom:

```rust
fn format_set_data(set: &SetData) -> String {
    use crate::i18n::tr_args;
    use fluent_bundle::FluentValue;
    match set {
        SetData::Weighted { weight, reps } => tr_args("set-weighted", &[
            ("weight", FluentValue::from(*weight)),
            ("reps", FluentValue::from(*reps as f64)),
        ]),
        SetData::Bodyweight { reps } => tr_args("set-bodyweight", &[
            ("reps", FluentValue::from(*reps as f64)),
        ]),
        SetData::Distance { distance } => tr_args("set-distance", &[
            ("distance", FluentValue::from(*distance)),
        ]),
        SetData::Endurance { duration } => tr_args("set-endurance", &[
            ("duration", FluentValue::from(*duration)),
        ]),
    }
}

fn metric_label_for_type(pt: &PracticeType) -> String {
    use crate::i18n::tr;
    match pt {
        PracticeType::Weighted => tr("metric-kg-vol"),
        PracticeType::Bodyweight => tr("metric-reps"),
        PracticeType::Distance => tr("metric-km"),
        PracticeType::Endurance => tr("metric-min"),
    }
}
```

- [ ] **Step 5: Fix all compilation errors from changed return types**

The return type changes from `&'static str` to `String` will cause errors at call sites that do `format!("... {}", label)` or `Span::styled(label, ...)`. These should mostly "just work" since `String` coerces in format macros and `Span::styled` accepts `Into<String>`. But check:

- `src/tui/log_entry.rs` line 419-423: `label` variable used in format string — now returns `String`, still works with `format!`
- `src/tui/log_entry.rs` line 662-666: same pattern
- `src/tui/dashboard.rs` line 334: `entry.metric_label()` used in format string
- `src/tui/history.rs` line 145: `entry.metric_label()` used in format string
- `src/tui/trends.rs` line 267-268: `metric_label` used in format string

All of these use `.metric_label()` inside `format!()` which accepts `String` fine, but `Span::styled` calls might need `&label` or `label.as_str()` adjustments if the borrow checker complains.

Run: `cargo check`
Fix any remaining type errors.

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/model.rs src/tui/log_entry.rs
git commit -m "feat(i18n): translate practice type labels and metric labels"
```

---

### Task 6: i18n the Dashboard screen

**Files:**
- Modify: `src/tui/dashboard.rs`

Replace all user-facing string literals with `tr()` / `tr_args()` calls. Add `use crate::i18n::{tr, tr_args};` and `use fluent_bundle::FluentValue;` at the top.

- [ ] **Step 1: Add imports**

At the top of `src/tui/dashboard.rs`, add after the existing use statements:

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Replace strings in `render()` method**

All changes in the `render()` method of `DashboardScreen`:

Line 117-119 — empty quote text:
```rust
// Before
"No quotes yet — press Q to add one".to_string()
// After
tr("dashboard-no-quotes")
```

Line 271 — "Last 14 Days" title:
```rust
// Before
.title(Span::styled("Last 14 Days", Style::default().fg(Color::White).bold()))
// After
.title(Span::styled(tr("dashboard-last-14-days"), Style::default().fg(Color::White).bold()))
```

Line 283 — sessions count:
```rust
// Before
format!("{} sessions", self.stats.sessions)
// After
tr_args("dashboard-sessions", &[("count", FluentValue::from(self.stats.sessions as f64))])
```

Line 289 — total volume:
```rust
// Before
format!("{:.0} kg", self.stats.total_volume)
// After
tr_args("dashboard-total-volume", &[("value", FluentValue::from(self.stats.total_volume))])
```

Line 296 — total reps:
```rust
// Before
format!("{:.0} reps", self.stats.total_reps)
// After
tr_args("dashboard-total-reps", &[("value", FluentValue::from(self.stats.total_reps))])
```

Line 303 — total distance:
```rust
// Before
format!("{:.1} km", self.stats.total_distance)
// After
tr_args("dashboard-total-distance", &[("value", FluentValue::from(self.stats.total_distance))])
```

Line 310 — total duration:
```rust
// Before
format!("{:.0} min", self.stats.total_duration)
// After
tr_args("dashboard-total-duration", &[("value", FluentValue::from(self.stats.total_duration))])
```

Line 326 — no entries:
```rust
// Before
"No entries in the last 14 days"
// After
tr("dashboard-no-entries")
```

Line 339 — sets/metric in entry line:
```rust
// Before
format!("  {} sets, {:.0} {}", sets_count, total, label)
// After
format!("  {}", tr_args("dashboard-sets-metric", &[
    ("sets", FluentValue::from(sets_count as f64)),
    ("total", FluentValue::from(total)),
    ("label", FluentValue::from(label.clone())),
]))
```

- [ ] **Step 3: Replace strings in footer spans**

Lines 209-259 — all footer labels. Replace human-readable labels like `" Log  "`, `" History  "`, etc. with calls like `format!(" {}  ", tr("key-log"))`. Keep the key bindings themselves (`[l]`, `[h]`, etc.) unchanged.

For example the Normal mode footer (lines 209-223):
```rust
vec![
    Span::styled(" [l]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-log")), Style::default().fg(Color::Gray)),
    Span::styled("[h]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-history")), Style::default().fg(Color::Gray)),
    Span::styled("[t]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-trends")), Style::default().fg(Color::Gray)),
    Span::styled("[e]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-practices")), Style::default().fg(Color::Gray)),
    Span::styled("[g]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-goals")), Style::default().fg(Color::Gray)),
    Span::styled("[Q]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}  ", tr("key-quotes")), Style::default().fg(Color::Gray)),
    Span::styled("[q]", Style::default().fg(ACCENT)),
    Span::styled(format!(" {}", tr("key-quit")), Style::default().fg(Color::Gray)),
]
```

Apply the same pattern for QuotesManage, Goals, and confirm/cancel footers.

- [ ] **Step 4: Replace strings in goals pane**

Line 422 — "Goals" title:
```rust
.title(Span::styled(tr("dashboard-goals"), Style::default().fg(Color::White).bold()))
```

Line 431 — "Press [g] to add goals":
```rust
tr("dashboard-press-g")
```

Line 485 — "Date (YYYY-MM-DD): ":
```rust
tr("dashboard-date-prompt")
```

Line 507/559 — "Delete? (y/n)":
```rust
tr("dashboard-delete-confirm")
```

Line 586 — "Press [a] to add a goal":
```rust
tr("dashboard-press-a-goal")
```

- [ ] **Step 5: Replace strings in quotes modal**

Line 641 — quotes count title:
```rust
// Before
format!(" Quotes ({}) ", self.quotes.len())
// After
format!(" {} ", tr_args("dashboard-quotes-count", &[("count", FluentValue::from(self.quotes.len() as f64))]))
```

Line 659 — "No quotes — press [a] to add one":
```rust
tr("dashboard-no-quotes-modal")
```

Lines 706-714 — quotes modal shortcut labels: same pattern as footer, use `tr("key-add")`, `tr("key-edit")`, etc.

- [ ] **Step 6: Verify compilation**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 8: Commit**

```bash
git add src/tui/dashboard.rs
git commit -m "feat(i18n): translate Dashboard screen"
```

---

### Task 7: i18n the Log Entry screen

**Files:**
- Modify: `src/tui/log_entry.rs`

- [ ] **Step 1: Add imports**

Add at top of file:

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Replace strings in `render_select_practice`**

Line 144 — "Select Practice":
```rust
tr("log-select-practice")
```

Line 155 — "Press / to filter":
```rust
tr("log-press-filter")
```

Lines 193-203 — footer labels: use `tr("key-navigate")`, `tr("key-filter")`, `tr("key-select")`, `tr("key-back")`.

- [ ] **Step 3: Replace strings in `render_enter_sets`**

Line 315 — "Date: ":
```rust
tr("log-date-label")
```

Line 322 — "(YYYY-MM-DD, Enter to confirm)":
```rust
tr("log-date-edit-hint")
```

Line 329 — "[Enter] confirm  [D] edit":
```rust
tr("log-date-confirm-hint")
```

Line 335 — "[D] to change":
```rust
tr("log-date-change-hint")
```

Line 345 — "Set N: data":
```rust
// Before
format!("  Set {}: {}", i + 1, format_set_data(set))
// After
format!("  {}", tr_args("log-set-line", &[
    ("number", FluentValue::from((i + 1) as f64)),
    ("data", FluentValue::from(format_set_data(set))),
]))
```

Lines 360, 365, 379, 393, 407 — field labels "Weight (kg): ", "Reps: ", "Distance (km): ", "Duration (min): ":
```rust
tr("log-weight-label")   // line 360
tr("log-reps-label")     // line 365
tr("log-reps-label")     // line 379 (bodyweight)
tr("log-distance-label") // line 393
tr("log-duration-label") // line 407
```

Lines 429-432 — sets/total line: replace the `format!` with `tr_args("log-sets-total-reps", ...)` or `tr_args("log-sets-total", ...)` depending on whether `total_reps > 0`.

Lines 449-461 — footer labels: use `tr("key-add-set")`, `tr("key-save")`, `tr("key-date")`, `tr("key-del-last")`, `tr("key-cancel")`.

- [ ] **Step 4: Replace strings in `render_enter_note`**

Line 655 — title "Log {name} — Add Note":
```rust
tr_args("log-add-note-title", &[("name", FluentValue::from(practice.name.clone()))])
```

Line 673 — "N sets logged":
```rust
tr_args("log-sets-logged", &[("count", FluentValue::from(self.sets.len() as f64))])
```

Line 678 — "Total: X label":
```rust
tr_args("log-total-value", &[
    ("total", FluentValue::from(total)),
    ("label", FluentValue::from(label.to_string())),
])
```

Line 686 — "Note (optional)":
```rust
tr("log-note-optional")
```

Lines 702-708 — footer: use `tr("key-save")`, `tr("key-cancel")`.

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src/tui/log_entry.rs
git commit -m "feat(i18n): translate Log Entry screen"
```

---

### Task 8: i18n the History screen

**Files:**
- Modify: `src/tui/history.rs`

- [ ] **Step 1: Add imports**

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Replace all strings**

Line 72 — "History" title:
```rust
tr("history-title")
```

Line 90 — "Delete this entry? ":
```rust
tr("history-delete-confirm")
```

Lines 93, 98-105 — shortcut labels: use `tr("key-yes")`, `tr("key-cancel")`, `tr("key-navigate")`, `tr("key-edit")`, `tr("key-delete")`, `tr("key-back")`.

Line 127 — "No entries yet":
```rust
tr("history-no-entries")
```

Lines 147-149 — entry line:
```rust
// Before
format!("{}{}  {}  {} sets  {:.0} {}", marker, date, entry.practice_name, sets_count, total, label)
// After  
format!("{}{}", marker, tr_args("history-entry", &[
    ("date", FluentValue::from(date)),
    ("name", FluentValue::from(entry.practice_name.clone())),
    ("sets", FluentValue::from(sets_count as f64)),
    ("total", FluentValue::from(total)),
    ("label", FluentValue::from(label.to_string())),
]))
```

Lines 175-186 — set detail lines:
```rust
SetData::Weighted { weight, reps } => tr_args("history-set-weighted", &[
    ("number", FluentValue::from(set.set_number as f64)),
    ("weight", FluentValue::from(*weight)),
    ("reps", FluentValue::from(*reps as f64)),
]),
// etc. for each variant
```

Line 196 — "Note: ":
```rust
tr_args("history-note", &[("note", FluentValue::from(note.clone()))])
```

- [ ] **Step 3: Verify and commit**

Run: `cargo check && cargo test`

```bash
git add src/tui/history.rs
git commit -m "feat(i18n): translate History screen"
```

---

### Task 9: i18n the Trends screen

**Files:**
- Modify: `src/tui/trends.rs`

- [ ] **Step 1: Add imports**

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Replace all strings**

Line 111 — "Trends — Select Practice":
```rust
tr("trends-title")
```

Line 123 — "Press / to filter":
```rust
tr("log-press-filter")
```

Lines 161-170 — select practice footer: same pattern, `tr("key-navigate")`, etc.

Line 288 — "Last N days":
```rust
tr_args("trends-last-days", &[("days", FluentValue::from(self.days_window as f64))])
```

Line 296 — "No data for this period.":
```rust
tr("trends-no-data")
```

Lines 332-342 — stats line:
```rust
tr_args("trends-avg", &[("value", FluentValue::from(avg))])
tr_args("trends-peak", &[("value", FluentValue::from(peak))])
tr_args("trends-trend", &[
    ("sign", FluentValue::from(trend_sign.to_string())),
    ("value", FluentValue::from(trend_pct)),
])
```

Lines 351-357 — chart footer: `tr("key-window")`, `tr("key-pick-practice")`, `tr("key-dashboard")`.

- [ ] **Step 3: Verify and commit**

Run: `cargo check && cargo test`

```bash
git add src/tui/trends.rs
git commit -m "feat(i18n): translate Trends screen"
```

---

### Task 10: i18n the Practices screen

**Files:**
- Modify: `src/tui/practices.rs`

- [ ] **Step 1: Add imports**

```rust
use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentValue;
```

- [ ] **Step 2: Replace all strings**

Line 89 — "Practices":
```rust
tr("practices-title")
```

Line 96 — "No practices yet...":
```rust
tr("practices-no-items")
```

Line 140 — "New practice name:":
```rust
tr("practices-new-name")
```

Line 155 — "Select type:":
```rust
tr("practices-select-type")
```

Line 176 — "Rename practice:":
```rust
tr("practices-rename")
```

Line 197 — "Delete {name}?":
```rust
tr_args("practices-delete-confirm", &[("name", FluentValue::from(name.to_string()))])
```

Line 201 — "This removes all its logs.":
```rust
tr("practices-delete-warning")
```

Lines 217-249 — all shortcut labels: use `tr("key-navigate")`, `tr("key-add")`, etc.

- [ ] **Step 3: Verify and commit**

Run: `cargo check && cargo test`

```bash
git add src/tui/practices.rs
git commit -m "feat(i18n): translate Practices screen"
```

---

### Task 11: i18n the CLI output

**Files:**
- Modify: `src/main.rs`
- Modify: `src/export.rs`

- [ ] **Step 1: Translate CLI output in `main.rs`**

```rust
// Line 38: "Export complete."
println!("{}", i18n::tr("cli-export-complete"));

// Line 43: "Imported {} logs."
println!("{}", i18n::tr_args("cli-imported", &[
    ("count", fluent_bundle::FluentValue::from(count as f64)),
]));
```

- [ ] **Step 2: Translate export.rs output**

Line 185 — "Exported to {}":
```rust
use crate::i18n::tr_args;
use fluent_bundle::FluentValue;

eprintln!("{}", tr_args("cli-exported-to", &[
    ("path", FluentValue::from(out_path.display().to_string())),
]));
```

- [ ] **Step 3: Translate clap help text**

The clap derive macro uses doc comments for help text. To make these dynamic, use the `help` attribute instead:

```rust
#[derive(Parser)]
#[command(name = "iron", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export all data to JSON
    Export {
        /// Output file path (defaults to ~/.ironcli/iron-export-YYYY-MM-DD.json)
        path: Option<PathBuf>,
    },
    /// Import data from JSON
    Import {
        /// Input file path
        path: PathBuf,
    },
}
```

Note: clap's derive macro processes the `about` attribute at compile time, not runtime. Since we want runtime i18n, we need to use `clap::Command::mut_cmd` or set the about dynamically. The simplest approach is to set it after parsing:

Actually, clap doc comments are baked in at compile time. For true runtime i18n of clap help, we'd need to use the builder API. This is low value — CLI help in English is standard even in localized apps. **Skip clap help translation** — it's the one exception. Keep the doc-comment approach.

- [ ] **Step 4: Verify and commit**

Run: `cargo check && cargo test`

```bash
git add src/main.rs src/export.rs
git commit -m "feat(i18n): translate CLI output messages"
```

---

### Task 12: Update README and bump version

**Files:**
- Modify: `README.md`
- Modify: `Cargo.toml` (version bump)

- [ ] **Step 1: Bump version**

In `Cargo.toml`, increment the minor version (this is a feature, not a patch):
```toml
version = "0.3.0"
```

- [ ] **Step 2: Update README.md**

Add an "i18n / Language" section to the README documenting:
- Supported languages: English (default), Chinese (Simplified)
- Auto-detection from `LANG`/`LC_ALL` environment variables
- How to switch: `LANG=zh_CN.UTF-8 iron` to run in Chinese
- All UI elements and CLI messages are translated

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml README.md
git commit -m "feat: add i18n support for English and Chinese (Simplified)"
```

---

### Task 13: Manual verification

- [ ] **Step 1: Build and test in English**

```bash
LANG=en_US.UTF-8 cargo run
```

Navigate through all screens: Dashboard, Log Entry, History, Trends, Practices. Verify all text is English.

- [ ] **Step 2: Build and test in Chinese**

```bash
LANG=zh_CN.UTF-8 cargo run
```

Navigate through all screens. Verify all text is Chinese, units are translated (公斤, 公里, 分钟), keyboard shortcuts still work.

- [ ] **Step 3: Test export/import CLI messages**

```bash
LANG=zh_CN.UTF-8 cargo run -- export
LANG=en_US.UTF-8 cargo run -- export
```

Verify output messages are in the correct language.
