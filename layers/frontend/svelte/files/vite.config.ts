import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

// Named after the layer rather than called vite.config.ts, so a workspace
// holding a second Vite frontend can bring its own. The dev script passes this
// file with --config.
export default defineConfig({
  plugins: [sveltekit()],
});
