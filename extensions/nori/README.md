# Nori VS Code extension

Syntax highlighting and formatting for `.nori` files.

## Features

- **Language mode** `nori` for `.nori`
- **Syntax highlighting** based on TypeScript/TSX, plus emphasis for `$state` / `$derived` / `$effect`
- **Formatting** via Prettier (TypeScript/TSX parser) — Format Document, Format Selection, and Format on Save
- Bracket matching / comment toggling via language configuration

## Install (development)

From the repo root:

```sh
cd extensions/nori
bun install   # or: npm install
```

Then in VS Code / Cursor:

1. **File → Open Folder…** → `extensions/nori`  
   **or** open the Nori monorepo and run the launch config below
2. Press **F5** (*Run Extension*) to open an Extension Development Host
3. Open any `*.nori` file (for example `examples/app/src/routes/index.nori`)

### Workspace launch (monorepo)

A launch config lives at `.vscode/launch.json` in the repo root: **Run Nori Extension**.

### Optional: package a `.vsix`

```sh
cd extensions/nori
bun install
bun run package
# installs like: code --install-extension nori-0.1.0.vsix
```

## Settings

| Setting | Default | Meaning |
| --- | --- | --- |
| `nori.format.enable` | `true` | Turn the formatter on/off |
| `nori.format.printWidth` | `80` | Prettier width |
| `nori.format.tabWidth` | `2` | Indent size |
| `nori.format.singleQuote` | `false` | Prefer `'` |
| `nori.format.semi` | `true` | Semicolons |

Recommended in `.vscode/settings.json`:

```json
{
  "[nori]": {
    "editor.defaultFormatter": "nori.nori",
    "editor.formatOnSave": true
  }
}
```

(`nori.nori` is `publisher.name` from `package.json`.)

## Notes

- Highlighting embeds `source.tsx`, so JS/TS/markup in `.nori` share editor colors with TSX.
- Formatting is Prettier-based, not a Nori AST printer — it expects syntactically Prettier-friendly code (same as typical TSX). Invalid snippets may fail with an error toast.
