// === Node globals for this layer's own sources

/* This service runs under Node, not a browser, and process and console are
 * the only globals its config, server and test files reach for. Named
 * individually rather than pulled from a whole environment package, the same
 * way the workspace's other fragments handle a global the base config does
 * not already know. */
export default [
  {
    files: ["{{app_dir}}/**/*.js", "{{test_dir}}/fastify.test.js"],
    languageOptions: {
      globals: {
        process: "readonly",
        console: "readonly",
      },
    },
  },
];
