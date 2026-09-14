export default defineNuxtConfig({
  srcDir: ".",

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
