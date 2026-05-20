# Compiler Pipeline

Nori compiles `.nori` source through a small compiler pipeline written from scratch.

```text
Nori source
  -> lexer
  -> parser
  -> AST
  -> analyzer
  -> code generator
  -> JavaScript
```

## Lexer

The lexer converts source text into tokens with byte spans, line numbers, and columns.

It recognizes:

- JavaScript punctuation and operators.
- Identifiers, keywords, strings, and numbers.
- Element tag boundaries.
- Element text.
- Element expression containers.

The lexer has basic element modes so it can distinguish normal script from tag/text/expression contexts.

## Parser

The parser is hand-written. It uses:

- Recursive descent for programs, statements, functions, blocks, variable declarations, and component markup.
- Pratt-style precedence parsing for expressions.

The parser produces the custom AST in `src/ast.rs`.

## Analyzer

The analyzer walks the AST and records:

- Signal variables created by `$state`.
- Computed variables created by `$derived`.
- Effects created by `$effect`.
- Runtime symbols needed in output.

This is where future semantic checks should live.

## Code Generator

The code generator emits JavaScript for a bundler to finish.

It currently:

- Converts `$state` to `signal`.
- Converts `$derived` to `computed(() => ...)`.
- Converts `$effect` to `effect`.
- Inserts a runtime import when needed.
- Strips supported type annotations.
- Preserves component markup for Vite, Rspack, or another bundler.

## Why Preserve Markup?

V1 focuses on learning the compiler pipeline and Nori-specific reactivity. Markup lowering is a separate compiler problem, so Nori leaves component markup intact and lets existing bundlers lower it.
