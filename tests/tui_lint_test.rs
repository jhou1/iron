use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use syn::visit::{self, Visit};

fn starts_char_literal(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut probe = chars.clone();
    match probe.next() {
        Some('\\') => match probe.next() {
            Some('u') if probe.next() == Some('{') => {
                for character in probe.by_ref() {
                    if character == '}' {
                        return probe.next() == Some('\'');
                    }
                }
                false
            }
            Some('x') => probe.nth(1).is_some() && probe.next() == Some('\''),
            Some(_) => probe.next() == Some('\''),
            None => false,
        },
        Some(character) if character != '\n' && character != '\'' => probe.next() == Some('\''),
        _ => false,
    }
}

fn source_code(source: &str) -> String {
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut block_comment_depth = 0usize;
    let mut in_line_comment = false;
    let mut in_string = false;
    let mut in_char = false;
    let mut raw_string_hashes = None;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if in_line_comment {
            if character == '\n' {
                in_line_comment = false;
                code.push(character);
            }
            continue;
        }

        if block_comment_depth > 0 {
            match (character, chars.peek().copied()) {
                ('/', Some('*')) => {
                    chars.next();
                    block_comment_depth += 1;
                }
                ('*', Some('/')) => {
                    chars.next();
                    block_comment_depth -= 1;
                }
                ('\n', _) => code.push(character),
                _ => {}
            }
            continue;
        }

        if let Some(hash_count) = raw_string_hashes {
            if character == '"' {
                let mut probe = chars.clone();
                if (0..hash_count).all(|_| probe.next() == Some('#')) {
                    for _ in 0..hash_count {
                        chars.next();
                    }
                    raw_string_hashes = None;
                }
            } else if character == '\n' {
                code.push(character);
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            } else if character == '\n' {
                code.push(character);
            }
            continue;
        }

        if in_char {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                in_char = false;
            }
            continue;
        }

        if character == 'r' {
            let mut probe = chars.clone();
            let mut hash_count = 0;
            while probe.next_if_eq(&'#').is_some() {
                hash_count += 1;
            }
            if probe.next() == Some('"') {
                for _ in 0..=hash_count {
                    chars.next();
                }
                raw_string_hashes = Some(hash_count);
                continue;
            }
        }

        match (character, chars.peek().copied()) {
            ('/', Some('/')) => {
                chars.next();
                in_line_comment = true;
            }
            ('/', Some('*')) => {
                chars.next();
                block_comment_depth = 1;
            }
            ('"', _) => {
                in_string = true;
            }
            ('\'', _) if starts_char_literal(&chars) => {
                in_char = true;
            }
            _ => code.push(character),
        }
    }

    code
}

fn source_code_contains(source: &str, needle: &str) -> bool {
    source_code(source).contains(needle)
}

fn source_code_count(source: &str, needle: &str) -> usize {
    source_code(source).match_indices(needle).count()
}

fn dashboard_production_source(source: &str) -> String {
    let code = source_code(source);
    code.split("#[cfg(test)]")
        .next()
        .unwrap_or(&code)
        .to_owned()
}

fn dashboard_ownership_violations(source: &str) -> Vec<String> {
    let production = dashboard_production_source(source);
    let mut violations = Vec::new();
    for (needle, expected, label) in [
        ("render_screen_frame_sections(", 1, "ScreenFrameSections"),
        ("render_static_panel(", 4, "StaticPanel"),
    ] {
        let actual = source_code_count(&production, needle);
        if actual != expected {
            violations.push(format!(
                "Dashboard must delegate to {label} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    for forbidden in [".borders(Borders::ALL)", "dashboard_block_inner("] {
        if source_code_contains(&production, forbidden) {
            violations.push(format!("Dashboard still owns {forbidden}"));
        }
    }
    violations
}

fn method_region(source: &str, start: &str, end: &str) -> String {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("missing {start}"));
    let tail = &source[start_index..];
    let end_index = tail.find(end).unwrap_or_else(|| panic!("missing {end}"));
    source_code(&tail[..end_index])
}

fn log_entry_selector_ownership_violations(source: &str) -> Vec<String> {
    let selector = method_region(
        source,
        "fn render_select_practice",
        "fn handle_select_practice",
    );
    let mut violations = Vec::new();
    for (needle, expected, label) in [
        ("render_screen_frame_sections(", 1, "ScreenFrameSections"),
        ("render_scrollable_list_panel(", 1, "ScrollableListPanel"),
    ] {
        let actual = selector.match_indices(needle).count();
        if actual != expected {
            violations.push(format!(
                "Log Entry selector must call {label} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    for forbidden in [
        ".borders(Borders::ALL)",
        ".inner(",
        "render_status_line(",
        "render_shortcuts(",
        "highlight_row(",
        ".skip(",
        ".take(",
    ] {
        if selector.contains(forbidden) {
            violations.push(format!("Log Entry selector still owns {forbidden}"));
        }
    }
    violations
}

const LOG_ENTRY_REQUIRED_COMPONENTS: [&str; 3] = [
    "render_screen_frame_sections",
    "render_scrollable_list_panel",
    "render_bounded_panel",
];

fn is_log_entry_required_component(name: &str) -> bool {
    LOG_ENTRY_REQUIRED_COMPONENTS.contains(&name)
}

fn is_log_entry_forbidden_function(name: &str) -> bool {
    matches!(
        name,
        "render_status_line" | "render_shortcuts" | "highlight_row"
    )
}

#[derive(Clone)]
struct LogEntryBinding {
    original: String,
    canonical_framework_import: bool,
}

#[derive(Default)]
struct LogEntryScope {
    values: HashMap<String, LogEntryBinding>,
    types: HashMap<String, String>,
    value_types: HashMap<String, String>,
}

#[derive(Clone, Copy)]
enum LogEntryLocalCallable<'ast> {
    Function(&'ast syn::ItemFn),
    Closure(&'ast syn::ExprClosure),
    Shadowed,
}

fn collect_log_entry_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    scope: &mut LogEntryScope,
    wrong_origin_required: &mut BTreeSet<String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_log_entry_imports(&path.tree, prefix, scope, wrong_origin_required);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_log_entry_imports(item, prefix, scope, wrong_origin_required);
            }
        }
        syn::UseTree::Name(name) => {
            let local_name = name.ident.to_string();
            let mut origin = prefix.clone();
            origin.push(local_name.clone());
            let canonical = matches!(
                origin.as_slice(),
                [root, framework, component]
                    if root == "super"
                        && framework == "framework"
                        && component == &local_name
                        && is_log_entry_required_component(component)
            );
            if is_log_entry_required_component(&local_name)
                || is_log_entry_forbidden_function(&local_name)
            {
                scope.values.insert(
                    local_name.clone(),
                    LogEntryBinding {
                        original: local_name.clone(),
                        canonical_framework_import: canonical,
                    },
                );
                if is_log_entry_required_component(&local_name) && !canonical {
                    wrong_origin_required.insert(local_name);
                }
            } else if local_name == "Block" {
                scope.types.insert(local_name.clone(), local_name);
            }
        }
        syn::UseTree::Rename(rename) => {
            let original = rename.ident.to_string();
            let local_name = rename.rename.to_string();
            if is_log_entry_required_component(&original)
                || is_log_entry_forbidden_function(&original)
            {
                scope.values.insert(
                    local_name,
                    LogEntryBinding {
                        original: original.clone(),
                        canonical_framework_import: false,
                    },
                );
                if is_log_entry_required_component(&original) {
                    wrong_origin_required.insert(original);
                }
            } else if original == "Block" {
                scope.types.insert(local_name, original);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

#[derive(Default)]
struct LogEntryPatternBindingCollector {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for LogEntryPatternBindingCollector {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.names.insert(pattern.ident.to_string());
        visit::visit_pat_ident(self, pattern);
    }
}

fn log_entry_single_binding_ident(pattern: &syn::Pat) -> Option<&syn::Ident> {
    match pattern {
        syn::Pat::Ident(pattern) => Some(&pattern.ident),
        syn::Pat::Paren(pattern) => log_entry_single_binding_ident(&pattern.pat),
        syn::Pat::Type(pattern) => log_entry_single_binding_ident(&pattern.pat),
        _ => None,
    }
}

#[derive(Default)]
struct LogEntryRenderOwnership {
    screen_frame_calls: usize,
    list_panel_calls: usize,
    bounded_panel_calls: usize,
    borders_method_calls: usize,
    block_constructor_calls: usize,
    status_line_calls: usize,
    shortcuts_calls: usize,
    highlight_calls: usize,
    selector_skip_calls: usize,
    selector_take_calls: usize,
    called_methods: BTreeSet<String>,
    wrong_origin_required_calls: BTreeSet<String>,
}

impl LogEntryRenderOwnership {
    fn merge(&mut self, other: Self) {
        self.screen_frame_calls += other.screen_frame_calls;
        self.list_panel_calls += other.list_panel_calls;
        self.bounded_panel_calls += other.bounded_panel_calls;
        self.borders_method_calls += other.borders_method_calls;
        self.block_constructor_calls += other.block_constructor_calls;
        self.status_line_calls += other.status_line_calls;
        self.shortcuts_calls += other.shortcuts_calls;
        self.highlight_calls += other.highlight_calls;
        self.selector_skip_calls += other.selector_skip_calls;
        self.selector_take_calls += other.selector_take_calls;
        self.called_methods.extend(other.called_methods);
        self.wrong_origin_required_calls
            .extend(other.wrong_origin_required_calls);
    }
}

struct LogEntryRenderVisitor<'a> {
    scopes: Vec<LogEntryScope>,
    callable_scopes: Vec<HashMap<String, LogEntryLocalCallable<'a>>>,
    active_local_callables: HashSet<String>,
    ownership: LogEntryRenderOwnership,
    _canonical_framework_imports: &'a BTreeSet<String>,
}

impl<'a> LogEntryRenderVisitor<'a> {
    fn new(canonical_framework_imports: &'a BTreeSet<String>) -> Self {
        let values = canonical_framework_imports
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    LogEntryBinding {
                        original: name.clone(),
                        canonical_framework_import: true,
                    },
                )
            })
            .collect();
        Self {
            scopes: vec![LogEntryScope {
                values,
                types: HashMap::new(),
                value_types: HashMap::new(),
            }],
            callable_scopes: vec![HashMap::new()],
            active_local_callables: HashSet::new(),
            ownership: LogEntryRenderOwnership::default(),
            _canonical_framework_imports: canonical_framework_imports,
        }
    }

    fn push_signature_scope(&mut self, signature: &syn::Signature) {
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        for input in &signature.inputs {
            if let syn::FnArg::Typed(input) = input {
                self.shadow_pattern(&input.pat);
                self.record_pattern_type(&input.pat, &input.ty);
            }
        }
    }

    fn block_scope(&self, block: &syn::Block) -> LogEntryScope {
        let mut scope = LogEntryScope::default();
        let mut ignored_wrong_origins = BTreeSet::new();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            match item {
                syn::Item::Const(item) if !is_cfg_test(&item.attrs) => {
                    self.insert_required_shadow(&mut scope, item.ident.to_string());
                }
                syn::Item::Fn(item) if !is_cfg_test(&item.attrs) => {
                    self.insert_required_shadow(&mut scope, item.sig.ident.to_string());
                }
                syn::Item::Static(item) if !is_cfg_test(&item.attrs) => {
                    self.insert_required_shadow(&mut scope, item.ident.to_string());
                }
                syn::Item::Type(item) if !is_cfg_test(&item.attrs) => {
                    if let Some(target) = type_path_name(&item.ty) {
                        scope.types.insert(item.ident.to_string(), target);
                    }
                }
                syn::Item::Use(item) if !is_cfg_test(&item.attrs) => collect_log_entry_imports(
                    &item.tree,
                    &mut Vec::new(),
                    &mut scope,
                    &mut ignored_wrong_origins,
                ),
                _ => {}
            }
        }
        scope
    }

    fn block_callable_scope(
        &self,
        block: &'a syn::Block,
    ) -> HashMap<String, LogEntryLocalCallable<'a>> {
        block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(syn::Item::Fn(function)) if !is_cfg_test(&function.attrs) => {
                    Some((
                        function.sig.ident.to_string(),
                        LogEntryLocalCallable::Function(function),
                    ))
                }
                _ => None,
            })
            .collect()
    }

    fn insert_required_shadow(&self, scope: &mut LogEntryScope, name: String) {
        if is_log_entry_required_component(&name) {
            scope.values.insert(
                name.clone(),
                LogEntryBinding {
                    original: name,
                    canonical_framework_import: false,
                },
            );
        }
    }

    fn record_pattern_type(&mut self, pattern: &syn::Pat, ty: &syn::Type) {
        let Some(type_name) = type_path_name(ty) else {
            return;
        };
        let mut bindings = LogEntryPatternBindingCollector::default();
        bindings.visit_pat(pattern);
        let scope = self.scopes.last_mut().unwrap();
        for name in bindings.names {
            scope.value_types.insert(name, type_name.clone());
        }
    }

    fn shadow_pattern(&mut self, pattern: &syn::Pat) {
        let mut bindings = LogEntryPatternBindingCollector::default();
        bindings.visit_pat(pattern);
        let scope = self.scopes.last_mut().unwrap();
        for name in bindings.names {
            if is_log_entry_required_component(&name) {
                scope.values.insert(
                    name.clone(),
                    LogEntryBinding {
                        original: name.clone(),
                        canonical_framework_import: false,
                    },
                );
            }
            self.callable_scopes
                .last_mut()
                .unwrap()
                .insert(name, LogEntryLocalCallable::Shadowed);
        }
    }

    fn resolve_local_callable(&self, name: &str) -> Option<LogEntryLocalCallable<'a>> {
        for scope in self.callable_scopes.iter().rev() {
            if let Some(callable) = scope.get(name) {
                return match callable {
                    LogEntryLocalCallable::Function(function) => {
                        Some(LogEntryLocalCallable::Function(function))
                    }
                    LogEntryLocalCallable::Closure(closure) => {
                        Some(LogEntryLocalCallable::Closure(closure))
                    }
                    LogEntryLocalCallable::Shadowed => None,
                };
            }
        }
        None
    }

    fn visit_local_callable(&mut self, name: &str, callable: LogEntryLocalCallable<'a>) {
        if !self.active_local_callables.insert(name.to_owned()) {
            return;
        }
        match callable {
            LogEntryLocalCallable::Function(function) => {
                self.push_signature_scope(&function.sig);
                self.visit_block(&function.block);
                self.callable_scopes.pop();
                self.scopes.pop();
            }
            LogEntryLocalCallable::Closure(closure) => self.visit_executable_closure(closure),
            LogEntryLocalCallable::Shadowed => {}
        }
        self.active_local_callables.remove(name);
    }

    fn resolve_unqualified(&self, local_name: &str) -> Option<&LogEntryBinding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.values.get(local_name))
    }

    fn resolved_call(&self, function: &syn::ExprPath) -> (String, bool) {
        let called_name = function
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if function.qself.is_none() && function.path.segments.len() == 1 {
            if let Some(binding) = self.resolve_unqualified(&called_name) {
                return (binding.original.clone(), binding.canonical_framework_import);
            }
        }
        (called_name, false)
    }

    fn resolved_type_name(&self, name: &str) -> String {
        let mut resolved = name.to_owned();
        let mut visited = HashSet::new();
        while visited.insert(resolved.clone()) {
            let Some(next) = self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.types.get(&resolved))
            else {
                break;
            };
            if next == &resolved {
                break;
            }
            resolved = next.clone();
        }
        resolved
    }

    fn resolved_value_type(&self, name: &str) -> Option<String> {
        let type_name = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.value_types.get(name))?;
        Some(self.resolved_type_name(type_name))
    }

    fn receiver_is_block(&self, receiver: &syn::Expr) -> bool {
        match receiver {
            syn::Expr::Group(group) => self.receiver_is_block(&group.expr),
            syn::Expr::Paren(paren) => self.receiver_is_block(&paren.expr),
            syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => self
                .resolved_value_type(&path.path.segments[0].ident.to_string())
                .is_some_and(|name| name == "Block"),
            syn::Expr::Reference(reference) => self.receiver_is_block(&reference.expr),
            _ => false,
        }
    }

    fn is_block_constructor(&self, function: &syn::ExprPath) -> bool {
        let Some(method) = function.path.segments.last() else {
            return false;
        };
        if !matches!(
            method.ident.to_string().as_str(),
            "default" | "bordered" | "inner"
        ) {
            return false;
        }
        let receiver = if let Some(qself) = &function.qself {
            type_path_name(&qself.ty)
        } else if function.path.segments.len() >= 2 {
            function
                .path
                .segments
                .iter()
                .nth(function.path.segments.len() - 2)
                .map(|segment| segment.ident.to_string())
        } else {
            None
        };
        receiver.is_some_and(|name| self.resolved_type_name(&name) == "Block")
    }

    fn visit_condition_with_bindings(&mut self, expression: &'a syn::Expr) {
        match expression {
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                self.visit_condition_with_bindings(&binary.left);
                self.visit_condition_with_bindings(&binary.right);
            }
            syn::Expr::Group(group) => self.visit_condition_with_bindings(&group.expr),
            syn::Expr::Let(expression) => {
                self.visit_expr(&expression.expr);
                self.shadow_pattern(&expression.pat);
            }
            syn::Expr::Paren(paren) => self.visit_condition_with_bindings(&paren.expr),
            _ => self.visit_expr(expression),
        }
    }

    fn visit_executable_closure(&mut self, closure: &'a syn::ExprClosure) {
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        for input in &closure.inputs {
            self.shadow_pattern(input);
        }
        self.visit_expr(&closure.body);
        self.callable_scopes.pop();
        self.scopes.pop();
    }
}

