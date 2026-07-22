import { runNoriStdin } from "../../cli/src/index.js";

/**
 * Prefer in-process `@nori/compiler` (WASM when built, else CLI inside that
 * package). Only shell out here if the package cannot be resolved at all.
 */
async function compileWithNori(code, id, runtimeImport) {
  try {
    const compiler = await import("@nori/compiler");
    if (typeof compiler.compile === "function") {
      return compiler.compile(code, { runtimeImport, filename: id });
    }
  } catch {
    // Workspace may not have linked @nori/compiler yet.
  }

  return runNoriStdin(code, ["--runtime-import", runtimeImport, id]);
}

export default function nori(options = {}) {
  const include = options.include ?? /\.nori$/;
  const runtimeImport = options.runtimeImport ?? "@nori/core";

  return {
    name: "nori",
    enforce: "pre",
    async transform(code, id) {
      if (!include.test(id)) {
        return null;
      }

      // Output is plain JS with h() calls — no JSX transform needed.
      const compiled = await compileWithNori(code, id, runtimeImport);
      return {
        code: compiled,
        map: null
      };
    }
  };
}
