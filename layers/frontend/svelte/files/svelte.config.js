import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// Sources live under src/svelte rather than at the root, so a workspace holding
// a second frontend has somewhere of its own to put them. SvelteKit resolves
// routes, lib and the app template inside `files`, so they all move together.
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    files: {
      assets: "static/svelte",
      lib: "src/svelte/lib",
      routes: "src/svelte/routes",
      appTemplate: "src/svelte/app.html",
    },
    outDir: ".svelte-kit",
  },
};
