// === The application module

import { Module } from "@nestjs/common";

import { AppController } from "./app.controller.js";

@Module({
  controllers: [AppController],
})
export class AppModule {}
