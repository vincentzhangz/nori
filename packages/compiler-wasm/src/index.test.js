import { expect, test } from "bun:test";
import { compile, hasWasmBuild } from "./index.js";

test("compile falls back to CLI and emits h() calls", async () => {
  const js = await compile(
    "export default function C() { return <p>hi</p>; }",
    { filename: "C.nori" }
  );
  expect(js).toContain("h(");
  expect(js).toContain('"p"');
}, 120_000);

test("hasWasmBuild reflects pkg presence", () => {
  expect(typeof hasWasmBuild()).toBe("boolean");
});
