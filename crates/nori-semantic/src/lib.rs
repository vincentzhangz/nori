//! Semantic analysis for Nori (oxc_semantic analog).
//!
//! Walks the arena AST (via [`nori_ast::Visit`]) to build a [`SemanticModel`]:
//! a scope tree, a flat symbol table, and a list of identifier references
//! resolved (or left unresolved) against that scope tree.
//!
//! The model intentionally owns its data (`String`s instead of `Atom<'a>`s)
//! so it can outlive the arena/allocator used to parse the program, which
//! keeps downstream consumers (like `nori-checker`) simple.

use std::collections::BTreeMap;

use nori_ast::{
    ArrowBody, BlockStmt, ClassDecl, ClassMember, Expr, ExprKind, ForInit, FunctionDecl,
    ImportDecl, ImportSpecifier, MarkupNode, Param, Pattern, Program, PropertyKey, Stmt, VarDecl,
    VarKind, Visit, walk_markup_node,
};
use nori_span::Span;

/// Identifies a [`Symbol`] within a [`SemanticModel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

/// Identifies a [`Scope`] within a [`SemanticModel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub u32);

/// What kind of declaration introduced a [`Symbol`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Let,
    Const,
    Function,
    Class,
    Param,
    Import,
    /// Reserved for `type`/`interface` declarations once the Phase 2 type
    /// AST lands.
    Type,
}

/// A single declared binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    /// The scope the binding lives in (its home scope, e.g. `var` hoists to
    /// the nearest function/program scope rather than the block it appears
    /// in).
    pub scope_id: ScopeId,
    pub span: Span,
}

/// What kind of lexical/variable scope a [`Scope`] represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Program,
    Function,
    Block,
    Class,
}

/// A node in the scope tree.
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    /// Bindings declared directly in this scope, keyed by name.
    pub bindings: BTreeMap<String, SymbolId>,
}

/// An identifier use-site, resolved against the scope chain at build time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub name: String,
    pub span: Span,
    pub scope_id: ScopeId,
    /// `None` when the identifier didn't resolve to any declared binding
    /// (e.g. a global or a typo).
    pub symbol_id: Option<SymbolId>,
}

/// The result of running [`build_semantic`] over a [`Program`].
#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub scopes: Vec<Scope>,
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub root_scope: ScopeId,
}

impl Default for SemanticModel {
    fn default() -> Self {
        let root = ScopeId(0);
        Self {
            scopes: vec![Scope {
                id: root,
                kind: ScopeKind::Program,
                parent: None,
                children: Vec::new(),
                bindings: BTreeMap::new(),
            }],
            symbols: Vec::new(),
            references: Vec::new(),
            root_scope: root,
        }
    }
}

impl SemanticModel {
    pub fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }

    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Walks up the scope chain from `scope_id` looking for a binding named
    /// `name`, returning the nearest one (JS/TS shadowing semantics).
    pub fn resolve_binding(&self, scope_id: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current = Some(scope_id);
        while let Some(id) = current {
            let scope = self.scope(id);
            if let Some(symbol_id) = scope.bindings.get(name) {
                return Some(*symbol_id);
            }
            current = scope.parent;
        }
        None
    }

    /// References that failed to resolve to any declared symbol.
    pub fn unresolved_references(&self) -> impl Iterator<Item = &Reference> {
        self.references.iter().filter(|r| r.symbol_id.is_none())
    }
}

/// Builds a [`SemanticModel`] for `program`.
pub fn build_semantic(program: &Program<'_>) -> SemanticModel {
    SemanticBuilder::new().build(program)
}

/// Walks the AST via [`nori_ast::Visit`], maintaining a scope stack and
/// declaring/resolving symbols as it goes.
struct SemanticBuilder {
    model: SemanticModel,
    scope_stack: Vec<ScopeId>,
}

impl SemanticBuilder {
    fn new() -> Self {
        let root = ScopeId(0);
        let model = SemanticModel {
            scopes: vec![Scope {
                id: root,
                kind: ScopeKind::Program,
                parent: None,
                children: Vec::new(),
                bindings: BTreeMap::new(),
            }],
            symbols: Vec::new(),
            references: Vec::new(),
            root_scope: root,
        };
        Self {
            model,
            scope_stack: vec![root],
        }
    }

