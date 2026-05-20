import { expect, test } from "bun:test";
import { batch, computed, effect, signal } from "./index";

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
