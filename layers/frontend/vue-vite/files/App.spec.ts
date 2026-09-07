import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import App from "@vue-vite/App.vue";

describe("the application", () => {
  it("names the application", () => {
    const page = mount(App);

    expect(page.text()).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", () => {
    const page = mount(App);

    expect(page.text()).toContain("VITE_API_BASE");
  });
});
