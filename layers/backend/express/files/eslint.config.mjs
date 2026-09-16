// === Node environment for this layer's own files
//
// The root config's default match already covers plain .js, but it declares
// no environment, so this layer's own Node globals go undetected without
// this. Scoped to this layer's own paths, not left global, for the same
// reason every other layer's fragment is: a second backend in the same
// workspace claims its own paths and the two fragments never overlap.
export default [
  {
    files: ["{{app_dir}}/**/*.js", "{{test_dir}}/**/*.js"],
    languageOptions: {
      globals: {
        process: "readonly",
        console: "readonly",
        fetch: "readonly",
      },
    },
    rules: {
      /* The shared error handler's signature is fixed by Express, so an
       * unused trailing parameter is how a middleware declares its arity
       * rather than a mistake. Prefixed with an underscore rather than
       * silenced workspace-wide. */
      "no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
    },
  },
];
