# Nori example app (JavaScript + vanilla CSS)

Plain **JavaScript** host + **vanilla CSS** (`src/styles.css`) + `.nori` routes in `src/routes/`.

Full guide: **[docs/examples.md](../../docs/examples.md)**

## Run

```bash
# from repo root
bun install
bun run --cwd packages/core build

bun run --cwd examples/app dev      # http://localhost:5173
bun run --cwd examples/app build
```

TypeScript + Tailwind twin: [`../app-ts`](../app-ts)
