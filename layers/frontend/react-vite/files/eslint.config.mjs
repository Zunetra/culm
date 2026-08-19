import tseslint from "typescript-eslint";

/* Claimed by explicit globs rather than left to the default match, which covers
 * only .js, .mjs and .cjs. Without this the workspace's TypeScript is not
 * linted at all and `lint` still exits 0, which is worse than no linter.
 *
 * Every glob names this layer's own paths, so a second frontend in the same
 * workspace lints its files under its own fragment and the two never overlap. */
export default tseslint.config({
  files: [
    "src/react-vite/**/*.ts",
    "src/react-vite/**/*.tsx",
    "test/react-vite/**/*.ts",
    "test/react-vite/**/*.tsx",
    "vite.react-vite.config.ts",
    "vitest.react-vite.config.ts",
  ],
  extends: [...tseslint.configs.recommended],
});