    fn build(mut self, program: &Program<'_>) -> SemanticModel {
        self.visit_program(program);
        self.model
    }

    fn current_scope(&self) -> ScopeId {
        *self
            .scope_stack
            .last()
            .expect("semantic builder always has a scope")
    }

    fn push_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.model.scopes.len() as u32);
        let parent = self.current_scope();
        self.model.scopes.push(Scope {
            id,
            kind,
            parent: Some(parent),
            children: Vec::new(),
            bindings: BTreeMap::new(),
        });
        self.model.scopes[parent.0 as usize].children.push(id);
        self.scope_stack.push(id);
        id
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// `var`-kind bindings hoist to the nearest function (or program) scope;
    /// everything else binds in the current (lexical) scope.
    fn nearest_function_scope(&self) -> ScopeId {
        for &id in self.scope_stack.iter().rev() {
            let scope = &self.model.scopes[id.0 as usize];
            if matches!(scope.kind, ScopeKind::Function | ScopeKind::Program) {
                return id;
            }
        }
        self.model.root_scope
    }

    fn declare(&mut self, name: &str, kind: SymbolKind, span: Span) -> SymbolId {
        let scope_id = if matches!(kind, SymbolKind::Var) {
            self.nearest_function_scope()
        } else {
            self.current_scope()
        };
        let id = SymbolId(self.model.symbols.len() as u32);
        self.model.symbols.push(Symbol {
            id,
            name: name.to_string(),
            kind,
            scope_id,
            span,
        });
        self.model.scopes[scope_id.0 as usize]
            .bindings
            .insert(name.to_string(), id);
        id
    }

    fn reference(&mut self, name: &str, span: Span) {
        let scope_id = self.current_scope();
        let symbol_id = self.model.resolve_binding(scope_id, name);
        self.model.references.push(Reference {
            name: name.to_string(),
            span,
            scope_id,
            symbol_id,
        });
    }

    fn var_symbol_kind(kind: VarKind) -> SymbolKind {
        match kind {
            VarKind::Var => SymbolKind::Var,
            VarKind::Let => SymbolKind::Let,
            VarKind::Const => SymbolKind::Const,
        }
    }

    fn visit_var_decl(&mut self, var: &VarDecl<'_>) {
        let kind = Self::var_symbol_kind(var.kind);
        for declarator in &var.declarators {
            if let Some(pattern) = &declarator.pattern {
                self.declare_pattern(pattern, kind, declarator.span);
            } else {
                self.declare(declarator.name.as_str(), kind, declarator.span);
            }
            if let Some(init) = &declarator.init {
                self.visit_expr(init);
            }
        }
    }

    fn declare_pattern(&mut self, pattern: &Pattern<'_>, kind: SymbolKind, fallback_span: Span) {
        match pattern {
            Pattern::Ident(name) => {
                self.declare(name.as_str(), kind, fallback_span);
            }
            Pattern::Rest(inner) => self.declare_pattern(inner, kind, fallback_span),
            Pattern::Array {
                elements, rest, ..
            } => {
                for element in elements.iter().flatten() {
                    self.declare_pattern(element, kind, fallback_span);
                }
                if let Some(rest) = rest {
                    self.declare_pattern(rest, kind, fallback_span);
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                for prop in properties {
                    if let Some(value) = &prop.value {
                        self.declare_pattern(value, kind, prop.span);
                    } else {
                        self.declare(prop.key.as_str(), kind, prop.span);
                    }
                    if let Some(default) = &prop.default {
                        self.visit_expr(default);
                    }
                }
                if let Some(rest) = rest {
                    self.declare_pattern(rest, kind, fallback_span);
                }
            }
            Pattern::Assign { left, right } => {
                self.declare_pattern(left, kind, fallback_span);
                self.visit_expr(right);
            }
        }
    }

    fn visit_function_decl(&mut self, function: &FunctionDecl<'_>) {
        if let Some(name) = &function.name {
            self.declare(name.as_str(), SymbolKind::Function, function.span);
        }
        self.push_scope(ScopeKind::Function);
        self.declare_params(&function.params, function.span);
        for stmt in &function.body.body {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    fn declare_params(&mut self, params: &[Param<'_>], fallback_span: Span) {
        for param in params {
            self.declare(param.name.as_str(), SymbolKind::Param, fallback_span);
            if let Some(default) = &param.default {
                self.visit_expr(default);
            }
        }
    }

    fn visit_callable(&mut self, params: &[Param<'_>], body: &BlockStmt<'_>) {
        self.push_scope(ScopeKind::Function);
        self.declare_params(params, body.span);
        for stmt in &body.body {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    fn visit_class_decl(&mut self, class: &ClassDecl<'_>) {
        self.declare(class.name.as_str(), SymbolKind::Class, class.span);
        self.push_scope(ScopeKind::Class);
        for member in &class.members {
            self.visit_class_member(member);
        }
        self.pop_scope();
    }

    fn visit_class_member(&mut self, member: &ClassMember<'_>) {
        match member {
            ClassMember::Field(field) => {
                if let Some(computed) = &field.computed {
                    self.visit_expr(computed);
                }
                if let Some(value) = &field.value {
                    self.visit_expr(value);
                }
            }
            ClassMember::Constructor(ctor) => self.visit_callable(&ctor.params, &ctor.body),
            ClassMember::Method(method) => {
                if let Some(computed) = &method.computed {
                    self.visit_expr(computed);
                }
                self.visit_callable(&method.params, &method.body);
            }
            ClassMember::Accessor(accessor) => {
                if let Some(computed) = &accessor.computed {
                    self.visit_expr(computed);
                }
                self.visit_callable(&accessor.params, &accessor.body);
            }
            ClassMember::StaticBlock(block) => {
                for stmt in &block.body.body {
                    self.visit_stmt(stmt);
                }
            }
        }
    }

    fn visit_import_decl(&mut self, import: &ImportDecl<'_>) {
        for specifier in &import.specifiers {
            match specifier {
                ImportSpecifier::Default(name) | ImportSpecifier::Namespace(name) => {
                    self.declare(name.as_str(), SymbolKind::Import, import.span);
                }
                ImportSpecifier::Named { local, .. } => {
                    self.declare(local.as_str(), SymbolKind::Import, import.span);
                }
            }
        }
    }
}

impl<'a> Visit<'a> for SemanticBuilder {
    fn visit_program(&mut self, program: &Program<'a>) {
        for stmt in &program.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt<'a>) {
        match stmt {
            Stmt::Var(var) => self.visit_var_decl(var),
            Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
                self.visit_function_decl(function);
            }
            Stmt::Return(Some(expr), _)
            | Stmt::Expr(expr)
            | Stmt::ExportDefaultExpr(expr) => self.visit_expr(expr),
            Stmt::Block(block) => self.visit_block(block),
            Stmt::If(if_stmt) => {
                self.visit_expr(&if_stmt.condition);
                self.visit_stmt(&if_stmt.consequent);
                if let Some(alternate) = &if_stmt.alternate {
                    self.visit_stmt(alternate);
                }
            }
            Stmt::Class(class) => self.visit_class_decl(class),
            Stmt::Try(try_stmt) => {
                self.visit_block(&try_stmt.body);
                self.push_scope(ScopeKind::Block);
                if let Some(param) = &try_stmt.catch_param {
                    self.declare(param.as_str(), SymbolKind::Let, try_stmt.span);
                }
                for stmt in &try_stmt.catch_body.body {
                    self.visit_stmt(stmt);
                }
                self.pop_scope();
                if let Some(finally_body) = &try_stmt.finally_body {
                    self.visit_block(finally_body);
                }
            }
            Stmt::For(for_stmt) => {
                self.visit_expr(&for_stmt.iterable);
                self.push_scope(ScopeKind::Block);
                let kind = Self::var_symbol_kind(for_stmt.variable);
                self.declare(for_stmt.name.as_str(), kind, for_stmt.span);
                self.visit_stmt(&for_stmt.body);
                self.pop_scope();
            }
            Stmt::ClassicFor(for_stmt) => {
                self.push_scope(ScopeKind::Block);
                if let Some(init) = &for_stmt.init {
                    match init {
                        ForInit::Var(var) => self.visit_var_decl(var),
                        ForInit::Expr(expr) => self.visit_expr(expr),
                    }
                }
                if let Some(condition) = &for_stmt.condition {
                    self.visit_expr(condition);
                }
                if let Some(update) = &for_stmt.update {
                    self.visit_expr(update);
                }
                self.visit_stmt(&for_stmt.body);
                self.pop_scope();
            }
            Stmt::While(while_stmt) => {
                self.visit_expr(&while_stmt.condition);
                self.visit_stmt(&while_stmt.body);
            }
            Stmt::DoWhile(do_while) => {
                self.visit_stmt(&do_while.body);
                self.visit_expr(&do_while.condition);
            }
            Stmt::Switch(switch) => {
                self.visit_expr(&switch.discriminant);
                self.push_scope(ScopeKind::Block);
                for case in &switch.cases {
                    if let Some(test) = &case.test {
                        self.visit_expr(test);
                    }
                    for stmt in &case.consequent {
                        self.visit_stmt(stmt);
                    }
                }
                self.pop_scope();
            }
            Stmt::Throw(throw_stmt) => self.visit_expr(&throw_stmt.argument),
            Stmt::Label(label) => self.visit_stmt(&label.body),
            Stmt::With(with) => {
                self.visit_expr(&with.object);
                self.visit_stmt(&with.body);
            }
            Stmt::Import(import) => self.visit_import_decl(import),
            Stmt::Export(nori_ast::ExportDecl::Declaration(inner)) => self.visit_stmt(inner),
            Stmt::TypeAlias(alias) => {
                self.declare(alias.name.as_str(), SymbolKind::Type, alias.span);
            }
            Stmt::Interface(iface) => {
                self.declare(iface.name.as_str(), SymbolKind::Type, iface.span);
            }
            Stmt::Enum(enum_decl) => {
                self.declare(enum_decl.name.as_str(), SymbolKind::Var, enum_decl.span);
                for member in &enum_decl.members {
                    if let Some(init) = &member.init {
                        self.visit_expr(init);
                    }
                }
            }
            Stmt::Module(module) => self.visit_block(&module.body),
            Stmt::Export(_)
            | Stmt::TypeOnly(_)
            | Stmt::Return(None, _)
            | Stmt::Debugger(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Raw(_) => {}
        }
    }

    fn visit_block(&mut self, block: &BlockStmt<'a>) {
        self.push_scope(ScopeKind::Block);
        for stmt in &block.body {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    fn visit_expr(&mut self, expr: &Expr<'a>) {
        match &expr.kind {
            ExprKind::Ident(name) => self.reference(name.as_str(), expr.span),
            ExprKind::New { callee, args } | ExprKind::Call { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::Delete(inner)
            | ExprKind::Void(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Import(inner)
            | ExprKind::Await(inner)
            | ExprKind::Spread { expr: inner }
            | ExprKind::Unary { expr: inner, .. }
            | ExprKind::Update { expr: inner, .. }
            | ExprKind::TypeErasure { expr: inner, .. } => self.visit_expr(inner),
            ExprKind::Binary { left, right, .. } | ExprKind::Assign { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            ExprKind::Conditional {
                test,
                consequent,
                alternate,
            } => {
                self.visit_expr(test);
                self.visit_expr(consequent);
                self.visit_expr(alternate);
            }
            ExprKind::Member { object, .. } => self.visit_expr(object),
            ExprKind::Index { object, index, .. } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            ExprKind::Arrow { params, body } => {
                self.push_scope(ScopeKind::Function);
                for param in params {
                    self.declare(param.as_str(), SymbolKind::Param, expr.span);
                }
                match body {
                    ArrowBody::Expression(inner) => self.visit_expr(inner),
                    ArrowBody::Block(block) => {
                        for stmt in &block.body {
                            self.visit_stmt(stmt);
                        }
                    }
                }
                self.pop_scope();
            }
            ExprKind::TemplateLiteral { exprs, .. } | ExprKind::Sequence(exprs) => {
                for expr in exprs {
                    self.visit_expr(expr);
                }
            }
            ExprKind::TaggedTemplate { tag, quasi } => {
                self.visit_expr(tag);
                self.visit_expr(quasi);
            }
            ExprKind::Array(items) => {
                for item in items {
                    self.visit_expr(item);
                }
            }
            ExprKind::Object(properties) => {
                for prop in properties {
                    if let PropertyKey::Computed(key) = &prop.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&prop.value);
                }
            }
            ExprKind::Yield { value, .. } => {
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            ExprKind::Markup(node) => self.visit_markup_node(node),
            ExprKind::Number(_)
            | ExprKind::BigInt(_)
            | ExprKind::String(_)
            | ExprKind::RegExp { .. }
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::This
            | ExprKind::MetaProperty { .. }
            | ExprKind::Raw => {}
        }
    }

    fn visit_markup_node(&mut self, node: &MarkupNode<'a>) {
        walk_markup_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nori_allocator::Allocator;

    fn build(source: &str) -> SemanticModel {
        let allocator = Allocator::new();
        let tokens = nori_lexer::lex(source).expect("lex");
        let program =
            nori_parser::parse_in(&allocator, source, "<test>.nori", tokens).expect("parse");
        build_semantic(&program)
    }

    fn find_symbol<'m>(model: &'m SemanticModel, name: &str) -> Vec<&'m Symbol> {
        model.symbols.iter().filter(|s| s.name == name).collect()
    }

    #[test]
    fn resolves_simple_var_reference() {
        let model = build("let x = 1;\nx;");
        let symbols = find_symbol(&model, "x");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, SymbolKind::Let);

        let reference = model
            .references
            .iter()
            .find(|r| r.name == "x" && r.symbol_id.is_some())
            .expect("resolved reference to x");
        assert_eq!(reference.symbol_id, Some(symbols[0].id));
    }

    #[test]
    fn nested_block_shadowing_resolves_to_nearest_declaration() {
        let source = r#"
            let x = 1;
            {
                let x = 2;
                x;
            }
            x;
        "#;
        let model = build(source);
        let symbols = find_symbol(&model, "x");
        assert_eq!(symbols.len(), 2, "expected outer + inner `x` symbols");

        let outer = symbols
            .iter()
            .find(|s| s.scope_id == model.root_scope)
            .expect("outer x lives in root scope");
        let inner = symbols
            .iter()
            .find(|s| s.scope_id != model.root_scope)
            .expect("inner x lives in the block scope");
        assert_ne!(outer.id, inner.id);

        let resolved: Vec<_> = model
            .references
            .iter()
            .filter(|r| r.name == "x")
            .collect();
        assert_eq!(resolved.len(), 2);

        // First `x;` reference is inside the block -> resolves to the inner shadow.
        assert_eq!(resolved[0].symbol_id, Some(inner.id));
        // Second `x;` reference is back in the outer scope -> resolves to the outer symbol.
        assert_eq!(resolved[1].symbol_id, Some(outer.id));
    }

    #[test]
    fn function_scope_isolates_params_and_locals() {
        let source = r#"
            function add(a, b) {
                let sum = a + b;
                return sum;
            }
            add(1, 2);
        "#;
        let model = build(source);

        assert_eq!(find_symbol(&model, "add").len(), 1);
        assert_eq!(find_symbol(&model, "a").len(), 1);
        assert_eq!(find_symbol(&model, "b").len(), 1);
        assert_eq!(find_symbol(&model, "sum").len(), 1);

        // `sum`, `a`, and `b` should not leak into the root/program scope.
        assert!(!model.scope(model.root_scope).bindings.contains_key("sum"));
        assert!(!model.scope(model.root_scope).bindings.contains_key("a"));

        // All identifier references inside the function body resolve.
        assert_eq!(model.unresolved_references().count(), 0);
    }

    #[test]
    fn unresolved_identifiers_have_no_symbol() {
        let model = build("console.log(doesNotExist);");
        let unresolved: Vec<_> = model.unresolved_references().collect();
        assert!(unresolved.iter().any(|r| r.name == "console"));
        assert!(unresolved.iter().any(|r| r.name == "doesNotExist"));
    }

    #[test]
    fn var_hoists_to_function_scope_not_block_scope() {
        let source = r#"
            function f() {
                {
                    var hoisted = 1;
                }
                hoisted;
            }
        "#;
        let model = build(source);
        let symbols = find_symbol(&model, "hoisted");
        assert_eq!(symbols.len(), 1);

        // The binding should live in the function scope, not the inner block scope.
        let function_scope = symbols[0].scope_id;
        assert_ne!(function_scope, model.root_scope);
        assert!(
            model
                .scope(function_scope)
                .bindings
                .contains_key("hoisted")
        );

        assert_eq!(model.unresolved_references().count(), 0);
    }

    #[test]
    fn const_reassignment_is_tracked_via_symbol_kind() {
        let model = build("const x = 1;\nx = 2;");
        let symbols = find_symbol(&model, "x");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, SymbolKind::Const);
    }
}
