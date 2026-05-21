# CLI

The Rust binary is named `nori`.

During local development, run it through Cargo:

```sh
cargo run -p nori -- <command>
```

## Commands

Compile a file and print output:

```sh
cargo run -p nori -- compile examples/Counter.nori
```

Compile a file to a directory:

```sh
cargo run -p nori -- compile examples/Counter.nori -o dist/
```

Compile a file to a specific output file:

```sh
cargo run -p nori -- compile examples/Counter.nori -o dist/Counter.js
```

Watch a file and recompile on changes:

```sh
cargo run -p nori -- watch examples/Counter.nori -o dist/
```

Print tokens:

```sh
cargo run -p nori -- lex examples/Counter.nori
```

Print the parsed AST:

```sh
cargo run -p nori -- parse examples/Counter.nori
```

Use a custom runtime import:

```sh
cargo run -p nori -- compile examples/Counter.nori --runtime-import "@/nori/core"
```

## Output Format

Nori emits JavaScript that should be handled by a bundler.

When the output target is a directory, Nori writes JavaScript files.
