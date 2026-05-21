use nori::{
    analyze_source, compile_source,
    lexer::lex,
    parse_source,
    parser::{Parser, Syntax},
    CompileOptions,
};
use std::path::Path;
use std::process::Command;

#[test]
fn lexer_tracks_markup_text_and_spans() {
    let tokens =
        lex("return <button type=\"button\" onclick={() => count.value += 1}>Click</button>;")
            .unwrap();

    assert!(tokens.iter().any(|token| token.lexeme == "button"));
    assert!(tokens.iter().any(|token| token.lexeme == "Click"));
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.column, 1);
}

#[test]
fn parser_builds_component_ast() {
    let source = include_str!("fixtures/Counter.nori");
    let program = parse_source(source, "Counter.nori").unwrap();

    assert!(!program.body.is_empty());
}

#[test]
fn parser_accepts_explicit_nori_syntax_config() {
    let source = include_str!("fixtures/Counter.nori");
    let tokens = lex(source).unwrap();
    let program =
        Parser::new_with_syntax(source, "Counter.nori".to_string(), tokens, Syntax::nori())
            .parse_program()
            .unwrap();

    assert!(!program.body.is_empty());
}

#[test]
fn parser_accepts_keyword_markup_attributes() {
    let source = r#"export default function Button() {
  return <button type="button">Click</button>;
}"#;

    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains(r#"<button type="button">Click</button>"#)
    );
}

#[test]
fn codegen_defaults_button_type_when_missing() {
    let source = r#"export default function Button() {
  return (
    <button onclick={save}>Save</button>
  );
}"#;

    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains(r#"<button type="button" onclick={save}>Save</button>"#)
    );
}

