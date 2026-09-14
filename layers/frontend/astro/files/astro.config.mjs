import { defineConfig } from "astro/config";

export default defineConfig({
  srcDir: ".",

  vite: {
    envPrefix: ["PUBLIC_"],
  },
});
