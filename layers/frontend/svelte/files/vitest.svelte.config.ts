import { fileURLToPath } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// This config file always sits at the workspace root, so a path built from
// it reaches into {{app_dir}} or {{test_dir}} without a relative "../" count
// that would depend on how many segments either one has.
function fromRoot(path: string): string {
  return fileURLToPath(new URL(path, import.meta.url));
}

// The bare svelte plugin rather than sveltekit(), because the SvelteKit plugin
// expects a running dev server and a component test has none. Named after the
// layer so a second frontend can bring its own config.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
    alias: {
      // The bare svelte plugin above does not run SvelteKit's own alias
      // resolution, so each of these has to be wired up by hand for anything
      // a tested component imports.
      $lib: fromRoot("./{{app_dir}}/lib"),
      "$app-routes": fromRoot("./{{app_dir}}/routes"),
      "$env/dynamic/public": fromRoot("./{{test_dir}}/env-dynamic-public.stub.ts"),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["{{test_dir}}/**/*.spec.ts"],
  },
});
