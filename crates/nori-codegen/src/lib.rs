use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use nori_analyzer::{Analysis, primitive_call_name};
use nori_ast::{
    ArrowBody, BlockStmt, ClassAccessor, ClassConstructor, ClassField, ClassMember, ClassMethod,
    ClassStaticBlock, ClassicForStmt, DoWhileStmt, EnumDecl, Expr, ExprKind, ForInit, ForStmt,
    FunctionDecl, IfStmt, MarkupAttribute, MarkupChild, MarkupElement, MarkupNode, Param, Pattern,
    Program, Stmt, TryStmt, VarDecl, VarKind, WhileStmt,
};

type ConstEnumTable = BTreeMap<String, BTreeMap<String, String>>;

thread_local! {
    static CONST_ENUMS: RefCell<ConstEnumTable> = const { RefCell::new(BTreeMap::new()) };
}

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
    let const_enums = collect_const_enums(source, program);
    CONST_ENUMS.with(|slot| {
        *slot.borrow_mut() = const_enums;
    });

    let mut out = String::new();

    if !analysis.runtime_symbols.is_empty() && !has_runtime_import(source, runtime_import_path) {
        out.push_str(&runtime_import_fn(
            &analysis.runtime_symbols,
            runtime_import_path,
        ));
        out.push('\n');
    }

    let mut first = out.is_empty();
    for stmt in &program.body {
        if is_erased_stmt(stmt) {
            continue;
        }
        if !first {
            out.push('\n');
        }
        first = false;
        emit_stmt(source, stmt, &mut out, 0);
    }

    CONST_ENUMS.with(|slot| slot.borrow_mut().clear());
    out
}

fn collect_const_enums(source: &str, program: &Program<'_>) -> ConstEnumTable {
    let mut table = ConstEnumTable::new();
    for stmt in &program.body {
        let Stmt::Enum(enum_decl) = stmt else {
            continue;
        };
        if !enum_decl.is_const {
            continue;
        }
        let mut members = BTreeMap::new();
        let mut auto = 0i64;
        for member in &enum_decl.members {
            let value = if let Some(init) = &member.init {
                match &init.kind {
                    ExprKind::Number(n) => {
                        if let Ok(v) = n.as_str().replace('_', "").parse::<i64>() {
                            auto = v + 1;
                        }
                        n.as_str().to_string()
                    }
                    ExprKind::String(s) => {
                        auto += 1;
                        format!("\"{}\"", s.as_str())
                    }
                    _ => {
                        auto += 1;
                        source_slice(source, init.span.start, init.span.end)
                            .trim()
                            .to_string()
                    }
                }
            } else {
                let v = auto.to_string();
                auto += 1;
                v
            };
            members.insert(member.name.as_str().to_string(), value);
        }
        table.insert(enum_decl.name.as_str().to_string(), members);
    }
    table
}

fn is_erased_stmt(stmt: &Stmt<'_>) -> bool {
    matches!(
        stmt,
        Stmt::TypeOnly(_)
            | Stmt::TypeAlias(_)
            | Stmt::Interface(_)
            | Stmt::Module(_)
            | Stmt::Export(nori_ast::ExportDecl::TypeOnly(_))
            | Stmt::Import(nori_ast::ImportDecl { is_type: true, .. })
            | Stmt::Enum(EnumDecl { is_const: true, .. })
    )
}

fn emit_import(_source: &str, import: &nori_ast::ImportDecl, out: &mut String) {
    out.push_str("import ");
    if !import.specifiers.is_empty() {
        for (idx, spec) in import.specifiers.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            match spec {
                nori_ast::ImportSpecifier::Default(name) => out.push_str(name),
                nori_ast::ImportSpecifier::Named { local, imported } => {
                    if let Some(imported) = imported {
                        out.push_str(imported);
                        out.push_str(" as ");
                        out.push_str(local);
                    } else {
                        out.push_str(local);
                    }
                }
                nori_ast::ImportSpecifier::Namespace(name) => {
                    out.push_str("* as ");
                    out.push_str(name);
                }
            }
        }
        out.push_str(" from ");
    }
    out.push_str(&import.source);
    out.push(';');
}

