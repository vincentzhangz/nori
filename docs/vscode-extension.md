# VS Code extension (Nori)

The Nori editor extension lives in [`extensions/nori`](../extensions/nori).

## What you get

- Syntax highlighting for `.nori` (TSX grammar + `$state` / `$derived` / `$effect`)
- Format Document / Format on Save (Prettier TypeScript parser)
- Language config: comments, brackets, indentation

## Run it while developing Nori

```sh
cd extensions/nori
bun install
```

From the **repo root** in VS Code or Cursor:

1. Open **Run and Debug**
2. Choose **Run Nori Extension**
3. Press **F5**

An Extension Development Host opens with the extension loaded. Open e.g. `examples/app/src/routes/index.nori` and try Format Document.

## Install into your normal editor

```sh
# from repo root (after: bun install --cwd extensions/nori)
bun run package
```

Or from the extension folder:

```sh
cd extensions/nori
bun install
bun run package
```

Then install the generated `.vsix` (includes Prettier):

```sh
code --install-extension nori-0.1.0.vsix
# or in Cursor:
cursor --install-extension nori-0.1.0.vsix
```

## Settings

See [extensions/nori/README.md](../extensions/nori/README.md). Repo defaults in [`.vscode/settings.json`](../.vscode/settings.json) enable format-on-save for `[nori]`.
