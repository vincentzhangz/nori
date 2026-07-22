use std::{fs, path::Path};

pub use nori_allocator::Allocator;
pub use nori_analyzer::Analysis;
pub use nori_ast::Program;
pub use nori_codegen::generate;
pub use nori_diagnostic::NoriError;
pub use nori_lexer::lex;
pub use nori_parser::{Parser, Syntax, parse_in};
pub use nori_checker::{CheckResult, GlobalTypeEnv, check, check_files, check_with_globals, lib_es5_globals};
pub use nori_semantic::{SemanticModel, build_semantic};
pub mod ast {
    pub use nori_ast::*;
}
pub mod lexer {
    pub use nori_lexer::{
        Keyword, LexContext, LexOutput, Token, TokenKind, Trivia, TriviaKind, lex,
        lex_with_context, lex_with_trivia,
    };
}
pub mod parser {
    pub use nori_parser::{Parser, Syntax, parse_in};
}

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub filename: String,
    pub runtime_import: String,
    /// When true, run the type checker and surface type diagnostics in
    /// [`CompileOutput::diagnostics`] (compile still emits JS).
    ///
    /// Defaults to `false` so emit stays permissive. Use `nori check` (or set
    /// this to `true`) to run the full M1–M8 checker on the production path.
    pub type_check: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            filename: "<anonymous>.nori".to_string(),
            runtime_import: "@nori/core".to_string(),
            type_check: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    pub code: String,
    pub diagnostics: Vec<String>,
}

pub fn parse_source<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    filename: impl Into<String>,
) -> Result<Program<'a>, NoriError> {
    let tokens = lex(source)?;
    parse_in(allocator, source, filename, tokens)
}

pub fn analyze_source(source: &str, filename: impl Into<String>) -> Result<Analysis, NoriError> {
    let allocator = Allocator::new();
    let program = parse_source(&allocator, source, filename)?;
    Ok(Analysis::from_program(source, &program))
}

pub fn compile_source(
    source: &str,
    mut options: CompileOptions,
) -> Result<CompileOutput, NoriError> {
    if options.filename.is_empty() {
        options.filename = "<anonymous>.nori".to_string();
    }

    let allocator = Allocator::new();
    let program = parse_source(&allocator, source, options.filename.clone())?;
    let analysis = Analysis::from_program(source, &program);
    let code = generate(source, &program, &analysis, &options.runtime_import);

    let mut diagnostics = analysis.diagnostics;
    if options.type_check {
        let checked = check(&program);
        for diag in checked.diagnostics {
            diagnostics.push(format!("{}: {}", diag.code, diag.message));
        }
    }

    Ok(CompileOutput { code, diagnostics })
}

pub fn compile_file(path: &Path, options: CompileOptions) -> Result<CompileOutput, NoriError> {
    let source = fs::read_to_string(path).map_err(|source| NoriError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut options = options;
    if options.filename == "<anonymous>.nori" {
        options.filename = path.display().to_string();
    }
    compile_source(&source, options)
}

pub fn check_source(
    source: &str,
    filename: impl Into<String>,
) -> Result<CheckResult, NoriError> {
    let allocator = Allocator::new();
    let program = parse_source(&allocator, source, filename)?;
    Ok(check(&program))
}
