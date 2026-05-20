#!/usr/bin/env node
import { runNori } from "./index.js";

try {
  const output = runNori(process.argv.slice(2), { stdio: "inherit" });
  if (output) {
    process.stdout.write(output);
  }
} catch (error) {
  if (error.result?.stderr) {
    process.stderr.write(error.result.stderr);
  } else {
    process.stderr.write(`${error.message}\n`);
  }
  process.exit(error.result?.status ?? 1);
}
