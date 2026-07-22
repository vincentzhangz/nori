import { fileURLToPath } from "node:url";

export { default as loader } from "./loader.js";

const loaderPath = fileURLToPath(new URL("./loader.js", import.meta.url));

export class NoriRspackPlugin {
  constructor(options = {}) {
    this.options = options;
  }

  apply(compiler) {
    const runtimeImport = this.options.runtimeImport ?? "@nori/core";
    const include = this.options.include ?? /\.nori$/;

    compiler.options.module ??= {};
    compiler.options.module.rules ??= [];
    // Nori emits plain JS with h() calls — no JSX/SWC React transform.
    compiler.options.module.rules.push({
      test: include,
      type: "javascript/auto",
      use: [
        {
          loader: loaderPath,
          options: { runtimeImport }
        }
      ]
    });
  }
}

export default function nori(options) {
  return new NoriRspackPlugin(options);
}
