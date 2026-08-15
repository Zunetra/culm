// === Configuration

/**
 * Everything the service reads from the environment.
 * Resolved once at startup rather than at each use, so a misconfigured
 * deployment fails immediately instead of on the first request that happens to
 * need a value.
 */
export function loadConfig(env = process.env) {
  const port = Number(env.PORT ?? 3000);
  if (!Number.isInteger(port) || port < 0 || port > 65535) {
    throw new Error(`PORT is set to '${env.PORT}', which is not a port number`);
  }
  return { host: env.HOST ?? "0.0.0.0", port };
}
