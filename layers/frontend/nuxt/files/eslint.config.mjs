import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";

/* Two entries rather than one, because a single file extension cannot be
 * parsed two ways. The .vue entry needs vue-eslint-parser to reach the script
 * block at all, and typescript-eslint cannot parse a single file component.
 * vue-eslint-parser only unwraps the SFC; it does not infer `lang="ts"` on its
 * own, so the TypeScript parser has to be named for the script block explicitly. */
export default [
  ...tseslint.config({
    files: ["{{app_dir}}/**/*.ts"],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginVue.configs["flat/recommended"].map((entry) => ({
    ...entry,
    files: ["{{app_dir}}/**/*.vue"],
    languageOptions: {
      ...entry.languageOptions,
      parserOptions: {
        ...entry.languageOptions?.parserOptions,
        parser: tseslint.parser,
      },
      /* Nuxt auto-imports these composables rather than having a page import
       * them, so ESLint has no import statement to see and calls them
       * undefined without this. Named rather than pulled from a Nuxt ESLint
       * module, because the shipped pages only ever call the composables
       * listed here; a page calling another one fails loudly, which is
       * cheaper than a module that has to run `nuxt prepare` to know what
       * exists. */
      globals: {
        ...entry.languageOptions?.globals,
        useHead: "readonly",
        useRuntimeConfig: "readonly",
        clearError: "readonly",
      },
    },
  })),
  {
    files: ["{{app_dir}}/**/*.vue"],
    rules: {
      /* Nuxt's file-based routing names pages and layouts for what they are
       * (`index.vue`, `default.vue`, `error.vue`), not for being multi-word. */
      "vue/multi-word-component-names": "off",
    },
  },
];
