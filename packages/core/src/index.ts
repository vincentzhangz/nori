export interface NoriSignal<T> {
  value: T;
  get(): T;
  set(value: T): void;
}

type Subscriber = () => void;

let activeEffect: Subscriber | null = null;
let batchDepth = 0;
const pending = new Set<Subscriber>();

export function signal<T>(initial: T): NoriSignal<T> {
  let current = initial;
  const subscribers = new Set<Subscriber>();

  const notify = () => {
    for (const subscriber of subscribers) {
      if (batchDepth > 0) {
        pending.add(subscriber);
      } else {
        subscriber();
      }
    }
  };

  return {
    get value() {
      if (activeEffect) {
        subscribers.add(activeEffect);
      }
      return current;
    },
    set value(next: T) {
      if (Object.is(current, next)) {
        return;
      }
      current = next;
      notify();
    },
    get() {
      return this.value;
    },
    set(next: T) {
      this.value = next;
    }
  };
}

export function computed<T>(fn: () => T): NoriSignal<T> {
  const computedSignal = signal(fn());
  effect(() => {
    computedSignal.value = fn();
  });
  return computedSignal;
}

export function effect(fn: () => void): () => void {
  const runner = () => {
    const previous = activeEffect;
    activeEffect = runner;
    try {
      fn();
    } finally {
      activeEffect = previous;
    }
  };

  runner();
  return () => {
    pending.delete(runner);
  };
}

export function batch(fn: () => void): void {
  batchDepth += 1;
  try {
    fn();
  } finally {
    batchDepth -= 1;
    if (batchDepth === 0) {
      const queued = [...pending];
      pending.clear();
      for (const runner of queued) {
        runner();
      }
    }
  }
}

/** Sentinel tag for fragment vnodes. */
export const fragment = Symbol.for("nori.fragment");

export type Component = (props: Props | null) => Child;
export type Tag = string | Component | typeof fragment;
export type Props = Record<string, unknown>;
export type Child =
  | VNode
  | string
  | number
  | boolean
  | null
  | undefined
  | (() => Child)
  | Child[];

export interface VNode {
  tag: Tag;
  props: Props | null;
  children: Child[];
}

export function h(tag: Tag, props: Props | null, ...children: Child[]): VNode {
  return { tag, props, children: flattenChildren(children) };
}

export function text(value: unknown): string {
  if (value == null || value === false) {
    return "";
  }
  return String(value);
}

/**
 * Mount a vnode tree into `el`, replacing existing children.
 * Function children `() => signal.value` re-run via `effect` when signals change.
 */
export function mount(
  component: Component | (() => Child) | VNode | Child,
  el: Element
): () => void {
  const cleanups: Array<() => void> = [];
  el.replaceChildren();

  const vnode = resolveRenderable(component);
  if (vnode != null) {
    mountChild(el, vnode, cleanups);
  }

  return () => {
    for (const cleanup of cleanups.splice(0).reverse()) {
      cleanup();
    }
    el.replaceChildren();
  };
}

/**
 * Hydration stub: attach event handlers and reactive effects to existing DOM
 * produced by `renderToString`, without clearing `el`.
 *
 * Walks `el`'s child nodes in order against the vnode tree. Function children
 * bind to text nodes (or create one if missing) and subscribe via `effect`.
 */
export function hydrate(
  component: Component | (() => Child) | VNode | Child,
  el: Element
): () => void {
  const cleanups: Array<() => void> = [];
  const cursor = { index: 0 };
  const vnode = resolveRenderable(component);
  if (vnode != null) {
    hydrateChild(el, vnode, cleanups, cursor);
  }
  return () => {
    for (const cleanup of cleanups.splice(0).reverse()) {
      cleanup();
    }
  };
}

export function renderToString(vnode: Child): string {
  const child = resolveChild(vnode);
  if (child == null || child === false || child === true) {
    return "";
  }
  if (typeof child === "string" || typeof child === "number") {
    return escapeHtml(String(child));
  }
  if (Array.isArray(child)) {
    return child.map(renderToString).join("");
  }
  if (!isVNode(child)) {
    return "";
  }

  if (child.tag === fragment) {
    return child.children.map(renderToString).join("");
  }

  if (typeof child.tag === "function") {
    return renderToString(child.tag(child.props));
  }

  const tag = child.tag;
  const attrs = serializeAttrs(child.props);
  const inner = child.children.map(renderToString).join("");
  if (VOID_TAGS.has(tag)) {
    return `<${tag}${attrs} />`;
  }
  return `<${tag}${attrs}>${inner}</${tag}>`;
}

const VOID_TAGS = new Set([
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "param",
  "source",
  "track",
  "wbr"
]);

function resolveRenderable(
  component: Component | (() => Child) | VNode | Child
): Child {
  if (typeof component === "function") {
    return normalizeChild((component as Component)(null));
  }
  return normalizeChild(component);
}

function flattenChildren(children: Child[]): Child[] {
  const out: Child[] = [];
  for (const child of children) {
    if (Array.isArray(child)) {
      out.push(...flattenChildren(child));
    } else if (child !== undefined) {
      out.push(child);
    }
  }
  return out;
}

function isVNode(value: unknown): value is VNode {
  return (
    typeof value === "object" &&
    value !== null &&
    "tag" in value &&
    "children" in value
  );
}

function normalizeChild(child: Child): Child {
  return resolveChild(child);
}

function resolveChild(child: Child): Child {
  if (typeof child === "function") {
    return child();
  }
  return child;
}

