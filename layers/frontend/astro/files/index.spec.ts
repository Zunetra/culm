import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";

import IndexPage from "@astro/pages/index.astro";

// The container renders a component to a string without starting a server,
// which is the only way to assert on a page that has no client runtime.
describe("the entry page", () => {
  it("names the application", async () => {
    const container = await AstroContainer.create();
    const html = await container.renderToString(IndexPage);

    expect(html).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", async () => {
    const container = await AstroContainer.create();
    const html = await container.renderToString(IndexPage);

    expect(html).toContain("PUBLIC_API_BASE");
  });
});
