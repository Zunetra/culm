const express = require("express");

const app = express();

app.get("/health", (_request, response) => {
  response.json({ status: "ok" });
});

app.listen(Number(process.env.PORT || 3000));
