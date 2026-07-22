//! Type checker for Nori (Phase 3 / M1–M8).
//!
//! Builds an interned [`TypeId`] arena, then checks:
//! - M1: annotated variable assignability
//! - M2: object/interface structural assignability + excess property checks
//! - M3: function parameter / return annotations
//! - M4: basic generic instantiation for simple one-arg type refs / aliases
//! - M5: `typeof` / truthiness narrowing in if-branches
//! - M6: concrete conditional types + simple `keyof` on object aliases
//! - M7: multi-file `check_files` + ambient lib globals (Array/Promise/String)
//! - M8: component prop checking for markup attributes

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nori_ast::{
    Expr, ExprKind, FunctionDecl, InterfaceDecl, MarkupAttribute, MarkupElement, MarkupNode,
    Program, PropertyKey, Stmt, TSKeywordKind, TSLiteral, TSType, TSTypeElement, TSTypeOperator,
    TypeAliasDecl, VarDecl, Visit,
};
use nori_diagnostic::{Diagnostic, Severity};
use nori_semantic::{SemanticModel, SymbolId, build_semantic};

/// Opaque handle into a [`TypeArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u32);

/// Interned type representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Any,
    Unknown,
    Never,
    String,
    Number,
    Boolean,
    Symbol,
    Bigint,
    Object,
    Void,
    Undefined,
    Null,
    StringLiteral(String),
    NumberLiteral(String),
    BooleanLiteral(bool),
    /// Named reference we have not yet resolved (treated as `any` for assignability).
    Reference(String),
    /// Structural object / interface type.
    ObjectShape {
        props: Vec<ObjectProp>,
    },
    /// Function type from annotations.
    Function {
        params: Vec<TypeId>,
        return_type: TypeId,
    },
    /// Union of types (`A | B`).
    Union(Vec<TypeId>),
    /// Fallback for mapped/etc. not yet modeled.
    Complex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectProp {
    pub name: String,
    pub ty: TypeId,
    pub optional: bool,
    pub readonly: bool,
}

#[derive(Debug, Default)]
pub struct TypeArena {
    types: Vec<Type>,
}

impl TypeArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, ty: Type) -> TypeId {
        if let Some((idx, _)) = self
            .types
            .iter()
            .enumerate()
            .find(|(_, existing)| *existing == &ty)
        {
            return TypeId(idx as u32);
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    pub fn get(&self, id: TypeId) -> &Type {
        &self.types[id.0 as usize]
    }
}

/// Ambient lib primitive names shared across files (M7).
#[derive(Debug, Clone, Default)]
pub struct GlobalTypeEnv {
    names: BTreeSet<String>,
}

impl GlobalTypeEnv {
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn names(&self) -> &BTreeSet<String> {
        &self.names
    }
}

/// Hardcoded ES5-ish ambient globals (see `libs/lib.es5.min.d.ts`).
pub fn lib_es5_globals() -> GlobalTypeEnv {
    let mut names = BTreeSet::new();
    for name in [
        "Array",
        "ReadonlyArray",
        "Promise",
        "String",
        "Number",
        "Boolean",
        "Object",
        "Function",
        "Symbol",
        "Date",
        "RegExp",
        "Error",
    ] {
        names.insert(name.to_string());
    }
    GlobalTypeEnv { names }
}

/// Result of running the checker.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub semantic: SemanticModel,
}

/// Check `program` and return diagnostics (empty when there are no type errors).
pub fn check(program: &Program<'_>) -> CheckResult {
    let globals = lib_es5_globals();
    check_with_globals(program, &globals)
}

/// Check with an explicit ambient global type environment.
pub fn check_with_globals(program: &Program<'_>, globals: &GlobalTypeEnv) -> CheckResult {
    let semantic = build_semantic(program);
    let mut checker = Checker::new(semantic, globals);
    checker.collect_type_decls(program);
    checker.visit_program(program);
    CheckResult {
        diagnostics: checker.diagnostics,
        semantic: checker.semantic,
    }
}

/// Multi-file stub (M7): parse each path, build semantic per file, share lib globals.
pub fn check_files(paths: &[&Path]) -> CheckResult {
    let globals = lib_es5_globals();
    let mut diagnostics = Vec::new();
    let mut last_semantic = SemanticModel::default();

    for path in paths {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    message: format!("failed to read {}: {err}", path.display()),
                    severity: Severity::Error,
                    span: nori_diagnostic::span(0, 0),
                    code: "nori::check",
                });
                continue;
            }
        };
        let allocator = nori_allocator::Allocator::new();
        let tokens = match nori_lexer::lex(&source) {
            Ok(t) => t,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    message: format!("{}: {err}", path.display()),
                    severity: Severity::Error,
                    span: nori_diagnostic::span(0, 0),
                    code: "nori::check",
                });
                continue;
            }
        };
        let filename = path.display().to_string();
        let program = match nori_parser::parse_in(&allocator, &source, filename, tokens) {
            Ok(p) => p,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    message: format!("{}: {err}", path.display()),
                    severity: Severity::Error,
                    span: nori_diagnostic::span(0, 0),
                    code: "nori::check",
                });
                continue;
            }
        };
        let checked = check_with_globals(&program, &globals);
        diagnostics.extend(checked.diagnostics);
        last_semantic = checked.semantic;
    }

    CheckResult {
        diagnostics,
        semantic: last_semantic,
    }
}

struct TypeAliasInfo<'a> {
    params: Vec<String>,
    type_ann: &'a TSType<'a>,
}

