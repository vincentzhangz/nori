import { runNoriStdin } from "../../cli/src/index.js";

export function pitch(request) {
  const options = this.query?.runtimeImport
    ? { runtimeImport: this.query.runtimeImport }
    : {};

  const source = runNoriStdin(this.resourceQuery || request, [
    "--runtime-import",
    options.runtimeImport || "@nori/core",
    request
  ]);

  return source;
}

export default function noriLoader(source) {
  const options = this.query || {};
  const compiled = runNoriStdin(source, [
    "--runtime-import",
    options.runtimeImport || "@nori/core",
    this.resource
  ]);
  return compiled;
}