import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

// Named after the layer rather than called vite.config.ts, so a workspace
// holding a second Vite frontend can bring its own. The dev script passes this
// file with --config.
export default defineConfig({
  plugins: [react()],
  root: fileURLToPath(new URL("./src/react-vite", import.meta.url)),
  build: {
    outDir: fileURLToPath(new URL("./dist/react-vite", import.meta.url)),
    emptyOutDir: true,
  },
});
