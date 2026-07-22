use nori::{
    Allocator, CompileOptions, analyze_source, compile_source,
    lexer::{LexContext, TokenKind, lex, lex_with_context},
    parse_source,
    parser::{Parser, Syntax},
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("nori-{name}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn lexer_tracks_markup_text_and_spans() {
    let source = "return <button type=\"button\" onclick={() => count.value += 1}>Click</button>;";
    let tokens = lex(source).unwrap();
    let map = nori::ast::SourceMap::new(source);

    assert!(tokens.iter().any(|token| token.lexeme(source) == "button"));
    assert!(tokens.iter().any(|token| token.lexeme(source) == "Click"));
    let pos = map.span_start(tokens[0].span);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
}

#[test]
fn lexer_handles_new_keywords() {
    let source = "new this super delete void typeof instanceof";
    let tokens = lex(source).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::New)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::This)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Super)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Delete)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Void)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Typeof)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Instanceof)));
}

#[test]
fn lexer_handles_statement_keywords() {
    let source = "switch case throw debugger with";
    let tokens = lex(source).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Switch)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Case)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Throw)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::Debugger)));
    assert!(kinds.contains(&TokenKind::Keyword(nori::lexer::Keyword::With)));
}

#[test]
fn lexer_handles_eqeqeq_and_bangeqeq() {
    let tokens = lex("=== !==").unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::EqEqEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::BangEqEq));
}

#[test]
fn lexer_handles_optional_chaining_and_nullish() {
    let tokens = lex("?. ?? ??=").unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::QuestionDot));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::QuestionQuestion));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::QuestionQuestionEq)
    );
}

#[test]
fn lexer_handles_shift_operators() {
    let tokens = lex("<< <<= >> >>= >>> >>>=").unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::ShiftLeft));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::ShiftLeftEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::ShiftRight));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::ShiftRightEq));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::ShiftRightUnsigned)
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::ShiftRightUnsignedEq)
    );
}

#[test]
fn lexer_handles_bitwise_operators() {
    let tokens = lex("~ ^ ^= & &= &&= | |= ||=").unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Tilde));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Caret));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::CaretEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Ampersand));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::AmpersandEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::AndAndEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Pipe));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::PipeEq));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::OrOrEq));
}

#[test]
fn lexer_handles_exponentiation() {
    let tokens = lex("** **=").unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::StarStar));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::StarStarEq));
}

#[test]
fn lexer_handles_bigint() {
    let tokens = lex("42n 0n 1_000n").unwrap();
    let bigints: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind == TokenKind::BigInt)
        .collect();
    assert_eq!(bigints.len(), 3);
}

#[test]
fn lexer_handles_numeric_separators() {
    let source = "1_000 1_000.5_5";
    let tokens = lex(source).unwrap();
    assert!(tokens.iter().any(|t| t.lexeme(source) == "1_000"));
    assert!(
        tokens
            .iter()
            .any(|t| t.lexeme(source).contains("1_000.5_5"))
    );
}

#[test]
fn lexer_skips_shebang() {
    let source = "#!/usr/bin/env node\nconst x = 1;";
    let tokens = lex(source).unwrap();
    assert!(tokens.iter().any(|t| t.lexeme(source) == "const"));
    assert!(!tokens.iter().any(|t| t.lexeme(source).contains("#!/usr")));
}

#[test]
fn lexer_records_comment_trivia() {
    let source = "// line\nconst x = 1; /* block */";
    let output = nori::lexer::lex_with_trivia(source, LexContext::Normal).unwrap();
    assert_eq!(output.trivia.len(), 2);
    assert_eq!(output.trivia[0].kind, nori::lexer::TriviaKind::LineComment);
    assert_eq!(output.trivia[0].span.source_text(source), "// line");
    assert_eq!(output.trivia[1].kind, nori::lexer::TriviaKind::BlockComment);
    assert_eq!(output.trivia[1].span.source_text(source), "/* block */");
    assert!(
        output
            .tokens
            .iter()
            .any(|t| t.kind == TokenKind::Keyword(nori::lexer::Keyword::Const))
    );
}

