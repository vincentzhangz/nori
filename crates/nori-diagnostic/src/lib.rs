use std::{io, path::PathBuf};

use miette::{Diagnostic as MietteDiagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: SourceSpan,
    pub severity: Severity,
    pub code: &'static str,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Error,
            code: "nori::parse",
        }
    }

    pub fn lex_error(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Error,
            code: "nori::lex",
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[derive(Debug, Error, MietteDiagnostic)]
pub enum NoriError {
    #[error("failed to read `{}`", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{message}")]
    #[diagnostic(code(nori::lex))]
    Lex {
        message: String,
        #[label("here")]
        span: SourceSpan,
    },

    #[error("{message}")]
    #[diagnostic(code(nori::parse))]
    Parse {
        message: String,
        #[label("here")]
        span: SourceSpan,
    },
}

impl From<Diagnostic> for NoriError {
    fn from(diag: Diagnostic) -> Self {
        match diag.code {
            "nori::lex" => NoriError::Lex {
                message: diag.message,
                span: diag.span,
            },
            _ => NoriError::Parse {
                message: diag.message,
                span: diag.span,
            },
        }
    }
}

pub fn span(start: usize, end: usize) -> SourceSpan {
    (start, end.saturating_sub(start).max(1)).into()
}
