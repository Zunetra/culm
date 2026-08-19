import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

// Named after the layer, so a second Vite frontend in the same workspace can
// bring its own without a collision.
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/vue-vite/**/*.spec.ts"],
  },
});
