# Framework API (`@nori/framework`)

Package: `packages/nori-framework`.

Nori aims for Next.js-class *capabilities* without cloning Next.js APIs. The compiler owns reactivity and `h()` lowering; the framework adds routing, data loading hooks, and Module Federation helpers.

## Design identity

| Capability | Inspiration | Nori direction |
| --- | --- | --- |
| Reactivity | Svelte | `$state` / `$derived` / `$effect` |
| Rendering | Solid | Fine-grained signals; `h()` |
| Data | SvelteKit / Remix | Co-located `load` (+ planned `actions`) |
| Routing | Next.js-class | File routes under `routes/` |
| Remotes | Module Federation 2.0 | Share `@nori/core` as singleton |

## Quick usage (matches the examples)

```js
import { mount } from "@nori/core";
import { createApp, matchFileRoutes } from "@nori/framework";

const routes = matchFileRoutes(import.meta.glob("./routes/**/*.nori"));

createApp({
  target: document.querySelector("#app"),
  routes,
  mount
});
```

See runnable apps in [examples.md](./examples.md).

## File routes

| File | URL |
| --- | --- |
| `src/routes/index.nori` | `/` |
| `src/routes/blog/[slug].nori` | `/blog/:slug` |
| `src/routes/blog/index.nori` | `/blog` |
| `src/routes/layout.nori` | layout only (not a page) |

Helpers: `filePathToRoutePath`, `matchFileRoutes`, `compilePath`, `matchRoute`, `composeLayouts`.

## Route module shape

```js
export async function load({ params, url, fetch }) {
  return { post: await fetch(`/api/${params.slug}`).then((r) => r.json()) };
}

export default function Page({ data, params }) {
  return <article>{data.post.title}</article>;
}
```

## Config

```js
import { defineConfig } from "@nori/framework";

export default defineConfig({
  routesDir: "src/routes",
  runtimeImport: "@nori/core",
  name: "my_app"
});
```

## Module Federation

```js
import { shareNoriCore, mountRemote } from "@nori/framework";

// rspack/rsbuild shared config
shared: {
  ...shareNoriCore({ singleton: true })
}

await mountRemote("remote_app", document.querySelector("#slot"), {
  module: "./App",
  mount: (component, el) => mount(component, el)
});
```

Stub apps: `examples/framework-host`, `examples/framework-remote`.

## Milestone status

| Milestone | Status |
| --- | --- |
| M1 file router + `createApp` + History API | Available (see examples) |
| M2 `nori dev` / `nori build` orchestration | Use Vite scripts for now |
| M3 SSR + hydration | Runtime APIs present; app wiring still thin |
| M4 nested layouts + `load` waterfall | Helpers stubbed; expand as needed |
| M5 Module Federation | Helpers + example configs |
