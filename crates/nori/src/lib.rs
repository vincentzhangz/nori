use std::{fs, path::Path};

pub use nori_analyzer::Analysis;
pub use nori_ast::Program;
pub use nori_codegen::generate;
pub use nori_diagnostic::NoriError;
pub use nori_lexer::lex;
pub use nori_parser::{Parser, Syntax};
pub mod lexer {
    pub use nori_lexer::lex;
}
pub mod parser {
    pub use nori_parser::{Parser, Syntax};
}

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub filename: String,
    pub runtime_import: String,
    pub source_map: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            filename: "<anonymous>.nori".to_string(),
            runtime_import: "@nori/core".to_string(),
            source_map: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    pub code: String,
    pub map: Option<String>,
    pub diagnostics: Vec<String>,
}

pub fn parse_source(source: &str, filename: impl Into<String>) -> Result<Program, NoriError> {
    let filename = filename.into();
    let tokens = lex(source)?;
    Parser::new(source, filename, tokens).parse_program()
}

pub fn analyze_source(source: &str, filename: impl Into<String>) -> Result<Analysis, NoriError> {
    let program = parse_source(source, filename)?;
    Ok(Analysis::from_program(source, &program))
}

pub fn compile_source(
    source: &str,
    mut options: CompileOptions,
) -> Result<CompileOutput, NoriError> {
    if options.filename.is_empty() {
        options.filename = "<anonymous>.nori".to_string();
    }

    let program = parse_source(source, options.filename.clone())?;
    let analysis = Analysis::from_program(source, &program);
    let code = generate(source, &program, &analysis, &options.runtime_import);

    Ok(CompileOutput {
        code,
        map: options.source_map.then(|| "{}".to_string()),
        diagnostics: analysis.diagnostics,
    })
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
