mod cursor;
mod syntax;

use cursor::TokenCursor;
use nori_ast::{
    BlockStmt, ClassDecl, DestructuringKind, DestructuringPattern, Expr, ExprKind, ForStmt,
    FunctionDecl, IfStmt, MarkupAttribute, MarkupChild, MarkupElement, MarkupNode, Param, Program,
    RawStmt, Span, Stmt, TryStmt, VarDecl, VarDeclarator, VarKind,
};
use nori_diagnostic::{NoriError, span as source_span};
use nori_lexer::{Keyword, Token, TokenKind};
pub use syntax::Syntax;

pub struct Parser {
    filename: String,
    input: TokenCursor,
    syntax: Syntax,
}

impl Parser {
    pub fn new(source: &str, filename: String, tokens: Vec<Token>) -> Self {
        Self::new_with_syntax(source, filename, tokens, Syntax::default())
    }

    pub fn new_with_syntax(
        _source: &str,
        filename: String,
        tokens: Vec<Token>,
        syntax: Syntax,
    ) -> Self {
        Self {
            filename,
            input: TokenCursor::new(tokens),
            syntax,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, NoriError> {
        let mut body = Vec::new();
        while !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Semicolon) {
                continue;
            }
            body.push(self.parse_stmt()?);
        }
        Ok(Program { body })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, NoriError> {
        if self.at_keyword(Keyword::Import) {
            return Ok(Stmt::Import(self.parse_raw_until_semicolon()));
        }
        if self.syntax.typescript
            && (self.at_keyword(Keyword::Type) || self.at_keyword(Keyword::Interface))
        {
            return self.parse_type_only().map(Stmt::TypeOnly);
        }
        if self.syntax.typescript && self.at_keyword(Keyword::Class) {
            return self.parse_class().map(Stmt::Class);
        }
        if self.at_keyword(Keyword::Export) {
            return self.parse_export();
        }
        if self.at(TokenKind::At) {
            return self.parse_decorated().map(Stmt::Function);
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
            return self.parse_for();
        }
        if self.at(TokenKind::LeftBrace) {
            return self.parse_block().map(Stmt::Block);
        }
        self.parse_expr_stmt()
    }

    fn parse_export(&mut self) -> Result<Stmt, NoriError> {
        let start = self.bump().span;
        self.expect_keyword(Keyword::Default, "expected `default` after `export`")?;
        if self.at_keyword(Keyword::Function) {
            return self
                .parse_function_with_start(start)
                .map(Stmt::ExportDefaultFunction);
        }
        if self.at_keyword(Keyword::Async)
            && self.peek_next_kind() == Some(TokenKind::Keyword(Keyword::Function))
        {
            self.bump();
            self.expect_keyword(Keyword::Function, "expected `function`")?;
            let async_token = Some(start);
            let name = if self.at(TokenKind::Ident) {
                Some(self.bump().lexeme)
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
            if self.matches(TokenKind::Colon) {
                self.skip_type_until(&[TokenKind::LeftBrace]);
            }
            let body = self.parse_block()?;
            let span = Span {
                start: start.start,
                end: body.span.end,
                line: start.line,
                column: start.column,
            };
            return Ok(Stmt::ExportDefaultFunction(FunctionDecl {
                name,
                params,
                body,
                async_token,
                decorators: Vec::new(),
                span,
            }));
        }
        let expr = self.parse_expression_until_statement_end()?;
        self.consume_optional_semicolon();
        Ok(Stmt::ExportDefaultExpr(expr))
    }

    fn parse_class(&mut self) -> Result<ClassDecl, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::Ident, "expected class name")?;
        let name = self.previous().lexeme.clone();

        let extends = if self.at_keyword(Keyword::Extends) {
            self.bump();
            self.expect(TokenKind::Ident, "expected parent class name")?;
            Some(self.previous().lexeme.clone())
        } else {
            None
        };

        let body = self.parse_block()?;
        let span = Span {
            start: start.start,
            end: body.span.end,
            line: start.line,
            column: start.column,
        };

        Ok(ClassDecl {
            name,
            extends,
            body: body.body,
            span,
        })
    }

