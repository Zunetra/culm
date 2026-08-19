import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { includeIgnoreFile } from "@eslint/compat";
import js from "@eslint/js";

// Flat config. There is no .eslintignore: flat config removed it, and ignores
// live in the config itself.

const here = dirname(fileURLToPath(import.meta.url));

/* Ignores are inherited from .gitignore rather than restated here. Every layer
 * already declares its own build output there, so this file never has to learn
 * that Next writes .next/ or that Astro writes dist/. Restating them would make
 * this layer aware of another layer, which is the combination-authoring trap. */
const ignores = includeIgnoreFile(join(here, ".gitignore"));

/* On its own this config lints JavaScript and nothing else, because a flat
 * config entry only covers .js, .mjs and .cjs unless something claims more.
 * A layer whose sources ESLint would otherwise skip ships its own
 * eslint.<layer-id>.config.mjs beside this file and claims them there.
 *
 * Discovered rather than imported by name, so adding a framework later needs no
 * edit here and no edit in any sibling layer. Sorted so the emitted array is
 * the same on every machine, and each fragment scopes itself with `files`, so
 * fragments stay disjoint and their relative order cannot change a result. */
const fragments = [];
for (const entry of readdirSync(here).sort()) {
  if (!/^eslint\..+\.config\.mjs$/.test(entry)) {
    continue;
  }
  const fragment = pathToFileURL(join(here, entry)).href;
  const { default: config } = await import(fragment);
  fragments.push(...config);
}

export default [ignores, js.configs.recommended, ...fragments];
