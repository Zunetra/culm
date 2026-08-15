// === Configuration

/** Everything the service reads from the environment. */
export interface Config {
  host: string;
  port: number;
}

/**
 * Reads the environment once at startup, rather than at each use, so a
 * misconfigured deployment fails immediately instead of on the first request
 * that happens to need a value.
 */
export function loadConfig(env: NodeJS.ProcessEnv = process.env): Config {
  const port = Number(env.PORT ?? 3000);
  if (!Number.isInteger(port) || port < 0 || port > 65535) {
    throw new Error(`PORT is set to '${env.PORT}', which is not a port number`);
  }
  return { host: env.HOST ?? "0.0.0.0", port };
}
