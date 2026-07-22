# @nori/framework

App framework helpers for Nori: file-based routing, History API client router,
`load` helpers, nested layout stub, and Module Federation shared-scope utils.

## Install / link

Workspace package — import from `@nori/framework` after `bun install` at the repo root.

## ESM exports

```js
import {
  defineConfig,
  createApp,
  createRoutes,
  matchFileRoutes,
  filePathToRoutePath,
  defineLoad,
  composeLayouts,
  shareNoriCore,
  mountRemote,
} from "@nori/framework";
```

Subpath exports: `@nori/framework/router`, `@nori/framework/federation`,
`@nori/framework/define-config`.

## Tests

```bash
bun test packages/nori-framework
```
