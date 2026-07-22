# Runtime (`@nori/core`)

Package: `packages/core` → npm name **`@nori/core`**.

Build before running examples:

```sh
bun run --cwd packages/core build
```

## Reactivity

```ts
export function signal<T>(initial: T): NoriSignal<T>;
export function computed<T>(fn: () => T): NoriSignal<T>;
export function effect(fn: () => void): () => void;
export function batch(fn: () => void): void;
```

```ts
export interface NoriSignal<T> {
  value: T;
  get(): T;
  set(value: T): void;
}
```

Nori preserves `.value` in generated code. Reads subscribe effects; writes notify.

| Nori source | Emitted JS |
| --- | --- |
| `$state(0)` | `signal(0)` |
| `$derived(count.value * 2)` | `computed(() => count.value * 2)` |
| `$effect(() => { ... })` | `effect(() => { ... })` |

## Renderer

Markup is lowered to `h` (not React `createElement`):

```ts
export function h(
  tag: string | ((props: unknown) => unknown),
  props: Record<string, unknown> | null,
  ...children: unknown[]
): VNode;

export function fragment(...children: unknown[]): VNode;
export function text(value: unknown): VNode;

export function mount(
  component: unknown,
  el: Element,
  props?: Record<string, unknown>
): () => void;

export function hydrate(vnode: unknown, el: Element): () => void;
export function renderToString(vnode: unknown): string;
```

### Conventions

- **Events:** attributes are kept as authored (e.g. `onclick`). The runtime treats `on*` props as DOM listeners (`onclick` → `click`).
- **Fine-grained children:** codegen wraps dynamic children in `() => ...`. `mount` / `h` re-run those getters inside `effect` when signals change.
- **SSR:** `renderToString` evaluates signals once to HTML; `hydrate` attaches listeners/effects to existing DOM.

## Tests

```sh
bun test packages/core
# or from root:
bun test
```
