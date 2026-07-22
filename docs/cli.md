# CLI

The Rust binary is named `nori`. During local development:

```sh
cargo run -p nori -- <command> [args]
```

After `cargo build -p nori --release`, you can also run `target/release/nori` (path may vary with Cargo target-dir).

## Commands

### `compile`

Compile `.nori` → JavaScript.

```sh
# stdout
cargo run -p nori -- compile examples/Counter.nori

# directory → Counter.js
cargo run -p nori -- compile examples/Counter.nori -o dist/

# explicit file
cargo run -p nori -- compile examples/Counter.nori -o dist/Counter.js

# custom runtime import path
cargo run -p nori -- compile examples/Counter.nori --runtime-import "@nori/core"
```

### `watch`

Recompile when the input file changes.

```sh
cargo run -p nori -- watch examples/Counter.nori -o dist/
```

### `check`

Run semantic analysis + type checker (M1–M8). Exits non-zero on type/parse errors.

```sh
cargo run -p nori -- check examples/Todo.nori
```

### `bundle`

Build a simple relative ESM bundle via `nori-bundler`.

```sh
cargo run -p nori -- bundle examples/Todo.nori -o /tmp/todo.js
cargo run -p nori -- bundle examples/Todo.nori -o /tmp/out --multi-file
```

### `lex` / `parse`

Debug front-end stages.

```sh
cargo run -p nori -- lex examples/Counter.nori
cargo run -p nori -- parse examples/Counter.nori
```

## Output

`compile` emits **plain JavaScript** (ESM imports from `@nori/core`, `h(...)` for markup). A bundler (Vite/Rspack) still resolves packages and serves/bundles for the browser.

For app workflows, prefer the Vite examples — see [examples.md](./examples.md).
