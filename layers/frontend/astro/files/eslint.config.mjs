import pluginAstro from "eslint-plugin-astro";
import tseslint from "typescript-eslint";

/* Two entries rather than one, because a single file extension cannot be
 * parsed two ways. The .astro entry needs astro-eslint-parser to reach the
 * frontmatter, and typescript-eslint cannot parse a component file. */
export default [
  ...tseslint.config({
    files: [
      "src/astro/**/*.ts",
      "test/astro/**/*.ts",
      "vitest.astro.config.ts",
    ],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginAstro.configs["flat/recommended"].map((entry) => ({
    ...entry,
    files: ["src/astro/**/*.astro"],
  })),
];
