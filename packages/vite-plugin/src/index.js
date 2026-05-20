import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runNori } from "../../cli/src/index.js";

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

      const dir = mkdtempSync(join(tmpdir(), "nori-"));
      const input = join(dir, "input.nori");
      writeFileSync(input, code);
      try {
        const compiled = runNori([
          "compile",
          input,
          "--runtime-import",
          runtimeImport
        ]);
        return { code: compiled, map: null };
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    }
  };
}
