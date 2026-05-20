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