    fn parse_decorated(&mut self) -> Result<FunctionDecl, NoriError> {
        let mut decorators = Vec::new();
        while self.matches(TokenKind::At) {
            decorators.push(self.previous().span);
            self.expect(TokenKind::Ident, "expected decorator name")?;
        }
        let func_start = self.peek().span;
        let mut func = self.parse_function_with_start(func_start)?;
        func.decorators = decorators;
        Ok(func)
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, NoriError> {
        let start = self.peek().span;
        self.parse_function_with_start(start)
    }

    fn parse_async_function(&mut self) -> Result<Stmt, NoriError> {
        let start = self.bump().span;
        let async_token = Some(start);
        self.expect_keyword(Keyword::Function, "expected `function`")?;
        let name = if self.at(TokenKind::Ident) {
            Some(self.bump().lexeme)
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
        if self.matches(TokenKind::Colon) {
            self.skip_type_until(&[TokenKind::LeftBrace]);
        }
        let body = self.parse_block()?;
        let span = Span {
            start: start.start,
            end: body.span.end,
            line: start.line,
            column: start.column,
        };
        Ok(Stmt::Function(FunctionDecl {
            name,
            params,
            body,
            async_token,
            decorators: Vec::new(),
            span,
        }))
    }

    fn parse_try(&mut self) -> Result<Stmt, NoriError> {
        let try_start = self.bump().span;
        let body = self.parse_block()?;
        let mut catch_param = None;
        let mut catch_body = BlockStmt {
            body: Vec::new(),
            span: try_start,
        };
        if self.at_keyword(Keyword::Catch) {
            self.bump();
            if self.at(TokenKind::LeftParen) {
                self.bump();
                if self.at(TokenKind::Ident) {
                    catch_param = Some(self.bump().lexeme);
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
            span: Span {
                start: try_start.start,
                end: self.previous().span.end,
                line: try_start.line,
                column: try_start.column,
            },
        }))
    }

    fn parse_for(&mut self) -> Result<Stmt, NoriError> {
        let for_start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `for`")?;

        let var_kind = if self.at_keyword(Keyword::Const) {
            self.bump();
            VarKind::Const
        } else if self.at_keyword(Keyword::Let) {
            self.bump();
            VarKind::Let
        } else if self.at_keyword(Keyword::Var) {
            self.bump();
            VarKind::Var
        } else {
            return Err(self.error_here("expected variable keyword in for loop"));
        };

        self.expect(TokenKind::Ident, "expected variable name in for loop")?;
        let name = self.previous().lexeme.clone();

        let is_of = if self.at_keyword(Keyword::Of) {
            self.bump();
            true
        } else if self.at_keyword(Keyword::In) {
            self.bump();
            false
        } else {
            return Err(self.error_here("expected `in` or `of` in for loop"));
        };
        let iterable = self.parse_expression_until_statement_end()?;

        self.expect(TokenKind::RightParen, "expected `)` after for loop")?;
        let body = self.parse_block()?;
        let body_span = body.span;

        Ok(Stmt::For(ForStmt {
            variable: var_kind,
            name,
            iterable,
            is_of,
            body,
            span: Span {
                start: for_start.start,
                end: body_span.end,
                line: for_start.line,
                column: for_start.column,
            },
        }))
    }

    fn parse_function_with_start(&mut self, start: Span) -> Result<FunctionDecl, NoriError> {
        let async_token = if self.at_keyword(Keyword::Async) {
            let tok = self.bump();
            Some(tok.span)
        } else {
            None
        };
        self.expect_keyword(Keyword::Function, "expected `function`")?;
        let name = if self.at(TokenKind::Ident) {
            Some(self.bump().lexeme)
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
        if self.matches(TokenKind::Colon) {
            self.skip_type_until(&[TokenKind::LeftBrace]);
        }
        let body = self.parse_block()?;
        let span = Span {
            start: start.start,
            end: body.span.end,
            line: start.line,
            column: start.column,
        };
        Ok(FunctionDecl {
            name,
            params,
            body,
            async_token,
            decorators: Vec::new(),
            span,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, NoriError> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.matches(TokenKind::Ellipsis);
            let name = if self.at(TokenKind::Ident) {
                self.bump().lexeme
            } else {
                return Err(self.error_here("expected parameter name"));
            };
            if self.matches(TokenKind::Colon) {
                self.skip_type_until(&[TokenKind::Comma, TokenKind::Eq, TokenKind::RightParen]);
            }
            let default = if self.matches(TokenKind::Eq) {
                Some(self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?)
            } else {
                None
            };
            params.push(Param { name, default });
            if !self.matches(TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_block(&mut self) -> Result<BlockStmt, NoriError> {
        let start = self.expect(TokenKind::LeftBrace, "expected block")?.span;
        let mut body = Vec::new();
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

    fn parse_var(&mut self) -> Result<VarDecl, NoriError> {
        let kind_token = self.bump();
        let kind = match kind_token.kind {
            TokenKind::Keyword(Keyword::Const) => VarKind::Const,
            TokenKind::Keyword(Keyword::Let) => VarKind::Let,
            TokenKind::Keyword(Keyword::Var) => VarKind::Var,
            _ => unreachable!("caller checked keyword"),
        };
        let mut declarators = Vec::new();
        loop {
            let start = self.peek().span;
            let (name, pattern) =
                if self.at(TokenKind::LeftBracket) || self.at(TokenKind::LeftBrace) {
                    let pattern = self.parse_destructuring_pattern()?;
                    (String::new(), Some(pattern))
                } else {
                    let name_token = self.expect(TokenKind::Ident, "expected variable name")?;
                    (name_token.lexeme, None)
                };
            if name.is_empty() && self.peek().kind != TokenKind::Eq {
                return Err(self.error_here("expected variable name or pattern"));
            }
            if self.matches(TokenKind::Colon) {
                self.skip_type_until(&[TokenKind::Eq, TokenKind::Comma, TokenKind::Semicolon]);
            }
            let init = if self.matches(TokenKind::Eq) {
                Some(self.parse_expression_until(&[TokenKind::Comma, TokenKind::Semicolon])?)
            } else {
                None
            };
            let end = init.as_ref().map_or(start, |expr| expr.span);
            declarators.push(VarDeclarator {
                name,
                pattern,
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

    fn parse_destructuring_pattern(&mut self) -> Result<DestructuringPattern, NoriError> {
        let start = self.peek().span;
        if self.at(TokenKind::LeftBracket) {
            self.bump();
            let mut names = Vec::new();
            while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::Ident) {
                    names.push(self.bump().lexeme);
                } else if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RightBracket, "expected `]` after array pattern")?;
            let end = self.previous().span;
            Ok(DestructuringPattern {
                kind: DestructuringKind::Array(names, join_span(start, end)),
                span: join_span(start, end),
            })
        } else if self.at(TokenKind::LeftBrace) {
            self.bump();
            let mut props = Vec::new();
            while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::Ident) {
                    let name = self.bump().lexeme;
                    let default = if self.matches(TokenKind::Eq) {
                        Some(self.bump().lexeme)
                    } else {
                        None
                    };
                    props.push((name, default));
                } else if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RightBrace, "expected `}` after object pattern")?;
            let end = self.previous().span;
            Ok(DestructuringPattern {
                kind: DestructuringKind::Object(props, join_span(start, end)),
                span: join_span(start, end),
            })
        } else {
            Err(self.error_here("expected `[` or `{` for destructuring pattern"))
        }
    }

    fn parse_return(&mut self) -> Result<Stmt, NoriError> {
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

    fn parse_if(&mut self) -> Result<Stmt, NoriError> {
        let start = self.bump().span;
        self.expect(TokenKind::LeftParen, "expected `(` after `if`")?;
        let condition = self.parse_expression_until(&[TokenKind::RightParen])?;
        self.expect(TokenKind::RightParen, "expected `)` after if condition")?;
        let consequent = Box::new(self.parse_stmt()?);
        let alternate = if self.matches_keyword(Keyword::Else) {
            Some(Box::new(self.parse_stmt()?))
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
            span: Span { end, ..start },
        }))
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, NoriError> {
        let expr = self.parse_expression_until_statement_end()?;
        self.consume_optional_semicolon();
        Ok(Stmt::Expr(expr))
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

    fn parse_expression_until_statement_end(&mut self) -> Result<Expr, NoriError> {
        self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace])
    }

    fn parse_expression_until(&mut self, stop: &[TokenKind]) -> Result<Expr, NoriError> {
        self.parse_expression_until_bp(stop, 0)
    }

    fn parse_expression_until_bp(
        &mut self,
        stop: &[TokenKind],
        min_bp: u8,
    ) -> Result<Expr, NoriError> {
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
                        test: Box::new(lhs),
                        consequent: Box::new(consequent),
                        alternate: Box::new(alternate),
                    },
                    span,
                };
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
                        left: Box::new(lhs),
                        op: op.to_string(),
                        right: Box::new(rhs),
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
                    left: Box::new(lhs),
                    op: op.to_string(),
                    right: Box::new(rhs),
                },
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self, stop: &[TokenKind]) -> Result<Expr, NoriError> {
        if self.at_any(stop) || self.at(TokenKind::Eof) {
            return Err(self.error_here("expected expression"));
        }

        let token = self.bump();
        let mut expr = match token.kind {
            TokenKind::Ident => {
                if self.at(TokenKind::Arrow) {
                    self.bump();
                    let body = self.parse_expression_until(stop)?;
                    let span = join_span(token.span, body.span);
                    Expr {
                        kind: ExprKind::Arrow {
                            params: vec![token.lexeme],
                            body: Box::new(body),
                        },
                        span,
                    }
                } else {
                    Expr {
                        kind: ExprKind::Ident(token.lexeme),
                        span: token.span,
                    }
                }
            }
            TokenKind::Number => Expr {
                kind: ExprKind::Number(token.lexeme),
                span: token.span,
            },
            TokenKind::String => Expr {
                kind: ExprKind::String(token.lexeme),
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
            TokenKind::Bang | TokenKind::Minus | TokenKind::Plus => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Unary {
                        op: token.lexeme,
                        expr: Box::new(rhs),
                    },
                    span,
                }
            }
            TokenKind::LeftParen => {
                if self.looks_like_arrow_params()? {
                    let params = self.collect_arrow_params()?;
                    self.expect(TokenKind::RightParen, "expected `)` after arrow parameters")?;
                    self.expect(TokenKind::Arrow, "expected `=>` after arrow parameters")?;
                    let body = self.parse_expression_until(stop)?;
                    let span = join_span(token.span, body.span);
                    Expr {
                        kind: ExprKind::Arrow {
                            params,
                            body: Box::new(body),
                        },
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
            TokenKind::LeftBrace => self.parse_object_raw(token.span)?,
            TokenKind::Less if self.syntax.markup => self.parse_markup_after_less(token.span)?,
            TokenKind::Keyword(Keyword::Await) => {
                let rhs = self.parse_expression_until_bp(stop, 15)?;
                let span = join_span(token.span, rhs.span);
                Expr {
                    kind: ExprKind::Await(Box::new(rhs)),
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

    fn parse_postfix(&mut self, mut expr: Expr, stop: &[TokenKind]) -> Result<Expr, NoriError> {
        loop {
            if self.at_any(stop) || self.at(TokenKind::Eof) {
                break;
            }
            if self.matches(TokenKind::Dot) {
                let prop = self.expect(TokenKind::Ident, "expected property name after `.`")?;
                let span = join_span(expr.span, prop.span);
                expr = Expr {
                    kind: ExprKind::Member {
                        object: Box::new(expr),
                        property: prop.lexeme,
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
                        object: Box::new(expr),
                        index: Box::new(index),
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
                        callee: Box::new(expr),
                        args,
                    },
                    span,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, NoriError> {
        let mut args = Vec::new();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Ellipsis) {
                let expr =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightParen])?;
                let span = expr.span;
                args.push(Expr {
                    kind: ExprKind::Spread {
                        expr: Box::new(expr),
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

    fn parse_array(&mut self, start: Span, _stop: &[TokenKind]) -> Result<Expr, NoriError> {
        let mut items = Vec::new();
        while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
            if self.matches(TokenKind::Ellipsis) {
                let expr =
                    self.parse_expression_until(&[TokenKind::Comma, TokenKind::RightBracket])?;
                let span = expr.span;
                items.push(Expr {
                    kind: ExprKind::Spread {
                        expr: Box::new(expr),
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

    fn parse_object_raw(&mut self, start: Span) -> Result<Expr, NoriError> {
        self.skip_until_matching(TokenKind::LeftBrace, TokenKind::RightBrace)?;
        let end = self.previous().span;
        Ok(Expr {
            kind: ExprKind::Object,
            span: join_span(start, end),
        })
    }

    fn parse_markup_after_less(&mut self, start: Span) -> Result<Expr, NoriError> {
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

    fn parse_markup_element_after_less(&mut self, start: Span) -> Result<MarkupElement, NoriError> {
        let name = self.parse_markup_name()?;
        let mut attributes = Vec::new();
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
                children: Vec::new(),
                self_closing: true,
                span: join_span(start, end),
            });
        }
        self.expect(TokenKind::Greater, "expected `>` after markup opening tag")?;
        let children = self.parse_markup_children(Some(&name))?;
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
    ) -> Result<Vec<MarkupChild>, NoriError> {
        let mut children = Vec::new();
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
                if !token.lexeme.trim().is_empty() {
                    children.push(MarkupChild::Text(token.lexeme, token.span));
                }
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

    fn parse_markup_attribute(&mut self) -> Result<MarkupAttribute, NoriError> {
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
                    kind: ExprKind::String(token.lexeme),
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
            name: name_token.lexeme,
            value,
            span,
        })
    }

    fn parse_markup_name(&mut self) -> Result<String, NoriError> {
        let mut name = self.expect_markup_ident("expected markup tag name")?.lexeme;
        while self.matches(TokenKind::Dot) {
            name.push('.');
            name.push_str(
                &self
                    .expect_markup_ident("expected markup member name")?
                    .lexeme,
            );
        }
        Ok(name)
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

    fn parse_raw_expression_from(&mut self, start: Span, stop: &[TokenKind]) -> Expr {
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

    fn collect_arrow_params(&mut self) -> Result<Vec<String>, NoriError> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.matches(TokenKind::Ellipsis);
            let name = self
                .expect(TokenKind::Ident, "expected arrow parameter")?
                .lexeme;
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
            message, self.filename, token.span.line, token.span.column
        );
        NoriError::Parse {
            message,
            span: source_span(token.span.start, token.span.end),
        }
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Import(raw) | Stmt::TypeOnly(raw) | Stmt::Raw(raw) => raw.span,
        Stmt::Var(var) => var.span,
        Stmt::Function(function) | Stmt::ExportDefaultFunction(function) => function.span,
        Stmt::ExportDefaultExpr(expr) | Stmt::Expr(expr) => expr.span,
        Stmt::Return(_, span) => *span,
        Stmt::Block(block) => block.span,
        Stmt::If(stmt) => stmt.span,
        Stmt::Class(class) => class.span,
        Stmt::Try(stmt) => stmt.span,
        Stmt::For(stmt) => stmt.span,
    }
}

fn join_span(start: Span, end: Span) -> Span {
    Span {
        start: start.start,
        end: end.end,
        line: start.line,
        column: start.column,
    }
}

fn assignment_op(kind: TokenKind) -> Option<&'static str> {
    Some(match kind {
        TokenKind::Eq => "=",
        TokenKind::PlusEq => "+=",
        TokenKind::MinusEq => "-=",
        TokenKind::StarEq => "*=",
        TokenKind::SlashEq => "/=",
        _ => return None,
    })
}

fn infix_binding_power(kind: TokenKind) -> Option<(u8, u8, &'static str)> {
    Some(match kind {
        TokenKind::OrOr => (3, 4, "||"),
        TokenKind::AndAnd => (5, 6, "&&"),
        TokenKind::EqEq | TokenKind::BangEq => {
            (7, 8, if kind == TokenKind::EqEq { "==" } else { "!=" })
        }
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
        _ => return None,
    })
}
