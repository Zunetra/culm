import tseslint from "typescript-eslint";

/* Claimed by explicit globs rather than left to the default match, which covers
 * only .js, .mjs and .cjs. Without this prisma.config.ts is not linted at all
 * and `lint` still exits 0, which is worse than no linter. */
export default tseslint.config({
  files: ["prisma.config.ts"],
  extends: [...tseslint.configs.recommended],
});