struct Checker<'a> {
    arena: TypeArena,
    semantic: SemanticModel,
    globals: &'a GlobalTypeEnv,
    /// Inferred/annotated types for value symbols.
    symbol_types: Vec<Option<TypeId>>,
    /// Interface name → structural type.
    interfaces: BTreeMap<String, TypeId>,
    /// Type alias name → alias info (AST retained for instantiation).
    aliases: BTreeMap<String, TypeAliasInfo<'a>>,
    /// Function name → function type.
    functions: BTreeMap<String, TypeId>,
    /// Function name → first param object-shape props (for M8), when available.
    component_props: BTreeMap<String, TypeId>,
    /// Stack of expected return types while visiting function bodies.
    return_stack: Vec<Option<TypeId>>,
    /// Narrowing overlays (innermost last).
    narrowing_stack: Vec<BTreeMap<SymbolId, TypeId>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Checker<'a> {
    fn new(semantic: SemanticModel, globals: &'a GlobalTypeEnv) -> Self {
        let symbol_types = vec![None; semantic.symbols.len()];
        let mut checker = Self {
            arena: TypeArena::new(),
            semantic,
            globals,
            symbol_types,
            interfaces: BTreeMap::new(),
            aliases: BTreeMap::new(),
            functions: BTreeMap::new(),
            component_props: BTreeMap::new(),
            return_stack: Vec::new(),
            narrowing_stack: Vec::new(),
            diagnostics: Vec::new(),
        };
        checker.install_lib_globals();
        checker
    }

    fn install_lib_globals(&mut self) {
        for name in self.globals.names() {
            let id = self.arena.intern(Type::Reference(name.clone()));
            // Treat as named ambient types so aliases/refs resolve consistently.
            self.interfaces.entry(name.clone()).or_insert(id);
        }
    }

    fn set_symbol_type(&mut self, id: SymbolId, ty: TypeId) {
        let idx = id.0 as usize;
        if idx < self.symbol_types.len() {
            self.symbol_types[idx] = Some(ty);
        }
    }

    fn lookup_symbol_type(&self, id: SymbolId) -> Option<TypeId> {
        for layer in self.narrowing_stack.iter().rev() {
            if let Some(ty) = layer.get(&id) {
                return Some(*ty);
            }
        }
        self.symbol_types
            .get(id.0 as usize)
            .copied()
            .flatten()
    }

    fn resolve_ident_symbol(&self, name: &str) -> Option<SymbolId> {
        self.semantic
            .resolve_binding(self.semantic.root_scope, name)
            .or_else(|| {
                self.semantic
                    .symbols
                    .iter()
                    .find(|s| s.name == name)
                    .map(|s| s.id)
            })
    }

    fn collect_type_decls(&mut self, program: &'a Program<'a>) {
        for stmt in &program.body {
            match stmt {
                Stmt::Interface(iface) => self.register_interface(iface),
                Stmt::TypeAlias(alias) => self.register_alias(alias),
                Stmt::Function(func) | Stmt::ExportDefaultFunction(func) => {
                    self.register_function(func);
                }
                _ => {}
            }
        }
    }

    fn register_interface(&mut self, iface: &InterfaceDecl<'_>) {
        let shape = self.lower_object_elements(&iface.body);
        let id = self.arena.intern(shape);
        self.interfaces.insert(iface.name.as_str().to_string(), id);
    }

    fn register_alias(&mut self, alias: &'a TypeAliasDecl<'a>) {
        let params: Vec<String> = alias
            .type_params
            .iter()
            .map(|p| p.as_str().to_string())
            .collect();
        self.aliases.insert(
            alias.name.as_str().to_string(),
            TypeAliasInfo {
                params,
                type_ann: &alias.type_ann,
            },
        );
    }

    fn register_function(&mut self, func: &FunctionDecl<'_>) {
        let Some(name) = &func.name else {
            return;
        };
        let mut params = Vec::new();
        for param in &func.params {
            let ty = param
                .type_ann
                .as_ref()
                .map(|t| self.lower_ts_type(t))
                .unwrap_or_else(|| self.arena.intern(Type::Any));
            params.push(ty);
        }
        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.lower_ts_type(t))
            .unwrap_or_else(|| self.arena.intern(Type::Any));
        let fn_ty = self.arena.intern(Type::Function {
            params: params.clone(),
            return_type,
        });
        let name_str = name.as_str().to_string();
        self.functions.insert(name_str.clone(), fn_ty);
        if let Some(first) = params.first() {
            if matches!(self.arena.get(*first), Type::ObjectShape { .. }) {
                self.component_props.insert(name_str, *first);
            }
        }
    }

    fn lower_object_elements(&mut self, elements: &[TSTypeElement<'_>]) -> Type {
        let mut props = Vec::new();
        for element in elements {
            if let TSTypeElement::Property {
                key,
                optional,
                readonly,
                type_ann,
                ..
            } = element
            {
                let ty = type_ann
                    .as_ref()
                    .map(|t| self.lower_ts_type(t))
                    .unwrap_or_else(|| self.arena.intern(Type::Any));
                props.push(ObjectProp {
                    name: key.as_str().to_string(),
                    ty,
                    optional: *optional,
                    readonly: *readonly,
                });
            }
        }
        Type::ObjectShape { props }
    }

    fn lower_ts_type(&mut self, ty: &TSType<'_>) -> TypeId {
        self.lower_ts_type_with_params(ty, &[], &[])
    }

    fn lower_ts_type_with_params(
        &mut self,
        ty: &TSType<'_>,
        param_names: &[String],
        type_args: &[TypeId],
    ) -> TypeId {
        match ty {
            TSType::Keyword(kind, _) => {
                let lowered = match kind {
                    TSKeywordKind::Any => Type::Any,
                    TSKeywordKind::Unknown => Type::Unknown,
                    TSKeywordKind::Never => Type::Never,
                    TSKeywordKind::String => Type::String,
                    TSKeywordKind::Number => Type::Number,
                    TSKeywordKind::Boolean => Type::Boolean,
                    TSKeywordKind::Symbol => Type::Symbol,
                    TSKeywordKind::Bigint => Type::Bigint,
                    TSKeywordKind::Object => Type::Object,
                    TSKeywordKind::Void => Type::Void,
                    TSKeywordKind::Undefined => Type::Undefined,
                    TSKeywordKind::Null => Type::Null,
                };
                self.arena.intern(lowered)
            }
            TSType::Literal(lit) => {
                let lowered = match lit {
                    TSLiteral::String(s, _) => Type::StringLiteral(unquote(s.as_str())),
                    TSLiteral::Number(n, _) => Type::NumberLiteral(n.to_string()),
                    TSLiteral::Bool(b, _) => Type::BooleanLiteral(*b),
                    TSLiteral::Null(_) => Type::Null,
                };
                self.arena.intern(lowered)
            }
            TSType::Reference {
                name,
                type_args: ref_args,
                ..
            } => {
                let name_str = name.as_str();
                // Type parameter substitution (M4).
                if let Some(idx) = param_names.iter().position(|p| p == name_str) {
                    if let Some(arg) = type_args.get(idx) {
                        return *arg;
                    }
                }
                if let Some(iface) = self.interfaces.get(name_str) {
                    return *iface;
                }
                if let Some(alias_name) = self.aliases.keys().find(|k| *k == name_str).cloned() {
                    let (alias_params, alias_ann) = {
                        let alias = self.aliases.get(&alias_name).unwrap();
                        (alias.params.clone(), alias.type_ann)
                    };
                    let mut args = Vec::new();
                    if let Some(ref_args) = ref_args {
                        for arg in ref_args.iter() {
                            args.push(self.lower_ts_type_with_params(arg, param_names, type_args));
                        }
                    }
                    while args.len() < alias_params.len() {
                        args.push(self.arena.intern(Type::Any));
                    }
                    return self.lower_ts_type_with_params(alias_ann, &alias_params, &args);
                }
                if self.globals.contains(name_str) {
                    return self.arena.intern(Type::Reference(name_str.to_string()));
                }
                if let Some(ref_args) = ref_args {
                    if ref_args.len() == 1 && matches!(name_str, "Array" | "ReadonlyArray") {
                        let _elem =
                            self.lower_ts_type_with_params(&ref_args[0], param_names, type_args);
                        return self.arena.intern(Type::Reference(name_str.to_string()));
                    }
                }
                self.arena.intern(Type::Reference(name_str.to_string()))
            }
            TSType::Parenthesized(inner, _) => {
                self.lower_ts_type_with_params(inner, param_names, type_args)
            }
            TSType::Object(elements, _) => {
                let shape = self.lower_object_elements(elements);
                self.arena.intern(shape)
            }
            TSType::Function {
                params,
                return_type,
                ..
            } => {
                let mut param_tys = Vec::new();
                for param in params {
                    let ty = param
                        .type_ann
                        .as_ref()
                        .map(|t| self.lower_ts_type_with_params(t, param_names, type_args))
                        .unwrap_or_else(|| self.arena.intern(Type::Any));
                    param_tys.push(ty);
                }
                let ret = self.lower_ts_type_with_params(return_type, param_names, type_args);
                self.arena.intern(Type::Function {
                    params: param_tys,
                    return_type: ret,
                })
            }
            TSType::Union(members, _) => {
                let mut ids = Vec::new();
                for member in members {
                    ids.push(self.lower_ts_type_with_params(member, param_names, type_args));
                }
                self.intern_union(ids)
            }
            TSType::Conditional {
                check,
                extends,
                true_type,
                false_type,
                ..
            } => {
                let check_ty = self.lower_ts_type_with_params(check, param_names, type_args);
                let extends_ty = self.lower_ts_type_with_params(extends, param_names, type_args);
                // M6: evaluate when both sides are concrete (no open type params as refs).
                if is_concrete(self.arena.get(check_ty)) && is_concrete(self.arena.get(extends_ty))
                {
                    if self.is_assignable(extends_ty, check_ty) {
                        return self.lower_ts_type_with_params(true_type, param_names, type_args);
                    }
                    return self.lower_ts_type_with_params(false_type, param_names, type_args);
                }
                self.arena.intern(Type::Complex)
            }
            TSType::Operator {
                op: TSTypeOperator::Keyof,
                operand,
                ..
            } => {
                let operand_ty = self.lower_ts_type_with_params(operand, param_names, type_args);
                self.keyof_type(operand_ty)
            }
            TSType::Any(_) => self.arena.intern(Type::Any),
            TSType::This(_)
            | TSType::Intersection(_, _)
            | TSType::Array(_, _)
            | TSType::Tuple(_, _)
            | TSType::Infer { .. }
            | TSType::Typeof { .. }
            | TSType::IndexedAccess { .. }
            | TSType::Operator { .. }
            | TSType::Mapped { .. }
            | TSType::TemplateLiteral { .. } => self.arena.intern(Type::Complex),
        }
    }

    fn keyof_type(&mut self, operand: TypeId) -> TypeId {
        match self.arena.get(operand).clone() {
            Type::ObjectShape { props } => {
                if props.is_empty() {
                    return self.arena.intern(Type::Never);
                }
                let mut lits = Vec::new();
                for prop in props {
                    lits.push(self.arena.intern(Type::StringLiteral(prop.name)));
                }
                self.intern_union(lits)
            }
            Type::Reference(name) => {
                // Resolve alias/interface again if possible.
                if let Some(alias) = self.aliases.get(&name).map(|a| a.type_ann) {
                    let resolved = self.lower_ts_type(alias);
                    return self.keyof_type(resolved);
                }
                if let Some(iface) = self.interfaces.get(&name).copied() {
                    return self.keyof_type(iface);
                }
                self.arena.intern(Type::Complex)
            }
            _ => self.arena.intern(Type::Complex),
        }
    }

    fn intern_union(&mut self, mut members: Vec<TypeId>) -> TypeId {
        members.sort_by_key(|id| id.0);
        members.dedup();
        if members.len() == 1 {
            return members[0];
        }
        self.arena.intern(Type::Union(members))
    }

    fn flatten_union(&self, ty: TypeId) -> Vec<TypeId> {
        match self.arena.get(ty) {
            Type::Union(members) => {
                let mut out = Vec::new();
                for m in members {
                    out.extend(self.flatten_union(*m));
                }
                out
            }
            _ => vec![ty],
        }
    }

    fn infer_expr_type(&mut self, expr: &Expr<'_>) -> TypeId {
        let ty = match &expr.kind {
            ExprKind::String(s) => Type::StringLiteral(unquote(s.as_str())),
            ExprKind::Number(n) => Type::NumberLiteral(n.to_string()),
            ExprKind::BigInt(_) => Type::Bigint,
            ExprKind::Bool(b) => Type::BooleanLiteral(*b),
            ExprKind::Null => Type::Null,
            ExprKind::Object(props) => {
                let mut shape_props = Vec::new();
                for prop in props {
                    if prop.shorthand || prop.computed {
                        continue;
                    }
                    let Some(name) = prop_key_name(&prop.key) else {
                        continue;
                    };
                    let value_ty = self.infer_expr_type(&prop.value);
                    shape_props.push(ObjectProp {
                        name,
                        ty: value_ty,
                        optional: false,
                        readonly: false,
                    });
                }
                return self.arena.intern(Type::ObjectShape {
                    props: shape_props,
                });
            }
            ExprKind::Ident(name) => {
                if let Some(symbol_id) = self.resolve_ident_symbol(name.as_str()) {
                    if let Some(existing) = self.lookup_symbol_type(symbol_id) {
                        return existing;
                    }
                }
                if let Some(fn_ty) = self.functions.get(name.as_str()) {
                    return *fn_ty;
                }
                Type::Any
            }
            ExprKind::Call { callee, args, .. } => {
                let callee_ty = self.infer_expr_type(callee);
                if let Type::Function {
                    params,
                    return_type,
                } = self.arena.get(callee_ty).clone()
                {
                    for (idx, arg) in args.iter().enumerate() {
                        let arg_ty = self.infer_expr_type(arg);
                        if let Some(&param_ty) = params.get(idx) {
                            if !self.is_assignable(param_ty, arg_ty) {
                                self.diagnostics.push(Diagnostic {
                                    message: format!(
                                        "argument of type '{}' is not assignable to parameter of type '{}'",
                                        type_display(self.arena.get(arg_ty)),
                                        type_display(self.arena.get(param_ty)),
                                    ),
                                    severity: Severity::Error,
                                    span: nori_diagnostic::span(
                                        arg.span.start as usize,
                                        arg.span.end as usize,
                                    ),
                                    code: "nori::check",
                                });
                            }
                        }
                    }
                    return return_type;
                }
                Type::Any
            }
            ExprKind::TypeErasure { expr, .. } => return self.infer_expr_type(expr),
            ExprKind::Unary { expr, .. } => return self.infer_expr_type(expr),
            // Markup prop checking happens once in Visit::visit_expr.
            ExprKind::Markup(_) => Type::Any,
            _ => Type::Any,
        };
        self.arena.intern(ty)
    }

    fn is_assignable(&self, target: TypeId, source: TypeId) -> bool {
        if target == source {
            return true;
        }
        let target_ty = self.arena.get(target);
        let source_ty = self.arena.get(source);

        // Union targets: source must be assignable to some member.
        if let Type::Union(members) = target_ty {
            return members.iter().any(|m| self.is_assignable(*m, source));
        }
        // Union sources: every member must be assignable to target.
        if let Type::Union(members) = source_ty {
            return members.iter().all(|m| self.is_assignable(target, *m));
        }

        match (target_ty, source_ty) {
            (Type::Any, _) | (_, Type::Any) | (Type::Unknown, _) | (_, Type::Never) => true,
            (Type::String, Type::StringLiteral(_)) => true,
            (Type::Number, Type::NumberLiteral(_)) => true,
            (Type::Boolean, Type::BooleanLiteral(_)) => true,
            (Type::Complex, _) | (_, Type::Complex) => true,
            (Type::Reference(a), Type::Reference(b)) => a == b,
            (Type::Reference(_), _) | (_, Type::Reference(_)) => true,
            (
                Type::ObjectShape {
                    props: target_props,
                },
                Type::ObjectShape {
                    props: source_props,
                },
            ) => {
                for tp in target_props {
                    if let Some(sp) = source_props.iter().find(|p| p.name == tp.name) {
                        if !self.is_assignable(tp.ty, sp.ty) {
                            return false;
                        }
                    } else if !tp.optional {
                        return false;
                    }
                }
                true
            }
            (
                Type::Function {
                    params: t_params,
                    return_type: t_ret,
                },
                Type::Function {
                    params: s_params,
                    return_type: s_ret,
                },
            ) => {
                if t_params.len() != s_params.len() {
                    return false;
                }
                for (t, s) in t_params.iter().zip(s_params.iter()) {
                    if !self.is_assignable(*s, *t) {
                        return false;
                    }
                }
                self.is_assignable(*t_ret, *s_ret)
            }
            (Type::StringLiteral(a), Type::StringLiteral(b)) => a == b,
            (Type::NumberLiteral(a), Type::NumberLiteral(b)) => a == b,
            (Type::BooleanLiteral(a), Type::BooleanLiteral(b)) => a == b,
            _ => false,
        }
    }

    /// Excess property check: when assigning an object literal to a shape,
    /// reject properties not present on the target (fresh literal check).
    fn check_excess_properties(
        &mut self,
        target: TypeId,
        source_expr: &Expr<'_>,
        source_ty: TypeId,
    ) {
        let Type::ObjectShape {
            props: target_props,
        } = self.arena.get(target).clone()
        else {
            return;
        };
        let ExprKind::Object(props) = &source_expr.kind else {
            return;
        };
        let Type::ObjectShape {
            props: source_props,
        } = self.arena.get(source_ty).clone()
        else {
            return;
        };
        for sp in &source_props {
            if !target_props.iter().any(|tp| tp.name == sp.name) {
                let span = props
                    .iter()
                    .find(|p| prop_key_name(&p.key).as_deref() == Some(sp.name.as_str()))
                    .map(|p| p.span)
                    .unwrap_or(source_expr.span);
                self.diagnostics.push(Diagnostic {
                    message: format!(
                        "object literal may only specify known properties, and '{}' does not exist on type '{}'",
                        sp.name,
                        type_display(self.arena.get(target)),
                    ),
                    severity: Severity::Error,
                    span: nori_diagnostic::span(span.start as usize, span.end as usize),
                    code: "nori::check",
                });
            }
        }
    }

    fn check_var_decl(&mut self, var: &VarDecl<'_>) {
        for declarator in &var.declarators {
            let annotated = declarator
                .type_ann
                .as_ref()
                .map(|ty| self.lower_ts_type(ty));
            let init_ty = declarator
                .init
                .as_ref()
                .map(|init| self.infer_expr_type(init));

            if let (Some(target), Some(source)) = (annotated, init_ty) {
                if let Some(init) = &declarator.init {
                    self.check_excess_properties(target, init, source);
                }
                if !self.is_assignable(target, source) {
                    self.diagnostics.push(Diagnostic {
                        message: format!(
                            "type '{}' is not assignable to type '{}'",
                            type_display(self.arena.get(source)),
                            type_display(self.arena.get(target)),
                        ),
                        severity: Severity::Error,
                        span: nori_diagnostic::span(
                            declarator.span.start as usize,
                            declarator.span.end as usize,
                        ),
                        code: "nori::check",
                    });
                }
            }

            let binding_ty = annotated.or(init_ty);
            if let Some(ty) = binding_ty {
                if let Some(symbol) = self
                    .semantic
                    .symbols
                    .iter()
                    .find(|s| s.name == declarator.name.as_str() && s.span == declarator.span)
                    .or_else(|| {
                        self.semantic
                            .symbols
                            .iter()
                            .find(|s| s.name == declarator.name.as_str())
                    })
                {
                    self.set_symbol_type(symbol.id, ty);
                }
            }
        }
    }

    fn check_function(&mut self, func: &FunctionDecl<'a>) {
        for param in &func.params {
            let ty = param
                .type_ann
                .as_ref()
                .map(|t| self.lower_ts_type(t))
                .unwrap_or_else(|| self.arena.intern(Type::Any));
            if let Some(symbol) = self
                .semantic
                .symbols
                .iter()
                .find(|s| s.name == param.name.as_str())
            {
                self.set_symbol_type(symbol.id, ty);
            }
            if let Some(default) = &param.default {
                let default_ty = self.infer_expr_type(default);
                if param.type_ann.is_some() && !self.is_assignable(ty, default_ty) {
                    self.diagnostics.push(Diagnostic {
                        message: format!(
                            "default argument of type '{}' is not assignable to parameter of type '{}'",
                            type_display(self.arena.get(default_ty)),
                            type_display(self.arena.get(ty)),
                        ),
                        severity: Severity::Error,
                        span: nori_diagnostic::span(
                            default.span.start as usize,
                            default.span.end as usize,
                        ),
                        code: "nori::check",
                    });
                }
            }
        }

        let return_ty = func.return_type.as_ref().map(|t| self.lower_ts_type(t));
        self.return_stack.push(return_ty);
        self.visit_block(&func.body);
        self.return_stack.pop();
    }

    // --- M5 narrowing ---

    fn check_if(&mut self, stmt: &nori_ast::IfStmt<'a>) {
        let (then_map, else_map) = self.narrowing_from_condition(&stmt.condition);
        self.narrowing_stack.push(then_map);
        self.visit_stmt(&stmt.consequent);
        self.narrowing_stack.pop();
        if let Some(alt) = &stmt.alternate {
            self.narrowing_stack.push(else_map);
            self.visit_stmt(alt);
            self.narrowing_stack.pop();
        }
        // Still walk the condition for nested calls/markup.
        self.visit_expr(&stmt.condition);
    }

    fn narrowing_from_condition(
        &mut self,
        condition: &Expr<'_>,
    ) -> (BTreeMap<SymbolId, TypeId>, BTreeMap<SymbolId, TypeId>) {
        let mut then_map = BTreeMap::new();
        let mut else_map = BTreeMap::new();

        if let Some((name, tag, positive)) = match_typeof_equality(condition) {
            if let Some(symbol) = self.resolve_ident_symbol(&name) {
                if let Some(current) = self.lookup_symbol_type(symbol) {
                    let narrowed = self.narrow_by_typeof(current, &tag);
                    let excluded = self.exclude_typeof(current, &tag);
                    if positive {
                        then_map.insert(symbol, narrowed);
                        else_map.insert(symbol, excluded);
                    } else {
                        then_map.insert(symbol, excluded);
                        else_map.insert(symbol, narrowed);
                    }
                }
            }
            return (then_map, else_map);
        }

        if let ExprKind::Unary { op, expr: inner } = &condition.kind {
            if op.as_str() == "!" {
                if let ExprKind::Ident(name) = &inner.kind {
                    if let Some(symbol) = self.resolve_ident_symbol(name.as_str()) {
                        if let Some(current) = self.lookup_symbol_type(symbol) {
                            let truthy = self.narrow_truthy(current);
                            let falsy = self.narrow_falsy(current);
                            then_map.insert(symbol, falsy);
                            else_map.insert(symbol, truthy);
                        }
                    }
                }
                return (then_map, else_map);
            }
        }

        if let ExprKind::Ident(name) = &condition.kind {
            if let Some(symbol) = self.resolve_ident_symbol(name.as_str()) {
                if let Some(current) = self.lookup_symbol_type(symbol) {
                    then_map.insert(symbol, self.narrow_truthy(current));
                    else_map.insert(symbol, self.narrow_falsy(current));
                }
            }
        }

        (then_map, else_map)
    }

    fn narrow_by_typeof(&mut self, current: TypeId, tag: &str) -> TypeId {
        let target = typeof_tag_type(tag);
        let members = self.flatten_union(current);
        let mut kept = Vec::new();
        for m in members {
            if typeof_matches(self.arena.get(m), tag) || matches_primitive(self.arena.get(m), target)
            {
                kept.push(m);
            }
        }
        if kept.is_empty() {
            return self.arena.intern(match target {
                TypeofTag::String => Type::String,
                TypeofTag::Number => Type::Number,
                TypeofTag::Boolean => Type::Boolean,
                TypeofTag::Bigint => Type::Bigint,
                TypeofTag::Symbol => Type::Symbol,
                TypeofTag::Undefined => Type::Undefined,
                TypeofTag::Object => Type::Object,
                TypeofTag::Function => Type::Any,
                TypeofTag::Other => Type::Never,
            });
        }
        self.intern_union(kept)
    }

    fn exclude_typeof(&mut self, current: TypeId, tag: &str) -> TypeId {
        let target = typeof_tag_type(tag);
        let members = self.flatten_union(current);
        let mut kept = Vec::new();
        for m in members {
            if !(typeof_matches(self.arena.get(m), tag)
                || matches_primitive(self.arena.get(m), target))
            {
                kept.push(m);
            }
        }
        if kept.is_empty() {
            return self.arena.intern(Type::Never);
        }
        self.intern_union(kept)
    }

    fn narrow_truthy(&mut self, current: TypeId) -> TypeId {
        let members = self.flatten_union(current);
        let mut kept = Vec::new();
        for m in members {
            if !is_definitely_falsy(self.arena.get(m)) {
                kept.push(m);
            }
        }
        if kept.is_empty() {
            return self.arena.intern(Type::Never);
        }
        self.intern_union(kept)
    }

    fn narrow_falsy(&mut self, current: TypeId) -> TypeId {
        let members = self.flatten_union(current);
        let mut kept = Vec::new();
        for m in members {
            if is_possibly_falsy(self.arena.get(m)) {
                kept.push(m);
            }
        }
        if kept.is_empty() {
            return self.arena.intern(Type::Never);
        }
        self.intern_union(kept)
    }

    // --- M8 component props ---

    fn check_markup_node(&mut self, node: &MarkupNode<'_>) {
        match node {
            MarkupNode::Element(el) => self.check_markup_element(el),
            MarkupNode::Fragment { children, .. } => {
                for child in children {
                    if let nori_ast::MarkupChild::Node(n) = child {
                        self.check_markup_node(n);
                    } else if let nori_ast::MarkupChild::Expr(e) = child {
                        let _ = self.infer_expr_type(e);
                    }
                }
            }
        }
    }

    fn check_markup_element(&mut self, el: &MarkupElement<'_>) {
        let name = el.name.as_str();
        // Intrinsic elements start lowercase; components are capitalized.
        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
            if let Some(&props_ty) = self.component_props.get(name) {
                let Type::ObjectShape { props } = self.arena.get(props_ty).clone() else {
                    return;
                };
                for attr in &el.attributes {
                    if let MarkupAttribute::Named {
                        name: attr_name,
                        value,
                        span,
                    } = attr
                    {
                        let attr_key = attr_name.as_str();
                        let Some(prop) = props.iter().find(|p| p.name == attr_key) else {
                            continue;
                        };
                        let value_ty = match value {
                            Some(v) => self.infer_expr_type(v),
                            None => self.arena.intern(Type::BooleanLiteral(true)),
                        };
                        if !self.is_assignable(prop.ty, value_ty) {
                            self.diagnostics.push(Diagnostic {
                                message: format!(
                                    "type '{}' is not assignable to prop '{}' of type '{}'",
                                    type_display(self.arena.get(value_ty)),
                                    attr_key,
                                    type_display(self.arena.get(prop.ty)),
                                ),
                                severity: Severity::Error,
                                span: nori_diagnostic::span(
                                    span.start as usize,
                                    span.end as usize,
                                ),
                                code: "nori::check",
                            });
                        }
                    }
                }
            }
        }

        for attr in &el.attributes {
            match attr {
                MarkupAttribute::Named {
                    value: Some(v), ..
                }
                | MarkupAttribute::Spread { expr: v, .. } => {
                    let _ = self.infer_expr_type(v);
                }
                MarkupAttribute::Named { value: None, .. } => {}
            }
        }
        for child in &el.children {
            match child {
                nori_ast::MarkupChild::Node(n) => self.check_markup_node(n),
                nori_ast::MarkupChild::Expr(e) => {
                    let _ = self.infer_expr_type(e);
                }
                nori_ast::MarkupChild::Text(_, _) => {}
            }
        }
    }
}

