import { defineVitestConfig } from "@nuxt/test-utils/config";

// Namespaced by layer id rather than claiming vitest.config.ts, which is a
// whole-workspace filename a second frontend would collide on.
export default defineVitestConfig({
  test: {
    include: ["test/nuxt/**/*.spec.ts"],
    environment: "nuxt",
  },
});
