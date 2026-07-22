mod cursor;
mod syntax;

use cursor::TokenCursor;
use nori_allocator::{Allocator, Atom, Box, Vec as ArenaVec};
use nori_ast::{
    ArrowBody, BlockStmt, ClassAccessor, ClassConstructor, ClassDecl, ClassField, ClassMember,
    ClassMethod, ClassStaticBlock, ClassicForStmt, Decorator, DoWhileStmt, EnumDecl, EnumMember,
    ExportDecl, Expr, ExprKind, ForInit, ForStmt, FunctionDecl, IfStmt, InterfaceDecl,
    MarkupAttribute, MarkupChild, MarkupElement, MarkupNode, ModuleDecl, Param, Pattern, Program,
    RawStmt, Span, Stmt, TSFnParam, TSKeywordKind, TSLiteral, TSType, TSTypeElement,
    TSTypeOperator, TryStmt, TypeAliasDecl, TypeErasureKind, VarDecl, VarDeclarator, VarKind,
    WhileStmt,
};
use nori_diagnostic::{Diagnostic, NoriError, span as source_span};
use nori_lexer::{Keyword, Token, TokenKind};
pub use syntax::Syntax;

pub struct Parser<'a> {
    allocator: &'a Allocator,
    source: &'a str,
    filename: String,
    input: TokenCursor,
    syntax: Syntax,
    loop_depth: usize,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct ParseResult<'a> {
    pub program: Program<'a>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> ParseResult<'a> {
    pub fn into_result(self) -> Result<Program<'a>, NoriError> {
        if let Some(diag) = self.diagnostics.into_iter().find(|d| d.is_error()) {
            Err(diag.into())
        } else {
            Ok(self.program)
        }
    }
}

#[derive(Default)]
struct ClassMemberModifiers {
    is_static: bool,
    is_async: bool,
    is_get: bool,
    is_set: bool,
    declaration_only: bool,
}

/// Parse `source` into an arena-allocated [`Program`].
///
/// The returned program borrows from `allocator` (and, for identifiers, from
/// `source`), so both must outlive it.
pub fn parse_in<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    filename: impl Into<String>,
    tokens: Vec<Token>,
) -> Result<Program<'a>, NoriError> {
    Parser::new(allocator, source, filename.into(), tokens)
        .parse_program()
        .into_result()
}

/// Parse with error recovery. Always returns a (possibly partial) program plus diagnostics.
pub fn parse_in_recovering<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    filename: impl Into<String>,
    tokens: Vec<Token>,
) -> ParseResult<'a> {
    Parser::new(allocator, source, filename.into(), tokens).parse_program()
}

impl<'a> Parser<'a> {
    pub fn new(
        allocator: &'a Allocator,
        source: &'a str,
        filename: String,
        tokens: Vec<Token>,
    ) -> Self {
        Self::new_with_syntax(allocator, source, filename, tokens, Syntax::default())
    }

    pub fn new_with_syntax(
        allocator: &'a Allocator,
        source: &'a str,
        filename: String,
        tokens: Vec<Token>,
        syntax: Syntax,
    ) -> Self {
        Self {
            allocator,
            source,
            filename,
            input: TokenCursor::new(tokens),
            syntax,
            loop_depth: 0,
            diagnostics: Vec::new(),
        }
    }

