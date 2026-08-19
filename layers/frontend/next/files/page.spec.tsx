import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import IndexPage from "../../src/app/page";

// Plain vitest matchers rather than jest-dom, so the test needs no setup file
// and no extra dependency to say the same thing.
describe("the entry page", () => {
  it("names the application", () => {
    render(<IndexPage />);

    expect(screen.getByRole("heading").textContent).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", () => {
    render(<IndexPage />);

    expect(screen.queryByText(/NEXT_PUBLIC_API_BASE/)).not.toBeNull();
  });
});
