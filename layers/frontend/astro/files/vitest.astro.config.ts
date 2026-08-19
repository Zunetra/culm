import { getViteConfig } from "astro/config";

// getViteConfig rather than a bare defineConfig, because rendering a component
// through the container API needs the same resolver aliases and integrations
// the real build uses.
export default getViteConfig({
  test: {
    globals: false,
    include: ["test/astro/**/*.spec.ts"],
  },
});