fn emit_export(source: &str, export: &nori_ast::ExportDecl, out: &mut String, indent: usize) {
    match export {
        nori_ast::ExportDecl::TypeOnly(_) => {}
        nori_ast::ExportDecl::Named {
            specifiers,
            source: src,
            is_type,
            ..
        } => {
            if *is_type {
                return;
            }
            out.push_str("export { ");
            for (idx, spec) in specifiers.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                if let Some(exported) = &spec.exported {
                    out.push_str(&spec.local);
                    out.push_str(" as ");
                    out.push_str(exported);
                } else {
                    out.push_str(&spec.local);
                }
            }
            out.push_str(" }");
            if let Some(src) = src {
                out.push_str(" from ");
                out.push_str(src);
            }
            out.push(';');
        }
        nori_ast::ExportDecl::All {
            source: src,
            as_namespace,
            ..
        } => {
            out.push_str("export *");
            if let Some(ns) = as_namespace {
                out.push_str(" as ");
                out.push_str(ns);
            }
            out.push_str(" from ");
            out.push_str(src);
            out.push(';');
        }
        nori_ast::ExportDecl::Declaration(stmt) => {
            push_indent(out, indent);
            out.push_str("export ");
            emit_stmt(source, stmt, out, 0);
        }
    }
}

fn emit_stmt(source: &str, stmt: &Stmt, out: &mut String, indent: usize) {
    match stmt {
        Stmt::Import(import) => {
            push_indent(out, indent);
            emit_import(source, import, out);
        }
        Stmt::Export(export) => {
            emit_export(source, export, out, indent);
        }
        Stmt::Raw(raw) => {
            push_indent(out, indent);
            out.push_str(source_slice(source, raw.span.start, raw.span.end).trim());
        }
        Stmt::TypeOnly(_) | Stmt::TypeAlias(_) | Stmt::Interface(_) | Stmt::Module(_) => {}
        Stmt::Enum(enum_decl) => emit_enum(source, enum_decl, out, indent),
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
        Stmt::Class(class) => emit_class(source, class, out, indent),
        Stmt::Try(stmt) => emit_try(source, stmt, out, indent),
        Stmt::For(stmt) => emit_for(source, stmt, out, indent),
        Stmt::ClassicFor(stmt) => emit_classic_for(source, stmt, out, indent),
        Stmt::While(stmt) => emit_while(source, stmt, out, indent),
        Stmt::DoWhile(stmt) => emit_do_while(source, stmt, out, indent),
        Stmt::Break(_) => {
            push_indent(out, indent);
            out.push_str("break;");
        }
        Stmt::Continue(_) => {
            push_indent(out, indent);
            out.push_str("continue;");
        }
        Stmt::Switch(stmt) => emit_switch(source, stmt, out, indent),
        Stmt::Throw(stmt) => {
            push_indent(out, indent);
            out.push_str("throw ");
            emit_expr(source, &stmt.argument, out);
            out.push(';');
        }
        Stmt::Label(stmt) => {
            push_indent(out, indent);
            out.push_str(&stmt.label);
            out.push_str(": ");
            emit_stmt(source, &stmt.body, out, 0);
        }
        Stmt::Debugger(_) => {
            push_indent(out, indent);
            out.push_str("debugger;");
        }
        Stmt::With(stmt) => {
            push_indent(out, indent);
            out.push_str("with (");
            emit_expr(source, &stmt.object, out);
            out.push_str(") ");
            emit_stmt(source, &stmt.body, out, 0);
        }
    }
}

fn emit_var(source: &str, var: &VarDecl, out: &mut String, indent: usize) {
    push_indent(out, indent);
    emit_var_head(source, var, out);
    out.push(';');
}

