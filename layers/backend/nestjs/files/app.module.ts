import { Controller, Get, Module } from "@nestjs/common";

@Controller()
export class AppController {
  @Get()
  health(): { status: string } {
    return { status: "ok" };
  }
}

@Module({
  controllers: [AppController],
})
export class AppModule {}
