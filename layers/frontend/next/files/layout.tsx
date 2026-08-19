import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";

import { appName } from "@/app/config";

// Exported rather than assembled in the page, so every route inherits the title
// and a route that forgets to set one still gets the application's.
export const metadata: Metadata = {
  title: appName,
  description: `The ${appName} application.`,
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
};

export default function RootLayout({
  children,
}: {
  children: ReactNode;
}): ReactNode {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
