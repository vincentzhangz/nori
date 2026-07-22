# nori-bundler

Nori-native bundler: relative resolve, module graph, concatenated ESM emit, and
Module Federation protocol helpers. Eventually replaces Rspack for
`nori dev` / `nori build`.

## CLI

```bash
# Preferred — via the nori binary
cargo run -p nori -- bundle path/to/entry.js
cargo run -p nori -- bundle path/to/entry.js -o out.js
cargo run -p nori -- bundle path/to/entry.js --multi-file

# Library / tests
cargo test -p nori-bundler
cargo check -p nori-bundler
```

## Current surface

| API | Status |
| --- | --- |
| `ModuleGraph` / `Module` / `ModuleId` | Recursive relative graph |
| `resolve(specifier, from)` | Relative `./` `../` + extensions + `index.*` |
| `collect_imports` | `nori-parser` first, regex fallback |
| `bundle` / `bundle_with_options` | Walk graph → concat or multi-file emit |
| `RemoteEntry` / `SharedScope` / `create_remote_entry` | MF protocol types + helper |

## Roadmap (replace Rspack)

1. **Module graph** — full static import/export analysis; CSS and asset side effects.
2. **Dev server** — ESM unbundled serving with transform middleware.
3. **HMR** — fine-grained invalidate by module id.
4. **Production** — chunk splitting, tree-shaking, scope hoisting, minifier.
5. **Module Federation** — emit `remoteEntry` compatible with MF 2.0.
6. **Swap** — `nori build` stops shelling to Rspack.

## Non-goals (current milestone)

- Full Node resolution / `exports` maps / `node_modules` walk
- Sourcemaps
- CSS pipeline
- Scope-hoisted dedupe of imports (concatenation is ordered source paste)