impl<'a> Visit<'a> for Checker<'a> {
    fn visit_stmt(&mut self, stmt: &Stmt<'a>) {
        match stmt {
            Stmt::Var(var) => {
                self.check_var_decl(var);
                for declarator in &var.declarators {
                    if let Some(init) = &declarator.init {
                        self.visit_expr(init);
                    }
                }
            }
            Stmt::Function(func) | Stmt::ExportDefaultFunction(func) => {
                self.check_function(func);
            }
            Stmt::If(if_stmt) => {
                self.check_if(if_stmt);
            }
            Stmt::Return(expr, span) => {
                if let Some(Some(expected)) = self.return_stack.last().copied() {
                    let actual = match expr {
                        Some(e) => self.infer_expr_type(e),
                        None => self.arena.intern(Type::Void),
                    };
                    if !self.is_assignable(expected, actual) {
                        self.diagnostics.push(Diagnostic {
                            message: format!(
                                "type '{}' is not assignable to return type '{}'",
                                type_display(self.arena.get(actual)),
                                type_display(self.arena.get(expected)),
                            ),
                            severity: Severity::Error,
                            span: nori_diagnostic::span(span.start as usize, span.end as usize),
                            code: "nori::check",
                        });
                    }
                }
                if let Some(expr) = expr {
                    self.visit_expr(expr);
                }
            }
            other => nori_ast::walk_stmt(self, other),
        }
    }

    fn visit_expr(&mut self, expr: &Expr<'a>) {
        match &expr.kind {
            ExprKind::Call { .. } => {
                let _ = self.infer_expr_type(expr);
            }
            ExprKind::Markup(node) => {
                self.check_markup_node(node);
            }
            _ => nori_ast::walk_expr(self, expr),
        }
    }
}

