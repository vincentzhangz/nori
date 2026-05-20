import { readFileSync } from "node:fs";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runNori } from "../../cli/src/index.js";

export class NoriRspackPlugin {
  constructor(options = {}) {
    this.options = options;
  }

  apply(compiler) {
    const pluginName = "NoriRspackPlugin";
    const runtimeImport = this.options.runtimeImport ?? "@nori/core";
    const include = this.options.include ?? /\.nori$/;

    compiler.hooks.thisCompilation.tap(pluginName, (compilation) => {
      compilation.hooks.processAssets.tap(
        {
          name: pluginName,
          stage: compiler.webpack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONS
        },
        () => {
          for (const module of compilation.modules) {
            const resource = module.resource;
            if (!resource || !include.test(resource)) {
              continue;
            }
            const source = readFileSync(resource, "utf8");
            const dir = mkdtempSync(join(tmpdir(), "nori-"));
            const input = join(dir, "input.nori");
            writeFileSync(input, source);
            try {
              runNori(["compile", input, "--runtime-import", runtimeImport]);
            } finally {
              rmSync(dir, { recursive: true, force: true });
            }
          }
        }
      );
    });
  }
}

export default function nori(options) {
  return new NoriRspackPlugin(options);
}