#[test]
fn lexer_parses_regex_literals_with_context() {
    let source = "/foo/g /bar/i";
    let tokens = lex_with_context(source, LexContext::ExpectRegex).unwrap();
    let regexps: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind == TokenKind::RegExp)
        .collect();
    assert_eq!(regexps.len(), 2);
    assert_eq!(regexps[0].lexeme(source), "/foo/g");
    assert_eq!(regexps[1].lexeme(source), "/bar/i");
}

#[test]
fn lexer_does_not_parse_regex_in_normal_context() {
    let tokens = lex("/foo/g").unwrap();
    assert!(!tokens.iter().any(|t| t.kind == TokenKind::RegExp));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Slash));
}

#[test]
fn lexer_handles_regex_with_escaped_slash() {
    let source = r"/foo\/bar/g";
    let tokens = lex_with_context(source, LexContext::ExpectRegex).unwrap();
    let regexp = tokens.iter().find(|t| t.kind == TokenKind::RegExp).unwrap();
    assert_eq!(regexp.lexeme(source), r"/foo\/bar/g");
}

#[test]
fn lexer_handles_regex_character_class() {
    let source = r"/[a-z]/g";
    let tokens = lex_with_context(source, LexContext::ExpectRegex).unwrap();
    let regexp = tokens.iter().find(|t| t.kind == TokenKind::RegExp).unwrap();
    assert_eq!(regexp.lexeme(source), r"/[a-z]/g");
}

#[test]
fn lexer_handles_regex_in_binary_expression() {
    let source = "x /y/g";
    let tokens = lex(source).unwrap();
    assert!(tokens.iter().any(|t| t.lexeme(source) == "x"));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Slash));
}

#[test]
fn lexer_slash_eq_not_affected_by_regex_context() {
    let tokens = lex_with_context("/=", LexContext::ExpectRegex).unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::SlashEq));
}

#[test]
fn lexer_slash_greater_not_affected_by_regex_context() {
    let tokens = lex_with_context("/>", LexContext::ExpectRegex).unwrap();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::SlashGreater));
}

#[test]
fn parser_builds_component_ast() {
    let source = include_str!("fixtures/Counter.nori");
    let allocator = Allocator::new();
    let program = parse_source(&allocator, source, "Counter.nori").unwrap();

    assert!(!program.body.is_empty());
}

