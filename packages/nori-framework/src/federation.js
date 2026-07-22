/**
 * Module Federation helpers for Nori apps.
 *
 * Hosts share `@nori/core` as a singleton; remotes mount into named slots.
 */

/**
 * Shared scope config fragment for Rspack/Rsbuild Module Federation 2.0.
 * @param {{ singleton?: boolean, requiredVersion?: string | false, eager?: boolean }} [options]
 */
export function shareNoriCore(options = {}) {
  return {
    "@nori/core": {
      singleton: options.singleton ?? true,
      requiredVersion: options.requiredVersion ?? false,
      eager: options.eager ?? false
    }
  };
}

/**
 * Load a remote entry and mount its default export into a DOM slot.
 *
 * @param {string} name remote container name (e.g. "remote_app")
 * @param {Element} slot mount target
 * @param {{
 *   module?: string,
 *   mount?: (component: unknown, el: Element) => () => void,
 *   getContainer?: () => Promise<RemoteContainer>,
 *   shareScope?: object,
 * }} [options]
 *
 * @typedef {{
 *   init: (shareScope: object) => Promise<void> | void,
 *   get: (module: string) => Promise<() => unknown>,
 * }} RemoteContainer
 */
export async function mountRemote(name, slot, options = {}) {
  const moduleId = options.module ?? "./App";
  const shareScope = options.shareScope ?? {};
  const getContainer =
    options.getContainer ??
    (async () => {
      const container = globalThis[name];
      if (!container) {
        throw new Error(`Remote container "${name}" is not loaded on globalThis`);
      }
      return container;
    });

  const container = await getContainer();
  // Share scope is initialized by the host bundler; remotes still call init.
  await container.init(shareScope);
  const factory = await container.get(moduleId);
  const mod = factory();
  const component = mod?.default ?? mod;

  if (typeof options.mount === "function") {
    return options.mount(component, slot);
  }

  // Prefer @nori/core mount when available so remotes share one signal graph.
  try {
    const core = await import("@nori/core");
    if (typeof core.mount === "function") {
      return core.mount(component, slot);
    }
  } catch {
    // Fall through to a minimal DOM attach.
  }

  slot.replaceChildren();
  if (typeof component === "function") {
    const result = component(null);
    if (result instanceof Node) {
      slot.appendChild(result);
    } else if (result != null) {
      slot.textContent = String(result);
    }
  }

  return () => {
    slot.replaceChildren();
  };
}
