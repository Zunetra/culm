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
      /* This app runs in a browser, and the template's error boundary calls
       * console.error directly rather than through an import, so ESLint has
       * no statement to see it come from. Named rather than pulled from a
       * browser globals package, because console is the only global the
       * shipped component calls; a component reaching for another one fails
       * loudly, which is cheaper than a whole environment's worth of names. */
      globals: {
        ...entry.languageOptions?.globals,
        console: "readonly",
      },
    },
  })),
  /* This workspace also runs Prettier, which reflows a .vue template's
   * whitespace and attribute layout on every save. Left alone,
   * flat/recommended's own layout rules disagree with Prettier's choices, and
   * lint and format:check would fight over the same lines forever. Applied
   * after recommended so it only turns those rules off, rather than replacing
   * the ruleset that also catches real defects. */
  {
    ...pluginVue.configs["no-layout-rules"],
    files: ["{{app_dir}}/**/*.vue"],
  },
];
