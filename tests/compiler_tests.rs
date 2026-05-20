use nori::{CompileOptions, analyze_source, compile_source, lexer::lex, parse_source};
use std::process::Command;

#[test]
fn lexer_tracks_markup_text_and_spans() {
    let tokens = lex("return <button onclick={() => count.value += 1}>Click</button>;").unwrap();

    assert!(tokens.iter().any(|token| token.lexeme == "button"));
    assert!(tokens.iter().any(|token| token.lexeme == "Click"));
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.column, 1);
}

#[test]
fn parser_builds_component_ast() {
    let source = include_str!("../examples/Counter.nori");
    let program = parse_source(source, "Counter.nori").unwrap();

    assert!(!program.body.is_empty());
}

#[test]
fn analyzer_discovers_reactive_bindings() {
    let source = "const count = $state(0); const doubled = $derived(count.value * 2);";
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.signals.contains("count"));
    assert!(analysis.computeds.contains("doubled"));
    assert!(analysis.runtime_symbols.contains("signal"));
    assert!(analysis.runtime_symbols.contains("computed"));
}

#[test]
fn codegen_transforms_primitives_and_strips_types() {
    let source = r"
type Count = number;
const count: Count = $state(0);
const doubled = $derived(count.value * 2);

export default function Counter() {
  return <p>{doubled.value}</p>;
}
";

    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains("import { computed, signal } from \"@nori/core\";")
    );
    assert!(output.code.contains("const count = signal(0);"));
    assert!(
        output
            .code
            .contains("const doubled = computed(() => count.value * 2);")
    );
    assert!(output.code.contains("export default function Counter()"));
    assert!(!output.code.contains("type Count"));
    assert!(!output.code.contains(": "));
}

#[test]
fn codegen_keeps_value_api() {
    let source = "const count = $state(0); count.value += 1;";
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("count.value += 1;"));
    assert!(!output.code.contains("count.set"));
}

#[test]
fn cli_help_and_lex_work() {
    let bin = env!("CARGO_BIN_EXE_nori");
    let help = Command::new(bin).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("compile"));

    let lex = Command::new(bin)
        .args(["lex", "examples/Counter.nori"])
        .output()
        .unwrap();
    assert!(lex.status.success());
    assert!(String::from_utf8_lossy(&lex.stdout).contains("MarkupText"));
}

#[test]
fn all_examples_compile() {
    for fixture in [
        "examples/Counter.nori",
        "examples/Todo.nori",
        "examples/ShadcnCard.nori",
        "examples/DerivedEffect.nori",
    ] {
        let source = std::fs::read_to_string(fixture).unwrap();
        let output = compile_source(
            &source,
            CompileOptions {
                filename: fixture.to_string(),
                ..CompileOptions::default()
            },
        )
        .unwrap();
        assert!(
            output.code.contains("export default function"),
            "{fixture} did not compile to a component"
        );
    }
}
