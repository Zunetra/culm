import type { NextConfig } from "next";

// Next resolves the app directory at app/ (relative to this app's own root)
// and offers no way to move it, so unlike a framework with a configurable
// source root this layer claims that one path. A second frontend in the same
// workspace gets a directory named after its own layer, so nothing overlaps.
const config: NextConfig = {
  // Not this app's own top-level tsconfig.json name, because that name is
  // reserved for a file at the app's own root. It sits inside app/ instead,
  // because an editor finds a project only by walking up to a file called
  // tsconfig.json.
  typescript: {
    tsconfigPath: "app/tsconfig.json",
  },

  reactStrictMode: true,
};

export default config;
