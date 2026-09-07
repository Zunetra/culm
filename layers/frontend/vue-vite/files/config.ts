// Read from the environment rather than baked in at build time, so the same
// build can run in more than one place. VITE_ is the prefix Vite exposes to the
// browser; anything without it is not part of the bundle.
//
// The fallback is the project's own name, so an unset variable still renders
// something true rather than an empty heading.
export const appName: string =
  import.meta.env.VITE_APP_NAME ?? "{{project_name}}";

// Deliberately empty when unset. A default here would be a URL somebody ships
// to production without noticing, where an empty value is visible in the page.
export const apiBase: string = import.meta.env.VITE_API_BASE ?? "";
