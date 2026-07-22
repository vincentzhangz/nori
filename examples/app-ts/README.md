# Nori example app (TypeScript + Tailwind CSS)

**TypeScript** host + **Tailwind CSS v4** (`@tailwindcss/vite`) + `.nori` routes in `src/routes/` with type annotations (erased by the compiler).

Full guide: **[docs/examples.md](../../docs/examples.md)**

## Run

```bash
# from repo root
bun install
bun run --cwd packages/core build

bun run --cwd examples/app-ts typecheck
bun run --cwd examples/app-ts dev      # http://localhost:5174
bun run --cwd examples/app-ts build
```

JavaScript + vanilla CSS twin: [`../app`](../app)