function mountChild(parent: Node, child: Child, cleanups: Array<() => void>): void {
  if (child == null || child === false || child === true) {
    return;
  }

  if (typeof child === "function") {
    mountReactiveChild(parent, child, cleanups);
    return;
  }

  if (typeof child === "string" || typeof child === "number") {
    parent.appendChild(document.createTextNode(String(child)));
    return;
  }

  if (Array.isArray(child)) {
    for (const item of child) {
      mountChild(parent, item, cleanups);
    }
    return;
  }

  if (!isVNode(child)) {
    return;
  }

  if (child.tag === fragment) {
    for (const item of child.children) {
      mountChild(parent, item, cleanups);
    }
    return;
  }

  if (typeof child.tag === "function") {
    const result = child.tag(child.props);
    mountChild(parent, result, cleanups);
    return;
  }

  const el = document.createElement(child.tag);
  applyProps(el, child.props, cleanups);
  for (const item of child.children) {
    mountChild(el, item, cleanups);
  }
  parent.appendChild(el);
}

/**
 * Fine-grained child: subscribe to signals read inside the getter and update
 * a text node (common codegen shape: `() => signal.value`).
 */
function mountReactiveChild(
  parent: Node,
  getter: () => Child,
  cleanups: Array<() => void>
): void {
  const textNode = document.createTextNode("");
  parent.appendChild(textNode);
  const stop = effect(() => {
    textNode.textContent = text(getter());
  });
  cleanups.push(stop);
}

function hydrateChild(
  parent: Element | Node,
  child: Child,
  cleanups: Array<() => void>,
  cursor: { index: number }
): void {
  if (child == null || child === false || child === true) {
    return;
  }

  if (typeof child === "function") {
    hydrateReactiveChild(parent, child, cleanups, cursor);
    return;
  }

  if (typeof child === "string" || typeof child === "number") {
    // Static text already present from SSR — advance past matching text node.
    const node = parent.childNodes[cursor.index];
    if (node && node.nodeType === 3 /* TEXT_NODE */) {
      cursor.index += 1;
    }
    return;
  }

  if (Array.isArray(child)) {
    for (const item of child) {
      hydrateChild(parent, item, cleanups, cursor);
    }
    return;
  }

  if (!isVNode(child)) {
    return;
  }

  if (child.tag === fragment) {
    for (const item of child.children) {
      hydrateChild(parent, item, cleanups, cursor);
    }
    return;
  }

  if (typeof child.tag === "function") {
    hydrateChild(parent, child.tag(child.props), cleanups, cursor);
    return;
  }

  const node = parent.childNodes[cursor.index];
  cursor.index += 1;
  if (!node || node.nodeType !== 1 /* ELEMENT_NODE */) {
    return;
  }

  const el = node as Element;
  applyProps(el, child.props, cleanups);
  const childCursor = { index: 0 };
  for (const item of child.children) {
    hydrateChild(el, item, cleanups, childCursor);
  }
}

function hydrateReactiveChild(
  parent: Node,
  getter: () => Child,
  cleanups: Array<() => void>,
  cursor: { index: number }
): void {
  let textNode = parent.childNodes[cursor.index] as Text | null;
  if (!textNode || textNode.nodeType !== 3 /* TEXT_NODE */) {
    textNode = document.createTextNode("");
    if (parent.childNodes[cursor.index]) {
      parent.insertBefore(textNode, parent.childNodes[cursor.index]);
    } else {
      parent.appendChild(textNode);
    }
  }
  cursor.index += 1;

  const stop = effect(() => {
    textNode!.textContent = text(getter());
  });
  cleanups.push(stop);
}

/**
 * Event naming: codegen preserves source attribute names (`onclick`, `onClick`).
 * Both `onclick` and `onClick` bind to `addEventListener("click", ...)`.
 * Prefer lowercase DOM names (`onclick`) in `.nori` sources — they match HTML.
 */
function applyProps(
  el: Element,
  props: Props | null,
  cleanups: Array<() => void>
): void {
  if (!props) {
    return;
  }

  for (const [key, value] of Object.entries(props)) {
    if (key === "children" || key === "ref") {
      continue;
    }

    if (key.startsWith("on") && typeof value === "function") {
      const event = key.slice(2).toLowerCase();
      const handler = value as EventListener;
      el.addEventListener(event, handler);
      cleanups.push(() => el.removeEventListener(event, handler));
      continue;
    }

    if (typeof value === "function") {
      const stop = effect(() => {
        setDomProp(el, key, value());
      });
      cleanups.push(stop);
      continue;
    }

    setDomProp(el, key, value);
  }
}

function setDomProp(el: Element, key: string, value: unknown): void {
  if (value == null || value === false) {
    el.removeAttribute(key === "className" ? "class" : key);
    return;
  }

  if (key === "className" || key === "class") {
    el.setAttribute("class", String(value));
    return;
  }

  if (key === "style" && typeof value === "object") {
    Object.assign((el as HTMLElement).style, value);
    return;
  }

  if (key in el && key !== "list" && key !== "form" && key !== "type") {
    try {
      (el as unknown as Record<string, unknown>)[key] = value === true ? true : value;
      return;
    } catch {
      // fall through to setAttribute
    }
  }

  if (value === true) {
    el.setAttribute(key, "");
  } else {
    el.setAttribute(key, String(value));
  }
}

function serializeAttrs(props: Props | null): string {
  if (!props) {
    return "";
  }

  let out = "";
  for (const [key, raw] of Object.entries(props)) {
    if (key.startsWith("on") || key === "ref" || key === "children") {
      continue;
    }
    const value = typeof raw === "function" ? raw() : raw;
    if (value == null || value === false) {
      continue;
    }
    const name = key === "className" ? "class" : key;
    if (value === true) {
      out += ` ${name}`;
    } else {
      out += ` ${name}="${escapeHtml(String(value))}"`;
    }
  }
  return out;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
