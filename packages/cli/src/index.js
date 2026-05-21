import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");

export function runNori(args, options = {}) {
	const command = resolveBinary();
	const result = spawnSync(command.bin, [...command.args, ...args], {
		cwd: command.cwd,
		encoding: "utf8",
		...options,
	});

	if (result.status !== 0) {
		const error = new Error(result.stderr || "nori command failed");
		error.result = result;
		throw error;
	}

	return result.stdout;
}

export function runNoriStdin(code, args = [], options = {}) {
	const command = resolveBinary();
	const fullArgs = ["compile", "--stdin", ...args];
	const result = spawnSync(command.bin, fullArgs, {
		cwd: command.cwd,
		encoding: "utf8",
		input: code,
		...options,
	});

	if (result.status !== 0) {
		const error = new Error(result.stderr || "nori command failed");
		error.result = result;
		throw error;
	}

	return result.stdout;
}

function resolveBinary() {
	const binaryName = `nori-${process.platform}-${process.arch}`;
	const extension = process.platform === "win32" ? ".exe" : "";
	const packagedBinary = resolve(here, "../bin", `${binaryName}${extension}`);

	if (existsSync(packagedBinary)) {
		return {
			bin: packagedBinary,
			args: [],
			cwd: process.cwd(),
		};
	}

	return {
		bin: "cargo",
		args: ["run", "--quiet", "--"],
		cwd: repoRoot,
	};
}
