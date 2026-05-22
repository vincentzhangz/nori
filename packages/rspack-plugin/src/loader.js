import { runNoriStdin } from "../../cli/src/index.js";

export default function noriLoader(source) {
  const options = this.getOptions?.() ?? {};
  const runtimeImport = options.runtimeImport ?? "@nori/core";

  try {
    return runNoriStdin(source, [
      "--runtime-import",
      runtimeImport,
      this.resourcePath || "input.nori"
    ]);
  } catch (error) {
    throw new Error(`Nori loader failed: ${error.message}`);
  }
}