fn emit_enum(source: &str, enum_decl: &EnumDecl, out: &mut String, indent: usize) {
    push_indent(out, indent);
    let name = enum_decl.name.as_str();
    // Regular enums lower to a bidirectional object IIFE (numeric) or
    // forward-only assignments (string members). Const enums are erased and
    // inlined at use sites (see `CONST_ENUMS` / Member emit).
    out.push_str("var ");
    out.push_str(name);
    out.push_str("; (function (");
    out.push_str(name);
    out.push_str(") {\n");
    let mut auto = 0i64;
    for member in &enum_decl.members {
        push_indent(out, indent + 1);
        let member_name = member.name.as_str();
        let is_string = matches!(
            member.init.as_ref().map(|e| &e.kind),
            Some(ExprKind::String(_))
        );
        if is_string {
            // String enums: no reverse mapping.
            out.push_str(name);
            out.push_str("[\"");
            out.push_str(member_name);
            out.push_str("\"] = ");
            if let Some(init) = &member.init {
                emit_expr(source, init, out);
            }
            out.push_str(";\n");
            auto += 1;
        } else {
            out.push_str(name);
            out.push('[');
            out.push_str(name);
            out.push_str("[\"");
            out.push_str(member_name);
            out.push_str("\"] = ");
            if let Some(init) = &member.init {
                emit_expr(source, init, out);
                if let ExprKind::Number(n) = &init.kind {
                    if let Ok(v) = n.as_str().replace('_', "").parse::<i64>() {
                        auto = v + 1;
                    } else {
                        auto += 1;
                    }
                } else {
                    auto += 1;
                }
            } else {
                out.push_str(&auto.to_string());
                auto += 1;
            }
            out.push_str("] = \"");
            out.push_str(member_name);
            out.push_str("\";\n");
        }
    }
    push_indent(out, indent);
    out.push_str("})(");
    out.push_str(name);
    out.push_str(" || (");
    out.push_str(name);
    out.push_str(" = {}));");
}

fn emit_var_head(source: &str, var: &VarDecl, out: &mut String) {
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
}

fn emit_destructuring_pattern(pattern: &Pattern, out: &mut String) {
    match pattern {
        Pattern::Ident(name) => out.push_str(name),
        Pattern::Array { elements, rest, .. } => {
            out.push('[');
            for (idx, element) in elements.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                if let Some(pattern) = element {
                    emit_destructuring_pattern(pattern, out);
                }
            }
            if let Some(rest) = rest {
                if !elements.is_empty() {
                    out.push_str(", ");
                }
                out.push_str("...");
                emit_destructuring_pattern(rest, out);
            }
            out.push(']');
        }
        Pattern::Object {
            properties, rest, ..
        } => {
            out.push('{');
            for (idx, prop) in properties.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                if let Some(alias) = &prop.alias {
                    out.push_str(&prop.key);
                    out.push_str(": ");
                    out.push_str(alias);
                } else {
                    out.push_str(&prop.key);
                }
                if let Some(default) = &prop.default {
                    out.push_str(" = ");
                    emit_expr("", default, out);
                }
            }
            if let Some(rest) = rest {
                if !properties.is_empty() {
                    out.push_str(", ");
                }
                out.push_str("...");
                emit_destructuring_pattern(rest, out);
            }
            out.push('}');
        }
        Pattern::Rest(pattern) => {
            out.push_str("...");
            emit_destructuring_pattern(pattern, out);
        }
        Pattern::Assign { left, right } => {
            emit_destructuring_pattern(left, out);
            out.push_str(" = ");
            emit_expr("", right, out);
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
    if function.generator {
        out.push('*');
    }
    if let Some(name) = &function.name {
        out.push(' ');
        out.push_str(name);
    }
    out.push('(');
    emit_params(source, &function.params, out);
    out.push_str(") ");
    emit_block(source, &function.body, out, indent);
}

fn emit_params(source: &str, params: &[Param], out: &mut String) {
    for (idx, param) in params.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        if let Some(default) = &param.default {
            out.push_str(" = ");
            emit_expr(source, default, out);
        }
    }
}

fn emit_class(source: &str, class: &nori_ast::ClassDecl, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("class ");
    out.push_str(&class.name);
    if let Some(extends) = &class.extends {
        out.push_str(" extends ");
        out.push_str(extends);
    }
    out.push_str(" {\n");
    for member in &class.members {
        emit_class_member(source, member, out, indent + 1, class.extends.is_some());
        out.push('\n');
    }
    push_indent(out, indent);
    out.push('}');
}

