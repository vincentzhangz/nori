/**
 * Host shell — Module Federation 2.0 via Rspack `ModuleFederationPlugin`.
 *
 * Run (once deps are installed):
 *   bunx rsbuild dev --config examples/framework-host/rsbuild.config.mjs
 */
import { ModuleFederationPlugin } from "@module-federation/enhanced/rspack";
import { shareNoriCore } from "@nori/framework";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(__dirname, "../..");

/** @type {import('@rsbuild/core').RsbuildConfig} */
export default {
  source: {
    entry: {
      index: "./src/main.js"
    },
    alias: {
      "@nori/core": resolve(workspaceRoot, "packages/core/src/index.ts"),
      "@nori/framework": resolve(workspaceRoot, "packages/nori-framework/src/index.js")
    }
  },
  server: {
    port: 3001
  },
  tools: {
    rspack: {
      plugins: [
        new ModuleFederationPlugin({
          name: "framework_host",
          remotes: {
            framework_remote:
              "framework_remote@http://localhost:3002/remoteEntry.js"
          },
          shared: {
            ...shareNoriCore({ singleton: true })
          }
        })
      ]
    }
  }
};
