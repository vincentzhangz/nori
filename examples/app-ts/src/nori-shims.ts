import type { RouteRecord } from "@nori/framework";

/** Minimal mount signature used by createApp. */
export type MountFn = (
  component: unknown,
  el: Element,
  props?: Record<string, unknown>
) => () => void;

export type AppRoutes = RouteRecord[];
