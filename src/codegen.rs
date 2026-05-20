use std::collections::BTreeSet;

use crate::{
    CompileOptions,
    analyzer::{Analysis, primitive_call_name},
    ast::{BlockStmt, Expr, ExprKind, FunctionDecl, IfStmt, Program, Stmt, VarDecl, VarKind},
};

pub fn generate(
    source: &str,
    program: &Program,
    analysis: &Analysis,
    options: &CompileOptions,
) -> String {
    let mut out = String::new();
    let mut emitted_runtime = false;

    if !analysis.runtime_symbols.is_empty() && !has_runtime_import(source, &options.runtime_import)
    {
        out.push_str(&runtime_import(
            &analysis.runtime_symbols,
            &options.runtime_import,
        ));
        out.push('\n');
        emitted_runtime = true;
    }

    for (idx, stmt) in program.body.iter().enumerate() {
        if idx > 0 || emitted_runtime {
            out.push('\n');
        }
        emit_stmt(source, stmt, &mut out, 0);
    }

    out.trim_end().to_string()
}

fn emit_stmt(source: &str, stmt: &Stmt, out: &mut String, indent: usize) {
    match stmt {
        Stmt::Import(raw) | Stmt::Raw(raw) => {
            push_indent(out, indent);
            out.push_str(source_slice(source, raw.span.start, raw.span.end).trim());
        }
        Stmt::TypeOnly(_) => {}
        Stmt::Var(var) => emit_var(source, var, out, indent),
        Stmt::Function(function) => emit_function(source, function, out, indent, false),
        Stmt::ExportDefaultFunction(function) => emit_function(source, function, out, indent, true),
        Stmt::ExportDefaultExpr(expr) => {
            push_indent(out, indent);
            out.push_str("export default ");
            emit_expr(source, expr, out);
            out.push(';');
        }
        Stmt::Return(expr, _) => {
            push_indent(out, indent);
            out.push_str("return");
            if let Some(expr) = expr {
                out.push(' ');
                emit_expr(source, expr, out);
            }
            out.push(';');
        }
        Stmt::Expr(expr) => {
            push_indent(out, indent);
            emit_expr(source, expr, out);
            out.push(';');
        }
        Stmt::Block(block) => emit_block(source, block, out, indent),
        Stmt::If(stmt) => emit_if(source, stmt, out, indent),
    }
}

fn emit_var(source: &str, var: &VarDecl, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str(match var.kind {
        VarKind::Const => "const ",
        VarKind::Let => "let ",
        VarKind::Var => "var ",
    });

    for (idx, declarator) in var.declarators.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&declarator.name);
        if let Some(init) = &declarator.init {
            out.push_str(" = ");
            emit_expr(source, init, out);
        }
    }
    out.push(';');
}

fn emit_function(
    source: &str,
    function: &FunctionDecl,
    out: &mut String,
    indent: usize,
    export_default: bool,
) {
    push_indent(out, indent);
    if export_default {
        out.push_str("export default ");
    }
    out.push_str("function");
    if let Some(name) = &function.name {
        out.push(' ');
        out.push_str(name);
    }
    out.push('(');
    for (idx, param) in function.params.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        if let Some(default) = &param.default {
            out.push_str(" = ");
            emit_expr(source, default, out);
        }
    }
    out.push_str(") ");
    emit_block(source, &function.body, out, indent);
}

fn emit_block(source: &str, block: &BlockStmt, out: &mut String, indent: usize) {
    out.push_str("{\n");
    for stmt in &block.body {
        emit_stmt(source, stmt, out, indent + 1);
        out.push('\n');
    }
    push_indent(out, indent);
    out.push('}');
}

fn emit_if(source: &str, stmt: &IfStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("if (");
    emit_expr(source, &stmt.condition, out);
    out.push_str(") ");
    emit_stmt(source, &stmt.consequent, out, 0);
    if let Some(alternate) = &stmt.alternate {
        out.push_str(" else ");
        emit_stmt(source, alternate, out, 0);
    }
}

fn emit_expr(source: &str, expr: &Expr, out: &mut String) {
    if let Some(name) = primitive_call_name(expr) {
        emit_primitive_call(source, expr, name, out);
        return;
    }

    match &expr.kind {
        ExprKind::Ident(name) => out.push_str(name),
        ExprKind::Number(number) | ExprKind::String(number) => out.push_str(number),
        ExprKind::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        ExprKind::Null => out.push_str("null"),
        ExprKind::Unary { op, expr } => {
            out.push_str(op);
            emit_expr(source, expr, out);
        }
        ExprKind::Binary { left, op, right } | ExprKind::Assign { left, op, right } => {
            emit_expr(source, left, out);
            out.push(' ');
            out.push_str(op);
            out.push(' ');
            emit_expr(source, right, out);
        }
        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            emit_expr(source, test, out);
            out.push_str(" ? ");
            emit_expr(source, consequent, out);
            out.push_str(" : ");
            emit_expr(source, alternate, out);
        }
        ExprKind::Call { callee, args } => {
            emit_expr(source, callee, out);
            out.push('(');
            for (idx, arg) in args.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                emit_expr(source, arg, out);
            }
            out.push(')');
        }
        ExprKind::Member { object, property } => {
            emit_expr(source, object, out);
            out.push('.');
            out.push_str(property);
        }
        ExprKind::Arrow { params, body } => {
            if params.len() == 1 {
                out.push_str(&params[0]);
            } else {
                out.push('(');
                out.push_str(&params.join(", "));
                out.push(')');
            }
            out.push_str(" => ");
            emit_expr(source, body, out);
        }
        ExprKind::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                emit_expr(source, item, out);
            }
            out.push(']');
        }
        ExprKind::Object | ExprKind::Markup(_) | ExprKind::Raw => {
            out.push_str(source_slice(source, expr.span.start, expr.span.end).trim());
        }
    }
}

fn emit_primitive_call(source: &str, expr: &Expr, name: &str, out: &mut String) {
    let ExprKind::Call { args, .. } = &expr.kind else {
        return;
    };
    match name {
        "$state" => {
            out.push_str("signal(");
            emit_arg_list(source, args, out);
            out.push(')');
        }
        "$derived" => {
            out.push_str("computed(() => ");
            if let Some(first) = args.first() {
                emit_expr(source, first, out);
            }
            out.push(')');
        }
        "$effect" => {
            out.push_str("effect(");
            emit_arg_list(source, args, out);
            out.push(')');
        }
        _ => {}
    }
}

fn emit_arg_list(source: &str, args: &[Expr], out: &mut String) {
    for (idx, arg) in args.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        emit_expr(source, arg, out);
    }
}

fn has_runtime_import(source: &str, runtime_import: &str) -> bool {
    source.contains(&format!("from '{runtime_import}'"))
        || source.contains(&format!("from \"{runtime_import}\""))
}

fn runtime_import(symbols: &BTreeSet<String>, runtime_import: &str) -> String {
    let symbols = symbols.iter().cloned().collect::<Vec<_>>().join(", ");
    format!("import {{ {symbols} }} from \"{runtime_import}\";")
}

fn source_slice(source: &str, start: usize, end: usize) -> &str {
    source.get(start..end).unwrap_or_default()
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}
