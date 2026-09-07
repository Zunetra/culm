/* Stands in for SvelteKit's $env/dynamic/public module under vitest. The bare
 * svelte plugin the component tests run under has no dev server, so it never
 * injects that virtual module the way SvelteKit's own vite plugin would. An
 * empty object mirrors the unconfigured case the application already treats
 * as its default. */
export const env: Record<string, string | undefined> = {};