impl<'ast> Visit<'ast> for LogEntryRenderVisitor<'ast> {
    fn visit_item(&mut self, _item: &'ast syn::Item) {
        // Nested declarations are not executable calls on the production render path.
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes.push(self.block_scope(block));
        self.callable_scopes.push(self.block_callable_scope(block));
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.callable_scopes.pop();
        self.scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if is_cfg_test(&local.attrs) {
            return;
        }
        if let Some(initializer) = &local.init {
            self.visit_expr(&initializer.expr);
            if let Some((_, diverge)) = &initializer.diverge {
                self.visit_expr(diverge);
            }
        }
        self.shadow_pattern(&local.pat);
        if let syn::Pat::Type(pattern) = &local.pat {
            self.record_pattern_type(&pattern.pat, &pattern.ty);
        }
        if let (Some(binding), Some(closure)) = (
            log_entry_single_binding_ident(&local.pat),
            local
                .init
                .as_ref()
                .and_then(|initializer| direct_invoked_closure(&initializer.expr)),
        ) {
            self.callable_scopes
                .last_mut()
                .unwrap()
                .insert(binding.to_string(), LogEntryLocalCallable::Closure(closure));
        }
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if is_cfg_test(log_entry_expr_attrs(expression)) {
            return;
        }
        visit::visit_expr(self, expression);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let _ = closure;
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        self.shadow_pattern(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.callable_scopes.pop();
        self.scopes.pop();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        self.shadow_pattern(&expression.pat);
        self.visit_block(&expression.body);
        self.callable_scopes.pop();
        self.scopes.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        self.visit_condition_with_bindings(&expression.cond);
        self.visit_block(&expression.then_branch);
        self.callable_scopes.pop();
        self.scopes.pop();
        if let Some((_, else_branch)) = &expression.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.scopes.push(LogEntryScope::default());
        self.callable_scopes.push(HashMap::new());
        self.visit_condition_with_bindings(&expression.cond);
        self.visit_block(&expression.body);
        self.callable_scopes.pop();
        self.scopes.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Some(closure) = direct_invoked_closure(&call.func) {
            self.visit_executable_closure(closure);
            for argument in &call.args {
                self.visit_expr(argument);
            }
            return;
        }
        if let syn::Expr::Path(function) = call.func.as_ref() {
            let unqualified_name = (function.qself.is_none() && function.path.segments.len() == 1)
                .then(|| function.path.segments[0].ident.to_string());
            if let Some((name, callable)) = unqualified_name.as_deref().and_then(|name| {
                self.resolve_local_callable(name)
                    .map(|callable| (name, callable))
            }) {
                self.visit_local_callable(name, callable);
            }
            let (called_name, canonical_framework_import) = self.resolved_call(function);
            let executes_callback =
                canonical_framework_import && is_log_entry_required_component(&called_name);
            match called_name.as_str() {
                "render_screen_frame_sections" if canonical_framework_import => {
                    self.ownership.screen_frame_calls += 1;
                }
                "render_scrollable_list_panel" if canonical_framework_import => {
                    self.ownership.list_panel_calls += 1;
                }
                "render_bounded_panel" if canonical_framework_import => {
                    self.ownership.bounded_panel_calls += 1;
                }
                "render_screen_frame_sections"
                | "render_scrollable_list_panel"
                | "render_bounded_panel" => {
                    self.ownership
                        .wrong_origin_required_calls
                        .insert(called_name);
                }
                "render_status_line" => self.ownership.status_line_calls += 1,
                "render_shortcuts" => self.ownership.shortcuts_calls += 1,
                "highlight_row" => self.ownership.highlight_calls += 1,
                _ => {}
            }
            if self.is_block_constructor(function) {
                self.ownership.block_constructor_calls += 1;
            }
            if executes_callback {
                for argument in &call.args {
                    if let syn::Expr::Closure(closure) = argument {
                        self.visit_executable_closure(closure);
                    }
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        match call.method.to_string().as_str() {
            "borders" => self.ownership.borders_method_calls += 1,
            "inner" if self.receiver_is_block(&call.receiver) => {
                self.ownership.block_constructor_calls += 1;
            }
            "skip" => self.ownership.selector_skip_calls += 1,
            "take" => self.ownership.selector_take_calls += 1,
            _ => {}
        }
        if matches!(call.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) {
            self.ownership
                .called_methods
                .insert(call.method.to_string());
        }
        visit::visit_expr_method_call(self, call);
    }
}

fn log_entry_ownership_violations(source: &str) -> Vec<String> {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => return vec![format!("Log Entry source must parse as Rust: {error}")],
    };
    let production_impls = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_none()
                    && !is_cfg_test(&item_impl.attrs)
                    && is_screen_type(item_impl.self_ty.as_ref(), "LogEntryScreen") =>
            {
                Some(item_impl)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if production_impls.len() != 1 {
        return vec![format!(
            "Log Entry must define exactly one production inherent impl LogEntryScreen; found {}",
            production_impls.len()
        )];
    }
    let item_impl = production_impls[0];
    let methods = item_impl
        .items
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) if !is_cfg_test(&method.attrs) => {
                Some((method.sig.ident.to_string(), method))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    if !methods.contains_key("render") {
        return vec!["Log Entry must define one production LogEntryScreen::render method".into()];
    }

    let mut root_scope = LogEntryScope::default();
    let mut import_wrong_origins = BTreeSet::new();
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            if !is_cfg_test(&item.attrs) {
                collect_log_entry_imports(
                    &item.tree,
                    &mut Vec::new(),
                    &mut root_scope,
                    &mut import_wrong_origins,
                );
            }
        }
    }
    for item in &file.items {
        match item {
            syn::Item::Const(item) if !is_cfg_test(&item.attrs) => {
                let name = item.ident.to_string();
                if is_log_entry_required_component(&name) {
                    root_scope.values.insert(
                        name.clone(),
                        LogEntryBinding {
                            original: name,
                            canonical_framework_import: false,
                        },
                    );
                }
            }
            syn::Item::Fn(item) if !is_cfg_test(&item.attrs) => {
                let name = item.sig.ident.to_string();
                if is_log_entry_required_component(&name) {
                    root_scope.values.insert(
                        name.clone(),
                        LogEntryBinding {
                            original: name,
                            canonical_framework_import: false,
                        },
                    );
                }
            }
            syn::Item::Static(item) if !is_cfg_test(&item.attrs) => {
                let name = item.ident.to_string();
                if is_log_entry_required_component(&name) {
                    root_scope.values.insert(
                        name.clone(),
                        LogEntryBinding {
                            original: name,
                            canonical_framework_import: false,
                        },
                    );
                }
            }
            syn::Item::Type(item) if !is_cfg_test(&item.attrs) => {
                if let Some(target) = type_path_name(&item.ty) {
                    root_scope.types.insert(item.ident.to_string(), target);
                }
            }
            _ => {}
        }
    }
    let canonical_framework_imports = root_scope
        .values
        .iter()
        .filter(|(_, binding)| binding.canonical_framework_import)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();

    let mut ownership = LogEntryRenderOwnership::default();
    ownership
        .wrong_origin_required_calls
        .extend(import_wrong_origins);
    let mut pending = vec!["render".to_owned()];
    let mut visited = HashSet::new();
    while let Some(method_name) = pending.pop() {
        if !visited.insert(method_name.clone()) {
            continue;
        }
        let Some(method) = methods.get(&method_name) else {
            continue;
        };
        let mut visitor = LogEntryRenderVisitor::new(&canonical_framework_imports);
        visitor.scopes[0].types = root_scope.types.clone();
        for (name, binding) in &root_scope.values {
            visitor.scopes[0]
                .values
                .insert(name.clone(), binding.clone());
        }
        visitor.push_signature_scope(&method.sig);
        visitor.visit_block(&method.block);
        visitor.callable_scopes.pop();
        visitor.scopes.pop();
        pending.extend(
            visitor
                .ownership
                .called_methods
                .iter()
                .filter(|called| {
                    methods.contains_key(called.as_str()) && !visited.contains(called.as_str())
                })
                .cloned(),
        );
        ownership.merge(visitor.ownership);
    }

    let mut violations = Vec::new();
    for (actual, expected, component) in [
        (ownership.screen_frame_calls, 2, "ScreenFrameSections"),
        (ownership.list_panel_calls, 1, "ScrollableListPanel"),
        (ownership.bounded_panel_calls, 4, "BoundedPanel"),
    ] {
        if actual != expected {
            violations.push(format!(
                "LogEntryScreen production render path must call {component} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    for name in ownership.wrong_origin_required_calls {
        violations.push(format!(
            "LogEntryScreen production render path has wrong-origin required component `{name}`"
        ));
    }
    for (count, forbidden) in [
        (ownership.borders_method_calls, ".borders"),
        (
            ownership.block_constructor_calls,
            "local Block construction/inner",
        ),
        (ownership.status_line_calls, "render_status_line"),
        (ownership.shortcuts_calls, "render_shortcuts"),
        (ownership.highlight_calls, "highlight_row"),
        (ownership.selector_skip_calls, "selector .skip"),
        (ownership.selector_take_calls, "selector .take"),
    ] {
        if count != 0 {
            violations.push(format!(
                "LogEntryScreen production render path must not own {forbidden}; found {count}"
            ));
        }
    }
    violations
}

fn dashboard_heatmap_construction_violations(source: &str) -> Vec<String> {
    let production = dashboard_production_source(source);
    let mut violations = Vec::new();
    for (needle, expected, label) in [
        ("Heatmap::new(", 1, "local-date Heatmap constructor"),
        (
            "Heatmap::with_today(",
            0,
            "test-only fixed-date Heatmap seam",
        ),
    ] {
        let actual = source_code_count(&production, needle);
        if actual != expected {
            violations.push(format!(
                "Dashboard production must call {label} exactly {expected} time(s); found {actual}"
            ));
        }
    }
    violations
}

fn is_screen_type(ty: &syn::Type, expected: &str) -> bool {
    matches!(
        ty,
        syn::Type::Path(path)
            if path.qself.is_none()
                && path.path.segments.last().is_some_and(|segment| segment.ident == expected)
    )
}

fn is_history_screen_type(ty: &syn::Type) -> bool {
    is_screen_type(ty, "HistoryScreen")
}

fn is_goals_screen_type(ty: &syn::Type) -> bool {
    is_screen_type(ty, "GoalsScreen")
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    fn meta_is_test_only(meta: &syn::Meta) -> bool {
        match meta {
            syn::Meta::Path(path) => path.is_ident("test"),
            syn::Meta::List(list) => {
                let Ok(items) = list.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                ) else {
                    return false;
                };
                if list.path.is_ident("all") {
                    items.iter().any(meta_is_test_only)
                } else if list.path.is_ident("any") {
                    !items.is_empty() && items.iter().all(meta_is_test_only)
                } else {
                    false
                }
            }
            syn::Meta::NameValue(_) => false,
        }
    }

    attrs.iter().any(|attribute| {
        let syn::Meta::List(list) = &attribute.meta else {
            return false;
        };
        list.path.is_ident("cfg")
            && list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|items| items.iter().any(meta_is_test_only))
    })
}

fn log_entry_expr_attrs(expression: &syn::Expr) -> &[syn::Attribute] {
    macro_rules! attrs {
        ($($variant:ident),+ $(,)?) => {
            match expression {
                $(syn::Expr::$variant(expression) => &expression.attrs,)+
                syn::Expr::Verbatim(_) => &[],
                _ => &[],
            }
        };
    }

    attrs!(
        Array, Assign, Async, Await, Binary, Block, Break, Call, Cast, Closure, Const, Continue,
        Field, ForLoop, Group, If, Index, Infer, Let, Lit, Loop, Macro, Match, MethodCall, Paren,
        Path, Range, RawAddr, Reference, Repeat, Return, Struct, Try, TryBlock, Tuple, Unary,
        Unsafe, While, Yield,
    )
}

fn collect_ownership_aliases(tree: &syn::UseTree, aliases: &mut HashMap<String, String>) {
    match tree {
        syn::UseTree::Path(path) => collect_ownership_aliases(&path.tree, aliases),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_ownership_aliases(item, aliases);
            }
        }
        syn::UseTree::Rename(rename) => {
            let original = rename.ident.to_string();
            if matches!(
                original.as_str(),
                "Block"
                    | "Table"
                    | "Row"
                    | "Cell"
                    | "Clear"
                    | "Paragraph"
                    | "render_screen_frame"
                    | "render_scrollable_list_panel_spanned"
                    | "render_detail_panel_with_header"
                    | "render_status_line"
                    | "render_shortcuts"
            ) {
                aliases.insert(rename.rename.to_string(), original);
            }
        }
        syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
    }
}

