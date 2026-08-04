import js from "@eslint/js";

// Flat config. There is no .eslintignore: flat config removed it, and ignores
// live in the config itself.
export default [
  js.configs.recommended,
  {
    ignores: ["node_modules/", "dist/", "build/", "coverage/"],
  },
];
