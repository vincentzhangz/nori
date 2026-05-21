use nori_lexer::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserCheckpoint {
    pos: usize,
}

#[derive(Debug, Clone)]
pub struct TokenCursor {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenCursor {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn checkpoint(&self) -> ParserCheckpoint {
        ParserCheckpoint { pos: self.pos }
    }

    pub fn rewind(&mut self, checkpoint: ParserCheckpoint) {
        self.pos = checkpoint.pos;
    }

    pub fn bump(&mut self) -> Token {
        let token = self.peek().clone();
        self.pos += 1;
        token
    }

    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    pub fn peek_next_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.pos + 1).map(|token| token.kind)
    }
}