fn emit_class_member(
    source: &str,
    member: &ClassMember,
    out: &mut String,
    indent: usize,
    derived: bool,
) {
    match member {
        ClassMember::Field(field) => emit_class_field(source, field, out, indent),
        ClassMember::Constructor(constructor) => {
            emit_class_constructor(source, constructor, out, indent, derived);
        }
        ClassMember::Method(method) => emit_class_method(source, method, out, indent),
        ClassMember::Accessor(accessor) => emit_class_accessor(source, accessor, out, indent),
        ClassMember::StaticBlock(block) => emit_class_static_block(source, block, out, indent),
    }
}

fn emit_class_field(source: &str, field: &ClassField, out: &mut String, indent: usize) {
    push_indent(out, indent);
    if field.is_static {
        out.push_str("static ");
    }
    emit_member_name(&field.name, field.is_private, &field.computed, source, out);
    if let Some(value) = &field.value {
        out.push_str(" = ");
        emit_expr(source, value, out);
    }
    out.push(';');
}

fn emit_member_name(
    name: &str,
    is_private: bool,
    computed: &Option<nori_ast::Box<'_, Expr<'_>>>,
    source: &str,
    out: &mut String,
) {
    if is_private {
        out.push('#');
    }
    if let Some(expr) = computed {
        out.push('[');
        emit_expr(source, expr, out);
        out.push(']');
    } else {
        out.push_str(name);
    }
}

fn emit_class_constructor(
    source: &str,
    constructor: &ClassConstructor,
    out: &mut String,
    indent: usize,
    derived: bool,
) {
    push_indent(out, indent);
    out.push_str("constructor(");
    emit_params(source, &constructor.params, out);
    out.push_str(") ");
    emit_constructor_body(source, constructor, out, indent, derived);
}

fn emit_class_method(source: &str, method: &ClassMethod, out: &mut String, indent: usize) {
    push_indent(out, indent);
    if method.is_static {
        out.push_str("static ");
    }
    if method.is_async {
        out.push_str("async ");
    }
    if method.is_get {
        out.push_str("get ");
    }
    if method.is_set {
        out.push_str("set ");
    }
    emit_member_name(
        &method.name,
        method.is_private,
        &method.computed,
        source,
        out,
    );
    out.push('(');
    emit_params(source, &method.params, out);
    out.push_str(") ");
    emit_block(source, &method.body, out, indent);
}

fn emit_class_accessor(source: &str, accessor: &ClassAccessor, out: &mut String, indent: usize) {
    push_indent(out, indent);
    if accessor.is_static {
        out.push_str("static ");
    }
    if accessor.is_get {
        out.push_str("get ");
    } else {
        out.push_str("set ");
    }
    emit_member_name(
        &accessor.name,
        accessor.is_private,
        &accessor.computed,
        source,
        out,
    );
    out.push('(');
    emit_params(source, &accessor.params, out);
    out.push_str(") ");
    emit_block(source, &accessor.body, out, indent);
}

fn emit_class_static_block(
    source: &str,
    block: &ClassStaticBlock,
    out: &mut String,
    indent: usize,
) {
    push_indent(out, indent);
    out.push_str("static ");
    emit_block(source, &block.body, out, indent);
}

fn emit_constructor_body(
    source: &str,
    constructor: &ClassConstructor,
    out: &mut String,
    indent: usize,
    derived: bool,
) {
    out.push_str("{\n");
    let params = constructor
        .params
        .iter()
        .filter(|param| param.is_property)
        .collect::<Vec<_>>();
    let first_body_index = if derived
        && constructor
            .body
            .body
            .first()
            .is_some_and(is_super_call_stmt)
    {
        emit_stmt(source, &constructor.body.body[0], out, indent + 1);
        out.push('\n');
        1
    } else {
        0
    };
    for param in params {
        push_indent(out, indent + 1);
        out.push_str("this.");
        out.push_str(&param.name);
        out.push_str(" = ");
        out.push_str(&param.name);
        out.push_str(";\n");
    }
    for stmt in &constructor.body.body[first_body_index..] {
        emit_stmt(source, stmt, out, indent + 1);
        out.push('\n');
    }
    push_indent(out, indent);
    out.push('}');
}

