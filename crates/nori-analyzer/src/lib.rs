use std::collections::{BTreeMap, BTreeSet};

use nori_ast::{
    ArrowBody, BlockStmt, ClassMember, Expr, ExprKind, ForInit, FunctionDecl, MarkupAttribute,
    MarkupChild, MarkupNode, Param, Pattern, Program, Stmt, VarDecl, VarDeclarator, VarKind,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    pub signals: BTreeSet<String>,
    pub computeds: BTreeSet<String>,
    pub value_reads: BTreeSet<String>,
    pub value_writes: BTreeSet<String>,
    pub effects: usize,
    pub runtime_symbols: BTreeSet<String>,
    pub imports: Vec<String>,
    pub nori_imports: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl Analysis {
    pub fn from_program(_source: &str, program: &Program) -> Self {
        Analyzer::new().analyze(program)
    }
}

struct Analyzer {
    analysis: Analysis,
    scopes: Vec<Scope>,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            analysis: Analysis::default(),
            scopes: vec![Scope::function()],
        }
    }

    fn analyze(mut self, program: &Program) -> Analysis {
        for stmt in &program.body {
            self.visit_stmt(stmt);
        }
        self.analysis
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Var(var) => self.visit_var(var),
            Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
                self.visit_function(function);
            }
            Stmt::Return(Some(expr), _) | Stmt::Expr(expr) | Stmt::ExportDefaultExpr(expr) => {
                self.visit_expr(expr);
            }
            Stmt::Block(block) => self.visit_block(block),
            Stmt::If(stmt) => {
                self.visit_expr(&stmt.condition);
                self.visit_stmt(&stmt.consequent);
                if let Some(alternate) = &stmt.alternate {
                    self.visit_stmt(alternate);
                }
            }
            Stmt::Class(class) => {
                self.declare_lexical(&class.name, Binding::Local);
                for member in &class.members {
                    self.visit_class_member(member);
                }
            }
            Stmt::Try(stmt) => {
                self.visit_block(&stmt.body);

                self.push_block_scope();
                if let Some(param) = &stmt.catch_param {
                    self.declare_lexical(param, Binding::Local);
                }
                self.visit_block_body(&stmt.catch_body);
                self.pop_scope();

                if let Some(finally_body) = &stmt.finally_body {
                    self.visit_block(finally_body);
                }
            }
            Stmt::For(stmt) => {
                self.visit_expr(&stmt.iterable);
                self.push_block_scope();
                self.declare_var(stmt.variable, &stmt.name, Binding::Local);
                self.visit_stmt(&stmt.body);
                self.pop_scope();
            }
            Stmt::ClassicFor(stmt) => {
                self.push_block_scope();
                if let Some(init) = &stmt.init {
                    match init {
                        ForInit::Var(var) => self.visit_var(var),
                        ForInit::Expr(expr) => self.visit_expr(expr),
                    }
                }
                if let Some(condition) = &stmt.condition {
                    self.visit_expr(condition);
                }
                if let Some(update) = &stmt.update {
                    self.visit_expr(update);
                }
                self.visit_stmt(&stmt.body);
                self.pop_scope();
            }
            Stmt::While(stmt) => {
                self.visit_expr(&stmt.condition);
                self.visit_stmt(&stmt.body);
            }
            Stmt::DoWhile(stmt) => {
                self.visit_stmt(&stmt.body);
                self.visit_expr(&stmt.condition);
            }
            Stmt::Switch(stmt) => {
                self.visit_expr(&stmt.discriminant);
                for case in &stmt.cases {
                    if let Some(test) = &case.test {
                        self.visit_expr(test);
                    }
                    for s in &case.consequent {
                        self.visit_stmt(s);
                    }
                }
            }
            Stmt::Throw(stmt) => {
                self.visit_expr(&stmt.argument);
            }
            Stmt::Label(stmt) => {
                self.visit_stmt(&stmt.body);
            }
            Stmt::Debugger(_) => {}
            Stmt::With(stmt) => {
                self.visit_expr(&stmt.object);
                self.visit_stmt(&stmt.body);
            }
            Stmt::Import(import) => {
                if import.source.ends_with(".nori") {
                    self.analysis.nori_imports.push(import.source.clone());
                } else if !import.source.starts_with('.') && !import.source.starts_with('@') {
                    self.analysis.imports.push(import.source.clone());
                }
            }
            Stmt::Export(_)
            | Stmt::TypeOnly(_)
            | Stmt::Return(None, _)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Raw(_) => {}
        }
    }

    fn visit_function(&mut self, function: &FunctionDecl) {
        if let Some(name) = &function.name {
            self.declare_lexical(name, Binding::Local);
        }

        self.push_function_scope();
        for param in &function.params {
            self.visit_param(param);
        }
        self.visit_block_body(&function.body);
        self.pop_scope();
    }

    fn visit_class_member(&mut self, member: &ClassMember) {
        match member {
            ClassMember::Field(field) => {
                if let Some(computed) = &field.computed {
                    self.visit_expr(computed);
                }
                if let Some(value) = &field.value {
                    self.visit_expr(value);
                }
            }
            ClassMember::Constructor(constructor) => {
                self.visit_callable_body(&constructor.params, &constructor.body);
            }
            ClassMember::Method(method) => {
                if let Some(computed) = &method.computed {
                    self.visit_expr(computed);
                }
                self.visit_callable_body(&method.params, &method.body);
            }
            ClassMember::Accessor(accessor) => {
                if let Some(computed) = &accessor.computed {
                    self.visit_expr(computed);
                }
                self.visit_callable_body(&accessor.params, &accessor.body);
            }
            ClassMember::StaticBlock(block) => {
                self.visit_block(&block.body);
            }
        }
    }

    fn visit_callable_body(&mut self, params: &[Param], body: &BlockStmt) {
        self.push_function_scope();
        for param in params {
            self.visit_param(param);
        }
        self.visit_block_body(body);
        self.pop_scope();
    }

    fn visit_param(&mut self, param: &Param) {
        self.declare_lexical(&param.name, Binding::Local);
        if let Some(default) = &param.default {
            self.visit_expr(default);
        }
    }

    fn visit_block(&mut self, block: &BlockStmt) {
        self.push_block_scope();
        self.visit_block_body(block);
        self.pop_scope();
    }

    fn visit_block_body(&mut self, block: &BlockStmt) {
        for stmt in &block.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_var(&mut self, var: &VarDecl) {
        for declarator in &var.declarators {
            let binding = reactive_binding(declarator);
            self.declare_declarator(var.kind, declarator, binding);

            match binding {
                Binding::Signal => {
                    self.analysis.signals.insert(declarator.name.clone());
                    self.analysis.runtime_symbols.insert("signal".to_string());
                }
                Binding::Computed => {
                    self.analysis.computeds.insert(declarator.name.clone());
                    self.analysis.runtime_symbols.insert("computed".to_string());
                }
                Binding::Local => {}
            }

            if let Some(init) = &declarator.init {
                self.visit_expr(init);
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        self.visit_expr_with_access(expr, ValueAccess::Read);
    }

    fn visit_expr_with_access(&mut self, expr: &Expr, access: ValueAccess) {
        if let Some(name) = self.reactive_value_name(expr) {
            self.record_value_access(name, access);
        }

        match &expr.kind {
            ExprKind::Call { callee, args, .. } => {
                if let ExprKind::Ident(name) = &callee.kind {
                    match name.as_str() {
                        "$state" => {
                            self.analysis.runtime_symbols.insert("signal".to_string());
                        }
                        "$derived" => {
                            self.analysis.runtime_symbols.insert("computed".to_string());
                        }
                        "$effect" => {
                            self.analysis.runtime_symbols.insert("effect".to_string());
                            self.analysis.effects += 1;
                        }
                        _ => {}
                    }
                }
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::New { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::Delete(expr) | ExprKind::Void(expr) | ExprKind::Typeof(expr) => {
                self.visit_expr(expr)
            }
            ExprKind::Unary { expr, .. } => self.visit_expr(expr),
            ExprKind::Update { expr, .. } => {
                self.visit_expr_with_access(expr, ValueAccess::ReadWrite);
            }
            ExprKind::TypeErasure { expr, .. } => {
                self.visit_expr_with_access(expr, access);
            }
            ExprKind::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            ExprKind::Assign { left, op, right } => {
                self.visit_expr_with_access(left, assignment_access(op));
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
            ExprKind::Arrow { body, .. } => match body {
                ArrowBody::Expression(expr) => self.visit_expr(expr),
                ArrowBody::Block(block) => self.visit_block(block),
            },
            ExprKind::TemplateLiteral { quasis: _, exprs } => {
                for expr in exprs {
                    self.visit_expr(expr);
                }
            }
            ExprKind::TaggedTemplate { tag, quasi } => {
                self.visit_expr(tag);
                self.visit_expr(quasi);
            }
            ExprKind::Await(expr) => self.visit_expr(expr),
            ExprKind::Spread { expr } => self.visit_expr(expr),
            ExprKind::Array(items) => {
                for item in items {
                    self.visit_expr(item);
                }
            }
            ExprKind::Object(properties) => {
                for prop in properties {
                    self.visit_expr(&prop.value);
                }
            }
            ExprKind::Yield { value, .. } => {
                if let Some(expr) = value {
                    self.visit_expr(expr);
                }
            }
            ExprKind::Sequence(exprs) => {
                for expr in exprs {
                    self.visit_expr(expr);
                }
            }
            ExprKind::Import(expr) => self.visit_expr(expr),
            ExprKind::Markup(node) => self.visit_markup_node(node),
            ExprKind::RegExp { .. }
            | ExprKind::Ident(_)
            | ExprKind::Number(_)
            | ExprKind::BigInt(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::This
            | ExprKind::MetaProperty { .. }
            | ExprKind::Raw => {}
        }
    }

    fn visit_markup_node(&mut self, node: &MarkupNode) {
        match node {
            MarkupNode::Element(element) => {
                for attribute in &element.attributes {
                    match attribute {
                        MarkupAttribute::Named {
                            value: Some(value), ..
                        }
                        | MarkupAttribute::Spread { expr: value, .. } => self.visit_expr(value),
                        MarkupAttribute::Named { value: None, .. } => {}
                    }
                }
                self.visit_markup_children(&element.children);
            }
            MarkupNode::Fragment { children, .. } => self.visit_markup_children(children),
        }
    }

    fn visit_markup_children(&mut self, children: &[MarkupChild]) {
        for child in children {
            match child {
                MarkupChild::Expr(expr) => self.visit_expr(expr),
                MarkupChild::Node(node) => self.visit_markup_node(node),
                MarkupChild::Text(_, _) => {}
            }
        }
    }

    fn declare_declarator(&mut self, kind: VarKind, declarator: &VarDeclarator, binding: Binding) {
        if let Some(pattern) = &declarator.pattern {
            self.declare_pattern(kind, pattern);
        } else {
            self.declare_var(kind, &declarator.name, binding);
        }
    }

    fn declare_pattern(&mut self, kind: VarKind, pattern: &Pattern) {
        match pattern {
            Pattern::Ident(name) => self.declare_var(kind, name, Binding::Local),
            Pattern::Array { elements, rest, .. } => {
                for element in elements.iter().flatten() {
                    self.declare_pattern(kind, element);
                }
                if let Some(rest) = rest {
                    self.declare_pattern(kind, rest);
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                for prop in properties {
                    if let Some(value) = &prop.value {
                        self.declare_pattern(kind, value);
                    } else {
                        self.declare_var(kind, &prop.key, Binding::Local);
                    }
                }
                if let Some(rest) = rest {
                    self.declare_pattern(kind, rest);
                }
            }
            Pattern::Rest(pattern) => self.declare_pattern(kind, pattern),
            Pattern::Assign { left, .. } => self.declare_pattern(kind, left),
        }
    }

    fn declare_var(&mut self, kind: VarKind, name: &str, binding: Binding) {
        if kind == VarKind::Var {
            self.declare_function_scoped(name, binding);
        } else {
            self.declare_lexical(name, binding);
        }
    }

    fn declare_lexical(&mut self, name: &str, binding: Binding) {
        self.scopes
            .last_mut()
            .expect("analyzer always has a scope")
            .bindings
            .insert(name.to_string(), binding);
    }

    fn declare_function_scoped(&mut self, name: &str, binding: Binding) {
        self.scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.function)
            .expect("analyzer always has a function scope")
            .bindings
            .insert(name.to_string(), binding);
    }

    fn reactive_value_name<'expr>(&self, expr: &'expr Expr) -> Option<&'expr str> {
        let expr = erase_type_wrappers(expr);
        let ExprKind::Member {
            object, property, ..
        } = &expr.kind
        else {
            return None;
        };
        if property != "value" {
            return None;
        }
        let object = erase_type_wrappers(object);
        let ExprKind::Ident(name) = &object.kind else {
            return None;
        };
        self.is_reactive(name).then_some(name)
    }

    fn is_reactive(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name))
            .copied()
            .is_some_and(Binding::is_reactive)
    }

    fn record_value_access(&mut self, name: &str, access: ValueAccess) {
        if access.reads() {
            self.analysis.value_reads.insert(name.to_string());
        }
        if access.writes() {
            self.analysis.value_writes.insert(name.to_string());
        }
    }

    fn push_block_scope(&mut self) {
        self.scopes.push(Scope::block());
    }

    fn push_function_scope(&mut self) {
        self.scopes.push(Scope::function());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

#[derive(Debug, Default)]
struct Scope {
    bindings: BTreeMap<String, Binding>,
    function: bool,
}

impl Scope {
    fn block() -> Self {
        Self::default()
    }

    fn function() -> Self {
        Self {
            function: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Binding {
    Local,
    Signal,
    Computed,
}

impl Binding {
    fn is_reactive(self) -> bool {
        matches!(self, Self::Signal | Self::Computed)
    }
}

#[derive(Debug, Clone, Copy)]
enum ValueAccess {
    Read,
    Write,
    ReadWrite,
}

impl ValueAccess {
    fn reads(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    fn writes(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

fn reactive_binding(declarator: &VarDeclarator) -> Binding {
    let Some(init) = &declarator.init else {
        return Binding::Local;
    };
    match primitive_call_name(init) {
        Some("$state") => Binding::Signal,
        Some("$derived") => Binding::Computed,
        _ => Binding::Local,
    }
}

fn assignment_access(op: &str) -> ValueAccess {
    if op == "=" {
        ValueAccess::Write
    } else {
        ValueAccess::ReadWrite
    }
}

pub fn primitive_call_name(expr: &Expr) -> Option<&str> {
    let expr = erase_type_wrappers(expr);
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return None;
    };
    let ExprKind::Ident(name) = &callee.kind else {
        return None;
    };
    matches!(name.as_str(), "$state" | "$derived" | "$effect").then_some(name)
}

fn erase_type_wrappers(mut expr: &Expr) -> &Expr {
    while let ExprKind::TypeErasure { expr: inner, .. } = &expr.kind {
        expr = inner;
    }
    expr
}
