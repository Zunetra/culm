import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { ErrorBoundary } from "./ErrorBoundary";

const container = document.getElementById("root");

// Thrown rather than defaulted. A missing mount point means the HTML and this
// entry point disagree, and rendering into a silently created element would
// hide that until somebody wonders why the page is blank.
if (!container) {
  throw new Error("no #root element to mount into");
}

createRoot(container).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