fn is_super_call_stmt(stmt: &Stmt) -> bool {
    let Stmt::Expr(expr) = stmt else {
        return false;
    };
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return false;
    };
    matches!(&callee.kind, ExprKind::Ident(name) if name == "super")
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
    emit_stmt(source, &stmt.body, out, 0);
}

fn emit_classic_for(source: &str, stmt: &ClassicForStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("for (");
    if let Some(init) = &stmt.init {
        match init {
            ForInit::Var(var) => emit_var_head(source, var, out),
            ForInit::Expr(expr) => emit_expr(source, expr, out),
        }
    }
    out.push_str("; ");
    if let Some(condition) = &stmt.condition {
        emit_expr(source, condition, out);
    }
    out.push_str("; ");
    if let Some(update) = &stmt.update {
        emit_expr(source, update, out);
    }
    out.push_str(") ");
    emit_stmt(source, &stmt.body, out, 0);
}

fn emit_while(source: &str, stmt: &WhileStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("while (");
    emit_expr(source, &stmt.condition, out);
    out.push_str(") ");
    emit_stmt(source, &stmt.body, out, 0);
}

fn emit_do_while(source: &str, stmt: &DoWhileStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("do ");
    emit_stmt(source, &stmt.body, out, 0);
    out.push_str(" while (");
    emit_expr(source, &stmt.condition, out);
    out.push_str(");");
}

fn emit_switch(source: &str, stmt: &nori_ast::SwitchStmt, out: &mut String, indent: usize) {
    push_indent(out, indent);
    out.push_str("switch (");
    emit_expr(source, &stmt.discriminant, out);
    out.push_str(") {\n");
    for case in &stmt.cases {
        push_indent(out, indent + 1);
        match &case.test {
            Some(test) => {
                out.push_str("case ");
                emit_expr(source, test, out);
                out.push_str(":\n");
            }
            None => out.push_str("default:\n"),
        }
        for s in &case.consequent {
            emit_stmt(source, s, out, indent + 2);
            out.push('\n');
        }
    }
    push_indent(out, indent);
    out.push('}');
}

