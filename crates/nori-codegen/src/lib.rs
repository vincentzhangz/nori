use std::collections::BTreeSet;

use nori_analyzer::{Analysis, primitive_call_name};
use nori_ast::{
    BlockStmt, DestructuringKind, DestructuringPattern, Expr, ExprKind, ForStmt, FunctionDecl,
    IfStmt, MarkupAttribute, MarkupChild, MarkupElement, MarkupNode, Program, Span, Stmt, TryStmt,
    VarDecl, VarKind,
};

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub runtime_import: String,
}

pub fn generate(
    source: &str,
    program: &Program,
    analysis: &Analysis,
    runtime_import_path: &str,
) -> String {
    let mut out = String::new();
    let mut emitted_runtime = false;

    if !analysis.runtime_symbols.is_empty() && !has_runtime_import(source, runtime_import_path) {
        out.push_str(&runtime_import_fn(
            &analysis.runtime_symbols,
            runtime_import_path,
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
        Stmt::Class(class) => {
            push_indent(out, indent);
            out.push_str("class ");
            out.push_str(&class.name);
            if let Some(extends) = &class.extends {
                out.push_str(" extends ");
                out.push_str(extends);
            }
            out.push_str(" { ");
            for stmt in &class.body {
                emit_stmt(source, stmt, out, 0);
                out.push(' ');
            }
            out.push('}');
        }
        Stmt::Try(stmt) => emit_try(source, stmt, out, indent),
        Stmt::For(stmt) => emit_for(source, stmt, out, indent),
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
        if let Some(pattern) = &declarator.pattern {
            emit_destructuring_pattern(pattern, out);
        } else {
            out.push_str(&declarator.name);
        }
        if let Some(init) = &declarator.init {
            out.push_str(" = ");
            emit_expr(source, init, out);
        }
    }
    out.push(';');
}

fn emit_destructuring_pattern(pattern: &DestructuringPattern, out: &mut String) {
    match &pattern.kind {
        DestructuringKind::Array(names, _) => {
            out.push('[');
            for (idx, name) in names.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
            }
            out.push(']');
        }
        DestructuringKind::Object(props, _) => {
            out.push('{');
            for (idx, (name, default)) in props.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                if let Some(default) = default {
                    out.push_str(" = ");
                    out.push_str(default);
                }
            }
            out.push('}');
        }
    }
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
    if function.async_token.is_some() {
        out.push_str("async ");
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

fn emit_try(source: &str, stmt: &TryStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("try ");
    emit_block(source, &stmt.body, out, indent);
    if let Some(param) = &stmt.catch_param {
        out.push_str(&format!(" catch ({param}) "));
        emit_block(source, &stmt.catch_body, out, indent);
    } else if !stmt.catch_body.body.is_empty() {
        out.push_str(" catch ");
        emit_block(source, &stmt.catch_body, out, indent);
    }
    if let Some(finally_body) = &stmt.finally_body {
        out.push_str(" finally ");
        emit_block(source, finally_body, out, indent);
    }
}

fn emit_for(source: &str, stmt: &ForStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("for (");
    out.push_str(match stmt.variable {
        VarKind::Const => "const ",
        VarKind::Let => "let ",
        VarKind::Var => "var ",
    });
    out.push_str(&stmt.name);
    out.push_str(if stmt.is_of { " of " } else { " in " });
    emit_expr(source, &stmt.iterable, out);
    out.push_str(") ");
    emit_block(source, &stmt.body, out, indent);
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
        ExprKind::Index { object, index } => {
            emit_expr(source, object, out);
            out.push('[');
            emit_expr(source, index, out);
            out.push(']');
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
        ExprKind::Spread { expr } => {
            out.push_str("...");
            emit_expr(source, expr, out);
        }
        ExprKind::Await(expr) => {
            out.push_str("await ");
            emit_expr(source, expr, out);
        }
        ExprKind::Markup(node) => emit_markup_source(source, node, out),
        ExprKind::Object | ExprKind::Raw => {
            out.push_str(source_slice(source, expr.span.start, expr.span.end).trim());
        }
    }
}

fn emit_markup_source(source: &str, node: &MarkupNode, out: &mut String) {
    let span = markup_span(node);
    let mut text = source_slice(source, span.start, span.end).to_string();
    let mut insertions = Vec::new();
    collect_button_type_insertions(node, &mut insertions);
    insertions.sort_unstable_by(|a, b| b.cmp(a));

    for insertion in insertions {
        if (span.start..=span.end).contains(&insertion) {
            text.insert_str(insertion - span.start, r#" type="button""#);
        }
    }

    out.push_str(text.trim());
}

fn collect_button_type_insertions(node: &MarkupNode, insertions: &mut Vec<usize>) {
    match node {
        MarkupNode::Element(element) => {
            collect_button_type_insertions_from_element(element, insertions)
        }
        MarkupNode::Fragment { children, .. } => {
            for child in children {
                collect_button_type_insertions_from_child(child, insertions);
            }
        }
    }
}

fn collect_button_type_insertions_from_element(
    element: &MarkupElement,
    insertions: &mut Vec<usize>,
) {
    if element.name == "button"
        && !element.attributes.iter().any(|attribute| {
            matches!(attribute, MarkupAttribute::Named { name, .. } if name.eq_ignore_ascii_case("type"))
        })
    {
        insertions.push(element.span.start + 1 + element.name.len());
    }

    for child in &element.children {
        collect_button_type_insertions_from_child(child, insertions);
    }
}

fn collect_button_type_insertions_from_child(child: &MarkupChild, insertions: &mut Vec<usize>) {
    if let MarkupChild::Node(node) = child {
        collect_button_type_insertions(node, insertions);
    }
}

fn markup_span(node: &MarkupNode) -> Span {
    match node {
        MarkupNode::Element(element) => element.span,
        MarkupNode::Fragment { span, .. } => *span,
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

fn runtime_import_fn(symbols: &BTreeSet<String>, runtime_import: &str) -> String {
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
