import { getViteConfig } from "astro/config";
import { fileURLToPath } from "node:url";

// getViteConfig rather than a bare defineConfig, because rendering a component
// through the container API needs the same resolver aliases and integrations
// the real build uses.
//
// The alias is resolved from this file's own location, which is always the
// workspace root regardless of the layout a single-app or multi-app selection
// gives {{app_dir}}, so the spec file can reach the source it exercises without
// a relative path whose "../" count would otherwise depend on that layout.
export default getViteConfig({
  resolve: {
    alias: {
      "@astro": fileURLToPath(new URL("./{{app_dir}}", import.meta.url)),
    },
  },
  test: {
    globals: false,
    include: ["{{test_dir}}/**/*.spec.ts"],
  },
});
