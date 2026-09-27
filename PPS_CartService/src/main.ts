import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { DynamoDBDocumentClient } from "@aws-sdk/lib-dynamodb";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import { MemcachedCartCache } from "./infrastructure/memcached-cart-cache.js";
import { DynamoCartDatabase } from "./infrastructure/dynamo-cart-database.js";
import { CartRepository } from "./repositories/cart-repository.js";
import { CartService } from "./services/cart-service.js";

const config = loadConfig();
const logger = pino({ level: config.LOG_LEVEL, redact: ["req.headers.authorization"] });
const dynamoClient = new DynamoDBClient({
  region: config.AWS_REGION,
  ...(config.DYNAMODB_ENDPOINT ? { endpoint: config.DYNAMODB_ENDPOINT } : {}),
});
const documentClient = DynamoDBDocumentClient.from(dynamoClient, {
  marshallOptions: { removeUndefinedValues: true },
});
const database = new DynamoCartDatabase(documentClient, config.DYNAMODB_CART_TABLE);
const cache = new MemcachedCartCache(
  config.MEMCACHED_SERVERS,
  config.MEMCACHED_USERNAME,
  config.MEMCACHED_PASSWORD,
);
const repository = new CartRepository(database, cache, logger);
const service = new CartService(repository);
const app = createApp({
  service,
  logger,
  trustedUserHeader: config.TRUSTED_USER_HEADER,
  corsOrigin: config.CORS_ORIGIN,
});

const server = app.listen(config.PORT, () => {
  logger.info({ port: config.PORT }, "Cart service listening");
});

async function shutdown(signal: string): Promise<void> {
  logger.info({ signal }, "Shutting down cart service");
  server.close();
  cache.close();
  dynamoClient.destroy();
}

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, () => void shutdown(signal));
}
