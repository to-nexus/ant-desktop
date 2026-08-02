#!/usr/bin/env node
// The Tauri CLI refuses to build when an `@tauri-apps/*` npm package and its
// Rust crate disagree on major.minor. That check lives inside `tauri build`,
// which no CI job runs — so a lockfile-only npm bump stays green through PR CI
// and only fails when a release tag is pushed. That is exactly how v0.1.4 broke.
//
// This reproduces the check in under a second. Versions come from the INSTALLED
// packages, not from the `package.json` ranges: `^2` tells you nothing about
// what pnpm resolved.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

// `@tauri-apps/api` is the `tauri` crate; every `@tauri-apps/plugin-x` is
// `tauri-plugin-x`. Derived, so adding a plugin needs no edit here.
const crateFor = (npmName) => {
  const short = npmName.slice("@tauri-apps/".length);
  return short === "api" ? "tauri" : `tauri-${short}`;
};

// The CLI ships as an npm package with no crate in our dependency graph.
const NOT_A_CRATE = new Set(["cli"]);

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const npmNames = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies })
  .filter((n) => n.startsWith("@tauri-apps/"))
  .filter((n) => !NOT_A_CRATE.has(n.slice("@tauri-apps/".length)));

const installedVersion = (name) => {
  const manifest = join(root, "node_modules", name, "package.json");
  try {
    return JSON.parse(readFileSync(manifest, "utf8")).version;
  } catch {
    return null;
  }
};

const cargoLock = readFileSync(join(root, "src-tauri", "Cargo.lock"), "utf8");
const crateVersion = (crate) => {
  const m = cargoLock.match(
    new RegExp(`^name = "${crate}"\\nversion = "([^"]+)"$`, "m"),
  );
  return m ? m[1] : null;
};

const minor = (v) => v.split(".").slice(0, 2).join(".");

const problems = [];
for (const name of npmNames) {
  const crate = crateFor(name);
  const npmVersion = installedVersion(name);
  const rustVersion = crateVersion(crate);

  if (!npmVersion) {
    problems.push(`${name}: not installed — run \`pnpm install\` first`);
    continue;
  }
  if (!rustVersion) {
    problems.push(`${name}: no \`${crate}\` crate in src-tauri/Cargo.lock`);
    continue;
  }
  const ok = minor(npmVersion) === minor(rustVersion);
  console.log(
    `${ok ? "ok  " : "FAIL"}  ${name} ${npmVersion}  <->  ${crate} ${rustVersion}`,
  );
  if (!ok) {
    problems.push(
      `${name} ${npmVersion} vs ${crate} ${rustVersion} — major.minor must match`,
    );
  }
}

if (problems.length > 0) {
  console.error(`\nTauri npm/crate version parity failed:`);
  for (const p of problems) console.error(`  - ${p}`);
  console.error(
    `\nFix the Rust side with:\n  cd src-tauri && cargo update -p <crate> --precise <version>`,
  );
  process.exit(1);
}

console.log(`\n${npmNames.length} Tauri packages in sync.`);
