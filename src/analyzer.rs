use std::collections::BTreeSet;

use crate::ast::{Expr, ExprKind, Program, Stmt, VarDecl};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    pub signals: BTreeSet<String>,
    pub computeds: BTreeSet<String>,
    pub effects: usize,
    pub runtime_symbols: BTreeSet<String>,
    pub diagnostics: Vec<String>,
}

impl Analysis {
    pub fn from_program(program: &Program) -> Self {
        let mut analysis = Self::default();
        for stmt in &program.body {
            analysis.visit_stmt(stmt);
        }
        analysis
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Var(var) => self.visit_var(var),
            Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
                for stmt in &function.body.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Return(Some(expr), _) | Stmt::Expr(expr) | Stmt::ExportDefaultExpr(expr) => {
                self.visit_expr(expr);
            }
            Stmt::Block(block) => {
                for stmt in &block.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::If(stmt) => {
                self.visit_expr(&stmt.condition);
                self.visit_stmt(&stmt.consequent);
                if let Some(alternate) = &stmt.alternate {
                    self.visit_stmt(alternate);
                }
            }
            Stmt::Import(_) | Stmt::TypeOnly(_) | Stmt::Return(None, _) | Stmt::Raw(_) => {}
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
            ExprKind::Arrow { body, .. } => self.visit_expr(body),
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

pub fn primitive_call_name(expr: &Expr) -> Option<&str> {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return None;
    };
    let ExprKind::Ident(name) = &callee.kind else {
        return None;
    };
    matches!(name.as_str(), "$state" | "$derived" | "$effect").then_some(name)
}
