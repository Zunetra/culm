import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import Page from "../../src/svelte/routes/+page.svelte";

describe("the entry page", () => {
  it("names the application", () => {
    render(Page);

    expect(screen.getByRole("heading").textContent).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", () => {
    render(Page);

    expect(screen.queryByText(/PUBLIC_API_BASE/)).not.toBeNull();
  });
});
