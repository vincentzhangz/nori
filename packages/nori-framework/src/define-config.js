/**
 * Framework config helpers for Rsbuild / Rspack + Nori.
 */

/**
 * @typedef {{
 *   routesDir?: string,
 *   runtimeImport?: string,
 *   remotes?: Record<string, string>,
 *   exposes?: Record<string, string>,
 *   name?: string,
 *   shared?: Record<string, unknown>,
 * }} NoriConfig
 */

/**
 * Define a Nori app config (identity helper for tooling / type inference).
 * @param {NoriConfig} config
 * @returns {NoriConfig}
 */
export function defineConfig(config = {}) {
  return {
    routesDir: "src/routes",
    runtimeImport: "@nori/core",
    ...config
  };
}
