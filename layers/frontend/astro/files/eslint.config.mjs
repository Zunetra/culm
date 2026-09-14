import pluginAstro from "eslint-plugin-astro";
import tseslint from "typescript-eslint";

/* eslint-plugin-astro finds typescript-eslint's parser on its own when it is a
 * project dependency, which this layer already installs, so the frontmatter
 * inside a .astro file is parsed as TypeScript with no extra wiring here.
 * The plugin's own recommended config is spread unchanged rather than mapped
 * over: two of its entries scope the virtual `*.astro/*.ts` and `*.astro/*.js`
 * sub-files a processor produces, and forcing a `files` override onto every
 * entry (to scope this workspace's own paths) makes those virtual-file
 * entries match real .astro files too, so their own parser then runs last and
 * clobbers the frontmatter parser the plugin's base entry set. */
export default [
  ...tseslint.config({
    files: ["{{app_dir}}/**/*.ts"],
    extends: [...tseslint.configs.recommended],
  }),
  ...pluginAstro.configs["flat/recommended"],
];
