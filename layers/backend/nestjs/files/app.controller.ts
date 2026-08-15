// === Routes

import { Controller, Get } from "@nestjs/common";

interface HealthBody {
  status: string;
}

@Controller()
export class AppController {
  /** Answers the liveness probe. */
  @Get("health")
  health(): HealthBody {
    return { status: "ok" };
  }
}
