import pluginSvelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";

/* Two entries rather than one, because a single file extension cannot be
 * parsed two ways. The .svelte entry needs svelte-eslint-parser to reach the
 * script block, and typescript-eslint cannot parse a component file. */
export default [
  ...tseslint.config({
    files: [
      "src/svelte/**/*.ts",
      "test/svelte/**/*.ts",
      "vite.svelte.config.ts",
      "vitest.svelte.config.ts",
    ],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginSvelte.configs["flat/recommended"].map((entry) => ({
    ...entry,
    files: ["src/svelte/**/*.svelte"],
  })),
];
