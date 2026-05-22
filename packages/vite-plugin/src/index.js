import { runNoriStdin } from "../../cli/src/index.js";
import { transformWithOxc } from "vite";

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

      const compiled = runNoriStdin(code, [
        "--runtime-import",
        runtimeImport,
        id
      ]);
      return transformWithOxc(compiled, id, {
        lang: "jsx",
        jsx: { runtime: "classic" }
      });
    }
  };
}
