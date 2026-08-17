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

#[derive(Clone)]
enum GoalsCallResolution {
    Framework(Vec<String>),
    Shadowed,
}

#[derive(Clone)]
enum GoalsOwnershipResolution {
    Origin(Vec<String>),
    Shadowed,
}

fn collect_goals_ownership_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    resolutions: &mut HashMap<String, GoalsOwnershipResolution>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_goals_ownership_imports(&path.tree, prefix, resolutions);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_goals_ownership_imports(item, prefix, resolutions);
            }
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(local_name) = prefix.last() {
                resolutions.insert(
                    local_name.clone(),
                    GoalsOwnershipResolution::Origin(prefix.clone()),
                );
            }
        }
        syn::UseTree::Name(name) => {
            let local_name = name.ident.to_string();
            let mut origin = prefix.clone();
            origin.push(local_name.clone());
            resolutions.insert(local_name, GoalsOwnershipResolution::Origin(origin));
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            origin.push(rename.ident.to_string());
            resolutions.insert(
                rename.rename.to_string(),
                GoalsOwnershipResolution::Origin(origin),
            );
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn expanded_goals_ownership_path(
    path: &syn::Path,
    scopes: &[HashMap<String, GoalsOwnershipResolution>],
) -> Vec<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let Some(first) = segments.first().cloned() else {
        return segments;
    };
    for scope in scopes.iter().rev() {
        match scope.get(&first) {
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

fn insert_goals_call_resolution(
    resolutions: &mut HashMap<String, GoalsCallResolution>,
    local_name: String,
    origin: Vec<String>,
) {
    let resolution = if canonical_goals_framework_path(&origin) {
        GoalsCallResolution::Framework(origin)
    } else {
        GoalsCallResolution::Shadowed
    };
    resolutions
        .entry(local_name)
        .and_modify(|existing| {
            if matches!(resolution, GoalsCallResolution::Shadowed) {
                *existing = GoalsCallResolution::Shadowed;
            }
        })
        .or_insert(resolution);
}

fn collect_goals_call_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    resolutions: &mut HashMap<String, GoalsCallResolution>,
    has_glob_import: &mut bool,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_goals_call_imports(&path.tree, prefix, resolutions, has_glob_import);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_goals_call_imports(item, prefix, resolutions, has_glob_import);
            }
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(local_name) = prefix.last() {
                insert_goals_call_resolution(resolutions, local_name.clone(), prefix.clone());
            }
        }
        syn::UseTree::Name(name) => {
            let local_name = name.ident.to_string();
            let mut origin = prefix.clone();
            origin.push(local_name.clone());
            insert_goals_call_resolution(resolutions, local_name, origin);
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            origin.push(rename.ident.to_string());
            insert_goals_call_resolution(resolutions, rename.rename.to_string(), origin);
        }
        syn::UseTree::Glob(_) => *has_glob_import = true,
    }
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
    scopes: &[HashMap<String, GoalsCallResolution>],
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
            if let Some(resolution) = scope.get(local_name) {
                return match resolution {
                    GoalsCallResolution::Framework(origin) => {
                        canonical_goals_framework_path(origin)
                            .then(|| origin.last().unwrap().clone())
                    }
                    GoalsCallResolution::Shadowed => None,
                };
            }
        }
        return None;
    }
    canonical_goals_framework_path(&path).then(|| path.last().unwrap().clone())
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
    call_scopes: Vec<HashMap<String, GoalsCallResolution>>,
    ownership_scopes: Vec<HashMap<String, GoalsOwnershipResolution>>,
    ownership: GoalsRenderOwnership,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl GoalsRenderVisitor<'_> {
    fn block_call_scope(&self, block: &syn::Block) -> HashMap<String, GoalsCallResolution> {
        let mut scope = HashMap::new();
        let mut has_glob_import = false;
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            match item {
                syn::Item::Const(item) => {
                    scope.insert(item.ident.to_string(), GoalsCallResolution::Shadowed);
                }
                syn::Item::Fn(item) => {
                    scope.insert(item.sig.ident.to_string(), GoalsCallResolution::Shadowed);
                }
                syn::Item::Static(item) => {
                    scope.insert(item.ident.to_string(), GoalsCallResolution::Shadowed);
                }
                syn::Item::Use(item) => collect_goals_call_imports(
                    &item.tree,
                    &mut Vec::new(),
                    &mut scope,
                    &mut has_glob_import,
                ),
                _ => {}
            }
        }
        let _ = has_glob_import;
        scope
    }

    fn block_ownership_scope(
        &self,
        block: &syn::Block,
    ) -> HashMap<String, GoalsOwnershipResolution> {
        let mut scope = HashMap::new();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            match item {
                syn::Item::Enum(item) => {
                    scope.insert(item.ident.to_string(), GoalsOwnershipResolution::Shadowed);
                }
                syn::Item::Struct(item) => {
                    scope.insert(item.ident.to_string(), GoalsOwnershipResolution::Shadowed);
                }
                syn::Item::Type(item) => {
                    scope.insert(item.ident.to_string(), GoalsOwnershipResolution::Shadowed);
                }
                syn::Item::Union(item) => {
                    scope.insert(item.ident.to_string(), GoalsOwnershipResolution::Shadowed);
                }
                syn::Item::Use(item) => {
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
            scope.insert(name, GoalsCallResolution::Shadowed);
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
        self.call_scopes.push(HashMap::new());
        self.shadow_pattern_bindings(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.call_scopes.pop();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.call_scopes.push(HashMap::new());
        self.shadow_pattern_bindings(&expression.pat);
        self.visit_block(&expression.body);
        self.call_scopes.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.call_scopes.push(HashMap::new());
        self.visit_condition_with_let_bindings(&expression.cond);
        self.visit_block(&expression.then_branch);
        self.call_scopes.pop();
        if let Some((_, else_branch)) = &expression.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.call_scopes.push(HashMap::new());
        self.visit_condition_with_let_bindings(&expression.cond);
        self.visit_block(&expression.body);
        self.call_scopes.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
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
        match canonical_goals_forbidden_function(&expanded) {
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
    let mut root_call_scope = HashMap::new();
    let mut root_has_glob_import = false;
    let mut root_ownership_scope = HashMap::new();
    for item in &file.items {
        if let syn::Item::Use(item_use) = item {
            collect_goals_call_imports(
                &item_use.tree,
                &mut Vec::new(),
                &mut root_call_scope,
                &mut root_has_glob_import,
            );
            collect_goals_ownership_imports(
                &item_use.tree,
                &mut Vec::new(),
                &mut root_ownership_scope,
            );
        }
    }
    let _ = root_has_glob_import;

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
        let mut signature_scope = HashMap::new();
        for input in &method.sig.inputs {
            if let syn::FnArg::Typed(input) = input {
                let mut bindings = GoalsPatternBindingCollector::default();
                bindings.visit_pat(&input.pat);
                for name in bindings.names {
                    signature_scope.insert(name, GoalsCallResolution::Shadowed);
                }
            }
        }
        visitor.call_scopes.push(signature_scope);
        visitor.ownership_scopes.push(HashMap::new());
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
        "dashboard.rs:136",
        "dashboard.rs:410",
        "dashboard.rs:430",
        "dashboard.rs:465",
        "log_entry.rs:544",
        "log_entry.rs:736",
        "log_entry.rs:835",
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

/// Ensures that main content screens use bordered blocks for visual consistency.
/// Every screen that displays a primary list or content area should wrap it in
/// a Block with Borders::ALL.
#[test]
fn screens_must_have_bordered_blocks() {
    let tui_dir = Path::new("src/tui");
    let required_screens = ["log_entry.rs"];

    let mut missing = Vec::new();
    for filename in &required_screens {
        let path = tui_dir.join(filename);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read {}", path.display()));
        if !source.contains(".borders(Borders::ALL)") {
            missing.push(format!("  {} — missing .borders(Borders::ALL)", filename));
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
