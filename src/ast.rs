use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Import(RawStmt),
    TypeOnly(RawStmt),
    Var(VarDecl),
    Function(FunctionDecl),
    ExportDefaultFunction(FunctionDecl),
    ExportDefaultExpr(Expr),
    Return(Option<Expr>, Span),
    Expr(Expr),
    Block(BlockStmt),
    If(IfStmt),
    Raw(RawStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStmt {
    pub span: Span,
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
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
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
    String(String),
    Bool(bool),
    Null,
    Unary {
        op: String,
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
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    Arrow {
        params: Vec<String>,
        body: Box<Expr>,
    },
    Array(Vec<Expr>),
    Object,
    Markup(MarkupNode),
    Raw,
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
