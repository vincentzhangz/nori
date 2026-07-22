/// <reference types="vite/client" />

declare module "*.nori" {
  const component: unknown;
  export default component;
  export const load: ((event: unknown) => unknown) | undefined;
}

declare module "@nori/vite-plugin" {
  import type { Plugin } from "vite";
  export default function nori(options?: {
    include?: RegExp;
    runtimeImport?: string;
  }): Plugin;
}

declare module "@tailwindcss/vite" {
  import type { Plugin } from "vite";
  export default function tailwindcss(): Plugin;
}

declare module "@nori/framework" {
  export type RouteModule = {
    default: unknown;
    load?: (event: unknown) => unknown | Promise<unknown>;
  };

  export type RouteRecord = {
    id: string;
    path: string;
    pattern: RegExp;
    paramNames: string[];
    file?: string;
    module: () => Promise<RouteModule>;
    layouts?: Array<() => Promise<RouteModule>>;
  };

  export function matchFileRoutes(
    modules: Record<string, () => Promise<unknown>>
  ): RouteRecord[];

  export function createApp(options: {
    target: Element;
    routes: RouteRecord[];
    mount: (
      component: unknown,
      el: Element,
      props?: Record<string, unknown>
    ) => () => void;
    onNavigate?: (info: {
      pathname: string;
      params: Record<string, string>;
    }) => void;
  }): {
    navigate: (to: string, nav?: { replace?: boolean }) => Promise<void>;
    destroy: () => void;
  };
}
