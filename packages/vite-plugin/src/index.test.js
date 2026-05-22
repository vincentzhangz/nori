import { describe, expect, it } from "bun:test";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { build } from "vite";

import nori from "./index.js";

function runNoriStdinTest(code, args = []) {
  const repoRoot = resolve(here, "../..");
  const result = spawnSync("cargo", ["run", "--quiet", "--", "compile", "--stdin", ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    input: code,
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || "nori command failed");
  }
  return result.stdout;
}

const here = resolve(import.meta.dirname);

describe("vite-plugin", () => {
  it("should compile nori code via stdin", () => {
    const source = `
const count = $state(0);
export default function Counter() {
  return <p>{count.value}</p>;
}
`;
    const result = runNoriStdinTest(source, ["--runtime-import", "@nori/core", "input.nori"]);
    expect(result).toContain("signal(0)");
    expect(result).toContain("export default function Counter()");
  });

  it("should inject runtime imports when using primitives", () => {
    const source = `
const count = $state(0);
const doubled = $derived(count.value * 2);
export default function Counter() {
  return <p>{doubled.value}</p>;
}
`;
    const result = runNoriStdinTest(source, ["--runtime-import", "@nori/core", "input.nori"]);
    expect(result).toContain('from "@nori/core"');
    expect(result).toContain("signal");
    expect(result).toContain("computed");
  });

  it("should strip type annotations", () => {
    const source = `
type Count = number;
const count: Count = $state(0);
export default function Counter(): JSX.Element {
  return <p>{count.value}</p>;
}
`;
    const result = runNoriStdinTest(source, ["--runtime-import", "@nori/core", "input.nori"]);
    expect(result).not.toContain("type Count");
    expect(result).not.toContain(": number");
    expect(result).not.toContain(": JSX.Element");
  });

  it("should build the example app through Vite", async () => {
    const exampleRoot = resolve(here, "../examples/counter-app");
    const result = await build({
      root: exampleRoot,
      configFile: false,
      logLevel: "silent",
      plugins: [nori()],
      resolve: {
        alias: {
          "@nori/core": resolve(here, "../../core/src/index.ts")
        }
      },
      build: {
        minify: false,
        write: false
      }
    });
    const chunks = (Array.isArray(result) ? result : [result]).flatMap((output) => output.output);
    const app = chunks.find((chunk) => chunk.type === "chunk" && chunk.isEntry);

    expect(app?.code).toContain("signal(0)");
    expect(app?.code).toContain("computed");
    expect(app?.code).toContain("Counter");
  });
});
