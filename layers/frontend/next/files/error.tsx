"use client";

import type { ReactNode } from "react";

// A route error boundary has to be a client component: Next hands it a reset
// callback, and a server component has nowhere to put the handler.
export default function RouteError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}): ReactNode {
  // The thrown message is only trustworthy in development. In production the
  // digest is the safe handle: it identifies the error in the server log
  // without putting a stack trace in front of whoever hit the page.
  const detail: string =
    process.env.NODE_ENV === "development"
      ? error.message
      : "Something went wrong.";

  return (
    <section>
      <h1>Something went wrong</h1>
      <p>{detail}</p>
      {error.digest ? (
        <p>
          Reference <code>{error.digest}</code>
        </p>
      ) : null}
      <button type="button" onClick={reset}>
        Try again
      </button>
    </section>
  );
}
