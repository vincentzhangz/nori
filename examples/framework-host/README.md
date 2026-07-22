# Framework host (Module Federation)

Shell app that consumes `framework_remote` and mounts it with `mountRemote`.

## Prerequisites

```bash
# from repo root — build core types for alias if needed
bun run --cwd packages/core build
```

Optional peer deps for a real Rsbuild run:

```bash
bun add -d @rsbuild/core @module-federation/enhanced
```

## Dev

```bash
# terminal 1 — remote first
bunx rsbuild dev --config examples/framework-remote/rsbuild.config.mjs

# terminal 2 — host
bunx rsbuild dev --config examples/framework-host/rsbuild.config.mjs
```

Open `http://localhost:3001`. The host loads `remoteEntry.js` from `:3002` and
calls `mountRemote("framework_remote", slot, { mount })`.

`@nori/core` is shared as a **singleton** via `shareNoriCore()` so both apps
share one signal graph.
