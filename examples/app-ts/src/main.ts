import type { MountFn } from "./nori-shims";
import { mount } from "@nori/core";
import { createApp, matchFileRoutes } from "@nori/framework";
import "./styles.css";

/**
 * TypeScript example — typed host + Tailwind CSS.
 * `.nori` routes still go through the Nori compiler (types erased at emit).
 */
const modules = import.meta.glob("./routes/**/*.nori");
const routes = matchFileRoutes(modules);

const target = document.querySelector("#app");
if (!target) {
  throw new Error("missing #app mount target");
}

createApp({
  target,
  routes,
  mount: mount as MountFn
});
