import type { ReactNode } from "react";

import { apiBase, appName } from "@/app/config";

export default function IndexPage(): ReactNode {
  return (
    <section>
      <h1>{appName}</h1>
      <p>The Next application is running.</p>

      {/* Shown rather than defaulted, so an unset NEXT_PUBLIC_API_BASE is
          something the first person to open the page notices. */}
      {apiBase ? (
        <p>
          Talking to <code>{apiBase}</code>
        </p>
      ) : (
        <p>
          No API base is configured. Set <code>NEXT_PUBLIC_API_BASE</code>.
        </p>
      )}
    </section>
  );
}
