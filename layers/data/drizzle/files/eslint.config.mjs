import tseslint from "typescript-eslint";

/* Claimed by explicit globs rather than left to the default match, which covers
 * only .js, .mjs and .cjs. Without this the schema and the config are not
 * linted at all and `lint` still exits 0, which is worse than no linter. */
export default tseslint.config({
  files: ["src/drizzle/**/*.ts", "drizzle.config.ts"],
  extends: [...tseslint.configs.recommended],
});
