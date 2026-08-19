// Read from the environment rather than baked in at build time, so the same
// build can run in more than one place. NEXT_PUBLIC_ is the prefix Next inlines
// into the browser bundle; anything without it stays on the server.
//
// The fallback is the project's own name, so an unset variable still renders
// something true rather than an empty heading.
export const appName: string = process.env.NEXT_PUBLIC_APP_NAME ?? "{{project_name}}";

// Deliberately empty when unset. A default here would be a URL somebody ships
// to production without noticing, where an empty value is visible in the page.
export const apiBase: string = process.env.NEXT_PUBLIC_API_BASE ?? "";
