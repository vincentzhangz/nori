# Nori documentation

Start here if you want to **understand Nori** and **run the code**.

| Doc | What it covers |
| --- | --- |
| [Getting started](./getting-started.md) | Install, build, compile a file, run tests |
| [Examples](./examples.md) | JavaScript + TypeScript Vite apps (`dev` / `build`) |
| [Compiler pipeline](./compiler-pipeline.md) | Lexer → parser → AST → analyze → codegen |
| [Language subset](./language-subset.md) | Supported JS/TS/markup and the type checker |
| [CLI](./cli.md) | `nori compile`, `check`, `bundle`, `lex`, `parse`, … |
| [Runtime](./runtime.md) | `@nori/core` signals + `h()` renderer |
| [Framework API](./framework-api.md) | Routing, `createApp`, Module Federation helpers |
| [VS Code extension](./vscode-extension.md) | `.nori` highlighting + formatting |
| [Development](./development.md) | Day-to-day workflow, clippy, crates layout |

## What Nori is (short)

Nori is a **from-scratch Rust compiler** for `.nori` files (JS/TS-like syntax + markup + `$state` / `$derived` / `$effect`).

It emits **plain JavaScript** that:

1. Rewrites reactivity to `signal` / `computed` / `effect` from `@nori/core`
2. Lowers markup to `h(...)` calls (no React JSX required)
3. Strips TypeScript types (and lowers `enum`s)

Vite / Rspack plugins compile `.nori` during bundling. Optional: `nori check` for type diagnostics, `nori bundle` for a simple ESM graph.

## Fastest path to a running UI

```sh
# from repo root
bun install
bun run --cwd packages/core build

# JavaScript + vanilla CSS → http://localhost:5173
bun run --cwd examples/app dev

# TypeScript + Tailwind CSS → http://localhost:5174
bun run --cwd examples/app-ts dev
```

Details: [Examples](./examples.md) (vanilla CSS vs Tailwind).
