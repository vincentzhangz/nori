import { runNoriStdin } from "../../cli/src/index.js";

export class NoriRspackPlugin {
  constructor(options = {}) {
    this.options = options;
  }

  apply(compiler) {
    const pluginName = "NoriRspackPlugin";
    const runtimeImport = this.options.runtimeImport ?? "@nori/core";
    const include = this.options.include ?? /\.nori$/;

    compiler.options.module.rules.push({
      test: include,
      use: {
        loader: require.resolve("./loader.js"),
        options: { runtimeImport }
      }
    });
  }
}

export function loader(source) {
  const options = this.getOptions() || {};
  const runtimeImport = options.runtimeImport ?? "@nori/core";

  try {
    const result = runNoriStdin(source, [
      "--runtime-import",
      runtimeImport,
      this.resourcePath || "input.nori"
    ]);
    return result;
  } catch (error) {
    throw new Error(`Nori loader failed: ${error.message}`);
  }
}

export default function nori(options) {
  return new NoriRspackPlugin(options);
}
