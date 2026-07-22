import { composeLayouts, matchRoute } from "./router.js";

/**
 * @typedef {import("./router.js").RouteRecord} RouteRecord
 * @typedef {import("./router.js").RouteModule} RouteModule
 */

/**
 * Create a client-side Nori app with History API navigation.
 *
 * @param {{
 *   target: Element,
 *   routes: RouteRecord[],
 *   mount: (component: unknown, el: Element, props?: Record<string, unknown>) => () => void,
 *   onNavigate?: (info: { pathname: string, params: Record<string, string> }) => void,
 * }} options
 */
export function createApp(options) {
  const { target, routes, mount, onNavigate } = options;
  let dispose = () => {};

  async function render(pathname = location.pathname) {
    const matched = matchRoute(routes, pathname);
    dispose();
    target.replaceChildren();

    if (!matched) {
      target.textContent = "404";
      return;
    }

    const mod = await matched.route.module();
    let data = undefined;
    if (typeof mod.load === "function") {
      data = await mod.load({
        params: matched.params,
        url: new URL(location.href),
        fetch
      });
    }

    /** @type {unknown[]} */
    const layoutFns = [];
    for (const layoutLoader of matched.route.layouts ?? []) {
      const layoutMod = await layoutLoader();
      if (typeof layoutMod.default === "function") {
        layoutFns.push(layoutMod.default);
      } else if (typeof layoutMod.layout === "function") {
        layoutFns.push(layoutMod.layout);
      }
    }

    const page = mod.default;
    const props = { data, params: matched.params };
    const component = composeLayouts(page, layoutFns, props);
    dispose = mount(component, target, props);
    onNavigate?.({ pathname, params: matched.params });
  }

  /**
   * Client navigate via History API.
   * @param {string} to
   * @param {{ replace?: boolean }} [nav]
   */
  function navigate(to, nav = {}) {
    const url = new URL(to, location.href);
    if (nav.replace) {
      history.replaceState({}, "", url);
    } else {
      history.pushState({}, "", url);
    }
    return render(url.pathname);
  }

  function onPopState() {
    void render(location.pathname);
  }

  window.addEventListener("popstate", onPopState);
  void render(location.pathname);

  return {
    navigate,
    destroy() {
      window.removeEventListener("popstate", onPopState);
      dispose();
    }
  };
}