struct LocalOwnershipAliasCollector<'a> {
    aliases: &'a mut HashMap<String, String>,
}

impl<'ast> Visit<'ast> for LocalOwnershipAliasCollector<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if let syn::Item::Use(item_use) = item {
            collect_ownership_aliases(&item_use.tree, self.aliases);
        }
    }
}

fn is_goals_framework_component(name: &str) -> bool {
    matches!(
        name,
        "render_screen_frame"
            | "render_scrollable_list_panel_spanned"
            | "render_detail_panel_with_header"
    )
}

fn is_goals_known_value_name(name: &str) -> bool {
    is_goals_framework_component(name) || matches!(name, "render_status_line" | "render_shortcuts")
}

fn is_goals_known_type_module_name(name: &str) -> bool {
    matches!(name, "Block" | "Clear" | "Paragraph" | "framework")
}

#[derive(Clone)]
enum GoalsCallResolution {
    Framework(Vec<String>),
    FrameworkModule(Vec<String>),
    Shadowed,
}

#[derive(Default)]
struct GoalsCallScope {
    value_bindings: HashMap<String, GoalsCallResolution>,
    type_module_bindings: HashMap<String, GoalsCallResolution>,
    ownership_value_shadows: HashSet<String>,
    canonical_framework_glob: bool,
}

impl GoalsCallScope {
    fn shadow_value(&mut self, name: String) {
        self.value_bindings
            .insert(name.clone(), GoalsCallResolution::Shadowed);
        self.ownership_value_shadows.insert(name);
    }

    fn shadow_type_module(&mut self, name: String) {
        self.type_module_bindings
            .insert(name, GoalsCallResolution::Shadowed);
    }
}

#[derive(Clone)]
enum GoalsOwnershipResolution {
    Origin(Vec<String>),
    Shadowed,
}

#[derive(Default)]
struct GoalsOwnershipScope {
    value_bindings: HashMap<String, GoalsOwnershipResolution>,
    type_module_bindings: HashMap<String, GoalsOwnershipResolution>,
}

fn insert_goals_ownership_resolution(
    scope: &mut GoalsOwnershipScope,
    local_name: String,
    origin: Vec<String>,
) {
    let original_name = origin.last().map(String::as_str);
    if original_name.is_some_and(is_goals_known_value_name) {
        scope
            .value_bindings
            .insert(local_name, GoalsOwnershipResolution::Origin(origin));
    } else if original_name.is_some_and(is_goals_known_type_module_name) {
        scope
            .type_module_bindings
            .insert(local_name, GoalsOwnershipResolution::Origin(origin));
    } else {
        scope.value_bindings.insert(
            local_name.clone(),
            GoalsOwnershipResolution::Origin(origin.clone()),
        );
        scope
            .type_module_bindings
            .insert(local_name, GoalsOwnershipResolution::Origin(origin));
    }
}

fn collect_goals_ownership_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    scope: &mut GoalsOwnershipScope,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_goals_ownership_imports(&path.tree, prefix, scope);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_goals_ownership_imports(item, prefix, scope);
            }
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(local_name) = prefix.last() {
                insert_goals_ownership_resolution(scope, local_name.clone(), prefix.clone());
            }
        }
        syn::UseTree::Name(name) => {
            let local_name = name.ident.to_string();
            let mut origin = prefix.clone();
            origin.push(local_name.clone());
            insert_goals_ownership_resolution(scope, local_name, origin);
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            if rename.ident != "self" {
                origin.push(rename.ident.to_string());
            }
            insert_goals_ownership_resolution(scope, rename.rename.to_string(), origin);
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn expanded_goals_ownership_path(path: &syn::Path, scopes: &[GoalsOwnershipScope]) -> Vec<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let Some(first) = segments.first().cloned() else {
        return segments;
    };
    for scope in scopes.iter().rev() {
        match scope.type_module_bindings.get(&first) {
            Some(GoalsOwnershipResolution::Origin(origin)) => {
                let mut expanded = origin.clone();
                expanded.extend(segments.into_iter().skip(1));
                return expanded;
            }
            Some(GoalsOwnershipResolution::Shadowed) => return Vec::new(),
            None => {}
        }
    }
    segments
}

fn expanded_goals_ownership_value_path(
    path: &syn::Path,
    ownership_scopes: &[GoalsOwnershipScope],
    call_scopes: &[GoalsCallScope],
) -> Vec<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let Some(first) = segments.first().cloned() else {
        return segments;
    };
    let qualified = segments.len() > 1;
    for (call_scope, ownership_scope) in call_scopes.iter().rev().zip(ownership_scopes.iter().rev())
    {
        if !qualified && call_scope.ownership_value_shadows.contains(&first) {
            return Vec::new();
        }
        let binding = if qualified {
            ownership_scope.type_module_bindings.get(&first)
        } else {
            ownership_scope.value_bindings.get(&first)
        };
        match binding {
            Some(GoalsOwnershipResolution::Origin(origin)) => {
                let mut expanded = origin.clone();
                expanded.extend(segments.iter().skip(1).cloned());
                return expanded;
            }
            Some(GoalsOwnershipResolution::Shadowed) => return Vec::new(),
            None => {}
        }
    }
    segments
}

fn canonical_goals_widget_type(path: &[String]) -> Option<&'static str> {
    match path {
        [name] => match name.as_str() {
            "Block" => Some("Block"),
            "Clear" => Some("Clear"),
            "Paragraph" => Some("Paragraph"),
            _ => None,
        },
        [prefix @ .., widgets, name]
            if widgets == "widgets"
                && prefix.iter().any(|segment| segment == "ratatui")
                && matches!(name.as_str(), "Block" | "Clear" | "Paragraph") =>
        {
            match name.as_str() {
                "Block" => Some("Block"),
                "Clear" => Some("Clear"),
                "Paragraph" => Some("Paragraph"),
                _ => None,
            }
        }
        _ => None,
    }
}

fn canonical_goals_forbidden_function(path: &[String]) -> Option<&'static str> {
    match path {
        [name] => match name.as_str() {
            "render_status_line" => Some("render_status_line"),
            "render_shortcuts" => Some("render_shortcuts"),
            _ => None,
        },
        [root, framework, name]
            if root == "super"
                && framework == "framework"
                && matches!(name.as_str(), "render_status_line" | "render_shortcuts") =>
        {
            match name.as_str() {
                "render_status_line" => Some("render_status_line"),
                "render_shortcuts" => Some("render_shortcuts"),
                _ => None,
            }
        }
        [root, tui, name]
            if matches!(root.as_str(), "crate" | "iron")
                && tui == "tui"
                && matches!(name.as_str(), "render_status_line" | "render_shortcuts") =>
        {
            match name.as_str() {
                "render_status_line" => Some("render_status_line"),
                "render_shortcuts" => Some("render_shortcuts"),
                _ => None,
            }
        }
        [root, tui, framework, name]
            if matches!(root.as_str(), "crate" | "iron")
                && tui == "tui"
                && framework == "framework"
                && matches!(name.as_str(), "render_status_line" | "render_shortcuts") =>
        {
            match name.as_str() {
                "render_status_line" => Some("render_status_line"),
                "render_shortcuts" => Some("render_shortcuts"),
                _ => None,
            }
        }
        _ => None,
    }
}

fn insert_goals_call_binding(
    bindings: &mut HashMap<String, GoalsCallResolution>,
    local_name: String,
    resolution: GoalsCallResolution,
) {
    bindings
        .entry(local_name)
        .and_modify(|existing| {
            if matches!(&resolution, GoalsCallResolution::Shadowed) {
                *existing = GoalsCallResolution::Shadowed;
            }
        })
        .or_insert(resolution);
}

fn insert_goals_call_resolution(
    scope: &mut GoalsCallScope,
    local_name: String,
    origin: Vec<String>,
) {
    if canonical_goals_framework_path(&origin) {
        insert_goals_call_binding(
            &mut scope.value_bindings,
            local_name,
            GoalsCallResolution::Framework(origin),
        );
    } else if canonical_goals_framework_module(&origin) {
        insert_goals_call_binding(
            &mut scope.type_module_bindings,
            local_name,
            GoalsCallResolution::FrameworkModule(origin),
        );
    } else {
        let original_name = origin.last().map(String::as_str);
        if !original_name.is_some_and(is_goals_known_type_module_name) {
            insert_goals_call_binding(
                &mut scope.value_bindings,
                local_name.clone(),
                GoalsCallResolution::Shadowed,
            );
        }
        if !original_name.is_some_and(is_goals_known_value_name) {
            insert_goals_call_binding(
                &mut scope.type_module_bindings,
                local_name,
                GoalsCallResolution::Shadowed,
            );
        }
    }
}

fn collect_goals_call_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    scope: &mut GoalsCallScope,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_goals_call_imports(&path.tree, prefix, scope);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_goals_call_imports(item, prefix, scope);
            }
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(local_name) = prefix.last() {
                insert_goals_call_resolution(scope, local_name.clone(), prefix.clone());
            }
        }
        syn::UseTree::Name(name) => {
            let local_name = name.ident.to_string();
            let mut origin = prefix.clone();
            origin.push(local_name.clone());
            insert_goals_call_resolution(scope, local_name, origin);
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            if rename.ident != "self" {
                origin.push(rename.ident.to_string());
            }
            insert_goals_call_resolution(scope, rename.rename.to_string(), origin);
        }
        syn::UseTree::Glob(_) => {
            scope.canonical_framework_glob |= canonical_goals_framework_module(prefix);
        }
    }
}

fn canonical_goals_framework_module(path: &[String]) -> bool {
    matches!(path, [root, framework] if root == "super" && framework == "framework")
        || matches!(path, [root, tui, framework]
            if matches!(root.as_str(), "crate" | "iron")
                && tui == "tui" && framework == "framework")
}

fn canonical_goals_framework_path(path: &[String]) -> bool {
    match path {
        [root, framework, component] => {
            root == "super" && framework == "framework" && is_goals_framework_component(component)
        }
        [root, tui, framework, component] => {
            matches!(root.as_str(), "crate" | "iron")
                && tui == "tui"
                && framework == "framework"
                && is_goals_framework_component(component)
        }
        _ => false,
    }
}

fn canonical_goals_framework_call(
    function: &syn::ExprPath,
    scopes: &[GoalsCallScope],
) -> Option<String> {
    if function.qself.is_some() {
        return None;
    }
    let path = function
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    if path.len() == 1 {
        let local_name = path.first()?;
        for scope in scopes.iter().rev() {
            if let Some(resolution) = scope.value_bindings.get(local_name) {
                return match resolution {
                    GoalsCallResolution::Framework(origin) => {
                        canonical_goals_framework_path(origin)
                            .then(|| origin.last().unwrap().clone())
                    }
                    GoalsCallResolution::FrameworkModule(_) => None,
                    GoalsCallResolution::Shadowed => None,
                };
            }
            if scope.canonical_framework_glob && is_goals_framework_component(local_name) {
                return Some(local_name.clone());
            }
        }
        return None;
    }
    if path.len() == 2 {
        let module_name = path.first()?;
        for scope in scopes.iter().rev() {
            if let Some(resolution) = scope.type_module_bindings.get(module_name) {
                return match resolution {
                    GoalsCallResolution::FrameworkModule(origin) => {
                        let mut expanded = origin.clone();
                        expanded.push(path.last()?.clone());
                        canonical_goals_framework_path(&expanded)
                            .then(|| expanded.last().unwrap().clone())
                    }
                    GoalsCallResolution::Framework(_) | GoalsCallResolution::Shadowed => None,
                };
            }
        }
        return None;
    }
    canonical_goals_framework_path(&path).then(|| path.last().unwrap().clone())
}

