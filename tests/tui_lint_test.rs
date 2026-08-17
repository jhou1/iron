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
    attrs.iter().any(|attribute| {
        let syn::Meta::List(list) = &attribute.meta else {
            return false;
        };
        list.path.is_ident("cfg")
            && list
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
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

fn collect_goals_framework_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    imports: &mut HashMap<String, Vec<String>>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_goals_framework_imports(&path.tree, prefix, imports);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_goals_framework_imports(item, prefix, imports);
            }
        }
        syn::UseTree::Name(name) if is_goals_framework_component(&name.ident.to_string()) => {
            let original = name.ident.to_string();
            let mut path = prefix.clone();
            path.push(original.clone());
            imports.insert(original, path);
        }
        syn::UseTree::Rename(rename) if is_goals_framework_component(&rename.ident.to_string()) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            imports.insert(rename.rename.to_string(), path);
        }
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => {}
    }
}

struct LocalGoalsFrameworkImportCollector<'a> {
    imports: &'a mut HashMap<String, Vec<String>>,
}

impl<'ast> Visit<'ast> for LocalGoalsFrameworkImportCollector<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if let syn::Item::Use(item_use) = item {
            collect_goals_framework_imports(&item_use.tree, &mut Vec::new(), self.imports);
        }
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
    imports: &HashMap<String, Vec<String>>,
    local_bindings: &HashSet<String>,
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
    let canonical = if path.len() == 1 {
        let local_name = path.first()?;
        if local_bindings.contains(local_name) {
            return None;
        }
        imports.get(local_name)?
    } else {
        &path
    };
    canonical_goals_framework_path(canonical).then(|| canonical.last().unwrap().clone())
}

#[derive(Default)]
struct GoalsLocalBindingCollector {
    names: HashSet<String>,
    has_glob_import: bool,
}

fn collect_local_use_bindings(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    bindings: &mut GoalsLocalBindingCollector,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_local_use_bindings(&path.tree, prefix, bindings);
            prefix.pop();
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if !canonical_goals_framework_path(prefix) {
                if let Some(local_name) = prefix.last() {
                    bindings.names.insert(local_name.clone());
                }
            }
        }
        syn::UseTree::Name(name) => {
            let mut origin = prefix.clone();
            origin.push(name.ident.to_string());
            if !canonical_goals_framework_path(&origin) {
                bindings.names.insert(name.ident.to_string());
            }
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            origin.push(rename.ident.to_string());
            if !canonical_goals_framework_path(&origin) {
                bindings.names.insert(rename.rename.to_string());
            }
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_local_use_bindings(item, prefix, bindings);
            }
        }
        syn::UseTree::Glob(_) => bindings.has_glob_import = true,
    }
}

impl<'ast> Visit<'ast> for GoalsLocalBindingCollector {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        match item {
            syn::Item::Const(item) => {
                self.names.insert(item.ident.to_string());
            }
            syn::Item::Fn(item) => {
                self.names.insert(item.sig.ident.to_string());
            }
            syn::Item::Static(item) => {
                self.names.insert(item.ident.to_string());
            }
            syn::Item::Use(item) => {
                collect_local_use_bindings(&item.tree, &mut Vec::new(), self);
            }
            _ => {}
        }
        // Nested item bodies are declarations outside the render execution path.
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.names.insert(pattern.ident.to_string());
        visit::visit_pat_ident(self, pattern);
    }
}

fn normalized_path_name(path: &syn::Path, aliases: &HashMap<String, String>) -> Option<String> {
    let name = path.segments.last()?.ident.to_string();
    Some(aliases.get(&name).cloned().unwrap_or(name))
}

fn path_contains_name(path: &syn::Path, aliases: &HashMap<String, String>, expected: &str) -> bool {
    path.segments.iter().any(|segment| {
        let name = segment.ident.to_string();
        aliases.get(&name).map(String::as_str).unwrap_or(&name) == expected
    })
}

