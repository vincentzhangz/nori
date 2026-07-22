//! Arena-allocated AST for Nori (oxc-style).
//!
//! Every node borrows from a single [`nori_allocator::Allocator`] via the
//! lifetime `'a`. Owned collections use [`nori_allocator::Vec`] and owned
//! pointers use [`nori_allocator::Box`]. Identifiers and other short strings
//! are interned as [`nori_allocator::Atom`], which borrows from the arena (and
//! usually points back into the original source text).

pub use nori_allocator::{Allocator, Atom, Box, Vec};
pub use nori_span::{SourceMap, SourcePosition, Span};

#[derive(Debug, PartialEq)]
pub struct Program<'a> {
    pub body: Vec<'a, Stmt<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Stmt<'a> {
    Import(ImportDecl<'a>),
    /// Fallback for TypeScript constructs we have not fully parsed yet.
    TypeOnly(RawStmt),
    TypeAlias(TypeAliasDecl<'a>),
    Interface(InterfaceDecl<'a>),
    Enum(EnumDecl<'a>),
    Module(ModuleDecl<'a>),
    Class(ClassDecl<'a>),
    Var(VarDecl<'a>),
    Function(FunctionDecl<'a>),
    Export(ExportDecl<'a>),
    ExportDefaultFunction(FunctionDecl<'a>),
    ExportDefaultExpr(Expr<'a>),
    Return(Option<Expr<'a>>, Span),
    Expr(Expr<'a>),
    Block(BlockStmt<'a>),
    If(IfStmt<'a>),
    Try(TryStmt<'a>),
    For(ForStmt<'a>),
    ClassicFor(Box<'a, ClassicForStmt<'a>>),
    While(WhileStmt<'a>),
    DoWhile(DoWhileStmt<'a>),
    Switch(SwitchStmt<'a>),
    Throw(ThrowStmt<'a>),
    Label(LabelStmt<'a>),
    Debugger(Span),
    With(WithStmt<'a>),
    Break(Span),
    Continue(Span),
    Raw(RawStmt),
}

#[derive(Debug, PartialEq)]
pub struct TryStmt<'a> {
    pub body: BlockStmt<'a>,
    pub catch_param: Option<Atom<'a>>,
    pub catch_body: BlockStmt<'a>,
    pub finally_body: Option<BlockStmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ForStmt<'a> {
    pub variable: VarKind,
    pub name: Atom<'a>,
    pub iterable: Expr<'a>,
    pub is_of: bool,
    pub body: Box<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassicForStmt<'a> {
    pub init: Option<ForInit<'a>>,
    pub condition: Option<Expr<'a>>,
    pub update: Option<Expr<'a>>,
    pub body: Box<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum ForInit<'a> {
    Var(VarDecl<'a>),
    Expr(Expr<'a>),
}

#[derive(Debug, PartialEq)]
pub struct WhileStmt<'a> {
    pub condition: Expr<'a>,
    pub body: Box<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct DoWhileStmt<'a> {
    pub body: Box<'a, Stmt<'a>>,
    pub condition: Expr<'a>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct SwitchStmt<'a> {
    pub discriminant: Expr<'a>,
    pub cases: Vec<'a, SwitchCase<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct SwitchCase<'a> {
    pub test: Option<Expr<'a>>,
    pub consequent: Vec<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ThrowStmt<'a> {
    pub argument: Expr<'a>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct LabelStmt<'a> {
    pub label: Atom<'a>,
    pub body: Box<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct WithStmt<'a> {
    pub object: Expr<'a>,
    pub body: Box<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassDecl<'a> {
    pub name: Atom<'a>,
    pub extends: Option<Atom<'a>>,
    pub members: Vec<'a, ClassMember<'a>>,
    pub decorators: Vec<'a, Decorator<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct Decorator<'a> {
    pub name: Atom<'a>,
    pub args: Option<Vec<'a, Expr<'a>>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum ClassMember<'a> {
    Field(ClassField<'a>),
    Constructor(ClassConstructor<'a>),
    Method(ClassMethod<'a>),
    Accessor(ClassAccessor<'a>),
    StaticBlock(ClassStaticBlock<'a>),
}

#[derive(Debug, PartialEq)]
pub struct ClassField<'a> {
    pub name: Atom<'a>,
    pub value: Option<Expr<'a>>,
    pub is_static: bool,
    pub is_private: bool,
    pub computed: Option<Box<'a, Expr<'a>>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassConstructor<'a> {
    pub params: Vec<'a, Param<'a>>,
    pub body: BlockStmt<'a>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassMethod<'a> {
    pub name: Atom<'a>,
    pub params: Vec<'a, Param<'a>>,
    pub body: BlockStmt<'a>,
    pub is_static: bool,
    pub is_async: bool,
    pub is_get: bool,
    pub is_set: bool,
    pub is_private: bool,
    pub computed: Option<Box<'a, Expr<'a>>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassAccessor<'a> {
    pub name: Atom<'a>,
    pub params: Vec<'a, Param<'a>>,
    pub body: BlockStmt<'a>,
    pub is_static: bool,
    pub is_get: bool,
    pub is_private: bool,
    pub computed: Option<Box<'a, Expr<'a>>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ClassStaticBlock<'a> {
    pub body: BlockStmt<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawStmt {
    pub span: Span,
}

/// TypeScript type AST (oxc-style `TSType`).
#[derive(Debug, PartialEq)]
pub enum TSType<'a> {
    Keyword(TSKeywordKind, Span),
    Reference {
        name: Atom<'a>,
        type_args: Option<Vec<'a, TSType<'a>>>,
        span: Span,
    },
    Union(Vec<'a, TSType<'a>>, Span),
    Intersection(Vec<'a, TSType<'a>>, Span),
    Array(Box<'a, TSType<'a>>, Span),
    Tuple(Vec<'a, TSType<'a>>, Span),
    Object(Vec<'a, TSTypeElement<'a>>, Span),
    Function {
        params: Vec<'a, TSFnParam<'a>>,
        return_type: Box<'a, TSType<'a>>,
        span: Span,
    },
    Conditional {
        check: Box<'a, TSType<'a>>,
        extends: Box<'a, TSType<'a>>,
        true_type: Box<'a, TSType<'a>>,
        false_type: Box<'a, TSType<'a>>,
        span: Span,
    },
    Infer {
        name: Atom<'a>,
        span: Span,
    },
    Typeof {
        expr_name: Atom<'a>,
        span: Span,
    },
    IndexedAccess {
        object: Box<'a, TSType<'a>>,
        index: Box<'a, TSType<'a>>,
        span: Span,
    },
    Operator {
        op: TSTypeOperator,
        operand: Box<'a, TSType<'a>>,
        span: Span,
    },
    /// `{ [K in keyof T]: U }` (and optional `readonly` / `?` modifiers).
    Mapped {
        readonly: bool,
        key: Atom<'a>,
        constraint: Box<'a, TSType<'a>>,
        optional: bool,
        type_ann: Box<'a, TSType<'a>>,
        span: Span,
    },
    /// `` `foo${string}` `` template literal types.
    TemplateLiteral {
        quasis: Vec<'a, Atom<'a>>,
        types: Vec<'a, TSType<'a>>,
        span: Span,
    },
    Literal(TSLiteral<'a>),
    Parenthesized(Box<'a, TSType<'a>>, Span),
    This(Span),
    /// Fallback when the type grammar cannot fully parse a fragment.
    Any(Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TSKeywordKind {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TSTypeOperator {
    Keyof,
    Readonly,
    Unique,
}

#[derive(Debug, PartialEq)]
pub enum TSLiteral<'a> {
    String(Atom<'a>, Span),
    Number(Atom<'a>, Span),
    Bool(bool, Span),
    Null(Span),
}

#[derive(Debug, PartialEq)]
pub enum TSTypeElement<'a> {
    Property {
        key: Atom<'a>,
        optional: bool,
        readonly: bool,
        type_ann: Option<Box<'a, TSType<'a>>>,
        span: Span,
    },
    Method {
        key: Atom<'a>,
        optional: bool,
        params: Vec<'a, TSFnParam<'a>>,
        return_type: Option<Box<'a, TSType<'a>>>,
        span: Span,
    },
    Index {
        key_name: Atom<'a>,
        key_type: Box<'a, TSType<'a>>,
        type_ann: Box<'a, TSType<'a>>,
        span: Span,
    },
    Call {
        params: Vec<'a, TSFnParam<'a>>,
        return_type: Option<Box<'a, TSType<'a>>>,
        span: Span,
    },
    Construct {
        params: Vec<'a, TSFnParam<'a>>,
        return_type: Option<Box<'a, TSType<'a>>>,
        span: Span,
    },
}

#[derive(Debug, PartialEq)]
pub struct TSFnParam<'a> {
    pub name: Atom<'a>,
    pub optional: bool,
    pub type_ann: Option<Box<'a, TSType<'a>>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct TypeAliasDecl<'a> {
    pub name: Atom<'a>,
    /// Simple type parameter names (`type Box<T> = ...`). Bounds are erased.
    pub type_params: Vec<'a, Atom<'a>>,
    pub type_ann: Box<'a, TSType<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct InterfaceDecl<'a> {
    pub name: Atom<'a>,
    pub extends: Vec<'a, Atom<'a>>,
    pub body: Vec<'a, TSTypeElement<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct EnumDecl<'a> {
    pub name: Atom<'a>,
    pub is_const: bool,
    pub members: Vec<'a, EnumMember<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct EnumMember<'a> {
    pub name: Atom<'a>,
    pub init: Option<Expr<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ModuleDecl<'a> {
    pub name: Atom<'a>,
    pub body: BlockStmt<'a>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct ImportDecl<'a> {
    pub specifiers: Vec<'a, ImportSpecifier<'a>>,
    pub source: Atom<'a>,
    /// `import type { ... }` — erased at emit time.
    pub is_type: bool,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum ImportSpecifier<'a> {
    Default(Atom<'a>),
    Named {
        local: Atom<'a>,
        imported: Option<Atom<'a>>,
    },
    Namespace(Atom<'a>),
}

#[derive(Debug, PartialEq)]
pub enum ExportDecl<'a> {
    Named {
        specifiers: Vec<'a, ExportSpecifier<'a>>,
        source: Option<Atom<'a>>,
        /// `export type { ... }` — erased at emit time.
        is_type: bool,
        span: Span,
    },
    All {
        source: Atom<'a>,
        as_namespace: Option<Atom<'a>>,
        span: Span,
    },
    /// `export type Foo = ...` / `export type` interface — erased.
    TypeOnly(Span),
    Declaration(Box<'a, Stmt<'a>>),
}

#[derive(Debug, PartialEq)]
pub struct ExportSpecifier<'a> {
    pub local: Atom<'a>,
    pub exported: Option<Atom<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct BlockStmt<'a> {
    pub body: Vec<'a, Stmt<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct IfStmt<'a> {
    pub condition: Expr<'a>,
    pub consequent: Box<'a, Stmt<'a>>,
    pub alternate: Option<Box<'a, Stmt<'a>>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Const,
    Let,
    Var,
}

#[derive(Debug, PartialEq)]
pub struct VarDecl<'a> {
    pub kind: VarKind,
    pub declarators: Vec<'a, VarDeclarator<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct VarDeclarator<'a> {
    pub name: Atom<'a>,
    pub pattern: Option<Pattern<'a>>,
    pub type_ann: Option<Box<'a, TSType<'a>>>,
    pub init: Option<Expr<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum Pattern<'a> {
    Ident(Atom<'a>),
    Rest(Box<'a, Pattern<'a>>),
    Array {
        elements: Vec<'a, Option<Pattern<'a>>>,
        rest: Option<Box<'a, Pattern<'a>>>,
        span: Span,
    },
    Object {
        properties: Vec<'a, ObjectPatternProp<'a>>,
        rest: Option<Box<'a, Pattern<'a>>>,
        span: Span,
    },
    Assign {
        left: Box<'a, Pattern<'a>>,
        right: Box<'a, Expr<'a>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct ObjectPatternProp<'a> {
    pub key: Atom<'a>,
    pub alias: Option<Atom<'a>>,
    pub value: Option<Box<'a, Pattern<'a>>>,
    pub default: Option<Expr<'a>>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct FunctionDecl<'a> {
    pub name: Option<Atom<'a>>,
    pub params: Vec<'a, Param<'a>>,
    pub return_type: Option<Box<'a, TSType<'a>>>,
    pub body: BlockStmt<'a>,
    pub async_token: Option<Span>,
    pub generator: bool,
    pub decorators: Vec<'a, Span>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub struct Param<'a> {
    pub name: Atom<'a>,
    pub type_ann: Option<Box<'a, TSType<'a>>>,
    pub default: Option<Expr<'a>>,
    pub is_property: bool,
}

#[derive(Debug, PartialEq)]
pub struct Expr<'a> {
    pub kind: ExprKind<'a>,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum ExprKind<'a> {
    Ident(Atom<'a>),
    Number(Atom<'a>),
    BigInt(Atom<'a>),
    String(Atom<'a>),
    RegExp {
        pattern: Atom<'a>,
        flags: Atom<'a>,
    },
    Bool(bool),
    Null,
    This,
    New {
        callee: Box<'a, Expr<'a>>,
        args: Vec<'a, Expr<'a>>,
    },
    Delete(Box<'a, Expr<'a>>),
    Void(Box<'a, Expr<'a>>),
    Typeof(Box<'a, Expr<'a>>),
    MetaProperty {
        meta: Atom<'a>,
        property: Atom<'a>,
    },
    Import(Box<'a, Expr<'a>>),
    Sequence(Vec<'a, Expr<'a>>),
    Yield {
        value: Option<Box<'a, Expr<'a>>>,
        delegate: bool,
    },
    Unary {
        op: Atom<'a>,
        expr: Box<'a, Expr<'a>>,
    },
    Update {
        op: Atom<'a>,
        expr: Box<'a, Expr<'a>>,
        prefix: bool,
    },
    TypeErasure {
        kind: TypeErasureKind,
        expr: Box<'a, Expr<'a>>,
    },
    Binary {
        left: Box<'a, Expr<'a>>,
        op: Atom<'a>,
        right: Box<'a, Expr<'a>>,
    },
    Assign {
        left: Box<'a, Expr<'a>>,
        op: Atom<'a>,
        right: Box<'a, Expr<'a>>,
    },
    Conditional {
        test: Box<'a, Expr<'a>>,
        consequent: Box<'a, Expr<'a>>,
        alternate: Box<'a, Expr<'a>>,
    },
    Call {
        callee: Box<'a, Expr<'a>>,
        args: Vec<'a, Expr<'a>>,
        optional: bool,
    },
    Member {
        object: Box<'a, Expr<'a>>,
        property: Atom<'a>,
        optional: bool,
    },
    Index {
        object: Box<'a, Expr<'a>>,
        index: Box<'a, Expr<'a>>,
        optional: bool,
    },
    Arrow {
        params: Vec<'a, Atom<'a>>,
        body: ArrowBody<'a>,
    },
    TemplateLiteral {
        quasis: Vec<'a, Atom<'a>>,
        exprs: Vec<'a, Expr<'a>>,
    },
    TaggedTemplate {
        tag: Box<'a, Expr<'a>>,
        quasi: Box<'a, Expr<'a>>,
    },
    Array(Vec<'a, Expr<'a>>),
    Object(Vec<'a, ObjectProperty<'a>>),
    Spread {
        expr: Box<'a, Expr<'a>>,
    },
    Await(Box<'a, Expr<'a>>),
    Markup(MarkupNode<'a>),
    Raw,
}

#[derive(Debug, PartialEq)]
pub enum ArrowBody<'a> {
    Expression(Box<'a, Expr<'a>>),
    Block(BlockStmt<'a>),
}

#[derive(Debug, PartialEq)]
pub struct ObjectProperty<'a> {
    pub key: PropertyKey<'a>,
    pub value: Expr<'a>,
    pub kind: PropertyKind,
    pub computed: bool,
    pub shorthand: bool,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum PropertyKey<'a> {
    Ident(Atom<'a>),
    String(Atom<'a>),
    Number(Atom<'a>),
    Computed(Box<'a, Expr<'a>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Init,
    Get,
    Set,
    Method,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeErasureKind {
    As,
    Satisfies,
    NonNull,
}

#[derive(Debug, PartialEq)]
pub enum MarkupNode<'a> {
    Element(MarkupElement<'a>),
    Fragment {
        children: Vec<'a, MarkupChild<'a>>,
        span: Span,
    },
}

#[derive(Debug, PartialEq)]
pub struct MarkupElement<'a> {
    pub name: Atom<'a>,
    pub attributes: Vec<'a, MarkupAttribute<'a>>,
    pub children: Vec<'a, MarkupChild<'a>>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, PartialEq)]
pub enum MarkupAttribute<'a> {
    Named {
        name: Atom<'a>,
        value: Option<Expr<'a>>,
        span: Span,
    },
    Spread {
        expr: Expr<'a>,
        span: Span,
    },
}

#[derive(Debug, PartialEq)]
pub enum MarkupChild<'a> {
    Text(Atom<'a>, Span),
    Expr(Expr<'a>),
    Node(MarkupNode<'a>),
}

/// A lightweight, borrowed handle to any major AST node.
///
/// Useful for building tables/stacks of nodes without generics, mirroring
/// oxc's `AstKind`.
#[derive(Debug, Clone, Copy)]
pub enum AstKind<'a> {
    Program(&'a Program<'a>),
    Stmt(&'a Stmt<'a>),
    Expr(&'a Expr<'a>),
    Block(&'a BlockStmt<'a>),
    VarDecl(&'a VarDecl<'a>),
    VarDeclarator(&'a VarDeclarator<'a>),
    FunctionDecl(&'a FunctionDecl<'a>),
    ClassDecl(&'a ClassDecl<'a>),
    ClassMember(&'a ClassMember<'a>),
    Param(&'a Param<'a>),
    ImportDecl(&'a ImportDecl<'a>),
    ExportDecl(&'a ExportDecl<'a>),
    MarkupNode(&'a MarkupNode<'a>),
    MarkupElement(&'a MarkupElement<'a>),
    MarkupAttribute(&'a MarkupAttribute<'a>),
    MarkupChild(&'a MarkupChild<'a>),
}

impl<'a> AstKind<'a> {
    /// The source span of the underlying node, when it carries one.
    pub fn span(self) -> Option<Span> {
        match self {
            AstKind::Program(_) => None,
            AstKind::Stmt(_) => None,
            AstKind::Expr(node) => Some(node.span),
            AstKind::Block(node) => Some(node.span),
            AstKind::VarDecl(node) => Some(node.span),
            AstKind::VarDeclarator(node) => Some(node.span),
            AstKind::FunctionDecl(node) => Some(node.span),
            AstKind::ClassDecl(node) => Some(node.span),
            AstKind::ClassMember(_) => None,
            AstKind::Param(_) => None,
            AstKind::ImportDecl(node) => Some(node.span),
            AstKind::ExportDecl(_) => None,
            AstKind::MarkupNode(node) => Some(markup_node_span(node)),
            AstKind::MarkupElement(node) => Some(node.span),
            AstKind::MarkupAttribute(node) => Some(markup_attribute_span(node)),
            AstKind::MarkupChild(_) => None,
        }
    }
}

fn markup_node_span(node: &MarkupNode<'_>) -> Span {
    match node {
        MarkupNode::Element(element) => element.span,
        MarkupNode::Fragment { span, .. } => *span,
    }
}

fn markup_attribute_span(attribute: &MarkupAttribute<'_>) -> Span {
    match attribute {
        MarkupAttribute::Named { span, .. } | MarkupAttribute::Spread { span, .. } => *span,
    }
}

/// Immutable, hand-written AST visitor.
///
/// Default method bodies recurse into children via the free `walk_*`
/// functions, so implementors only override the nodes they care about.
pub trait Visit<'a>: Sized {
    fn visit_program(&mut self, program: &Program<'a>) {
        walk_program(self, program);
    }

    fn visit_stmt(&mut self, stmt: &Stmt<'a>) {
        walk_stmt(self, stmt);
    }

    fn visit_block(&mut self, block: &BlockStmt<'a>) {
        walk_block(self, block);
    }

    fn visit_expr(&mut self, expr: &Expr<'a>) {
        walk_expr(self, expr);
    }

    fn visit_markup_node(&mut self, node: &MarkupNode<'a>) {
        walk_markup_node(self, node);
    }
}

pub fn walk_program<'a, V: Visit<'a>>(visitor: &mut V, program: &Program<'a>) {
    for stmt in &program.body {
        visitor.visit_stmt(stmt);
    }
}

pub fn walk_block<'a, V: Visit<'a>>(visitor: &mut V, block: &BlockStmt<'a>) {
    for stmt in &block.body {
        visitor.visit_stmt(stmt);
    }
}

pub fn walk_stmt<'a, V: Visit<'a>>(visitor: &mut V, stmt: &Stmt<'a>) {
    match stmt {
        Stmt::Var(var) => {
            for declarator in &var.declarators {
                if let Some(type_ann) = &declarator.type_ann {
                    walk_ts_type(visitor, type_ann);
                }
                if let Some(init) = &declarator.init {
                    visitor.visit_expr(init);
                }
            }
        }
        Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
            for param in &function.params {
                if let Some(type_ann) = &param.type_ann {
                    walk_ts_type(visitor, type_ann);
                }
                if let Some(default) = &param.default {
                    visitor.visit_expr(default);
                }
            }
            if let Some(return_type) = &function.return_type {
                walk_ts_type(visitor, return_type);
            }
            visitor.visit_block(&function.body);
        }
        Stmt::Return(Some(expr), _)
        | Stmt::Expr(expr)
        | Stmt::ExportDefaultExpr(expr)
        | Stmt::Throw(ThrowStmt { argument: expr, .. }) => visitor.visit_expr(expr),
        Stmt::Block(block) => visitor.visit_block(block),
        Stmt::If(if_stmt) => {
            visitor.visit_expr(&if_stmt.condition);
            visitor.visit_stmt(&if_stmt.consequent);
            if let Some(alternate) = &if_stmt.alternate {
                visitor.visit_stmt(alternate);
            }
        }
        Stmt::Class(class) => {
            for member in &class.members {
                walk_class_member(visitor, member);
            }
        }
        Stmt::Try(try_stmt) => {
            visitor.visit_block(&try_stmt.body);
            visitor.visit_block(&try_stmt.catch_body);
            if let Some(finally_body) = &try_stmt.finally_body {
                visitor.visit_block(finally_body);
            }
        }
        Stmt::For(for_stmt) => {
            visitor.visit_expr(&for_stmt.iterable);
            visitor.visit_stmt(&for_stmt.body);
        }
        Stmt::ClassicFor(for_stmt) => {
            if let Some(init) = &for_stmt.init {
                match init {
                    ForInit::Var(var) => {
                        for declarator in &var.declarators {
                            if let Some(init) = &declarator.init {
                                visitor.visit_expr(init);
                            }
                        }
                    }
                    ForInit::Expr(expr) => visitor.visit_expr(expr),
                }
            }
            if let Some(condition) = &for_stmt.condition {
                visitor.visit_expr(condition);
            }
            if let Some(update) = &for_stmt.update {
                visitor.visit_expr(update);
            }
            visitor.visit_stmt(&for_stmt.body);
        }
        Stmt::While(while_stmt) => {
            visitor.visit_expr(&while_stmt.condition);
            visitor.visit_stmt(&while_stmt.body);
        }
        Stmt::DoWhile(do_while) => {
            visitor.visit_stmt(&do_while.body);
            visitor.visit_expr(&do_while.condition);
        }
        Stmt::Switch(switch) => {
            visitor.visit_expr(&switch.discriminant);
            for case in &switch.cases {
                if let Some(test) = &case.test {
                    visitor.visit_expr(test);
                }
                for stmt in &case.consequent {
                    visitor.visit_stmt(stmt);
                }
            }
        }
        Stmt::Label(label) => visitor.visit_stmt(&label.body),
        Stmt::With(with) => {
            visitor.visit_expr(&with.object);
            visitor.visit_stmt(&with.body);
        }
        Stmt::Export(ExportDecl::Declaration(stmt)) => visitor.visit_stmt(stmt),
        Stmt::TypeAlias(alias) => walk_ts_type(visitor, &alias.type_ann),
        Stmt::Interface(iface) => {
            for element in &iface.body {
                walk_ts_type_element(visitor, element);
            }
        }
        Stmt::Enum(enum_decl) => {
            for member in &enum_decl.members {
                if let Some(init) = &member.init {
                    visitor.visit_expr(init);
                }
            }
        }
        Stmt::Module(module) => visitor.visit_block(&module.body),
        Stmt::Import(_)
        | Stmt::Export(_)
        | Stmt::TypeOnly(_)
        | Stmt::Return(None, _)
        | Stmt::Debugger(_)
        | Stmt::Break(_)
        | Stmt::Continue(_)
        | Stmt::Raw(_) => {}
    }
}

fn walk_ts_type<'a, V: Visit<'a>>(visitor: &mut V, ty: &TSType<'a>) {
    match ty {
        TSType::Reference {
            type_args: Some(args),
            ..
        } => {
            for arg in args {
                walk_ts_type(visitor, arg);
            }
        }
        TSType::Union(types, _) | TSType::Intersection(types, _) | TSType::Tuple(types, _) => {
            for ty in types {
                walk_ts_type(visitor, ty);
            }
        }
        TSType::Array(inner, _) | TSType::Parenthesized(inner, _) => walk_ts_type(visitor, inner),
        TSType::Object(elements, _) => {
            for element in elements {
                walk_ts_type_element(visitor, element);
            }
        }
        TSType::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                if let Some(type_ann) = &param.type_ann {
                    walk_ts_type(visitor, type_ann);
                }
            }
            walk_ts_type(visitor, return_type);
        }
        TSType::Conditional {
            check,
            extends,
            true_type,
            false_type,
            ..
        } => {
            walk_ts_type(visitor, check);
            walk_ts_type(visitor, extends);
            walk_ts_type(visitor, true_type);
            walk_ts_type(visitor, false_type);
        }
        TSType::IndexedAccess { object, index, .. } => {
            walk_ts_type(visitor, object);
            walk_ts_type(visitor, index);
        }
        TSType::Operator { operand, .. } => walk_ts_type(visitor, operand),
        TSType::Mapped {
            constraint,
            type_ann,
            ..
        } => {
            walk_ts_type(visitor, constraint);
            walk_ts_type(visitor, type_ann);
        }
        TSType::TemplateLiteral { types, .. } => {
            for ty in types {
                walk_ts_type(visitor, ty);
            }
        }
        TSType::Keyword(_, _)
        | TSType::Reference {
            type_args: None, ..
        }
        | TSType::Infer { .. }
        | TSType::Typeof { .. }
        | TSType::Literal(_)
        | TSType::This(_)
        | TSType::Any(_) => {}
    }
}

fn walk_ts_type_element<'a, V: Visit<'a>>(visitor: &mut V, element: &TSTypeElement<'a>) {
    match element {
        TSTypeElement::Property { type_ann, .. } => {
            if let Some(type_ann) = type_ann {
                walk_ts_type(visitor, type_ann);
            }
        }
        TSTypeElement::Method {
            params,
            return_type,
            ..
        }
        | TSTypeElement::Call {
            params,
            return_type,
            ..
        }
        | TSTypeElement::Construct {
            params,
            return_type,
            ..
        } => {
            for param in params {
                if let Some(type_ann) = &param.type_ann {
                    walk_ts_type(visitor, type_ann);
                }
            }
            if let Some(return_type) = return_type {
                walk_ts_type(visitor, return_type);
            }
        }
        TSTypeElement::Index {
            key_type, type_ann, ..
        } => {
            walk_ts_type(visitor, key_type);
            walk_ts_type(visitor, type_ann);
        }
    }
}

fn walk_class_member<'a, V: Visit<'a>>(visitor: &mut V, member: &ClassMember<'a>) {
    match member {
        ClassMember::Field(field) => {
            if let Some(computed) = &field.computed {
                visitor.visit_expr(computed);
            }
            if let Some(value) = &field.value {
                visitor.visit_expr(value);
            }
        }
        ClassMember::Constructor(constructor) => {
            for param in &constructor.params {
                if let Some(default) = &param.default {
                    visitor.visit_expr(default);
                }
            }
            visitor.visit_block(&constructor.body);
        }
        ClassMember::Method(method) => {
            if let Some(computed) = &method.computed {
                visitor.visit_expr(computed);
            }
            for param in &method.params {
                if let Some(default) = &param.default {
                    visitor.visit_expr(default);
                }
            }
            visitor.visit_block(&method.body);
        }
        ClassMember::Accessor(accessor) => {
            if let Some(computed) = &accessor.computed {
                visitor.visit_expr(computed);
            }
            for param in &accessor.params {
                if let Some(default) = &param.default {
                    visitor.visit_expr(default);
                }
            }
            visitor.visit_block(&accessor.body);
        }
        ClassMember::StaticBlock(block) => visitor.visit_block(&block.body),
    }
}

pub fn walk_expr<'a, V: Visit<'a>>(visitor: &mut V, expr: &Expr<'a>) {
    match &expr.kind {
        ExprKind::New { callee, args } | ExprKind::Call { callee, args, .. } => {
            visitor.visit_expr(callee);
            for arg in args {
                visitor.visit_expr(arg);
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
        | ExprKind::TypeErasure { expr: inner, .. } => visitor.visit_expr(inner),
        ExprKind::Binary { left, right, .. } | ExprKind::Assign { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            visitor.visit_expr(test);
            visitor.visit_expr(consequent);
            visitor.visit_expr(alternate);
        }
        ExprKind::Member { object, .. } => visitor.visit_expr(object),
        ExprKind::Index { object, index, .. } => {
            visitor.visit_expr(object);
            visitor.visit_expr(index);
        }
        ExprKind::Arrow { body, .. } => match body {
            ArrowBody::Expression(expr) => visitor.visit_expr(expr),
            ArrowBody::Block(block) => visitor.visit_block(block),
        },
        ExprKind::TemplateLiteral { exprs, .. } | ExprKind::Sequence(exprs) => {
            for expr in exprs {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::TaggedTemplate { tag, quasi } => {
            visitor.visit_expr(tag);
            visitor.visit_expr(quasi);
        }
        ExprKind::Array(items) => {
            for item in items {
                visitor.visit_expr(item);
            }
        }
        ExprKind::Object(properties) => {
            for prop in properties {
                if let PropertyKey::Computed(key) = &prop.key {
                    visitor.visit_expr(key);
                }
                visitor.visit_expr(&prop.value);
            }
        }
        ExprKind::Yield { value, .. } => {
            if let Some(value) = value {
                visitor.visit_expr(value);
            }
        }
        ExprKind::Markup(node) => visitor.visit_markup_node(node),
        ExprKind::Ident(_)
        | ExprKind::Number(_)
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

pub fn walk_markup_node<'a, V: Visit<'a>>(visitor: &mut V, node: &MarkupNode<'a>) {
    match node {
        MarkupNode::Element(element) => {
            for attribute in &element.attributes {
                match attribute {
                    MarkupAttribute::Named {
                        value: Some(value), ..
                    }
                    | MarkupAttribute::Spread { expr: value, .. } => visitor.visit_expr(value),
                    MarkupAttribute::Named { value: None, .. } => {}
                }
            }
            for child in &element.children {
                walk_markup_child(visitor, child);
            }
        }
        MarkupNode::Fragment { children, .. } => {
            for child in children {
                walk_markup_child(visitor, child);
            }
        }
    }
}

fn walk_markup_child<'a, V: Visit<'a>>(visitor: &mut V, child: &MarkupChild<'a>) {
    match child {
        MarkupChild::Expr(expr) => visitor.visit_expr(expr),
        MarkupChild::Node(node) => visitor.visit_markup_node(node),
        MarkupChild::Text(_, _) => {}
    }
}

/// Mutable, hand-written AST visitor (mirrors [`Visit`]).
pub trait VisitMut<'a>: Sized {
    fn visit_program(&mut self, program: &mut Program<'a>) {
        walk_program_mut(self, program);
    }

    fn visit_stmt(&mut self, stmt: &mut Stmt<'a>) {
        walk_stmt_mut(self, stmt);
    }

    fn visit_block(&mut self, block: &mut BlockStmt<'a>) {
        walk_block_mut(self, block);
    }

    fn visit_expr(&mut self, expr: &mut Expr<'a>) {
        walk_expr_mut(self, expr);
    }

    fn visit_markup_node(&mut self, node: &mut MarkupNode<'a>) {
        walk_markup_node_mut(self, node);
    }
}

pub fn walk_program_mut<'a, V: VisitMut<'a>>(visitor: &mut V, program: &mut Program<'a>) {
    for stmt in program.body.iter_mut() {
        visitor.visit_stmt(stmt);
    }
}

pub fn walk_block_mut<'a, V: VisitMut<'a>>(visitor: &mut V, block: &mut BlockStmt<'a>) {
    for stmt in block.body.iter_mut() {
        visitor.visit_stmt(stmt);
    }
}

pub fn walk_stmt_mut<'a, V: VisitMut<'a>>(visitor: &mut V, stmt: &mut Stmt<'a>) {
    match stmt {
        Stmt::Var(var) => {
            for declarator in var.declarators.iter_mut() {
                if let Some(type_ann) = &mut declarator.type_ann {
                    walk_ts_type_mut(visitor, type_ann);
                }
                if let Some(init) = &mut declarator.init {
                    visitor.visit_expr(init);
                }
            }
        }
        Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => {
            for param in function.params.iter_mut() {
                if let Some(type_ann) = &mut param.type_ann {
                    walk_ts_type_mut(visitor, type_ann);
                }
                if let Some(default) = &mut param.default {
                    visitor.visit_expr(default);
                }
            }
            if let Some(return_type) = &mut function.return_type {
                walk_ts_type_mut(visitor, return_type);
            }
            visitor.visit_block(&mut function.body);
        }
        Stmt::Return(Some(expr), _)
        | Stmt::Expr(expr)
        | Stmt::ExportDefaultExpr(expr)
        | Stmt::Throw(ThrowStmt { argument: expr, .. }) => visitor.visit_expr(expr),
        Stmt::Block(block) => visitor.visit_block(block),
        Stmt::If(if_stmt) => {
            visitor.visit_expr(&mut if_stmt.condition);
            visitor.visit_stmt(&mut if_stmt.consequent);
            if let Some(alternate) = &mut if_stmt.alternate {
                visitor.visit_stmt(alternate);
            }
        }
        Stmt::Class(_) => {}
        Stmt::Try(try_stmt) => {
            visitor.visit_block(&mut try_stmt.body);
            visitor.visit_block(&mut try_stmt.catch_body);
            if let Some(finally_body) = &mut try_stmt.finally_body {
                visitor.visit_block(finally_body);
            }
        }
        Stmt::For(for_stmt) => {
            visitor.visit_expr(&mut for_stmt.iterable);
            visitor.visit_stmt(&mut for_stmt.body);
        }
        Stmt::ClassicFor(for_stmt) => {
            if let Some(init) = &mut for_stmt.init {
                match init {
                    ForInit::Var(var) => {
                        for declarator in var.declarators.iter_mut() {
                            if let Some(init) = &mut declarator.init {
                                visitor.visit_expr(init);
                            }
                        }
                    }
                    ForInit::Expr(expr) => visitor.visit_expr(expr),
                }
            }
            if let Some(condition) = &mut for_stmt.condition {
                visitor.visit_expr(condition);
            }
            if let Some(update) = &mut for_stmt.update {
                visitor.visit_expr(update);
            }
            visitor.visit_stmt(&mut for_stmt.body);
        }
        Stmt::While(while_stmt) => {
            visitor.visit_expr(&mut while_stmt.condition);
            visitor.visit_stmt(&mut while_stmt.body);
        }
        Stmt::DoWhile(do_while) => {
            visitor.visit_stmt(&mut do_while.body);
            visitor.visit_expr(&mut do_while.condition);
        }
        Stmt::Switch(switch) => {
            visitor.visit_expr(&mut switch.discriminant);
            for case in switch.cases.iter_mut() {
                if let Some(test) = &mut case.test {
                    visitor.visit_expr(test);
                }
                for stmt in case.consequent.iter_mut() {
                    visitor.visit_stmt(stmt);
                }
            }
        }
        Stmt::Label(label) => visitor.visit_stmt(&mut label.body),
        Stmt::With(with) => {
            visitor.visit_expr(&mut with.object);
            visitor.visit_stmt(&mut with.body);
        }
        Stmt::Export(ExportDecl::Declaration(stmt)) => visitor.visit_stmt(stmt),
        Stmt::TypeAlias(alias) => walk_ts_type_mut(visitor, &mut alias.type_ann),
        Stmt::Interface(iface) => {
            for element in iface.body.iter_mut() {
                walk_ts_type_element_mut(visitor, element);
            }
        }
        Stmt::Enum(enum_decl) => {
            for member in enum_decl.members.iter_mut() {
                if let Some(init) = &mut member.init {
                    visitor.visit_expr(init);
                }
            }
        }
        Stmt::Module(module) => visitor.visit_block(&mut module.body),
        Stmt::Import(_)
        | Stmt::Export(_)
        | Stmt::TypeOnly(_)
        | Stmt::Return(None, _)
        | Stmt::Debugger(_)
        | Stmt::Break(_)
        | Stmt::Continue(_)
        | Stmt::Raw(_) => {}
    }
}

fn walk_ts_type_mut<'a, V: VisitMut<'a>>(visitor: &mut V, ty: &mut TSType<'a>) {
    match ty {
        TSType::Reference {
            type_args: Some(args),
            ..
        } => {
            for arg in args.iter_mut() {
                walk_ts_type_mut(visitor, arg);
            }
        }
        TSType::Union(types, _) | TSType::Intersection(types, _) | TSType::Tuple(types, _) => {
            for ty in types.iter_mut() {
                walk_ts_type_mut(visitor, ty);
            }
        }
        TSType::Array(inner, _) | TSType::Parenthesized(inner, _) => {
            walk_ts_type_mut(visitor, inner)
        }
        TSType::Object(elements, _) => {
            for element in elements.iter_mut() {
                walk_ts_type_element_mut(visitor, element);
            }
        }
        TSType::Function {
            params,
            return_type,
            ..
        } => {
            for param in params.iter_mut() {
                if let Some(type_ann) = &mut param.type_ann {
                    walk_ts_type_mut(visitor, type_ann);
                }
            }
            walk_ts_type_mut(visitor, return_type);
        }
        TSType::Conditional {
            check,
            extends,
            true_type,
            false_type,
            ..
        } => {
            walk_ts_type_mut(visitor, check);
            walk_ts_type_mut(visitor, extends);
            walk_ts_type_mut(visitor, true_type);
            walk_ts_type_mut(visitor, false_type);
        }
        TSType::IndexedAccess { object, index, .. } => {
            walk_ts_type_mut(visitor, object);
            walk_ts_type_mut(visitor, index);
        }
        TSType::Operator { operand, .. } => walk_ts_type_mut(visitor, operand),
        TSType::Mapped {
            constraint,
            type_ann,
            ..
        } => {
            walk_ts_type_mut(visitor, constraint);
            walk_ts_type_mut(visitor, type_ann);
        }
        TSType::TemplateLiteral { types, .. } => {
            for ty in types.iter_mut() {
                walk_ts_type_mut(visitor, ty);
            }
        }
        TSType::Keyword(_, _)
        | TSType::Reference {
            type_args: None, ..
        }
        | TSType::Infer { .. }
        | TSType::Typeof { .. }
        | TSType::Literal(_)
        | TSType::This(_)
        | TSType::Any(_) => {}
    }
}

fn walk_ts_type_element_mut<'a, V: VisitMut<'a>>(visitor: &mut V, element: &mut TSTypeElement<'a>) {
    match element {
        TSTypeElement::Property { type_ann, .. } => {
            if let Some(type_ann) = type_ann {
                walk_ts_type_mut(visitor, type_ann);
            }
        }
        TSTypeElement::Method {
            params,
            return_type,
            ..
        }
        | TSTypeElement::Call {
            params,
            return_type,
            ..
        }
        | TSTypeElement::Construct {
            params,
            return_type,
            ..
        } => {
            for param in params.iter_mut() {
                if let Some(type_ann) = &mut param.type_ann {
                    walk_ts_type_mut(visitor, type_ann);
                }
            }
            if let Some(return_type) = return_type {
                walk_ts_type_mut(visitor, return_type);
            }
        }
        TSTypeElement::Index {
            key_type, type_ann, ..
        } => {
            walk_ts_type_mut(visitor, key_type);
            walk_ts_type_mut(visitor, type_ann);
        }
    }
}

pub fn walk_expr_mut<'a, V: VisitMut<'a>>(visitor: &mut V, expr: &mut Expr<'a>) {
    match &mut expr.kind {
        ExprKind::New { callee, args } | ExprKind::Call { callee, args, .. } => {
            visitor.visit_expr(callee);
            for arg in args.iter_mut() {
                visitor.visit_expr(arg);
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
        | ExprKind::TypeErasure { expr: inner, .. } => visitor.visit_expr(inner),
        ExprKind::Binary { left, right, .. } | ExprKind::Assign { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            visitor.visit_expr(test);
            visitor.visit_expr(consequent);
            visitor.visit_expr(alternate);
        }
        ExprKind::Member { object, .. } => visitor.visit_expr(object),
        ExprKind::Index { object, index, .. } => {
            visitor.visit_expr(object);
            visitor.visit_expr(index);
        }
        ExprKind::Arrow { body, .. } => match body {
            ArrowBody::Expression(expr) => visitor.visit_expr(expr),
            ArrowBody::Block(block) => visitor.visit_block(block),
        },
        ExprKind::TemplateLiteral { exprs, .. } | ExprKind::Sequence(exprs) => {
            for expr in exprs.iter_mut() {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::TaggedTemplate { tag, quasi } => {
            visitor.visit_expr(tag);
            visitor.visit_expr(quasi);
        }
        ExprKind::Array(items) => {
            for item in items.iter_mut() {
                visitor.visit_expr(item);
            }
        }
        ExprKind::Object(properties) => {
            for prop in properties.iter_mut() {
                if let PropertyKey::Computed(key) = &mut prop.key {
                    visitor.visit_expr(key);
                }
                visitor.visit_expr(&mut prop.value);
            }
        }
        ExprKind::Yield { value, .. } => {
            if let Some(value) = value {
                visitor.visit_expr(value);
            }
        }
        ExprKind::Markup(node) => visitor.visit_markup_node(node),
        ExprKind::Ident(_)
        | ExprKind::Number(_)
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

pub fn walk_markup_node_mut<'a, V: VisitMut<'a>>(visitor: &mut V, node: &mut MarkupNode<'a>) {
    match node {
        MarkupNode::Element(element) => {
            for attribute in element.attributes.iter_mut() {
                match attribute {
                    MarkupAttribute::Named {
                        value: Some(value), ..
                    }
                    | MarkupAttribute::Spread { expr: value, .. } => visitor.visit_expr(value),
                    MarkupAttribute::Named { value: None, .. } => {}
                }
            }
            for child in element.children.iter_mut() {
                walk_markup_child_mut(visitor, child);
            }
        }
        MarkupNode::Fragment { children, .. } => {
            for child in children.iter_mut() {
                walk_markup_child_mut(visitor, child);
            }
        }
    }
}

fn walk_markup_child_mut<'a, V: VisitMut<'a>>(visitor: &mut V, child: &mut MarkupChild<'a>) {
    match child {
        MarkupChild::Expr(expr) => visitor.visit_expr(expr),
        MarkupChild::Node(node) => visitor.visit_markup_node(node),
        MarkupChild::Text(_, _) => {}
    }
}
