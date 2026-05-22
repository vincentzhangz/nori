import { describe, expect, it } from "bun:test";
import { basename } from "node:path";

import nori, { loader } from "./index.js";

function compiler() {
  return { options: {} };
}

function addedRule(plugin) {
  const mockCompiler = compiler();
  plugin.apply(mockCompiler);
  return mockCompiler.options.module.rules[0];
}

describe("rspack-plugin", () => {
  it("injects a complete default Nori rule", () => {
    const rule = addedRule(nori());
    const [swc, noriLoader] = rule.use;

    expect(rule.test.test("Counter.nori")).toBe(true);
    expect(rule.type).toBe("javascript/auto");
    expect(swc.loader).toBe("builtin:swc-loader");
    expect(swc.options.jsc.parser).toEqual({
      syntax: "ecmascript",
      jsx: true
    });
    expect(swc.options.jsc.transform.react.runtime).toBe("classic");
    expect(basename(noriLoader.loader)).toBe("loader.js");
    expect(noriLoader.options.runtimeImport).toBe("@nori/core");
  });

  it("passes custom rule options through to the loader", () => {
    const include = /components\/.*\.nori$/;
    const rule = addedRule(nori({ include, runtimeImport: "@/runtime" }));
    const [, noriLoader] = rule.use;

    expect(rule.test).toBe(include);
    expect(noriLoader.options.runtimeImport).toBe("@/runtime");
  });

  it("compiles source through the shared loader", () => {
    const code = loader.call(
      {
        getOptions: () => ({ runtimeImport: "@nori/core" }),
        resourcePath: "/fixtures/Counter.nori"
      },
      `
const count = $state(0);
export default function Counter() {
  return <p>{count.value}</p>;
}
`
    );

    expect(code).toContain("signal(0)");
    expect(code).toContain('from "@nori/core"');
    expect(code).toContain("export default function Counter()");
  });
});
