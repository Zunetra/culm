// === The application

import express from "express";

import { errorHandler, notFound } from "./errors.js";

/**
 * Builds the application without binding a port.
 * Separated from the server so an integration test drives the real routes in
 * process, rather than a copy of them that can drift.
 */
export function createApp() {
  const app = express();

  app.use(express.json());

  app.get("/health", (_request, response) => {
    response.json({ status: "ok" });
  });

  app.use(notFound);
  app.use(errorHandler);

  return app;
}
