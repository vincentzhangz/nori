/**
 * Remote widget — exposes `./App` and shares `@nori/core` as a singleton.
 *
 * Run (once deps are installed):
 *   bunx rsbuild dev --config examples/framework-remote/rsbuild.config.mjs
 */
import { ModuleFederationPlugin } from "@module-federation/enhanced/rspack";
import { shareNoriCore } from "@nori/framework";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { NoriRspackPlugin } from "../../packages/rspack-plugin/src/index.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(__dirname, "../..");

/** @type {import('@rsbuild/core').RsbuildConfig} */
export default {
  source: {
    entry: {
      index: "./src/expose.js"
    },
    alias: {
      "@nori/core": resolve(workspaceRoot, "packages/core/src/index.ts"),
      "@nori/framework": resolve(workspaceRoot, "packages/nori-framework/src/index.js")
    }
  },
  server: {
    port: 3002
  },
  tools: {
    rspack: {
      plugins: [
        new NoriRspackPlugin(),
        new ModuleFederationPlugin({
          name: "framework_remote",
          filename: "remoteEntry.js",
          exposes: {
            "./App": "./src/expose.js"
          },
          shared: {
            ...shareNoriCore({ singleton: true })
          }
        })
      ]
    }
  }
};
