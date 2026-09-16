// === Errors

/** A failure carrying an HTTP status, so a route throws rather than assembling a response by hand. */
export class AppError extends Error {
  constructor(message, status = 500) {
    super(message);
    this.name = "AppError";
    this.status = status;
  }
}

/* Registered rather than left to the framework's default, so an unknown path
 * fails in the same shape as every other error. */
export function notFound(_request, _response, next) {
  next(new AppError("not found", 404));
}

/**
 * One body shape for every failure, so a client parses errors the same way
 * whatever produced them. An unexpected error is reported without its message,
 * because an internal detail is not a client's business.
 */
export function errorHandler(error, _request, response, _next) {
  const status = error instanceof AppError ? error.status : 500;
  const message = status === 500 ? "internal server error" : error.message;
  response.status(status).json({ error: message });
}
