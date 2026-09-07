import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

/* Named after the layer rather than called vite.config.ts, so a workspace
 * holding a second Vite frontend can bring its own. The dev script passes this
 * file with --config. */
export default defineConfig({
  plugins: [react()],
  root: fileURLToPath(new URL("./{{app_dir}}", import.meta.url)),
  build: {
    outDir: fileURLToPath(new URL("./dist{{config_suffix}}", import.meta.url)),
    emptyOutDir: true,
  },
});
