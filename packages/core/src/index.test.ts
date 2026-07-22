import { expect, test, beforeEach, afterEach } from "bun:test";
import {
  batch,
  computed,
  effect,
  fragment,
  h,
  hydrate,
  mount,
  renderToString,
  signal,
  text
} from "./index";

/** Minimal DOM shim so mount/hydrate tests run under Bun without happy-dom. */
function installDomShim() {
  class FakeText {
    nodeType = 3;
    textContent: string;
    constructor(value = "") {
      this.textContent = value;
    }
  }

  class FakeElement {
    nodeType = 1;
    tagName: string;
    style: Record<string, string> = {};
    childNodes: Array<FakeElement | FakeText> = [];
    private attrs = new Map<string, string>();
    private listeners = new Map<string, Set<EventListener>>();

    constructor(tag: string) {
      this.tagName = tag.toUpperCase();
    }

    get children() {
      return this.childNodes.filter((n) => n.nodeType === 1) as FakeElement[];
    }

    replaceChildren(...nodes: Array<FakeElement | FakeText>) {
      this.childNodes = nodes;
    }

    appendChild(node: FakeElement | FakeText) {
      this.childNodes.push(node);
      return node;
    }

    insertBefore(node: FakeElement | FakeText, ref: FakeElement | FakeText | null) {
      const idx = ref ? this.childNodes.indexOf(ref) : -1;
      if (idx >= 0) {
        this.childNodes.splice(idx, 0, node);
      } else {
        this.childNodes.push(node);
      }
      return node;
    }

    setAttribute(name: string, value: string) {
      this.attrs.set(name, value);
    }

    removeAttribute(name: string) {
      this.attrs.delete(name);
    }

    getAttribute(name: string) {
      return this.attrs.get(name) ?? null;
    }

    addEventListener(type: string, handler: EventListener) {
      if (!this.listeners.has(type)) {
        this.listeners.set(type, new Set());
      }
      this.listeners.get(type)!.add(handler);
    }

    removeEventListener(type: string, handler: EventListener) {
      this.listeners.get(type)?.delete(handler);
    }

    dispatchEvent(type: string) {
      for (const handler of this.listeners.get(type) ?? []) {
        handler(new Event(type));
      }
    }

    get textContent() {
      return this.childNodes.map((n) => ("textContent" in n ? n.textContent : "")).join("");
    }
  }

  const previous = {
    document: globalThis.document,
    Element: (globalThis as { Element?: unknown }).Element,
    Text: (globalThis as { Text?: unknown }).Text,
    HTMLElement: (globalThis as { HTMLElement?: unknown }).HTMLElement
  };

  const doc = {
    createElement: (tag: string) => new FakeElement(tag),
    createTextNode: (value: string) => new FakeText(value)
  };

  Object.assign(globalThis, {
    document: doc,
    Element: FakeElement,
    Text: FakeText,
    HTMLElement: FakeElement
  });

  return {
    FakeElement,
    FakeText,
    restore() {
      Object.assign(globalThis, previous);
    }
  };
}

let dom: ReturnType<typeof installDomShim> | null = null;

beforeEach(() => {
  if (!(globalThis as { document?: { createElement?: unknown } }).document?.createElement) {
    dom = installDomShim();
  }
});

afterEach(() => {
  dom?.restore();
  dom = null;
});

test("signal exposes value, get, and set", () => {
  const count = signal(0);

  expect(count.value).toBe(0);
  count.value = 1;
  expect(count.get()).toBe(1);
  count.set(2);
  expect(count.value).toBe(2);
});

test("computed and effect react to signal updates", () => {
  const count = signal(1);
  const doubled = computed(() => count.value * 2);
  let observed = 0;

  effect(() => {
    observed = doubled.value;
  });

  expect(observed).toBe(2);
  count.value = 3;
  expect(observed).toBe(6);
});

test("batch flushes effects once", () => {
  const count = signal(0);
  let runs = 0;

  effect(() => {
    count.value;
    runs += 1;
  });

  batch(() => {
    count.value = 1;
    count.value = 2;
  });

  expect(runs).toBe(2);
});

test("h builds vnodes and renderToString serializes them", () => {
  const count = signal(1);
  const tree = h("div", { class: "box" }, h("p", null, () => text(count.value)), h(fragment, null, "done"));

  expect(renderToString(tree)).toBe('<div class="box"><p>1</p>done</div>');
  count.value = 2;
  expect(renderToString(tree)).toBe('<div class="box"><p>2</p>done</div>');
});

test("h supports fragments, spreads, and event props", () => {
  const handler = () => {};
  const extra = { id: "x", "data-role": "item" };
  const tree = h(
    fragment,
    null,
    h("button", { type: "button", onclick: handler, ...extra }, "Go"),
    h("span", { onClick: handler }, "alt")
  );

  expect(tree.tag).toBe(fragment);
  expect(tree.children).toHaveLength(2);
  const button = tree.children[0] as { props: Record<string, unknown> };
  expect(button.props.onclick).toBe(handler);
  expect(button.props.id).toBe("x");
  expect(renderToString(tree)).toBe('<button type="button" id="x" data-role="item">Go</button><span>alt</span>');
});

test("mount updates fine-grained function children when signals change", () => {
  const count = signal(0);
  const root = document.createElement("div");
  const dispose = mount(() => h("p", null, () => text(count.value)), root);

  expect(root.textContent).toBe("0");
  count.value = 7;
  expect(root.textContent).toBe("7");
  dispose();
  expect(root.childNodes.length).toBe(0);
});

test("mount binds onclick and onClick to the click event", () => {
  let clicks = 0;
  const root = document.createElement("div");
  mount(
    () =>
      h("div", null, h("button", { onclick: () => (clicks += 1) }, "a"), h("button", { onClick: () => (clicks += 1) }, "b")),
    root
  );

  const wrapper = root.childNodes[0] as { children: Array<{ dispatchEvent: (t: string) => void }> };
  wrapper.children[0].dispatchEvent("click");
  wrapper.children[1].dispatchEvent("click");
  expect(clicks).toBe(2);
});

test("hydrate attaches reactive effects to existing DOM", () => {
  const count = signal(1);
  const root = document.createElement("div");
  // Pretend SSR already wrote the markup.
  const p = document.createElement("p");
  p.appendChild(document.createTextNode("1"));
  root.appendChild(p);

  const dispose = hydrate(() => h("p", null, () => text(count.value)), root);
  expect(root.textContent).toBe("1");

  count.value = 9;
  expect(root.textContent).toBe("9");
  // Hydrate must not wipe the existing element node.
  expect(root.childNodes.length).toBe(1);
  dispose();
});

test("hydrate attaches event handlers without clearing DOM", () => {
  let clicks = 0;
  const root = document.createElement("div");
  const button = document.createElement("button");
  button.appendChild(document.createTextNode("Hi"));
  root.appendChild(button);

  hydrate(() => h("button", { onclick: () => (clicks += 1) }, "Hi"), root);
  (button as unknown as { dispatchEvent: (t: string) => void }).dispatchEvent("click");
  expect(clicks).toBe(1);
  expect(root.childNodes[0]).toBe(button);
});
