import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";

/* Two entries rather than one, because a single file extension cannot be
 * parsed two ways. The .vue entry needs vue-eslint-parser to reach the script
 * block at all, and typescript-eslint cannot parse a single file component. */
export default [
  ...tseslint.config({
    files: [
      "src/vue-vite/**/*.ts",
      "test/vue-vite/**/*.ts",
      "vite.vue-vite.config.ts",
      "vitest.vue-vite.config.ts",
    ],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginVue.configs["flat/recommended"].map((entry) => ({
    ...entry,
    files: ["src/vue-vite/**/*.vue"],
  })),
];
