/**
 * Minimal ambient lib stub for nori-checker M7.
 * The checker currently hardcodes these globals via `lib_es5_globals()`;
 * this file documents the intended surface and can be parsed later.
 */

interface Array<T> {
  length: number;
  [n: number]: T;
}

interface ReadonlyArray<T> {
  length: number;
  readonly [n: number]: T;
}

interface Promise<T> {
  then<TResult>(
    onfulfilled?: (value: T) => TResult,
    onrejected?: (reason: any) => TResult
  ): Promise<TResult>;
}

interface String {
  length: number;
}

interface Number {}
interface Boolean {}
interface Object {}
interface Function {}
interface Symbol {}
interface Date {}
interface RegExp {}
interface Error {
  message: string;
}

declare var Array: {
  new <T>(...items: T[]): Array<T>;
};
declare var Promise: {
  new <T>(executor: (resolve: (value: T) => void, reject: (reason?: any) => void) => void): Promise<T>;
};
declare var String: {
  new (value?: any): String;
};
