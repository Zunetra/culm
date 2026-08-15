// === Errors

import {
  ArgumentsHost,
  Catch,
  ExceptionFilter,
  HttpException,
  HttpStatus,
} from "@nestjs/common";
import type { Response } from "express";

/**
 * One body shape for every failure, so a client parses errors the same way
 * whatever produced them. An unexpected error is reported without its message,
 * because an internal detail is not a client's business.
 */
@Catch()
export class AllExceptionsFilter implements ExceptionFilter {
  catch(exception: unknown, host: ArgumentsHost): void {
    const response = host.switchToHttp().getResponse<Response>();

    const status =
      exception instanceof HttpException
        ? exception.getStatus()
        : HttpStatus.INTERNAL_SERVER_ERROR;

    const message =
      status === HttpStatus.INTERNAL_SERVER_ERROR
        ? "internal server error"
        : exception instanceof HttpException
          ? exception.message
          : "error";

    response.status(status).json({ error: message });
  }
}
