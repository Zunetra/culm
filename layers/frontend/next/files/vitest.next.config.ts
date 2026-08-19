import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Named after the layer rather than called vitest.config.ts, so a second
// frontend in the same workspace can bring its own without a collision.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/next/**/*.spec.tsx"],
  },
});
