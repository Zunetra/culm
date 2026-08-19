import { defineConfig } from "astro/config";

// Sources live under src/astro rather than at the root, so a workspace holding
// a second frontend has somewhere of its own to put them. Astro resolves pages,
// layouts and components inside srcDir, so all three move together.
export default defineConfig({
  srcDir: "src/astro",

  // Astro reads PUBLIC_-prefixed variables into the client bundle and leaves
  // the rest on the server. The values themselves are read where they are used
  // rather than restated here, so there is one place to change them.
  vite: {
    envPrefix: ["PUBLIC_"],
  },
});
