import { mount } from "@nori/core";
import { createApp, matchFileRoutes } from "@nori/framework";
import "./styles.css";

/** JavaScript example — vanilla CSS + .nori routes. */
const modules = import.meta.glob("./routes/**/*.nori");
const routes = matchFileRoutes(modules);

createApp({
  target: document.querySelector("#app"),
  routes,
  mount
});
