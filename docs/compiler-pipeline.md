# Compiler pipeline

Nori compiles `.nori` source through a hand-rolled Rust pipeline (oxc-inspired architecture: arena AST, zero-copy tokens, error recovery).

```text
.nori source
  → nori-lexer       Token { kind, span }  (+ comment trivia)
  → nori-parser      Program<'a> in bump arena  (+ ParseResult diagnostics)
  → nori-analyzer    $state / $derived / $effect, markup → h/fragment imports
  → nori-codegen     JavaScript (signals + h(...) + type strip)
```

Optional side path:

```text
Program → nori-semantic → nori-checker → diagnostics   (nori check)
```

## Allocator and spans

- **`nori-allocator`**: bumpalo-backed `Allocator`, `Box<'a, T>`, `Vec<'a, T>`, `Atom<'a>`.
- **`nori-span`**: `Span { start: u32, end: u32 }`. Line/column come from `SourceMap` when printing diagnostics — not stored on every token.

## Lexer

Zero-copy: tokens do **not** own lexeme strings; callers slice the source with `token.lexeme(source)` / `span.source_text(source)`.

Modes:

- Normal script
- Markup tag / text / expression (JSX-like)
- Regex vs division via `LexContext`

Comments are skipped and recorded as trivia (`lex_with_trivia`).

## Parser

- Recursive descent for programs, statements, classes, imports/exports, markup.
- Pratt binding-power parsing for expressions and a second layer for `TSType`.
- Arena allocation: `Parser` holds `&Allocator` + `&str` source.
- **Error recovery:** on a statement error, records a `Diagnostic`, synchronizes (semicolon / `}` / statement keywords), continues. `parse_program` returns `ParseResult { program, diagnostics }`.

## AST

Arena-allocated nodes in `nori-ast` with lifetime `'a`. Includes:

- Statements / expressions / patterns / classes / markup
- Real TypeScript type nodes (`TSType`, aliases, interfaces, enums, modules)
- Hand-written `Visit` / `VisitMut` + `AstKind`

## Analyzer

Records reactive bindings and decides which `@nori/core` symbols to import (`signal`, `computed`, `effect`, `h`, `fragment`).

## Code generator

Emits JavaScript for a bundler:

| Input | Output |
| --- | --- |
| `$state(x)` | `signal(x)` |
| `$derived(expr)` | `computed(() => expr)` |
| `$effect(fn)` | `effect(fn)` |
| Markup | `h("div", props, ...children)` |
| Expr children in markup | `() => ...` (fine-grained updates) |
| Types / interfaces | erased |
| `enum` | object IIFE (const enum inlined when simple) |

## Semantic + checker

- **`nori-semantic`**: scope tree, symbols, references.
- **`nori-checker`**: assignability, structural types, narrowing, simple generics/conditionals, component prop checks (`nori check`).

Compile / Vite emit keeps `type_check` **off** by default so type errors don’t block JS generation. Use `nori check` for diagnostics.

## Bundler (scaffold)

`nori-bundler` + `nori bundle` resolve relative imports, build a module graph, and concat (or multi-file emit) ESM. Not a full Rspack replacement yet — see `crates/nori-bundler/README.md`.

## WASM

`crates/nori-wasm` exposes `compile` via wasm-bindgen. `packages/compiler-wasm` (`@nori/compiler`) prefers WASM when built, otherwise falls back to the CLI. The Vite plugin tries `@nori/compiler` first.
