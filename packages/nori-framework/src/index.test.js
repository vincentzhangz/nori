import { expect, test } from "bun:test";
import {
  compilePath,
  composeLayouts,
  createRoutes,
  defineConfig,
  defineLoad,
  filePathToRoutePath,
  matchFileRoutes,
  matchRoute,
  shareNoriCore
} from "./index.js";

test("defineConfig fills defaults", () => {
  expect(defineConfig()).toEqual({
    routesDir: "src/routes",
    runtimeImport: "@nori/core"
  });
  expect(defineConfig({ name: "app", routesDir: "routes" }).routesDir).toBe("routes");
});

test("filePathToRoutePath maps routes/**/*.nori", () => {
  expect(filePathToRoutePath("routes/index.nori")).toBe("/");
  expect(filePathToRoutePath("src/routes/index.nori")).toBe("/");
  expect(filePathToRoutePath("./routes/blog/[slug].nori")).toBe("/blog/[slug]");
  expect(filePathToRoutePath("/app/routes/blog/index.nori")).toBe("/blog");
  expect(filePathToRoutePath("routes/layout.nori")).toBeNull();
  expect(filePathToRoutePath("routes/blog/layout.nori")).toBeNull();
});

test("matchFileRoutes builds sorted records", () => {
  const routes = matchFileRoutes({
    "./routes/blog/[slug].nori": async () => ({ default: () => null }),
    "./routes/index.nori": async () => ({ default: () => null }),
    "./routes/layout.nori": async () => ({ default: () => null })
  });

  expect(routes.map((r) => r.path)).toEqual(["/", "/blog/[slug]"]);
  expect(matchRoute(routes, "/blog/hello")?.params).toEqual({ slug: "hello" });
});

test("compilePath and createRoutes match params", () => {
  const routes = createRoutes({
    "/users/[id]": async () => ({ default: () => null })
  });
  const compiled = compilePath("/users/[id]");
  expect(compiled.paramNames).toEqual(["id"]);
  expect(matchRoute(routes, "/users/42")?.params.id).toBe("42");
});

test("defineLoad is an identity helper", async () => {
  const load = defineLoad(async ({ params }) => ({ id: params.id }));
  const data = await load({
    params: { id: "1" },
    url: new URL("http://localhost/"),
    fetch
  });
  expect(data).toEqual({ id: "1" });
});

test("composeLayouts wraps page outside-in", () => {
  const page = () => "page";
  const outer = ({ children }) => `outer(${children})`;
  const inner = ({ children }) => `inner(${children})`;
  const composed = composeLayouts(page, [outer, inner]);
  expect(typeof composed).toBe("function");
  expect(composed()).toBe("outer(inner(page))");
});

test("shareNoriCore marks singleton", () => {
  expect(shareNoriCore()).toEqual({
    "@nori/core": {
      singleton: true,
      requiredVersion: false,
      eager: false
    }
  });
});