fn goals_type_module_item(item: &syn::Item) -> Option<(String, &[syn::Attribute])> {
    match item {
        syn::Item::Enum(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::ExternCrate(item) => Some((
            item.rename
                .as_ref()
                .map_or(&item.ident, |(_, rename)| rename)
                .to_string(),
            &item.attrs,
        )),
        syn::Item::Mod(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::Struct(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::Trait(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::TraitAlias(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::Type(item) => Some((item.ident.to_string(), &item.attrs)),
        syn::Item::Union(item) => Some((item.ident.to_string(), &item.attrs)),
        _ => None,
    }
}

fn direct_invoked_closure(expression: &syn::Expr) -> Option<&syn::ExprClosure> {
    match expression {
        syn::Expr::Closure(closure) => Some(closure),
        syn::Expr::Group(group) => direct_invoked_closure(&group.expr),
        syn::Expr::Paren(paren) => direct_invoked_closure(&paren.expr),
        _ => None,
    }
}

#[derive(Default)]
struct GoalsPatternBindingCollector {
    names: HashSet<String>,
}

impl<'ast> Visit<'ast> for GoalsPatternBindingCollector {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.names.insert(pattern.ident.to_string());
        visit::visit_pat_ident(self, pattern);
    }
}

fn type_path_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Group(group) => type_path_name(&group.elem),
        syn::Type::Paren(paren) => type_path_name(&paren.elem),
        syn::Type::Path(path) => Some(path.path.segments.last()?.ident.to_string()),
        syn::Type::Reference(reference) => type_path_name(&reference.elem),
        _ => None,
    }
}

fn called_type_and_method(call: &syn::ExprPath) -> Option<(String, String)> {
    let method = call.path.segments.last()?.ident.to_string();
    let receiver = if let Some(qself) = &call.qself {
        type_path_name(&qself.ty)?
    } else if call.path.segments.len() >= 2 {
        call.path
            .segments
            .iter()
            .nth(call.path.segments.len() - 2)?
            .ident
            .to_string()
    } else {
        return None;
    };
    Some((receiver, method))
}

#[derive(Default)]
struct HistoryRenderOwnership {
    screen_frame_calls: usize,
    list_panel_calls: usize,
    detail_panel_calls: usize,
    borders_method_calls: usize,
    block_bordered_calls: usize,
    table_new_calls: usize,
    row_new_calls: usize,
    cell_from_calls: usize,
    alias_uses: BTreeSet<(String, String)>,
}

struct HistoryRenderVisitor<'a> {
    aliases: &'a HashMap<String, String>,
    ownership: HistoryRenderOwnership,
}

impl HistoryRenderVisitor<'_> {
    fn record_alias_use(&mut self, path: &syn::Path) {
        let Some(first) = path.segments.first() else {
            return;
        };
        let alias = first.ident.to_string();
        if let Some(original) = self
            .aliases
            .get(&alias)
            .filter(|original| matches!(original.as_str(), "Block" | "Table" | "Row" | "Cell"))
        {
            self.ownership.alias_uses.insert((original.clone(), alias));
        }
    }
}

impl<'ast> Visit<'ast> for HistoryRenderVisitor<'_> {
    fn visit_item(&mut self, _item: &'ast syn::Item) {
        // Nested items are declarations, not calls made by HistoryScreen::render.
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            match function
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .as_deref()
            {
                Some("render_screen_frame_sections") => self.ownership.screen_frame_calls += 1,
                Some("render_scrollable_list_panel") => self.ownership.list_panel_calls += 1,
                Some("render_detail_panel") => self.ownership.detail_panel_calls += 1,
                _ => {}
            }

            if let Some((receiver, method)) = called_type_and_method(function) {
                let original = self
                    .aliases
                    .get(&receiver)
                    .map(String::as_str)
                    .unwrap_or(receiver.as_str());
                match (original, method.as_str()) {
                    ("Block", "bordered") => self.ownership.block_bordered_calls += 1,
                    ("Table", "new") => self.ownership.table_new_calls += 1,
                    ("Row", "new") => self.ownership.row_new_calls += 1,
                    ("Cell", "from") => self.ownership.cell_from_calls += 1,
                    _ => {}
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "borders" {
            self.ownership.borders_method_calls += 1;
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.record_alias_use(&path.path);
        visit::visit_expr_path(self, path);
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.record_alias_use(&path.path);
        visit::visit_type_path(self, path);
    }
}

fn history_ownership_violations(source: &str) -> Vec<String> {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => return vec![format!("History source must parse as Rust: {error}")],
    };
    let render_methods = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_none()
                    && !is_cfg_test(&item_impl.attrs)
                    && is_history_screen_type(item_impl.self_ty.as_ref()) =>
            {
                Some(item_impl)
            }
            _ => None,
        })
        .flat_map(|item_impl| &item_impl.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method)
                if method.sig.ident == "render" && !is_cfg_test(&method.attrs) =>
            {
                Some(method)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    if render_methods.len() != 1 {
        return vec![format!(
            "History must define exactly one production inherent HistoryScreen::render method; found {}",
            render_methods.len()
        )];
    }
    let render = render_methods[0];
    let mut aliases = HashMap::new();
    for item in &file.items {
        if let syn::Item::Use(item_use) = item {
            collect_ownership_aliases(&item_use.tree, &mut aliases);
        }
    }
    LocalOwnershipAliasCollector {
        aliases: &mut aliases,
    }
    .visit_block(&render.block);

    let mut visitor = HistoryRenderVisitor {
        aliases: &aliases,
        ownership: HistoryRenderOwnership::default(),
    };
    visitor.visit_block(&render.block);
    let ownership = visitor.ownership;
    let mut violations = Vec::new();
    for (count, component) in [
        (
            ownership.screen_frame_calls,
            "ScreenFrame (render_screen_frame_sections)",
        ),
        (
            ownership.list_panel_calls,
            "ScrollableListPanel (render_scrollable_list_panel)",
        ),
        (ownership.detail_panel_calls, "render_detail_panel"),
    ] {
        if count != 1 {
            violations.push(format!(
                "HistoryScreen::render must call {component} exactly once; found {count}"
            ));
        }
    }
    if ownership.borders_method_calls != 0 {
        violations.push(format!(
            "HistoryScreen::render must not own a local border through .borders(...); found {}",
            ownership.borders_method_calls
        ));
    }
    if ownership.block_bordered_calls != 0 {
        violations.push(format!(
            "HistoryScreen::render must not own a local border through Block::bordered(); found {}",
            ownership.block_bordered_calls
        ));
    }
    for (count, constructor) in [
        (ownership.table_new_calls, "Table::new"),
        (ownership.row_new_calls, "Row::new"),
        (ownership.cell_from_calls, "Cell::from"),
    ] {
        if count != 0 {
            violations.push(format!(
                "HistoryScreen::render must not own local table cells through {constructor}; found {count}"
            ));
        }
    }
    for (original, alias) in ownership.alias_uses {
        violations.push(format!(
            "HistoryScreen::render must not use aliased {original} `{alias}` for local presentation ownership"
        ));
    }
    violations
}

#[derive(Default)]
struct GoalsRenderOwnership {
    screen_frame_calls: usize,
    list_panel_calls: usize,
    detail_panel_calls: usize,
    borders_method_calls: usize,
    block_bordered_calls: usize,
    status_line_calls: usize,
    shortcuts_calls: usize,
    clear_paths: usize,
    paragraph_new_calls: usize,
    called_methods: BTreeSet<String>,
}

struct GoalsRenderVisitor<'a> {
    call_scopes: Vec<GoalsCallScope>,
    ownership_scopes: Vec<GoalsOwnershipScope>,
    ownership: GoalsRenderOwnership,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl GoalsRenderVisitor<'_> {
    fn block_call_scope(&self, block: &syn::Block) -> GoalsCallScope {
        let mut scope = GoalsCallScope::default();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            if let Some((name, attrs)) = goals_type_module_item(item) {
                if !is_cfg_test(attrs) {
                    scope.shadow_type_module(name);
                }
            }
            match item {
                syn::Item::Const(item) if !is_cfg_test(&item.attrs) => {
                    scope.shadow_value(item.ident.to_string());
                }
                syn::Item::Fn(item) if !is_cfg_test(&item.attrs) => {
                    scope.shadow_value(item.sig.ident.to_string());
                }
                syn::Item::Static(item) if !is_cfg_test(&item.attrs) => {
                    scope.shadow_value(item.ident.to_string());
                }
                syn::Item::Use(item) if !is_cfg_test(&item.attrs) => {
                    collect_goals_call_imports(&item.tree, &mut Vec::new(), &mut scope)
                }
                _ => {}
            }
        }
        scope
    }

    fn block_ownership_scope(&self, block: &syn::Block) -> GoalsOwnershipScope {
        let mut scope = GoalsOwnershipScope::default();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            if let Some((name, attrs)) = goals_type_module_item(item) {
                if !is_cfg_test(attrs) {
                    scope
                        .type_module_bindings
                        .insert(name, GoalsOwnershipResolution::Shadowed);
                }
            }
            match item {
                syn::Item::Use(item) if !is_cfg_test(&item.attrs) => {
                    collect_goals_ownership_imports(&item.tree, &mut Vec::new(), &mut scope)
                }
                _ => {}
            }
        }
        scope
    }

    fn resolved_widget_type(&self, path: &syn::Path) -> Option<&'static str> {
        let expanded = expanded_goals_ownership_path(path, &self.ownership_scopes);
        canonical_goals_widget_type(&expanded)
    }

    fn resolved_receiver_type(&self, path: &syn::ExprPath) -> Option<&'static str> {
        if let Some(qself) = &path.qself {
            return self.resolved_type(&qself.ty);
        }
        if path.path.segments.len() < 2 {
            return None;
        }
        let mut receiver = path.path.clone();
        receiver.segments.pop();
        self.resolved_widget_type(&receiver)
    }

    fn resolved_type(&self, ty: &syn::Type) -> Option<&'static str> {
        match ty {
            syn::Type::Group(group) => self.resolved_type(&group.elem),
            syn::Type::Paren(paren) => self.resolved_type(&paren.elem),
            syn::Type::Path(path) => self.resolved_widget_type(&path.path),
            syn::Type::Reference(reference) => self.resolved_type(&reference.elem),
            _ => None,
        }
    }

    fn shadow_pattern_bindings(&mut self, pattern: &syn::Pat) {
        let mut bindings = GoalsPatternBindingCollector::default();
        bindings.visit_pat(pattern);
        let scope = self.call_scopes.last_mut().unwrap();
        for name in bindings.names {
            scope.shadow_value(name);
        }
    }

    fn visit_condition_with_let_bindings(&mut self, expression: &syn::Expr) {
        match expression {
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                self.visit_condition_with_let_bindings(&binary.left);
                self.visit_condition_with_let_bindings(&binary.right);
            }
            syn::Expr::Group(group) => self.visit_condition_with_let_bindings(&group.expr),
            syn::Expr::Let(expression) => {
                self.visit_expr(&expression.expr);
                self.shadow_pattern_bindings(&expression.pat);
            }
            syn::Expr::Paren(paren) => self.visit_condition_with_let_bindings(&paren.expr),
            _ => self.visit_expr(expression),
        }
    }
}

