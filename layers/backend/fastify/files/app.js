// === The application

import Fastify from "fastify";

import { AppError, errorHandler } from "./errors.js";

/**
 * Builds the application without binding a port.
 * Separated from the server so an integration test drives the real routes in
 * process, rather than a copy of them that can drift.
 */
export function createApp(options = {}) {
  const app = Fastify(options);

  app.setErrorHandler(errorHandler);

  /* Routed rather than left to the framework's default, so an unknown path
   * fails in the same shape as every other error. */
  app.setNotFoundHandler((_request, reply) => {
    errorHandler(new AppError("not found", 404), _request, reply);
  });

  app.get("/health", async () => ({ status: "ok" }));

  return app;
}
