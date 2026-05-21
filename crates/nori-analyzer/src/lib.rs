use std::collections::BTreeSet;

use nori_ast::{Expr, ExprKind, Program, Stmt, VarDecl};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    pub signals: BTreeSet<String>,
    pub computeds: BTreeSet<String>,
    pub effects: usize,
    pub runtime_symbols: BTreeSet<String>,
    pub imports: Vec<String>,
    pub nori_imports: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl Analysis {
    pub fn from_program(source: &str, program: &Program) -> Self {
        let mut analysis = Self::default();
        for stmt in &program.body {
            analysis.visit_stmt(source, stmt);
        }
        analysis
    }

    fn visit_stmt(&mut self, source: &str, stmt: &Stmt) {
        match stmt {
            Stmt::Var(var) => self.visit_var(var),
            Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
                for stmt in &function.body.body {
                    self.visit_stmt(source, stmt);
                }
            }
            Stmt::Return(Some(expr), _) | Stmt::Expr(expr) | Stmt::ExportDefaultExpr(expr) => {
                self.visit_expr(expr);
            }
            Stmt::Block(block) => {
                for stmt in &block.body {
                    self.visit_stmt(source, stmt);
                }
            }
            Stmt::If(stmt) => {
                self.visit_expr(&stmt.condition);
                self.visit_stmt(source, &stmt.consequent);
                if let Some(alternate) = &stmt.alternate {
                    self.visit_stmt(source, alternate);
                }
            }
            Stmt::Class(class) => {
                for stmt in &class.body {
                    self.visit_stmt(source, stmt);
                }
            }
            Stmt::Try(stmt) => {
                for s in &stmt.body.body {
                    self.visit_stmt(source, s);
                }
                for s in &stmt.catch_body.body {
                    self.visit_stmt(source, s);
                }
                if let Some(finally_body) = &stmt.finally_body {
                    for s in &finally_body.body {
                        self.visit_stmt(source, s);
                    }
                }
            }
            Stmt::For(stmt) => {
                self.visit_expr(&stmt.iterable);
                for s in &stmt.body.body {
                    self.visit_stmt(source, s);
                }
            }
            Stmt::Import(raw) => {
                let span = &raw.span;
                if let Some(import_path) = extract_import_path(source, span.start, span.end) {
                    if import_path.ends_with(".nori") {
                        self.nori_imports.push(import_path);
                    } else if !import_path.starts_with('.') && !import_path.starts_with('@') {
                        self.imports.push(import_path);
                    }
                }
            }
            Stmt::TypeOnly(_) | Stmt::Return(None, _) | Stmt::Raw(_) => {}
        }
    }

    fn visit_var(&mut self, var: &VarDecl) {
        for declarator in &var.declarators {
            if let Some(init) = &declarator.init {
                if let Some(name) = primitive_call_name(init) {
                    match name {
                        "$state" => {
                            self.signals.insert(declarator.name.clone());
                            self.runtime_symbols.insert("signal".to_string());
                        }
                        "$derived" => {
                            self.computeds.insert(declarator.name.clone());
                            self.runtime_symbols.insert("computed".to_string());
                        }
                        "$effect" => {
                            self.effects += 1;
                            self.runtime_symbols.insert("effect".to_string());
                        }
                        _ => {}
                    }
                }
                self.visit_expr(init);
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => {
                if let ExprKind::Ident(name) = &callee.kind {
                    match name.as_str() {
                        "$state" => {
                            self.runtime_symbols.insert("signal".to_string());
                        }
                        "$derived" => {
                            self.runtime_symbols.insert("computed".to_string());
                        }
                        "$effect" => {
                            self.runtime_symbols.insert("effect".to_string());
                            self.effects += 1;
                        }
                        _ => {}
                    }
                }
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::Unary { expr, .. } => self.visit_expr(expr),
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
            ExprKind::Index { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            ExprKind::Arrow { body, .. } => self.visit_expr(body),
            ExprKind::Await(expr) => self.visit_expr(expr),
            ExprKind::Spread { expr } => self.visit_expr(expr),
            ExprKind::Array(items) => {
                for item in items {
                    self.visit_expr(item);
                }
            }
            ExprKind::Ident(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Object
            | ExprKind::Markup(_)
            | ExprKind::Raw => {}
        }
    }
}

fn extract_import_path(source: &str, start: usize, end: usize) -> Option<String> {
    let import_text = source.get(start..end)?;
    let after_from = import_text.strip_prefix("import")?.split("from").nth(1)?;
    let after_from = after_from.trim();
    let quote = after_from.chars().next()?;
    let path_end = after_from[1..].find(quote)? + 2;
    Some(after_from[..path_end].to_string())
}

pub fn primitive_call_name(expr: &Expr) -> Option<&str> {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return None;
    };
    let ExprKind::Ident(name) = &callee.kind else {
        return None;
    };
    matches!(name.as_str(), "$state" | "$derived" | "$effect").then_some(name)
}