impl<'ast> Visit<'ast> for GoalsRenderVisitor<'_> {
    fn visit_item(&mut self, _item: &'ast syn::Item) {
        // Nested item declarations are not calls made by GoalsScreen::render.
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.call_scopes.push(self.block_call_scope(block));
        self.ownership_scopes
            .push(self.block_ownership_scope(block));
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.ownership_scopes.pop();
        self.call_scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(initializer) = &local.init {
            self.visit_expr(&initializer.expr);
            if let Some((_, diverge)) = &initializer.diverge {
                self.visit_expr(diverge);
            }
        }
        self.visit_pat(&local.pat);
        self.shadow_pattern_bindings(&local.pat);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let _ = closure;
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.call_scopes.push(GoalsCallScope::default());
        self.ownership_scopes.push(GoalsOwnershipScope::default());
        self.shadow_pattern_bindings(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.ownership_scopes.pop();
        self.call_scopes.pop();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.call_scopes.push(GoalsCallScope::default());
        self.ownership_scopes.push(GoalsOwnershipScope::default());
        self.shadow_pattern_bindings(&expression.pat);
        self.visit_block(&expression.body);
        self.ownership_scopes.pop();
        self.call_scopes.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.call_scopes.push(GoalsCallScope::default());
        self.ownership_scopes.push(GoalsOwnershipScope::default());
        self.visit_condition_with_let_bindings(&expression.cond);
        self.visit_block(&expression.then_branch);
        self.ownership_scopes.pop();
        self.call_scopes.pop();
        if let Some((_, else_branch)) = &expression.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.call_scopes.push(GoalsCallScope::default());
        self.ownership_scopes.push(GoalsOwnershipScope::default());
        self.visit_condition_with_let_bindings(&expression.cond);
        self.visit_block(&expression.body);
        self.ownership_scopes.pop();
        self.call_scopes.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Some(closure) = direct_invoked_closure(&call.func) {
            self.call_scopes.push(GoalsCallScope::default());
            self.ownership_scopes.push(GoalsOwnershipScope::default());
            for input in &closure.inputs {
                self.shadow_pattern_bindings(input);
            }
            self.visit_expr(&closure.body);
            self.ownership_scopes.pop();
            self.call_scopes.pop();
            for argument in &call.args {
                self.visit_expr(argument);
            }
            return;
        }
        if let syn::Expr::Path(function) = call.func.as_ref() {
            match canonical_goals_framework_call(function, &self.call_scopes).as_deref() {
                Some("render_screen_frame") => self.ownership.screen_frame_calls += 1,
                Some("render_scrollable_list_panel_spanned") => {
                    self.ownership.list_panel_calls += 1;
                }
                Some("render_detail_panel_with_header") => self.ownership.detail_panel_calls += 1,
                _ => {}
            }
            let segments = function
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            if let [prefix @ .., method] = segments.as_slice() {
                if prefix
                    .last()
                    .is_some_and(|owner| matches!(owner.as_str(), "Self" | "GoalsScreen"))
                {
                    self.ownership.called_methods.insert(method.clone());
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "borders" {
            self.ownership.borders_method_calls += 1;
        }
        if matches!(call.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) {
            self.ownership
                .called_methods
                .insert(call.method.to_string());
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        let expanded = expanded_goals_ownership_path(&path.path, &self.ownership_scopes);
        let expanded_value = expanded_goals_ownership_value_path(
            &path.path,
            &self.ownership_scopes,
            &self.call_scopes,
        );
        match canonical_goals_forbidden_function(&expanded_value) {
            Some("render_status_line") => self.ownership.status_line_calls += 1,
            Some("render_shortcuts") => self.ownership.shortcuts_calls += 1,
            _ => {}
        }
        let direct_clear = canonical_goals_widget_type(&expanded) == Some("Clear");
        let receiver = self.resolved_receiver_type(path);
        if direct_clear || receiver == Some("Clear") {
            self.ownership.clear_paths += 1;
        }
        if let Some(method) = path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        {
            match (receiver, method.as_str()) {
                (Some("Block"), "bordered") => self.ownership.block_bordered_calls += 1,
                (Some("Paragraph"), "new") => self.ownership.paragraph_new_calls += 1,
                (Some("Block"), "borders") => self.ownership.borders_method_calls += 1,
                _ => {}
            }
        }
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if self.resolved_widget_type(&expression.path) == Some("Clear") {
            self.ownership.clear_paths += 1;
        }
        visit::visit_expr_struct(self, expression);
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if self.resolved_widget_type(&path.path) == Some("Clear") {
            self.ownership.clear_paths += 1;
        }
        visit::visit_type_path(self, path);
    }
}

fn goals_ownership_violations(source: &str) -> Vec<String> {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => return vec![format!("Goals source must parse as Rust: {error}")],
    };
    let production_impls = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_none()
                    && !is_cfg_test(&item_impl.attrs)
                    && is_goals_screen_type(item_impl.self_ty.as_ref()) =>
            {
                Some(item_impl)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let methods = production_impls
        .iter()
        .flat_map(|item_impl| &item_impl.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) if !is_cfg_test(&method.attrs) => {
                Some((method.sig.ident.to_string(), method))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let render_methods = production_impls
        .iter()
        .flat_map(|item_impl| &item_impl.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method)
                if method.sig.ident == "render" && !is_cfg_test(&method.attrs) =>
            {
                Some(method)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    if render_methods.len() != 1 {
        return vec![format!(
            "Goals must define exactly one production inherent GoalsScreen::render method; found {}",
            render_methods.len()
        )];
    }
    let render = render_methods[0];
    let mut root_call_scope = GoalsCallScope::default();
    let mut root_ownership_scope = GoalsOwnershipScope::default();
    for item in &file.items {
        if let Some((name, attrs)) = goals_type_module_item(item) {
            if !is_cfg_test(attrs) {
                root_call_scope.shadow_type_module(name.clone());
                root_ownership_scope
                    .type_module_bindings
                    .insert(name, GoalsOwnershipResolution::Shadowed);
            }
        }
        match item {
            syn::Item::Const(item) if !is_cfg_test(&item.attrs) => {
                root_call_scope.shadow_value(item.ident.to_string());
            }
            syn::Item::Fn(item) if !is_cfg_test(&item.attrs) => {
                root_call_scope.shadow_value(item.sig.ident.to_string());
            }
            syn::Item::Static(item) if !is_cfg_test(&item.attrs) => {
                root_call_scope.shadow_value(item.ident.to_string());
            }
            syn::Item::Use(item_use) if !is_cfg_test(&item_use.attrs) => {
                collect_goals_call_imports(&item_use.tree, &mut Vec::new(), &mut root_call_scope);
                collect_goals_ownership_imports(
                    &item_use.tree,
                    &mut Vec::new(),
                    &mut root_ownership_scope,
                );
            }
            _ => {}
        }
    }
    let mut visitor = GoalsRenderVisitor {
        call_scopes: vec![root_call_scope],
        ownership_scopes: vec![root_ownership_scope],
        ownership: GoalsRenderOwnership::default(),
        _marker: std::marker::PhantomData,
    };
    let mut pending = vec![render.sig.ident.to_string()];
    let mut visited = HashSet::new();
    while let Some(method_name) = pending.pop() {
        if !visited.insert(method_name.clone()) {
            continue;
        }
        let Some(method) = methods.get(&method_name) else {
            continue;
        };
        let mut signature_scope = GoalsCallScope::default();
        for input in &method.sig.inputs {
            if let syn::FnArg::Typed(input) = input {
                let mut bindings = GoalsPatternBindingCollector::default();
                bindings.visit_pat(&input.pat);
                for name in bindings.names {
                    signature_scope.shadow_value(name);
                }
            }
        }
        visitor.call_scopes.push(signature_scope);
        visitor
            .ownership_scopes
            .push(GoalsOwnershipScope::default());
        visitor.visit_block(&method.block);
        visitor.ownership_scopes.pop();
        visitor.call_scopes.pop();

        pending.extend(
            visitor
                .ownership
                .called_methods
                .iter()
                .filter(|called| methods.contains_key(*called) && !visited.contains(*called))
                .cloned(),
        );
    }
    let ownership = visitor.ownership;
    let mut violations = Vec::new();
    for (count, component) in [
        (ownership.screen_frame_calls, "render_screen_frame"),
        (
            ownership.list_panel_calls,
            "render_scrollable_list_panel_spanned",
        ),
        (
            ownership.detail_panel_calls,
            "render_detail_panel_with_header",
        ),
    ] {
        if count != 1 {
            violations.push(format!(
                "GoalsScreen::render must call {component} exactly once; found {count}"
            ));
        }
    }
    for (count, forbidden) in [
        (ownership.borders_method_calls, ".borders"),
        (ownership.block_bordered_calls, "Block::bordered"),
        (ownership.status_line_calls, "render_status_line"),
        (ownership.shortcuts_calls, "render_shortcuts"),
        (ownership.clear_paths, "Clear"),
        (ownership.paragraph_new_calls, "Paragraph::new"),
    ] {
        if count != 0 {
            violations.push(format!(
                "GoalsScreen::render must not own local presentation through {forbidden}; found {count}"
            ));
        }
    }
    violations
}

fn rust_sources(root: &Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .unwrap_or_else(|_| panic!("failed to read {}", directory.display()))
        {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

fn paragraph_violations(root: &Path, allowlist: &HashSet<&str>) -> Vec<String> {
    let mut violations = Vec::new();
    for path in rust_sources(root) {
        let relative = path.strip_prefix(root).unwrap().to_string_lossy();
        let source = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        for (line_num_0, line) in lines.iter().enumerate() {
            if !line.trim().contains("Paragraph::new(") {
                continue;
            }
            let end = (line_num_0 + 4).min(lines.len());
            if lines[line_num_0..end].join(" ").contains(".wrap(") {
                continue;
            }
            let line_num = line_num_0 + 1;
            let key = format!("{}:{}", relative, line_num);
            if !allowlist.contains(key.as_str()) {
                violations.push(format!(
                    "  {}:{} — Paragraph::new() without .wrap(). Add .wrap(Wrap {{ trim: false }}) or allowlist with reason.",
                    relative, line_num
                ));
            }
        }
    }
    violations
}

#[test]
fn paragraph_lint_scans_nested_framework_sources() {
    let temp = tempfile::tempdir().unwrap();
    let nested = temp.path().join("framework");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        nested.join("bad_panel.rs"),
        "fn render() { Paragraph::new(variable_content); }\n",
    )
    .unwrap();

    let violations = paragraph_violations(temp.path(), &HashSet::new());

    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("framework/bad_panel.rs:1")),
        "{violations:?}"
    );
}

#[test]
fn ownership_source_check_rejects_non_code_evidence() {
    let comments_only = r#"
        // render_scrollable_list_panel(
        /* render_scrollable_list_panel( */
    "#;
    let strings_only = r###"
        const ORDINARY: &str = "render_scrollable_list_panel(";
        const RAW: &str = r#"render_scrollable_list_panel("#;
    "###;

    assert!(!source_code_contains(
        comments_only,
        "render_scrollable_list_panel("
    ));
    assert!(!source_code_contains(
        strings_only,
        "render_scrollable_list_panel("
    ));
    assert!(!source_code_contains("const MARKER: char = '(';", "("));
    assert!(source_code_contains(
        "render_scrollable_list_panel(frame, area);",
        "render_scrollable_list_panel("
    ));
    assert!(source_code_contains(
        "fn render<'a>(frame: &'a mut Frame) { render_scrollable_list_panel(frame, area); }",
        "render_scrollable_list_panel("
    ));
}

#[test]
fn dashboard_delegates_static_presentation_to_framework() {
    let source = std::fs::read_to_string("src/tui/dashboard.rs").unwrap();
    let violations = dashboard_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn dashboard_production_uses_only_the_local_date_heatmap_constructor() {
    let source = std::fs::read_to_string("src/tui/dashboard.rs").unwrap();
    let violations = dashboard_heatmap_construction_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn dashboard_ownership_ignores_nonproduction_decoys() {
    let source = r#"
        fn render() {
            // render_screen_frame_sections(); render_static_panel();
            let _text = "render_static_panel(";
        }
        #[cfg(test)]
        mod tests {
            fn decoy() {
                render_screen_frame_sections();
                render_static_panel(); render_static_panel();
                render_static_panel(); render_static_panel();
                Block::default().borders(Borders::ALL);
            }
        }
    "#;
    let violations = dashboard_ownership_violations(source);
    assert!(violations
        .iter()
        .any(|item| item.contains("ScreenFrameSections")));
    assert!(violations.iter().any(|item| item.contains("StaticPanel")));
    assert!(!violations.iter().any(|item| item.contains("Borders::ALL")));
}

#[test]
fn log_entry_selector_delegates_shared_presentation() {
    let source = std::fs::read_to_string("src/tui/log_entry.rs").unwrap();
    let violations = log_entry_selector_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn log_entry_selector_ownership_ignores_comments_strings_and_tests() {
    let source = r###"
        fn render_select_practice(&mut self) {
            // render_screen_frame_sections(); render_scrollable_list_panel();
            let _text = r#"render_scrollable_list_panel("#;
        }
        fn handle_select_practice(&mut self) {}
        #[cfg(test)]
        fn decoy() {
            render_screen_frame_sections();
            render_scrollable_list_panel();
        }
    "###;
    let violations = log_entry_selector_ownership_violations(source);
    assert!(violations
        .iter()
        .any(|item| item.contains("ScreenFrameSections")));
    assert!(violations
        .iter()
        .any(|item| item.contains("ScrollableListPanel")));
}

#[test]
fn log_entry_delegates_all_presentation_to_framework() {
    let source = std::fs::read_to_string("src/tui/log_entry.rs").unwrap();
    let violations = log_entry_ownership_violations(&source);
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn log_entry_ownership_rejects_wrong_origin_dead_and_test_decoys() {
    let source = r#"
        use wrong::framework::render_bounded_panel;
        use super::framework::{render_screen_frame_sections, render_scrollable_list_panel};
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel();
                Block::default().borders(Borders::ALL);
            }
            fn dead_helper(&self) {
                render_bounded_panel(); render_bounded_panel(); render_bounded_panel();
            }
            #[cfg(test)]
            fn test_decoy(&self) {
                render_bounded_panel(); render_bounded_panel(); render_bounded_panel();
            }
        }
    "#;
    let violations = log_entry_ownership_violations(source);
    assert!(violations.iter().any(|item| item.contains("wrong-origin")));
    assert!(violations
        .iter()
        .any(|item| item.contains("BoundedPanel") && item.contains("found 0")));
    assert!(violations.iter().any(|item| item.contains("local Block")));
}

#[test]
fn log_entry_ownership_accepts_only_the_exact_reachable_component_path() {
    let source = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
            }
        }
    "#;
    assert!(log_entry_ownership_violations(source).is_empty());
}

#[test]
fn log_entry_ownership_rejects_alias_glob_ufcs_and_nested_decoys() {
    for (label, imports, call) in [
        (
            "alias",
            "use super::framework::{render_bounded_panel as panel, render_screen_frame_sections, render_scrollable_list_panel};",
            "panel(); panel(); panel(); panel();",
        ),
        (
            "glob",
            "use super::framework::*;",
            "render_bounded_panel(); render_bounded_panel(); render_bounded_panel(); render_bounded_panel();",
        ),
        (
            "UFCS",
            "use super::framework::{render_bounded_panel, render_screen_frame_sections, render_scrollable_list_panel};",
            "<Decoy as Components>::render_bounded_panel(); render_bounded_panel(); render_bounded_panel(); render_bounded_panel();",
        ),
    ] {
        let source = format!(
            r#"
                {imports}
                struct LogEntryScreen;
                impl LogEntryScreen {{
                    fn render(&mut self) {{
                        self.render_select_practice();
                        self.render_enter_log();
                        fn nested() {{
                            render_bounded_panel(); render_bounded_panel();
                            Block::bordered();
                        }}
                        let _text = "render_bounded_panel(); Block::default()";
                    }}
                    fn render_select_practice(&mut self) {{
                        render_screen_frame_sections();
                        render_scrollable_list_panel();
                    }}
                    fn render_enter_log(&mut self) {{
                        render_screen_frame_sections();
                        {call}
                    }}
                    #[cfg(test)]
                    fn test_decoy(&self) {{
                        render_bounded_panel(); Block::default().borders(Borders::ALL);
                    }}
                }}
            "#
        );
        let violations = log_entry_ownership_violations(&source);
        assert!(
            violations
                .iter()
                .any(|item| item.contains("wrong-origin")),
            "{label}: {violations:?}"
        );
        assert!(
            !violations.iter().any(|item| item.contains("local Block")),
            "nested/test Block decoys must be ignored for {label}: {violations:?}"
        );
    }
}

#[test]
fn log_entry_ownership_rejects_uninvoked_closure_and_root_shadow_decoys() {
    let uninvoked_closure = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                let _dead = || {
                    render_bounded_panel(); render_bounded_panel();
                    render_bounded_panel(); render_bounded_panel();
                };
            }
        }
    "#;
    let violations = log_entry_ownership_violations(uninvoked_closure);
    assert!(
        violations
            .iter()
            .any(|item| item.contains("BoundedPanel") && item.contains("found 0")),
        "{violations:?}"
    );

    let root_shadow_before_import = r#"
        fn render_bounded_panel() {}
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
            }
        }
    "#;
    let violations = log_entry_ownership_violations(root_shadow_before_import);
    assert!(
        violations.iter().any(|item| item.contains("wrong-origin")),
        "{violations:?}"
    );
}

#[test]
fn log_entry_ownership_rejects_aliased_ufcs_block_ownership() {
    let source = r#"
        use ratatui::widgets::{Block as Panel, Borders as Edges};
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
                <Panel as Default>::default().borders(Edges::ALL);
            }
        }
    "#;
    let violations = log_entry_ownership_violations(source);
    assert!(
        violations.iter().any(|item| item.contains("local Block")),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|item| item.contains(".borders")),
        "{violations:?}"
    );
}

#[test]
fn log_entry_ownership_excludes_pure_test_statements_but_keeps_production_cfg() {
    let test_only = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                #[cfg(test)]
                render_screen_frame_sections();
                #[cfg(test)]
                let _list = render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                #[cfg(test)]
                render_screen_frame_sections();
                #[cfg(test)]
                let _one = render_bounded_panel();
                #[cfg(test)]
                let _two = render_bounded_panel();
                #[cfg(test)]
                let _three = render_bounded_panel();
                #[cfg(test)]
                let _four = render_bounded_panel();
            }
        }
    "#;
    let violations = log_entry_ownership_violations(test_only);
    for component in ["ScreenFrameSections", "ScrollableListPanel", "BoundedPanel"] {
        assert!(
            violations
                .iter()
                .any(|item| item.contains(component) && item.contains("found 0")),
            "{component}: {violations:?}"
        );
    }

    for cfg in ["not(test)", "any(test, feature = \"production\")"] {
        let production_relevant = format!(
            r#"
                use super::framework::{{
                    render_bounded_panel,
                    render_screen_frame_sections,
                    render_scrollable_list_panel,
                }};
                struct LogEntryScreen;
                impl LogEntryScreen {{
                    fn render(&mut self) {{
                        self.render_select_practice();
                        self.render_enter_log();
                    }}
                    fn render_select_practice(&mut self) {{
                        #[cfg({cfg})]
                        render_screen_frame_sections();
                        #[cfg({cfg})]
                        let _list = render_scrollable_list_panel();
                    }}
                    fn render_enter_log(&mut self) {{
                        #[cfg({cfg})]
                        render_screen_frame_sections();
                        #[cfg({cfg})]
                        let _one = render_bounded_panel();
                        #[cfg({cfg})]
                        let _two = render_bounded_panel();
                        #[cfg({cfg})]
                        let _three = render_bounded_panel();
                        #[cfg({cfg})]
                        let _four = render_bounded_panel();
                    }}
                }}
            "#
        );
        assert!(
            log_entry_ownership_violations(&production_relevant).is_empty(),
            "cfg({cfg}) must remain production-relevant"
        );
    }
}

