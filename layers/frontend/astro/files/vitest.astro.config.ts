/// <reference types="vitest/config" />
import { getViteConfig } from "astro/config";
import { fileURLToPath } from "node:url";

export default getViteConfig({
  resolve: {
    alias: {
      "@astro": fileURLToPath(new URL(".", import.meta.url)),
    },
  },
  test: {
    globals: false,
    include: ["test/**/*.spec.ts"],
  },
});