    #[inline]
    fn box_in<T>(&self, value: T) -> Box<'a, T> {
        self.allocator.box_in(value)
    }

    #[inline]
    fn new_vec<T>(&self) -> ArenaVec<'a, T> {
        ArenaVec::new_in(self.allocator.as_bump())
    }

    #[inline]
    fn atom(&self, s: &str) -> Atom<'a> {
        self.allocator.atom(s)
    }

    #[inline]
    fn lexeme_of(&self, token: &Token) -> &'a str {
        token.lexeme(self.source)
    }

    #[inline]
    fn owned_lexeme(&self, token: &Token) -> Atom<'a> {
        Atom::new(self.lexeme_of(token))
    }

    #[inline]
    fn bump_lexeme(&mut self) -> Atom<'a> {
        let token = self.bump();
        self.owned_lexeme(&token)
    }

    #[inline]
    fn expect_lexeme(&mut self, kind: TokenKind, message: &str) -> Result<Atom<'a>, NoriError> {
        let token = self.expect(kind, message)?;
        Ok(self.owned_lexeme(&token))
    }

    #[inline]
    fn expect_markup_lexeme(&mut self, message: &str) -> Result<Atom<'a>, NoriError> {
        let token = self.expect_markup_ident(message)?;
        Ok(self.owned_lexeme(&token))
    }

    pub fn parse_program(&mut self) -> ParseResult<'a> {
        let mut body = self.new_vec();
        while !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Semicolon) {
                continue;
            }
            match self.parse_stmt() {
                Ok(stmt) => body.push(stmt),
                Err(err) => {
                    self.record_error(err);
                    self.synchronize_statement();
                    if self.at(TokenKind::Eof) {
                        break;
                    }
                }
            }
        }
        ParseResult {
            program: Program { body },
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    fn record_error(&mut self, err: NoriError) {
        match err {
            NoriError::Parse { message, span } => {
                self.diagnostics.push(Diagnostic::error(message, span));
            }
            NoriError::Lex { message, span } => {
                self.diagnostics.push(Diagnostic::lex_error(message, span));
            }
            NoriError::Io { .. } => {
                self.diagnostics
                    .push(Diagnostic::error(err.to_string(), source_span(0, 1)));
            }
        }
    }

    /// Skip tokens until a statement boundary (`;`, `}`, or a keyword that starts a statement).
    fn synchronize_statement(&mut self) {
        // Always consume at least one token so we cannot loop forever on the
        // same failing statement head (e.g. `const` after a bad declarator).
        if !self.at(TokenKind::Eof) {
            self.bump();
        }
        while !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Semicolon) {
                return;
            }
            if self.at(TokenKind::RightBrace) {
                return;
            }
            if self.at_any_keyword(&[
                Keyword::Import,
                Keyword::Export,
                Keyword::Function,
                Keyword::Class,
                Keyword::Const,
                Keyword::Let,
                Keyword::Var,
                Keyword::If,
                Keyword::For,
                Keyword::While,
                Keyword::Do,
                Keyword::Switch,
                Keyword::Try,
                Keyword::Return,
                Keyword::Throw,
                Keyword::Break,
                Keyword::Continue,
                Keyword::Debugger,
                Keyword::With,
                Keyword::Type,
                Keyword::Interface,
                Keyword::Enum,
            ]) {
                return;
            }
            self.bump();
        }
    }

    fn parse_stmt(&mut self) -> Result<Stmt<'a>, NoriError> {
        if self.at_keyword(Keyword::Import) {
            return self.parse_import();
        }
        if self.syntax.typescript
            && (self.at_keyword(Keyword::Type) || self.at_keyword(Keyword::Interface))
        {
            return self.parse_type_declaration();
        }
        if self.syntax.typescript
            && self.at_keyword(Keyword::Const)
            && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Enum))
        {
            return self.parse_enum(true);
        }
        if self.syntax.typescript && self.at_keyword(Keyword::Enum) {
            return self.parse_enum(false);
        }
        if self.syntax.typescript
            && (self.at_contextual_ident("module") || self.at_contextual_ident("namespace"))
        {
            return self.parse_module();
        }
        if self.syntax.typescript
            && (self.at_keyword(Keyword::Class)
                || self.at(TokenKind::At)
                || (self.at_contextual_ident("abstract")
                    && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Class))))
        {
            return self.parse_class().map(Stmt::Class);
        }
        if self.at_keyword(Keyword::Export) {
            return self.parse_export();
        }
        if self.at_keyword(Keyword::Function) {
            return self.parse_function().map(Stmt::Function);
        }
        if self.at_keyword(Keyword::Async)
            && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Function))
        {
            return self.parse_async_function();
        }
        if self.at_any_keyword(&[Keyword::Const, Keyword::Let, Keyword::Var]) {
            return self.parse_var().map(Stmt::Var);
        }
        if self.at_keyword(Keyword::Return) {
            return self.parse_return();
        }
        if self.at_keyword(Keyword::If) {
            return self.parse_if();
        }
        if self.at_keyword(Keyword::Try) {
            return self.parse_try();
        }
        if self.at_keyword(Keyword::For) {
            if self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Await)) {
                let for_span = self.bump().span;
                self.bump(); // await
                return self.parse_for_await(for_span);
            }
            return self.parse_for();
        }
        if self.at_keyword(Keyword::While) {
            return self.parse_while();
        }
        if self.at_keyword(Keyword::Do) {
            return self.parse_do_while();
        }
        if self.at_keyword(Keyword::Switch) {
            return self.parse_switch();
        }
        if self.at_keyword(Keyword::Throw) {
            return self.parse_throw();
        }
        if self.at_keyword(Keyword::Debugger) {
            let span = self.bump().span;
            self.consume_optional_semicolon();
            return Ok(Stmt::Debugger(span));
        }
        if self.at_keyword(Keyword::With) {
            return self.parse_with();
        }
        if self.at(TokenKind::Ident) && self.peek_next_kind() == Some(TokenKind::Colon) {
            return self.parse_label();
        }
        if self.at_keyword(Keyword::Break) {
            return self.parse_loop_control("`break`", Stmt::Break);
        }
        if self.at_keyword(Keyword::Continue) {
            return self.parse_loop_control("`continue`", Stmt::Continue);
        }
        if self.at(TokenKind::LeftBrace) {
            return self.parse_block().map(Stmt::Block);
        }
        self.parse_expr_stmt()
    }

    fn parse_export(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        // `export type ...` — erase.
        if self.syntax.typescript
            && (self.at_keyword(Keyword::Type)
                || (self.at_contextual_ident("type")
                    && matches!(
                        self.peek_next_kind(),
                        Some(TokenKind::Ident)
                            | Some(TokenKind::LeftBrace)
                            | Some(TokenKind::Keyword(_))
                    )))
        {
            if self.at_keyword(Keyword::Type) || self.at_contextual_ident("type") {
                self.bump();
            }
            if self.at(TokenKind::LeftBrace) {
                // `export type { A, B }` / `export type { A } from "..."`
                self.bump();
                while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                    if self.at(TokenKind::Ident)
                        || matches!(self.peek().kind, TokenKind::Keyword(_))
                    {
                        self.bump();
                    }
                    if self.matches_contextual_ident("as")
                        && (self.at(TokenKind::Ident)
                            || matches!(self.peek().kind, TokenKind::Keyword(_)))
                    {
                        self.bump();
                    }
                    if !self.matches(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(
                    TokenKind::RightBrace,
                    "expected `}` after export type specifiers",
                )?;
                if self.matches_keyword(Keyword::From) {
                    self.expect(TokenKind::String, "expected export source")?;
                }
                self.consume_optional_semicolon();
                return Ok(Stmt::Export(ExportDecl::TypeOnly(join_span(
                    start,
                    self.previous().span,
                ))));
            }
            // `export type Foo = ...` / `export type Foo<T> = ...`
            if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
                self.bump();
            }
            if self.at(TokenKind::Less) {
                self.skip_balanced_angle_list()?;
            }
            if self.matches(TokenKind::Eq) {
                self.skip_type_until(&[TokenKind::Semicolon]);
            }
            self.consume_optional_semicolon();
            return Ok(Stmt::Export(ExportDecl::TypeOnly(join_span(
                start,
                self.previous().span,
            ))));
        }
        if self.matches_keyword(Keyword::Default) {
            if self.at_keyword(Keyword::Function) {
                return self
                    .parse_function_with_start(start)
                    .map(Stmt::ExportDefaultFunction);
            }
            if self.at_keyword(Keyword::Class)
                || self.at(TokenKind::At)
                || (self.at_contextual_ident("abstract")
                    && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Class)))
            {
                let class = self.parse_class()?;
                return Ok(Stmt::ExportDefaultExpr(Expr {
                    kind: ExprKind::Raw,
                    span: class.span,
                }));
            }
            if self.at_keyword(Keyword::Async)
                && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Function))
            {
                self.bump();
                self.expect_keyword(Keyword::Function, "expected `function`")?;
                let generator = self.matches(TokenKind::Star);
                let async_token = Some(start);
                let name = if self.at(TokenKind::Ident) {
                    Some(self.bump_lexeme())
                } else {
                    None
                };
                if self.at(TokenKind::Less) {
                    self.skip_balanced_angle_list()?;
                }
                self.expect(
                    TokenKind::LeftParen,
                    "expected `(` before function parameters",
                )?;
                let params = self.parse_params()?;
                self.expect(
                    TokenKind::RightParen,
                    "expected `)` after function parameters",
                )?;
                let return_type = if self.matches(TokenKind::Colon) {
                    let ty = self.parse_ts_type()?;
                    Some(self.box_in(ty))
                } else {
                    None
                };
                let body = self.parse_function_body()?;
                let span = Span::new(start.start, body.span.end);
                return Ok(Stmt::ExportDefaultFunction(FunctionDecl {
                    name,
                    params,
                    return_type,
                    body,
                    async_token,
                    generator,
                    decorators: self.new_vec(),
                    span,
                }));
            }
            let expr = self.parse_expression_until_statement_end()?;
            self.consume_optional_semicolon();
            return Ok(Stmt::ExportDefaultExpr(expr));
        }
        Ok(Stmt::Raw(self.parse_raw_until_semicolon()))
    }

    fn parse_import(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        let is_type = if self.syntax.typescript {
            if self.matches_keyword(Keyword::Type) {
                true
            } else if self.at_contextual_ident("type")
                && matches!(
                    self.peek_next_kind(),
                    Some(TokenKind::Ident)
                        | Some(TokenKind::LeftBrace)
                        | Some(TokenKind::Star)
                        | Some(TokenKind::String)
                )
            {
                self.bump();
                true
            } else {
                false
            }
        } else {
            false
        };
        let mut specifiers = self.new_vec();
        if self.at(TokenKind::String) {
            let source = self.bump_lexeme();
            self.consume_optional_semicolon();
            return Ok(Stmt::Import(nori_ast::ImportDecl {
                specifiers,
                source,
                is_type,
                span: join_span(start, self.previous().span),
            }));
        }
        if self.matches(TokenKind::Star) {
            self.expect_contextual("as", "expected `as` after `*`")?;
            let name = self.expect_lexeme(TokenKind::Ident, "expected namespace import name")?;
            specifiers.push(nori_ast::ImportSpecifier::Namespace(name));
        } else if self.at(TokenKind::LeftBrace) {
            self.bump();
            while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                let name = self.expect_lexeme(TokenKind::Ident, "expected import name")?;
                let imported = if self.matches_contextual_ident("as") {
                    Some(self.expect_lexeme(TokenKind::Ident, "expected import alias")?)
                } else {
                    None
                };
                specifiers.push(nori_ast::ImportSpecifier::Named {
                    local: name,
                    imported,
                });
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(
                TokenKind::RightBrace,
                "expected `}` after import specifiers",
            )?;
        } else {
            let name = self.expect_lexeme(TokenKind::Ident, "expected default import name")?;
            specifiers.push(nori_ast::ImportSpecifier::Default(name));
            if self.matches(TokenKind::Comma) {
                if self.matches(TokenKind::Star) {
                    self.expect_contextual("as", "expected `as` after `*`")?;
                    let name =
                        self.expect_lexeme(TokenKind::Ident, "expected namespace import name")?;
                    specifiers.push(nori_ast::ImportSpecifier::Namespace(name));
                } else if self.at(TokenKind::LeftBrace) {
                    self.bump();
                    while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                        let name = self.expect_lexeme(TokenKind::Ident, "expected named import")?;
                        let imported = if self.matches_contextual_ident("as") {
                            Some(self.expect_lexeme(TokenKind::Ident, "expected import alias")?)
                        } else {
                            None
                        };
                        specifiers.push(nori_ast::ImportSpecifier::Named {
                            local: name,
                            imported,
                        });
                        if !self.matches(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(
                        TokenKind::RightBrace,
                        "expected `}` after import specifiers",
                    )?;
                }
            }
        }
        self.expect_keyword(Keyword::From, "expected `from` in import declaration")?;
        let source = self.expect_lexeme(TokenKind::String, "expected import source")?;
        self.consume_optional_semicolon();
        let span = join_span(start, self.previous().span);
        Ok(Stmt::Import(nori_ast::ImportDecl {
            specifiers,
            source,
            is_type,
            span,
        }))
    }

    fn parse_class(&mut self) -> Result<ClassDecl<'a>, NoriError> {
        self.matches_contextual_ident("abstract");
        let class_start = self.peek().span;
        let mut decorators = self.new_vec();
        while self.at(TokenKind::At) {
            decorators.push(self.parse_decorator()?);
        }
        self.expect_keyword(Keyword::Class, "expected `class` keyword")?;
        let start = class_start;
        self.expect(TokenKind::Ident, "expected class name")?;
        let name = self.owned_lexeme(self.previous());
        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }

        let extends = if self.at_keyword(Keyword::Extends) {
            self.bump();
            self.expect(TokenKind::Ident, "expected parent class name")?;
            let extends = self.owned_lexeme(self.previous());
            if self.at(TokenKind::Less) {
                self.skip_balanced_angle_list()?;
            }
            Some(extends)
        } else {
            None
        };
        if self.matches_contextual_ident("implements") {
            self.skip_type_until(&[TokenKind::LeftBrace]);
        }

        self.expect(TokenKind::LeftBrace, "expected class body")?;
        let mut members = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Semicolon) {
                continue;
            }
            if let Some(member) = self.parse_class_member()? {
                members.push(member);
            }
        }
        let body_end = self
            .expect(TokenKind::RightBrace, "expected `}` after class body")?
            .span;
        let span = Span::new(start.start, body_end.end);
        Ok(ClassDecl {
            name,
            extends,
            members,
            decorators,
            span,
        })
    }

    fn parse_class_member(&mut self) -> Result<Option<ClassMember<'a>>, NoriError> {
        while self.at(TokenKind::At) {
            self.parse_decorator()?;
        }
        let start = self.peek().span;
        let modifiers = self.parse_class_member_modifiers();

        if modifiers.is_static
            && self.at(TokenKind::LeftBrace)
            && !modifiers.is_async
            && !modifiers.is_get
            && !modifiers.is_set
        {
            let body = self.parse_block()?;
            let span = join_span(start, body.span);
            return Ok(Some(ClassMember::StaticBlock(ClassStaticBlock {
                body,
                span,
            })));
        }

        let (name, computed, is_private) = self.parse_class_member_name()?;

        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }

        if self.matches(TokenKind::LeftParen) {
            let params = self.parse_class_params(name == "constructor")?;
            self.expect(TokenKind::RightParen, "expected `)` after class parameters")?;
            if self.matches(TokenKind::Colon) {
                self.skip_type_until(&[TokenKind::LeftBrace, TokenKind::Semicolon]);
            }
            if !self.at(TokenKind::LeftBrace) {
                self.consume_optional_semicolon();
                return Ok(None);
            }
            if modifiers.declaration_only {
                self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace)?;
                return Ok(None);
            }

            let body = self.parse_function_body()?;
            let span = join_span(start, body.span);

            if name == "constructor" {
                return Ok(Some(ClassMember::Constructor(ClassConstructor {
                    params,
                    body,
                    span,
                })));
            }

            if modifiers.is_get || modifiers.is_set {
                return Ok(Some(ClassMember::Accessor(ClassAccessor {
                    name,
                    params,
                    body,
                    is_static: modifiers.is_static,
                    is_get: modifiers.is_get,
                    is_private,
                    computed,
                    span,
                })));
            }

            return Ok(Some(ClassMember::Method(ClassMethod {
                name,
                params,
                body,
                is_static: modifiers.is_static,
                is_async: modifiers.is_async,
                is_get: false,
                is_set: false,
                is_private,
                computed,
                span,
            })));
        }

        let optional = self.matches(TokenKind::Question);
        let definite = self.matches(TokenKind::Bang);
        let typed = if self.matches(TokenKind::Colon) {
            self.skip_type_until(&[TokenKind::Eq, TokenKind::Semicolon, TokenKind::RightBrace]);
            true
        } else {
            false
        };
        let value = if self.matches(TokenKind::Eq) {
            Some(self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace])?)
        } else {
            None
        };
        self.consume_optional_semicolon();

        if modifiers.declaration_only || (value.is_none() && (typed || optional || definite)) {
            return Ok(None);
        }
        let end = value
            .as_ref()
            .map_or(self.previous().span, |expr| expr.span);
        Ok(Some(ClassMember::Field(ClassField {
            name,
            value,
            is_static: modifiers.is_static,
            is_private,
            computed,
            span: join_span(start, end),
        })))
    }

    fn parse_class_member_modifiers(&mut self) -> ClassMemberModifiers {
        let mut modifiers = ClassMemberModifiers::default();
        loop {
            if self.matches_contextual_ident("static") {
                modifiers.is_static = true;
            } else if self.matches_keyword(Keyword::Async) {
                modifiers.is_async = true;
            } else if self.matches_contextual_ident("get") {
                modifiers.is_get = true;
            } else if self.matches_contextual_ident("set") {
                modifiers.is_set = true;
            } else if self.at_any_contextual_ident(&["abstract", "declare"]) {
                self.bump();
                modifiers.declaration_only = true;
            } else if self.at_any_contextual_ident(&[
                "public",
                "private",
                "protected",
                "readonly",
                "override",
            ]) {
                self.bump();
            } else {
                break;
            }
        }
        modifiers
    }

    fn parse_decorator(&mut self) -> Result<Decorator<'a>, NoriError> {
        let start = self.bump().span;
        let name = self.expect_lexeme(TokenKind::Ident, "expected decorator name")?;
        let args = if self.at(TokenKind::LeftParen) {
            self.bump();
            let mut args = self.new_vec();
            while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
                args.push(self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?);
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(
                TokenKind::RightParen,
                "expected `)` after decorator arguments",
            )?;
            Some(args)
        } else {
            None
        };
        let span = Span::new(start.start, self.previous().span.end);
        Ok(Decorator { name, args, span })
    }

    fn parse_class_member_name(
        &mut self,
    ) -> Result<(Atom<'a>, Option<Box<'a, Expr<'a>>>, bool), NoriError> {
        let is_private = self.matches(TokenKind::Hash);

        if self.at(TokenKind::LeftBracket) {
            self.bump();
            let expr = self.parse_expression_until(&[TokenKind::RightBracket])?;
            self.expect(TokenKind::RightBracket, "expected `]` after computed name")?;
            Ok((self.atom(""), Some(self.box_in(expr)), is_private))
        } else {
            let name = self.expect_lexeme(TokenKind::Ident, "expected class member name")?;
            Ok((name, None, is_private))
        }
    }

    fn parse_function(&mut self) -> Result<FunctionDecl<'a>, NoriError> {
        let start = self.peek().span;
        self.parse_function_with_start(start)
    }

    fn parse_async_function(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        let async_token = Some(start);
        self.expect_keyword(Keyword::Function, "expected `function`")?;
        let generator = self.matches(TokenKind::Star);
        let name = if self.at(TokenKind::Ident) {
            Some(self.bump_lexeme())
        } else {
            None
        };
        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }
        self.expect(
            TokenKind::LeftParen,
            "expected `(` before function parameters",
        )?;
        let params = self.parse_params()?;
        self.expect(
            TokenKind::RightParen,
            "expected `)` after function parameters",
        )?;
        let return_type = if self.matches(TokenKind::Colon) {
            let ty = self.parse_ts_type()?;
            Some(self.box_in(ty))
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = Span::new(start.start, body.span.end);
        Ok(Stmt::Function(FunctionDecl {
            name,
            params,
            return_type,
            body,
            async_token,
            generator,
            decorators: self.new_vec(),
            span,
        }))
    }

    fn parse_try(&mut self) -> Result<Stmt<'a>, NoriError> {
        let try_start = self.bump().span;
        let body = self.parse_block()?;
        let mut catch_param = None;
        let mut catch_body = BlockStmt {
            body: self.new_vec(),
            span: try_start,
        };
        if self.at_keyword(Keyword::Catch) {
            self.bump();
            if self.at(TokenKind::LeftParen) {
                self.bump();
                if self.at(TokenKind::Ident) {
                    catch_param = Some(self.bump_lexeme());
                }
                self.expect(TokenKind::RightParen, "expected `)` after catch param")?;
            }
            catch_body = self.parse_block()?;
        }
        let finally_body = if self.at_keyword(Keyword::Finally) {
            self.bump();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::Try(TryStmt {
            body,
            catch_param,
            catch_body,
            finally_body,
            span: Span::new(try_start.start, self.previous().span.end),
        }))
    }

    fn parse_for(&mut self) -> Result<Stmt<'a>, NoriError> {
        let for_start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `for`")?;

        let init = if self.matches(TokenKind::Semicolon) {
            None
        } else if self.at_any_keyword(&[Keyword::Const, Keyword::Let, Keyword::Var]) {
            let var = self.parse_var()?;
            if self.at_keyword(Keyword::Of) || self.at_keyword(Keyword::In) {
                return self.finish_for_each(for_start, var);
            }
            Some(ForInit::Var(var))
        } else {
            let expr = self.parse_expression_until(&[TokenKind::Semicolon])?;
            self.expect(TokenKind::Semicolon, "expected `;` after for initializer")?;
            Some(ForInit::Expr(expr))
        };

        let condition = if self.matches(TokenKind::Semicolon) {
            None
        } else {
            let expr = self.parse_expression_until(&[TokenKind::Semicolon])?;
            self.expect(TokenKind::Semicolon, "expected `;` after for condition")?;
            Some(expr)
        };
        let update = if self.at(TokenKind::RightParen) {
            None
        } else {
            Some(self.parse_expression_until(&[TokenKind::RightParen])?)
        };
        self.expect(TokenKind::RightParen, "expected `)` after for clauses")?;
        let body = {
            let body = self.parse_loop_body()?;
            self.box_in(body)
        };
        let end = stmt_span(&body);

        Ok(Stmt::ClassicFor(self.box_in(ClassicForStmt {
            init,
            condition,
            update,
            body,
            span: join_span(for_start, end),
        })))
    }

    fn finish_for_each(&mut self, start: Span, var: VarDecl<'a>) -> Result<Stmt<'a>, NoriError> {
        let declarator = var
            .declarators
            .first()
            .filter(|_| var.declarators.len() == 1)
            .ok_or_else(|| self.error_here("expected one binding in for loop"))?;
        if declarator.pattern.is_some() || declarator.init.is_some() {
            return Err(self.error_here("expected a simple binding in for loop"));
        }
        let is_of = if self.matches_keyword(Keyword::Of) {
            true
        } else {
            self.expect_keyword(Keyword::In, "expected `in` or `of` in for loop")?;
            false
        };
        let iterable = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after for loop")?;
        let body = {
            let body = self.parse_loop_body()?;
            self.box_in(body)
        };
        let end = stmt_span(&body);

        Ok(Stmt::For(ForStmt {
            variable: var.kind,
            name: declarator.name,
            iterable,
            is_of,
            body,
            span: join_span(start, end),
        }))
    }

    fn parse_for_await(&mut self, for_span: Span) -> Result<Stmt<'a>, NoriError> {
        self.expect(TokenKind::LeftParen, "expected `(` after `for await`")?;
        let var = self.parse_var()?;
        let declarator = var
            .declarators
            .first()
            .filter(|_| var.declarators.len() == 1)
            .ok_or_else(|| self.error_here("expected one binding in for-await loop"))?;
        if declarator.pattern.is_some() || declarator.init.is_some() {
            return Err(self.error_here("expected a simple binding in for-await loop"));
        }
        self.expect_keyword(Keyword::Of, "expected `of` in for-await loop")?;
        let iterable = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after for-await loop")?;
        let body = {
            let body = self.parse_loop_body()?;
            self.box_in(body)
        };
        let end = stmt_span(&body);
        Ok(Stmt::For(ForStmt {
            variable: var.kind,
            name: declarator.name,
            iterable,
            is_of: true,
            body,
            span: join_span(for_span, end),
        }))
    }

    fn parse_while(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `while`")?;
        let condition = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after while condition")?;
        let body = {
            let body = self.parse_loop_body()?;
            self.box_in(body)
        };
        let end = stmt_span(&body);
        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: join_span(start, end),
        }))
    }

    fn parse_do_while(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        let body = {
            let body = self.parse_loop_body()?;
            self.box_in(body)
        };
        self.expect_keyword(Keyword::While, "expected `while` after do body")?;
        self.expect(TokenKind::LeftParen, "expected `(` after `while`")?;
        let condition = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(
            TokenKind::RightParen,
            "expected `)` after do-while condition",
        )?;
        self.consume_optional_semicolon();
        Ok(Stmt::DoWhile(DoWhileStmt {
            body,
            condition,
            span: join_span(start, self.previous().span),
        }))
    }

    fn parse_loop_body(&mut self) -> Result<Stmt<'a>, NoriError> {
        self.loop_depth += 1;
        let body = self.parse_stmt();
        self.loop_depth = self.loop_depth.saturating_sub(1);
        body
    }

    fn parse_loop_control(
        &mut self,
        name: &str,
        constructor: fn(Span) -> Stmt<'a>,
    ) -> Result<Stmt<'a>, NoriError> {
        if self.loop_depth == 0 {
            return Err(self.error_here(&format!("{name} is only valid inside a loop")));
        }
        let span = self.bump().span;
        self.consume_optional_semicolon();
        Ok(constructor(span))
    }

    fn parse_switch(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `switch`")?;
        let discriminant = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after switch condition")?;
        self.expect(TokenKind::LeftBrace, "expected `{` after switch")?;
        let mut cases = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let case_start = self.peek().span;
            let test = if self.matches_keyword(Keyword::Default) {
                None
            } else {
                self.expect_keyword(Keyword::Case, "expected `case` or `default`")?;
                Some(self.parse_expression_until(&[TokenKind::Colon])?)
            };
            self.expect(TokenKind::Colon, "expected `:` after case clause")?;
            let mut consequent = self.new_vec();
            while !self.at(TokenKind::RightBrace)
                && !self.at(TokenKind::Eof)
                && !self.at_keyword(Keyword::Case)
                && !self.at_keyword(Keyword::Default)
            {
                consequent.push(self.parse_stmt()?);
            }
            let span = join_span(case_start, self.previous().span);
            cases.push(nori_ast::SwitchCase {
                test,
                consequent,
                span,
            });
        }
        let end = self
            .expect(TokenKind::RightBrace, "expected `}` after switch body")?
            .span;
        Ok(Stmt::Switch(nori_ast::SwitchStmt {
            discriminant,
            cases,
            span: join_span(start, end),
        }))
    }

    fn parse_throw(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        let argument = self.parse_expression_until_statement_end()?;
        self.consume_optional_semicolon();
        Ok(Stmt::Throw(nori_ast::ThrowStmt {
            argument,
            span: join_span(start, self.previous().span),
        }))
    }

    fn parse_with(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `with`")?;
        let object = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after with expression")?;
        let body = {
            let body = self.parse_stmt()?;
            self.box_in(body)
        };
        let end = stmt_span(&body);
        Ok(Stmt::With(nori_ast::WithStmt {
            object,
            body,
            span: join_span(start, end),
        }))
    }

    fn parse_label(&mut self) -> Result<Stmt<'a>, NoriError> {
        let label_token = self.bump();
        self.bump(); // colon
        let body = {
            let body = self.parse_stmt()?;
            self.box_in(body)
        };
        let span = join_span(label_token.span, stmt_span(&body));
        Ok(Stmt::Label(nori_ast::LabelStmt {
            label: self.owned_lexeme(&label_token),
            body,
            span,
        }))
    }

    fn parse_function_with_start(&mut self, start: Span) -> Result<FunctionDecl<'a>, NoriError> {
        let async_token = if self.at_keyword(Keyword::Async) {
            let tok = self.bump();
            Some(tok.span)
        } else {
            None
        };
        self.expect_keyword(Keyword::Function, "expected `function`")?;
        let generator = self.matches(TokenKind::Star);
        let name = if self.at(TokenKind::Ident) {
            Some(self.bump_lexeme())
        } else {
            None
        };
        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }
        self.expect(
            TokenKind::LeftParen,
            "expected `(` before function parameters",
        )?;
        let params = self.parse_params()?;
        self.expect(
            TokenKind::RightParen,
            "expected `)` after function parameters",
        )?;
        let return_type = if self.matches(TokenKind::Colon) {
            let ty = self.parse_ts_type()?;
            Some(self.box_in(ty))
        } else {
            None
        };
        let body = self.parse_function_body()?;
        let span = Span::new(start.start, body.span.end);
        Ok(FunctionDecl {
            name,
            params,
            return_type,
            body,
            async_token,
            generator,
            decorators: self.new_vec(),
            span,
        })
    }

    fn parse_params(&mut self) -> Result<ArenaVec<'a, Param<'a>>, NoriError> {
        self.parse_params_with_properties(false)
    }

    fn parse_class_params(
        &mut self,
        parameter_properties: bool,
    ) -> Result<ArenaVec<'a, Param<'a>>, NoriError> {
        self.parse_params_with_properties(parameter_properties)
    }

    fn parse_params_with_properties(
        &mut self,
        parameter_properties: bool,
    ) -> Result<ArenaVec<'a, Param<'a>>, NoriError> {
        let mut params = self.new_vec();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.matches(TokenKind::Ellipsis);
            let is_property =
                parameter_properties && self.consume_constructor_parameter_property_modifiers();
            let name = if self.at(TokenKind::Ident) {
                self.bump_lexeme()
            } else {
                return Err(self.error_here("expected parameter name"));
            };
            self.matches(TokenKind::Question);
            let type_ann = if self.matches(TokenKind::Colon) {
                {
                    let ty = self.parse_ts_type()?;
                    Some(self.box_in(ty))
                }
            } else {
                None
            };
            let default = if self.matches(TokenKind::Eq) {
                Some(self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?)
            } else {
                None
            };
            params.push(Param {
                name,
                type_ann,
                default,
                is_property,
            });
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn consume_constructor_parameter_property_modifiers(&mut self) -> bool {
        let mut is_property = false;
        while self.at_any_contextual_ident(&["public", "private", "protected", "readonly"]) {
            self.bump();
            is_property = true;
        }
        is_property
    }

    fn parse_block(&mut self) -> Result<BlockStmt<'a>, NoriError> {
        let start = self.expect(TokenKind::LeftBrace, "expected block")?.span;
        let mut body = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            body.push(self.parse_stmt()?);
        }
        let end = self
            .expect(TokenKind::RightBrace, "expected `}` after block")?
            .span;
        Ok(BlockStmt {
            body,
            span: join_span(start, end),
        })
    }

    fn parse_function_body(&mut self) -> Result<BlockStmt<'a>, NoriError> {
        let loop_depth = self.loop_depth;
        self.loop_depth = 0;
        let body = self.parse_block();
        self.loop_depth = loop_depth;
        body
    }

    fn parse_var(&mut self) -> Result<VarDecl<'a>, NoriError> {
        let kind_token = self.bump();
        let kind = match kind_token.kind {
            TokenKind::Keyword(Keyword::Const) => VarKind::Const,
            TokenKind::Keyword(Keyword::Let) => VarKind::Let,
            TokenKind::Keyword(Keyword::Var) => VarKind::Var,
            _ => unreachable!("caller checked keyword"),
        };
        let mut declarators = self.new_vec();
        loop {
            let start = self.peek().span;
            let (name, pattern) =
                if self.at(TokenKind::LeftBracket) || self.at(TokenKind::LeftBrace) {
                    let pattern = self.parse_destructuring_pattern()?;
                    (self.atom(""), Some(pattern))
                } else {
                    let name_token = self.expect(TokenKind::Ident, "expected variable name")?;
                    (self.owned_lexeme(&name_token), None)
                };
            if name.is_empty() && self.peek().kind != TokenKind::Eq {
                return Err(self.error_here("expected variable name or pattern"));
            }
            let type_ann = if self.matches(TokenKind::Colon) {
                {
                    let ty = self.parse_ts_type()?;
                    Some(self.box_in(ty))
                }
            } else {
                None
            };
            let init = if self.matches(TokenKind::Eq) {
                Some(self.parse_expression_until(&[TokenKind::Comma, TokenKind::Semicolon])?)
            } else {
                None
            };
            let end = init.as_ref().map_or(start, |expr| expr.span);
            declarators.push(VarDeclarator {
                name,
                pattern,
                type_ann,
                init,
                span: join_span(start, end),
            });
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        let end = if self.matches(TokenKind::Semicolon) {
            self.previous().span
        } else {
            declarators
                .last()
                .map_or(kind_token.span, |declarator| declarator.span)
        };
        Ok(VarDecl {
            kind,
            declarators,
            span: join_span(kind_token.span, end),
        })
    }

    fn parse_destructuring_pattern(&mut self) -> Result<Pattern<'a>, NoriError> {
        let start = self.peek().span;
        if self.at(TokenKind::LeftBracket) {
            self.bump();
            let mut elements = self.new_vec();
            while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::Ident) {
                    elements.push(Some(Pattern::Ident(self.bump_lexeme())));
                } else if self.at(TokenKind::Comma) {
                    self.bump();
                    elements.push(None);
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RightBracket, "expected `]` after array pattern")?;
            let end = self.previous().span;
            Ok(Pattern::Array {
                elements,
                rest: None,
                span: join_span(start, end),
            })
        } else if self.at(TokenKind::LeftBrace) {
            self.bump();
            let mut properties = self.new_vec();
            while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::Ident) {
                    let name = self.bump_lexeme();
                    let default =
                        if self.matches(TokenKind::Eq) {
                            Some(self.parse_expression_until(&[
                                TokenKind::Comma,
                                TokenKind::RightBrace,
                            ])?)
                        } else {
                            None
                        };
                    let span = Span::new(start.start, self.previous().span.end);
                    properties.push(nori_ast::ObjectPatternProp {
                        key: name,
                        alias: None,
                        value: Some(self.box_in(Pattern::Ident(name))),
                        default,
                        span,
                    });
                } else if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RightBrace, "expected `}` after object pattern")?;
            let end = self.previous().span;
            Ok(Pattern::Object {
                properties,
                rest: None,
                span: join_span(start, end),
            })
        } else {
            Err(self.error_here("expected `[` or `{` for destructuring pattern"))
        }
    }

    fn parse_return(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        if self.matches(TokenKind::Semicolon) {
            return Ok(Stmt::Return(None, start));
        }
        if self.at(TokenKind::RightBrace) {
            return Ok(Stmt::Return(None, start));
        }
        let expr = self.parse_expression_until_statement_end()?;
        self.consume_optional_semicolon();
        Ok(Stmt::Return(Some(expr), start))
    }

    fn parse_if(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `if`")?;
        let condition = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after if condition")?;
        let consequent = {
            let stmt = self.parse_stmt()?;
            self.box_in(stmt)
        };
        let alternate = if self.matches_keyword(Keyword::Else) {
            {
                let stmt = self.parse_stmt()?;
                Some(self.box_in(stmt))
            }
        } else {
            None
        };
        let end = alternate
            .as_ref()
            .map_or(stmt_span(&consequent).end, |stmt| stmt_span(stmt).end);
        Ok(Stmt::If(IfStmt {
            condition,
            consequent,
            alternate,
            span: Span::new(start.start, end),
        }))
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt<'a>, NoriError> {
        let expr = self.parse_expression_until_statement_end()?;
        self.consume_optional_semicolon();
        Ok(Stmt::Expr(expr))
    }

    fn parse_type_declaration(&mut self) -> Result<Stmt<'a>, NoriError> {
        if self.at_keyword(Keyword::Interface) {
            return self.parse_interface();
        }
        let checkpoint = self.input.checkpoint();
        match self.try_parse_type_alias() {
            Ok(stmt) => Ok(stmt),
            Err(_) => {
                self.input.rewind(checkpoint);
                self.parse_type_only().map(Stmt::TypeOnly)
            }
        }
    }

    fn try_parse_type_alias(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.expect_keyword(Keyword::Type, "expected `type`")?.span;
        let name = self.expect_lexeme(TokenKind::Ident, "expected type alias name")?;
        let mut type_params = self.new_vec();
        if self.at(TokenKind::Less) {
            self.bump();
            while !self.at(TokenKind::Greater) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
                    type_params.push(self.bump_lexeme());
                } else {
                    break;
                }
                // Skip optional constraints / defaults on type params.
                if self.matches_keyword(Keyword::Extends) {
                    let _ = self.parse_ts_type()?;
                }
                if self.matches(TokenKind::Eq) {
                    let _ = self.parse_ts_type()?;
                }
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "expected `>` after type parameters")?;
        }
        self.expect(TokenKind::Eq, "expected `=` in type alias")?;
        let type_ann = {
            let ty = self.parse_ts_type()?;
            self.box_in(ty)
        };
        self.consume_optional_semicolon();
        let end = self.previous().span;
        Ok(Stmt::TypeAlias(TypeAliasDecl {
            name,
            type_params,
            type_ann,
            span: join_span(start, end),
        }))
    }

    fn parse_interface(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self
            .expect_keyword(Keyword::Interface, "expected `interface`")?
            .span;
        let name = self.expect_lexeme(TokenKind::Ident, "expected interface name")?;
        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }
        let mut extends = self.new_vec();
        if self.matches_keyword(Keyword::Extends) {
            loop {
                let parent = self.expect_lexeme(TokenKind::Ident, "expected interface parent")?;
                extends.push(parent);
                if self.at(TokenKind::Less) {
                    self.skip_balanced_angle_list()?;
                }
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
        }
        let body = self.parse_ts_object_members()?;
        let end = self.previous().span;
        Ok(Stmt::Interface(InterfaceDecl {
            name,
            extends,
            body,
            span: join_span(start, end),
        }))
    }

    fn parse_enum(&mut self, is_const: bool) -> Result<Stmt<'a>, NoriError> {
        let start = if is_const {
            let const_span = self
                .expect_keyword(Keyword::Const, "expected `const`")?
                .span;
            self.expect_keyword(Keyword::Enum, "expected `enum`")?;
            const_span
        } else {
            self.expect_keyword(Keyword::Enum, "expected `enum`")?.span
        };
        let name = self.expect_lexeme(TokenKind::Ident, "expected enum name")?;
        self.expect(TokenKind::LeftBrace, "expected `{` after enum name")?;
        let mut members = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let member_start = self.peek().span;
            let member_name = if self.at(TokenKind::Ident)
                || matches!(self.peek().kind, TokenKind::Keyword(_))
                || self.at(TokenKind::String)
            {
                self.bump_lexeme()
            } else {
                return Err(self.error_here("expected enum member name"));
            };
            let init = if self.matches(TokenKind::Eq) {
                Some(self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBrace])?)
            } else {
                None
            };
            let end = init.as_ref().map_or(member_start, |expr| expr.span);
            members.push(EnumMember {
                name: member_name,
                init,
                span: join_span(member_start, end),
            });
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        let end = self
            .expect(TokenKind::RightBrace, "expected `}` after enum body")?
            .span;
        Ok(Stmt::Enum(EnumDecl {
            name,
            is_const,
            members,
            span: join_span(start, end),
        }))
    }

    fn parse_module(&mut self) -> Result<Stmt<'a>, NoriError> {
        let start = self.bump().span; // module | namespace
        let name = if self.at(TokenKind::String) || self.at(TokenKind::Ident) {
            self.bump_lexeme()
        } else {
            return Err(self.error_here("expected module/namespace name"));
        };
        let body = self.parse_block()?;
        Ok(Stmt::Module(ModuleDecl {
            name,
            body,
            span: join_span(start, self.previous().span),
        }))
    }

    fn parse_type_only(&mut self) -> Result<RawStmt, NoriError> {
        let start = self.bump().span;
        if self.at(TokenKind::Ident) {
            self.bump();
        }
        if self.at(TokenKind::Less) {
            self.skip_balanced_angle_list()?;
        }
        if self.matches(TokenKind::Eq) {
            self.skip_type_until(&[TokenKind::Semicolon]);
            self.matches(TokenKind::Semicolon);
            let end = self.previous().span;
            return Ok(RawStmt {
                span: join_span(start, end),
            });
        }
        if self.at(TokenKind::LeftBrace) {
            self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace)?;
        }
        let end = self.previous().span;
        Ok(RawStmt {
            span: join_span(start, end),
        })
    }

    /// Pratt-style TypeScript type grammar.
    fn parse_ts_type(&mut self) -> Result<TSType<'a>, NoriError> {
        self.parse_ts_union()
    }

    fn parse_ts_union(&mut self) -> Result<TSType<'a>, NoriError> {
        self.matches(TokenKind::Pipe); // leading |
        let first = self.parse_ts_intersection()?;
        if !self.at(TokenKind::Pipe) {
            return Ok(first);
        }
        let start = ts_type_span(&first);
        let mut types = self.new_vec();
        types.push(first);
        while self.matches(TokenKind::Pipe) {
            types.push(self.parse_ts_intersection()?);
        }
        let end = ts_type_span(types.last().unwrap());
        Ok(TSType::Union(types, join_span(start, end)))
    }

    fn parse_ts_intersection(&mut self) -> Result<TSType<'a>, NoriError> {
        self.matches(TokenKind::Ampersand); // leading &
        let first = self.parse_ts_conditional()?;
        if !self.at(TokenKind::Ampersand) {
            return Ok(first);
        }
        let start = ts_type_span(&first);
        let mut types = self.new_vec();
        types.push(first);
        while self.matches(TokenKind::Ampersand) {
            types.push(self.parse_ts_conditional()?);
        }
        let end = ts_type_span(types.last().unwrap());
        Ok(TSType::Intersection(types, join_span(start, end)))
    }

    fn parse_ts_conditional(&mut self) -> Result<TSType<'a>, NoriError> {
        let check = self.parse_ts_postfix()?;
        if self.matches_keyword(Keyword::Extends) {
            let extends = self.parse_ts_postfix()?;
            if self.matches(TokenKind::Question) {
                let true_type = self.parse_ts_type()?;
                self.expect(TokenKind::Colon, "expected `:` in conditional type")?;
                let false_type = self.parse_ts_type()?;
                let span = join_span(ts_type_span(&check), ts_type_span(&false_type));
                return Ok(TSType::Conditional {
                    check: self.box_in(check),
                    extends: self.box_in(extends),
                    true_type: self.box_in(true_type),
                    false_type: self.box_in(false_type),
                    span,
                });
            }
            // `T extends U` without `?` — treat as intersection-like Any fallback by
            // re-wrapping; rare in annotations. Fall through with check only.
            let _ = extends;
        }
        Ok(check)
    }

    fn parse_ts_postfix(&mut self) -> Result<TSType<'a>, NoriError> {
        let mut ty = self.parse_ts_primary()?;
        loop {
            if self.matches(TokenKind::LeftBracket) {
                if self.matches(TokenKind::RightBracket) {
                    let span = join_span(ts_type_span(&ty), self.previous().span);
                    ty = TSType::Array(self.box_in(ty), span);
                } else {
                    let index = self.parse_ts_type()?;
                    let end = self
                        .expect(
                            TokenKind::RightBracket,
                            "expected `]` in indexed access type",
                        )?
                        .span;
                    let span = join_span(ts_type_span(&ty), end);
                    ty = TSType::IndexedAccess {
                        object: self.box_in(ty),
                        index: self.box_in(index),
                        span,
                    };
                }
            } else {
                break;
            }
        }
        Ok(ty)
    }

    fn parse_ts_primary(&mut self) -> Result<TSType<'a>, NoriError> {
        let start = self.peek().span;

        if self.matches_keyword(Keyword::This) {
            return Ok(TSType::This(start));
        }
        if self.matches_keyword(Keyword::Null) {
            return Ok(TSType::Literal(TSLiteral::Null(start)));
        }
        if self.matches_keyword(Keyword::True) {
            return Ok(TSType::Literal(TSLiteral::Bool(true, start)));
        }
        if self.matches_keyword(Keyword::False) {
            return Ok(TSType::Literal(TSLiteral::Bool(false, start)));
        }
        if self.matches_keyword(Keyword::Void) {
            return Ok(TSType::Keyword(TSKeywordKind::Void, start));
        }
        if self.matches_keyword(Keyword::Typeof) {
            let name =
                if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
                    self.bump_lexeme()
                } else {
                    self.atom("unknown")
                };
            return Ok(TSType::Typeof {
                expr_name: name,
                span: join_span(start, self.previous().span),
            });
        }
        if self.at_contextual_ident("keyof") {
            self.bump();
            let operand = self.parse_ts_postfix()?;
            return Ok(TSType::Operator {
                op: TSTypeOperator::Keyof,
                operand: self.box_in(operand),
                span: join_span(start, self.previous().span),
            });
        }
        if self.at_contextual_ident("readonly") {
            self.bump();
            let operand = self.parse_ts_postfix()?;
            return Ok(TSType::Operator {
                op: TSTypeOperator::Readonly,
                operand: self.box_in(operand),
                span: join_span(start, self.previous().span),
            });
        }
        if self.at_contextual_ident("infer") {
            self.bump();
            let name = self.expect_lexeme(TokenKind::Ident, "expected infer type name")?;
            return Ok(TSType::Infer {
                name,
                span: join_span(start, self.previous().span),
            });
        }
        if self.at(TokenKind::String) {
            let value = self.bump_lexeme();
            return Ok(TSType::Literal(TSLiteral::String(value, start)));
        }
        if self.at(TokenKind::Number) || self.at(TokenKind::BigInt) {
            let value = self.bump_lexeme();
            return Ok(TSType::Literal(TSLiteral::Number(value, start)));
        }
        if self.at(TokenKind::LeftBrace) {
            if self.looks_like_mapped_type() {
                return self.parse_ts_mapped_type();
            }
            let members = self.parse_ts_object_members()?;
            return Ok(TSType::Object(
                members,
                join_span(start, self.previous().span),
            ));
        }
        if self.at(TokenKind::BackTick) {
            return self.parse_ts_template_literal_type();
        }
        if self.at(TokenKind::LeftBracket) {
            self.bump();
            let mut elements = self.new_vec();
            while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
                elements.push(self.parse_ts_type()?);
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
            let end = self
                .expect(TokenKind::RightBracket, "expected `]` after tuple type")?
                .span;
            return Ok(TSType::Tuple(elements, join_span(start, end)));
        }
        if self.at(TokenKind::LeftParen) {
            // Function type `(a: T) => U` or parenthesized `(T)`.
            let checkpoint = self.input.checkpoint();
            self.bump(); // (
            let is_fn = self.looks_like_ts_function_type();
            self.input.rewind(checkpoint);
            if is_fn {
                return self.parse_ts_function_type();
            }
            self.bump(); // (
            let inner = self.parse_ts_type()?;
            let end = self
                .expect(TokenKind::RightParen, "expected `)` after type")?
                .span;
            return Ok(TSType::Parenthesized(
                self.box_in(inner),
                join_span(start, end),
            ));
        }
        if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
            let name_token = self.bump();
            let mut name = self.owned_lexeme(&name_token).as_str().to_string();
            if let Some(keyword) = ts_keyword_kind(name.as_str()) {
                return Ok(TSType::Keyword(keyword, name_token.span));
            }
            // Qualified names: `JSX.Element`, `foo.bar.Baz`
            while self.matches(TokenKind::Dot) {
                let part = self.expect_lexeme(
                    TokenKind::Ident,
                    "expected identifier after `.` in type name",
                )?;
                name.push('.');
                name.push_str(part.as_str());
            }
            let type_args = if self.at(TokenKind::Less) {
                Some(self.parse_ts_type_args()?)
            } else {
                None
            };
            let end = self.previous().span;
            return Ok(TSType::Reference {
                name: self.atom(&name),
                type_args,
                span: join_span(start, end),
            });
        }

        // Fallback: skip a balanced fragment and emit Any.
        let fallback_start = self.peek().span;
        self.skip_type_until(&[
            TokenKind::Semicolon,
            TokenKind::Comma,
            TokenKind::Eq,
            TokenKind::RightBrace,
            TokenKind::RightParen,
            TokenKind::RightBracket,
            TokenKind::Pipe,
            TokenKind::Ampersand,
            TokenKind::Greater,
        ]);
        Ok(TSType::Any(join_span(fallback_start, self.previous().span)))
    }

    fn looks_like_mapped_type(&mut self) -> bool {
        // `{ [K in ...]: ... }` — peek without consuming.
        let checkpoint = self.input.checkpoint();
        if !self.matches(TokenKind::LeftBrace) {
            self.input.rewind(checkpoint);
            return false;
        }
        if !self.matches(TokenKind::LeftBracket) {
            self.input.rewind(checkpoint);
            return false;
        }
        if !(self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_))) {
            self.input.rewind(checkpoint);
            return false;
        }
        self.bump();
        let is_mapped = self.at_keyword(Keyword::In) || self.at_contextual_ident("in");
        self.input.rewind(checkpoint);
        is_mapped
    }

    fn parse_ts_mapped_type(&mut self) -> Result<TSType<'a>, NoriError> {
        let start = self.expect(TokenKind::LeftBrace, "expected `{`")?.span;
        let readonly = self.matches_contextual_ident("readonly");
        self.expect(TokenKind::LeftBracket, "expected `[` in mapped type")?;
        let key = self.expect_lexeme(TokenKind::Ident, "expected mapped type key")?;
        if self.at_keyword(Keyword::In) || self.at_contextual_ident("in") {
            self.bump();
        } else {
            return Err(self.error_here("expected `in` in mapped type"));
        }
        let constraint = {
            let ty = self.parse_ts_type()?;
            self.box_in(ty)
        };
        self.expect(TokenKind::RightBracket, "expected `]` in mapped type")?;
        let optional = self.matches(TokenKind::Question);
        self.expect(TokenKind::Colon, "expected `:` in mapped type")?;
        let type_ann = {
            let ty = self.parse_ts_type()?;
            self.box_in(ty)
        };
        let end = self
            .expect(TokenKind::RightBrace, "expected `}` after mapped type")?
            .span;
        Ok(TSType::Mapped {
            readonly,
            key,
            constraint,
            optional,
            type_ann,
            span: join_span(start, end),
        })
    }

    fn parse_ts_template_literal_type(&mut self) -> Result<TSType<'a>, NoriError> {
        let start = self
            .expect(TokenKind::BackTick, "expected template literal type")?
            .span;
        let mut quasis = self.new_vec();
        let mut types = self.new_vec();
        loop {
            let quasi_start = self.peek().span.start;
            while !self.at(TokenKind::Eof)
                && !self.at(TokenKind::BackTick)
                && !(self.at(TokenKind::Dollar)
                    && self.peek_next_kind() == Some(TokenKind::LeftBrace))
            {
                self.bump();
            }
            let quasi_end = self.peek().span.start;
            let quasi = if quasi_end > quasi_start {
                self.atom(&self.source[quasi_start as usize..quasi_end as usize])
            } else {
                self.atom("")
            };
            quasis.push(quasi);

            if self.at(TokenKind::BackTick) {
                let end = self.bump().span;
                return Ok(TSType::TemplateLiteral {
                    quasis,
                    types,
                    span: join_span(start, end),
                });
            }
            if self.at(TokenKind::Eof) {
                return Err(self.error_here("unterminated template literal type"));
            }
            // `${`
            self.bump(); // $
            self.expect(TokenKind::LeftBrace, "expected `{` in template type")?;
            types.push(self.parse_ts_type()?);
            self.expect(
                TokenKind::RightBrace,
                "expected `}` after template type interpolation",
            )?;
        }
    }

    fn looks_like_ts_function_type(&mut self) -> bool {
        // Cursor is just after `(`. Scan for `) =>`.
        let mut depth = 1usize;
        while !self.at(TokenKind::Eof) {
            match self.bump().kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self.at(TokenKind::Arrow);
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn parse_ts_function_type(&mut self) -> Result<TSType<'a>, NoriError> {
        let start = self.expect(TokenKind::LeftParen, "expected `(`")?.span;
        let mut params = self.new_vec();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            let param_start = self.peek().span;
            self.matches(TokenKind::Ellipsis);
            let name =
                if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
                    self.bump_lexeme()
                } else {
                    self.atom("_")
                };
            let optional = self.matches(TokenKind::Question);
            let type_ann = if self.matches(TokenKind::Colon) {
                {
                    let ty = self.parse_ts_type()?;
                    Some(self.box_in(ty))
                }
            } else {
                None
            };
            params.push(TSFnParam {
                name,
                optional,
                type_ann,
                span: join_span(param_start, self.previous().span),
            });
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        self.expect(
            TokenKind::RightParen,
            "expected `)` after function type params",
        )?;
        self.expect(TokenKind::Arrow, "expected `=>` in function type")?;
        let return_type = {
            let ty = self.parse_ts_type()?;
            self.box_in(ty)
        };
        let span = join_span(start, ts_type_span(&return_type));
        Ok(TSType::Function {
            params,
            return_type,
            span,
        })
    }

    fn parse_ts_type_args(&mut self) -> Result<ArenaVec<'a, TSType<'a>>, NoriError> {
        self.expect(TokenKind::Less, "expected `<`")?;
        let mut args = self.new_vec();
        while !self.at(TokenKind::Greater) && !self.at(TokenKind::Eof) {
            args.push(self.parse_ts_type()?);
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Greater, "expected `>` after type arguments")?;
        Ok(args)
    }

    fn parse_ts_object_members(&mut self) -> Result<ArenaVec<'a, TSTypeElement<'a>>, NoriError> {
        self.expect(TokenKind::LeftBrace, "expected `{`")?;
        let mut members = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let member_start = self.peek().span;
            let readonly = self.matches_contextual_ident("readonly");
            if self.at(TokenKind::LeftBracket) {
                // Index signature `[key: string]: T`
                self.bump();
                let key_name = self.expect_lexeme(TokenKind::Ident, "expected index key name")?;
                self.expect(TokenKind::Colon, "expected `:` in index signature")?;
                let key_type = {
                    let ty = self.parse_ts_type()?;
                    self.box_in(ty)
                };
                self.expect(TokenKind::RightBracket, "expected `]` after index key")?;
                self.expect(TokenKind::Colon, "expected `:` after index signature")?;
                let type_ann = {
                    let ty = self.parse_ts_type()?;
                    self.box_in(ty)
                };
                members.push(TSTypeElement::Index {
                    key_name,
                    key_type,
                    type_ann,
                    span: join_span(member_start, self.previous().span),
                });
            } else {
                let key = if self.at(TokenKind::Ident)
                    || matches!(self.peek().kind, TokenKind::Keyword(_))
                    || self.at(TokenKind::String)
                {
                    self.bump_lexeme()
                } else {
                    break;
                };
                let optional = self.matches(TokenKind::Question);
                if self.matches(TokenKind::LeftParen) {
                    let mut params = self.new_vec();
                    while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
                        let p_start = self.peek().span;
                        let pname = if self.at(TokenKind::Ident) {
                            self.bump_lexeme()
                        } else {
                            self.atom("_")
                        };
                        let p_optional = self.matches(TokenKind::Question);
                        let p_type = if self.matches(TokenKind::Colon) {
                            {
                                let ty = self.parse_ts_type()?;
                                Some(self.box_in(ty))
                            }
                        } else {
                            None
                        };
                        params.push(TSFnParam {
                            name: pname,
                            optional: p_optional,
                            type_ann: p_type,
                            span: join_span(p_start, self.previous().span),
                        });
                        if !self.matches(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RightParen, "expected `)` after method params")?;
                    let return_type = if self.matches(TokenKind::Colon) {
                        {
                            let ty = self.parse_ts_type()?;
                            Some(self.box_in(ty))
                        }
                    } else {
                        None
                    };
                    members.push(TSTypeElement::Method {
                        key,
                        optional,
                        params,
                        return_type,
                        span: join_span(member_start, self.previous().span),
                    });
                } else {
                    let type_ann = if self.matches(TokenKind::Colon) {
                        {
                            let ty = self.parse_ts_type()?;
                            Some(self.box_in(ty))
                        }
                    } else {
                        None
                    };
                    members.push(TSTypeElement::Property {
                        key,
                        optional,
                        readonly,
                        type_ann,
                        span: join_span(member_start, self.previous().span),
                    });
                }
            }
            self.matches(TokenKind::Comma);
            self.matches(TokenKind::Semicolon);
        }
        self.expect(TokenKind::RightBrace, "expected `}` after type members")?;
        Ok(members)
    }

    fn parse_raw_until_semicolon(&mut self) -> RawStmt {
        let start = self.bump().span;
        while !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Eof) {
            self.bump();
        }
        self.matches(TokenKind::Semicolon);
        let end = self.previous().span;
        RawStmt {
            span: join_span(start, end),
        }
    }

    fn parse_expression_until_statement_end(&mut self) -> Result<Expr<'a>, NoriError> {
        self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace])
    }

    fn parse_expression_until(&mut self, stop: &[TokenKind]) -> Result<Expr<'a>, NoriError> {
        self.parse_expression_until_bp(stop, 0)
    }

    fn parse_expression_until_bp(
        &mut self,
        stop: &[TokenKind],
        min_bp: u8,
    ) -> Result<Expr<'a>, NoriError> {
        let mut lhs = self.parse_prefix(stop)?;

        loop {
            if self.at_any(stop) || self.at(TokenKind::Eof) {
                break;
            }

            if self.at(TokenKind::Question) && min_bp <= 2 {
                self.bump();
                let consequent = self.parse_expression_until(&[TokenKind::Colon])?;
                self.expect(TokenKind::Colon, "expected `:` in conditional expression")?;
                let alternate = self.parse_expression_until_bp(stop, 2)?;
                let span = join_span(lhs.span, alternate.span);
                lhs = Expr {
                    kind: ExprKind::Conditional {
                        test: self.box_in(lhs),
                        consequent: self.box_in(consequent),
                        alternate: self.box_in(alternate),
                    },
                    span,
                };
                continue;
            }

            if self.syntax.typescript && self.at_contextual_ident("as") {
                lhs = self.parse_type_erasure(lhs, TypeErasureKind::As, stop);
                continue;
            }

            if self.syntax.typescript && self.at_contextual_ident("satisfies") {
                lhs = self.parse_type_erasure(lhs, TypeErasureKind::Satisfies, stop);
                continue;
            }

            if let Some(op) = assignment_op(self.peek().kind) {
                if min_bp > 1 {
                    break;
                }
                self.bump();
                let rhs = self.parse_expression_until_bp(stop, 1)?;
                let span = join_span(lhs.span, rhs.span);
                lhs = Expr {
                    kind: ExprKind::Assign {
                        left: self.box_in(lhs),
                        op: self.atom(op),
                        right: self.box_in(rhs),
                    },
                    span,
                };
                continue;
            }

            let Some((left_bp, right_bp, op)) = infix_binding_power(self.peek().kind) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expression_until_bp(stop, right_bp)?;
            let span = join_span(lhs.span, rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    left: self.box_in(lhs),
                    op: self.atom(op),
                    right: self.box_in(rhs),
                },
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self, stop: &[TokenKind]) -> Result<Expr<'a>, NoriError> {
        if self.at_any(stop) || self.at(TokenKind::Eof) {
            return Err(self.error_here("expected expression"));
        }

        let token = self.bump();
        let mut expr = match token.kind {
            TokenKind::Ident => {
                if self.at(TokenKind::Arrow) {
                    self.bump();
                    let body = self.parse_arrow_body(stop)?;
                    let span = match &body {
                        ArrowBody::Expression(e) => join_span(token.span, e.span),
                        ArrowBody::Block(b) => join_span(token.span, b.span),
                    };
                    Expr {
                        kind: ExprKind::Arrow {
                            params: {
                                let mut p = self.new_vec();
                                p.push(self.owned_lexeme(&token));
                                p
                            },
                            body,
                        },
                        span,
                    }
                } else {
                    Expr {
                        kind: ExprKind::Ident(self.owned_lexeme(&token)),
                        span: token.span,
                    }
                }
            }
            TokenKind::Number => Expr {
                kind: ExprKind::Number(self.owned_lexeme(&token)),
                span: token.span,
            },
            TokenKind::String => Expr {
                kind: ExprKind::String(self.owned_lexeme(&token)),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::True) => Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::False) => Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Null) => Expr {
                kind: ExprKind::Null,
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Super) => Expr {
                kind: ExprKind::Ident(self.owned_lexeme(&token)),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::This) => Expr {
                kind: ExprKind::This,
                span: token.span,
            },
            TokenKind::Keyword(Keyword::New) => {
                if self.matches(TokenKind::Dot) {
                    let prop =
                        self.expect_lexeme(TokenKind::Ident, "expected `target` after `new.`")?;
                    let span = join_span(token.span, self.previous().span);
                    Expr {
                        kind: ExprKind::MetaProperty {
                            meta: self.atom("new"),
                            property: prop,
                        },
                        span,
                    }
                } else {
                    let callee = self.parse_new_callee(stop)?;
                    self.expect(TokenKind::LeftParen, "expected `(` after `new` callee")?;
                    let args = self.parse_args()?;
                    let end = self
                        .expect(TokenKind::RightParen, "expected `)` after `new` arguments")?
                        .span;
                    let span = join_span(token.span, end);
                    Expr {
                        kind: ExprKind::New {
                            callee: self.box_in(callee),
                            args,
                        },
                        span,
                    }
                }
            }
            TokenKind::Keyword(Keyword::Delete) => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Delete(self.box_in(rhs)),
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Void) => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Void(self.box_in(rhs)),
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Typeof) => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Typeof(self.box_in(rhs)),
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Yield) => {
                if self.loop_depth > 0 {
                    return Err(self.error_here("`yield` is not allowed inside loops"));
                }
                let delegate = self.matches(TokenKind::Star);
                let value = if self.at_any(stop)
                    || self.at(TokenKind::Semicolon)
                    || self.at(TokenKind::RightBrace)
                    || self.at(TokenKind::RightParen)
                    || self.at(TokenKind::Comma)
                    || self.at(TokenKind::Colon)
                    || self.at(TokenKind::Eq)
                {
                    None
                } else {
                    {
                        let e = self.parse_expression_until_bp(stop, 2)?;
                        Some(self.box_in(e))
                    }
                };
                let span = join_span(token.span, value.as_ref().map_or(token.span, |v| v.span));
                Expr {
                    kind: ExprKind::Yield { value, delegate },
                    span,
                }
            }
            TokenKind::BigInt => Expr {
                kind: ExprKind::BigInt(self.owned_lexeme(&token)),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Import) if self.at(TokenKind::Dot) => {
                self.bump();
                let prop =
                    self.expect_lexeme(TokenKind::Ident, "expected `meta` after `import.`")?;
                let span = join_span(token.span, self.previous().span);
                Expr {
                    kind: ExprKind::MetaProperty {
                        meta: self.atom("import"),
                        property: prop,
                    },
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Import) if self.at(TokenKind::LeftParen) => {
                self.bump();
                let arg = self.parse_expression_until(&[TokenKind::RightParen])?;
                let end = self
                    .expect(TokenKind::RightParen, "expected `)` after `import(`")?
                    .span;
                let span = join_span(token.span, end);
                Expr {
                    kind: ExprKind::Import(self.box_in(arg)),
                    span,
                }
            }
            TokenKind::Bang | TokenKind::Minus | TokenKind::Plus => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Unary {
                        op: self.owned_lexeme(&token),
                        expr: self.box_in(rhs),
                    },
                    span,
                }
            }
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Update {
                        op: self.owned_lexeme(&token),
                        expr: self.box_in(rhs),
                        prefix: true,
                    },
                    span,
                }
            }
            TokenKind::LeftParen => {
                if self.looks_like_arrow_params()? {
                    let params = self.collect_arrow_params()?;
                    self.expect(TokenKind::RightParen, "expected `)` after arrow parameters")?;
                    self.expect(TokenKind::Arrow, "expected `=>` after arrow parameters")?;
                    let body = self.parse_arrow_body(stop)?;
                    let span = match &body {
                        ArrowBody::Expression(e) => join_span(token.span, e.span),
                        ArrowBody::Block(b) => join_span(token.span, b.span),
                    };
                    Expr {
                        kind: ExprKind::Arrow { params, body },
                        span,
                    }
                } else {
                    let inner = self.parse_expression_until(&[TokenKind::RightParen])?;
                    let end = self.expect(TokenKind::RightParen, "expected `)`")?.span;
                    if matches!(inner.kind, ExprKind::Markup(_)) {
                        let span = join_span(token.span, end);
                        return Ok(Expr { span, ..inner });
                    }
                    Expr {
                        kind: ExprKind::Raw,
                        span: join_span(token.span, end),
                    }
                }
            }
            TokenKind::LeftBracket => self.parse_array(token.span, stop)?,
            TokenKind::Keyword(Keyword::Class) => {
                let class = self.parse_class()?;
                Expr {
                    kind: ExprKind::Raw,
                    span: class.span,
                }
            }
            TokenKind::LeftBrace => self.parse_object_raw(token.span)?,
            TokenKind::Less if self.syntax.markup => self.parse_markup_after_less(token.span)?,
            TokenKind::Keyword(Keyword::Await) => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Await(self.box_in(rhs)),
                    span,
                }
            }
            _ => {
                let expr = self.parse_raw_expression_from(token.span, stop);
                return Ok(expr);
            }
        };

        expr = self.parse_postfix(expr, stop)?;
        Ok(expr)
    }

    fn parse_postfix(
        &mut self,
        mut expr: Expr<'a>,
        stop: &[TokenKind],
    ) -> Result<Expr<'a>, NoriError> {
        loop {
            if self.at_any(stop) || self.at(TokenKind::Eof) {
                break;
            }
            if self.matches(TokenKind::Dot) {
                let optional = false;
                let prop = self.expect(TokenKind::Ident, "expected property name after `.`")?;
                let span = join_span(expr.span, prop.span);
                expr = Expr {
                    kind: ExprKind::Member {
                        object: self.box_in(expr),
                        property: self.owned_lexeme(&prop),
                        optional,
                    },
                    span,
                };
                continue;
            }
            if self.matches(TokenKind::QuestionDot) {
                if self.at(TokenKind::LeftBracket) {
                    self.bump();
                    let index = self.parse_expression_until(&[TokenKind::RightBracket])?;
                    let end = self
                        .expect(TokenKind::RightBracket, "expected `]` after optional index")?
                        .span;
                    let span = join_span(expr.span, end);
                    expr = Expr {
                        kind: ExprKind::Index {
                            object: self.box_in(expr),
                            index: self.box_in(index),
                            optional: true,
                        },
                        span,
                    };
                    continue;
                }
                if self.at(TokenKind::LeftParen) {
                    self.bump();
                    let args = self.parse_args()?;
                    let end = self
                        .expect(TokenKind::RightParen, "expected `)` after optional call")?
                        .span;
                    let span = join_span(expr.span, end);
                    expr = Expr {
                        kind: ExprKind::Call {
                            callee: self.box_in(expr),
                            args,
                            optional: true,
                        },
                        span,
                    };
                    continue;
                }
                let prop = self.expect(TokenKind::Ident, "expected property name after `?.`")?;
                let span = join_span(expr.span, prop.span);
                expr = Expr {
                    kind: ExprKind::Member {
                        object: self.box_in(expr),
                        property: self.owned_lexeme(&prop),
                        optional: true,
                    },
                    span,
                };
                continue;
            }
            if self.matches(TokenKind::LeftBracket) {
                let index = self.parse_expression_until(&[TokenKind::RightBracket])?;
                let end = self
                    .expect(TokenKind::RightBracket, "expected `]` after index")?
                    .span;
                let span = join_span(expr.span, end);
                expr = Expr {
                    kind: ExprKind::Index {
                        object: self.box_in(expr),
                        index: self.box_in(index),
                        optional: false,
                    },
                    span,
                };
                continue;
            }
            if self.matches(TokenKind::LeftParen) {
                let args = self.parse_args()?;
                let end = self
                    .expect(TokenKind::RightParen, "expected `)` after arguments")?
                    .span;
                let span = join_span(expr.span, end);
                expr = Expr {
                    kind: ExprKind::Call {
                        callee: self.box_in(expr),
                        args,
                        optional: false,
                    },
                    span,
                };
                continue;
            }
            if self.syntax.typescript
                && allows_call_type_arguments(&expr)
                && self.looks_like_call_type_arguments()
            {
                self.skip_balanced_angle_list()?;
                continue;
            }
            if self.syntax.typescript && self.matches(TokenKind::Bang) {
                let span = join_span(expr.span, self.previous().span);
                expr = Expr {
                    kind: ExprKind::TypeErasure {
                        kind: TypeErasureKind::NonNull,
                        expr: self.box_in(expr),
                    },
                    span,
                };
                continue;
            }
            if self.at(TokenKind::PlusPlus) || self.at(TokenKind::MinusMinus) {
                let update = self.bump();
                let span = join_span(expr.span, update.span);
                expr = Expr {
                    kind: ExprKind::Update {
                        op: self.owned_lexeme(&update),
                        expr: self.box_in(expr),
                        prefix: false,
                    },
                    span,
                };
                break;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_type_erasure(
        &mut self,
        expr: Expr<'a>,
        kind: TypeErasureKind,
        stop: &[TokenKind],
    ) -> Expr<'a> {
        self.bump();
        self.skip_type_until_expression_boundary(stop);
        let span = join_span(expr.span, self.previous().span);
        Expr {
            kind: ExprKind::TypeErasure {
                kind,
                expr: self.box_in(expr),
            },
            span,
        }
    }

    fn parse_arrow_body(&mut self, stop: &[TokenKind]) -> Result<ArrowBody<'a>, NoriError> {
        if self.at(TokenKind::LeftBrace) {
            let block = self.parse_block()?;
            Ok(ArrowBody::Block(block))
        } else {
            let expr = self.parse_expression_until(stop)?;
            Ok(ArrowBody::Expression(self.box_in(expr)))
        }
    }

    fn parse_new_callee(&mut self, stop: &[TokenKind]) -> Result<Expr<'a>, NoriError> {
        let mut expr = self.parse_prefix(stop)?;
        loop {
            if self.at_any(stop)
                || self.at(TokenKind::Eof)
                || self.at(TokenKind::LeftParen)
                || self.at(TokenKind::Semicolon)
                || self.at(TokenKind::RightBrace)
            {
                break;
            }
            if self.at(TokenKind::Dot) {
                expr = self.parse_postfix(expr, stop)?;
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<ArenaVec<'a, Expr<'a>>, NoriError> {
        let mut args = self.new_vec();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Ellipsis) {
                let expr =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?;
                let span = expr.span;
                args.push(Expr {
                    kind: ExprKind::Spread {
                        expr: self.box_in(expr),
                    },
                    span,
                });
            } else {
                args.push(self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?);
            }
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_array(&mut self, start: Span, _stop: &[TokenKind]) -> Result<Expr<'a>, NoriError> {
        let mut items = self.new_vec();
        while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Ellipsis) {
                let expr =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBracket])?;
                let span = expr.span;
                items.push(Expr {
                    kind: ExprKind::Spread {
                        expr: self.box_in(expr),
                    },
                    span,
                });
            } else {
                items.push(
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBracket])?,
                );
            }
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        let end = self
            .expect(TokenKind::RightBracket, "expected `]` after array")?
            .span;
        Ok(Expr {
            kind: ExprKind::Array(items),
            span: join_span(start, end),
        })
    }

    fn parse_object_raw(&mut self, start: Span) -> Result<Expr<'a>, NoriError> {
        let mut properties = self.new_vec();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Comma) {
                continue;
            }
            if self.at(TokenKind::Ellipsis) {
                self.bump();
                let expr =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBrace])?;
                let span = join_span(self.previous().span, expr.span);
                properties.push(nori_ast::ObjectProperty {
                    key: nori_ast::PropertyKey::Ident(self.atom("")),
                    value: Expr {
                        kind: ExprKind::Spread {
                            expr: self.box_in(expr),
                        },
                        span,
                    },
                    kind: nori_ast::PropertyKind::Init,
                    computed: false,
                    shorthand: false,
                    span,
                });
                if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Comma) {
                    break;
                }
                continue;
            }
            let prop_start = self.peek().span;
            let (key, computed, name) = if self.at(TokenKind::LeftBracket) {
                self.bump();
                let expr = self.parse_expression_until(&[TokenKind::RightBracket])?;
                self.expect(TokenKind::RightBracket, "expected `]`")?;
                (
                    nori_ast::PropertyKey::Computed(self.box_in(expr)),
                    true,
                    self.atom(""),
                )
            } else if self.at(TokenKind::String) {
                let s = self.bump_lexeme();
                (nori_ast::PropertyKey::String(s), false, s)
            } else if self.at(TokenKind::Number) {
                let n = self.bump_lexeme();
                (nori_ast::PropertyKey::Number(n), false, n)
            } else if self.at(TokenKind::Ident) {
                let n = self.bump_lexeme();
                (nori_ast::PropertyKey::Ident(n), false, n)
            } else {
                break;
            };
            if self.matches(TokenKind::Colon) {
                let value =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBrace])?;
                let span = join_span(prop_start, value.span);
                properties.push(nori_ast::ObjectProperty {
                    key,
                    value,
                    kind: nori_ast::PropertyKind::Init,
                    computed,
                    shorthand: false,
                    span,
                });
            } else if name.is_empty() {
                break;
            } else {
                let span = join_span(prop_start, self.previous().span);
                properties.push(nori_ast::ObjectProperty {
                    key,
                    value: Expr {
                        kind: ExprKind::Ident(name),
                        span,
                    },
                    kind: nori_ast::PropertyKind::Init,
                    computed,
                    shorthand: true,
                    span,
                });
            }
        }
        let end = self
            .expect(TokenKind::RightBrace, "expected `}` after object literal")?
            .span;
        Ok(Expr {
            kind: ExprKind::Object(properties),
            span: join_span(start, end),
        })
    }

    fn parse_markup_after_less(&mut self, start: Span) -> Result<Expr<'a>, NoriError> {
        let node = if self.matches(TokenKind::Greater) {
            let children = self.parse_markup_children(None)?;
            let end = self.expect_fragment_close()?;
            MarkupNode::Fragment {
                children,
                span: join_span(start, end),
            }
        } else {
            MarkupNode::Element(self.parse_markup_element_after_less(start)?)
        };
        let span = match &node {
            MarkupNode::Element(element) => element.span,
            MarkupNode::Fragment { span, .. } => *span,
        };
        Ok(Expr {
            kind: ExprKind::Markup(node),
            span,
        })
    }

    fn parse_markup_element_after_less(
        &mut self,
        start: Span,
    ) -> Result<MarkupElement<'a>, NoriError> {
        let name = self.parse_markup_name()?;
        let mut attributes = self.new_vec();
        while !self.at(TokenKind::Greater)
            && !self.at(TokenKind::SlashGreater)
            && !self.at(TokenKind::Eof)
        {
            attributes.push(self.parse_markup_attribute()?);
        }
        if self.matches(TokenKind::SlashGreater) {
            let end = self.previous().span;
            return Ok(MarkupElement {
                name,
                attributes,
                children: self.new_vec(),
                self_closing: true,
                span: join_span(start, end),
            });
        }
        self.expect(TokenKind::Greater, "expected `>` after markup opening tag")?;
        let children = self.parse_markup_children(Some(name.as_str()))?;
        let end = self.expect_markup_close(&name)?;
        Ok(MarkupElement {
            name,
            attributes,
            children,
            self_closing: false,
            span: join_span(start, end),
        })
    }

    fn parse_markup_children(
        &mut self,
        closing_name: Option<&str>,
    ) -> Result<ArenaVec<'a, MarkupChild<'a>>, NoriError> {
        let mut children = self.new_vec();
        loop {
            if self.at(TokenKind::Eof) {
                return Err(self.error_here("unterminated markup element"));
            }
            if self.at(TokenKind::Less) && self.peek_next_kind() == Some(TokenKind::Slash) {
                break;
            }
            if closing_name.is_none()
                && self.at(TokenKind::Less)
                && self.peek_next_kind() == Some(TokenKind::Greater)
            {
                break;
            }
            if self.at(TokenKind::MarkupText) {
                let token = self.bump();
                let text = self.owned_lexeme(&token);
                children.push(MarkupChild::Text(text, token.span));
                continue;
            }
            if self.matches(TokenKind::LeftBrace) {
                let expr = self.parse_expression_until(&[TokenKind::RightBrace])?;
                self.expect(
                    TokenKind::RightBrace,
                    "expected `}` after markup expression",
                )?;
                children.push(MarkupChild::Expr(expr));
                continue;
            }
            if self.matches(TokenKind::Less) {
                if self.matches(TokenKind::Greater) {
                    let fragment_start = self.previous().span;
                    let fragment_children = self.parse_markup_children(None)?;
                    let end = self.expect_fragment_close()?;
                    children.push(MarkupChild::Node(MarkupNode::Fragment {
                        children: fragment_children,
                        span: join_span(fragment_start, end),
                    }));
                } else {
                    let start = self.previous().span;
                    children.push(MarkupChild::Node(MarkupNode::Element(
                        self.parse_markup_element_after_less(start)?,
                    )));
                }
                continue;
            }
            return Err(self.error_here("unexpected token in markup children"));
        }
        Ok(children)
    }

    fn parse_markup_attribute(&mut self) -> Result<MarkupAttribute<'a>, NoriError> {
        if self.matches(TokenKind::LeftBrace) {
            let start = self.previous().span;
            self.expect(
                TokenKind::Ellipsis,
                "expected `...` in markup spread attribute",
            )?;
            let expr = self.parse_expression_until(&[TokenKind::RightBrace])?;
            let end = self
                .expect(TokenKind::RightBrace, "expected `}` after spread attribute")?
                .span;
            return Ok(MarkupAttribute::Spread {
                expr,
                span: join_span(start, end),
            });
        }
        let name_token = self.expect_markup_ident("expected markup attribute name")?;
        let value = if self.matches(TokenKind::Eq) {
            if self.at(TokenKind::String) {
                let token = self.bump();
                Some(Expr {
                    kind: ExprKind::String(self.owned_lexeme(&token)),
                    span: token.span,
                })
            } else if self.matches(TokenKind::LeftBrace) {
                let expr = self.parse_expression_until(&[TokenKind::RightBrace])?;
                self.expect(
                    TokenKind::RightBrace,
                    "expected `}` after markup attribute expression",
                )?;
                Some(expr)
            } else {
                return Err(self.error_here("expected markup attribute value"));
            }
        } else {
            None
        };
        let span = value.as_ref().map_or(name_token.span, |expr| {
            join_span(name_token.span, expr.span)
        });
        Ok(MarkupAttribute::Named {
            name: self.owned_lexeme(&name_token),
            value,
            span,
        })
    }

    fn parse_markup_name(&mut self) -> Result<Atom<'a>, NoriError> {
        let first = self.expect_markup_lexeme("expected markup tag name")?;
        if !self.at(TokenKind::Dot) {
            return Ok(first);
        }
        let mut name = first.as_str().to_string();
        while self.matches(TokenKind::Dot) {
            name.push('.');
            name.push_str(
                self.expect_markup_lexeme("expected markup member name")?
                    .as_str(),
            );
        }
        Ok(self.atom(&name))
    }

    fn expect_markup_close(&mut self, name: &str) -> Result<Span, NoriError> {
        self.expect(TokenKind::Less, "expected markup closing tag")?;
        self.expect(TokenKind::Slash, "expected `/` in markup closing tag")?;
        let close_name = self.parse_markup_name()?;
        if close_name != name {
            return Err(self.error_here(&format!(
                "expected markup closing tag `</{name}>`, found `</{close_name}>`"
            )));
        }
        Ok(self
            .expect(TokenKind::Greater, "expected `>` after markup closing tag")?
            .span)
    }

    fn expect_fragment_close(&mut self) -> Result<Span, NoriError> {
        self.expect(TokenKind::Less, "expected fragment closing tag")?;
        self.expect(TokenKind::Slash, "expected `/` in fragment closing tag")?;
        self.expect(
            TokenKind::Greater,
            "expected `>` after fragment closing tag",
        )
        .map(|token| token.span)
    }

    fn parse_raw_expression_from(&mut self, start: Span, stop: &[TokenKind]) -> Expr<'a> {
        while !self.at_any(stop) && !self.at(TokenKind::Eof) {
            self.bump();
        }
        let end = self.previous().span;
        Expr {
            kind: ExprKind::Raw,
            span: join_span(start, end),
        }
    }

    fn looks_like_arrow_params(&mut self) -> Result<bool, NoriError> {
        let checkpoint = self.input.checkpoint();
        let mut depth = 1usize;
        loop {
            match self.peek().kind {
                TokenKind::LeftParen => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::RightParen => {
                    self.bump();
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let is_arrow = self.at(TokenKind::Arrow);
                        self.input.rewind(checkpoint);
                        return Ok(is_arrow);
                    }
                }
                TokenKind::Eof => {
                    self.input.rewind(checkpoint);
                    return Ok(false);
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn collect_arrow_params(&mut self) -> Result<ArenaVec<'a, Atom<'a>>, NoriError> {
        let mut params = self.new_vec();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.matches(TokenKind::Ellipsis);
            let name = self.expect_lexeme(TokenKind::Ident, "expected arrow parameter")?;
            if self.matches(TokenKind::Colon) {
                self.skip_type_until(&[TokenKind::Comma, TokenKind::RightParen]);
            }
            params.push(name);
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn skip_type_until(&mut self, stop: &[TokenKind]) {
        let mut paren = 0usize;
        let mut bracket = 0usize;
        let mut brace = 0usize;
        let mut angle = 0usize;
        while !self.at(TokenKind::Eof) {
            let kind = self.peek().kind;
            if paren == 0 && bracket == 0 && brace == 0 && angle == 0 && stop.contains(&kind) {
                break;
            }
            match kind {
                TokenKind::LeftParen => paren += 1,
                TokenKind::RightParen => paren = paren.saturating_sub(1),
                TokenKind::LeftBracket => bracket += 1,
                TokenKind::RightBracket => bracket = bracket.saturating_sub(1),
                TokenKind::LeftBrace => brace += 1,
                TokenKind::RightBrace => brace = brace.saturating_sub(1),
                TokenKind::Less => angle += 1,
                TokenKind::Greater => angle = angle.saturating_sub(1),
                TokenKind::Pipe => {}      // union types
                TokenKind::Ampersand => {} // intersection types
                _ => {}
            }
            self.bump();
        }
    }

    fn skip_type_until_expression_boundary(&mut self, stop: &[TokenKind]) {
        let mut paren = 0usize;
        let mut bracket = 0usize;
        let mut brace = 0usize;
        let mut angle = 0usize;
        while !self.at(TokenKind::Eof) {
            let kind = self.peek().kind;
            if paren == 0
                && bracket == 0
                && brace == 0
                && angle == 0
                && (stop.contains(&kind) || is_expression_type_boundary(kind))
            {
                break;
            }
            match kind {
                TokenKind::LeftParen => paren += 1,
                TokenKind::RightParen => paren = paren.saturating_sub(1),
                TokenKind::LeftBracket => bracket += 1,
                TokenKind::RightBracket => bracket = bracket.saturating_sub(1),
                TokenKind::LeftBrace => brace += 1,
                TokenKind::RightBrace => brace = brace.saturating_sub(1),
                TokenKind::Less => angle += 1,
                TokenKind::Greater => angle = angle.saturating_sub(1),
                _ => {}
            }
            self.bump();
        }
    }

    fn looks_like_call_type_arguments(&mut self) -> bool {
        if !self.at(TokenKind::Less) {
            return false;
        }

        let checkpoint = self.input.checkpoint();
        let mut depth = 0usize;
        let looks_like_call = loop {
            match self.bump().kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break self.at(TokenKind::LeftParen);
                    }
                }
                TokenKind::Eof => break false,
                _ => {}
            }
        };
        self.input.rewind(checkpoint);
        looks_like_call
    }

    fn skip_balanced_angle_list(&mut self) -> Result<(), NoriError> {
        self.expect(TokenKind::Less, "expected `<`")?;
        let mut depth = 1usize;
        while depth > 0 {
            if self.at(TokenKind::Eof) {
                return Err(self.error_here("unterminated generic parameter list"));
            }
            match self.bump().kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    fn skip_balanced(&mut self, open: TokenKind, close: TokenKind) -> Result<(), NoriError> {
        self.expect(open, "expected opening delimiter")?;
        self.skip_until_matching(open, close)
    }

    fn skip_until_matching(&mut self, open: TokenKind, close: TokenKind) -> Result<(), NoriError> {
        let mut depth = 1usize;
        while depth > 0 {
            if self.at(TokenKind::Eof) {
                return Err(self.error_here("unterminated balanced construct"));
            }
            let kind = self.bump().kind;
            if kind == open {
                depth += 1;
            } else if kind == close {
                depth -= 1;
            }
        }
        Ok(())
    }

    fn consume_optional_semicolon(&mut self) {
        self.matches(TokenKind::Semicolon);
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, NoriError> {
        if self.at(kind) {
            Ok(self.bump())
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword, message: &str) -> Result<Token, NoriError> {
        if self.at_keyword(keyword) {
            Ok(self.bump())
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_contextual(&mut self, name: &str, message: &str) -> Result<Token, NoriError> {
        if self.at_contextual_ident(name) {
            Ok(self.bump())
        } else {
            Err(self.error_here(message))
        }
    }

    fn at_contextual_ident(&self, name: &str) -> bool {
        self.at(TokenKind::Ident) && self.lexeme_of(self.peek()) == name
    }

    fn at_any_contextual_ident(&self, names: &[&str]) -> bool {
        names.iter().any(|name| self.at_contextual_ident(name))
    }

    fn matches_contextual_ident(&mut self, name: &str) -> bool {
        if self.at_contextual_ident(name) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_markup_ident(&mut self, message: &str) -> Result<Token, NoriError> {
        if self.at(TokenKind::Ident) || matches!(self.peek().kind, TokenKind::Keyword(_)) {
            Ok(self.bump())
        } else {
            Err(self.error_here(message))
        }
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn matches_keyword(&mut self, keyword: Keyword) -> bool {
        if self.at_keyword(keyword) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.peek().kind == TokenKind::Keyword(keyword)
    }

    fn at_any_keyword(&self, keywords: &[Keyword]) -> bool {
        keywords.iter().any(|keyword| self.at_keyword(*keyword))
    }

    fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.iter().any(|kind| self.at(*kind))
    }

    fn bump(&mut self) -> Token {
        self.input.bump()
    }

    fn peek(&self) -> &Token {
        self.input.peek()
    }

    fn previous(&self) -> &Token {
        self.input.previous()
    }

    fn peek_next_kind(&self) -> Option<TokenKind> {
        self.input.peek_next_kind()
    }

    fn error_here(&self, message: &str) -> NoriError {
        let token = self.peek();
        let message = format!(
            "{} in {} at {}:{}",
            message, self.filename, token.span.start, token.span.end
        );
        NoriError::Parse {
            message,
            span: source_span(token.span.start as usize, token.span.end as usize),
        }
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Import(import) => import.span,
        Stmt::Export(export) => match export {
            nori_ast::ExportDecl::Named { span, .. } | nori_ast::ExportDecl::All { span, .. } => {
                *span
            }
            nori_ast::ExportDecl::TypeOnly(span) => *span,
            nori_ast::ExportDecl::Declaration(stmt) => stmt_span(stmt),
        },
        Stmt::TypeOnly(raw) | Stmt::Raw(raw) => raw.span,
        Stmt::TypeAlias(alias) => alias.span,
        Stmt::Interface(iface) => iface.span,
        Stmt::Enum(enum_decl) => enum_decl.span,
        Stmt::Module(module) => module.span,
        Stmt::Var(var) => var.span,
        Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => function.span,
        Stmt::ExportDefaultExpr(expr) | Stmt::Expr(expr) => expr.span,
        Stmt::Return(_, span) => *span,
        Stmt::Block(block) => block.span,
        Stmt::If(stmt) => stmt.span,
        Stmt::Class(class) => class.span,
        Stmt::Try(stmt) => stmt.span,
        Stmt::For(stmt) => stmt.span,
        Stmt::ClassicFor(stmt) => stmt.span,
        Stmt::While(stmt) => stmt.span,
        Stmt::DoWhile(stmt) => stmt.span,
        Stmt::Switch(stmt) => stmt.span,
        Stmt::Throw(stmt) => stmt.span,
        Stmt::Label(stmt) => stmt.span,
        Stmt::Debugger(span) => *span,
        Stmt::With(stmt) => stmt.span,
        Stmt::Break(span) | Stmt::Continue(span) => *span,
    }
}

fn join_span(start: Span, end: Span) -> Span {
    Span::new(start.start, end.end)
}

fn ts_type_span(ty: &TSType<'_>) -> Span {
    match ty {
        TSType::Keyword(_, span)
        | TSType::Reference { span, .. }
        | TSType::Union(_, span)
        | TSType::Intersection(_, span)
        | TSType::Array(_, span)
        | TSType::Tuple(_, span)
        | TSType::Object(_, span)
        | TSType::Function { span, .. }
        | TSType::Conditional { span, .. }
        | TSType::Infer { span, .. }
        | TSType::Typeof { span, .. }
        | TSType::IndexedAccess { span, .. }
        | TSType::Operator { span, .. }
        | TSType::Mapped { span, .. }
        | TSType::TemplateLiteral { span, .. }
        | TSType::Parenthesized(_, span)
        | TSType::This(span)
        | TSType::Any(span) => *span,
        TSType::Literal(lit) => match lit {
            TSLiteral::String(_, span)
            | TSLiteral::Number(_, span)
            | TSLiteral::Bool(_, span)
            | TSLiteral::Null(span) => *span,
        },
    }
}

fn ts_keyword_kind(name: &str) -> Option<TSKeywordKind> {
    Some(match name {
        "any" => TSKeywordKind::Any,
        "unknown" => TSKeywordKind::Unknown,
        "never" => TSKeywordKind::Never,
        "string" => TSKeywordKind::String,
        "number" => TSKeywordKind::Number,
        "boolean" => TSKeywordKind::Boolean,
        "symbol" => TSKeywordKind::Symbol,
        "bigint" => TSKeywordKind::Bigint,
        "object" => TSKeywordKind::Object,
        "void" => TSKeywordKind::Void,
        "undefined" => TSKeywordKind::Undefined,
        "null" => TSKeywordKind::Null,
        _ => return None,
    })
}

fn assignment_op(kind: TokenKind) -> Option<&'static str> {
    Some(match kind {
        TokenKind::Eq => "=",
        TokenKind::PlusEq => "+=",
        TokenKind::MinusEq => "-=",
        TokenKind::StarEq => "*=",
        TokenKind::StarStarEq => "**=",
        TokenKind::SlashEq => "/=",
        TokenKind::AndAndEq => "&&=",
        TokenKind::OrOrEq => "||=",
        TokenKind::QuestionQuestionEq => "??=",
        TokenKind::PipeEq => "|=",
        TokenKind::AmpersandEq => "&=",
        TokenKind::CaretEq => "^=",
        TokenKind::ShiftLeftEq => "<<=",
        TokenKind::ShiftRightEq => ">>=",
        TokenKind::ShiftRightUnsignedEq => ">>>=",
        _ => return None,
    })
}

fn infix_binding_power(kind: TokenKind) -> Option<(u8, u8, &'static str)> {
    match kind {
        TokenKind::Keyword(Keyword::In) => return Some((9, 10, "in")),
        TokenKind::Keyword(Keyword::Instanceof) => return Some((9, 10, "instanceof")),
        _ => {}
    }
    Some(match kind {
        TokenKind::OrOr => (3, 4, "||"),
        TokenKind::QuestionQuestion => (3, 4, "??"),
        TokenKind::AndAnd => (5, 6, "&&"),
        TokenKind::Pipe => (5, 6, "|"),
        TokenKind::Caret => (6, 7, "^"),
        TokenKind::Ampersand => (7, 8, "&"),
        TokenKind::EqEq | TokenKind::BangEq => {
            (7, 8, if kind == TokenKind::EqEq { "==" } else { "!=" })
        }
        TokenKind::EqEqEq | TokenKind::BangEqEq => (
            7,
            8,
            if kind == TokenKind::EqEqEq {
                "==="
            } else {
                "!=="
            },
        ),
        TokenKind::Less | TokenKind::LessEq | TokenKind::Greater | TokenKind::GreaterEq => {
            let op = match kind {
                TokenKind::Less => "<",
                TokenKind::LessEq => "<=",
                TokenKind::Greater => ">",
                TokenKind::GreaterEq => ">=",
                _ => unreachable!(),
            };
            (9, 10, op)
        }
        TokenKind::ShiftLeft | TokenKind::ShiftRight | TokenKind::ShiftRightUnsigned => {
            let op = match kind {
                TokenKind::ShiftLeft => "<<",
                TokenKind::ShiftRight => ">>",
                TokenKind::ShiftRightUnsigned => ">>>",
                _ => unreachable!(),
            };
            (10, 11, op)
        }
        TokenKind::Plus | TokenKind::Minus => {
            (11, 12, if kind == TokenKind::Plus { "+" } else { "-" })
        }
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
            let op = match kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                _ => unreachable!(),
            };
            (13, 14, op)
        }
        TokenKind::StarStar => (15, 14, "**"),
        _ => return None,
    })
}

fn allows_call_type_arguments(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::Index { .. }
    )
}

fn is_expression_type_boundary(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Question
            | TokenKind::Plus
            | TokenKind::PlusPlus
            | TokenKind::Minus
            | TokenKind::MinusMinus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Bang
            | TokenKind::Eq
            | TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::LessEq
            | TokenKind::GreaterEq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::AndAnd
            | TokenKind::OrOr
            | TokenKind::Arrow
    )
}
