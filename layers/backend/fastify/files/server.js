const fastify = require("fastify")();

fastify.get("/health", async () => ({ status: "ok" }));

fastify.listen({ port: Number(process.env.PORT || 3000) });
