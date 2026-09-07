import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// Sources live under {{app_dir}}, the directory the scaffolder hands this
// layer, so a workspace holding a second frontend has somewhere of its own to
// put them. SvelteKit resolves routes, lib and the app template inside
// `files`, so they all move together.
//
// outDir is resolved against this file's own location, and this file is
// always written to the workspace root (see layer.toml), never into app_dir
// itself, so a plain ".svelte-kit" reaches the root regardless of how many
// segments app_dir has - no relative count needed here at all.
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    files: {
      assets: "static",
      lib: "{{app_dir}}/lib",
      routes: "{{app_dir}}/routes",
      appTemplate: "{{app_dir}}/app.html",
    },
    outDir: ".svelte-kit",
  },
};
