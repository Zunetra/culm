import { fileURLToPath } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

function fromRoot(path: string): string {
  return fileURLToPath(new URL(path, import.meta.url));
}

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
    alias: {
      $lib: fromRoot("./lib"),
      "$app-routes": fromRoot("./routes"),
      "$env/dynamic/public": fromRoot("./test/env-dynamic-public.stub.ts"),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["test/**/*.spec.ts"],
  },
});
