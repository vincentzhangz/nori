# Repository Guidelines

## Project Structure & Module Organization

Nori is a Rust compiler (workspace under `crates/`) with Bun-managed JavaScript packages (`packages/`). Reference docs live in `docs/` — start at [docs/README.md](docs/README.md). Runnable apps: `examples/app` (JS) and `examples/app-ts` (TS). Editor support: [`extensions/nori`](extensions/nori) (syntax + format; see [docs/vscode-extension.md](docs/vscode-extension.md)).

## Build, Test, and Development Commands

- `cargo test -p nori` / `cargo test --workspace`: Rust tests.
- `cargo fmt`: formats Rust code.
- `cargo clippy --all-targets -- -D warnings`: strict Rust linting.
- `cargo run -p nori -- compile examples/Counter.nori`: compile a fixture.
- `cargo run -p nori -- check examples/Todo.nori`: type-check.
- `bun test`: Bun package tests.
- `bun run --cwd examples/app dev`: JS example (port 5173).
- `bun run --cwd examples/app-ts dev`: TS example (port 5174).
- `bun run --cwd extensions/nori package`: build the VS Code `.vsix`.
- `bun run test`: `cargo test && bun test`.

See [docs/getting-started.md](docs/getting-started.md) and [docs/examples.md](docs/examples.md).

## Coding Style & Naming Conventions

Use Rust 2024 style and keep code `rustfmt` clean. Prefer clear, small compiler stages over broad abstractions. Rust test names use descriptive snake case, for example `codegen_transforms_primitives_and_strips_types`. TypeScript and JavaScript packages use ESM syntax, two-space indentation, and named exports where practical. Keep generated JavaScript behavior explicit and readable.

## Testing Guidelines

Add Rust tests in `tests/compiler_tests.rs` for lexer, parser, analyzer, codegen, and CLI behavior. Add or update `.nori` fixtures in `examples/` when changing supported syntax. Runtime behavior belongs in Bun tests near the affected package, currently `packages/core/src/index.test.ts`. Unsupported syntax should produce a clear diagnostic rather than silently compiling incorrectly.

## Commit & Pull Request Guidelines

The current history uses Conventional Commit style, for example `feat: bootstrap Nori compiler prototype`. Continue with short imperative subjects such as `fix: improve markup diagnostics` or `test: cover derived effects`. Pull requests should describe the compiler/runtime behavior changed, list tests run, link related issues when available, and include before/after output or screenshots only for CLI or tooling-visible changes.

## Agent-Specific Instructions

Keep changes narrow and aligned with the learning-first compiler design. Do not introduce new parser dependencies unless the project direction changes. When editing compiler behavior, update docs in `docs/` if the language subset, runtime API, or CLI surface changes.
