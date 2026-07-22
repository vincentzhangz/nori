import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { runNoriStdin } from "../../cli/src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const wasmEntry = resolve(here, "../pkg/nori_wasm.js");

let wasmCompile = null;
let wasmLoadAttempted = false;

async function loadWasm() {
  if (wasmLoadAttempted) {
    return wasmCompile;
  }
  wasmLoadAttempted = true;
  if (!existsSync(wasmEntry)) {
    return null;
  }
  try {
    const mod = await import(wasmEntry);
    wasmCompile = mod.compileWithRuntime ?? mod.compile ?? null;
  } catch {
    wasmCompile = null;
  }
  return wasmCompile;
}

/**
 * Compile Nori source to JavaScript.
 * Prefers WASM bindings when `pkg/` exists; otherwise falls back to the CLI.
 *
 * @param {string} source
 * @param {{ runtimeImport?: string, filename?: string }} [options]
 * @returns {Promise<string>}
 */
export async function compile(source, options = {}) {
  const runtimeImport = options.runtimeImport ?? "@nori/core";
  const filename = options.filename ?? "input.nori";

  const wasm = await loadWasm();
  if (typeof wasm === "function") {
    try {
      if (wasm.length >= 2 || wasm.name === "compileWithRuntime") {
        return await wasm(source, runtimeImport);
      }
      return await wasm(source);
    } catch {
      // Fall through to CLI if the wasm artifact is stale/broken.
    }
  }

  return runNoriStdin(source, ["--runtime-import", runtimeImport, filename]);
}

/** True when a built `pkg/` WASM module is present on disk. */
export function hasWasmBuild() {
  return existsSync(wasmEntry);
}

export default { compile, hasWasmBuild };
