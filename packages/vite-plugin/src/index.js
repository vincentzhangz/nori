import { runNoriStdin } from "../../cli/src/index.js";

export default function nori(options = {}) {
  const include = options.include ?? /\.nori$/;
  const runtimeImport = options.runtimeImport ?? "@nori/core";

  return {
    name: "nori",
    enforce: "pre",
    transform(code, id) {
      if (!include.test(id)) {
        return null;
      }

      const compiled = runNoriStdin(code, [
        "--runtime-import",
        runtimeImport,
        "dummy.nori"
      ]);
      return { code: compiled, map: null };
    }
  };
}
