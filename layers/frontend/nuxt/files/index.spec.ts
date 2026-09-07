import { describe, expect, it } from "vitest";
import { mountSuspended } from "@nuxt/test-utils/runtime";

import IndexPage from "~/pages/index.vue";

describe("the entry page", () => {
  it("names the application", async () => {
    const page = await mountSuspended(IndexPage);

    expect(page.text()).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", async () => {
    const page = await mountSuspended(IndexPage);

    expect(page.text()).toContain("NUXT_PUBLIC_API_BASE");
  });
});
