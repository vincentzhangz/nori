// Advanced type forms — erased at emit time.
type Mapped<T> = { [K in keyof T]: T[K] };
type Cond<T> = T extends string ? "yes" : "no";
type Inf<T> = T extends infer U ? U : never;
type Keys = keyof { a: number; b: string };
type RO = readonly string[];
type Tid = `id-${string}`;

import type { Helper } from "./helper";
export type PublicId = string;

const label: string = "ok";
