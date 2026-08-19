// Sources live under src/nuxt rather than at the root, so a workspace holding a
// second frontend has somewhere of its own to put it. Nuxt's directory
// conventions all resolve inside srcDir, so pages, layouts and app.vue move
// with it and nothing else has to be told where they went.
export default defineNuxtConfig({
  srcDir: "src/nuxt",

  // Read at runtime, not baked in at build time. Public keys reach the browser
  // and are overridden by NUXT_PUBLIC_*; anything outside `public` stays on the
  // server and is overridden by NUXT_*. An empty default is deliberate: a
  // missing value should be visible in the page, not silently a placeholder
  // someone ships to production.
  runtimeConfig: {
    public: {
      appName: "{{project_name}}",
      apiBase: "",
    },
  },

  typescript: {
    strict: true,
    typeCheck: false,
  },

  devtools: { enabled: true },
});