fn emit_expr(source: &str, expr: &Expr, out: &mut String) {
    if let Some(name) = primitive_call_name(expr) {
        emit_primitive_call(source, expr, name, out);
        return;
    }

    match &expr.kind {
        ExprKind::Ident(name) => out.push_str(name),
        ExprKind::Number(number) => out.push_str(number),
        ExprKind::BigInt(number) => {
            out.push_str(number);
        }
        ExprKind::String(s) => out.push_str(s),
        ExprKind::RegExp { pattern, flags } => {
            out.push('/');
            out.push_str(pattern);
            out.push('/');
            out.push_str(flags);
        }
        ExprKind::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        ExprKind::Null => out.push_str("null"),
        ExprKind::This => out.push_str("this"),
        ExprKind::New { callee, args } => {
            out.push_str("new ");
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
        ExprKind::Delete(expr) => {
            out.push_str("delete ");
            emit_expr(source, expr, out);
        }
        ExprKind::Void(expr) => {
            out.push_str("void ");
            emit_expr(source, expr, out);
        }
        ExprKind::Typeof(expr) => {
            out.push_str("typeof ");
            emit_expr(source, expr, out);
        }
        ExprKind::MetaProperty { meta, property } => {
            out.push_str(meta);
            out.push('.');
            out.push_str(property);
        }
        ExprKind::Import(expr) => {
            out.push_str("import(");
            emit_expr(source, expr, out);
            out.push(')');
        }
        ExprKind::Sequence(exprs) => {
            for (idx, e) in exprs.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                emit_expr(source, e, out);
            }
        }
        ExprKind::Yield { value, delegate } => {
            out.push_str("yield");
            if *delegate {
                out.push('*');
            }
            if let Some(expr) = value {
                out.push(' ');
                emit_expr(source, expr, out);
            }
        }
        ExprKind::Unary { op, expr } => {
            out.push_str(op);
            emit_expr(source, expr, out);
        }
        ExprKind::Update { op, expr, prefix } => {
            if *prefix {
                out.push_str(op);
            }
            emit_expr(source, expr, out);
            if !*prefix {
                out.push_str(op);
            }
        }
        ExprKind::TypeErasure { expr, .. } => emit_expr(source, expr, out),
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
        ExprKind::Call {
            callee,
            args,
            optional,
        } => {
            emit_expr(source, callee, out);
            if *optional {
                out.push_str("?.");
            }
            out.push('(');
            for (idx, arg) in args.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                emit_expr(source, arg, out);
            }
            out.push(')');
        }
        ExprKind::Member {
            object,
            property,
            optional,
        } => {
            if !*optional {
                if let ExprKind::Ident(enum_name) = &object.kind {
                    let inlined = CONST_ENUMS.with(|slot| {
                        slot.borrow()
                            .get(enum_name.as_str())
                            .and_then(|members| members.get(property.as_str()).cloned())
                    });
                    if let Some(value) = inlined {
                        out.push_str(&value);
                        return;
                    }
                }
            }
            emit_expr(source, object, out);
            if *optional {
                out.push_str("?.");
            } else {
                out.push('.');
            }
            out.push_str(property);
        }
        ExprKind::Index {
            object,
            index,
            optional,
        } => {
            emit_expr(source, object, out);
            if *optional {
                out.push_str("?.[");
            } else {
                out.push('[');
            }
            emit_expr(source, index, out);
            out.push(']');
        }
        ExprKind::Arrow { params, body } => {
            if params.len() == 1 {
                out.push_str(params[0].as_str());
            } else {
                out.push('(');
                let joined = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&joined);
                out.push(')');
            }
            out.push_str(" => ");
            match body {
                ArrowBody::Expression(expr) => emit_expr(source, expr, out),
                ArrowBody::Block(block) => emit_block(source, block, out, 0),
            }
        }
        ExprKind::TemplateLiteral { quasis, exprs } => {
            out.push('`');
            for (idx, quasi) in quasis.iter().enumerate() {
                out.push_str(quasi);
                if let Some(expr) = exprs.get(idx) {
                    out.push_str("${");
                    emit_expr(source, expr, out);
                    out.push('}');
                }
            }
            out.push('`');
        }
        ExprKind::TaggedTemplate { tag, quasi } => {
            emit_expr(source, tag, out);
            emit_expr(source, quasi, out);
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
        ExprKind::Object(properties) => {
            if properties.is_empty() {
                out.push_str("{}");
            } else {
                out.push_str("{ ");
                for (idx, prop) in properties.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(", ");
                    }
                    if prop.shorthand {
                        out.push_str(match &prop.key {
                            nori_ast::PropertyKey::Ident(name) => name.as_str(),
                            _ => "",
                        });
                    } else {
                        match &prop.key {
                            nori_ast::PropertyKey::Ident(name) => out.push_str(name.as_str()),
                            nori_ast::PropertyKey::String(s) => {
                                out.push('"');
                                out.push_str(s.as_str());
                                out.push('"');
                            }
                            nori_ast::PropertyKey::Number(n) => out.push_str(n.as_str()),
                            nori_ast::PropertyKey::Computed(expr) => {
                                out.push('[');
                                emit_expr(source, expr, out);
                                out.push(']');
                            }
                        }
                        out.push_str(": ");
                        emit_expr(source, &prop.value, out);
                    }
                }
                out.push_str(" }");
            }
        }
        ExprKind::Spread { expr } => {
            out.push_str("...");
            emit_expr(source, expr, out);
        }
        ExprKind::Await(expr) => {
            out.push_str("await ");
            emit_expr(source, expr, out);
        }
        ExprKind::Markup(node) => emit_markup(source, node, out),
        ExprKind::Raw => {
            out.push_str(source_slice(source, expr.span.start, expr.span.end).trim());
        }
    }
}

fn emit_markup(source: &str, node: &MarkupNode, out: &mut String) {
    match node {
        MarkupNode::Element(element) => emit_markup_element(source, element, out),
        MarkupNode::Fragment { children, .. } => {
            out.push_str("h(fragment, null");
            emit_markup_children_args(source, children, out);
            out.push(')');
        }
    }
}