fn type_contains_name(ty: &syn::Type, aliases: &HashMap<String, String>, expected: &str) -> bool {
    match ty {
        syn::Type::Group(group) => type_contains_name(&group.elem, aliases, expected),
        syn::Type::Paren(paren) => type_contains_name(&paren.elem, aliases, expected),
        syn::Type::Path(path) => path_contains_name(&path.path, aliases, expected),
        syn::Type::Reference(reference) => type_contains_name(&reference.elem, aliases, expected),
        _ => false,
    }
}

fn expr_path_contains_name(
    expression: &syn::ExprPath,
    aliases: &HashMap<String, String>,
    expected: &str,
) -> bool {
    path_contains_name(&expression.path, aliases, expected)
        || expression
            .qself
            .as_ref()
            .is_some_and(|qself| type_contains_name(&qself.ty, aliases, expected))
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
}

struct GoalsRenderVisitor<'a> {
    aliases: &'a HashMap<String, String>,
    framework_imports: &'a HashMap<String, Vec<String>>,
    local_bindings: &'a HashSet<String>,
    ownership: GoalsRenderOwnership,
}

impl<'ast> Visit<'ast> for GoalsRenderVisitor<'_> {
    fn visit_item(&mut self, _item: &'ast syn::Item) {
        // Nested item declarations are not calls made by GoalsScreen::render.
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            match canonical_goals_framework_call(
                function,
                self.framework_imports,
                self.local_bindings,
            )
            .as_deref()
            {
                Some("render_screen_frame") => self.ownership.screen_frame_calls += 1,
                Some("render_scrollable_list_panel_spanned") => {
                    self.ownership.list_panel_calls += 1;
                }
                Some("render_detail_panel_with_header") => self.ownership.detail_panel_calls += 1,
                _ => {}
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
        match normalized_path_name(&path.path, self.aliases).as_deref() {
            Some("render_status_line") => self.ownership.status_line_calls += 1,
            Some("render_shortcuts") => self.ownership.shortcuts_calls += 1,
            _ => {}
        }
        if expr_path_contains_name(path, self.aliases, "Clear") {
            self.ownership.clear_paths += 1;
        }
        if let Some((receiver, method)) = called_type_and_method(path) {
            let original = self
                .aliases
                .get(&receiver)
                .map(String::as_str)
                .unwrap_or(receiver.as_str());
            match (original, method.as_str()) {
                ("Block", "bordered") => self.ownership.block_bordered_calls += 1,
                ("Paragraph", "new") => self.ownership.paragraph_new_calls += 1,
                _ => {}
            }
        }
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if path_contains_name(&expression.path, self.aliases, "Clear") {
            self.ownership.clear_paths += 1;
        }
        visit::visit_expr_struct(self, expression);
    }
}

fn goals_ownership_violations(source: &str) -> Vec<String> {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => return vec![format!("Goals source must parse as Rust: {error}")],
    };
    let render_methods = file
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
    let mut aliases = HashMap::new();
    let mut framework_imports = HashMap::new();
    for item in &file.items {
        if let syn::Item::Use(item_use) = item {
            collect_ownership_aliases(&item_use.tree, &mut aliases);
            collect_goals_framework_imports(
                &item_use.tree,
                &mut Vec::new(),
                &mut framework_imports,
            );
        }
    }
    LocalOwnershipAliasCollector {
        aliases: &mut aliases,
    }
    .visit_block(&render.block);
    let mut local_framework_imports = HashMap::new();
    LocalGoalsFrameworkImportCollector {
        imports: &mut local_framework_imports,
    }
    .visit_block(&render.block);
    framework_imports.extend(local_framework_imports.clone());
    let mut local_bindings = GoalsLocalBindingCollector::default();
    local_bindings.visit_block(&render.block);
    if local_bindings.has_glob_import {
        local_bindings
            .names
            .extend(framework_imports.keys().cloned());
    }

    let mut visitor = GoalsRenderVisitor {
        aliases: &aliases,
        framework_imports: &framework_imports,
        local_bindings: &local_bindings.names,
        ownership: GoalsRenderOwnership::default(),
    };
    visitor.visit_block(&render.block);
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
