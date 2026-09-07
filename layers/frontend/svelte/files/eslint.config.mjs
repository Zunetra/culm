import pluginSvelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";

/* Two entries rather than one, because a single file extension cannot be
 * parsed two ways. The .svelte entry needs svelte-eslint-parser to reach the
 * script block, and typescript-eslint cannot parse a component file.
 * svelte-eslint-parser only unwraps the component; it does not infer
 * `lang="ts"` on its own, so the TypeScript parser has to be named explicitly. */
export default [
  ...tseslint.config({
    files: [
      "{{app_dir}}/**/*.ts",
      "{{test_dir}}/**/*.ts",
      "vite{{config_suffix}}.config.ts",
      "vitest{{config_suffix}}.config.ts",
    ],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginSvelte.configs["flat/recommended"].map((entry) => ({
    ...entry,
    files: ["{{app_dir}}/**/*.svelte"],
    languageOptions: {
      ...entry.languageOptions,
      parserOptions: {
        ...entry.languageOptions?.parserOptions,
        parser: tseslint.parser,
      },
    },
  })),
];
