#!/usr/bin/env node
/**
 * Minimal compile-time harness.
 *
 * Measures wall-clock for compiling examples/Todo.nori via @nori/compiler
 * (WASM when pkg/ exists, else CLI fallback).
 *
 * Usage:
 *   node bench/scripts/measure-compile.mjs
 *   node bench/scripts/measure-compile.mjs --iterations 50
 */
import { performance } from "node:perf_hooks";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const iterations = Number(
  process.argv.includes("--iterations")
    ? process.argv[process.argv.indexOf("--iterations") + 1]
    : 20
);

const source = readFileSync(resolve(root, "examples/Todo.nori"), "utf8");

async function measureInProcess() {
  const { compile } = await import(
    resolve(root, "packages/compiler-wasm/src/index.js")
  );
  const samples = [];
  // warmup
  await compile(source, { filename: "Todo.nori" });
  for (let i = 0; i < iterations; i++) {
    const t0 = performance.now();
    await compile(source, { filename: "Todo.nori" });
    samples.push(performance.now() - t0);
  }
  return samples;
}

function measureCli() {
  const samples = [];
  for (let i = 0; i < iterations; i++) {
    const t0 = performance.now();
    const result = spawnSync(
      "cargo",
      ["run", "-q", "-p", "nori", "--", "compile", "examples/Todo.nori"],
      { cwd: root, encoding: "utf8" }
    );
    if (result.status !== 0) {
      throw new Error(result.stderr || "cargo compile failed");
    }
    samples.push(performance.now() - t0);
  }
  return samples;
}

function stats(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const sum = sorted.reduce((a, b) => a + b, 0);
  return {
    n: sorted.length,
    mean_ms: sum / sorted.length,
    p50_ms: sorted[Math.floor(sorted.length / 2)],
    min_ms: sorted[0],
    max_ms: sorted[sorted.length - 1]
  };
}

const report = {
  date: new Date().toISOString(),
  git: spawnSync("git", ["rev-parse", "--short", "HEAD"], {
    cwd: root,
    encoding: "utf8"
  }).stdout.trim(),
  iterations,
  in_process: null,
  cli: null
};

try {
  report.in_process = stats(await measureInProcess());
} catch (err) {
  report.in_process = { error: String(err) };
}

try {
  report.cli = stats(measureCli());
} catch (err) {
  report.cli = { error: String(err) };
}

const outDir = resolve(root, "bench/results");
mkdirSync(outDir, { recursive: true });
const outFile = resolve(outDir, `${report.date.slice(0, 10)}.json`);
writeFileSync(outFile, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
console.log(`wrote ${outFile}`);