#[derive(Clone, Copy)]
enum TypeofTag {
    String,
    Number,
    Boolean,
    Bigint,
    Symbol,
    Undefined,
    Object,
    Function,
    Other,
}

fn typeof_tag_type(tag: &str) -> TypeofTag {
    match tag {
        "string" => TypeofTag::String,
        "number" => TypeofTag::Number,
        "boolean" => TypeofTag::Boolean,
        "bigint" => TypeofTag::Bigint,
        "symbol" => TypeofTag::Symbol,
        "undefined" => TypeofTag::Undefined,
        "object" => TypeofTag::Object,
        "function" => TypeofTag::Function,
        _ => TypeofTag::Other,
    }
}

fn typeof_matches(ty: &Type, tag: &str) -> bool {
    matches_primitive(ty, typeof_tag_type(tag))
}

fn matches_primitive(ty: &Type, tag: TypeofTag) -> bool {
    match tag {
        TypeofTag::String => matches!(ty, Type::String | Type::StringLiteral(_)),
        TypeofTag::Number => matches!(ty, Type::Number | Type::NumberLiteral(_)),
        TypeofTag::Boolean => matches!(ty, Type::Boolean | Type::BooleanLiteral(_)),
        TypeofTag::Bigint => matches!(ty, Type::Bigint),
        TypeofTag::Symbol => matches!(ty, Type::Symbol),
        TypeofTag::Undefined => matches!(ty, Type::Undefined),
        TypeofTag::Object => {
            matches!(
                ty,
                Type::Object | Type::ObjectShape { .. } | Type::Null | Type::Reference(_)
            )
        }
        TypeofTag::Function => matches!(ty, Type::Function { .. }),
        TypeofTag::Other => false,
    }
}

