# Repository Guidelines

## Project Structure & Module Organization

Nori is a Rust compiler with Bun-managed JavaScript packages. Core compiler code lives in `src/`: `lexer.rs`, `parser.rs`, `ast.rs`, `analyzer.rs`, `codegen.rs`, `lib.rs`, and the CLI entrypoint in `main.rs`. Parser internals are split under `src/parser/`. Rust integration tests live in `tests/compiler_tests.rs`. Example `.nori` fixtures are in `examples/` and should be updated with syntax or codegen changes. JavaScript packages live under `packages/`: `core` runtime, `cli`, `vite-plugin`, and `rspack-plugin`. Reference docs are in `docs/`.

## Build, Test, and Development Commands

- `cargo test`: runs Rust compiler and CLI tests.
- `cargo fmt`: formats Rust code.
- `cargo clippy --all-targets -- -D warnings`: runs strict Rust linting.
- `cargo run -- compile examples/Counter.nori`: compiles a fixture.
- `cargo run -- lex examples/Counter.nori`: prints lexer tokens.
- `cargo run -- parse examples/Counter.nori`: prints the parsed AST.
- `bun test`: runs Bun tests, including `packages/core/src/index.test.ts`.
- `bun run build`: builds/checks all workspace packages.
- `bun run typecheck`: type-checks all packages.
- `bun run test`: runs `cargo test && bun test`.

## Coding Style & Naming Conventions

Use Rust 2024 style and keep code `rustfmt` clean. Prefer clear, small compiler stages over broad abstractions. Rust test names use descriptive snake case, for example `codegen_transforms_primitives_and_strips_types`. TypeScript and JavaScript packages use ESM syntax, two-space indentation, and named exports where practical. Keep generated JavaScript behavior explicit and readable.

## Testing Guidelines

Add Rust tests in `tests/compiler_tests.rs` for lexer, parser, analyzer, codegen, and CLI behavior. Add or update `.nori` fixtures in `examples/` when changing supported syntax. Runtime behavior belongs in Bun tests near the affected package, currently `packages/core/src/index.test.ts`. Unsupported syntax should produce a clear diagnostic rather than silently compiling incorrectly.

## Commit & Pull Request Guidelines

The current history uses Conventional Commit style, for example `feat: bootstrap Nori compiler prototype`. Continue with short imperative subjects such as `fix: improve markup diagnostics` or `test: cover derived effects`. Pull requests should describe the compiler/runtime behavior changed, list tests run, link related issues when available, and include before/after output or screenshots only for CLI or tooling-visible changes.

## Agent-Specific Instructions

Keep changes narrow and aligned with the learning-first compiler design. Do not introduce new parser dependencies unless the project direction changes. When editing compiler behavior, update docs in `docs/` if the language subset, runtime API, or CLI surface changes.
