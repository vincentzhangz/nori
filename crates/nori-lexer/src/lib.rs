use nori_ast::Span;
use nori_diagnostic::{NoriError, span as source_span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Import,
    Export,
    Default,
    From,
    Const,
    Let,
    Var,
    Function,
    Return,
    If,
    Else,
    Try,
    Catch,
    Finally,
    For,
    While,
    Do,
    Break,
    Continue,
    In,
    Of,
    Yield,
    Async,
    Await,
    Type,
    Interface,
    Class,
    Extends,
    True,
    False,
    Null,
    New,
    This,
    Super,
    Delete,
    Void,
    Typeof,
    Instanceof,
    Switch,
    Case,
    Throw,
    Debugger,
    With,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Number,
    BigInt,
    String,
    MarkupText,
    RegExp,
    Keyword(Keyword),
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Less,
    Greater,
    Slash,
    SlashGreater,
    Dot,
    DotDotDot,
    Comma,
    Colon,
    Semicolon,
    Question,
    QuestionDot,
    QuestionQuestion,
    QuestionQuestionEq,
    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    StarStar,
    StarStarEq,
    Percent,
    Bang,
    BangEq,
    BangEqEq,
    At,
    Dollar,
    BackTick,
    Hash,
    Tilde,
    Caret,
    CaretEq,
    Eq,
    EqEq,
    EqEqEq,
    LessEq,
    GreaterEq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AndAnd,
    AndAndEq,
    Ampersand,
    AmpersandEq,
    OrOr,
    OrOrEq,
    Pipe,
    PipeEq,
    ShiftLeft,
    ShiftLeftEq,
    ShiftRight,
    ShiftRightEq,
    ShiftRightUnsigned,
    ShiftRightUnsignedEq,
    Arrow,
    DotDot,
    Ellipsis,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    MarkupTag,
    MarkupText,
    MarkupExpr { return_to_tag: bool, depth: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexContext {
    Normal,
    ExpectRegex,
}

pub fn lex(source: &str) -> Result<Vec<Token>, NoriError> {
    Lexer::new(source, LexContext::Normal).lex()
}

pub fn lex_with_context(source: &str, context: LexContext) -> Result<Vec<Token>, NoriError> {
    Lexer::new(source, context).lex()
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    column: usize,
    mode: Mode,
    markup_depth: usize,
    pending_closing_tag: bool,
    context: LexContext,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, context: LexContext) -> Self {
        Self {
            source,
            pos: 0,
            line: 1,
            column: 1,
            mode: Mode::Normal,
            markup_depth: 0,
            pending_closing_tag: false,
            context,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, NoriError> {
        if self.source.starts_with("#!") {
            while !matches!(self.peek(), None | Some('\n')) {
                self.advance();
            }
        }
        let mut tokens = Vec::new();
        while !self.is_at_end() {
            if matches!(self.mode, Mode::MarkupText) {
                if let Some(token) = self.lex_markup_text() {
                    tokens.push(token);
                }
                continue;
            }

            self.skip_whitespace_and_comments()?;
            if self.is_at_end() {
                break;
            }

            tokens.push(self.next_token()?);
        }

        tokens.push(self.make_token(TokenKind::Eof, self.pos, self.pos));
        Ok(tokens)
    }

    #[allow(clippy::too_many_lines)]
    fn next_token(&mut self) -> Result<Token, NoriError> {
        let start = self.pos;
        let ch = self.advance().expect("checked by caller");

        let kind = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            '{' => {
                if matches!(self.mode, Mode::MarkupTag) {
                    self.mode = Mode::MarkupExpr {
                        return_to_tag: true,
                        depth: 1,
                    };
                } else if let Mode::MarkupExpr { depth, .. } = &mut self.mode {
                    *depth += 1;
                }
                TokenKind::LeftBrace
            }
            '}' => {
                if let Mode::MarkupExpr {
                    return_to_tag,
                    depth,
                } = &mut self.mode
                {
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        self.mode = if *return_to_tag {
                            Mode::MarkupTag
                        } else {
                            Mode::MarkupText
                        };
                    }
                }
                TokenKind::RightBrace
            }
            '<' => {
                if self.should_start_markup(start) {
                    self.mode = Mode::MarkupTag;
                    self.pending_closing_tag = self.peek() == Some('/');
                    TokenKind::Less
                } else if self.matches('<') {
                    if self.matches('=') {
                        TokenKind::ShiftLeftEq
                    } else {
                        TokenKind::ShiftLeft
                    }
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if matches!(self.mode, Mode::MarkupTag) {
                    if self.pending_closing_tag {
                        self.markup_depth = self.markup_depth.saturating_sub(1);
                    } else {
                        self.markup_depth += 1;
                    }
                    self.pending_closing_tag = false;
                    self.mode = if self.markup_depth == 0 {
                        Mode::Normal
                    } else {
                        Mode::MarkupText
                    };
                    TokenKind::Greater
                } else if self.matches('>') {
                    if self.matches('>') {
                        if self.matches('=') {
                            TokenKind::ShiftRightUnsignedEq
                        } else {
                            TokenKind::ShiftRightUnsigned
                        }
                    } else if self.matches('=') {
                        TokenKind::ShiftRightEq
                    } else {
                        TokenKind::ShiftRight
                    }
                } else {
                    TokenKind::Greater
                }
            }
            '/' => {
                if self.matches('>') {
                    if matches!(self.mode, Mode::MarkupTag) {
                        self.pending_closing_tag = false;
                        self.mode = if self.markup_depth == 0 {
                            Mode::Normal
                        } else {
                            Mode::MarkupText
                        };
                    }
                    TokenKind::SlashGreater
                } else if self.matches('=') {
                    TokenKind::SlashEq
                } else if self.context == LexContext::ExpectRegex
                    && !matches!(
                        self.mode,
                        Mode::MarkupTag | Mode::MarkupText | Mode::MarkupExpr { .. }
                    )
                {
                    return self.lex_regex(start);
                } else {
                    TokenKind::Slash
                }
            }
            '.' => {
                if self.matches('.') {
                    if self.matches('.') {
                        TokenKind::Ellipsis
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '?' => {
                if self.matches('?') {
                    if self.matches('=') {
                        TokenKind::QuestionQuestionEq
                    } else {
                        TokenKind::QuestionQuestion
                    }
                } else if self.matches('.') {
                    TokenKind::QuestionDot
                } else {
                    TokenKind::Question
                }
            }
            '+' => {
                if self.matches('+') {
                    TokenKind::PlusPlus
                } else if self.matches('=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.matches('-') {
                    TokenKind::MinusMinus
                } else if self.matches('=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.matches('*') {
                    if self.matches('=') {
                        TokenKind::StarStarEq
                    } else {
                        TokenKind::StarStar
                    }
                } else if self.matches('=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '%' => TokenKind::Percent,
            '~' => TokenKind::Tilde,
            '^' => {
                if self.matches('=') {
                    TokenKind::CaretEq
                } else {
                    TokenKind::Caret
                }
            }
            '$' => {
                if let Some(next) = self.peek() {
                    if next.is_ascii_alphabetic() || next == '_' || next == '$' {
                        return Ok(self.lex_ident(start));
                    }
                }
                TokenKind::Dollar
            }
            '!' => {
                if self.matches('=') {
                    if self.matches('=') {
                        TokenKind::BangEqEq
                    } else {
                        TokenKind::BangEq
                    }
                } else {
                    TokenKind::Bang
                }
            }
            '@' => TokenKind::At,
            '#' => TokenKind::Hash,
            '`' => TokenKind::BackTick,
            '=' => {
                if self.matches('>') {
                    TokenKind::Arrow
                } else if self.matches('=') {
                    if self.matches('=') {
                        TokenKind::EqEqEq
                    } else {
                        TokenKind::EqEq
                    }
                } else {
                    TokenKind::Eq
                }
            }
            '&' => {
                if self.matches('&') {
                    if self.matches('=') {
                        TokenKind::AndAndEq
                    } else {
                        TokenKind::AndAnd
                    }
                } else if self.matches('=') {
                    TokenKind::AmpersandEq
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.matches('|') {
                    if self.matches('=') {
                        TokenKind::OrOrEq
                    } else {
                        TokenKind::OrOr
                    }
                } else if self.matches('=') {
                    TokenKind::PipeEq
                } else {
                    TokenKind::Pipe
                }
            }
            '\'' | '"' => return self.lex_string(start, ch),
            c if c.is_ascii_digit() => return Ok(self.lex_number(start)),
            c if is_ident_start(c) => return Ok(self.lex_ident(start)),
            _ => {
                return Err(NoriError::Lex {
                    message: format!("unexpected character `{ch}`"),
                    span: source_span(start, self.pos),
                });
            }
        };

        Ok(self.make_token(kind, start, self.pos))
    }

    fn lex_markup_text(&mut self) -> Option<Token> {
        if self.is_at_end() {
            return None;
        }

        let start = self.pos;
        match self.peek().expect("checked above") {
            '<' => {
                self.advance();
                self.mode = Mode::MarkupTag;
                self.pending_closing_tag = self.peek() == Some('/');
                Some(self.make_token(TokenKind::Less, start, self.pos))
            }
            '{' => {
                self.advance();
                self.mode = Mode::MarkupExpr {
                    return_to_tag: false,
                    depth: 1,
                };
                Some(self.make_token(TokenKind::LeftBrace, start, self.pos))
            }
            _ => {
                while let Some(ch) = self.peek() {
                    if ch == '<' || ch == '{' {
                        break;
                    }
                    self.advance();
                }
                Some(self.make_token(TokenKind::MarkupText, start, self.pos))
            }
        }
    }

    fn lex_string(&mut self, start: usize, quote: char) -> Result<Token, NoriError> {
        if quote == '`' {
            return self.lex_template_literal(start);
        }
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                return Ok(self.make_token(TokenKind::String, start, self.pos));
            }
            if ch == '\\' {
                self.advance();
                if !self.is_at_end() {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        Err(NoriError::Lex {
            message: "unterminated string literal".to_string(),
            span: source_span(start, self.pos),
        })
    }

    fn lex_template_literal(&mut self, start: usize) -> Result<Token, NoriError> {
        let mut brace_depth = 0;
        while let Some(ch) = self.peek() {
            match ch {
                '`' => {
                    self.advance();
                    return Ok(self.make_token(TokenKind::String, start, self.pos));
                }
                '$' if self.peek_next() == Some('{') => {
                    self.advance();
                    self.advance();
                    brace_depth += 1;
                }
                '}' if brace_depth > 0 => {
                    self.advance();
                    brace_depth -= 1;
                }
                '\\' => {
                    self.advance();
                    if !self.is_at_end() {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        Err(NoriError::Lex {
            message: "unterminated template literal".to_string(),
            span: source_span(start, self.pos),
        })
    }

    fn lex_regex(&mut self, start: usize) -> Result<Token, NoriError> {
        let mut in_class = false;
        loop {
            match self.peek() {
                None => {
                    return Err(NoriError::Lex {
                        message: "unterminated regex literal".to_string(),
                        span: source_span(start, self.pos),
                    });
                }
                Some('\\') => {
                    self.advance();
                    if !self.is_at_end() {
                        self.advance();
                    }
                }
                Some('[') => {
                    self.advance();
                    in_class = true;
                }
                Some(']') => {
                    self.advance();
                    in_class = false;
                }
                Some('/') if !in_class => {
                    self.advance();
                    break;
                }
                Some('\n') if !in_class => {
                    return Err(NoriError::Lex {
                        message: "unterminated regex literal".to_string(),
                        span: source_span(start, self.pos),
                    });
                }
                _ => {
                    self.advance();
                }
            }
        }
        while matches!(self.peek(), Some(ch) if ch.is_ascii_alphabetic()) {
            self.advance();
        }
        Ok(self.make_token(TokenKind::RegExp, start, self.pos))
    }

    fn lex_number(&mut self, start: usize) -> Token {
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '_') {
            self.advance();
        }
        if self.peek() == Some('.')
            && self
                .source
                .get(self.pos + 1..)
                .and_then(|tail| tail.chars().next())
                .is_some_and(|ch| ch.is_ascii_digit() || ch == '_')
        {
            self.advance();
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '_') {
                self.advance();
            }
        }
        if self.peek() == Some('n') {
            self.advance();
            return self.make_token(TokenKind::BigInt, start, self.pos);
        }
        self.make_token(TokenKind::Number, start, self.pos)
    }

    fn lex_ident(&mut self, start: usize) -> Token {
        while matches!(self.peek(), Some(ch) if is_ident_continue(ch)) {
            self.advance();
        }
        let text = &self.source[start..self.pos];
        let kind = keyword(text).map_or(TokenKind::Ident, TokenKind::Keyword);
        self.make_token(kind, start, self.pos)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), NoriError> {
        loop {
            while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
                self.advance();
            }
            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.advance();
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                let start = self.pos;
                self.advance();
                self.advance();
                while !(self.peek() == Some('*') && self.peek_next() == Some('/')) {
                    if self.is_at_end() {
                        return Err(NoriError::Lex {
                            message: "unterminated block comment".to_string(),
                            span: source_span(start, self.pos),
                        });
                    }
                    self.advance();
                }
                self.advance();
                self.advance();
                continue;
            }
            break;
        }
        Ok(())
    }

    fn should_start_markup(&self, start: usize) -> bool {
        if matches!(self.mode, Mode::MarkupExpr { .. }) {
            return false;
        }
        if matches!(self.mode, Mode::MarkupText | Mode::MarkupTag) {
            return true;
        }
        if self.source[..start].trim_end().ends_with("return") {
            return true;
        }
        if self.follows_type_angle_callee(start) {
            return false;
        }
        let next = self.peek();
        next.is_some_and(|ch| ch.is_ascii_alphabetic() || matches!(ch, '/' | '>'))
    }

    fn follows_type_angle_callee(&self, start: usize) -> bool {
        self.source[..start]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_some_and(|ch| is_ident_continue(ch) || matches!(ch, ')' | ']' | '>'))
    }

    fn make_token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        let before = &self.source[..start];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = before
            .rsplit_once('\n')
            .map_or(before.len() + 1, |(_, tail)| tail.len() + 1);

        Token {
            kind,
            lexeme: self.source[start..end].to_string(),
            span: Span {
                start,
                end,
                line,
                column,
            },
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn matches(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos..)?.chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source.get(self.pos..)?.chars();
        chars.next()?;
        chars.next()
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || matches!(ch, '_' | '$')
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn keyword(text: &str) -> Option<Keyword> {
    Some(match text {
        "import" => Keyword::Import,
        "export" => Keyword::Export,
        "default" => Keyword::Default,
        "from" => Keyword::From,
        "const" => Keyword::Const,
        "let" => Keyword::Let,
        "var" => Keyword::Var,
        "function" => Keyword::Function,
        "return" => Keyword::Return,
        "if" => Keyword::If,
        "else" => Keyword::Else,
        "try" => Keyword::Try,
        "catch" => Keyword::Catch,
        "finally" => Keyword::Finally,
        "for" => Keyword::For,
        "while" => Keyword::While,
        "do" => Keyword::Do,
        "break" => Keyword::Break,
        "continue" => Keyword::Continue,
        "in" => Keyword::In,
        "of" => Keyword::Of,
        "yield" => Keyword::Yield,
        "async" => Keyword::Async,
        "await" => Keyword::Await,
        "type" => Keyword::Type,
        "interface" => Keyword::Interface,
        "class" => Keyword::Class,
        "extends" => Keyword::Extends,
        "true" => Keyword::True,
        "false" => Keyword::False,
        "null" => Keyword::Null,
        "new" => Keyword::New,
        "this" => Keyword::This,
        "super" => Keyword::Super,
        "delete" => Keyword::Delete,
        "void" => Keyword::Void,
        "typeof" => Keyword::Typeof,
        "instanceof" => Keyword::Instanceof,
        "switch" => Keyword::Switch,
        "case" => Keyword::Case,
        "throw" => Keyword::Throw,
        "debugger" => Keyword::Debugger,
        "with" => Keyword::With,
        _ => return None,
    })
}