fn is_definitely_falsy(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Null | Type::Undefined | Type::BooleanLiteral(false)
    )
}

fn is_possibly_falsy(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Null
            | Type::Undefined
            | Type::Boolean
            | Type::BooleanLiteral(false)
            | Type::String
            | Type::Number
            | Type::Any
            | Type::Unknown
    )
}

fn is_concrete(ty: &Type) -> bool {
    !matches!(ty, Type::Reference(_) | Type::Complex | Type::Unknown)
}

/// Match `typeof x === "string"` / `!==` / reversed forms.
fn match_typeof_equality(expr: &Expr<'_>) -> Option<(String, String, bool)> {
    let ExprKind::Binary { left, op, right } = &expr.kind else {
        return None;
    };
    let positive = match op.as_str() {
        "===" | "==" => true,
        "!==" | "!=" => false,
        _ => return None,
    };

    match (&left.kind, &right.kind) {
        (ExprKind::Typeof(inner), ExprKind::String(lit)) => {
            if let ExprKind::Ident(name) = &inner.kind {
                return Some((
                    name.as_str().to_string(),
                    unquote(lit.as_str()),
                    positive,
                ));
            }
        }
        (ExprKind::String(lit), ExprKind::Typeof(inner)) => {
            if let ExprKind::Ident(name) = &inner.kind {
                return Some((
                    name.as_str().to_string(),
                    unquote(lit.as_str()),
                    positive,
                ));
            }
        }
        _ => {}
    }
    None
}

