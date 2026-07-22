# @nori/compiler

In-process Nori compiler for bundler plugins and browser playgrounds.

## Build (preferred)

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
bun run --cwd packages/compiler-wasm build:wasm
# or:
wasm-pack build crates/nori-wasm --target bundler --out-dir packages/compiler-wasm/pkg --release
```

This produces `packages/compiler-wasm/pkg/` with the WASM module and JS glue.

Verify the Rust bindings without a wasm toolchain:

```bash
cargo check -p nori-wasm
```

## Usage

```js
import { compile } from "@nori/compiler";

const js = await compile(source, {
  runtimeImport: "@nori/core",
  filename: "Counter.nori",
});
```

## Fallback

If `pkg/` has not been built (wasm-pack missing), `compile()` shells out to the Nori CLI via `@nori/cli` / `cargo run`. Plugins should still prefer this package so they automatically pick up WASM once built.

```bash
# Detect whether pkg/ is present
import { hasWasmBuild } from "@nori/compiler";
```

Vite / Rspack plugins call `@nori/compiler` **in-process first**; they only
spawn the CLI if the package cannot be imported at all.
