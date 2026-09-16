import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Sits at this app's own root (the same directory as its package.json), so the
// alias below matches the "@/*" path tsconfig.json declares from app/.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL(".", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/**/*.spec.tsx"],
  },
});