#[test]
fn log_entry_ownership_checks_invoked_nested_functions_and_named_closures() {
    for (label, declaration, invocation) in [
        (
            "nested function",
            "fn forbidden() { Block::default().borders(Borders::ALL); }",
            "forbidden();",
        ),
        (
            "named closure",
            "let forbidden = || { Block::default().borders(Borders::ALL); };",
            "forbidden();",
        ),
    ] {
        let source = format!(
            r#"
                use super::framework::{{
                    render_bounded_panel,
                    render_screen_frame_sections,
                    render_scrollable_list_panel,
                }};
                struct LogEntryScreen;
                impl LogEntryScreen {{
                    fn render(&mut self) {{
                        self.render_select_practice();
                        self.render_enter_log();
                    }}
                    fn render_select_practice(&mut self) {{
                        render_screen_frame_sections();
                        render_scrollable_list_panel();
                    }}
                    fn render_enter_log(&mut self) {{
                        render_screen_frame_sections();
                        render_bounded_panel(); render_bounded_panel();
                        render_bounded_panel(); render_bounded_panel();
                        {declaration}
                        {invocation}
                    }}
                }}
            "#
        );
        let violations = log_entry_ownership_violations(&source);
        assert!(
            violations.iter().any(|item| item.contains("local Block")),
            "{label}: {violations:?}"
        );
        assert!(
            violations.iter().any(|item| item.contains(".borders")),
            "{label}: {violations:?}"
        );
    }

    let uninvoked = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
                fn dead_function() { Block::default().borders(Borders::ALL); }
                let _dead_closure = || { Block::default().borders(Borders::ALL); };
            }
        }
    "#;
    assert!(log_entry_ownership_violations(uninvoked).is_empty());
}

#[test]
fn log_entry_ownership_checks_invoked_typed_named_closures() {
    let source = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
                let forbidden: fn() = || {
                    Block::default().borders(Borders::ALL);
                };
                forbidden();
            }
        }
    "#;
    let violations = log_entry_ownership_violations(source);
    assert!(
        violations.iter().any(|item| item.contains("local Block")),
        "typed named closure: {violations:?}"
    );
    assert!(
        violations.iter().any(|item| item.contains(".borders")),
        "typed named closure: {violations:?}"
    );
}

#[test]
fn log_entry_ownership_resolves_block_inner_and_type_aliases_precisely() {
    for (label, root_declaration, local_ownership) in [
        (
            "method-form inner",
            "fn obtain_block() -> Block<'static> { todo!() }",
            "let panel: Block<'static> = obtain_block(); panel.inner(area);",
        ),
        (
            "root type alias",
            "type Panel<'a> = Block<'a>;",
            "Panel::default();",
        ),
        (
            "local type alias",
            "",
            "type Panel<'a> = Block<'a>; Panel::bordered();",
        ),
    ] {
        let source = format!(
            r#"
                use super::framework::{{
                    render_bounded_panel,
                    render_screen_frame_sections,
                    render_scrollable_list_panel,
                }};
                use ratatui::widgets::Block;
                {root_declaration}
                struct LogEntryScreen;
                impl LogEntryScreen {{
                    fn render(&mut self) {{
                        self.render_select_practice();
                        self.render_enter_log();
                    }}
                    fn render_select_practice(&mut self) {{
                        render_screen_frame_sections();
                        render_scrollable_list_panel();
                    }}
                    fn render_enter_log(&mut self) {{
                        render_screen_frame_sections();
                        render_bounded_panel(); render_bounded_panel();
                        render_bounded_panel(); render_bounded_panel();
                        {local_ownership}
                    }}
                }}
            "#
        );
        let violations = log_entry_ownership_violations(&source);
        assert!(
            violations.iter().any(|item| item.contains("local Block")),
            "{label}: {violations:?}"
        );
    }

    let unrelated_inner = r#"
        use super::framework::{
            render_bounded_panel,
            render_screen_frame_sections,
            render_scrollable_list_panel,
        };
        struct Domain;
        struct LogEntryScreen;
        impl LogEntryScreen {
            fn render(&mut self) {
                self.render_select_practice();
                self.render_enter_log();
            }
            fn render_select_practice(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
            fn render_enter_log(&mut self) {
                render_screen_frame_sections();
                render_bounded_panel(); render_bounded_panel();
                render_bounded_panel(); render_bounded_panel();
                let domain: Domain = todo!();
                domain.inner(area);
            }
        }
    "#;
    assert!(log_entry_ownership_violations(unrelated_inner).is_empty());
}

#[test]
fn history_ownership_rejects_duplicate_shared_calls_and_local_border() {
    let mixed_ownership = r###"
        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_screen_frame_sections();
                render_scrollable_list_panel();
                render_scrollable_list_panel();
                render_detail_panel();
                render_detail_panel();
                Block::default().borders(Borders::ALL);
            }
        }
        // render_screen_frame_sections( render_scrollable_list_panel( .borders(Borders::ALL)
        const DECOYS: &str = r#"render_screen_frame_sections( render_scrollable_list_panel( .borders(Borders::ALL)"#;
    "###;

    // The old presence-only assertions accepted this mixed ownership.
    assert!(source_code_contains(
        mixed_ownership,
        "render_screen_frame_sections("
    ));
    assert!(source_code_contains(
        mixed_ownership,
        "render_scrollable_list_panel("
    ));

    let violations = history_ownership_violations(mixed_ownership);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("ScreenFrame") && violation.contains("found 2")),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|violation| {
            violation.contains("ScrollableListPanel") && violation.contains("found 2")
        }),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("render_detail_panel") && violation.contains("2")),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("local") && violation.contains("border")),
        "{violations:?}"
    );
}

#[test]
fn history_ownership_requires_calls_in_production_render_not_decoys() {
    let decoy_delegation = r#"
        impl HistoryScreen {
            fn render(&mut self) {
                render_detail_panel();
            }
        }

        fn dead_helper() {
            render_screen_frame_sections();
        }

        #[cfg(test)]
        mod tests {
            fn test_only_helper() {
                render_scrollable_list_panel();
            }
        }
    "#;

    let violations = history_ownership_violations(decoy_delegation);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("ScreenFrame") && violation.contains("found 0")),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|violation| {
            violation.contains("ScrollableListPanel") && violation.contains("found 0")
        }),
        "{violations:?}"
    );
}

#[test]
fn history_ownership_requires_detail_integration_in_production_render() {
    let missing_detail = r#"
        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
            }
        }
    "#;

    let violations = history_ownership_violations(missing_detail);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("render_detail_panel") && violation.contains("0")),
        "{violations:?}"
    );
}

#[test]
fn history_ownership_rejects_method_and_bordered_block_syntax() {
    for local_border in [
        "Block::default().borders(Borders::TOP);",
        "ratatui::widgets::Block::bordered();",
    ] {
        let source = format!(
            r#"
                impl HistoryScreen {{
                    fn render(&mut self) {{
                        render_screen_frame_sections();
                        render_scrollable_list_panel();
                        render_detail_panel();
                        {local_border}
                    }}
                }}
            "#
        );
        let violations = history_ownership_violations(&source);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("local") && violation.contains("border")),
            "{local_border}: {violations:?}"
        );
    }
}

#[test]
fn history_ownership_rejects_qualified_table_construction() {
    let qualified_table = r#"
        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
                render_detail_panel();
                ratatui::widgets::Table::new([], []);
            }
        }
    "#;

    let violations = history_ownership_violations(qualified_table);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("Table::new")),
        "{violations:?}"
    );
}

#[test]
fn history_ownership_rejects_local_table_row_and_cell_constructors() {
    let local_widgets = r#"
        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
                render_detail_panel();
                Table::new([], []);
                Row::new([]);
                Cell::from("");
            }
        }
    "#;

    let violations = history_ownership_violations(local_widgets);
    for constructor in ["Table::new", "Row::new", "Cell::from"] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(constructor)),
            "{constructor}: {violations:?}"
        );
    }
}

#[test]
fn history_ownership_rejects_aliased_block_and_table_use() {
    let aliased_ownership = r#"
        use ratatui::widgets::{Block as Panel, Table as Grid};

        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
                render_detail_panel();
                Panel::bordered();
                Grid::new([], []);
            }
        }
    "#;

    let violations = history_ownership_violations(aliased_ownership);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("Block") && violation.contains("Panel")),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("Table") && violation.contains("Grid")),
        "{violations:?}"
    );
}

#[test]
fn history_ownership_accepts_the_component_only_render_path() {
    let component_only = r#"
        impl HistoryScreen {
            fn render(&mut self) {
                render_screen_frame_sections();
                render_scrollable_list_panel();
                render_detail_panel();
            }
        }
    "#;

    assert!(
        history_ownership_violations(component_only).is_empty(),
        "component-only render must satisfy History ownership"
    );
}

#[test]
fn goals_ownership_requires_one_production_render_and_ignores_decoys() {
    let decoy_delegation = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                // render_status_line(); Clear; Paragraph::new("");
                let _literal = "render_shortcuts(); Block::bordered();";
                fn nested_item() {
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_status_line();
                    Clear;
                }
                render_detail_panel_with_header();
            }
        }

        fn dead_helper() {
            render_screen_frame();
            render_scrollable_list_panel_spanned();
            Paragraph::new("");
        }

        #[cfg(test)]
        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
            }
        }
    "#;

    let violations = goals_ownership_violations(decoy_delegation);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "{component}: {violations:?}"
        );
    }
    assert!(
        !violations.iter().any(|violation| {
            violation.contains("render_status_line")
                || violation.contains("Clear")
                || violation.contains("Paragraph::new")
        }),
        "nested and dead helpers must not create ownership violations: {violations:?}"
    );

    let duplicate_production_render = r#"
        impl GoalsScreen {
            fn render(&mut self) {}
            fn render(&self) {}
        }
    "#;
    let violations = goals_ownership_violations(duplicate_production_render);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("exactly one") && violation.contains("found 2")),
        "{violations:?}"
    );
}

#[test]
fn goals_ownership_rejects_wrong_module_required_calls() {
    let wrong_module = r#"
        impl GoalsScreen {
            fn render(&mut self) {
                decoy::render_screen_frame();
                decoy::render_scrollable_list_panel_spanned();
                decoy::render_detail_panel_with_header();
            }
        }
    "#;

    let violations = goals_ownership_violations(wrong_module);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "{component}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_rejects_wrong_origin_required_aliases() {
    let wrong_aliases = r#"
        use decoy::{
            render_detail_panel_with_header as details,
            render_screen_frame as frame_shell,
            render_scrollable_list_panel_spanned as goals_panel,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                frame_shell();
                goals_panel();
                details();
            }
        }
    "#;

    let violations = goals_ownership_violations(wrong_aliases);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "{component}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_rejects_same_name_local_wrappers() {
    let local_wrappers = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header as details,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                fn render_screen_frame() {}
                let render_scrollable_list_panel_spanned = || {};
                let details = || {};
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                details();
            }
        }
    "#;

    let violations = goals_ownership_violations(local_wrappers);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "{component}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_rejects_if_while_let_and_let_chain_shadows() {
    let if_while_let = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header as details,
            render_screen_frame as frame_shell,
            render_scrollable_list_panel_spanned as goals_panel,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                if let Some((frame_shell, goals_panel)) = handlers {
                    while let Some(details) = next_handler() {
                        frame_shell();
                        goals_panel();
                        details();
                    }
                }
            }
        }
    "#;
    let let_chain = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header as details,
            render_screen_frame as frame_shell,
            render_scrollable_list_panel_spanned as goals_panel,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                if let Some(frame_shell) = first_handler()
                    && let Some(goals_panel) = second_handler()
                    && let Some(details) = third_handler()
                {
                    frame_shell();
                    goals_panel();
                    details();
                }
            }
        }
    "#;

    for source in [if_while_let, let_chain] {
        let violations = goals_ownership_violations(source);
        for component in [
            "render_screen_frame",
            "render_scrollable_list_panel_spanned",
            "render_detail_panel_with_header",
        ] {
            assert!(
                violations.iter().any(
                    |violation| violation.contains(component) && violation.contains("found 0")
                ),
                "{component}: {violations:?}"
            );
        }
    }
}

#[test]
fn goals_ownership_rejects_render_local_wrong_origin_aliases() {
    let local_aliases = r#"
        impl GoalsScreen {
            fn render(&mut self) {
                use crate::tui::framework::{
                    render_detail_panel_with_header as details,
                    render_screen_frame as frame_shell,
                    render_scrollable_list_panel_spanned as goals_panel,
                };
                {
                    use decoy::{
                        details as details,
                        frame as frame_shell,
                        list as goals_panel,
                    };
                    frame_shell();
                    goals_panel();
                    details();
                }
            }
        }
    "#;

    let violations = goals_ownership_violations(local_aliases);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "{component}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_accepts_canonical_calls_outside_nested_shadows() {
    let nested_shadows = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header as details,
            render_screen_frame as frame_shell,
            render_scrollable_list_panel_spanned as goals_panel,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                {
                    use decoy::frame as frame_shell;
                    let goals_panel = || {};
                    fn details() {}
                }
                frame_shell();
                goals_panel();
                details();
            }
        }
    "#;

    let violations = goals_ownership_violations(nested_shadows);
    assert!(
        violations.is_empty(),
        "nested shadows must not invalidate canonical calls in the outer scope: {violations:?}"
    );
}