#[test]
fn codegen_preserves_explicit_button_type() {
    let source = r#"export default function Button() {
  return <button type="submit">Save</button>;
}"#;

    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains(r#"<button type="submit">Save</button>"#)
    );
    assert!(!output.code.contains(r#"type="button" type="submit""#));
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

    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Counter.nori");
    let lex = Command::new(bin)
        .args(["lex", examples.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(lex.status.success(), "lex command failed: {:?}", lex.stderr);
    assert!(String::from_utf8_lossy(&lex.stdout).contains("MarkupText"));
}

#[test]
fn all_examples_compile() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for fixture in [
        "Counter.nori",
        "Todo.nori",
        "ShadcnCard.nori",
        "DerivedEffect.nori",
    ] {
        let source = std::fs::read_to_string(manifest_dir.join("tests/fixtures").join(fixture)).unwrap();
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

#[test]
fn codegen_injects_runtime_import_when_needed() {
    let source = r#"
const count = $state(0);
export default function Counter() {
  return <p>{count.value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains(r#"import { signal } from "@nori/core""#),
        "Should inject signal import when $state is used"
    );
    assert!(
        !output.code.contains("computed"),
        "Should not import computed when not used"
    );
}

#[test]
fn codegen_injects_multiple_runtime_imports() {
    let source = r#"
const count = $state(0);
const doubled = $derived(count.value * 2);
export default function Counter() {
  return <p>{doubled.value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("signal") && output.code.contains("computed"), "Should inject both signal and computed imports");
    assert!(
        output
            .code
            .contains(r#"import { computed, signal } from "@nori/core""#)
            || output
                .code
                .contains(r#"import { signal, computed } from "@nori/core""#),
        "Should import both runtime symbols"
    );
}

#[test]
fn codegen_strips_type_annotations_completely() {
    let source = r#"
type Props = { name: string };
interface CounterProps { initial: number };
const count: CounterProps['initial'] = $state(0);
export default function Counter(): JSX.Element {
  return <p>{count.value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        !output.code.contains("type Props"),
        "Should strip type declarations"
    );
    assert!(
        !output.code.contains("interface CounterProps"),
        "Should strip interface"
    );
    assert!(
        !output.code.contains(": string"),
        "Should strip type annotations"
    );
    assert!(
        !output.code.contains(": JSX.Element"),
        "Should strip return type"
    );
    assert!(
        !output.code.contains("['initial']"),
        "Should strip bracket notation"
    );
    assert!(
        output.code.contains("signal(0)"),
        "Should still transform $state"
    );
}

#[test]
fn codegen_strips_generic_type_parameters() {
    let source = r#"
const items: string[] = [];
export default function List() {
  return <p>{items.length}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        !output.code.contains(": string[]"),
        "Should strip array type annotation"
    );
    assert!(
        output.code.contains("const items = [];"),
        "Should have empty array"
    );
}

#[test]
fn codegen_handles_no_runtime_import_when_none_needed() {
    let source = r#"
const greeting = "Hello";
export default function Greet() {
  return <p>{greeting}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        !output.code.contains("@nori/core"),
        "Should not inject runtime import when no primitives used"
    );
    assert!(output.code.contains("export default function"));
}

#[test]
fn codegen_effect_creates_effect_import() {
    let source = r#"
const count = $state(0);
$effect(() => console.log(count.value));
export default function Counter() {
  return <p>{count.value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output.code.contains("effect"),
        "Should transform $effect to effect"
    );
    assert!(
        output
            .code
            .contains(r#"import { effect, signal } from "@nori/core""#),
        "Should import effect when $effect is used"
    );
}

#[test]
fn unsupported_decorator_produces_diagnostic() {
    let source = r#"
class Controller {
  @decorator()
  method() {}
}
export default function App() {
  return <p>Hello</p>;
}
"#;
    let result = compile_source(source, CompileOptions::default());

    assert!(
        result.is_err(),
        "decorator should produce an error since it's not yet supported"
    );
}

#[test]
fn unsupported_yield_produces_diagnostic() {
    let source = r#"
function* gen() {
  yield 1;
  yield 2;
}
export default function App() {
  return <p>Hello</p>;
}
"#;
    let result = compile_source(source, CompileOptions::default());

    assert!(
        result.is_err(),
        "yield should produce an error since it's not yet supported"
    );
}

#[test]
fn class_declaration_with_extends_is_supported() {
    let source = r#"
class Point extends Base {
  x = 1;
  y = 2;
}
export default function App() {
  return <p>Hello</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("class Point extends Base {"));
    assert!(output.code.contains("x = 1"));
    assert!(output.code.contains("y = 2"));
}

#[test]
fn template_literals_are_preserved() {
    let source = r#"
const greeting = `Hello`;
export default function App() {
  return <p>Template</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("`Hello`"));
}

#[test]
fn try_catch_finally_is_supported() {
    let source = r#"
export default function App() {
  try {
    risky();
  } catch (e) {
    console.error(e);
  } finally {
    cleanup();
  }
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("try {"));
    assert!(output.code.contains("} catch (e) {"));
    assert!(output.code.contains("} finally {"));
    assert!(output.code.contains("risky()"));
    assert!(output.code.contains("console.error(e)"));
    assert!(output.code.contains("cleanup()"));
}

#[test]
fn try_catch_without_param_is_supported() {
    let source = r#"
export default function App() {
  try {
    fallible();
  } catch {
    handle();
  }
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("try {"));
    assert!(output.code.contains("} catch {"));
    assert!(output.code.contains("handle()"));
}

#[test]
fn for_in_loop_is_supported() {
    let source = r#"
export default function App() {
  const obj = { a: 1, b: 2 };
  for (const key in obj) {
    console.log(key);
  }
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("for (const key in obj)"));
    assert!(output.code.contains("console.log(key)"));
}

#[test]
fn async_function_is_supported() {
    let source = r#"
async function fetchData() {
  const result = await Promise.resolve(42);
  return result;
}
export default function App() {
  return <p>Async</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("async function fetchData()"));
    assert!(output.code.contains("await Promise.resolve(42)"));
}

#[test]
fn await_expression_is_supported() {
    let source = r#"
export default async function App() {
  const value = await fetchSomething();
  return <p>{value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("async function App()"));
    assert!(output.code.contains("await fetchSomething()"));
}

#[test]
fn spread_in_array_is_preserved() {
    let source = r#"
export default function App() {
  const arr = [1, 2, ...more];
  return <p>{arr}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("[1, 2, ...more]"));
}

#[test]
fn spread_in_function_call_is_preserved() {
    let source = r#"
export default function App() {
  const result = sum(1, 2, ...rest);
  return <p>{result}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("sum(1, 2, ...rest)"));
}