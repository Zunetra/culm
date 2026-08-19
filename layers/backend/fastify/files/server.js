// === Entry point

import { createApp } from "./app.js";
import { loadConfig } from "./config.js";

const config = loadConfig();

createApp({ logger: true })
  .listen({ host: config.host, port: config.port })
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