#[test]
fn goals_ownership_rejects_duplicate_delegation_and_local_rendering() {
    let mixed_ownership = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                render_detail_panel_with_header();
                Block::default().borders(Borders::ALL);
                Block::bordered();
                render_status_line();
                render_shortcuts();
                frame.render_widget(Clear, area);
                Paragraph::new("local footer");
            }
        }
    "#;

    let violations = goals_ownership_violations(mixed_ownership);
    for symbol in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(symbol) && violation.contains("found 2")),
            "{symbol}: {violations:?}"
        );
    }
    for forbidden in [
        ".borders",
        "Block::bordered",
        "render_status_line",
        "render_shortcuts",
        "Clear",
        "Paragraph::new",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(forbidden)),
            "{forbidden}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_normalizes_qualified_paths() {
    let qualified_ownership = r#"
        impl crate::screens::GoalsScreen {
            fn render(&mut self) {
                crate::tui::framework::render_screen_frame();
                crate::tui::framework::render_scrollable_list_panel_spanned();
                crate::tui::framework::render_detail_panel_with_header();
                ratatui::widgets::Block::bordered();
                crate::tui::render_status_line();
                crate::tui::render_shortcuts();
                frame.render_widget(ratatui::widgets::Clear, area);
                ratatui::widgets::Paragraph::new("local footer");
            }
        }
    "#;

    let violations = goals_ownership_violations(qualified_ownership);
    for forbidden in [
        "Block::bordered",
        "render_status_line",
        "render_shortcuts",
        "Clear",
        "Paragraph::new",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(forbidden)),
            "{forbidden}: {violations:?}"
        );
    }
    assert!(
        !violations
            .iter()
            .any(|violation| violation.contains("found 0")),
        "qualified component calls must count: {violations:?}"
    );
}

#[test]
fn goals_ownership_rejects_forbidden_function_and_constructor_paths() {
    let indirect_ownership = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                let _border = ratatui::widgets::Block::bordered;
                let _status = crate::tui::render_status_line;
                let _keys = crate::tui::render_shortcuts;
                let _clear = ratatui::widgets::Clear;
                let _paragraph = ratatui::widgets::Paragraph::new;
            }
        }
    "#;

    let violations = goals_ownership_violations(indirect_ownership);
    for forbidden in [
        "Block::bordered",
        "render_status_line",
        "render_shortcuts",
        "Clear",
        "Paragraph::new",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(forbidden)),
            "{forbidden}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_rejects_trait_qualified_ufcs_widget_paths() {
    let trait_qualified_widgets = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        use ratatui::widgets::{Block as Panel, Paragraph as Copy};

        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                let _border = <Block as LocalBorderFactory>::bordered;
                let _qualified_border =
                    <ratatui::widgets::Block as LocalBorderFactory>::bordered;
                let _aliased_border = <Panel as LocalBorderFactory>::bordered;
                let _paragraph = <Paragraph as LocalParagraphFactory>::new;
                let _qualified_paragraph =
                    <ratatui::widgets::Paragraph as LocalParagraphFactory>::new;
                let _aliased_paragraph = <Copy as LocalParagraphFactory>::new;
            }
        }
    "#;

    let violations = goals_ownership_violations(trait_qualified_widgets);
    assert!(
        violations.iter().any(|violation| {
            violation.contains("Block::bordered") && violation.contains("found 3")
        }),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|violation| {
            violation.contains("Paragraph::new") && violation.contains("found 3")
        }),
        "{violations:?}"
    );
}

#[test]
fn goals_ownership_rejects_all_clear_construction_paths() {
    let clear_ownership = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        use ratatui::widgets::Clear as Eraser;

        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                frame.render_widget(Clear::default(), area);
                frame.render_widget(ratatui::widgets::Clear::default(), area);
                frame.render_widget(Eraser::default(), area);
                frame.render_widget(Clear {}, area);
                frame.render_widget(<Clear>::default(), area);
                frame.render_widget(<ratatui::widgets::Clear as Default>::default(), area);
                frame.render_widget(<Eraser as Default>::default(), area);
            }
        }
    "#;

    let violations = goals_ownership_violations(clear_ownership);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("Clear") && violation.contains("found 7")),
        "{violations:?}"
    );
}

#[test]
fn goals_ownership_normalizes_aliased_paths() {
    let aliased_ownership = r#"
        use crate::tui::{render_shortcuts as local_keys, render_status_line as local_status};
        use ratatui::widgets::{Block as Panel, Clear as Eraser, Paragraph as Copy};

        impl GoalsScreen {
            fn render(&mut self) {
                use crate::tui::framework::{
                    render_detail_panel_with_header as details,
                    render_screen_frame as frame_shell,
                    render_scrollable_list_panel_spanned as goals_panel,
                };
                frame_shell();
                goals_panel();
                details();
                Panel::bordered();
                local_status();
                local_keys();
                frame.render_widget(Eraser, area);
                Copy::new("local footer");
            }
        }
    "#;

    let violations = goals_ownership_violations(aliased_ownership);
    for forbidden in [
        "Block::bordered",
        "render_status_line",
        "render_shortcuts",
        "Clear",
        "Paragraph::new",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(forbidden)),
            "{forbidden}: {violations:?}"
        );
    }
    assert!(
        !violations
            .iter()
            .any(|violation| violation.contains("found 0")),
        "aliased component calls must count: {violations:?}"
    );
}

#[test]
fn goals_ownership_accepts_component_only_render_path() {
    let component_only = r#"
        use crate::tui::framework::{
            render_detail_panel_with_header,
            render_screen_frame,
            render_scrollable_list_panel_spanned,
        };

        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
            }
        }
    "#;

    assert!(
        goals_ownership_violations(component_only).is_empty(),
        "component-only render must satisfy Goals ownership"
    );
}

#[test]
fn goals_ownership_does_not_accept_delegation_inside_an_uninvoked_closure() {
    let closure_decoy = r#"
        use super::framework::{
            render_detail_panel_with_header, render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                let _never_called = || {
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_detail_panel_with_header();
                };
            }
        }
    "#;

    let violations = goals_ownership_violations(closure_decoy);
    for component in [
        "render_screen_frame",
        "render_scrollable_list_panel_spanned",
        "render_detail_panel_with_header",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(component) && violation.contains("found 0")),
            "uninvoked closure must not satisfy {component}: {violations:?}"
        );
    }
}

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
    assert!(violations
        .iter()
        .any(|item| item.contains("Paragraph::new")));
}

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

#[test]
fn goals_ownership_accepts_grouped_self_framework_module_aliases() {
    let cases = [
        (
            "super grouped self alias",
            r#"
                use super::framework::{self as shared};
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "crate grouped self alias",
            r#"
                use crate::tui::framework::{self as shared};
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "iron grouped self alias",
            r#"
                use iron::tui::framework::{self as shared};
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    let failures = cases
        .iter()
        .filter_map(|(name, source)| {
            let violations = goals_ownership_violations(source);
            (!violations.is_empty()).then(|| format!("{name}: {violations:?}"))
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn goals_ownership_checks_forbidden_functions_through_grouped_self_aliases() {
    let cases = [
        r#"
            use super::framework::{self as shared};
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    shared::render_screen_frame();
                    shared::render_scrollable_list_panel_spanned();
                    shared::render_detail_panel_with_header();
                    shared::render_status_line();
                }
            }
        "#,
        r#"
            use crate::tui::framework::{self as shared};
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    shared::render_screen_frame();
                    shared::render_scrollable_list_panel_spanned();
                    shared::render_detail_panel_with_header();
                    shared::render_status_line();
                }
            }
        "#,
        r#"
            use iron::tui::framework::{self as shared};
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    shared::render_screen_frame();
                    shared::render_scrollable_list_panel_spanned();
                    shared::render_detail_panel_with_header();
                    shared::render_status_line();
                }
            }
        "#,
    ];

    for source in cases {
        let violations = goals_ownership_violations(source);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("render_status_line")),
            "a grouped framework module alias must preserve forbidden ownership: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_accepts_crate_and_iron_framework_module_aliases_and_globs() {
    let cases = [
        (
            "crate module alias",
            r#"
                use crate::tui::framework as shared;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "crate module glob",
            r#"
                use crate::tui::framework::*;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "iron module alias",
            r#"
                use iron::tui::framework as shared;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "iron module glob",
            r#"
                use iron::tui::framework::*;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    let failures = cases
        .iter()
        .filter_map(|(name, source)| {
            let violations = goals_ownership_violations(source);
            (!violations.is_empty()).then(|| format!("{name}: {violations:?}"))
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn goals_ownership_value_bindings_do_not_shadow_framework_module_aliases() {
    let source = r#"
        use super::framework as shared;
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                let shared = || {};
                shared::render_screen_frame();
                shared::render_scrollable_list_panel_spanned();
                shared::render_detail_panel_with_header();
                shared();
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_value_imports_do_not_shadow_framework_module_aliases() {
    let source = r#"
        use super::framework as shared;
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                use domain::render_status_line as shared;
                shared::render_screen_frame();
                shared::render_scrollable_list_panel_spanned();
                shared::render_detail_panel_with_header();
                shared();
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_type_module_bindings_shadow_framework_module_aliases() {
    let cases = [
        (
            "inner module",
            r#"
                use super::framework as shared;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        mod shared {
                            pub fn render_screen_frame() {}
                            pub fn render_scrollable_list_panel_spanned() {}
                            pub fn render_detail_panel_with_header() {}
                        }
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "inner type alias",
            r#"
                use super::framework as shared;
                struct DomainFramework;
                impl DomainFramework {
                    fn render_screen_frame() {}
                    fn render_scrollable_list_panel_spanned() {}
                    fn render_detail_panel_with_header() {}
                }
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        type shared = DomainFramework;
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        for component in [
            "render_screen_frame",
            "render_scrollable_list_panel_spanned",
            "render_detail_panel_with_header",
        ] {
            assert!(
                violations.iter().any(
                    |violation| violation.contains(component) && violation.contains("found 0")
                ),
                "{name}: {component} must resolve through the inner type/module namespace: {violations:?}"
            );
        }
    }
}

#[test]
fn goals_ownership_type_module_bindings_do_not_shadow_forbidden_value_functions() {
    let cases = [
        (
            "inner module",
            r#"
                use crate::tui::render_status_line as status;
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        mod status {}
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        status();
                    }
                }
            "#,
        ),
        (
            "inner type alias",
            r#"
                use crate::tui::render_status_line as status;
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        type status = ();
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        status();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("render_status_line")),
            "{name}: a type/module binding must not hide a value function: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_does_not_resolve_forbidden_aliases_through_iife_parameters() {
    let source = r#"
        use crate::tui::render_status_line as status;
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
                (|status| { status(); })(|| {});
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_same_block_let_shadows_forbidden_import() {
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
                use crate::tui::render_status_line as status;
                let status = || {};
                status();
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_resolves_iife_value_and_import_precedence_lexically() {
    let inner_import = r#"
        use crate::tui::render_status_line as status;
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
                (|status| {
                    {
                        use crate::tui::render_status_line as status;
                        status();
                    }
                })(|| {});
            }
        }
    "#;
    let inner_value_shadow = r#"
        use crate::tui::render_status_line as status;
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
                (|status| { status(); })(|| {});
            }
        }
    "#;

    let inner_import_violations = goals_ownership_violations(inner_import);
    assert!(inner_import_violations
        .iter()
        .any(|item| item.contains("render_status_line")));
    assert!(goals_ownership_violations(inner_value_shadow).is_empty());
}

#[test]
fn goals_ownership_checks_widget_types_inside_iifes_with_same_named_parameters() {
    let source = r#"
        use ratatui::widgets::Paragraph as Copy;
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
                (|Copy| { Copy::new("forbidden"); })(());
            }
        }
    "#;

    let violations = goals_ownership_violations(source);
    assert!(violations
        .iter()
        .any(|item| item.contains("Paragraph::new")));
}

#[test]
fn goals_ownership_root_items_shadow_canonical_framework_globs() {
    let source = r#"
        use super::framework::*;
        fn render_screen_frame() {}
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
            }
        }
    "#;

    let violations = goals_ownership_violations(source);
    assert!(violations.iter().any(|violation| {
        violation.contains("render_screen_frame") && violation.contains("found 0")
    }));
}

#[test]
fn goals_ownership_ignores_cfg_test_nested_value_item_shadows() {
    let source = r#"
        use crate::tui::render_status_line as status;
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
                {
                    #[cfg(test)]
                    fn status() {}
                    status();
                }
            }
        }
    "#;

    let violations = goals_ownership_violations(source);
    assert!(violations
        .iter()
        .any(|item| item.contains("render_status_line")));
}

#[test]
fn goals_ownership_keeps_production_relevant_cfg_nested_value_item_shadows() {
    let cases = [
        r#"
            use crate::tui::render_status_line as status;
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
                    {
                        #[cfg(not(test))]
                        fn status() {}
                        status();
                    }
                }
            }
        "#,
        r#"
            use crate::tui::render_status_line as status;
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
                    {
                        #[cfg(any(test, feature = "fixture"))]
                        fn status() {}
                        status();
                    }
                }
            }
        "#,
    ];

    for source in cases {
        assert!(goals_ownership_violations(source).is_empty());
    }
}

#[test]
fn goals_ownership_ignores_cfg_test_root_item_shadows() {
    let source = r#"
        use super::framework::*;
        #[cfg(test)]
        fn render_screen_frame() {}
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
            }
        }
    "#;

    assert!(goals_ownership_violations(source).is_empty());
}

#[test]
fn goals_ownership_keeps_production_relevant_cfg_root_item_shadows() {
    let cases = [
        r#"
            use super::framework::*;
            #[cfg(not(test))]
            fn render_screen_frame() {}
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_detail_panel_with_header();
                }
            }
        "#,
        r#"
            use super::framework::*;
            #[cfg(any(test, feature = "fixture"))]
            fn render_screen_frame() {}
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
        let violations = goals_ownership_violations(source);
        assert!(violations.iter().any(|violation| {
            violation.contains("render_screen_frame") && violation.contains("found 0")
        }));
    }
}

#[test]
fn goals_ownership_cfg_test_required_imports_do_not_enter_production_resolution() {
    let cases = [
        (
            "root import",
            r#"
                #[cfg(test)]
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
                    }
                }
            "#,
        ),
        (
            "nested import",
            r#"
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(all(test, feature = "fixture"))]
                        use super::framework::{
                            render_detail_panel_with_header, render_screen_frame,
                            render_scrollable_list_panel_spanned,
                        };
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        for component in [
            "render_screen_frame",
            "render_scrollable_list_panel_spanned",
            "render_detail_panel_with_header",
        ] {
            assert!(
                violations.iter().any(
                    |violation| violation.contains(component) && violation.contains("found 0")
                ),
                "{name}: a pure-test import must not satisfy {component}: {violations:?}"
            );
        }
    }
}

