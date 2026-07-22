# Nori vs Next.js benchmark plan

This directory holds harness notes and minimal runnable scripts for comparing
Nori against Next.js on build/dev speed and runtime characteristics.

## Measurable commands (today)

These work against the current compiler / fixtures without a full twin app:

```bash
# Compile throughput (Criterion)
cargo bench -p nori --bench compile_todo

# Single-file wall clock
hyperfine --warmup 3 'cargo run -q -p nori -- compile examples/Todo.nori'

# In-process JS compile (CLI fallback until wasm pkg is built)
hyperfine --warmup 2 \
  'bun -e "import { compile } from \"./packages/compiler-wasm/src/index.js\"; const s = await Bun.file(\"examples/Todo.nori\").text(); await compile(s);"'

# Workspace unit tests as a regression gate
bun test
cargo test -p nori-bundler
```

Scaffold timing helper:

```bash
node bench/scripts/measure-compile.mjs
```

## Goals (full twin apps)

| Metric | What we measure | Why |
| --- | --- | --- |
| Cold dev-server start | Time to first usable page | WASM compiler vs tsc/webpack/turbopack startup |
| HMR update latency | Edit → DOM update | In-process transform + signal updates |
| Production build time | Clean `build` wall clock | Rspack (near-term) / nori-bundler (Phase 5) |
| Runtime | js-framework-benchmark-style ops | Fine-grained DOM vs React VDOM |

## App shape (planned)

```text
bench/
  nori-app/       # Nori + @nori/framework + Rspack
  next-app/       # Next.js App Router equivalent
  scripts/        # harness: start, HMR poke, build, report JSON
  README.md       # this file
```

Both apps should implement the same UI:

1. Counter with derived value (reactivity baseline)
2. Todo list with add/toggle/filter (list keyed updates)
3. Nested layout + one `load`-style data fetch (routing/data)

## Harness sketch (after twin apps land)

```bash
# Cold start
hyperfine --warmup 1 'bun run --cwd bench/nori-app dev' 'bun run --cwd bench/next-app dev'

# Production build
hyperfine 'bun run --cwd bench/nori-app build' 'bun run --cwd bench/next-app build'

# HMR: script opens page, edits a source line, measures mutation observer timing
node bench/scripts/hmr-latency.mjs --target nori|next
```

## Reporting

Emit `bench/results/<date>.json` with:

- machine info (os, cpu)
- git SHA
- metric name → `{ nori_ms, next_ms, ratio }`

Re-run after each framework milestone (M1–M5) and after nori-bundler swaps out Rspack.

## Non-goals (for now)

- Full js-framework-benchmark suite upstream contribution
- Micro-benchmarking WASM DOM (runtime stays JS on purpose)
