pub use nori_span::{SourceMap, SourcePosition, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Import(ImportDecl),
    TypeOnly(RawStmt),
    Class(ClassDecl),
    Var(VarDecl),
    Function(FunctionDecl),
    Export(ExportDecl),
    ExportDefaultFunction(FunctionDecl),
    ExportDefaultExpr(Expr),
    Return(Option<Expr>, Span),
    Expr(Expr),
    Block(BlockStmt),
    If(IfStmt),
    Try(TryStmt),
    For(ForStmt),
    ClassicFor(Box<ClassicForStmt>),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Switch(SwitchStmt),
    Throw(ThrowStmt),
    Label(LabelStmt),
    Debugger(Span),
    With(WithStmt),
    Break(Span),
    Continue(Span),
    Raw(RawStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryStmt {
    pub body: BlockStmt,
    pub catch_param: Option<String>,
    pub catch_body: BlockStmt,
    pub finally_body: Option<BlockStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForStmt {
    pub variable: VarKind,
    pub name: String,
    pub iterable: Expr,
    pub is_of: bool,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicForStmt {
    pub init: Option<ForInit>,
    pub condition: Option<Expr>,
    pub update: Option<Expr>,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForInit {
    Var(VarDecl),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoWhileStmt {
    pub body: Box<Stmt>,
    pub condition: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchStmt {
    pub discriminant: Expr,
    pub cases: Vec<SwitchCase>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchCase {
    pub test: Option<Expr>,
    pub consequent: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrowStmt {
    pub argument: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelStmt {
    pub label: String,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithStmt {
    pub object: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub members: Vec<ClassMember>,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decorator {
    pub name: String,
    pub args: Option<Vec<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassMember {
    Field(ClassField),
    Constructor(ClassConstructor),
    Method(ClassMethod),
    Accessor(ClassAccessor),
    StaticBlock(ClassStaticBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassField {
    pub name: String,
    pub value: Option<Expr>,
    pub is_static: bool,
    pub is_private: bool,
    pub computed: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassConstructor {
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub is_static: bool,
    pub is_async: bool,
    pub is_get: bool,
    pub is_set: bool,
    pub is_private: bool,
    pub computed: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassAccessor {
    pub name: String,
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub is_static: bool,
    pub is_get: bool,
    pub is_private: bool,
    pub computed: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassStaticBlock {
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStmt {
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSpecifier {
    Default(String),
    Named {
        local: String,
        imported: Option<String>,
    },
    Namespace(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportDecl {
    Named {
        specifiers: Vec<ExportSpecifier>,
        source: Option<String>,
        span: Span,
    },
    All {
        source: String,
        as_namespace: Option<String>,
        span: Span,
    },
    Declaration(Box<Stmt>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSpecifier {
    pub local: String,
    pub exported: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStmt {
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStmt {
    pub condition: Expr,
    pub consequent: Box<Stmt>,
    pub alternate: Option<Box<Stmt>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDecl {
    pub kind: VarKind,
    pub declarators: Vec<VarDeclarator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDeclarator {
    pub name: String,
    pub pattern: Option<Pattern>,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Ident(String),
    Rest(Box<Pattern>),
    Array {
        elements: Vec<Option<Pattern>>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },
    Object {
        properties: Vec<ObjectPatternProp>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },
    Assign {
        left: Box<Pattern>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectPatternProp {
    pub key: String,
    pub alias: Option<String>,
    pub value: Option<Box<Pattern>>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub async_token: Option<Span>,
    pub generator: bool,
    pub decorators: Vec<Span>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub is_property: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Ident(String),
    Number(String),
    BigInt(String),
    String(String),
    RegExp {
        pattern: String,
        flags: String,
    },
    Bool(bool),
    Null,
    This,
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Delete(Box<Expr>),
    Void(Box<Expr>),
    Typeof(Box<Expr>),
    MetaProperty {
        meta: String,
        property: String,
    },
    Import(Box<Expr>),
    Sequence(Vec<Expr>),
    Yield {
        value: Option<Box<Expr>>,
        delegate: bool,
    },
    Unary {
        op: String,
        expr: Box<Expr>,
    },
    Update {
        op: String,
        expr: Box<Expr>,
        prefix: bool,
    },
    TypeErasure {
        kind: TypeErasureKind,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Assign {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Conditional {
        test: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        optional: bool,
    },
    Member {
        object: Box<Expr>,
        property: String,
        optional: bool,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
    },
    Arrow {
        params: Vec<String>,
        body: ArrowBody,
    },
    TemplateLiteral {
        quasis: Vec<String>,
        exprs: Vec<Expr>,
    },
    TaggedTemplate {
        tag: Box<Expr>,
        quasi: Box<Expr>,
    },
    Array(Vec<Expr>),
    Object(Vec<ObjectProperty>),
    Spread {
        expr: Box<Expr>,
    },
    Await(Box<Expr>),
    Markup(MarkupNode),
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowBody {
    Expression(Box<Expr>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectProperty {
    pub key: PropertyKey,
    pub value: Expr,
    pub kind: PropertyKind,
    pub computed: bool,
    pub shorthand: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyKey {
    Ident(String),
    String(String),
    Number(String),
    Computed(Box<Expr>),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupNode {
    Element(MarkupElement),
    Fragment {
        children: Vec<MarkupChild>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupElement {
    pub name: String,
    pub attributes: Vec<MarkupAttribute>,
    pub children: Vec<MarkupChild>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupAttribute {
    Named {
        name: String,
        value: Option<Expr>,
        span: Span,
    },
    Spread {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupChild {
    Text(String, Span),
    Expr(Expr),
    Node(MarkupNode),
}
