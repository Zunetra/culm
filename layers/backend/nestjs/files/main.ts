// === Entry point

import "reflect-metadata";

import { NestFactory } from "@nestjs/core";

/* The .js extensions are deliberate and are not a mistake in a TypeScript file.
 * Under NodeNext the specifier is emitted verbatim, so it has to name the file
 * the output resolves rather than the one the source is written in. */
import { AllExceptionsFilter } from "./all-exceptions.filter.js";
import { AppModule } from "./app.module.js";
import { loadConfig } from "./config.js";

async function bootstrap(): Promise<void> {
  const config = loadConfig();
  const app = await NestFactory.create(AppModule);

  app.useGlobalFilters(new AllExceptionsFilter());

  await app.listen(config.port, config.host);
  console.log(`listening on http://${config.host}:${config.port}`);
}

void bootstrap();
