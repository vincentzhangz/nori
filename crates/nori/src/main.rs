use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use clap::{Parser as ClapParser, Subcommand};
use miette::{IntoDiagnostic, Result};
use nori::{CompileOptions, analyze_source, parse_source};
use nori_lexer::lex;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

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
        stdin: bool,
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
            stdin,
        } => compile_command(&input, output.as_deref(), &runtime_import, stdin),
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
    stdin: bool,
) -> Result<()> {
    let source = if stdin {
        std::io::read_to_string(std::io::stdin()).into_diagnostic()?
    } else {
        fs::read_to_string(input).into_diagnostic()?
    };

    let compiled = nori::compile_source(
        &source,
        CompileOptions {
            filename: if stdin {
                "<stdin>.nori".to_string()
            } else {
                input.display().to_string()
            },
            runtime_import: runtime_import.to_string(),
        },
    )?;

    if let Some(output) = output {
        if !stdin {
            let output_path = output_path(input, output);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).into_diagnostic()?;
            }
            fs::write(&output_path, compiled.code).into_diagnostic()?;
            println!("{}", output_path.display());
        } else {
            println!("{}", compiled.code);
        }
    } else {
        println!("{}", compiled.code);
    }
    Ok(())
}

fn watch_command(input: &Path, output: Option<&Path>, runtime_import: &str) -> Result<()> {
    let base_dir = if input.is_file() {
        input.parent().unwrap_or(Path::new("."))
    } else {
        input
    };

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).into_diagnostic()?;
    watcher
        .watch(base_dir, RecursiveMode::Recursive)
        .into_diagnostic()?;

    compile_command(input, output, runtime_import, false)?;
    println!("Watching {} for changes...", base_dir.display());

    let mut tracked_files = HashSet::new();
    tracked_files.insert(input.canonicalize().unwrap_or_default());

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(notify::Event { kind, paths, .. })) => {
                if matches!(
                    kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    for path in &paths {
                        if path.extension().is_some_and(|ext| ext == "nori") {
                            let canonical = path.canonicalize().unwrap_or_default();
                            if tracked_files.contains(&canonical) || path == input {
                                compile_command(input, output, runtime_import, false)?;
                                emit_hmr_event(path);
                            }
                        }
                    }

                    let source = fs::read_to_string(input).into_diagnostic()?;
                    if let Ok(analysis) = analyze_source(&source, input.display().to_string()) {
                        for nori_import in &analysis.nori_imports {
                            let import_path = resolve_nori_import(nori_import, input);
                            let canonical = import_path.canonicalize().unwrap_or_default();
                            if !tracked_files.contains(&canonical) {
                                tracked_files.insert(canonical);
                            }
                        }
                    }
                }
            }
            Ok(Err(_)) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    Ok(())
}

fn resolve_nori_import(import_path: &str, source_file: &Path) -> PathBuf {
    let source_dir = source_file.parent().unwrap_or(Path::new("."));
    if import_path.starts_with("./") || import_path.starts_with("../") {
        source_dir.join(
            import_path
                .trim_start_matches("./")
                .trim_start_matches("../"),
        )
    } else {
        source_dir.join(import_path)
    }
}

fn emit_hmr_event(path: &Path) {
    if let Ok(json) = serde_json::to_string(&serde_json::json!({
        "type": "update",
        "path": path.display().to_string(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    })) {
        println!("{json}");
    }
}

fn lex_command(input: &Path) -> Result<()> {
    let source = fs::read_to_string(input).into_diagnostic()?;
    let map = nori::ast::SourceMap::new(&source);
    for token in lex(&source)? {
        let pos = map.span_start(token.span);
        println!(
            "{:?} {:?} @ {}:{}",
            token.kind, token.lexeme, pos.line, pos.column
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
