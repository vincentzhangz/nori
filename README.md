# Nori

Nori is a learning-first Rust UI compiler for `.nori` files.

Nori source is a small component syntax with explicit reactivity primitives. Nori compiles those primitives into runtime calls and emits JavaScript for a bundler to finish.

```ts
const count: number = $state(0);
const doubled = $derived(count.value * 2);

export default function Counter() {
  return <p>{count.value} / {doubled.value}</p>;
}
```

```ts
import { computed, signal } from "@nori/core";

const count = signal(0);
const doubled = computed(() => count.value * 2);
export default function Counter() {
  return <p>{count.value} / {doubled.value}</p>;
}
```

## Why This Exists

Nori is an educational compiler project.

The repo is designed to make these pieces approachable:

- Lexing source text into tokens and spans.
- Parsing Nori syntax into a custom AST.
- Walking the AST for semantic analysis.
- Transforming reactivity primitives.
- Generating JavaScript output.
- Connecting a Rust compiler to JavaScript tooling.

## Status

Nori is experimental and not production-ready.

Current capabilities:

- Hand-written lexer with source spans.
- Custom recursive descent parser with Pratt-style expression parsing and SWC-style parser internals.
- Custom Nori AST for statements, expressions, functions, and element boundaries.
- Reactivity analyzer for `$state`, `$derived`, and `$effect`.
- Code generator that strips supported type syntax and preserves component markup.
- Rust CLI with `compile`, `watch`, `lex`, and `parse`.
- Bun workspace with `@nori/core`, `@nori/cli`, Vite plugin, and Rspack plugin scaffolds.
- Example `.nori` components and tests.

Nori intentionally starts as a small syntax subset, not a full JavaScript parser. Unsupported syntax should become a clear diagnostic instead of a silent miscompile.

## Quick Start

Requirements:

- Rust
- Bun

Clone the repo and run the tests:

```sh
cargo test --workspace
bun test
```

Run all current tests:

```sh
bun run test
```

Compile an example:

```sh
cargo run -p nori -- compile examples/Counter.nori
```

Write compiled output to a directory:

```sh
cargo run -p nori -- compile examples/Counter.nori -o dist/
```

Inspect tokens:

```sh
cargo run -p nori -- lex examples/Counter.nori
```

Inspect the parsed AST:

```sh
cargo run -p nori -- parse examples/Counter.nori
```

## Reactivity Model

Nori transforms explicit compiler primitives:

| Nori source       | JavaScript output      |
| ----------------- | ---------------------- |
| `$state(initial)` | `signal(initial)`      |
| `$derived(expr)`  | `computed(() => expr)` |
| `$effect(fn)`     | `effect(fn)`           |

The `.value` API is preserved in generated code. The runtime implements `.value`, `.get()`, and `.set()`.

## Language Scope

Nori V1 supports a small component syntax subset:

- `import` declarations.
- `export default function`.
- `const`, `let`, and `var`.
- Function declarations.
- Blocks, `return`, expression statements, and basic `if` / `else`.
- Calls, member access, assignments, binary expressions, ternaries, arrays, objects, and arrow functions.
- Elements, fragments, text children, expression containers, and common attribute forms.
- Basic type stripping for annotations, simple generics, `type`, and `interface`.

See [Language Subset](docs/language-subset.md) for details.

## Documentation

- [Compiler Pipeline](docs/compiler-pipeline.md)
- [Language Subset](docs/language-subset.md)
- [CLI](docs/cli.md)
- [Runtime](docs/runtime.md)
- [Development](docs/development.md)

## Development

Useful commands:

```sh
cargo fmt --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun test
```

Recommended workflow:

1. Add or update a `.nori` fixture in `examples/`.
2. Add parser/codegen expectations in `crates/nori/tests/compiler_tests.rs`.
3. Run the Rust tests.
4. Run the Bun runtime tests if runtime behavior changed.

## Contributing

Contributions are welcome, especially in areas that make the compiler easier to understand.

Good first areas:

- Lexer tests for tricky component syntax and type cases.
- Parser diagnostics for unsupported syntax.
- Small grammar additions with focused fixtures.
- Runtime tests for signal behavior.
- Documentation that explains compiler concepts clearly.

Please keep changes narrow and include tests for compiler behavior.

## Roadmap

- Better diagnostics and error recovery.
- More complete component markup parsing.
- More complete type stripping.
- Source maps.
- Vite plugin hardening.
- Rspack plugin hardening.
- Native binary packaging for `@nori/cli`.
- Optional markup lowering in a later compiler phase.

## License

Nori is licensed under the [MIT License](LICENSE).
