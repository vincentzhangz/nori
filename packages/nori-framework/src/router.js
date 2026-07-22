/**
 * File-based route helpers.
 *
 * Route modules are expected to export:
 * - `default` — page component
 * - optional `load(event)` — SvelteKit-style data loader
 * - optional `actions` — form actions map
 * - optional `layout` — nested layout wrapper (stub)
 */

/**
 * @typedef {{
 *   id: string,
 *   path: string,
 *   pattern: RegExp,
 *   paramNames: string[],
 *   file?: string,
 *   module: () => Promise<RouteModule>,
 *   layouts?: Array<() => Promise<RouteModule>>,
 * }} RouteRecord
 *
 * @typedef {{
 *   default: unknown,
 *   load?: (event: LoadEvent) => unknown | Promise<unknown>,
 *   actions?: Record<string, (event: ActionEvent) => unknown | Promise<unknown>>,
 *   layout?: unknown,
 * }} RouteModule
 *
 * @typedef {{
 *   params: Record<string, string>,
 *   url: URL,
 *   fetch: typeof fetch,
 * }} LoadEvent
 *
 * @typedef {LoadEvent & { request: Request }} ActionEvent
 *
 * @typedef {{
 *   params: Record<string, string>,
 *   data: unknown,
 *   children?: unknown,
 * }} PageProps
 */

/**
 * Convert a file-route path pattern like `/blog/[slug]` into a matcher.
 * @param {string} path
 */
export function compilePath(path) {
  const paramNames = [];
  const patternSource = path
    .split("/")
    .map((segment) => {
      if (segment.startsWith("[") && segment.endsWith("]")) {
        const name = segment.slice(1, -1);
        if (name.startsWith("...")) {
          paramNames.push(name.slice(3));
          return "(.*)";
        }
        paramNames.push(name);
        return "([^/]+)";
      }
      if (segment === "") {
        return "";
      }
      return segment.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    })
    .join("/");

  return {
    path,
    paramNames,
    pattern: new RegExp(`^${patternSource}/?$`)
  };
}

/**
 * Map a file path under `routes/` to a URL path.
 *
 * Examples:
 * - `routes/index.nori` → `/`
 * - `src/routes/index.nori` → `/`
 * - `routes/blog/[slug].nori` → `/blog/[slug]`
 * - `routes/blog/index.nori` → `/blog`
 * - `routes/layout.nori` → `null` (layout module, not a page)
 *
 * @param {string} filePath absolute or relative path containing `routes/`
 * @returns {string | null}
 */
export function filePathToRoutePath(filePath) {
  const normalized = filePath.replace(/\\/g, "/");
  const marker = "/routes/";
  const idx = normalized.lastIndexOf(marker);
  const relative =
    idx >= 0
      ? normalized.slice(idx + marker.length)
      : normalized.startsWith("routes/")
        ? normalized.slice("routes/".length)
        : null;

  if (relative == null) {
    return null;
  }

  const base = relative.replace(/\.nori$/i, "");
  if (base === "layout" || base.endsWith("/layout")) {
    return null;
  }

  const segments = base.split("/").filter(Boolean);
  if (segments.length === 0 || (segments.length === 1 && segments[0] === "index")) {
    return "/";
  }

  if (segments[segments.length - 1] === "index") {
    segments.pop();
  }

  return `/${segments.join("/")}`;
}

/**
 * Build route records from a Vite-style `import.meta.glob` map keyed by file path.
 *
 * @param {Record<string, () => Promise<RouteModule>>} modules
 *   Keys are file paths under routes/ (e.g. from import.meta.glob).
 * @returns {RouteRecord[]}
 */
export function matchFileRoutes(modules) {
  /** @type {RouteRecord[]} */
  const routes = [];

  for (const [file, loader] of Object.entries(modules)) {
    const path = filePathToRoutePath(file);
    if (path == null) {
      continue;
    }
    const compiled = compilePath(path);
    routes.push({
      id: path,
      path: compiled.path,
      pattern: compiled.pattern,
      paramNames: compiled.paramNames,
      file,
      module: loader,
      layouts: []
    });
  }

  // Longer (more specific) paths first; static before dynamic when equal length.
  routes.sort((a, b) => {
    const dyn = (p) => (p.includes("[") ? 1 : 0);
    if (dyn(a.path) !== dyn(b.path)) {
      return dyn(a.path) - dyn(b.path);
    }
    return b.path.length - a.path.length;
  });

  return routes;
}

/**
 * Build route records from a map of path → dynamic import.
 * @param {Record<string, () => Promise<RouteModule>>} modules
 * @returns {RouteRecord[]}
 */
export function createRoutes(modules) {
  return Object.entries(modules).map(([path, module]) => {
    const compiled = compilePath(path);
    return {
      id: path,
      path: compiled.path,
      pattern: compiled.pattern,
      paramNames: compiled.paramNames,
      module,
      layouts: []
    };
  });
}

/**
 * Match a pathname against route records.
 * @param {RouteRecord[]} routes
 * @param {string} pathname
 */
export function matchRoute(routes, pathname) {
  for (const route of routes) {
    const match = route.pattern.exec(pathname);
    if (!match) {
      continue;
    }
    /** @type {Record<string, string>} */
    const params = {};
    route.paramNames.forEach((name, index) => {
      params[name] = decodeURIComponent(match[index + 1] ?? "");
    });
    return { route, params };
  }
  return null;
}

/**
 * Type-helper identity for `load` functions (JSDoc / TS consumers).
 * @template T
 * @param {(event: LoadEvent) => T | Promise<T>} fn
 * @returns {(event: LoadEvent) => T | Promise<T>}
 */
export function defineLoad(fn) {
  return fn;
}

/**
 * Nested layout stub: wrap a page component with layout modules (outside-in).
 * Layouts receive `{ data, params, children }` where `children` is the inner page.
 *
 * @param {unknown} page
 * @param {unknown[]} layouts
 * @param {{ data?: unknown, params?: Record<string, string> }} [props]
 */
export function composeLayouts(page, layouts = [], props = {}) {
  return layouts.reduceRight((child, layout) => {
    if (typeof layout !== "function") {
      return child;
    }
    return () =>
      layout({
        ...props,
        children: typeof child === "function" ? child() : child
      });
  }, page);
}
