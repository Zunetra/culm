// === Test runner

/* Sits at this app's own root, alongside its own real package.json, so no
 * namespacing is needed: a second Node app in the same workspace brings its
 * own config in its own directory rather than fighting over one.
 *
 * `export default` rather than `module.exports`, and no .mjs rename, because
 * decision 21 puts `"type": "module"` at the workspace root and this file is
 * therefore already ESM. */
export default {
  rootDir: ".",
  testEnvironment: "node",
  testMatch: ["<rootDir>/test/nestjs.e2e-spec.ts"],
  extensionsToTreatAsEsm: [".ts"],
  transform: {
    "^.+\\.ts$": ["ts-jest", { tsconfig: "tsconfig.json", useESM: true }],
  },
  /* The sources import each other with a .js extension, which is what NodeNext
   * requires and what the emitted JavaScript actually resolves. Jest resolves
   * the TypeScript directly and never sees that output, so the extension has to
   * come back off before it looks. */
  moduleNameMapper: {
    "^(\\.{1,2}/.*)\\.js$": "$1",
  },
};
