import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Named after the layer, so a second Vite frontend in the same workspace can
// bring its own without a collision.
//
// The alias is resolved from this file's own location, which is always the
// workspace root regardless of the layout a single-app or multi-app selection
// gives {{app_dir}}, so the spec file can reach the source it exercises without
// a relative path whose "../" count would otherwise depend on that layout.
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@vue-vite": fileURLToPath(new URL("./{{app_dir}}", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["{{test_dir}}/**/*.spec.ts"],
  },
});
