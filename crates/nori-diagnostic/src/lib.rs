use std::{io, path::PathBuf};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
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

pub fn span(start: usize, end: usize) -> SourceSpan {
    (start, end.saturating_sub(start).max(1)).into()
}