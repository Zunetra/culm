import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// The bare svelte plugin rather than sveltekit(), because the SvelteKit plugin
// expects a running dev server and a component test has none. Named after the
// layer so a second frontend can bring its own config.
export default defineConfig({
  plugins: [svelte({ hot: false })],
  resolve: {
    conditions: ["browser"],
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/svelte/**/*.spec.ts"],
  },
});
