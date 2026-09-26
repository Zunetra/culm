// Stages the npm packages for one release, run by release.yml:
//
//   node npm/stage.mjs <version> <dist-dir> <out-dir>
//
// Writes `<out-dir>/culm` (the launcher) and one `<out-dir>/culm-<platform>`
// per binary in <dist-dir>, every version set to <version>. The versions are
// culm's own release version, written here at release time, never kept in a
// source file that could drift from the tag.

import { chmodSync, copyFileSync, cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const [version, dist, out] = process.argv.slice(2);
if (!version || !dist || !out) {
  console.error("usage: node npm/stage.mjs <version> <dist-dir> <out-dir>");
  process.exit(2);
}

const here = dirname(fileURLToPath(import.meta.url));

// Rust target triple, npm platform key, and the `os`/`cpu` npm matches on.
const PLATFORMS = [
  ["x86_64-unknown-linux-gnu", "linux-x64", "linux", "x64"],
  ["aarch64-unknown-linux-gnu", "linux-arm64", "linux", "arm64"],
  ["x86_64-apple-darwin", "darwin-x64", "darwin", "x64"],
  ["aarch64-apple-darwin", "darwin-arm64", "darwin", "arm64"],
  ["x86_64-pc-windows-msvc", "win32-x64", "win32", "x64"],
];

const launcher = JSON.parse(readFileSync(join(here, "culm", "package.json"), "utf8"));
launcher.version = version;

for (const [target, key, os, cpu] of PLATFORMS) {
  const name = `@zunetra/culm-${key}`;
  const exe = os === "win32" ? ".exe" : "";
  const dir = join(out, `culm-${key}`);
  mkdirSync(join(dir, "bin"), { recursive: true });

  const binary = join(dir, "bin", `culm${exe}`);
  copyFileSync(join(dist, `culm-${target}${exe}`), binary);
  chmodSync(binary, 0o755);

  writeFileSync(
    join(dir, "package.json"),
    JSON.stringify(
      {
        name,
        version,
        description: `The culm binary for ${key}. Install \`culm\`, not this.`,
        license: "MIT",
        repository: launcher.repository,
        os: [os],
        cpu: [cpu],
        files: ["bin"],
        preferUnplugged: true,
      },
      null,
      2,
    ) + "\n",
  );
  launcher.optionalDependencies[name] = version;
}

cpSync(join(here, "culm"), join(out, "culm"), { recursive: true });
writeFileSync(join(out, "culm", "package.json"), JSON.stringify(launcher, null, 2) + "\n");
console.log(`staged culm ${version} and ${PLATFORMS.length} platform packages in ${out}`);
