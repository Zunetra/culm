import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

// Named after the layer rather than called vite.config.ts, so a workspace
// holding a second Vite frontend can bring its own. The dev script passes this
// file with --config.
export default defineConfig({
  plugins: [vue()],
  root: fileURLToPath(new URL("./{{app_dir}}", import.meta.url)),
  build: {
    outDir: fileURLToPath(new URL("./dist/vue-vite", import.meta.url)),
    emptyOutDir: true,
  },
});
