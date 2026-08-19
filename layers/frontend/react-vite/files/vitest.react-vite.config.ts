import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Named after the layer, so a second Vite frontend in the same workspace can
// bring its own without a collision.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/react-vite/**/*.spec.tsx"],
  },
});
