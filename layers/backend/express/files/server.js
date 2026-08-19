// === Entry point

import { createApp } from "./app.js";
import { loadConfig } from "./config.js";

const config = loadConfig();

createApp().listen(config.port, config.host, () => {
  console.log(`listening on http://${config.host}:${config.port}`);
});
