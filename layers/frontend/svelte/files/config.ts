// Read from the environment rather than baked in at build time, so the same
// build can run in more than one place. PUBLIC_ is the prefix SvelteKit allows
// into the browser bundle; anything without it is server-only and importing it
// from a component is a build error rather than a leak.
//
// The fallback is the project's own name, so an unset variable still renders
// something true rather than an empty heading.
import { env } from "$env/dynamic/public";

export const appName: string = env.PUBLIC_APP_NAME ?? "{{project_name}}";

// Deliberately empty when unset. A default here would be a URL somebody ships
// to production without noticing, where an empty value is visible in the page.
export const apiBase: string = env.PUBLIC_API_BASE ?? "";