fn prop_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::Ident(name) | PropertyKey::String(name) => Some(unquote(name.as_str())),
        PropertyKey::Number(n) => Some(n.as_str().to_string()),
        PropertyKey::Computed(_) => None,
    }
}

fn unquote(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn type_display(ty: &Type) -> String {
    match ty {
        Type::Any => "any".into(),
        Type::Unknown => "unknown".into(),
        Type::Never => "never".into(),
        Type::String => "string".into(),
        Type::Number => "number".into(),
        Type::Boolean => "boolean".into(),
        Type::Symbol => "symbol".into(),
        Type::Bigint => "bigint".into(),
        Type::Object => "object".into(),
        Type::Void => "void".into(),
        Type::Undefined => "undefined".into(),
        Type::Null => "null".into(),
        Type::StringLiteral(s) => format!("\"{s}\""),
        Type::NumberLiteral(n) => n.clone(),
        Type::BooleanLiteral(b) => b.to_string(),
        Type::Reference(name) => name.clone(),
        Type::ObjectShape { props } => {
            let inner = props
                .iter()
                .map(|p| {
                    format!(
                        "{}{}: {}",
                        p.name,
                        if p.optional { "?" } else { "" },
                        "{...}"
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("{{ {inner} }}")
        }
        Type::Function { .. } => "function".into(),
        Type::Union(_) => "union".into(),
        Type::Complex => "complex".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nori_allocator::Allocator;

    fn check_source(source: &str) -> CheckResult {
        let allocator = Allocator::new();
        let tokens = nori_lexer::lex(source).expect("lex");
        let program =
            nori_parser::parse_in(&allocator, source, "<test>.ts", tokens).expect("parse");
        check(&program)
    }

    #[test]
    fn assignability_rejects_number_to_string() {
        let result = check_source("let x: string = 1;");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not assignable")),
            "expected assignability error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn assignability_accepts_string_to_string() {
        let result = check_source("let x: string = \"hi\";");
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn assignability_accepts_number_literal_to_number() {
        let result = check_source("const n: number = 42;");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn assignability_rejects_boolean_to_number() {
        let result = check_source("let flag: number = true;");
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn structural_assignability_accepts_matching_object() {
        let result = check_source(
            r#"
interface Point { x: number; y: number }
const p: Point = { x: 1, y: 2 };
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "unexpected: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn excess_property_check_rejects_unknown_keys() {
        let result = check_source(
            r#"
interface Point { x: number; y: number }
const p: Point = { x: 1, y: 2, z: 3 };
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("z") || d.message.contains("known properties")),
            "expected excess property error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn function_return_type_is_checked() {
        let result = check_source(
            r#"
function f(): string {
  return 1;
}
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("return type")),
            "expected return type error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn function_param_types_are_checked_at_calls() {
        let result = check_source(
            r#"
function take(s: string) {}
take(1);
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("argument") || d.message.contains("parameter")),
            "expected param error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn generic_identity_alias_instantiates() {
        let result = check_source(
            r#"
type Id<T> = T;
const ok: Id<string> = "hi";
const bad: Id<string> = 1;
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not assignable")),
            "expected generic instantiation assignability error, got {:?}",
            result.diagnostics
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("not assignable"))
                .count(),
            1
        );
    }

    // --- M5: control-flow narrowing ---

    #[test]
    fn typeof_narrowing_accepts_string_in_then_branch() {
        let result = check_source(
            r#"
let x: string | number = 1;
if (typeof x === "string") {
  let s: string = x;
}
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "unexpected: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn typeof_narrowing_rejects_number_in_then_branch() {
        let result = check_source(
            r#"
let x: string | number = 1;
if (typeof x === "string") {
  let n: number = x;
}
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not assignable")),
            "expected assignability error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn typeof_narrowing_else_branch_excludes_string() {
        let result = check_source(
            r#"
let x: string | number = 1;
if (typeof x === "string") {
} else {
  let n: number = x;
}
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "unexpected: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn truthiness_narrowing_removes_null() {
        let result = check_source(
            r#"
let x: string | null = null;
if (x) {
  let s: string = x;
}
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "unexpected: {:?}",
            result.diagnostics
        );
    }

    // --- M6: conditional types + keyof ---

    #[test]
    fn conditional_type_evaluates_when_concrete() {
        let result = check_source(
            r#"
type IsString<T> = T extends string ? true : false;
const yes: IsString<"hi"> = true;
const no: IsString<number> = false;
const bad: IsString<number> = true;
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not assignable")),
            "expected IsString<number> = true to fail, got {:?}",
            result.diagnostics
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("not assignable"))
                .count(),
            1
        );
    }

    #[test]
    fn keyof_object_alias_yields_string_literal_union() {
        let result = check_source(
            r#"
type Point = { a: number; b: string };
type Keys = keyof Point;
const ok: Keys = "a";
const bad: Keys = "c";
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not assignable")),
            "expected keyof assignability error, got {:?}",
            result.diagnostics
        );
    }

    // --- M7: multi-file + lib globals ---

    #[test]
    fn check_files_aggregates_diagnostics_across_files() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "nori-checker-m7-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.ts");
        let b = dir.join("b.ts");
        {
            let mut f = std::fs::File::create(&a).unwrap();
            writeln!(f, "let x: string = 1;").unwrap();
        }
        {
            let mut f = std::fs::File::create(&b).unwrap();
            writeln!(f, "let y: number = true;").unwrap();
        }
        let result = check_files(&[a.as_path(), b.as_path()]);
        assert!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("not assignable"))
                .count()
                >= 2,
            "expected errors from both files, got {:?}",
            result.diagnostics
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lib_globals_include_array_promise_string() {
        let globals = lib_es5_globals();
        for name in ["Array", "Promise", "String"] {
            assert!(
                globals.contains(name),
                "expected lib global `{name}` in {globals:?}"
            );
        }
    }

    #[test]
    fn lib_globals_array_annotation_is_accepted() {
        let result = check_source(
            r#"
let xs: Array<number>;
let ys: Array<number> = xs;
let p: Promise<string>;
let q: Promise<string> = p;
let s: String;
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "lib globals should resolve, got {:?}",
            result.diagnostics
        );
    }

    // --- M8: component prop checking ---

    #[test]
    fn component_prop_types_are_checked() {
        let result = check_source(
            r#"
function Foo(props: { bar: string }) {
  return <div />;
}
const el = <Foo bar={1} />;
"#,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| {
                    d.message.contains("bar")
                        || d.message.contains("not assignable")
                        || d.message.contains("prop")
                }),
            "expected prop type error, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn component_prop_types_accept_matching() {
        let result = check_source(
            r#"
function Foo(props: { bar: string }) {
  return <div />;
}
const el = <Foo bar={"hi"} />;
"#,
        );
        assert!(
            result.diagnostics.is_empty(),
            "unexpected: {:?}",
            result.diagnostics
        );
    }
}
