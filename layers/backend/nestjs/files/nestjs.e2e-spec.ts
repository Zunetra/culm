// === The health endpoint, over the real application

import "reflect-metadata";

import type { INestApplication } from "@nestjs/common";
import { Test } from "@nestjs/testing";
import request from "supertest";

import { AllExceptionsFilter } from "../src/nestjs/all-exceptions.filter.js";
import { AppModule } from "../src/nestjs/app.module.js";

describe("health", () => {
  let app: INestApplication;

  /* The real module, so the test drives the routes the binary serves rather
   * than a copy of them that can drift. */
  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleRef.createNestApplication();
    app.useGlobalFilters(new AllExceptionsFilter());
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  it("reports ok", async () => {
    const response = await request(app.getHttpServer()).get("/health");

    expect(response.status).toBe(200);
    expect(response.body).toEqual({ status: "ok" });
  });

  it("returns the shared error body for an unknown path", async () => {
    const response = await request(app.getHttpServer()).get("/nope");

    expect(response.status).toBe(404);
    expect(response.body).toHaveProperty("error");
  });
});
