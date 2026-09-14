import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    files: {
      assets: "static",
      lib: "lib",
      routes: "routes",
      appTemplate: "app.html",
    },
    outDir: ".svelte-kit",
  },
};
