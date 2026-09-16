import tseslint from "typescript-eslint";

/* Claimed by explicit globs rather than left to the default match, which covers
 * only .js, .mjs and .cjs. Without this the workspace's TypeScript is not
 * linted at all and `lint` still exits 0, which is worse than no linter. */
export default tseslint.config({
  files: ["{{app_dir}}/**/*.ts"],
  extends: [...tseslint.configs.recommended],
  rules: {
    /* Nest's dependency injection is declaration-driven, and a constructor
     * parameter that exists only to be injected reads as unused to a linter
     * that does not know about the decorator. */
    "@typescript-eslint/no-useless-constructor": "off",
  },
});
