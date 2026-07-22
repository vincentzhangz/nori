# Examples

Nori ships two runnable Vite apps so you can compare a **JavaScript** host and a **TypeScript** host. Both use `.nori` routes compiled by `@nori/vite-plugin`.

## Prerequisites (once per clone)

From the **repo root**:

```sh
bun install
bun run --cwd packages/core build
```

You need a working Rust toolchain so the Vite plugin can compile `.nori` (via `@nori/compiler` CLI fallback or a built WASM package).

---

## 1. JavaScript example — `examples/app` (vanilla CSS)

**What it is:** plain JS entry (`src/main.js`), JS Vite config, **vanilla CSS** in `src/styles.css`, `.nori` routes using `class="..."`.

| Script | Command |
| --- | --- |
| Dev server | `bun run --cwd examples/app dev` |
| Production build | `bun run --cwd examples/app build` |
| Preview build | `bun run --cwd examples/app preview` |

- **Dev URL:** http://localhost:5173  
- **Styles:** import `./styles.css` from `main.js` (no Tailwind)  
- **Route:** `src/routes/index.nori` → `/`
- **Entry:** `src/main.js` globs `./routes/**/*.nori`

---

## 2. TypeScript example — `examples/app-ts` (Tailwind CSS)

**What it is:** typed host (`src/main.ts`, `vite.config.ts`) + **Tailwind CSS v4** via `@tailwindcss/vite`. Utility classes live on elements in `.nori` (e.g. `class="flex items-center ..."`).

| Script | Command |
| --- | --- |
| Typecheck host | `bun run --cwd examples/app-ts typecheck` |
| Dev server | `bun run --cwd examples/app-ts dev` |
| Production build | `bun run --cwd examples/app-ts build` |

- **Dev URL:** http://localhost:5174  
- **Styles:** `src/styles.css` starts with `@import "tailwindcss";`; Vite plugin `@tailwindcss/vite`  
- **Route:** `src/routes/index.nori` includes TS annotations (erased) + Tailwind classes
- **Entry:** `src/main.ts` globs `./routes/**/*.nori`

---

## Run both at once

```sh
# terminal 1
bun run --cwd examples/app dev

# terminal 2
bun run --cwd examples/app-ts dev
```

Then open:

- JS → http://localhost:5173  
- TS → http://localhost:5174  

---

## Standalone `.nori` fixtures (no Vite)

These compile with the CLI only — useful for learning the compiler:

```sh
cargo run -p nori -- compile examples/Counter.nori
cargo run -p nori -- compile examples/Todo.nori -o dist/
cargo run -p nori -- check examples/Todo.nori
```

| Fixture | Focus |
| --- | --- |
| `examples/Counter.nori` | `$state` + markup |
| `examples/Todo.nori` | list UI + types |
| `examples/DerivedEffect.nori` | `$derived` / `$effect` |
| `examples/ShadcnCard.nori` | richer markup |

---

## Module Federation stubs

Under `examples/framework-host` and `examples/framework-remote` are Rspack Module Federation config shapes that share `@nori/core` as a singleton. See their READMEs and [framework-api.md](./framework-api.md).

---

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `Cannot find module '@nori/core'` / empty dist | `bun run --cwd packages/core build` |
| Vite fails transforming `.nori` | Ensure Rust can build `nori` (`cargo build -p nori`); plugin shells to CLI if WASM isn’t built |
| Port already in use | Pass `-- --port 5180` after `dev`, or stop the other Vite process |
| TS example `tsc` errors on plugins | Keep `src/vite-env.d.ts`; run `bun run --cwd examples/app-ts typecheck` |
| App shows 404 | File route must be `src/routes/index.nori` for `/` (see `@nori/framework` `filePathToRoutePath`) |

---

## What you should see after compile

A route like:

```ts
const count = $state(0);
export default function Home() {
  return <p>{count.value}</p>;
}
```

becomes roughly:

```js
import { h, signal } from "@nori/core";
const count = signal(0);
export default function Home() {
  return h("p", null, () => count.value);
}
```

Then Vite serves that module to the browser; `@nori/core`’s `mount` / `h` render into `#app`.
