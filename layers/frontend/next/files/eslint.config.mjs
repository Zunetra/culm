import tseslint from "typescript-eslint";

/* Claimed by explicit globs rather than left to the default match, which covers
 * only .js, .mjs and .cjs. Without this the workspace's TypeScript is not
 * linted at all and `lint` still exits 0, which is worse than no linter.
 *
 * Every glob names this layer's own paths, so a second frontend in the same
 * workspace lints its files under its own fragment and the two never overlap.
 * Disjoint globs are also what makes the load order of the fragments
 * irrelevant, which is the property the root config relies on. */
export default tseslint.config({
  files: [
    "src/app/**/*.ts",
    "src/app/**/*.tsx",
    "test/next/**/*.ts",
    "test/next/**/*.tsx",
    "next.config.ts",
    "vitest.next.config.ts",
  ],
  extends: [...tseslint.configs.recommended],
});
