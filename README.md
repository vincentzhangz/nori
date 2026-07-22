# Nori

Nori is a learning-first **Rust UI compiler** for `.nori` files (JS/TS-like syntax, markup, and `$state` / `$derived` / `$effect`).

It compiles to **plain JavaScript**: reactivity → `@nori/core` signals, markup → `h(...)`, TypeScript types stripped.

```ts
const count: number = $state(0);
const doubled = $derived(count.value * 2);

export default function Counter() {
  return <p>{count.value} / {doubled.value}</p>;
}
```

↓

```js
import { computed, h, signal } from "@nori/core";

const count = signal(0);
const doubled = computed(() => count.value * 2);

export default function Counter() {
  return h("p", null, () => count.value, " / ", () => doubled.value);
}
```

## Documentation

**Start here:** [docs/README.md](docs/README.md)

| Guide | Link |
| --- | --- |
| Getting started (install, CLI, tests) | [docs/getting-started.md](docs/getting-started.md) |
| **Run the JS + TS examples** | [docs/examples.md](docs/examples.md) |
| **VS Code extension** | [docs/vscode-extension.md](docs/vscode-extension.md) |
| Compiler pipeline | [docs/compiler-pipeline.md](docs/compiler-pipeline.md) |
| Language + checker | [docs/language-subset.md](docs/language-subset.md) |
| CLI reference | [docs/cli.md](docs/cli.md) |
| Runtime `@nori/core` | [docs/runtime.md](docs/runtime.md) |
| Framework API | [docs/framework-api.md](docs/framework-api.md) |
| Development workflow | [docs/development.md](docs/development.md) |

## Quick start

Requirements: **Rust**, **Bun**.

```sh
bun install
bun run --cwd packages/core build

# compile a fixture
cargo run -p nori -- compile examples/Counter.nori

# run the JavaScript example (vanilla CSS)
bun run --cwd examples/app dev
# → http://localhost:5173

# run the TypeScript example (Tailwind CSS)
bun run --cwd examples/app-ts dev
# → http://localhost:5174
```

Tests:

```sh
cargo test -p nori
cargo clippy --all-targets -- -D warnings
bun test
```

## Reactivity cheat sheet

| Nori | JavaScript |
| --- | --- |
| `$state(initial)` | `signal(initial)` |
| `$derived(expr)` | `computed(() => expr)` |
| `$effect(fn)` | `effect(fn)` |

## Status

Nori is under active development. The compiler front-end, `h()` codegen, checker (`nori check`), Vite examples, and bundler scaffold are usable for learning and demos. It is **not** a drop-in replacement for the full TypeScript / Next.js / Rspack ecosystems yet — see docs for current limits.

## License

MIT — see [LICENSE](LICENSE).
