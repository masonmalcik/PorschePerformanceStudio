import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { DynamoDBDocumentClient } from "@aws-sdk/lib-dynamodb";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import { DynamoCartDatabase } from "./infrastructure/dynamo-cart-database.js";
import { MemcachedCartCache } from "./infrastructure/memcached-cart-cache.js";
import { NullCartCache } from "./infrastructure/null-cart-cache.js";
import { CartRepository } from "./repositories/cart-repository.js";
import { CartService } from "./services/cart-service.js";

export function createRuntime() {
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
  const cache = config.MEMCACHED_SERVERS === "disabled"
    ? new NullCartCache()
    : new MemcachedCartCache(config.MEMCACHED_SERVERS, config.MEMCACHED_USERNAME, config.MEMCACHED_PASSWORD);
  const service = new CartService(new CartRepository(database, cache, logger));
  const app = createApp({ service, logger, trustedUserHeader: config.TRUSTED_USER_HEADER, corsOrigin: config.CORS_ORIGIN });
  return { app, config, logger, dynamoClient, cache };
}
