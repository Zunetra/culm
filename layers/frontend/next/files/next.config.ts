import type { NextConfig } from "next";

// Next resolves the app directory at src/app and offers no way to move it, so
// unlike a framework with a configurable source root this layer claims that one
// path. A second frontend in the same workspace gets a directory named after
// its own layer, so nothing overlaps.
const config: NextConfig = {
  // Not the root tsconfig.json. That file belongs to the workspace, and two
  // frontends both claiming it would be a file collision rather than a merge.
  // It sits inside src/app rather than under another name at the root, because
  // an editor finds a project only by walking up to a file called
  // tsconfig.json.
  typescript: {
    tsconfigPath: "src/app/tsconfig.json",
  },

  reactStrictMode: true,
};

export default config;
