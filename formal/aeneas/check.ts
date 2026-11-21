#!/usr/bin/env bun
/** 実ソースを再生成して証明する。--mutation-test は一時コピーだけを壊す。 */

import {
  copyFileSync,
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const PROOF = import.meta.dir;
const REPO = resolve(PROOF, "../..");
const AENEAS_REV = "2e3d4488280bb8a95411db81019a7854a784861f";
const AENEAS_VERSION = "aeneas nightly-2026.11.21-2e3d448";
const CHARON_VERSION = "0.1.322 (9e193f0317dacc822cc864c1798b117c70dcf7df)";
const RUST_TOOLCHAIN = "nightly-2026-09-17";
const DATA = process.env.XDG_DATA_HOME ?? join(homedir(), ".local/share");
const TOOLS = resolve(process.env.AENEAS_HOME ?? join(DATA, "nostos-formal/aeneas-2e3d448"));
const METHODS = ["is_done", "is_reborn", "is_failed", "done", "reborn", "failed"] as const;
type Method = typeof METHODS[number];

type RunOptions = {
  cwd: string;
  capture?: boolean;
  check?: boolean;
  env?: Record<string, string | undefined>;
};

function run(args: string[], { cwd, capture = false, check = true, env = process.env }: RunOptions) {
  const result = Bun.spawnSync(args, {
    cwd,
    env,
    stdout: capture ? "pipe" : "inherit",
    stderr: capture ? "pipe" : "inherit",
  });
  const output = capture ? (result.stdout?.toString() ?? "") + (result.stderr?.toString() ?? "") : "";
  if (result.signalCode || (check && !result.success)) {
    throw new Error(`Command failed (${result.signalCode ?? result.exitCode}): ${args.join(" ")}\n${output}`);
  }
  return { exitCode: result.exitCode, output };
}

function withTempDir(prefix: string, action: (directory: string) => void) {
  const directory = mkdtempSync(prefix);
  try {
    action(directory);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function checkTools() {
  const versions: [string[], string][] = [
    [[join(TOOLS, "aeneas"), "-version"], AENEAS_VERSION],
    [[join(TOOLS, "charon"), "version"], CHARON_VERSION],
    [[join(TOOLS, "charon"), "toolchain-version"], RUST_TOOLCHAIN],
  ];
  for (const [command, expected] of versions) {
    const actual = run(command, { cwd: PROOF, capture: true }).output.trim();
    if (actual !== expected) {
      throw new Error(`Tool version mismatch: ${JSON.stringify(actual)}; expected ${JSON.stringify(expected)}`);
    }
  }
  const manifest: { packages: { name: string; rev: string }[] } =
    JSON.parse(readFileSync(join(PROOF, "lake-manifest.json"), "utf8"));
  const dependency = manifest.packages.find((entry) => entry.name === "aeneas");
  if (dependency?.rev !== AENEAS_REV) {
    throw new Error("Aeneas executable and Lean dependency must use the same revision");
  }
}

function extract(sourceRoot: string, proofRoot: string) {
  const cache = join(proofRoot, ".lake");
  mkdirSync(cache, { recursive: true });
  const llbc = join(cache, "nostos.llbc");
  // 古い抽出結果を再利用して成功したことにしない。
  rmSync(llbc, { force: true });
  run([
    join(TOOLS, "charon"), "cargo", "--preset=aeneas", "--sysroot", "default",
    ...METHODS.flatMap((method) => ["--start-from", `crate::outcome::Outcome::${method}`]),
    "--dest-file", llbc, "--", "--manifest-path",
    "crates/nostos-core/Cargo.toml", "--lib",
  ], { cwd: sourceRoot, env: { ...process.env, CARGO_TARGET_DIR: join(cache, "rust-target") } });
  if (!statSync(llbc, { throwIfNoEntry: false })?.isFile()) {
    throw new Error("Charon did not produce fresh LLBC");
  }
  withTempDir(join(cache, "extract-"), (generated) => {
    run([
      join(TOOLS, "aeneas"), "-backend", "lean", "-namespace", "NostosGenerated",
      "-dest", generated, "-use-lean-modules", "false", "-abort-on-error",
      "-warnings-as-errors", "-no-progress-bar", llbc,
    ], { cwd: sourceRoot });
    // 今回の出力だけを採用する。生成されなければ copyFileSync が失敗する。
    copyFileSync(join(generated, "Nostos.lean"), join(proofRoot, "Nostos.lean"));
  });
}

function prove(proofRoot: string, display = true) {
  // 生成定義をビルドした上で、手書きの証明は毎回検査する。
  run(["lake", "build", "Nostos"], { cwd: proofRoot });
  const result = run(
    ["lake", "env", "lean", "-DwarningAsError=true", "NostosProof.lean"],
    { cwd: proofRoot, capture: true, check: false },
  );
  if (display) process.stdout.write(result.output);
  return result;
}

function proofAxioms(output: string, method: Method) {
  const theorem = `${method}_matches_spec`;
  const match = output.match(new RegExp(`'NostosProof\\.${theorem}' depends on axioms: \\[([^\\]]*)\\]`));
  if (!match) {
    throw new Error(`Missing axiom report for ${theorem}`);
  }
  return match[1].split(",").map((name) => name.trim()).filter(Boolean);
}

function checkAxioms(output: string, methods: readonly Method[] = METHODS) {
  const allowed = new Set(["propext", "Quot.sound"]);
  for (const method of methods) {
    const axioms = proofAxioms(output, method);
    if (axioms.some((name) => !allowed.has(name))) {
      throw new Error(`Unexpected proof assumptions for ${method}: ${axioms.join(", ")}`);
    }
  }
}

function mutationTest() {
  const mutations: Record<Method, { from: string; to: string; cases: string[] }> = {
    is_done: {
      from: "matches!(self, Outcome::Done(_))",
      to: "matches!(self, Outcome::Reborn(_))",
      cases: ["Done", "Reborn"],
    },
    is_reborn: {
      from: "matches!(self, Outcome::Reborn(_))",
      to: "matches!(self, Outcome::Failed(_))",
      cases: ["Reborn", "Failed"],
    },
    is_failed: {
      from: "matches!(self, Outcome::Failed(_))",
      to: "matches!(self, Outcome::Done(_))",
      cases: ["Done", "Failed"],
    },
    done: {
      from: "Outcome::Done(o) => Some(o),",
      to: "Outcome::Done(_o) => None,",
      cases: ["Done"],
    },
    reborn: {
      from: "Outcome::Reborn(i) => Some(i),",
      to: "Outcome::Reborn(_i) => None,",
      cases: ["Reborn"],
    },
    failed: {
      from: "Outcome::Failed(e) => Some(e),",
      to: "Outcome::Failed(_e) => None,",
      cases: ["Failed"],
    },
  };
  const sourceFile = join(REPO, "crates/nostos-core/src/outcome.rs");
  const originalSource = readFileSync(sourceFile);
  const originalGenerated = readFileSync(join(PROOF, "Nostos.lean"));
  withTempDir(join(tmpdir(), "nostos-rust-mutation-"), (root) => {
    for (const name of ["Cargo.toml", "rust-toolchain.toml"]) {
      copyFileSync(join(REPO, name), join(root, name));
    }
    cpSync(join(REPO, "crates"), join(root, "crates"), { recursive: true, dereference: true });
    const source = join(root, "crates/nostos-core/src/outcome.rs");
    const trial = join(root, "proof");
    mkdirSync(trial);
    for (const name of ["lakefile.toml", "lake-manifest.json", "lean-toolchain", "NostosProof.lean"]) {
      copyFileSync(join(PROOF, name), join(trial, name));
    }
    mkdirSync(join(trial, ".lake"));
    // 依存ライブラリだけを共有し、生成定義と証明のビルド成果物は隔離する。
    symlinkSync(join(PROOF, ".lake/packages"), join(trial, ".lake/packages"), "dir");
    for (const method of METHODS) {
      const mutation = mutations[method];
      // 毎回、正しい元ソースから一つのメソッドだけを壊す。
      const text = originalSource.toString();
      if (text.split(mutation.from).length - 1 !== 1) {
        throw new Error(`Mutation target changed for ${method}; review the mutation test`);
      }
      writeFileSync(source, text.replace(mutation.from, mutation.to));
      console.log(`MUTATION: testing an intentionally broken temporary copy of ${method}`);
      extract(root, trial);
      if (readFileSync(join(trial, "Nostos.lean")).equals(originalGenerated)) {
        throw new Error(`Rust mutation of ${method} did not reach the generated Lean definition`);
      }
      const result = prove(trial, false);
      const log = join(PROOF, `.lake/mutation-${method}.log`);
      writeFileSync(log, result.output);
      const expectedFailure = [
        "NostosProof.lean:", "Tactic `rfl` failed",
        ...mutation.cases.map((variant) => `case ${variant}`),
      ].every((part) => result.output.includes(part));
      if (result.exitCode !== 1 || !expectedFailure
        || !proofAxioms(result.output, method).includes("sorryAx")) {
        throw new Error(`Expected a proof failure for ${method}, not a tool failure; see ${log}`);
      }
      // 対象外の5つの証明は成功したままでなければならない。
      checkAxioms(result.output, METHODS.filter((other) => other !== method));
      console.log(`PASS: intentionally broken ${method} was rejected (log: ${log})`);
    }
  });
  if (!readFileSync(sourceFile).equals(originalSource)
    || !readFileSync(join(PROOF, "Nostos.lean")).equals(originalGenerated)) {
    throw new Error("Mutation test changed the original source or generated definition");
  }
  console.log(`PASS: all ${METHODS.length} intentionally broken copies were rejected; original files are unchanged`);
}

function main() {
  const { values } = parseArgs({
    args: Bun.argv.slice(2),
    options: {
      "mutation-test": { type: "boolean" },
      help: { type: "boolean", short: "h" },
    },
    strict: true,
    allowPositionals: false,
  });
  if (values.help) {
    console.log("Usage: bun formal/aeneas/check.ts [--mutation-test]\n実ソースを再生成して証明する。--mutation-test は一時コピーだけを壊す。");
    return;
  }
  checkTools();
  extract(REPO, PROOF);
  const result = prove(PROOF);
  if (result.exitCode !== 0) {
    throw new Error(`Lean proof failed (exit ${result.exitCode})`);
  }
  checkAxioms(result.output);
  console.log(`PASS: current Rust matches all ${METHODS.length} specifications (${METHODS.join(", ")})`);
  if (values["mutation-test"]) mutationTest();
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
