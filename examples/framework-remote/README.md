# Framework remote (Module Federation)

Exposes `./App` (a `.nori` component) as a federated module.

## Dev

```bash
bunx rsbuild dev --config examples/framework-remote/rsbuild.config.mjs
```

Serves `remoteEntry.js` on `http://localhost:3002`. Shared `@nori/core` must
match the host's singleton (`shareNoriCore`).
