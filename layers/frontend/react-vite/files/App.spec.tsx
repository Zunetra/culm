import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "@react-vite/App";

// Plain vitest matchers rather than jest-dom, so the test needs no setup file
// and no extra dependency to say the same thing.
// Testing Library registers its own cleanup only when a global afterEach
// exists, and the vitest config here sets globals: false deliberately. Without
// this line every render accumulates into the same document and the second test
// in this file matches twice.
afterEach(cleanup);

describe("the application", () => {
  it("names the application", () => {
    render(<App />);

    expect(screen.getByRole("heading").textContent).toContain("{{project_name}}");
  });

  // The unconfigured case is the one a new checkout is actually in, so it is
  // the one worth pinning.
  it("says so when no API base is configured", () => {
    render(<App />);

    expect(screen.queryByText(/VITE_API_BASE/)).not.toBeNull();
  });
});
