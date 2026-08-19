// === Errors

/** A failure carrying an HTTP status, so a route throws rather than assembling a response by hand. */
export class AppError extends Error {
  constructor(message, status = 500) {
    super(message);
    this.name = "AppError";
    this.status = status;
  }
}

/**
 * One body shape for every failure, so a client parses errors the same way
 * whatever produced them. An unexpected error is reported without its message,
 * because an internal detail is not a client's business.
 */
export function errorHandler(error, _request, reply) {
  const status = error.status ?? error.statusCode ?? 500;
  const message = status === 500 ? "internal server error" : error.message;
  reply.status(status).send({ error: message });
}
