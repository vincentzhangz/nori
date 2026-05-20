use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use clap::{Parser as ClapParser, Subcommand};
use miette::{IntoDiagnostic, Result};
use nori::{CompileOptions, compile_file, lexer::lex, parse_source};

#[derive(Debug, ClapParser)]
#[command(
    name = "nori",
    version,
    about = "A from-scratch Rust UI compiler for .nori files"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compile a Nori source file to JavaScript
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "@nori/core")]
        runtime_import: String,
        #[arg(long)]
        sourcemap: bool,
    },
    /// Watch a file and recompile when it changes
    Watch {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "@nori/core")]
        runtime_import: String,
    },
    /// Print the token stream for a Nori source file
    Lex { input: PathBuf },
    /// Print the parsed AST for a Nori source file
    Parse { input: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compile {
            input,
            output,
            runtime_import,
            sourcemap,
        } => compile_command(&input, output.as_deref(), &runtime_import, sourcemap),
        Command::Watch {
            input,
            output,
            runtime_import,
        } => watch_command(&input, output.as_deref(), &runtime_import),
        Command::Lex { input } => lex_command(&input),
        Command::Parse { input } => parse_command(&input),
    }
}

fn compile_command(
    input: &Path,
    output: Option<&Path>,
    runtime_import: &str,
    sourcemap: bool,
) -> Result<()> {
    let compiled = compile_file(
        input,
        CompileOptions {
            filename: input.display().to_string(),
            runtime_import: runtime_import.to_string(),
            source_map: sourcemap,
        },
    )?;

    if let Some(output) = output {
        let output_path = output_path(input, output);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }
        fs::write(&output_path, compiled.code).into_diagnostic()?;
        println!("{}", output_path.display());
    } else {
        println!("{}", compiled.code);
    }
    Ok(())
}

fn watch_command(input: &Path, output: Option<&Path>, runtime_import: &str) -> Result<()> {
    let mut last_modified = modified(input)?;
    compile_command(input, output, runtime_import, false)?;
    loop {
        thread::sleep(Duration::from_millis(500));
        let modified = modified(input)?;
        if modified > last_modified {
            last_modified = modified;
            compile_command(input, output, runtime_import, false)?;
        }
    }
}

fn lex_command(input: &Path) -> Result<()> {
    let source = fs::read_to_string(input).into_diagnostic()?;
    for token in lex(&source)? {
        println!(
            "{:?} {:?} @ {}:{}",
            token.kind, token.lexeme, token.span.line, token.span.column
        );
    }
    Ok(())
}

fn parse_command(input: &Path) -> Result<()> {
    let source = fs::read_to_string(input).into_diagnostic()?;
    let program = parse_source(&source, input.display().to_string())?;
    println!("{program:#?}");
    Ok(())
}

fn output_path(input: &Path, output: &Path) -> PathBuf {
    if output.extension().is_some() {
        return output.to_path_buf();
    }
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output");
    output.join(format!("{stem}.js"))
}

fn modified(path: &Path) -> Result<SystemTime> {
    fs::metadata(path)
        .into_diagnostic()?
        .modified()
        .into_diagnostic()
}
