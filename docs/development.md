# Development

## Requirements

- Rust for compiler development.
- Bun for JavaScript package tests.

## Common Commands

Run Rust tests:

```sh
cargo test --workspace
```

Run Bun tests:

```sh
bun test
```

Run all current tests:

```sh
bun run test
```

Run strict Rust linting:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Format Rust code:

```sh
cargo fmt --workspace
```

## Suggested Workflow

1. Add or update a `.nori` fixture in `examples/`.
2. Add parser/codegen expectations in `crates/nori/tests/compiler_tests.rs`.
3. Run `cargo test --workspace`.
4. Run `cargo clippy --workspace --all-targets -- -D warnings`.
5. Run `bun test` if runtime behavior changed.

## Design Rule

Prefer clear diagnostics over accepting unsupported syntax incorrectly. Nori intentionally starts as a small syntax subset while the lexer and parser are being built from scratch.
