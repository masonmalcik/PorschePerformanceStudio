import { createRuntime } from "./runtime.js";

const { app, config, logger, cache, dynamoClient } = createRuntime();

const server = app.listen(config.PORT, () => {
  logger.info({ port: config.PORT }, "Cart service listening");
});

async function shutdown(signal: string): Promise<void> {
  logger.info({ signal }, "Shutting down cart service");
  server.close();
  if ("close" in cache && typeof cache.close === "function") cache.close();
  dynamoClient.destroy();
}

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, () => void shutdown(signal));
}
