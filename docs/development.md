# Development

## Requirements

- Rust toolchain for compiler crates
- Bun for JS packages and examples

## Common commands

```sh
# Install JS workspace
bun install
bun run --cwd packages/core build

# Rust tests / lint / format
cargo test -p nori
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --workspace

# JS tests
bun test

# Root combo
bun run test
```

## Run examples while developing

```sh
bun run --cwd examples/app dev       # JS  :5173
bun run --cwd examples/app-ts dev    # TS  :5174
```

Full guide: [examples.md](./examples.md) · [getting-started.md](./getting-started.md).

## Suggested workflow (compiler change)

1. Add or update a `.nori` fixture under `examples/` or `crates/nori/tests/fixtures/`.
2. Add expectations in `crates/nori/tests/compiler_tests.rs`.
3. `cargo test -p nori`
4. `cargo clippy --all-targets -- -D warnings`
5. If runtime / plugins / framework changed: `bun test`
6. If docs / public behavior changed: update `docs/` (and this file’s links if needed)

## Design rules

- Prefer **clear diagnostics** over silently accepting unsupported syntax.
- Keep the compiler **learning-first** and dependency-light (bumpalo allocator is OK; no external JS parser crates).
- Markup emits **`h()`** from `@nori/core` — do not reintroduce a React-classic JSX handoff unless intentionally reverting that decision.

## Crate map

| Crate | Role |
| --- | --- |
| `nori-span` / `nori-allocator` | Spans + bump arena |
| `nori-lexer` / `nori-parser` / `nori-ast` | Front-end |
| `nori-analyzer` / `nori-codegen` | Reactivity + emit |
| `nori-semantic` / `nori-checker` | Check path |
| `nori-bundler` / `nori-wasm` | Bundle + WASM |
| `nori` | CLI + integration tests |