#[test]
fn goals_ownership_cfg_test_wrong_required_imports_do_not_shadow_production() {
    let cases = [
        (
            "root import",
            r#"
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                #[cfg(test)]
                use domain::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "nested import",
            r#"
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(test)]
                        use domain::{
                            render_detail_panel_with_header, render_screen_frame,
                            render_scrollable_list_panel_spanned,
                        };
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }
}

#[test]
fn goals_ownership_cfg_test_forbidden_imports_do_not_enter_production_resolution() {
    let cases = [
        (
            "root imports",
            r#"
                use domain::{Paragraph as Copy, render_status_line as status};
                #[cfg(test)]
                use ratatui::widgets::Paragraph as Copy;
                #[cfg(test)]
                use crate::tui::render_status_line as status;
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
                        Copy::new("production domain copy");
                        status();
                    }
                }
            "#,
        ),
        (
            "nested imports",
            r#"
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        use domain::{Paragraph as Copy, render_status_line as status};
                        #[cfg(test)]
                        use ratatui::widgets::Paragraph as Copy;
                        #[cfg(test)]
                        use crate::tui::render_status_line as status;
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        Copy::new("production domain copy");
                        status();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }
}

#[test]
fn goals_ownership_cfg_test_wrong_forbidden_imports_do_not_shadow_production() {
    let cases = [
        (
            "root imports",
            r#"
                use ratatui::widgets::Paragraph as Copy;
                use crate::tui::render_status_line as status;
                #[cfg(test)]
                use domain::{Paragraph as Copy, render_status_line as status};
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
                        Copy::new("forbidden");
                        status();
                    }
                }
            "#,
        ),
        (
            "nested imports",
            r#"
                use ratatui::widgets::Paragraph as Copy;
                use crate::tui::render_status_line as status;
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(test)]
                        use domain::{Paragraph as Copy, render_status_line as status};
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        Copy::new("forbidden");
                        status();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        for forbidden in ["Paragraph::new", "render_status_line"] {
            assert!(
                violations
                    .iter()
                    .any(|violation| violation.contains(forbidden)),
                "{name}: {forbidden} must remain visible in production: {violations:?}"
            );
        }
    }
}

#[test]
fn goals_ownership_cfg_test_type_module_items_do_not_shadow_production() {
    let module_alias_cases = [
        (
            "root module",
            r#"
                use super::framework as shared;
                #[cfg(test)]
                mod shared {
                    pub fn render_screen_frame() {}
                    pub fn render_scrollable_list_panel_spanned() {}
                    pub fn render_detail_panel_with_header() {}
                }
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
        (
            "nested type alias",
            r#"
                use super::framework as shared;
                struct DomainFramework;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(test)]
                        type shared = DomainFramework;
                        shared::render_screen_frame();
                        shared::render_scrollable_list_panel_spanned();
                        shared::render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in module_alias_cases {
        let violations = goals_ownership_violations(source);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }

    let forbidden_types = r#"
        use ratatui::widgets::{
            Paragraph as RootModuleCopy, Paragraph as RootTypeCopy,
            Paragraph as RootEnumCopy, Paragraph as RootStructCopy,
            Paragraph as RootUnionCopy, Paragraph as RootTraitCopy,
            Paragraph as NestedModuleCopy, Paragraph as NestedTypeCopy,
            Paragraph as NestedEnumCopy, Paragraph as NestedStructCopy,
            Paragraph as NestedUnionCopy, Paragraph as NestedTraitCopy,
        };
        #[cfg(test)] mod RootModuleCopy {}
        #[cfg(test)] type RootTypeCopy = ();
        #[cfg(test)] enum RootEnumCopy {}
        #[cfg(test)] struct RootStructCopy;
        #[cfg(test)] union RootUnionCopy { byte: u8 }
        #[cfg(test)] trait RootTraitCopy {}
        use super::framework::{
            render_detail_panel_with_header, render_screen_frame,
            render_scrollable_list_panel_spanned,
        };
        struct GoalsScreen;
        impl GoalsScreen {
            fn render(&mut self) {
                #[cfg(test)] mod NestedModuleCopy {}
                #[cfg(test)] type NestedTypeCopy = ();
                #[cfg(test)] enum NestedEnumCopy {}
                #[cfg(test)] struct NestedStructCopy;
                #[cfg(test)] union NestedUnionCopy { byte: u8 }
                #[cfg(test)] trait NestedTraitCopy {}
                render_screen_frame();
                render_scrollable_list_panel_spanned();
                render_detail_panel_with_header();
                RootModuleCopy::new("forbidden");
                RootTypeCopy::new("forbidden");
                RootEnumCopy::new("forbidden");
                RootStructCopy::new("forbidden");
                RootUnionCopy::new("forbidden");
                RootTraitCopy::new("forbidden");
                NestedModuleCopy::new("forbidden");
                NestedTypeCopy::new("forbidden");
                NestedEnumCopy::new("forbidden");
                NestedStructCopy::new("forbidden");
                NestedUnionCopy::new("forbidden");
                NestedTraitCopy::new("forbidden");
            }
        }
    "#;
    let violations = goals_ownership_violations(forbidden_types);
    assert!(violations.iter().any(|violation| {
        violation.contains("Paragraph::new") && violation.contains("found 12")
    }));
}

#[test]
fn goals_ownership_cfg_production_relevant_imports_and_items_still_resolve() {
    let cases = [
        (
            "not-test canonical import",
            r#"
                #[cfg(not(test))]
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
                    }
                }
            "#,
            true,
        ),
        (
            "mixed canonical import",
            r#"
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(any(test, feature = "fixture"))]
                        use super::framework::{
                            render_detail_panel_with_header, render_screen_frame,
                            render_scrollable_list_panel_spanned,
                        };
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
            true,
        ),
        (
            "not-test wrong required import",
            r#"
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        #[cfg(not(test))]
                        use domain::{
                            render_detail_panel_with_header, render_screen_frame,
                            render_scrollable_list_panel_spanned,
                        };
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
            false,
        ),
        (
            "mixed forbidden type item",
            r#"
                use ratatui::widgets::Paragraph as Copy;
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                #[cfg(any(test, feature = "fixture"))]
                struct Copy;
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        Copy::new("not framework ownership");
                    }
                }
            "#,
            true,
        ),
    ];

    for (name, source, expect_clean) in cases {
        let violations = goals_ownership_violations(source);
        assert_eq!(
            violations.is_empty(),
            expect_clean,
            "{name}: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_rejects_noncanonical_framework_module_aliases_globs_and_shadows() {
    let cases = [
        r#"
            use domain::framework as shared;
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
            use super::framework as shared;
            struct GoalsScreen;
            impl GoalsScreen {
                fn render(&mut self) {
                    use domain::*;
                    render_screen_frame();
                    render_scrollable_list_panel_spanned();
                    render_detail_panel_with_header();
                }
            }
        "#,
    ];

    for source in cases {
        let violations = goals_ownership_violations(source);
        for component in [
            "render_screen_frame",
            "render_scrollable_list_panel_spanned",
            "render_detail_panel_with_header",
        ] {
            assert!(
                violations.iter().any(
                    |violation| violation.contains(component) && violation.contains("found 0")
                ),
                "{component} must not be satisfied by a noncanonical import: {violations:?}"
            );
        }
    }
}

#[test]
fn goals_ownership_checks_forbidden_rendering_in_reachable_sibling_helpers() {
    let reachable_helper = r#"
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
                self.paint_locally();
            }

            fn paint_locally(&self) {
                Block::default().borders(Borders::ALL);
            }
        }
    "#;

    let violations = goals_ownership_violations(reachable_helper);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains(".borders") && violation.contains("found 1")),
        "reachable helpers must be part of the production render path: {violations:?}"
    );
}

#[test]
fn goals_ownership_rejects_ufcs_borders_and_clear_type_paths() {
    let equivalent_syntax = r#"
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
                let _: Option<Clear> = None;
                Block::borders(Block::default(), Borders::ALL);
            }
        }
    "#;

    let violations = goals_ownership_violations(equivalent_syntax);
    for forbidden in [".borders", "Clear"] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(forbidden)),
            "equivalent {forbidden} syntax must be rejected: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_excludes_nested_cfg_test_render_impls() {
    let nested_cfg_test = r#"
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
            }
        }

        #[cfg(all(test, feature = "fixture"))]
        impl GoalsScreen {
            fn render(&mut self) {
                Clear;
            }
        }
    "#;

    assert!(
        goals_ownership_violations(nested_cfg_test).is_empty(),
        "nested cfg(test) forms must be excluded"
    );
}

#[test]
fn goals_ownership_keeps_cfg_forms_that_can_compile_outside_tests() {
    for condition in ["not(test)", "any(test, feature = \"fixture\")"] {
        let source = format!(
            r#"
                use super::framework::{{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                }};
                struct GoalsScreen;
                #[cfg({condition})]
                impl GoalsScreen {{
                    fn render(&mut self) {{
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }}
                }}
            "#
        );

        let violations = goals_ownership_violations(&source);
        assert!(
            violations.is_empty(),
            "cfg({condition}) can compile outside tests: {violations:?}"
        );
    }
}

#[test]
fn goals_ownership_resolves_forbidden_aliases_and_globs_by_lexical_scope() {
    let cases = [
        (
            "wrong-origin forbidden alias",
            r#"
                use domain::Block as Panel;
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
                        Panel::bordered();
                    }
                }
            "#,
        ),
        (
            "inner explicit import shadows forbidden alias",
            r#"
                use ratatui::widgets::Block as Panel;
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        use domain::Panel;
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                        Panel::bordered();
                    }
                }
            "#,
        ),
        (
            "unrelated glob does not hide explicit component imports",
            r#"
                use super::framework::{
                    render_detail_panel_with_header, render_screen_frame,
                    render_scrollable_list_panel_spanned,
                };
                struct GoalsScreen;
                impl GoalsScreen {
                    fn render(&mut self) {
                        use domain::*;
                        render_screen_frame();
                        render_scrollable_list_panel_spanned();
                        render_detail_panel_with_header();
                    }
                }
            "#,
        ),
    ];

    for (name, source) in cases {
        let violations = goals_ownership_violations(source);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }
}

/// Scans all TUI source files and verifies that every `Paragraph::new(`
/// rendering user-entered or variable-length content uses `.wrap(`.
///
/// Fixed UI elements (titles, footers, shortcuts, single-line labels) are
/// allowlisted by file:line. If you add a new Paragraph without `.wrap()`,
/// either add `.wrap(Wrap { trim: false })` or add the line to the allowlist
/// with a comment explaining why wrapping is unnecessary.
#[test]
fn paragraph_widgets_must_wrap_user_content() {
    // Lines that are known-safe: fixed UI text that cannot overflow.
    // Format: "filename:line_number"
    // When adding entries, include a brief reason.
    let allowlist: HashSet<&str> = [
        "dashboard.rs:141",
        "dashboard.rs:339",
        "dashboard.rs:359",
        "dashboard.rs:395",
        "log_entry.rs:567",
        "log_entry.rs:748",
        "log_entry.rs:829",
    ]
    .into_iter()
    .collect();

    let tui_dir = Path::new("src/tui");
    let violations = paragraph_violations(tui_dir, &allowlist);

    // Also check that allowlist entries still correspond to actual Paragraph::new( lines
    // (catch stale entries after refactors)
    let mut stale = Vec::new();
    for &entry in &allowlist {
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        let filename = parts[0];
        let line_num: usize = parts[1].parse().unwrap();
        let path = tui_dir.join(filename);
        if !path.exists() {
            stale.push(format!("  {} — file does not exist", entry));
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = source.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            stale.push(format!(
                "  {} — line number out of range (file has {} lines)",
                entry,
                lines.len()
            ));
            continue;
        }
        if !lines[line_num - 1].contains("Paragraph::new(") {
            stale.push(format!(
                "  {} — no Paragraph::new( on this line (found: {:?}). Update or remove from allowlist.",
                entry,
                lines[line_num - 1].trim()
            ));
        }
    }

    let mut msg = String::new();
    if !violations.is_empty() {
        msg.push_str(&format!(
            "Paragraph widgets without .wrap() found (see CLAUDE.md constraint):\n{}\n",
            violations.join("\n")
        ));
    }
    if !stale.is_empty() {
        msg.push_str(&format!(
            "Stale allowlist entries (line numbers shifted after edits):\n{}\n",
            stale.join("\n")
        ));
    }
    assert!(msg.is_empty(), "\n{}", msg);
}

/// Ensures that main content screens use bordered panels for visual consistency.
/// Log Entry's executable ownership is validated precisely by the Syn-based guard
/// above; this legacy visual-consistency check accepts its shared panel owner.
#[test]
fn screens_must_have_bordered_blocks() {
    let tui_dir = Path::new("src/tui");
    let required_screens = ["log_entry.rs"];

    let mut missing = Vec::new();
    for filename in &required_screens {
        let path = tui_dir.join(filename);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read {}", path.display()));
        if !source.contains("render_bounded_panel(") {
            missing.push(format!("  {} — missing render_bounded_panel", filename));
        }
    }

    assert!(
        missing.is_empty(),
        "\nScreens missing bordered blocks for visual consistency:\n{}\n",
        missing.join("\n")
    );

    for (filename, screen_name) in [
        ("goals.rs", "Goals"),
        ("practices.rs", "Practices"),
        ("quotes_screen.rs", "Quotes"),
        ("trends.rs", "Trends"),
        ("history.rs", "History"),
    ] {
        let source = std::fs::read_to_string(tui_dir.join(filename)).unwrap();
        match filename {
            "goals.rs" => {
                let violations = goals_ownership_violations(&source);
                assert!(
                    violations.is_empty(),
                    "Goals presentation ownership violations:\n{}",
                    violations.join("\n")
                );
            }
            "history.rs" => {
                let violations = history_ownership_violations(&source);
                assert!(
                    violations.is_empty(),
                    "History presentation ownership violations:\n{}",
                    violations.join("\n")
                );
            }
            _ => {
                assert!(
                    source_code_contains(&source, "render_scrollable_list_panel("),
                    "{screen_name} must delegate its bordered list to ScrollableListPanel"
                );
            }
        }
    }

    let list_panel_source =
        std::fs::read_to_string(tui_dir.join("framework/list_panel.rs")).unwrap();
    assert!(
        source_code_contains(&list_panel_source, ".borders(Borders::ALL)"),
        "ScrollableListPanel must own the delegated bordered-list contract"
    );
}
