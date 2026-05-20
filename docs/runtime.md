# Runtime

The runtime package lives in `packages/core` and is published as `@nori/core`.

It currently exports:

```ts
export function signal<T>(initial: T): NoriSignal<T>;
export function computed<T>(fn: () => T): NoriSignal<T>;
export function effect(fn: () => void): () => void;
export function batch(fn: () => void): void;
```

## Signal API

```ts
export interface NoriSignal<T> {
  value: T;
  get(): T;
  set(value: T): void;
}
```

Nori preserves `.value` reads and writes in compiled output.

Example:

```ts
const count = $state(0);
count.value += 1;
```

Compiles to:

```ts
const count = signal(0);
count.value += 1;
```

The runtime implements `.value` as an accessor, so reads can subscribe effects and writes can notify subscribers.

## Testing Runtime Behavior

Run:

```sh
bun test
```

The current tests cover:

- `signal.value`, `get`, and `set`.
- `computed`.
- `effect`.
- `batch`.