fn emit_markup_element(source: &str, element: &MarkupElement, out: &mut String) {
    out.push_str("h(");
    emit_markup_tag(element.name.as_str(), out);
    out.push_str(", ");
    emit_markup_props(source, element, out);
    emit_markup_children_args(source, &element.children, out);
    out.push(')');
}

fn emit_markup_tag(name: &str, out: &mut String) {
    if is_component_tag(name) {
        out.push_str(name);
    } else {
        out.push('"');
        out.push_str(name);
        out.push('"');
    }
}

fn is_component_tag(name: &str) -> bool {
    name.contains('.')
        || name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn emit_markup_props(source: &str, element: &MarkupElement, out: &mut String) {
    let needs_button_type = element.name == "button"
        && !element.attributes.iter().any(|attribute| {
            matches!(
                attribute,
                MarkupAttribute::Named { name, .. } if name.eq_ignore_ascii_case("type")
            )
        });

    if element.attributes.is_empty() && !needs_button_type {
        out.push_str("null");
        return;
    }

    out.push_str("{ ");
    let mut first = true;
    if needs_button_type {
        out.push_str("type: \"button\"");
        first = false;
    }

    for attribute in &element.attributes {
        if !first {
            out.push_str(", ");
        }
        first = false;
        match attribute {
            MarkupAttribute::Named { name, value, .. } => {
                emit_prop_key(name.as_str(), out);
                out.push_str(": ");
                match value {
                    Some(expr) => emit_expr(source, expr, out),
                    None => out.push_str("true"),
                }
            }
            MarkupAttribute::Spread { expr, .. } => {
                out.push_str("...");
                emit_expr(source, expr, out);
            }
        }
    }
    out.push_str(" }");
}

fn emit_prop_key(name: &str, out: &mut String) {
    if is_ident_prop_key(name) {
        out.push_str(name);
    } else {
        out.push('"');
        out.push_str(name);
        out.push('"');
    }
}

fn is_ident_prop_key(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(ch) if ch == '_' || ch == '$' || ch.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
}

fn emit_markup_children_args(source: &str, children: &[MarkupChild], out: &mut String) {
    for child in children {
        match child {
            MarkupChild::Text(text, _) => {
                let trimmed = collapse_markup_text(text.as_str());
                if trimmed.is_empty() {
                    continue;
                }
                out.push_str(", ");
                emit_js_string(&trimmed, out);
            }
            MarkupChild::Expr(expr) => {
                out.push_str(", () => ");
                emit_expr(source, expr, out);
            }
            MarkupChild::Node(node) => {
                out.push_str(", ");
                emit_markup(source, node, out);
            }
        }
    }
}

fn collapse_markup_text(text: &str) -> String {
    if text.chars().all(|ch| ch.is_whitespace()) {
        // Drop indentation / newline-only text; keep interstitial spaces
        // between expression children (JSX-style).
        if text.contains('\n') {
            return String::new();
        }
        return " ".to_string();
    }
    text.to_string()
}

fn emit_js_string(value: &str, out: &mut String) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

fn emit_primitive_call(source: &str, expr: &Expr, name: &str, out: &mut String) {
    let expr = erase_type_wrappers(expr);
    let ExprKind::Call { args, .. } = &expr.kind else {
        return;
    };
    match name {
        "$state" => {
            out.push_str("signal(");
            emit_arg_list(source, args.as_slice(), out);
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
            emit_arg_list(source, args.as_slice(), out);
            out.push(')');
        }
        _ => {}
    }
}

fn erase_type_wrappers<'a, 'ast>(mut expr: &'a Expr<'ast>) -> &'a Expr<'ast> {
    while let ExprKind::TypeErasure { expr: inner, .. } = &expr.kind {
        expr = inner;
    }
    expr
}

fn emit_arg_list(source: &str, args: &[Expr<'_>], out: &mut String) {
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

fn source_slice(source: &str, start: u32, end: u32) -> &str {
    source.get(start as usize..end as usize).unwrap_or_default()
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}
