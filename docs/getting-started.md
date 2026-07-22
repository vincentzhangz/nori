# Getting started

## Requirements

- **Rust** (edition 2024 toolchain; see `rust-version` in crate `Cargo.toml` files)
- **Bun** (workspace package manager / test runner)
- Optional: `wasm-pack` if you want to build `@nori/compiler` WASM artifacts

## Install

```sh
git clone <repo-url> nori
cd nori
bun install
bun run --cwd packages/core build
```

`bun install` links workspace packages (`@nori/core`, `@nori/vite-plugin`, example apps, …).  
Building `@nori/core` produces `packages/core/dist/` used by the examples.

## Compile a `.nori` file (CLI)

```sh
# print JS to stdout
cargo run -p nori -- compile examples/Counter.nori

# write to a directory
cargo run -p nori -- compile examples/Counter.nori -o dist/

# watch + recompile
cargo run -p nori -- watch examples/Counter.nori -o dist/
```

Inspect intermediate stages:

```sh
cargo run -p nori -- lex examples/Counter.nori
cargo run -p nori -- parse examples/Counter.nori
```

Type-check (does **not** emit JS by default path — diagnostics only):

```sh
cargo run -p nori -- check examples/Todo.nori
```

Simple relative ESM bundle:

```sh
cargo run -p nori -- bundle examples/Todo.nori -o /tmp/todo.bundle.js
```

Full CLI reference: [cli.md](./cli.md).

## Run the example apps

See **[examples.md](./examples.md)** for both hosts:

| Example | Command | URL |
| --- | --- | --- |
| JavaScript | `bun run --cwd examples/app dev` | http://localhost:5173 |
| TypeScript | `bun run --cwd examples/app-ts dev` | http://localhost:5174 |

## Tests and lint

```sh
# Rust compiler tests
cargo test -p nori

# Strict lint (must be clean)
cargo clippy --all-targets -- -D warnings

# JS/TS package tests (core, plugins, framework, …)
bun test

# Everything the root script runs
bun run test
```

Compiler bench (Todo.nori):

```sh
cargo bench -p nori --bench compile_todo
# or
bun run bench:compile
```

## VS Code / Cursor extension

```sh
cd extensions/nori && bun install
# Then F5 with launch config "Run Nori Extension", or see docs/vscode-extension.md
```

```text
.nori source
    │
    ├─ nori-lexer      zero-copy tokens + trivia
    ├─ nori-parser     arena AST (bump allocator)
    ├─ nori-analyzer   $state / $derived / $effect + markup imports
    ├─ nori-semantic   scopes / symbols (for check)
    ├─ nori-checker    optional type checking (nori check)
    └─ nori-codegen    signal/computed/effect + h(...) + type strip
            │
            ▼
     plain JavaScript  ──Vite / Rspack──►  browser
            │
            └─ @nori/core runtime (signals, h, mount, hydrate)
```

More detail: [compiler-pipeline.md](./compiler-pipeline.md).

## Repo layout (high level)

```text
crates/
  nori/              CLI + integration tests
  nori-lexer/        tokens
  nori-parser/       recursive descent + Pratt
  nori-ast/          arena-allocated AST + Visit
  nori-allocator/    bumpalo wrapper
  nori-span/         u32 spans + SourceMap
  nori-analyzer/     reactivity analysis
  nori-semantic/     scopes / symbols
  nori-checker/      type checker
  nori-codegen/      JS emit
  nori-bundler/      simple module graph / concat
  nori-wasm/         wasm-bindgen bindings
packages/
  core/              @nori/core runtime
  vite-plugin/       compile .nori in Vite
  rspack-plugin/     compile .nori in Rspack
  nori-framework/    router + createApp + MF helpers
  compiler-wasm/     @nori/compiler (WASM or CLI fallback)
examples/
  app/               JavaScript Vite app
  app-ts/            TypeScript Vite app
  Counter.nori …     standalone compile fixtures
docs/                you are here
```
