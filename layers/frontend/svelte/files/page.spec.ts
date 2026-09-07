import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";

import Page from "$app-routes/+page.svelte";

// @testing-library/svelte's automatic cleanup hooks into a global afterEach,
// which globals: false above never registers, so each render is unmounted by
// hand or the next test finds two of everything.
afterEach(cleanup);

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