#[test]
fn parser_accepts_explicit_nori_syntax_config() {
    let source = include_str!("fixtures/Counter.nori");
    let tokens = lex(source).unwrap();
    let allocator = Allocator::new();
    let program = Parser::new_with_syntax(
        &allocator,
        source,
        "Counter.nori".to_string(),
        tokens,
        Syntax::nori(),
    )
    .parse_program()
    .into_result()
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
            .contains(r#"h("button", { type: "button" }, "Click")"#)
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
            .contains(r#"h("button", { type: "button", onclick: save }, "Save")"#)
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
            .contains(r#"h("button", { type: "submit" }, "Save")"#)
    );
    assert!(!output.code.contains(r#"type: "button", type: "submit""#));
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
fn analyzer_tracks_reactive_value_reads_and_writes() {
    let source = r#"
const count = $state(0);
const doubled = $derived(count.value * 2);
count.value += 1;

export default function Counter() {
  return <p>{count.value} / {doubled.value}</p>;
}
"#;
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.value_reads.contains("count"));
    assert!(analysis.value_reads.contains("doubled"));
    assert!(analysis.value_writes.contains("count"));
    assert!(!analysis.value_writes.contains("doubled"));
}

#[test]
fn analyzer_tracks_reactive_value_updates_as_reads_and_writes() {
    let source = r#"
const count = $state(0);
count.value++;
--count.value;
"#;
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.value_reads.contains("count"));
    assert!(analysis.value_writes.contains("count"));
}

#[test]
fn analyzer_tracks_reactive_bindings_through_erased_typescript_expressions() {
    let source = r#"
type Signal<T> = { value: T };
const count = $state<number>(0) as Signal<number>;
const doubled = $derived<number>(count!.value as number);
count!.value += 1;
"#;
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.signals.contains("count"));
    assert!(analysis.computeds.contains("doubled"));
    assert!(analysis.value_reads.contains("count"));
    assert!(analysis.value_writes.contains("count"));
}

#[test]
fn analyzer_walks_runtime_class_members() {
    let source = r#"
const count = $state(0);
class Counter {
  doubled = $derived(count.value * 2);

  bump(step: number): void {
    count.value += step;
  }
}
"#;
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.runtime_symbols.contains("computed"));
    assert!(analysis.value_reads.contains("count"));
    assert!(analysis.value_writes.contains("count"));
}

#[test]
fn analyzer_ignores_value_accesses_on_shadowed_reactive_bindings() {
    let source = r#"
const count = $state(0);

function read(count) {
  count.value += 1;
  return count.value;
}
"#;
    let analysis = analyze_source(source, "inline.nori").unwrap();

    assert!(analysis.signals.contains("count"));
    assert!(!analysis.value_reads.contains("count"));
    assert!(!analysis.value_writes.contains("count"));
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
            .contains("import { computed, h, signal } from \"@nori/core\";")
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
fn cli_compile_writes_explicit_output_file() {
    let output_dir = TempDir::new("explicit-output");
    let output_path = output_dir.path.join("Counter.compiled.js");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Counter.nori");

    let output = Command::new(env!("CARGO_BIN_EXE_nori"))
        .args(["compile", fixture.to_str().unwrap(), "-o"])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "compile command failed: {:?}",
        output.stderr
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        output_path.display().to_string()
    );

    let code = fs::read_to_string(output_path).unwrap();
    assert!(code.contains("const count = signal(0);"));
    assert!(code.contains("export default function Counter()"));
}

#[test]
fn cli_compile_writes_named_file_to_output_directory() {
    let output_dir = TempDir::new("directory-output");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Counter.nori");
    let output_path = output_dir.path.join("Counter.js");

    let output = Command::new(env!("CARGO_BIN_EXE_nori"))
        .args(["compile", fixture.to_str().unwrap(), "-o"])
        .arg(&output_dir.path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "compile command failed: {:?}",
        output.stderr
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        output_path.display().to_string()
    );

    let code = fs::read_to_string(output_path).unwrap();
    assert!(code.contains("from \"@nori/core\";"));
    assert!(code.contains(r#"type: "button""#));
    assert!(code.contains("h("));
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
        let source =
            std::fs::read_to_string(manifest_dir.join("tests/fixtures").join(fixture)).unwrap();
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
            .contains(r#"import { h, signal } from "@nori/core""#),
        "Should inject h and signal imports when markup and $state are used"
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

    assert!(
        output.code.contains("signal")
            && output.code.contains("computed")
            && output.code.contains("h("),
        "Should inject signal, computed, and h"
    );
    assert!(
        output
            .code
            .contains(r#"import { computed, h, signal } from "@nori/core""#),
        "Should import runtime symbols including h"
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
fn codegen_erases_typescript_expression_suffixes_and_generic_calls() {
    let source = r#"
type Signal<T> = { value: T };
const count = $state<number>(0) as Signal<number>;
const stable = count!.value satisfies number;
const label = count!.value.toString();
export default function Counter() {
  return <p>{label} {count!.value}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("const count = signal(0);"));
    assert!(output.code.contains("const stable = count.value;"));
    assert!(
        output
            .code
            .contains("const label = count.value.toString();")
    );
    assert!(
        output
            .code
            .contains(r#"h("p", null, () => label, " ", () => count.value)"#)
    );
    assert!(!output.code.contains("$state<number>"));
    assert!(!output.code.contains(" as Signal"));
    assert!(!output.code.contains(" satisfies "));
    assert!(!output.code.contains("count!"));
}

#[test]
fn codegen_handles_no_runtime_import_when_none_needed() {
    let source = r#"
const greeting = "Hello";
export default function Greet() {
  return greeting;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        !output.code.contains("@nori/core"),
        "Should not inject runtime import when no primitives or markup used"
    );
    assert!(output.code.contains("export default function"));
}

#[test]
fn codegen_injects_h_for_markup_without_signals() {
    let source = r#"
const greeting = "Hello";
export default function Greet() {
  return <p>{greeting}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output.code.contains(r#"import { h } from "@nori/core""#),
        "Should inject h when markup is present"
    );
    assert!(output.code.contains(r#"h("p", null, () => greeting)"#));
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
            .contains(r#"import { effect, h, signal } from "@nori/core""#),
        "Should import effect and h when both are used"
    );
}

#[test]
fn decorators_are_parsed_and_stripped() {
    let source = include_str!("fixtures/unsupported/Decorator.nori");
    let output = compile_source(
        source,
        CompileOptions {
            filename: "Decorator.nori".to_string(),
            ..CompileOptions::default()
        },
    )
    .unwrap();

    assert!(output.code.contains("class Controller {"));
    assert!(output.code.contains("method()"));
    assert!(!output.code.contains("@decorator"));
}

#[test]
fn yield_expression_is_supported() {
    let source = include_str!("fixtures/unsupported/Yield.nori");
    let output = compile_source(
        source,
        CompileOptions {
            filename: "Yield.nori".to_string(),
            ..CompileOptions::default()
        },
    )
    .unwrap();

    assert!(output.code.contains("yield 1"));
    assert!(output.code.contains("function gen()"));
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
fn typescript_class_members_emit_runtime_javascript() {
    let source = r#"
interface Named { name: string };
abstract class Store<T> extends Base<T> implements Named, Iterable<T> {
  declare omitted: string;
  abstract describe(value: T): string;
  public required!: string;
  protected typed: number;
  private readonly version: number = 1;
  plain;
  static label: string = "store";

  constructor(public id: number, private readonly title: string) {
    super(id);
    this.version += id;
  }

  override rename(value: string): void {
    this.title = value;
  }

  async load<U>(value: U): Promise<U> {
    return value;
  }

  read(value: T): T;
  read(value: T): T {
    return value;
  }
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("class Store extends Base {"));
    assert!(output.code.contains("version = 1;"));
    assert!(output.code.contains("plain;"));
    assert!(output.code.contains("static label = \"store\";"));
    assert!(output.code.contains("rename(value)"));
    assert!(output.code.contains("async load(value)"));
    assert!(output.code.contains("read(value)"));
    assert!(!output.code.contains("implements"));
    assert!(!output.code.contains("abstract"));
    assert!(!output.code.contains("declare"));
    assert!(!output.code.contains("omitted"));
    assert!(!output.code.contains("required"));
    assert!(!output.code.contains("typed;"));
    assert!(!output.code.contains(": number"));

    let super_index = output.code.find("super(id);").unwrap();
    let id_assignment = output.code.find("this.id = id;").unwrap();
    let title_assignment = output.code.find("this.title = title;").unwrap();
    assert!(super_index < id_assignment);
    assert!(id_assignment < title_assignment);
}

#[test]
fn constructor_parameter_properties_precede_base_constructor_body() {
    let source = r#"
class Task {
  constructor(public id: number) {
    log(id);
  }
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    let assignment = output.code.find("this.id = id;").unwrap();
    let body_call = output.code.find("log(id);").unwrap();
    assert!(assignment < body_call);
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
fn while_loop_with_break_is_supported() {
    let source = r#"
export default function App() {
  let index = 0;
  while (index < 4) {
    if (index == 2) {
      break;
    }
    index++;
  }
  return <p>{index}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("while (index < 4)"));
    assert!(output.code.contains("break;"));
    assert!(output.code.contains("index++;"));
}

#[test]
fn do_while_loop_with_continue_is_supported() {
    let source = r#"
export default function App() {
  let index = 0;
  do {
    index++;
    if (index < 2) {
      continue;
    }
  } while (index < 4);
  return <p>{index}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("do {"));
    assert!(output.code.contains("continue;"));
    assert!(output.code.contains("} while (index < 4);"));
}

#[test]
fn classic_for_loop_with_update_is_supported() {
    let source = r#"
export default function App() {
  let total = 0;
  for (let index = 0; index < limit; index++) {
    total += index;
  }
  return <p>{total}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(
        output
            .code
            .contains("for (let index = 0; index < limit; index++)")
    );
    assert!(output.code.contains("total += index;"));
}

#[test]
fn classic_for_loop_allows_empty_clauses() {
    let source = r#"
export default function App() {
  for (;;) {
    break;
  }
  return <p>Done</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("for (; ; )"));
    assert!(output.code.contains("break;"));
}

#[test]
fn prefix_and_postfix_updates_are_preserved() {
    let source = r#"
export default function App() {
  let index = 0;
  ++index;
  index--;
  return <p>{index}</p>;
}
"#;
    let output = compile_source(source, CompileOptions::default()).unwrap();

    assert!(output.code.contains("++index;"));
    assert!(output.code.contains("index--;"));
}

#[test]
fn break_outside_loop_produces_diagnostic() {
    let error = compile_source("break;", CompileOptions::default()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("`break` is only valid inside a loop"),
        "{error}"
    );
}

#[test]
fn continue_outside_loop_produces_diagnostic() {
    let source = r#"
export default function App() {
  continue;
}
"#;
    let error = compile_source(source, CompileOptions::default()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("`continue` is only valid inside a loop"),
        "{error}"
    );
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

#[test]
fn parser_recovers_after_syntax_error() {
    let source = r#"
const ok = 1;
const !!! = 2;
const alsoOk = 3;
"#;
    let allocator = Allocator::new();
    let tokens = lex(source).unwrap();
    let result = Parser::new(&allocator, source, "bad.nori".to_string(), tokens).parse_program();
    assert!(!result.diagnostics.is_empty(), "expected parse diagnostics");
    assert!(
        result.diagnostics.iter().any(|d| d.is_error()),
        "expected at least one error"
    );
    // Recovery should still produce a program with the statements that parsed.
    assert!(
        !result.program.body.is_empty(),
        "expected a partial program after recovery"
    );
}

fn normalize_js_whitespace(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn typescript_enum_lowers_to_object_iife() {
    let source = include_str!("fixtures/typescript/enum_basic.nori");
    let expected = include_str!("fixtures/typescript/enum_basic.expected.js");
    let output = compile_source(source, CompileOptions::default()).unwrap();
    assert_eq!(
        normalize_js_whitespace(&output.code),
        normalize_js_whitespace(expected)
    );
}

#[test]
fn typescript_type_and_interface_are_stripped() {
    let source = include_str!("fixtures/typescript/strip_types.nori");
    let expected = include_str!("fixtures/typescript/strip_types.expected.js");
    let output = compile_source(source, CompileOptions::default()).unwrap();
    assert_eq!(
        normalize_js_whitespace(&output.code),
        normalize_js_whitespace(expected)
    );
    assert!(!output.code.contains("type Id"));
    assert!(!output.code.contains("interface Point"));
}

#[test]
fn typescript_differential_corpus_matches_expected_js() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript/corpus");
    let mut cases = 0usize;
    let mut failures = Vec::new();
    for entry in fs::read_dir(&corpus_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }
        let expected_path = path.with_extension("expected.js");
        if !expected_path.exists() {
            failures.push(format!("missing expected JS for {}", path.display()));
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let expected = fs::read_to_string(&expected_path).unwrap();
        match compile_source(
            &source,
            CompileOptions {
                filename: path.display().to_string(),
                ..CompileOptions::default()
            },
        ) {
            Ok(output) => {
                let actual = normalize_js_whitespace(&output.code);
                let want = normalize_js_whitespace(&expected);
                if actual != want {
                    failures.push(format!(
                        "mismatch for {}\n  actual:   {actual}\n  expected: {want}",
                        path.display()
                    ));
                }
            }
            Err(err) => {
                failures.push(format!("failed to compile {}: {err}", path.display()));
            }
        }
        cases += 1;
    }
    assert!(
        cases >= 3,
        "expected at least 3 corpus .ts fixtures, found {cases}"
    );
    assert!(
        failures.is_empty(),
        "corpus differentials failed:\n{}",
        failures.join("\n")
    );
}

#[test]
fn checker_reports_assignability_errors() {
    let result = nori::check_source("let x: string = 1;", "assign.ts").unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not assignable")),
        "expected assignability diagnostic"
    );
}

#[test]
fn checker_reports_excess_property_and_return_errors() {
    let excess = nori::check_source(
        "interface Point { x: number }\nconst p: Point = { x: 1, y: 2 };",
        "excess.nori",
    )
    .unwrap();
    assert!(
        excess
            .diagnostics
            .iter()
            .any(|d| d.message.contains("known properties") || d.message.contains("'y'")),
        "expected excess property diagnostic: {:?}",
        excess.diagnostics
    );

    let ret = nori::check_source("function f(): string { return 1; }", "ret.nori").unwrap();
    assert!(
        ret.diagnostics
            .iter()
            .any(|d| d.message.contains("return type")),
        "expected return type diagnostic: {:?}",
        ret.diagnostics
    );
}

#[test]
fn compile_with_type_check_surfaces_diagnostics() {
    let output = compile_source(
        "const x: string = 1;",
        CompileOptions {
            type_check: true,
            ..CompileOptions::default()
        },
    )
    .unwrap();
    assert!(output.code.contains("const x = 1"));
    assert!(
        output
            .diagnostics
            .iter()
            .any(|d| d.contains("not assignable")),
        "expected type diagnostic in compile output: {:?}",
        output.diagnostics
    );
}

#[test]
fn checker_typeof_narrowing_via_check_source() {
    let ok = nori::check_source(
        r#"
let x: string | number = 1;
if (typeof x === "string") {
  let s: string = x;
}
"#,
        "narrow.nori",
    )
    .unwrap();
    assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
}

#[test]
fn checker_component_props_via_check_source() {
    let bad = nori::check_source(
        r#"
function Foo(props: { bar: string }) {
  return <div />;
}
const el = <Foo bar={1} />;
"#,
        "props.nori",
    )
    .unwrap();
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.message.contains("not assignable") || d.message.contains("prop")),
        "{:?}",
        bad.diagnostics
    );
}

#[test]
fn const_enum_members_are_inlined() {
    let source = "const enum E { A = 1, B = 2 }\nconst x = E.B;";
    let output = compile_source(source, CompileOptions::default()).unwrap();
    assert!(!output.code.contains("var E"));
    assert!(output.code.contains("const x = 2"));
}

#[test]
fn semantic_model_tracks_scopes_for_nested_lets() {
    let allocator = Allocator::new();
    let source = "let x = 1; { let x = 2; x; }";
    let program = parse_source(&allocator, source, "sem.nori").unwrap();
    let model = nori::build_semantic(&program);
    assert_eq!(model.symbols.iter().filter(|s| s.name == "x").count(), 2);
}
